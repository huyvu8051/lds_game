//! The bottom-centre panel: what you are carrying, how hurt you are, and
//! what you can do about it.
//!
//! Laid out the way a battle royale lays it out, because that arrangement
//! answers a real question in one glance -- the bars are the state you are
//! in, the row above them is what you can reach for, and the thing beside
//! them is the one action that changes either.

use bevy::prelude::*;

use crate::*;

const PANEL_BOTTOM: f32 = 16.0;
const BAR_WIDTH: f32 = 360.0;
const HEALTH_BAR_HEIGHT: f32 = 12.0;
const BOOST_BAR_HEIGHT: f32 = 5.0;
const SLOT_WIDTH: f32 = 112.0;
const SLOT_HEIGHT: f32 = 26.0;
const ITEM_BUTTON: f32 = 44.0;

fn health_colour(fraction: f32) -> Color {
    // One colour until it matters. A bar that is already orange at three
    // quarters has spent the warning it needs at one quarter.
    if fraction > 0.5 {
        Color::srgb(0.90, 0.92, 0.88)
    } else if fraction > 0.25 {
        Color::srgb(0.95, 0.70, 0.25)
    } else {
        Color::srgb(0.90, 0.25, 0.22)
    }
}

#[derive(Component)]
pub(crate) struct HealthBarFill;
#[derive(Component)]
pub(crate) struct BoostBarFill;
#[derive(Component)]
pub(crate) struct WeaponSlotButton(pub(crate) inventory::Slot);
#[derive(Component)]
pub(crate) struct WeaponSlotLabel(pub(crate) inventory::Slot);
#[derive(Component)]
pub(crate) struct ItemUseButton;
#[derive(Component)]
pub(crate) struct ItemUseLabel;
#[derive(Component)]
pub(crate) struct ItemExpandButton;
#[derive(Component)]
pub(crate) struct ItemList;
#[derive(Component)]
pub(crate) struct ItemChoiceButton(pub(crate) usize);
#[derive(Component)]
pub(crate) struct ItemChoiceLabel(pub(crate) usize);

fn text(content: &str, size: f32, colour: Color) -> impl Bundle {
    (
        Text::new(content.to_string()),
        TextFont { font_size: FontSize::Px(size), ..default() },
        TextColor(colour),
    )
}

fn bar(width: f32, height: f32, fill: Color, marker: impl Component) -> impl Bundle {
    (
        Node {
            width: Val::Px(width),
            height: Val::Px(height),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.25)),
        children![(
            Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
            BackgroundColor(fill),
            marker,
        )],
    )
}

pub(crate) fn spawn_loadout_hud(commands: &mut Commands) {
    commands
        .spawn((
            // Anchored from the left across the full width, the same way the
            // kill feed had to be: a node positioned by `right` cannot be
            // placed until its contents have been measured, and UI layout
            // does not run every frame.
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                bottom: Val::Px(PANEL_BOTTOM),
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::End,
                column_gap: Val::Px(10.0),
                ..default()
            },
            GlobalZIndex(STATIC_UI_Z_INDEX),
            NativeUiRoot,
        ))
        .with_children(|row| {
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                align_items: AlignItems::Start,
                ..default()
            })
            .with_children(|col| {
                // The three slots, above the bars: what you can reach for
                // sits over what reaching for it is protecting.
                col.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|slots| {
                    for slot in inventory::Slot::ALL {
                        slots
                            .spawn((
                                Node {
                                    width: Val::Px(SLOT_WIDTH),
                                    height: Val::Px(SLOT_HEIGHT),
                                    border: UiRect::all(Val::Px(1.0)),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    column_gap: Val::Px(5.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                                BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.2)),
                                Button,
                                Interaction::default(),
                                WeaponSlotButton(slot),
                            ))
                            .with_children(|cell| {
                                cell.spawn(text(slot.label(), 11.0, Color::srgb(1.0, 0.85, 0.2)));
                                cell.spawn((
                                    text("--", 12.0, Color::srgba(1.0, 1.0, 1.0, 0.85)),
                                    WeaponSlotLabel(slot),
                                ));
                            });
                    }
                });

                col.spawn(bar(
                    BAR_WIDTH,
                    BOOST_BAR_HEIGHT,
                    Color::srgb(0.95, 0.80, 0.25),
                    BoostBarFill,
                ));
                col.spawn(bar(
                    BAR_WIDTH,
                    HEALTH_BAR_HEIGHT,
                    health_colour(1.0),
                    HealthBarFill,
                ));
            });

            // Beside the bars: the selected item, and the way to change it.
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|side| {
                // The list, above the buttons and hidden until asked for, so
                // opening it never covers the bars it belongs to.
                side.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(3.0),
                        display: Display::None,
                        ..default()
                    },
                    ItemList,
                ))
                .with_children(|list| {
                    for (i, kind) in inventory::HEAL_ITEMS.into_iter().enumerate() {
                        list.spawn((
                            Node {
                                width: Val::Px(140.0),
                                height: Val::Px(24.0),
                                border: UiRect::all(Val::Px(1.0)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.2)),
                            Button,
                            Interaction::default(),
                            ItemChoiceButton(i),
                            children![(
                                Text::new(kind.label().to_string()),
                                TextFont { font_size: FontSize::Px(11.0), ..default() },
                                TextColor(kind.colour()),
                                ItemChoiceLabel(i),
                            )],
                        ));
                    }
                });

                side.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|buttons| {
                    buttons
                        .spawn((
                            Node {
                                width: Val::Px(ITEM_BUTTON),
                                height: Val::Px(ITEM_BUTTON),
                                border: UiRect::all(Val::Px(1.0)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                border_radius: BorderRadius::all(Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.35)),
                            Button,
                            Interaction::default(),
                            ItemUseButton,
                            children![(
                                Text::new("BND 0".to_string()),
                                TextFont { font_size: FontSize::Px(11.0), ..default() },
                                TextColor(Color::WHITE),
                                ItemUseLabel,
                            )],
                        ));
                    buttons.spawn((
                        Node {
                            width: Val::Px(22.0),
                            height: Val::Px(ITEM_BUTTON),
                            border: UiRect::all(Val::Px(1.0)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.35)),
                        Button,
                        Interaction::default(),
                        ItemExpandButton,
                        children![(
                            Text::new("^".to_string()),
                            TextFont { font_size: FontSize::Px(14.0), ..default() },
                            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
                        )],
                    ));
                });
            });
        });
}

