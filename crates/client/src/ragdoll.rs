use avian3d::prelude::*;
use bevy::prelude::*;

use crate::*;
use shared::GameLayer;

pub(crate) const DEATH_PUSH_SPEED: f32 = 2.2;
const SETTLE_SPEED: f32 = 0.35;
/// How many bodies may be live physics at once.
///
/// Each one is fifteen rigid bodies, fourteen joints and, since muscles, a
/// torque written per bone per frame. A wave pushes zombies out in bursts,
/// so corpses piling up is a matter of when rather than whether, and the
/// phone has no room for the pile.
///
/// Four is a guess until it is measured on the device; the figure to watch
/// is the solver's share of the frame, not the body count.
const MAX_ACTIVE_RAGDOLLS: usize = 4;

/// What the bone that was hit takes, against the torso's share of 1.
const HIT_BONE_SHARE: f32 = 2.5;

/// A blast has no clip behind it, so its bodies get a flat run.
const BLAST_RAGDOLL_SECONDS: f32 = 6.0;

const BONE_DENSITY: f32 = 900.0;
const MIN_BONE_LENGTH: f32 = 0.06;
/// Both coefficients: a corpse should stop where it lands.
const CORPSE_FRICTION: f32 = 0.9;
const LEAF_BONE_LENGTH: f32 = 0.12;

const SWING_LIMIT_SPINE: f32 = 0.5;
const SWING_LIMIT_SHOULDER: f32 = 1.6;
const SWING_LIMIT_ELBOW: f32 = 1.3;
const SWING_LIMIT_HIP: f32 = 1.2;
const SWING_LIMIT_KNEE: f32 = 1.3;
const TWIST_LIMIT: f32 = 0.4;

/// How long a body keeps the pose it died in before it is only weight.
///
/// A ragdoll handed nothing but gravity folds in one frame and reads as a
/// sack. A real body stops holding itself over about half a second, and that
/// slide from held to limp is most of what makes a death look like one.
const MUSCLE_SECONDS: f32 = 0.55;
/// Spring and damper on each joint, as torque per radian and per rad/s.
/// Tuned together: too little damping and the limb rings, too much and it is
/// stiff all the way down rather than fading.
const MUSCLE_STIFFNESS: f32 = 18.0;
const MUSCLE_DAMPING: f32 = 2.2;
/// A body thrown by a blast is already past arguing with.
const MUSCLE_MAX_TORQUE: f32 = 40.0;

/// How far the torso has to tip before the body stops trying to save itself.
/// Past this it is committed and the legs may as well let go.
const FALL_GIVEN_UP: f32 = 1.1;

/// What staying on its feet is worth, on the joints that carry the body.
/// The arms get none of it: they are along for the ride either way, and a
/// shoulder stiffening to keep a torso up reads as a mannequin.
const BRACE_GAIN: f32 = 1.6;

/// How far the arms throw out when the balance goes, and what the throw is
/// worth in tone. Both ways out of a fall come through here: while there is
/// any balance left the arms windmill against the tip, and once there is
/// none they reach for the ground instead.
const WINDMILL_RADIANS: f32 = 0.9;
const BRACE_RADIANS: f32 = 1.25;
const ARM_URGENCY: f32 = 2.6;
/// Below this there is nothing left to save and the arms stop fighting the
/// fall and start expecting the floor.
const BALANCE_LOST: f32 = 0.18;
/// How fast the body has to be coming down before the arms go out to meet
/// the floor. A body sliding off a step is not falling.
const BRACING_FALL_SPEED: f32 = 1.4;
/// What a joint keeps while a behaviour is asking for it, whatever is left
/// of the body otherwise.
const REFLEX_FLOOR: f32 = 0.5;
const TWIST_LIMIT_HINGE: f32 = 0.08;

#[derive(Component)]
pub(crate) struct RagdollArming {
    captured: Vec<(Entity, Vec3, Quat)>,
    lifetime: f32,
}

impl RagdollArming {
    /// No captured motion: used when a blast takes over, where the throw
    /// dwarfs whatever the body was already doing.
    pub(crate) fn empty() -> Self {
        Self { captured: Vec::new(), lifetime: BLAST_RAGDOLL_SECONDS }
    }
}

