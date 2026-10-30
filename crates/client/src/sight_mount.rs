use bevy::prelude::*;

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
    Scope2x,
    Scope3x,
    Scope6x,
    Scope8x,
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
}

static RED_DOT_A: SightDef = SightDef {
    name: "Red dot",
    mesh_path: "models/sights/reddot_a/reddot_a.glb",
    eye_back: 0.041,
    eye_rise: 0.053,
    aim_fov_multiplier: 0.75,
};

static RED_DOT_B: SightDef = SightDef {
    name: "Red dot II",
    mesh_path: "models/sights/reddot_b/reddot_b.glb",
    eye_back: 0.032,
    eye_rise: 0.032,
    aim_fov_multiplier: 0.75,
};

static HOLO_A: SightDef = SightDef {
    name: "Holographic",
    mesh_path: "models/sights/holo_a/holo_a.glb",
    eye_back: 0.052,
    eye_rise: 0.031,
    aim_fov_multiplier: 0.75,
};

static HOLO_B: SightDef = SightDef {
    name: "Holographic II",
    mesh_path: "models/sights/holo_b/holo_b.glb",
    eye_back: 0.043,
    eye_rise: 0.031,
    aim_fov_multiplier: 0.75,
};

static HOLO_C: SightDef = SightDef {
    name: "Holographic III",
    mesh_path: "models/sights/holo_c/holo_c.glb",
    eye_back: 0.014,
    eye_rise: 0.048,
    aim_fov_multiplier: 0.75,
};

static SCOPE_2X: SightDef = SightDef {
    name: "2x scope",
    mesh_path: "models/sights/scope_2x/scope_2x.glb",
    eye_back: 0.127,
    eye_rise: 0.044,
    aim_fov_multiplier: 0.620,
};

static SCOPE_3X: SightDef = SightDef {
    name: "3x scope",
    mesh_path: "models/sights/scope_3x/scope_3x.glb",
    eye_back: 0.092,
    eye_rise: 0.035,
    aim_fov_multiplier: 0.560,
};

static SCOPE_6X: SightDef = SightDef {
    name: "6x scope",
    mesh_path: "models/sights/scope_6x/scope_6x.glb",
    eye_back: 0.105,
    eye_rise: 0.040,
    aim_fov_multiplier: 0.440,
};

static SCOPE_8X: SightDef = SightDef {
    name: "8x scope",
    mesh_path: "models/sights/scope_8x/scope_8x.glb",
    eye_back: 0.203,
    eye_rise: 0.033,
    aim_fov_multiplier: 0.380,
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
            Scope2x => SightId::Scope2x,
            Scope3x => SightId::Scope3x,
            Scope6x => SightId::Scope6x,
            Scope8x => SightId::Scope8x,
            _ => return None,
        })
    }

    pub(crate) const ALL: [SightId; 9] = [
        SightId::RedDotA,
        SightId::RedDotB,
        SightId::HoloA,
        SightId::HoloB,
        SightId::HoloC,
        SightId::Scope2x,
        SightId::Scope3x,
        SightId::Scope6x,
        SightId::Scope8x,
    ];

    pub(crate) fn def(self) -> &'static SightDef {
        match self {
            SightId::RedDotA => &RED_DOT_A,
            SightId::RedDotB => &RED_DOT_B,
            SightId::HoloA => &HOLO_A,
            SightId::HoloB => &HOLO_B,
            SightId::HoloC => &HOLO_C,
            SightId::Scope2x => &SCOPE_2X,
            SightId::Scope3x => &SCOPE_3X,
            SightId::Scope6x => &SCOPE_6X,
            SightId::Scope8x => &SCOPE_8X,
        }
    }
}

/// Whether this weapon has somewhere for a sight to clamp.
///
/// A list rather than a field on the weapon, because what decides it is the
/// mesh: these are the ones carrying `Socket_RailRear` and `Socket_RailFront`
/// -- twelve that `add_rails.sh` built a rail onto, and six that came wearing
/// a sight and had it split off by `strip_builtin_sights.sh`, which marked
/// the rail it had been standing on. Fitting a sight to anything else puts it
/// in the bag's socket and nowhere on screen.
pub(crate) fn weapon_takes_sight(weapon: WeaponId) -> bool {
    matches!(
        weapon,
        WeaponId::Famas
            | WeaponId::Mk47Mutant
            | WeaponId::P90
            | WeaponId::Vector
            | WeaponId::M249
            | WeaponId::AugA3
            | WeaponId::Bizon
            | WeaponId::Dp28
            | WeaponId::Groza
            | WeaponId::Mini14
            | WeaponId::MosinNagant
            | WeaponId::P1911
            | WeaponId::Qbz95
            | WeaponId::R1895
            | WeaponId::S686
            | WeaponId::Skorpion
            | WeaponId::Slr
            | WeaponId::TommyGun
    )
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
            "builtin_sight" => 3,
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
    if !keys.just_pressed(KeyCode::KeyU) {
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
