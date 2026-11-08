//! Where rounds went into a body, kept in the mesh's own bind space.
//!
//! A body cannot take a projected decal: the depth buffer knows where a chest
//! is this frame, not which bone a mark belongs to. What the shipped games do
//! instead is paint into a texture in the character's UV space, which wants
//! UVs that do not overlap -- and four of the seven zombie characters here
//! fail that, two of them with UVs running outside the unit square entirely.
//!
//! Bind space needs neither. `v.position` in the skinned vertex shader is
//! already the vertex before any bone moved it, so a point recorded in that
//! space follows the animation for nothing: the pose changes, the bind pose
//! does not. The cost is one varying and a short loop per fragment.
//!
//! What it gives up is a painted shape. Each wound is a sphere, so it wraps a
//! thin limb and marks the far side, and there is a ceiling on how many a
//! body carries. Neither reads wrong on a zombie.

use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::prelude::*;

/// Mirrors `MAX_WOUNDS` in the renderer and `kMaxWounds` in its shader.
/// Past this the oldest goes, so a body shot twenty times stops gaining
/// marks rather than growing a buffer nobody drains.
pub(crate) const MAX_WOUNDS: usize = 12;

/// How far the blood reaches from where the round went in, in metres.
pub(crate) const WOUND_REACH: f32 = 0.16;

#[derive(Component, Default)]
pub(crate) struct ZombieWounds {
    /// Hits waiting on a skin to resolve against: the bone the round landed
    /// on, where on it in that bone's own frame, and the reach.
    ///
    /// Held rather than converted on the spot because the conversion wants
    /// the skin's inverse bind matrices, and the place that knows a round
    /// landed is the shot path, which has no business querying assets.
    pending: Vec<(Entity, Vec3, f32)>,
    /// Resolved: xyz in bind space, w the reach. What the renderer uploads.
    pub(crate) bind: Vec<[f32; 4]>,
}

impl ZombieWounds {
    pub(crate) fn record(&mut self, bone: Entity, bone_global: &GlobalTransform, at: Vec3, reach: f32) {
        let local = bone_global.affine().inverse().transform_point3(at);
        self.pending.push((bone, local, reach));
    }
}

/// The skin somewhere under a zombie, which is not the zombie itself: the
/// mesh arrives as a child scene from the glb.
fn skin_under(
    root: Entity,
    children: &Query<&Children>,
    skins: &Query<&SkinnedMesh>,
) -> Option<Entity> {
    if skins.contains(root) {
        return Some(root);
    }
    children.get(root).ok()?.iter().find_map(|child| skin_under(child, children, skins))
}

/// Takes each pending hit into bind space, once the skin is there to do it.
///
/// `joint_matrices_for` builds the skinning matrix as `joint_global *
/// inverse_bind`, and the renderer hands that straight to the shader with an
/// identity model matrix. So a world point maps back as
/// `inverse_bind⁻¹ * joint_global⁻¹ * world`, and the right-hand half of that
/// is what `record` already stored.
pub(crate) fn resolve_zombie_wounds(
    mut bodies: Query<(Entity, &mut ZombieWounds)>,
    children: Query<&Children>,
    skins: Query<&SkinnedMesh>,
    bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
) {
    for (entity, mut wounds) in &mut bodies {
        if wounds.pending.is_empty() {
            continue;
        }
        let Some(skin) = skin_under(entity, &children, &skins).and_then(|e| skins.get(e).ok()) else {
            continue;
        };
        let Some(inverse_bindposes) = bindposes.get(&skin.inverse_bindposes) else {
            continue;
        };
        for (bone, local, reach) in std::mem::take(&mut wounds.pending) {
            let Some(joint) = skin.joints.iter().position(|&e| e == bone) else {
                continue;
            };
            let Some(inverse_bind) = inverse_bindposes.iter().nth(joint) else {
                continue;
            };
            let at = inverse_bind.inverse().transform_point3(local);
            if wounds.bind.len() >= MAX_WOUNDS {
                wounds.bind.remove(0);
            }
            wounds.bind.push([at.x, at.y, at.z, reach]);
        }
    }
}
