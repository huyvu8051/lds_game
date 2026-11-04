//! The bottom-centre bar: the three weapon slots and the four throwables.
//!
//! What you are made of rather than what you carry -- the health and boost
//! bars, and the armour -- moved up beside the compass and lives in
//! `condition_hud`. This is the reaching-for half.
//!
//! Laid out the way a battle royale lays it out, because that arrangement
//! answers a real question in one glance: which gun is in hand, how much is
//! in it, and what else could be.

use bevy::prelude::*;

use crate::*;

const PANEL_BOTTOM: f32 = 16.0;
/// Wide enough for the longest weapon name. "DESERT EAGLE" overflowed 112
/// and spilled into whatever stood beside it.
const SLOT_WIDTH: f32 = 132.0;
/// Two lines now: the name, and the rounds under it. The readout used to be
/// its own box in the far corner, which meant checking the weapon and
/// checking what was in it were two looks at opposite ends of the screen.
pub(crate) const SLOT_HEIGHT: f32 = 38.0;

/// One button per kind, so a grenade can be cooked without first cycling to
/// it. Held, not tapped: the hold is the cook.
#[derive(Component)]
pub(crate) struct ThrowButton(pub(crate) ItemKind);
#[derive(Component)]
pub(crate) struct ThrowButtonLabel(pub(crate) ItemKind);
#[derive(Component)]
pub(crate) struct CookRing;

const THROW_BUTTON: f32 = 70.0;
const THROW_GAP: f32 = 3.0;
#[derive(Component)]
pub(crate) struct WeaponSlotButton(pub(crate) inventory::Slot);
#[derive(Component)]
pub(crate) struct WeaponSlotLabel(pub(crate) inventory::Slot);
#[derive(Component)]
pub(crate) struct WeaponSlotAmmo(pub(crate) inventory::Slot);
pub(crate) fn text(content: &str, size: f32, colour: Color) -> impl Bundle {
    (
        Text::new(content.to_string()),
        TextFont { font_size: FontSize::Px(size), ..default() },
        TextColor(colour),
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
            // Which throwable sits where is `inventory::THROWABLES`.
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
                for pair in inventory::THROWABLES {
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

/// The rounds, in the cell for the weapon they belong to.
///
/// A slot that is not in hand has no `Gun` of its own, so what it is holding
/// is read from the magazines the loadout keeps for it. A weapon that has
/// never been held has none kept, and comes out full. The reserve is the
/// real count either way, since it lives in the inventory by calibre.
pub(crate) fn update_weapon_slot_ammo(
    loadout: Res<Loadout>,
    inventory: Res<Inventory>,
    magazines: Res<inventory::Magazines>,
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
                let rounds = match held {
                    Some(gun) if active => (!gun.magazine_out).then_some(gun.ammo),
                    _ => magazines.peek(label.0, weapon).unwrap_or(Some(def.magazine)),
                };
                let reserve = inventory.count(def.ammo);
                match rounds {
                    // No magazine at all, which is not the same as an empty
                    // one and should not read as a weapon worth raising.
                    None => format!("-- / {reserve}"),
                    Some(mag) => format!("{mag} / {reserve}"),
                }
            }
        };
    }
}

pub(crate) fn loadout_hud_buttons(
    mut loadout: ResMut<Loadout>,
    mut request: ResMut<weapon::WeaponSwitchRequest>,
    slots: Query<(&Interaction, &WeaponSlotButton), Changed<Interaction>>,
) {
    for (interaction, slot) in &slots {
        if *interaction == Interaction::Pressed {
            inventory::equip_slot(&mut loadout, &mut request, slot.0);
        }
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