#[derive(Component)]
pub(crate) struct Ragdolling {
    bodies: Vec<(Entity, Entity, Quat)>,
    settled_for: f32,
    elapsed: f32,
    lifetime: f32,
}

impl Ragdolling {
    pub(crate) fn bodies(&self) -> &[(Entity, Entity, Quat)] {
        &self.bodies
    }
}

#[derive(Component)]
pub(crate) struct RagdollBody;

#[derive(Clone, Copy)]
enum JointKind {
    Spine,
    Shoulder,
    Elbow,
    Hip,
    Knee,
}

impl JointKind {
    /// What holds a body up is not evenly spread: the spine and hips carry
    /// it, the knees lock, and the arms are along for the ride. Flat tone
    /// everywhere is the other way to read as a sack -- an evenly stiff one.
    /// What a joint is worth spending on not falling over. The arms get
    /// nothing: they are along for the ride either way, and a shoulder
    /// stiffening to hold a torso up reads as a mannequin.
    fn carry(self) -> f32 {
        match self {
            Self::Spine => 1.0,
            Self::Hip => 1.0,
            Self::Knee => 0.8,
            Self::Shoulder | Self::Elbow => 0.0,
        }
    }

    fn tone(self) -> f32 {
        match self {
            Self::Spine => 1.0,
            Self::Hip => 0.9,
            Self::Knee => 0.7,
            Self::Shoulder => 0.35,
            Self::Elbow => 0.25,
        }
    }
}

/// What a bone is still trying to hold.
///
/// Not a pose but a chase: `ghost` is where the clip still running on the
/// skeleton says this bone ought to be, rewritten every frame. A body shot
/// mid-stride goes on trying to take the step while gravity takes it, which
/// is the whole difference between a corpse and a sack.
#[derive(Component)]
pub(crate) struct Muscle {
    parent: Entity,
    /// The child's rotation in the parent's frame at the moment physics took
    /// over -- the pose the animation left it in, and what the chase starts
    /// from.
    rest: Quat,
    /// Where the clip wants it now. Starts at `rest` and walks away from it
    /// by however far the animation has moved since.
    ghost: Quat,
    /// The bone's own local rotation the first frame the ghost was read,
    /// which is what the walk away from `rest` is measured against.
    anchor: Option<Quat>,
    /// How much of the body is still trying to stay up: one while it is on
    /// its feet, nothing once the fall is past saving. Written beside the
    /// ghost because the same pass over the corpse knows both.
    fighting: f32,
    /// How much this joint is worth spending on staying up, from its kind.
    carry: f32,
    /// Whether this is an arm, and so whether the behaviours may take it
    /// over. Nothing takes over a leg: the legs are the fall.
    arm: bool,
    /// What a behaviour is asking of this joint on top of its own tone.
    /// One when nothing is.
    urgency: f32,
    tone: f32,
    age: f32,
}

struct BoneLink {
    bone: Entity,
    parent_slot: Option<usize>,
    joint: JointKind,
    radius: f32,
}

fn bone_links(bones: &ZombieHitboxBones, severed: Option<&SeveredParts>) -> Option<Vec<BoneLink>> {
    let radius_torso = TORSO_HITBOX_RADIUS;
    let radius_limb = LIMB_HITBOX_RADIUS;
    let radius_head = HEAD_HITBOX_RADIUS;

    let mut links = Vec::with_capacity(15);
    let mut push = |bone: Option<Entity>, parent_slot, joint, radius| -> Option<usize> {
        let bone = bone?;
        if is_severed(severed, bones, bone) {
            return None;
        }
        links.push(BoneLink { bone, parent_slot, joint, radius });
        Some(links.len() - 1)
    };

    let belly = push(bones.belly, None, JointKind::Spine, radius_torso)?;
    let chest = push(bones.chest, Some(belly), JointKind::Spine, radius_torso)?;
    push(bones.head, Some(chest), JointKind::Spine, radius_head);

    for arm in [&bones.left_arm, &bones.right_arm] {
        let mut parent = chest;
        for (depth, joint) in [JointKind::Shoulder, JointKind::Elbow, JointKind::Elbow].into_iter().enumerate() {
            let Some(slot) = push(arm[depth], Some(parent), joint, radius_limb) else { break };
            parent = slot;
        }
    }
    for leg in [&bones.left_leg, &bones.right_leg] {
        let mut parent = belly;
        for (depth, joint) in [JointKind::Hip, JointKind::Knee, JointKind::Knee].into_iter().enumerate() {
            let Some(slot) = push(leg[depth], Some(parent), joint, radius_limb) else { break };
            parent = slot;
        }
    }
    Some(links)
}

