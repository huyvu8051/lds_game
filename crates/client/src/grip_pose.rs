use bevy::prelude::*;

use crate::keybinds::Bind;

/// Which way round the weak hand is on the handguard.
///
/// Three ways to hold the same bar, and they differ only in where the palm
/// faces. The fingers wrap it the same way in all three -- what changes is
/// how far the hand has rolled around it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum GripStyle {
    /// Palm up, the weapon resting in it.
    Below,
    /// Palm down, the hand over the top.
    Top,
    /// Palm facing the strong side, squeezing the handguard from the weak one.
    Side,
}

impl GripStyle {
    pub(crate) const ALL: [GripStyle; 3] = [GripStyle::Below, GripStyle::Top, GripStyle::Side];

    pub(crate) fn label(self) -> &'static str {
        match self {
            GripStyle::Below => "below",
            GripStyle::Top => "over the top",
            GripStyle::Side => "from the side",
        }
    }

    /// Where the palm faces, in the weapon's own frame.
    fn palm(self) -> Vec3 {
        match self {
            GripStyle::Below => Vec3::Y,
            GripStyle::Top => Vec3::NEG_Y,
            GripStyle::Side => Vec3::X,
        }
    }
}

/// How far the weapon is up, which is what decides the angle the hand holds
/// it at: the more the weapon comes into line with the body, the further the
/// support hand has to reach and the more it lies along the gun.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Posture {
    Rest,
    Ready,
    Ads,
}

/// The angle between the hand's own axis -- wrist to knuckles -- and the
/// bore. Ninety degrees would be a straight wrap across the handguard;
/// smaller is the hand turning to lie along the weapon.
fn posture_degrees(posture: Posture) -> f32 {
    match posture {
        Posture::Rest | Posture::Ready => 60.0,
        Posture::Ads => 30.0,
    }
}

/// At rest there is one way to hold it, because the weapon is not being
/// held out: the hand is under it, carrying it. The other two are things
/// done to a weapon that is up.
fn style_for(posture: Posture, style: GripStyle) -> GripStyle {
    match posture {
        Posture::Rest => GripStyle::Below,
        _ => style,
    }
}

/// The wrist's turn in the socket's frame, built from the two things that
/// describe a grip: where the palm faces and how far the hand is canted.
///
/// Built rather than posed, because this part is geometry: the hand's three
/// axes follow from the palm direction and the angle, and a quaternion
/// typed out by hand would only be a worse copy of the same thing. What is
/// posed is the fingers, which geometry has nothing to say about.
fn hand_on_gun(degrees: f32, palm: Vec3) -> Quat {
    let bore = Vec3::NEG_Z;
    // Where the knuckles point: across the bore, in the plane the palm
    // leaves free, and always towards the strong side or up -- the weak hand
    // reaches across the body, never away from it.
    let mut across = bore.cross(palm);
    if across.dot(Vec3::X + Vec3::Y) < 0.0 {
        across = -across;
    }
    let t = degrees.to_radians();
    let y = (bore * t.cos() + across * t.sin()).normalize();
    let z = palm.normalize();
    Quat::from_mat3(&Mat3::from_cols(y.cross(z), y, z))
}

/// The left hand's bones, in the order every pose table is written in.
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

/// The curl the game shipped, solved in Blender against a 22mm bar with the
/// hand canted about fifty degrees -- which is where this started and what
/// every slot below still falls back to.
const SHIPPED_FINGERS: [Quat; 15] = [
    Quat::from_xyzw(0.43597, 0.02352, 0.04846, 0.89835),
    Quat::from_xyzw(0.38315, 0.0, 0.0, 0.92369),
    Quat::from_xyzw(0.27647, 0.0, 0.0, 0.96102),
    Quat::from_xyzw(0.42245, -0.00591, -0.01269, 0.90628),
    Quat::from_xyzw(0.46652, 0.0, 0.0, 0.88451),
    Quat::from_xyzw(0.30535, 0.0, 0.0, 0.95224),
    Quat::from_xyzw(0.56899, -0.00243, -0.00351, 0.82234),
    Quat::from_xyzw(0.56859, 0.0, 0.0, 0.82262),
    Quat::from_xyzw(0.22812, 0.0, 0.0, 0.97363),
    Quat::from_xyzw(0.47205, -0.00991, -0.01850, 0.88132),
    Quat::from_xyzw(0.46805, 0.0, 0.0, 0.88370),
    Quat::from_xyzw(0.22178, 0.0, 0.0, 0.97510),
    Quat::from_xyzw(0.13362, 0.06153, 0.16997, 0.97441),
    Quat::from_xyzw(0.10248, 0.17281, -0.04630, 0.97852),
    Quat::from_xyzw(-0.12531, 0.02766, 0.08647, 0.98796),
];

