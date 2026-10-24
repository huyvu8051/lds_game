use avian3d::prelude::*;
use bevy::prelude::*;

use crate::{
    ClientInput, Player, PlayerStance, AIM_MOVE_SPEED_MULTIPLIER, CROUCH_LENGTH, CROUCH_SPEED_MULTIPLIER,
    GameLayer, GRAVITY, ground_normal_min_y, JUMP_SPEED, MOVE_SPEED, NOCLIP_SPEED, PLAYER_LENGTH, PLAYER_RADIUS,
    PRONE_LENGTH, PRONE_SPEED_MULTIPLIER, SPRINT_MULTIPLIER,
};

const PROGRESS_MEANING_UNOBSTRUCTED: f32 = 0.7;
const MIN_HEADROOM_FRACTION: f32 = 0.5;
const LANDING_PROBE_OVERSHOOT: f32 = 0.05;
const STEP_FORWARD_PROBE: f32 = 0.3;

pub const STEP_HEIGHT: f32 = 0.45;

pub fn blocked_horizontally(start: Vec3, end: Vec3, horizontal_velocity: Vec3, dt: f32) -> bool {
    let wanted = horizontal_velocity.length() * dt;
    if wanted <= 1e-4 {
        return false;
    }
    let moved = Vec3::new(end.x - start.x, 0.0, end.z - start.z).length();
    moved < wanted * PROGRESS_MEANING_UNOBSTRUCTED
}

pub fn try_step_up(
    mover: &MoveAndSlide,
    shape: &Collider,
    rotation: Quat,
    start: Vec3,
    horizontal_velocity: Vec3,
    config: &MoveAndSlideConfig,
    filter: &SpatialQueryFilter,
) -> Option<Vec3> {
    let speed = horizontal_velocity.length();
    if speed < 1e-4 {
        return None;
    }

    let headroom = mover
        .spatial_query
        .cast_shape(shape, start, rotation, Dir3::Y, &ShapeCastConfig::from_max_distance(STEP_HEIGHT), filter)
        .map_or(STEP_HEIGHT, |hit| hit.distance);
    if headroom < STEP_HEIGHT * MIN_HEADROOM_FRACTION {
        return None;
    }

    let raised = start + Vec3::Y * headroom;
    let stepped = mover.move_and_slide(
        shape,
        raised,
        rotation,
        horizontal_velocity,
        std::time::Duration::from_secs_f32(STEP_FORWARD_PROBE / speed),
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
    if landing.normal1.y <= ground_normal_min_y() {
        return None;
    }
    Some(stepped.position - Vec3::Y * landing.distance)
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
/// The floor under the friction calculation, and the whole reason this stops
/// rather than creeping. Scaling a velocity down by a fixed fraction each
/// frame never reaches zero; holding the drop constant below this speed makes
/// the last of it finite.
pub const STOP_SPEED: f32 = 1.2;

/// Quake's, near enough unchanged since 1996 and still what most shooters run.
///
/// Friction pulls down whatever direction you are already going; acceleration
/// pushes along the one you are asking for. Turning falls out of the two
/// without a turn rate existing anywhere -- a hard reverse decelerates and
/// accelerates again because that is what the arithmetic does.
///
/// `add = wishspeed - current` is what keeps it honest: pushing along your own
/// velocity can never take you past the limit, because the amount left to add
/// shrinks to nothing as you approach it.
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
    // The direction asked for, not the velocity. What the player gets is
    // whatever friction and acceleration leave behind.
    let wish = forward * input.move_axis.y + right * input.move_axis.x;
    let wish_dir = wish.normalize_or_zero();

    // Friction first, then acceleration, in that order -- the same order
    // Quake uses. Reversed, a standing start would be scaled down by friction
    // on the very frame it was granted and the first step would be short.
    //
    // Only on the ground: in the air there is nothing to rub against, which is
    // what makes a jump commit you to the direction you left at.
    let mut horizontal = if player.grounded {
        apply_friction(player.horizontal_velocity, GROUND_FRICTION, dt)
    } else {
        player.horizontal_velocity
    };
    let accel = if player.grounded { GROUND_ACCEL } else { AIR_ACCEL };
    if wish_dir != Vec3::ZERO {
        horizontal = accelerate(horizontal, wish_dir, move_speed, accel, dt);
    }
    // Clipped to the ground underfoot, which is what Quake's PM_ClipVelocity
    // does and why it needs no step for a ramp.
    //
    // Fed in flat, a walk into an up-slope points into the surface: the solver
    // has to slide it off, and most of the forward motion goes with it.
    // Horizontal progress then falls under what blocked_horizontally calls
    // obstructed, step-up fires -- every frame, on a slope -- and each one is
    // a free teleport up and along. That is the fast slide, and it was never
    // about velocity.
    //
    // Clipped, the walk already runs along the slope and arrives with its
    // speed intact, so nothing reads as blocked and no step is asked for.
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

    let velocity = horizontal + Vec3::Y * vertical_velocity;
    let shape = Collider::capsule(PLAYER_RADIUS, capsule_length);
    let config = MoveAndSlideConfig::default();
    // Debris is excluded: a shell landing inside the capsule would otherwise
    // read as a wall and slide the player off it.
    let filter = SpatialQueryFilter::from_mask(GameLayer::Default);

    let before = transform.translation;
    let was_grounded = player.grounded;
    let mut grounded_this_frame = false;
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
                // The flattest walkable thing underfoot wins: a capsule in a
                // corner touches a wall and a floor, and the floor is the one
                // being stood on.
                let normal = hit.normal.as_vec3();
                if normal.y > ground_normal.y {
                    ground_normal = normal;
                }
            }
            MoveAndSlideHitResponse::Accept
        },
    );

    let horizontal_velocity = Vec3::new(velocity.x, 0.0, velocity.z);
    let stepped_up = was_grounded
        .then(|| blocked_horizontally(transform.translation, output.position, horizontal_velocity, dt))
        .unwrap_or(false)
        .then(|| {
            try_step_up(
                &mover,
                &shape,
                transform.rotation,
                transform.translation,
                horizontal_velocity,
                &config,
                &filter,
            )
        })
        .flatten();

    transform.translation = stepped_up.unwrap_or(output.position);
    player.grounded = grounded_this_frame || stepped_up.is_some();
    player.ground_normal = if ground_normal == Vec3::ZERO { Vec3::Y } else { ground_normal };
    player.vertical_velocity = if player.grounded { vertical_velocity.min(0.0) } else { vertical_velocity };
    // What the solver actually allowed, not what was asked for. Carrying the
    // requested velocity instead would let a player held against a wall build
    // up speed they never travelled at, and release it the moment they turned
    // away from it.
    //
    // Capped at what was asked for, because not all of the movement is
    // travel. try_step_up teleports the capsule onto a ledge -- a correction,
    // not a journey -- and a third of a metre over one frame reads as thirty
    // metres a second. Carried forward, that launched the player off every
    // kerb they walked up.
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
