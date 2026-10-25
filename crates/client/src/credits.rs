//! Who has to be named, and what the game names them for.
//!
//! Three of the models carry CC-BY-4.0, which allows commercial use on one
//! condition: the author is credited. A game that ships them without this
//! screen is in breach of the licence it was given them under, so this is not
//! decoration -- it is the thing that makes shipping them legal.
//!
//! The wording comes from each asset's own license.txt. When an asset is
//! swapped out, its line here goes with it.

use bevy::prelude::*;
use bevy::text::FontSize;

pub(crate) struct Credit {
    pub(crate) work: &'static str,
    pub(crate) author: &'static str,
    pub(crate) licence: &'static str,
}

/// Required, not courteous. Everything here is used under a licence that asks
/// for the credit in return.
pub(crate) const REQUIRED: &[Credit] = &[
    // Desert_City on Sketchfab; the underscore has no glyph on the native
    // path and would be dropped without trace.
    Credit { work: "Desert City", author: "Mikey x pc", licence: "CC-BY-4.0" },
    Credit { work: "AK47 with scope", author: "Wolf Schevelenbos", licence: "CC-BY-4.0" },
    Credit { work: "Low Poly Colt M4A1", author: "puresaltt", licence: "CC-BY-4.0" },
    Credit { work: "footstep-concrete.wav", author: "swuing / Freesound", licence: "CC-BY-3.0" },
];

/// Asked for nothing, named anyway. The Sonniss bundle is royalty free with no
/// attribution required; the recordings are still somebody's work.
pub(crate) const COURTESY: &[Credit] = &[
    Credit { work: "Weapon recordings", author: "Pole Position Production", licence: "Sonniss GDC" },
    Credit { work: "m4a1 reload", author: "Coll Anderson", licence: "Sonniss GDC" },
];

pub(crate) fn spawn_credits(page: &mut ChildSpawnerCommands) {
    let heading = |page: &mut ChildSpawnerCommands, text: &str| {
        page.spawn((
            Text::new(text.to_string()),
            TextFont { font_size: FontSize::Px(16.0), ..default() },
            TextColor(Color::WHITE),
        ));
    };
    // Two lines each, because the panel clips at a fixed width and the
    // licence sits at the end -- on one line the longest credits lost exactly
    // the part that makes them a credit.
    //
    // Commas rather than dashes throughout: the native path draws from a 5x7
    // bitmap font and silently drops anything it has no glyph for.
    let line = |page: &mut ChildSpawnerCommands, credit: &Credit| {
        page.spawn((
            Text::new(credit.work.to_string()),
            TextFont { font_size: FontSize::Px(13.0), ..default() },
            TextColor(Color::srgb(0.78, 0.78, 0.80)),
        ));
        page.spawn((
            Text::new(format!("   {}, {}", credit.author, credit.licence)),
            TextFont { font_size: FontSize::Px(13.0), ..default() },
            TextColor(Color::srgb(0.60, 0.60, 0.64)),
        ));
    };

    heading(page, "Credits");
    for credit in REQUIRED {
        line(page, credit);
    }
    heading(page, "Audio");
    for credit in COURTESY {
        line(page, credit);
    }
}
