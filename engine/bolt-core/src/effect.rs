//! The effect language: what spells and abilities do, as data (CR 608, 609).
//!
//! Most cards are a list of target slots plus a list of effects that refer to those slots. Cards
//! that don't fit get hand-written behavior later; until then they are unsupported, never faked.

use crate::card::Keywords;

/// Whose permanent or which player a target or effect may refer to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Controller {
    #[default]
    Any,
    You,
    Opponent,
}

/// What a target slot accepts (CR 115). A permanent qualifies if it has one of the allowed types
/// and passes that type's restriction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Filter {
    pub players: bool,
    pub creatures: bool,
    pub planeswalkers: bool,
    pub artifacts: bool,
    pub enchantments: bool,
    /// Any nonland permanent.
    pub nonland_permanents: bool,
    /// Spells on the stack. With `spell_creature`, only creature (true) or noncreature (false) spells.
    pub spells: bool,
    pub spell_creature: Option<bool>,
    /// Cards in graveyards instead of permanents: `Some(true)` for creature cards only. The
    /// controller restriction then means the graveyard's owner.
    pub graveyard: Option<bool>,
    /// "other": not the source of the ability.
    pub other: bool,
    /// Cards with at most this mana value (graveyard targets).
    pub max_mana_value: Option<u32>,
    pub controller: Controller,
    /// Creatures only if they have flying ("creature with flying").
    pub creature_needs_flying: bool,
    /// Creatures only if their power is at least this.
    pub creature_min_power: Option<i32>,
    /// Creatures only if attacking or blocking.
    pub in_combat: bool,
}

impl Filter {
    /// "any target": a creature, player, planeswalker or battle (CR 115.4).
    pub const ANY: Filter = Filter {
        players: true,
        creatures: true,
        planeswalkers: true,
        artifacts: false,
        enchantments: false,
        nonland_permanents: false,
        spells: false,
        spell_creature: None,
        graveyard: None,
        other: false,
        max_mana_value: None,
        controller: Controller::Any,
        creature_needs_flying: false,
        creature_min_power: None,
        in_combat: false,
    };

    pub const CREATURE: Filter = Filter {
        players: false,
        planeswalkers: false,
        ..Filter::ANY
    };

    pub const PLAYER: Filter = Filter {
        creatures: false,
        planeswalkers: false,
        ..Filter::ANY
    };

    pub const fn controlled_by(self, c: Controller) -> Filter {
        Filter {
            controller: c,
            ..self
        }
    }
}

/// Who an effect applies to, relative to the ability's controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    You,
    EachOpponent,
    /// The player in target slot `n`.
    TargetPlayer(u8),
    /// The controller of the permanent in target slot `n`.
    ControllerOf(u8),
}

