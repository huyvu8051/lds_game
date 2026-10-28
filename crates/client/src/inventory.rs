//! What the player is carrying: three weapon slots, a few consumables, and
//! the boost meter those consumables feed.
//!
//! The slots are the battle-royale arrangement rather than a list of every
//! weapon in the game -- two long guns and a sidearm, and a weapon that is
//! not in one of them is not in your hands. Picking one up has to displace
//! something, which is the whole of the decision the arrangement exists to
//! create.

use bevy::prelude::*;

use crate::*;

/// Which of the three the weapon is held in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Slot {
    Primary(usize),
    Sidearm,
}

impl Slot {
    pub(crate) const ALL: [Slot; 3] = [Slot::Primary(0), Slot::Primary(1), Slot::Sidearm];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Slot::Primary(0) => "1",
            Slot::Primary(_) => "2",
            Slot::Sidearm => "3",
        }
    }
}

/// A weapon small enough for the third slot. The sidearm slot is the only one
/// that asks, and only one weapon answers yes.
pub(crate) fn is_sidearm(weapon: WeaponId) -> bool {
    matches!(weapon, WeaponId::Deagle)
}

#[derive(Resource)]
pub(crate) struct Loadout {
    pub(crate) primary: [Option<WeaponId>; 2],
    pub(crate) sidearm: Option<WeaponId>,
    pub(crate) active: Slot,
}

impl Default for Loadout {
    /// A rifle and a sidearm to start with. Starting empty would be the purer
    /// reading of a looting game and would also leave the player, and the
    /// autopilot, standing in a street with nothing to do.
    ///
    /// WEAPON= still names the weapon to begin holding, as it did when the
    /// number keys chose one directly -- every measurement script passes it.
    fn default() -> Self {
        let mut loadout = Self {
            primary: [Some(WeaponId::Ak47), None],
            sidearm: Some(WeaponId::Deagle),
            active: Slot::Primary(0),
        };
        if let Some(wanted) = std::env::var("WEAPON")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .and_then(|n| WeaponId::ALL.get(n.checked_sub(1)?).copied())
        {
            loadout.take(wanted);
        }
        loadout
    }
}

impl Loadout {
    pub(crate) fn get(&self, slot: Slot) -> Option<WeaponId> {
        match slot {
            Slot::Primary(i) => self.primary[i],
            Slot::Sidearm => self.sidearm,
        }
    }

    fn set(&mut self, slot: Slot, weapon: Option<WeaponId>) {
        match slot {
            Slot::Primary(i) => self.primary[i] = weapon,
            Slot::Sidearm => self.sidearm = weapon,
        }
    }

    /// Where a weapon would go, and what it would displace getting there.
    ///
    /// An empty slot of the right kind first, then the one in hand if it is
    /// of that kind, then the first of that kind -- so a pickup never
    /// silently replaces a weapon the player is not looking at while an empty
    /// slot stands beside it.
    fn slot_for(&self, weapon: WeaponId) -> Slot {
        if is_sidearm(weapon) {
            return Slot::Sidearm;
        }
        if let Some(i) = self.primary.iter().position(|s| s.is_none()) {
            return Slot::Primary(i);
        }
        match self.active {
            Slot::Primary(i) => Slot::Primary(i),
            Slot::Sidearm => Slot::Primary(0),
        }
    }

    pub(crate) fn holds(&self, weapon: WeaponId) -> bool {
        self.primary.iter().any(|s| *s == Some(weapon)) || self.sidearm == Some(weapon)
    }

    /// Empties a slot. Used by the backpack screen, which is the only place
    /// a weapon can be put down without another taking its place.
    pub(crate) fn clear(&mut self, slot: Slot) {
        self.set(slot, None);
        if self.active == slot {
            // Something has to be in hand, so fall to whatever is left.
            self.active = Slot::ALL.into_iter().find(|s| self.get(*s).is_some()).unwrap_or(slot);
        }
    }

    /// Takes a weapon in, and hands back whatever had to come out for it.
    pub(crate) fn take(&mut self, weapon: WeaponId) -> Option<WeaponId> {
        let slot = self.slot_for(weapon);
        let displaced = self.get(slot);
        self.set(slot, Some(weapon));
        self.active = slot;
        displaced
    }
}

