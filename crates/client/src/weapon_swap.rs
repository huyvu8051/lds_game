//! Putting one weapon away and bringing the next one out.
//!
//! A swap used to be a single frame: the old mesh despawned, the new one
//! appeared in its place. Nothing in the hand ever travelled, so the only
//! thing separating an AK from an M24 was that the shape changed.
//!
//! Which way a weapon travels is decided by where it is kept. A long gun
//! comes off the back and down into the hands; a sidearm comes up off the
//! hip. That is the two ends of the slot bar, and the middle slot holds
//! another long gun, so it goes with the left.

use bevy::prelude::*;

use crate::*;

/// Seconds to put one away, and to bring the next out. Drawing is the
/// slower half: it is the part the player is waiting on, and a draw that
/// matches the holster reads as a cut rather than a handover.
const HOLSTER_SECONDS: f32 = 0.18;
const DRAW_SECONDS: f32 = 0.30;

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
    /// Held back until the hand is empty.
    pending: Option<WeaponId>,
}

impl Default for WeaponSwap {
    fn default() -> Self {
        Self { phase: Phase::Idle, elapsed: 0.0, direction: -1.0, pending: None }
    }
}

/// Down for a long gun, up for a sidearm.
fn direction_of(slot: inventory::Slot) -> f32 {
    match slot {
        inventory::Slot::Sidearm => 1.0,
        _ => -1.0,
    }
}

impl WeaponSwap {
    /// How far out of the hand the weapon is, zero to one.
    fn away(&self) -> f32 {
        match self.phase {
            Phase::Idle => 0.0,
            // Eased at both ends, so the weapon leaves and arrives without
            // a corner on either.
            Phase::Holster => smooth(self.elapsed / HOLSTER_SECONDS),
            Phase::Draw => 1.0 - smooth(self.elapsed / DRAW_SECONDS),
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
    swap.phase = Phase::Holster;
    swap.elapsed = 0.0;
    // Which way out is decided by where the weapon being put away lives,
    // and which way in by where the next one does.
    swap.direction = direction_of(
        inventory::Slot::ALL
            .into_iter()
            .find(|slot| loadout.get(*slot) == Some(active.0))
            .unwrap_or(loadout.active),
    );
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
        Phase::Holster if swap.elapsed >= HOLSTER_SECONDS => {
            // The hand is empty: now the mesh may change.
            let Some(wanted) = swap.pending.take() else {
                swap.phase = Phase::Idle;
                return;
            };
            request.0 = Some(wanted);
            swap.phase = Phase::Draw;
            swap.elapsed = 0.0;
            swap.direction = direction_of(
                inventory::Slot::ALL
                    .into_iter()
                    .find(|slot| loadout.get(*slot) == Some(wanted))
                    .unwrap_or(loadout.active),
            );
        }
        Phase::Draw if swap.elapsed >= DRAW_SECONDS => {
            swap.phase = Phase::Idle;
            swap.elapsed = 0.0;
        }
        _ => {}
    }
}
