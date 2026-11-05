use bevy::prelude::*;

use crate::*;

/// A round landing on a body that is still playing its death clip.
///
/// Local and unserialised, unlike the shot events in `shared`: it names an
/// entity, and it is feedback rather than a fact about the world. The body is
/// already dead, so nothing downstream of this takes damage, scores a kill,
/// or queues a respawn.
#[derive(Event, Clone, Copy)]
pub(crate) struct CorpseHit {
    pub(crate) body: Entity,
    pub(crate) point: Vec3,
    pub(crate) normal: Vec3,
    pub(crate) damage: f32,
    /// Which way the round was going, for the spray to follow.
    pub(crate) direction: Vec3,
}

/// The same flinch a living zombie gets. It survives the death clip because
/// `apply_bone_flinch` runs in PostUpdate after `AnimationSystems` and writes
/// the bone transform last -- the clip poses the body, the flinch then throws
/// the limb that was hit.
pub(crate) fn on_corpse_hit(
    trigger: On<CorpseHit>,
    mut commands: Commands,
    bodies: Query<&ZombieHitboxBones>,
    globals: Query<&GlobalTransform>,
    child_of: Query<&ChildOf>,
    blood_assets: Res<crate::blood::BloodAssets>,
    blood_enabled: Res<BloodEnabled>,
    spatial_query: SpatialQuery,
    time: Res<Time>,
) {
    let CorpseHit { body, point, normal, damage, direction } = *trigger.event();
    let Ok(bones) = bodies.get(body) else {
        return;
    };
    let Some((bone, bone_global)) = nearest_zombie_bone(bones, point, &globals) else {
        return;
    };
    // A corpse takes the hit too: it bleeds, and it moves. Nothing else
    // about the round counts -- no damage, no kill, no chime -- but one
    // going into a body has to look like one.
    spawn_bone_flinch(&mut commands, bones, bone, -normal, damage, &globals, &child_of);
    spawn_blood_effects(
        &mut commands,
        &blood_assets,
        &spatial_query,
        &time,
        point,
        normal,
        Some((bone, &bone_global)),
        false,
        direction,
        *blood_enabled,
    );
}

pub(crate) fn spawn_zombie_hit_feedback(
    commands: &mut Commands,
    blood_assets: &BloodAssets,
    spark_assets: &SparkAssets,
    spatial_query: &SpatialQuery,
    time: &Time,
    point: Vec3,
    normal: Vec3,
    damage: f32,
    bone_for_blood: Option<(Entity, &GlobalTransform)>,
    killed: bool,
    show_floating_damage: bool,
    shot_direction: Vec3,
    blood: BloodEnabled,
    sparks: bool,
) {
    if damage > 0.0 {
        if show_floating_damage {
            spawn_floating_damage(commands, point, damage);
        }
        spawn_blood_effects(commands, blood_assets, spatial_query, time, point, normal, bone_for_blood, killed, shot_direction, blood);
    } else {
        spawn_spark_effects(commands, spark_assets, time, point, normal, sparks);
    }
}

/// How hard the spring pulls a knocked bone home, and how much of the swing
/// it eats on the way. Tuned together: stiffness alone sets how long the
/// whole thing takes, damping sets how much of it is overshoot.
const FLINCH_STIFFNESS: f32 = 190.0;
const FLINCH_DAMPING: f32 = 15.0;
/// Below both of these the bone is home and the spring is thrown away.
const FLINCH_AT_REST: f32 = 0.004;
/// Turns the throw a hit used to be set to into the speed it is lent. The
/// spring turns speed back into roughly that throw, so the old angles mean
/// what they always did.
const FLINCH_KICK: f32 = 11.0;
const FLINCH_MAX_ANGLE_DEG: f32 = 78.0;
const FLINCH_CHAIN_FALLOFF: [f32; 3] = [1.0, 0.68, 0.38];
const FLINCH_HEAD_TO_CHEST: f32 = 0.3;

/// A bone knocked off the pose the clip put it in, on its way back.
///
/// A spring with momentum rather than a deflection that decays: a shot
/// lends the bone speed and the spring carries it out, back through where
/// it started and a little past, then home. The decay it replaces began at
/// full throw on the frame of the hit, which is a bone that was already
/// bent when the round arrived.
///
/// And speed adds where a deflection only overwrote, so a burst builds on
/// itself instead of every round restarting the same twitch.
#[derive(Component)]
pub(crate) struct BoneFlinch {
    world_axis: Vec3,
    /// Where the bone is now, in radians about that axis.
    angle: f32,
    /// How fast it is going, in radians a second.
    speed: f32,
    /// What this put on the bone last frame, and the rotation it left
    /// behind.
    ///
    /// The flinch is a turn laid over whatever pose the bone already has,
    /// which is right while something else writes that pose outright every
    /// frame -- the clip does while the body walks, the ragdoll does while
    /// it falls. Past both there is nobody, and a turn laid over its own
    /// output sixty times a second stops being a flinch and becomes a
    /// motor. So it checks: if the bone is exactly where this left it, no
    /// one else has written it, and its own last turn comes off first.
    added: Quat,
    left_at: Quat,
}

