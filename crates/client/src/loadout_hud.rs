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
/// Wide enough for the longest weapon name. "DESERT EAGLE" overflowed 112
/// and spilled into whatever stood beside it.
const SLOT_WIDTH: f32 = 132.0;
/// Two lines now: the name, and the rounds under it. The readout used to be
/// its own box in the far corner, which meant checking the weapon and
/// checking what was in it were two looks at opposite ends of the screen.
const SLOT_HEIGHT: f32 = 38.0;
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

/// Where each throwable sits. The smokescreens on the left, the ones that
/// hurt on the right -- so a hand reaching in a fight only has to be on the
/// correct side to be harmless or not.
const THROW_GRID: [[ItemKind; 2]; 2] = [
    [ItemKind::StunGrenade, ItemKind::FragGrenade],
    [ItemKind::SmokeGrenade, ItemKind::Molotov],
];
const THROW_BUTTON: f32 = 70.0;
const THROW_GAP: f32 = 3.0;

#[derive(Component)]
pub(crate) struct GearCellLabel(pub(crate) WearSlot);
#[derive(Component)]
pub(crate) struct GearCellWear(pub(crate) WearSlot);

/// How wide the three equipment cells are. PUBG puts icons here; this puts
/// three letters, which says the same thing and needs no artist.
const GEAR_CELL: f32 = 58.0;

#[derive(Component)]
pub(crate) struct HealthBarFill;
#[derive(Component)]
pub(crate) struct BoostBarFill;
#[derive(Component)]
pub(crate) struct WeaponSlotButton(pub(crate) inventory::Slot);
#[derive(Component)]
pub(crate) struct WeaponSlotLabel(pub(crate) inventory::Slot);
#[derive(Component)]
pub(crate) struct WeaponSlotAmmo(pub(crate) inventory::Slot);
fn text(content: &str, size: f32, colour: Color) -> impl Bundle {
    (
        Text::new(content.to_string()),
        TextFont { font_size: FontSize::Px(size), ..default() },
        TextColor(colour),
    )
}

/// The shape every cell on this panel shares: a short label over a thin bar.
/// Health and boost ask the same question the armour cells ask -- how much of
/// it is left -- so they are drawn the same way rather than as two wide bars
/// of their own.
fn stat_cell(
    label: &str,
    fill: Color,
    label_marker: impl Component,
    fill_marker: impl Component,
) -> impl Bundle {
    (
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
        children![
            (text(label, 10.0, Color::srgba(1.0, 1.0, 1.0, 0.4)), label_marker),
            (
                Node { width: Val::Px(GEAR_CELL - 12.0), height: Val::Px(3.0), ..default() },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)),
                children![(
                    Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                    BackgroundColor(fill),
                    fill_marker,
                )],
            ),
        ],
    )
}

/// Which medicine a condition cell spends, and the list it drops.
#[derive(Component, Clone, Copy)]
pub(crate) struct ConditionCell(pub(crate) crate::items::Category);

#[derive(Component, Clone, Copy)]
pub(crate) struct ConditionList(pub(crate) crate::items::Category);

#[derive(Component, Clone, Copy)]
pub(crate) struct ConditionChoice {
    pub(crate) category: crate::items::Category,
    pub(crate) index: usize,
}

#[derive(Component, Clone, Copy)]
pub(crate) struct ConditionChoiceLabel {
    pub(crate) index: usize,
}

/// How far down a finger has to travel before it means "show me the list"
/// rather than "use this". Short enough to feel like a flick, long enough that
/// a thumb that rolls while tapping still spends the item it meant to.
const CONDITION_DRAG_OPEN: f32 = 12.0;

