//! What has to be put down when a different weapon comes up.
//!
//! A weapon change cancels whatever the hands were doing, and each system
//! that had something in hand used to be responsible for noticing. One of
//! them did not: the magazine carried out of the old weapon stayed, and the
//! fetch mark -- crossed in the same frame the reload timer jumped to the
//! end -- built another one out of a weapon that was already away. It went
//! into the new weapon's hand.
//!
//! One place notices now, and clears all of it.

use bevy::prelude::*;

use crate::magazine_drop::{CarriedMagazine, MagazineInHand};
use crate::sight_swap::{CarriedSight, SightSwap};
use crate::weapon::ActiveWeapon;
use crate::{ViewmodelMagazine, WeaponId};

/// Set for the one frame a different weapon came up.
///
/// Its own comparison rather than `Res::is_changed`: that fires whenever
/// anything writes the resource, including writing the same weapon back.
#[derive(Resource, Default)]
pub(crate) struct WeaponChanged(pub(crate) bool);

pub(crate) fn mark_weapon_change(
    active: Res<ActiveWeapon>,
    mut changed: ResMut<WeaponChanged>,
    mut last: Local<Option<WeaponId>>,
) {
    changed.0 = last.replace(active.0) != Some(active.0);
}

/// Puts down everything the old weapon's hands were holding.
///
/// A cooking grenade is deliberately not on this list: its fuse is already
/// running, and clearing it would delete a live grenade rather than drop
/// one. Nothing it holds refers to a weapon anyway.
pub(crate) fn clear_weapon_state(
    changed: Res<WeaponChanged>,
    mut commands: Commands,
    mut held: ResMut<MagazineInHand>,
    mut swap: ResMut<SightSwap>,
    carried_mag: Query<Entity, With<CarriedMagazine>>,
    carried_sight: Query<Entity, With<CarriedSight>>,
    mut magazines: Query<&mut Visibility, With<ViewmodelMagazine>>,
) {
    if !changed.0 {
        return;
    }
    for entity in &carried_mag {
        commands.entity(entity).despawn();
    }
    for entity in &carried_sight {
        commands.entity(entity).despawn();
    }
    held.clear();
    swap.running = None;
    swap.elapsed = 0.0;
    // Whatever was hidden belonged to the weapon that is gone, and the one
    // arriving must not come up missing its magazine.
    for mut visibility in &mut magazines {
        if *visibility == Visibility::Hidden {
            *visibility = Visibility::Inherited;
        }
    }
}
