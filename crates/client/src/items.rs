//! Everything that can be in a pack, taken from PUBG's own list.
//!
//! Text and a coloured cube for now. None of these have a mesh, and several
//! have no mechanical effect yet either -- an attachment with nowhere to
//! attach is still worth existing, because the slot it goes in is what makes
//! the pack a decision rather than a list.
//!
//! Capacities are PUBG's, in its own units, so a backpack level means what it
//! means there: 150, 200, 250, against ammo at a half to one and a quarter
//! apiece and a frag grenade at eighteen.

use bevy::prelude::*;

/// Which shelf of the pack a thing sits on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Category {
    Ammo,
    Attachment,
    Armour,
    Healing,
    Boost,
    Throwable,
}

impl Category {
    pub(crate) const ALL: [Category; 6] = [
        Category::Ammo,
        Category::Attachment,
        Category::Armour,
        Category::Healing,
        Category::Boost,
        Category::Throwable,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Category::Ammo => "AMMO",
            Category::Attachment => "ATTACHMENTS",
            Category::Armour => "ARMOUR",
            Category::Healing => "HEALING",
            Category::Boost => "BOOST",
            Category::Throwable => "THROWABLES",
        }
    }

    pub(crate) fn colour(self) -> Color {
        match self {
            Category::Ammo => Color::srgb(0.80, 0.72, 0.45),
            Category::Attachment => Color::srgb(0.55, 0.78, 0.95),
            Category::Armour => Color::srgb(0.60, 0.95, 0.70),
            Category::Healing => Color::srgb(0.95, 0.35, 0.35),
            Category::Boost => Color::srgb(1.0, 0.80, 0.20),
            Category::Throwable => Color::srgb(0.95, 0.55, 0.25),
        }
    }
}

/// Which of a weapon's five sockets an attachment goes in. PUBG's split.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AttachSlot {
    Muzzle,
    Grip,
    Magazine,
    Stock,
    Sight,
}

impl AttachSlot {
    pub(crate) const ALL: [AttachSlot; 5] = [
        AttachSlot::Muzzle,
        AttachSlot::Grip,
        AttachSlot::Magazine,
        AttachSlot::Stock,
        AttachSlot::Sight,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            AttachSlot::Muzzle => "MUZZLE",
            AttachSlot::Grip => "GRIP",
            AttachSlot::Magazine => "MAG",
            AttachSlot::Stock => "STOCK",
            AttachSlot::Sight => "SIGHT",
        }
    }
}

/// Where a piece of armour is worn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum WearSlot {
    Helmet,
    Vest,
    Backpack,
}

impl WearSlot {
    pub(crate) const ALL: [WearSlot; 3] = [WearSlot::Helmet, WearSlot::Vest, WearSlot::Backpack];

