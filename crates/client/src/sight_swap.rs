use bevy::prelude::*;

use crate::sight_mount::{RailFrontSocket, RailRearSocket, SightId};
use crate::{
    load_gltf_scene, view_pullback, FpsCamera, GunMeshPart, ReloadHandTarget, WorldAssetRoot,
};

/// Putting a sight on the rail and taking one off, as a gesture rather than
/// a change of state.
///
/// The firing hand never leaves the grip. The weapon turns to bring its rail
/// round to the eye -- a rail laid flat on top is edge-on from behind and
/// there is nothing to watch -- and the weak hand does the work, carrying the
/// sight the whole way rather than having it appear once the hand arrives.
///
/// Authored as waypoints, like the reload it borrows its hand target from:
/// these weapons carry no skeleton, and the hands are posed by IK that a clip
/// would fight over the same bones.
const SWAP_SECONDS: f32 = 1.05;

/// Rolled toward the weak side and yawed across, so the rail faces the eye,
/// and pulled back far enough to be inside focus.
const PRESENT_ROLL_DEG: f32 = -34.0;
const PRESENT_YAW_DEG: f32 = 15.0;
const PRESENT_PITCH_DEG: f32 = -6.0;
const PRESENT_PULL: Vec3 = Vec3::new(0.030, -0.025, 0.085);

/// Where a sight is fetched from and dropped to: low on the weak side, a
/// hand's width out from the pouch the magazine comes from. Relative to the
/// eye, the same frame the reload waypoints use.
const POCKET_FROM_EYE: Vec3 = Vec3::new(-0.24, -0.44, -0.22);

/// Fractions of the gesture: when the hand arrives at the rail, and when the
/// sight changes hands between the rail and the hand.
const AT_RAIL: f32 = 0.34;
const CHANGES_HANDS: f32 = 0.60;
/// How quickly the weapon turns into and out of the presenting pose. Short of
/// the ends, so it is already round before the hand arrives and back square
/// before the gesture finishes.
const PRESENT_IN: f32 = 0.22;
const PRESENT_OUT: f32 = 0.86;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SwapKind {
    Fit(SightId),
    Remove(SightId),
}

/// What is on the weapon right now, which lags what the bag says by the
/// length of the gesture.
///
/// Two resources rather than one because the bag changes in an instant and
/// the weapon does not: `FittedSight` is the intent, this is what has
/// actually been bolted on, and the gesture is what carries one to the other.
#[derive(Resource, Default)]
pub(crate) struct ShownSight(pub(crate) Option<SightId>);

#[derive(Resource, Default)]
pub(crate) struct SightSwap {
    pub(crate) running: Option<SwapKind>,
    pub(crate) elapsed: f32,
}

impl SightSwap {
    pub(crate) fn progress(&self) -> f32 {
        (self.elapsed / SWAP_SECONDS).clamp(0.0, 1.0)
    }

    /// How far the weapon is turned to show its rail, 0 to 1.
    pub(crate) fn presenting(&self) -> f32 {
        if self.running.is_none() {
            return 0.0;
        }
        let t = self.progress();
        let ramp = if t < PRESENT_IN {
            t / PRESENT_IN
        } else if t > PRESENT_OUT {
            (1.0 - t) / (1.0 - PRESENT_OUT)
        } else {
            1.0
        };
        smooth(ramp.clamp(0.0, 1.0))
    }

    /// The turn and the pull that bring the rail round to the eye.
    pub(crate) fn present_pose(&self) -> (Quat, Vec3) {
        let amount = self.presenting();
        let turn = Quat::IDENTITY.slerp(
            Quat::from_rotation_y(PRESENT_YAW_DEG.to_radians())
                * Quat::from_rotation_x(PRESENT_PITCH_DEG.to_radians())
                * Quat::from_rotation_z(PRESENT_ROLL_DEG.to_radians()),
            amount,
        );
        (turn, PRESENT_PULL * amount)
    }
}

/// The sight in the hand: on its way to the rail, or on its way to the floor.
#[derive(Component)]
pub(crate) struct CarriedSight;

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn eased(from: Vec3, to: Vec3, t: f32) -> Vec3 {
    from.lerp(to, smooth(t.clamp(0.0, 1.0)))
}

/// Starts the gesture when the bag and the weapon disagree.
///
/// Exchanging one sight for another is two gestures, not one: the old comes
/// off and only then does the new go on, which is what the hand would have to
/// do and what makes the rail readable in between.
pub(crate) fn begin_sight_swap(
    fitted: Res<crate::sight_mount::FittedSight>,
    shown: Res<ShownSight>,
    rail: Query<(), With<RailRearSocket>>,
    mut swap: ResMut<SightSwap>,
) {
    if swap.running.is_some() || fitted.0 == shown.0 {
        return;
    }
    // Nothing to present on: the change lands without a gesture.
    if rail.is_empty() {
        return;
    }
    swap.elapsed = 0.0;
    swap.running = Some(match shown.0 {
        Some(old) => SwapKind::Remove(old),
        None => match fitted.0 {
            Some(new) => SwapKind::Fit(new),
            None => return,
        },
    });
}

