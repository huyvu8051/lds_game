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

/// Slots the pack holds. There is no weight model, so capacity is counted in
/// items -- the number means something even if it is not kilograms.
const PACK_CAPACITY: u32 = 12;

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
                    row.spawn(column(GearColumn));
                });
            screen.spawn((
                Text::new("TAB to close".to_string()),
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
    if keys.just_pressed(KeyCode::Tab) {
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

fn row(
    parent: &mut ChildSpawnerCommands<'_>,
    label: String,
    note: String,
    colour: Color,
    action: RowAction,
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
            r.spawn((
                Text::new(note),
                TextFont { font_size: FontSize::Px(11.0), ..default() },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.45)),
            ));
        });
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
    player: Query<&Transform, With<Player>>,
    pickups: Query<(Entity, &inventory::Pickup, &Transform)>,
    ground: Query<Entity, With<GroundColumn>>,
    pack: Query<Entity, With<PackColumn>>,
    gear: Query<Entity, With<GearColumn>>,
    mut capacity: Query<&mut Text, With<CapacityText>>,
) {
    if !open.0 {
        return;
    }
    let (Ok(ground), Ok(pack), Ok(gear)) = (ground.single(), pack.single(), gear.single()) else {
        return;
    };
    for column in [ground, pack, gear] {
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
                inventory::PickupKind::Consumable(k) => (k.label().to_string(), k.colour()),
            };
            row(col, label, format!("{distance:.1}m"), colour, RowAction::Take(entity));
        }
    });

    commands.entity(pack).with_children(|col| {
        col.spawn(heading("PACK"));
        for (i, kind) in ConsumableKind::ALL.into_iter().enumerate() {
            let held = inventory.count(kind);
            if held == 0 {
                continue;
            }
            row(col, kind.label().to_string(), format!("x{held}  USE"), kind.colour(), RowAction::Use(i));
            row(col, String::new(), "drop one".to_string(), Color::srgba(1.0, 1.0, 1.0, 0.3), RowAction::Drop(i));
        }
    });

    commands.entity(gear).with_children(|col| {
        col.spawn(heading("CARRIED"));
        for slot in inventory::Slot::ALL {
            match loadout.get(slot) {
                Some(weapon) => {
                    let held = slot == loadout.active;
                    row(
                        col,
                        format!("{}  {}", slot.label(), weapon.def().name),
                        if held { "IN HAND".to_string() } else { "equip".to_string() },
                        if held { Color::srgb(1.0, 0.85, 0.2) } else { Color::WHITE },
                        RowAction::Equip(slot),
                    );
                    row(col, String::new(), "drop".to_string(), Color::srgba(1.0, 1.0, 1.0, 0.3), RowAction::Unequip(slot));
                }
                None => {
                    row(
                        col,
                        format!("{}  --", slot.label()),
                        "empty".to_string(),
                        Color::srgba(1.0, 1.0, 1.0, 0.3),
                        RowAction::Equip(slot),
                    );
                }
            }
        }
    });

    if let Ok(mut text) = capacity.single_mut() {
        let carried: u32 = ConsumableKind::ALL.iter().map(|k| inventory.count(*k)).sum();
        text.0 = format!("PACK  {carried} / {PACK_CAPACITY}");
    }
}

/// Acts on whichever row was pressed.
pub(crate) fn inventory_screen_rows(
    mut commands: Commands,
    mut loadout: ResMut<Loadout>,
    mut inventory: ResMut<Inventory>,
    mut request: ResMut<weapon::WeaponSwitchRequest>,
    boost: ResMut<Boost>,
    assets: Option<Res<inventory::PickupAssets>>,
    health: Query<&mut Health, With<Player>>,
    player: Query<&Transform, With<Player>>,
    pickups: Query<(&inventory::Pickup, &Transform)>,
    rows: Query<(&Interaction, &RowAction), Changed<Interaction>>,
) {
    let mut used = false;
    for (interaction, action) in &rows {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *action {
            RowAction::Take(entity) => {
                let Ok((pickup, _)) = pickups.get(entity) else { continue };
                match pickup.kind {
                    inventory::PickupKind::Consumable(kind) => inventory.add(kind, 1),
                    inventory::PickupKind::Weapon(weapon) => {
                        let displaced = loadout.take(weapon);
                        request.0 = Some(weapon);
                        if let (Some(displaced), Some(assets), Ok(at)) =
                            (displaced, assets.as_ref(), player.single())
                        {
                            inventory::drop_weapon(&mut commands, assets, displaced, at.translation);
                        }
                    }
                }
                commands.entity(entity).try_despawn();
            }
            RowAction::Use(index) => {
                inventory.selected = index;
                used = true;
            }
            RowAction::Drop(index) => {
                let kind = ConsumableKind::ALL[index.min(2)];
                if inventory.count(kind) > 0 {
                    inventory.counts[index] -= 1;
                    if let (Some(assets), Ok(at)) = (assets.as_ref(), player.single()) {
                        inventory::drop_consumable(&mut commands, assets, kind, at.translation);
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
                    inventory::drop_weapon(&mut commands, assets, weapon, at.translation);
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
