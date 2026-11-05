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
/// The hand sits forward of `Socket_FrontGrip`, out towards the gas block,
/// because that is where the support hand lands once the weapon is actually
/// shouldered -- the socket marks where a handguard is, not where a hand
/// goes.
impl Default for GripPose {
    fn default() -> Self {
        Self {
            hand_on_gun: Quat::from_xyzw(-0.390641, -0.238717, -0.488558, 0.742782),
            grip_in_hand: Vec3::new(0.0052, 0.0350, 0.0383),
            fingers: [
                Quat::from_xyzw(0.40410, 0.01205, 0.01484, 0.91451),
                Quat::from_xyzw(0.49843, -0.00037, -0.05339, 0.86528),
                Quat::from_xyzw(0.23786, 0.00101, -0.02235, 0.97104),
                Quat::from_xyzw(0.48533, -0.02273, -0.06194, 0.87184),
                Quat::from_xyzw(0.58843, 0.00393, -0.06010, 0.80630),
                Quat::from_xyzw(0.19564, 0.00090, -0.01736, 0.98052),
                Quat::from_xyzw(0.67950, -0.07139, -0.01909, 0.72994),
                Quat::from_xyzw(0.58355, 0.00496, -0.06275, 0.80963),
                Quat::from_xyzw(0.00950, -0.00167, -0.01433, 0.99985),
                Quat::from_xyzw(0.57831, -0.03339, -0.06969, 0.81215),
                Quat::from_xyzw(0.62137, 0.00350, -0.06943, 0.78043),
                Quat::from_xyzw(0.16601, -0.00161, -0.02635, 0.98577),
                Quat::from_xyzw(0.13262, 0.06174, 0.16859, 0.97477),
                Quat::from_xyzw(0.10188, 0.17140, -0.04716, 0.97878),
                Quat::from_xyzw(-0.12548, 0.02774, 0.08647, 0.98793),
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