/// The items the heal selector cycles: everything that mends or boosts.
/// A view over the item list rather than a list of its own, so adding a
/// medicine to items.rs puts it in the selector without a second edit.
pub(crate) const HEAL_ITEMS: [ItemKind; 6] = [
    ItemKind::Bandage,
    ItemKind::FirstAidKit,
    ItemKind::MedKit,
    ItemKind::EnergyDrink,
    ItemKind::Painkiller,
    ItemKind::AdrenalineSyringe,
];

/// Bandages stop short of full, the way they do in the games this is taken
/// from: they are for getting off the floor, not for being ready again.
/// PUBG's own ceilings and amounts.
const BANDAGE_CEILING: f32 = 75.0;

/// Long enough that using one is a decision rather than a key to hold down.
const USE_COOLDOWN: f32 = 1.2;

/// What a medicine does. Health to a ceiling, or boost.
enum Effect {
    HealTo { add: f32, ceiling: f32 },
    Boost(f32),
    Nothing,
}

fn effect_of(kind: ItemKind) -> Effect {
    match kind {
        ItemKind::Bandage => Effect::HealTo { add: 25.0, ceiling: BANDAGE_CEILING },
        ItemKind::FirstAidKit => Effect::HealTo { add: 100.0, ceiling: BANDAGE_CEILING },
        ItemKind::MedKit => Effect::HealTo { add: 100.0, ceiling: PLAYER_MAX_HEALTH },
        ItemKind::EnergyDrink => Effect::Boost(40.0),
        ItemKind::Painkiller => Effect::Boost(60.0),
        ItemKind::AdrenalineSyringe => Effect::Boost(100.0),
        _ => Effect::Nothing,
    }
}

#[derive(Resource)]
pub(crate) struct Inventory {
    /// One count per entry in ALL_ITEMS.
    pub(crate) counts: Vec<u32>,
    /// Which of HEAL_ITEMS the bottom-right button spends.
    pub(crate) selected: usize,
    pub(crate) expanded: bool,
    cooldown: f32,
}

impl Default for Inventory {
    fn default() -> Self {
        let mut inventory =
            Self { counts: vec![0; ALL_ITEMS.len()], selected: 0, expanded: false, cooldown: 0.0 };
        // Enough to start with, so the pack is not an empty list the first
        // time anybody opens it -- and under the 100 a player with no
        // backpack can carry, so the first thing the screen shows is not an
        // over-capacity warning.
        for (kind, n) in [
            (ItemKind::Bandage, 3),
            (ItemKind::FirstAidKit, 1),
            (ItemKind::EnergyDrink, 2),
            (ItemKind::Ammo762, 60),
            (ItemKind::Ammo45, 30),
        ] {
            inventory.add(kind, n);
        }
        inventory
    }
}

impl Inventory {
    pub(crate) fn count(&self, kind: ItemKind) -> u32 {
        self.counts.get(kind.index()).copied().unwrap_or(0)
    }

    pub(crate) fn selected_kind(&self) -> ItemKind {
        HEAL_ITEMS[self.selected.min(HEAL_ITEMS.len() - 1)]
    }

    pub(crate) fn add(&mut self, kind: ItemKind, n: u32) {
        if let Some(slot) = self.counts.get_mut(kind.index()) {
            *slot += n;
        }
    }

    pub(crate) fn take_one(&mut self, kind: ItemKind) -> bool {
        match self.counts.get_mut(kind.index()) {
            Some(slot) if *slot > 0 => {
                *slot -= 1;
                true
            }
            _ => false,
        }
    }

    /// What the pack is carrying, in PUBG's capacity units.
    pub(crate) fn carried(&self) -> f32 {
        ALL_ITEMS.iter().map(|k| self.count(*k) as f32 * k.capacity()).sum()
    }
}

/// What is being worn. Armour turns damage away; the backpack sets capacity.
#[derive(Resource, Default)]
pub(crate) struct Equipment {
    pub(crate) helmet: Option<u8>,
    pub(crate) vest: Option<u8>,
    pub(crate) backpack: Option<u8>,
}

impl Equipment {
    pub(crate) fn level(&self, slot: WearSlot) -> Option<u8> {
        match slot {
            WearSlot::Helmet => self.helmet,
            WearSlot::Vest => self.vest,
            WearSlot::Backpack => self.backpack,
        }
    }

