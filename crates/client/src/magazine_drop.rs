use avian3d::prelude::*;
use bevy::camera::primitives::Aabb;
use bevy::math::ops;
use bevy::prelude::*;

use crate::*;

/// Fractions of the reload at which the magazine changes hands.
///
/// The old one comes off into the hand, is let go partway to the pouch, and a
/// fresh one is already in the hand on the way back. It seats well before the
/// reload timer runs out on purpose: the gun is level again by then, and an
/// empty well that late reads as a missing part rather than a reload.
const MAG_TAKEN_AT: f32 = RELOAD_HAND_AT_MAG;
const MAG_RELEASED_AT: f32 = 0.33;
const MAG_FETCHED_AT: f32 = 0.56;
const MAG_SEATED_AT: f32 = RELOAD_HAND_BACK_AT_MAG;

const MAG_LIFETIME: f32 = 20.0;
const MAG_DROP_SPEED: f32 = 0.8;
const MAG_DROP_FORWARD: f32 = 0.2;
const MAG_SPIN: f32 = 6.0;
const MAG_DENSITY: f32 = 300.0;
const MAX_DROPPED_MAGS: usize = 6;

#[derive(Component)]
pub(crate) struct ViewmodelMagazine;

/// Where the left hand is told to be while reloading, instead of on the
/// weapon's fore grip.
///
/// A separate entity rather than moving Socket_FrontGrip: that socket belongs
/// to the mesh, is rebuilt on every weapon swap, and is what the hand must
/// return to. The solver takes whichever entity it is handed, so swapping the
/// target is the whole of the change.
#[derive(Component)]
pub(crate) struct ReloadHandTarget;

/// Fractions of the reload at which the hand reaches each point. The magazine
/// leaves when the hand is on it, not when the reload starts.
const RELOAD_HAND_AT_MAG: f32 = 0.18;
const RELOAD_HAND_AT_POUCH: f32 = 0.50;
const RELOAD_HAND_BACK_AT_MAG: f32 = 0.74;
/// Off the eye, low and on the weak-hand side: where a spare magazine lives.
const RELOAD_POUCH_FROM_EYE: Vec3 = Vec3::new(-0.16, -0.58, -0.26);

/// Grows a world-space box to take in one more part of the magazine.
///
/// The part's own node origin is no use for this. A magazine welded out of
/// loose shards by the asset pipeline keeps whatever origin it was joined at,
/// and for the SLR that is the muzzle -- six hundred and ninety millimetres
/// from the magazine, and where the hand was being sent. The mesh bounds say
/// where the thing actually is.
fn grow_to_part(span: &mut Option<(Vec3, Vec3)>, global: &GlobalTransform, aabb: &Aabb) {
    let centre = global.transform_point(Vec3::from(aabb.center));
    let half = (global.affine().matrix3 * Vec3::from(aabb.half_extents)).abs();
    *span = Some(match *span {
        Some((lo, hi)) => (lo.min(centre - half), hi.max(centre + half)),
        None => (centre - half, centre + half),
    });
}

fn eased(from: Vec3, to: Vec3, t: f32) -> Vec3 {
    let t = t.clamp(0.0, 1.0);
    from.lerp(to, t * t * (3.0 - 2.0 * t))
}

/// Walks the left hand through a reload: off the fore grip, down to the
/// magazine, away to the pouch, back up with a fresh one, and home.
///
/// Authored as waypoints rather than a clip because the weapons carry no
/// skeleton at all -- every mesh reports skins=0 -- and because the hands are
/// already posed by IK, which a clip would fight over the same bones.
/// Where the free hand rides while sprinting with a one-handed weapon: down
/// and across, off the gun entirely. Relative to the eye, same frame as
/// RELOAD_POUCH_FROM_EYE.
const SPRINT_FREE_HAND_FROM_EYE: Vec3 = Vec3::new(-0.26, -0.40, -0.26);

