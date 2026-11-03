use bevy::prelude::*;

use crate::keybinds::Bind;

use crate::{
    load_gltf_scene, ActiveWeapon, AimSocket, Gun, GunMeshPart, WeaponId, WorldAssetRoot,
};

/// A sight that clamps to a rail. Each is a real mesh taken off a weapon that
/// wore one, so two red dots are two products rather than one with a skin:
/// the FAMAS's carries its line of sight 53mm over the rail and the MK47's
/// 32mm, which is twenty-one millimetres of difference in where the weapon
/// has to hang.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SightId {
    RedDotA,
    RedDotB,
    HoloA,
    HoloB,
    HoloC,
    HoloD,
    Scope2x,
    Scope3x,
    Scope4x,
    Scope6x,
    Scope8x,
    ScopeAk,
    ScopeM24,
}

pub(crate) struct SightDef {
    pub(crate) name: &'static str,
    pub(crate) mesh_path: &'static str,
    /// How far the eye sits behind the clamp. This is what decides where on
    /// the rail the sight lands: a long scope's clamp has to sit as far
    /// forward of the eye as its eyepiece reaches back.
    pub(crate) eye_back: f32,
    /// How far the line of sight sits above the rail's top face. The weapon
    /// drops by the difference between this and its own iron sight height,
    /// so every sight ends up the same distance under the player's eye.
    pub(crate) eye_rise: f32,
    /// Anchored on the two numbers already shipping: 0.75 is what every
    /// weapon sighting down irons uses, and 0.525 is the AK's four-power
    /// scope. The rest are spaced between and want a balance pass.
    pub(crate) aim_fov_multiplier: f32,
    /// How far down the bore from `Socket_Eye` the glass sits.
    ///
    /// Where the disc goes, and the one number that decides whether the
    /// eyepiece survives. The socket is the rear mouth of the tube, so a
    /// disc left there has the whole eyepiece body in front of it -- and
    /// anything in front of the disc is what the circle takes away. Put it
    /// at the glass instead and the housing the eye is behind stays, which
    /// is the part a shooter sees around the picture.
    pub(crate) window_forward: f32,
    /// The sight picture: how much of the opening the eye sees down the tube.
    ///
    /// Short of `window_radius` by the width of the rim a scope always shows,
    /// and the gap between the two is what swells into a crescent when the
    /// eye comes off the axis. Zero leaves the opening clean.
    pub(crate) picture_radius: f32,
    /// The radius of the opening the eye looks through, if this sight has a
    /// tube at all. What the depth disc is cut to.
    pub(crate) window_radius: Option<f32>,
    /// How far the eye sits behind this sight's own glass, overriding the
    /// weapon's.
    ///
    /// Eye relief belongs to the optic, not to what it is bolted to: a tube
    /// holds the eye the same distance back whichever rifle carries it. The
    /// weapon's own number is for the irons it was drawn with, and it only
    /// applies while nothing is fitted -- the same deal `aim_fov_multiplier`
    /// already strikes.
    pub(crate) ads_eye_relief: f32,
}

static RED_DOT_A: SightDef = SightDef {
    name: "Red dot",
    mesh_path: "models/sights/reddot_a/reddot_a.glb",
    eye_back: 0.045,
    eye_rise: 0.047,
    aim_fov_multiplier: 0.75,
    ads_eye_relief: 0.11,
    picture_radius: 0.0000,
    window_forward: 0.000,
    window_radius: None,
};

static RED_DOT_B: SightDef = SightDef {
    name: "Red dot II",
    mesh_path: "models/sights/reddot_b/reddot_b.glb",
    eye_back: 0.033,
    eye_rise: 0.045,
    aim_fov_multiplier: 0.75,
    ads_eye_relief: 0.11,
    picture_radius: 0.0000,
    window_forward: 0.000,
    window_radius: None,
};

static HOLO_A: SightDef = SightDef {
    name: "Holographic",
    mesh_path: "models/sights/holo_a/holo_a.glb",
    eye_back: 0.052,
    eye_rise: 0.031,
    aim_fov_multiplier: 0.75,
    ads_eye_relief: 0.11,
    picture_radius: 0.0000,
    window_forward: 0.000,
    window_radius: None,
};

