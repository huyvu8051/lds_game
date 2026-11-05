use bevy::prelude::*;

use crate::{
    load_gltf_scene, ActiveWeapon, Gun, GunMeshPart, WeaponId, WorldAssetRoot,
};

/// Something strapped to the back of the weapon.
///
/// PUBG's three, and they go in three different places on the same stock:
/// the tactical stock at the butt, the cheek pad on the comb, the bullet
/// loops down the flank under it. One anchor serves all three because all
/// three are authored from the same point -- the top of the comb, 80mm
/// forward of the butt plate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StockId {
    Tactical,
    CheekPad,
    BulletLoops,
}

pub(crate) struct StockDef {
    pub(crate) name: &'static str,
    pub(crate) mesh_path: &'static str,
}

static TACTICAL: StockDef = StockDef {
    name: "Tactical Stock",
    mesh_path: "models/stocks/tactical_stock/tactical_stock.glb",
};

static CHEEK_PAD: StockDef = StockDef {
    name: "Cheek Pad",
    mesh_path: "models/stocks/cheek_pad/cheek_pad.glb",
};

static BULLET_LOOPS: StockDef = StockDef {
    name: "Bullet Loops",
    mesh_path: "models/stocks/bullet_loops/bullet_loops.glb",
};

impl StockId {
    pub(crate) fn def(self) -> &'static StockDef {
        match self {
            StockId::Tactical => &TACTICAL,
            StockId::CheekPad => &CHEEK_PAD,
            StockId::BulletLoops => &BULLET_LOOPS,
        }
    }

    /// What this item becomes on this weapon, if that weapon takes it.
    ///
    /// Who takes what is PUBG's answer, with its guns read across to ours:
    /// the tactical stock on the carbines, the cheek pad on anything shot
    /// at distance. Nothing here is fed a round at a time any more, so the
    /// bullet loops fit nothing -- the item stays, with no weapon to go on.
    pub(crate) fn from_item(item: crate::items::ItemKind, weapon: WeaponId) -> Option<StockId> {
        use crate::items::ItemKind::*;
        use WeaponId::*;
        let id = match item {
            TacticalStock => StockId::Tactical,
            CheekPad => StockId::CheekPad,
            BulletLoops => StockId::BulletLoops,
            _ => return None,
        };
        let fits = match id {
            StockId::Tactical => matches!(weapon, M4a1),
            StockId::CheekPad => matches!(weapon, M24),
            StockId::BulletLoops => false,
        };
        fits.then_some(id)
    }
}

/// Where the stock attachment hangs, in the weapon's own frame.
///
/// Measured off each mesh rather than typed: a ray dropped on the comb 80mm
/// forward of the butt plate, by `tools/stock_anchor.py` in zombie_assets.
/// A weapon with no row here wears no stock attachment, which is the same
/// answer `from_item` gives for everything off PUBG's lists.
fn anchor(weapon: WeaponId) -> Option<Vec3> {
    use WeaponId::*;
    Some(match weapon {
        M4a1 => Vec3::new(0.0, 0.0157, 0.6770),
        M24 => Vec3::new(0.0, -0.0054, 1.1401),
        _ => return None,
    })
}

/// `STOCK=1..3` fits one on everything, counting the stock items in order.
pub(crate) fn stock_from_env() -> Option<crate::items::ItemKind> {
    let n = std::env::var("STOCK").ok()?.trim().parse::<usize>().ok()?;
    crate::items::ALL_ITEMS
        .iter()
        .copied()
        .filter(|item| item.attach_slot() == Some(crate::items::AttachSlot::Stock))
        .nth(n.checked_sub(1)?)
}

/// What is strapped to the weapon in hand, derived from the bag.
#[derive(Resource, Default)]
pub(crate) struct FittedStock(pub(crate) Option<StockId>);

#[derive(Component)]
pub(crate) struct MountedStock;

/// Keeps what the fitting ran against, so a weapon swap refits rather than
/// leaving the last one hanging where the last butt plate was.
#[derive(Component)]
pub(crate) struct FittedStockFor {
    pub(crate) stock: Option<StockId>,
    pub(crate) weapon: WeaponId,
}

pub(crate) fn follow_fitted_stock(
    loadout: Res<crate::inventory::Loadout>,
    attachments: Res<crate::inventory::Attachments>,
    active: Res<ActiveWeapon>,
    mut fitted: ResMut<FittedStock>,
) {
    if !loadout.is_changed() && !attachments.is_changed() && !active.is_changed() {
        return;
    }
    let wanted = attachments
        .get(loadout.active, crate::items::AttachSlot::Stock)
        .and_then(|item| StockId::from_item(item, active.0));
    if fitted.0 != wanted {
        fitted.0 = wanted;
    }
}

/// Spawns the fitted attachment on the stock.
///
/// A translation and nothing else: the meshes are authored in the weapon's
/// own axes, with their origin on the anchor.
pub(crate) fn fit_stock(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    fitted: Res<FittedStock>,
    active: Res<ActiveWeapon>,
    gun: Query<Entity, With<Gun>>,
    mounted: Query<(Entity, &FittedStockFor), With<MountedStock>>,
) {
    let Ok(gun_entity) = gun.single() else { return };
    let want = FittedStockFor { stock: fitted.0, weapon: active.0 };
    if mounted
        .single()
        .is_ok_and(|(_, at)| at.stock == want.stock && at.weapon == want.weapon)
    {
        return;
    }
    if let Ok((entity, _)) = mounted.single() {
        commands.entity(entity).despawn();
    }
    let (Some(stock), Some(at)) = (fitted.0, anchor(active.0)) else { return };
    let def = stock.def();
    commands.spawn((
        Name::new(format!("Stock: {}", def.name)),
        WorldAssetRoot(load_gltf_scene(&asset_server, def.mesh_path, crate::gltf_usages())),
        Transform::from_translation(at),
        GunMeshPart,
        MountedStock,
        want,
        ChildOf(gun_entity),
    ));
    info!("[stock] {} on {:?}", def.name, active.0);
}