fn flinch_chain(bones: &ZombieHitboxBones, hit: Entity) -> Vec<Entity> {
    for limb in [&bones.left_arm, &bones.right_arm, &bones.left_leg, &bones.right_leg] {
        if let Some(index) = limb.iter().position(|bone| *bone == Some(hit)) {
            return limb[..=index].iter().rev().copied().flatten().collect();
        }
    }
    if Some(hit) == bones.head {
        return [Some(hit), bones.chest].into_iter().flatten().collect();
    }
    vec![hit]
}

pub(crate) fn spawn_bone_flinch(
    commands: &mut Commands,
    bones: &ZombieHitboxBones,
    hit_bone: Entity,
    impulse: Vec3,
    damage: f32,
    globals: &Query<&GlobalTransform>,
    child_of: &Query<&ChildOf>,
) {
    let chain = flinch_chain(bones, hit_bone);
    let Some(&first) = chain.first() else { return };

    let bone_direction = child_of
        .get(first)
        .ok()
        .and_then(|parent| globals.get(parent.0).ok().zip(globals.get(first).ok()))
        .map(|(parent, bone)| (bone.translation() - parent.translation()).normalize_or_zero())
        .unwrap_or(Vec3::Y);

    let mut axis = bone_direction.cross(impulse).normalize_or_zero();
    if axis == Vec3::ZERO {
        axis = impulse.cross(Vec3::Y).normalize_or_zero();
    }
    if axis == Vec3::ZERO {
        return;
    }

    let head_hit = Some(hit_bone) == bones.head;
    let base = FLINCH_MAX_ANGLE_DEG.to_radians() * (damage / shared::CHEST_DAMAGE).clamp(0.4, 1.0);

    for (index, bone) in chain.into_iter().enumerate() {
        let falloff = if head_hit && index > 0 {
            FLINCH_HEAD_TO_CHEST
        } else {
            FLINCH_CHAIN_FALLOFF[index.min(FLINCH_CHAIN_FALLOFF.len() - 1)]
        };
        // Lent as speed, not set as a bend, and added to whatever the last
        // round left so a burst builds.
        commands.entity(bone).queue(move |mut bone: EntityWorldMut| {
            match bone.get_mut::<BoneFlinch>() {
                Some(mut flinch) if flinch.world_axis.dot(axis) > 0.0 => {
                    flinch.speed += base * falloff * FLINCH_KICK;
                }
                _ => {
                    bone.insert(BoneFlinch {
                        world_axis: axis,
                        angle: 0.0,
                        speed: base * falloff * FLINCH_KICK,
                        added: Quat::IDENTITY,
                        left_at: Quat::IDENTITY,
                    });
                }
            }
        });
    }
}

pub(crate) fn apply_bone_flinch(
    time: Res<Time>,
    mut commands: Commands,
    child_of: Query<&ChildOf>,
    globals: Query<&GlobalTransform>,
    mut flinches: Query<(Entity, &mut BoneFlinch)>,
    mut transforms: Query<&mut Transform>,
) {
    let dt = time.delta_secs().min(1.0 / 30.0);
    for (entity, mut flinch) in &mut flinches {
        // Implicit, so a frame spike cannot pump the spring: the same form
        // the sights and the look sway already ride.
        let f = 1.0 + dt * FLINCH_DAMPING;
        let speed = (flinch.speed - dt * FLINCH_STIFFNESS * flinch.angle)
            / (f + dt * dt * FLINCH_STIFFNESS);
        flinch.speed = speed;
        flinch.angle += speed * dt;
        let at_rest = flinch.angle.abs() < FLINCH_AT_REST && speed.abs() < FLINCH_AT_REST * 8.0;
        let Ok(mut transform) = transforms.get_mut(entity) else {
            if at_rest {
                commands.entity(entity).remove::<BoneFlinch>();
            }
            continue;
        };
        // The pose to lay the turn over. Exactly where this left the bone
        // means nothing else has written it since, so its own last turn is
        // still in there and comes off first.
        let base = if transform.rotation == flinch.left_at {
            flinch.added.inverse() * transform.rotation
        } else {
            transform.rotation
        };
        if at_rest {
            transform.rotation = base;
            commands.entity(entity).remove::<BoneFlinch>();
            continue;
        }
        let parent_rotation = child_of
            .get(entity)
            .ok()
            .and_then(|parent| globals.get(parent.0).ok())
            .map(|global| global.rotation())
            .unwrap_or(Quat::IDENTITY);
        let world_rotation = Quat::from_axis_angle(flinch.world_axis, flinch.angle);
        let local_rotation = parent_rotation.inverse() * world_rotation * parent_rotation;
        transform.rotation = local_rotation * base;
        flinch.added = local_rotation;
        flinch.left_at = transform.rotation;
    }
}