pub(crate) fn drive_reload_hand(
    active: Res<ActiveWeapon>,
    gun: Query<&Gun>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
    fore: Query<&GlobalTransform, With<ForeGripSocket>>,
    magazine: Query<(&GlobalTransform, &Aabb), With<ViewmodelMagazine>>,
    mut target: Query<&mut Transform, With<ReloadHandTarget>>,
) {
    let (Ok(gun), Ok(eye), Ok(mut target), Ok(fore)) =
        (gun.single(), camera.single(), target.single_mut(), fore.single())
    else {
        return;
    };
    let home = fore.reparented_to(eye);
    // The magazine is only hidden while it is away, never despawned, so its
    // place on the weapon stays readable throughout.
    //
    // The box its parts fill, not the mean of their origins. A magazine can
    // be more than one part -- the UMP's shell and the rounds inside it, the
    // DP-28's pan in three -- and those parts do not share an origin either.
    let mut span = None;
    for (global, aabb) in &magazine {
        grow_to_part(&mut span, global, aabb);
    }
    let to_eye = eye.affine().inverse();
    let magwell = span
        .map(|(lo, hi)| to_eye.transform_point3((lo + hi) * 0.5))
        .unwrap_or(home.translation);
    if magazine_log_enabled() && gun.reload_remaining > 0.0 {
        debug!(
            "[magazine] hand sent to {:.0},{:.0},{:.0}mm from the eye, grip is at {:.0},{:.0},{:.0}",
            magwell.x * 1000.0, magwell.y * 1000.0, magwell.z * 1000.0,
            home.translation.x * 1000.0, home.translation.y * 1000.0, home.translation.z * 1000.0,
        );
    }
    let pouch = RELOAD_POUCH_FROM_EYE - Vec3::Z * view_pullback();

    let progress = 1.0 - (gun.reload_remaining / active.def().reload_duration).clamp(0.0, 1.0);
    let span = |from: f32, to: f32| (progress - from) / (to - from);
    target.translation = if progress < RELOAD_HAND_AT_MAG {
        eased(home.translation, magwell, span(0.0, RELOAD_HAND_AT_MAG))
    } else if progress < RELOAD_HAND_AT_POUCH {
        eased(magwell, pouch, span(RELOAD_HAND_AT_MAG, RELOAD_HAND_AT_POUCH))
    } else if progress < RELOAD_HAND_BACK_AT_MAG {
        eased(pouch, magwell, span(RELOAD_HAND_AT_POUCH, RELOAD_HAND_BACK_AT_MAG))
    } else {
        eased(magwell, home.translation, span(RELOAD_HAND_BACK_AT_MAG, 1.0))
    };
    // Running with a sidearm takes the free hand off the weapon altogether,
    // and it has to be somewhere: blended off the reload path rather than
    // replacing it, so a reload begun mid-sprint still wins.
    if active.def().sprint_one_handed && gun.reload_remaining <= 0.0 {
        let swing = SPRINT_FREE_HAND_FROM_EYE - Vec3::Z * view_pullback();
        target.translation = target.translation.lerp(swing, gun.sprint_blend);
    }
    // The grip's own turn throughout: the wrist has no business rolling just
    // because the hand went for a magazine.
    target.rotation = home.rotation;
}

#[derive(Component)]
pub(crate) struct DroppedMagazine {
    remaining: f32,
}

fn scatter(seed: f32) -> Vec3 {
    Vec3::new(ops::sin(seed * 1.7), ops::sin(seed * 2.3), ops::sin(seed * 3.1))
}

pub(crate) fn magazine_log_enabled() -> bool {
    std::env::var("MAG_LOG").map(|value| value == "1").unwrap_or(false)
}

/// A magazine in the hand, on its way out of the weapon or back into it.
#[derive(Component)]
pub(crate) struct CarriedMagazine;

/// How the magazine sat in the hand when it came off.
///
/// Kept because the fresh one has to be put back the same way, and by then
/// the hand is at the pouch while the weapon's own magazine parts are still
/// where they always were -- measuring it again at that moment would place
/// the magazine wherever the gun is rather than in the hand.
#[derive(Resource, Default)]
pub(crate) struct MagazineInHand {
    parts: Vec<(Transform, Handle<Mesh>, Handle<StandardMaterial>)>,
    /// Where the magazine's middle sits in the hand's own frame.
    centre: Vec3,
    half: Vec3,
}

fn spawn_carried(commands: &mut Commands, held: &MagazineInHand, hand: &GlobalTransform) {
    if held.parts.is_empty() {
        return;
    }
    let at = hand.mul_transform(Transform::from_translation(held.centre)).compute_transform();
    let body = commands
        .spawn((Name::new("CarriedMagazine"), at, Visibility::default(), CarriedMagazine))
        .id();
    for (part, mesh, material) in &held.parts {
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            *part,
            ChildOf(body),
        ));
    }
}

