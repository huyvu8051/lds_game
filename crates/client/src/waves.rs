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
use shared::ZombieKind;

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
    /// Everything this wave will send, decided the moment it begins and
    /// drained a burst at a time.
    ///
    /// A list rather than a running budget, because a budget cannot be
    /// counted in zombies: a Soldier costs three and is still one of them,
    /// so "12 left to spend" is anywhere between four and twelve bodies. The
    /// HUD says how many are left to kill, and that number has to be true.
    pending: Vec<shared::ZombieKind>,
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
            pending: Vec::new(),
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

    /// How many are left to kill: the ones still standing plus the ones
    /// this wave has not sent yet.
    pub(crate) fn left(&self, alive: u32) -> u32 {
        self.pending.len() as u32 + alive
    }

    /// Turns a budget into a crowd, once, when the wave starts.
    fn roll(&mut self) {
        let budget = Self::budget_for(self.number);
        let weights = self.kind_weights();
        let mut spent = 0;
        let mut i = 0u32;
        self.pending.clear();
        while spent < budget {
            // A number that walks rather than repeats, so the crowd is a mix
            // rather than runs of one kind.
            let phase = ((self.number * 31 + i * 7) as f32 * 0.6180339).fract();
            let kind = shared::zombie_kind_for_phase(phase, weights);
            let cost = kind.stats().cost;
            // The cheap kinds all cost one, so the remainder always finds
            // something to buy and this cannot spin.
            if spent + cost > budget {
                self.pending.push(shared::ZombieKind::Classic);
                break;
            }
            spent += cost;
            self.pending.push(kind);
            i += 1;
        }
    }

    /// What a zombie born in this wave starts with.
    pub(crate) fn health(&self) -> f32 {
        let n = self.number.max(1) - 1;
        shared::ZOMBIE_MAX_HEALTH * (1.0 + WAVE_HEALTH_GAIN * n as f32)
    }

    /// What the wave is willing to send, as weights over the kinds.
    ///
    /// Classic alone to begin with, because the first wave should read as the
    /// game the player already knows. Everything else joins on its own wave
    /// and climbs from there, and the classic never drops out -- a wave of
    /// nothing but soldiers is slow, and slow is not hard.
    ///
    /// The order they arrive in is the order they change the fight: another
    /// body first, then something that takes a magazine, then something that
    /// reaches you early, then armour, then the thing you have to walk
    /// backwards from.
    ///
    /// The female's note here used to say she was dropped for want of clips
    /// and 192MB of texture. The Mixamo rebuild gave her thirteen clips of
    /// her own and 1.5MB, so she is in from wave two with the rest.
    fn kind_weights(&self) -> [f32; ZombieKind::COUNT] {
        let from = |wave: u32, rate: f32| {
            if self.number >= wave { (self.number - wave + 1) as f32 * rate } else { 0.0 }
        };
        [
            1.0,              // classic
            from(3, 0.20),    // soldier
            from(2, 0.25),    // female
            from(5, 0.20),    // cop
            from(4, 0.25),    // parasite
            from(2, 0.30),    // prisoner
            from(6, 0.15),    // romero
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
        wave.roll();
        wave.burst_timer = 0.0;
        info!("[wave] {} begins, {} zombies", wave.number, wave.pending.len());
        return;
    }

    let alive = zombies.iter().count() as u32;

    if wave.pending.is_empty() {
        // Spent and empty: the wave is done. Nothing is spawned during the
        // break, so the field stays clear for as long as the banner is up.
        if alive == 0 {
            wave.just_cleared = Some(wave.number);
            wave.break_timer = WAVE_BREAK_SECONDS;
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
    let take = (WAVE_BURST_SIZE.min(room) as usize).min(wave.pending.len());
    let health = wave.health();
    let number = wave.number;
    let mut sending = Vec::new();
    for (i, kind) in wave.pending.drain(..take).enumerate() {
        let phase = ((number * 17 + i as u32 * 7) as f32 * 0.6180339).fract();
        let point = points.0[(phase * points.0.len() as f32) as usize % points.0.len()];
        sending.push((point + Vec3::Y * shared::ZOMBIE_REST_HEIGHT, kind, health * kind.stats().health));
    }
    if !sending.is_empty() {
        commands.trigger(SpawnWaveZombies(sending));
    }
}

/// One burst, as an event, so the spawning itself stays in `sim` beside the
/// sandbox's and the two cannot drift apart.
#[derive(Event)]
pub(crate) struct SpawnWaveZombies(pub(crate) Vec<(Vec3, shared::ZombieKind, f32)>);


/// The banner that says a wave is done.
///
/// Built once at startup and hidden, rather than spawned on the clear. A
/// runtime-spawned root node reported a laid-out 3190x106 box, Visibility
/// Inherited and InheritedVisibility true, and still drew nothing -- even
/// with an opaque background and the game-over z-index. Every other piece of
/// UI in this game is built in the same startup pass; this one is now too,
/// and all a cleared wave does is set a timer.
#[derive(Component)]
pub(crate) struct WaveBanner {
    remaining: f32,
}

#[derive(Component)]
pub(crate) struct WaveBannerText;

const BANNER_SECONDS: f32 = 5.0;
const BANNER_FADE: f32 = 1.2;

pub(crate) fn spawn_wave_banner(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(26.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            // Above the HUD, not below it: every HUD root carries
            // STATIC_UI_Z_INDEX, so the 40 this started at put the banner
            // underneath all of them.
            GlobalZIndex(crate::STATIC_UI_Z_INDEX + 25),
            Visibility::Hidden,
            WaveBanner { remaining: 0.0 },
            crate::NativeUiRoot,
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(String::new()),
                TextFont { font_size: FontSize::Px(44.0), ..default() },
                TextColor(Color::srgb(0.92, 0.22, 0.14)),
                WaveBannerText,
            ));
        });
}