/// Captures where every bone is one frame before physics takes over, so the
/// next frame can work out how fast each was already moving. Handing a ragdoll
/// the pose but not the motion is what makes a body stop dead and then drop.
pub(crate) fn arm_ragdoll(
    mut commands: Commands,
    globals: Query<&GlobalTransform>,
    arming: Query<(Entity, &ZombieHitboxBones, Option<&SeveredParts>), (With<DeathClipPlaying>, Without<RagdollArming>)>,
    ready: Query<(Entity, &ReadyToRagdoll)>,
) {
    for (entity, ready_for) in &ready {
        let Ok((_, bones, severed)) = arming.get(entity) else { continue };
        let Some(links) = bone_links(bones, severed) else { continue };
        let captured = links
            .iter()
            .filter_map(|link| {
                let global = globals.get(link.bone).ok()?;
                Some((link.bone, global.translation(), global.rotation()))
            })
            .collect();
        commands
            .entity(entity)
            .remove::<ReadyToRagdoll>()
            .insert(RagdollArming { captured, lifetime: ready_for.0 });
    }
}

#[derive(Component)]
pub(crate) struct ReadyToRagdoll(pub(crate) f32);

#[derive(Component)]
pub(crate) struct DeathClipPlaying;

/// Runs before `arm_ragdoll` on purpose. Arming records bone transforms and
/// this consumes them, so putting it first leaves exactly one frame between
/// the two — which is where the per-bone velocity comes from. Run them the
/// other way round and every capture is same-frame, every velocity is zero,
/// and the corpse stops dead before it falls.
pub(crate) fn spawn_what_was_armed_last_frame(
    time: Res<Time>,
    mut commands: Commands,
    globals: Query<&GlobalTransform>,
    arming: Query<(
        Entity,
        &ZombieHitboxBones,
        &RagdollArming,
        Option<&DeathImpulse>,
        Option<&ZombieAnimationPlayer>,
        Option<&SeveredParts>,
    )>,
    mut players: Query<&mut AnimationPlayer>,
) {
    let dt = time.delta_secs().max(1e-4);
    for (entity, bones, armed, impulse, link, severed) in &arming {
        // The clip has to stop here. Left running it keeps writing bone
        // rotations every frame and simply overwrites whatever physics does.
        if let Some(link) = link {
            if let Ok(mut player) = players.get_mut(link.0) {
                player.stop_all();
            }
        }

        let Some(links) = bone_links(bones, severed) else {
            commands.entity(entity).remove::<RagdollArming>();
            continue;
        };

        // Which bone the round went into, by distance. Done once here rather
        // than per bone, and only when there is a point to measure against.
        let hit_slot = impulse.filter(|i| i.at != Vec3::ZERO).and_then(|i| {
            links
                .iter()
                .enumerate()
                .filter_map(|(slot, link)| {
                    globals.get(link.bone).ok().map(|g| (slot, g.translation().distance_squared(i.at)))
                })
                .min_by(|(_, a), (_, b)| a.total_cmp(b))
                .map(|(slot, _)| slot)
        });

        let mut bodies: Vec<Option<Entity>> = vec![None; links.len()];
        let mut facings: Vec<Quat> = vec![Quat::IDENTITY; links.len()];
        let mut pairs = Vec::with_capacity(links.len());

        for (slot, link) in links.iter().enumerate() {
            let Ok(global) = globals.get(link.bone) else { continue };
            let position = global.translation();
            let rotation = global.rotation();

            let tail = links
                .iter()
                .find(|child| child.parent_slot == Some(slot))
                .and_then(|child| globals.get(child.bone).ok())
                .map(|child| child.translation())
                .unwrap_or_else(|| position + rotation * Vec3::Y * LEAF_BONE_LENGTH);
            let span = tail - position;
            let length = span.length().max(MIN_BONE_LENGTH);
            let along = span.try_normalize().unwrap_or(rotation * Vec3::Y);

            // The capsule spans the bone rather than straddling its head, and
            // its local Y runs down the limb. That puts the mass where the limb
            // is, and makes the joint twist axis the limb axis, so a twist limit
            // actually limits twist instead of some arbitrary bone axis.
            let centre = position + along * (length * 0.5);
            let facing = Quat::from_rotation_arc(Vec3::Y, along);
            let bone_from_body = facing.inverse() * rotation;

            let (velocity, spin) = armed
                .captured
                .iter()
                .find(|(bone, _, _)| *bone == link.bone)
                .map(|&(_, was_at, was_facing)| {
                    let turn = rotation * was_facing.inverse();
                    let (axis, angle) = turn.to_axis_angle();
                    ((position - was_at) / dt, axis * (angle / dt))
                })
                .unwrap_or((Vec3::ZERO, Vec3::ZERO));

            // Avian 0.7 has no impulse component, so the shot rides in as extra
            // starting velocity: on the torso, to carry the whole body, and
            // again on the bone it actually struck.
            let shove = impulse.map(|i| i.push).unwrap_or(Vec3::ZERO);
            let struck = impulse.is_some_and(|i| {
                hit_slot.is_some_and(|hit| hit == slot) && i.at != Vec3::ZERO
            });
            let velocity = match (slot == 0, struck) {
                (true, true) => velocity + shove * (1.0 + HIT_BONE_SHARE),
                (true, false) => velocity + shove,
                (false, true) => velocity + shove * HIT_BONE_SHARE,
                (false, false) => velocity,
            };

            let body = commands
                .spawn((
                    Name::new("RagdollBone"),
                    RigidBody::Dynamic,
                    Collider::capsule(link.radius, length),
                    ColliderDensity(BONE_DENSITY),
                    // Flesh on stone, not ice on glass. The default 0.5 let a
                    // body keep sliding after it had stopped falling, which
                    // reads as the corpse being shoved by nothing.
                    Friction::new(CORPSE_FRICTION),
                    // Off the layer the player's capsule cast reads, so a body
                    // settling underfoot stops shoving whoever dropped it.
                    // Still filtered against Corpse, because JointCollisionDisabled
                    // only covers bones that share a joint -- an arm and the
                    // opposite leg keep each other out, and that is what stops
                    // the corpse folding through itself.
                    CollisionLayers::new(GameLayer::Corpse, [GameLayer::Default, GameLayer::Corpse]),
                    Transform::from_translation(centre).with_rotation(facing),
                    LinearVelocity(velocity),
                    AngularVelocity(spin),
                    ConstantTorque::default(),
                    RagdollBody,
                ))
                .id();
            bodies[slot] = Some(body);
            facings[slot] = facing;
            pairs.push((link.bone, body, bone_from_body));
        }

        for (slot, link) in links.iter().enumerate() {
            let (Some(body), Some(parent_slot)) = (bodies[slot], link.parent_slot) else { continue };
            let Some(parent_body) = bodies[parent_slot] else { continue };
            let Ok(pivot) = globals.get(link.bone).map(|g| g.translation()) else { continue };
            commands.spawn((joint_for(link.joint, parent_body, body, pivot), JointCollisionDisabled));
            // Against the parent, not against the world: a muscle holding a
            // world rotation would hold the whole corpse up off the floor.
            commands.entity(body).insert(Muscle {
                parent: parent_body,
                rest: facings[parent_slot].inverse() * facings[slot],
                ghost: facings[parent_slot].inverse() * facings[slot],
                anchor: None,
                fighting: 0.0,
                carry: link.joint.carry(),
                arm: matches!(link.joint, JointKind::Shoulder | JointKind::Elbow),
                urgency: 1.0,
                tone: link.joint.tone(),
                age: 0.0,
            });
        }


        commands
            .entity(entity)
            .remove::<RagdollArming>()
            .remove::<DeathClipPlaying>()
            .insert(Ragdolling { bodies: pairs, settled_for: 0.0, elapsed: 0.0, lifetime: armed.lifetime });
    }
}


