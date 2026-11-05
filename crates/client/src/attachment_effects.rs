use bevy::prelude::*;

/// What the things bolted to a weapon are worth, gathered into one set of
/// multipliers so that every number they touch is touched in one place.
///
/// The figures are PUBG's where PUBG publishes one, because that is what the
/// parts were modelled from and a player who knows the game should find them
/// where they expect. What they multiply is this game's own numbers, which
/// are not PUBG's, so the percentages carry over and the absolutes do not.
///
/// Three of them sit at one and nothing moves them: `ads_speed`,
/// `settle_speed` and `reload` were the stock's, and stocks are gone. They
/// stay because the magazines are the obvious next slot and a quickdraw
/// magazine is a `reload` multiplier and nothing else.
#[derive(Resource)]
pub(crate) struct AttachmentEffects {
    /// Scales the climb, the shove and the weapon's own kick, per shot.
    pub(crate) recoil: f32,
    /// Scales how far the weapon lags behind a turn.
    pub(crate) sway: f32,
    /// Scales the cone a shot is allowed to leave in.
    pub(crate) spread: f32,
    /// Scales how fast the sights come up.
    pub(crate) ads_speed: f32,
    /// Scales how fast the weapon settles after a shot.
    pub(crate) settle_speed: f32,
    /// Scales how long a magazine change takes, so smaller is quicker.
    pub(crate) reload: f32,
    /// Scales the flash at the muzzle. Zero puts it out entirely.
    pub(crate) flash: f32,
    /// Scales how loud the shot is.
    pub(crate) report: f32,
    /// Whether the shot reaches for the suppressed recording instead of the
    /// weapon.s own. Its own flag rather than a quiet report: what decides
    /// is a can on the muzzle, and some other part could be quiet without
    /// changing what the shot is.
    pub(crate) suppressed: bool,
}

impl Default for AttachmentEffects {
    fn default() -> Self {
        Self {
            recoil: 1.0,
            sway: 1.0,
            spread: 1.0,
            ads_speed: 1.0,
            settle_speed: 1.0,
            reload: 1.0,
            flash: 1.0,
            report: 1.0,
            suppressed: false,
        }
    }
}

impl AttachmentEffects {
    fn apply(&mut self, other: &Self) {
        self.recoil *= other.recoil;
        self.sway *= other.sway;
        self.spread *= other.spread;
        self.ads_speed *= other.ads_speed;
        self.settle_speed *= other.settle_speed;
        self.reload *= other.reload;
        self.flash *= other.flash;
        self.report *= other.report;
        self.suppressed |= other.suppressed;
    }
}

/// A suppressor buys silence and darkness and nothing else, which is what it
/// buys in PUBG too.
///
/// And the silence buys no stealth here: the zombies in this game score a
/// target on distance alone and have no ears to fool. What it changes is
/// what the player hears and what the room sees, which on a night map is
/// most of what a muzzle device is for.
static SUPPRESSOR: AttachmentEffects =
    AttachmentEffects { flash: 0.0, report: 0.70, suppressed: true, ..CLEAN };

/// A brake does not make the blast go away, it points it sideways: less of
/// the shot comes back into the shoulder and more of it goes out as noise
/// and light. Both halves are here.
static COMPENSATOR: AttachmentEffects =
    AttachmentEffects { recoil: 0.85, sway: 0.90, flash: 1.15, report: 1.10, ..CLEAN };

/// A hider hides. The tenth off the cone is PUBG's, and the little it takes
/// off the kick is the prongs having to be bolted to something.
static FLASH_HIDER: AttachmentEffects =
    AttachmentEffects { flash: 0.22, spread: 0.90, recoil: 0.95, ..CLEAN };

/// Nothing fitted. Spelled out because a struct update needs a base and
/// `Default::default()` is not a const.
const CLEAN: AttachmentEffects = AttachmentEffects {
    recoil: 1.0,
    sway: 1.0,
    spread: 1.0,
    ads_speed: 1.0,
    settle_speed: 1.0,
    reload: 1.0,
    flash: 1.0,
    report: 1.0,
    suppressed: false,
};

/// Rebuilds the multipliers from whatever is actually on the weapon.
///
/// Off the fitted part rather than off the bag: a suppressor in a pocket
/// quiets nothing. `FittedMuzzle` has already decided that, and reading it
/// is how this cannot disagree.
pub(crate) fn gather_attachment_effects(
    muzzle: Res<crate::muzzle_mount::FittedMuzzle>,
    mut effects: ResMut<AttachmentEffects>,
) {
    if !muzzle.is_changed() {
        return;
    }
    use crate::muzzle_mount::MuzzleId;
    let mut out = AttachmentEffects::default();
    if let Some(muzzle) = muzzle.0 {
        out.apply(match muzzle {
            MuzzleId::SuppressorSmg | MuzzleId::SuppressorAr | MuzzleId::SuppressorSr => {
                &SUPPRESSOR
            }
            MuzzleId::Compensator => &COMPENSATOR,
            MuzzleId::FlashHider => &FLASH_HIDER,
        });
    }
    debug!(
        "[attach] recoil {:.2} sway {:.2} spread {:.2} ads {:.2} settle {:.2} reload {:.2} flash {:.2} report {:.2} suppressed {}",
        out.recoil, out.sway, out.spread, out.ads_speed, out.settle_speed, out.reload, out.flash, out.report, out.suppressed,
    );
    *effects = out;
}
