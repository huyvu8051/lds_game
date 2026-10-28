//! The backpack screen, laid out the way a battle royale lays one out.
//!
//! Three columns: what is on the ground within reach, what is in the pack,
//! and what is being carried. PUBG puts the ground on one side, the pack in
//! the middle and the character with its weapons on the other, and the
//! arrangement is worth copying because it matches the direction things
//! travel -- picked up from the left, worn on the right.
//!
//! What it does not copy is everything this game has no model for. There are
//! no attachments, no armour and no helmet, so there is no paper doll to hang
//! them on; a panel of empty silhouettes would be an impression of depth
//! rather than any.
//!
//! Clicked rather than dragged. Dragging is what PUBG does with a mouse, but
//! this is played with a thumb as often as not, and a drag across a phone
//! screen is a gesture the look control already owns.

use bevy::prelude::*;

use crate::*;

/// How far a thing can be and still count as within reach. Wider than the
/// radius that picks things up by walking over them, so the screen can show
/// something to reach for rather than only what is underfoot.
const GROUND_REACH: f32 = 6.0;

#[derive(Resource)]
pub(crate) struct InventoryScreenOpen(pub(crate) bool);

impl Default for InventoryScreenOpen {
    /// BAG_OPEN=1 starts with it up, which is the only way a screenshot gets
    /// taken of a screen that needs a key held to exist.
    fn default() -> Self {
        Self(std::env::var("BAG_OPEN").is_ok())
    }
}

#[derive(Component)]
pub(crate) struct InventoryScreenRoot;
#[derive(Component)]
pub(crate) struct GroundColumn;
#[derive(Component)]
pub(crate) struct PackColumn;
#[derive(Component)]
pub(crate) struct GearColumn;

/// The two long guns. Split off the worn column because that one was
/// carrying three weapons' worth of sockets under the armour and ran off the
/// bottom of the screen.
#[derive(Component)]
pub(crate) struct GunColumn;
#[derive(Component)]
pub(crate) struct CapacityText;
#[derive(Component)]
pub(crate) struct InventoryToggleButton;

/// A row that does something when it is pressed.
#[derive(Component, Clone, Copy)]
pub(crate) enum RowAction {
    /// Pick a thing up off the ground.
    Take(Entity),
    /// Spend one from the pack.
    Use(usize),
    /// Put one back on the ground.
    Drop(usize),
    /// Hold what is in this slot.
    Equip(inventory::Slot),
    /// Throw out what is in this slot.
    Unequip(inventory::Slot),
    /// Take off what is worn here.
    Unwear(WearSlot),
    /// Take out what is fitted in this socket.
    Unfit(inventory::Slot, AttachSlot),
}

fn heading(text: &str) -> impl Bundle {
    (
        Text::new(text.to_string()),
        TextFont { font_size: FontSize::Px(15.0), ..default() },
        TextColor(Color::srgb(1.0, 0.85, 0.2)),
        Node { margin: UiRect::bottom(Val::Px(6.0)), ..default() },
    )
}

fn column(marker: impl Component) -> impl Bundle {
    (
        Node {
            width: Val::Px(230.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        marker,
    )
}

pub(crate) fn spawn_inventory_screen(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                display: Display::None,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
            GlobalZIndex(STATIC_UI_Z_INDEX + 10),
            InventoryScreenRoot,
            NativeUiRoot,
        ))
        .with_children(|screen| {
            screen.spawn((
                Text::new("0 / 12".to_string()),
                TextFont { font_size: FontSize::Px(14.0), ..default() },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
                CapacityText,
            ));
            screen
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(10.0),
                    align_items: AlignItems::Start,
                    ..default()
                })
                .with_children(|row| {
                    row.spawn(column(GroundColumn));
                    row.spawn(column(PackColumn));
                    row.spawn(column(GunColumn));
                    row.spawn(column(GearColumn));
                });
            screen.spawn((
                Text::new("B to close".to_string()),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.4)),
            ));
        });
}

