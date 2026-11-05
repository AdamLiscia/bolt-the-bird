//! Card definitions: the printed characteristics an engine needs (CR 200-209).

use crate::effect::{Ability, Condition, Static, Trigger};
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
    UNBLOCKABLE = 14, "Can't be blocked";
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

/// An additional cost to cast a spell (CR 601.2b, 601.2h).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtraCost {
    SacrificeCreature,
    /// Discard a card other than the spell.
    Discard,
    /// Sacrifice a creature, or pay this mana instead.
    SacrificeCreatureOrPay(ManaCost),
}

/// "This spell costs {N} less to cast if ..." (CR 601.2f).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reduction {
    /// ... you control a permanent with this subtype.
    IfYouControl(String, u8),
    /// ... it targets a tapped creature.
    IfTargetsTapped(u8),
}

/// An activated ability: "[cost]: [effect]" (CR 602).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Activated {
    pub mana: ManaCost,
    /// {T} in the cost.
    pub tap: bool,
    /// "Sacrifice this ..." in the cost.
    pub sacrifice_self: bool,
    /// "Sacrifice another creature" in the cost.
    pub sacrifice_other_creature: bool,
    /// "Activate only as a sorcery."
    pub sorcery: bool,
    pub ability: Ability,
}

/// The printed card. Shared by every copy; per-copy state lives on the game object.
#[derive(Clone, Debug, Default, PartialEq)]
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
    /// What an instant or sorcery does when it resolves.
    pub spell: Option<Ability>,
    /// A modal spell's modes ("Choose one —"); the mode is chosen as it's cast (CR 700.2).
    pub modes: Vec<Ability>,
    /// An Aura: cast with a target (its `spell` ability) and attached to it on entering (CR 303.4).
    pub aura: bool,
    /// Triggered abilities.
    pub triggers: Vec<Trigger>,
    /// Kicker cost (CR 702.33).
    pub kicker: Option<ManaCost>,
    /// A token, which ceases to exist outside the battlefield (CR 111.7, 704.5d).
    pub token: bool,
    pub activated: Vec<Activated>,
    /// A mana source that is sacrificed when used, like a Treasure.
    pub sac_for_mana: bool,
    pub extra_cost: Option<ExtraCost>,
    pub reduction: Option<Reduction>,
    /// Flashback cost (CR 702.34).
    pub flashback: Option<ManaCost>,
    pub statics: Vec<Static>,
    /// "This ... enters tapped."
    pub enters_tapped: bool,
    /// "This creature can't block."
    pub cant_block: bool,
    /// "This creature doesn't untap during your untap step."
    pub doesnt_untap: bool,
    /// "Instant and sorcery spells you cast cost {N} less to cast."
    pub spell_discount: u8,
    /// "Enters with N +1/+1 counters if ..." (a replacement effect, CR 614.1c).
    pub enters_counters: Option<(i32, Condition)>,
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
            spell: None,
            modes: Vec::new(),
            aura: false,
            triggers: Vec::new(),
            kicker: None,
            token: false,
            activated: Vec::new(),
            sac_for_mana: false,
            extra_cost: None,
            reduction: None,
            flashback: None,
            statics: Vec::new(),
            enters_tapped: false,
            cant_block: false,
            doesnt_untap: false,
            spell_discount: 0,
            enters_counters: None,
        }
    }

    /// An instant or sorcery, mostly for tests.
    pub fn spell(name: &str, cost: &str, instant: bool, ability: Ability) -> CardDef {
        CardDef {
            name: name.to_string(),
            cost: ManaCost::parse(cost).expect("valid cost"),
            types: Types {
                instant,
                sorcery: !instant,
                ..Types::default()
            },
            subtypes: Vec::new(),
            power: 0,
            toughness: 0,
            keywords: Keywords::NONE,
            mana: None,
            spell: Some(ability),
            modes: Vec::new(),
            aura: false,
            triggers: Vec::new(),
            kicker: None,
            token: false,
            activated: Vec::new(),
            sac_for_mana: false,
            extra_cost: None,
            reduction: None,
            flashback: None,
            statics: Vec::new(),
            enters_tapped: false,
            cant_block: false,
            doesnt_untap: false,
            spell_discount: 0,
            enters_counters: None,
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
            spell: None,
            modes: Vec::new(),
            aura: false,
            triggers: Vec::new(),
            kicker: None,
            token: false,
            activated: Vec::new(),
            sac_for_mana: false,
            extra_cost: None,
            reduction: None,
            flashback: None,
            statics: Vec::new(),
            enters_tapped: false,
            cant_block: false,
            doesnt_untap: false,
            spell_discount: 0,
            enters_counters: None,
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
