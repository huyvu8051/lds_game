
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const MAX_SHOT_DISTANCE: f32 = 200.0;

/// Where the three damage bands meet.
///
/// Both are distances this game already measures something in, rather than
/// numbers picked to look round: 25m is how far a zombie's footsteps carry,
/// and 60m is how far one will notice you and start walking. So the near band
/// is the fight you can hear, the middle band is the fight that is coming for
/// you, and the far band is everything out to the 200m a bullet travels.
pub const RANGE_NEAR_LIMIT: f32 = 25.0;
pub const RANGE_MID_LIMIT: f32 = 60.0;

/// What is left of a shot after the flight, in each of the three bands.
///
/// Steps, not a curve: the same shot is worth less at 26m than at 24m, and
/// there is nothing between the two. Three bands is what was asked for, and
/// the edge is where it shows -- a zombie walking across 25m changes what it
/// costs to kill without anything else about it changing. Blending across
/// the edge would be a third constant here and nowhere else.
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct RangeDamage {
    pub near: f32,
    pub mid: f32,
    pub far: f32,
}

impl RangeDamage {
    pub fn at(&self, distance: f32) -> f32 {
        if distance <= RANGE_NEAR_LIMIT {
            self.near
        } else if distance <= RANGE_MID_LIMIT {
            self.mid
        } else {
            self.far
        }
    }
}

#[derive(Event, Clone, Copy, Serialize, Deserialize)]
pub struct FireShot {
    pub origin: Vec3,
    pub direction: Vec3,
    /// What the firing weapon multiplies a hitbox's own damage by. Carried on
    /// the shot rather than read where it lands: the weapon can be swapped
    /// between a shot leaving the barrel and it arriving.
    pub damage_scale: f32,
    /// What the flight costs it, for the same reason and carried the same way.
    pub range: RangeDamage,
}

#[derive(Event, Clone, Copy, Serialize, Deserialize)]
pub struct PlayerFiredFx {
    pub shooter_network_id: u64,
}

#[derive(Event, Clone, Copy, Serialize, Deserialize)]
pub struct PlayerDiedFx {
    pub who_network_id: u64,
}

#[derive(Event, Clone, Copy, Serialize, Deserialize)]
pub enum ShotFx {
    ZombieHit { origin: Vec3, point: Vec3, normal: Vec3, damage: f32, killed: bool },
    SurfaceHit { origin: Vec3, point: Vec3, normal: Vec3 },
    /// A wall the bullet went through. Same impact marks as a surface hit, but
    /// no tracer: one shot draws one tracer, however many walls it crosses.
    Penetrated { point: Vec3, normal: Vec3 },
}

#[derive(Event, Clone, Copy, Serialize, Deserialize)]
pub struct ShotConfirmed {
    pub origin: Vec3,
    pub point: Vec3,
    pub normal: Vec3,
    pub damage: f32,
    pub killed: bool,
}
