//! Throwables, from the pin to the bang.
//!
//! Cooked rather than simply thrown: the fuse starts when the pin comes out,
//! not when the thing leaves the hand. That is the whole skill in a grenade
//! -- a frag thrown fresh gives everyone five seconds to walk away from it --
//! and it is why the remaining fuse is written on the grenade itself rather
//! than left for the player to count under their breath.
//!
//! Hold to cook, release to throw. Cook too long and it goes off where you
//! are standing, which is the other half of the same bargain.

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_seedling::prelude as seedling;

use crate::*;

/// Seconds from the pin to the bang. PUBG's frag runs five.
pub(crate) const FUSE_SECONDS: f32 = 5.0;
/// How hard it leaves the hand, and the lob put on top.
const THROW_SPEED: f32 = 16.0;
const THROW_LIFT: f32 = 3.0;
const GRENADE_RADIUS: f32 = 0.055;

/// Radius and shove of a frag. A molotov uses the same, weaker and wider.
const FRAG_RADIUS: f32 = 6.5;
const FRAG_FORCE: f32 = 28.0;
const MOLOTOV_RADIUS: f32 = 4.5;
const MOLOTOV_FORCE: f32 = 12.0;

/// What is in the hand with the pin out.
#[derive(Resource, Default)]
pub(crate) struct Cooking {
    pub(crate) kind: Option<ItemKind>,
    pub(crate) elapsed: f32,
}

#[derive(Component)]
pub(crate) struct Grenade {
    pub(crate) kind: ItemKind,
    pub(crate) fuse: f32,
}

/// The countdown drawn over the grenade. A node rather than world text,
/// positioned the same way floating damage is -- projected each frame, which
/// is cheap for the one or two of these that exist at a time.
#[derive(Component)]
pub(crate) struct FuseLabel(pub(crate) Entity);

#[derive(Resource)]
pub(crate) struct GrenadeAssets {
    mesh: Handle<Mesh>,
    materials: Vec<Handle<StandardMaterial>>,
}

pub(crate) fn init_grenade_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(GrenadeAssets {
        // A sphere, because a grenade is one and because a thrown cube reads
        // as a crate every time it tumbles.
        mesh: meshes.add(Sphere::new(GRENADE_RADIUS).mesh().ico(3).unwrap()),
        materials: inventory::THROWABLES
            .iter()
            .map(|kind| {
                materials.add(StandardMaterial {
                    base_color: kind.colour(),
                    perceptual_roughness: 0.75,
                    ..default()
                })
            })
            .collect(),
    });
}

fn material_for(assets: &GrenadeAssets, kind: ItemKind) -> Handle<StandardMaterial> {
    let index = inventory::THROWABLES.iter().position(|k| *k == kind).unwrap_or(0);
    assets.materials[index.min(assets.materials.len() - 1)].clone()
}

/// Starts the fuse. Called by the key and by the on-screen buttons alike.
pub(crate) fn begin_cook(cooking: &mut Cooking, inventory: &Inventory, kind: ItemKind) {
    if cooking.kind.is_some() || inventory.count(kind) == 0 {
        return;
    }
    cooking.kind = Some(kind);
    cooking.elapsed = 0.0;
}

/// Lets go. Hands back what was thrown, or nothing if the hand was empty.
pub(crate) fn release_cook(cooking: &mut Cooking) -> Option<(ItemKind, f32)> {
    let kind = cooking.kind.take()?;
    Some((kind, (FUSE_SECONDS - cooking.elapsed).max(0.0)))
}