    pub(crate) fn set(&mut self, slot: WearSlot, level: Option<u8>) {
        match slot {
            WearSlot::Helmet => self.helmet = level,
            WearSlot::Vest => self.vest = level,
            WearSlot::Backpack => self.backpack = level,
        }
    }

    pub(crate) fn capacity(&self) -> f32 {
        backpack_capacity(self.backpack.unwrap_or(0))
    }
}

/// What is bolted to each weapon, one set of five sockets per slot.
#[derive(Resource, Default)]
pub(crate) struct Attachments {
    pub(crate) fitted: [[Option<ItemKind>; 5]; 3],
}

impl Attachments {
    fn slot_index(slot: Slot) -> usize {
        match slot {
            Slot::Primary(i) => i.min(1),
            Slot::Sidearm => 2,
        }
    }

    pub(crate) fn get(&self, slot: Slot, socket: AttachSlot) -> Option<ItemKind> {
        self.fitted[Self::slot_index(slot)][AttachSlot::ALL.iter().position(|s| *s == socket)?]
    }

    pub(crate) fn set(&mut self, slot: Slot, socket: AttachSlot, item: Option<ItemKind>) {
        if let Some(i) = AttachSlot::ALL.iter().position(|s| *s == socket) {
            self.fitted[Self::slot_index(slot)][i] = item;
        }
    }
}

/// The meter the consumables fill. Drains on its own, and while it has
/// anything in it the player heals; above half it also runs faster.
#[derive(Resource, Default)]
pub(crate) struct Boost(pub(crate) f32);

pub(crate) const BOOST_MAX: f32 = 100.0;
const BOOST_DRAIN_PER_SEC: f32 = 1.6;
const BOOST_REGEN_PER_SEC: f32 = 2.4;
/// Below this the meter still heals but no longer carries the legs.
pub(crate) const BOOST_SPRINT_THRESHOLD: f32 = 50.0;

pub(crate) fn boosted(boost: &Boost) -> bool {
    boost.0 >= BOOST_SPRINT_THRESHOLD
}

/// Drains the meter and turns what it drains into health.
pub(crate) fn advance_boost(
    time: Res<Time>,
    mut boost: ResMut<Boost>,
    mut player: Query<&mut Health, With<Player>>,
) {
    if boost.0 <= 0.0 {
        return;
    }
    let dt = time.delta_secs();
    boost.0 = (boost.0 - BOOST_DRAIN_PER_SEC * dt).max(0.0);
    if let Ok(mut health) = player.single_mut() {
        if health.0 > 0.0 && health.0 < PLAYER_MAX_HEALTH {
            health.0 = (health.0 + BOOST_REGEN_PER_SEC * dt).min(PLAYER_MAX_HEALTH);
        }
    }
}

/// Spends one of the selected item, if there is one and the last one has
/// finished going down.
pub(crate) fn use_selected_consumable(
    mut inventory: ResMut<Inventory>,
    mut boost: ResMut<Boost>,
    mut player: Query<&mut Health, With<Player>>,
) -> bool {
    if inventory.cooldown > 0.0 {
        return false;
    }
    let kind = inventory.selected_kind();
    if inventory.count(kind) == 0 {
        return false;
    }
    let Ok(mut health) = player.single_mut() else { return false };
    // Refused rather than wasted when it would do nothing: spending a med kit
    // at full health is the mistake the ceiling check exists to prevent.
    let spent = match effect_of(kind) {
        Effect::HealTo { add, ceiling } => {
            if health.0 >= ceiling {
                false
            } else {
                health.0 = (health.0 + add).min(ceiling);
                true
            }
        }
        Effect::Boost(amount) => {
            if boost.0 >= BOOST_MAX {
                false
            } else {
                boost.0 = (boost.0 + amount).min(BOOST_MAX);
                true
            }
        }
        Effect::Nothing => false,
    };
    if !spent {
        return false;
    }
    inventory.take_one(kind);
    inventory.cooldown = USE_COOLDOWN;
    true
}

pub(crate) fn advance_use_cooldown(time: Res<Time>, mut inventory: ResMut<Inventory>) {
    if inventory.cooldown > 0.0 {
        inventory.cooldown = (inventory.cooldown - time.delta_secs()).max(0.0);
    }
}

