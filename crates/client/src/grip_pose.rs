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
///
/// The sockets are siblings of the weapon mesh in the glb, not children of
/// it, so rotating the mesh in Blender leaves every mark behind without a
/// word. A set measured that way reads a handguard that is no longer where
/// the mesh says it is -- the tell was `grip_in_hand`'s z at 90mm against
/// the right hand's 48mm on the same palm. Parent the sockets to the body
/// before posing.
impl Default for GripPose {
    fn default() -> Self {
        Self {
            hand_on_gun: Quat::from_xyzw(-0.651500, 0.275675, -0.489854, 0.509504),
            // Measured (-0.0080, 0.0593, 0.0030), then taken three and a
            // half centimetres left by eye. The offset is carried in the hand's
            // own frame, so a centimetre of view-space travel is not a
            // centimetre on any one axis here: it is
            // -hand_on_gun.inverse() * (-0.035, 0, 0), applied once from the
            // measurement rather than a step at a time from itself, so the
            // rounding does not compound.
            grip_in_hand: Vec3::new(0.0049, 0.0642, 0.0352),
            fingers: [
                Quat::from_xyzw(0.40358, 0.01200, 0.01499, 0.91474),
                Quat::from_xyzw(0.49837, -0.00037, -0.05339, 0.86532),
                Quat::from_xyzw(0.23790, 0.00101, -0.02236, 0.97103),
                Quat::from_xyzw(0.48514, -0.02285, -0.06168, 0.87196),
                Quat::from_xyzw(0.58739, 0.00392, -0.05998, 0.80707),
                Quat::from_xyzw(0.19567, 0.00090, -0.01736, 0.98052),
                Quat::from_xyzw(0.78818, -0.03787, -0.07864, 0.60922),
                Quat::from_xyzw(0.58355, 0.00496, -0.06275, 0.80963),
                Quat::from_xyzw(0.00991, -0.00167, -0.01438, 0.99985),
                Quat::from_xyzw(0.58070, -0.03363, -0.06987, 0.81042),
                Quat::from_xyzw(0.62121, 0.00350, -0.06941, 0.78056),
                Quat::from_xyzw(0.16601, -0.00161, -0.02635, 0.98577),
                Quat::from_xyzw(0.13332, 0.06042, 0.16891, 0.97470),
                Quat::from_xyzw(0.06754, 0.15785, 0.11112, 0.97886),
                Quat::from_xyzw(-0.12498, 0.02753, 0.08647, 0.98800),
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