pub(crate) fn cook_and_throw(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    selection: Res<ThrowableSelection>,
    mut cooking: ResMut<Cooking>,
    mut inventory: ResMut<Inventory>,
    mut commands: Commands,
    assets: Option<Res<GrenadeAssets>>,
    sounds: Option<Res<GrenadeSounds>>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
) {
    let Some(assets) = assets else { return };

    if keys.just_pressed(KeyCode::KeyG) {
        let kind = selection.kind();
        if cooking.kind.is_none() && inventory.count(kind) > 0 {
            begin_cook(&mut cooking, &inventory, kind);
            if let Some(sounds) = sounds.as_ref() {
                play_sample(&mut commands, sounds.pin.clone(), 0.9);
            }
        }
    }

    if cooking.kind.is_none() {
        return;
    }
    cooking.elapsed += time.delta_secs();

    // Held past the fuse and it goes off in the hand. The alternative --
    // capping the cook and holding it safely for ever -- removes the only
    // thing that makes cooking a decision.
    let out_of_time = cooking.elapsed >= FUSE_SECONDS;
    if !keys.just_released(KeyCode::KeyG) && !out_of_time {
        return;
    }
    let Ok(eye) = camera.single() else { return };

    if out_of_time {
        let Some((kind, _)) = release_cook(&mut cooking) else { return };
        if !inventory.take_one(kind) {
            return;
        }
        // In the hand, so it is thrown nowhere.
        commands.trigger(detonation_for(kind, eye.translation()));
        if let Some(sounds) = sounds.as_ref() {
            play_sample(&mut commands, sounds.blast.clone(), 1.0);
        }
        return;
    }

    throw_now(&mut commands, &mut cooking, &mut inventory, &assets, sounds.as_deref(), eye);
}

/// Lets go of whatever is cooking and puts it in the air.
///
/// Shared by the key and by the on-screen buttons, so the two cannot drift:
/// a grenade thrown by a thumb has to leave the hand exactly as one thrown
/// by a key does.
pub(crate) fn throw_now(
    commands: &mut Commands,
    cooking: &mut Cooking,
    inventory: &mut Inventory,
    assets: &GrenadeAssets,
    sounds: Option<&GrenadeSounds>,
    eye: &GlobalTransform,
) {
    let Some((kind, fuse)) = release_cook(cooking) else { return };
    if !inventory.take_one(kind) {
        return;
    }
    if let Some(sounds) = sounds {
        play_sample(commands, sounds.throw.clone(), 0.8);
    }
    commands.spawn((
        Name::new("Grenade"),
        Mesh3d(assets.mesh.clone()),
        MeshMaterial3d(material_for(assets, kind)),
        Transform::from_translation(eye.translation() + eye.forward() * 0.45),
        RigidBody::Dynamic,
        Collider::sphere(GRENADE_RADIUS),
        Restitution::new(0.35),
        LinearVelocity(eye.forward() * THROW_SPEED + Vec3::Y * THROW_LIFT),
        AngularVelocity(Vec3::new(7.0, 4.0, 5.0)),
        Grenade { kind, fuse },
    ));
}

fn detonation_for(kind: ItemKind, at: Vec3) -> crate::explosion::Explosion {
    let (radius, force) = match kind {
        ItemKind::Molotov => (MOLOTOV_RADIUS, MOLOTOV_FORCE),
        _ => (FRAG_RADIUS, FRAG_FORCE),
    };
    crate::explosion::Explosion { at, radius, force }
}

/// Runs the fuse down and sets the thing off.
pub(crate) fn advance_grenades(
    time: Res<Time>,
    mut commands: Commands,
    sounds: Option<Res<GrenadeSounds>>,
    mut grenades: Query<(Entity, &mut Grenade, &Transform)>,
) {
    for (entity, mut grenade, transform) in &mut grenades {
        grenade.fuse -= time.delta_secs();
        if grenade.fuse > 0.0 {
            continue;
        }
        let at = transform.translation;
        match grenade.kind {
            ItemKind::FragGrenade | ItemKind::Molotov => {
                commands.trigger(detonation_for(grenade.kind, at));
                if let Some(sounds) = sounds.as_ref() {
                    play_sample(&mut commands, sounds.blast.clone(), 1.0);
                }
            }
            ItemKind::SmokeGrenade => {
                commands.trigger(SmokeStarted { at });
                if let Some(sounds) = sounds.as_ref() {
                    play_sample(&mut commands, sounds.hiss.clone(), 0.9);
                }
            }
            ItemKind::StunGrenade => {
                commands.trigger(FlashStarted { at });
                if let Some(sounds) = sounds.as_ref() {
                    play_sample(&mut commands, sounds.flash.clone(), 1.0);
                }
            }
            _ => {}
        }
        commands.entity(entity).try_despawn();
    }
}

