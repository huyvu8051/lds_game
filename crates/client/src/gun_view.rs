use bevy::prelude::*;

use crate::*;






/// One mark per kind of glass, built once and swapped on the single reticle
/// entity rather than respawned, so nothing has to be rewired on a weapon
/// change.




/// Pins the sights up so the scope can be looked at without waiting for the
/// autopilot to pick a target far enough away to want them.
pub(crate) fn force_ads() -> bool {
    std::env::var("FORCE_ADS").map(|value| value == "1").unwrap_or(false)
}

/// Where the eye lines up behind the sights: the rear glass on a scope, the
/// rear sight on irons. Distinct from EyepieceSocket, which is where the
/// picture draws, further down the tube than this.
#[derive(Component)]
pub(crate) struct AimSocket;

/// Where the brass leaves the weapon, marked on the mesh so the shell comes
/// out of the port rather than out of a number that has to be guessed against
/// a receiver nobody measured.
#[derive(Component)]
pub(crate) struct EjectSocket;

/// The front post on a weapon that sights down irons. Paired with AimSocket it
/// gives the sight line, the same way the scope pair gives the optical axis.
#[derive(Component)]
pub(crate) struct FrontSightSocket;

/// The bore's end. Everything that leaves the barrel is placed and aimed from
/// here, so flash, smoke and casings follow the weapon through recoil and sway
/// without any of them holding their own copy of where the barrel is.
#[derive(Component)]
pub(crate) struct MuzzleSocket;

/// Caches where the bore ends inside the weapon.
///
/// The socket does not move within the weapon, so reparenting last frame's
/// world transforms yields a value that is exact rather than one frame late.
/// What it is composed with afterwards is the current weapon transform.
pub(crate) fn track_muzzle_socket(
    muzzle: Query<&GlobalTransform, With<MuzzleSocket>>,
    eject: Query<&GlobalTransform, With<EjectSocket>>,
    mut gun: Query<(&GlobalTransform, &mut Gun)>,
) {
    let Ok((gun_global, mut gun)) = gun.single_mut() else { return };
    if let Ok(socket) = muzzle.single() {
        gun.muzzle_local = socket.reparented_to(gun_global);
    }
    if let Ok(socket) = eject.single() {
        gun.eject_local = socket.reparented_to(gun_global);
    }
}


/// Hands the mesh's own named points to the systems that want them, so the
/// weapon table carries none of this geometry twice.
///
/// Only a weapon with glass marks the scope pair; one that sights down irons
/// leaves both unmarked and the whole scope view stays off for it.
pub(crate) fn wire_weapon_sockets(
    mut commands: Commands,
    child_of: Query<&ChildOf>,
    gun: Query<(), With<Gun>>,
    new_names: Query<(Entity, &Name), Added<Name>>,
) {
    for (entity, name) in &new_names {
        if !name.as_str().starts_with("Socket_") {
            continue;
        }
        let mut current = entity;
        let mut under_gun = false;
        for _ in 0..20 {
            if gun.contains(current) {
                under_gun = true;
                break;
            }
            let Ok(parent) = child_of.get(current) else { break };
            current = parent.0;
        }
        if !under_gun {
            continue;
        }
        let mut socket = commands.entity(entity);
        match name.as_str() {
            "Socket_Aim" => socket.insert(AimSocket),
            "Socket_SightFront" => socket.insert(FrontSightSocket),
            "Socket_Eject" => socket.insert(EjectSocket),
            "Socket_Muzzle" => socket.insert(MuzzleSocket),
            "Socket_RearGrip" => socket.insert(TriggerGripSocket),
            "Socket_FrontGrip" => socket.insert(ForeGripSocket),
            _ => continue,
        };
        debug!("[socket] {name} wired");
    }
}



pub(crate) struct GunPoseTimings {
    recoil_t: f32,
    dip: f32,
    bob_x: f32,
    bob_y: f32,
    bob_roll: f32,
    bob_pitch: f32,
}

pub(crate) fn gun_pose_timings(gun: &Gun, def: &WeaponDef, cycle: &WalkCycle) -> GunPoseTimings {
    let recoil_t = gun.recoil;
    let reload_t = gun.reload_remaining / def.reload_duration;

    // A raised cosine rather than the parabola 4t(1-t) it replaces. Both peak
    // at 1 halfway through and both end at 0, but the parabola arrives and
    // leaves at full speed, so the weapon snapped into the dip and snapped
    // back out of it. This one is flat at both ends: it eases in and eases
    // out, which is the curve the sights and the kick already use.
    let dip = ops::sin(std::f32::consts::PI * reload_t).powi(2);
    let amount = cycle.amount();
    let bob_x = ops::sin(cycle.phase) * GUN_BOB_AMOUNT_X * amount;
    // abs() already doubles the rate: |sin| repeats every pi, not every tau.
    // One dip per footfall, against a full sway every two.
    let bob_y = ops::sin(cycle.phase).abs() * GUN_BOB_AMOUNT_Y * amount;

    // The weapon trails the body. Arms are compliant, and a weapon that cants
    // in exact step with the hips reads as welded to them -- the lag is most
    // of what makes it look carried rather than attached.
    let lagged = cycle.phase - GUN_BOB_LAG;
    let bob_roll = ops::sin(lagged) * GUN_BOB_ROLL_DEG * amount;
    // Pitch turns about the mesh origin, which sits on the bore at the muzzle,
    // so this pins the nose and swings the stock -- the muzzle stays the
    // steadiest thing on screen, which is what a carried weapon does.
    let bob_pitch = ops::sin(lagged).abs() * GUN_BOB_PITCH_DEG * amount;
    GunPoseTimings { recoil_t, dip, bob_x, bob_y, bob_roll, bob_pitch }
}