static HOLO_B: SightDef = SightDef {
    name: "Holographic II",
    mesh_path: "models/sights/holo_b/holo_b.glb",
    eye_back: 0.043,
    eye_rise: 0.031,
    aim_fov_multiplier: 0.75,
    ads_eye_relief: 0.11,
    picture_radius: 0.0000,
    window_forward: 0.000,
    window_radius: None,
};

static HOLO_C: SightDef = SightDef {
    name: "Holographic III",
    mesh_path: "models/sights/holo_c/holo_c.glb",
    eye_back: 0.033,
    eye_rise: 0.048,
    aim_fov_multiplier: 0.75,
    ads_eye_relief: 0.11,
    picture_radius: 0.0000,
    window_forward: 0.000,
    window_radius: None,
};

static HOLO_D: SightDef = SightDef {
    name: "Holographic IV",
    mesh_path: "models/sights/holo_d/holo_d.glb",
    eye_back: 0.023,
    eye_rise: 0.037,
    aim_fov_multiplier: 0.75,
    ads_eye_relief: 0.11,
    picture_radius: 0.0000,
    window_forward: 0.000,
    window_radius: None,
};

static SCOPE_2X: SightDef = SightDef {
    name: "2x scope",
    mesh_path: "models/sights/scope_2x/scope_2x.glb",
    eye_back: 0.141,
    eye_rise: 0.044,
    aim_fov_multiplier: 0.620,
    ads_eye_relief: 0.12,
    picture_radius: 0.0000,
    window_forward: 0.000,
    window_radius: None,
};

static SCOPE_3X: SightDef = SightDef {
    name: "3x scope",
    mesh_path: "models/sights/scope_3x/scope_3x.glb",
    eye_back: 0.092,
    eye_rise: 0.035,
    aim_fov_multiplier: 0.560,
    ads_eye_relief: 0.12,
    picture_radius: 0.0000,
    window_forward: 0.000,
    window_radius: None,
};

static SCOPE_4X: SightDef = SightDef {
    name: "4x scope",
    mesh_path: "models/sights/scope_4x/scope_4x.glb",
    eye_back: 0.141,
    eye_rise: 0.033,
    // Not interpolated like the others: 0.525 is the number the AK's own
    // four-power scope has been shipping with since long before any of this.
    aim_fov_multiplier: 0.525,
    ads_eye_relief: 0.12,
    picture_radius: 0.0000,
    window_forward: 0.000,
    window_radius: None,
};

static SCOPE_6X: SightDef = SightDef {
    name: "6x scope",
    mesh_path: "models/sights/scope_6x/scope_6x.glb",
    eye_back: 0.105,
    eye_rise: 0.040,
    aim_fov_multiplier: 0.440,
    ads_eye_relief: 0.12,
    picture_radius: 0.0000,
    window_forward: 0.000,
    window_radius: None,
};

static SCOPE_8X: SightDef = SightDef {
    name: "8x scope",
    mesh_path: "models/sights/scope_8x/scope_8x.glb",
    eye_back: 0.198,
    eye_rise: 0.053,
    aim_fov_multiplier: 0.380,
    ads_eye_relief: 0.12,
    picture_radius: 0.0000,
    window_forward: 0.000,
    window_radius: None,
};

/// The AK's own optic, which arrived bolted to the rifle.
///
/// Its mount is deeper than any of the others: the sight sits a hand's
/// width above the rail on a bracket that reaches down the receiver's left
/// side, which is how a Soviet side-rail optic mounts and why eye_rise is
/// nearly double a red dot's.
static SCOPE_AK: SightDef = SightDef {
    name: "AK optic",
    mesh_path: "models/sights/scope_ak/scope_ak.glb",
    eye_back: 0.057,
    eye_rise: 0.045,
    aim_fov_multiplier: 0.62,
    ads_eye_relief: 0.12,
    picture_radius: 0.0000,
    window_forward: 0.000,
    window_radius: None,
};

