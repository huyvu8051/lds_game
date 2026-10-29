//! Which groups of the game are built at all, so the cost of each can be
//! measured one at a time.
//!
//! The phone's frame is paced by the display, not by this app: it sits at
//! ~117 fps with 2-3.6 ms a frame spent blocked in `nextDrawable`, and the GPU
//! never leaves its minimum performance state. Turning work off therefore does
//! not move the frame rate — it moves `update`, which `NATIVE_STATS=1` already
//! prints. That is the number this module exists to make vary.
//!
//! Each group starts `false` here and is turned on by one later commit, so the
//! diff that enables a group is one line and the last commit is provably the
//! whole game again. Nothing is deleted.
//!
//! `ABLATE` overrides the compiled-in defaults for a single run:
//!
//! ```text
//! ABLATE=all                 everything on, i.e. the shipping game
//! ABLATE=physics,ui          the floor plus those two
//! ABLATE=none                the floor alone
//! ```
//!
//! It is deliberately not part of `PersistedSettings`. The settings file is
//! rewritten on the first frame of the first run and then applied *after* the
//! resources that read environment variables, so anything that goes through it
//! stops responding to the environment from the second launch onward.

/// A group of work that can be left out of the build.
///
/// The floor — the map's meshes, the camera, the sun, the player and noclip —
/// has no variant: it is what remains when every group is off.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Group {
    /// Avian's colliders for the map. The plugin itself is always built: the
    /// system that drives noclip takes `MoveAndSlide`, an Avian system param,
    /// so without the plugin that system fails to validate and the player
    /// cannot move at all.
    Physics,
    /// `AnimationPlugin` and the systems that drive skinned meshes from it.
    Animation,
    /// Spawning zombies and keeping their number up.
    Zombies,
    /// big-brain scoring and landmass pathfinding.
    ZombieAi,
    /// `UiPlugin`, the settings menu and the HUD.
    Ui,
    /// bevy_seedling and, off iOS, Steam Audio.
    Audio,
    /// Looking, shooting, reloading, leaning — `lib.rs`'s `InGame` block.
    Core,
    /// Pickups, grenades, ragdolls, gore and the wiring that attaches them.
    Items,
    /// Muzzle flashes, tracers, blood, bullet holes, casings, screen effects.
    Vfx,
}

impl Group {
    /// The name `ABLATE` uses.
    const fn key(self) -> &'static str {
        match self {
            Group::Physics => "physics",
            Group::Animation => "animation",
            Group::Zombies => "zombies",
            Group::ZombieAi => "zombie_ai",
            Group::Ui => "ui",
            Group::Audio => "audio",
            Group::Core => "core",
            Group::Items => "items",
            Group::Vfx => "vfx",
        }
    }

    /// What this commit builds. One of these flips per commit, lowest first.
    const fn compiled_default(self) -> bool {
        match self {
            Group::Physics => true,
            Group::Animation => true,
            Group::Zombies => true,
            Group::ZombieAi => false,
            Group::Ui => false,
            Group::Audio => false,
            Group::Core => false,
            Group::Items => false,
            Group::Vfx => false,
        }
    }
}

/// Whether `group` is built this run.
pub(crate) fn enabled(group: Group) -> bool {
    let Ok(raw) = std::env::var("ABLATE") else {
        return group.compiled_default();
    };
    let raw = raw.trim();
    match raw {
        "all" => true,
        "" | "none" => false,
        _ => raw
            .split(',')
            .map(str::trim)
            .any(|name| name.eq_ignore_ascii_case(group.key())),
    }
}

/// `enabled` as a run condition, for the `Update` blocks in `lib.rs`.
///
/// Read once at insertion rather than per frame: the answer cannot change
/// within a run, and a closure that calls `std::env::var` every frame for every
/// group would be its own measurable cost in the number being measured.
pub(crate) fn group(group: Group) -> impl Fn() -> bool + Clone {
    let on = enabled(group);
    move || on
}

/// Printed once at startup so a log can never be mistaken for the wrong
/// configuration — the question these runs answer is which group costs what,
/// and a mislabelled run is worse than a missing one.
pub(crate) fn log_configuration() {
    const ALL: [Group; 9] = [
        Group::Physics,
        Group::Animation,
        Group::Zombies,
        Group::ZombieAi,
        Group::Ui,
        Group::Audio,
        Group::Core,
        Group::Items,
        Group::Vfx,
    ];
    let on: Vec<&str> = ALL.iter().copied().filter(|g| enabled(*g)).map(Group::key).collect();
    if on.is_empty() {
        println!("[ablate] floor only: map, camera, sun, player, noclip");
    } else {
        println!("[ablate] floor + {}", on.join(", "));
    }
}