/// Turns the mark to point down the weapon's own sight axis.
///
/// It hangs off the scope camera, so it is always in front of the lens and
/// facing it; all the weapon contributes is the direction. That makes it a
/// perfectly parallax-free sight: the mark shows where the barrel points,
/// which is where the bullet goes, however the weapon sits against the eye.
/// The socket says which way that is, so no weapon carries its own copy.
/// The holographic dot. Marks the main view, not the scope's.
#[derive(Component)]
pub(crate) struct HoloReticle;

/// Aims the dot down the bore and decides whether the eye can see it.
///
/// Both halves fall out of the one fact that makes a holographic sight a
/// holographic sight: its dot sits at infinity, so it is a *direction*, not a
/// place on the glass. Aiming it is the same unit throw down the barrel the
/// scope mark already uses, and world-locked comes free -- the dot tracks
/// where the barrel points while the weapon sways under it, which is the
/// whole behaviour being asked for.
///
/// Seeing it is simpler still. The ray from the eye to a point at infinity
/// down the bore runs parallel to the bore, which in the weapon's own frame
/// is straight down -Z -- so it meets every plane of the window at the eye's
/// own x and y. The dot is visible exactly while the eye is inside the tube,
/// which is one rectangle test and no GPU work at all. No stencil mask, no
/// second camera, no extra pass to pay for on a phone that is already warm.
///
/// It is also what the real thing does: a holographic sight has no parallax,
/// so moving your head does not walk the dot off the target -- it takes the
/// dot away entirely.
pub(crate) fn place_holo_reticle(
    active: Res<ActiveWeapon>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
    aim: Query<&GlobalTransform, With<AimSocket>>,
    gun: Query<(&Transform, &GlobalTransform), (With<Gun>, Without<HoloReticle>)>,
    pose: Res<WeaponPose>,
    mut reticle: Query<(&mut Transform, &mut Visibility), With<HoloReticle>>,
) {
    let Ok((mut dot, mut visibility)) = reticle.single_mut() else { return };
    let (Ok(eye), Ok((gun_local, gun_global))) = (camera.single(), gun.single()) else {
        *visibility = Visibility::Hidden;
        return;
    };
    let def = active.def();

    // A scope's mark answers to the sights being up, not to a window. There
    // is no window: past four times the glass is not what the player is
    // looking through, the screen is.
    let visible = match def.holo.as_ref() {
        Some(window) => {
            // Where the eye sits in the weapon's own frame. The bore runs down
            // -Z there, so this is also where the sight line crosses every
            // plane of the window, and the whole test is whether it lands
            // inside the opening.
            let in_weapon = gun_global.affine().inverse().transform_point3(eye.translation());
            in_weapon.x >= window.min.x
                && in_weapon.x <= window.max.x
                && in_weapon.y >= window.min.y
                && in_weapon.y <= window.max.y
        }
        None => def.scoped && pose.sighted > SCOPED_RETICLE_SIGHTED,
    };
    *visibility = if visible { Visibility::Visible } else { Visibility::Hidden };
    if !visible {
        return;
    }

    // The same throw the scope mark takes, and for the same reason: only the
    // direction survives, since the quad hangs off the eye.
    let down_the_barrel = match aim.single() {
        Ok(aim) => gun_local.rotation * aim.reparented_to(gun_global).rotation * Vec3::NEG_Z,
        Err(_) => gun_local.rotation * Vec3::NEG_Z,
    };
    dot.translation = down_the_barrel;
    dot.rotation = Quat::IDENTITY;
}




/// Where the look sway turns the weapon about, ahead of the weapon's own
/// origin and down the bore.
///
/// It does not change where the weapon points -- a rigid body turned by an
/// angle about any point turns its bore by that angle, so the reticle sweeps
/// the same wherever this sits. What it picks is which part of the weapon
/// holds still on screen while the rest swings.
const SWAY_PIVOT_DOWN_BORE: f32 = 0.40;

/// The look-over, as a handful of poses with times against them.
///
/// Offset is camera space off the rest pose; the angles are the weapon's own,
/// turned about the trigger grip. Roll does most of the work -- the point is
/// to bring the receiver round where it can be seen, first one flank and then
/// the other.
///
/// The weapon carries no clips of its own. It is a static mesh posed by code,
/// with the hands following it through the grip IK, so this table is the
/// animation.
/// A hold repeats the key before it exactly. Carrying a little drift through
/// one looks like the right idea and is not: each segment eases in and out, so
/// a few degrees spread over most of a second become a slow creep that starts
/// and stops, and the eye reads it as the animation labouring rather than the
/// weapon being looked at. A hold that holds is still.
const INSPECT_KEYS: [(f32, Vec3, Vec3); 6] = [
    //  t     offset (x, y, z)                 yaw, pitch, roll (degrees)
    (0.00, Vec3::new(0.00, 0.00, 0.00), Vec3::new(0.0, 0.0, 0.0)),
    (0.17, Vec3::new(-0.18, 0.03, 0.00), Vec3::new(56.0, 6.0, -30.0)),
    (0.42, Vec3::new(-0.18, 0.03, 0.00), Vec3::new(56.0, 6.0, -30.0)),
    (0.68, Vec3::new(-0.07, 0.01, 0.06), Vec3::new(-7.0, -5.0, 62.0)),
    (0.86, Vec3::new(-0.07, 0.01, 0.06), Vec3::new(-7.0, -5.0, 62.0)),
    (1.00, Vec3::new(0.00, 0.00, 0.00), Vec3::new(0.0, 0.0, 0.0)),
];

/// Reads that table at `t`, smoothed between the two keys it falls between so
/// each turn arrives and leaves rather than starting and stopping.
fn inspect_pose(t: f32) -> (Vec3, Vec3) {
    let t = t.clamp(0.0, 1.0);
    let last = INSPECT_KEYS.len() - 1;
    let mut from = &INSPECT_KEYS[0];
    let mut to = &INSPECT_KEYS[last];
    for pair in INSPECT_KEYS.windows(2) {
        if t <= pair[1].0 {
            from = &pair[0];
            to = &pair[1];
            break;
        }
    }
    let span = (to.0 - from.0).max(1e-6);
    let k = ((t - from.0) / span).clamp(0.0, 1.0);
    let eased = k * k * (3.0 - 2.0 * k);
    (from.1.lerp(to.1, eased), from.2.lerp(to.2, eased))
}

