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

use crate::keybinds::Bind;
use bevy_seedling::prelude as seedling;

use crate::*;

/// Seconds from the pin to the bang. PUBG's frag runs five.
pub(crate) const FUSE_SECONDS: f32 = 5.0;
/// How hard it leaves the hand, and the lob put on top.
///
/// 16 and 3 put it down at thirteen metres, which is a lob rather than a
/// throw -- a grenade is meant to clear a street. These reach about
/// eighteen, measured against the same damping the body gets.
const THROW_SPEED: f32 = 22.0;
const THROW_LIFT: f32 = 4.0;
const GRENADE_RADIUS: f32 = 0.055;
/// Avian takes density, and the collider is a 5.5cm ball, so this is what a
/// 400g grenade works out to. Left at the default it weighed 0.7g, which is
/// a ping-pong ball with a fuse in it.
const GRENADE_DENSITY: f32 = 574.0;
/// Averaged with the map's own 0.0, so the bounce it actually gets is half
/// this. Steel on concrete does come back, but a 400g casting full of
/// filler is not a ball bearing: at 0.2 it skipped down the street like a
/// thrown stone.
const GRENADE_RESTITUTION: f32 = 0.07;
/// A sphere has nothing to catch on, so the grip has to come from here or it
/// skates away down the street.
const GRENADE_FRICTION: f32 = 0.9;
/// Up with the restitution down: what kills a bounce is not only the
/// rebound but what is left of the speed afterwards.
const GRENADE_LINEAR_DAMPING: f32 = 0.55;
/// The heaviest hand here: a perfect ball rolls forever otherwise, and
/// nothing in the solver models a lumpy casting scrubbing itself to a stop.
const GRENADE_ANGULAR_DAMPING: f32 = 4.5;

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
    /// By kind rather than by position. A list meant the order it was built
    /// in and the order it was searched in had to stay the same list, and
    /// nothing said so.
    materials: bevy::platform::collections::HashMap<ItemKind, Handle<StandardMaterial>>,
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
        materials: inventory::throwables()
            .map(|kind| {
                let material = materials.add(StandardMaterial {
                    base_color: kind.colour(),
                    perceptual_roughness: 0.75,
                    ..default()
                });
                (kind, material)
            })
            .collect(),
    });
}