/// The M24's own glass, which the rifle was drawn wearing.
///
/// A hunting scope rather than a military one: 320mm of tube on two rings
/// that sit low, so the line of sight is 30mm over the rail -- lower than
/// every other scope here and closer to a red dot's. The eyepiece reaches
/// 203mm back from the rings, which is what puts the clamp so far forward
/// on the rail.
///
/// Level with the 8x rather than with the rifle it came off: 0.425 is what
/// the M24 carried when the glass was part of its mesh, and through a tube
/// this long it read as less reach than the scope looks like it has.
static SCOPE_M24: SightDef = SightDef {
    name: "M24 scope",
    mesh_path: "models/sights/scope_m24/scope_m24.glb",
    eye_back: 0.203,
    eye_rise: 0.030,
    aim_fov_multiplier: 0.280,
    ads_eye_relief: 0.12,
    picture_radius: 0.0125,
    window_forward: 0.073,
    window_radius: Some(0.015),
};

impl SightId {
    /// The item that is this sight. One each: a sight the player can carry
    /// and a sight the weapon can wear are the same object, so the bag and
    /// the rail cannot disagree about which products exist.
    pub(crate) fn from_item(item: crate::items::ItemKind) -> Option<SightId> {
        use crate::items::ItemKind::*;
        Some(match item {
            RedDot => SightId::RedDotA,
            TubeDot => SightId::RedDotB,
            Holographic => SightId::HoloA,
            HoloCompact => SightId::HoloB,
            HoloHeavy => SightId::HoloC,
            HoloWide => SightId::HoloD,
            Scope2x => SightId::Scope2x,
            Scope3x => SightId::Scope3x,
            Scope4x => SightId::Scope4x,
            Scope6x => SightId::Scope6x,
            Scope8x => SightId::Scope8x,
            ScopeAk => SightId::ScopeAk,
            ScopeM24 => SightId::ScopeM24,
            _ => return None,
        })
    }

    pub(crate) const ALL: [SightId; 13] = [
        SightId::RedDotA,
        SightId::RedDotB,
        SightId::HoloA,
        SightId::HoloB,
        SightId::HoloC,
        SightId::HoloD,
        SightId::Scope2x,
        SightId::Scope3x,
        SightId::Scope4x,
        SightId::Scope6x,
        SightId::Scope8x,
        SightId::ScopeAk,
        SightId::ScopeM24,
    ];

    pub(crate) fn def(self) -> &'static SightDef {
        match self {
            SightId::RedDotA => &RED_DOT_A,
            SightId::RedDotB => &RED_DOT_B,
            SightId::HoloA => &HOLO_A,
            SightId::HoloB => &HOLO_B,
            SightId::HoloC => &HOLO_C,
            SightId::HoloD => &HOLO_D,
            SightId::Scope2x => &SCOPE_2X,
            SightId::Scope3x => &SCOPE_3X,
            SightId::Scope4x => &SCOPE_4X,
            SightId::Scope6x => &SCOPE_6X,
            SightId::Scope8x => &SCOPE_8X,
            SightId::ScopeAk => &SCOPE_AK,
            SightId::ScopeM24 => &SCOPE_M24,
        }
    }
}

/// What is clamped to the weapon the player is holding, if anything.
///
/// Derived, never authored: the bag's sight socket is what decides, so the
/// screen that fits one and the rail that carries it cannot disagree.
#[derive(Resource, Default)]
pub(crate) struct FittedSight(pub(crate) Option<SightId>);

/// `SIGHT=` fits one on everything the player is carrying, by the same number
/// the cycle key counts through. Checking where a sight sits means looking at
/// it, and a screenshot cannot press a key.
pub(crate) fn sight_from_env() -> Option<crate::items::ItemKind> {
    let id = std::env::var("SIGHT")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .and_then(|n| SightId::ALL.get(n.checked_sub(1)?).copied())?;
    crate::items::ALL_ITEMS
        .iter()
        .copied()
        .find(|item| SightId::from_item(*item) == Some(id))
}

/// The sight a weapon was drawn wearing, split off into a node of its own.
///
/// Hidden, always, so every weapon starts bare and the bag decides what it
/// carries. Kept rather than deleted because hiding is reversible and costs
/// a visibility flag, and because the mesh it holds is the same one the
/// matching attachment is made from -- there is nothing to re-align if it
/// is ever wanted back.
#[derive(Component)]
pub(crate) struct BuiltinSight;