#[derive(Event, Clone, Copy)]
pub(crate) struct SmokeStarted {
    pub(crate) at: Vec3,
}

#[derive(Event, Clone, Copy)]
pub(crate) struct FlashStarted {
    pub(crate) at: Vec3,
}

#[derive(Resource)]
pub(crate) struct GrenadeSounds {
    pub(crate) pin: Handle<seedling::AudioSample>,
    pub(crate) throw: Handle<seedling::AudioSample>,
    pub(crate) blast: Handle<seedling::AudioSample>,
    pub(crate) hiss: Handle<seedling::AudioSample>,
    pub(crate) flash: Handle<seedling::AudioSample>,
}

pub(crate) fn play_sample(commands: &mut Commands, sample: Handle<seedling::AudioSample>, volume: f32) {
    commands.spawn((
        seedling::SamplePlayer::new(sample).with_volume(seedling::Volume::Linear(volume)),
        seedling::PlaybackSettings { on_complete: seedling::OnComplete::Despawn, ..default() },
    ));
}

/// Keeps a countdown over every grenade in the air, and one over the hand
/// while something is cooking in it.
pub(crate) fn update_fuse_labels(
    mut commands: Commands,
    cooking: Res<Cooking>,
    camera: Query<(&Camera, &GlobalTransform), With<FpsCamera>>,
    grenades: Query<(Entity, &Grenade, &GlobalTransform)>,
    mut labels: Query<(Entity, &FuseLabel, &mut Node, &mut Text, &mut TextColor)>,
) {
    let Ok((camera, eye)) = camera.single() else { return };

    // One label per grenade, made and unmade with it.
    for (entity, _, _) in &grenades {
        if !labels.iter().any(|(_, label, ..)| label.0 == entity) {
            commands.spawn((
                Text::new(String::new()),
                TextFont { font_size: FontSize::Px(18.0), ..default() },
                TextColor(Color::srgb(1.0, 0.85, 0.2)),
                Node { position_type: PositionType::Absolute, display: Display::None, ..default() },
                GlobalZIndex(STATIC_UI_Z_INDEX),
                FuseLabel(entity),
                NativeUiRoot,
            ));
        }
    }

    for (label_entity, label, mut node, mut text, mut colour) in &mut labels {
        let Ok((_, grenade, global)) = grenades.get(label.0) else {
            commands.entity(label_entity).try_despawn();
            continue;
        };
        text.0 = format!("{:.1}", grenade.fuse.max(0.0));
        // Red for the last second, which is the only one worth reading.
        *colour = TextColor(if grenade.fuse <= 1.0 {
            Color::srgb(1.0, 0.3, 0.25)
        } else {
            Color::srgb(1.0, 0.85, 0.2)
        });
        match camera.world_to_viewport(eye, global.translation()) {
            Ok(screen) => {
                node.display = Display::Flex;
                node.left = Val::Px(screen.x);
                node.top = Val::Px(screen.y);
            }
            Err(_) => node.display = Display::None,
        }
    }
    let _ = &cooking;
}

/// How long a smoke stands and how wide it grows.
const SMOKE_SECONDS: f32 = 14.0;
const SMOKE_RADIUS: f32 = 3.2;
const SMOKE_GROW_SECONDS: f32 = 1.8;

#[derive(Component)]
pub(crate) struct SmokeCloud {
    remaining: f32,
}

/// A smoke is one large soft sphere rather than a cloud of billboards.
///
/// Billboards are what a smoke grenade is in a game with a particle budget;
/// this one is running on a phone that is already warm, and a hundred
/// overlapping transparent quads is exactly the shape of frame cost that put
/// it there. One sphere blocks the sight line, which is what the grenade is
/// thrown for.
pub(crate) fn on_smoke_started(
    trigger: On<SmokeStarted>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let SmokeStarted { at } = *trigger.event();
    commands.spawn((
        Name::new("Smoke"),
        Mesh3d(meshes.add(Sphere::new(1.0).mesh().ico(4).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.82, 0.82, 0.84, 0.92),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        })),
        Transform::from_translation(at + Vec3::Y * 0.6).with_scale(Vec3::splat(0.2)),
        SmokeCloud { remaining: SMOKE_SECONDS },
    ));
}

