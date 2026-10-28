use bevy::prelude::*;

use crate::*;

/// Everything that belongs to the weapon rather than to the player holding it.
///
/// These are gathered in one place because every one of them is tuned against
/// a particular mesh: swap the mesh and all of them are wrong together. They
/// were global constants while there was only ever one gun.
/// How the trigger behaves. A bolt rifle has one, a carbine has three.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FireMode {
    Single,
    Burst,
    Auto,
}

impl FireMode {
    /// One letter. The word does not fit beside a name and a round count in
    /// 132 pixels, and the initial is what the eye is checking for anyway.
    pub(crate) fn label(self) -> &'static str {
        match self {
            FireMode::Single => "S",
            FireMode::Burst => "B",
            FireMode::Auto => "A",
        }
    }
}

/// Rounds a burst sends.
pub(crate) const BURST_ROUNDS: u32 = 3;

/// Which mode the weapon in hand is on. One index rather than one per
/// weapon: a switch that remembers each gun separately is a nicety nobody
/// asked for, and the index is clamped to whatever the weapon offers.
#[derive(Resource, Default)]
pub(crate) struct FireModeSelection {
    pub(crate) index: usize,
    pub(crate) burst_left: u32,
}

impl FireModeSelection {
    pub(crate) fn mode(&self, def: &WeaponDef) -> FireMode {
        def.fire_modes[self.index.min(def.fire_modes.len() - 1)]
    }

    pub(crate) fn next(&mut self, def: &WeaponDef) {
        self.index = (self.index + 1) % def.fire_modes.len();
        self.burst_left = 0;
    }
}

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

    /// Whether this weapon's optic is deep enough that the sight picture is
    /// the screen rather than something seen through the glass.
    ///
    /// Past about four times, the weapon model is magnified along with
    /// everything else and fills the view, so it is taken away while sighted
    /// and the reticle drawn on its own. Which is what looking through a
    /// scope looks like: the rifle is not in the picture.
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
    pub(crate) fire_rate: f32,
    pub(crate) magazine: u32,
/// The fire modes it offers, in the order the switch walks them. First is
/// what it starts on.
pub(crate) fire_modes: &'static [FireMode],
/// The calibre it feeds on, which is what the reserve count beside the
/// magazine is counting and what a reload draws from.
pub(crate) ammo: crate::ItemKind,
    pub(crate) recoil: Recoil,
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
    pub(crate) const ALL: [WeaponId; 17] = [
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
        }
    }

}

