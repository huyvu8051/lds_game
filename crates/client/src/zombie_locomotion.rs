//! Bending the walk clip so a turn reads as one.
//!
//! The clip knows how to walk in a straight line and nothing else. Everything
//! here is the difference between that and what the body is actually doing.
//!
//! None of it needs a new clip. Authoring turn animations, or matching motion
//! against a library of them, is what a game with a library does -- each
//! zombie here has a handful of clips, so the work has to come from bending
//! the ones there are.

use bevy::prelude::*;
use shared::{Zombie, ZombieHitboxBones, ZombieMotion};

/// Degrees the body banks into a turn, per radian a second of turning.
/// Capped, because a shambling thing leans, it does not corner.
const LEAN_DEG_PER_RAD_SEC: f32 = 9.0;
const LEAN_MAX_DEG: f32 = 14.0;

/// How much of the gap between where the body faces and where it is going
/// the hips are allowed to take up.
///
/// Orientation warping, cheaply. Because the body only turns at a limited
/// rate, it spends a turn pointing somewhere the path has already left; this
/// lets the legs point along the path while the torso is still coming round,
/// which is what stops a turn reading as the whole model being rotated by a
/// crane. The chest hands part of it back so the shoulders do not travel
/// with the hips.
///
/// It costs nothing in a straight line: the gap is zero, so the rotation is
/// identity.
const HIP_LEAD_FRACTION: f32 = 0.55;
const HIP_LEAD_MAX_DEG: f32 = 38.0;
const CHEST_RECOVER_FRACTION: f32 = 0.45;

/// Banks the body into its turn.
///
/// On a bone rather than the root transform on purpose: the root is what the
/// hitbox cylinder is cast against, and a leaning hitbox is a different thing
/// to shoot at.
pub(crate) fn lean_zombie_into_turn(
    zombies: Query<(&Transform, &ZombieMotion, &ZombieHitboxBones), With<Zombie>>,
    child_of: Query<&ChildOf>,
    mut transforms: Query<&mut Transform, Without<Zombie>>,
) {
    for (root, motion, bones) in &zombies {
        // The hips are the parent of either thigh. Not in the hitbox table,
        // because nothing shoots at them.
        let Some(hips) = bones.left_leg[0].and_then(|leg| child_of.get(leg).ok()).map(|p| p.0)
        else {
            continue;
        };
        // Into the turn, so a body swinging left drops its left side.
        let lean = (-motion.turn_rate * LEAN_DEG_PER_RAD_SEC)
            .clamp(-LEAN_MAX_DEG, LEAN_MAX_DEG)
            .to_radians();
        // How far the path has got ahead of the body. Zero once the turn is
        // over, which is what makes this free in a straight line.
        let lead = if motion.horizontal_velocity.length_squared() > 1e-4 {
            let facing = (root.rotation * Vec3::NEG_Z).xz();
            let going = -motion.horizontal_velocity.xz().normalize_or_zero();
            let gap = (facing.x * going.y - facing.y * going.x).atan2(facing.dot(going));
            (gap * HIP_LEAD_FRACTION)
                .clamp(-HIP_LEAD_MAX_DEG.to_radians(), HIP_LEAD_MAX_DEG.to_radians())
        } else {
            0.0
        };

        if let Ok(mut hip) = transforms.get_mut(hips) {
            hip.rotation = Quat::from_rotation_y(lead) * Quat::from_rotation_z(lean) * hip.rotation;
        }
        // Part of the lead handed back, so the shoulders stay nearer where
        // the body is pointed than where its feet are going.
        if let Some(chest) = bones.chest {
            if let Ok(mut chest) = transforms.get_mut(chest) {
                chest.rotation =
                    Quat::from_rotation_y(-lead * CHEST_RECOVER_FRACTION) * chest.rotation;
            }
        }
    }
}

/// Below this the foot counts as planted and gets pinned.
const FOOT_PLANT_HEIGHT: f32 = 0.12;
/// How far a pinned foot may be dragged before the pin is given up. A foot
/// held past this is a leg stretched past its reach, which looks worse than
/// the slide it was fixing.
const FOOT_PIN_MAX_DRAG: f32 = 0.45;

#[derive(Component, Default)]
pub(crate) struct ZombieFootPins {
    /// Where each foot was when it planted, in world space, or nothing while
    /// it is in the air.
    planted: [Option<Vec3>; 2],
}

/// Keeps a planted foot where it was planted.
///
/// The clip plants feet along a straight line and the body turns underneath
/// them, which is the whole of why a turn slides. Pinning the foot and
/// bending the leg to reach it is the direct answer: the foot stops moving
/// because the foot is told not to move.
/// Off with NO_FOOT_PIN=1.
///
/// Of the four things done to the walk this is the only one that can look
/// badly wrong rather than merely wrong: the others nudge a bone by a few
/// degrees, this one decides where a leg must reach and bends it there. The
/// plant test leans on LEG_SPAN_GUESS, which is a guess.
fn foot_pinning_enabled() -> bool {
    static CACHED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CACHED.get_or_init(|| std::env::var("NO_FOOT_PIN").is_err())
}