pub(crate) fn toggle_inventory_screen(
    keys: Res<ButtonInput<KeyCode>>,
    mut open: ResMut<InventoryScreenOpen>,
    mut was_pressed: Local<bool>,
    button: Query<&Interaction, With<InventoryToggleButton>>,
) {
    // B, not Tab. Tab belongs to the touch-UI toggle on the phone, and a
    // key that means two things is a key that means neither.
    if keys.just_pressed(KeyCode::KeyB) {
        open.0 = !open.0;
    }
    // Edge-triggered off the button's own state rather than Changed<>, which
    // the settings toggle already does for the same reason: a held press
    // would otherwise open and close it every frame.
    let pressed = button.iter().any(|i| *i == Interaction::Pressed);
    if pressed && !*was_pressed {
        open.0 = !open.0;
    }
    *was_pressed = pressed;
}

pub(crate) fn update_inventory_screen(
    open: Res<InventoryScreenOpen>,
    mut screen: Query<&mut Node, With<InventoryScreenRoot>>,
) {
    let Ok(mut node) = screen.single_mut() else { return };
    node.display = if open.0 { Display::Flex } else { Display::None };
}

/// A row, and optionally a small button at its end for throwing the thing
/// away. Nested inside the row rather than given a line of its own, because
/// a list where every item takes two lines is twice as long for one verb
/// nobody uses often.
fn row_with_drop(
    parent: &mut ChildSpawnerCommands<'_>,
    label: String,
    note: String,
    colour: Color,
    action: RowAction,
    drop: Option<RowAction>,
) {
    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(7.0), Val::Px(4.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.06)),
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.14)),
            Button,
            Interaction::default(),
            action,
        ))
        .with_children(|r| {
            r.spawn((
                Text::new(label),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(colour),
            ));
            r.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|end| {
                end.spawn((
                    Text::new(note),
                    TextFont { font_size: FontSize::Px(11.0), ..default() },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 0.45)),
                ));
                if let Some(drop) = drop {
                    end.spawn((
                        Node {
                            padding: UiRect::axes(Val::Px(5.0), Val::Px(1.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.6, 0.15, 0.15, 0.5)),
                        BorderColor::all(Color::srgba(1.0, 0.5, 0.5, 0.3)),
                        Button,
                        Interaction::default(),
                        drop,
                        children![(
                            Text::new("x".to_string()),
                            TextFont { font_size: FontSize::Px(11.0), ..default() },
                            TextColor(Color::srgba(1.0, 0.8, 0.8, 0.9)),
                        )],
                    ));
                }
            });
        });
}

fn row(
    parent: &mut ChildSpawnerCommands<'_>,
    label: String,
    note: String,
    colour: Color,
    action: RowAction,
) {
    row_with_drop(parent, label, note, colour, action, None);
}