/// Tap a condition cell to spend what it shows; drag down to change it.
///
/// Tap and drag start the same way, so the decision cannot be made on press —
/// it is made on release, by whether the finger travelled. Which is why this
/// hit-tests positions itself instead of reading `Interaction`: that reports
/// pressed or not and never says where, and the two gestures differ only in
/// where the finger went.
///
/// Built around the same three facts `multi_touch_button_interaction` is:
/// a gesture belongs to the finger that started it, every finger counts, and
/// the answer has to come in the same frame the finger moves.
pub(crate) fn condition_cell_input(
    touches: Res<ButtonInput<MouseButton>>,
    fingers: Res<Touches>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut drag: Local<Option<(crate::items::Category, f32, bool, Vec2)>>,
    cells: Query<(&ComputedNode, &UiGlobalTransform, &ConditionCell)>,
    mut lists: Query<(&mut Node, &ConditionList)>,
    inventory: ResMut<Inventory>,
    boost: ResMut<Boost>,
    player: Query<&mut Health, With<Player>>,
) {
    let Ok(window) = windows.single() else { return };
    let scale = window.scale_factor();
    let finger = fingers.iter().next().map(|t| t.position() * scale);
    let down = finger.is_some() || touches.pressed(MouseButton::Left);

    // Where to judge from: the finger still down, else the one that just came
    // up, else the mouse, else wherever the gesture was last seen.
    //
    // A lifted finger is already out of `Touches::iter()` by the time this
    // runs, and a phone has no cursor standing behind it — so the frame that
    // ends a tap arrives with no pointer at all. Falling back to the last
    // known position is what lets a tap be a tap; without it the release is
    // judged against nothing and the item is never spent.
    let at = finger
        .or_else(|| fingers.iter_just_released().next().map(|t| t.position() * scale))
        .or_else(|| window.cursor_position().map(|p| p * scale))
        .or_else(|| drag.map(|(_, _, _, last)| last));

    let Some(at) = at else {
        *drag = None;
        return;
    };

    if down {
        match drag.as_mut() {
            None => {
                let landed = cells
                    .iter()
                    .find(|(node, transform, _)| node.contains_point(**transform, at))
                    .map(|(_, _, cell)| cell.0);
                if let Some(category) = landed {
                    *drag = Some((category, at.y, false, at));
                }
            }
            Some((category, start_y, opened, last)) => {
                *last = at;
                if !*opened && at.y - *start_y > CONDITION_DRAG_OPEN {
                    *opened = true;
                    let category = *category;
                    for (mut node, list) in &mut lists {
                        node.display =
                            if list.0 == category { Display::Flex } else { Display::None };
                    }
                }
            }
        }
        return;
    }

    // Released. A travelled finger already opened the list and has said its
    // piece; a still one spends the item.
    let Some((category, _, opened, _)) = drag.take() else { return };
    if opened {
        return;
    }
    let over = cells
        .iter()
        .any(|(node, transform, cell)| cell.0 == category && node.contains_point(*transform, at));
    if !over {
        return;
    }
    for (mut node, _) in &mut lists {
        node.display = Display::None;
    }
    crate::inventory::use_selected_consumable(inventory, boost, player, category);
}

/// Picking a row points that cell at that medicine and puts the list away.
pub(crate) fn condition_choice_buttons(
    mut inventory: ResMut<Inventory>,
    choices: Query<(&Interaction, &ConditionChoice), Changed<Interaction>>,
    mut lists: Query<&mut Node, With<ConditionList>>,
) {
    for (interaction, choice) in &choices {
        if *interaction != Interaction::Pressed {
            continue;
        }
        inventory.set_selected(choice.category, choice.index);
        for mut node in &mut lists {
            node.display = Display::None;
        }
    }
}

/// The cell says what it will spend and how many are left; the bar underneath
/// says how much of the thing it mends you still have.
///
/// The label used to read "HP 100", which repeated the bar directly beneath it
/// and left nowhere to see which medicine a tap would burn.
pub(crate) fn update_condition_labels(
    inventory: Res<Inventory>,
    mut labels: Query<(&mut Text, &mut TextColor, &ConditionCellLabel)>,
    mut choices: Query<(&mut Text, &ConditionChoiceLabel), Without<ConditionCellLabel>>,
) {
    if !inventory.is_changed() {
        return;
    }
    for (mut text, mut colour, cell) in &mut labels {
        let kind = inventory.selected_kind(cell.0);
        let count = inventory.count(kind);
        let short: String =
            kind.label().chars().filter(|c| c.is_alphanumeric()).take(3).collect();
        text.0 = format!("{} {}", short.to_uppercase(), count);
        colour.0 = if count == 0 { Color::srgba(1.0, 1.0, 1.0, 0.35) } else { kind.colour() };
    }
    for (mut text, choice) in &mut choices {
        let kind = crate::inventory::HEAL_ITEMS[choice.index];
        text.0 = format!("{} {}", kind.label(), inventory.count(kind));
    }
}

/// Marks the label inside a condition cell.
#[derive(Component, Clone, Copy)]
pub(crate) struct ConditionCellLabel(pub(crate) crate::items::Category);