pub(crate) fn advance_smoke(
    time: Res<Time>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut clouds: Query<(
        Entity,
        &mut SmokeCloud,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
) {
    for (entity, mut cloud, mut transform, material) in &mut clouds {
        cloud.remaining -= time.delta_secs();
        if cloud.remaining <= 0.0 {
            commands.entity(entity).try_despawn();
            continue;
        }
        let age = SMOKE_SECONDS - cloud.remaining;
        let grown = (age / SMOKE_GROW_SECONDS).clamp(0.0, 1.0);
        transform.scale = Vec3::splat(0.2 + (SMOKE_RADIUS - 0.2) * grown);
        // Thins out over its last two seconds rather than vanishing, so the
        // sight line comes back gradually and nobody is surprised by it.
        let fade = (cloud.remaining / 2.0).clamp(0.0, 1.0);
        if let Some(mut material) = materials.get_mut(&material.0) {
            material.base_color.set_alpha(0.92 * fade);
        }
    }
}

/// How long the screen stays white, and how far a flash reaches.
const FLASH_SECONDS: f32 = 2.6;
const FLASH_RANGE: f32 = 12.0;

/// How blind the player is, one to nothing.
#[derive(Resource, Default)]
pub(crate) struct Flashed(pub(crate) f32);

/// Scaled by distance and by whether it went off behind you.
///
/// Facing is what makes a flash a skill rather than a dice roll: turning
/// away works, and it only works if the game checks.
pub(crate) fn on_flash_started(
    trigger: On<FlashStarted>,
    mut flashed: ResMut<Flashed>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
) {
    let FlashStarted { at } = *trigger.event();
    let Ok(eye) = camera.single() else { return };
    let to = at - eye.translation();
    let distance = to.length();
    if distance > FLASH_RANGE {
        return;
    }
    let near = 1.0 - (distance / FLASH_RANGE).clamp(0.0, 1.0);
    let facing = eye.forward().dot(to.normalize_or_zero()).clamp(0.0, 1.0);
    // A quarter of it lands even with your back turned, which is what
    // happens: the light comes off the walls.
    let strength = near * (0.25 + 0.75 * facing);
    flashed.0 = flashed.0.max(strength);
}

pub(crate) fn advance_flash(time: Res<Time>, mut flashed: ResMut<Flashed>) {
    if flashed.0 > 0.0 {
        flashed.0 = (flashed.0 - time.delta_secs() / FLASH_SECONDS).max(0.0);
    }
}

#[derive(Component)]
pub(crate) struct FlashOverlay;

pub(crate) fn spawn_flash_overlay(commands: &mut Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.0)),
        // Over everything, including the menus: a flash that the HUD punches
        // through is not a flash.
        GlobalZIndex(STATIC_UI_Z_INDEX + 100),
        FlashOverlay,
        NativeUiRoot,
    ));
}

pub(crate) fn update_flash_overlay(
    flashed: Res<Flashed>,
    mut overlay: Query<&mut BackgroundColor, With<FlashOverlay>>,
) {
    if !flashed.is_changed() {
        return;
    }
    let Ok(mut colour) = overlay.single_mut() else { return };
    // Squared on the way out, so it holds white and then clears quickly
    // rather than spending the whole two seconds as a grey veil.
    *colour = BackgroundColor(Color::srgba(1.0, 1.0, 1.0, flashed.0 * flashed.0));
}

/// How the arm moves through a throw: cocked back while cooking, then over
/// the top. PUBG's shape -- the weapon drops out of the way, the hand comes
/// back past the ear, and the follow-through carries the whole view with it.
const WIND_BACK: Vec3 = Vec3::new(0.10, -0.02, 0.22);
const WIND_PITCH_DEG: f32 = -34.0;
const WIND_YAW_DEG: f32 = 26.0;
/// How long the follow-through runs after the hand opens.
const FOLLOW_SECONDS: f32 = 0.42;
const FOLLOW_PITCH_DEG: f32 = 30.0;
const FOLLOW_PUSH: f32 = 0.30;

