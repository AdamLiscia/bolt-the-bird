//! Card definitions: the printed characteristics an engine needs (CR 200-209).

use crate::mana::{ManaCost, ManaSource};

/// Keyword abilities as a bit set (CR 702). Only the evergreen combat keywords exist so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Keywords(u32);

macro_rules! keywords {
    ($($name:ident = $bit:expr, $text:expr;)*) => {
        impl Keywords {
            $(pub const $name: Keywords = Keywords(1 << $bit);)*

            /// Every keyword the engine implements, with its rules-text name.
            pub const ALL: &'static [(Keywords, &'static str)] = &[$((Keywords::$name, $text)),*];
        }
    };
}

keywords! {
    FLYING = 0, "Flying";
    REACH = 1, "Reach";
    FIRST_STRIKE = 2, "First strike";
    DOUBLE_STRIKE = 3, "Double strike";
    DEATHTOUCH = 4, "Deathtouch";
    LIFELINK = 5, "Lifelink";
    TRAMPLE = 6, "Trample";
    VIGILANCE = 7, "Vigilance";
    HASTE = 8, "Haste";
    MENACE = 9, "Menace";
    DEFENDER = 10, "Defender";
    INDESTRUCTIBLE = 11, "Indestructible";
    HEXPROOF = 12, "Hexproof";
    FLASH = 13, "Flash";
}

impl Keywords {
    pub const NONE: Keywords = Keywords(0);

    pub fn has(self, k: Keywords) -> bool {
        self.0 & k.0 == k.0
    }

    pub fn with(self, k: Keywords) -> Keywords {
        Keywords(self.0 | k.0)
    }

    /// Looks up a keyword by its rules-text name, ignoring case.
    pub fn named(name: &str) -> Option<Keywords> {
        Keywords::ALL
            .iter()
            .find(|(_, n)| n.eq_ignore_ascii_case(name))
            .map(|&(k, _)| k)
    }
}

/// Card types and supertypes (CR 205).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Types {
    pub land: bool,
    pub creature: bool,
    pub artifact: bool,
    pub enchantment: bool,
    pub instant: bool,
    pub sorcery: bool,
    pub planeswalker: bool,
    pub legendary: bool,
    pub basic: bool,
}

impl Types {
    /// Parses the part of a type line before the em dash, e.g. `Legendary Creature`.
    pub fn parse(type_line: &str) -> Types {
        let main = type_line.split('—').next().unwrap_or("");
        let mut t = Types::default();
        for word in main.split_whitespace() {
            match word {
                "Land" => t.land = true,
                "Creature" => t.creature = true,
                "Artifact" => t.artifact = true,
                "Enchantment" => t.enchantment = true,
                "Instant" => t.instant = true,
                "Sorcery" => t.sorcery = true,
                "Planeswalker" => t.planeswalker = true,
                "Legendary" => t.legendary = true,
                "Basic" => t.basic = true,
                _ => {}
            }
        }
        t
    }

    /// A permanent card enters the battlefield when it resolves (CR 110.4).
    pub fn is_permanent(&self) -> bool {
        !(self.instant || self.sorcery)
    }
}

/// The printed card. Shared by every copy; per-copy state lives on the game object.
#[derive(Clone, Debug, PartialEq)]
pub struct CardDef {
    pub name: String,
    pub cost: ManaCost,
    pub types: Types,
    pub subtypes: Vec<String>,
    pub power: i32,
    pub toughness: i32,
    pub keywords: Keywords,
    /// What the permanent taps for, if it is a mana source (lands, for now).
    pub mana: Option<ManaSource>,
}

impl CardDef {
    /// A vanilla creature, mostly for tests.
    pub fn creature(name: &str, cost: &str, power: i32, toughness: i32, keywords: Keywords) -> CardDef {
        CardDef {
            name: name.to_string(),
            cost: ManaCost::parse(cost).expect("valid cost"),
            types: Types {
                creature: true,
                ..Types::default()
            },
            subtypes: Vec::new(),
            power,
            toughness,
            keywords,
            mana: None,
        }
    }

    /// A basic land that taps for one color.
    pub fn basic_land(name: &str, color: crate::Color) -> CardDef {
        CardDef {
            name: name.to_string(),
            cost: ManaCost::default(),
            types: Types {
                land: true,
                basic: true,
                ..Types::default()
            },
            subtypes: vec![name.to_string()],
            power: 0,
            toughness: 0,
            keywords: Keywords::NONE,
            mana: Some(ManaSource::of(&[color])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_set_ops() {
        let k = Keywords::FLYING.with(Keywords::LIFELINK);
        assert!(k.has(Keywords::FLYING) && k.has(Keywords::LIFELINK));
        assert!(!k.has(Keywords::TRAMPLE));
        assert_eq!(Keywords::named("first strike"), Some(Keywords::FIRST_STRIKE));
        assert_eq!(Keywords::named("Ward"), None);
    }

    #[test]
    fn parses_type_lines() {
        let t = Types::parse("Legendary Creature — Human Soldier");
        assert!(t.legendary && t.creature && !t.land && t.is_permanent());
        let b = Types::parse("Basic Land — Forest");
        assert!(b.basic && b.land);
        assert!(!Types::parse("Instant").is_permanent());
    }
}
