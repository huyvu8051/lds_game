//! Endless waves, on their own scenario so the sandbox is left alone.
//!
//! The sandbox keeps a fixed live population and refills it a second after
//! anything dies. That is a good place to test a weapon and a poor place to
//! find out whether a weapon is enough, because the pressure never changes.
//! A wave answers a different question: here is a fixed amount of trouble,
//! can you finish it before it finishes you.
//!
//! Nothing here runs outside `Scenario::DesertSiege`, and the sandbox's own
//! respawn tick is gated off inside it -- the two would otherwise both be
//! deciding how many zombies there should be.

use bevy::prelude::*;

/// What a wave is allowed to spend. Rises by a flat amount so the early
/// waves grow fast in proportion and the late ones grow slowly, which is the
/// shape that keeps wave 3 from being twice wave 2 forever.
const WAVE_BUDGET_BASE: u32 = 12;
const WAVE_BUDGET_GAIN: u32 = 4;

/// What a zombie is born with, as a multiple of the sandbox's figure.
///
/// 0.11 was picked against the magazine rather than by feel. A zombie's
/// chest takes 1.5 a hit, an AK carries 30 rounds, so a full magazine stops
/// being enough to put one down by body shots at about wave 26. At 0.15 that
/// arrives at wave 20, which is early; at 0.08 it is wave 40, by which point
/// the curve has not been felt. The head is unaffected by any of it -- that
/// rule lives in combat.rs -- so the answer to a thickening zombie is to aim
/// rather than to carry more ammunition.
const WAVE_HEALTH_GAIN: f32 = 0.11;

/// What a cleared wave puts back in the pouches.
///
/// A floor rather than a gift: the reserve is topped *up to* this for the
/// calibres of the weapons actually being carried, so a careful player gets
/// nothing and a player who emptied everything gets a wave's worth. Nothing
/// is handed over for a calibre nobody is holding, which is what stops the
/// pack filling with rounds for weapons left on the ground.
///
/// Only in the wave mode. The sandbox keeps living off what it finds.
const WAVE_AMMO_FLOOR: u32 = 90;

/// How long the banner holds, and how long after a wave is cleared before
/// the next one begins. The same figure: the pause *is* the banner.
const WAVE_BREAK_SECONDS: f32 = 6.0;

/// How often the wave sends another handful, and how big a handful is.
///
/// A budget delivered all at once is one fight; delivered in bursts it is
/// several, and the gap is where you reload and move. The cap on how many
/// may be alive at once is `shared::ZOMBIE_MAX_COUNT`, which the sandbox
/// already respects.
const WAVE_BURST_SECONDS: f32 = 2.5;
const WAVE_BURST_SIZE: u32 = 4;

/// Whether this run is the wave mode at all.
pub(crate) fn in_wave_mode() -> bool {
    crate::game_flow::current_scenario() == shared::Scenario::DesertSiege
}

pub(crate) fn not_in_wave_mode() -> bool {
    !in_wave_mode()
}

/// Where a wave has got to.
///
/// `budget` is what is left to spend, not what the wave started with: a wave
/// is over when this reaches zero *and* the field is clear, and keeping only
/// the remainder means the two halves of that test read the same way.
#[derive(Resource)]
pub(crate) struct Wave {
    pub(crate) number: u32,
    budget: u32,
    burst_timer: f32,
    /// Counts down the pause after a wave is cleared. Zero while a wave runs.
    break_timer: f32,
    /// Set the frame a wave is cleared, read by the banner and the sound.
    pub(crate) just_cleared: Option<u32>,
}

impl Default for Wave {
    fn default() -> Self {
        // Wave one has not started: the break timer runs first, so the player
        // gets the same breath before the first wave as between any two.
        Self {
            number: 0,
            budget: 0,
            burst_timer: 0.0,
            break_timer: WAVE_BREAK_SECONDS,
            just_cleared: None,
        }
    }
}

impl Wave {
    fn budget_for(number: u32) -> u32 {
        WAVE_BUDGET_BASE + WAVE_BUDGET_GAIN * number.saturating_sub(1)
    }

    /// What a zombie born in this wave starts with.
    pub(crate) fn health(&self) -> f32 {
        let n = self.number.max(1) - 1;
        shared::ZOMBIE_MAX_HEALTH * (1.0 + WAVE_HEALTH_GAIN * n as f32)
    }

    /// What the wave is willing to send, as weights over the three kinds.
    ///
    /// Classic alone to begin with, because the first wave should read as the
    /// game the player already knows. The quick one arrives at three and the
    /// heavy one at five, and from there the mix shifts steadily towards the
    /// heavy one without ever dropping the others -- a wave of nothing but
    /// Soldiers is slow, and slow is not hard.
    fn kind_weights(&self) -> [f32; 3] {
        let n = self.number as f32;
        [
            1.0,
            if self.number >= 5 { (n - 4.0) * 0.12 } else { 0.0 },
            if self.number >= 3 { (n - 2.0) * 0.20 } else { 0.0 },
        ]
    }
}