fn every_bone(bones: &ZombieHitboxBones) -> impl Iterator<Item = Entity> + use<> {
    bones
        .head
        .into_iter()
        .chain(bones.belly)
        .chain(bones.chest)
        .chain(bones.left_arm.into_iter().flatten())
        .chain(bones.right_arm.into_iter().flatten())
        .chain(bones.left_leg.into_iter().flatten())
        .chain(bones.right_leg.into_iter().flatten())
}

pub(crate) fn nearest_zombie_bone(bones: &ZombieHitboxBones, point: Vec3, globals: &Query<&GlobalTransform>) -> Option<(Entity, GlobalTransform)> {
    every_bone(bones)
        .filter_map(|entity| globals.get(entity).ok().map(|g| (entity, *g)))
        .min_by(|(_, a), (_, b)| a.translation().distance_squared(point).total_cmp(&b.translation().distance_squared(point)))
}

const ZOMBIE_HEAD_YAW_LIMIT_DEG: f32 = 72.0;
const ZOMBIE_HEAD_PITCH_LIMIT_DEG: f32 = 32.0;
const ZOMBIE_HEAD_TURN_RATE: f32 = 9.0;

#[derive(Component, Default)]
pub(crate) struct ZombieHeadLook {
    yaw: f32,
    pitch: f32,
}

pub(crate) fn aim_zombie_heads(
    time: Res<Time>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
    mut zombies: Query<(&Transform, &ZombieHitboxBones, &mut ZombieHeadLook), With<Zombie>>,
    child_of: Query<&ChildOf>,
    globals: Query<&GlobalTransform>,
    mut bone_transforms: Query<&mut Transform, Without<Zombie>>,
) {
    let Ok(camera_global) = camera.single() else {
        return;
    };
    let target = camera_global.translation();
    let blend = (ZOMBIE_HEAD_TURN_RATE * time.delta_secs()).min(1.0);
    let yaw_limit = ZOMBIE_HEAD_YAW_LIMIT_DEG.to_radians();
    let pitch_limit = ZOMBIE_HEAD_PITCH_LIMIT_DEG.to_radians();

    for (body, bones, mut look) in &mut zombies {
        let Some(head) = bones.head else {
            continue;
        };
        let Ok(head_global) = globals.get(head) else {
            continue;
        };

        let to_target = target - head_global.translation();
        let flat = Vec3::new(to_target.x, 0.0, to_target.z);
        let forward = body.rotation * Vec3::Z;
        let forward_flat = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        if flat.length_squared() < 1e-6 || forward_flat == Vec3::ZERO {
            continue;
        }
        let target_flat = flat.normalize();

        let yaw = forward_flat
            .cross(target_flat)
            .dot(Vec3::Y)
            .atan2(forward_flat.dot(target_flat))
            .clamp(-yaw_limit, yaw_limit);
        let pitch = to_target.y.atan2(flat.length()).clamp(-pitch_limit, pitch_limit);

        look.yaw += (yaw - look.yaw) * blend;
        look.pitch += (pitch - look.pitch) * blend;

        let yaw_rotation = Quat::from_axis_angle(Vec3::Y, look.yaw);
        let right = (yaw_rotation * forward_flat.cross(Vec3::Y)).normalize_or_zero();
        let world_rotation = Quat::from_axis_angle(right, look.pitch) * yaw_rotation;

        let parent_rotation = child_of
            .get(head)
            .ok()
            .and_then(|parent| globals.get(parent.0).ok())
            .map(|global| global.rotation())
            .unwrap_or(Quat::IDENTITY);
        let local_rotation = parent_rotation.inverse() * world_rotation * parent_rotation;
        let Ok(mut transform) = bone_transforms.get_mut(head) else {
            continue;
        };
        transform.rotation = local_rotation * transform.rotation;
    }
}