/// Standardised to match the M4: metres, barrel down -Z, origin on the bore at
/// the muzzle. The turn and the millimetres are baked into the asset's root
/// node, so the grips below are the Socket_RearGrip / Socket_FrontGrip the
/// mesh now ships with.
static AK47: WeaponDef = WeaponDef {
    name: "AK-47",
    mesh_path: "models/ak47_scope/raw/scene.gltf",
    rest_offset: Vec3::new(0.160, -0.0905, -0.7094),
    ready_offset: Vec3::new(0.045, -0.1905, -0.6194),
    ads_eye_relief: 0.12,
    ads_speed: AdsSpeed { from_ready: 8.0, from_rest: 5.5 },
    hold_ads_fraction: 0.192,
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, -0.03, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.028,
    ads_settle_miss_deg: 0.5,
    // 8x, where the optic camera used to carry 12. The whole screen is the
    // sight picture now, and a narrower field than this leaves nothing round
    // the target to read.
    aim_fov_multiplier: 0.525,
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
    fire_rate: 0.12,
    magazine: 50,
    fire_modes: &[FireMode::Auto, FireMode::Single],
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
    ready_offset: Vec3::new(0.071, -0.1718, -0.536),
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
    fire_rate: 0.105,
    magazine: 45,
    fire_modes: &[FireMode::Auto, FireMode::Burst, FireMode::Single],
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
    ready_offset: Vec3::new(0.071, -0.1952, -0.6820),
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
    // The real SCAR-L is 625rpm against the M416's 750. The M4 here runs at
    // 571, so holding that ratio puts this one at 476.
    fire_rate: 0.126,
    magazine: 40,
    // Safe, semi, auto. No burst on a Mk16.
    fire_modes: &[FireMode::Auto, FireMode::Single],
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
    ready_offset: Vec3::new(0.071, -0.1788, -0.6785),
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
    // 800rpm real against the M416's 750, so a shade quicker than the M4 here.
    fire_rate: 0.098,
    magazine: 45,
    // No auto on an A4. Three rounds or one, which is the whole character of
    // the rifle: a 474mm sight radius and a trigger that will not run away.
    fire_modes: &[FireMode::Burst, FireMode::Single],
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
    ready_offset: Vec3::new(0.071, -0.2731, -0.5670),
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
    // The fastest cyclic rate in the game. A real F1 runs about 1000rpm
    // against the M416's 750, and the M4 here is 571, so this lands at 761.
    fire_rate: 0.079,
    magazine: 35,
    fire_modes: &[FireMode::Auto, FireMode::Burst, FireMode::Single],
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
    ready_offset: Vec3::new(0.071, -0.1831, -0.4715),
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
    // 750rpm real, the same as the M416, so the same number the M4 carries.
    fire_rate: 0.105,
    magazine: 40,
    fire_modes: &[FireMode::Auto, FireMode::Single],
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
    ready_offset: Vec3::new(0.071, -0.1973, -0.5385),
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
    // 650rpm real against the M416's 750, which lands at 495 on the scale
    // the M4's 571 sets.
    fire_rate: 0.121,
    magazine: 40,
    fire_modes: &[FireMode::Auto, FireMode::Single],
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
    ready_offset: Vec3::new(0.071, -0.1891, -0.4285),
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
    // 700rpm real, which is 533 on the scale the M4's 571 sets.
    fire_rate: 0.1125,
    magazine: 40,
    fire_modes: &[FireMode::Auto, FireMode::Single],
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
    ready_offset: Vec3::new(0.071, -0.1586, -0.6935),
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
    // 700rpm real against the AK's 600, so a shade quicker than the AK's
    // 0.12 here.
    fire_rate: 0.103,
    magazine: 45,
    fire_modes: &[FireMode::Auto, FireMode::Burst, FireMode::Single],
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
    ready_offset: Vec3::new(0.071, -0.1648, -0.6085),
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
    // 650rpm real against the AK's 600.
    fire_rate: 0.111,
    magazine: 45,
    fire_modes: &[FireMode::Auto, FireMode::Single],
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
    ready_offset: Vec3::new(0.071, -0.1835, -0.5364),
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
    // The AK's cyclic rate, which is all it shares with it.
    fire_rate: 0.12,
    magazine: 30,
    // No auto. A burst of two, or one at a time.
    fire_modes: &[FireMode::Burst, FireMode::Single],
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
    ready_offset: Vec3::new(0.071, -0.2335, -0.3515),
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
    // 700rpm real against the AK's 600, and the heaviest round in the game
    // going out that fast.
    fire_rate: 0.103,
    magazine: 40,
    fire_modes: &[FireMode::Auto, FireMode::Single],
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
    ready_offset: Vec3::new(0.071, -0.1813, -0.3680),
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
    // 1200rpm real against the UMP's 600, so half the UMP's 0.10.
    fire_rate: 0.055,
    magazine: 25,
    fire_modes: &[FireMode::Auto, FireMode::Burst, FireMode::Single],
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
    ready_offset: Vec3::new(0.071, -0.1948, -0.4300),
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
    // 900rpm real against the UMP's 600.
    fire_rate: 0.067,
    magazine: 40,
    fire_modes: &[FireMode::Auto, FireMode::Burst, FireMode::Single],
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
    ready_offset: Vec3::new(0.045, -0.1905, -0.8870),
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
    // 12x, down from the optic camera's 24. Still the longest reach of the
    // five, and still what this weapon is for.
    aim_fov_multiplier: 0.425,
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
    fire_rate: 1.3,
    magazine: 5,
    fire_modes: &[FireMode::Single],
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
    ready_offset: Vec3::new(0.025, -0.1400, -0.2475),
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
    fire_rate: 0.10,
    magazine: 35,
    fire_modes: &[FireMode::Auto, FireMode::Burst, FireMode::Single],
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
    ready_offset: Vec3::new(0.045, -0.1450, -0.3300),
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
    // Semi-automatic: the ceiling is the trigger finger, not the action.
    fire_rate: 0.26,
    magazine: 7,
    fire_modes: &[FireMode::Single],
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