/// The two points a pair of hands holds the weapon at, as one parameter.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct GripSockets<'w, 's> {
    trigger: Query<'w, 's, &'static GlobalTransform, With<TriggerGripSocket>>,
    fore: Query<'w, 's, &'static GlobalTransform, With<ForeGripSocket>>,
}

/// Two resources as one parameter: gun_view_animation is at Bevy's sixteen.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct LocalPlayerState<'w> {
    moving: Res<'w, LocalPlayerMoving>,
    sprinting: Res<'w, LocalPlayerSprinting>,
    move_axis: Res<'w, LocalPlayerMoveAxis>,
}

/// The last wall reading and how long it has left to live. Kept between
/// frames so the ray can run at its own rate rather than the display's.
#[derive(Default)]
pub(crate) struct WallProbe {
    /// The wall ahead, which raises the muzzle over it.
    target: f32,
    /// The nearer of the two walls beside the player, which does not -- there
    /// is nothing out there to raise the weapon over, only nowhere to hold it.
    side: f32,
    until_next: f32,
}

pub(crate) fn gun_view_animation(
    grips: GripSockets,
    time: Res<Time>,
    aim_held: Res<AimHeld>,
    touch_aiming: Res<TouchAiming>,
    game_over: Res<GameOver>,
    keys: Res<ButtonInput<KeyCode>>,
    mut look_delta: ResMut<MouseLookDelta>,
    mut pose: ResMut<WeaponPose>,
    cursor: Query<&CursorOptions, With<PrimaryWindow>>,
    camera: Query<(&Transform, &Lean), (With<FpsCamera>, Without<Gun>)>,
    spatial_query: SpatialQuery,
    mut query: Query<(&mut Transform, &GlobalTransform, &mut Gun), Without<FpsCamera>>,
    aim: Query<&GlobalTransform, (With<AimSocket>, Without<Gun>, Without<FpsCamera>)>,
    local: LocalPlayerState,
    (active, cycle, shock, mut wall_probe): (
        Res<ActiveWeapon>,
        Res<WalkCycle>,
        Res<crate::movement_combat::FireShock>,
        Local<WallProbe>,
    ),
) {
    // Taken, not read: the single consumer clearing it is what lets each
    // input path write only when it has a delta, instead of every path having
    // to zero it and racing the others to do so.
    let look_delta = std::mem::take(&mut look_delta.0);
    let camera_single = camera.single().ok();
    let aiming = cursor.single().is_ok_and(|cursor| is_aiming(cursor, &aim_held, touch_aiming.0));

    // Reads the requested lean, not the eased camera value: keying off the
    // eased one makes the gun wait for the camera to settle before it starts
    // moving, instead of travelling alongside it.
    let leaning = camera_single
        .is_some_and(|(_, lean)| lean.target.abs() > GUN_READY_LEAN_CUTOFF);

    // Keyed off the requested lean, not the eased camera value. The swap only
    // uses the negative half of the range, so coming out of a right lean it
    // stays pinned at zero until the camera crosses the middle and then picks
    // up on the slowest part of the camera ease. That is the gun lagging the
    // press. Reading the input instead starts both at the same instant.
    let shoulder_target = camera_single.map_or(0.0, |(_, lean)| (-lean.target).clamp(0.0, 1.0));

    // Sampled rather than cast every frame. This is one ray into Avian's
    // whole collision tree, and it was running at the refresh rate to answer
    // a question about walking pace -- nobody closes the last metre in 8ms.
    // The smoothing below hides the steps: at WALL_GUN_RAISE_SPEED the chase
    // has a 100ms time constant against an 83ms sample.
    wall_probe.until_next -= time.delta_secs();
    if wall_probe.until_next <= 0.0 {
        wall_probe.until_next = WALL_PROBE_INTERVAL;
        let filter = SpatialQueryFilter::default();
        let reading = |origin: Vec3, dir: Vec3, reach: f32| {
            Dir3::new(dir)
                .ok()
                .and_then(|dir| spatial_query.cast_ray(origin, dir, reach, true, &filter))
                .map_or(0.0, |hit| 1.0 - (hit.distance / reach).clamp(0.0, 1.0))
        };
        let (target, side) = camera_single.map_or((0.0, 0.0), |(camera_transform, _)| {
            let eye = camera_transform.translation;
            // Flattened: a lean rolls the camera, and a side probe that rolls
            // with it stops measuring the wall and starts measuring the floor.
            let right = camera_transform.right().as_vec3();
            let flat_right = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();
            let side = WALL_SIDE_ANGLES_DEG
                .iter()
                .flat_map(|deg| [flat_right, -flat_right].map(|basis| (*deg, basis)))
                .map(|(deg, basis)| Quat::from_rotation_y(deg.to_radians()) * basis)
                .map(|dir| reading(eye, dir, WALL_SIDE_DISTANCE))
                .fold(0.0f32, f32::max);
            (
                reading(eye, camera_transform.forward().as_vec3(), WALL_GUN_RAISE_DISTANCE),
                side,
            )
        });
        wall_probe.target = target;
        wall_probe.side = side;
    }
    let wall_target = wall_probe.target;
    // A wall close ahead takes the weapon over completely. Letting both rules
    // speak would have the muzzle asked up and in at the same time, and the
    // one the player is about to walk into is the one worth answering.
    let wall_side = if wall_target >= WALL_AHEAD_OWNS { 0.0 } else { wall_probe.side };

    // Which way there is room to move the weapon. Up by default, because that
    // is where the room usually is. Looking down there is floor in front of
    // the muzzle and open air behind the stock, so it drops away from the
    // wall rather than being hoisted into it.
    //
    // Read every frame rather than with the probe: this is the player's own
    // look, which costs nothing to ask and would lag visibly at twelve
    // samples a second.
    let raise_sign = camera_single.map_or(1.0, |(camera_transform, _)| {
        let pitch_deg = camera_transform.forward().y.clamp(-1.0, 1.0).asin().to_degrees();
        let span = WALL_RAISE_DOWN_FULL_DEG - WALL_RAISE_DOWN_START_DEG;
        let down = ((-pitch_deg - WALL_RAISE_DOWN_START_DEG) / span).clamp(0.0, 1.0);
        1.0 - 2.0 * down
    });
    for (mut transform, gun_global, mut gun) in &mut query {
        // The mark eases home while the kick runs after it. Two rates, so the
        // blow lands fast and the weapon comes back slowly, which is the
        // shape of the thing rather than one curve doing both.
        let kick = &active.def().recoil;
        gun.recoil_target *= ops::exp(-kick.weapon.settle_speed * time.delta_secs());
        let strike = 1.0 - ops::exp(-kick.strike_speed * time.delta_secs());
        gun.recoil += (gun.recoil_target - gun.recoil) * strike;
        if gun.reload_remaining > 0.0 {
            gun.reload_remaining = (gun.reload_remaining - time.delta_secs()).max(0.0);
            if gun.reload_remaining == 0.0 {
                gun.ammo = active.def().magazine;
            }
        }

        // The sights come down to reload. The weapon tilts 22 degrees and rolls
        // 20 to clear the magwell, which is not a thing to be looking through.
        let ads_target =
            if (aiming || force_ads()) && gun.reload_remaining <= 0.0 { 1.0 } else { 0.0 };
        // Latched on the rising edge of the request, not on ads_blend nearing
        // zero -- it decays towards zero and never arrives. ready_blend is
        // last frame's, before aiming drives it down. The max covers a raise
        // that starts before the last one finished coming down.
        if ads_target > 0.0 && !gun.ads_wanted {
            gun.ads_from_ready = gun.ready_blend.max(gun.ads_blend);
        }
        gun.ads_wanted = ads_target > 0.0;
        // Dropping the sights is the ready speed either way: what the rest
        // pose costs is the distance up, not the distance back down.
        let speed = &active.def().ads_speed;
        let ads_rate = if ads_target > 0.0 {
            speed.from_rest + (speed.from_ready - speed.from_rest) * gun.ads_from_ready
        } else {
            speed.from_ready
        };
        // A damped spring, not an exponential ease. The ease could only ever
        // arrive from one side: the weapon slid up to the sight line and
        // stopped dead on it, which is the one thing arms carrying three kilos
        // of rifle never do. This overshoots and comes back, twice, smaller
        // each time.
        //
        // Implicit Euler rather than the obvious `v += a*dt; x += v*dt`. That
        // one gains energy when `rate * dt` gets large and a weapon raised
        // during a frame spike would oscillate wider instead of settling.
        // This form is unconditionally stable at any timestep.
        let dt = time.delta_secs();
        let omega = ads_rate;
        let f = 1.0 + 2.0 * dt * ADS_SPRING_DAMPING * omega;
        let oo = omega * omega;
        let hoo = dt * oo;
        let hhoo = dt * hoo;
        let det_inv = 1.0 / (f + hhoo);
        let det_x = f * gun.ads_blend + dt * gun.ads_velocity + hhoo * ads_target;
        let det_v = gun.ads_velocity + hoo * (ads_target - gun.ads_blend);
        gun.ads_blend = det_x * det_inv;
        gun.ads_velocity = det_v * det_inv;
        // How fast the sights are moving, as a fraction of how fast this
        // weapon can move them. Oscillates through zero and decays, which is
        // what the miss below is after.
        //
        // Not `ads_blend - ads_target`, which was the first attempt: the
        // target is a step, so the instant the player let go of the sights it
        // jumped from nearly nothing to a whole unit and threw the weapon
        // seven degrees in one frame. Velocity is integrated state and cannot
        // jump, however abruptly the player changes their mind.
        gun.ads_settle_error = gun.ads_velocity / ads_rate.max(0.001);

        // Having aimed once means the player is working a sight line and will
        // likely aim again, so the gun waits near the middle instead of
        // swinging all the way back to the hip. Breaking into a sprint says
        // they are done with that sight line, so it drops to the normal rest.
        if aiming {
            gun.ready_armed = true;
        } else if local.sprinting.0 || game_over.0 {
            gun.ready_armed = false;
        }
        // Leaning takes the gun back to its shoulder: the swap is a deliberate
        // move to the other side of cover, and a centred ready pose has nothing
        // to swap. Releasing the lean settles back to ready.
        // A wall *beside* the player, not the one in front. The one in front
        // has something to raise the muzzle over and keeps doing it; a wall
        // at the shoulder has nowhere to hold the weapon out, so it comes in
        // to the ready.
        //
        // No timer: `ready_blend` below chases this at GUN_READY_SPEED, so
        // the weapon still swings in over a few frames rather than snapping,
        // and a held-down counter would only delay a move that is already
        // eased. What it does need is somewhere to leave from that is not
        // where it arrived, or standing on the boundary chatters.
        gun.wall_near = if gun.wall_near {
            wall_side >= WALL_SIDE_PROXIMITY - WALL_SIDE_HYSTERESIS
        } else {
            wall_side >= WALL_SIDE_PROXIMITY
        };
        // Not while sprinting: the run already has the weapon carried across
        // the body, and a wall it happens to pass would otherwise pull it back
        // to the ready and fight that pose the length of the alley.
        let wall_ready = gun.wall_near && !local.sprinting.0;

        let ready_target =
            if (gun.ready_armed || wall_ready) && !aiming && !leaning { 1.0 } else { 0.0 };
        let ready_step = (GUN_READY_SPEED * time.delta_secs()).min(1.0);
        gun.ready_blend += (ready_target - gun.ready_blend) * ready_step;

        // Carried across the body, but only once the sprint has gone a second
        // without a shot. Aiming, reloading or standing still all mean the
        // weapon is wanted, so none of them reach it.
        gun.since_shot = (gun.since_shot + dt).min(f32::MAX / 2.0);
        let carrying = local.sprinting.0
            && local.moving.0
            && !aiming
            && gun.reload_remaining <= 0.0
            && gun.since_shot >= SPRINT_CARRY_DELAY;
        let carry_step = (SPRINT_CARRY_SPEED * dt).min(1.0);
        gun.sprint_blend += (f32::from(carrying) - gun.sprint_blend) * carry_step;

        // Nothing asked of the weapon for long enough and it gets looked over.
        // Walking still counts as idle -- standing perfectly still is rare in
        // a game where the dead come to you -- but running does not.
        let busy = aiming
            || local.sprinting.0
            || gun.reload_remaining > 0.0
            || gun.since_shot < SPRINT_CARRY_DELAY;
        if busy {
            gun.idle_elapsed = 0.0;
        } else {
            gun.idle_elapsed += dt;
        }
        // T asks for it directly, so nobody has to stand around to see it.
        // Still refused while busy: the weapon is in the middle of something.
        let asked = keys.just_pressed(INSPECT_KEY);
        let wait = INSPECT_WAIT + gun.inspect_done as f32 * INSPECT_WAIT_EXTRA;
        if gun.inspect_t <= 0.0 && !busy && (asked || gun.idle_elapsed >= wait) {
            gun.inspect_t = f32::EPSILON;
        }
        if gun.inspect_t > 0.0 {
            gun.inspect_t += dt / INSPECT_SECONDS;
            // Held at the end rather than wound back to the start.
            //
            // A frame lands wherever dt puts it, so the last one before the
            // end sits at something like 0.97 -- a twentieth of the way back
            // from the final look, still turned. Winding t to zero there read
            // as the pose jumping the rest of the way home in one frame, and
            // that was the jolt. Clamped, the sequence always finishes on its
            // own last key.
            if gun.inspect_t >= 1.0 {
                gun.inspect_t = 1.0;
                gun.inspect_done += 1;
                gun.idle_elapsed = 0.0;
            }
            // Cancelled is different: the player wants to shoot, so it stops
            // where it is and the blend takes it away.
            if busy {
                gun.inspect_t = 0.0;
                gun.idle_elapsed = 0.0;
            }
        }
        let inspecting = gun.inspect_t > 0.0 && gun.inspect_t < 1.0;
        if !inspecting && gun.inspect_blend < 0.01 {
            gun.inspect_t = 0.0;
        }
        // Three rates, because the three ways out are not the same thing. Going
        // in is a decision and takes its time; finishing is the weapon coming
        // home; being cancelled is the player wanting to shoot, and that one
        // gets out of the way.
        let inspect_rate = if inspecting {
            INSPECT_IN_SPEED
        } else if busy {
            INSPECT_OUT_SPEED
        } else {
            INSPECT_RETURN_SPEED
        };
        gun.inspect_blend +=
            (f32::from(inspecting) - gun.inspect_blend) * (inspect_rate * dt).min(1.0);
        let (want_offset, want_angles) = inspect_pose(gun.inspect_t);
        // Implicit Euler, as everywhere else here: stable at any timestep,
        // where `v += a*dt; x += v*dt` gains energy on a frame spike and a
        // weapon caught by one would swing wider instead of settling.
        let omega = INSPECT_SPRING_HZ;
        let f = 1.0 + 2.0 * dt * INSPECT_SPRING_DAMPING * omega;
        let hoo = dt * omega * omega;
        let hhoo = dt * hoo;
        let det_inv = 1.0 / (f + hhoo);
        let (x, v) = (gun.inspect_offset, gun.inspect_offset_vel);
        gun.inspect_offset = (x * f + v * dt + want_offset * hhoo) * det_inv;
        gun.inspect_offset_vel = (v + (want_offset - x) * hoo) * det_inv;
        let (x, v) = (gun.inspect_angles, gun.inspect_angles_vel);
        gun.inspect_angles = (x * f + v * dt + want_angles * hhoo) * det_inv;
        gun.inspect_angles_vel = (v + (want_angles - x) * hoo) * det_inv;
        let (inspect_offset, inspect_angles) = (gun.inspect_offset, gun.inspect_angles);

        // One roll per disturbance, so each return lands somewhere new.
        let disturbed = aiming || leaning || local.sprinting.0;
        if disturbed {
            if !gun.settle_rolled {
                gun.settle_rolled = true;
                let (offset, twist) = settle_jitter(time.elapsed_secs());
                gun.settle_jitter = offset;
                gun.settle_twist_jitter = twist;
            }
        } else {
            gun.settle_rolled = false;
        }

        // Chased rather than used raw. The offset lands straight on the final
        // translation with nothing else smoothing it, so swapping the value in
        // one frame moves the gun in one frame.
        let settle_step = (GUN_SETTLE_SPEED * time.delta_secs()).min(1.0);
        let settle_gap = gun.settle_jitter - gun.settle_current;
        gun.settle_current += settle_gap * settle_step;
        let twist_gap = gun.settle_twist_jitter - gun.settle_twist_current;
        gun.settle_twist_current += twist_gap * settle_step;

        // Signed, so one chased value carries both how far the weapon is moved
        // and which way, and the flip between them is eased by the same rate
        // the raise itself is. Everything reading this for "is the weapon out
        // of position" wants its size and not its direction, and says abs().
        let wall_raise_step = (WALL_GUN_RAISE_SPEED * time.delta_secs()).min(1.0);
        gun.wall_raise += (wall_target * raise_sign - gun.wall_raise) * wall_raise_step;

        let shoulder_step = (GUN_SHOULDER_SPEED * time.delta_secs()).min(1.0);
        gun.shoulder_blend += (shoulder_target - gun.shoulder_blend) * shoulder_step;


        // Late to a turn, not rolled by one. Negated because the weapon is
        // still aimed back where the view was, opposite the way it just moved.
        let lag = -look_delta;
        let sway_target = (lag * LOOK_SWAY_SENSITIVITY)
            .clamp(Vec2::splat(-LOOK_SWAY_MAX_DEG), Vec2::splat(LOOK_SWAY_MAX_DEG));
        // The same damped spring the sights ride: an exponential ease can only
        // arrive from one side, so the weapon stopped dead whenever a turn
        // ended. The target here is how fast the view is turning right now,
        // not a step, so it falls to zero the instant the turn stops and the
        // spring is what carries the weapon on past it.
        let omega = LOOK_SWAY_FREQUENCY;
        let f = 1.0 + 2.0 * dt * LOOK_SWAY_DAMPING * omega;
        let hoo = dt * omega * omega;
        let hhoo = dt * hoo;
        let det_inv = 1.0 / (f + hhoo);
        let (x, v) = (gun.look_sway, gun.look_sway_velocity);
        gun.look_sway = (x * f + v * dt + sway_target * hhoo) * det_inv;
        gun.look_sway_velocity = (v + (sway_target - x) * hoo) * det_inv;

        // Cut rather than cancelled in the sights. Most of it has to go --
        // the pitch is read along the barrel, so sighted it moves the point
        // of aim and not just the picture -- but a weapon that goes
        // perfectly rigid the moment it reaches the eye reads as a prop on a
        // rail. Scaled through ads_blend, so bringing the sights up settles
        // the weapon on the way instead of snapping it level.
        let aim_damp = 1.0 - gun.ads_blend * (1.0 - AIM_STEP_LEAN_MULTIPLIER);
        let lean_target = local.move_axis.0.clamp_length_max(1.0) * aim_damp;
        let lean = gun.step_lean;
        gun.step_lean = lean + (lean_target - lean) * (GUN_STEP_LEAN_SPEED * dt).min(1.0);

        let kick = &active.def().recoil;
        let timings = gun_pose_timings(&gun, active.def(), &cycle);

        // Faded out by ads_blend, the way the positional jitter already is
        // through resting: a weapon canted on its own bore would take the
        // sights off the camera axis. The optical correction fades the other
        // way, since it is only wanted while looking through the glass.
        let settle_twist = gun.settle_twist_current * (1.0 - gun.ads_blend);
        // Muzzle up and rolled onto its side, so the magazine well comes round
        // to face the hand that has to empty it.
        //
        // Turned about the firing grip rather than about the mesh origin. That
        // origin is on the bore at the muzzle, so turning about it pins the
        // muzzle and swings the stock down instead -- the weapon drops its
        // tail rather than lifting its nose, which is the wrong half moving.
        // Rotating about a point means turning, then putting back what the
        // turn moved that point by.
        let reload_turn = Quat::from_rotation_x(RELOAD_TILT_DEG.to_radians() * timings.dip)
            * Quat::from_rotation_z(RELOAD_ROLL_DEG.to_radians() * timings.dip);
        let grip_in_gun = grips
            .trigger
            .single()
            .map(|socket| socket.reparented_to(gun_global).translation)
            .unwrap_or(Vec3::ZERO);
        let carry = &active.def().sprint_carry;
        let sprint_turn = Quat::IDENTITY.slerp(
            Quat::from_rotation_y(carry.yaw_deg.to_radians())
                * Quat::from_rotation_x(carry.pitch_deg.to_radians())
                * Quat::from_rotation_z(carry.roll_deg.to_radians()),
            gun.sprint_blend,
        );
        // One turn for both poses, so the correction below is exact rather
        // than two approximations added together.
        let inspect_turn = Quat::IDENTITY.slerp(
            Quat::from_rotation_y(inspect_angles.x.to_radians())
                * Quat::from_rotation_x(inspect_angles.y.to_radians())
                * Quat::from_rotation_z(inspect_angles.z.to_radians()),
            gun.inspect_blend,
        );
        let pose_turn = reload_turn * sprint_turn;
        let pose_swing = grip_in_gun - pose_turn * grip_in_gun;

        let base_rotation = Quat::from_rotation_z(settle_twist.to_radians())
            * pose_turn
            * Quat::IDENTITY.slerp(gun.optical_fix, gun.ads_blend);

        // Back to the rest height first. The ready pose deliberately sits a
        // hand lower -- muzzle down, waiting -- and the look-over was being
        // added on top of wherever the weapon happened to be, so starting it
        // from ready put the whole gesture ten centimetres under where
        // starting it from rest did. It is the same gesture either way.
        let posture = gun
            .base_offset
            .lerp(active.def().ready_offset, gun.ready_blend)
            .lerp(gun.base_offset, gun.inspect_blend);
        let resting = posture
            + gun.settle_current
            + active.def().sprint_carry.offset * gun.sprint_blend
            + inspect_offset * gun.inspect_blend;
        // Sighted means Socket_Aim on the camera axis at this weapon.s eye
        // relief; the mesh says where that point is, so the offset is a
        // subtraction rather than a dialled-in vector. Where it sits inside
        // the gun never animates, so last frame.s copy of that is fine.
        //
        // Turned before it is subtracted. The socket sits 0.91m down the
        // weapon, so a straight subtraction only lands it on the axis while
        // the weapon is square to the camera -- the moment the pose turns it,
        // that lever throws the sight off by the turn times its length. Only
        // base_rotation is taken out: sway and recoil are meant to show.
        let sighted = aim
            .single()
            .map(|aim_global| {
                let in_gun = aim_global.reparented_to(gun_global);
                Vec3::new(0.0, 0.0, -active.def().ads_eye_relief) - base_rotation * in_gun.translation
            })
            .unwrap_or(resting);
        // Slides off the sight line and corrects on, without turning.
        //
        // This was a rotation, which cannot work here: the reticle takes its
        // position from the bore direction, and rotating a rigid weapon about
        // any pivot turns the bore by the full angle. Moving the pivot forward
        // lengthens the stock's lever but does nothing for the bore, so the
        // sight picture swung by the whole miss -- and a scope multiplies it,
        // 1.4 degrees filling a third of a 12x view.
        //
        // A translation leaves the bore pointing where it was. The weapon
        // visibly overshoots and settles; the reticle does not move at all,
        // because nothing it reads has changed.
        //
        // Across the approach, not along it: along was the first attempt and
        // read as one straight line with a wobble on the end, because the
        // weapon was never anywhere it had not been going anyway.
        let travel = sighted - resting;
        let across = Dir3::new(travel.cross(Vec3::NEG_Z)).ok();
        let settle_offset = across
            .map(|across| *across * (active.def().ads_settle_miss_m * gun.ads_settle_error))
            .unwrap_or(Vec3::ZERO);
        // And a tilt to go with the slide. A weapon whose bore stayed exactly
        // true while the rest of it settled read as fake, because the reticle
        // takes its place from the bore and a reticle that never moves is a
        // weapon that was never really moving.
        //
        // Kept small, and small by magnification rather than by mass: the
        // reticle sweeps by the whole tilt and a scope multiplies it. The
        // weapon with the most glass needs the least angle, which is the
        // reverse of what the slide above wants.
        let settle_tilt = across
            .map(|axis| {
                Quat::from_axis_angle(
                    *axis,
                    (active.def().ads_settle_miss_deg * gun.ads_settle_error).to_radians(),
                )
            })
            .unwrap_or(Quat::IDENTITY);

        let base_pos =
            resting.lerp(sighted, gun.ads_blend) - Vec3::Z * view_pullback() + settle_offset;

        // Breathing and two tremors, read off the clock rather than integrated,
        // so it is the same motion whatever the frame rate and two weapons
        // picked up a second apart are not in step.
        //
        // Pitch carries the breath because that is the axis a chest moves the
        // muzzle on. The tremors take yaw as well, and sit at rates with no
        // common multiple so the loop never lands where the eye can catch it.
        let clock = time.elapsed_secs();
        let tau = std::f32::consts::TAU;
        let breath = ops::sin(tau * HOLD_BREATH_HZ * clock);
        let tremor_a = ops::sin(tau * HOLD_TREMOR_A_HZ * clock);
        let tremor_b = ops::sin(tau * HOLD_TREMOR_B_HZ * clock + 1.3);
        // Braced against the shoulder, most of it goes; what is left the glass
        // makes the most of.
        // Each weapon says how much of this its sights show, because a red dot
        // and a 24x scope do not magnify the same wander by the same amount
        // and are not braced the same way either.
        let ads = gun.ads_blend.clamp(0.0, 1.0);
        let hold = 1.0 - ads * (1.0 - active.def().hold_ads_fraction);
        let hold_yaw = (HOLD_BREATH_YAW_DEG * breath + HOLD_TREMOR_YAW_DEG * tremor_a) * hold;
        let hold_pitch = (HOLD_BREATH_PITCH_DEG * breath + HOLD_TREMOR_PITCH_DEG * tremor_b) * hold;
        let hold_turn = Quat::from_rotation_y(hold_yaw.to_radians())
            * Quat::from_rotation_x(hold_pitch.to_radians());
        // Handed to the view, which takes a share of the same numbers rather
        // than running its own sines and slowly drifting out of step.
        pose.hold_yaw_deg = hold_yaw;
        pose.hold_pitch_deg = hold_pitch;
        pose.sighted = ads;

        let outer = Quat::from_rotation_x(WALL_GUN_RAISE_ROT_DEG.to_radians() * gun.wall_raise);
        let sway = Quat::from_rotation_y(gun.look_sway.x.to_radians())
            * Quat::from_rotation_x(gun.look_sway.y.to_radians());
        let inner = settle_tilt
            // About Z, which is the bore: the weapon points down NEG_Z, so
            // this is a roll along the barrel rather than a tilt across it.
            // Negated so the cant goes with the step -- strafing right drops
            // the weapon's right side, the way a shoulder does.
            * Quat::from_rotation_z(-GUN_STRAFE_ROLL_DEG.to_radians() * gun.step_lean.x)
            // About X. A positive turn here takes NEG_Z towards +Y, which is
            // the muzzle coming up, so forward has to be negative to drop it.
            * Quat::from_rotation_x(-GUN_STEP_PITCH_DEG.to_radians() * gun.step_lean.y)
            * Quat::from_rotation_z(timings.bob_roll.to_radians())
            * Quat::from_rotation_x(timings.bob_pitch.to_radians())
            * Quat::from_rotation_y(kick.weapon.yaw_deg.to_radians() * gun.recoil_jitter.x * timings.recoil_t)
            * Quat::from_rotation_z(kick.weapon.roll_deg.to_radians() * gun.recoil_jitter.y * timings.recoil_t)
            * Quat::from_rotation_x(kick.weapon.pitch_deg.to_radians() * timings.recoil_t)
            * base_rotation;

        // Turning about a point means turning, then putting back what the turn
        // moved that point by -- the same arithmetic reload_swing uses. Taken
        // through the rotations either side, since they carry the pivot
        // before and after the sway reaches it.
        let turned = inner * (Vec3::NEG_Z * SWAY_PIVOT_DOWN_BORE);
        let sway_swing = outer * (turned - sway * turned);
        // About the firing grip, like the reload and the carry: a hand holding
        // a rifle is the thing that stays put while the muzzle wanders.
        let held = inner * grip_in_gun;
        let hold_swing = outer * sway * (held - hold_turn * held);

        // Between the hands, not at the trigger. Rolling about the firing grip
        // alone swings the muzzle through an arc of whatever the grip sits
        // below the bore, which reads as the weapon being thrown around rather
        // than turned over. Both hands turning it together pivot it near its
        // own middle, and that is what the gesture looks like.
        let fore_in_gun = grips
            .fore
            .single()
            .map(|socket| socket.reparented_to(gun_global).translation)
            .unwrap_or(grip_in_gun);
        let between_hands = inner * ((grip_in_gun + fore_in_gun) * 0.5);
        let inspect_swing =
            outer * sway * hold_turn * (between_hands - inspect_turn * between_hands);

        let (shock_pitch, shock_yaw, shock_roll) = shock.gun_angles();
        let swapped_x = base_pos.x * (1.0 - 2.0 * gun.shoulder_blend);
        transform.translation = Vec3::new(swapped_x, base_pos.y, base_pos.z)
            + pose_swing
            + sway_swing
            + hold_swing
            + inspect_swing
            + Vec3::new(
                timings.bob_x,
                kick.weapon.lift * timings.recoil_t + RELOAD_LIFT * timings.dip - timings.bob_y
                    + WALL_GUN_RAISE_HEIGHT * gun.wall_raise,
                kick.weapon.slide_back * timings.recoil_t,
            )
            // The same ring-down shifting the weapon as well as turning it:
            // a rifle in the shoulder judders along its own travel, not only
            // about a point. Reusing the angles keeps the two in phase.
            + Vec3::new(shock_yaw, shock_pitch, shock_roll)
                * shock.gun_shift();

        // Outermost, so the whole weapon rattles in view rather than each
        // part twisting about its own pivot. Several times what the eye gets:
        // this is the thing the round was fired out of.
        transform.rotation = Quat::from_euler(EulerRot::YXZ, shock_yaw, shock_pitch, shock_roll)
            * outer
            * sway
            * hold_turn
            * inspect_turn
            * inner;
    }
}


