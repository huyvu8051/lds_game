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

/// Degrees the hips lead a turn, per radian a second of turning.
///
/// This began as orientation warping -- take up the gap between where the
/// body faces and where it is going, so the legs point along the path while
/// the torso comes round. Measured over a minute of play, that gap was
/// exactly zero on every one of fifty-eight thousand frames: the body turns
/// at up to 130 degrees a second, a path's direction changes by far less
/// than that in a frame, and turn_towards therefore lands on its target
/// every time. There is no lag to fill.
///
/// So it reads the turn itself instead. The hips lead and the shoulders
/// follow, which is how a person turns -- not a gap being filled but an
/// order the body does things in. The chest hands part of it back so the
/// shoulders do not simply travel with the hips.
const HIP_LEAD_DEG_PER_RAD_SEC: f32 = 11.0;
const HIP_LEAD_MAX_DEG: f32 = 26.0;
const CHEST_RECOVER_FRACTION: f32 = 0.45;

/// Banks the body into its turn.
///
/// On a bone rather than the root transform on purpose: the root is what the
/// hitbox cylinder is cast against, and a leaning hitbox is a different thing
/// to shoot at.
pub(crate) fn lean_zombie_into_turn(
    walk: Res<crate::ZombieWalk>,
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
        // Into the turn, so a body swinging left drops its left side. The
        // sign is the one that was wrong: it leaned away, which is what a
        // thing being pushed does, not what a thing turning itself does.
        let lean = if !walk.lean { 0.0 } else { motion.turn_rate * LEAN_DEG_PER_RAD_SEC }
        .clamp(-LEAN_MAX_DEG, LEAN_MAX_DEG)
        .to_radians();
        // Negated against turn_rate's own sign. That rate is positive when
        // the forward vector sweeps counterclockwise in the x/z plane, and a
        // positive rotation about Y sweeps the other way -- so leading the
        // turn means turning the hips by the opposite sign.
        let lead = if walk.hips {
            (-motion.turn_rate * HIP_LEAD_DEG_PER_RAD_SEC)
                .clamp(-HIP_LEAD_MAX_DEG, HIP_LEAD_MAX_DEG)
                .to_radians()
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
