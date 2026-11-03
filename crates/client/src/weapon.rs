use bevy::prelude::*;

use crate::*;

/// Everything that belongs to the weapon rather than to the player holding it.
///
/// These are gathered in one place because every one of them is tuned against
/// a particular mesh: swap the mesh and all of them are wrong together. They
/// were global constants while there was only ever one gun.
/// How the trigger behaves. A bolt rifle has one, a carbine has three.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// How a weapon answers a held trigger.
///
/// `Single` and `Auto` now do the same thing -- fire for as long as it is
/// held, one round every `fire_rate`. What used to separate them was a
/// switch, and the switch is gone; what separates the guns that carry them is
/// the rate itself, which is 0.125s on an AK and 1.3s on an M24. Both names
/// are kept because they say what the weapon is, and because getting the
/// distinction back means changing this match rather than retyping
/// thirty-five weapons.
pub(crate) enum FireMode {
    Single,
    Burst,
    Auto,
}


/// Rounds a burst sends.
pub(crate) const BURST_ROUNDS: u32 = 3;

/// How long after a burst before the next one starts.
///
/// Without it a burst weapon held down is an automatic one: three rounds, no
/// pause, three more. The pause is the whole of what makes it a burst.
pub(crate) const BURST_GAP_SECONDS: f32 = 0.3;

pub(crate) struct WeaponDef {
    pub(crate) name: &'static str,
    pub(crate) mesh_path: &'static str,

    pub(crate) rest_offset: Vec3,
    pub(crate) ready_offset: Vec3,
    /// How far the eye sits behind Socket_Aim when sighted: the rear glass on
    /// a scope, the rear sight on irons. The gun is then placed so that point
    /// lands on the camera axis, which is the whole of ads_offset.
    pub(crate) ads_eye_relief: f32,
    pub(crate) ads_speed: AdsSpeed,
    pub(crate) sprint_carry: SprintCarry,
    /// How much of the breathing and tremor the sights show on this weapon.
    ///
    /// Per weapon rather than one number divided by magnification: what the
    /// player sees is the sway times the glass, but how much of it a rifle
    /// shows is also a matter of how it is held, and a 12x hunting scope and a
    /// 24x sniper scope are not braced the same way. One number each says so
    /// directly instead of falling out of an arithmetic nobody can tune.
    pub(crate) hold_ads_fraction: f32,
    /// Metres the weapon slides off the sight line per unit of settling
    /// velocity -- how far it misses before it corrects on. Scaled roughly
    /// against how fast each weapon handles: the one that comes up slowest
    /// also has the most mass to stop.
    pub(crate) ads_settle_miss_m: f32,
    /// Degrees the bore tilts off the sight line per unit of settling
    /// velocity -- how far the sight picture swims while the weapon corrects.
    ///
    /// Scaled against magnification, not mass. The reticle reads the bore, and
    /// a scope multiplies whatever the bore does: the same degree that is a
    /// nudge through irons throws the target out of a 24x view. The weapon
    /// with the most glass needs the smallest number, which is the opposite of
    /// what ads_settle_miss_m wants.
    pub(crate) ads_settle_miss_deg: f32,
    /// What the main camera's field of view is multiplied by while sighted.
    /// This is the whole of the magnification on irons, and only the pull
    /// around the glass on a weapon that has any -- so the better the scope,
    /// the less this has to do.
    pub(crate) aim_fov_multiplier: f32,
    /// How far out the autopilot bothers raising the sights. A scope earns its
    /// slower handling at range; irons do not, so they engage from the hip for
    /// longer.
    pub(crate) autopilot_ads_min_distance: f32,
    /// The furthest the autopilot will open fire from, and the ceiling of the
    /// window it raises the sights in. Keep it above ads_min_distance or the
    /// weapon never aims at all.
    pub(crate) autopilot_engage_range: f32,
    /// Past this it sprints to close. Set near the engage range for a weapon
    /// that has no business closing at all.
    pub(crate) autopilot_sprint_approach_distance: f32,
    /// The same question for a blind approach -- pathing round a corner to
    /// something it cannot see. Shorter than the sighted threshold, since
    /// nothing is being covered while it runs.
    pub(crate) autopilot_sprint_pathing_distance: f32,

    /// Whether a sprint leaves this one in the firing hand alone, muzzle up
    /// beside the head, the way a sidearm or a short gun is actually run
    /// with. A rifle stays in two hands and goes across the body instead --
    /// there is too much of it to swing off one wrist.
    pub(crate) sprint_one_handed: bool,

    /// Whether a weapon with no holo window still shows its mark once the
    /// sights are up.
    ///
    /// It used to claim more than that: the comment here said the weapon
    /// model was taken away while sighted, and nothing anywhere did that --
    /// `GunMeshPart` was only ever read to despawn it on a weapon change.
    /// Nothing takes the weapon off the screen; this only decides whether a
    /// mark is drawn.
    pub(crate) scoped: bool,

/// Which mark this weapon's optic draws. Separate from `holo` and `scoped`,
/// which decide *whether* it shows: a bored AK scope hides its mark on a
/// window test and still draws a PSO, not a dot.
pub(crate) reticle: ReticleKind,

    /// The opening a holographic sight leaves, in the weapon's own frame, or
    /// None for a weapon that has no glass to look through.
    ///
    /// A holo's dot sits at infinity down the bore, so the ray from the eye to
    /// it runs parallel to the bore -- which in this frame is straight down
    /// -Z. That makes the whole visibility question one rectangle test on
    /// where the *eye* falls, with no stencil pass and no second camera: the
    /// dot is there exactly while the eye is inside the tube.
    pub(crate) holo: Option<HoloWindow>,

    pub(crate) sound: WeaponSound,
    /// How long the weapon takes to reload. A fifty round magazine and a five
    /// round bolt gun are not the same job.
    pub(crate) reload_duration: f32,
    /// Multiplies the damage the hitbox it lands on is worth. The hitbox
    /// decides where a shot counts for more; this decides what the weapon
    /// firing it is worth at all.
    pub(crate) damage_scale: f32,
    /// Which falloff table over range this weapon reads. The only thing the
    /// class decides so far, which is why it is a class and not a role: an
    /// M249 is here for its round, not for what it is carried for.
    pub(crate) class: WeaponClass,
    pub(crate) fire_rate: f32,
    pub(crate) magazine: u32,
/// How this weapon fires, and the only way it fires.
///
/// It was a list with a selector on top: most rifles carried auto, burst and
/// single, and a button walked between them. Holding the trigger now does
/// whichever one thing the weapon does -- a single-shot rifle repeats at its
/// own rate, a burst weapon keeps bursting -- so there was nothing left for
/// a list to choose between. Each weapon kept the mode it used to start on.
pub(crate) fire_mode: FireMode,
/// The calibre it feeds on, which is what the reserve count beside the
/// magazine is counting and what a reload draws from.
pub(crate) ammo: crate::ItemKind,
    pub(crate) recoil: Recoil,
}

/// What a weapon is, for the one question that is answered by the kind rather
/// than the weapon: what a shot is worth at the distance it lands.
///
/// Five rows. `Rifle` holds the two light machine guns alongside the assault
/// rifles because they fire the same round at the same ranges, and `Marksman`
/// holds the designated marksman rifles with the bolt guns for the same
/// reason -- a Mini 14 and an M24 are both weapons for shooting a long way.
/// `Shotgun` has one member and the steepest row of all, because here the
/// falloff is the whole model: a single ray carries the shot, so nothing else
/// is going to make buckshot stop working at a hundred metres.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WeaponClass {
    Pistol,
    Smg,
    Shotgun,
    Rifle,
    Marksman,
}

impl WeaponClass {
    /// The table, anchored so that eight out of ten is a whole shot.
    ///
    /// Authored as 8/7/6, 8/8/7, 8/9/9 and 8/3/1 out of ten, then multiplied
    /// through by 1.25 so the eight becomes one. The ratios between every
    /// pair of figures are the ones that were written down; what the anchor
    /// decides is only whether the scale's eight means a whole shot or four
    /// fifths of one. A whole shot, because a headshot is dealt exactly the
    /// zombie's health -- taking a fifth off it would have cost ten weapons,
    /// the AK-47 among them, the one-shot headshot they have today.
    ///
    /// Which is also why the marksman row passes one: nine out of ten on a
    /// scale whose eight is whole is a ninth more than whole.
    pub(crate) fn range_damage(self) -> shared::RangeDamage {
        let (near, mid, far) = match self {
            Self::Pistol | Self::Smg => (1.0, 0.875, 0.75),
            Self::Rifle => (1.0, 1.0, 0.875),
            Self::Marksman => (1.0, 1.125, 1.125),
            Self::Shotgun => (1.0, 0.375, 0.125),
        };
        shared::RangeDamage { near, mid, far }
    }
}

/// How far off a shot can land, by band and by whether the sights are up.
///
/// Written as metres of miss rather than as an angle, because an angle lies
/// about what it will do. A cone of fixed width already misses six times
/// further at 150m than at 25m, for free, by geometry -- so a table of angle
/// multipliers per band compounds with that and the figures stop meaning
/// anything anyone can picture. These are measured at each band's own anchor
/// distance and converted back to an angle there.
///
/// Two things to know about reading the numbers. They are a bound on one
/// axis, so the diagonal corner is 1.41 times further out, and the wander
/// that drives them is a sine -- it spends most of its time away from its
/// peaks, so a typical shot lands about 64% of the figure off. For scale: a
/// zombie's head is 21cm in radius and its chest 18cm.
///
/// What the class actually decides is how fast the cone opens with range,
/// and the one rule is that it may never close: a figure must be at least
/// the previous band's figure scaled by the ratio of their anchors, or a
/// zombie that steps further away becomes easier to hit in absolute terms.
/// That is not a theoretical worry -- the first draft of this table had the
/// rifle at 0.22m at 60m and 0.35m at 150m, which made a target at 61m land
/// 9cm off where one at 59m landed 14cm off. `debug_assert_opens` is why
/// nobody has to notice that again.
pub(crate) struct RangeAccuracy {
    /// Metres of miss at RANGE_ANCHORS, sights up.
    pub(crate) ads: [f32; 3],
    /// The same from the hip.
    pub(crate) hip: [f32; 3],
    /// What being on the move costs this weapon.
    pub(crate) moving: f32,
}

/// Where each band's figures are measured. The first two are the band edges
/// the damage table already uses; the third is not the far band's end -- that
/// is the 200m a ray travels -- but a distance someone might actually fight
/// at, which is what the figure wants to describe.
const RANGE_ANCHORS: [f32; 3] = [shared::RANGE_NEAR_LIMIT, shared::RANGE_MID_LIMIT, 150.0];

impl RangeAccuracy {
    /// The cone half-angle, in radians, for a shot going this far.
    pub(crate) fn at(&self, distance: f32, aiming: bool) -> f32 {
        let band = if distance <= shared::RANGE_NEAR_LIMIT {
            0
        } else if distance <= shared::RANGE_MID_LIMIT {
            1
        } else {
            2
        };
        let miss = if aiming { self.ads[band] } else { self.hip[band] };
        miss / RANGE_ANCHORS[band]
    }

    /// Panics in a debug build on a cone that closes as the target recedes.
    fn debug_assert_opens(&self, name: &str) {
        for row in [&self.ads, &self.hip] {
            for band in 1..3 {
                let before = row[band - 1] / RANGE_ANCHORS[band - 1];
                let after = row[band] / RANGE_ANCHORS[band];
                debug_assert!(
                    after >= before - f32::EPSILON,
                    "{name}: the cone closes between band {} and {band} ({before} to {after} rad)",
                    band - 1
                );
            }
        }
    }
}