/// Where the gun settles, nudged a little each time. Coming to rest on the
/// exact same millimetre after every aim is what makes the pose read as a
/// still image rather than a held weapon.
fn settle_jitter(seed: f32) -> (Vec3, f32) {
    let axis = |salt: f32| (ops::sin(seed * salt) * 43758.547).fract().abs() * 2.0 - 1.0;
    (
        Vec3::new(
            axis(12.9898) * GUN_SETTLE_JITTER.x,
            axis(78.233) * GUN_SETTLE_JITTER.y,
            axis(37.719) * GUN_SETTLE_JITTER.z,
        ),
        axis(94.673) * GUN_SETTLE_TWIST_DEG,
    )
}

/// One walk cycle, shared by the weapon and the view.
///
/// Both have to ride the *same* cycle or they drift against each other and the
/// weapon stops following the step it is supposed to be following. `phase`
/// advances only while moving, `blend` fades the whole thing in and out so
/// stopping does not cut it off mid-stride, and `aim_damp` is how much of it
/// survives the sights coming up.
#[derive(Resource, Default)]
pub(crate) struct WalkCycle {
    pub(crate) phase: f32,
    pub(crate) blend: f32,
    pub(crate) aim_damp: f32,
    /// 1.0 at a walk, higher at a sprint.
    pub(crate) gait: f32,
}

