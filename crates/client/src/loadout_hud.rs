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
/// Wide enough for the longest weapon name. "DESERT EAGLE" overflowed 112
/// and the throwable cell beside it was what the overflow landed on.
const SLOT_WIDTH: f32 = 132.0;
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

/// One button per kind, so a grenade can be cooked without first cycling to
/// it. Held, not tapped: the hold is the cook.
#[derive(Component)]
pub(crate) struct ThrowButton(pub(crate) ItemKind);
#[derive(Component)]
pub(crate) struct ThrowButtonLabel(pub(crate) ItemKind);
#[derive(Component)]
pub(crate) struct CookRing;

#[derive(Component)]
pub(crate) struct ThrowableSlotButton;
#[derive(Component)]
pub(crate) struct ThrowableSlotLabel;

#[derive(Component)]
pub(crate) struct GearCellLabel(pub(crate) WearSlot);
#[derive(Component)]
pub(crate) struct GearCellWear(pub(crate) WearSlot);

/// How wide the three equipment cells are. PUBG puts icons here; this puts
/// three letters, which says the same thing and needs no artist.
const GEAR_CELL: f32 = 46.0;

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
            // Left of the bars, which is where PUBG's equipment HUD sits.
            // A level alone would be half the point: what the indicator is
            // mostly read for is how much of the plate is left.
            row.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                align_items: AlignItems::End,
                ..default()
            })
            .with_children(|gear| {
                for slot in WearSlot::ALL {
                    gear.spawn((
                        Node {
                            width: Val::Px(GEAR_CELL),
                            height: Val::Px(SLOT_HEIGHT),
                            border: UiRect::all(Val::Px(1.0)),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            row_gap: Val::Px(2.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.2)),
                    ))
                    .with_children(|cell| {
                        cell.spawn((
                            text(slot.label(), 10.0, Color::srgba(1.0, 1.0, 1.0, 0.4)),
                            GearCellLabel(slot),
                        ));
                        cell.spawn((
                            Node {
                                width: Val::Px(GEAR_CELL - 12.0),
                                height: Val::Px(3.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)),
                            children![(
                                Node {
                                    width: Val::Percent(0.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.6, 0.95, 0.7)),
                                GearCellWear(slot),
                            )],
                        ));
                    });
                }
            });

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
                                    // Clipped rather than allowed to spill:
                                    // a name too long for its cell should
                                    // lose its own tail, not cover the cell
                                    // next door.
                                    overflow: Overflow::clip(),
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
                    // A fourth cell beside the three. PUBG keeps throwables
                    // out of the weapon slots because they are not weapons
                    // you hold, they are weapons you spend.
                    slots
                        .spawn((
                            Node {
                                width: Val::Px(GEAR_CELL + 24.0),
                                height: Val::Px(SLOT_HEIGHT),
                                border: UiRect::all(Val::Px(1.0)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.2)),
                            Button,
                            Interaction::default(),
                            ThrowableSlotButton,
                        ))
                        .with_children(|cell| {
                            cell.spawn((
                                text("--", 11.0, Color::srgb(0.95, 0.55, 0.25)),
                                ThrowableSlotLabel,
                            ));
                        });
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

            // One button per throwable, above the heal selector. PUBG puts
            // these on the weapon bar; here they are their own column
            // because holding one is a different gesture from tapping a
            // weapon slot and the two should not be neighbours by accident.
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|col| {
                for kind in inventory::THROWABLES {
                    col.spawn((
                        Node {
                            width: Val::Px(74.0),
                            height: Val::Px(22.0),
                            border: UiRect::all(Val::Px(1.0)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.2)),
                        Button,
                        Interaction::default(),
                        ThrowButton(kind),
                        children![(
                            Text::new(String::new()),
                            TextFont { font_size: FontSize::Px(10.0), ..default() },
                            TextColor(kind.colour()),
                            ThrowButtonLabel(kind),
                        )],
                    ));
                }
                // How much fuse is left while one is in the hand. The only
                // thing worth seeing at that moment.
                col.spawn((
                    Node {
                        width: Val::Px(74.0),
                        height: Val::Px(4.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.1)),
                    children![(
                        Node { width: Val::Percent(0.0), height: Val::Percent(100.0), ..default() },
                        BackgroundColor(Color::srgb(1.0, 0.5, 0.2)),
                        CookRing,
                    )],
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

/// Keeps the three equipment cells saying what is worn and what is left of
/// it. Empty reads as the slot's own name, dimmed -- the slot being there is
/// information too.
pub(crate) fn update_gear_cells(
    equipment: Res<Equipment>,
    mut labels: Query<(&GearCellLabel, &mut Text, &mut TextColor)>,
    mut wear: Query<(&GearCellWear, &mut Node, &mut BackgroundColor)>,
) {
    if !equipment.is_changed() {
        return;
    }
    for (cell, mut text, mut colour) in &mut labels {
        match equipment.level(cell.0) {
            Some(level) => {
                text.0 = format!("{} {}", cell.0.label(), level);
                *colour = TextColor(Color::srgb(0.75, 0.95, 0.8));
            }
            None => {
                text.0 = cell.0.label().to_string();
                *colour = TextColor(Color::srgba(1.0, 1.0, 1.0, 0.25));
            }
        }
    }
    for (cell, mut node, mut background) in &mut wear {
        let condition = equipment.condition(cell.0);
        node.width = Val::Percent(condition * 100.0);
        // Warns before it breaks, which is the whole reason to watch it.
        *background = BackgroundColor(if condition > 0.35 {
            Color::srgb(0.6, 0.95, 0.7)
        } else {
            Color::srgb(0.95, 0.5, 0.3)
        });
    }
}

/// The throwable cell: what is selected, how many are left, and a tap to
/// move to the next one that is actually carried.
pub(crate) fn throwable_slot(
    mut selection: ResMut<ThrowableSelection>,
    inventory: Res<Inventory>,
    button: Query<&Interaction, (With<ThrowableSlotButton>, Changed<Interaction>)>,
    mut label: Query<&mut Text, With<ThrowableSlotLabel>>,
) {
    if button.iter().any(|i| *i == Interaction::Pressed) {
        // Steps over the ones there are none of, so the slot never lands on
        // something that cannot be thrown.
        for step in 1..=inventory::THROWABLES.len() {
            let next = (selection.0 + step) % inventory::THROWABLES.len();
            if inventory.count(inventory::THROWABLES[next]) > 0 || step == inventory::THROWABLES.len() {
                selection.0 = next;
                break;
            }
        }
    }
    if !selection.is_changed() && !inventory.is_changed() {
        return;
    }
    let kind = selection.kind();
    let held = inventory.count(kind);
    if let Ok(mut text) = label.single_mut() {
        // First word only: "Frag Grenade" and "Molotov Cocktail" both say
        // everything they need to in one.
        let short = kind.label().split_whitespace().next().unwrap_or("--");
        text.0 = if held > 0 { format!("{short} {held}") } else { "--".to_string() };
    }
}

/// The per-kind throw buttons. Pressing one pulls the pin; letting go throws
/// it, with whatever fuse is left.
///
/// Edge-detected against its own last state rather than Changed<Interaction>,
/// because a hold has to be seen for its whole length and Changed only fires
/// when the state turns over.
pub(crate) fn throw_buttons(
    mut cooking: ResMut<grenade::Cooking>,
    mut inventory: ResMut<Inventory>,
    mut commands: Commands,
    assets: Option<Res<grenade::GrenadeAssets>>,
    sounds: Option<Res<GrenadeSounds>>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
    buttons: Query<(&Interaction, &ThrowButton)>,
    mut held: Local<Option<ItemKind>>,
    mut labels: Query<(&ThrowButtonLabel, &mut Text, &mut TextColor)>,
    mut ring: Query<&mut Node, With<CookRing>>,
) {
    let pressed = buttons
        .iter()
        .find(|(interaction, _)| **interaction == Interaction::Pressed)
        .map(|(_, button)| button.0);

    match (*held, pressed) {
        (None, Some(kind)) => {
            grenade::begin_cook(&mut cooking, &inventory, kind);
            if cooking.kind == Some(kind) {
                *held = Some(kind);
                if let Some(sounds) = sounds.as_ref() {
                    grenade::play_sample(&mut commands, sounds.pin.clone(), 0.9);
                }
            }
        }
        (Some(_), None) => {
            *held = None;
            if let (Some(assets), Ok(eye)) = (assets.as_ref(), camera.single()) {
                grenade::throw_now(&mut commands, &mut cooking, &mut inventory, assets, sounds.as_deref(), eye);
            }
        }
        _ => {}
    }

    for (label, mut text, mut colour) in &mut labels {
        let count = inventory.count(label.0);
        let short = label.0.label().split_whitespace().next().unwrap_or("--");
        text.0 = format!("{short} {count}");
        let base = label.0.colour().to_srgba();
        *colour = TextColor(Color::srgba(
            base.red,
            base.green,
            base.blue,
            if count == 0 { 0.25 } else { 1.0 },
        ));
    }
    if let Ok(mut node) = ring.single_mut() {
        let left = match cooking.kind {
            Some(_) => ((grenade::FUSE_SECONDS - cooking.elapsed) / grenade::FUSE_SECONDS)
                .clamp(0.0, 1.0),
            None => 0.0,
        };
        node.width = Val::Percent(left * 100.0);
    }
}
