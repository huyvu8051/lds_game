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
