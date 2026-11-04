use bevy::prelude::*;
use bevy_seedling::prelude as seedling;

use crate::*;

#[derive(Resource)]
pub(crate) struct GunshotSound(pub(crate) Vec<Handle<seedling::AudioSample>>);

impl GunshotSound {
    pub(crate) fn of(&self, weapon: WeaponId) -> Handle<seedling::AudioSample> {
        self.0[weapon.index()].clone()
    }
}

#[derive(Resource)]
pub(crate) struct ReloadSound(pub(crate) Vec<Handle<seedling::AudioSample>>);

/// What a shot sounds like with a can on it.
///
/// Two recordings, not one per weapon: once the muzzle blast is bottled,
/// what is left to hear is the calibre and the action rather than which
/// receiver they belong to. Both were cut from Pole Position takes on the
/// same microphone at the same metre as the rest of the weapons, so a
/// suppressed shot arrives in the same room as a bare one.
#[derive(Resource)]
pub(crate) struct SuppressedSound {
    /// A De Lisle in .45 ACP: spent in a third of a second, because a
    /// subsonic round leaves no crack behind once the blast is gone.
    pub(crate) pistol: Handle<seedling::AudioSample>,
    /// A Sako TRG in .308: it rings on, because a rifle round goes
    /// supersonic whatever is screwed to the muzzle and the crack travels
    /// with the bullet, out where no can reaches it.
    pub(crate) rifle: Handle<seedling::AudioSample>,
}

impl SuppressedSound {
    pub(crate) fn of(&self, class: WeaponClass) -> Handle<seedling::AudioSample> {
        match class {
            WeaponClass::Pistol | WeaponClass::Smg => self.pistol.clone(),
            _ => self.rifle.clone(),
        }
    }
}

/// What a cleared wave sounds like.
///
/// Inserted only when the files are actually on disk, so the wave mode can
/// be played and tuned before anyone has gone and found a zombie screaming.
/// `announce_wave` takes it as an Option and skips the sound when it is
/// missing, rather than the asset server logging a failure every start.
#[derive(Resource)]
pub(crate) struct WaveScreams(pub(crate) Vec<Handle<seedling::AudioSample>>);

impl ReloadSound {
    pub(crate) fn of(&self, weapon: WeaponId) -> Handle<seedling::AudioSample> {
        self.0[weapon.index()].clone()
    }
}

#[derive(Resource)]
pub(crate) struct FootstepSounds {
    pub(crate) player: Vec<Handle<seedling::AudioSample>>,
    pub(crate) zombie: Vec<Handle<seedling::AudioSample>>,
}

/// The marks a shooter gets told about: a head hit, a kill, and the ladder
/// for kills that land close enough together to be one piece of work.
#[derive(Resource)]
pub(crate) struct KillFeedbackSounds {
    pub(crate) headshot: Handle<seedling::AudioSample>,
    pub(crate) kill: Handle<seedling::AudioSample>,
    /// Double, triple, then everything above it.
    pub(crate) multi: Vec<Handle<seedling::AudioSample>>,
}

impl KillFeedbackSounds {
    /// The announcement for an n-kill streak, or nothing for the first.
    pub(crate) fn for_streak(&self, kills: u32) -> Option<Handle<seedling::AudioSample>> {
        if kills < 2 {
            return None;
        }
        // Saturating at the top rung rather than going silent: a five-kill
        // streak is still worth saying something about.
        let rung = ((kills - 2) as usize).min(self.multi.len().saturating_sub(1));
        self.multi.get(rung).cloned()
    }
}

/// How long after a kill another one still counts as part of the same streak.
/// Generous by arena-shooter standards, because a zombie takes longer to put
/// down than a player does.
pub(crate) const MULTI_KILL_WINDOW: f32 = 3.0;

const KILL_FEEDBACK_VOLUME: f32 = 0.8;
const MULTI_KILL_VOLUME: f32 = 1.0;

/// Plays one of these clips, which are announcements rather than world sounds
/// -- no position, no spatialisation, straight down the middle.
pub(crate) fn announce(commands: &mut Commands, sample: Handle<seedling::AudioSample>, volume: f32) {
    commands.spawn((
        seedling::SamplePlayer::new(sample).with_volume(seedling::Volume::Linear(volume)),
        seedling::PlaybackSettings { on_complete: seedling::OnComplete::Despawn, ..default() },
    ));
}