/// Reads the clip that is still running and tells each muscle where it wants
/// its bone, and how much of the body is still trying to stay upright.
///
/// Both in one pass because one pass over the corpse knows both, and because
/// each needs the skeleton as the animation left it -- before
/// `drive_skeleton_from_ragdoll` writes the simulated pose over the top.
///
/// The target is carried as a delta rather than rebuilt from the hierarchy:
/// a bone's parent in the skeleton is not always the bone the next joint
/// hangs off, so anything derived from the chain would be right on the spine
/// and wrong on a shoulder. How far the clip has turned this bone in its own
/// frame is true whatever sits between them.
pub(crate) fn track_muscle_targets(
    corpses: Query<&Ragdolling>,
    globals: Query<&GlobalTransform>,
    speeds: Query<&LinearVelocity, With<RagdollBody>>,
    locals: Query<&Transform, Without<RagdollBody>>,
    mut muscles: Query<&mut Muscle>,
) {
    for ragdoll in &corpses {
        let Some(&(_, root, _)) = ragdoll.bodies.first() else { continue };
        let Ok(root_global) = globals.get(root) else { continue };
        let up = root_global.rotation() * Vec3::Y;
        // How far past saving the fall is: one on its feet, nothing on its
        // side. And which way it is going over, which is what the arms
        // answer -- the horizontal part of where the torso is pointing.
        let tilt = up.angle_between(Vec3::Y);
        let fighting = 1.0 - (tilt / FALL_GIVEN_UP).clamp(0.0, 1.0);
        let toppling = Vec3::new(up.x, 0.0, up.z).normalize_or_zero();
        let dropping = speeds.get(root).map(|v| -v.0.y).unwrap_or(0.0);

        // One mechanism, two ways out of a fall. While there is balance left
        // the arms throw out against the tip, which is what a body does
        // before it believes it is going down. Once there is none they turn
        // over and reach for the floor instead.
        let saving = fighting > BALANCE_LOST;
        // The same axis both ways -- across the way it is going over. Only
        // the sign differs, which is the difference between throwing the
        // arms back against the tip and putting them out in front of it.
        let arm_axis = toppling.cross(Vec3::Y).normalize_or_zero();
        let arm_angle = if saving {
            WINDMILL_RADIANS * fighting
        } else if dropping > BRACING_FALL_SPEED {
            -BRACE_RADIANS
        } else {
            0.0
        };

        for &(bone, body, bone_from_body) in &ragdoll.bodies {
            let Ok(local) = locals.get(bone) else { continue };
            let parent = muscles.get(body).map(|m| m.parent).ok();
            let into_parent = parent
                .and_then(|p| globals.get(p).ok())
                .map(|g| g.rotation().inverse())
                .unwrap_or(Quat::IDENTITY);
            let Ok(mut muscle) = muscles.get_mut(body) else { continue };
            let anchor = *muscle.anchor.get_or_insert(local.rotation);
            let moved = anchor.inverse() * local.rotation;
            let mut ghost = muscle.rest * (bone_from_body * moved * bone_from_body.inverse());
            if muscle.arm && arm_angle != 0.0 && arm_axis != Vec3::ZERO {
                ghost = Quat::from_axis_angle(into_parent * arm_axis, arm_angle) * ghost;
            }
            muscle.ghost = ghost;
            muscle.fighting = fighting;
            muscle.urgency = if muscle.arm && arm_angle != 0.0 { ARM_URGENCY } else { 1.0 };
        }
    }
}
/// Holds each bone at the pose it died in, with the grip fading to nothing.
///
/// A spring and damper in joint space, written as torque rather than as a
/// joint motor: Avian's `AngularMotor` only fits a revolute joint, and every
/// joint in a body but the knees and elbows is a ball.
///
/// The damper reads the child's spin relative to its parent, not its spin in
/// the world. A corpse tumbling down a slope is turning as one piece, and a
/// world-space damper would fight the tumble instead of the limb.
pub(crate) fn drive_muscles(
    time: Res<Time>,
    enabled: Res<RagdollMuscles>,
    mut bodies: Query<(&mut Muscle, &Rotation, &AngularVelocity, &mut ConstantTorque)>,
    parents: Query<(&Rotation, &AngularVelocity), With<RagdollBody>>,
) {
    let dt = time.delta_secs();
    for (mut muscle, rotation, spin, mut torque) in &mut bodies {
        muscle.age += dt;
        let left = 1.0 - (muscle.age / MUSCLE_SECONDS).clamp(0.0, 1.0);
        // Squared, so the grip goes soft early and the last of it drains
        // slowly -- a linear fade reads as a switch being turned down.
        // Two terms, not one. The fade is what is left of the body; the brace
        // is what it is spending on not falling, and it drains with the
        // first. A body shot on its feet stiffens before it lets go; one
        // already on its side never does.
        let brace = 1.0 + BRACE_GAIN * muscle.fighting * muscle.carry;
        let holding = muscle.tone * left * left * brace;
        // A reflex outlives the grip. An arm put out to meet the floor has to
        // still be out when the floor arrives, and the floor arrives after
        // the body has finished going limp. It ends on its own: the brace is
        // only asked for while the body is still coming down.
        let reflex = if muscle.urgency > 1.0 {
            muscle.tone * muscle.urgency * REFLEX_FLOOR
        } else {
            0.0
        };
        let tone = holding.max(reflex) * f32::from(enabled.0);
        if tone <= 0.0 {
            torque.0 = Vec3::ZERO;
            continue;
        }
        let Ok((parent_rotation, parent_spin)) = parents.get(muscle.parent) else {
            torque.0 = Vec3::ZERO;
            continue;
        };
        let wanted = parent_rotation.0 * muscle.ghost;
        let error = wanted * rotation.0.inverse();
        // The short way round: a quaternion and its negative are the same
        // rotation, and taking the long one would wrench the limb.
        let error = if error.w < 0.0 { -error } else { error };
        let (axis, angle) = error.to_axis_angle();
        let push = axis * angle * MUSCLE_STIFFNESS - (spin.0 - parent_spin.0) * MUSCLE_DAMPING;
        torque.0 = (push * tone).clamp_length_max(MUSCLE_MAX_TORQUE);
    }
}

