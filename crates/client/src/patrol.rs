//! Running the map on a route, with nobody fighting.
//!
//! This exists to split a question that the autopilot cannot answer on its
//! own. Turning the autopilot on and watching the phone warm up says only
//! that *something* in it costs heat -- it picks targets, casts line-of-sight
//! rays every tick, leans, dodges, aims and shoots, all at once. Patrol walks
//! the same ground, through the same navmesh, and does none of the rest. Run
//! both and the difference is the fighting.
//!
//! `PATROL=1` to walk, `PATROL_FIRE=1` to also shoot at what it passes. The
//! two are separate on purpose: shooting is the loudest thing a frame can do
//! -- muzzle flash, tracer, casing, blood, sound -- so it is worth weighing
//! on its own rather than bundled into "the autopilot".
//!
//! The route is authored beside the map, in the same format and by the same
//! editor as the zombie spawns: see `shared::patrol_points`.

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use bevy_landmass::prelude::{AgentDesiredVelocity3d, AgentTarget3d, Archipelago3d};

use crate::{FpsCamera, MouseLookDelta, Player};
use shared::{NetPlayer, Zombie};

/// Whether the patrol is walking.
#[derive(Resource)]
pub(crate) struct Patrol(pub(crate) bool);

impl Default for Patrol {
    fn default() -> Self {
        Self(std::env::var("PATROL").is_ok())
    }
}

/// Whether it shoots at what it passes.
#[derive(Resource)]
pub(crate) struct PatrolFire(pub(crate) bool);

impl Default for PatrolFire {
    fn default() -> Self {
        Self(std::env::var("PATROL_FIRE").is_ok())
    }
}

/// Whether anybody other than the player's hands is steering.
///
/// Both drivers write `AutopilotIntent`, and everything that reads it -- the
/// walk, the look, the lean, the trigger -- cares only that it is being
/// written by something. One flag rather than `autopilot.0 || patrol.0`
/// repeated at four call sites that must never disagree.
#[derive(Resource, Default)]
pub(crate) struct Driven(pub(crate) bool);

/// How often the route is reconsidered: has it arrived, is it stuck, which
/// way does the navmesh say to go.
///
/// Ten a second against a walk of six metres a second is a decision every
/// sixty centimetres, which is finer than the arrival radius. The turn and
/// the move axis are recomputed every frame from the answer, because those
/// are a few floats and a stuttering view is the one artefact that would
/// make a performance capture unreadable.
const PATROL_DECISION_HZ: f32 = 10.0;

/// Close enough to call it arrived. Wider than it looks: landmass steers to
/// the point, and a route that demands the exact metre spends its time
/// shuffling on the spot instead of walking.
const PATROL_ARRIVE_RADIUS: f32 = 3.0;

/// How far above or below a waypoint still counts as standing on it.
///
/// Without this, arrival is measured on the floor plan alone, and the patrol
/// declares it has reached a waypoint while standing on a terrace two metres
/// under it -- which is how the first run of this ended up wedged on a lower
/// level with every remaining waypoint unreachable.
const PATROL_ARRIVE_HEIGHT: f32 = 3.0;

/// How long a waypoint may fail to get closer before it is given up on.
///
/// Twelve seconds, not the six it started at. A route around a building
/// takes the straight-line distance *up* for a while, and six was short
/// enough to call that a failure and abandon waypoints mid-walk.
///
/// This is the check that a point is actually reachable, and it runs every
/// time rather than once at startup: the navmesh is streamed in as chunks and
/// is not there to ask at startup, and a point can be walkable from one side
/// of the map and not another.
const PATROL_STUCK_SECONDS: f32 = 12.0;
const PATROL_STUCK_PROGRESS: f32 = 1.0;

/// How long the player may not move at all before it is put back on the map.
///
/// A different failure from the one above: there the route is wrong, here the
/// body is wedged -- off a ledge, in a corner -- and no waypoint is reachable
/// because none of them can be walked to from where it is standing. Picking
/// another waypoint cannot help, so it is lifted to the one it was heading
/// for.
///
/// A measurement rig that dies in ninety seconds measures nothing, and this
/// is meant to be left running for an hour. The recovery is printed every
/// time so a run that spent itself teleporting is never mistaken for a run
/// that walked.
const PATROL_UNSTICK_SECONDS: f32 = 20.0;
const PATROL_UNSTICK_PROGRESS: f32 = 1.5;

/// How often the patrol looks for something to shoot at.
///
/// Twice a second. The scan is a distance test over every zombie plus one ray
/// for the nearest, and nothing it can find needs answering faster than that
/// -- the cadence below is slower still.
const PATROL_SCAN_HZ: f32 = 2.0;

