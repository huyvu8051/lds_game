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
pub(crate) fn hold_subject(
    mut bench: ResMut<RagdollBench>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
    mut zombies: Query<(Entity, &mut Transform), With<Zombie>>,
) {
    let Ok(eye) = camera.single() else { return };
    let stand = eye.transform_point(STAND_AT);
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
            let toward = eye.translation() - transform.translation;
            transform.look_to(toward, Vec3::Y);
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
pub(crate) fn quiet_the_gore(mut blood: ResMut<crate::blood::BloodEnabled>) {
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
    let across = hitboxes
        .belly
        .and_then(|b| globals.get(b).ok())
        .map(|g| g.rotation() * Vec3::X)
        .unwrap_or(Vec3::X);

    // Signed by the way the joint turns, not by the way the limb ends up
    // pointing. The first version read the latter, and a running stride --
    // one leg forward, one back, both knees flexed the same way -- came out
    // with opposite signs on the two knees.
    let bend = |from: Vec3, to: Vec3| -> f32 {
        let turn = from.cross(to).dot(across);
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
            bend(-up, thigh),
            bend(thigh, shin),
        ));
    }
    for (tag, limb) in [("L", &hitboxes.left_arm), ("R", &hitboxes.right_arm)] {
        let (Some(shoulder), Some(elbow), Some(hand)) = (at(limb[0]), at(limb[1]), at(limb[2]))
        else {
            continue;
        };
        let upper = (elbow - shoulder).normalize_or_zero();
        let fore = (hand - elbow).normalize_or_zero();
        parts.push(format!("elb{tag}={:+4.0}", bend(upper, fore)));
    }
    info!("[bones] t={:.2} {}", bench.elapsed - shot_at(), parts.join(" "));
}
