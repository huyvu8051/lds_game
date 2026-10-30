use bevy::prelude::*;

/// Every key the game reads, in one place.
///
/// Three times now a key has been bound twice, and every one of them was
/// found by a player doing one thing and getting two: the heal key was also
/// prone, so drinking a bandage lay you down; the hitbox toggle was also the
/// grenade; and the sight cycler was also the backpack, so every trip into
/// the bag fitted a sight on the way in.
///
/// The table below is the only place a key is named, and `report_clashes`
/// says so at startup if two of them are the same.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Bind {
    Forward,
    Back,
    StrafeLeft,
    StrafeRight,
    Sprint,
    Jump,
    LeanLeft,
    LeanRight,
    Crouch,
    Prone,

    Reload,
    Grenade,
    Inspect,

    Bag,
    Heal,
    Slot1,
    Slot2,
    Slot3,

    Restart,
    Settings,
    Help,
    ReleaseCursor,
    Quit,

    Noclip,
    GodMode,
    Autopilot,
    FreezeZombies,
    ShowHitboxes,
    HideHealthBars,
    StopZombieAnimation,
    AimBlur,
    FocalAlternate,
    CycleSight,
    TuningBack,
    TuningForward,
    TuningDown,
    TuningUp,
    TuningSave,
}

/// When a key is live. Two binds may share a key only if they are never
/// listening at the same time.
///
/// Debug keys count as playing: the player is playing when they press one,
/// which is the whole reason a debug toggle must not sit on a key the player
/// reaches for on purpose.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum When {
    Playing,
    GameOver,
}

pub(crate) struct BindDef {
    pub(crate) key: KeyCode,
    pub(crate) when: When,
    pub(crate) label: &'static str,
}

impl Bind {
    pub(crate) fn def(self) -> BindDef {
        use Bind::*;
        use When::*;
        let (key, when, label) = match self {
            Forward => (KeyCode::KeyW, Playing, "forward"),
            Back => (KeyCode::KeyS, Playing, "back"),
            StrafeLeft => (KeyCode::KeyA, Playing, "left"),
            StrafeRight => (KeyCode::KeyD, Playing, "right"),
            Sprint => (KeyCode::ShiftLeft, Playing, "sprint"),
            Jump => (KeyCode::Space, Playing, "jump"),
            LeanLeft => (KeyCode::KeyQ, Playing, "lean left"),
            LeanRight => (KeyCode::KeyE, Playing, "lean right"),
            Crouch => (KeyCode::KeyC, Playing, "crouch"),
            Prone => (KeyCode::KeyX, Playing, "prone"),

            Reload => (KeyCode::KeyR, Playing, "reload"),
            Grenade => (KeyCode::KeyG, Playing, "grenade"),
            Inspect => (KeyCode::KeyT, Playing, "inspect weapon"),

            Bag => (KeyCode::KeyB, Playing, "backpack"),
            Heal => (KeyCode::KeyH, Playing, "heal"),
            Slot1 => (KeyCode::Digit1, Playing, "slot one"),
            Slot2 => (KeyCode::Digit2, Playing, "slot two"),
            Slot3 => (KeyCode::Digit3, Playing, "slot three"),

            // The one key deliberately shared. Nothing reloads while the
            // round is over and nothing restarts while it is not.
            Restart => (KeyCode::KeyR, GameOver, "restart"),
            Settings => (KeyCode::Tab, Playing, "settings"),
            Help => (KeyCode::F1, Playing, "help"),
            ReleaseCursor => (KeyCode::Backquote, Playing, "release the cursor"),
            Quit => (KeyCode::Escape, Playing, "quit"),

            Noclip => (KeyCode::KeyN, Playing, "noclip"),
            GodMode => (KeyCode::KeyO, Playing, "god mode"),
            Autopilot => (KeyCode::KeyI, Playing, "autopilot"),
            FreezeZombies => (KeyCode::KeyF, Playing, "freeze the zombies"),
            ShowHitboxes => (KeyCode::KeyY, Playing, "show hitboxes"),
            HideHealthBars => (KeyCode::KeyJ, Playing, "hide health bars"),
            StopZombieAnimation => (KeyCode::KeyM, Playing, "stop zombie animation"),
            AimBlur => (KeyCode::KeyK, Playing, "aim blur"),
            FocalAlternate => (KeyCode::KeyV, Playing, "alternate focal plane"),
            CycleSight => (KeyCode::KeyU, Playing, "cycle the fitted sight"),
            TuningBack => (KeyCode::Comma, Playing, "tuning: back"),
            TuningForward => (KeyCode::Period, Playing, "tuning: forward"),
            TuningDown => (KeyCode::Minus, Playing, "tuning: down"),
            TuningUp => (KeyCode::Equal, Playing, "tuning: up"),
            TuningSave => (KeyCode::F9, Playing, "tuning: save"),
        };
        BindDef { key, when, label }
    }

    pub(crate) fn key(self) -> KeyCode {
        self.def().key
    }

    pub(crate) fn pressed(self, keys: &ButtonInput<KeyCode>) -> bool {
        keys.pressed(self.key())
    }

    pub(crate) fn just_pressed(self, keys: &ButtonInput<KeyCode>) -> bool {
        keys.just_pressed(self.key())
    }

    pub(crate) fn just_released(self, keys: &ButtonInput<KeyCode>) -> bool {
        keys.just_released(self.key())
    }

    pub(crate) const ALL: [Bind; 38] = [
        Bind::Forward,
        Bind::Back,
        Bind::StrafeLeft,
        Bind::StrafeRight,
        Bind::Sprint,
        Bind::Jump,
        Bind::LeanLeft,
        Bind::LeanRight,
        Bind::Crouch,
        Bind::Prone,
        Bind::Reload,
        Bind::Grenade,
        Bind::Inspect,
        Bind::Bag,
        Bind::Heal,
        Bind::Slot1,
        Bind::Slot2,
        Bind::Slot3,
        Bind::Restart,
        Bind::Settings,
        Bind::Help,
        Bind::ReleaseCursor,
        Bind::Quit,
        Bind::Noclip,
        Bind::GodMode,
        Bind::Autopilot,
        Bind::FreezeZombies,
        Bind::ShowHitboxes,
        Bind::HideHealthBars,
        Bind::StopZombieAnimation,
        Bind::AimBlur,
        Bind::FocalAlternate,
        Bind::CycleSight,
        Bind::TuningBack,
        Bind::TuningForward,
        Bind::TuningDown,
        Bind::TuningUp,
        Bind::TuningSave,
    ];
}

/// Says at startup if two binds that are both listening share a key.
///
/// Loudly rather than fatally: a clash makes one feature misbehave, and a
/// game that will not start is a worse answer to that than a line that says
/// which two to look at.
pub(crate) fn report_clashes() {
    for (i, a) in Bind::ALL.iter().enumerate() {
        let a = a.def();
        for b in &Bind::ALL[i + 1..] {
            let b = b.def();
            if a.key == b.key && a.when == b.when {
                error!("[keys] {:?} is bound to both {} and {}", a.key, a.label, b.label);
            }
        }
    }
}
