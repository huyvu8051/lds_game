//! A bench for watching one zombie fall over.
//!
//! `RAGDOLL_TEST=1` takes a zombie, stands it a few metres in front of the
//! player, holds it there, and shoots it on a timer. Paired with a capture
//! burst it writes a filmstrip of the fall, which is the only way anything
//! about a ragdoll gets checked without a second pair of hands: the autopilot
//! cannot kill a zombie, so every death in this game has been taken on trust.
//!
//!     RAGDOLL_TEST=1 MAP_ID=1 NO_AUTOPILOT=1 \
//!       CAPTURE_SCREENSHOT_TO=/tmp/fall.png CAPTURE_SCREENSHOT_AFTER=6 \
//!       CAPTURE_BURST=14 CAPTURE_EVERY=0.12 \
//!       cargo run -p client --bin native_desktop --release
//!
//! The hold is blunt -- the transform is written back every frame -- because
//! what is being watched is what happens *after* the shot, and a subject that
//! has walked out of frame by then has answered nothing. Its clip keeps
//! running underneath, which is wanted: the muscles chase that clip, so a
//! body shot mid-stride is the case worth seeing.

use bevy::prelude::*;

use crate::ragdoll::{flex_axis, JointKind};
use crate::*;

/// Where the subject stands, from the player's eye.
/// Left of the weapon on purpose: the viewmodel owns the right of the
/// screen and a subject stood in front of it falls behind a receiver.
const STAND_AT: Vec3 = Vec3::new(-0.95, 0.0, -3.2);
/// When the shot goes, unless `RAGDOLL_TEST_AT` says otherwise. Long enough
/// for the map to finish loading and the zombie to have been placed.
const SHOT_AT_SECONDS: f32 = 6.0;
/// A rifle round is worth about a zombie's eighth, so a burst is what it
/// takes. Capped so a subject that will not die does not get shot forever.
const MAX_SHOTS: u32 = 12;
const SHOT_GAP: f32 = 0.09;

pub(crate) fn enabled() -> bool {
    std::env::var("RAGDOLL_TEST").is_ok_and(|v| v != "0")
}

fn shot_at() -> f32 {
    std::env::var("RAGDOLL_TEST_AT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(SHOT_AT_SECONDS)
}

/// Which bone the shot is aimed at: `head`, `chest` or `leg`. The fall is a
/// different thing in each case, and that difference is most of what the
/// muscles and the balance term were written for.
fn aim_at() -> &'static str {
    match std::env::var("RAGDOLL_TEST_BONE").as_deref() {
        Ok("head") => "head",
        Ok("leg") => "leg",
        _ => "chest",
    }
}

#[derive(Resource, Default)]
pub(crate) struct RagdollBench {
    subject: Option<Entity>,
    /// Where every other zombie was first seen, so it can be left there.
    parked: bevy::platform::collections::HashMap<Entity, Vec3>,
    elapsed: f32,
    /// How many rounds have gone. One does not kill: a zombie carries eight
    /// and a rifle round is worth about one, so a bench that fired once
    /// filmed a man standing still being shot at.
    shots: u32,
    next_shot: f32,
}

/// Picks one zombie, stands it where the camera can see it, and parks the
/// rest where they were.
///
/// All of them, not just the subject: the others walk at the player and the
/// first run had one chewing the lens while the body being measured fell over
/// somewhere off screen. Parking them also keeps the player alive, which a
/// six-second wait otherwise does not.
const POSE_STAND_AT: Vec3 = Vec3::new(-1.35, 0.0, -4.6);

pub(crate) fn hold_subject(
    mut bench: ResMut<RagdollBench>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
    mut zombies: Query<(Entity, &mut Transform), With<Zombie>>,
) {
    let Ok(eye) = camera.single() else { return };
    // Further out and to the left when a joint is being held up for a look:
    // the whole body has to fit, side-on, clear of the viewmodel's half of
    // the screen.
    let stand = eye.transform_point(if posing() { POSE_STAND_AT } else { STAND_AT });
    // Chosen once and kept, even after it stops being a zombie. Re-picking
    // when the subject died put a fresh one on the same mark and shot that
    // instead, so the late frames of the film were a second man standing up
    // where the first had just fallen.
    if bench.subject.is_none() {
        bench.subject = zombies.iter().next().map(|(entity, _)| entity);
    }
    let subject = bench.subject;
    for (entity, mut transform) in &mut zombies {
        if Some(entity) == subject {
            // Held, not slowed: navigation would walk it off the mark between
            // frames and the shot would land on a different pose every run.
            transform.translation.x = stand.x;
            transform.translation.z = stand.z;
            if posing() {
                // In profile, where a knee closing forward and a knee
                // closing backward are different pictures.
                transform.look_to(eye.right(), Vec3::Y);
            } else {
                let toward = eye.translation() - transform.translation;
                transform.look_to(toward, Vec3::Y);
            }
            continue;
        }
        let parked = *bench.parked.entry(entity).or_insert(transform.translation);
        transform.translation.x = parked.x;
        transform.translation.z = parked.z;
    }
}

