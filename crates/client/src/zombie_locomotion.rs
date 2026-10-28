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

/// Banks the body into its turn.
///
/// On a bone rather than the root transform on purpose: the root is what the
/// hitbox cylinder is cast against, and a leaning hitbox is a different thing
/// to shoot at.
pub(crate) fn lean_zombie_into_turn(
    zombies: Query<(&ZombieMotion, &ZombieHitboxBones), With<Zombie>>,
    child_of: Query<&ChildOf>,
    mut transforms: Query<&mut Transform, Without<Zombie>>,
) {
    for (motion, bones) in &zombies {
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
        if let Ok(mut hip) = transforms.get_mut(hips) {
            hip.rotation = Quat::from_rotation_z(lean) * hip.rotation;
        }
    }
}
