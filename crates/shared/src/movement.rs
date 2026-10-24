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

/// The tallest thing that can be walked onto without jumping. Just under
/// Quake's eighteen units and Source's eighteen inches, both about 0.45.
/// Zombies read the same constant.
pub const STEP_HEIGHT: f32 = 0.4;

/// TEMPORARY: `NO_STEP_UP=1` refuses every step, so the slide move has to
/// manage ramps and stairs on its own. Here to see what it does without.
fn step_up_disabled() -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OFF.get_or_init(|| std::env::var("NO_STEP_UP").is_ok())
}

/// Where a step put the capsule, and what it landed on. Landing on something
/// too steep to stand on does not refuse the step -- Quake steps first and
/// decides about ground separately, so the caller reads the normal.
#[derive(Clone, Copy)]
pub struct StepUp {
    pub position: Vec3,
    pub ground_normal: Vec3,
}

/// Quake's PM_StepSlideMove: when the slide met something, run the move again
/// from a step-height higher and keep whichever attempt got further. No test
/// for how badly the slide was obstructed -- both thresholds that stood here
/// were wrong, the last by one percent at a 45 degree approach.
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
        // comparison on length alone whatever it met.
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
    // has to slide it off, and most of the forward motion goes with it, so a
    // ramp gets climbed at a crawl. Clipped, the walk already runs along the
    // slope and arrives with its speed intact.
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

    // The push that keeps the feet on the ground goes along the ground's own
    // normal, not straight down.
    //
    // Straight down is fine on the flat and wrong on a slope: it aims into
    // the hill and cancels the climb the ground clip produced a few lines
    // above, which is why that clip looked like it did nothing. It worked for
    // exactly one line.
    //
    // With the step switched off, a 30 degree ramp could not be climbed at
    // all -- so the step had been doing the climbing all along, on slopes as
    // well as stairs, one teleport per frame. That is the whole of the fast
    // slide, and it was never the step's fault: it was covering for this.
    //
    // Quake never hits this because it does not add a downward velocity while
    // walking. Staying on the ground is a separate trace there, not something
    // mixed into the movement vector.
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
                // The flattest walkable thing underfoot wins: a capsule in a
                // corner touches a wall and a floor, and the floor is the one
                // being stood on.
                let normal = hit.normal.as_vec3();
                if normal.y > ground_normal.y {
                    ground_normal = normal;
                }
            }
            // Walkable is not the same as out of the way: a round bottom
            // meets a low ledge on its top edge, where the normal is
            // 1 - height/radius, so anything under 17.5cm reads as floor.
            // Facing back against the heading is what being in the way is.
            if -hit.normal.as_vec3().dot(heading) > OPPOSING_CONTACT {
                obstructed = true;
            }
            MoveAndSlideHitResponse::Accept
        },
    );

    let horizontal_velocity = Vec3::new(velocity.x, 0.0, velocity.z);
    // Quake refuses a step while still going up, so a jump brushing a ledge
    // is a jump and not a free lift onto it.
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
