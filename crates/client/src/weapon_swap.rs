//! Putting one weapon away and bringing the next one out.
//!
//! A swap used to be a single frame: the old mesh despawned, the new one
//! appeared in its place. Nothing in the hand ever travelled, so the only
//! thing separating an AK from an M24 was that the shape changed.
//!
//! Which way a weapon travels is decided by where it is kept. Slot one is
//! slung on the left of the back and slot two on the right, which puts them
//! on opposite sides of the shoulder: one is reached for low and across, the
//! other over the top. Slot three is the hip.
//!
//! So each slot owns a side rather than a direction. One and three come up
//! from below and go back down; two comes down from above and goes back up.
//! The draw and the holster of any slot run through the same half of the
//! frame, which is what makes them read as one motion reversed.

use bevy::prelude::*;

use crate::*;

/// Seconds to put one away, and to bring the next out. Drawing is the
/// slower half: it is the part the player is waiting on, and a draw that
/// matches the holster reads as a cut rather than a handover.
const HOLSTER_SECONDS: f32 = 0.18;
const DRAW_SECONDS: f32 = 0.30;

/// What a sidearm is worth in time. It is lighter, it is carried where the
/// hand already is, and reaching for it is what a player does when the thing
/// in their hands has run out -- so the whole exchange is quicker, including
/// putting away the rifle that failed them.
const SIDEARM_SPEEDUP: f32 = 0.55;

fn is_sidearm(slot: inventory::Slot) -> bool {
    matches!(slot, inventory::Slot::Sidearm)
}

/// How far out of frame the weapon goes, and how far it rolls doing it.
const TRAVEL: f32 = 0.55;
const TRAVEL_PITCH_DEG: f32 = 52.0;
const TRAVEL_ROLL_DEG: f32 = 22.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Holster,
    Draw,
}

#[derive(Resource)]
pub(crate) struct WeaponSwap {
    phase: Phase,
    elapsed: f32,
    /// Which way the thing in hand is travelling: -1 down, +1 up.
    direction: f32,
    /// How long this phase runs. Held rather than recomputed, so a slot
    /// changing mid-swap cannot stretch an animation already under way.
    duration: f32,
    /// Held back until the hand is empty.
    pending: Option<WeaponId>,
}

impl Default for WeaponSwap {
    fn default() -> Self {
        Self { phase: Phase::Idle, elapsed: 0.0, direction: -1.0, duration: DRAW_SECONDS, pending: None }
    }
}

/// Which half of the frame a slot's weapon travels through: -1 below, +1
/// above. The same for its draw and its holster, which is why one number
/// serves both -- a weapon that comes up from below goes back down.
fn direction_of(slot: inventory::Slot) -> f32 {
    match slot {
        // Left of the back: reached for low and across.
        inventory::Slot::Primary(0) => -1.0,
        // Right of the back: taken over the shoulder.
        inventory::Slot::Primary(_) => 1.0,
        // The hip.
        inventory::Slot::Sidearm => -1.0,
    }
}

impl WeaponSwap {
    /// How far out of the hand the weapon is, zero to one.
    fn away(&self) -> f32 {
        match self.phase {
            Phase::Idle => 0.0,
            // Eased at both ends, so the weapon leaves and arrives without
            // a corner on either.
            Phase::Holster => smooth(self.elapsed / self.duration),
            Phase::Draw => 1.0 - smooth(self.elapsed / self.duration),
        }
    }

    /// The offset and turn the weapon wears this frame.
    pub(crate) fn pose(&self) -> (Vec3, f32, f32) {
        let away = self.away();
        if away <= 0.0 {
            return (Vec3::ZERO, 0.0, 0.0);
        }
        let offset = Vec3::new(0.0, self.direction * TRAVEL * away, TRAVEL * 0.35 * away);
        // Turned as it goes, because a weapon lowered without rotating reads
        // as a prop being slid off a shelf.
        let pitch = (TRAVEL_PITCH_DEG * self.direction * away).to_radians();
        let roll = (TRAVEL_ROLL_DEG * away).to_radians();
        (offset, pitch, roll)
    }

    /// True while the weapon is not in a state to be fired from.
    pub(crate) fn busy(&self) -> bool {
        self.phase != Phase::Idle
    }
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Catches a swap request and puts the current weapon away first.
///
/// The request is taken rather than left for switch_weapon, and handed back
/// when the hand is empty. That keeps one place deciding what is held and
/// stops the mesh changing underneath a weapon that is still on screen.
pub(crate) fn begin_weapon_swap(
    loadout: Res<Loadout>,
    active: Res<ActiveWeapon>,
    mut request: ResMut<weapon::WeaponSwitchRequest>,
    mut swap: ResMut<WeaponSwap>,
) {
    if swap.phase != Phase::Idle {
        return;
    }
    let Some(wanted) = request.0 else { return };
    if wanted == active.0 {
        request.0 = None;
        return;
    }
    request.0 = None;
    let from = inventory::Slot::ALL
        .into_iter()
        .find(|slot| loadout.get(*slot) == Some(active.0))
        .unwrap_or(loadout.active);
    let to = inventory::Slot::ALL
        .into_iter()
        .find(|slot| loadout.get(*slot) == Some(wanted))
        .unwrap_or(loadout.active);
    swap.phase = Phase::Holster;
    swap.elapsed = 0.0;
    // Shortened by whichever end is the sidearm, including the rifle being
    // put away to reach for it.
    swap.duration = HOLSTER_SECONDS
        * if is_sidearm(from) || is_sidearm(to) { SIDEARM_SPEEDUP } else { 1.0 };
    // Which way out is decided by where the weapon being put away lives,
    // and which way in by where the next one does.
    swap.direction = direction_of(from);
    swap.pending = Some(wanted);
}

pub(crate) fn advance_weapon_swap(
    time: Res<Time>,
    loadout: Res<Loadout>,
    mut request: ResMut<weapon::WeaponSwitchRequest>,
    mut swap: ResMut<WeaponSwap>,
) {
    if swap.phase == Phase::Idle {
        return;
    }
    swap.elapsed += time.delta_secs();
    match swap.phase {
        Phase::Holster if swap.elapsed >= swap.duration => {
            // The hand is empty: now the mesh may change.
            let Some(wanted) = swap.pending.take() else {
                swap.phase = Phase::Idle;
                return;
            };
            request.0 = Some(wanted);
            let to = inventory::Slot::ALL
                .into_iter()
                .find(|slot| loadout.get(*slot) == Some(wanted))
                .unwrap_or(loadout.active);
            swap.phase = Phase::Draw;
            swap.elapsed = 0.0;
            swap.direction = direction_of(to);
            swap.duration = DRAW_SECONDS * if is_sidearm(to) { SIDEARM_SPEEDUP } else { 1.0 };
        }
        Phase::Draw if swap.elapsed >= swap.duration => {
            swap.phase = Phase::Idle;
            swap.elapsed = 0.0;
        }
        _ => {}
    }
}