pub(crate) fn announce_hit(commands: &mut Commands, sample: Handle<seedling::AudioSample>) {
    announce(commands, sample, KILL_FEEDBACK_VOLUME);
}

pub(crate) fn announce_streak(commands: &mut Commands, sample: Handle<seedling::AudioSample>) {
    announce(commands, sample, MULTI_KILL_VOLUME);
}

/// A shot still ringing, and how loud it was.
///
/// The level is carried rather than inferred: what ducks the rest of the mix
/// has to know a suppressed shot from a bare one, and a tag alone cannot.
/// Quieting the sample while still cutting the room to a third is how a
/// suppressor ends up sounding loud -- the ear hears a small noise and the
/// whole world holding its breath around it.
#[derive(Component)]
pub(crate) struct GunshotVoice(pub(crate) f32);

/// The reload sample, tagged so a reload that stops early can take its own
/// sound with it -- a magazine slapped back in while the clatter of changing
/// it plays on is the one thing that would give the cancel away.
#[derive(Component)]
pub(crate) struct ReloadVoice;

#[derive(Component)]
pub(crate) struct ZombieFootstepVoice;

pub(crate) const PLAYER_FOOTSTEP_SURFACE: &str = "gravel";
pub(crate) const PLAYER_FOOTSTEP_SAMPLES: usize = 10;
pub(crate) const ZOMBIE_FOOTSTEP_SURFACE: &str = "grass";
pub(crate) const ZOMBIE_FOOTSTEP_SAMPLES: usize = 9;

const FOOTSTEP_WALK_INTERVAL: f32 = 0.45;
const FOOTSTEP_RUN_INTERVAL: f32 = 0.30;
const FOOTSTEP_PLAYER_VOLUME: f32 = 0.45;
const FOOTSTEP_PITCH_SPREAD: f32 = 0.12;

const ZOMBIE_FOOTFALL_PHASES: [f32; 2] = [0.0, 0.5];
const ZOMBIE_FOOTSTEP_VOLUME: f32 = 1.5;
const ZOMBIE_FOOTSTEP_SPATIAL_SCALE: f32 = 0.15;
const ZOMBIE_FOOTSTEP_AUDIBLE_RANGE: f32 = 25.0;
const ZOMBIE_FOOTSTEP_MAX_VOICES: usize = 4;
const ZOMBIE_FOOTSTEP_MIN_GAP: f32 = 0.07;
const GUNSHOT_DUCK_FACTOR: f32 = 0.35;

fn hash01(seed: f32) -> f32 {
    (ops::sin(seed) * 43758.547).fract().abs()
}

fn pick_sample(samples: &[Handle<seedling::AudioSample>], seed: f32) -> Option<Handle<seedling::AudioSample>> {
    if samples.is_empty() {
        return None;
    }
    let index = (hash01(seed) * samples.len() as f32) as usize % samples.len();
    samples.get(index).cloned()
}

fn pitch_jitter(seed: f32) -> f32 {
    1.0 + (hash01(seed) - 0.5) * 2.0 * FOOTSTEP_PITCH_SPREAD
}

pub(crate) fn play_footsteps(
    time: Res<Time>,
    moving: Res<LocalPlayerMoving>,
    sprinting: Res<LocalPlayerSprinting>,
    sounds: Option<Res<FootstepSounds>>,
    player: Query<&Player, Without<NetPlayer>>,
    mut until_next: Local<f32>,
    mut commands: Commands,
) {
    let Some(sounds) = sounds else {
        return;
    };
    let grounded = player.single().map(|p| p.grounded).unwrap_or(false);
    if !grounded || !moving.0 {
        *until_next = 0.0;
        return;
    }

    *until_next -= time.delta_secs();
    if *until_next > 0.0 {
        return;
    }
    *until_next = if sprinting.0 { FOOTSTEP_RUN_INTERVAL } else { FOOTSTEP_WALK_INTERVAL };

    let seed = time.elapsed_secs() * 311.7;
    let Some(sample) = pick_sample(&sounds.player, seed) else {
        return;
    };
    commands.spawn((
        seedling::SamplePlayer::new(sample).with_volume(seedling::Volume::Linear(FOOTSTEP_PLAYER_VOLUME)),
        seedling::PlaybackSettings {
            speed: pitch_jitter(seed * 1.7) as f64,
            on_complete: seedling::OnComplete::Despawn,
            ..default()
        },
    ));
}

