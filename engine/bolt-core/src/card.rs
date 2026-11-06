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
    CANT_BLOCK = 15, "Can't block";
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
    /// ... {1} less for each permanent you control with this subtype (affinity).
    PerYouControl(&'static str),
    /// ... {1} less for each instant and sorcery card in your graveyard.
    PerInstantSorceryInGraveyard,
    /// ... {X} less, X the total power of creatures you control.
    TotalPowerOfYourCreatures,
    /// ... {1} less for each attacking creature you control.
    PerAttackingCreature,
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
    /// Activated from the graveyard ("Return this card from your graveyard ...").
    pub from_graveyard: bool,
    /// Remove this many incubation counters from the source as a cost.
    pub remove_incubation: i32,
    /// "Activate only once."
    pub once: bool,
    /// "Tap N untapped [subtype] you control" (other than the source) as a cost.
    pub tap_untapped: Option<(&'static str, u8)>,
    /// Tap the creature this is attached to as part of the cost (Fishing Pole).
    pub tap_attached: bool,
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
    /// Doubling Season: tokens you'd create and counters you'd put on your permanents are doubled
    /// (CR 614.1a).
    pub doubles_tokens_and_counters: bool,
    /// Twinflame Tyrant: damage your sources would deal to an opponent or their permanents is
    /// doubled.
    pub doubles_damage_to_opponents: bool,
    /// Omniscience: you may cast spells from your hand without paying their mana costs.
    pub free_from_hand: bool,
    /// Protection from everything (CR 702.16j).
    pub protection_from_everything: bool,
    /// "If this would be put into a graveyard from anywhere, shuffle it into its owner's library
    /// instead."
    pub shuffle_instead_of_graveyard: bool,
    /// "Enters with N +1/+1 counters if ..." (a replacement effect, CR 614.1c).
    pub enters_counters: Option<(i32, Condition)>,
    /// Ward {cost} (CR 702.21).
    pub ward: Option<ManaCost>,
    /// "Attacks each combat if able" (CR 508.1d).
    pub must_attack: bool,
    /// "Can't be blocked by [subtype]".
    pub not_blocked_by: Option<&'static str>,
    /// "Creatures your opponents control enter tapped."
    pub opponents_creatures_enter_tapped: bool,
    /// "You can't lose the game and your opponents can't win the game."
    pub cant_lose: bool,
    /// "Ward—Pay N life."
    pub ward_life: Option<i32>,
    /// "This spell can't be countered."
    pub uncounterable: bool,
    /// "You have no maximum hand size."
    pub no_max_hand_size: bool,
    /// With kicker paid, this replaces `spell` (different targets).
    pub kicked_spell: Option<Ability>,
    /// "You may play an additional land on each of your turns."
    pub extra_land_drops: u8,
    /// Power and toughness each equal to the number of cards in opponents' graveyards.
    pub pt_opponents_graveyards: bool,
    /// "Hexproof from instants".
    pub hexproof_from_instants: bool,
    /// "You may pay {cost} rather than pay this spell's mana cost if there are N or more creatures
    /// on the battlefield."
    pub alt_cost: Option<(ManaCost, u8)>,
    /// "If this card is in your opening hand, you may begin the game with it on the battlefield."
    pub leyline: bool,
    /// "You may cast spells as though they had flash."
    pub grants_flash: bool,
    /// Colors, WUBRG (CR 105).
    pub colors: [bool; 5],
    /// "Enters with eight revival counters on it if you cast it."
    pub revival_on_cast: bool,
    /// "As this enters, choose a color", and "{T}: Add one mana of the chosen color".
    pub choose_color: bool,
    /// "As this enters, choose a creature type" (CR 614.12); also gives it a fellowship counter for
    /// each creature you control of that type (Banner of Kinship).
    pub choose_creature_type: bool,
    /// Vivid: one activation adds one mana of each color among permanents you control.
    pub vivid_mana: bool,
    /// Its mana can be spent only to cast spells with this subtype.
    pub mana_only_for_subtype: Option<&'static str>,
    /// "Each other [subtype] you control enters with an additional +1/+1 counter on it for each
    /// [subtype] you already control" (Giada).
    pub other_subtype_enter_bonus: Option<&'static str>,
    /// "You may cast this card from your graveyard by removing N counters from among creatures you
    /// control in addition to paying its other costs."
    pub graveyard_cast_remove_counters: u8,
    pub mana_of_chosen_color: bool,
    /// Its controller may play opponents' stashed cards from exile during their turn (Tinybones).
    pub stash_player: bool,
    /// "You have hexproof."
    pub player_hexproof: bool,
    /// "Prevent all noncombat damage that would be dealt to other creatures you control."
    pub prevent_noncombat_to_others: bool,
    /// "{T}: Add {C} for each [subtype] you control": the source makes this many of its mana.
    pub mana_per_subtype: Option<&'static str>,
    /// "This creature enters with X +1/+1 counters on it."
    pub enters_with_x_counters: bool,
    /// "+1/+1 for each creature card in your graveyard."
    pub pt_per_graveyard_creature: bool,
    /// Starting loyalty (CR 306.5b) and loyalty abilities as (cost, ability) (CR 606).
    pub loyalty: Option<i32>,
    pub loyalty_abilities: Vec<(i32, Ability)>,
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
            doubles_tokens_and_counters: false,
            doubles_damage_to_opponents: false,
            free_from_hand: false,
            protection_from_everything: false,
            shuffle_instead_of_graveyard: false,
            enters_counters: None,
            ward: None,
            must_attack: false,
            not_blocked_by: None,
            opponents_creatures_enter_tapped: false,
            cant_lose: false,
            ward_life: None,
            uncounterable: false,
            no_max_hand_size: false,
            kicked_spell: None,
            extra_land_drops: 0,
            pt_opponents_graveyards: false,
            hexproof_from_instants: false,
            alt_cost: None,
            leyline: false,
            grants_flash: false,
            colors: [false; 5],
            revival_on_cast: false,
            choose_color: false,
            choose_creature_type: false,
            vivid_mana: false,
            mana_only_for_subtype: None,
            other_subtype_enter_bonus: None,
            graveyard_cast_remove_counters: 0,
            mana_of_chosen_color: false,
            stash_player: false,
            player_hexproof: false,
            prevent_noncombat_to_others: false,
            mana_per_subtype: None,
            enters_with_x_counters: false,
            pt_per_graveyard_creature: false,
            loyalty: None,
            loyalty_abilities: Vec::new(),
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
            doubles_tokens_and_counters: false,
            doubles_damage_to_opponents: false,
            free_from_hand: false,
            protection_from_everything: false,
            shuffle_instead_of_graveyard: false,
            enters_counters: None,
            ward: None,
            must_attack: false,
            not_blocked_by: None,
            opponents_creatures_enter_tapped: false,
            cant_lose: false,
            ward_life: None,
            uncounterable: false,
            no_max_hand_size: false,
            kicked_spell: None,
            extra_land_drops: 0,
            pt_opponents_graveyards: false,
            hexproof_from_instants: false,
            alt_cost: None,
            leyline: false,
            grants_flash: false,
            colors: [false; 5],
            revival_on_cast: false,
            choose_color: false,
            choose_creature_type: false,
            vivid_mana: false,
            mana_only_for_subtype: None,
            other_subtype_enter_bonus: None,
            graveyard_cast_remove_counters: 0,
            mana_of_chosen_color: false,
            stash_player: false,
            player_hexproof: false,
            prevent_noncombat_to_others: false,
            mana_per_subtype: None,
            enters_with_x_counters: false,
            pt_per_graveyard_creature: false,
            loyalty: None,
            loyalty_abilities: Vec::new(),
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
            doubles_tokens_and_counters: false,
            doubles_damage_to_opponents: false,
            free_from_hand: false,
            protection_from_everything: false,
            shuffle_instead_of_graveyard: false,
            enters_counters: None,
            ward: None,
            must_attack: false,
            not_blocked_by: None,
            opponents_creatures_enter_tapped: false,
            cant_lose: false,
            ward_life: None,
            uncounterable: false,
            no_max_hand_size: false,
            kicked_spell: None,
            extra_land_drops: 0,
            pt_opponents_graveyards: false,
            hexproof_from_instants: false,
            alt_cost: None,
            leyline: false,
            grants_flash: false,
            colors: [false; 5],
            revival_on_cast: false,
            choose_color: false,
            choose_creature_type: false,
            vivid_mana: false,
            mana_only_for_subtype: None,
            other_subtype_enter_bonus: None,
            graveyard_cast_remove_counters: 0,
            mana_of_chosen_color: false,
            stash_player: false,
            player_hexproof: false,
            prevent_noncombat_to_others: false,
            mana_per_subtype: None,
            enters_with_x_counters: false,
            pt_per_graveyard_creature: false,
            loyalty: None,
            loyalty_abilities: Vec::new(),
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