/// How far it will shoot, and how close it has to be pointed first.
const PATROL_FIRE_RANGE: f32 = 45.0;
const PATROL_FIRE_AIM_TOLERANCE_DEG: f32 = 8.0;

/// One shot every this long, at most.
///
/// Deliberately slow. The point of the patrol is to hold everything else
/// still while the map is walked, and a held trigger would bury that under a
/// muzzle flash, a tracer, a casing and a sound forty times a second.
const PATROL_FIRE_INTERVAL: f32 = 1.2;

/// How fast the view swings, in degrees a second.
const PATROL_TURN_RATE_DEG: f32 = 180.0;

/// Chest height on a zombie, measured from the transform origin at its feet.
const PATROL_AIM_HEIGHT: f32 = 1.2;

#[derive(Resource, Default)]
pub(crate) struct PatrolRoute {
    points: Vec<Vec3>,
    /// Which point it is walking to, and the navmesh position it was given.
    /// Held so the target is written once per leg rather than once per tick:
    /// landmass re-paths when the target changes, and a target rewritten ten
    /// times a second is ten paths a second for one journey.
    heading_for: Option<usize>,
    assigned: Option<Vec3>,
    /// Which way the navmesh last said to walk, in world space.
    travel: Option<Vec3>,
    decision_timer: f32,
    /// Closest it has come to the current waypoint, and how long ago.
    best_distance: f32,
    stalled_for: f32,
    /// Where the body actually was, and for how long it has been there.
    last_seen: Vec3,
    wedged_for: f32,
    scan_timer: f32,
    fire_timer: f32,
    /// Where to shoot, refreshed by the scan rather than tracked per frame.
    aim_at: Option<Vec3>,
    /// A plain counter hashed into a choice. Fixed seed on purpose: two runs
    /// of the same build walk the same route, which is the difference between
    /// a comparable measurement and an anecdote.
    rng: u32,
}

impl PatrolRoute {
    fn next_index(&mut self) -> Option<usize> {
        if self.points.is_empty() {
            return None;
        }
        if self.points.len() == 1 {
            return Some(0);
        }
        // Drawn from the points that are not the one being left, so a draw
        // can never be "stay where you are" -- which would read as arriving
        // repeatedly and never moving.
        self.rng = self.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let draw = (self.rng >> 16) as usize % (self.points.len() - 1);
        let current = self.heading_for.unwrap_or(usize::MAX);
        Some(if draw >= current { draw + 1 } else { draw })
    }
}

pub(crate) fn load_patrol_route(mut route: ResMut<PatrolRoute>, patrol: Res<Patrol>) {
    route.points = shared::patrol_points(crate::current_scenario());
    route.rng = 0x5eed_1337;
    if patrol.0 {
        // println! rather than info!: tracing does not reach stdout on iOS,
        // and this is read from a device console more often than a terminal.
        println!("[patrol] {} waypoint(s) loaded", route.points.len());
    }
}

/// One flag for "something other than a hand is steering", and the armour
/// that keeps a patrol alive long enough to measure.
///
/// The patrol walks past zombies without shooting unless asked to, so it gets
/// eaten -- and a dead player stops `send_player_input` dead, which looks
/// exactly like a navigation failure from the outside. That cost a whole
/// round of debugging before a run with no zombies in it made it obvious.
///
/// The previous setting is remembered and handed back, so turning patrol off
/// does not leave the player quietly invulnerable.
pub(crate) fn track_driven(
    autopilot: Res<crate::Autopilot>,
    patrol: Res<Patrol>,
    mut driven: ResMut<Driven>,
    mut god: ResMut<crate::GodMode>,
    mut god_before: Local<Option<bool>>,
) {
    driven.0 = autopilot.0 || patrol.0;
    match (patrol.0, *god_before) {
        (true, None) => {
            *god_before = Some(god.0);
            god.0 = true;
            println!("[patrol] invulnerable while patrolling, so a zombie cannot end the run");
        }
        (false, Some(before)) => {
            god.0 = before;
            *god_before = None;
        }
        _ => {}
    }
}