    pub(crate) fn label(self) -> &'static str {
        match self {
            WearSlot::Helmet => "HELMET",
            WearSlot::Vest => "VEST",
            WearSlot::Backpack => "PACK",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum ItemKind {
    // Ammunition, one per calibre the game's weapons take.
    Ammo556,
    Ammo762,
    Ammo9mm,
    Ammo45,
    Ammo12g,

    // Muzzle.
    Suppressor,
    FlashHider,
    Compensator,
    BarrelExtender,
    Choke,
    Duckbill,

    // Grip.
    VerticalForegrip,
    AngledForegrip,
    HalfGrip,
    LightGrip,
    ThumbGrip,
    LaserSight,

    // Magazine.
    QuickdrawMag,
    ExtendedMag,
    ExtendedQuickdrawMag,

    // Stock.
    TacticalStock,
    CheekPad,
    BulletLoops,

    // Sights.
    RedDot,
    Holographic,
    Scope2x,
    Scope3x,
    Scope4x,
    Scope6x,
    Scope8x,
    Scope15x,
    CantedSight,

    // Armour, three levels apiece.
    Helmet1,
    Helmet2,
    Helmet3,
    Vest1,
    Vest2,
    Vest3,
    Backpack1,
    Backpack2,
    Backpack3,

    // Healing and boost.
    Bandage,
    FirstAidKit,
    MedKit,
    EnergyDrink,
    Painkiller,
    AdrenalineSyringe,

    // Throwables.
    FragGrenade,
    SmokeGrenade,
    StunGrenade,
    Molotov,
}

/// Everything, in the order the pack lists it.
pub(crate) const ALL_ITEMS: [ItemKind; 51] = [
    ItemKind::Ammo556,
    ItemKind::Ammo762,
    ItemKind::Ammo9mm,
    ItemKind::Ammo45,
    ItemKind::Ammo12g,
    ItemKind::Suppressor,
    ItemKind::FlashHider,
    ItemKind::Compensator,
    ItemKind::BarrelExtender,
    ItemKind::Choke,
    ItemKind::Duckbill,
    ItemKind::VerticalForegrip,
    ItemKind::AngledForegrip,
    ItemKind::HalfGrip,
    ItemKind::LightGrip,
    ItemKind::ThumbGrip,
    ItemKind::LaserSight,
    ItemKind::QuickdrawMag,
    ItemKind::ExtendedMag,
    ItemKind::ExtendedQuickdrawMag,
    ItemKind::TacticalStock,
    ItemKind::CheekPad,
    ItemKind::BulletLoops,
    ItemKind::RedDot,
    ItemKind::Holographic,
    ItemKind::Scope2x,
    ItemKind::Scope3x,
    ItemKind::Scope4x,
    ItemKind::Scope6x,
    ItemKind::Scope8x,
    ItemKind::Scope15x,
    ItemKind::CantedSight,
    ItemKind::Helmet1,
    ItemKind::Helmet2,
    ItemKind::Helmet3,
    ItemKind::Vest1,
    ItemKind::Vest2,
    ItemKind::Vest3,
    ItemKind::Backpack1,
    ItemKind::Backpack2,
    ItemKind::Backpack3,
    ItemKind::Bandage,
    ItemKind::FirstAidKit,
    ItemKind::MedKit,
    ItemKind::EnergyDrink,
    ItemKind::Painkiller,
    ItemKind::AdrenalineSyringe,
    ItemKind::FragGrenade,
    ItemKind::SmokeGrenade,
    ItemKind::StunGrenade,
    ItemKind::Molotov,
];

impl ItemKind {
    pub(crate) fn index(self) -> usize {
        ALL_ITEMS.iter().position(|k| *k == self).unwrap_or(0)
    }

    pub(crate) fn label(self) -> &'static str {
        use ItemKind::*;
        match self {
            Ammo556 => "5.56mm",
            Ammo762 => "7.62mm",
            Ammo9mm => "9mm",
            Ammo45 => ".45 ACP",
            Ammo12g => "12 Gauge",
            Suppressor => "Suppressor",
            FlashHider => "Flash Hider",
            Compensator => "Compensator",
            BarrelExtender => "Barrel Extender",
            Choke => "Choke",
            Duckbill => "Duckbill",
            VerticalForegrip => "Vertical Foregrip",
            AngledForegrip => "Angled Foregrip",
            HalfGrip => "Half Grip",
            LightGrip => "Light Grip",
            ThumbGrip => "Thumb Grip",
            LaserSight => "Laser Sight",
            QuickdrawMag => "Quickdraw Mag",
            ExtendedMag => "Extended Mag",
            ExtendedQuickdrawMag => "Ext. Quickdraw Mag",
            TacticalStock => "Tactical Stock",
            CheekPad => "Cheek Pad",
            BulletLoops => "Bullet Loops",
            RedDot => "Red Dot Sight",
            Holographic => "Holographic Sight",
            Scope2x => "2x Scope",
            Scope3x => "3x Scope",
            Scope4x => "4x Scope",
            Scope6x => "6x Scope",
            Scope8x => "8x Scope",
            Scope15x => "15x Scope",
            CantedSight => "Canted Sight",
            Helmet1 => "Helmet Lv1",
            Helmet2 => "Helmet Lv2",
            Helmet3 => "Helmet Lv3",
            Vest1 => "Vest Lv1",
            Vest2 => "Vest Lv2",
            Vest3 => "Vest Lv3",
            Backpack1 => "Backpack Lv1",
            Backpack2 => "Backpack Lv2",
            Backpack3 => "Backpack Lv3",
            Bandage => "Bandage",
            FirstAidKit => "First Aid Kit",
            MedKit => "Med Kit",
            EnergyDrink => "Energy Drink",
            Painkiller => "Painkiller",
            AdrenalineSyringe => "Adrenaline Syringe",
            FragGrenade => "Frag Grenade",
            SmokeGrenade => "Smoke Grenade",
            StunGrenade => "Stun Grenade",
            Molotov => "Molotov Cocktail",
        }
    }

    pub(crate) fn category(self) -> Category {
        use ItemKind::*;
        match self {
            Ammo556 | Ammo762 | Ammo9mm | Ammo45 | Ammo12g => Category::Ammo,
            Helmet1 | Helmet2 | Helmet3 | Vest1 | Vest2 | Vest3 | Backpack1 | Backpack2
            | Backpack3 => Category::Armour,
            Bandage | FirstAidKit | MedKit => Category::Healing,
            EnergyDrink | Painkiller | AdrenalineSyringe => Category::Boost,
            FragGrenade | SmokeGrenade | StunGrenade | Molotov => Category::Throwable,
            _ => Category::Attachment,
        }
    }

    /// Which socket it goes in, for the ones that go in a socket.
    pub(crate) fn attach_slot(self) -> Option<AttachSlot> {
        use ItemKind::*;
        Some(match self {
            Suppressor | FlashHider | Compensator | BarrelExtender | Choke | Duckbill => {
                AttachSlot::Muzzle
            }
            VerticalForegrip | AngledForegrip | HalfGrip | LightGrip | ThumbGrip | LaserSight => {
                AttachSlot::Grip
            }
            QuickdrawMag | ExtendedMag | ExtendedQuickdrawMag => AttachSlot::Magazine,
            TacticalStock | CheekPad | BulletLoops => AttachSlot::Stock,
            RedDot | Holographic | Scope2x | Scope3x | Scope4x | Scope6x | Scope8x | Scope15x
            | CantedSight => AttachSlot::Sight,
            _ => return None,
        })
    }

    /// Where it is worn, and at what level.
    pub(crate) fn wear_slot(self) -> Option<(WearSlot, u8)> {
        use ItemKind::*;
        Some(match self {
            Helmet1 => (WearSlot::Helmet, 1),
            Helmet2 => (WearSlot::Helmet, 2),
            Helmet3 => (WearSlot::Helmet, 3),
            Vest1 => (WearSlot::Vest, 1),
            Vest2 => (WearSlot::Vest, 2),
            Vest3 => (WearSlot::Vest, 3),
            Backpack1 => (WearSlot::Backpack, 1),
            Backpack2 => (WearSlot::Backpack, 2),
            Backpack3 => (WearSlot::Backpack, 3),
            _ => return None,
        })
    }

    /// Space it takes in the pack, in PUBG's own units: ammo from 0.375 to
    /// 1.25 a round, a frag grenade eighteen, a med kit twenty.
    pub(crate) fn capacity(self) -> f32 {
        use ItemKind::*;
        match self {
            Ammo556 => 0.5,
            Ammo762 => 0.7,
            Ammo9mm => 0.375,
            Ammo45 => 0.4,
            Ammo12g => 1.25,
            Bandage => 3.0,
            FirstAidKit => 8.0,
            MedKit => 20.0,
            EnergyDrink | Painkiller => 4.0,
            AdrenalineSyringe => 8.0,
            FragGrenade => 18.0,
            Molotov => 16.0,
            SmokeGrenade => 14.0,
            StunGrenade => 12.0,
            Helmet1 | Vest1 => 30.0,
            Helmet2 | Vest2 => 35.0,
            Helmet3 | Vest3 => 40.0,
            Backpack1 | Backpack2 | Backpack3 => 0.0,
            Scope8x | Scope15x => 12.0,
            Scope4x | Scope6x => 10.0,
            _ => 6.0,
        }
    }

    /// Its own shade of the category's colour, so two things on the ground
    /// can be told apart before either is read.
    pub(crate) fn colour(self) -> Color {
        let base = self.category().colour().to_srgba();
        // Spread through the category by index, so neighbours differ.
        let step = (self.index() % 5) as f32 * 0.09 - 0.18;
        Color::srgb(
            (base.red + step).clamp(0.08, 1.0),
            (base.green + step * 0.6).clamp(0.08, 1.0),
            (base.blue - step * 0.4).clamp(0.08, 1.0),
        )
    }
}

/// What a backpack level is worth, and what is carried with none at all.
pub(crate) fn backpack_capacity(level: u8) -> f32 {
    match level {
        1 => 150.0,
        2 => 200.0,
        3 => 250.0,
        _ => 100.0,
    }
}

/// Damage a helmet or vest turns away, by level. PUBG's figures.
pub(crate) fn armour_reduction(level: u8) -> f32 {
    match level {
        1 => 0.30,
        2 => 0.40,
        3 => 0.55,
        _ => 0.0,
    }
}
