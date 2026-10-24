use avian3d::prelude::*;
use bevy::prelude::*;

use crate::{
    ClientInput, Player, PlayerStance, AIM_MOVE_SPEED_MULTIPLIER, CROUCH_LENGTH, CROUCH_SPEED_MULTIPLIER,
    GameLayer, GRAVITY, ground_normal_min_y, JUMP_SPEED, MOVE_SPEED, NOCLIP_SPEED, PLAYER_LENGTH, PLAYER_RADIUS,
    PRONE_LENGTH, PRONE_SPEED_MULTIPLIER, SPRINT_MULTIPLIER,
};

/// Under this a contact is sideways or behind and did not stand in the way.
const OPPOSING_CONTACT: f32 = 0.01;
const LANDING_PROBE_OVERSHOOT: f32 = 0.05;

/// The tallest thing that can be walked onto without jumping. Zombies read
/// the same constant.
pub const STEP_HEIGHT: f32 = 0.4;

/// `NO_STEP_UP=1` refuses every step, leaving ramps and stairs to the slide.
fn step_up_disabled() -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OFF.get_or_init(|| std::env::var("NO_STEP_UP").is_ok())
}

/// Where a step put the capsule, and what it landed on. Too steep to stand on
/// does not refuse the step; the caller reads the normal and decides.
#[derive(Clone, Copy)]
pub struct StepUp {
    pub position: Vec3,
    pub ground_normal: Vec3,
}

/// Quake's PM_StepSlideMove: when the slide met something, run the move again
/// from a step-height higher and keep whichever attempt got further.
pub fn try_step_up(
    mover: &MoveAndSlide,
    shape: &Collider,
    rotation: Quat,
    start: Vec3,
    slid_to: Vec3,
    horizontal_velocity: Vec3,
    config: &MoveAndSlideConfig,
    filter: &SpatialQueryFilter,
    dt: f32,
) -> Option<StepUp> {
    if step_up_disabled() {
        return None;
    }
    let speed = horizontal_velocity.length();
    if speed < 1e-4 {
        return None;
    }
    let direction = horizontal_velocity / speed;

    let headroom = mover
        .spatial_query
        .cast_shape(shape, start, rotation, Dir3::Y, &ShapeCastConfig::from_max_distance(STEP_HEIGHT), filter)
        .map_or(STEP_HEIGHT, |hit| hit.distance);

    let raised = start + Vec3::Y * headroom;
    let stepped = mover.move_and_slide(
        shape,
        raised,
        rotation,
        horizontal_velocity,
        // The same frame the slide got, or the raised attempt wins the
        // comparison on length alone.
        std::time::Duration::from_secs_f32(dt.max(0.0)),
        config,
        filter,
        |_| MoveAndSlideHitResponse::Accept,
    );

    let landing = mover.spatial_query.cast_shape(
        shape,
        stepped.position,
        rotation,
        Dir3::NEG_Y,
        &ShapeCastConfig::from_max_distance(headroom + LANDING_PROBE_OVERSHOOT),
        filter,
    )?;

    let position = stepped.position - Vec3::Y * landing.distance;
    if (position - start).dot(direction) <= (slid_to - start).dot(direction) {
        return None;
    }
    Some(StepUp { position, ground_normal: Vec3::from(landing.normal1) })
}

fn slope_tangent(normal: Vec3) -> Vec3 {
    (Vec3::Y - normal * normal.dot(Vec3::Y)).normalize_or_zero()
}

pub fn resolve_slope_hit(hit: &mut MoveAndSlideHitData, input_velocity: Vec3, grounded: bool) -> bool {
    let normal = hit.normal.as_vec3();
    let walkable = normal.y > ground_normal_min_y();
    let tangent = slope_tangent(normal);
    let intent = input_velocity.dot(tangent);
    let along_slope = hit.velocity.dot(tangent);

    let pushed_up_a_cliff = !walkable && along_slope > 0.0 && intent <= 0.0;
    let dragged_down_a_ramp = walkable && grounded && along_slope < 0.0 && intent >= 0.0;
    if pushed_up_a_cliff || dragged_down_a_ramp {
        *hit.velocity -= tangent * along_slope;
    }

    walkable
}

