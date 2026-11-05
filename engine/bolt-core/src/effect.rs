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
    /// Permanent cards only, lands included (graveyard targets).
    pub permanent_cards: bool,
    /// Not a target: the object that caused the trigger ("that creature"). Filled in when the
    /// ability triggers; hexproof doesn't apply.
    pub triggering: bool,
    /// Only cards or permanents with this subtype, or without it.
    pub subtype: Option<&'static str>,
    pub not_subtype: Option<&'static str>,
    /// Cards with mana value at most the source's power (last known if it left).
    pub mana_value_at_most_source_power: bool,
    /// Any permanent, lands included.
    pub any_permanent: bool,
    /// Graveyard cards put there this turn.
    pub put_there_this_turn: bool,
    /// Instant and sorcery cards only (graveyard targets).
    pub instant_or_sorcery: bool,
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
        permanent_cards: false,
        triggering: false,
        subtype: None,
        not_subtype: None,
        mana_value_at_most_source_power: false,
        any_permanent: false,
        put_there_this_turn: false,
        instant_or_sorcery: false,
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
    EachPlayer,
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
    /// Creatures `who` controls get +P/+T until end of turn.
    PumpAll {
        who: Who,
        power: i32,
        toughness: i32,
    },
    /// Creatures `who` controls gain keywords until end of turn.
    GrantAll {
        who: Who,
        keywords: Keywords,
    },
    /// Put the top `n` cards of `who`'s library into their graveyard (CR 701.13).
    Mill {
        who: Who,
        n: u8,
    },
    /// The creature the source is attached to gets +P/+T until end of turn.
    PumpAttached {
        power: i32,
        toughness: i32,
    },
    /// Attach the source (an Equipment or Aura) to the creature in `slot` (CR 701.3).
    Attach {
        slot: u8,
    },
    /// The creatures in slots `a` and `b` fight (CR 701.12).
    FightBetween {
        a: u8,
        b: u8,
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
    /// Untap the source.
    UntapSelf,
    /// Destroy all creatures (CR 701.8).
    DestroyAllCreatures,
    /// The source deals damage equal to its power to the target in `slot`, using its last known
    /// power if it has left the battlefield (CR 608.2h).
    DamageEqualSourcePower(u8),
    /// Put the creature in `slot` on the bottom of its owner's library; its controller gains life
    /// equal to its toughness (Condemn).
    BottomAndGainToughness(u8),
    /// Return all attacking creatures to their owners' hands.
    BounceAttackers,
    /// Put creature cards from graveyards onto the battlefield under your control: yours only, or
    /// all graveyards; optionally only those with mana value at most `max_mana_value`.
    ReanimateAll {
        all_graveyards: bool,
        max_mana_value: Option<u32>,
    },
    /// Double the +1/+1 counters on the source.
    DoubleCountersSelf,
    /// The creature in `slot` gets +X/+0 until end of turn, where X is its power (doubling it).
    DoublePower(u8),
    /// "You may sacrifice another creature. If you do, ..." — the rest of the effects happen only
    /// if a creature is sacrificed.
    MaySacrificeAnother,
    /// Return the source from your graveyard to the battlefield tapped.
    ReturnSelfTapped,
    /// Look at the top `n` cards of your library; you may pick one that matches `pick` and put it
    /// `chosen_to`; the rest go `rest_to`.
    LookAtTop {
        n: u8,
        pick: Pick,
        chosen_to: LookTo,
        rest_to: LookRest,
    },
    /// Choose a card matching `pick` in your graveyard (not targeted) and return it to your hand.
    ReturnFromGraveyard {
        pick: Pick,
    },
    /// If the card in slot 0 was a creature card and is now exiled, put a +1/+1 counter on the source
    /// and gain `life` (Scavenging Ooze).
    CounterAndGainIfCreatureExiled {
        life: i32,
    },
    /// Tap the creature in `slot` and put a stun counter on it (CR 122.1d).
    TapAndStun(u8),
    /// Stop here unless you control a creature with at least this power.
    GateControlPowerAtLeast(i32),
    /// "You may pay {cost}. If you do, ..." — the rest happens only if paid.
    MayPay(crate::mana::ManaCost),
    /// Each opponent loses `n` life unless they discard a card (their choice).
    LoseUnlessDiscard {
        n: i32,
    },
    /// Each opponent loses `n` life unless they sacrifice a nonland permanent or discard a card.
    LoseUnlessSacrificeOrDiscard {
        n: i32,
    },
    /// Reveal cards from the top until a creature card; put it in your hand and the rest on the
    /// bottom in a random order.
    RevealUntilCreature,
    /// Double the +1/+1 counters on the permanent in `slot`.
    DoubleCounters(u8),
    /// You gain life equal to the trigger's amount.
    GainLifeAmount,
    /// You draw cards equal to the trigger's amount.
    DrawAmount,
    /// Create tokens equal to the trigger's amount.
    CreateTokensAmount {
        def: usize,
    },
    /// Put incubation counters on the source equal to the trigger's amount.
    IncubationAmount,
    /// Each opponent reveals cards from the top until a land and puts them all into their graveyard.
    OpponentsMillUntilLand,
    /// Return a card from the graveyard to the battlefield under its owner's control with `counters`
    /// +1/+1 counters, an extra creature type and keywords: the card in `slot`, or the source.
    ReturnWith {
        slot: Option<u8>,
        counters: i32,
        subtype: Option<&'static str>,
        keywords: Keywords,
    },
    /// The source's creature types become just this one.
    BecomeSubtype(&'static str),
    /// `who` sacrifices `n` creatures of their choice (all of them if they have `n` or fewer).
    SacrificeCreatures {
        who: Who,
        n: u8,
    },
    /// The instant or sorcery card in `slot` gains flashback until end of turn, for its mana cost.
    GrantFlashback(u8),
    /// Put a soul counter on the source.
    SoulCounterSelf,
    /// Each opponent loses life equal to the source's soul counters (last known).
    OpponentsLoseSoul,
    /// Exile the creature in `slot`; return it under its owner's control at the beginning of the
    /// next end step.
    Flicker(u8),
    /// Return the source at the beginning of the next end step with one fewer revival counter.
    ReturnNextEndStepWithFewerRevival,
    /// End the turn (CR 724).
    EndTheTurn,
    /// Exile the top `n` cards of your library; choose one, which you may play until the end of
    /// your next turn.
    ExileTopChooseOneToPlay {
        n: u8,
    },
    /// Look at the top `n` cards and split them into two piles; an opponent chooses one for your
    /// hand, and the other goes to your graveyard.
    Piles {
        n: u8,
    },
    /// The player in `slot` mills two; repeat while two nonland cards sharing a color were milled.
    TutelageMill(u8),
    /// Put a bait counter on the source.
    BaitCounterSelf,
    /// Remove a bait counter from the source; if you do, create a token.
    RemoveBaitForToken {
        def: usize,
    },
    /// Exile the card in `slot` from a graveyard with a stash counter on it.
    ExileWithStash(u8),
    /// Prevent all combat damage that would be dealt to the creature in `slot` this turn.
    PreventCombatDamageTo(u8),
    /// +P/+T until end of turn, or the morbid amounts if a creature died this turn.
    PumpMorbid {
        slot: u8,
        power: i32,
        toughness: i32,
        morbid_power: i32,
        morbid_toughness: i32,
    },
    /// Gain 1 life for each attacking creature you control with this subtype.
    GainLifePerAttacking(&'static str),
    /// Create a token for each permanent you control with this subtype.
    CreateTokensPerYouControl {
        def: usize,
        sub: &'static str,
    },
    /// Create a token (tapped or not) for each creature card in your graveyard.
    CreateTokensPerGraveyardCreature {
        def: usize,
        tapped: bool,
    },
    /// Create a token for each land you control.
    CreateTokensPerLand {
        def: usize,
    },
    /// Draw a card for each different mana value among nonland permanents you control.
    DrawPerDistinctManaValue,
    /// Draw a card for each creature you control with a +1/+1 counter; those gain indestructible.
    InspiringCall,
    /// The player in `slot` loses life equal to the life you gained this turn.
    LoseLifeEqualGainedThisTurn(u8),
    /// Take an extra turn after this one (CR 500.7).
    ExtraTurn,
    /// The creature in `slot` gets +X/+X until end of turn, X the source's power.
    PumpBySourcePower(u8),
    /// The source deals X damage (CR 107.3).
    DamageX {
        to: Recipient,
    },
    /// Each opponent loses X life; you gain life equal to the life lost.
    DrainX,
    /// X damage to the creature in `slot`, then a token for each point of excess damage
    /// (CR 120.10).
    DamageXExcessTokens {
        slot: u8,
        def: usize,
    },
    /// Reveal the top X cards; put any number of permanent cards with mana value X or less onto the
    /// battlefield and the rest into your graveyard.
    GenesisWave,
    /// Create a token that's a copy of the permanent or card in `slot` (or the source), optionally
    /// with haste and "sacrifice it at the beginning of the end step", or an extra creature type
    /// (CR 707.2, 111.10).
    CopyToken {
        slot: Option<u8>,
        haste_and_sacrifice: bool,
        extra_subtype: Option<&'static str>,
    },
    /// Exile all other tokens you control with this subtype (other than the one just created).
    ExileOtherTokensWithSubtype(&'static str),
    /// Put loyalty counters on the source.
    LoyaltySelf(i32),
    /// Create a token for each point of your life total.
    CreateTokensPerLife {
        def: usize,
    },
    /// Each opponent keeps one permanent of each permanent type and sacrifices the rest.
    KeepOneOfEachType,
    /// You get an emblem (CR 114).
    Emblem(EmblemKind),
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
    /// A creature card with mana value at most X.
    CreatureMvAtMostX,
    /// A creature card with mana value at most this (X filled in on resolution).
    CreatureMvAtMost(u32),
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
    /// The creature this Equipment or Aura is attached to.
    Attached,
    /// Other creatures you control with flying (printed or granted until end of turn).
    OtherYourFliers,
    /// Other creatures you control with this subtype.
    OtherYourSubtype(&'static str),
    /// Creatures you control of the source's chosen color.
    YourCreaturesOfChosenColor,
}

/// What a scaled static ability counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    /// Permanents you control with this subtype ("for each Forest you control").
    PerYouControlSubtype(&'static str),
}

/// A condition on a static ability ("as long as ...").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticCondition {
    /// Threshold: seven or more cards in your graveyard.
    Threshold,
    /// The source is untapped.
    Untapped,
    /// The source has at least this many +1/+1 counters.
    CountersAtLeast(i32),
    /// Your life total is at least this.
    LifeAtLeast(i32),
}

/// A static ability that changes characteristics (CR 604, 613 layers 6 and 7c).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Static {
    pub scope: Scope,
    /// "During your turn".
    pub your_turn_only: bool,
    pub condition: Option<StaticCondition>,
    /// An extra creature type it gives.
    pub subtype: Option<&'static str>,
    /// The creature loses all abilities (layer 6) and has this base power and toughness (layer 7b).
    pub lose_abilities: bool,
    pub base_pt: Option<(i32, i32)>,
    /// Protection from these colors, WUBRG (CR 702.16).
    pub protection: [bool; 5],
    /// Multiply power and toughness by a count.
    pub scale: Option<Scale>,
    pub power: i32,
    pub toughness: i32,
    pub keywords: Keywords,
}

/// What an emblem does (CR 114).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmblemKind {
    /// Creatures you control get +2/+2 and have vigilance, trample and indestructible.
    VivienAnthem,
    /// Whenever a player casts a spell, its owner's card's trigger number `trigger` happens (Kaito).
    OnAnySpellCast { trigger: usize },
}

/// Which looked-at card may be picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Any,
    CreaturePowerAtMost(i32),
    InstantOrSorcery,
    /// A creature card with mana value at most the number of lands you control.
    CreatureMvAtMostLands,
    CreatureOrLand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookTo {
    Hand,
    Top,
    Battlefield,
    /// Stays in exile; you may play it until the end of your next turn.
    PlayFromExile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookRest {
    Graveyard,
    /// The bottom of the library in a random order.
    BottomRandom,
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
    /// You control another creature with at least this power.
    ControlAnotherPowerAtLeast(i32),
    /// Morbid: a creature died this turn.
    CreatureDiedThisTurn,
    /// Always true (an unconditional "enters with" counter).
    Always,
    /// An opponent lost life this turn.
    OpponentLostLifeThisTurn,
    /// The source wasn't this creature type when it left the battlefield.
    SourceWasNot(&'static str),
    /// The source had a revival counter when it left the battlefield.
    HadRevival,
    /// Threshold: seven or more cards in your graveyard.
    Threshold,
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
    /// A modal triggered ability's modes ("choose one —"): if not empty, the controller picks one as
    /// the ability goes on the stack (CR 700.2b), and its targets come from that mode.
    pub modes: Vec<Ability>,
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
            modes: Vec::new(),
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
    /// Landfall: "Whenever a land you control enters".
    LandYouControlEnters,
    /// "Whenever this creature deals combat damage to a player".
    DealsCombatDamageToPlayer,
    /// "Whenever equipped creature deals combat damage to a player".
    EquippedDealsCombatDamageToPlayer,
    /// "Whenever another nontoken creature dies" (any player's).
    AnotherNontokenCreatureDies,
    /// "Whenever another nontoken creature you control dies".
    AnotherNontokenCreatureYouControlDies,
    /// "Whenever this or another nontoken [subtype] you control enters".
    NontokenYouControlEntersWithSubtype(&'static str),
    /// "Whenever a creature you control with power N or greater enters".
    YourCreatureEntersWithPowerAtLeast(i32),
    /// "Whenever you attack with N or more creatures".
    YouAttackWithAtLeast(u8),
    /// "Whenever an opponent casts a spell".
    OpponentCasts,
    /// "Whenever you put one or more +1/+1 counters on this creature", at most once each turn.
    CountersPutOnThis,
    /// "Whenever one or more +1/+1 counters are put on another non-Hydra creature you control".
    CountersOnAnotherNonHydraYouControl,
    /// "Whenever a creature you control deals combat damage to a player".
    YourCreatureDealsCombatDamageToPlayer,
    /// "Whenever an opponent loses life" (the amount travels with the trigger).
    OpponentLosesLife,
    /// "Whenever a source you control deals noncombat damage to an opponent" (with the amount).
    YourSourceDealsNoncombatDamageToOpponent,
    /// "Whenever you cast a spell".
    YouCast,
    /// "Whenever a nontoken creature you control without [subtype] dies"; slot 0 is that card.
    YourNontokenCreatureDiesWithout(&'static str),
    /// "At the beginning of combat on your turn", for a card in your graveyard.
    BeginCombatYourTurnFromGraveyard,
    /// "Whenever an opponent discards a card"; slot 0 is that card.
    OpponentDiscards,
    /// "Whenever equipped creature becomes untapped".
    EquippedBecomesUntapped,
    /// "Whenever you gain life" (every time).
    YouGainLife,
    /// "Whenever you attack": once per combat, when you declare one or more attackers.
    YouAttack,
    /// "Whenever another [subtype] you control enters".
    AnotherYouControlEntersWithSubtype(&'static str),
    /// "Whenever this creature or another creature you control dies".
    CreatureYouControlDies,
    /// "At the beginning of combat on your turn".
    BeginCombatYourTurn,
    /// "At the beginning of [your / each] end step".
    EndStep { your_turn_only: bool },
    /// "Whenever you cast a spell during an opponent's turn".
    YouCastOnOpponentsTurn,
    /// "At the beginning of [your / each] upkeep".
    Upkeep { your_turn_only: bool },
    /// "Whenever you draw a card".
    YouDraw,
    /// "Whenever an opponent draws a card".
    OpponentDraws,
    /// "Whenever you cast an instant or sorcery spell".
    YouCastInstantOrSorcery,
    /// "Whenever you cast a noncreature or [subtype] spell".
    YouCastNoncreatureOrSubtype(&'static str),
    /// "Whenever a creature an opponent controls enters".
    CreatureOpponentControlsEnters,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trigger {
    pub when: When,
    pub ability: Ability,
}