fn material_for(assets: &GrenadeAssets, kind: ItemKind) -> Handle<StandardMaterial> {
    assets.materials[&kind].clone()
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
    mut cooking: ResMut<Cooking>,
    mut inventory: ResMut<Inventory>,
    mut commands: Commands,
    assets: Option<Res<GrenadeAssets>>,
    sounds: Option<Res<GrenadeSounds>>,
    camera: Query<&GlobalTransform, With<FpsCamera>>,
) {
    let Some(assets) = assets else { return };

    if Bind::Grenade.just_pressed(&keys) {
        // The first one carried. There is no selected throwable any more --
        // the HUD gives each its own button, so choosing happens by pressing
        // the one you want, and only the key still needs told which.
        let kind = crate::inventory::throwables().find(|kind| inventory.count(*kind) > 0);
        if let (None, Some(kind)) = (cooking.kind, kind) {
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
    if !Bind::Grenade.just_released(&keys) && !out_of_time {
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
        ColliderDensity(GRENADE_DENSITY),
        Restitution::new(GRENADE_RESTITUTION),
        Friction::new(GRENADE_FRICTION),
        LinearDamping(GRENADE_LINEAR_DAMPING),
        AngularDamping(GRENADE_ANGULAR_DAMPING),
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

/// Where the weapon goes while a grenade is out: down and in, out of the
/// way, and it stays there.
///
/// It used to be given the throw itself -- back over the shoulder, pitched
/// up thirty-four degrees, yawed twenty-six -- so the rifle was the thing
/// being thrown. A weapon held in one hand while the other hand works does
/// not wind up; it drops and waits.
const LOWER_DROP: Vec3 = Vec3::new(0.035, -0.115, 0.055);
const LOWER_PITCH_DEG: f32 = 17.0;
const LOWER_YAW_DEG: f32 = 7.0;
/// The nudge the weapon takes when the throwing arm snaps forward, and how
/// long it runs. Small: it is felt through the shoulder, not swung.
const FOLLOW_SECONDS: f32 = 0.42;
const FOLLOW_PITCH_DEG: f32 = -7.0;
const FOLLOW_PUSH: f32 = 0.045;

/// The grenade hand, in the eye's own frame.
///
/// Rest is where the weak hand already lives on the fore-grip; the rest are
/// the beats of a throw -- it comes up, the pin is torn out, it is held
/// cocked, then it goes back past the ear and over.
const HAND_READY: Vec3 = Vec3::new(-0.17, -0.14, -0.30);
const HAND_PIN_TUG: Vec3 = Vec3::new(-0.11, -0.17, -0.26);
const HAND_COCKED: Vec3 = Vec3::new(-0.21, 0.02, -0.12);
const HAND_RELEASE: Vec3 = Vec3::new(-0.10, 0.05, -0.52);
/// How long the pin takes to come out, and how long the arm spends going
/// over the top once the hand has opened.
const PIN_SECONDS: f32 = 0.28;
const THROW_SECONDS: f32 = 0.26;

/// Where the throw is, so the weapon and the hand can both read it.
#[derive(Resource, Default)]
pub(crate) struct ThrowPose {
    /// Zero to one as the weapon lowers and the grenade comes up.
    pub(crate) wind: f32,
    /// Counts down through the follow-through.
    pub(crate) follow: f32,
    /// Counts down while the pin is being pulled.
    pub(crate) pin: f32,
}

impl ThrowPose {
    /// The offset and turn to add to the weapon this frame.
    pub(crate) fn pose(&self) -> (Vec3, f32, f32) {
        // The follow-through is eased out rather than linear: an arm that
        // decelerates evenly reads as a machine returning to a rest position.
        let release = (self.follow / FOLLOW_SECONDS).clamp(0.0, 1.0);
        let swing = release * release;
        let offset = LOWER_DROP * self.wind - Vec3::Z * FOLLOW_PUSH * swing;
        let pitch = (LOWER_PITCH_DEG * self.wind + FOLLOW_PITCH_DEG * swing).to_radians();
        let yaw = (LOWER_YAW_DEG * self.wind).to_radians();
        (offset, pitch, yaw)
    }

    /// Whether the weak hand is off the weapon holding a grenade.
    pub(crate) fn hand_busy(&self) -> bool {
        self.wind > 0.02 || self.follow > 0.0
    }

    /// Where that hand is, given where it rests on the weapon.
    pub(crate) fn hand(&self, rest: Vec3) -> Vec3 {
        let smooth = |t: f32| t * t * (3.0 - 2.0 * t);
        if self.follow > 0.0 {
            // Over the top: back past the ear first, then forward fast. The
            // grenade is already gone by the time the arm is forward, which
            // is why the release spawns it rather than this does.
            let t = 1.0 - (self.follow / FOLLOW_SECONDS).clamp(0.0, 1.0);
            let over = (t / (THROW_SECONDS / FOLLOW_SECONDS)).min(1.0);
            let forward = HAND_COCKED.lerp(HAND_RELEASE, smooth(over));
            return forward.lerp(rest, smooth(((t - 0.45) / 0.55).clamp(0.0, 1.0)));
        }
        let up = smooth(self.wind);
        if self.pin > 0.0 {
            // The tug is a there-and-back inside the raise, not a detour
            // from it: the hand is still on its way up while it pulls.
            let p = 1.0 - (self.pin / PIN_SECONDS).clamp(0.0, 1.0);
            let yank = (p * std::f32::consts::PI).sin();
            let base = rest.lerp(HAND_READY, up);
            return base.lerp(HAND_PIN_TUG, yank * 0.8);
        }
        rest.lerp(HAND_COCKED, up)
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
    if !*was_cooking && cooking_now {
        pose.pin = PIN_SECONDS;
    }
    *was_cooking = cooking_now;
    pose.pin = (pose.pin - dt).max(0.0);

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



/// The grenade in the weak hand, and that hand's path through the throw.
///
/// Spawned and moved rather than parented to the hand bone: the bone is
/// solved by IK from this very target, so hanging the mesh off it would
/// make the thing being aimed at depend on where the aiming landed.
#[derive(Component)]
pub(crate) struct HeldGrenade;

/// Walks the weak hand through the throw, overriding the reload path.
///
/// The same target entity the reload and the sight swap use, because the
/// solver takes one and three authorities writing it would fight. This runs
/// after both, so while a grenade is out it is the one that decides.
pub(crate) fn drive_throw_hand(
    pose: Res<ThrowPose>,
    camera: Query<&GlobalTransform, With<crate::FpsCamera>>,
    fore: Query<&GlobalTransform, With<crate::ForeGripSocket>>,
    mut hand_bids: ResMut<crate::weak_hand::WeakHand>,
) {
    if !pose.hand_busy() {
        return;
    }
    let Ok(eye) = camera.single() else { return };
    let (rest, turn) = fore
        .single()
        .map(|f| {
            let local = f.reparented_to(eye);
            (local.translation, local.rotation)
        })
        .unwrap_or((HAND_READY, Quat::IDENTITY));
    hand_bids.claim(crate::weak_hand::THROW, pose.hand(rest), turn);
}

/// Keeps a grenade in that hand for as long as one is out.
pub(crate) fn carry_held_grenade(
    mut commands: Commands,
    pose: Res<ThrowPose>,
    cooking: Res<Cooking>,
    assets: Option<Res<GrenadeAssets>>,
    camera: Query<Entity, With<crate::FpsCamera>>,
    target: Query<&Transform, With<crate::ReloadHandTarget>>,
    held: Query<Entity, With<HeldGrenade>>,
) {
    let Ok(camera) = camera.single() else { return };
    // Gone the instant the hand opens: the thrown one is a body in the world
    // from that frame on, and two of them in the air reads as a duplicate.
    let kind = cooking.kind.filter(|_| pose.follow <= 0.0);
    let (Some(kind), Some(assets)) = (kind, assets.as_ref()) else {
        for entity in &held {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Ok(hand) = target.single() else { return };
    // Out of the palm rather than inside it, and turned so the fuse sits up.
    let at = Transform::from_translation(hand.translation + Vec3::new(0.0, 0.015, -0.01))
        .with_rotation(Quat::from_rotation_x(-0.35));
    match held.single() {
        Ok(entity) => {
            commands.entity(entity).insert(at);
        }
        Err(_) => {
            commands.spawn((
                Name::new("HeldGrenade"),
                Mesh3d(assets.mesh.clone()),
                MeshMaterial3d(material_for(assets, kind)),
                at,
                crate::GunMeshPart,
                HeldGrenade,
                ChildOf(camera),
            ));
        }
    }
}