/// Reaches full speed in about a tenth of a second on the ground.
pub const GROUND_ACCEL: f32 = 10.0;
/// Air control, and deliberately a fraction of the ground figure: a jump
/// should commit you to roughly where you were going.
pub const AIR_ACCEL: f32 = 1.0;
pub const GROUND_FRICTION: f32 = 8.0;
/// The floor under the friction calculation: a fixed fraction per frame never
/// reaches zero, a constant drop below this speed does.
pub const STOP_SPEED: f32 = 1.2;

/// Quake's. `add = wishspeed - current` is what bounds it: the amount left to
/// add shrinks to nothing as the velocity approaches the limit, so pushing
/// along your own velocity can never overshoot. Turning falls out of friction
/// and acceleration without a turn rate existing anywhere.
pub fn accelerate(velocity: Vec3, wish_dir: Vec3, wish_speed: f32, accel: f32, dt: f32) -> Vec3 {
    let current = velocity.dot(wish_dir);
    let add = wish_speed - current;
    if add <= 0.0 {
        return velocity;
    }
    velocity + wish_dir * (accel * wish_speed * dt).min(add)
}

pub fn apply_friction(velocity: Vec3, friction: f32, dt: f32) -> Vec3 {
    let speed = velocity.length();
    if speed < 1.0e-4 {
        return Vec3::ZERO;
    }
    let drop = speed.max(STOP_SPEED) * friction * dt;
    velocity * (speed - drop).max(0.0) / speed
}