/// Walking the route, and shooting on the way if asked.
///
/// One system rather than two because the view has to have one author. A
/// separate shooter would fight the walker for the camera every frame that a
/// zombie was in range, and the view would shear between the two.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn patrol_think(
    time: Res<Time>,
    patrol: Res<Patrol>,
    fire: Res<PatrolFire>,
    mut route: ResMut<PatrolRoute>,
    mut intent: ResMut<crate::AutopilotIntent>,
    mut look_delta: ResMut<MouseLookDelta>,
    mut camera: Query<&mut FpsCamera>,
    mut player: Query<&mut Transform, (With<Player>, Without<NetPlayer>)>,
    archipelagos: Query<&Archipelago3d>,
    mut pathing: Query<
        (&mut AgentTarget3d, &AgentDesiredVelocity3d),
        (With<Player>, Without<NetPlayer>),
    >,
    zombies: Query<&GlobalTransform, With<Zombie>>,
    spatial_query: SpatialQuery,
) {
    if !patrol.0 {
        return;
    }
    intent.fire = false;
    let (Ok(mut cam), Ok(mut player_transform)) = (camera.single_mut(), player.single_mut()) else {
        return;
    };
    let at = player_transform.translation;

    route.decision_timer -= time.delta_secs();
    if route.decision_timer <= 0.0 {
        route.decision_timer = 1.0 / PATROL_DECISION_HZ;
        if let Some(rescue) = advance_leg(&mut route, at, &archipelagos, &mut pathing) {
            player_transform.translation = rescue;
        }
    }

    route.scan_timer -= time.delta_secs();
    if route.scan_timer <= 0.0 {
        route.scan_timer = 1.0 / PATROL_SCAN_HZ;
        route.aim_at =
            fire.0.then(|| nearest_visible_zombie(&zombies, &spatial_query, at)).flatten();
    }

    // Shooting borrows the view; walking gets it back the moment there is
    // nothing to shoot at.
    let look_at = route.aim_at.map(|point| point - at).or(route.travel);
    let on_target = turn_towards(&mut cam, &mut look_delta, look_at, time.delta_secs());

    intent.move_axis = route
        .travel
        .map(|dir| yaw_relative_axis(cam.yaw, dir))
        .unwrap_or(Vec2::ZERO);
    // Never sprints. A sprint carries the weapon across the body and changes
    // what the frame draws, which is a different thing being measured.
    intent.sprint = false;
    intent.aim = false;

    route.fire_timer -= time.delta_secs();
    if route.aim_at.is_some() && on_target && route.fire_timer <= 0.0 {
        route.fire_timer = PATROL_FIRE_INTERVAL;
        intent.fire = true;
    }
}

/// Arrival, giving up, and asking the navmesh which way to walk.
///
/// Returns somewhere to put the player when it has stopped being able to walk
/// there itself.
fn advance_leg(
    route: &mut PatrolRoute,
    at: Vec3,
    archipelagos: &Query<&Archipelago3d>,
    pathing: &mut Query<
        (&mut AgentTarget3d, &AgentDesiredVelocity3d),
        (With<Player>, Without<NetPlayer>),
    >,
) -> Option<Vec3> {
    let step = 1.0 / PATROL_DECISION_HZ;
    // Measured against the body, not the waypoint: a wedge survives every
    // change of destination, so the clock for it has to as well.
    if at.distance(route.last_seen) > PATROL_UNSTICK_PROGRESS {
        route.last_seen = at;
        route.wedged_for = 0.0;
    } else {
        route.wedged_for += step;
    }

    let Some(index) = route.heading_for else {
        choose_next(route);
        return None;
    };
    let Some(waypoint) = route.points.get(index).copied() else {
        choose_next(route);
        return None;
    };

    let flat = (waypoint - at) * Vec3::new(1.0, 0.0, 1.0);
    let distance = flat.length();
    if distance < PATROL_ARRIVE_RADIUS && (waypoint.y - at.y).abs() < PATROL_ARRIVE_HEIGHT {
        println!("[patrol] reached waypoint {index}");
        choose_next(route);
        return None;
    }

    // Progress, or the lack of it. A waypoint nothing can path to looks
    // exactly like one behind a wall: the distance simply stops falling.
    if distance < route.best_distance - PATROL_STUCK_PROGRESS {
        route.best_distance = distance;
        route.stalled_for = 0.0;
    } else {
        route.stalled_for += step;
        if route.stalled_for > PATROL_STUCK_SECONDS {
            let steer = route
                .travel
                .map(|d| format!("{:.2},{:.2}", d.x, d.z))
                .unwrap_or_else(|| "none".to_string());
            println!(
                "[patrol] waypoint {index} not closing from {:.0},{:.1},{:.0} -- {distance:.0}m out, navmesh says {steer}, skipping",
                at.x, at.y, at.z
            );
            choose_next(route);
            return None;
        }
    }

    let snapped = archipelagos
        .single()
        .ok()
        .and_then(|archipelago| {
            archipelago
                .sample_point(waypoint, &crate::navigation::off_mesh_target_tolerance())
                .ok()
                .map(|sampled| sampled.point())
        })
        .unwrap_or(waypoint);
    if route.wedged_for > PATROL_UNSTICK_SECONDS {
        println!(
            "[patrol] wedged at {:.0},{:.1},{:.0} for {PATROL_UNSTICK_SECONDS:.0}s -- lifting to waypoint {index}",
            at.x, at.y, at.z
        );
        route.wedged_for = 0.0;
        route.last_seen = snapped;
        choose_next(route);
        return Some(snapped);
    }

    let Ok((mut agent_target, desired)) = pathing.single_mut() else {
        return None;
    };
    // Written once a leg. Assigning the same point again is a fresh path
    // request for a journey already under way, which is the whole of what
    // "don't path every frame" means here.
    if route.assigned != Some(snapped) {
        *agent_target = AgentTarget3d::Point(snapped);
        route.assigned = Some(snapped);
    }
    let velocity = desired.velocity();
    let direction = Vec3::new(velocity.x, 0.0, velocity.z).normalize_or_zero();
    // Falls back to the straight line while landmass is still thinking, so
    // the first moments of a leg are a walk rather than a stand.
    route.travel = Some(if direction == Vec3::ZERO { flat.normalize_or_zero() } else { direction });
    None
}