pub(crate) fn use_consumable_key(
    keys: Res<ButtonInput<KeyCode>>,
    inventory: ResMut<Inventory>,
    boost: ResMut<Boost>,
    player: Query<&mut Health, With<Player>>,
) {
    if keys.just_pressed(KeyCode::KeyX) {
        use_selected_consumable(inventory, boost, player);
    }
}

/// Keys 1 to 3 pick a slot rather than a weapon: what is in the slot is the
/// player's business, not the keyboard's.
pub(crate) fn select_weapon_slot(
    keys: Res<ButtonInput<KeyCode>>,
    mut loadout: ResMut<Loadout>,
    mut request: ResMut<weapon::WeaponSwitchRequest>,
) {
    let wanted = if keys.just_pressed(KeyCode::Digit1) {
        Slot::Primary(0)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Slot::Primary(1)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Slot::Sidearm
    } else {
        return;
    };
    equip_slot(&mut loadout, &mut request, wanted);
}

/// Switches to a slot, unless it is empty -- an empty hand is not a weapon.
pub(crate) fn equip_slot(
    loadout: &mut Loadout,
    request: &mut weapon::WeaponSwitchRequest,
    slot: Slot,
) {
    let Some(weapon) = loadout.get(slot) else { return };
    loadout.active = slot;
    request.0 = Some(weapon);
}

/// What a dropped thing is.
#[derive(Clone, Copy)]
pub(crate) enum PickupKind {
    Weapon(WeaponId),
    Item(ItemKind),
}

#[derive(Component)]
pub(crate) struct Pickup {
    pub(crate) kind: PickupKind,
    remaining: f32,
}

const PICKUP_LIFETIME: f32 = 45.0;
const PICKUP_RADIUS: f32 = 1.8;
const PICKUP_SIZE: f32 = 0.22;
const PICKUP_SPIN_PER_SEC: f32 = 1.4;
const PICKUP_HOVER: f32 = 0.35;
/// How much of what dies leaves something behind.
const WEAPON_DROP_CHANCE: f32 = 0.12;
const CONSUMABLE_DROP_CHANCE: f32 = 0.45;

#[derive(Resource)]
pub(crate) struct PickupAssets {
    mesh: Handle<Mesh>,
    /// One per entry in ALL_ITEMS, so a thing on the ground is its own
    /// colour before anybody is near enough to read its name.
    materials: Vec<Handle<StandardMaterial>>,
    weapon_material: Handle<StandardMaterial>,
}

impl PickupAssets {
    fn material_for(&self, kind: ItemKind) -> Handle<StandardMaterial> {
        self.materials[kind.index().min(self.materials.len() - 1)].clone()
    }
}

pub(crate) fn init_pickup_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let lit = |colour: Color| StandardMaterial {
        base_color: colour,
        emissive: LinearRgba::from(colour) * 3.0,
        unlit: true,
        ..default()
    };
    commands.insert_resource(PickupAssets {
        mesh: meshes.add(Cuboid::new(PICKUP_SIZE, PICKUP_SIZE, PICKUP_SIZE)),
        materials: ALL_ITEMS.iter().map(|k| materials.add(lit(k.colour()))).collect(),
        weapon_material: materials.add(lit(Color::srgb(0.45, 0.75, 1.0))),
    });
}

fn spawn_pickup(commands: &mut Commands, assets: &PickupAssets, kind: PickupKind, at: Vec3) {
    let material = match kind {
        PickupKind::Weapon(_) => assets.weapon_material.clone(),
        PickupKind::Item(k) => assets.material_for(k),
    };
    commands.spawn((
        Name::new("Pickup"),
        Mesh3d(assets.mesh.clone()),
        MeshMaterial3d(material),
        Transform::from_translation(at + Vec3::Y * PICKUP_HOVER),
        Pickup { kind, remaining: PICKUP_LIFETIME },
    ));
}

/// A deterministic dribble of randomness from the clock, which is all a drop
/// table needs and avoids carrying an rng through the observer.
fn roll(seed: f32, salt: f32) -> f32 {
    (ops::sin(seed * 127.1 + salt * 311.7) * 43758.547).fract().abs()
}

