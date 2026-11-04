use bevy::prelude::*;

use crate::{
    gun_view::MuzzleSocket, load_gltf_scene, ActiveWeapon, Gun, GunMeshPart, WeaponId,
    WorldAssetRoot,
};

/// A device screwed onto the end of the bore.
///
/// Three cans rather than one, because a suppressor is sized to the round it
/// is quieting: the same item fits every weapon and the weapon's class picks
/// which of the three it gets.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MuzzleId {
    SuppressorSmg,
    SuppressorAr,
    SuppressorSr,
    Compensator,
    FlashHider,
}

pub(crate) struct MuzzleDef {
    pub(crate) name: &'static str,
    pub(crate) mesh_path: &'static str,
}

static SUPPRESSOR_SMG: MuzzleDef = MuzzleDef {
    name: "Suppressor (SMG)",
    mesh_path: "models/muzzles/suppressor_smg/suppressor_smg.glb",
};

static SUPPRESSOR_AR: MuzzleDef = MuzzleDef {
    name: "Suppressor (AR)",
    mesh_path: "models/muzzles/suppressor_ar/suppressor_ar.glb",
};

static SUPPRESSOR_SR: MuzzleDef = MuzzleDef {
    name: "Suppressor (Sniper)",
    mesh_path: "models/muzzles/suppressor_sr/suppressor_sr.glb",
};

static COMPENSATOR: MuzzleDef = MuzzleDef {
    name: "Compensator",
    mesh_path: "models/muzzles/compensator/compensator.glb",
};

static FLASH_HIDER: MuzzleDef = MuzzleDef {
    name: "Flash Hider",
    mesh_path: "models/muzzles/flash_hider/flash_hider.glb",
};

impl MuzzleId {
    /// What this item becomes on a weapon of this class, if anything.
    ///
    /// Three of the six muzzle items have no mesh yet -- the barrel extender
    /// and the two shotgun chokes -- and a shotgun has none of the five.
    /// `None` leaves the bore bare, which is what the item already did.
    pub(crate) fn from_item(
        item: crate::items::ItemKind,
        class: crate::weapon::WeaponClass,
    ) -> Option<MuzzleId> {
        use crate::items::ItemKind::*;
        use crate::weapon::WeaponClass as C;
        if class == C::Shotgun {
            return None;
        }
        Some(match item {
            Suppressor => match class {
                C::Pistol | C::Smg => MuzzleId::SuppressorSmg,
                C::Marksman => MuzzleId::SuppressorSr,
                _ => MuzzleId::SuppressorAr,
            },
            Compensator => MuzzleId::Compensator,
            FlashHider => MuzzleId::FlashHider,
            _ => return None,
        })
    }

    pub(crate) fn def(self) -> &'static MuzzleDef {
        match self {
            MuzzleId::SuppressorSmg => &SUPPRESSOR_SMG,
            MuzzleId::SuppressorAr => &SUPPRESSOR_AR,
            MuzzleId::SuppressorSr => &SUPPRESSOR_SR,
            MuzzleId::Compensator => &COMPENSATOR,
            MuzzleId::FlashHider => &FLASH_HIDER,
        }
    }
}

/// `MUZZLE=` fits one on everything the player is carrying, counting the
/// muzzle items the same way `SIGHT=` counts the sights.
pub(crate) fn muzzle_from_env() -> Option<crate::items::ItemKind> {
    let n = std::env::var("MUZZLE").ok()?.trim().parse::<usize>().ok()?;
    crate::items::ALL_ITEMS
        .iter()
        .copied()
        .filter(|item| item.attach_slot() == Some(crate::items::AttachSlot::Muzzle))
        .nth(n.checked_sub(1)?)
}

/// What is screwed to the weapon in hand, if anything. Derived from the bag,
/// never authored, the same deal `FittedSight` strikes.
#[derive(Resource, Default)]
pub(crate) struct FittedMuzzle(pub(crate) Option<MuzzleId>);

/// The device currently on the bore, and everything under it.
#[derive(Component)]
pub(crate) struct MountedMuzzle;

