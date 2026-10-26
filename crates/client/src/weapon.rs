use bevy::prelude::*;

use crate::*;

/// Everything that belongs to the weapon rather than to the player holding it.
///
/// These are gathered in one place because every one of them is tuned against
/// a particular mesh: swap the mesh and all of them are wrong together. They
/// were global constants while there was only ever one gun.
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
    pub(crate) recoil: Recoil,
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
    pub(crate) const ALL: [WeaponId; 5] =
        [WeaponId::Ak47, WeaponId::M4a1, WeaponId::M24, WeaponId::Ump45, WeaponId::Deagle];

    pub(crate) fn index(self) -> usize {
        match self {
            WeaponId::Ak47 => 0,
            WeaponId::M4a1 => 1,
            WeaponId::M24 => 2,
            WeaponId::Ump45 => 3,
            WeaponId::Deagle => 4,
        }
    }

    pub(crate) fn def(self) -> &'static WeaponDef {
        match self {
            WeaponId::Ak47 => &AK47,
            WeaponId::M4a1 => &M4A1,
            WeaponId::M24 => &M24,
            WeaponId::Ump45 => &UMP45,
            WeaponId::Deagle => &DEAGLE,
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
    autopilot_ads_min_distance: 3.0,
    autopilot_engage_range: 30.0,
    autopilot_sprint_approach_distance: 22.0,
    autopilot_sprint_pathing_distance: 15.0,
    // The floor is half the default, because that is where aim sensitivity,
    // which is default/zoom against a half speed base, comes back out at
    // exactly the hip fire rate.
    sprint_one_handed: false,
    scoped: true,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/ak47_fire.wav", reload: "audio/weapons/ak47_reload.wav" },
    reload_duration: 2.3,
    damage_scale: 1.0,
    fire_rate: 0.12,
    magazine: 50,
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
            pitch_deg: 0.0,
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
    sprint_carry: SprintCarry { offset: Vec3::new(-0.05, 0.07, 0.08), yaw_deg: 40.0, pitch_deg: -14.0, roll_deg: 20.0 },
    ads_settle_miss_m: 0.014,
    ads_settle_miss_deg: 1.2,
    aim_fov_multiplier: 0.75,
    autopilot_ads_min_distance: 3.0,
    autopilot_engage_range: 30.0,
    autopilot_sprint_approach_distance: 14.0,
    autopilot_sprint_pathing_distance: 9.0,
    // Irons: the scope view never comes on, so these only have to be sane.
    sprint_one_handed: false,
    scoped: false,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/m4a1_fire.wav", reload: "audio/weapons/m4a1_reload.wav" },
    reload_duration: 1.9,
    damage_scale: 0.85,
    fire_rate: 0.105,
    magazine: 45,
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
            pitch_deg: 0.0,
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
    autopilot_ads_min_distance: 32.0,
    autopilot_engage_range: 60.0,
    autopilot_sprint_approach_distance: 55.0,
    autopilot_sprint_pathing_distance: 38.0,
    // Twice the AK at every stop: the glass it carries is the one thing this
    // rifle is for.
    sprint_one_handed: false,
    scoped: true,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/m24_fire.wav", reload: "audio/weapons/m24_reload.wav" },
    reload_duration: 3.1,
    damage_scale: 3.2,
    fire_rate: 1.3,
    magazine: 5,
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
    autopilot_ads_min_distance: 3.0,
    autopilot_engage_range: 22.0,
    autopilot_sprint_approach_distance: 11.0,
    autopilot_sprint_pathing_distance: 7.0,
    sprint_one_handed: true,
    // The hole cut through the glass, inset off the pane's own rim so the rim
    // survives. Not the box below it -- that one has a window too, and
    // cutting it reads as a sight until the eye goes to the glass and finds
    // the view blocked 9mm higher up.
    scoped: false,
    holo: Some(HoloWindow { min: Vec2::new(-0.0100, 0.0655), max: Vec2::new(0.0110, 0.0805) }),
    sound: WeaponSound { fire: "audio/weapons/ump45_fire.wav", reload: "audio/weapons/ump45_reload.wav" },
    reload_duration: 2.0,
    damage_scale: 0.80,
    fire_rate: 0.10,
    magazine: 25,
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
            pitch_deg: 0.0,
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
    autopilot_ads_min_distance: 4.0,
    autopilot_engage_range: 18.0,
    autopilot_sprint_approach_distance: 10.0,
    autopilot_sprint_pathing_distance: 6.0,
    sprint_one_handed: true,
    scoped: false,
    holo: None,
    sound: WeaponSound { fire: "audio/weapons/deagle_fire.wav", reload: "audio/weapons/deagle_reload.wav" },
    reload_duration: 2.2,
    // Seven rounds of .50 AE. It trades everything else for what each one is
    // worth, which is a little over twice an AK round.
    damage_scale: 2.10,
    // Semi-automatic: the ceiling is the trigger finger, not the action.
    fire_rate: 0.26,
    magazine: 7,
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
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut active: ResMut<ActiveWeapon>,
    mut request: ResMut<WeaponSwitchRequest>,
    mut gun: Query<(Entity, &mut Gun)>,
    mesh_parts: Query<Entity, With<GunMeshPart>>,
    sockets: Query<Entity, Or<(With<TriggerGripSocket>, With<ForeGripSocket>)>>,
) {
    let wanted = if let Some(wanted) = request.0.take() {
        wanted
    } else if keys.just_pressed(KeyCode::Digit1) {
        WeaponId::Ak47
    } else if keys.just_pressed(KeyCode::Digit2) {
        WeaponId::M4a1
    } else if keys.just_pressed(KeyCode::Digit3) {
        WeaponId::M24
    } else if keys.just_pressed(KeyCode::Digit4) {
        WeaponId::Ump45
    } else if keys.just_pressed(KeyCode::Digit5) {
        WeaponId::Deagle
    } else {
        return;
    };
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

    for entity in &mesh_parts {
        commands.entity(entity).despawn();
    }
    for entity in &sockets {
        commands.entity(entity).despawn();
    }
    spawn_gun_mesh_parts(&mut commands, &asset_server, &mut materials, gun_entity, def.mesh_path);
    debug!("[weapon] switched to {}", def.name);
}