/// Walks the magazine through the reload in the hand that is carrying it.
///
/// One system for the whole journey, because every step is the same question
/// asked at a different fraction: the old one off the weapon, let go of, a
/// fresh one fetched, and seated. It replaces two that read the same clock
/// from opposite ends and had to agree about the middle of it.
pub(crate) fn carry_reload_magazine(
    time: Res<Time>,
    active: Res<ActiveWeapon>,
    mut commands: Commands,
    mut held: ResMut<MagazineInHand>,
    gun: Query<&Gun>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
    hand: Query<&GlobalTransform, With<ReloadHandTarget>>,
    mut magazines: Query<
        (&GlobalTransform, &Mesh3d, &MeshMaterial3d<StandardMaterial>, &Aabb, &mut Visibility),
        With<ViewmodelMagazine>,
    >,
    mut carried: Query<(Entity, &mut Transform), With<CarriedMagazine>>,
    mut last: Local<f32>,
) {
    let (Ok(gun), Ok(eye), Ok(hand)) = (gun.single(), camera.single(), hand.single()) else {
        return;
    };
    // Idle sits at one, so a reload beginning drops the progress rather than
    // raising it, and none of the marks below fire on the way down.
    let now = if gun.reload_remaining > 0.0 {
        1.0 - (gun.reload_remaining / active.def().reload_duration).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let was = std::mem::replace(&mut *last, now);
    let crossed = |mark: f32| was < mark && now >= mark;

    // In the hand for as long as it is out of the weapon.
    let in_hand =
        hand.mul_transform(Transform::from_translation(held.centre)).compute_transform();
    for (_, mut at) in &mut carried {
        *at = in_hand;
    }

    if crossed(MAG_TAKEN_AT) {
        let into_hand = hand.affine().inverse();
        let mut span = None;
        let mut taken = Vec::new();
        for (global, mesh, material, aabb, mut visibility) in &mut magazines {
            if *visibility == Visibility::Hidden {
                continue;
            }
            *visibility = Visibility::Hidden;
            grow_to_part(&mut span, global, aabb);
            taken.push((
                Transform::from_matrix((into_hand * global.affine()).into()),
                mesh.0.clone(),
                material.0.clone(),
            ));
        }
        let (Some((lo, hi)), false) = (span, taken.is_empty()) else { return };
        let centre = into_hand.transform_point3((lo + hi) * 0.5);
        // Measured against the hand, then moved onto the magazine's own
        // middle, so the body they hang off is centred where its collider is.
        for (part, _, _) in &mut taken {
            part.translation -= centre;
        }
        held.parts = taken;
        held.centre = centre;
        held.half = (hi - lo) * 0.5;
        spawn_carried(&mut commands, &held, hand);
        if magazine_log_enabled() {
            debug!(
                "[magazine] out of the well, {} part(s), middle {:.0}mm from the hand, {:.0}x{:.0}x{:.0}mm",
                held.parts.len(),
                held.centre.length() * 1000.0,
                held.half.x * 2000.0, held.half.y * 2000.0, held.half.z * 2000.0,
            );
        }
    }

    if crossed(MAG_RELEASED_AT) {
        // The same entity, let go of rather than swapped for another: it is
        // already where the hand is, which is the whole point of carrying it.
        let seed = time.elapsed_secs();
        let away = (Vec3::NEG_Y + *eye.forward() * MAG_DROP_FORWARD).normalize_or_zero();
        for (entity, _) in &carried {
            commands.entity(entity).remove::<CarriedMagazine>().insert((
                Name::new("DroppedMagazine"),
                RigidBody::Dynamic,
                Collider::cuboid(held.half.x * 2.0, held.half.y * 2.0, held.half.z * 2.0),
                ColliderDensity(MAG_DENSITY),
                // Map geometry is a trimesh with no thickness, and a magazine
                // crosses its own depth in one step once gravity has it.
                SweptCcd::default(),
                LinearVelocity(away * MAG_DROP_SPEED),
                AngularVelocity(scatter(seed * 1.9) * MAG_SPIN),
                DroppedMagazine { remaining: MAG_LIFETIME },
            ));
        }
    }

    if crossed(MAG_FETCHED_AT) {
        spawn_carried(&mut commands, &held, hand);
    }

    if crossed(MAG_SEATED_AT) {
        for (entity, _) in &carried {
            commands.entity(entity).despawn();
        }
        for (_, _, _, _, mut visibility) in &mut magazines {
            if *visibility == Visibility::Hidden {
                *visibility = Visibility::Inherited;
            }
        }
        if magazine_log_enabled() {
            debug!("[magazine] seated, {:.2}s left on the reload", gun.reload_remaining);
        }
    }
}

/// Dropped magazines are rigid bodies the solver keeps revisiting, so the
/// oldest are cut short once there are more than a firefight's worth.
pub(crate) fn update_dropped_magazines(
    time: Res<Time>,
    mut commands: Commands,
    mut magazines: Query<(Entity, &Transform, &mut DroppedMagazine)>,
    mut next_log: Local<f32>,
) {
    let mut over_budget = magazines.iter().len().saturating_sub(MAX_DROPPED_MAGS);
    let now = time.elapsed_secs();
    let logging = magazine_log_enabled() && now >= *next_log;
    if logging {
        *next_log = now + 0.5;
    }
    for (entity, transform, mut magazine) in &mut magazines {
        if over_budget > 0 && magazine.remaining > 1.0 {
            over_budget -= 1;
            magazine.remaining = 1.0;
        }
        if logging {
            info!(
                "[magazine] {entity:?} at {:.3?} age {:.1}s",
                transform.translation,
                MAG_LIFETIME - magazine.remaining
            );
        }
        magazine.remaining -= time.delta_secs();
        if magazine.remaining <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