/// Drops something where a zombie fell.
///
/// Hung off ShotConfirmed rather than the death itself, because that is the
/// one message that already carries both the fact of the kill and the place
/// it happened; by the time the body despawns its position is gone.
pub(crate) fn drop_loot_on_kill(
    trigger: On<ShotConfirmed>,
    mut commands: Commands,
    time: Res<Time>,
    assets: Option<Res<PickupAssets>>,
    loadout: Option<Res<Loadout>>,
    allowed: Option<Res<WeaponSpawnEnabled>>,
) {
    let ShotConfirmed { point, killed, .. } = *trigger.event();
    if !killed {
        return;
    }
    let Some(assets) = assets else { return };
    let seed = time.elapsed_secs();
    let at = Vec3::new(point.x, point.y.max(0.0), point.z);

    if roll(seed, 1.0) < WEAPON_DROP_CHANCE {
        // Drawn from what the player is not already carrying, so looting
        // broadens the loadout instead of handing back the rifle in their
        // hands. If they somehow hold all of them, nothing drops.
        let pool: Vec<WeaponId> = WeaponId::ALL
            .into_iter()
            .enumerate()
            .filter(|(i, _)| allowed.as_ref().is_none_or(|a| a.0[*i]))
            .map(|(_, w)| w)
            .filter(|w| !loadout.as_ref().is_some_and(|l| l.holds(*w)))
            .collect();
        if !pool.is_empty() {
            let weapon = pool[(roll(seed, 2.0) * pool.len() as f32) as usize % pool.len()];
            spawn_pickup(&mut commands, &assets, PickupKind::Weapon(weapon), at);
            return;
        }
    }
    if roll(seed, 3.0) < CONSUMABLE_DROP_CHANCE {
        let kind = ALL_ITEMS[(roll(seed, 4.0) * ALL_ITEMS.len() as f32) as usize % ALL_ITEMS.len()];
        spawn_pickup(&mut commands, &assets, PickupKind::Item(kind), at);
    }
}

/// Puts a weapon on the ground, a short step from wherever the player is.
pub(crate) fn drop_weapon(
    commands: &mut Commands,
    assets: &PickupAssets,
    weapon: WeaponId,
    at: Vec3,
) {
    spawn_pickup(commands, assets, PickupKind::Weapon(weapon), at + DROP_AHEAD);
}

pub(crate) fn drop_item(commands: &mut Commands, assets: &PickupAssets, kind: ItemKind, at: Vec3) {
    spawn_pickup(commands, assets, PickupKind::Item(kind), at + DROP_AHEAD);
}

/// Far enough out that the pickup radius does not take it straight back.
const DROP_AHEAD: Vec3 = Vec3::new(0.0, -0.4, 0.0);

/// Turns the dropped things over and takes back the ones nobody collected.
pub(crate) fn advance_pickups(
    mut commands: Commands,
    time: Res<Time>,
    mut pickups: Query<(Entity, &mut Pickup, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (entity, mut pickup, mut transform) in &mut pickups {
        pickup.remaining -= dt;
        if pickup.remaining <= 0.0 {
            commands.entity(entity).try_despawn();
            continue;
        }
        transform.rotate_y(PICKUP_SPIN_PER_SEC * dt);
    }
}

/// Walking over a thing takes it.
///
/// Proximity rather than a prompt: the game is played with a thumb on a
/// phone as often as not, and a button that only exists while you are stood
/// in the right place is the worst kind of button to have to find.
pub(crate) fn collect_pickups(
    mut commands: Commands,
    mut loadout: ResMut<Loadout>,
    mut inventory: ResMut<Inventory>,
    mut request: ResMut<weapon::WeaponSwitchRequest>,
    assets: Option<Res<PickupAssets>>,
    player: Query<&Transform, With<Player>>,
    pickups: Query<(Entity, &Pickup, &Transform)>,
) {
    let Ok(player) = player.single() else { return };
    for (entity, pickup, transform) in &pickups {
        if player.translation.distance_squared(transform.translation) > PICKUP_RADIUS * PICKUP_RADIUS
        {
            continue;
        }
        match pickup.kind {
            PickupKind::Item(kind) => inventory.add(kind, 1),
            PickupKind::Weapon(weapon) => {
                // What came out goes back on the ground, so a swap is
                // reversible by walking back two steps.
                let displaced = loadout.take(weapon);
                request.0 = Some(weapon);
                if let (Some(displaced), Some(assets)) = (displaced, assets.as_ref()) {
                    spawn_pickup(
                        &mut commands,
                        assets,
                        PickupKind::Weapon(displaced),
                        transform.translation,
                    );
                }
            }
        }
        commands.entity(entity).try_despawn();
    }
}