/// One condition cell: what it will spend, how much of that thing is left, and
/// the list it drops when dragged.
///
/// The list hangs off the cell with `position_type: Absolute` so opening it
/// cannot push the compass sideways, and `top: 100%` keeps it pinned under the
/// cell it belongs to wherever that cell ends up.
fn condition_cell(
    parent: &mut ChildSpawnerCommands,
    category: crate::items::Category,
    fill: Color,
    fill_marker: impl Component,
) {
    parent
        .spawn((
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
            ConditionCell(category),
        ))
        .with_children(|cell| {
            cell.spawn((
                text("--", 10.0, Color::srgba(1.0, 1.0, 1.0, 0.4)),
                ConditionCellLabel(category),
            ));
            cell.spawn((
                Node { width: Val::Px(GEAR_CELL - 12.0), height: Val::Px(3.0), ..default() },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12)),
                children![(
                    Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
                    BackgroundColor(fill),
                    fill_marker,
                )],
            ));
            cell.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Percent(100.0),
                    left: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    display: Display::None,
                    ..default()
                },
                ConditionList(category),
            ))
            .with_children(|list| {
                for (index, kind) in crate::inventory::HEAL_ITEMS.into_iter().enumerate() {
                    if kind.category() != category {
                        continue;
                    }
                    list.spawn((
                        Node {
                            width: Val::Px(120.0),
                            height: Val::Px(24.0),
                            border: UiRect::all(Val::Px(1.0)),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
                        BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.2)),
                        Button,
                        Interaction::default(),
                        ConditionChoice { category, index },
                        children![(
                            text(kind.label(), 11.0, kind.colour()),
                            ConditionChoiceLabel { index },
                        )],
                    ));
                }
            });
        });
}