/// Where the handguard sits inside the hand bone: +Y wrist to knuckles,
/// +X across the palm, +Z out through it. Paired with the finger curl and
/// only meaningful beside it.
const GRIP_IN_HAND: Vec3 = Vec3::new(0.0, 0.075, 0.0485);

/// The fingers for one posture and style, once they have been posed.
///
/// `None` is the honest state for a slot nobody has sat down with in
/// Blender yet: the wrist angle is already right, and the curl is the
/// shipped one, which was solved at fifty degrees rather than this slot's
/// thirty or sixty. Close, and visibly not exact at the bar.
fn posed_fingers(_posture: Posture, _style: GripStyle) -> Option<&'static [Quat; 15]> {
    None
}

/// Everything the solver and the fingers need for one grip.
#[derive(Clone, Copy)]
pub(crate) struct GripPose {
    pub(crate) hand_on_gun: Quat,
    pub(crate) grip_in_hand: Vec3,
    pub(crate) fingers: [Quat; 15],
}

impl Default for GripPose {
    fn default() -> Self {
        pose(Posture::Rest, GripStyle::Below)
    }
}

fn pose(posture: Posture, style: GripStyle) -> GripPose {
    let style = style_for(posture, style);
    GripPose {
        hand_on_gun: hand_on_gun(posture_degrees(posture), style.palm()),
        grip_in_hand: GRIP_IN_HAND,
        fingers: *posed_fingers(posture, style).unwrap_or(&SHIPPED_FINGERS),
    }
}

/// Averages quaternions that are already near each other, which these are:
/// every grip is the same hand within a few tens of degrees. Signs are
/// pulled onto the first one, or opposite representations of the same
/// rotation cancel instead of blending.
fn blend(parts: [(Quat, f32); 3]) -> Quat {
    let reference = parts[0].0;
    let mut sum = Vec4::ZERO;
    for (q, weight) in parts {
        let q = if q.dot(reference) < 0.0 { -q } else { q };
        sum += Vec4::from(q) * weight;
    }
    Quat::from_vec4(sum.normalize_or(Vec4::from(reference)))
}

fn blend_pose(a: &GripPose, b: &GripPose, t: f32) -> GripPose {
    let mut fingers = a.fingers;
    for (out, (from, to)) in fingers.iter_mut().zip(a.fingers.iter().zip(b.fingers.iter())) {
        *out = from.slerp(*to, t);
    }
    GripPose {
        hand_on_gun: a.hand_on_gun.slerp(b.hand_on_gun, t),
        grip_in_hand: a.grip_in_hand.lerp(b.grip_in_hand, t),
        fingers,
    }
}

/// What the weak hand is doing this frame, and what it is on its way to.
#[derive(Resource)]
pub(crate) struct GripChoice {
    pub(crate) style: GripStyle,
    from: GripStyle,
    /// 0 at `from`, 1 at `style`.
    shift: f32,
    /// Seconds before the hand gets tired of this one.
    rest_left: f32,
    rng: u32,
}

impl Default for GripChoice {
    fn default() -> Self {
        Self {
            style: GripStyle::Below,
            from: GripStyle::Below,
            shift: 1.0,
            rest_left: SHIFT_AFTER_SECONDS.1,
            rng: 0x9E37_79B9,
        }
    }
}

/// How long one grip lasts before the hand changes it, drawn between these.
/// Long enough that it reads as the player shifting rather than as a twitch.
const SHIFT_AFTER_SECONDS: (f32, f32) = (22.0, 48.0);
/// How long the change itself takes. A hand lets go, rolls and closes again;
/// anything quicker reads as the pose being swapped rather than changed.
const SHIFT_SECONDS: f32 = 0.55;