/// Off with `NO_MUSCLES=1`, so the two can be watched side by side.
#[derive(Resource)]
pub(crate) struct RagdollMuscles(pub(crate) bool);

impl Default for RagdollMuscles {
    fn default() -> Self {
        Self(std::env::var("NO_MUSCLES").is_err())
    }
}

fn joint_for(kind: JointKind, parent: Entity, child: Entity, pivot: Vec3) -> SphericalJoint {
    let (swing, twist) = match kind {
        JointKind::Spine => (SWING_LIMIT_SPINE, TWIST_LIMIT),
        JointKind::Shoulder => (SWING_LIMIT_SHOULDER, TWIST_LIMIT),
        JointKind::Elbow => (SWING_LIMIT_ELBOW, TWIST_LIMIT_HINGE),
        JointKind::Hip => (SWING_LIMIT_HIP, TWIST_LIMIT),
        JointKind::Knee => (SWING_LIMIT_KNEE, TWIST_LIMIT_HINGE),
    };
    // Both bodies anchor on the joint they share. Left at the default both
    // anchors sit at their own origin, so the solver drags the two bone
    // centres onto one point and the skeleton tears itself apart.
    let shared = JointFrame { anchor: JointAnchor::FromGlobal(pivot), basis: JointBasis::IDENTITY };
    SphericalJoint {
        frame1: shared,
        frame2: shared,
        swing_limit: Some(AngleLimit { min: -swing, max: swing }),
        twist_limit: Some(AngleLimit { min: -twist, max: twist }),
        ..SphericalJoint::new(parent, child)
    }
}