impl WeaponClass {
    /// Standing, still, first shot of the magazine. Heat, stance and sprint
    /// multiply on top of this; they are the same for every weapon and were
    /// already tuned.
    pub(crate) fn accuracy(self) -> RangeAccuracy {
        let (accuracy, name) = match self {
            // Tight up close with the sights up or down -- 6cm and 9cm at
            // 25m against a head of 21cm -- and barely troubled by running,
            // which is the whole of what they are for. The cone then opens
            // faster than anything else here: 96cm at 150m is not a weapon,
            // and a pistol at 150m should not be one.
            Self::Pistol | Self::Smg | Self::Shotgun => (
                RangeAccuracy {
                    ads: [0.10, 0.45, 1.50],
                    hip: [0.14, 0.70, 2.20],
                    moving: 1.2,
                },
                "pistol/smg/shotgun",
            ),
            // As tight as an SMG up close, and the cone then opens about as
            // slowly as it can without closing: 17cm at 60m, 45cm at 150m,
            // which is a body hit more often than not. From the hip it is a
            // different weapon entirely, and moving costs it twice an SMG.
            Self::Rifle => (
                RangeAccuracy {
                    ads: [0.10, 0.26, 0.70],
                    hip: [0.40, 0.96, 2.40],
                    moving: 2.0,
                },
                "rifle",
            ),
            // The cone does not open at all: one angle, held from the muzzle
            // to 150m, which is what reaching out means. 23cm at 150m is a
            // head's radius, so a body is a certainty and a head is a
            // question. From the hip it is exactly as bad as a rifle, and
            // standing still matters more to it than to anything else.
            Self::Marksman => (
                RangeAccuracy {
                    ads: [0.06, 0.144, 0.36],
                    hip: [0.40, 0.96, 2.40],
                    moving: 3.0,
                },
                "marksman",
            ),
        };
        accuracy.debug_assert_opens(name);
        accuracy
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReticleKind {
    /// One lit dot at infinity, as a holographic sight carries.
    Dot,
    /// Thick posts tapering to a hairline cross: a hunting scope.
    Duplex,
}

/// The rectangle cut through a holographic sight, in the weapon's own frame.
/// Read off the mesh rather than guessed: these are the bounds the triangles
/// were removed inside.
pub(crate) struct HoloWindow {
    pub(crate) min: Vec2,
    pub(crate) max: Vec2,
}

/// How fast the sights come up, from each starting pose. A raise that begins
/// between them interpolates; dropping the sights always uses from_ready.
pub(crate) struct AdsSpeed {
    pub(crate) from_ready: f32,
    pub(crate) from_rest: f32,
}

/// Where a weapon sits when it is being carried rather than held ready.
///
/// The offset is in camera space off the rest pose -- x right, y up, -z away.
/// The angles are in the weapon's own, turned about the trigger grip, so yaw
/// swings the muzzle across the body and pitch tips it down.
pub(crate) struct SprintCarry {
    pub(crate) offset: Vec3,
    pub(crate) yaw_deg: f32,
    pub(crate) pitch_deg: f32,
    pub(crate) roll_deg: f32,
}

/// Cut by tools/make_weapon_sounds.sh. The reload is composed against
/// reload_duration, so changing that means re-running the script.
pub(crate) struct WeaponSound {
    pub(crate) fire: &'static str,
    pub(crate) reload: &'static str,
}


/// What a shot does, split by what it moves. The weapon is a child of the
/// camera, so `view` never shifts the gun on screen and `weapon` never moves
/// the aim.
pub(crate) struct Recoil {
    /// How hard the blow lands, on both halves: each value chases the mark a
    /// shot set at this rate. Well above either recovery rate or it reads as
    /// a drift.
    pub(crate) strike_speed: f32,
    pub(crate) view: ViewKick,
    pub(crate) weapon: WeaponKick,
    pub(crate) shock: ShockKick,
}

/// The ring-down a round leaves behind: what shakes, how fast, and for how
/// long. Separate from the two kicks above because it moves nothing -- it is
/// spent and gone before either of them has finished, and leaves the aim
/// exactly where it found it.
///
/// Copied into `FireShock` at the shot rather than read from whatever is in
/// hand when it is sampled: a ring-down belongs to the round that started it,
/// so swapping weapons mid-burst cannot change the shape of a blow already
/// landed.
#[derive(Clone, Copy, Default)]
pub(crate) struct ShockKick {
    /// Peak degrees at the eye. Well under the weapon's: the rifle is struck,
    /// the head only hears about it through a shoulder. Roll carries the most
    /// of the three because it costs no aim -- it does not move where the
    /// barrel points.
    pub(crate) view_pitch_deg: f32,
    pub(crate) view_yaw_deg: f32,
    pub(crate) view_roll_deg: f32,
    /// Peak degrees at the weapon, and metres along its travel. Several times
    /// the eye's: this is the thing the round was fired out of.
    pub(crate) gun_pitch_deg: f32,
    pub(crate) gun_yaw_deg: f32,
    pub(crate) gun_roll_deg: f32,
    pub(crate) gun_shift_m: f32,
    /// Hz, and the one number here with a hard ceiling. At 60fps anything
    /// above about fifteen is sampled fewer than four times a cycle and
    /// aliases into a slow wobble that changes shape with the frame rate.
    /// Heavier calibres sit lower. The three are never harmonics of each
    /// other, or they stay in step and read as one motion along a line
    /// rather than a rattle.
    pub(crate) freq_pitch: f32,
    pub(crate) freq_yaw: f32,
    pub(crate) freq_roll: f32,
    /// How fast the ring-down is spent, as a rate like the others. Matched to
    /// `strike_speed`, which is what keeps the shake inside the blow: a
    /// weapon that takes twice as long to reach the top of its kick needs
    /// half this, or it stops shaking halfway up and the rest of the climb
    /// glides.
    pub(crate) decay: f32,
}

/// What a shot does to the camera: where the player is aiming afterwards.
pub(crate) struct ViewKick {
    /// Radians the aim climbs per shot, and the ceiling a burst stacks to.
    /// The ceiling also sets what bullet spread reads as heat.
    pub(crate) climb_per_shot: f32,
    pub(crate) climb_max: f32,
    /// Radians the aim wanders sideways per shot, sign alternating so it
    /// wobbles rather than drifts -- which is why it needs no ceiling.
    pub(crate) wander_per_shot: f32,
    /// Metres the camera slides along its own back per shot, and the ceiling.
    /// Its own back rather than a levelled one, so a climbing view is shoved
    /// up and back together.
    pub(crate) shove_per_shot: f32,
    pub(crate) shove_max: f32,
    /// How fast the marks above tend back to zero: the aim coming home.
    pub(crate) recover_speed: f32,
}

/// What a shot does to the weapon in the hands: how much it visibly jumps,
/// none of which moves the aim.
pub(crate) struct WeaponKick {
    /// Metres straight back towards the shoulder, and straight up. Plain
    /// translation, no pivot, so they move the whole weapon.
    pub(crate) slide_back: f32,
    pub(crate) lift: f32,
    /// Degrees about the three axes. Pitch turns about the mesh origin, which
    /// sits on the bore at the muzzle -- so it pins the nose and swings the
    /// tail down, rather than lifting the nose. Roll and yaw flip sign per
    /// shot.
    pub(crate) pitch_deg: f32,
    pub(crate) roll_deg: f32,
    pub(crate) yaw_deg: f32,
    /// How fast the weapon settles back after a shot. A rate, like
    /// recover_speed: the kick eases out rather than sliding home at a
    /// constant rate and stopping dead, which is the same curve the sights
    /// come up on.
    pub(crate) settle_speed: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum WeaponId {
    #[default]
    Ak47,
    M4a1,
    M24,
    Ump45,
    Deagle,
    ScarL,
    M16a4,
    Famas,
    G36c,
    Qbz95,
    AugA3,
    BerylM762,
    Ace32,
    Mk47Mutant,
    Groza,
    Vector,
    Mp5k,
    Sks,
    P1911,
    Bizon,
    Skorpion,
    TommyGun,
    MicroUzi,
    Dp28,
    M249,
    Mini14,
    Mk12,
    Mk14Ebr,
    Slr,
    Vss,
    Svd,
    MosinNagant,
    R1895,
    P90,
    S686,
}

#[derive(Resource)]
pub(crate) struct ActiveWeapon(pub(crate) WeaponId);

impl Default for ActiveWeapon {
    /// `WEAPON=` picks what the player starts holding, by the same number the
    /// hotkeys use. Tuning a weapon's pose means looking at it, and reaching
    /// it by pressing a key is one thing a screenshot cannot do.
    fn default() -> Self {
        let wanted = std::env::var("WEAPON")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .and_then(|n| WeaponId::ALL.get(n.checked_sub(1)?).copied());
        Self(wanted.unwrap_or_default())
    }
}

impl ActiveWeapon {
    pub(crate) fn def(&self) -> &'static WeaponDef {
        self.0.def()
    }
}

impl WeaponId {
    pub(crate) const ALL: [WeaponId; 35] = [
        WeaponId::Ak47,
        WeaponId::M4a1,
        WeaponId::M24,
        WeaponId::Ump45,
        WeaponId::Deagle,
        WeaponId::ScarL,
        WeaponId::M16a4,
        WeaponId::Famas,
        WeaponId::G36c,
        WeaponId::Qbz95,
        WeaponId::AugA3,
        WeaponId::BerylM762,
        WeaponId::Ace32,
        WeaponId::Mk47Mutant,
        WeaponId::Groza,
        WeaponId::Vector,
        WeaponId::Mp5k,
        WeaponId::Sks,
        WeaponId::P1911,
        WeaponId::Bizon,
        WeaponId::Skorpion,
        WeaponId::TommyGun,
        WeaponId::MicroUzi,
        WeaponId::Dp28,
        WeaponId::M249,
        WeaponId::Mini14,
        WeaponId::Mk12,
        WeaponId::Mk14Ebr,
        WeaponId::Slr,
        WeaponId::Vss,
        WeaponId::Svd,
        WeaponId::MosinNagant,
        WeaponId::R1895,
        WeaponId::P90,
        WeaponId::S686,
    ];

    pub(crate) fn index(self) -> usize {
        match self {
            WeaponId::Ak47 => 0,
            WeaponId::M4a1 => 1,
            WeaponId::M24 => 2,
            WeaponId::Ump45 => 3,
            WeaponId::Deagle => 4,
            WeaponId::ScarL => 5,
            WeaponId::M16a4 => 6,
            WeaponId::Famas => 7,
            WeaponId::G36c => 8,
            WeaponId::Qbz95 => 9,
            WeaponId::AugA3 => 10,
            WeaponId::BerylM762 => 11,
            WeaponId::Ace32 => 12,
            WeaponId::Mk47Mutant => 13,
            WeaponId::Groza => 14,
            WeaponId::Vector => 15,
            WeaponId::Mp5k => 16,
            WeaponId::Sks => 17,
            WeaponId::P1911 => 18,
            WeaponId::Bizon => 19,
            WeaponId::Skorpion => 20,
            WeaponId::TommyGun => 21,
            WeaponId::MicroUzi => 22,
            WeaponId::Dp28 => 23,
            WeaponId::M249 => 24,
            WeaponId::Mini14 => 25,
            WeaponId::Mk12 => 26,
            WeaponId::Mk14Ebr => 27,
            WeaponId::Slr => 28,
            WeaponId::Vss => 29,
            WeaponId::Svd => 30,
            WeaponId::MosinNagant => 31,
            WeaponId::R1895 => 32,
            WeaponId::P90 => 33,
            WeaponId::S686 => 34,
        }
    }

    pub(crate) fn def(self) -> &'static WeaponDef {
        match self {
            WeaponId::Ak47 => &AK47,
            WeaponId::M4a1 => &M4A1,
            WeaponId::M24 => &M24,
            WeaponId::Ump45 => &UMP45,
            WeaponId::Deagle => &DEAGLE,
            WeaponId::ScarL => &SCAR_L,
            WeaponId::M16a4 => &M16A4,
            WeaponId::Famas => &FAMAS,
            WeaponId::G36c => &G36C,
            WeaponId::Qbz95 => &QBZ95,
            WeaponId::AugA3 => &AUG_A3,
            WeaponId::BerylM762 => &BERYL_M762,
            WeaponId::Ace32 => &ACE32,
            WeaponId::Mk47Mutant => &MK47_MUTANT,
            WeaponId::Groza => &GROZA,
            WeaponId::Vector => &VECTOR,
            WeaponId::Mp5k => &MP5K,
            WeaponId::Sks => &SKS,
            WeaponId::P1911 => &P1911,
            WeaponId::Bizon => &BIZON,
            WeaponId::Skorpion => &SKORPION,
            WeaponId::TommyGun => &TOMMY_GUN,
            WeaponId::MicroUzi => &MICRO_UZI,
            WeaponId::Dp28 => &DP28,
            WeaponId::M249 => &M249,
            WeaponId::Mini14 => &MINI14,
            WeaponId::Mk12 => &MK12,
            WeaponId::Mk14Ebr => &MK14_EBR,
            WeaponId::Slr => &SLR,
            WeaponId::Vss => &VSS,
            WeaponId::Svd => &SVD,
            WeaponId::MosinNagant => &MOSIN_NAGANT,
            WeaponId::R1895 => &R1895,
            WeaponId::P90 => &P90,
            WeaponId::S686 => &S686,
        }
    }

}

/// Standardised to match the M4: metres, barrel down -Z, origin on the bore at
/// the muzzle. The turn and the millimetres are baked into the asset's root
/// node, so the grips below are the Socket_RearGrip / Socket_FrontGrip the
/// mesh now ships with.
static AK47: WeaponDef = WeaponDef {
    name: "AK-47",
    mesh_path: "models/ak47_scope/mesh/ak47_scope.glb",
    rest_offset: Vec3::new(0.160, -0.0905, -0.7094),
    ready_offset: Vec3::new(0.045, -0.1655, -0.6194),
    ads_eye_relief: 0.11,
    ads_speed: AdsSpeed { from_ready: 8.0, from_rest: 5.5 },
    hold_ads_fraction: 0.192,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, -0.03, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.028,
    ads_settle_miss_deg: 0.5,
    // Irons. The rifle's own optic is a fitted sight now, hidden until the
    // bag puts one back on the rail, so bare this aims down the notch like
    // every other weapon without glass -- 0.525 was the magnification of a
    // scope that is no longer on it.
    aim_fov_multiplier: 0.75,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 22.0,
    autopilot_sprint_approach_distance: 22.0,
    autopilot_sprint_pathing_distance: 15.0,
    // The floor is half the default, because that is where aim sensitivity,
    // which is default/zoom against a half speed base, comes back out at
    // exactly the hip fire rate.
    sprint_one_handed: false,
    scoped: true,
    reticle: ReticleKind::Dot,
    // The bored aperture, as the rectangle that fits inside it. Magnified or
    // not, the eye still has to be behind the hole: swing hard enough that it
    // is not and the mark goes, the same as the UMP's.
    holo: Some(HoloWindow { min: Vec2::new(-0.0090, 0.0783), max: Vec2::new(0.0090, 0.0943) }),
    sound: WeaponSound { fire: "audio/weapons/ak47_fire.wav", reload: "audio/weapons/ak47_reload.wav" },
    reload_duration: 2.3,
    damage_scale: 1.0,
    class: WeaponClass::Rifle,
    fire_rate: 0.12,
    magazine: 50,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo762,
    // 7.62: slower, heavier, looser at the top of a burst. Every axis above
    // the M4's.
    recoil: Recoil {
        strike_speed: 60.0,
        view: ViewKick {
            climb_per_shot: 0.0110,
            climb_max: 0.090,
            wander_per_shot: 0.0060,
            shove_per_shot: 0.03,
            shove_max: 0.058,
            recover_speed: 6.6,
        },
        weapon: WeaponKick {
            slide_back: 0.14,
            lift: 0.000,
            pitch_deg: 1.0,
            roll_deg: 0.6,
            yaw_deg: 0.5,
            settle_speed: 10.0,
        },
        // Low: a heavy round out of a loose action. Spent in 56ms, inside a
        // strike that takes about 50.
        //
        // Amplitudes are around a third of the first pass. At this level the
        // weapon carries the read and the eye barely moves, which is both
        // what was asked for and closer to the thing -- a rifle in the hands
        // visibly judders, the sight picture mostly just loses its edge.
        shock: ShockKick {
            view_pitch_deg: 0.09,
            view_yaw_deg: 0.07,
            view_roll_deg: 0.17,
            gun_pitch_deg: 0.35,
            gun_yaw_deg: 0.26,
            gun_roll_deg: 0.48,
            gun_shift_m: 0.0012,
            freq_pitch: 12.0,
            freq_yaw: 8.5,
            freq_roll: 10.0,
            decay: 25.0,
        },
    },
};

/// No glass: a carry handle and a post, so it marks no scope sockets and the
/// scope view stays off for it.
///
/// rest_offset is anchored on Socket_RearGrip rather than on the origin,
/// because what holds still across a weapon swap is the firing hand. Anchoring
/// on the muzzle threw this 213mm-shorter rifle 208mm from the shoulder.
static M4A1: WeaponDef = WeaponDef {
    name: "M4A1",
    mesh_path: "models/m4a1/mesh/m4a1_rigged.glb",
    rest_offset: Vec3::new(0.160, -0.1018, -0.626),
    ready_offset: Vec3::new(0.071, -0.1468, -0.536),
    ads_eye_relief: 0.11,
    ads_speed: AdsSpeed { from_ready: 16.0, from_rest: 9.0 },
    hold_ads_fraction: 0.300,
    // 0.031 rather than 0.07. The carry offset rides on rest_offset, which
    // differs per weapon, so the number to compare is the sum: the M4 sat at
    // -0.032 against the AK's -0.121 and the M24's -0.071, nine centimetres
    // above one rifle and four above the other while wearing the same carry
    // pose as both. This puts it level with the M24.
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.031, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.014,
    ads_settle_miss_deg: 1.2,
    aim_fov_multiplier: 0.75,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 22.0,
    autopilot_sprint_approach_distance: 14.0,
    autopilot_sprint_pathing_distance: 9.0,
    // Irons: the scope view never comes on, so these only have to be sane.
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/m4a1_fire.wav", reload: "audio/weapons/m4a1_reload.wav" },
    reload_duration: 1.9,
    damage_scale: 0.85,
    class: WeaponClass::Rifle,
    fire_rate: 0.105,
    magazine: 45,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo556,
    // 5.56: about two thirds the AK's kick, settling faster. Cut by more than
    // a flat ratio because it also fires faster, and matching per shot would
    // have climbed harder for the same trigger hold.
    recoil: Recoil {
        strike_speed: 70.0,
        view: ViewKick {
            climb_per_shot: 0.0080,
            climb_max: 0.068,
            wander_per_shot: 0.0044,
            shove_per_shot: 0.0095,
            shove_max: 0.038,
            recover_speed: 7.4,
        },
        weapon: WeaponKick {
            slide_back: 0.12,
            lift: 0.0,
            pitch_deg: 0.2,
            roll_deg: 0.8,
            yaw_deg: 0.6,
            settle_speed: 15.0,
        },
        // About two thirds the AK's, pitched higher and gone sooner -- a
        // lighter round, and a faster cyclic rate that would otherwise pile
        // one ring-down on the next.
        shock: ShockKick {
            view_pitch_deg: 0.06,
            view_yaw_deg: 0.05,
            view_roll_deg: 0.11,
            gun_pitch_deg: 0.23,
            gun_yaw_deg: 0.17,
            gun_roll_deg: 0.34,
            gun_shift_m: 0.0008,
            freq_pitch: 14.0,
            freq_yaw: 10.5,
            freq_roll: 12.5,
            decay: 22.0,
        },
    },
};

/// Aligned by hand in Blender to the convention in
/// `zombie_assets/WEAPON_ALIGNMENT.md`. Its two sights agree to 0.1mm over a
/// 426mm sight radius, and the flash hider and magazine measure 23.8mm and
/// 24.8mm against the real rifle's 22mm and 25.4mm, which is the check that
/// the scale is right rather than merely plausible.
///
/// Socket_Aim is the centre of the rear aperture, 84.7mm over the bore
/// against the M4's 61.3mm: the download's flip-up sights are modelled tall.
/// ADS is unaffected, being subtracted from Socket_Aim, but the weapon rides
/// lower in the sighted frame than the M4 does.
static SCAR_L: WeaponDef = WeaponDef {
    name: "SCAR-L",
    mesh_path: "models/scar_l/mesh/scar_l.glb",
    // Socket_Aim sits 0.6585 down this weapon against the M4's 0.5125, so the
    // muzzle goes 146mm further out to leave the sight the same distance from
    // the eye at rest.
    // rest_offset.y places the bore, and this weapon's sight sits higher
    // over it than the M4's, so the whole thing drops by the difference:
    // every gun then carries its sight 40.5mm under the eye, as the M4
    // does. Copying the M4's own -0.1018 put the AUG's eyepiece 23mm under
    // the eye, where it filled the screen.
    rest_offset: Vec3::new(0.160, -0.1252, -0.7720),
    ready_offset: Vec3::new(0.071, -0.1702, -0.682),
    ads_eye_relief: 0.11,
    // About a kilo heavier than the M4, so it comes up slower.
    ads_speed: AdsSpeed { from_ready: 14.0, from_rest: 8.0 },
    hold_ads_fraction: 0.280,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0547, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.018,
    ads_settle_miss_deg: 1.0,
    aim_fov_multiplier: 0.75,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 22.0,
    autopilot_sprint_approach_distance: 14.0,
    autopilot_sprint_pathing_distance: 9.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    // Its own files, which are copies of the M4's until the SCAR has
    // recordings of its own.
    sound: WeaponSound { fire: "audio/weapons/scar_l_fire.wav", reload: "audio/weapons/scar_l_reload.wav" },
    reload_duration: 2.2,
    damage_scale: 0.85,
    class: WeaponClass::Rifle,
    // The real SCAR-L is 625rpm against the M416's 750. The M4 here runs at
    // 571, so holding that ratio puts this one at 476.
    fire_rate: 0.126,
    magazine: 40,
    // Safe, semi, auto. No burst on a Mk16.
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo556,
    // The M4's, kicking a little harder per shot and ringing lower and longer
    // for the extra mass, which the slower cyclic rate leaves room for.
    recoil: Recoil {
        strike_speed: 70.0,
        view: ViewKick {
            climb_per_shot: 0.0090,
            climb_max: 0.072,
            wander_per_shot: 0.0049,
            shove_per_shot: 0.0106,
            shove_max: 0.042,
            recover_speed: 7.4,
        },
        weapon: WeaponKick {
            slide_back: 0.13,
            lift: 0.0,
            pitch_deg: 0.24,
            roll_deg: 0.85,
            yaw_deg: 0.6,
            settle_speed: 14.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.07,
            view_yaw_deg: 0.055,
            view_roll_deg: 0.12,
            gun_pitch_deg: 0.26,
            gun_yaw_deg: 0.19,
            gun_roll_deg: 0.37,
            gun_shift_m: 0.0009,
            freq_pitch: 12.5,
            freq_yaw: 9.5,
            freq_roll: 11.5,
            decay: 20.0,
        },
    },
};

/// The download was already muzzle-forward, Z-up and in metres -- the flash
/// hider measures 22.3mm against the real rifle's 22 -- so this is a move of
/// the origin and nothing else. Two cross-sections of the barrel 42mm apart
/// put the bore on the Y axis to within 0.01mm.
///
/// A flat-top with an empty rail: the download carries no rear sight, so
/// Socket_Aim sits at the back of the receiver rail at the front post's own
/// height, which is where a carry handle or a folding sight would put the
/// aperture on a real one.
static M16A4: WeaponDef = WeaponDef {
    name: "M16A4",
    mesh_path: "models/m16a4/mesh/m16a4.glb",
    // Socket_Aim sits 0.655 down this weapon against the M4's 0.5125.
    rest_offset: Vec3::new(0.160, -0.1088, -0.7685),
    ready_offset: Vec3::new(0.071, -0.1538, -0.6785),
    ads_eye_relief: 0.11,
    // A full-length rifle: slower onto the eye than either carbine.
    ads_speed: AdsSpeed { from_ready: 12.5, from_rest: 7.0 },
    hold_ads_fraction: 0.260,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0383, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.020,
    ads_settle_miss_deg: 1.0,
    aim_fov_multiplier: 0.75,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 26.0,
    autopilot_sprint_approach_distance: 16.0,
    autopilot_sprint_pathing_distance: 10.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    // Its own files, which are copies of the M4's until the M16 has
    // recordings of its own.
    sound: WeaponSound { fire: "audio/weapons/m16a4_fire.wav", reload: "audio/weapons/m16a4_reload.wav" },
    reload_duration: 2.0,
    // A 20in barrel against the M4's 14.5: the same round, leaving faster.
    damage_scale: 0.90,
    class: WeaponClass::Rifle,
    // 800rpm real against the M416's 750, so a shade quicker than the M4 here.
    fire_rate: 0.098,
    magazine: 45,
    // No auto on an A4. Three rounds or one, which is the whole character of
    // the rifle: a 474mm sight radius and a trigger that will not run away.
    fire_mode: FireMode::Burst,
    ammo: crate::ItemKind::Ammo556,
    // The M4's, a touch lighter per shot for the longer rifle's mass, and
    // recovering sooner because a burst ends on its own.
    recoil: Recoil {
        strike_speed: 72.0,
        view: ViewKick {
            climb_per_shot: 0.0076,
            climb_max: 0.064,
            wander_per_shot: 0.0040,
            shove_per_shot: 0.0090,
            shove_max: 0.036,
            recover_speed: 8.2,
        },
        weapon: WeaponKick {
            slide_back: 0.115,
            lift: 0.0,
            pitch_deg: 0.19,
            roll_deg: 0.72,
            yaw_deg: 0.55,
            settle_speed: 15.5,
        },
        shock: ShockKick {
            view_pitch_deg: 0.055,
            view_yaw_deg: 0.046,
            view_roll_deg: 0.10,
            gun_pitch_deg: 0.21,
            gun_yaw_deg: 0.155,
            gun_roll_deg: 0.31,
            gun_shift_m: 0.0007,
            freq_pitch: 13.0,
            freq_yaw: 10.0,
            freq_roll: 12.0,
            decay: 23.0,
        },
    },
};

/// A bullpup, and the download comes with a suppressor and a red dot already
/// on it -- neither is an attachment the game can take off. Scale is set by
/// the weapon's own 757mm, measured to where the suppressor screws on; that
/// leaves the can 189mm and 33mm across, which is a real one.
///
/// Socket_Aim is the middle of the red dot's glass, 163mm over the bore. A
/// FAMAS carries its sights high to begin with and this adds an optic on top,
/// so it sits lower in the sighted frame than anything else here.
static FAMAS: WeaponDef = WeaponDef {
    name: "FAMAS",
    mesh_path: "models/famas/mesh/famas.glb",
    rest_offset: Vec3::new(0.160, -0.2031, -0.6570),
    ready_offset: Vec3::new(0.071, -0.2481, -0.567),
    ads_eye_relief: 0.11,
    // Short and balanced over the hand: the quickest of the rifles onto the
    // eye.
    ads_speed: AdsSpeed { from_ready: 17.0, from_rest: 10.0 },
    hold_ads_fraction: 0.310,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.1326, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.013,
    ads_settle_miss_deg: 1.3,
    aim_fov_multiplier: 0.75,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 20.0,
    autopilot_sprint_approach_distance: 13.0,
    autopilot_sprint_pathing_distance: 8.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/famas_fire.wav", reload: "audio/weapons/famas_reload.wav" },
    // Bullpup: the magazine goes in behind the hand, which is slower.
    reload_duration: 2.4,
    damage_scale: 0.85,
    class: WeaponClass::Rifle,
    // The fastest cyclic rate in the game. A real F1 runs about 1000rpm
    // against the M416's 750, and the M4 here is 571, so this lands at 761.
    fire_rate: 0.079,
    magazine: 35,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo556,
    // Light per shot, but at this rate the climb stacks faster than anything
    // else, so the ceiling is what holds it rather than the step.
    recoil: Recoil {
        strike_speed: 68.0,
        view: ViewKick {
            climb_per_shot: 0.0062,
            climb_max: 0.070,
            wander_per_shot: 0.0048,
            shove_per_shot: 0.0088,
            shove_max: 0.040,
            recover_speed: 7.0,
        },
        weapon: WeaponKick {
            slide_back: 0.105,
            lift: 0.0,
            pitch_deg: 0.17,
            roll_deg: 0.66,
            yaw_deg: 0.50,
            settle_speed: 17.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.048,
            view_yaw_deg: 0.042,
            view_roll_deg: 0.095,
            gun_pitch_deg: 0.185,
            gun_yaw_deg: 0.140,
            gun_roll_deg: 0.280,
            gun_shift_m: 0.0006,
            freq_pitch: 15.5,
            freq_yaw: 11.5,
            freq_roll: 13.5,
            decay: 24.0,
        },
    },
};

/// The one download that came apart properly: fifteen named parts, so the
/// magazine needed no splitting and the sights could be found by name. The
/// artist's 148.68-degree placement yaw sat on the root node and came off
/// in one line.
///
/// 720mm overall for a G36C puts the flash hider at 22.1mm against the real
/// 22 -- the length and the muzzle agreeing is the check that the scale is
/// right. Its ghost ring and front post meet at 72.6mm over the bore, which
/// is where Socket_Aim goes.
static G36C: WeaponDef = WeaponDef {
    name: "G36C",
    mesh_path: "models/g36c/mesh/g36c.glb",
    rest_offset: Vec3::new(0.160, -0.1131, -0.5615),
    ready_offset: Vec3::new(0.071, -0.1581, -0.4715),
    ads_eye_relief: 0.11,
    // The shortest rifle here, and it comes up like one.
    ads_speed: AdsSpeed { from_ready: 16.0, from_rest: 9.5 },
    hold_ads_fraction: 0.300,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0426, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.014,
    ads_settle_miss_deg: 1.2,
    aim_fov_multiplier: 0.75,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 19.0,
    autopilot_sprint_approach_distance: 12.0,
    autopilot_sprint_pathing_distance: 8.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/g36c_fire.wav", reload: "audio/weapons/g36c_reload.wav" },
    reload_duration: 2.1,
    // A 228mm barrel: the same round, leaving slower than any other rifle
    // here.
    damage_scale: 0.80,
    class: WeaponClass::Rifle,
    // 750rpm real, the same as the M416, so the same number the M4 carries.
    fire_rate: 0.105,
    magazine: 40,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo556,
    // Short and light: more of the shot reaches the shoulder, and there is
    // less weapon to absorb it, so the gun moves more than the view does.
    recoil: Recoil {
        strike_speed: 66.0,
        view: ViewKick {
            climb_per_shot: 0.0086,
            climb_max: 0.070,
            wander_per_shot: 0.0050,
            shove_per_shot: 0.0100,
            shove_max: 0.040,
            recover_speed: 7.6,
        },
        weapon: WeaponKick {
            slide_back: 0.125,
            lift: 0.0,
            pitch_deg: 0.26,
            roll_deg: 0.92,
            yaw_deg: 0.66,
            settle_speed: 16.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.062,
            view_yaw_deg: 0.052,
            view_roll_deg: 0.115,
            gun_pitch_deg: 0.270,
            gun_yaw_deg: 0.200,
            gun_roll_deg: 0.390,
            gun_shift_m: 0.0009,
            freq_pitch: 14.5,
            freq_yaw: 11.0,
            freq_roll: 13.0,
            decay: 23.0,
        },
    },
};

/// A bullpup with no textures at all -- the download is flat-shaded material
/// colours, so the packer copies it across untouched at 1MB.
///
/// It arrived with a backdrop sphere, a camera and a lamp in the scene, and
/// an armature whose display bone reaches 4.6km from the gun. All of that is
/// scenery and none of it ships.
///
/// 746mm overall puts its muzzle device at 17mm across, and Socket_Aim is
/// the aperture in the carry handle, 86.8mm over the bore.
static QBZ95: WeaponDef = WeaponDef {
    name: "QBZ-95",
    mesh_path: "models/qbz95/mesh/qbz95.glb",
    rest_offset: Vec3::new(0.160, -0.1273, -0.6285),
    ready_offset: Vec3::new(0.071, -0.1723, -0.5385),
    ads_eye_relief: 0.11,
    ads_speed: AdsSpeed { from_ready: 15.0, from_rest: 8.5 },
    hold_ads_fraction: 0.290,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0568, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.016,
    ads_settle_miss_deg: 1.1,
    aim_fov_multiplier: 0.75,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 22.0,
    autopilot_sprint_approach_distance: 14.0,
    autopilot_sprint_pathing_distance: 9.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/qbz95_fire.wav", reload: "audio/weapons/qbz95_reload.wav" },
    // Bullpup: the magazine goes in behind the hand.
    reload_duration: 2.3,
    damage_scale: 0.85,
    class: WeaponClass::Rifle,
    // 650rpm real against the M416's 750, which lands at 495 on the scale
    // the M4's 571 sets.
    fire_rate: 0.121,
    magazine: 40,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo556,
    // A bullpup fires into the shoulder rather than above it, so less of the
    // shot becomes climb and more of it becomes a shove.
    recoil: Recoil {
        strike_speed: 70.0,
        view: ViewKick {
            climb_per_shot: 0.0074,
            climb_max: 0.062,
            wander_per_shot: 0.0042,
            shove_per_shot: 0.0104,
            shove_max: 0.044,
            recover_speed: 7.8,
        },
        weapon: WeaponKick {
            slide_back: 0.118,
            lift: 0.0,
            pitch_deg: 0.18,
            roll_deg: 0.70,
            yaw_deg: 0.56,
            settle_speed: 15.5,
        },
        shock: ShockKick {
            view_pitch_deg: 0.052,
            view_yaw_deg: 0.048,
            view_roll_deg: 0.105,
            gun_pitch_deg: 0.200,
            gun_yaw_deg: 0.160,
            gun_roll_deg: 0.300,
            gun_shift_m: 0.0008,
            freq_pitch: 13.5,
            freq_yaw: 10.5,
            freq_roll: 12.5,
            decay: 22.0,
        },
    },
};

/// A bullpup with a 1.5x optic sitting on it, which the game cannot take off
/// any more than it can the FAMAS's red dot. Socket_Aim is the middle of the
/// eyepiece, 78.6mm over the bore, and aim_fov_multiplier carries the
/// magnification instead of the scope view -- there is no second camera for
/// a sight this weak.
///
/// 790mm overall puts the muzzle device at 25.4mm across.
static AUG_A3: WeaponDef = WeaponDef {
    name: "AUG A3",
    mesh_path: "models/aug_a3/mesh/aug_a3.glb",
    rest_offset: Vec3::new(0.160, -0.1191, -0.5185),
    ready_offset: Vec3::new(0.071, -0.1641, -0.4285),
    // An eyepiece, not an aperture: the eye sits further back than on irons.
    ads_eye_relief: 0.085,
    ads_speed: AdsSpeed { from_ready: 13.5, from_rest: 7.5 },
    hold_ads_fraction: 0.270,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0486, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.017,
    ads_settle_miss_deg: 0.9,
    // 1.5x, from the glass that is modelled on it.
    aim_fov_multiplier: 0.62,
    autopilot_ads_min_distance: 1.5,
    autopilot_engage_range: 24.0,
    autopilot_sprint_approach_distance: 15.0,
    autopilot_sprint_pathing_distance: 10.0,
    sprint_one_handed: false,
    // Real glass, so the reticle comes on when it is sighted. The lenses
    // ship blended rather than opaque, or the sight picture is a wall.
    scoped: true,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/aug_a3_fire.wav", reload: "audio/weapons/aug_a3_reload.wav" },
    reload_duration: 2.3,
    damage_scale: 0.85,
    class: WeaponClass::Rifle,
    // 700rpm real, which is 533 on the scale the M4's 571 sets.
    fire_rate: 0.1125,
    magazine: 40,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo556,
    // Heavy for a 5.56 and balanced over the shoulder, so it is the steadiest
    // of them: the least climb per shot and the soonest back on target.
    recoil: Recoil {
        strike_speed: 72.0,
        view: ViewKick {
            climb_per_shot: 0.0068,
            climb_max: 0.058,
            wander_per_shot: 0.0038,
            shove_per_shot: 0.0092,
            shove_max: 0.038,
            recover_speed: 8.4,
        },
        weapon: WeaponKick {
            slide_back: 0.110,
            lift: 0.0,
            pitch_deg: 0.16,
            roll_deg: 0.62,
            yaw_deg: 0.48,
            settle_speed: 16.5,
        },
        shock: ShockKick {
            view_pitch_deg: 0.046,
            view_yaw_deg: 0.040,
            view_roll_deg: 0.090,
            gun_pitch_deg: 0.175,
            gun_yaw_deg: 0.135,
            gun_roll_deg: 0.265,
            gun_shift_m: 0.0007,
            freq_pitch: 13.0,
            freq_yaw: 10.0,
            freq_roll: 12.0,
            decay: 24.0,
        },
    },
};

/// 7.62, and the first thing in the game to stand beside the AK at that
/// calibre. It arrived already muzzle-forward, Z-up and 940mm long against
/// the real rifle's 943, so only its origin moved.
///
/// Its front post stands 48.1mm over the bore, which is the AK's own sight
/// height, and there is no rear sight on the rail -- so Socket_Aim sits at
/// the back of the rail at the post's height, where a sight would clamp on.
static BERYL_M762: WeaponDef = WeaponDef {
    name: "Beryl M762",
    mesh_path: "models/beryl_m762/mesh/beryl_m762.glb",
    rest_offset: Vec3::new(0.160, -0.0886, -0.7835),
    ready_offset: Vec3::new(0.071, -0.1336, -0.6935),
    ads_eye_relief: 0.12,
    // Heavier than any of the 5.56s and it comes up like it.
    ads_speed: AdsSpeed { from_ready: 11.0, from_rest: 6.5 },
    hold_ads_fraction: 0.230,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0181, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.024,
    ads_settle_miss_deg: 0.8,
    aim_fov_multiplier: 0.75,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 22.0,
    autopilot_sprint_approach_distance: 16.0,
    autopilot_sprint_pathing_distance: 10.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/beryl_m762_fire.wav", reload: "audio/weapons/beryl_m762_reload.wav" },
    reload_duration: 2.3,
    damage_scale: 1.0,
    class: WeaponClass::Rifle,
    // 700rpm real against the AK's 600, so a shade quicker than the AK's
    // 0.12 here.
    fire_rate: 0.103,
    magazine: 45,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo762,
    // The AK's round out of a lighter rifle fired faster: the same step, a
    // higher ceiling, and less time between shots to give any of it back.
    recoil: Recoil {
        strike_speed: 62.0,
        view: ViewKick {
            climb_per_shot: 0.0115,
            climb_max: 0.098,
            wander_per_shot: 0.0064,
            shove_per_shot: 0.031,
            shove_max: 0.062,
            recover_speed: 6.4,
        },
        weapon: WeaponKick {
            slide_back: 0.145,
            lift: 0.0,
            pitch_deg: 1.05,
            roll_deg: 0.64,
            yaw_deg: 0.53,
            settle_speed: 10.5,
        },
        shock: ShockKick {
            view_pitch_deg: 0.094,
            view_yaw_deg: 0.073,
            view_roll_deg: 0.176,
            gun_pitch_deg: 0.36,
            gun_yaw_deg: 0.27,
            gun_roll_deg: 0.50,
            gun_shift_m: 0.0012,
            freq_pitch: 12.5,
            freq_yaw: 9.0,
            freq_roll: 10.5,
            decay: 25.0,
        },
    },
};

/// 7.62 on an AR frame: the AK's round with a rail, a telescoping stock and
/// a trigger you can actually feel. The download has no textures and came
/// with a backdrop sphere and a stray empty twenty metres off.
///
/// Its folding rear aperture sits 54.3mm over the bore.
static ACE32: WeaponDef = WeaponDef {
    name: "ACE32",
    mesh_path: "models/ace32/mesh/ace32.glb",
    rest_offset: Vec3::new(0.160, -0.0948, -0.6985),
    ready_offset: Vec3::new(0.071, -0.1398, -0.6085),
    ads_eye_relief: 0.115,
    ads_speed: AdsSpeed { from_ready: 12.5, from_rest: 7.2 },
    hold_ads_fraction: 0.250,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0243, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.022,
    ads_settle_miss_deg: 0.9,
    aim_fov_multiplier: 0.75,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 22.0,
    autopilot_sprint_approach_distance: 15.0,
    autopilot_sprint_pathing_distance: 10.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/ace32_fire.wav", reload: "audio/weapons/ace32_reload.wav" },
    reload_duration: 2.2,
    damage_scale: 1.0,
    class: WeaponClass::Rifle,
    // 650rpm real against the AK's 600.
    fire_rate: 0.111,
    magazine: 45,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo762,
    // The AK's round through a stock that lines up with the bore, so less of
    // it turns into pitch and the rifle settles sooner.
    recoil: Recoil {
        strike_speed: 63.0,
        view: ViewKick {
            climb_per_shot: 0.0100,
            climb_max: 0.086,
            wander_per_shot: 0.0056,
            shove_per_shot: 0.028,
            shove_max: 0.056,
            recover_speed: 7.0,
        },
        weapon: WeaponKick {
            slide_back: 0.135,
            lift: 0.0,
            pitch_deg: 0.80,
            roll_deg: 0.58,
            yaw_deg: 0.48,
            settle_speed: 11.5,
        },
        shock: ShockKick {
            view_pitch_deg: 0.082,
            view_yaw_deg: 0.066,
            view_roll_deg: 0.160,
            gun_pitch_deg: 0.32,
            gun_yaw_deg: 0.24,
            gun_roll_deg: 0.45,
            gun_shift_m: 0.0011,
            freq_pitch: 12.5,
            freq_yaw: 9.0,
            freq_roll: 10.5,
            decay: 25.0,
        },
    },
};

/// 7.62 with no full auto: two rounds or one, out of a twenty-round
/// magazine. The heaviest hit of the automatic rifles and the least of them
/// you get to spend at once.
///
/// A red dot is part of the mesh; Socket_Aim is the middle of its window,
/// 73mm over the bore.
static MK47_MUTANT: WeaponDef = WeaponDef {
    name: "MK47 Mutant",
    mesh_path: "models/mk47_mutant/mesh/mk47_mutant.glb",
    rest_offset: Vec3::new(0.160, -0.1135, -0.6264),
    ready_offset: Vec3::new(0.071, -0.1585, -0.5364),
    ads_eye_relief: 0.115,
    ads_speed: AdsSpeed { from_ready: 12.0, from_rest: 7.0 },
    hold_ads_fraction: 0.240,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.043, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.022,
    ads_settle_miss_deg: 0.9,
    aim_fov_multiplier: 0.75,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 24.0,
    autopilot_sprint_approach_distance: 16.0,
    autopilot_sprint_pathing_distance: 10.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/mk47_mutant_fire.wav", reload: "audio/weapons/mk47_mutant_reload.wav" },
    reload_duration: 2.3,
    damage_scale: 1.0,
    class: WeaponClass::Rifle,
    // The AK's cyclic rate, which is all it shares with it.
    fire_rate: 0.12,
    magazine: 30,
    // No auto. A burst of two, or one at a time.
    fire_mode: FireMode::Burst,
    ammo: crate::ItemKind::Ammo762,
    // The AK's kick through an AR stock: it climbs less and shoves more, and
    // a burst of two ends before any of it stacks.
    recoil: Recoil {
        strike_speed: 64.0,
        view: ViewKick {
            climb_per_shot: 0.0098,
            climb_max: 0.082,
            wander_per_shot: 0.0054,
            shove_per_shot: 0.032,
            shove_max: 0.060,
            recover_speed: 7.4,
        },
        weapon: WeaponKick {
            slide_back: 0.138,
            lift: 0.0,
            pitch_deg: 0.72,
            roll_deg: 0.56,
            yaw_deg: 0.46,
            settle_speed: 12.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.080,
            view_yaw_deg: 0.064,
            view_roll_deg: 0.155,
            gun_pitch_deg: 0.31,
            gun_yaw_deg: 0.23,
            gun_roll_deg: 0.44,
            gun_shift_m: 0.0011,
            freq_pitch: 12.0,
            freq_yaw: 9.0,
            freq_roll: 10.5,
            decay: 25.0,
        },
    },
};

/// A 7.62 bullpup: short, heavy, and the fastest-firing thing at that
/// calibre. Thirty-six named parts, so its magazine was already its own mesh
/// and only needed renaming.
///
/// 700mm overall puts the magazine at 30.4mm wide and the muzzle device at
/// 24.2mm, both right. Its carry handle, though, stands 128mm over the bore
/// where a real one is nearer 60 -- the model's own proportion -- so
/// Socket_Aim rides at the top of the handle and the weapon hangs low.
static GROZA: WeaponDef = WeaponDef {
    name: "Groza",
    mesh_path: "models/groza/mesh/groza.glb",
    rest_offset: Vec3::new(0.160, -0.1635, -0.4415),
    ready_offset: Vec3::new(0.071, -0.2085, -0.3515),
    ads_eye_relief: 0.115,
    // Short enough to swing like an SMG.
    ads_speed: AdsSpeed { from_ready: 15.0, from_rest: 8.8 },
    hold_ads_fraction: 0.280,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.093, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.019,
    ads_settle_miss_deg: 1.0,
    aim_fov_multiplier: 0.75,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 20.0,
    autopilot_sprint_approach_distance: 13.0,
    autopilot_sprint_pathing_distance: 8.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/groza_fire.wav", reload: "audio/weapons/groza_reload.wav" },
    // Bullpup: the magazine goes in behind the hand.
    reload_duration: 2.5,
    damage_scale: 1.0,
    class: WeaponClass::Rifle,
    // 700rpm real against the AK's 600, and the heaviest round in the game
    // going out that fast.
    fire_rate: 0.103,
    magazine: 40,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo762,
    // Heavy and fired straight into the shoulder: it shoves hard and climbs
    // less than the AK, but at this rate there is no time to give any back.
    recoil: Recoil {
        strike_speed: 66.0,
        view: ViewKick {
            climb_per_shot: 0.0092,
            climb_max: 0.092,
            wander_per_shot: 0.0058,
            shove_per_shot: 0.034,
            shove_max: 0.066,
            recover_speed: 6.2,
        },
        weapon: WeaponKick {
            slide_back: 0.132,
            lift: 0.0,
            pitch_deg: 0.66,
            roll_deg: 0.60,
            yaw_deg: 0.52,
            settle_speed: 12.5,
        },
        shock: ShockKick {
            view_pitch_deg: 0.076,
            view_yaw_deg: 0.062,
            view_roll_deg: 0.150,
            gun_pitch_deg: 0.30,
            gun_yaw_deg: 0.23,
            gun_roll_deg: 0.42,
            gun_shift_m: 0.0011,
            freq_pitch: 13.0,
            freq_yaw: 9.5,
            freq_roll: 11.0,
            decay: 24.0,
        },
    },
};

/// .45 at 1091 rounds a minute: the fastest trigger in the game and the
/// smallest magazine to feed it. Thirteen rounds go in about seven tenths of
/// a second.
///
/// A holographic sight is part of the mesh; Socket_Aim is the middle of its
/// window, 70.8mm over the bore.
static VECTOR: WeaponDef = WeaponDef {
    name: "Vector",
    mesh_path: "models/vector/mesh/vector.glb",
    rest_offset: Vec3::new(0.160, -0.1113, -0.4580),
    ready_offset: Vec3::new(0.071, -0.1563, -0.368),
    ads_eye_relief: 0.11,
    // Nothing comes up faster.
    ads_speed: AdsSpeed { from_ready: 18.0, from_rest: 11.0 },
    hold_ads_fraction: 0.330,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0408, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.012,
    ads_settle_miss_deg: 1.4,
    aim_fov_multiplier: 0.80,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 16.0,
    autopilot_sprint_approach_distance: 10.0,
    autopilot_sprint_pathing_distance: 7.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/vector_fire.wav", reload: "audio/weapons/vector_reload.wav" },
    reload_duration: 2.0,
    // A short barrel and a pistol round: the least damage per shot here, and
    // it makes it up in how many of them arrive.
    damage_scale: 0.65,
    class: WeaponClass::Smg,
    // 1200rpm real against the UMP's 600, so half the UMP's 0.10.
    fire_rate: 0.055,
    magazine: 25,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo45,
    // The action drives the bolt down and back rather than straight back, and
    // the weapon is built around keeping that off the shoulder: very little
    // climb, and what there is comes back fast.
    recoil: Recoil {
        strike_speed: 58.0,
        view: ViewKick {
            climb_per_shot: 0.0042,
            climb_max: 0.048,
            wander_per_shot: 0.0040,
            shove_per_shot: 0.0070,
            shove_max: 0.032,
            recover_speed: 9.0,
        },
        weapon: WeaponKick {
            slide_back: 0.085,
            lift: 0.0,
            pitch_deg: 0.12,
            roll_deg: 0.45,
            yaw_deg: 0.36,
            settle_speed: 19.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.034,
            view_yaw_deg: 0.030,
            view_roll_deg: 0.070,
            gun_pitch_deg: 0.140,
            gun_yaw_deg: 0.110,
            gun_roll_deg: 0.210,
            gun_shift_m: 0.0005,
            freq_pitch: 17.0,
            freq_yaw: 13.0,
            freq_roll: 15.0,
            decay: 26.0,
        },
    },
};

/// 9mm at 900 rounds a minute. Eight named parts, so the magazine only
/// needed renaming, and its charging handle sits on -X, which is how the
/// left side of a real one is -- the mesh is not mirrored.
///
/// Its muzzle slice catches the hooded front sight as well as the barrel, so
/// the bore is the lower of the two clusters in it rather than the middle of
/// both. Socket_Aim is the notch in the rear drum, 84.3mm over that bore.
static MP5K: WeaponDef = WeaponDef {
    name: "MP5K",
    mesh_path: "models/mp5k/mesh/mp5k.glb",
    rest_offset: Vec3::new(0.160, -0.1248, -0.5200),
    ready_offset: Vec3::new(0.071, -0.1698, -0.43),
    ads_eye_relief: 0.11,
    ads_speed: AdsSpeed { from_ready: 17.0, from_rest: 10.0 },
    hold_ads_fraction: 0.320,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0543, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.013,
    ads_settle_miss_deg: 1.3,
    aim_fov_multiplier: 0.80,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 17.0,
    autopilot_sprint_approach_distance: 11.0,
    autopilot_sprint_pathing_distance: 7.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/mp5k_fire.wav", reload: "audio/weapons/mp5k_reload.wav" },
    reload_duration: 2.0,
    // 9mm against the UMP's .45: lighter per shot, and more of them.
    damage_scale: 0.70,
    class: WeaponClass::Smg,
    // 900rpm real against the UMP's 600.
    fire_rate: 0.067,
    magazine: 40,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo9mm,
    // A roller-delayed action and a light round: the steadiest thing in the
    // game to hold on target, which is most of why the real one is famous.
    recoil: Recoil {
        strike_speed: 56.0,
        view: ViewKick {
            climb_per_shot: 0.0048,
            climb_max: 0.050,
            wander_per_shot: 0.0036,
            shove_per_shot: 0.0075,
            shove_max: 0.030,
            recover_speed: 8.8,
        },
        weapon: WeaponKick {
            slide_back: 0.090,
            lift: 0.0,
            pitch_deg: 0.14,
            roll_deg: 0.42,
            yaw_deg: 0.34,
            settle_speed: 18.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.036,
            view_yaw_deg: 0.032,
            view_roll_deg: 0.072,
            gun_pitch_deg: 0.150,
            gun_yaw_deg: 0.115,
            gun_roll_deg: 0.220,
            gun_shift_m: 0.0005,
            freq_pitch: 16.0,
            freq_yaw: 12.0,
            freq_roll: 14.0,
            decay: 26.0,
        },
    },
};

/// The first marksman rifle that is not a bolt gun: 7.62, semi only, and it
/// hits harder than anything automatic.
///
/// It arrived 1059mm long against the real rifle's 1021 and needed a 3.6%
/// trim; the barrel at the muzzle then measures 15.9mm, which is right.
/// Socket_Aim sits at the back of its rail at the front post's height --
/// 43.5mm over the bore, and the rail is empty.
static SKS: WeaponDef = WeaponDef {
    name: "SKS",
    mesh_path: "models/sks/mesh/sks.glb",
    rest_offset: Vec3::new(0.160, -0.0840, -0.6400),
    ready_offset: Vec3::new(0.071, -0.1290, -0.55),
    ads_eye_relief: 0.12,
    // Long and nose-heavy: slower onto the eye than any of the rifles.
    ads_speed: AdsSpeed { from_ready: 9.0, from_rest: 5.5 },
    hold_ads_fraction: 0.180,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0135, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.028,
    ads_settle_miss_deg: 0.6,
    // A marksman's field: narrower than the rifles, wider than the M24.
    aim_fov_multiplier: 0.55,
    autopilot_ads_min_distance: 2.0,
    autopilot_engage_range: 30.0,
    autopilot_sprint_approach_distance: 24.0,
    autopilot_sprint_pathing_distance: 16.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/sks_fire.wav", reload: "audio/weapons/sks_reload.wav" },
    reload_duration: 2.9,
    // A longer barrel than the AK's, and one shot at a time to use it.
    damage_scale: 1.15,
    class: WeaponClass::Marksman,
    fire_rate: 0.14,
    magazine: 20,
    fire_mode: FireMode::Single,
    ammo: crate::ItemKind::Ammo762,
    // Heavy per shot, but semi only: there is always time for the sight to
    // come back before the next one.
    recoil: Recoil {
        strike_speed: 52.0,
        view: ViewKick {
            climb_per_shot: 0.0150,
            climb_max: 0.075,
            wander_per_shot: 0.0048,
            shove_per_shot: 0.030,
            shove_max: 0.050,
            recover_speed: 8.0,
        },
        weapon: WeaponKick {
            slide_back: 0.155,
            lift: 0.0,
            pitch_deg: 1.20,
            roll_deg: 0.55,
            yaw_deg: 0.45,
            settle_speed: 9.5,
        },
        shock: ShockKick {
            view_pitch_deg: 0.105,
            view_yaw_deg: 0.072,
            view_roll_deg: 0.175,
            gun_pitch_deg: 0.40,
            gun_yaw_deg: 0.26,
            gun_roll_deg: 0.50,
            gun_shift_m: 0.0014,
            freq_pitch: 11.0,
            freq_yaw: 8.0,
            freq_roll: 9.5,
            decay: 24.0,
        },
    },
};

/// A second sidearm, and a far more ordinary one than the Deagle: .45, ten
/// rounds, and a trigger you can actually run.
///
/// Its muzzle slice is the whole front of the slide rather than the barrel,
/// so the bore is set to the middle of it -- about 1.5mm low against where a
/// real 1911's bore sits, which is nothing at arm's length.
static P1911: WeaponDef = WeaponDef {
    name: "P1911",
    mesh_path: "models/p1911/mesh/p1911.glb",
    // A pistol comes to the middle of the chest, not the right shoulder.
    rest_offset: Vec3::new(0.095, -0.0292, -0.3160),
    ready_offset: Vec3::new(0.045, -0.1042, -0.226),
    ads_eye_relief: 0.14,
    ads_speed: AdsSpeed { from_ready: 21.0, from_rest: 13.0 },
    hold_ads_fraction: 0.520,
    sprint_carry: SprintCarry { offset: Vec3::new(0.03, 0.06, 0.12), yaw_deg: 12.0, pitch_deg: 60.0, roll_deg: 8.0 },
    ads_settle_miss_m: 0.007,
    ads_settle_miss_deg: 1.6,
    aim_fov_multiplier: 0.90,
    autopilot_ads_min_distance: 2.0,
    autopilot_engage_range: 16.0,
    autopilot_sprint_approach_distance: 9.0,
    autopilot_sprint_pathing_distance: 6.0,
    sprint_one_handed: true,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/p1911_fire.wav", reload: "audio/weapons/p1911_reload.wav" },
    reload_duration: 2.0,
    // Half the Deagle's, which is the trade: a third again as many rounds and
    // nearly twice the rate.
    damage_scale: 1.05,
    class: WeaponClass::Pistol,
    fire_rate: 0.14,
    magazine: 10,
    fire_mode: FireMode::Single,
    ammo: crate::ItemKind::Ammo45,
    // A pistol with nothing braced behind it, but a quarter of the Deagle's
    // round: it moves in the hands and comes back before the finger does.
    recoil: Recoil {
        strike_speed: 50.0,
        view: ViewKick {
            climb_per_shot: 0.0115,
            climb_max: 0.060,
            wander_per_shot: 0.0060,
            shove_per_shot: 0.024,
            shove_max: 0.044,
            recover_speed: 9.5,
        },
        weapon: WeaponKick {
            slide_back: 0.100,
            lift: 0.0,
            pitch_deg: 1.60,
            roll_deg: 0.50,
            yaw_deg: 0.40,
            settle_speed: 13.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.090,
            view_yaw_deg: 0.058,
            view_roll_deg: 0.140,
            gun_pitch_deg: 0.42,
            gun_yaw_deg: 0.24,
            gun_roll_deg: 0.40,
            gun_shift_m: 0.0012,
            freq_pitch: 14.0,
            freq_yaw: 10.0,
            freq_roll: 12.0,
            decay: 27.0,
        },
    },
};

/// Fifty-three rounds in a helix under the barrel, which is most of what this gun is.
///
/// Socket_Aim sits 49.6mm over the bore and 216mm back from the
/// muzzle, which is what sets the two offsets below.
static BIZON: WeaponDef = WeaponDef {
    name: "PP-19 Bizon",
    mesh_path: "models/bizon/mesh/bizon.glb",
    rest_offset: Vec3::new(0.16, -0.0901, -0.326),
    ready_offset: Vec3::new(0.071, -0.1351, -0.236),
    ads_eye_relief: 0.11,
    ads_speed: AdsSpeed { from_ready: 16.0, from_rest: 9.5 },
    hold_ads_fraction: 0.3,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0196, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.014,
    ads_settle_miss_deg: 1.2,
    aim_fov_multiplier: 0.8,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 17.0,
    autopilot_sprint_approach_distance: 11.0,
    autopilot_sprint_pathing_distance: 7.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/bizon_fire.wav", reload: "audio/weapons/bizon_reload.wav" },
    reload_duration: 2.6,
    damage_scale: 0.7,
    class: WeaponClass::Smg,
    fire_rate: 0.075,
    magazine: 53,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo9mm,
    recoil: Recoil {
        strike_speed: 56.0,
        view: ViewKick {
            climb_per_shot: 0.005,
            climb_max: 0.0525,
            wander_per_shot: 0.0038,
            shove_per_shot: 0.0079,
            shove_max: 0.0315,
            recover_speed: 8.8,
        },
        weapon: WeaponKick {
            slide_back: 0.095,
            lift: 0.0,
            pitch_deg: 0.147,
            roll_deg: 0.441,
            yaw_deg: 0.357,
            settle_speed: 18.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.038,
            view_yaw_deg: 0.034,
            view_roll_deg: 0.076,
            gun_pitch_deg: 0.158,
            gun_yaw_deg: 0.121,
            gun_roll_deg: 0.231,
            gun_shift_m: 0.00053,
            freq_pitch: 16.0,
            freq_yaw: 12.0,
            freq_roll: 14.0,
            decay: 26.0,
        },
    },
};

/// Folded it is 270mm, shorter than the Deagle is long, and it empties in a second and a quarter.
///
/// Socket_Aim sits 29.2mm over the bore and 263mm back from the
/// muzzle, which is what sets the two offsets below.
static SKORPION: WeaponDef = WeaponDef {
    name: "Skorpion",
    mesh_path: "models/skorpion/mesh/skorpion.glb",
    rest_offset: Vec3::new(0.16, -0.0697, -0.383),
    ready_offset: Vec3::new(0.071, -0.1147, -0.293),
    ads_eye_relief: 0.12,
    ads_speed: AdsSpeed { from_ready: 19.0, from_rest: 12.0 },
    hold_ads_fraction: 0.36,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, -0.0008, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.011,
    ads_settle_miss_deg: 1.5,
    aim_fov_multiplier: 0.85,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 13.0,
    autopilot_sprint_approach_distance: 8.0,
    autopilot_sprint_pathing_distance: 6.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/skorpion_fire.wav", reload: "audio/weapons/skorpion_reload.wav" },
    reload_duration: 1.9,
    damage_scale: 0.6,
    class: WeaponClass::Smg,
    fire_rate: 0.06,
    magazine: 20,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo9mm,
    recoil: Recoil {
        strike_speed: 56.0,
        view: ViewKick {
            climb_per_shot: 0.0038,
            climb_max: 0.04,
            wander_per_shot: 0.0029,
            shove_per_shot: 0.006,
            shove_max: 0.024,
            recover_speed: 8.8,
        },
        weapon: WeaponKick {
            slide_back: 0.072,
            lift: 0.0,
            pitch_deg: 0.112,
            roll_deg: 0.336,
            yaw_deg: 0.272,
            settle_speed: 18.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.029,
            view_yaw_deg: 0.026,
            view_roll_deg: 0.058,
            gun_pitch_deg: 0.12,
            gun_yaw_deg: 0.092,
            gun_roll_deg: 0.176,
            gun_shift_m: 0.0004,
            freq_pitch: 16.0,
            freq_yaw: 12.0,
            freq_roll: 14.0,
            decay: 26.0,
        },
    },
};

/// A 150mm drum of .45 and the weight to go with it: slow up, slow to reload, heavy in the hands.
///
/// Socket_Aim sits 26.2mm over the bore and 595mm back from the
/// muzzle, which is what sets the two offsets below.
static TOMMY_GUN: WeaponDef = WeaponDef {
    name: "Tommy Gun",
    mesh_path: "models/tommy_gun/mesh/tommy_gun.glb",
    rest_offset: Vec3::new(0.16, -0.0667, -0.705),
    ready_offset: Vec3::new(0.071, -0.1117, -0.615),
    ads_eye_relief: 0.11,
    ads_speed: AdsSpeed { from_ready: 12.0, from_rest: 7.0 },
    hold_ads_fraction: 0.25,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, -0.0038, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.02,
    ads_settle_miss_deg: 1.0,
    aim_fov_multiplier: 0.8,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 18.0,
    autopilot_sprint_approach_distance: 12.0,
    autopilot_sprint_pathing_distance: 8.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/tommy_gun_fire.wav", reload: "audio/weapons/tommy_gun_reload.wav" },
    reload_duration: 3.2,
    damage_scale: 0.8,
    class: WeaponClass::Smg,
    fire_rate: 0.085,
    magazine: 50,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo45,
    recoil: Recoil {
        strike_speed: 56.0,
        view: ViewKick {
            climb_per_shot: 0.007,
            climb_max: 0.0725,
            wander_per_shot: 0.0052,
            shove_per_shot: 0.0109,
            shove_max: 0.0435,
            recover_speed: 8.8,
        },
        weapon: WeaponKick {
            slide_back: 0.131,
            lift: 0.0,
            pitch_deg: 0.203,
            roll_deg: 0.609,
            yaw_deg: 0.493,
            settle_speed: 18.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.052,
            view_yaw_deg: 0.046,
            view_roll_deg: 0.104,
            gun_pitch_deg: 0.217,
            gun_yaw_deg: 0.167,
            gun_roll_deg: 0.319,
            gun_shift_m: 0.00072,
            freq_pitch: 16.0,
            freq_yaw: 12.0,
            freq_roll: 14.0,
            decay: 26.0,
        },
    },
};

/// 1250rpm. Thirty-five rounds leave in one and seven tenths of a second.
///
/// Socket_Aim sits 74.5mm over the bore and 441mm back from the
/// muzzle, which is what sets the two offsets below.
static MICRO_UZI: WeaponDef = WeaponDef {
    name: "Micro UZI",
    mesh_path: "models/micro_uzi/mesh/micro_uzi.glb",
    rest_offset: Vec3::new(0.16, -0.115, -0.551),
    ready_offset: Vec3::new(0.071, -0.1600, -0.461),
    ads_eye_relief: 0.11,
    ads_speed: AdsSpeed { from_ready: 18.5, from_rest: 11.5 },
    hold_ads_fraction: 0.34,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0445, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.011,
    ads_settle_miss_deg: 1.5,
    aim_fov_multiplier: 0.85,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 14.0,
    autopilot_sprint_approach_distance: 9.0,
    autopilot_sprint_pathing_distance: 6.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/micro_uzi_fire.wav", reload: "audio/weapons/micro_uzi_reload.wav" },
    reload_duration: 2.0,
    damage_scale: 0.6,
    class: WeaponClass::Smg,
    fire_rate: 0.048,
    magazine: 35,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo9mm,
    recoil: Recoil {
        strike_speed: 56.0,
        view: ViewKick {
            climb_per_shot: 0.0043,
            climb_max: 0.045,
            wander_per_shot: 0.0032,
            shove_per_shot: 0.0067,
            shove_max: 0.027,
            recover_speed: 8.8,
        },
        weapon: WeaponKick {
            slide_back: 0.081,
            lift: 0.0,
            pitch_deg: 0.126,
            roll_deg: 0.378,
            yaw_deg: 0.306,
            settle_speed: 18.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.032,
            view_yaw_deg: 0.029,
            view_roll_deg: 0.065,
            gun_pitch_deg: 0.135,
            gun_yaw_deg: 0.104,
            gun_roll_deg: 0.198,
            gun_shift_m: 0.00045,
            freq_pitch: 16.0,
            freq_yaw: 12.0,
            freq_roll: 14.0,
            decay: 26.0,
        },
    },
};

/// A 47-round pan sits on top of it, which is why the sight line clears the receiver by 53mm.
///
/// Socket_Aim sits 53.0mm over the bore and 879mm back from the
/// muzzle, which is what sets the two offsets below.
static DP28: WeaponDef = WeaponDef {
    name: "DP-28",
    mesh_path: "models/dp28/mesh/dp28.glb",
    rest_offset: Vec3::new(0.16, -0.0935, -0.999),
    ready_offset: Vec3::new(0.071, -0.1385, -0.909),
    ads_eye_relief: 0.12,
    ads_speed: AdsSpeed { from_ready: 8.0, from_rest: 4.5 },
    hold_ads_fraction: 0.15,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.023, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.03,
    ads_settle_miss_deg: 0.7,
    aim_fov_multiplier: 0.72,
    autopilot_ads_min_distance: 1.5,
    autopilot_engage_range: 26.0,
    autopilot_sprint_approach_distance: 20.0,
    autopilot_sprint_pathing_distance: 13.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/dp28_fire.wav", reload: "audio/weapons/dp28_reload.wav" },
    reload_duration: 4.5,
    damage_scale: 1.05,
    class: WeaponClass::Rifle,
    fire_rate: 0.109,
    magazine: 47,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo762,
    recoil: Recoil {
        strike_speed: 64.0,
        view: ViewKick {
            climb_per_shot: 0.0105,
            climb_max: 0.105,
            wander_per_shot: 0.007,
            shove_per_shot: 0.03,
            shove_max: 0.07,
            recover_speed: 5.8,
        },
        weapon: WeaponKick {
            slide_back: 0.15,
            lift: 0.0,
            pitch_deg: 0.55,
            roll_deg: 0.7,
            yaw_deg: 0.6,
            settle_speed: 9.5,
        },
        shock: ShockKick {
            view_pitch_deg: 0.085,
            view_yaw_deg: 0.07,
            view_roll_deg: 0.15,
            gun_pitch_deg: 0.34,
            gun_yaw_deg: 0.26,
            gun_roll_deg: 0.46,
            gun_shift_m: 0.0013,
            freq_pitch: 11.5,
            freq_yaw: 8.5,
            freq_roll: 10.0,
            decay: 24.0,
        },
    },
};

/// A hundred rounds, and six seconds to change them. Nothing else here trades that way.
///
/// Socket_Aim sits 101.5mm over the bore and 583mm back from the
/// muzzle, which is what sets the two offsets below.
static M249: WeaponDef = WeaponDef {
    name: "M249",
    mesh_path: "models/m249/mesh/m249.glb",
    rest_offset: Vec3::new(0.16, -0.142, -0.693),
    ready_offset: Vec3::new(0.071, -0.1870, -0.603),
    ads_eye_relief: 0.11,
    ads_speed: AdsSpeed { from_ready: 7.5, from_rest: 4.0 },
    hold_ads_fraction: 0.14,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0715, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.032,
    ads_settle_miss_deg: 0.7,
    aim_fov_multiplier: 0.75,
    autopilot_ads_min_distance: 1.5,
    autopilot_engage_range: 26.0,
    autopilot_sprint_approach_distance: 20.0,
    autopilot_sprint_pathing_distance: 13.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/m249_fire.wav", reload: "audio/weapons/m249_reload.wav" },
    reload_duration: 6.0,
    damage_scale: 0.9,
    class: WeaponClass::Rifle,
    fire_rate: 0.075,
    magazine: 100,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo556,
    recoil: Recoil {
        strike_speed: 64.0,
        view: ViewKick {
            climb_per_shot: 0.0089,
            climb_max: 0.0892,
            wander_per_shot: 0.0059,
            shove_per_shot: 0.0255,
            shove_max: 0.0595,
            recover_speed: 5.8,
        },
        weapon: WeaponKick {
            slide_back: 0.128,
            lift: 0.0,
            pitch_deg: 0.468,
            roll_deg: 0.595,
            yaw_deg: 0.51,
            settle_speed: 9.5,
        },
        shock: ShockKick {
            view_pitch_deg: 0.072,
            view_yaw_deg: 0.06,
            view_roll_deg: 0.128,
            gun_pitch_deg: 0.289,
            gun_yaw_deg: 0.221,
            gun_roll_deg: 0.391,
            gun_shift_m: 0.0011,
            freq_pitch: 11.5,
            freq_yaw: 8.5,
            freq_roll: 10.0,
            decay: 24.0,
        },
    },
};

/// Semi only, and the lowest sight line in the game at 22mm over the bore.
///
/// Socket_Aim sits 22.2mm over the bore and 518mm back from the
/// muzzle, which is what sets the two offsets below.
static MINI14: WeaponDef = WeaponDef {
    name: "Mini 14",
    mesh_path: "models/mini14/mesh/mini14.glb",
    rest_offset: Vec3::new(0.16, -0.0627, -0.628),
    ready_offset: Vec3::new(0.071, -0.1077, -0.538),
    ads_eye_relief: 0.11,
    ads_speed: AdsSpeed { from_ready: 10.0, from_rest: 6.0 },
    hold_ads_fraction: 0.19,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, -0.0078, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.026,
    ads_settle_miss_deg: 0.6,
    aim_fov_multiplier: 0.55,
    autopilot_ads_min_distance: 2.0,
    autopilot_engage_range: 30.0,
    autopilot_sprint_approach_distance: 24.0,
    autopilot_sprint_pathing_distance: 16.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/mini14_fire.wav", reload: "audio/weapons/mini14_reload.wav" },
    reload_duration: 2.6,
    damage_scale: 1.0,
    class: WeaponClass::Marksman,
    fire_rate: 0.13,
    magazine: 20,
    fire_mode: FireMode::Single,
    ammo: crate::ItemKind::Ammo556,
    recoil: Recoil {
        strike_speed: 52.0,
        view: ViewKick {
            climb_per_shot: 0.012,
            climb_max: 0.06,
            wander_per_shot: 0.0038,
            shove_per_shot: 0.024,
            shove_max: 0.04,
            recover_speed: 8.0,
        },
        weapon: WeaponKick {
            slide_back: 0.124,
            lift: 0.0,
            pitch_deg: 0.96,
            roll_deg: 0.44,
            yaw_deg: 0.36,
            settle_speed: 9.5,
        },
        shock: ShockKick {
            view_pitch_deg: 0.084,
            view_yaw_deg: 0.058,
            view_roll_deg: 0.14,
            gun_pitch_deg: 0.32,
            gun_yaw_deg: 0.208,
            gun_roll_deg: 0.4,
            gun_shift_m: 0.00112,
            freq_pitch: 11.0,
            freq_yaw: 8.0,
            freq_roll: 9.5,
            decay: 24.0,
        },
    },
};

/// A flat-top with folding irons and an empty rail, which is what the real one is for.
///
/// Socket_Aim sits 67.0mm over the bore and 619mm back from the
/// muzzle, which is what sets the two offsets below.
static MK12: WeaponDef = WeaponDef {
    name: "Mk12",
    mesh_path: "models/mk12/mesh/mk12.glb",
    rest_offset: Vec3::new(0.16, -0.1075, -0.729),
    ready_offset: Vec3::new(0.071, -0.1525, -0.639),
    ads_eye_relief: 0.11,
    ads_speed: AdsSpeed { from_ready: 9.5, from_rest: 5.8 },
    hold_ads_fraction: 0.185,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.037, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.027,
    ads_settle_miss_deg: 0.6,
    aim_fov_multiplier: 0.52,
    autopilot_ads_min_distance: 2.0,
    autopilot_engage_range: 32.0,
    autopilot_sprint_approach_distance: 25.0,
    autopilot_sprint_pathing_distance: 17.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/mk12_fire.wav", reload: "audio/weapons/mk12_reload.wav" },
    reload_duration: 2.8,
    damage_scale: 1.05,
    class: WeaponClass::Marksman,
    fire_rate: 0.13,
    magazine: 20,
    fire_mode: FireMode::Single,
    ammo: crate::ItemKind::Ammo556,
    recoil: Recoil {
        strike_speed: 52.0,
        view: ViewKick {
            climb_per_shot: 0.0127,
            climb_max: 0.0638,
            wander_per_shot: 0.0041,
            shove_per_shot: 0.0255,
            shove_max: 0.0425,
            recover_speed: 8.0,
        },
        weapon: WeaponKick {
            slide_back: 0.132,
            lift: 0.0,
            pitch_deg: 1.02,
            roll_deg: 0.468,
            yaw_deg: 0.383,
            settle_speed: 9.5,
        },
        shock: ShockKick {
            view_pitch_deg: 0.089,
            view_yaw_deg: 0.061,
            view_roll_deg: 0.149,
            gun_pitch_deg: 0.34,
            gun_yaw_deg: 0.221,
            gun_roll_deg: 0.425,
            gun_shift_m: 0.00119,
            freq_pitch: 11.0,
            freq_yaw: 8.0,
            freq_roll: 9.5,
            decay: 24.0,
        },
    },
};

/// 7.62 that will still run on auto. Twenty rounds go in under three seconds and take the aim with them.
///
/// Socket_Aim sits 28.5mm over the bore and 720mm back from the
/// muzzle, which is what sets the two offsets below.
static MK14_EBR: WeaponDef = WeaponDef {
    name: "Mk14 EBR",
    mesh_path: "models/mk14_ebr/mesh/mk14_ebr.glb",
    rest_offset: Vec3::new(0.16, -0.069, -0.83),
    ready_offset: Vec3::new(0.071, -0.1140, -0.74),
    ads_eye_relief: 0.11,
    ads_speed: AdsSpeed { from_ready: 8.5, from_rest: 5.0 },
    hold_ads_fraction: 0.17,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, -0.0015, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.03,
    ads_settle_miss_deg: 0.6,
    aim_fov_multiplier: 0.5,
    autopilot_ads_min_distance: 2.0,
    autopilot_engage_range: 32.0,
    autopilot_sprint_approach_distance: 26.0,
    autopilot_sprint_pathing_distance: 18.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/mk14_ebr_fire.wav", reload: "audio/weapons/mk14_ebr_reload.wav" },
    reload_duration: 2.9,
    damage_scale: 1.35,
    class: WeaponClass::Marksman,
    fire_rate: 0.12,
    magazine: 20,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo762,
    recoil: Recoil {
        strike_speed: 52.0,
        view: ViewKick {
            climb_per_shot: 0.0195,
            climb_max: 0.0975,
            wander_per_shot: 0.0062,
            shove_per_shot: 0.039,
            shove_max: 0.065,
            recover_speed: 8.0,
        },
        weapon: WeaponKick {
            slide_back: 0.202,
            lift: 0.0,
            pitch_deg: 1.56,
            roll_deg: 0.715,
            yaw_deg: 0.585,
            settle_speed: 9.5,
        },
        shock: ShockKick {
            view_pitch_deg: 0.137,
            view_yaw_deg: 0.094,
            view_roll_deg: 0.227,
            gun_pitch_deg: 0.52,
            gun_yaw_deg: 0.338,
            gun_roll_deg: 0.65,
            gun_shift_m: 0.00182,
            freq_pitch: 11.0,
            freq_yaw: 8.0,
            freq_roll: 9.5,
            decay: 24.0,
        },
    },
};

/// Its download was 11.45 degrees nose-down. The barrel now runs true to two hundredths of a millimetre.
///
/// Socket_Aim sits 51.7mm over the bore and 878mm back from the
/// muzzle, which is what sets the two offsets below.
static SLR: WeaponDef = WeaponDef {
    name: "SLR",
    mesh_path: "models/slr/mesh/slr.glb",
    rest_offset: Vec3::new(0.16, -0.0922, -0.998),
    ready_offset: Vec3::new(0.071, -0.1372, -0.908),
    ads_eye_relief: 0.12,
    ads_speed: AdsSpeed { from_ready: 8.5, from_rest: 5.0 },
    hold_ads_fraction: 0.17,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0217, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.03,
    ads_settle_miss_deg: 0.6,
    aim_fov_multiplier: 0.52,
    autopilot_ads_min_distance: 2.0,
    autopilot_engage_range: 32.0,
    autopilot_sprint_approach_distance: 26.0,
    autopilot_sprint_pathing_distance: 18.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/slr_fire.wav", reload: "audio/weapons/slr_reload.wav" },
    reload_duration: 2.9,
    damage_scale: 1.3,
    class: WeaponClass::Marksman,
    fire_rate: 0.12,
    magazine: 20,
    fire_mode: FireMode::Single,
    ammo: crate::ItemKind::Ammo762,
    recoil: Recoil {
        strike_speed: 52.0,
        view: ViewKick {
            climb_per_shot: 0.0187,
            climb_max: 0.0938,
            wander_per_shot: 0.006,
            shove_per_shot: 0.0375,
            shove_max: 0.0625,
            recover_speed: 8.0,
        },
        weapon: WeaponKick {
            slide_back: 0.194,
            lift: 0.0,
            pitch_deg: 1.5,
            roll_deg: 0.688,
            yaw_deg: 0.562,
            settle_speed: 9.5,
        },
        shock: ShockKick {
            view_pitch_deg: 0.131,
            view_yaw_deg: 0.09,
            view_roll_deg: 0.219,
            gun_pitch_deg: 0.5,
            gun_yaw_deg: 0.325,
            gun_roll_deg: 0.625,
            gun_shift_m: 0.00175,
            freq_pitch: 11.0,
            freq_yaw: 8.0,
            freq_roll: 9.5,
            decay: 24.0,
        },
    },
};

/// Integrally suppressed and subsonic. It gives up reach for a 9mm round and gets quiet back.
///
/// Socket_Aim sits 39.0mm over the bore and 607mm back from the
/// muzzle, which is what sets the two offsets below.
static VSS: WeaponDef = WeaponDef {
    name: "VSS Vintorez",
    mesh_path: "models/vss/mesh/vss.glb",
    rest_offset: Vec3::new(0.16, -0.0795, -0.717),
    ready_offset: Vec3::new(0.071, -0.1245, -0.627),
    ads_eye_relief: 0.11,
    ads_speed: AdsSpeed { from_ready: 13.0, from_rest: 7.5 },
    hold_ads_fraction: 0.24,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.009, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.02,
    ads_settle_miss_deg: 0.9,
    aim_fov_multiplier: 0.55,
    autopilot_ads_min_distance: 1.5,
    autopilot_engage_range: 26.0,
    autopilot_sprint_approach_distance: 20.0,
    autopilot_sprint_pathing_distance: 13.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/vss_fire.wav", reload: "audio/weapons/vss_reload.wav" },
    reload_duration: 2.4,
    damage_scale: 0.95,
    class: WeaponClass::Marksman,
    fire_rate: 0.086,
    magazine: 20,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo9mm,
    recoil: Recoil {
        strike_speed: 52.0,
        view: ViewKick {
            climb_per_shot: 0.0083,
            climb_max: 0.0413,
            wander_per_shot: 0.0026,
            shove_per_shot: 0.0165,
            shove_max: 0.0275,
            recover_speed: 8.0,
        },
        weapon: WeaponKick {
            slide_back: 0.085,
            lift: 0.0,
            pitch_deg: 0.66,
            roll_deg: 0.303,
            yaw_deg: 0.248,
            settle_speed: 9.5,
        },
        shock: ShockKick {
            view_pitch_deg: 0.058,
            view_yaw_deg: 0.04,
            view_roll_deg: 0.096,
            gun_pitch_deg: 0.22,
            gun_yaw_deg: 0.143,
            gun_roll_deg: 0.275,
            gun_shift_m: 0.00077,
            freq_pitch: 11.0,
            freq_yaw: 8.0,
            freq_roll: 9.5,
            decay: 24.0,
        },
    },
};

/// The only scope in the pile whose lenses were their own meshes, so you can see through it.
///
/// Socket_Aim sits 66.3mm over the bore and 944mm back from the
/// muzzle, which is what sets the two offsets below.
static SVD: WeaponDef = WeaponDef {
    name: "SVD Dragunov",
    mesh_path: "models/svd/mesh/svd.glb",
    rest_offset: Vec3::new(0.16, -0.1068, -0.974),
    ready_offset: Vec3::new(0.071, -0.1518, -0.884),
    ads_eye_relief: 0.03,
    ads_speed: AdsSpeed { from_ready: 6.5, from_rest: 3.8 },
    hold_ads_fraction: 0.12,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0363, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.034,
    ads_settle_miss_deg: 0.5,
    aim_fov_multiplier: 0.42,
    autopilot_ads_min_distance: 3.0,
    autopilot_engage_range: 40.0,
    autopilot_sprint_approach_distance: 30.0,
    autopilot_sprint_pathing_distance: 20.0,
    sprint_one_handed: false,
    scoped: true,
    reticle: ReticleKind::Duplex,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/svd_fire.wav", reload: "audio/weapons/svd_reload.wav" },
    reload_duration: 3.0,
    damage_scale: 1.4,
    class: WeaponClass::Marksman,
    fire_rate: 0.13,
    magazine: 20,
    fire_mode: FireMode::Single,
    ammo: crate::ItemKind::Ammo762,
    recoil: Recoil {
        strike_speed: 52.0,
        view: ViewKick {
            climb_per_shot: 0.0217,
            climb_max: 0.1087,
            wander_per_shot: 0.007,
            shove_per_shot: 0.0435,
            shove_max: 0.0725,
            recover_speed: 8.0,
        },
        weapon: WeaponKick {
            slide_back: 0.225,
            lift: 0.0,
            pitch_deg: 1.74,
            roll_deg: 0.797,
            yaw_deg: 0.652,
            settle_speed: 9.5,
        },
        shock: ShockKick {
            view_pitch_deg: 0.152,
            view_yaw_deg: 0.104,
            view_roll_deg: 0.254,
            gun_pitch_deg: 0.58,
            gun_yaw_deg: 0.377,
            gun_roll_deg: 0.725,
            gun_shift_m: 0.00203,
            freq_pitch: 11.0,
            freq_yaw: 8.0,
            freq_roll: 9.5,
            decay: 24.0,
        },
    },
};

/// Stripper clips, so there is no magazine to drop and no node tagged for one.
///
/// Socket_Aim sits 15.8mm over the bore and 633mm back from the
/// muzzle, which is what sets the two offsets below.
static MOSIN_NAGANT: WeaponDef = WeaponDef {
    name: "Mosin-Nagant",
    mesh_path: "models/mosin_nagant/mesh/mosin_nagant.glb",
    rest_offset: Vec3::new(0.16, -0.0563, -0.753),
    ready_offset: Vec3::new(0.071, -0.1013, -0.663),
    ads_eye_relief: 0.12,
    ads_speed: AdsSpeed { from_ready: 5.5, from_rest: 3.0 },
    hold_ads_fraction: 0.1,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, -0.0142, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.04,
    ads_settle_miss_deg: 0.4,
    aim_fov_multiplier: 0.46,
    autopilot_ads_min_distance: 3.0,
    autopilot_engage_range: 45.0,
    autopilot_sprint_approach_distance: 35.0,
    autopilot_sprint_pathing_distance: 24.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/mosin_nagant_fire.wav", reload: "audio/weapons/mosin_nagant_reload.wav" },
    reload_duration: 3.4,
    damage_scale: 2.9,
    class: WeaponClass::Marksman,
    fire_rate: 1.5,
    magazine: 5,
    fire_mode: FireMode::Single,
    ammo: crate::ItemKind::Ammo762,
    recoil: Recoil {
        strike_speed: 34.0,
        view: ViewKick {
            climb_per_shot: 0.0276,
            climb_max: 0.1045,
            wander_per_shot: 0.0057,
            shove_per_shot: 0.0427,
            shove_max: 0.0712,
            recover_speed: 6.2,
        },
        weapon: WeaponKick {
            slide_back: 0.199,
            lift: 0.0,
            pitch_deg: 1.995,
            roll_deg: 0.665,
            yaw_deg: 0.522,
            settle_speed: 7.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.152,
            view_yaw_deg: 0.095,
            view_roll_deg: 0.228,
            gun_pitch_deg: 0.589,
            gun_yaw_deg: 0.332,
            gun_roll_deg: 0.665,
            gun_shift_m: 0.00209,
            freq_pitch: 9.0,
            freq_yaw: 6.5,
            freq_roll: 8.0,
            decay: 22.0,
        },
    },
};

/// A rifle round through a revolver: it hits like one and reloads like a chore.
///
/// Socket_Aim sits 10.0mm over the bore and 168mm back from the
/// muzzle, which is what sets the two offsets below.
static R1895: WeaponDef = WeaponDef {
    name: "R1895",
    mesh_path: "models/r1895/mesh/r1895.glb",
    rest_offset: Vec3::new(0.095, -0.0202, -0.308),
    ready_offset: Vec3::new(0.045, -0.0952, -0.218),
    ads_eye_relief: 0.14,
    ads_speed: AdsSpeed { from_ready: 20.0, from_rest: 12.0 },
    hold_ads_fraction: 0.54,
    sprint_carry: SprintCarry { offset: Vec3::new(0.03, 0.06, 0.12), yaw_deg: 12.0, pitch_deg: 60.0, roll_deg: 8.0 },
    ads_settle_miss_m: 0.008,
    ads_settle_miss_deg: 1.7,
    aim_fov_multiplier: 0.9,
    autopilot_ads_min_distance: 2.0,
    autopilot_engage_range: 18.0,
    autopilot_sprint_approach_distance: 10.0,
    autopilot_sprint_pathing_distance: 6.0,
    sprint_one_handed: true,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/r1895_fire.wav", reload: "audio/weapons/r1895_reload.wav" },
    reload_duration: 3.0,
    damage_scale: 1.3,
    class: WeaponClass::Pistol,
    fire_rate: 0.3,
    magazine: 7,
    fire_mode: FireMode::Single,
    ammo: crate::ItemKind::Ammo762,
    recoil: Recoil {
        strike_speed: 50.0,
        view: ViewKick {
            climb_per_shot: 0.0132,
            climb_max: 0.069,
            wander_per_shot: 0.0069,
            shove_per_shot: 0.0276,
            shove_max: 0.0506,
            recover_speed: 9.5,
        },
        weapon: WeaponKick {
            slide_back: 0.115,
            lift: 0.0,
            pitch_deg: 1.84,
            roll_deg: 0.575,
            yaw_deg: 0.46,
            settle_speed: 13.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.103,
            view_yaw_deg: 0.067,
            view_roll_deg: 0.161,
            gun_pitch_deg: 0.483,
            gun_yaw_deg: 0.276,
            gun_roll_deg: 0.46,
            gun_shift_m: 0.00138,
            freq_pitch: 14.0,
            freq_yaw: 10.0,
            freq_roll: 12.0,
            decay: 27.0,
        },
    },
};

/// 5.7mm, which nothing else in the game takes -- carrying one means carrying its ammunition too.
///
/// Socket_Aim sits 94.3mm over the bore and 180mm back from the
/// muzzle, which is what sets the two offsets below.
static P90: WeaponDef = WeaponDef {
    name: "P90",
    mesh_path: "models/p90/mesh/p90.glb",
    rest_offset: Vec3::new(0.16, -0.1348, -0.29),
    ready_offset: Vec3::new(0.071, -0.1798, -0.2),
    ads_eye_relief: 0.11,
    ads_speed: AdsSpeed { from_ready: 17.0, from_rest: 10.0 },
    hold_ads_fraction: 0.32,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.0643, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.012,
    ads_settle_miss_deg: 1.3,
    aim_fov_multiplier: 0.82,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 18.0,
    autopilot_sprint_approach_distance: 12.0,
    autopilot_sprint_pathing_distance: 8.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/p90_fire.wav", reload: "audio/weapons/p90_reload.wav" },
    reload_duration: 2.3,
    damage_scale: 0.65,
    class: WeaponClass::Smg,
    fire_rate: 0.063,
    magazine: 50,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo57mm,
    recoil: Recoil {
        strike_speed: 56.0,
        view: ViewKick {
            climb_per_shot: 0.0038,
            climb_max: 0.04,
            wander_per_shot: 0.0029,
            shove_per_shot: 0.006,
            shove_max: 0.024,
            recover_speed: 8.8,
        },
        weapon: WeaponKick {
            slide_back: 0.072,
            lift: 0.0,
            pitch_deg: 0.112,
            roll_deg: 0.336,
            yaw_deg: 0.272,
            settle_speed: 18.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.029,
            view_yaw_deg: 0.026,
            view_roll_deg: 0.058,
            gun_pitch_deg: 0.12,
            gun_yaw_deg: 0.092,
            gun_roll_deg: 0.176,
            gun_shift_m: 0.0004,
            freq_pitch: 16.0,
            freq_yaw: 12.0,
            freq_roll: 14.0,
            decay: 26.0,
        },
    },
};

/// Two barrels 23mm apart, the upper one first. It throws a single projectile: there is no shot pattern in the game yet.
///
/// Socket_Aim sits 17.6mm over the bore and 750mm back from the
/// muzzle, which is what sets the two offsets below.
static S686: WeaponDef = WeaponDef {
    name: "S686",
    mesh_path: "models/s686/mesh/s686.glb",
    rest_offset: Vec3::new(0.16, -0.0581, -0.87),
    ready_offset: Vec3::new(0.071, -0.1031, -0.78),
    ads_eye_relief: 0.12,
    ads_speed: AdsSpeed { from_ready: 11.0, from_rest: 6.5 },
    hold_ads_fraction: 0.22,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, -0.0124, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.024,
    ads_settle_miss_deg: 1.1,
    aim_fov_multiplier: 0.8,
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 14.0,
    autopilot_sprint_approach_distance: 9.0,
    autopilot_sprint_pathing_distance: 6.0,
    sprint_one_handed: false,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/s686_fire.wav", reload: "audio/weapons/s686_reload.wav" },
    reload_duration: 3.6,
    damage_scale: 2.6,
    class: WeaponClass::Shotgun,
    fire_rate: 0.22,
    magazine: 2,
    fire_mode: FireMode::Single,
    ammo: crate::ItemKind::Ammo12g,
    recoil: Recoil {
        strike_speed: 34.0,
        view: ViewKick {
            climb_per_shot: 0.0203,
            climb_max: 0.077,
            wander_per_shot: 0.0042,
            shove_per_shot: 0.0315,
            shove_max: 0.0525,
            recover_speed: 6.2,
        },
        weapon: WeaponKick {
            slide_back: 0.147,
            lift: 0.0,
            pitch_deg: 1.47,
            roll_deg: 0.49,
            yaw_deg: 0.385,
            settle_speed: 7.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.112,
            view_yaw_deg: 0.07,
            view_roll_deg: 0.168,
            gun_pitch_deg: 0.434,
            gun_yaw_deg: 0.245,
            gun_roll_deg: 0.49,
            gun_shift_m: 0.00154,
            freq_pitch: 9.0,
            freq_yaw: 6.5,
            freq_roll: 8.0,
            decay: 22.0,
        },
    },
};

/// Standardised in Blender on import, where it arrived 29.8m long down +X with
/// non-uniform scale on a nested empty. The barrel radius after scaling came
/// out 10.5mm against the real rifle's 8.5mm, which is the check that the
/// 0.039 factor was right rather than merely plausible.
///
/// A bolt gun: it fires about as often as the others reload, so the recoil can
/// be heavy without a burst ever stacking it.
///
/// Anchored on the bore, not on Socket_RearGrip like the other two. Its grip
/// sits 68mm under the bore against the AK's 130mm, so matching hands dropped
/// the whole rifle 61mm and it read as carried low. A shouldered rifle holds
/// its bore at the eye, not its grip.
static M24: WeaponDef = WeaponDef {
    name: "M24",
    mesh_path: "models/m24/mesh/m24.glb",
    rest_offset: Vec3::new(0.160, -0.0905, -0.9770),
    ready_offset: Vec3::new(0.045, -0.1655, -0.887),
    // Closest of the three by a wide margin: the eye comes right up behind the
    // rear glass, so the scope picture fills more of the view than either
    // carbine's sight. The glass itself sits 55mm further down the tube, so
    // there is still room before the near plane at 100mm.
    ads_eye_relief: 0.02,
    ads_speed: AdsSpeed { from_ready: 5.5, from_rest: 2.575 },
    hold_ads_fraction: 0.096,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.02, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.040,
    ads_settle_miss_deg: 0.3,
    // Irons, for the same reason as the AK: the scope came off into
    // SightId::ScopeM24, which carries the magnification this used to hold.
    // Left at 0.425 the bare rifle zoomed twelve times down an empty
    // receiver, which is not what a rifle without glass does.
    aim_fov_multiplier: 0.75,
    autopilot_ads_min_distance: 3.0,
    autopilot_engage_range: 22.0,
    autopilot_sprint_approach_distance: 55.0,
    autopilot_sprint_pathing_distance: 38.0,
    // Twice the AK at every stop: the glass it carries is the one thing this
    // rifle is for.
    sprint_one_handed: false,
    scoped: true,
    reticle: ReticleKind::Duplex,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/m24_fire.wav", reload: "audio/weapons/m24_reload.wav" },
    reload_duration: 3.1,
    damage_scale: 3.2,
    class: WeaponClass::Marksman,
    fire_rate: 1.3,
    magazine: 5,
    fire_mode: FireMode::Single,
    ammo: crate::ItemKind::Ammo762,
    recoil: Recoil {
        strike_speed: 28.0,
        view: ViewKick {
            climb_per_shot: 0.055,
            climb_max: 0.200,
            wander_per_shot: 0.012,
            shove_per_shot: 0.4,
            shove_max: 0.925,
            recover_speed: 2.6,
        },
        weapon: WeaponKick {
            slide_back: 0.07,
            lift: 0.014,
            pitch_deg: 3.0,
            roll_deg: 3.5,
            yaw_deg: 3.0,
            settle_speed: 5.5,
        },
        // One heavy thump rather than a rattle, and the only one that rings
        // long enough to watch. decay 9 against strike_speed 28: this weapon
        // takes twice as long as the AK to reach the top of its kick, so the
        // shake has to last twice as long to cover it. Nothing follows it for
        // 1.3 seconds, so there is no burst to keep tidy.
        shock: ShockKick {
            view_pitch_deg: 0.16,
            view_yaw_deg: 0.11,
            view_roll_deg: 0.27,
            gun_pitch_deg: 0.62,
            gun_yaw_deg: 0.46,
            gun_roll_deg: 0.82,
            gun_shift_m: 0.0020,
            freq_pitch: 9.0,
            freq_yaw: 6.5,
            freq_roll: 7.5,
            decay: 19.0,
        },
    },
};

/// Grip 70mm under the bore, which is the M24's problem rather than the AK's:
/// matching the firing hand to the other two would put this bore 60mm lower
/// than the AK's and it reads as carried at the belly. Anchored on the bore
/// instead, like the M24, with x and z still on the hand -- those two agree
/// across all three of the others to within half a millimetre.
///
/// No scope view: the mesh carries a small optic, but it magnifies about as
/// much as the M4's carry handle, so it goes down the irons path and the
/// scope sockets are left unmarked.
static UMP45: WeaponDef = WeaponDef {
    name: "UMP-45",
    mesh_path: "models/ump45/mesh/ump45.glb",
    // x is not shared with the rifles even though the firing hand is. The
    // offset is absolute and the angle it subtends is not: the AK's muzzle is
    // 0.71m out, so 0.160 puts it 13 degrees off the view axis, while this
    // one is 0.34m out and the same number threw it 26 degrees, half off the
    // right of the screen.
    rest_offset: Vec3::new(0.090, -0.0700, -0.3375),
    ready_offset: Vec3::new(0.025, -0.1150, -0.2475),
    ads_eye_relief: 0.11,
    // Lightest of the five and the shortest: it comes up faster than either
    // carbine and far faster than the bolt gun.
    ads_speed: AdsSpeed { from_ready: 18.0, from_rest: 10.0 },
    hold_ads_fraction: 0.330,
    // Muzzle up rather than across the body, and pitch_deg positive where
    // every rifle's is negative. The lift is small: the angle is what says
    // the weapon is pointed somewhere safe, and raising it as well put the
    // whole gun up by the ear where it covered half the run.
    sprint_carry: SprintCarry { offset: Vec3::new(0.02, 0.05, 0.10), yaw_deg: 16.0, pitch_deg: 52.0, roll_deg: 10.0 },
    // Little mass to stop, so it misses the sight line by less than the
    // carbines; but almost no magnification, so it can afford to swim more.
    ads_settle_miss_m: 0.010,
    ads_settle_miss_deg: 1.4,
    aim_fov_multiplier: 0.80,
    // A submachine gun has no business reaching: it engages close, closes
    // faster than anything else here, and gives up early.
    autopilot_ads_min_distance: 1.0,
    autopilot_engage_range: 22.0,
    autopilot_sprint_approach_distance: 11.0,
    autopilot_sprint_pathing_distance: 7.0,
    sprint_one_handed: true,
    // The hole cut through the glass, inset off the pane's own rim so the rim
    // survives. Not the box below it -- that one has a window too, and
    // cutting it reads as a sight until the eye goes to the glass and finds
    // the view blocked 9mm higher up.
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: Some(HoloWindow { min: Vec2::new(-0.0100, 0.0655), max: Vec2::new(0.0110, 0.0805) }),
    sound: WeaponSound { fire: "audio/weapons/ump45_fire.wav", reload: "audio/weapons/ump45_reload.wav" },
    reload_duration: 2.0,
    damage_scale: 0.80,
    class: WeaponClass::Smg,
    fire_rate: 0.10,
    magazine: 35,
    fire_mode: FireMode::Auto,
    ammo: crate::ItemKind::Ammo45,
    // .45 ACP is a heavy, slow round, but the UMP's bolt is built to swallow
    // it: more kick per shot than the M4 and less climb over a burst, because
    // what stacks is the rate and this fires no faster than the AK.
    recoil: Recoil {
        strike_speed: 65.0,
        view: ViewKick {
            climb_per_shot: 0.0075,
            climb_max: 0.075,
            wander_per_shot: 0.0050,
            shove_per_shot: 0.012,
            shove_max: 0.040,
            recover_speed: 7.0,
        },
        weapon: WeaponKick {
            slide_back: 0.130,
            lift: 0.0,
            pitch_deg: 0.5,
            roll_deg: 0.7,
            yaw_deg: 0.7,
            settle_speed: 20.0,
        },
        shock: ShockKick {
            view_pitch_deg: 0.08,
            view_yaw_deg: 0.06,
            view_roll_deg: 0.15,
            gun_pitch_deg: 0.30,
            gun_yaw_deg: 0.22,
            gun_roll_deg: 0.42,
            gun_shift_m: 0.0010,
            freq_pitch: 13.0,
            freq_yaw: 9.0,
            freq_roll: 11.0,
            decay: 20.0,
        },
    },
};

/// The only handgun, and the only weapon here held at arm's length rather than
/// in the shoulder. Its grip is 36mm under the bore, shallower than anything
/// else, so the bore anchor matters more here than it does on the M24.
///
/// Both hands still go on it. The support hand wraps the firing hand rather
/// than reaching for a handguard, so Socket_FrontGrip sits 40mm below
/// Socket_RearGrip instead of half a metre in front of it.
static DEAGLE: WeaponDef = WeaponDef {
    name: "Desert Eagle",
    mesh_path: "models/deagle/mesh/deagle.glb",
    // Nearest the centre of the five, and the furthest out. A rifle is braced
    // on the right shoulder and lives off to that side; a pistol in two hands
    // comes to the middle of the chest and goes out on the arms.

    rest_offset: Vec3::new(0.095, -0.0450, -0.4200),
    ready_offset: Vec3::new(0.045, -0.1200, -0.33),
    // Furthest eye relief of the five: a pistol's rear sight ends up further
    // from the eye than a carbine's, because the arms set the distance rather
    // than the stock.
    ads_eye_relief: 0.14,
    // Fastest up of anything here. There is nothing to shoulder.
    ads_speed: AdsSpeed { from_ready: 20.0, from_rest: 12.0 },
    // Highest of the five by a wide margin. Two hands and no stock is three
    // points of contact short of a rifle, and the sights show every one of
    // them missing.
    hold_ads_fraction: 0.550,
    sprint_carry: SprintCarry { offset: Vec3::new(0.03, 0.06, 0.12), yaw_deg: 12.0, pitch_deg: 60.0, roll_deg: 8.0 },
    ads_settle_miss_m: 0.008,
    ads_settle_miss_deg: 1.8,
    aim_fov_multiplier: 0.90,
    autopilot_ads_min_distance: 2.0,
    autopilot_engage_range: 18.0,
    autopilot_sprint_approach_distance: 10.0,
    autopilot_sprint_pathing_distance: 6.0,
    sprint_one_handed: true,
    scoped: false,
    reticle: ReticleKind::Dot,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/deagle_fire.wav", reload: "audio/weapons/deagle_reload.wav" },
    reload_duration: 2.2,
    // Seven rounds of .50 AE. It trades everything else for what each one is
    // worth, which is a little over twice an AK round.
    damage_scale: 2.10,
    class: WeaponClass::Pistol,
    // Semi-automatic: the ceiling is the trigger finger, not the action.
    fire_rate: 0.26,
    magazine: 7,
    fire_mode: FireMode::Single,
    ammo: crate::ItemKind::Ammo45,
    // The hardest kick here, and the slowest recovery. strike_speed below the
    // M24's because this is not braced against anything -- the blow pushes
    // the hands rather than being stopped by a shoulder.
    recoil: Recoil {
        strike_speed: 40.0,
        view: ViewKick {
            climb_per_shot: 0.040,
            climb_max: 0.150,
            wander_per_shot: 0.010,
            shove_per_shot: 0.250,
            shove_max: 0.600,
            recover_speed: 4.0,
        },
        weapon: WeaponKick {
            slide_back: 0.190,
            lift: 0.016,
            pitch_deg: 4.0,
            roll_deg: 2.0,
            yaw_deg: 1.8,
            settle_speed: 9.0,
        },
        // Lowest frequencies and the longest ring-down of the five, matched to
        // a strike_speed of 40 the way the M24's 9 is matched to 28. Nothing
        // follows for a quarter of a second, so there is no burst to keep out
        // of its way.
        shock: ShockKick {
            view_pitch_deg: 0.28,
            view_yaw_deg: 0.20,
            view_roll_deg: 0.48,
            gun_pitch_deg: 1.10,
            gun_yaw_deg: 0.80,
            gun_roll_deg: 1.50,
            gun_shift_m: 0.0035,
            freq_pitch: 10.0,
            freq_yaw: 7.0,
            freq_roll: 8.0,
            decay: 13.0,
        },
    },
};

/// A weapon asked for from somewhere other than the number keys -- the touch
/// button. `switch_weapon` owns the whole rebuild, so a request goes through
/// it rather than writing `ActiveWeapon` and leaving the mesh behind.
#[derive(Resource, Default)]
pub(crate) struct WeaponSwitchRequest(pub(crate) Option<WeaponId>);

/// Rebuilds the viewmodel for the weapon the player picked.
///
/// The mesh and the IK sockets are both built against one weapon's geometry,
/// so both are thrown away and made again rather than retargeted.
pub(crate) fn switch_weapon(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut active: ResMut<ActiveWeapon>,
    mut request: ResMut<WeaponSwitchRequest>,
    mut gun: Query<(Entity, &mut Gun)>,
    mesh_parts: Query<Entity, With<GunMeshPart>>,
    sockets: Query<Entity, Or<(With<TriggerGripSocket>, With<ForeGripSocket>)>>,
) {
    // The number keys no longer name a weapon. They name a slot, and what is
    // in the slot is the player's business -- select_weapon_slot turns one
    // into the other and puts the answer here.
    let Some(wanted) = request.0.take() else { return };
    if wanted == active.0 {
        return;
    }

    let Ok((gun_entity, mut gun)) = gun.single_mut() else {
        return;
    };
    active.0 = wanted;
    let def = wanted.def();

    gun.base_offset = def.rest_offset;
    gun.ammo = def.magazine;
    gun.reload_remaining = 0.0;
    // A sidearm comes out at the ready rather than level. It is short enough
    // that holding it out buys no sight picture, and one carried straight
    // with nothing to point at reads as a threat the player is not making.
    //
    // Armed rather than forced, so everything that already puts a weapon
    // back to rest still does -- breaking into a run most of all, which a
    // standing flag would have overridden.
    if crate::inventory::is_sidearm(wanted) {
        gun.ready_armed = true;
    }

    for entity in &mesh_parts {
        commands.entity(entity).despawn();
    }
    // The gun's own sockets are glTF nodes inside the mesh, so the loop above
    // already took them with their root. What is left here is the debug rig,
    // which hangs off the camera instead -- hence try_, for the ones that are
    // normally gone by now.
    for entity in &sockets {
        commands.entity(entity).try_despawn();
    }
    spawn_gun_mesh_parts(&mut commands, &asset_server, &mut materials, gun_entity, def.mesh_path);
    debug!("[weapon] switched to {}", def.name);
}

