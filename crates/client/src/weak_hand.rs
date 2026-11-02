//! Who the weak hand belongs to this frame.
//!
//! Three things take that hand off the weapon -- a reload, a sight going on
//! or coming off, a grenade -- and for a while each simply wrote the target
//! transform, with the later system in the chain winning. That works until
//! someone adds a fourth and has to read the whole chain to find out where
//! it goes, or reorders the chain for an unrelated reason and silently
//! changes which gesture wins.
//!
//! So they bid instead. Each says where it wants the hand and how strongly;
//! the highest bid is written once, at the end. The order the systems run in
//! stops mattering, and the precedence is a number in one place rather than
//! a position in a list.

use bevy::prelude::*;

use crate::ReloadHandTarget;

/// Precedence, lowest first. A reload can be interrupted by a sight going
/// on, and both by a grenade: the pin is already out.
pub(crate) const RELOAD: u8 = 1;
pub(crate) const SIGHT_SWAP: u8 = 2;
pub(crate) const THROW: u8 = 3;

/// The bid for the weak hand, in the eye's own frame.
#[derive(Resource, Default)]
pub(crate) struct WeakHand {
    best: Option<(u8, Vec3, Quat)>,
}

impl WeakHand {
    pub(crate) fn claim(&mut self, priority: u8, at: Vec3, turn: Quat) {
        // Equal bids let the later caller win, which keeps the old chain
        // behaviour for anything that ends up sharing a number.
        if self.best.is_none_or(|(p, _, _)| priority >= p) {
            self.best = Some((priority, at, turn));
        }
    }

    /// Whether anything has the hand, for the IK to know it is not on the gun.
    pub(crate) fn taken(&self) -> bool {
        self.best.is_some()
    }
}

/// Clears last frame's bids. Runs before anyone places one.
pub(crate) fn open_bidding(mut hand: ResMut<WeakHand>) {
    hand.best = None;
}

/// Writes the winning bid. Runs after all of them.
pub(crate) fn settle_weak_hand(
    hand: Res<WeakHand>,
    mut target: Query<&mut Transform, With<ReloadHandTarget>>,
) {
    let Some((_, at, turn)) = hand.best else { return };
    let Ok(mut target) = target.single_mut() else { return };
    target.translation = at;
    target.rotation = turn;
}
