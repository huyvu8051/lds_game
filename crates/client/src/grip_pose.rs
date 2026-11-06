//! How the weak hand holds the handguard.
//!
//! One grip, posed rather than solved. There were three -- palm up, palm
//! over the top, palm against the side -- cycled on a key and drifting from
//! one to another on a timer, with the wrist built from geometry and the
//! curl shared between them. It was a lot of machinery for a hand nobody
//! was looking at, and it still took one trip to Blender per slot to stop
//! being approximate. One grip, measured once, is the same picture.
//!
//! The numbers come out of an exported glTF, never out of Blender: a pose
//! bone's rotation is relative to a rest orientation the importer invents,
//! and setting a finger to 0.436 there writes 0.094 into the file. Only the
//! file is what the game loads. `zombie_assets/tools/read_grip.py` does the
//! round trip and prints this table.

use bevy::prelude::*;

/// The left hand's bones, in the order the pose table is written in.
pub(crate) const LEFT_FINGER_BONES: [&str; 15] = [
    "LeftHandIndex1",
    "LeftHandIndex2",
    "LeftHandIndex3",
    "LeftHandMiddle1",
    "LeftHandMiddle2",
    "LeftHandMiddle3",
    "LeftHandPinky1",
    "LeftHandPinky2",
    "LeftHandPinky3",
    "LeftHandRing1",
    "LeftHandRing2",
    "LeftHandRing3",
    "LeftHandThumb1",
    "LeftHandThumb2",
    "LeftHandThumb3",
];

/// Everything the arm solver and the fingers need.
#[derive(Clone, Copy)]
pub(crate) struct GripPose {
    /// The wrist's turn in the handguard socket's frame.
    pub(crate) hand_on_gun: Quat,
    /// Where the handguard sits inside the hand bone: +Y wrist to knuckles,
    /// +X across the palm, +Z out through it. Only meaningful beside the
    /// curl it was posed with.
    pub(crate) grip_in_hand: Vec3,
    pub(crate) fingers: [Quat; 15],
}

/// Posed on the player's own rig against an AK's handguard, in the rifle
/// aiming idle the game already plays.
///
/// Measured against `Socket_FrontGrip` itself. The first set was posed
/// against a dummy tube placed by eye, and the hand missed the weapon in
/// game by whatever that tube was from the socket -- which turned out to be
/// most of the length of a palm: `grip_in_hand`'s z went from 38mm to 90mm
/// once the reference was the node the solver actually reads.
///
/// An overhand hold: the palm comes down over the handguard from the left,
/// not up under it.
impl Default for GripPose {
    fn default() -> Self {
        Self {
            hand_on_gun: Quat::from_xyzw(-0.509497, 0.232052, -0.563400, 0.607573),
            grip_in_hand: Vec3::new(-0.0071, 0.0428, 0.0899),
            fingers: [
                Quat::from_xyzw(0.40253, 0.01206, 0.01497, 0.91520),
                Quat::from_xyzw(0.49890, -0.00037, -0.05344, 0.86501),
                Quat::from_xyzw(0.23785, 0.00101, -0.02235, 0.97104),
                Quat::from_xyzw(0.48658, -0.02284, -0.06201, 0.87113),
                Quat::from_xyzw(0.58715, 0.00392, -0.05995, 0.80725),
                Quat::from_xyzw(0.19563, 0.00090, -0.01736, 0.98052),
                Quat::from_xyzw(0.68019, -0.07158, -0.01911, 0.72928),
                Quat::from_xyzw(0.58355, 0.00496, -0.06275, 0.80963),
                Quat::from_xyzw(0.01033, -0.00168, -0.01443, 0.99984),
                Quat::from_xyzw(0.58149, -0.03363, -0.07004, 0.80984),
                Quat::from_xyzw(0.62112, 0.00350, -0.06940, 0.78063),
                Quat::from_xyzw(0.16601, -0.00161, -0.02635, 0.98577),
                Quat::from_xyzw(0.13359, 0.06145, 0.16997, 0.97442),
                Quat::from_xyzw(0.10248, 0.17281, -0.04630, 0.97851),
                Quat::from_xyzw(-0.12530, 0.02766, 0.08646, 0.98796),
            ],
        }
    }
}

/// The grip, as a resource so the arm solver and the fingers read the same
/// one. It never changes; it is a resource rather than a constant only
/// because that is how both of them already ask for it.
#[derive(Resource, Default)]
pub(crate) struct HeldGrip(pub(crate) GripPose);

/// The slot a left finger bone holds in the pose table.
pub(crate) fn left_finger_slot(bare_name: &str) -> Option<usize> {
    LEFT_FINGER_BONES.iter().position(|name| *name == bare_name)
}

/// Marks a left finger bone with which rotation of the grip is its own.
#[derive(Component)]
pub(crate) struct LeftFingerSlot(pub(crate) usize);

/// Writes the grip over whatever the clip did, beside the right hand's
/// fixed one and for the same reason.
pub(crate) fn hold_left_finger_grip(
    held: Res<HeldGrip>,
    mut fingers: Query<(&LeftFingerSlot, &mut Transform)>,
) {
    for (slot, mut transform) in &mut fingers {
        if let Some(rotation) = held.0.fingers.get(slot.0) {
            transform.rotation = *rotation;
        }
    }
}