/// Runs the wave: counts the break, spends the budget in bursts, and decides
/// when the wave is over.
pub(crate) fn advance_wave(
    time: Res<Time>,
    mut wave: ResMut<Wave>,
    points: Res<crate::map::ZombieSpawnPoints>,
    zombies: Query<(), With<shared::Zombie>>,
    mut commands: Commands,
) {
    wave.just_cleared = None;
    let dt = time.delta_secs();

    if wave.break_timer > 0.0 {
        wave.break_timer -= dt;
        if wave.break_timer > 0.0 {
            return;
        }
        wave.number += 1;
        wave.budget = Wave::budget_for(wave.number);
        wave.burst_timer = 0.0;
        info!("[wave] {} begins, budget {}", wave.number, wave.budget);
        return;
    }

    let alive = zombies.iter().count() as u32;

    if wave.budget == 0 {
        // Spent and empty: the wave is done. Nothing is spawned during the
        // break, so the field stays clear for as long as the banner is up.
        if alive > 0 {
            // Throttled by the burst timer, which is doing nothing else here.
            wave.burst_timer -= dt;
            if wave.burst_timer <= 0.0 {
                wave.burst_timer = 5.0;
                info!("[wave] {} spent, {} still alive", wave.number, alive);
            }
        }
        if alive == 0 {
            wave.just_cleared = Some(wave.number);
            wave.break_timer = WAVE_BREAK_SECONDS;
            info!("[wave] {} cleared", wave.number);
        }
        return;
    }

    wave.burst_timer -= dt;
    if wave.burst_timer > 0.0 {
        return;
    }
    wave.burst_timer = WAVE_BURST_SECONDS;

    if points.0.is_empty() {
        return;
    }

    let room = shared::ZOMBIE_MAX_COUNT.saturating_sub(alive);
    let mut sending = Vec::new();
    let weights = wave.kind_weights();
    let health = wave.health();
    for i in 0..WAVE_BURST_SIZE.min(room) {
        // The same phase trick the sandbox spawner uses to pick a kind: a
        // number that walks rather than repeats, so a burst is a mix.
        let phase = ((wave.number * 31 + i * 7) as f32 * 0.6180339).fract();
        let kind = shared::zombie_kind_for_phase(phase, weights);
        let cost = kind.stats().cost;
        if cost > wave.budget {
            break;
        }
        wave.budget -= cost;
        let point = points.0[(phase * points.0.len() as f32) as usize % points.0.len()];
        sending.push((point + Vec3::Y * shared::ZOMBIE_REST_HEIGHT, kind, health * kind.stats().health));
    }
    if !sending.is_empty() {
        info!(
            "[wave] {} burst of {}, budget {} left, {} alive",
            wave.number,
            sending.len(),
            wave.budget,
            alive
        );
        commands.trigger(SpawnWaveZombies(sending));
    }
}

/// One burst, as an event, so the spawning itself stays in `sim` beside the
/// sandbox's and the two cannot drift apart.
#[derive(Event)]
pub(crate) struct SpawnWaveZombies(pub(crate) Vec<(Vec3, shared::ZombieKind, f32)>);


/// The banner that says a wave is done.
///
/// Spawned on the clear and left to time itself out, the way a kill feed
/// entry does -- there is no state to keep, so there is no resource for it.
#[derive(Component)]
pub(crate) struct WaveBanner {
    remaining: f32,
}

const BANNER_SECONDS: f32 = 3.2;
const BANNER_FADE: f32 = 1.0;

pub(crate) fn announce_wave(
    mut commands: Commands,
    wave: Res<Wave>,
    screams: Option<Res<crate::audio::WaveScreams>>,
) {
    let Some(cleared) = wave.just_cleared else { return };

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(26.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        // Nothing behind it: a plate would read as a menu, and this is meant
        // to land over the fight rather than interrupt it.
        GlobalZIndex(40),
        WaveBanner { remaining: BANNER_SECONDS },
        children![
            (
                Text::new(format!("WAVE {cleared} CLEARED")),
                TextFont { font_size: FontSize::Px(44.0), ..default() },
                TextColor(Color::srgb(0.92, 0.22, 0.14)),
            ),
        ],
    ));

    if let Some(screams) = screams {
        if let Some(sample) = screams.0.first() {
            commands.spawn((
                bevy_seedling::prelude::SamplePlayer::new(sample.clone()),
                bevy_seedling::prelude::PlaybackSettings {
                    on_complete: bevy_seedling::prelude::OnComplete::Despawn,
                    ..default()
                },
            ));
        }
    }
}

pub(crate) fn fade_wave_banner(
    mut commands: Commands,
    time: Res<Time>,
    mut banners: Query<(Entity, &mut WaveBanner, &Children)>,
    mut colours: Query<&mut TextColor>,
) {
    for (entity, mut banner, children) in &mut banners {
        banner.remaining -= time.delta_secs();
        if banner.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let alpha = (banner.remaining / BANNER_FADE).clamp(0.0, 1.0);
        for &child in children {
            if let Ok(mut colour) = colours.get_mut(child) {
                colour.0.set_alpha(alpha);
            }
        }
    }
}


/// Tops the reserves up when a wave is cleared.
///
/// Separate from the banner so the two can be read apart, and so the one
/// that changes the game is not buried in the one that draws a word.
pub(crate) fn refill_between_waves(
    wave: Res<Wave>,
    loadout: Res<crate::inventory::Loadout>,
    mut inventory: ResMut<crate::inventory::Inventory>,
) {
    if wave.just_cleared.is_none() {
        return;
    }
    for slot in crate::inventory::Slot::ALL {
        let Some(weapon) = loadout.get(slot) else { continue };
        let kind = weapon.def().ammo;
        let had = inventory.count(kind);
        if had < WAVE_AMMO_FLOOR {
            inventory.add(kind, WAVE_AMMO_FLOOR - had);
            info!("[wave] refilled {} from {} to {}", kind.label(), had, WAVE_AMMO_FLOOR);
        }
    }
}