/// The condition cells, flanking the compass.
///
/// They used to sit in the bottom bar beside the weapons, which made that bar
/// 970px of fixed-width cells on a screen 852pt wide — every cell shrank and
/// `overflow: clip()` on the weapon slots started eating the gun names. Moving
/// them up buys back 306px and puts "how much is left" next to the thing the
/// eyes are already on.
///
/// Split either side of the compass rather than grouped: what you have left on
/// the left, what you are wearing on the right.
///
/// Both roots carry `NativeUiRoot`, without which they are invisible on the
/// phone and only on the phone — the extractor walks from those markers and
/// nowhere else.
pub(crate) fn spawn_condition_hud(commands: &mut Commands) {
    const GAP: f32 = 4.0;
    const EDGE_GAP: f32 = 8.0;
    let half_compass = crate::COMPASS_WIDTH / 2.0;
    let left_width = GEAR_CELL * 2.0 + GAP;

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(-(half_compass + EDGE_GAP + left_width))),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(GAP),
                ..default()
            },
            GlobalZIndex(STATIC_UI_Z_INDEX),
            NativeUiRoot,
        ))
        .with_children(|left| {
            condition_cell(left, crate::items::Category::Healing, health_colour(1.0), HealthBarFill);
            condition_cell(
                left,
                crate::items::Category::Boost,
                Color::srgb(0.95, 0.80, 0.25),
                BoostBarFill,
            );
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Percent(50.0),
                margin: UiRect::left(Val::Px(half_compass + EDGE_GAP)),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(GAP),
                ..default()
            },
            // Above the bag's own scrim, which is STATIC_UI_Z_INDEX + 10 and
            // covers the whole screen. PACK opens the bag, so PACK has to stay
            // both visible and reachable while it is open or there is no way
            // back out of it with a thumb.
            GlobalZIndex(STATIC_UI_Z_INDEX + 20),
            NativeUiRoot,
        ))
        .with_children(|gear| {
            for slot in WearSlot::ALL {
                let mut cell = gear.spawn(stat_cell(
                    slot.label(),
                    Color::srgb(0.6, 0.95, 0.7),
                    GearCellLabel(slot),
                    GearCellWear(slot),
                ));
                if slot == WearSlot::Backpack {
                    cell.insert((
                        Button,
                        Interaction::default(),
                        crate::inventory_screen::InventoryToggleButton,
                    ));
                }
            }
        });
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
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                align_items: AlignItems::End,
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
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                // Clipped rather than allowed to spill: a
                                // name too long for its cell should lose its
                                // own tail, not cover the cell next door.
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
                            cell.spawn(Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(5.0),
                                ..default()
                            })
                            .with_children(|line| {
                                line.spawn(text(slot.label(), 11.0, Color::srgb(1.0, 0.85, 0.2)));
                                line.spawn((
                                    text("--", 12.0, Color::srgba(1.0, 1.0, 1.0, 0.85)),
                                    WeaponSlotLabel(slot),
                                ));
                            });
                            cell.spawn((
                                text("", 11.0, Color::srgba(1.0, 1.0, 1.0, 0.6)),
                                WeaponSlotAmmo(slot),
                            ));
                        });
                }
            });

            // Two by two rather than a stack of four. Four in a line made a
            // column as tall as the weapon slots beside it; side by side the
            // block is half the height and a thumb crosses it in one move.
            //
            // PUBG puts these on the weapon bar; here they are their own
            // block, because holding one is a different gesture from tapping
            // a weapon slot and the two should not be neighbours by accident.
            row.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(THROW_GAP),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|col| {
                for pair in THROW_GRID {
                    col.spawn(Node {
                        column_gap: Val::Px(THROW_GAP),
                        ..default()
                    })
                    .with_children(|line| {
                        for kind in pair {
                            line.spawn((
                                Node {
                                    width: Val::Px(THROW_BUTTON),
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
                    });
                }
                // How much fuse is left while one is in the hand. The only
                // thing worth seeing at that moment.
                col.spawn((
                    Node {
                        width: Val::Px(THROW_BUTTON * 2.0 + THROW_GAP),
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

        });
}

pub(crate) fn update_bars(
    boost: Res<Boost>,
    player: Query<&Health, With<Player>>,
    mut health_bar: Query<(&mut Node, &mut BackgroundColor), With<HealthBarFill>>,
    mut boost_bar: Query<&mut Node, (With<BoostBarFill>, Without<HealthBarFill>)>,
) {
    // Numbers no longer: the cell's own text says which medicine a tap spends,
    // and a bar directly under it already said how much was left.
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

/// The rounds and the fire mode, in the cell for the weapon they belong to.
///
/// A slot that is not in hand has no `Gun` of its own -- switching to one
/// hands it a full magazine -- so that is what its cell shows. The reserve is
/// the real count either way, since it lives in the inventory by calibre.
pub(crate) fn update_weapon_slot_ammo(
    loadout: Res<Loadout>,
    inventory: Res<Inventory>,
    fire_mode: Res<weapon::FireModeSelection>,
    gun: Query<&Gun>,
    mut labels: Query<(&WeaponSlotAmmo, &mut Text)>,
) {
    let held = gun.single().ok();
    for (label, mut text) in &mut labels {
        let Some(weapon) = loadout.get(label.0) else {
            text.0 = String::new();
            continue;
        };
        let def = weapon.def();
        let active = label.0 == loadout.active;
        text.0 = match held {
            Some(gun) if active && gun.reload_remaining > 0.0 => "RELOADING".to_string(),
            _ => {
                let mag = match held {
                    Some(gun) if active => gun.ammo,
                    _ => def.magazine,
                };
                let reserve = inventory.count(def.ammo);
                if def.fire_modes.len() > 1 {
                    format!("{} {} / {}", fire_mode.mode(def).label(), mag, reserve)
                } else {
                    format!("{mag} / {reserve}")
                }
            }
        };
    }
}

/// Tapping a weapon slot draws it; tapping the one already drawn walks its
/// fire modes.
///
/// The second tap had nothing to do -- `equip_slot` would re-request a
/// weapon already in hand -- and a phone has no Z to press, so the modes
/// were unreachable there at all. Putting them on the slot keeps them with
/// the gun they belong to, which is also where a thumb already is.
pub(crate) fn loadout_hud_buttons(
    mut loadout: ResMut<Loadout>,
    mut request: ResMut<weapon::WeaponSwitchRequest>,
    active: Res<ActiveWeapon>,
    mut fire_mode: ResMut<weapon::FireModeSelection>,
    mut gun: Query<&mut Gun>,
    slots: Query<(&Interaction, &WeaponSlotButton), Changed<Interaction>>,
) {
    for (interaction, slot) in &slots {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if loadout.active != slot.0 || loadout.get(slot.0).is_none() {
            inventory::equip_slot(&mut loadout, &mut request, slot.0);
            continue;
        }
        let def = active.def();
        if def.fire_modes.len() < 2 {
            continue;
        }
        fire_mode.next(def);
        // Touched so the ammo readout, which only rebuilds on a changed Gun,
        // picks the new mode up. Cheaper than making it rebuild every frame.
        if let Ok(mut gun) = gun.single_mut() {
            gun.set_changed();
        }
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