pub(crate) fn update_bars(
    boost: Res<Boost>,
    player: Query<&Health, With<Player>>,
    mut health_bar: Query<(&mut Node, &mut BackgroundColor), With<HealthBarFill>>,
    mut boost_bar: Query<&mut Node, (With<BoostBarFill>, Without<HealthBarFill>)>,
) {
    if let (Ok(health), Ok((mut node, mut colour))) = (player.single(), health_bar.single_mut()) {
        let fraction = (health.0 / PLAYER_MAX_HEALTH).clamp(0.0, 1.0);
        node.width = Val::Percent(fraction * 100.0);
        *colour = BackgroundColor(health_colour(fraction));
    }
    if let Ok(mut node) = boost_bar.single_mut() {
        node.width = Val::Percent((boost.0 / inventory::BOOST_MAX).clamp(0.0, 1.0) * 100.0);
    }
}

pub(crate) fn update_weapon_slots(
    loadout: Res<Loadout>,
    mut labels: Query<(&WeaponSlotLabel, &mut Text)>,
    mut buttons: Query<(&WeaponSlotButton, &mut BorderColor, &mut BackgroundColor)>,
) {
    if !loadout.is_changed() {
        return;
    }
    for (label, mut text) in &mut labels {
        text.0 = loadout.get(label.0).map_or("--".to_string(), |w| w.def().name.to_string());
    }
    for (button, mut border, mut background) in &mut buttons {
        let active = button.0 == loadout.active;
        let filled = loadout.get(button.0).is_some();
        *border = BorderColor::all(if active {
            Color::srgba(1.0, 0.85, 0.2, 0.9)
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.2)
        });
        *background = BackgroundColor(if !filled {
            Color::srgba(0.0, 0.0, 0.0, 0.25)
        } else if active {
            Color::srgba(0.35, 0.3, 0.05, 0.6)
        } else {
            Color::srgba(0.0, 0.0, 0.0, 0.5)
        });
    }
}

pub(crate) fn update_item_ui(
    inventory: Res<Inventory>,
    mut use_label: Query<&mut Text, With<ItemUseLabel>>,
    mut choice_labels: Query<(&ItemChoiceLabel, &mut TextColor), Without<ItemUseLabel>>,
    mut list: Query<&mut Node, With<ItemList>>,
) {
    if !inventory.is_changed() {
        return;
    }
    let kind = inventory.selected_kind();
    if let Ok(mut text) = use_label.single_mut() {
        // Three letters, because the slot is forty-four pixels wide and
        // "Adrenaline Syringe" is not going in it.
        let short: String = kind.label().chars().filter(|c| c.is_alphanumeric()).take(3).collect();
        text.0 = format!("{} {}", short.to_uppercase(), inventory.count(kind));
    }
    for (label, mut colour) in &mut choice_labels {
        let this = inventory::HEAL_ITEMS[label.0.min(inventory::HEAL_ITEMS.len() - 1)];
        // An item you have none of is still listed, just plainly: the slot
        // existing is information, and hiding it would make the list jump
        // about as things are spent.
        let strength = if inventory.count(this) == 0 { 0.3 } else { 1.0 };
        let base = this.colour().to_srgba();
        *colour = TextColor(Color::srgba(base.red, base.green, base.blue, strength));
    }
    if let Ok(mut node) = list.single_mut() {
        node.display = if inventory.expanded { Display::Flex } else { Display::None };
    }
}

/// Every button on the panel, in one system because they all speak to the
/// same two resources and splitting them would mean borrowing both twice.
pub(crate) fn loadout_hud_buttons(
    mut loadout: ResMut<Loadout>,
    mut inventory: ResMut<Inventory>,
    mut request: ResMut<weapon::WeaponSwitchRequest>,
    boost: ResMut<Boost>,
    player: Query<&mut Health, With<Player>>,
    slots: Query<(&Interaction, &WeaponSlotButton), Changed<Interaction>>,
    use_button: Query<&Interaction, (With<ItemUseButton>, Changed<Interaction>)>,
    expand: Query<&Interaction, (With<ItemExpandButton>, Changed<Interaction>)>,
    choices: Query<(&Interaction, &ItemChoiceButton), Changed<Interaction>>,
) {
    for (interaction, slot) in &slots {
        if *interaction == Interaction::Pressed {
            inventory::equip_slot(&mut loadout, &mut request, slot.0);
        }
    }
    for (interaction, choice) in &choices {
        if *interaction == Interaction::Pressed {
            inventory.selected = choice.0;
            inventory.expanded = false;
        }
    }
    if expand.iter().any(|i| *i == Interaction::Pressed) {
        inventory.expanded = !inventory.expanded;
    }
    if use_button.iter().any(|i| *i == Interaction::Pressed) {
        inventory::use_selected_consumable(inventory, boost, player);
    }
}