/// The rail a sight clamps to, as a node in the weapon's own mesh.
///
/// Several of these weapons aim down a front post that a rail laid over the
/// receiver hides, so it is drawn only when something is standing on it.
#[derive(Component)]
pub(crate) struct Rail;

/// The two ends of that rail, so a sight can be slid between them.
#[derive(Component)]
pub(crate) struct RailRearSocket;

#[derive(Component)]
pub(crate) struct RailFrontSocket;

/// The weapon's own iron rear sight, parked while a sight is fitted. It keeps
/// the component so the aim point can go back to it.
#[derive(Component)]
pub(crate) struct IronAimSocket;

/// The sight currently standing on the rail, and everything under it.
#[derive(Component)]
pub(crate) struct MountedSight;

/// Keeps the sight the fitting ran against, so a weapon swap refits rather
/// than leaving the old one hanging in the air.
#[derive(Component)]
pub(crate) struct FittedFor {
    pub(crate) sight: Option<SightId>,
    /// The weapon it was fitted to. A swap leaves the mounted sight hanging
    /// on the gun entity, which outlives the mesh, so without this it stays
    /// where the last weapon's rail was.
    pub(crate) weapon: WeaponId,
}

const SIGHT_CLAMP_MARGIN: f32 = 0.010;

/// The length of tube between the eye and the glass, which is in the way.
///
/// There is one camera, so the magnification that blows up the world blows
/// up the sight with it, and the eyepiece end fills the screen it is meant
/// to be looking through. Authored in the mesh rather than worked out here:
/// which rings frame the picture is a judgement made while looking at the
/// sight, so the asset carries a node called `ads_hide` and this only reads
/// it. A sight without one keeps every part up, which is what a red dot
/// wants.
#[derive(Component)]
pub(crate) struct HideWhileSighted;

/// Tags that node wherever it turns up. No parent walk, unlike the rail
/// nodes below: nothing but a sight is called this, and the sight is
/// spawned on the weapon rather than loaded with it.
pub(crate) fn wire_ads_hidden_nodes(
    mut commands: Commands,
    new_names: Query<(Entity, &Name), Added<Name>>,
) {
    for (entity, name) in &new_names {
        if name.as_str().starts_with("ads_hide") {
            commands.entity(entity).insert(HideWhileSighted);
        }
    }
}