pub(crate) fn pin_zombie_feet(
    mut zombies: Query<(&ZombieHitboxBones, &mut ZombieFootPins), With<Zombie>>,
    children: Query<&Children>,
    child_of: Query<&ChildOf>,
    mut transforms: Query<(&mut Transform, &mut GlobalTransform)>,
) {
    if !foot_pinning_enabled() {
        return;
    }
    for (bones, mut pins) in &mut zombies {
        for (side, leg) in [bones.left_leg, bones.right_leg].into_iter().enumerate() {
            let (Some(thigh), Some(shin), Some(foot)) = (leg[0], leg[1], leg[2]) else {
                continue;
            };
            let Ok(foot_now) = transforms.get(foot).map(|(_, g)| g.translation()) else {
                continue;
            };

            // Height off the ground decides it. The clip's own vertical
            // travel is the plant signal, which is why this needs no phase
            // and no event from the animation.
            let hip_y = transforms.get(thigh).map(|(_, g)| g.translation().y).unwrap_or(foot_now.y);
            let planted = foot_now.y - (hip_y - LEG_SPAN_GUESS) < FOOT_PLANT_HEIGHT;

            if !planted {
                pins.planted[side] = None;
                continue;
            }
            let anchor = *pins.planted[side].get_or_insert(foot_now);
            if anchor.distance(foot_now) > FOOT_PIN_MAX_DRAG {
                // Given up rather than stretched: a leg reaching past itself
                // is worse than the slide it was fixing.
                pins.planted[side] = Some(foot_now);
                continue;
            }
            solve_leg(thigh, shin, foot, anchor, &child_of, &children, &mut transforms);
        }
    }
}

/// Roughly how far the foot hangs below the thigh when standing, used only to
/// turn an absolute foot height into one measured against the body.
const LEG_SPAN_GUESS: f32 = 0.85;

/// Two bones to a point, the same construction the hands use.
fn solve_leg(
    thigh: Entity,
    shin: Entity,
    foot: Entity,
    target: Vec3,
    child_of: &Query<&ChildOf>,
    children: &Query<&Children>,
    transforms: &mut Query<(&mut Transform, &mut GlobalTransform)>,
) -> Option<()> {
    let hip = transforms.get(thigh).ok()?.1.translation();
    let knee = transforms.get(shin).ok()?.1.translation();
    let ankle = transforms.get(foot).ok()?.1.translation();

    let upper = knee.distance(hip);
    let lower = ankle.distance(knee);
    if upper < 1e-4 || lower < 1e-4 {
        return None;
    }
    let span = (target - hip).length().clamp(1e-4, (upper + lower) * 0.999);
    let to_target = (target - hip) / (target - hip).length().max(1e-4);

    // The knee bends forward, so the pole is ahead of the hip. Taken from the
    // leg's own current bend rather than the body's facing, so a leg already
    // turned by the clip is not straightened out by this.
    let pole = knee - hip - to_target * (knee - hip).dot(to_target);
    let plane = if pole.length_squared() < 1e-6 {
        Vec3::Y.cross(to_target).normalize_or_zero()
    } else {
        to_target.cross(pole).normalize_or_zero()
    };
    if plane.length_squared() < 1e-6 {
        return None;
    }

    let cos_hip = ((upper * upper + span * span - lower * lower) / (2.0 * upper * span)).clamp(-1.0, 1.0);
    let thigh_dir = Quat::from_axis_angle(plane, -cos_hip.acos()) * to_target;
    let new_knee = hip + thigh_dir * upper;
    let shin_dir = (target - new_knee).normalize_or_zero();

    aim_bone(thigh, thigh_dir, plane, child_of, children, transforms);
    aim_bone(shin, shin_dir, plane, child_of, children, transforms);
    Some(())
}

/// Points a bone down `along`, keeping its twist in the plane, and carries
/// everything under it with the change.
fn aim_bone(
    bone: Entity,
    along: Vec3,
    plane: Vec3,
    child_of: &Query<&ChildOf>,
    children: &Query<&Children>,
    transforms: &mut Query<(&mut Transform, &mut GlobalTransform)>,
) {
    if along.length_squared() < 1e-6 {
        return;
    }
    // Mixamo legs run down -Y out of the joint.
    let wanted = Quat::from_mat3(&Mat3::from_cols(
        plane.cross(along).normalize_or_zero(),
        -along,
        plane,
    ));
    let Ok((_, global)) = transforms.get(bone) else { return };
    let here = global.translation();
    let parent_inverse = child_of
        .get(bone)
        .ok()
        .and_then(|p| transforms.get(p.0).ok())
        .map(|(_, g)| g.rotation().inverse())
        .unwrap_or(Quat::IDENTITY);
    if let Ok((mut local, mut global)) = transforms.get_mut(bone) {
        local.rotation = parent_inverse * wanted;
        *global = GlobalTransform::from(
            Transform::from_translation(here).with_rotation(wanted).with_scale(local.scale),
        );
    }
    let carried = transforms.get(bone).map(|(_, g)| *g).unwrap_or_default();
    repropagate(bone, carried, children, transforms);
}

/// Rebuilds the global transforms under a bone that was just turned.
fn repropagate(
    bone: Entity,
    bone_global: GlobalTransform,
    children: &Query<&Children>,
    transforms: &mut Query<(&mut Transform, &mut GlobalTransform)>,
) {
    let Ok(kids) = children.get(bone) else { return };
    for kid in kids {
        let Ok((local, _)) = transforms.get(*kid) else { continue };
        let next = bone_global * *local;
        if let Ok((_, mut global)) = transforms.get_mut(*kid) {
            *global = next;
        }
        repropagate(*kid, next, children, transforms);
    }
}

/// Gives every zombie somewhere to remember its feet.
pub(crate) fn attach_foot_pins(
    mut commands: Commands,
    zombies: Query<Entity, (With<Zombie>, Without<ZombieFootPins>)>,
) {
    for entity in &zombies {
        commands.entity(entity).try_insert(ZombieFootPins::default());
    }
}