/// Shoots it down the same path a real round takes, as many rounds as it
/// takes, until it goes down.
pub(crate) fn fire_on_cue(
    time: Res<Time>,
    mut commands: Commands,
    mut bench: ResMut<RagdollBench>,
    active: Res<ActiveWeapon>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
    bones: Query<&ZombieHitboxBones>,
    globals: Query<&GlobalTransform>,
) {
    // Nothing is shot while a joint is held to its limit for a look.
    if posing() {
        return;
    }
    bench.elapsed += time.delta_secs();
    if bench.elapsed < shot_at() || bench.elapsed < bench.next_shot || bench.shots >= MAX_SHOTS {
        return;
    }
    let (Some(subject), Ok(eye)) = (bench.subject, camera.single()) else { return };
    let Ok(hitboxes) = bones.get(subject) else { return };
    let target = match aim_at() {
        "head" => hitboxes.head,
        "leg" => hitboxes.left_leg.iter().copied().flatten().next(),
        _ => hitboxes.chest,
    }
    .and_then(|bone| globals.get(bone).ok())
    .map(|g| g.translation());
    let Some(target) = target else { return };

    bench.shots += 1;
    bench.next_shot = bench.elapsed + SHOT_GAP;
    let origin = eye.translation();
    let direction = (target - origin).normalize_or_zero();
    // Through the ordinary path, so what is watched is what a player would
    // have seen: the same damage, the same impulse, the same bone.
    commands.trigger(shared::FireShot {
        origin,
        direction,
        damage_scale: active.def().damage_scale,
        range: active.def().class.range_damage(),
    });
    if bench.shots == 1 {
        info!("[bench] firing at the {} from t={:.1}s", aim_at(), bench.elapsed);
    }
}

/// Turns the blood off while the bench is running.
///
/// Not squeamishness: a chest emptied at three metres paints the whole body
/// red, and what is being looked at is where the limbs ended up.
pub(crate) fn quiet_the_gore(
    mut blood: ResMut<crate::blood::BloodEnabled>,
    mut gun: Query<&mut Visibility, With<crate::Gun>>,
    mut shown: ResMut<crate::viewmodel_body::PlayerModelVisible>,
) {
    // And the weapon with it. The viewmodel owns the bottom right of the
    // screen and a body that falls behind a receiver is a body nobody can
    // judge.
    //
    // Through the resource, not the component: the body's own system owns
    // that visibility and puts back anything written behind its back, which
    // is how an arm stayed across the legs for a whole run of screenshots.
    for mut visibility in &mut gun {
        *visibility = Visibility::Hidden;
    }
    if shown.0 {
        shown.0 = false;
    }
    if blood.spray || blood.stains {
        blood.spray = false;
        blood.stains = false;
    }
}

/// Prints the angle at every joint of the subject while it falls.
///
/// A picture of a corpse says it looks wrong; it does not say which joint
/// went the wrong way, and on a body that has landed in a heap the hip and
/// the knee are hard to tell apart by eye. These are signed, and the sign is
/// the whole point:
///
/// * `hip` positive is the thigh swinging forward, which is flexion. Negative
///   is the leg going out behind, which a hip does by about twenty degrees
///   and no more.
/// * `knee` positive is the heel coming up behind, which is flexion. Negative
///   is a knee bending backwards, which a knee does not do at all.
///
/// Same for the arms, where negative at the elbow is the forearm bending the
/// wrong way out of the joint.
#[derive(Resource, Default)]
pub(crate) struct BoneLog(f32);