/// Writes the simulated pose back onto the skeleton. Only rotation is copied:
/// bone positions come from the hierarchy, so copying those too would fight it
/// and let the body stretch.
pub(crate) fn drive_skeleton_from_ragdoll(
    time: Res<Time>,
    mut commands: Commands,
    mut corpses: Query<(Entity, &mut Ragdolling, &mut Transform), Without<RagdollBody>>,
    child_of: Query<&ChildOf>,
    globals: Query<&GlobalTransform>,
    speeds: Query<(&LinearVelocity, &AngularVelocity), With<RagdollBody>>,
    mut bones: Query<&mut Transform, (Without<Ragdolling>, Without<RagdollBody>)>,
) {
    for (entity, mut ragdoll, mut root) in &mut corpses {
        ragdoll.elapsed += time.delta_secs();

        let mut fastest: f32 = 0.0;
        for &(bone, body, bone_from_body) in &ragdoll.bodies {
            if let Ok((linear, angular)) = speeds.get(body) {
                fastest = fastest.max(linear.0.length()).max(angular.0.length() * 0.2);
            }
            let (Ok(body_global), Ok(parent)) = (globals.get(body), child_of.get(bone)) else {
                continue;
            };
            let Ok(parent_global) = globals.get(parent.0) else { continue };
            let Ok(mut transform) = bones.get_mut(bone) else { continue };
            transform.rotation =
                parent_global.rotation().inverse() * body_global.rotation() * bone_from_body;
        }

        if death_logging() {
            info!(
                "[corpse] id={entity:?} t={:.2}s root_y={:.3} settled_for={:.2}",
                ragdoll.elapsed, root.translation.y, ragdoll.settled_for
            );
        }
        if let Some(&(_, root_body, _)) = ragdoll.bodies.first() {
            if let Ok(global) = globals.get(root_body) {
                root.translation = global.translation();
            }
        }

        ragdoll.settled_for = if fastest < SETTLE_SPEED {
            ragdoll.settled_for + time.delta_secs()
        } else {
            0.0
        };

        // Runs its whole term instead of stopping as soon as it comes to rest:
        // the body is meant to be watched, and a late blast should still be
        // able to move it. Avian sleeps bodies that stop, so the cost of
        // leaving them up is small.
        if ragdoll.elapsed >= ragdoll.lifetime {
            for &(_, body, _) in &ragdoll.bodies {
                commands.entity(body).despawn();
            }
            commands.entity(entity).remove::<Ragdolling>().insert(CorpseSettled);
        }
    }
}