fn next_random(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let inner = ((*state >> ((*state >> 28) + 4)) ^ *state).wrapping_mul(277_803_737);
    ((inner >> 22) ^ inner) as f32 / u32::MAX as f32
}

impl GripChoice {
    fn change_to(&mut self, style: GripStyle) {
        if style == self.style {
            return;
        }
        // From wherever the blend had got to, not from the last target: a
        // second change part way through the first would otherwise snap back
        // to a pose the hand had already half left.
        self.from = if self.shift >= 1.0 { self.style } else { self.from };
        self.style = style;
        self.shift = 0.0;
        info!("[grip] {}", style.label());
    }
}

/// Changes the grip on a key, and on its own once the hand has held one long
/// enough to want a change.
///
/// Not while aiming. A hand shifting its hold in the middle of a shot is the
/// one moment it would be felt as the game moving the weapon rather than the
/// player resting a hand.
pub(crate) fn choose_grip(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    gun: Query<&crate::Gun>,
    mut choice: ResMut<GripChoice>,
) {
    let dt = time.delta_secs();
    choice.shift = (choice.shift + dt / SHIFT_SECONDS).min(1.0);

    if Bind::CycleGrip.just_pressed(&keys) {
        let at = GripStyle::ALL.iter().position(|s| *s == choice.style).unwrap_or(0);
        let next = GripStyle::ALL[(at + 1) % GripStyle::ALL.len()];
        choice.change_to(next);
        choice.rest_left = SHIFT_AFTER_SECONDS.1;
        return;
    }

    let aiming = gun.single().is_ok_and(|gun| gun.ads_blend > 0.05);
    if aiming || choice.shift < 1.0 {
        return;
    }
    choice.rest_left -= dt;
    if choice.rest_left > 0.0 {
        return;
    }
    let roll = next_random(&mut choice.rng);
    let others: Vec<GripStyle> =
        GripStyle::ALL.into_iter().filter(|s| *s != choice.style).collect();
    let pick = others[(roll * others.len() as f32) as usize % others.len()];
    choice.change_to(pick);
    let (low, high) = SHIFT_AFTER_SECONDS;
    choice.rest_left = low + next_random(&mut choice.rng) * (high - low);
}

/// What the hand is actually holding this frame: the three postures mixed by
/// how far the weapon is up, then the old grip mixed with the new one.
#[derive(Resource, Default)]
pub(crate) struct HeldGrip(pub(crate) GripPose);

pub(crate) fn blend_held_grip(
    choice: Res<GripChoice>,
    gun: Query<&crate::Gun>,
    mut held: ResMut<HeldGrip>,
) {
    let (ads, ready) = gun.single().map_or((0.0, 0.0), |gun| {
        let ads = gun.ads_blend.clamp(0.0, 1.0);
        (ads, gun.ready_blend.clamp(0.0, 1.0) * (1.0 - ads))
    });
    let rest = (1.0 - ads - ready).max(0.0);

    let mix = |style: GripStyle| {
        let (r, y, a) = (pose(Posture::Rest, style), pose(Posture::Ready, style), pose(Posture::Ads, style));
        let mut fingers = r.fingers;
        for (i, out) in fingers.iter_mut().enumerate() {
            *out = blend([(r.fingers[i], rest), (y.fingers[i], ready), (a.fingers[i], ads)]);
        }
        GripPose {
            hand_on_gun: blend([(r.hand_on_gun, rest), (y.hand_on_gun, ready), (a.hand_on_gun, ads)]),
            grip_in_hand: r.grip_in_hand * rest + y.grip_in_hand * ready + a.grip_in_hand * ads,
            fingers,
        }
    };
    held.0 = if choice.shift >= 1.0 {
        mix(choice.style)
    } else {
        let t = choice.shift;
        blend_pose(&mix(choice.from), &mix(choice.style), t * t * (3.0 - 2.0 * t))
    };
}

/// The slot a left finger bone holds in every pose table.
pub(crate) fn left_finger_slot(bare_name: &str) -> Option<usize> {
    LEFT_FINGER_BONES.iter().position(|name| *name == bare_name)
}

/// Marks a left finger bone with which rotation of the held grip is its own.
#[derive(Component)]
pub(crate) struct LeftFingerSlot(pub(crate) usize);

/// Writes the blended grip over whatever the clip did, beside the right
/// hand's fixed one and for the same reason.
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