/// The new end of the bore, carried by the device.
///
/// Not named `Socket_Muzzle`: that name is wired to `MuzzleSocket` for
/// anything under the gun, and a second one would leave the query that reads
/// the bore with two to choose from and no way to pick.
#[derive(Component)]
pub(crate) struct MuzzleTipSocket;

/// Keeps what the fitting ran against, so a weapon swap refits rather than
/// leaving the last barrel's can hanging in the air.
#[derive(Component)]
pub(crate) struct FittedMuzzleFor {
    pub(crate) muzzle: Option<MuzzleId>,
    pub(crate) weapon: WeaponId,
}

/// Follows what the bag says is on the weapon in hand.
pub(crate) fn follow_fitted_muzzle(
    loadout: Res<crate::inventory::Loadout>,
    attachments: Res<crate::inventory::Attachments>,
    active: Res<ActiveWeapon>,
    mut fitted: ResMut<FittedMuzzle>,
) {
    if !loadout.is_changed() && !attachments.is_changed() && !active.is_changed() {
        return;
    }
    let class = active.def().class;
    let wanted = attachments
        .get(loadout.active, crate::items::AttachSlot::Muzzle)
        .and_then(|item| MuzzleId::from_item(item, class));
    if fitted.0 != wanted {
        fitted.0 = wanted;
    }
}

/// Spawns the fitted device on the end of the bore.
///
/// The mesh is authored with its origin on the mounting face and its length
/// down -Z, which is the weapon's own bore, so placing it is a translation
/// onto `Socket_Muzzle` and nothing else.
pub(crate) fn fit_muzzle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    fitted: Res<FittedMuzzle>,
    active: Res<ActiveWeapon>,
    gun: Query<(Entity, &GlobalTransform), With<Gun>>,
    bore: Query<&GlobalTransform, With<MuzzleSocket>>,
    mounted: Query<(Entity, &FittedMuzzleFor), With<MountedMuzzle>>,
) {
    let Ok((gun_entity, gun_global)) = gun.single() else { return };
    let Ok(bore) = bore.single() else { return };

    let want = FittedMuzzleFor { muzzle: fitted.0, weapon: active.0 };
    if mounted
        .single()
        .is_ok_and(|(_, at)| at.muzzle == want.muzzle && at.weapon == want.weapon)
    {
        return;
    }
    if let Ok((entity, _)) = mounted.single() {
        commands.entity(entity).despawn();
    }
    let Some(muzzle) = fitted.0 else { return };
    let def = muzzle.def();
    let at = gun_global.affine().inverse().transform_point3(bore.translation());

    commands.spawn((
        Name::new(format!("Muzzle: {}", def.name)),
        WorldAssetRoot(load_gltf_scene(&asset_server, def.mesh_path, crate::gltf_usages())),
        Transform::from_translation(at),
        GunMeshPart,
        MountedMuzzle,
        want,
        ChildOf(gun_entity),
    ));
    info!("[muzzle] {} on {:?}", def.name, active.0);
}

/// Tags the device's own tip once its mesh has loaded, two frames after the
/// fitting, because the glTF scene is spawned asynchronously.
pub(crate) fn wire_muzzle_tip(
    mut commands: Commands,
    child_of: Query<&ChildOf>,
    mounted: Query<(), With<MountedMuzzle>>,
    new_names: Query<(Entity, &Name), Added<Name>>,
) {
    for (entity, name) in &new_names {
        if name.as_str() != "Socket_Tip" {
            continue;
        }
        let mut current = entity;
        for _ in 0..20 {
            if mounted.contains(current) {
                commands.entity(entity).insert(MuzzleTipSocket);
                break;
            }
            let Ok(parent) = child_of.get(current) else { break };
            current = parent.0;
        }
    }
}

/// Moves the bore's end onto the device while one is fitted.
///
/// Flash, smoke and the shot all leave from `Gun::muzzle_local`, so a can
/// that did not move it would have the flash burning inside its own tube.
pub(crate) fn track_muzzle_tip(
    tip: Query<&GlobalTransform, With<MuzzleTipSocket>>,
    mut gun: Query<(&GlobalTransform, &mut Gun)>,
) {
    let Ok((gun_global, mut gun)) = gun.single_mut() else { return };
    if let Ok(tip) = tip.single() {
        gun.muzzle_local = tip.reparented_to(gun_global);
    }
}