/// Retires the oldest ragdolls once there are too many of them.
///
/// By cutting their term short rather than tearing them down here: the
/// handover to a settled corpse already exists at the end of
/// `drive_skeleton_from_ragdoll`, and two places that despawn bodies is one
/// too many.
///
/// Oldest rather than furthest away, which would look better and needs the
/// camera. The oldest is also the one that has stopped moving, so the freeze
/// is hardest to notice there.
pub(crate) fn cap_active_ragdolls(mut ragdolls: Query<(Entity, &mut Ragdolling)>) {
    let over = ragdolls.iter().count().saturating_sub(MAX_ACTIVE_RAGDOLLS);
    if over == 0 {
        return;
    }
    let mut by_age: Vec<(Entity, f32)> = ragdolls.iter().map(|(e, r)| (e, r.elapsed)).collect();
    by_age.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (entity, age) in by_age.into_iter().take(over) {
        if let Ok((_, mut ragdoll)) = ragdolls.get_mut(entity) {
            ragdoll.lifetime = ragdoll.elapsed;
            if death_logging() {
                info!("[corpse] id={entity:?} frozen early at {age:.2}s, {over} over the cap");
            }
        }
    }
}

#[derive(Component)]
pub(crate) struct CorpseSettled;

#[derive(Component)]
pub(crate) struct DeathImpulse {
    pub(crate) push: Vec3,
    /// Where the round went in. The bone nearest it takes the harder share,
    /// which is what turns a shoulder hit into a spin and a leg hit into a
    /// buckle instead of the same shove every time.
    pub(crate) at: Vec3,
}