pub(crate) fn announce_wave(
    wave: Res<Wave>,
    screams: Option<Res<crate::audio::WaveScreams>>,
    mut commands: Commands,
    mut banner: Query<(&mut WaveBanner, &mut Visibility)>,
    mut text: Query<&mut Text, With<WaveBannerText>>,
) {
    let Some(cleared) = wave.just_cleared else { return };

    let Ok((mut banner, mut visibility)) = banner.single_mut() else {
        warn!("[wave] no banner to show -- spawn_wave_banner did not run");
        return;
    };
    banner.remaining = BANNER_SECONDS;
    *visibility = Visibility::Visible;
    if let Ok(mut text) = text.single_mut() {
        text.0 = format!("WAVE {cleared} CLEARED");
    }
    info!("[wave] {cleared} cleared -- banner up");

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
    time: Res<Time>,
    mut banner: Query<(&mut WaveBanner, &mut Visibility)>,
    mut colours: Query<&mut TextColor, With<WaveBannerText>>,
) {
    let Ok((mut banner, mut visibility)) = banner.single_mut() else { return };
    if banner.remaining <= 0.0 {
        return;
    }
    banner.remaining -= time.delta_secs();
    if banner.remaining <= 0.0 {
        banner.remaining = 0.0;
        *visibility = Visibility::Hidden;
        return;
    }
    let alpha = (banner.remaining / BANNER_FADE).clamp(0.0, 1.0);
    for mut colour in &mut colours {
        colour.0.set_alpha(alpha);
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

/// Puts the siege back to wave one when the player dies and restarts.
///
/// A restart already clears the field, moves the player back and refills
/// their health. What it did not do was touch the wave: dying on wave
/// fourteen and pressing R left the counter on fourteen, so the next wave
/// out of the gate was fifteen against a player who had just died.
///
/// The kit goes back too. Carrying a wave-fourteen arsenal into wave one is
/// the opposite of a penalty -- dying would have been the fastest way to an
/// easy fight with a full pack.
pub(crate) fn reset_waves_on_restart(
    trigger: On<shared::GameStateEvent>,
    mut wave: ResMut<Wave>,
    mut inventory: ResMut<crate::inventory::Inventory>,
    mut loadout: ResMut<crate::inventory::Loadout>,
    mut equipment: ResMut<crate::inventory::Equipment>,
    mut attachments: ResMut<crate::inventory::Attachments>,
    mut request: ResMut<crate::weapon::WeaponSwitchRequest>,
    mut banner: Query<(&mut WaveBanner, &mut Visibility)>,
) {
    if !matches!(trigger.event(), shared::GameStateEvent::Restarted) || !in_wave_mode() {
        return;
    }
    *wave = Wave::default();
    *inventory = crate::inventory::Inventory::default();
    *loadout = crate::inventory::Loadout::default();
    *equipment = crate::inventory::Equipment::default();
    *attachments = crate::inventory::Attachments::default();
    // The viewmodel only follows a request, not the loadout: without this the
    // player keeps holding whatever they died with while the pack says
    // otherwise.
    request.0 = loadout.get(loadout.active);
    if let Ok((mut banner, mut visibility)) = banner.single_mut() {
        banner.remaining = 0.0;
        *visibility = Visibility::Hidden;
    }
    info!("[wave] restart -- back to wave one");
}