impl WalkCycle {
    /// What the weapon and the view both scale their bob by.
    pub(crate) fn amount(&self) -> f32 {
        self.blend * self.aim_damp * self.gait
    }
}

pub(crate) fn advance_walk_cycle(
    time: Res<Time>,
    moving: Res<LocalPlayerMoving>,
    sprinting: Res<LocalPlayerSprinting>,
    gun: Query<&Gun>,
    player: Query<&Player, Without<NetPlayer>>,
    mut cycle: ResMut<WalkCycle>,
) {
    let dt = time.delta_secs();
    // How fast the player is actually going, not whether a key is down.
    //
    // Aiming halves the move speed, crouching halves it, prone quarters it,
    // and a touch stick can be pushed any fraction of the way -- and every
    // one of those used to swing the weapon as hard as a full-speed walk,
    // because this was a boolean. Sighted and walking was the worst of them:
    // half the ground covered for all of the sway.
    //
    // Clamped at one rather than left to run: a sprint is faster than
    // MOVE_SPEED, but its extra amplitude belongs to `gait`, which raises it
    // without touching the cadence.
    let speed = player.single().map_or(0.0, |player| player.horizontal_velocity.length());
    let blend_target = (speed / shared::MOVE_SPEED).clamp(0.0, 1.0);
    cycle.blend += (blend_target - cycle.blend) * (GUN_BOB_BLEND_SPEED * dt).min(1.0);

    // A sprint is a different gait, not a louder walk. Raising the amplitude
    // while leaving the cadence at walking pace is the thing that reads as
    // wrong -- the steps stop matching the ground going past.
    let gait_target = if sprinting.0 { SPRINT_BOB_GAIT } else { 1.0 };
    cycle.gait += (gait_target - cycle.gait) * (BOB_GAIT_BLEND_SPEED * dt).min(1.0);

    if moving.0 {
        // Wrapped rather than left to grow: at 8 rad/s an hour of walking puts
        // the phase near 29000, where an f32 step of 0.13 starts to quantise.
        cycle.phase = (cycle.phase + GUN_BOB_SPEED * cycle.gait * dt)
            .rem_euclid(std::f32::consts::TAU);
    }

    cycle.aim_damp = gun
        .single()
        .map(|gun| 1.0 - gun.ads_blend * (1.0 - AIM_BOB_MULTIPLIER))
        .unwrap_or(1.0);
}