pub(crate) fn log_joint_angles(
    mut next: ResMut<BoneLog>,
    bench: Res<RagdollBench>,
    bones: Query<&ZombieHitboxBones>,
    globals: Query<&GlobalTransform>,
) {
    if bench.shots == 0 || bench.elapsed < next.0 {
        return;
    }
    next.0 = bench.elapsed + 0.2;
    let Some(subject) = bench.subject else { return };
    let Ok(hitboxes) = bones.get(subject) else { return };
    let at = |bone: Option<Entity>| bone.and_then(|b| globals.get(b).ok()).map(|g| g.translation());
    let (Some(chest), Some(belly)) = (at(hitboxes.chest), at(hitboxes.belly)) else { return };
    // The body's own up and its left, so the signs mean the same thing
    // however the corpse is lying.
    let up = (chest - belly).try_normalize().unwrap_or(Vec3::Y);
    // Off the pelvis's own frame, not off where the hips have ended up. A
    // corpse tangles its legs, hip-to-hip reverses when they cross, and
    // every sign below flipped with it -- a knee read +79 and then -80 two
    // tenths later, which no hinge allows and no hinge did.
    // The pelvis bone's own X, which runs to the body's left the way the
    // ragdoll means it. Read off the bone rather than off the hips: a corpse
    // tangles its legs, hip-to-hip reverses when they cross, and every sign
    // below went with it.
    let across = hitboxes
        .belly
        .and_then(|b| globals.get(b).ok())
        .map(|g| g.rotation() * Vec3::X)
        .unwrap_or(Vec3::X);

    // Signed by the way the joint turns, not by the way the limb ends up
    // pointing. The first version read the latter, and a running stride --
    // one leg forward, one back, both knees flexed the same way -- came out
    // with opposite signs on the two knees.
    //
    // The axis comes from the ragdoll so that flexion is positive for every
    // joint here and a negative reading means what it says.
    let bend = |axis: Vec3, from: Vec3, to: Vec3| -> f32 {
        let turn = from.cross(to).dot(axis);
        from.angle_between(to).to_degrees() * if turn < 0.0 { -1.0 } else { 1.0 }
    };
    let mut parts = Vec::new();
    for (tag, limb) in [("L", &hitboxes.left_leg), ("R", &hitboxes.right_leg)] {
        let (Some(hip), Some(knee), Some(foot)) = (at(limb[0]), at(limb[1]), at(limb[2])) else {
            continue;
        };
        let thigh = (knee - hip).normalize_or_zero();
        let shin = (foot - knee).normalize_or_zero();
        parts.push(format!(
            "hip{tag}={:+4.0} knee{tag}={:+4.0}",
            bend(flex_axis(JointKind::Hip, across), -up, thigh),
            bend(flex_axis(JointKind::Knee, across), thigh, shin),
        ));
    }
    for (tag, limb) in [("L", &hitboxes.left_arm), ("R", &hitboxes.right_arm)] {
        let (Some(shoulder), Some(elbow), Some(hand)) = (at(limb[0]), at(limb[1]), at(limb[2]))
        else {
            continue;
        };
        let upper = (elbow - shoulder).normalize_or_zero();
        let fore = (hand - elbow).normalize_or_zero();
        parts.push(format!(
            "elb{tag}={:+4.0}",
            bend(flex_axis(JointKind::Elbow, across), upper, fore)
        ));
    }
    info!("[bones] t={:.2} {}", bench.elapsed - shot_at(), parts.join(" "));
}

/// Which joints to bend, and how far, for a still of the limit rather than a
/// film of the fall.
///
/// A fall measures the whole chain at once and tells you a number is wrong
/// without telling you which. Bending one joint to the angle it is allowed
/// and looking at it answers that in a single frame.
fn posed() -> Option<String> {
    std::env::var("RAGDOLL_POSE").ok().filter(|v| !v.is_empty())
}

/// Overrides the angle a pose bends to, in degrees. For asking where a mesh
/// stops surviving its own limit rather than only whether it does.
fn posed_degrees() -> Option<f32> {
    std::env::var("RAGDOLL_POSE_DEG").ok().and_then(|v| v.parse().ok())
}

pub(crate) fn posing() -> bool {
    posed().is_some()
}