pub fn simulate_player_step(mover: &MoveAndSlide, transform: &mut Transform, player: &mut Player, stance: PlayerStance, input: &ClientInput, dt: f32) {
    if input.noclip {
        simulate_noclip_step(transform, input, dt);
        player.vertical_velocity = 0.0;
        player.grounded = true;
        return;
    }

    let move_speed = if input.aiming {
        MOVE_SPEED * AIM_MOVE_SPEED_MULTIPLIER
    } else {
        match stance {
            PlayerStance::Standing if input.sprint => MOVE_SPEED * SPRINT_MULTIPLIER,
            PlayerStance::Standing => MOVE_SPEED,
            PlayerStance::Crouching => MOVE_SPEED * CROUCH_SPEED_MULTIPLIER,
            PlayerStance::Prone => MOVE_SPEED * PRONE_SPEED_MULTIPLIER,
        }
    };
    let capsule_length = match stance {
        PlayerStance::Standing => PLAYER_LENGTH,
        PlayerStance::Crouching => CROUCH_LENGTH,
        PlayerStance::Prone => PRONE_LENGTH,
    };

    let yaw_rotation = Quat::from_rotation_y(input.yaw);
    let forward = yaw_rotation * Vec3::NEG_Z;
    let right = yaw_rotation * Vec3::X;
    let wish = forward * input.move_axis.y + right * input.move_axis.x;
    let wish_dir = wish.normalize_or_zero();

    // Friction before acceleration, as Quake has it: reversed, a standing
    // start would be scaled down on the frame it was granted.
    let mut horizontal = if player.grounded {
        apply_friction(player.horizontal_velocity, GROUND_FRICTION, dt)
    } else {
        player.horizontal_velocity
    };
    let accel = if player.grounded { GROUND_ACCEL } else { AIR_ACCEL };
    if wish_dir != Vec3::ZERO {
        horizontal = accelerate(horizontal, wish_dir, move_speed, accel, dt);
    }
    // Quake's PM_ClipVelocity. Fed in flat, a walk into an up-slope points
    // into the surface and most of it is lost sliding off.
    if player.grounded && player.ground_normal.y > 0.0 {
        let into_ground = horizontal.dot(player.ground_normal);
        if into_ground < 0.0 {
            horizontal -= player.ground_normal * into_ground;
        }
    } else {
        horizontal.y = 0.0;
    }

    let mut vertical_velocity = player.vertical_velocity;
    if player.grounded {
        if input.jump {
            vertical_velocity = JUMP_SPEED;
        } else if vertical_velocity < 0.0 {
            vertical_velocity = -1.0;
        }
    }
    vertical_velocity += GRAVITY * dt;

    // Along the ground's own normal, not straight down: straight down aims
    // into a slope and cancels the clip above.
    let velocity = if player.grounded && vertical_velocity < 0.0 && player.ground_normal.y > 0.0 {
        horizontal - player.ground_normal * vertical_velocity.abs()
    } else {
        horizontal + Vec3::Y * vertical_velocity
    };
    let shape = Collider::capsule(PLAYER_RADIUS, capsule_length);
    let config = MoveAndSlideConfig::default();
    // Debris is excluded: a shell landing inside the capsule would otherwise
    // read as a wall and slide the player off it.
    let filter = SpatialQueryFilter::from_mask(GameLayer::Default);

    let before = transform.translation;
    let was_grounded = player.grounded;
    let mut grounded_this_frame = false;
    let mut obstructed = false;
    let heading = Vec3::new(velocity.x, 0.0, velocity.z).normalize_or_zero();
    let mut ground_normal = Vec3::ZERO;
    let output = mover.move_and_slide(
        &shape,
        transform.translation,
        transform.rotation,
        velocity,
        std::time::Duration::from_secs_f32(dt.max(0.0)),
        &config,
        &filter,
        |mut hit| {
            if resolve_slope_hit(&mut hit, velocity, was_grounded) {
                grounded_this_frame = true;
                // The flattest walkable normal wins: a capsule in a corner
                // touches a wall and a floor, and the floor is underfoot.
                let normal = hit.normal.as_vec3();
                if normal.y > ground_normal.y {
                    ground_normal = normal;
                }
            }
            // Walkable is not out of the way: a round bottom meets a low
            // ledge on its top edge, where the normal is 1 - height/radius,
            // so anything under 17.5cm reads as floor.
            if -hit.normal.as_vec3().dot(heading) > OPPOSING_CONTACT {
                obstructed = true;
            }
            MoveAndSlideHitResponse::Accept
        },
    );

    let horizontal_velocity = Vec3::new(velocity.x, 0.0, velocity.z);
    // No step while still rising, so a jump brushing a ledge stays a jump.
    let stepped_up = (was_grounded && obstructed && vertical_velocity <= 0.0)
        .then(|| {
            try_step_up(
                &mover,
                &shape,
                transform.rotation,
                transform.translation,
                output.position,
                horizontal_velocity,
                &config,
                &filter,
                dt,
            )
        })
        .flatten();

    transform.translation = stepped_up.map_or(output.position, |step| step.position);
    if let Some(step) = stepped_up.filter(|step| step.ground_normal.y > ground_normal_min_y()) {
        grounded_this_frame = true;
        ground_normal = step.ground_normal;
    }
    player.grounded = grounded_this_frame;
    player.ground_normal = if ground_normal == Vec3::ZERO { Vec3::Y } else { ground_normal };
    player.vertical_velocity = if player.grounded { vertical_velocity.min(0.0) } else { vertical_velocity };
    // What the solver allowed, capped at what was asked for: a player held
    // against a wall would otherwise bank speed they never travelled at, and
    // a step is a correction rather than travel.
    let moved = transform.translation - before;
    player.horizontal_velocity = if dt > 0.0 {
        (Vec3::new(moved.x, 0.0, moved.z) / dt).clamp_length_max(horizontal.length())
    } else {
        Vec3::ZERO
    };
}

fn simulate_noclip_step(transform: &mut Transform, input: &ClientInput, dt: f32) {
    let speed = if input.sprint { NOCLIP_SPEED * SPRINT_MULTIPLIER } else { NOCLIP_SPEED };

    let yaw_rotation = Quat::from_rotation_y(input.yaw);
    let forward = yaw_rotation * Vec3::NEG_Z;
    let right = yaw_rotation * Vec3::X;
    let horizontal = forward * input.move_axis.y + right * input.move_axis.x;

    let vertical = if input.jump {
        1.0
    } else if input.stance == PlayerStance::Crouching {
        -1.0
    } else {
        0.0
    };

    let direction = horizontal + Vec3::Y * vertical;
    let direction = direction.normalize_or_zero();
    transform.translation += direction * speed * dt;
}