/// Rebuilds the three columns from scratch while the screen is open.
///
/// Thrown away and respawned rather than diffed: the lists are a dozen rows
/// between them and only exist while somebody is looking at them, so the
/// bookkeeping to update them in place would cost more than it saves.
#[allow(clippy::too_many_arguments)]
pub(crate) fn rebuild_inventory_screen(
    mut commands: Commands,
    open: Res<InventoryScreenOpen>,
    loadout: Res<Loadout>,
    inventory: Res<Inventory>,
    equipment: Res<Equipment>,
    attachments: Res<Attachments>,
    player: Query<&Transform, With<Player>>,
    pickups: Query<(Entity, &inventory::Pickup, &Transform)>,
    ground: Query<Entity, With<GroundColumn>>,
    pack: Query<Entity, With<PackColumn>>,
    gear: Query<Entity, With<GearColumn>>,
    guns: Query<Entity, With<GunColumn>>,
    mut capacity: Query<&mut Text, With<CapacityText>>,
) {
    if !open.0 {
        return;
    }
    let (Ok(ground), Ok(pack), Ok(gear), Ok(guns)) =
        (ground.single(), pack.single(), gear.single(), guns.single())
    else {
        return;
    };
    for column in [ground, pack, gear, guns] {
        commands.entity(column).despawn_related::<Children>();
    }

    let here = player.single().map(|t| t.translation).unwrap_or(Vec3::ZERO);
    commands.entity(ground).with_children(|col| {
        col.spawn(heading("GROUND"));
        let mut near: Vec<(Entity, &inventory::Pickup, f32)> = pickups
            .iter()
            .map(|(e, p, t)| (e, p, here.distance(t.translation)))
            .filter(|(_, _, d)| *d <= GROUND_REACH)
            .collect();
        near.sort_by(|a, b| a.2.total_cmp(&b.2));
        if near.is_empty() {
            col.spawn((
                Text::new("nothing nearby".to_string()),
                TextFont { font_size: FontSize::Px(11.0), ..default() },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.3)),
            ));
        }
        for (entity, pickup, distance) in near {
            let (label, colour) = match pickup.kind {
                inventory::PickupKind::Weapon(w) => {
                    (w.def().name.to_string(), Color::srgb(0.6, 0.85, 1.0))
                }
                inventory::PickupKind::Item(k) => (k.label().to_string(), k.colour()),
            };
            row(col, label, format!("{distance:.1}m"), colour, RowAction::Take(entity));
        }
    });

    commands.entity(pack).with_children(|col| {
        col.spawn(heading("PACK"));
        // Grouped the way the pack groups them, so a list of fifty is read
        // as six short ones. A heading with nothing under it is left out.
        for category in Category::ALL {
            let held: Vec<ItemKind> = ALL_ITEMS
                .into_iter()
                .filter(|k| k.category() == category && inventory.count(*k) > 0)
                .collect();
            if held.is_empty() {
                continue;
            }
            col.spawn((
                Text::new(category.label().to_string()),
                TextFont { font_size: FontSize::Px(11.0), ..default() },
                TextColor(category.colour()),
                Node { margin: UiRect::top(Val::Px(4.0)), ..default() },
            ));
            for kind in held {
                let note = match (kind.wear_slot(), kind.attach_slot()) {
                    (Some(_), _) => "wear".to_string(),
                    (_, Some(_)) => "fit".to_string(),
                    _ => format!("x{}", inventory.count(kind)),
                };
                row_with_drop(
                    col,
                    kind.label().to_string(),
                    note,
                    kind.colour(),
                    RowAction::Use(kind.index()),
                    Some(RowAction::Drop(kind.index())),
                );
            }
        }
    });

    commands.entity(guns).with_children(|col| {
        col.spawn(heading("WEAPONS"));
        for slot in [inventory::Slot::Primary(0), inventory::Slot::Primary(1)] {
            weapon_rows(col, slot, &loadout, &attachments);
        }
    });

    commands.entity(gear).with_children(|col| {
        col.spawn(heading("WORN"));
        for slot in WearSlot::ALL {
            let (note, colour) = match equipment.level(slot) {
                Some(level) => (format!("Lv{level}"), Category::Armour.colour()),
                None => ("empty".to_string(), Color::srgba(1.0, 1.0, 1.0, 0.3)),
            };
            row(col, slot.label().to_string(), note, colour, RowAction::Unwear(slot));
        }

        col.spawn(heading("SIDEARM"));
        weapon_rows(col, inventory::Slot::Sidearm, &loadout, &attachments);
    });

    if let Ok(mut text) = capacity.single_mut() {
        text.0 = format!("PACK  {:.0} / {:.0}", inventory.carried(), equipment.capacity());
    }
}

/// One weapon slot: the gun, its drop, and its five sockets.
///
/// Shared rather than written twice, now that the long guns and the sidearm
/// are listed in different columns -- two copies is how the two of them would
/// drift apart.
fn weapon_rows(
    col: &mut ChildSpawnerCommands<'_>,
    slot: inventory::Slot,
    loadout: &Loadout,
    attachments: &Attachments,
) {
    let Some(weapon) = loadout.get(slot) else {
        row(
            col,
            format!("{}  --", slot.label()),
            "empty".to_string(),
            Color::srgba(1.0, 1.0, 1.0, 0.3),
            RowAction::Equip(slot),
        );
        return;
    };
    let held = slot == loadout.active;
    row(
        col,
        format!("{}  {}", slot.label(), weapon.def().name),
        if held { "IN HAND".to_string() } else { "equip".to_string() },
        if held { Color::srgb(1.0, 0.85, 0.2) } else { Color::WHITE },
        RowAction::Equip(slot),
    );
    row(col, String::new(), "drop".to_string(), Color::srgba(1.0, 1.0, 1.0, 0.3), RowAction::Unequip(slot));
    // The five sockets, listed under the weapon they belong to. Empty ones
    // are shown: the socket existing is what makes an attachment on the
    // ground worth stopping for.
    for socket in AttachSlot::ALL {
        let (note, colour) = match attachments.get(slot, socket) {
            Some(item) => (item.label().to_string(), item.colour()),
            None => ("--".to_string(), Color::srgba(1.0, 1.0, 1.0, 0.25)),
        };
        row(col, format!("  {}", socket.label()), note, colour, RowAction::Unfit(slot, socket));
    }
}