fn animation_phase(
    anim_players: &Query<(&AnimationPlayer, &AnimationGraphHandle)>,
    graphs: &Assets<AnimationGraph>,
    clips: &Assets<AnimationClip>,
    anim_entity: Entity,
) -> Option<f32> {
    let (player, graph_handle) = anim_players.get(anim_entity).ok()?;
    let (node, active) = player.playing_animations().find(|(_, active)| !active.is_paused())?;
    let graph = graphs.get(&graph_handle.0)?;
    let AnimationNodeType::Clip(clip) = &graph.get(*node)?.node_type else {
        return None;
    };
    let duration = clips.get(clip)?.duration();
    if duration <= 0.0 {
        return None;
    }
    Some((active.seek_time() / duration).rem_euclid(1.0))
}

fn crossed_footfall(previous: f32, current: f32) -> bool {
    if current < previous {
        return true;
    }
    ZOMBIE_FOOTFALL_PHASES
        .iter()
        .any(|boundary| *boundary > 0.0 && previous < *boundary && current >= *boundary)
}

pub(crate) fn play_zombie_footsteps(
    time: Res<Time>,
    sounds: Option<Res<FootstepSounds>>,
    listener: Query<&GlobalTransform, With<FpsCamera>>,
    zombies: Query<(Entity, &Transform, &ZombieMotion, &ZombieAnimationPlayer), With<Zombie>>,
    anim_players: Query<(&AnimationPlayer, &AnimationGraphHandle)>,
    graphs: Res<Assets<AnimationGraph>>,
    clips: Res<Assets<AnimationClip>>,
    live_voices: Query<(), With<ZombieFootstepVoice>>,
    gunshots: Query<&GunshotVoice>,
    mut last_phase: Local<std::collections::HashMap<Entity, f32>>,
    mut last_step_at: Local<f32>,
    mut commands: Commands,
) {
    let Some(sounds) = sounds else {
        return;
    };
    let Ok(ear) = listener.single() else {
        return;
    };
    let ear = ear.translation();
    let now = time.elapsed_secs();

    last_phase.retain(|entity, _| zombies.contains(*entity));

    let mut due: Vec<(f32, Entity)> = Vec::new();
    for (entity, transform, motion, anim_link) in &zombies {
        let distance = transform.translation.distance(ear);
        if !motion.moving || distance > ZOMBIE_FOOTSTEP_AUDIBLE_RANGE {
            last_phase.remove(&entity);
            continue;
        }
        let Some(phase) = animation_phase(&anim_players, &graphs, &clips, anim_link.0) else {
            continue;
        };
        let Some(previous) = last_phase.insert(entity, phase) else {
            continue;
        };
        if crossed_footfall(previous, phase) {
            due.push((distance, entity));
        }
    }

    if due.is_empty() || now - *last_step_at < ZOMBIE_FOOTSTEP_MIN_GAP {
        return;
    }

    let budget = ZOMBIE_FOOTSTEP_MAX_VOICES.saturating_sub(live_voices.iter().count());
    if budget == 0 {
        return;
    }

    due.sort_by(|(a, _), (b, _)| a.total_cmp(b));
    // The loudest shot still ringing decides, and a silent one decides nothing.
    let loudest = gunshots.iter().fold(0.0f32, |most, shot| most.max(shot.0));
    let duck = 1.0 - (1.0 - GUNSHOT_DUCK_FACTOR) * loudest.clamp(0.0, 1.0);

    for (_, entity) in due.into_iter().take(budget) {
        let Ok((_, transform, _, _)) = zombies.get(entity) else {
            continue;
        };
        let seed = now * 97.3 + shared::entity_phase(entity) * 51.0;
        let Some(sample) = pick_sample(&sounds.zombie, seed) else {
            continue;
        };
        let volume = ZOMBIE_FOOTSTEP_VOLUME * duck;
        commands.spawn((
            // Into the spatial pool, which is where Steam Audio is wired in:
            // a footstep behind a wall now arrives through the wall rather
            // than merely quieter and to the left.
            seedling::SpatialPool,
            seedling::SamplePlayer::new(sample).with_volume(seedling::Volume::Linear(volume)),
            seedling::PlaybackSettings {
                speed: pitch_jitter(seed * 2.3) as f64,
                on_complete: seedling::OnComplete::Despawn,
                ..default()
            },
            seedling::SpatialScale(Vec3::splat(ZOMBIE_FOOTSTEP_SPATIAL_SCALE)),
            Transform::from_translation(transform.translation),
            ZombieFootstepVoice,
        ));
        *last_step_at = now;
    }
}