fn choose_next(route: &mut PatrolRoute) {
    route.heading_for = route.next_index();
    route.assigned = None;
    route.travel = None;
    route.best_distance = f32::MAX;
    route.stalled_for = 0.0;
}

/// The closest zombie that can actually be shot, or nothing.
fn nearest_visible_zombie(
    zombies: &Query<&GlobalTransform, With<Zombie>>,
    spatial_query: &SpatialQuery,
    eye: Vec3,
) -> Option<Vec3> {
    let mut best: Option<(f32, Vec3)> = None;
    for transform in zombies {
        let point = transform.translation() + Vec3::Y * PATROL_AIM_HEIGHT;
        let distance = point.distance(eye);
        if distance > PATROL_FIRE_RANGE {
            continue;
        }
        if best.is_none_or(|(closest, _)| distance < closest) {
            best = Some((distance, point));
        }
    }
    // One ray, for the nearest only. Casting for every zombie in range is
    // what the autopilot does and is a cost this is trying to leave out.
    let (distance, point) = best?;
    let direction = Dir3::new(point - eye).ok()?;
    let filter = SpatialQueryFilter::default();
    spatial_query
        .cast_ray(eye, direction, distance - 0.5, true, &filter)
        .is_none()
        .then_some(point)
}

/// Swings the view towards a direction, and says whether it arrived.
fn turn_towards(
    cam: &mut FpsCamera,
    look_delta: &mut MouseLookDelta,
    towards: Option<Vec3>,
    dt: f32,
) -> bool {
    let Some(towards) = towards.filter(|v| *v != Vec3::ZERO) else {
        return false;
    };
    let desired_yaw = (-towards.x).atan2(-towards.z);
    let flat = towards.with_y(0.0).length().max(0.01);
    let desired_pitch = towards.y.atan2(flat).clamp(-1.5, 1.5);

    let step = PATROL_TURN_RATE_DEG.to_radians() * dt;
    let yaw_error = shortest_angle(cam.yaw, desired_yaw);
    let pitch_error = desired_pitch - cam.pitch;
    let turned_yaw = cam.yaw + yaw_error.clamp(-step, step);
    let turned_pitch = cam.pitch + pitch_error.clamp(-step, step);

    // The same report a hand would leave. Everything that rides the look --
    // the weapon lagging the view, the roll into a swing -- reads this, and
    // mouse_look stands down while anything else is steering.
    look_delta.0 += Vec2::new(-shortest_angle(cam.yaw, turned_yaw), -(turned_pitch - cam.pitch));
    cam.yaw = turned_yaw;
    cam.pitch = turned_pitch;

    yaw_error.abs() < PATROL_FIRE_AIM_TOLERANCE_DEG.to_radians()
        && pitch_error.abs() < PATROL_FIRE_AIM_TOLERANCE_DEG.to_radians()
}

fn shortest_angle(from: f32, to: f32) -> f32 {
    let raw = (to - from).rem_euclid(std::f32::consts::TAU);
    if raw > std::f32::consts::PI { raw - std::f32::consts::TAU } else { raw }
}

fn yaw_relative_axis(yaw: f32, world_dir: Vec3) -> Vec2 {
    let forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());
    Vec2::new(world_dir.dot(right), world_dir.dot(forward)).normalize_or_zero()
}