/// Acts on whichever row was pressed.
pub(crate) fn inventory_screen_rows(
    mut commands: Commands,
    mut loadout: ResMut<Loadout>,
    mut inventory: ResMut<Inventory>,
    mut equipment: ResMut<Equipment>,
    mut attachments: ResMut<Attachments>,
    mut request: ResMut<weapon::WeaponSwitchRequest>,
    boost: ResMut<Boost>,
    assets: Option<Res<inventory::PickupAssets>>,
    health: Query<&mut Health, With<Player>>,
    player: Query<&Transform, With<Player>>,
    pickups: Query<(&inventory::Pickup, &Transform)>,
    rows: Query<(&Interaction, &RowAction), Changed<Interaction>>,
    eye: Query<&GlobalTransform, With<FpsCamera>>,
) {
    // Where the player is looking, so a drop leaves in front of them rather
    // than straight down between their feet.
    let facing = eye.single().map(|e| e.forward().as_vec3()).unwrap_or(Vec3::NEG_Z);
    let mut used = false;
    for (interaction, action) in &rows {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *action {
            RowAction::Take(entity) => {
                let Ok((pickup, _)) = pickups.get(entity) else { continue };
                match pickup.kind {
                    inventory::PickupKind::Item(kind) => inventory.add(kind, 1),
                    inventory::PickupKind::Weapon(weapon) => inventory::pick_up_weapon(
                        &mut commands,
                        &mut loadout,
                        &mut request,
                        assets.as_deref(),
                        weapon,
                        player.single().ok().map(|at| (at.translation, facing)),
                    ),
                }
                commands.entity(entity).try_despawn();
            }
            RowAction::Use(index) => {
                let kind = ALL_ITEMS[index.min(ALL_ITEMS.len() - 1)];
                if let Some((slot, level)) = kind.wear_slot() {
                    // What comes off goes back in the pack, so swapping a
                    // helmet for a better one does not throw the old away.
                    if inventory.take_one(kind) {
                        if let Some(old) = equipment.level(slot) {
                            if let Some(previous) = ALL_ITEMS
                                .into_iter()
                                .find(|k| k.wear_slot() == Some((slot, old)))
                            {
                                inventory.add(previous, 1);
                            }
                        }
                        equipment.set(slot, Some(level));
                    }
                } else if let Some(socket) = kind.attach_slot() {
                    let slot = loadout.active;
                    if loadout.get(slot).is_some() && inventory.take_one(kind) {
                        if let Some(previous) = attachments.get(slot, socket) {
                            inventory.add(previous, 1);
                        }
                        attachments.set(slot, socket, Some(kind));
                    }
                } else if let Some(i) = inventory::HEAL_ITEMS.iter().position(|k| *k == kind) {
                    inventory.selected = i;
                    used = true;
                }
            }
            RowAction::Unwear(slot) => {
                let Some(level) = equipment.level(slot) else { continue };
                if let Some(kind) =
                    ALL_ITEMS.into_iter().find(|k| k.wear_slot() == Some((slot, level)))
                {
                    inventory.add(kind, 1);
                }
                equipment.set(slot, None);
            }
            RowAction::Unfit(slot, socket) => {
                let Some(kind) = attachments.get(slot, socket) else { continue };
                inventory.add(kind, 1);
                attachments.set(slot, socket, None);
            }
            RowAction::Drop(index) => {
                let kind = ALL_ITEMS[index.min(ALL_ITEMS.len() - 1)];
                if inventory.take_one(kind) {
                    if let (Some(assets), Ok(at)) = (assets.as_ref(), player.single()) {
                        inventory::drop_item(&mut commands, assets, kind, at.translation, facing);
                    }
                }
            }
            RowAction::Equip(slot) => {
                inventory::equip_slot(&mut loadout, &mut request, slot);
            }
            RowAction::Unequip(slot) => {
                let Some(weapon) = loadout.get(slot) else { continue };
                loadout.clear(slot);
                if let (Some(assets), Ok(at)) = (assets.as_ref(), player.single()) {
                    inventory::drop_weapon(&mut commands, assets, weapon, at.translation, facing);
                }
            }
        }
    }
    // Spending happens after the loop so the selection made above is the one
    // spent, and so the two resources are not borrowed inside it.
    if used {
        inventory::use_selected_consumable(inventory, boost, health);
    }
}