/// Ticks the gesture, and hands the sight over at the moment the hand is on
/// the rail.
pub(crate) fn advance_sight_swap(
    time: Res<Time>,
    mut shown: ResMut<ShownSight>,
    mut swap: ResMut<SightSwap>,
) {
    let Some(kind) = swap.running else { return };
    let was = swap.progress();
    swap.elapsed += time.delta_secs();
    let now = swap.progress();
    if was < CHANGES_HANDS && now >= CHANGES_HANDS {
        shown.0 = match kind {
            SwapKind::Fit(id) => Some(id),
            SwapKind::Remove(_) => None,
        };
    }
    if now >= 1.0 {
        // A removal that was really an exchange leaves the bag still asking
        // for one. Nothing to do about it here: begin_sight_swap sees the
        // disagreement next frame and starts the fit.
        swap.running = None;
        swap.elapsed = 0.0;
    }
}

/// Walks the weak hand through the gesture, overriding the reload path.
///
/// The same target entity the reload uses, because the solver takes one and
/// two authorities writing it would fight. This runs after, so during a swap
/// it is the one that decides.
pub(crate) fn drive_sight_hand(
    swap: Res<SightSwap>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
    rear: Query<&GlobalTransform, With<RailRearSocket>>,
    front: Query<&GlobalTransform, With<RailFrontSocket>>,
    fore: Query<&GlobalTransform, With<crate::ForeGripSocket>>,
    mut hand_bids: ResMut<crate::weak_hand::WeakHand>,
) {
    let Some(kind) = swap.running else { return };
    let (Ok(eye), Ok(fore)) = (camera.single(), fore.single()) else {
        return;
    };
    let (Ok(rear), Ok(front)) = (rear.single(), front.single()) else { return };

    let home = fore.reparented_to(eye);
    let on_rail =
        (rear.reparented_to(eye).translation + front.reparented_to(eye).translation) * 0.5;
    let pocket = POCKET_FROM_EYE - Vec3::Z * view_pullback();
    // Fetching starts at the pocket with the sight already in hand; taking
    // one off starts on the weapon and ends there.
    let (from, to) = match kind {
        SwapKind::Fit(_) => (pocket, home.translation),
        SwapKind::Remove(_) => (home.translation, pocket),
    };

    let t = swap.progress();
    let want = if t < AT_RAIL {
        eased(from, on_rail, t / AT_RAIL)
    } else if t < CHANGES_HANDS {
        on_rail
    } else {
        eased(on_rail, to, (t - CHANGES_HANDS) / (1.0 - CHANGES_HANDS))
    };
    hand_bids.claim(crate::weak_hand::SIGHT_SWAP, want, home.rotation);
}

/// Keeps a sight in the hand for the half of the gesture it is not bolted on.
///
/// Spawned and moved rather than reparented: the mounted one is a child of
/// the weapon and belongs to `fit_sight`, and a sight that is being carried
/// is not on the weapon at all.
pub(crate) fn carry_swapped_sight(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    swap: Res<SightSwap>,
    camera: Query<Entity, With<FpsCamera>>,
    target: Query<&Transform, With<ReloadHandTarget>>,
    carried: Query<Entity, With<CarriedSight>>,
) {
    let Ok(camera) = camera.single() else { return };
    let in_hand = match swap.running {
        Some(SwapKind::Fit(id)) if swap.progress() < CHANGES_HANDS => Some(id),
        Some(SwapKind::Remove(id)) if swap.progress() >= CHANGES_HANDS => Some(id),
        _ => None,
    };

    let Some(id) = in_hand else {
        for entity in &carried {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Ok(hand) = target.single() else { return };
    // Sat slightly out of the palm and turned upright, so it reads as held
    // rather than growing out of the wrist.
    let held = Transform::from_translation(hand.translation + Vec3::new(0.0, 0.012, 0.0))
        .with_rotation(hand.rotation * Quat::from_rotation_x(-0.25));

    match carried.single() {
        Ok(entity) => {
            commands.entity(entity).insert(held);
        }
        Err(_) => {
            commands.spawn((
                Name::new("CarriedSight"),
                WorldAssetRoot(load_gltf_scene(
                    &asset_server,
                    id.def().mesh_path,
                    crate::gltf_usages(),
                )),
                held,
                GunMeshPart,
                CarriedSight,
                ChildOf(camera),
            ));
        }
    }
}

/// Whether the weak hand is off the weapon because of a swap.
pub(crate) fn hand_busy_swapping(swap: &SightSwap) -> bool {
    swap.running.is_some()
}