/// Where the throw is in its arc, so the weapon pose can read it.
#[derive(Resource, Default)]
pub(crate) struct ThrowPose {
    /// Zero to one as the arm winds back.
    pub(crate) wind: f32,
    /// Counts down through the follow-through.
    pub(crate) follow: f32,
}

impl ThrowPose {
    /// The offset and turn to add to the weapon this frame.
    pub(crate) fn pose(&self) -> (Vec3, f32, f32) {
        // The follow-through is eased out rather than linear: an arm that
        // decelerates evenly reads as a machine returning to a rest position.
        let release = (self.follow / FOLLOW_SECONDS).clamp(0.0, 1.0);
        let swing = release * release;
        let offset = WIND_BACK * self.wind - Vec3::Z * FOLLOW_PUSH * swing;
        let pitch = (WIND_PITCH_DEG * self.wind + FOLLOW_PITCH_DEG * swing).to_radians();
        let yaw = (WIND_YAW_DEG * self.wind).to_radians();
        (offset, pitch, yaw)
    }

}

/// Winds the arm while something is cooking and lets it go when it is not.
pub(crate) fn advance_throw_pose(
    time: Res<Time>,
    cooking: Res<Cooking>,
    mut pose: ResMut<ThrowPose>,
    mut was_cooking: Local<bool>,
) {
    let dt = time.delta_secs();
    let cooking_now = cooking.kind.is_some();
    // The release edge, not the cook state: the follow-through belongs to
    // the moment the hand opened, and a hand that opens on an empty pack
    // never wound back to begin with.
    if *was_cooking && !cooking_now && pose.wind > 0.2 {
        pose.follow = FOLLOW_SECONDS;
    }
    *was_cooking = cooking_now;

    // Fast to wind, because the pin is already out; slower to return.
    let target = if cooking_now { 1.0 } else { 0.0 };
    let rate = if cooking_now { 9.0 } else { 14.0 };
    pose.wind += (target - pose.wind) * (rate * dt).min(1.0);
    pose.follow = (pose.follow - dt).max(0.0);
}

/// How far a blast is felt, and how hard at the centre.
///
/// Further than it throws anybody: the shake is what a near miss feels like,
/// and a near miss is the common case.
const SHAKE_RANGE: f32 = 22.0;

/// The blow a blast lands on the camera.
///
/// Built as a ShockKick and handed to the same FireShock a rifle uses, so
/// there is one ring-down in the game rather than two that have to be kept
/// looking alike. Slower and heavier than any weapon's: the frequencies sit
/// at the bottom of what 60fps can carry without aliasing, and the decay is
/// long enough that the shake outlasts the sound.
fn blast_kick(strength: f32) -> weapon::ShockKick {
    weapon::ShockKick {
        view_pitch_deg: 3.4 * strength,
        view_yaw_deg: 2.6 * strength,
        // Roll carries the most, as it does for gunfire: it costs no aim,
        // because it does not move where the barrel points.
        view_roll_deg: 5.2 * strength,
        gun_pitch_deg: 7.0 * strength,
        gun_yaw_deg: 5.5 * strength,
        gun_roll_deg: 9.0 * strength,
        gun_shift_m: 0.05 * strength,
        freq_pitch: 7.4,
        freq_yaw: 5.9,
        freq_roll: 4.3,
        decay: 1.9,
    }
}

/// Shakes the view when something goes off nearby.
///
/// Hung off Explosion rather than off the grenade, so anything that ever
/// explodes shakes the camera without having to remember to.
pub(crate) fn shake_on_explosion(
    trigger: On<crate::explosion::Explosion>,
    time: Res<Time>,
    mut shock: ResMut<crate::movement_combat::FireShock>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
) {
    let crate::explosion::Explosion { at, .. } = *trigger.event();
    let Ok(eye) = camera.single() else { return };
    let distance = eye.translation().distance(at);
    if distance >= SHAKE_RANGE {
        return;
    }
    // Falls off with the square, the way the energy does. Linear falloff
    // makes a blast across the street feel like one at your feet.
    let near = 1.0 - distance / SHAKE_RANGE;
    let strength = near * near;
    if strength < 0.02 {
        return;
    }
    shock.strike(blast_kick(strength), time.elapsed_secs());
}