/// Turns the named joints to the limit the ragdoll would give them, straight
/// on the skeleton. No bodies, no solver: this asks what the number looks
/// like, not whether physics can reach it.
pub(crate) fn hold_the_pose(
    bench: Res<RagdollBench>,
    bones: Query<&ZombieHitboxBones>,
    parents: Query<&ChildOf>,
    globals: Query<&GlobalTransform>,
    mut poses: Query<&mut Transform>,
) {
    let Some(pose) = posed() else { return };
    let Some(subject) = bench.subject else { return };
    let Ok(hitboxes) = bones.get(subject) else { return };
    let at = |bone: Option<Entity>| bone.and_then(|b| globals.get(b).ok()).map(|g| g.translation());
    let (Some(chest), Some(belly)) = (at(hitboxes.chest), at(hitboxes.belly)) else { return };
    let up = (chest - belly).try_normalize().unwrap_or(Vec3::Y);
    let Some(across) = hitboxes
        .left_leg[0]
        .zip(hitboxes.right_leg[0])
        .and_then(|(l, r)| globals.get(l).ok().zip(globals.get(r).ok()))
        .and_then(|(l, r)| (l.translation() - r.translation()).try_normalize())
    else {
        return;
    };
    let forward = across.cross(up).normalize_or_zero();

    // Every pose is one world axis and one angle, applied to a named set of
    // bones. The axes are the ragdoll's own: flexion about the body's left,
    // the swing out of that plane about its forward, twist along the limb.
    let limbs = [&hitboxes.left_arm, &hitboxes.right_arm, &hitboxes.left_leg, &hitboxes.right_leg];
    let arms = [&hitboxes.left_arm, &hitboxes.right_arm];
    let legs = [&hitboxes.left_leg, &hitboxes.right_leg];
    let knee = crate::ragdoll::flex_axis(JointKind::Knee, across);
    let elbow = crate::ragdoll::flex_axis(JointKind::Elbow, across);
    let turns: Vec<(Entity, Vec3, f32)> = match pose.as_str() {
        "default" => Vec::new(),
        "knee" => legs
            .iter()
            .filter_map(|l| l[1].map(|b| (b, knee, crate::ragdoll::FLEX_LIMIT_KNEE)))
            .collect(),
        "elbow" => arms
            .iter()
            .filter_map(|l| l[1].map(|b| (b, elbow, crate::ragdoll::FLEX_LIMIT_ELBOW)))
            .collect(),
        "hip" | "hip_back" => {
            let way = if pose == "hip" { -1.0 } else { 1.0 };
            legs.iter()
                .filter_map(|l| {
                    l[0].map(|b| (b, across, way * crate::ragdoll::SWING_LIMIT_HIP))
                })
                .collect()
        }
        "hip_out" => legs
            .iter()
            .enumerate()
            .filter_map(|(i, l)| {
                let way = if i == 0 { 1.0 } else { -1.0 };
                l[0].map(|b| (b, forward, way * crate::ragdoll::SWING_LIMIT_HIP))
            })
            .collect(),
        "shoulder" | "shoulder_back" => {
            let way = if pose == "shoulder" { -1.0 } else { 1.0 };
            arms.iter()
                .filter_map(|l| {
                    l[0].map(|b| (b, across, way * crate::ragdoll::SWING_LIMIT_SHOULDER))
                })
                .collect()
        }
        "shoulder_out" => arms
            .iter()
            .enumerate()
            .filter_map(|(i, l)| {
                let way = if i == 0 { 1.0 } else { -1.0 };
                l[0].map(|b| (b, forward, way * crate::ragdoll::SWING_LIMIT_SHOULDER))
            })
            .collect(),
        "spine" => hitboxes
            .chest
            .map(|b| vec![(b, across, -crate::ragdoll::SWING_LIMIT_SPINE)])
            .unwrap_or_default(),
        "twist" => limbs
            .iter()
            .filter_map(|l| l[0])
            .filter_map(|b| {
                globals.get(b).ok().map(|g| (b, g.rotation() * Vec3::Y, crate::ragdoll::TWIST_LIMIT))
            })
            .collect(),
        other => {
            warn!("[pose] no such pose: {other}");
            Vec::new()
        }
    };

    let forced = posed_degrees().map(f32::to_radians);
    for (bone, axis, angle) in turns {
        let angle = forced.map_or(angle, |a| a.copysign(angle));
        // The axis is in world, the rotation it has to join is in the
        // bone's parent. A bone turned about a world axis written straight
        // into a local transform goes wherever the parent happens to face.
        let Ok(parent) = parents.get(bone).map(|p| p.parent()) else { continue };
        let Ok(frame) = globals.get(parent).map(|g| g.rotation()) else { continue };
        let Ok(mut local) = poses.get_mut(bone) else { continue };
        local.rotation = Quat::from_axis_angle(frame.inverse() * axis, angle) * local.rotation;
    }
}