/// Where damage goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recipient {
    Target(u8),
    EachOpponent,
    /// Each creature, optionally only those without flying.
    EachCreature {
        without_flying: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// The source deals damage (CR 120). With kicker paid, `kicked` replaces `amount`.
    Damage {
        amount: i32,
        kicked: Option<i32>,
        to: Recipient,
    },
    /// Damage only if the targeted player has exactly `life` life (Hidetsugu's Second Rite).
    DamageIfLifeIs {
        slot: u8,
        life: i32,
        amount: i32,
    },
    Destroy(u8),
    Draw {
        who: Who,
        n: u8,
    },
    GainLife {
        who: Who,
        n: i32,
    },
    LoseLife {
        who: Who,
        n: i32,
    },
    /// +P/+T until end of turn (CR 611.2, layer 7c).
    Pump {
        slot: u8,
        power: i32,
        toughness: i32,
    },
    /// Grants keywords until end of turn (layer 6).
    Grant {
        slot: u8,
        keywords: Keywords,
    },
    /// The source gets +P/+T until end of turn.
    PumpSelf {
        power: i32,
        toughness: i32,
    },
    /// +1/+1 counters (CR 122.1a).
    Counters {
        slot: u8,
        n: i32,
    },
    /// Create `n` tokens of definition `def` for `who` (CR 111).
    CreateToken {
        who: Who,
        def: usize,
        n: u8,
    },
    /// One token for each other creature you control with the source's name (Hare Apparent).
    CreateTokenPerNamesake {
        def: usize,
    },
    /// `who` discards `n` cards of their choice (CR 701.9).
    Discard {
        who: Who,
        n: u8,
    },
    /// "You may discard a card. If you do, draw a card."
    MayDiscardToDraw,
    /// The ability's controller chooses a nonland card from the hand of the player in `slot`, who
    /// discards it.
    ChooseDiscard {
        slot: u8,
    },
    /// Look at the top `n` cards; put any number into the graveyard, the rest back on top (CR 701.25).
    Surveil {
        n: u8,
    },
    /// Search your library, then shuffle (CR 701.19). With kicker paid, `kicked_n` replaces `n`.
    Search {
        what: SearchFor,
        to: SearchTo,
        n: u8,
        kicked_n: Option<u8>,
    },
    /// Counter the spell in `slot` (CR 701.5).
    Counter(u8),
    /// Put `n` +1/+1 counters on the source.
    CountersSelf {
        n: i32,
    },
    /// Creatures you control get +P/+T until end of turn.
    PumpAll {
        power: i32,
        toughness: i32,
    },
    /// The source gains keywords until end of turn.
    GrantSelf {
        keywords: Keywords,
    },
    /// Return the card in `slot` from a graveyard to the battlefield under your control.
    Reanimate(u8),
    /// Exile the permanent or card in `slot`.
    Exile(u8),
    /// Exile it until the source leaves the battlefield (CR 610.3).
    ExileUntilSourceLeaves(u8),
    /// If a creature card was among the cards exiled from the target slots, each opponent loses
    /// `n` life and you gain `n` life.
    DrainIfCreatureExiled {
        n: i32,
    },
    Tap(u8),
    /// "When you do": trigger number `n` of the source triggers now (CR 603.12).
    Reflexive(usize),
    /// Until end of turn, the creature in `slot` gains the source's trigger number `trigger` as a
    /// dies trigger, controlled by this ability's controller.
    GrantDiesTrigger {
        slot: u8,
        trigger: usize,
    },
    /// Return the card in `slot` from a graveyard to the battlefield tapped, under its owner's control.
    ReturnTapped(u8),
    /// Return the permanent in `slot` to its owner's hand.
    Bounce(u8),
    /// The source creature fights the creature in `slot` (CR 701.12).
    Fight(u8),
    /// The owner of the creature in `slot` puts it on the top or bottom of their library.
    Tuck(u8),
    /// The creature in slot `from` deals damage equal to its power to the one in slot `to`.
    Bite {
        from: u8,
        to: u8,
    },
    /// Exile a player's graveyard.
    ExileGraveyard {
        who: Who,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchFor {
    BasicLand,
    Any,
    /// An instant or sorcery card with this mana value.
    InstantOrSorceryWithManaValue(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchTo {
    Hand,
    Battlefield {
        tapped: bool,
    },
    /// Shuffle, then put the card on top.
    Top,
}

/// What a static ability affects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    This,
    YourCreatures,
    /// Creatures you control with a +1/+1 counter on them.
    YourCreaturesWithCounters,
}

/// A static ability that changes characteristics (CR 604, 613 layers 6 and 7c).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Static {
    pub scope: Scope,
    /// "During your turn".
    pub your_turn_only: bool,
    pub power: i32,
    pub toughness: i32,
    pub keywords: Keywords,
}

/// An intervening "if" clause, checked when the ability triggers and again on resolution
/// (CR 603.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Condition {
    /// Raid: you attacked this turn.
    Attacked,
    /// You control another creature with this subtype.
    ControlAnother(String),
    /// You control a creature with at least this power.
    ControlPowerAtLeast(i32),
    /// The source was kicked.
    Kicked,
}

/// A spell's effect or a triggered ability: target slots, then effects in order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ability {
    pub targets: Vec<Filter>,
    pub effects: Vec<Effect>,
    pub condition: Option<Condition>,
    /// "You may ..." for the whole ability: the controller chooses on resolution.
    pub optional: bool,
    /// "Up to N targets": only this many slots must be filled (CR 115.1, 601.2c).
    pub min_targets: Option<usize>,
    /// All graveyard targets must come from a single graveyard.
    pub same_graveyard: bool,
}

impl Ability {
    pub fn new(targets: Vec<Filter>, effects: Vec<Effect>) -> Ability {
        Ability {
            targets,
            effects,
            condition: None,
            optional: false,
            min_targets: None,
            same_graveyard: false,
        }
    }

    pub fn optional(mut self) -> Ability {
        self.optional = true;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    /// "When this creature enters" (CR 603.6a).
    Enters,
    /// "When this creature dies" (CR 603.6c, 603.10a).
    Dies,
    /// "Whenever you cast a noncreature spell" (prowess, CR 702.108).
    YouCastNoncreature,
    /// Only by an effect: a reflexive trigger or a granted ability.
    Never,
    /// "Whenever another creature you control enters".
    AnotherCreatureYouControlEnters,
    /// "Whenever this creature attacks" (CR 508.3a).
    Attacks,
    /// "Whenever you gain life for the first time each turn" (or "during each of your turns").
    YouGainLifeFirstTime { your_turn_only: bool },
    /// "Whenever you draw your second card each turn".
    YouDrawSecond,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trigger {
    pub when: When,
    pub ability: Ability,
}