/// Takes it off the screen while the sights are up.
///
/// The same threshold the scope's mark appears at, so the tube clears in
/// the frame the reticle arrives in rather than one on either side of it.
pub(crate) fn hide_sight_while_sighted(
    pose: Res<crate::WeaponPose>,
    mut parts: Query<&mut Visibility, With<HideWhileSighted>>,
) {
    let wanted = if pose.sighted > crate::SCOPED_RETICLE_SIGHTED {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut visibility in &mut parts {
        if *visibility != wanted {
            *visibility = wanted;
        }
    }
}

/// Tags the rail and its ends. `wire_weapon_sockets` handles the sockets the
/// rest of the weapon already had; these are the ones only a railed weapon
/// carries, and the rail itself is a mesh node rather than a socket.
pub(crate) fn wire_rail_nodes(
    mut commands: Commands,
    child_of: Query<&ChildOf>,
    gun: Query<(), With<Gun>>,
    new_names: Query<(Entity, &Name), Added<Name>>,
) {
    for (entity, name) in &new_names {
        let tag = match name.as_str() {
            "rail" => 0,
            "Socket_RailRear" => 1,
            "Socket_RailFront" => 2,
            n if n.starts_with("builtin_sight") => 3,
            _ => continue,
        };
        let mut current = entity;
        let mut under_gun = false;
        for _ in 0..20 {
            if gun.contains(current) {
                under_gun = true;
                break;
            }
            let Ok(parent) = child_of.get(current) else { break };
            current = parent.0;
        }
        if !under_gun {
            continue;
        }
        let mut node = commands.entity(entity);
        match tag {
            0 => node.insert((Rail, Visibility::Hidden)),
            1 => node.insert(RailRearSocket),
            2 => node.insert(RailFrontSocket),
            _ => node.insert((BuiltinSight, Visibility::Hidden)),
        };

    }
}

/// How far the weapon drops at rest to keep the sight line under the eye.
#[derive(Resource, Default)]
pub(crate) struct SightDrop(pub(crate) f32);

/// Spawns the fitted sight on the rail, and moves the aim point onto it.
///
/// Everything that aims reads one `AimSocket`, so handing that component to
/// the sight's own eye is the whole integration: ADS, the holo window test and
/// the sight line all follow without knowing a sight exists.
pub(crate) fn fit_sight(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    fitted: Res<crate::sight_swap::ShownSight>,
    active: Res<ActiveWeapon>,
    mut drop: ResMut<SightDrop>,
    gun: Query<(Entity, &GlobalTransform), With<Gun>>,
    rear: Query<&GlobalTransform, With<RailRearSocket>>,
    front: Query<&GlobalTransform, With<RailFrontSocket>>,
    irons: Query<(Entity, &GlobalTransform), With<IronAimSocket>>,
    mounted: Query<(Entity, &FittedFor), With<MountedSight>>,
) {
    let Ok((gun_entity, gun_global)) = gun.single() else { return };
    let (Ok(rear), Ok(front)) = (rear.single(), front.single()) else {
        // No rail on this weapon: it sights down whatever it was born with.
        drop.0 = 0.0;
        for (entity, _) in &mounted {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Ok((iron_entity, iron_global)) = irons.single() else { return };

    let want = FittedFor { sight: fitted.0, weapon: active.0 };
    let unchanged = mounted.single().is_ok_and(|(_, at)| at.sight == want.sight && at.weapon == want.weapon);

    // All three live in the weapon's own frame, where the bore runs down -Z
    // and up is +Y, so the rail is a line in z and the slide is along it.
    let to_gun = gun_global.affine().inverse();
    let rear = to_gun.transform_point3(rear.translation());
    let front = to_gun.transform_point3(front.translation());
    let iron = to_gun.transform_point3(iron_global.translation());

    // Recomputed every frame rather than once on the change, because a weapon
    // swap keeps the gun entity and swaps the mesh under it: the rail moves
    // without the fitted sight changing at all.
    drop.0 = match fitted.0 {
        Some(sight) => (rear.y + sight.def().eye_rise - iron.y).max(0.0),
        None => 0.0,
    };
    if unchanged {
        return;
    }
    if let Ok((entity, _)) = mounted.single() {
        commands.entity(entity).despawn();
    }
    let Some(sight) = fitted.0 else {
        commands.entity(iron_entity).insert(AimSocket);
        return;
    };
    let def = sight.def();

    // Slide the sight until its own eye lands where the iron rear sight was:
    // that is the distance from the player's eye the weapon is already
    // aligned for. Then keep the clamp on the rail, which is what stops a
    // long scope hanging its foot off the end.
    let wanted = iron.z - def.eye_back;
    let clamp = wanted.clamp(
        front.z.min(rear.z) + SIGHT_CLAMP_MARGIN,
        front.z.max(rear.z) - SIGHT_CLAMP_MARGIN,
    );

    // The sight is authored with its origin on the bottom of its clamp, so
    // placing it is a translation and nothing else.
    commands.spawn((
        Name::new(format!("Sight: {}", def.name)),
        WorldAssetRoot(load_gltf_scene(&asset_server, def.mesh_path, crate::gltf_usages())),
        Transform::from_xyz(0.0, rear.y, clamp),
        GunMeshPart,
        MountedSight,
        want,
        ChildOf(gun_entity),
    ));
    commands.entity(iron_entity).remove::<AimSocket>();
    info!(
        "[sight] {} on {:?}: clamp at {:.0}mm (wanted {:.0}), rail {:.0}..{:.0}, drop {:.1}mm",
        def.name,
        active.0,
        clamp * 1000.0,
        wanted * 1000.0,
        front.z.min(rear.z) * 1000.0,
        front.z.max(rear.z) * 1000.0,
        drop.0 * 1000.0,
    );
}

/// Hands the aim point to the sight's own eye once its mesh has loaded.
///
/// Two frames apart from `fit_sight` on purpose: the glTF scene is spawned
/// asynchronously, so the eye does not exist yet when the sight is placed.
pub(crate) fn wire_sight_eye(
    mut commands: Commands,
    child_of: Query<&ChildOf>,
    mounted: Query<(), With<MountedSight>>,
    new_names: Query<(Entity, &Name), Added<Name>>,
) {
    for (entity, name) in &new_names {
        if name.as_str() != "Socket_Eye" {
            continue;
        }
        let mut current = entity;
        for _ in 0..20 {
            if mounted.contains(current) {
                commands.entity(entity).insert(AimSocket);
                break;
            }
            let Ok(parent) = child_of.get(current) else { break };
            current = parent.0;
        }
    }
}

/// The rail shows only with something standing on it -- and while a sight is
/// on its way to or from it, which is the one time an empty rail is worth
/// looking at.
pub(crate) fn show_rail_with_sight(
    fitted: Res<crate::sight_swap::ShownSight>,
    swap: Res<crate::sight_swap::SightSwap>,
    mut rails: Query<&mut Visibility, With<Rail>>,
) {
    if !fitted.is_changed() && !swap.is_changed() {
        return;
    }
    let shown = if fitted.0.is_some() || swap.running.is_some() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut rails {
        *visibility = shown;
    }
}

/// Cycles the sight on the weapon in hand, because tuning where one sits
/// means looking at it and a screenshot cannot press a button.
///
/// On U, not on B. B opens the bag, and sharing it meant every trip into the
/// bag fitted a sight on the way in -- which read as the bag putting back
/// whatever had just been taken off, and was the more confusing for the fact
/// that the attachment is the very mesh that weapon was drawn wearing.
///
/// Writes the bag rather than the fitting: everything else reads the bag, and
/// two writers would have the key and the inventory screen fighting.
pub(crate) fn cycle_fitted_sight(
    keys: Res<ButtonInput<KeyCode>>,
    loadout: Res<crate::inventory::Loadout>,
    mut attachments: ResMut<crate::inventory::Attachments>,
) {
    if !Bind::CycleSight.just_pressed(&keys) {
        return;
    }
    let sights: Vec<crate::items::ItemKind> = crate::items::ALL_ITEMS
        .iter()
        .copied()
        .filter(|item| SightId::from_item(*item).is_some())
        .collect();
    let current = attachments.get(loadout.active, crate::items::AttachSlot::Sight);
    let next = match current {
        None => sights.first().copied(),
        Some(item) => sights
            .iter()
            .position(|k| *k == item)
            .and_then(|at| sights.get(at + 1))
            .copied(),
    };
    attachments.set(loadout.active, crate::items::AttachSlot::Sight, next);
    match next {
        Some(item) => info!("[sight] {}", item.label()),
        None => info!("[sight] irons"),
    }
}

/// Drops the weapon at rest by what the fitted sight added to the sight line.
///
/// `rest_offset` is tuned per weapon so the irons sit a fixed distance under
/// the eye; a sight standing 50mm over the rail would otherwise carry the
/// sight picture that much higher and the weapon would read as held too high.
/// ADS needs none of this -- it is subtracted from the aim socket, which has
/// already moved.
pub(crate) fn drop_weapon_for_sight(
    active: Res<ActiveWeapon>,
    drop: Res<SightDrop>,
    mut gun: Query<&mut Gun>,
) {
    let Ok(mut gun) = gun.single_mut() else { return };
    let wanted = active.def().rest_offset - Vec3::Y * drop.0;
    if gun.base_offset != wanted {
        gun.base_offset = wanted;
    }
}

/// Follows what the bag says is bolted to the weapon in hand.
///
/// The inventory screen already fits and unfits into an `AttachSlot::Sight`
/// socket; this is the one line that was missing between that and the rail.
pub(crate) fn follow_fitted_attachment(
    loadout: Res<crate::inventory::Loadout>,
    attachments: Res<crate::inventory::Attachments>,
    mut fitted: ResMut<FittedSight>,
) {
    if !loadout.is_changed() && !attachments.is_changed() {
        return;
    }
    let wanted = attachments
        .get(loadout.active, crate::items::AttachSlot::Sight)
        .and_then(SightId::from_item);
    if fitted.0 != wanted {
        fitted.0 = wanted;
    }
}
