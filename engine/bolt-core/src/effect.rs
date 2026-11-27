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
    /// Creatures with at most this power ("with power 2 or less").
    pub creature_max_power: Option<i32>,
    /// Creatures without flying.
    pub creature_without_flying: bool,
    /// Permanents with at least this mana value.
    pub min_mana_value: Option<u32>,
    /// Graveyard cards that are neither creatures nor lands.
    pub noncreature_nonland: bool,
    /// Creatures only if attacking or blocking.
    pub in_combat: bool,
    /// Only a permanent attached to the object in the first target slot ("Equipment attached to
    /// that creature").
    pub attached_to_first_target: bool,
    /// Lands.
    pub lands: bool,
    /// Creatures that were dealt damage this turn.
    pub was_dealt_damage: bool,
    /// Noncreature permanents only ("noncreature artifact").
    pub noncreature: bool,
    /// Nonbasic permanents only ("nonbasic land").
    pub nonbasic: bool,
    /// Cards whose mana value is the ability's X.
    pub mana_value_is_x: bool,
    /// Permanents that are one or more colors.
    pub colored: bool,
    /// Creature or land cards only (graveyard targets).
    pub creature_or_land: bool,
    /// Nonland cards (graveyard targets).
    pub nonland_cards: bool,
    /// Tokens only.
    pub tokens_only: bool,
    /// Nonlegendary permanents only.
    pub nonlegendary: bool,
    /// Nontoken permanents only.
    pub nontoken: bool,
    /// Permanents that entered the battlefield this turn.
    pub entered_this_turn: bool,
    /// Abilities on the stack too ("target spell or ability"): an activated or triggered ability is
    /// targeted as its source object (the topmost of that source's abilities).
    pub abilities: bool,
    /// With `spells`: only enchantment, instant or sorcery spells (Swan Song).
    pub spells_enchantment_instant_sorcery: bool,
    /// Legendary permanents only.
    pub legendary_only: bool,
    /// Artifact or creature spells only (Scatter Ray), with `spells`.
    pub spell_artifact_or_creature: bool,
    /// Nonartifact permanents or spells only (Go for the Throat).
    pub nonartifact: bool,
    /// Only permanents or spells of this color, WUBRG index (Red Elemental Blast: blue).
    pub color: Option<u8>,
    /// Only permanents or spells that aren't this color (Doom Blade: nonblack).
    pub not_color: Option<u8>,
    /// Monocolored permanents or spells only (Sultai Charm).
    pub monocolored: bool,
    /// Spells with at least this mana value, X included (Disdainful Stroke).
    pub spell_min_mana_value: Option<u32>,
    /// Artifact, creature or planeswalker spells only (Strix Serenade), with `spells`.
    pub spell_artifact_creature_or_planeswalker: bool,
    /// Spells or abilities that target you or a creature you control (Siren Stormtamer).
    pub targets_you_or_your_creature: bool,
    /// Spells with a single target only (Imp's Mischief).
    pub single_target: bool,
    /// Only while it's your main phase (Return to Dust's second target: "If you cast this spell
    /// during your main phase").
    pub only_in_your_main_phase: bool,
    /// Attacking creatures only (Reconnaissance).
    pub attacking: bool,
    /// Instant and sorcery spells only (Flusterstorm), with `spells`.
    pub spell_instant_or_sorcery: bool,
    /// Creatures with defender only (Smash to Dust).
    pub creature_needs_defender: bool,
    /// Artifact cards only (graveyard targets: Buried Ruin).
    pub artifact_cards: bool,
    /// Creatures with power or toughness at most this (Warping Wail: 1).
    pub creature_power_or_toughness_at_most: Option<i32>,
    /// Sorcery spells only (Warping Wail), with `spells`.
    pub spell_sorcery: bool,
    /// Only objects with an even mana value; zero is even (Desecrate Reality).
    pub mana_value_even: bool,
    /// Cards or permanents with mana value at most the ability's X (Kozilek's Command: "target
    /// creature with mana value X or less").
    pub mana_value_at_most_x: bool,
    /// One of "up to X targets" (Kozilek's Command: "up to X target cards from graveyards"): repeat
    /// the filter in as many slots as X can reach, with `min_targets: Some(0)`; a slot is only
    /// legal while fewer than X other targets are chosen, so no more than X are.
    pub up_to_x: bool,
    /// Tapped permanents only (Far Traveler: "up to one target tapped creature you control").
    pub tapped_only: bool,
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
        creature_max_power: None,
        creature_without_flying: false,
        min_mana_value: None,
        noncreature_nonland: false,
        in_combat: false,
        attached_to_first_target: false,
        lands: false,
        was_dealt_damage: false,
        noncreature: false,
        nonbasic: false,
        mana_value_is_x: false,
        colored: false,
        creature_or_land: false,
        nonland_cards: false,
        tokens_only: false,
        nonlegendary: false,
        nontoken: false,
        entered_this_turn: false,
        abilities: false,
        spells_enchantment_instant_sorcery: false,
        legendary_only: false,
        spell_artifact_or_creature: false,
        nonartifact: false,
        color: None,
        not_color: None,
        monocolored: false,
        spell_min_mana_value: None,
        spell_artifact_creature_or_planeswalker: false,
        targets_you_or_your_creature: false,
        single_target: false,
        only_in_your_main_phase: false,
        attacking: false,
        spell_instant_or_sorcery: false,
        creature_needs_defender: false,
        artifact_cards: false,
        creature_power_or_toughness_at_most: None,
        spell_sorcery: false,
        mana_value_even: false,
        mana_value_at_most_x: false,
        up_to_x: false,
        tapped_only: false,
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
    /// The active player ("that player" in an each-upkeep trigger, Seizan).
    ActivePlayer,
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
    /// Each creature and each planeswalker.
    EachCreatureAndPlaneswalker,
    /// The ability's controller ("deals 1 damage to you": City of Brass).
    You,
    /// Each creature with flying and each player (Squall Line).
    FliersAndEachPlayer,
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
    /// If the source has this subtype, it becomes stage `to` (Kellan).
    AdvanceStage {
        if_subtype: &'static str,
        to: u8,
    },
    /// The source becomes a creature with this base power and toughness, vigilance and every
    /// creature type, and stays one (Soulstone Sanctuary). It's still a land.
    BecomeCreatureSelf {
        power: i32,
        toughness: i32,
    },
    /// Exile the top card of each player's library; you may cast any number of spells from among
    /// them without paying their mana costs (Etali).
    ExileTopOfEachCastFree,
    /// Copy the triggering spell once per the trigger's amount, with the same targets (CR 707.10).
    CopySpellPerAmount,
    /// "that many" +1/+1 counters on the object in `slot`: the trigger's amount.
    CountersAmount(u8),
    /// Create `n` tokens of definition `def` for `who` (CR 111).
    CreateToken {
        who: Who,
        def: usize,
        n: u8,
    },
    /// Mobilize (Tarkir: Dragonstorm): create this many 1/1 red Warrior tokens (`def`) tapped and
    /// attacking; sacrifice them at the beginning of the next end step. `None`: X, the number of
    /// creature cards in your graveyard.
    Mobilize {
        def: usize,
        n: Option<u8>,
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
    /// "Discard a card. If you do, draw N cards."
    DiscardThenDraw(u8),
    /// The ability's controller chooses a nonland card from the hand of the player in `slot`, who
    /// discards it.
    ChooseDiscard {
        slot: u8,
    },
    /// Look at the top `n` cards; put any number into the graveyard, the rest back on top (CR 701.25).
    Surveil {
        n: u8,
    },
    /// Scry: look at the top `n`; put any number on the bottom, the rest back on top (CR 701.22). Kept
    /// cards stay in their order (a simplification: no reordering).
    Scry {
        n: u8,
    },
    /// "It endures N" (Tarkir: Dragonstorm): its controller puts N +1/+1 counters on the source, or
    /// creates an N/N white Spirit token (`spirit`, a 0/0 given N counters) (CR 701.62).
    Endure {
        n: u8,
        spirit: usize,
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
    /// "Double the power and toughness of" the creature in `slot` until end of turn.
    DoublePowerToughness(u8),
    /// "You may sacrifice another creature. If you do, ..." — the rest of the effects happen only
    /// if a creature is sacrificed.
    MaySacrificeAnother,
    /// Return the source from your graveyard to the battlefield tapped.
    ReturnSelfTapped,
    /// Return the source from your graveyard to your hand.
    ReturnSelfToHand,
    /// Remove all counters from the creature in `slot` (+1/+1, stun and keyword counters).
    RemoveAllCounters(u8),
    /// "Double the power and toughness of each creature you control until end of turn."
    DoublePowerToughnessAll,
    /// +1/+1 counters divided among its targets as chosen when it went on the stack (`Ability::divide`).
    DividedCounters,
    /// Return the source from the battlefield to its owner's hand.
    BounceSelf,
    /// The tokens just created gain these keywords until end of turn ("They gain menace and haste").
    GrantCreated(crate::Keywords),
    /// An X/X Spirit (`spirit`, given X counters), X the greatest toughness among your creatures.
    SpiritGreatestToughness {
        spirit: usize,
    },
    /// Keyword counters on the creature in `slot` (CR 122.1b): it has these keywords for good.
    KeywordCounter {
        slot: u8,
        keywords: crate::Keywords,
    },
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
    /// "Counter target spell unless its controller pays {n}" ({m} instead if the spell was kicked or
    /// a Dragon was beheld): (slot, n, m).
    CounterUnlessPays(u8, u8, u8),
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
        /// Playable only this turn, instead of until the end of your next turn.
        this_turn: bool,
    },
    /// Gain control of the permanent in `slot` until end of turn (CR 613.1b).
    GainControlUntilEot(u8),
    /// Untap the permanent in `slot`.
    UntapTarget(u8),
    /// "If that creature would die this turn, exile it instead" for the creature in `slot`.
    ExileIfDiesThisTurn(u8),
    /// You may cast the card in `slot` (in your graveyard) this turn.
    PlayableThisTurn(u8),
    /// Add `n` mana of one color to your mana pool (CR 106.4).
    AddMana {
        color: crate::Color,
        n: u8,
    },
    /// Deal the damage divided as the ability was put on the stack to each of its targets (CR
    /// 601.2d); see [`Ability::divide`].
    DividedDamage,
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
    /// Put the card in `slot` on top of (`true`) or at the bottom of its owner's library.
    ToLibrary(u8, bool),
    /// The creature in slot `from` deals damage equal to its power to the one in slot `to`.
    Bite {
        from: u8,
        to: u8,
    },
    /// Exile a player's graveyard.
    ExileGraveyard {
        who: Who,
    },
    /// The creature in `slot` gets +X/+X until end of turn, X the number of creatures you control
    /// (or, with `attacking`, the number of attacking creatures), counted on resolution.
    PumpPerCreatures {
        slot: u8,
        attacking: bool,
    },
    /// The source deals damage equal to the number of creatures you control to the target in `slot`.
    DamagePerCreatures(u8),
    /// Creatures you control get +X/+X until end of turn, X the number of creatures you control
    /// (Craterhoof Behemoth), counted once on resolution (CR 608.2h).
    PumpYoursPerCreatures,
    /// The creature the source is attached to gains keywords until end of turn (an Aura's enters
    /// trigger: "enchanted creature gains first strike until end of turn").
    GrantAttached(Keywords),
    /// The source can attack this turn as though it didn't have defender.
    AttackDespiteDefenderSelf,
    /// Put `n` +1/+1 counters on each creature `who` controls.
    CountersAll {
        who: Who,
        n: i32,
    },
    /// "Draw a card if you control a creature with a counter on it. If you don't draw a card this
    /// way, put a +1/+1 counter on this creature." (Trade Route Envoy)
    DrawIfCounterElseCounterSelf,
    /// "The next spell you cast this turn can't be countered." (Mistrise Village)
    NextSpellUncounterable,
    /// Tap the source ("Tap it.").
    TapSelf,
    /// The player in `slot` exiles a creature they control of their choice (Strategic Betrayal).
    PlayerExilesCreature(u8),
    /// The source deals damage equal to the power of the card in slot `card` to the target in slot
    /// `to` (Lie in Wait); nothing if that card's target became illegal.
    DamageEqualCardPower {
        card: u8,
        to: u8,
    },
    /// Destroy all nonland permanents your opponents control.
    DestroyOpponentsNonland,
    /// Destroy all creatures and enchantments, then draw a card for each permanent destroyed.
    DestroyCreaturesAndEnchantmentsDraw,
    /// Exile the top `n` cards of your library; until the end of your next turn, you may play them.
    ExileTopToPlayAll(u8),
    /// The tokens just created are sacrificed at the beginning of the next end step.
    SacrificeCreatedAtEnd,
    /// If the target in `slot` is a player (who was just dealt damage this way), they discard a card.
    DiscardIfPlayerTarget(u8),
    /// Put `n` stun counters on the source.
    StunSelf(u8),
    /// "Whenever you attack this turn, ...": the source's trigger number `trigger` triggers whenever
    /// you attack for the rest of this turn (a delayed triggered ability, CR 603.7).
    DelayedOnAttackThisTurn(usize),
    /// The ability's controller chooses a nonland card from the hand of the player in `slot`, and
    /// exiles it (Aggressive Negotiations).
    ChooseExileFromHand {
        slot: u8,
    },
    /// Return the permanent in `slot` to its owner's hand; if it was a token, draw a card.
    BounceDrawIfToken(u8),
    /// The player in `slot` mills cards equal to the source's power (last known if it left).
    MillSourcePower(u8),
    /// "You may exile `n` [creature] cards from your graveyard. If you do, ..." — the rest of the
    /// effects happen only if they are exiled.
    MayExileFromGraveyard {
        n: u8,
        creature_only: bool,
    },
    /// Stop here unless you control at least this many attacking creatures.
    GateAttackingAtLeast(u8),
    /// Tap the creature the source (an Aura) is attached to.
    TapAttached,
    /// Untap the creature in the stack item's `extra` (a granted "{cost}: Untap this creature").
    UntapExtra,
    /// "When it dies under your control this turn, ...": the source's trigger number `trigger`.
    GrantDiesUnderYourControl {
        slot: u8,
        trigger: usize,
    },
    /// Put the counters the creature in the stack item's `extra` had when it died on the creature
    /// in `slot` (Host of the Hereafter).
    PutDeadCountersOn(u8),
    /// Until end of turn, the creature in `slot` gains "Whenever this creature deals combat damage to
    /// a player, ..." (the source's trigger number `trigger`, controlled by that creature's
    /// controller).
    GrantCombatDamageTrigger {
        slot: u8,
        trigger: usize,
    },
    /// Return the spell or permanent in `slot` to its owner's hand.
    BounceSpellOrPermanent(u8),
    /// "You may sacrifice a nonland permanent. When you do, ...": the rest only if one is sacrificed.
    MaySacrificeNonland,
    /// Discard your hand, then draw a card for each spell you've cast this turn (Narset).
    DiscardHandDrawPerSpells,
    /// Betor: draw a card; then if creatures you control have total toughness 20 or greater, untap
    /// them; then if 40 or greater, each opponent loses half their life, rounded up.
    BetorEndStep,
    /// Look at the top `n` cards; put up to `up_to` (0: any number) permanent cards from among them
    /// onto the battlefield (only noncreature, nonland ones with mana value at most `max_mv`, if set);
    /// the rest go to your hand, or to the bottom of your library in a random order.
    TopCardsOntoBattlefield {
        n: u8,
        up_to: u8,
        noncreature_nonland: bool,
        max_mv: Option<u32>,
        rest_to_hand: bool,
    },
    /// Exile this spell as it resolves (instead of putting it into the graveyard).
    ExileSourceSpell,
    /// Return any number of permanent cards with different names from your graveyard to the
    /// battlefield.
    ReturnPermanentsDifferentNames,
    /// Discard up to `n` cards, then draw that many.
    DiscardUpToThenDrawThatMany(u8),
    /// Exile a card at random from your graveyard; you may play it this turn.
    ExileRandomFromGraveyardPlayable,
    /// Exile the top X cards of the damaged opponent's library (X: the trigger's amount, the combat
    /// damage dealt); you may cast spells with mana value X or less from among them free.
    ExileTopOfDamagedCastFree,
    /// "Target opponent reveals their hand. You may choose a nonland card from it. If you do, exile
    /// that card": the card is linked to the source (Severance Priest).
    ChooseExileFromHandLinked {
        slot: u8,
    },
    /// The owner of the card linked to the source creates an X/X Spirit (`spirit`, a 0/0 given X
    /// counters), X that card's mana value.
    LinkedOwnerCreatesSpirit {
        spirit: usize,
    },
    /// Copy the source spell (from its snapshot as it was cast): once, or with `storm`, once for each
    /// other spell cast before it this turn. A copy of a permanent spell becomes a token.
    CopySourceSpell {
        storm: bool,
    },
    /// Destroy the land in `slot`; its controller searches for a basic land card and puts it onto
    /// the battlefield tapped with a stun counter (Magmatic Hellkite).
    DestroyLandControllerFetchesStunned(u8),
    /// Stillness in Motion: if your library is empty, exile the source and put five cards from your
    /// graveyard on top of your library in any order.
    StillnessCheck,
    /// Call the Spirit Dragons: for each color, a +1/+1 counter on a Dragon you control of that
    /// color; five different Dragons and you win the game.
    SpiritDragonsUpkeep,
    /// Creatures you control gain these keywords until your next turn.
    GrantAllUntilYourNextTurn(crate::Keywords),
    /// "There is an additional combat phase after this phase followed by an additional main phase"
    /// (cast during a main phase).
    AdditionalCombatAndMain,
    /// "When you next attack this turn, ...": the source's trigger number `n`, once.
    DelayedOnNextAttackThisTurn(usize),
    /// Untap each creature you control.
    UntapCreaturesYouControl,
    /// "Copy the next spell you cast this turn when you cast it": the source's trigger number `n`
    /// triggers when you next cast a spell this turn, with that spell in `extra`.
    CopyNextSpellThisTurn(usize),
    /// Copy the spell in `extra` (still on the stack); its controller may choose new targets.
    CopySpellNewTargets,
    /// Add `n` colorless mana.
    AddColorless(u8),
    /// For the rest of this turn, the source's trigger number `trigger` triggers on `when` (one of
    /// `When::CreatureYouControlEntersThisTurn` or `When::YouCastNoncreature`): a delayed triggered
    /// ability (CR 603.7).
    DelayedThisTurn {
        when: When,
        trigger: usize,
    },
    /// "You may discard a card. When you do, ...": the rest only if a card is discarded.
    MayDiscardThen,
    /// Exile cards from the top until a nonland card; you may cast it without paying its mana cost
    /// if its mana value is at most `max_mv`, or else it goes to your hand (Breaching Dragonstorm).
    ExileUntilNonlandCastFree {
        max_mv: u32,
    },
    /// The source becomes a creature with this base power and toughness, creature type and keywords
    /// until end of turn, in addition to its other types.
    BecomeCreatureUntilEot {
        power: i32,
        toughness: i32,
        subtype: &'static str,
        keywords: crate::Keywords,
    },
    /// "The next time a source of your choice would deal damage to you this turn, prevent that
    /// damage. When damage is prevented this way, ..." (the source's trigger number `trigger`, with
    /// the amount and the source in `extra`).
    PreventNextFromChosenSource {
        trigger: usize,
    },
    /// The source deals damage equal to the trigger's amount to the controller of the object in
    /// `extra`, and you draw that many cards (New Way Forward).
    DamageExtrasControllerAndDrawAmount,
    /// "Counter target spell unless its controller pays {X}", X this spell's X.
    CounterUnlessPaysX(u8),
    /// "You may discard a card. When you discard a nonland card this way, ...": the source's trigger
    /// number `n` (a reflexive trigger).
    MayDiscardReflexiveIfNonland(usize),
    /// Creatures you control with counters on them gain keywords until end of turn.
    GrantYourCreaturesWithCounters(Keywords),
    /// "Discard two cards unless you discard a creature card."
    DiscardTwoUnlessCreature,
    /// "This creature endures X", X the ability's X.
    EndureX {
        spirit: usize,
    },
    /// The source deals damage equal to the number of creatures you control to the creature in `slot`.
    DamageEqualCreaturesYouControl(u8),
    /// Mill cards equal to the number of lands you control.
    MillLandsYouControl,
    /// +1/+1 counters on the creature in `slot`, one per land card in your graveyard.
    CountersPerLandCardInGraveyard(u8),
    /// The creature in `slot` endures X, X the number of counters on the source (Warden of the Grove).
    SlotEnduresSourceCounters {
        slot: u8,
        spirit: usize,
    },
    /// "You may ..." for the rest of the effects.
    MayContinue,
    /// Attach the source (an Equipment) to the token just created.
    AttachToCreated,
    /// The instant or sorcery card in `slot` gains harmonize until end of turn, its harmonize cost its
    /// mana cost (Songcrafter Mage).
    GrantHarmonize(u8),
    /// Return up to `n` permanent cards of your choice from your graveyard to your hand.
    ReturnUpToPermanentCardsToHand(u8),
    /// The player in `slot` shuffles up to `n` cards (your choice) from their graveyard into their
    /// library.
    ShuffleUpToFromGraveyard {
        slot: u8,
        n: u8,
    },
    /// "If this is the second time this ability has resolved this turn, you draw `n` cards." (Ms.
    /// Bumbleflower; counted per source, each turn)
    DrawIfSecondResolutionThisTurn(u8),
    /// "You may behold a Dragon. If you do, ...": the rest only if beheld.
    MayBeholdThen,
    /// The source gains this creature type until end of turn.
    SubtypeUntilEotSelf(&'static str),
    /// For each opponent, a tapped token copy of the card the source exiled (until it leaves)
    /// attacking that opponent; sacrificed at the beginning of the next end step (Mardu Siegebreaker).
    CopyLinkedExileAttacking,
    /// Exile the card in `slot` from your graveyard, copy it, and you may cast the copy free (Shiko).
    ExileCastCopyFree(u8),
    /// Search for up to three monocolored cards with different names and exile them; an opponent
    /// chooses one to shuffle back; you may cast the others free (Emergent Ultimatum).
    SearchMonocoloredOpponentPicks,
    /// Discard a card unless you attacked this turn (Chart a Course).
    DiscardUnlessAttacked,
    /// Offspring: create a 1/1 token copy of the source.
    OffspringCopy,
    /// Add X mana in any combination of colors (X the ability's X): X any-color mana in the pool.
    AddManaAnyColorsX,
    /// Create a token that's a copy of the token in `slot`; two if it has this subtype (Hazel).
    CopyTokenTwiceIfSubtype {
        slot: u8,
        sub: &'static str,
    },
    /// Put a decayed counter on the creature in `slot` (CR 702.147: it gains decayed).
    DecayedCounter(u8),
    /// Exile the creature in `slot`; its controller may search for a basic land card and put it
    /// onto the battlefield tapped (Path to Exile).
    ExileControllerMayFetchBasic(u8),
    /// Double the number of +1/+1 counters on each creature you control (Kalonian Hydra).
    DoubleCountersEachYours,
    /// "You may play an additional land this turn."
    ExtraLandThisTurn,
    /// Reveal the top card of your library: a land goes onto the battlefield, anything else to your
    /// hand (Coiling Oracle).
    RevealTopLandToBattlefieldElseHand,
    /// Destroy the permanent in `slot` and all other permanents with its name (Maelstrom Pulse).
    DestroyWithSameName(u8),
    /// Draw X cards (X the spell's X).
    DrawX,
    /// Add one mana of any color (into the pool).
    AddManaAnyColor,
    /// Exile the creature in `slot`; its controller gains life equal to its power (Swords to
    /// Plowshares).
    ExileControllerGainsPower(u8),
    /// Each creature without this subtype gets -X/-X until end of turn, X the spell's X (Exude Toxin).
    ShrinkAllWithoutSubtypeX(&'static str),
    /// You lose life equal to the mana value of the spell in the trigger's `extra` (Teval).
    LoseLifeCastSpellManaValue,
    /// Each other creature you control becomes a copy of the creature in `slot` until end of turn.
    OthersBecomeCopyOfTarget(u8),
    /// Copy the spell in the trigger's `extra`, then exile that spell with four time counters: it
    /// has suspend (Taigam's flurry).
    CopyThenSuspendSpell,
    /// Exile the permanent in `slot`, then return that card to the battlefield under your control.
    BlinkSlot(u8),
    /// Draw a card for each creature you control.
    DrawPerCreatureYouControl,
    /// Gain `per` life for each creature you control with power `power` or greater.
    GainLifePerCreaturePowerAtLeast {
        per: i32,
        power: i32,
    },
    /// Create X tokens of `def` (X the spell's X).
    CreateTokensX {
        def: usize,
    },
    /// Return a creature you control (your choice, not targeted) to its owner's hand.
    ReturnCreatureYouControl,
    /// Return all creature tokens (`tokens`) or all nontoken creatures to their owners' hands.
    BounceAllCreatures {
        tokens: bool,
    },
    /// Destroy all creatures with power `n` or greater.
    DestroyAllCreaturesPowerAtLeast(i32),
    /// Other creatures you control get +P/+T and gain keywords until end of turn.
    PumpOthersAndGrant {
        power: i32,
        toughness: i32,
        keywords: crate::Keywords,
    },
    /// Attacking creatures you control get +X/+X until end of turn, X the number of attacking
    /// creatures (Jazal Goldmane).
    PumpAttackersPerAttacker,
    /// Create tokens (`def`) equal to the source's power (last known), tapped (Nested Shambler).
    CreateTokensPowerTapped {
        def: usize,
    },
    /// Destroy all creatures if the spell's X is at least `n` (Martial Coup, before its tokens are
    /// made, so they survive: "destroy all other creatures").
    DestroyAllCreaturesIfXAtLeast(u8),
    /// Untap all nonland permanents (yours and the opponent's: Intellectual Offering in 1v1).
    UntapAllNonland,
    /// The source gets +X/+0 until end of turn, X its power ("double this creature's power").
    DoublePowerSelf,
    /// Create tapped tokens (`def`) equal to the trigger's amount (Prosperous Bandit).
    CreateTokensAmountTapped {
        def: usize,
    },
    /// "You win the game." (CR 104.2a)
    WinTheGame,
    /// Create a token (`def`) for each spell you've cast this turn (Murmuration).
    CreateTokensPerSpellCastThisTurn {
        def: usize,
    },
    /// Vanishing's upkeep trigger: remove a time counter from the source; when the last is removed,
    /// sacrifice it.
    VanishingTick,
    /// Tap the permanent in `slot`; it doesn't untap during its controller's next untap step.
    TapFreeze(u8),
    /// Each creature you control with power `min` or greater gets +P/+T and gains keywords until
    /// end of turn (Goreclaw).
    PumpYoursPowerAtLeastAndGrant {
        min: i32,
        power: i32,
        toughness: i32,
        keywords: crate::Keywords,
    },
    /// "Gift a card": the opponent draws a card (the gift was promised: the spell was cast kicked),
    /// and "whenever you give a gift" triggers.
    GiftCard,
    /// Put a +1/+1 counter on each green creature that entered the battlefield this turn.
    CountersOnGreenEnteredThisTurn,
    /// Put X +1/+1 counters on the creature in `slot`, X its power.
    CountersEqualPower(u8),
    /// You win the game if the source has at least `n` counters or you have at least `n` cards in
    /// hand (Twenty-Toed Toad).
    WinIfCountersOrHandAtLeast(i32),
    /// Add one mana that can be either of these colors.
    AddManaEither(crate::Color, crate::Color),
    /// Creature spells you cast this turn can't be countered.
    CreatureSpellsUncounterableThisTurn,
    /// Return the creature in the trigger's `extra` from its owner's graveyard to the battlefield
    /// under its owner's control with a flying counter on it (Luminous Broodmoth).
    ReturnDiedWithFlyingCounter,
    /// Put a quest counter on the source.
    QuestCounterSelf,
    /// Creatures the opponent controls attack this turn if able (Bident of Thassa).
    OpponentsAttackThisTurn,
    /// Create a token (`def`) for each +1/+1 counter the source had as it left the battlefield.
    CreateTokensPerLastCounters {
        def: usize,
    },
    /// Put a +1/+1 counter on the source for each other permanent you control with one of these
    /// subtypes.
    CountersSelfPerOtherWithSubtypes(&'static [&'static str]),
    /// For each token you control, create a token that's a copy of it.
    CopyEachTokenYouControl,
    /// Exile the permanent in `slot`; the source remembers it (its `linked_card`).
    ExileLinked(u8),
    /// The source's linked card's owner creates an X/X token (`def`), X that card's mana value.
    LinkedOwnerCreatesXx {
        def: usize,
    },
    /// The opponent may accept the offer: if they decline, the rest of the effects don't happen.
    OpponentMay,
    /// The creature in `slot` explores (CR 701.44): reveal the top card; a land goes to hand, else
    /// a +1/+1 counter on the creature and its controller may put the card into the graveyard.
    Explore(u8),
    /// Return a land you control (your choice, not targeted) to its owner's hand (the karoo lands).
    ReturnLandYouControl,
    /// The creature in slot `from` deals damage equal to its power (last known if it left) to the
    /// target in slot `to`, a player or a permanent (Warstorm Surge).
    DamageEqualSlotPower {
        from: u8,
        to: u8,
    },
    /// The active player draws `n` cards (Rites of Flourishing: "that player draws").
    ActivePlayerDraws(u8),
    /// Put a loyalty counter on each planeswalker you control with this subtype (Garruk's Wolves).
    LoyaltyEachYouControlSubtype(&'static str),
    /// `who` sacrifices a creature or planeswalker of their choice; if they can't, they discard a
    /// card (Plaguecrafter).
    SacrificeCreatureOrPlaneswalkerElseDiscard {
        who: Who,
    },
    /// The owner of the permanent in `slot` shuffles it into their library, then reveals the top
    /// card; a permanent card goes onto the battlefield (Chaos Warp).
    ChaosWarp(u8),
    /// Destroy all creatures, then draw a card for each one destroyed (Decree of Pain).
    DestroyAllCreaturesDrawPerDestroyed,
    /// Prevent all combat damage that would be dealt this turn (Spore Frog).
    PreventAllCombatDamageThisTurn,
    /// Prevent all damage that would be dealt to you this turn (Riot Control).
    PreventAllDamageToYouThisTurn,
    /// Gain 1 life for each creature your opponents control (Riot Control).
    GainLifePerOpponentCreature,
    /// Untap all lands you control.
    UntapLandsYouControl,
    /// Create tokens (`def`) equal to the source's power (last known), untapped (Jacked Rabbit).
    CreateTokensPower {
        def: usize,
    },
    /// Parley (Selvala): each player reveals their top card; for each nonland card, add {G} and you
    /// gain 1 life; then each player draws a card.
    SelvalaParley,
    /// Each creature without one of these subtypes gets -X/-X until end of turn, X the creatures you
    /// control with one of them (Swarmyard Massacre).
    ShrinkAllWithoutSubtypesPerYours(&'static [&'static str]),
    /// The land in `slot` becomes a creature with this base power and toughness, creature type and
    /// keywords until end of turn; it's still a land (Llanowar Loamspeaker).
    SlotBecomesCreatureUntilEot {
        slot: u8,
        power: i32,
        toughness: i32,
        subtype: &'static str,
        keywords: crate::Keywords,
    },
    /// Set the base power and toughness of each creature you control to X/X until end of turn, X the
    /// cards in your hand on resolution (Jolrael).
    YourCreaturesBasePtHandSize,
    /// You gain 1 life for each creature card in your graveyard (Skyfisher Spider).
    GainLifePerCreatureCardInGraveyard,
    /// Exile the source from its owner's graveyard.
    ExileSourceFromGraveyard,
    /// Evolve (CR 702.100): if the creature in `slot` has greater power or toughness than the source,
    /// put a +1/+1 counter on the source (checked as it resolves).
    Evolve(u8),
    /// Put `n` of the source's own kind of counter on it (hoofprint, acorn: `Object::kind_counters`).
    KindCounterSelf(i32),
    /// "You may sacrifice a token. If you do, ...": the rest only if one is sacrificed.
    MaySacrificeToken,
    /// Create a token that's a copy of the creature the source is attached to, except it isn't
    /// legendary; it has haste (Helm of the Host).
    CopyAttachedNonlegendaryHaste,
    /// Tempting offer (Tempt with Discovery), after your first land: each opponent may search for a
    /// land card and put it onto the battlefield; remembered for `SearchLandIfTempted`.
    TemptOpponentsLand,
    /// For each opponent who searched this way, search for a land card and put it onto the battlefield.
    SearchLandIfTempted,
    /// Cascade (CR 702.85) for the spell in the trigger's `extra`: exile from the top until a nonland
    /// card with lesser mana value; you may cast it free; the rest go to the bottom in a random order.
    Cascade,
    /// Permanents you control gain these keywords until end of turn (Heroic Intervention).
    GrantAllPermanentsYouControl(Keywords),
    /// Destroy the permanent in `slot`; its controller may search for a basic land card and put it
    /// onto the battlefield (untapped) (Assassin's Trophy).
    DestroyControllerMayFetchBasic(u8),
    /// Return each nonland permanent you don't control to its owner's hand (overloaded Cyclonic
    /// Rift).
    BounceOpponentsNonland,
    /// Put a +1/+1 counter on each creature you control with this subtype.
    CountersOnYourSubtype(&'static str),
    /// Return all land cards from your graveyard to the battlefield tapped.
    ReturnLandsFromGraveyardTapped,
    /// Scute Swarm: a token (`def`), or with six or more lands a token copy of the source.
    InsectOrCopyWithSixLands {
        def: usize,
    },
    /// The creature in `slot` gets +X/+X and gains keywords until end of turn, X its power (Xenagos).
    PumpByOwnPowerAndGrant {
        slot: u8,
        keywords: Keywords,
    },
    /// Reveal the top card of your library: a creature card goes to your hand (and the rest of the
    /// effects are skipped); otherwise you may go on (Nylea, Keen-Eyed: then mill it).
    RevealTopCreatureToHandElseMay,
    /// The source deals the trigger's amount of damage to each creature the damaged opponent controls
    /// (Balefire Dragon).
    DamageAmountToEachOpponentCreature,
    /// The source deals the trigger's amount of damage to the target in `slot` (Questing Beast).
    DamageAmountTo(u8),
    /// Shuffle the source from its owner's graveyard into their library (Kogla and Yidaro).
    ShuffleSourceFromGraveyardIntoLibrary,
    /// Destroy each artifact your opponents control (Vandalblast, overloaded).
    DestroyOpponentsArtifacts,
    /// The creature in `slot` gets +X/+0 and gains keywords until end of turn, X the ability's X
    /// (Kessig Wolf Run).
    PumpXAndGrant {
        slot: u8,
        keywords: Keywords,
    },
    /// "After this phase, there is an additional combat phase" (CR 500.8; no main phase between).
    AdditionalCombatAfterThisPhase,
    /// Untap each attacking creature; they gain these keywords until end of turn (Karlach).
    UntapAttackersAndGrant(Keywords),
    /// Exert the source (CR 701.39: it won't untap during your next untap step) and untap each
    /// other creature you control (Combat Celebrant).
    ExertSelfUntapOthers,
    /// Gain control of each artifact the opponent controls, for good (Hellkite Tyrant).
    GainControlOfOpponentsArtifacts,
    /// Create a token (`def`, a 0/0) with +1/+1 counters equal to the trigger's amount (an X/X).
    CreateTokenWithCountersAmount {
        def: usize,
    },
    /// You may put a creature card from your hand onto the battlefield tapped and attacking; it
    /// returns to your hand at the beginning of the next end step (Ilharg).
    PutCreatureFromHandAttackingUntilEnd,
    /// Put the source into its owner's library third from the top (from a graveyard or exile).
    SourceThirdFromTop,
    /// Regenerate the source (CR 701.15): the next time it would be destroyed this turn, instead tap
    /// it, remove all damage from it and remove it from combat.
    RegenerateSelf,
    /// Regenerate the creature in `slot`.
    Regenerate(u8),
    /// Destroy the permanent in `slot`; it can't be regenerated.
    DestroyNoRegen(u8),
    /// Creatures you control get +P/+T and gain keywords and infect until end of turn (Triumph of the
    /// Hordes). Infect (CR 702.90): damage to creatures as -1/-1 counters, to players as poison.
    PumpAllGrantInfect {
        power: i32,
        toughness: i32,
        keywords: Keywords,
    },
    /// Tap or untap the permanent in `slot` (the controller's choice: it toggles).
    TapOrUntap(u8),
    /// Draw a card if the creature in `slot` doesn't have the same name as another creature you
    /// control or a creature card in your graveyard (Guardian Project).
    DrawIfUniqueName(u8),
    /// You may return a land card from your graveyard to the battlefield tapped.
    ReturnLandFromGraveyardTapped,
    /// Sacrifice a land (your choice); if you don't, sacrifice the source (The Gitrog Monster).
    SacrificeLandOrSource,
    /// Put one of the creature cards put into your graveyard from your library this turn onto the
    /// battlefield (Colossal Grave-Reaver).
    MilledCreatureToBattlefield,
    /// Reveal the top `n` cards: lands into your hand, the rest into your graveyard.
    RevealTopLandsToHandRestGraveyard(u8),
    /// Put any number of land cards from your hand onto the battlefield tapped.
    LandsFromHandTapped,
    /// Return all permanent cards from your graveyard to your hand.
    ReturnPermanentCardsToHand,
    /// The source must be blocked each combat this turn if able (Anzrag).
    MustBeBlockedThisTurnSelf,
    /// Eternalize (CR 702.129): create a token that's a copy of the source card (in exile), except
    /// it's a 4/4 black Zombie with no mana cost.
    EternalizeSelf,
    /// "There's an additional combat phase after this [main] phase. At the beginning of that combat,
    /// untap all creatures you control." (Moraug: no additional main phase follows it.)
    AdditionalCombatAfterMainUntap,
    /// "After the second main phase this turn, there's an additional combat phase followed by an
    /// additional main phase. At the beginning of that combat, untap all creatures that attacked this
    /// turn." (World at War)
    AdditionalCombatAfterSecondMainUntapAttackers,
    /// Rebound (CR 702.88): cast from hand, the spell is exiled as it resolves; at the beginning of
    /// your next upkeep, you may cast it from exile without paying its mana cost.
    Rebound,
    /// You may put a creature card with mana value at most the lands you control from your hand or
    /// graveyard onto the battlefield with two +1/+1 counters (Nissa of Shadowed Boughs).
    CreatureFromHandOrGraveyardMvAtMostLands,
    /// Scapeshift: sacrifice any number of lands, then search for up to that many land cards, put
    /// them onto the battlefield tapped, then shuffle.
    Scapeshift,
    /// `who` sacrifices a creature with the greatest power among creatures they control.
    SacrificeGreatestPower(Who),
    /// You sacrifice a creature; if you did, you may return another permanent card from your
    /// graveyard to the battlefield (Rise of the Witch-king).
    SacrificeThenReturnOther,
    /// Tap the permanent in `slot`; its activated abilities can't be activated this turn (Koma).
    TapStopActivations(u8),
    /// Look at the top `n` cards of your library, then put them back in any order.
    LookTopReorder(u8),
    /// Draw a card, then put the source on top of its owner's library (Sensei's Divining Top).
    DrawThenSourceOnTop,
    /// Transform the source (CR 701.28): a transforming double-faced permanent turns over to its other
    /// face. Nothing else changes; it's the same object.
    TransformSelf,
    /// Return the source from your graveyard to the battlefield tapped and transformed, under its
    /// owner's control (Ojer Kaslem).
    ReturnSelfTappedTransformed,
    /// Exile the source, then return it to the battlefield under its owner's control, transformed
    /// (`true`) or front face up (`false`) (Vorinclex, The Grand Evolution).
    ExileSelfReturn {
        transformed: bool,
    },
    /// Reveal the top N cards (N the trigger's amount, "that many"); you may put a creature card
    /// and/or a land card from among them onto the battlefield; the rest go on the bottom in a
    /// random order (Ojer Kaslem).
    RevealAmountPutCreatureAndLand,
    /// Mill `n`; put up to `up_to` creature cards from among the milled cards onto the battlefield
    /// (The Grand Evolution).
    MillPutCreatures {
        n: u8,
        up_to: u8,
    },
    /// Until end of turn, creatures you control gain "{1}: This creature fights target creature you
    /// don't control." (The Grand Evolution; see `Game::granted_fight`.)
    GrantFightAbilityEot,
    /// "You may choose new targets for" the spell or ability in `slot` (Deflecting Swat): each of its
    /// targets in turn, as `Decision::ChooseNewTarget`.
    ChangeTargets(u8),
    /// The source becomes an artifact creature until end of turn (crew, CR 702.122a).
    CrewedSelf,
    /// Put the land cards milled into your graveyard this turn onto the battlefield tapped (Hedge
    /// Shredder: "put them").
    MilledLandsToBattlefieldTapped,
    /// You get `n` {E} (energy counters, CR 122.1).
    Energy(u8),
    /// Return the source to its owner's hand at the beginning of the next end step (Sakashima the
    /// Impostor's kept ability).
    ReturnSelfToHandAtNextEnd,
    /// Zimone: the first time this ability resolves this turn, manifest dread (CR 701.62);
    /// otherwise you may turn a face-down permanent you control face up.
    ManifestDreadFirstElseTurnFaceUp,
    /// Look at the top card of the library of the player in `slot`; you may put it on the bottom.
    LookTopMayBottom(u8),
    /// Draw `draw` cards, then put `back` cards from your hand on top of your library in any order.
    DrawThenPutBack {
        draw: u8,
        back: u8,
    },
    /// Exile all cards from the library of the player in `slot`; then that player shuffles their
    /// hand into their library.
    ExileLibraryShuffleHandIn(u8),
    /// Scroll Rack: exile any number of cards from your hand face down, put that many cards from the
    /// top of your library into your hand, then put the exiled cards on top in any order.
    ScrollRack,
    /// Conduit of Worlds: if you haven't cast a spell this turn, you may cast the card in `slot` from
    /// your graveyard this turn; if you do, you can't cast additional spells this turn.
    ConduitCast(u8),
    /// Until end of turn, the creature in `slot` gains "Whenever this creature becomes tapped, ..."
    /// (the source's trigger number `trigger`; the tapped creature is the item's `extra`).
    GrantTappedTrigger {
        slot: u8,
        trigger: usize,
    },
    /// The creature in the item's `extra` deals damage equal to its power to the target in `slot`.
    ExtraDealsDamageEqualPower(u8),
    /// Counter the spell, activated ability or triggered ability in `slot` (Voidslime).
    CounterSpellOrAbility(u8),
    /// Each player's life total becomes the number of creatures they control (Biorhythm).
    LifeBecomesCreatureCount,
    /// Return all lands to their owners' hands (Sunder).
    BounceAllLands,
    /// Untap up to `n` lands (your own tapped lands, the engine's choice; Palinchron).
    UntapLands(u8),
    /// You may put a land card from your hand onto the battlefield (Broken Bond).
    MayLandFromHand,
    /// Return all permanents to their owners' hands except lands and those with these subtypes
    /// (Cyclone Summoner).
    BounceAllExceptLandsAndSubtypes(&'static [&'static str]),
    /// Gain control of the creature in `slot` for as long as you control the source (Roil Elemental).
    GainControlWhileSource(u8),
    /// You may search your library for a Forest card and put it onto the battlefield tapped under
    /// the control of the player in `slot`, then shuffle (Yavimaya Dryad).
    SearchForestForPlayer(u8),
    /// Awaken `n` on the land in `slot`: `n` +1/+1 counters, and it becomes a 0/0 Elemental creature
    /// with haste (CR 702.113). It's still a land.
    Awaken {
        slot: u8,
        n: u8,
    },
    /// The source becomes a copy of the card in `slot` until end of turn (Shifting Woodland).
    BecomeCopyOfSlotUntilEot(u8),
    /// The permanent in `slot` becomes a `power`/`toughness` creature with `subtype` for good. It's
    /// still a land (Nissa, Sage Animist's −7).
    SlotBecomesCreatureForGood {
        slot: u8,
        power: i32,
        toughness: i32,
        subtype: &'static str,
    },
    /// Soulbond, as the source enters: you may pair it with another unpaired creature you control.
    SoulbondPairOnEnter,
    /// Soulbond, as another creature enters: pair the source with the creature in `slot`, if both
    /// are unpaired.
    SoulbondPairWith(u8),
    /// Untap up to `n` lands you control (Frantic Search): tapped ones, those making the most colors
    /// first.
    UntapLandsUpTo(u8),
    /// "You may put a land card from your hand onto the battlefield tapped." (Zimone, Quandrix
    /// Prodigy)
    MayPutLandFromHandTapped,
    /// Draw `n`, or `more` if you control `lands` or more lands.
    DrawMoreIfLands {
        n: u8,
        more: u8,
        lands: u8,
    },
    /// Search for up to X basic land cards, X the creatures attacking you, put them onto the
    /// battlefield tapped (Jaheira's Respite).
    SearchBasicsPerAttackersTapped,
    /// "... unless that player pays {n}": the opponent (the spell's caster) may pay {n} to stop the
    /// rest of the effects (Rhystic Study, Mystic Remora).
    UnlessOpponentPays(u8),
    /// Cumulative upkeep {n} (CR 702.24): an age counter, then pay {n} for each or sacrifice it.
    CumulativeUpkeep(u8),
    /// At the beginning of the next turn's upkeep, the controller of the spell in `slot` draws
    /// `theirs` and you draw `yours` (Arcane Denial; read before the spell is countered).
    NextUpkeepDraws {
        slot: u8,
        theirs: u8,
        yours: u8,
    },
    /// Unearth (CR 702.84): return the source from your graveyard to the battlefield with haste;
    /// exile it at the beginning of the next end step or if it would leave the battlefield.
    Unearth,
    /// The player in `slot` takes an extra turn after this one.
    ExtraTurnFor(u8),
    /// Nissa's +1: three +1/+1 counters on the land in `slot`, untap it, and it becomes a 0/0
    /// Elemental creature with vigilance and haste that's still a land (permanently).
    AnimateLandElemental(u8),
    /// Distribute X +1/+1 counters (the stack entry's X) among creatures you control, one at a time
    /// (`Decision::DistributeCounters`; not targeted).
    DistributeCountersX,
    /// Draw a card for each creature you control with power at least this.
    DrawPerCreaturePowerAtLeast(i32),
    /// Destroy all artifacts and enchantments; a +1/+1 counter on the source for each one destroyed
    /// (Bane of Progress).
    DestroyArtifactsEnchantmentsCountersSelf,
    /// The creature in `slot`: draw a card if its power is 3 or greater, else two +1/+1 counters on it
    /// (Tribute to the World Tree).
    DrawIfPowerThreeElseTwoCounters(u8),
    /// `who` sacrifices `n` permanents of their choice (annihilator, CR 702.86).
    SacrificePermanents {
        who: Who,
        n: u8,
    },
    /// Annihilator X, X the +1/+1 counters on the source (Ulamog, the Defiler).
    AnnihilatorPerCounters,
    /// The source's owner shuffles their graveyard into their library (Ulamog, the Infinite Gyre).
    OwnerShufflesGraveyardIntoLibrary,
    /// Reveal the top card of your library: a creature card goes onto the battlefield; otherwise
    /// you may put it on the bottom (Lurking Predators).
    RevealTopCreatureToBattlefieldElseMayBottom,
    /// Each player gains control of all creatures they own (Homeward Path).
    CreaturesToOwners,
    /// The player in `slot` exiles the top half of their library, rounded up (Ulamog, the Defiler).
    ExileTopHalfOfLibrary(u8),
    /// You create a Treasure for each artifact the opponent controls (Cavern-Hoard Dragon's combat
    /// damage trigger; one opponent).
    TreasuresPerOpponentArtifact {
        def: usize,
    },
    /// Each player exiles cards from the top of their library until they exile a nonland card; you
    /// may cast any number of those nonland cards without paying their mana costs (Etali).
    EachExileUntilNonlandCastFree,
    /// The damaged player (the opponent) gets poison counters equal to the trigger's amount.
    PoisonAmountToOpponent,
    /// You may put a creature card from your hand onto the battlefield; it gains haste, and you
    /// sacrifice it at the beginning of the next end step (Sneak Attack).
    SneakAttack,
    /// Add X mana in any combination of {R} and/or {G}, X the creatures you control (Xenagos).
    AddRedOrGreenPerCreature,
    /// Exile the top `n` cards; put any number of creature and/or land cards from among them onto
    /// the battlefield (Xenagos, the Reveler's −6).
    ExileTopPutCreaturesAndLands(u8),
    /// Put any number of creature cards from your hand onto the battlefield (Ghalta).
    CreaturesFromHand,
    /// Prevent all combat damage that would be dealt to and dealt by the creature in `slot` this
    /// turn (Maze of Ith).
    PreventCombatDamageToAndBy(u8),
    /// You may return another creature you control to its owner's hand; if you do, the source gains
    /// indestructible until end of turn (Temur Sabertooth).
    MayReturnAnotherThenIndestructible,
    /// You may put a land card from your hand onto the battlefield tapped; if you do, draw a card
    /// and repeat (Cultivator Colossus).
    LandFromHandTappedDrawRepeat,
    /// You may put a basic land card from your hand onto the battlefield tapped (Terrain Generator).
    BasicLandFromHandTapped,
    /// Create `n` token copies of the creature in `slot`, with haste; sacrifice them at the
    /// beginning of the next end step (Orthion).
    CopiesHasteSacrifice {
        slot: u8,
        n: u8,
    },
    /// Sarkhan's Unsealing: 4 damage to the target in `slot` (a cast creature spell with power 4, 5
    /// or 6), or with power 7 or more, 4 damage to each opponent and each creature and planeswalker
    /// they control. The cast spell is the item's `extra`.
    UnsealingDamage(u8),
    /// Create `n` token copies of the creature in `slot` with haste; exile them at the beginning of
    /// the next end step (`n` 0: one plus the instant and sorcery spells you've cast this turn,
    /// Rionya).
    CopiesHasteExileAtEnd {
        slot: u8,
        n: u8,
    },
    /// Molten Echoes: a hasty copy, exiled at end step, of the creature in `slot` if it has the
    /// source's chosen creature type.
    CopyIfChosenTypeExileAtEnd(u8),
    /// Mirror March: flip a coin until you lose a flip; a hasty copy (exiled at end step) of the
    /// creature in `slot` for each flip won.
    CoinFlipCopiesExileAtEnd(u8),
    /// You may put a commander you own from the command zone onto the battlefield; it gains haste,
    /// and returns to the command zone at the beginning of the next end step (Hellkite Courser).
    CommanderFromCommandZoneUntilEnd,
    /// Search for any number of creature cards with total mana value `n` or less, onto the
    /// battlefield (Protean Hulk).
    SearchCreaturesTotalManaValue(u32),
    /// The creature in `slot`'s controller draws a card if its power is greater than each other
    /// creature's power (Selvala).
    DrawIfGreatestPower(u8),
    /// Add X mana in any combination of colors, X the greatest power among creatures you control.
    AddAnyColorsGreatestPower,
    /// Primal Surge: exile the top card; if it's a permanent card, you may put it onto the
    /// battlefield, and if you do, repeat.
    PrimalSurge,
    /// A -1/-1 counter on the creature in `slot` (kept as negative `Object::counters`, so +1/+1 and
    /// -1/-1 counters cancel out, CR 704.5q).
    MinusCounter(u8),
    /// Proliferate (CR 701.27), choosing for you: your permanents with +1/+1 counters and your
    /// planeswalkers, and opponents' permanents with -1/-1 counters.
    Proliferate,
    /// Myrkul: you may exile the card in `slot` (the creature that died); if you do, create a token
    /// that's a copy of it, except it's an enchantment and loses its other card types.
    ExileMakeEnchantmentCopy(u8),
    /// "... unless that player pays {X}, where X is this creature's power" (Esper Sentinel).
    UnlessOpponentPaysPower,
    /// Destroy all creatures; they can't be regenerated (Damnation, Wrath of God).
    DestroyAllCreaturesNoRegen,
    /// Exalted (CR 702.83), from a "whenever you attack" trigger: if exactly one creature you
    /// control is attacking, it gets +1/+1 until end of turn.
    Exalted,
    /// Undying (CR 702.93), from the source's dies trigger: if it had no +1/+1 counters on it,
    /// return it to the battlefield under its owner's control with a +1/+1 counter on it.
    Undying,
    /// Persist (CR 702.79), from the source's dies trigger: if it had no -1/-1 counters on it,
    /// return it to the battlefield under its owner's control with a -1/-1 counter on it.
    Persist,
    /// Each opponent loses X life, X your devotion to these colors (WUBRG); you gain life equal to
    /// the life lost this way (Gray Merchant of Asphodel).
    DrainDevotion([bool; 5]),
    /// Add mana of this color equal to your devotion to it (Nykthos, Shrine to Nyx).
    AddManaDevotion(crate::Color),
    /// Add one mana of this color for each permanent `count` counts (Cabal Coffers: each Swamp you
    /// control).
    AddManaPerCount {
        color: crate::Color,
        count: ManaCount,
    },
    /// Add mana of this color equal to the source's power (Heronblade Elite: X mana of one color).
    AddManaSourcePower(crate::Color),
    /// Echo {cost} (CR 702.30), from an upkeep trigger: if the source came under your control since
    /// your last upkeep, sacrifice it unless you pay the cost (`Decision::PayUpkeep`). Two players:
    /// your last upkeep was two turns ago (extra turns aren't tracked).
    Echo(crate::mana::ManaCost),
    /// The opponent draws `n` cards if this spell was cast for its alternative cost (Baleful Mastery).
    OpponentDrawsIfAltCost(u8),
    /// Tap all lands the opponent controls (Nature's Will: "that player", one opponent).
    TapOpponentsLands,
    /// Add one mana of this color for each card in the hand of the player in `slot` (Jeska's Will).
    AddManaPerHandSize {
        color: crate::Color,
        slot: u8,
    },
    /// Exile the top `n` cards of your library; you may play them this turn (Jeska's Will).
    ExileTopToPlayThisTurn(u8),
    /// The source deals damage equal to the spells you've cast this turn to the target in `slot`
    /// (Noise Marine).
    DamagePerSpellsCastThisTurn(u8),
    /// Hideaway `n` (CR 702.75): look at the top `n` cards, exile one face down (linked to the
    /// source), and put the rest on the bottom in a random order.
    Hideaway(u8),
    /// You may play the source's hideaway card without paying its mana cost this turn, if `cond`
    /// holds: 0, creatures you control have total power 10 or greater (Mosswort Bridge); 1, an
    /// opponent was dealt 7 or more damage this turn (Spinerock Knoll).
    PlayHideaway(u8),
    /// Wand of Wonder: roll a d20 (X: 1 for 1–9, 2 for 10–19, 3 for 20); each opponent exiles
    /// cards from the top until an instant or sorcery card, then shuffles the rest in; you may cast
    /// up to X of those instants and sorceries free.
    WandOfWonder,
    /// Create the token `def`, then choose a reach, vigilance or trample counter for it (Vivien).
    CreateTokenWithKeywordCounterChoice {
        def: usize,
    },
    /// When you next cast a creature spell this turn, search for a creature card with lesser mana
    /// value and put it onto the battlefield (the source's trigger number `trigger`).
    NextCreatureSpellThisTurn(usize),
    /// Search for a creature card with lesser mana value than the item's `extra` (a cast spell),
    /// onto the battlefield.
    SearchCreatureLesserManaValue,
    /// Delina: roll a d20 and create a tapped and attacking, nonlegendary copy of the creature in
    /// `slot`, exiled at end of combat; on 15 or more, you may roll again.
    DelinaRoll(u8),
    /// Create a hasty token copy of the creature in `slot` with "When this token dies, draw a card"
    /// (the source's trigger number `trigger`); sacrifice it at the next end step (Jaxis).
    CopyHasteDiesDrawSacrifice {
        slot: u8,
        trigger: usize,
    },
    /// Put a keyword counter on the source (Zopandrel: indestructible).
    KeywordCounterSelf(Keywords),
    /// Mirage Phalanx: while paired, a hasty token copy of it and of its partner, without soulbond,
    /// exiled at end of combat.
    PhalanxCopies,
    /// For each creature your opponents control, create a token (`def`) that fights a different one
    /// of those creatures (Ezuri's Predation).
    TokenFightsEachOpponentCreature {
        def: usize,
    },
    /// The player in `slot` mills cards equal to the stack item's amount (Altar of Dementia: the
    /// sacrificed creature's power, with `Activated::amount_from_victim_power`).
    MillAmount(u8),
    /// Destroy all artifacts (Creeping Corrosion).
    DestroyAllArtifacts,
    /// Destroy all lands (Armageddon).
    DestroyAllLands,
    /// The player in `slot` draws X cards and loses X life, X the spell's X (Damnable Pact).
    TargetDrawsXLosesX(u8),
    /// Exile all permanents of one type: 0 artifacts, 1 creatures, 2 enchantments, 3 planeswalkers
    /// (Merciless Eviction).
    ExileAllOfType(u8),
    /// Untap all nonland permanents you control (Dramatic Reversal).
    UntapNonlandYouControl,
    /// You gain 1 life for each card in your hand (Venser's Journal).
    GainLifePerCardInHand,
    /// You gain X life and draw X cards, X the lands you control (Nissa, Voice of Zendikar).
    GainAndDrawPerLandYouControl,
    /// "You may exile it. If you do, ...": exile the source from its owner's graveyard; if it isn't
    /// there, the rest of the effects don't happen (Academy Rector).
    ExileSourceFromGraveyardOrStop,
    /// Destroy every artifact, enchantment, and/or creature with mana value in `creature_mv`
    /// (inclusive; (0, 0) for none) (Austere Command's modes).
    DestroyAllMatching {
        artifacts: bool,
        enchantments: bool,
        creature_mv: (u8, u8),
    },
    /// You gain hexproof until end of turn (Dawn's Truce).
    YouGainHexproofEot,
    /// Search for a creature card with mana value at most (or exactly, with `exact`) `plus` more
    /// than the sacrificed creature's (the stack entry's `extra`), put it onto the battlefield
    /// (Eldritch Evolution, Birthing Pod).
    SearchCreatureBySacrificedMv {
        plus: u8,
        exact: bool,
    },
    /// If the source was a creature when it died, return it to the battlefield as an enchantment and
    /// no other card type (Enduring Vitality).
    ReturnSelfAsEnchantment,
    /// Create one `def` token for each creature attacking you (Arachnogenesis).
    CreateTokensPerAttackers {
        def: usize,
    },
    /// Prevent all combat damage this turn by creatures without this subtype.
    PreventCombatDamageExceptSubtype(&'static str),
    /// Create X tokens that are copies of the token in `slot` (X the spell's X).
    CopyTokenX(u8),
    /// Tokens you control gain these keywords until your next turn.
    GrantTokensUntilYourNextTurn(crate::Keywords),
    /// You gain 1 life for each token you control.
    GainLifePerToken,
    /// Until end of turn, tokens created under your control are doubled (Kaya's −2).
    DoubleTokensThisTurn,
    /// Exile all cards from all graveyards, then create a `def` token for each (Kaya's −6).
    ExileGraveyardsTokenPerCard {
        def: usize,
    },
    /// Investigate `n` times (CR 701.36): create `n` Clue tokens (`def`).
    Investigate {
        def: usize,
        n: u8,
    },
    /// The creature in `slot` gets +1/+1 until end of turn for each permanent you control with this
    /// subtype (Confront the Unknown: Clue).
    PumpPerYourSubtype {
        slot: u8,
        sub: &'static str,
    },
    /// Mill `n`, then put a creature, enchantment or land card from among them onto the battlefield
    /// (Fetch Quest).
    MillPutCreatureEnchantmentOrLand(u8),
    /// Mill `n`, then return a creature card milled this way to your hand (Zoological Study).
    MillReturnCreatureToHand(u8),
    /// You may put a creature card from your hand onto the battlefield (Monster Manual).
    MayPutCreatureFromHand,
    /// Counter the spell in `slot`; if it's countered this way, exile it instead of putting it into
    /// its owner's graveyard (Force of Negation).
    CounterExile(u8),
    /// Double the number of each kind of counter on the permanent in `slot` (Deepglow Skate).
    DoubleEachCounter(u8),
    /// Until end of turn, the source becomes a creature of this type with this base power and
    /// toughness and these keywords (Bogardan Dragonheart: a 4/4 Dragon, flying and haste).
    BecomeSubtypeUntilEot {
        subtype: &'static str,
        power: i32,
        toughness: i32,
        keywords: crate::Keywords,
    },
    /// Return the creature card in `slot` from your graveyard to the battlefield; it gains haste,
    /// is exiled at the beginning of the next end step, and is exiled if it would leave the
    /// battlefield (Whip of Erebos: as unearth, `Object::unearthed`).
    ReanimateUnearthed(u8),
    /// The next time you would draw a card this turn, the source deals `n` damage to the target in
    /// `slot` instead (Words of War; the target is chosen as the ability is activated).
    NextDrawDealsDamage {
        slot: u8,
        n: i32,
    },
    /// Choose an exiled card an opponent owns with a void counter on it; you may play it this turn
    /// without paying its mana cost (Dauthi Voidwalker).
    PlayVoidCardFree,
    /// Put the creature card in `slot` (from a graveyard) onto the battlefield under your control
    /// and attach the source to it (Necromancy).
    ReanimateAttachSource(u8),
    /// All creatures get -X/-X until end of turn, X the spell's X (Toxic Deluge).
    AllCreaturesGetMinusX,
    /// Draw a card for each creature you control of this color (Freyalise, Regal Force: green).
    DrawPerCreatureYouControlOfColor(crate::Color),
    /// Evoke's "When this permanent enters, if its evoke cost was paid, its controller sacrifices
    /// it." (CR 702.74a)
    SacrificeIfEvoked,
    /// Untap the creature the source is attached to.
    UntapAttached,
    /// You gain life and draw cards equal to the source's power, its last known power if it left
    /// the battlefield (Lifeblood Hydra: "When this creature dies, ...").
    GainLifeAndDrawSourcePower,
    /// You gain 1 life for each spell you've cast this turn (Aetherflux Reservoir).
    GainLifePerSpellCastThisTurn,
    /// You gain half X life and draw half X cards, each rounded down, X the source spell's X (Hydroid
    /// Krasis: "When you cast this spell, ...").
    GainAndDrawHalfX,
    /// You lose life equal to the mana value of the card or permanent in `slot` (Reanimate, Feed the
    /// Swarm). Put it before the effect that moves that object, which keeps the slot legal.
    LoseLifeSlotManaValue(u8),
    /// Put the top card of your library into your hand (revealed); you lose life equal to its mana
    /// value (Darkstar Augur, Dark Confidant).
    RevealTopToHandLoseManaValue,
    /// You lose life equal to the number of cards in your hand (Castle Locthwain).
    LoseLifePerCardInHand,
    /// `who` draws X cards and loses X life, X the cards in their graveyard as this resolves
    /// (Eldritch Pact).
    DrawAndLosePerGraveyardCard(Who),
    /// Each player discards their hand, then draws that many cards minus one (Dark Deal).
    DiscardHandsDrawThatManyMinusOne,
    /// You draw X cards and lose X life, X the permanents you control with this subtype (Graveborn
    /// Muse: Zombies).
    DrawAndLosePerSubtypeYouControl(&'static str),
    /// "Sacrifice another creature. You gain X life and draw X cards, where X is that creature's
    /// power." (Disciple of Bolas; nothing if you control no other creature)
    SacrificeAnotherGainAndDrawPower,
    /// "Sacrifice it unless you return a non-[subtype] land you control to its owner's hand." (the
    /// Lairs: Darigaaz's Caldera)
    ReturnNonSubtypeLandOrSacrificeSelf(&'static str),
    /// `who` loses life equal to the trigger's amount ("target opponent loses that much life":
    /// Sanguine Bond, with `When::YouGainLife`).
    LoseLifeAmount(Who),
    /// `who` loses life equal to the life they've lost this turn (Wound Reflection).
    LoseLifeLostThisTurn(Who),
    /// Regenerate each creature you control (Loxodon Hierarch).
    RegenerateEachCreatureYouControl,
    /// Regenerate the creature the source (an Aura) is attached to (Blessing of Leeches).
    RegenerateAttached,
    /// `who` loses 1 life for each artifact they control (Emissary of Despair: the damaged player).
    LoseLifePerArtifactTheyControl(Who),
    /// Draw cards equal to the greatest power among non-Human creatures you control (Return of the
    /// Wildspeaker).
    DrawGreatestPowerNonHuman,
    /// Non-Human creatures you control get +P/+T until end of turn (Return of the Wildspeaker).
    PumpNonHumanYouControl {
        power: i32,
        toughness: i32,
    },
    /// Draw cards equal to the greatest toughness among creatures you control (Last March of the
    /// Ents).
    DrawGreatestToughnessYouControl,
    /// Put up to `n` creature cards from your hand onto the battlefield, as
    /// `Decision::CreaturesFromHandUpTo` (Tooth and Nail).
    CreaturesFromHandUpTo(u8),
    /// "You may shuffle": `Decision::MayShuffle` (Ponder).
    MayShuffle,
    /// Return a permanent you control to its owner's hand (only a nonland one with `nonland`), as
    /// `Decision::ReturnOwnPermanent` (Dream Stalker, Ancestral Statue).
    ReturnPermanentYouControl {
        nonland: bool,
    },
    /// `who` may search their library for up to X creature cards (X the spell's X), reveal them, put
    /// them into their hand, then shuffle (Weird Harvest; one effect per player).
    SearchCreaturesUpToX(Who),
    /// Untap all creatures that attacked this turn (Relentless Assault).
    UntapCreaturesAttackedThisTurn,
    /// Each noncreature artifact you control becomes a P/T artifact creature until end of turn
    /// (Cyberdrive Awakener: 4/4).
    NoncreatureArtifactsBecomeCreatures {
        power: i32,
        toughness: i32,
    },
    /// Search your library for a card, put it into your graveyard, then shuffle; if it's an instant
    /// or sorcery card, put a +1/+1 counter on the source (Oriq Loremage), as
    /// `Decision::SearchToGraveyardCounter`.
    SearchToGraveyardCounterIfInstantOrSorcery,
    /// Counter the spell in `slot` if it's this color, WUBRG index (Pyroblast: "if it's blue").
    CounterIfColor {
        slot: u8,
        color: u8,
    },
    /// Destroy the permanent in `slot` if it's this color (Pyroblast).
    DestroyIfColor {
        slot: u8,
        color: u8,
    },
    /// Creatures you control gain these keywords and get +X/+X until end of turn, X the greatest
    /// power among creatures you control as it resolves (Pathbreaker Ibex).
    PumpAllGreatestPowerGrant(Keywords),
    /// All creatures get -1/-1 until end of turn for each Swamp you control (Mutilate).
    AllCreaturesMinusPerSwampYouControl,
    /// The creature in `slot` gets -1/-1 until end of turn for each Swamp you control (Defile).
    MinusPerSwampYouControl(u8),
    /// Destroy all artifacts; they can't be regenerated; the controller of each gains life equal to
    /// its mana value (Seeds of Innocence).
    DestroyAllArtifactsNoRegenGainManaValue,
    /// Creatures you control with this subtype gain these keywords until end of turn (Jirina:
    /// Humans, hexproof and indestructible).
    GrantSubtypeYouControlEot {
        sub: &'static str,
        keywords: Keywords,
    },
    /// Tap or untap the creature the source (an Equipment) is attached to; it toggles (Sword of the
    /// Paruns).
    TapOrUntapAttached,
    /// Sacrifice the source, if it's still on the battlefield (Defense of the Heart).
    SacrificeSource,
    /// Stop here unless the ability's `condition` holds as it resolves: an intervening "if" on a
    /// spell (Gift of Estates), where conditions are otherwise checked only for triggers.
    StopUnlessCondition,
    /// Destroy the permanent in `slot`; its controller may search for a land card with a basic land
    /// type and put it onto the battlefield untapped (Boseiju, Who Endures).
    DestroyControllerMayFetchLandWithBasicType(u8),
    /// Destroy the permanent in `slot`; the source deals `n` damage to its controller (Cindervines).
    DestroyDamageController {
        slot: u8,
        n: i32,
    },
    /// Change the target of the single-target spell in `slot` to another legal one, if there is one
    /// (Imp's Mischief), as `Decision::ChangeTheTarget`.
    ChangeTheTarget(u8),
    /// You lose life equal to the mana value (X included) of the spell in `slot` (Imp's Mischief).
    LoseLifeSlotSpellManaValue(u8),
    /// Change a target of the spell or ability in `slot` to the source, if the source is a legal
    /// target for it (Spellskite); with a choice of targets, as `Decision::TargetToSource`.
    ChangeATargetToSource(u8),
    /// Each player chooses an artifact, a creature, an enchantment and a planeswalker from among
    /// the nonland permanents they control, then sacrifices the rest (Cataclysmic Gearhulk), as
    /// `Decision::KeepNonlandOfType`.
    EachKeepsOneNonlandOfEachType,
    /// `who` puts the cards in their hand on the bottom of their library in any order, then draws
    /// that many (Teferi's Puzzle Box), as `Decision::HandToBottom`.
    HandToBottomDrawThatMany(Who),
    /// Sacrifice any number of other permanents, then draw that many cards (God-Eternal Bontu), as
    /// `Decision::SacrificeAnyNumber`.
    SacrificeOthersDrawThatMany,
    /// Each player sacrifices a creature or planeswalker of their choice; if you sacrificed one, you
    /// may return another permanent card from your graveyard to your hand (Deadly Brew).
    EachSacrificeCreatureOrPlaneswalkerMayReturnOther,
    /// Each player sacrifices all artifacts, enchantments and nonbasic lands they control; for each
    /// land sacrificed, its controller may search for a basic land card and put it onto the
    /// battlefield tapped (Wave of Vitriol), as `Decision::BasicsTappedUpTo`.
    WaveOfVitriol,
    /// Search for up to three creature cards and reveal them; if they're three with different names,
    /// one at random goes to your hand; then shuffle (Signal the Clans), as
    /// `Decision::SignalTheClans`.
    SignalTheClans,
    /// You may sacrifice an artifact, creature, enchantment, land or planeswalker; if you do, the
    /// opponent may sacrifice a permanent that shares a card type with it, and if they don't, they
    /// lose 2 life and you draw a card (Braids, Arisen Nightmare).
    BraidsEndStep,
    /// Each noncreature artifact you control becomes a 0/0 artifact creature for good, with `n`
    /// +1/+1 counters (Rise and Shine, overloaded).
    NoncreatureArtifactsBecomeZeroZeroWithCounters(u8),
    /// Exile each creature you don't control; the opponent searches for that many basic land cards
    /// and puts them onto the battlefield tapped, then shuffles (Winds of Abandon, overloaded).
    ExileOpponentsCreaturesTheyFetchBasicsTapped,
    /// Exile the top card of your library; you may put it into your hand unless it has the same
    /// name as another card exiled this way; repeat until you put one into your hand or exile two
    /// with the same name (Tainted Pact), as `Decision::TaintedPact`.
    TaintedPact,
    /// Exile from the top until a nonland card and put it into your hand; repeat while its mana
    /// value is 4 or greater; the source deals 1 damage to you per card put into your hand
    /// (Demonlord Belzenlok).
    BelzenlokDig,
    /// The creature in `slot` (the source, with `None`) gains protection from the color of your
    /// choice until end of turn, as `Decision::ProtectionColor` (Benevolent Bodyguard, Cartel
    /// Aristocrat).
    ProtectionChosenColorEot(Option<u8>),
    /// Creatures you control gain protection from each color until end of turn (Akroma's Will).
    YourCreaturesProtectionEachColorEot,
    /// Remove the creature in `slot` from combat and untap it (Reconnaissance).
    RemoveFromCombatAndUntap(u8),
    /// The creature in `slot` connives X, X the number of attacking creatures: draw X, discard X,
    /// then a +1/+1 counter on it per nonland card discarded (Raffine), as
    /// `Decision::ConniveDiscard`.
    ConniveAttackers(u8),
    /// Draw cards until you have `n` in hand (Damia: "draw cards equal to the difference").
    DrawUpToHandSize(u8),
    /// Take an extra turn after this one; skip the untap step of that turn (Savor the Moment).
    ExtraTurnSkipUntap,
    /// Return a creature of this color (WUBRG index) you control to its owner's hand (Trusted
    /// Advisor).
    ReturnCreatureOfColorYouControl(u8),
    /// "You may put a land card from your hand onto the battlefield." (untapped: Sakura-Tribe Scout)
    MayPutLandFromHand,
    /// Until end of turn, lands you control become `power`/`toughness` creatures that are still lands
    /// (Rude Awakening).
    LandsBecomeCreaturesEot {
        power: i32,
        toughness: i32,
    },
    /// The player in `slot` draws X cards (X the spell's X) (Stroke of Genius).
    DrawXTarget(u8),
    /// The d4 subsystems' effects (dungeons, the initiative, dice, emblems for a player).
    Subsystem(SubsystemEffect),
    /// A manland: the source becomes a creature until end of turn, as `BecomeCreatureUntilEot`
    /// (base power and toughness, keywords, the first of `subtypes`), and also has the other
    /// `subtypes`, these `colors` (`None`: its own), is an artifact too if `artifact`, and has
    /// every creature type if `all_creature_types`. It's still a land. Kept in
    /// `Object::type_changes` (marked `own`, for `Condition::AnimatedByOwnAbility`).
    BecomeCreatureUntilEotWith {
        power: i32,
        toughness: i32,
        subtypes: &'static [&'static str],
        keywords: crate::Keywords,
        colors: Option<[bool; 5]>,
        artifact: bool,
        all_creature_types: bool,
    },
    /// "Until end of turn, creatures you control have base power and toughness X/X and gain all
    /// creature types." (Mirror Entity; X from the activation)
    YourCreaturesBaseXAllTypesEot,
    /// The permanent in `slot` loses all abilities, for as long as it stays on the battlefield
    /// (Abigale). Keyword counters put on it afterwards still count.
    LoseAllAbilities(u8),
    /// The source connives (CR 701.50): draw a card, then discard a card; if it was a nonland
    /// card, a +1/+1 counter on the source (Ledger Shredder), as `Decision::ConniveDiscard`.
    ConniveSelf,
    /// Return all creature cards from your graveyard to the battlefield; each is a base P/T
    /// `subtype` with `keywords` in addition to its other types, for as long as it stays (Storm of
    /// Souls: a 1/1 Spirit with flying).
    ReturnAllCreatureCardsAs {
        power: i32,
        toughness: i32,
        subtype: &'static str,
        keywords: crate::Keywords,
    },
    /// Each player, you first, puts a vow counter on a creature they control and sacrifices the
    /// rest (Promise of Loyalty), as `Decision::VowCreature`. The vows are kept in `Game::vows`
    /// (see `Game::vowed_against`).
    EachVowsOneSacrificesRest,
    /// The source deals the trigger's amount of damage to the player or object in `slot` (All
    /// Will Be One: "target opponent, creature an opponent controls, or planeswalker an opponent
    /// controls"). `DamageAmountTo` takes objects only.
    DamageAmountToAny(u8),
    /// `n` times: each opponent may discard a card; if they don't, they lose `life` life (Professor
    /// Onyx's −8: 7 times, 3 life), as `Decision::DiscardOrLoseLife`.
    OpponentDiscardsOrLosesLife {
        n: u8,
        life: i32,
    },
    /// The creature in `slot` gets +P/+T until your next turn (Fungi Cavern: −4/−0), kept in
    /// `Game::next_turn_pt`.
    PumpUntilYourNextTurn {
        slot: u8,
        power: i32,
        toughness: i32,
    },
    /// Each player, the active player first, loses `life` life unless they discard a card, or with
    /// `sacrifice`, unless they sacrifice an artifact, a creature or a land (Veils of Fear; Sandfall
    /// Cell), as `Decision::LoseUnless`.
    EachPlayerLosesUnless {
        life: i32,
        sacrifice: bool,
    },
    /// Sacrifice an artifact, a creature and a land (Oubliette, after its discard), as
    /// `Decision::SacrificeOfType`; one permanent counts once.
    SacrificeArtifactCreatureAndLand,
    /// Reveal the top `n` cards of your library; put a creature card from among them onto the
    /// battlefield with `counters` +1/+1 counters; it gains `keywords` until your next turn; then
    /// shuffle (Throne of the Dead Three: 10, 3, hexproof), as `Decision::RevealedCreatureOnto`.
    RevealTopCreatureOntoBattlefield {
        n: u8,
        counters: u8,
        keywords: crate::Keywords,
    },
    /// Phasing (CR 702.26): the permanents in the target slots (`all`: every permanent you control)
    /// phase out, with anything attached to them, until they phase in before their controller's next
    /// untap.
    PhaseOut {
        all: bool,
    },
    /// Until your next turn, your life total can't change (and damage to you is prevented: you have
    /// protection from everything) (Teferi's Protection).
    LifeLockedUntilYourNextTurn,
    /// "You may cast a spell with mana value `max_mv` or less from your hand without paying its mana
    /// cost." (Rishkar's Expertise): a free-cast window over those cards, for one spell.
    MayCastFromHandFree {
        max_mv: u32,
    },
    /// Each creature other than the source, anyone's, gets +P/+T until end of turn (Massacre Girl:
    /// −1/−1).
    PumpEachOtherCreature {
        power: i32,
        toughness: i32,
    },
    /// A −1/−1 counter on each creature `who` controls (Archfiend of Ifnir: each opponent).
    MinusCounterEach(Who),
    /// Each creature gets −X/−X until end of turn, X the source permanent's X as cast (The Meathook
    /// Massacre's enters trigger; `AllCreaturesGetMinusX` reads a spell's X).
    AllCreaturesGetMinusSourceX,
    /// Put X +1/+1 counters on the source, then draw half X cards rounded down, X the source's X as
    /// cast (Wan Shi Tong).
    CountersSelfXDrawHalfX,
    /// Sacrifice another permanent (not optional; nothing without one), as
    /// `Decision::SacrificeAnother` when there's a choice (Korvold).
    SacrificeAnotherPermanent,
    /// "You may pay N life. If you do, ...": the rest happens only if paid, offered only when your
    /// life total is at least N (CR 119.4), as `Decision::MayPayLife` (Savra).
    MayPayLife(i32),
    /// Untap all creatures, every player's (Benefactor's Draught).
    UntapAllCreatures,
    /// Put `counters` +1/+1 counters on the creature the source (an Aura) enchants; then that
    /// creature's controller gains `life` life and draws `cards` cards (Well Rested's granted ability).
    CountersAttachedGainDraw {
        counters: i32,
        life: i32,
        cards: u8,
    },
    /// `who` loses life equal to the source's power, last known if it left the battlefield
    /// (Rapacious Guest).
    LoseLifeSourcePower(Who),
    /// You gain life equal to the toughness of the creature in `slot` (Trostani).
    GainLifeSlotToughness(u8),
    /// Draw cards equal to the greatest power among creatures you control (Rishkar's Expertise).
    DrawGreatestPowerYouControl,
    /// Exile the graveyard card in `slot`. If it was a land card, add {R} or {G}; otherwise you gain
    /// `life` life and the source deals `damage` damage to each opponent (Klothys).
    ExileGraveyardCardManaIfLandElseDrain {
        slot: u8,
        life: i32,
        damage: i32,
    },
    /// Mill `n`; then you may pay {`mana`} and `life` life, and if you do, put a card from among the
    /// milled cards (those still in your graveyard) into your hand (Ripples of Undeath), as
    /// `Decision::MayPayManaAndLife` then `Decision::TakeMilledCard`.
    MillThenMayPayTakeOne {
        n: u8,
        mana: u8,
        life: i32,
    },
    /// You may return another permanent you control that shares a permanent type with the one in
    /// `slot` to its owner's hand (Cloudstone Curio), as `Decision::ReturnSharingType`.
    MayReturnAnotherSharingType(u8),
    /// The source deals `amount` damage to each creature each opponent controls, with `player` to
    /// each opponent too, and with `planeswalkers` to each planeswalker they control (Tectonic Hazard,
    /// End the Festivities, Smash to Dust).
    DamageEachOpponentAndTheirs {
        amount: i32,
        player: bool,
        planeswalkers: bool,
    },
    /// Each opponent loses `n` times X life; you gain life equal to the life lost (Debt to the
    /// Deathless: 2).
    DrainXTimes(u8),
    /// You gain life equal to the power of the creature in `slot` (Wall of Reverence).
    GainLifeSlotPower(u8),
    /// You gain 1 life for each colorless creature you control (Tomb of the Spirit Dragon).
    GainLifePerColorlessCreatureYouControl,
    /// Each player sacrifices all permanents they control that are one or more colors (All Is
    /// Dust).
    SacrificeAllColoredPermanents,
    /// The opponent (1v1: the defending player) exiles `n` permanents they control of their
    /// choice, as `Decision::ExileOwnPermanent` (Bane of Bala Ged: 2).
    OpponentExilesPermanents(u8),
    /// Put the creature card in graveyard `slot` onto the battlefield under your control with an
    /// extra creature type (Portal to Phyrexia: Phyrexian).
    ReanimateWithSubtype {
        slot: u8,
        subtype: &'static str,
    },
    /// Counter the spell or ability from the item's `extra` source (Kira's granted trigger).
    CounterExtra,
    /// All Suns' Dawn: for each color, return up to one card of that color from your graveyard to
    /// your hand.
    OneCardOfEachColorToHand,
    /// Chain of Smog: the player in `slot` may copy this spell, choosing a new target player.
    ChainCopy(u8),
    /// Exile the top `n` cards of your library; you may play them this turn. At the beginning of
    /// the next end step, those still exiled go to your graveyard and you create a `def` token for
    /// each (Glimpse the Impossible); see `Game::exiled_until_end`.
    ExileTopToPlayThenGraveyardTokens {
        n: u8,
        def: usize,
    },
    /// Populate (CR 701.31): create a token that's a copy of a creature token you control (your
    /// choice when there's more than one).
    Populate,
    /// You gain life equal to the toughness of the creature in `slot` (Trostani).
    GainLifeEqualSlotToughness(u8),
    /// Imprint (Isochron Scepter): you may exile an instant card with mana value 2 or less from your
    /// hand, linked to the source.
    ImprintInstantFromHand,
    /// You may cast a copy of the source's imprinted card without paying its mana cost.
    CastImprintedCopy,
    /// The creature in `slot` gets +X/-X until end of turn, X the ability's X (Chatterfang).
    PumpXMinusX(u8),
    /// "Gift an extra turn": the opponent takes an extra turn after this one (the gift was
    /// promised: cast kicked), and "whenever you give a gift" triggers (Perch Protection).
    GiftExtraTurn,
    /// The attacking (active) player draws `n` cards and gains control of the source; untap it
    /// (Coveted Jewel).
    AttackingPlayerDrawsAndTakesSource(u8),
    /// Remove all attacking creatures from combat and untap them; they can't attack the caster
    /// (the only opponent here) or their planeswalkers for the rest of the turn, so they don't
    /// attack in the additional combat (Illusionist's Gambit, with `AdditionalCombatAfterThisPhase`).
    RemoveAttackersFromCombatUntapNoAttack,
    /// Exile the graveyard card in `slot`, remembered as exiled with the source (Hazel's
    /// Brewmaster); see `Game::exiled_with`.
    ExileCardWithSource(u8),
    /// The creature in `slot` can't attack until your next turn (Twisted Caverns).
    CantAttackUntilYourNextTurn(u8),
    /// Goad the creature in `slot` until your next turn (CR 701.38; Arena).
    Goad(u8),
    /// Draw three cards and reveal them; you may cast one of them without paying its mana cost
    /// (Mad Wizard's Lair).
    DrawThreeMayCastOneFree,
    /// Flip a coin: if you win, gain control of the spell in `slot` and you may choose new targets
    /// for it; if you lose, counter it (Invert Polarity).
    FlipGainControlOfSpellOrCounter(u8),
    /// The opponent in target `slot` reveals the top X cards of their library (X the item's `x`);
    /// you may put a nonland permanent card with mana value X or less from among them onto the
    /// battlefield under your control, as `Decision::PutFromOpponentsTop`; they put the rest on the
    /// bottom in a random order (Lonis, Cryptozoologist).
    OpponentRevealsTopXPutNonlandPermanent(u8),
    /// Phantasmal Image's granted trigger (`StackKind::SacrificeWhenTargeted`): sacrifice the
    /// source, if it's still the same object.
    SacrificeSourceWhenTargeted,
    /// "If it's not a token, create a token that's a copy of it, except it's an artifact in
    /// addition to its other types." (Vaultborn Tyrant's dies trigger)
    TokenCopyOfSourceArtifactIfNontoken,
    /// Each player who controls an artifact or enchantment creates a token of definition `def`
    /// (Fade from History: a 2/2 green Bear), active player first.
    EachPlayerWithArtifactOrEnchantmentCreatesToken {
        def: usize,
    },
    /// Amass [subtype] `n` (CR 701.44): if you don't control an Army creature, create a 0/0 black
    /// [subtype] Army token (`def`); put `n` +1/+1 counters on an Army you control (the first one,
    /// when there are several); it becomes a [subtype] in addition to its other types.
    Amass {
        subtype: &'static str,
        n: u8,
        def: usize,
    },
    /// Return an instant or sorcery card at random from your graveyard to your hand (Exalted Flamer
    /// of Tzeentch).
    ReturnRandomInstantOrSorceryFromGraveyard,
    /// Look at the top `n` (3) cards of your library; put one into your hand, one on the bottom, and
    /// exile the rest; you may play the exiled card this turn (Expressive Iteration), as
    /// `Decision::IterationPick`.
    LookHandBottomExilePlay(u8),
    /// Choose an instant or sorcery card in your graveyard; you may cast it this turn, and it's exiled
    /// instead of going to the graveyard (Mission Briefing), as `Decision::CastFromGraveyardThisTurn`.
    ChooseGraveyardSpellToCastThisTurn,
    /// Exile each instant and sorcery card from your graveyard; copy each one, and you may cast the
    /// copies free (overloaded Mizzix's Mastery).
    ExileCastCopyFreeEachInstantSorcery,
    /// You gain `life`; if this is the first time this ability has resolved this turn, `counters`
    /// +1/+1 counters on the source (Teething Wurmlet: 1 and 1; counted per source, each turn).
    GainLifeCountersIfFirstResolutionThisTurn {
        life: i32,
        counters: i32,
    },
    /// Return a permanent card with an odd mana value from your graveyard to the battlefield
    /// (Desecrate Reality's adamant half), as `Decision::LookPick` with
    /// `Pick::PermanentOddManaValue`.
    ReturnOddManaValuePermanentToBattlefield,
    /// Tap all creatures (Ensnare).
    TapAllCreatures,
    /// Look at the top `n` cards of your library; put two of them into your hand and the rest on the
    /// bottom in any order (Dig Through Time: 7), as `Decision::TakeTwoBottomRest`.
    LookTakeTwoBottomAnyOrder(u8),
    /// X times: the opponent loses `life` life unless they sacrifice a nonland permanent or discard a
    /// card, their choice each time (Torment of Hailfire: 3), as `Decision::SacrificeDiscardOrLoseLife`.
    OpponentLosesUnlessSacrificesOrDiscardsX {
        life: i32,
    },
    /// Return up to one creature card with mana value `max_mv` or less from your graveyard to the
    /// battlefield, not targeted (Barrowin of Clan Undurr: 3), as `Decision::LookPick` with
    /// `Pick::CreatureManaValueAtMost`.
    ReturnCreatureMvAtMostToBattlefield(u32),
    /// "When [the creature in `slot`] is put into your graveyard this turn, ...": a delayed
    /// triggered ability (CR 603.7), the source's trigger number `trigger`, with that card in slot 0
    /// (Saffi Eriksdotter: return it to the battlefield). It triggers once, only for that object.
    DelayedWhenSlotToYourGraveyardThisTurn {
        slot: u8,
        trigger: usize,
    },
    /// The source deals `n` damage to each untapped creature (Calamity of Cinders).
    DamageEachUntappedCreature(i32),
    /// Destroy all creatures without this subtype (Realm-Cloaked Giant's Cast Off: Giant).
    DestroyAllCreaturesExceptSubtype(&'static str),
    /// The defending player exiles the top `n` cards of their library (Ulamog, the Ceaseless Hunger).
    DefendingPlayerExilesTop(u8),
    /// Exile the top X cards of your library; you may cast instant and sorcery spells with mana value
    /// X or less from among them free; the rest go to your graveyard (Epic Experiment).
    EpicExperiment,
    /// The source deals damage equal to the number of `subtype`s you control to the target in `slot`
    /// (Rolling Hamsphere: Hamsters).
    DamagePerYouControlSubtype {
        subtype: &'static str,
        slot: u8,
    },
    /// The source gets +X/+0 until end of turn, X the greatest mana value among artifacts you
    /// control (Pyreswipe Hawk).
    PumpSelfGreatestArtifactManaValue,
    /// If you're the attacking player, each other attacking creature gets +P/+T until end of turn
    /// (Martial Impetus: "each other creature that's attacking one of your opponents").
    PumpOtherAttackersIfYouAttack {
        power: i32,
        toughness: i32,
    },
    /// "Each player may draw a card, then each player who drew a card this way gains 1 life."
    /// (Kwain), as `Decision::MayDrawGainOne`, you first.
    EachPlayerMayDrawGainOne,
    /// Draw cards equal to the defending player's hand size minus yours; if none were drawn, you
    /// may put a creature card from your hand onto the battlefield (Mr. Foxglove).
    FoxgloveDraw,
    /// The source Class becomes this level (CR 716.2c), and "when this Class becomes level N"
    /// triggers.
    ClassLevelUp(u8),
    /// Return all creature cards with power `n` or less from your graveyard to your hand (Dawn).
    ReturnCreatureCardsPowerAtMostToHand(i32),
    /// Create a `def` token that's X/X, X the mana value of the spell that triggered this (the
    /// trigger's `extra`; Metallurgic Summonings' Construct).
    CreateTokenXxTriggeringSpellManaValue {
        def: usize,
    },
    /// Return all instant and sorcery cards from your graveyard to your hand (Metallurgic
    /// Summonings).
    ReturnAllInstantsAndSorceriesToHand,
    /// Mill `n`; you may put a permanent card from among them into your hand; then if you control
    /// a Squirrel or returned a Squirrel card, create a `food` token (Cache Grab), as
    /// `Decision::CacheGrabPick`.
    CacheGrab {
        n: u8,
        food: usize,
    },
    /// "You may move any number of +1/+1 counters from this creature onto other creatures." as
    /// `Decision::MoveCounterFrom`, one at a time (Forgotten Ancient).
    MayMoveCountersFromSource,
    /// Destroy the permanent in `slot`; its controller may search for a basic land card and put it
    /// onto the battlefield tapped (Sundering Eruption).
    DestroyControllerMayFetchBasicTapped(u8),
    /// "Creatures without flying can't block this turn." (Sundering Eruption): the creatures on
    /// the battlefield without flying as it resolves.
    CreaturesWithoutFlyingCantBlockThisTurn,
    /// Put the creature card in graveyard `slot` onto the battlefield under your control; it gains
    /// haste, and it's exiled at the beginning of the next end step (Puppeteer Clique).
    ReanimateHasteExileAtEnd(u8),
    /// Exile each permanent with mana value X or less that's one or more colors, X the ability's
    /// X (Ugin, the Spirit Dragon's −X).
    ExileEachColoredPermanentMvAtMostX,
    /// Put up to `n` permanent cards from your hand onto the battlefield, as
    /// `Decision::PermanentsFromHandUpTo` (Ugin, the Spirit Dragon's −10).
    PermanentsFromHandUpTo(u8),
    /// Put a shield counter on the creature in `slot`; you control it for as long as it has a
    /// shield counter (Shield Broker). A commander isn't affected (its filter can't exclude it).
    ShieldCounterAndControlWhileShielded(u8),
    /// "Exile any number of nontoken artifacts and/or creatures you control. Return those cards to
    /// the battlefield under their owner's control at the beginning of your next upkeep." (Nahiri's
    /// Resolve), one at a time as `Decision::ExileUntilYourNextUpkeep`.
    ExileYoursUntilYourNextUpkeep,
    /// Put a landmark counter on the source (its `kind_counters`); then with three or more, remove
    /// three, transform it, and create three `def` tokens (Treasure Map).
    LandmarkTransformTreasures {
        def: usize,
    },
    /// The permanent in `slot` phases out until its controller's next untap step (Galadriel's
    /// Dismissal).
    PhaseOutSlot(u8),
    /// Each creature the player in target `slot` controls phases out until that player's next untap
    /// step (Galadriel's Dismissal, kicked).
    PhaseOutCreaturesOfPlayer(u8),
    /// Create a Role token (`def`, an Aura) attached to the creature in `slot` (CR 111.10j). A Role
    /// you already control on that creature is put into its owner's graveyard (CR 303.7).
    CreateRoleAttached {
        def: usize,
        slot: u8,
    },
    /// "You may discard a card. If you do, draw `n` cards." (Witch's Mark: two)
    MayDiscardToDrawN(u8),
    /// Exile the top card of your library face down (`Object::face_down`; CR 406.3: no player may
    /// look at it); put it into your hand at the beginning of your next end step (Necropotence),
    /// see `Game::face_down_to_hand`.
    ExileTopFaceDownToHandAtYourNextEndStep,
    /// The creature in slot `from` deals X damage to the target in slot `to`, X the number of
    /// permanents with subtype `sub` you control, counted on resolution (Dragon Tempest: "it deals X
    /// damage to any target, where X is the number of Dragons you control"); it deals the damage
    /// even if it has left the battlefield (last known information).
    SlotDealsDamagePerSubtypeYouControl {
        from: u8,
        to: u8,
        sub: &'static str,
    },
    /// Destroy all nonartifact creatures (Organic Extinction).
    DestroyAllNonartifactCreatures,
    /// Put a +1/+1 counter on each other creature you control with this subtype (Camellia, the
    /// Seedmiser: Squirrels).
    CountersOnOtherYourSubtype(&'static str),
    /// Create a token (`def`, printed 0/0) with base power and toughness X/X, X the trigger's
    /// amount (Metallurgic Summonings: the cast spell's mana value), not counters.
    CreateTokenXxAmount {
        def: usize,
    },
    /// Return all instant and sorcery cards from your graveyard to your hand (Metallurgic
    /// Summonings).
    ReturnInstantsAndSorceriesFromGraveyard,
    /// Roll a d20; then put any number of creature cards with total mana value X or less from
    /// graveyards onto the battlefield under your control, X the result (Ancient Brass Dragon),
    /// chosen one at a time on resolution (not targeted) as `Decision::ReanimateTotalManaValue`.
    RollD20ReanimateTotalManaValue,
    /// Create a `def` token for each nontoken creature that died this turn (Gadrak).
    CreateTokenPerNontokenCreatureDiedThisTurn {
        def: usize,
    },
    /// Council's dilemma, time or money (Expropriate): each player votes, you first, as
    /// `Decision::ExpropriateVote`; each time vote is an extra turn after this one, and for each
    /// money vote you choose a permanent the voter owns and gain control of it.
    Expropriate,
    /// The player in `slot` scries X, X the spell's X (Kozilek's Command, before its draw).
    ScryXTargetPlayer(u8),
    /// The player in `slot` creates X `def` tokens, X the spell's X (Kozilek's Command's Spawn).
    TargetPlayerCreatesXTokens {
        slot: u8,
        def: usize,
    },
    /// Each player, the active player first, sacrifices a nontoken creature of their choice
    /// (Accursed Marauder), as `Decision::SacrificeNontokenCreatureEach`.
    EachPlayerSacrificesNontokenCreature,
    /// Put a +1/+1 counter on each attacking creature you control (Drana, Liberator of Malakir).
    CountersOnEachAttackingYouControl,
    /// Put +1/+1 counters on each other creature you control equal to that creature's toughness
    /// (Canopy Gargantuan).
    CountersOnOthersEqualToughness,
    /// Remove all counters from the permanent in `slot`: +1/+1 and -1/-1, loyalty, stun, keyword,
    /// lore, time, quest, fellowship, kind and void counters (Vampire Hexmage); `RemoveAllCounters`
    /// takes only +1/+1, stun and keyword counters.
    RemoveEveryKindOfCounter(u8),
    /// Return a card of your choice that the `Pick` allows from your graveyard to your hand, not
    /// targeted (Takenuma: a creature or planeswalker card), as `Decision::LookPick`.
    ReturnPickFromGraveyardToHand(Pick),
    /// "At the beginning of your next upkeep, ...": a delayed triggered ability (CR 603.7), the
    /// source's trigger number `trigger`, at its controller's next upkeep (Pact of Negation); see
    /// `Game::delayed_next_upkeep`.
    DelayedAtYourNextUpkeep {
        trigger: usize,
    },
    /// "Pay {cost}. If you don't, you lose the game." (a Pact's delayed trigger), as
    /// `Decision::PayOrLoseGame`.
    PayOrLoseGame(crate::mana::ManaCost),
    /// Exile the top card of your library face down (you may look at it) and create a `def` token;
    /// when that token leaves the battlefield, the source's trigger `trigger` (a `When::Never`
    /// trigger with `Effect::LinkedExiledCardToHand`) puts the card into your hand (Ugin, the
    /// Ineffable's +1). See `Game::token_linked_exile`.
    ExileTopFaceDownCreateLinkedToken {
        def: usize,
        trigger: usize,
    },
    /// The card in the stack item's `extra` (its zone-change count in `amount`) goes from exile to
    /// its owner's hand, if it's still that exiled card (Ugin, the Ineffable's token leaving).
    LinkedExiledCardToHand,
    /// Put a counter of the source's own kind on it (`Object::kind_counters`), then the player in
    /// `slot` (if any, and legal) draws a card, and another counter goes on it for each card drawn
    /// (Communal Brewing's ingredient counters).
    KindCounterSelfThenPerTargetDraw(u8),
    /// The creature spell in `extra` enters with X additional +1/+1 counters, X the source's kind
    /// counters (Communal Brewing), see `Game::spell_enters_with_counters`.
    CastSpellEntersWithCountersPerKindCounter,
    /// `who` sacrifices a nontoken artifact of their choice (Rootcast Apprenticeship), as
    /// `Decision::SacrificeNontokenArtifact` (automatic with one).
    SacrificeNontokenArtifact(Who),
    /// You may exile three cards from your graveyard; if you do, create a `food` token and ask
    /// again (Insatiable Frugivore's "repeat this process"), as `Decision::ExileThreeForFood`.
    MayExileThreeRepeatFood {
        food: usize,
    },
    /// Creatures you control get +X/+0 and gain `keywords` until end of turn, X the ability's X
    /// (Insatiable Frugivore: menace, X the Foods sacrificed).
    PumpYoursXPowerGrant(crate::Keywords),
    /// `who` gets `n` poison counters (Bloodroot Apothecary: "that player gets two poison
    /// counters").
    PoisonCounters {
        who: Who,
        n: i32,
    },
    /// You gain life equal to the number of creatures you control plus the number of planeswalkers
    /// you control (Ajani, Strength of the Pride's +1).
    GainLifePerCreatureAndPlaneswalkerYouControl,
    /// If you have at least `n` life more than your starting life total, exile the source and each
    /// artifact and creature your opponents control (Ajani, Strength of the Pride's 0: 15).
    ExileSelfAndOpponentsArtifactsCreaturesIfLifeAbove(i32),
    /// Bruna, Light of Alabaster: attach to the source any number of Auras on the battlefield, and
    /// put onto the battlefield attached to it any number of Aura cards that could enchant it from
    /// your graveyard and/or hand, one at a time as `Decision::AttachAura` (game/zones.rs).
    AttachAurasToSource,
    /// Light-Paws, Emperor's Voice: if the entering Aura in `slot` (a triggering slot) was cast, you
    /// may search your library for an Aura card with mana value at most that Aura's and a name
    /// different from each Aura you control, put it onto the battlefield attached to the source,
    /// then shuffle.
    SearchAuraForSourceIfCast(u8),
    /// Sovereigns of Lost Alara, from a "whenever you attack" trigger: if exactly one creature you
    /// control is attacking, you may search your library for an Aura card that could enchant it,
    /// put it onto the battlefield attached to it, then shuffle.
    SearchAuraForLoneAttacker,
    /// Olivia, Crimson Bride: return the creature card in `slot` from your graveyard to the
    /// battlefield tapped and attacking; it gains "When you don't control a legendary Vampire,
    /// exile this creature." (`Object::exile_without_legendary_vampire`).
    ReturnAttackingUntilNoLegendaryVampire(u8),
    /// Dragonhawk, Fate's Tempest: exile the top X cards of your library, X the number of creatures
    /// you control with power 4 or greater; you may play them until your next end step; at the
    /// beginning of that end step, the source's trigger number `trigger` deals 2 damage to each
    /// opponent for each of them still exiled (`Game::delayed_end_step_exiled`).
    ExileTopPowerFourPlayableUntilNextEndStep {
        trigger: usize,
    },
    /// Damage to each opponent equal to the trigger's amount (Dragonhawk's delayed trigger).
    DamageEachOpponentAmount,
    /// Suspend N (CR 702.62a), from an activated ability with `Activated::plot` (exiled from your
    /// hand as its cost): the card stays exiled with N time counters instead of plotted.
    SuspendSource(u8),
    /// Glimpse of Tomorrow: shuffle all permanents you own into your library, reveal that many
    /// cards from the top, put the non-Aura permanent cards onto the battlefield, then the Aura
    /// cards (each attached as `Decision::AuraEntersAttachedTo`), then the rest on the bottom in a
    /// random order.
    GlimpseOfTomorrow,
    /// "If the discarded card was a creature card, amass [subtype] `n`" (Moria Scavenger): the
    /// discard cost's card is the item's `extra` (`Activated::victim_in_extra`).
    AmassIfExtraCreatureCard {
        subtype: &'static str,
        n: u8,
        def: usize,
    },
    /// Stop here unless the card discarded or sacrificed for the cost (the item's `extra`, with
    /// `Activated::victim_in_extra`) is a creature card (Moria Scavenger: "If the discarded card
    /// was a creature card, ...").
    StopUnlessExtraIsCreatureCard,
    /// Look at the top card of your library; if it's a land card, reveal it and create a `def`
    /// token; then draw a card (Fisher's Talent: the Fish). "You may reveal it" is always taken:
    /// revealing only adds the token.
    CreateTokenIfTopIsLandThenDraw {
        def: usize,
    },
    /// The creature in `slot` can't block for as long as you control the source (There and Back
    /// Again's chapter I), see `Game::cant_block_while_controlled`.
    TargetCantBlockWhileYouControlSource(u8),
    /// Gisa, Glorious Resurrector: put all creature cards exiled with the source
    /// (`Game::exiled_with`) onto the battlefield under your control; they gain decayed.
    ReturnExiledWithSourceDecayed,
    /// The player in `slot` loses `life` life unless they sacrifice a nonland permanent or discard a
    /// card (Tergrid's Lantern: 3), as `Decision::SacrificeDiscardOrLoseLife`.
    TargetPlayerLosesUnlessSacrificesOrDiscards {
        slot: u8,
        life: i32,
    },
    /// Discard a card (`Decision::DiscardThenIfInstantOrSorcery`); the rest of the effects happen
    /// only if it was an instant or sorcery card (Vohar, Vodalian Desecrator).
    DiscardThenIfInstantOrSorcery,
    /// Prevent the next `n` damage that would be dealt to the permanent in `slot` this turn (Eiganjo
    /// Castle: 2), see `Game::damage_shields_on`.
    PreventNextDamageToTargetThisTurn {
        slot: u8,
        n: i32,
    },
    /// The source deals X damage to the target in `to`, X the permanents with subtype `sub` you
    /// control (Smaug the Magnificent: Treasures), counted on resolution.
    SourceDamagePerSubtypeYouControl {
        to: u8,
        sub: &'static str,
    },
    /// Create a `def` token for each creature that died this turn, tokens included (Mahadi).
    CreateTokenPerCreatureDiedThisTurn {
        def: usize,
    },
    /// Breath of Fury: sacrifice the creature the source Aura enchants and attach the Aura to a
    /// creature you control (`Decision::MoveAuraTo`); the rest of the effects happen only if it's
    /// attached ("If you do").
    SacrificeEnchantedMoveAura,
    /// Kaalia of the Vast: you may put a creature card with one of these subtypes from your hand onto
    /// the battlefield tapped and attacking (`Decision::CreatureFromHandAttacking`).
    CreatureFromHandAttacking(&'static [&'static str]),
    /// Command Beacon: put your commander into your hand from the command zone.
    CommanderFromCommandZoneToHand,
    /// Exile the top `n` cards of your library (Mystic Forge: 1).
    ExileTopCards(u8),
    /// Lithoform Engine: copy the activated or triggered ability in `slot` (targeted as its source,
    /// `Filter::abilities`) if you control it; its controller may choose new targets for the copy.
    CopyTargetAbilityYouControl(u8),
    /// Lithoform Engine: copy the spell in `slot` if you control it; you may choose new targets for
    /// the copy. With `permanent`, only a permanent spell is copied, and the copy becomes a token
    /// (CR 707.10f).
    CopyTargetSpellYouControl {
        slot: u8,
        permanent: bool,
    },
    /// The source becomes a copy of the permanent in `slot` for good (The Mycosynth Gardens). With
    /// `keep_own`, the copy keeps the source card's `copy_only` activated abilities (Thespian's
    /// Stage: "except it has this ability"; the card needs `copy_keeps_abilities`).
    SourceBecomesCopyOfSlot {
        slot: u8,
        keep_own: bool,
    },
    /// The source deals damage equal to the trigger's amount to the controller of the object in
    /// `extra` (Deflecting Palm, after `PreventNextFromChosenSource`).
    DamageExtrasControllerAmount,
    /// Sylvan Library: you may draw two additional cards (`Decision::SylvanDraw`); if you do,
    /// choose two cards in your hand drawn this turn (`Decision::SylvanPick`), and for each pay 4
    /// life or put it on top of your library (`Decision::SylvanPayOrTop`).
    SylvanLibrary,
    /// Destroy all creatures; you create a tapped `def` token for each nontoken creature destroyed
    /// this way (Blood Money: Treasure).
    DestroyAllCreaturesTappedTokenPerNontoken {
        def: usize,
    },
    /// The player in `slot` draws half the cards in their library and loses half their life, each
    /// rounded up (Peer into the Abyss).
    TargetPlayerDrawsHalfLibraryLosesHalfLife(u8),
    /// Each player draws a card for each of the source's own kind of counter (Otherworld Atlas's
    /// charge counters, `Object::kind_counters`).
    EachPlayerDrawsPerKindCounter,
    /// Search for a creature card with mana value exactly `plus` more than the sacrificed creature's
    /// (the stack entry's `extra`) and put it onto the battlefield with an additional +1/+1 counter,
    /// then shuffle (Neoform).
    SearchCreatureBySacrificedMvPlusCounter {
        plus: u8,
    },
    /// Exchange control of the source and the creature in `slot`; if you don't or can't (no target,
    /// or either has left), sacrifice the source (Gilded Drake).
    ExchangeControlWithSlotOrSacrifice(u8),
    /// Put `n` +1/+1 counters on the creature the source (an Equipment or Aura) is attached to (Bone
    /// Sabres).
    CountersOnAttached(i32),
    /// Create a token that's a copy of the artifact the source (an Aura) enchants; then, if you
    /// control eight or more artifacts with the same name as one another, you win the game
    /// (Mechanized Production).
    CopyAttachedThenEightSameNameWins,
    /// "You may pay {X}, where X is less than or equal to the amount of life you gained. If you do,
    /// draw X cards." (Well of Lost Dreams): the trigger's amount caps X, as `Decision::PayXDrawX`.
    MayPayXUpToAmountDrawX,
    /// Destroy all planeswalkers (Final Act).
    DestroyAllPlaneswalkers,
    /// Each opponent loses all counters (CR 122.1: a player's poison and energy counters) (Final
    /// Act).
    EachOpponentLosesAllCounters,
    /// Exile the creature in `slot` if its power is at most the power of the card in the item's
    /// `extra` (Titan's Presence: the revealed card).
    ExileIfPowerAtMostExtra(u8),
}

/// What an [`Effect::AddManaPerCount`] counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManaCount {
    /// Permanents you control with this subtype ("for each Swamp you control").
    YouControlSubtype(&'static str),
    /// Basic permanents you control with this subtype ("for each basic Swamp you control").
    YouControlBasicSubtype(&'static str),
    /// Permanents on the battlefield with this subtype, anyone's ("for each Elf on the battlefield").
    OnBattlefieldSubtype(&'static str),
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
    /// A land card with one of these basic land types (Plains, Island, Swamp, Mountain, Forest in
    /// WUBRG order); `basic`: basic land cards only ("a basic Plains, Swamp, or Forest card").
    LandTypes {
        types: [bool; 5],
        basic: bool,
    },
    /// A card with one of these names.
    Named(&'static [&'static str]),
    /// A card of this color (WUBRG index).
    Colored(u8),
    /// A card with this subtype ("a Dragon card").
    Subtype(&'static str),
    /// A colorless nonland card.
    ColorlessNonland,
    /// Any land card.
    Land,
    /// A creature card with mana value at least this (Fierce Empath).
    CreatureMvAtLeast(u32),
    /// A basic land card; with more to find, the rest must share a basic land type with the first
    /// found (Myriad Landscape: "up to two basic land cards that share a land type").
    BasicLandSharingType,
    /// A card with exactly this mana value (transmute).
    ManaValue(u32),
    /// A green creature card with mana value at most X (resolved to `ColoredCreatureMvAtMost`).
    GreenCreatureMvAtMostX,
    /// A creature card of this color (WUBRG index) with mana value at most this.
    ColoredCreatureMvAtMost(u8, u32),
    /// An instant or sorcery card (Mystical Tutor).
    InstantOrSorcery,
    /// A nonlegendary card (Unmarked Grave).
    Nonlegendary,
    /// An artifact or enchantment card (Enlightened Tutor).
    ArtifactOrEnchantment,
    /// An enchantment card (Academy Rector).
    Enchantment,
    /// A creature card with at most this power (Imperial Recruiter).
    CreaturePowerAtMost(i32),
    /// A creature card with exactly this mana value (Birthing Pod).
    CreatureMvExactly(u32),
    /// An artifact card with exactly this mana value (Trophy Mage: 3).
    ArtifactManaValue(u32),
    /// An artifact card with a mana ability, or a basic land card (Moonsilver Key).
    ArtifactWithManaAbilityOrBasicLand,
    /// An artifact card with mana cost {0} or {1} (Urza's Saga). The engine's costs don't tell
    /// {0} from no mana cost, so an artifact with none counts too.
    ArtifactManaCostZeroOrOne,
    /// A colorless creature card (Sanctum of Ugin).
    ColorlessCreature,
    /// An artifact card (Inventors' Fair, Goblin Engineer, Reckless Handling).
    Artifact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchTo {
    Hand,
    Battlefield {
        tapped: bool,
    },
    /// Shuffle, then put the card on top.
    Top,
    /// On top, or onto the battlefield tapped instead if you control a permanent with this subtype
    /// (Embermouth Sentinel). Decided as the search resolves.
    TopOrTappedIfYouControl(&'static str),
    /// Into the graveyard.
    Graveyard,
    /// Onto the battlefield tapped with a stun counter on it.
    BattlefieldTappedStunned,
    /// Exile it; you may cast it without paying its mana cost this turn (Ugin's −11).
    ExileCastFreeThisTurn,
    /// Onto the battlefield tapped under this player's control (Yavimaya Dryad).
    BattlefieldTappedUnder(u8),
    /// Onto the battlefield with an additional +1/+1 counter (Neoform).
    BattlefieldPlusCounter,
}

/// What a static ability affects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    This,
    /// Attacking tokens you control (Bone-Cairn Butcher).
    YourAttackingTokens,
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
    /// Creatures you control of the source's chosen creature type.
    YourCreaturesOfChosenType,
    /// Untapped creatures you control with this subtype ("Untapped Dragons you control").
    YourUntappedSubtype(&'static str),
    /// Attacking creatures you control (Berserkers' Onslaught).
    YourAttackingCreatures,
    /// Artifact creatures you control (Thopter Engineer).
    YourArtifactCreatures,
    /// Other creatures you control (Nylea, God of the Hunt).
    OtherYourCreatures,
    /// Every land on the battlefield (Yavimaya, Cradle of Growth).
    EachLand,
    /// Land creatures you control (Sylvan Advocate).
    YourLandCreatures,
    /// Other permanents you control, of any type (Avacyn, Angel of Hope).
    OtherYourPermanents,
    /// Creatures your opponents control (Elesh Norn, Grand Cenobite).
    OpponentsCreatures,
    /// Other artifact creatures you control (Cyberdrive Awakener).
    OtherYourArtifactCreatures,
    /// Other enchantments you control (Sterling Grove).
    OtherYourEnchantments,
    /// Creatures with flying, printed or until end of turn; anyone's (Gravitational Shift).
    CreaturesWithFlying,
    /// Creatures without flying (printed or until end of turn); anyone's (Gravitational Shift).
    CreaturesWithoutFlying,
    /// Tapped creatures you control (Sword of the Paruns).
    YourTappedCreatures,
    /// Untapped creatures you control (Sword of the Paruns).
    YourUntappedCreatures,
    /// Other non-Human creatures you control, by printed creature types (Mikaeus, the
    /// Unhallowed); a changeling is a Human, so it's left out.
    OtherYourNonHumanCreatures,
}

/// What a scaled static ability counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scale {
    /// Permanents you control with this subtype ("for each Forest you control").
    PerYouControlSubtype(&'static str),
    /// Fellowship counters on the source.
    PerFellowshipCounter,
    /// Creatures you control ("power is equal to the number of creatures you control").
    PerCreatureYouControl,
    /// Other creatures you control with base power 1 (Zinnia).
    PerOtherYourCreatureBasePowerOne,
    /// Creatures you control with base power or toughness 1 (Sword of the Squeak).
    PerYourCreatureBasePowerOrToughnessOne,
    /// Colors among permanents you control (Faeburrow Elder).
    PerColorAmongYourPermanents,
    /// The source's own kind of counters (Chitterspitter's acorn counters).
    PerKindCounter,
    /// Artifacts you control (Cranial Plating).
    PerArtifactYouControl,
    /// Enchantments on the battlefield other than the source (Ancestral Mask).
    PerOtherEnchantmentOnBattlefield,
    /// Cards you've drawn this turn (Knowledge Is Power).
    PerCardDrawnThisTurn,
    /// Other creatures on the battlefield with flying, printed or granted until end of turn (Pride
    /// of the Clouds).
    PerOtherCreatureWithFlying,
    /// Cards in all players' hands (Multani, Maro-Sorcerer).
    PerCardInAllHands,
    /// The number of cards in all graveyards (Lord of Extinction).
    PerCardInAllGraveyards,
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
    /// Its controller controls a creature with a counter (of any kind) on it.
    ControlCreatureWithCounter,
    /// The source hasn't dealt damage since it entered the battlefield (Karakyk Guardian).
    HasntDealtDamage,
    /// The source's mode, chosen as it entered, is this one (the Sieges).
    ChosenMode(u8),
    /// At least this many different kinds of counters among creatures its controller controls.
    CounterKindsAmongYoursAtLeast(u8),
    /// The source has at least this many quest counters.
    QuestCountersAtLeast(i32),
    /// Its controller has the city's blessing (ascend).
    CitysBlessing,
    /// Its controller controls at least this many lands.
    ControlLandsAtLeast(u8),
    /// The creature the source is attached to is tapped (`true`) or untapped (`false`) (Sword of
    /// the Paruns).
    AttachedTapped(bool),
    /// The creature the source is attached to is this color, WUBRG index (Shield of the Oversoul).
    AttachedIsColor(u8),
    /// The source's controller has completed a dungeon (Nadaar).
    CompletedDungeon,
    /// Its controller controls at least this many artifacts (Teething Wurmlet: three). Printed card
    /// types are counted, as `ControlLandsAtLeast` does.
    ControlArtifactsAtLeast(u8),
    /// The source Class is at this level or higher (CR 716: a Class's level 2 and 3 statics), see
    /// `Game::class_level`.
    ClassLevelAtLeast(u8),
    /// Another Aura is attached to the creature the source is attached to (Face of Divinity).
    AttachedHasAnotherAura,
    /// This color (WUBRG index) isn't in the source's controller's commander's color identity
    /// (with a partner, either's); with no commander, no color is (Commander's Plate).
    OutsideCommanderIdentity(u8),
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
    /// Sets the colors of what it applies to (Witness Protection: green and white; Imprisoned in
    /// the Moon: colorless).
    pub set_colors: Option<[bool; 5]>,
    /// What it applies to is a land and no other card type, with "{T}: Add {C}" (Imprisoned in the
    /// Moon). Supertypes stay.
    pub becomes_land: bool,
}

/// What an emblem does (CR 114).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmblemKind {
    /// Creatures you control get +2/+2 and have vigilance, trample and indestructible.
    VivienAnthem,
    /// Whenever a player casts a spell, its owner's card's trigger number `trigger` happens (Kaito).
    OnAnySpellCast { trigger: usize },
    /// Creatures you control get +2/+2 and have flying (Elspeth, Sun's Champion).
    ElspethAnthem,
    /// Creatures you control get +3/+3 and have trample (Garruk, Cursed Huntsman).
    GarrukAnthem,
    /// "You have no maximum hand size."
    NoMaxHandSize,
    /// "Lands you control have indestructible." (Nissa, Who Shakes the World)
    LandsIndestructible,
    /// An emblem with a triggered ability: trigger number `trigger` of the card that made it
    /// (written with `When::Never`) happens whenever `when` fires for the emblem's owner.
    Triggered { when: When, trigger: usize },
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
    /// A land card.
    Land,
    /// An instant, a sorcery, or a card with this subtype ("an instant, sorcery, or Dragon card").
    InstantSorceryOrSubtype(&'static str),
    /// An enchantment card.
    Enchantment,
    /// A non-Human creature card (Kinnan, Bonder Prodigy).
    NonHumanCreature,
    /// A permanent card with an odd mana value (Desecrate Reality).
    PermanentOddManaValue,
    /// A creature card with mana value at most this (Barrowin of Clan Undurr: 3).
    CreatureManaValueAtMost(u32),
    /// A creature or planeswalker card (Takenuma, Abandoned Mire).
    CreatureOrPlaneswalker,
    /// A noncreature, nonland card (Narset, Parter of Veils).
    NoncreatureNonland,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookTo {
    Hand,
    Top,
    Battlefield,
    /// Stays in exile; you may play it until the end of your next turn.
    PlayFromExile,
    /// Stays in exile; you may play it this turn.
    PlayFromExileThisTurn,
    /// Onto the battlefield tapped.
    BattlefieldTapped,
    /// Onto the battlefield with two +1/+1 counters.
    BattlefieldTwoCounters,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookRest {
    Graveyard,
    /// Into the graveyard, and one card must be taken if any can ("Put one of them into your hand").
    GraveyardTakeOne,
    /// Into the graveyard, after taking two (one at a time) ("Put two of them into your hand").
    GraveyardTakeTwo,
    /// Into the graveyard; if no card was taken, a +1/+1 counter on the source (Ainok Wayfarer).
    GraveyardElseCounter,
    /// The bottom of the library in a random order.
    BottomRandom,
    /// Each one you may put into your graveyard; the rest stay on top (Traveling Botanist).
    MayGraveyard,
    /// The bottom of the library in the order you choose; one card must be taken if any can.
    BottomAnyOrderTakeOne,
}

/// An intervening "if" clause, checked when the ability triggers and again on resolution
/// (CR 603.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Condition {
    /// The source has reached at least this stage (a gained triggered ability, Kellan). Checked
    /// when the ability would trigger.
    StageAtLeast(u8),
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
    /// You control a creature with a counter (of any kind) on it.
    ControlCreatureWithCounter,
    /// "If you cast it": the source entered the battlefield as a resolving spell.
    CastIt,
    /// You've cast two or more spells this turn.
    CastTwoSpellsThisTurn,
    /// The source's mode, chosen as it entered, is this one (the Sieges).
    ChosenMode(u8),
    /// Both conditions hold.
    Both(Box<Condition>, Box<Condition>),
    /// A creature died under your control this turn.
    CreatureDiedUnderYourControlThisTurn,
    /// You've cast both a creature spell and a noncreature spell this turn.
    CastCreatureAndNoncreatureThisTurn,
    /// Creatures you control have total toughness at least this.
    TotalToughnessAtLeast(i32),
    /// It's your main phase.
    YourMainPhase,
    /// You have exactly this many cards in your hand (Triskaidekaphile).
    HandSizeExactly(u8),
    /// You control a creature with the greatest power, or tied for it, among all creatures.
    ControlGreatestPower,
    /// An opponent controls more lands than you (Loyal Warhound).
    OpponentControlsMoreLands,
    /// The source's X (as it was cast) is at least this (ravenous: "If X is 5 or more").
    XAtLeast(u8),
    /// It's the first combat phase of the turn.
    FirstCombatOfTurn,
    /// The source hasn't been exerted this turn.
    NotExertedThisTurn,
    /// You control at least this many artifacts.
    ControlArtifactsAtLeast(u32),
    /// The source is in a graveyard or exile (it died or was exiled).
    SourceInGraveyardOrExile,
    /// It was cast from its owner's hand ("if you cast it from your hand").
    CastFromHand,
    /// Its controller controls at least this many lands.
    ControlLandsAtLeast(u8),
    /// The cast creature spell (the trigger's `extra`) has power from `.0` through `.1` (checked as
    /// it triggers).
    CastPowerBetween(i32, i32),
    /// The source is tapped ("if this artifact is tapped", Mana Vault).
    SourceTapped,
    /// The source is untapped ("if this artifact is untapped", Howling Mine).
    SourceUntapped,
    /// It's its controller's turn (a "your draw step" trigger built on `When::EachDrawStep`).
    YourTurn,
    /// Its controller controls their commander (lieutenant: "if you control your commander").
    ControlCommander,
    /// It isn't its controller's turn. With `When::PlayerPlaysLand`: "whenever an opponent plays a
    /// land" (a land is played only on its player's own turn).
    NotYourTurn,
    /// Its controller controls at least `.1` permanents with subtype `.0` (Revel in Riches: ten or
    /// more Treasures).
    ControlSubtypeAtLeast(&'static str, u32),
    /// An opponent controls at least this many creatures (Defense of the Heart).
    OpponentControlsCreaturesAtLeast(u8),
    /// An opponent lost at least this much life this turn (Bloodchief Ascension: 2).
    OpponentLostLifeAtLeast(i32),
    /// The source has at least this many quest counters (Bloodchief Ascension: three).
    SourceQuestCountersAtLeast(i32),
    /// You've completed a dungeon (CR 309.7).
    CompletedDungeon,
    /// You have the initiative (CR 725).
    HaveInitiative,
    /// The source is a creature this turn from its own "this becomes a creature" ability
    /// (`Effect::BecomeCreatureUntilEotWith`): the ability it gains then (Raging Ravine's attack
    /// trigger).
    AnimatedByOwnAbility,
    /// Its controller gained life this turn (Witch of the Moors).
    YouGainedLifeThisTurn,
    /// Its controller has at least this much life (Felidar Sovereign: 40).
    LifeAtLeast(i32),
    /// No creatures are on the battlefield (Pyrohemia).
    NoCreaturesOnBattlefield,
    /// Its controller lost life this turn (Lunar Convocation: "if you gained and lost life this
    /// turn", with `Both` and `YouGainedLifeThisTurn`).
    YouLostLifeThisTurn,
    /// Its Class is at this level or higher (a Class's level 2 and 3 triggered abilities).
    ClassLevelAtLeast(u8),
    /// At least this much colorless mana was spent to cast the spell (Desecrate Reality's
    /// adamant: 3), with `Effect::StopUnlessCondition` before the adamant part.
    ColorlessSpentAtLeast(u32),
    /// Its controller created a token this turn (Bennie Bracks), from `Game::token_created_turn`.
    YouCreatedTokenThisTurn,
    /// Its controller controls at least this many lands with different names (The Necrobloom, Field
    /// of the Dead: seven).
    ControlLandsWithDifferentNamesAtLeast(u8),
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
    /// "N damage divided as you choose among" its targets: the total, divided as it goes on the
    /// stack with at least 1 to each target (CR 601.2d). Zero for everything else.
    pub divide: u8,
    /// The divided total is the number of lands you control as it goes on the stack, instead of
    /// `divide` (Ureni: "X damage divided ..., where X is the number of lands you control").
    pub divide_lands: bool,
    /// "Each of X target ...": exactly the ability's X targets.
    pub targets_exactly_x: bool,
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
            divide: 0,
            divide_lands: false,
            targets_exactly_x: false,
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
    /// "Whenever a creature you control deals combat damage during your turn": the creature fills
    /// a triggering slot, and the amount is the damage it dealt this step.
    YourCreatureDealsCombatDamageOnYourTurn,
    /// "Whenever an opponent loses life" (the amount travels with the trigger).
    OpponentLosesLife,
    /// "Whenever a source you control deals noncombat damage to an opponent" (with the amount).
    YourSourceDealsNoncombatDamageToOpponent,
    /// "Whenever you cast a spell".
    YouCast,
    /// Flurry (Tarkir: Dragonstorm): "Whenever you cast your second spell each turn".
    YouCastSecondSpell,
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
    /// Like `YouCastInstantOrSorcery`, with the spell in `extra` and, as the amount, how many
    /// instants and sorceries you cast before it this turn (Thousand-Year Storm).
    YouCastInstantOrSorceryCounted,
    /// "Whenever you cast a noncreature or [subtype] spell".
    YouCastNoncreatureOrSubtype(&'static str),
    /// "Whenever a creature an opponent controls enters".
    CreatureOpponentControlsEnters,
    /// A Saga's chapter ability, when its Nth lore counter is put on it (CR 714.2).
    Chapter(u8),
    /// "Whenever this creature becomes tapped" (CR 603.2; entering tapped isn't becoming tapped).
    BecomesTapped,
    /// "Whenever one or more creatures you control deal combat damage to a player": once per
    /// combat damage step.
    YourCreaturesDealCombatDamageToPlayer,
    /// "Whenever a counter is put on a creature you control. This ability triggers only once each
    /// turn." (Counters it enters with count, CR 122.6.)
    CounterPutOnYourCreatureOnceEachTurn,
    /// "Whenever one or more counters are put on a creature you control, if it's the first time
    /// counters have been put on that creature this turn" (slot 0 is that creature).
    FirstCountersOnYourCreatureThisTurn,
    /// "Whenever you cast a [subtype] spell".
    YouCastSubtype(&'static str),
    /// "Whenever equipped creature attacks".
    EquippedAttacks,
    /// "Whenever a creature you control dies while this card is in your graveyard".
    CreatureYouControlDiesFromGraveyard,
    /// "Whenever this creature or another creature you control dies, if it had counters on it"; the
    /// dead creature is in the stack item's `extra`.
    CreatureYouControlWithCountersDies,
    /// "Whenever a creature you control with a counter on it deals combat damage to a player".
    YourCreatureWithCounterDealsCombatDamageToPlayer,
    /// "Whenever a creature you control or a creature spell you control becomes the target of a
    /// spell or ability an opponent controls" (once per target).
    YourCreatureTargetedByOpponent,
    /// "When this creature leaves the battlefield" (any zone it goes to).
    LeavesBattlefield,
    /// "When you cast this spell" (from the stack).
    YouCastThis,
    /// "When you cast this spell": triggers from the stack as the spell becomes cast (CR 601.2i).
    CastThisSpell,
    /// "Whenever you cast a colorless spell".
    YouCastColorless,
    /// "At the beginning of each combat" (every player's).
    BeginCombatEachTurn,
    /// Only as a delayed trigger: "Whenever a creature you control enters this turn".
    CreatureYouControlEntersThisTurn,
    /// "Whenever another nontoken creature you control enters": slot 0 (triggering) is that creature.
    AnotherNontokenCreatureYouControlEnters,
    /// "Whenever one or more cards leave your graveyard during your turn" (cards leaving between two
    /// times triggers are put on the stack count as one event).
    CardsLeaveYourGraveyardDuringYourTurn,
    /// "Whenever a card leaves your graveyard during your turn, ... This ability triggers only once
    /// each turn."
    CardLeavesYourGraveyardOnceEachTurn,
    /// "Whenever a creature you control enters, if you cast it": fired as a cast creature spell
    /// resolves; slot 0 (triggering) is that creature.
    CastCreatureYouControlEnters,
    /// "Whenever another creature dies" (any player's, tokens included).
    AnotherCreatureDies,
    /// "Whenever an opponent casts an instant or sorcery spell".
    OpponentCastsInstantOrSorcery,
    /// "Whenever you cast a creature spell".
    YouCastCreature,
    /// "Whenever another creature you control with power N or greater enters".
    AnotherCreatureYouControlEntersWithPowerAtLeast(i32),
    /// "Whenever one or more other creatures die, ... This ability triggers only once each turn."
    OtherCreaturesDieOnceEachTurn,
    /// "Whenever an opponent casts a noncreature spell".
    OpponentCastsNoncreature,
    /// "Whenever a creature token you control deals combat damage to a player" (each one).
    YourCreatureTokenDealsCombatDamageToPlayer,
    /// "Whenever this creature is dealt damage" (the amount is the trigger's amount).
    DealtDamage,
    /// "Whenever a token you control leaves the battlefield".
    TokenYouControlLeaves,
    /// "Whenever a token you control enters".
    TokenYouControlEnters,
    /// "Whenever you give a gift".
    YouGiveGift,
    /// "Whenever a creature you control without flying dies" (the creature in `extra`).
    CreatureYouControlWithoutFlyingDies,
    /// "Whenever a creature you control attacks" (once for each attacker).
    CreatureYouControlAttacks,
    /// "Whenever another [A] or [B] you control enters" (any permanent with one of the subtypes).
    AnotherYouControlEntersWithAnySubtype(&'static [&'static str]),
    /// "Whenever another creature you control with base power N enters" (that creature is the
    /// trigger's first target, if its first filter is `triggering`).
    AnotherCreatureYouControlEntersWithBasePower(i32),
    /// "Whenever another creature you control enters, if it wasn't cast".
    AnotherCreatureYouControlEntersNotCast,
    /// "Whenever you expend N": as you spend your Nth total mana on spells this turn.
    Expend(u8),
    /// "At the beginning of each player's draw step" (either player's permanent; the active player is
    /// "that player").
    EachDrawStep,
    /// "When you cycle this card": from the graveyard, as its cycling ability is activated.
    YouCycleThis,
    /// "Whenever equipped creature dies".
    EquippedDies,
    /// "Whenever a creature you control leaves the battlefield" (any zone it goes to).
    CreatureYouControlLeavesBattlefield,
    /// "Whenever an opponent casts a noncreature spell with mana value less than this creature's
    /// power" (Pollywog Prodigy): checked as it triggers.
    OpponentCastsNoncreatureMvBelowPower,
    /// "Whenever an opponent attacks with creatures, if two or more of those creatures are attacking
    /// you and/or planeswalkers you control" (two players: two or more attackers).
    OpponentAttacksWithAtLeastTwo,
    /// "Whenever an opponent casts their second spell each turn".
    OpponentCastsSecondSpell,
    /// "Enchantment spells you cast from your hand have cascade": when you cast one, with it in `extra`.
    YouCastEnchantmentFromHand,
    /// "Whenever this creature blocks" (CR 509.1i), as blockers are declared.
    Blocks,
    /// "Whenever you scry or surveil".
    YouScryOrSurveil,
    /// "Whenever a creature you control deals combat damage to a player": once for each creature, with
    /// the damage it dealt as the amount (Old Gnawbone).
    YourCreatureDealsCombatDamageToPlayerAmount,
    /// "Whenever a source you control deals 5 or more damage to a player" (Dragonborn Champion).
    YourSourceDealsFiveOrMoreToPlayer,
    /// "Whenever this creature deals 6 or more damage to an opponent" (Deus of Calamity).
    DealsSixOrMoreToOpponent,
    /// "Whenever one or more creatures you control with trample deal combat damage to a player": the
    /// amount is the damage those creatures dealt (Quartzwood Crasher).
    YourTramplersDealCombatDamageToPlayer,
    /// "Whenever one or more cards leave your graveyard" (any turn).
    CardsLeaveYourGraveyard,
    /// "Whenever one or more land cards are put into your graveyard from anywhere".
    LandCardsToYourGraveyard,
    /// "Whenever one or more creature cards are put into your graveyard from your library".
    CreatureCardsMilledToYourGraveyard,
    /// "Whenever this creature becomes blocked" (CR 509.3c), as blockers are finished.
    BecomesBlocked,
    /// "Whenever you cast a green permanent spell".
    YouCastGreenPermanent,
    /// "Whenever one or more land cards are put into your graveyard from your library".
    LandCardsMilledToYourGraveyard,
    /// "Whenever a player plays a land" (Horn of Greed): fires for every player's permanents.
    PlayerPlaysLand,
    /// "When this is put into a graveyard from anywhere" (the source itself; Worldspine Wurm).
    PutIntoGraveyardFromAnywhere,
    /// "Whenever an enchantment you control enters" (constellation), this one included.
    EnchantmentYouControlEnters,
    /// "At the beginning of your first main phase" (precombat main, CR 505.1a).
    FirstMainPhaseYourTurn,
    /// "Whenever you investigate for the first time each turn" (Erdwal Illuminator).
    YouInvestigateFirstTimeEachTurn,
    /// "Whenever a creature an opponent controls dies" (tokens included).
    OpponentCreatureDies,
    /// "Whenever another creature you control dies" (tokens included; `AnotherNontokenCreatureYouControlDies`
    /// is nontoken only).
    AnotherCreatureYouControlDies,
    /// "Whenever this creature or another [subtype] you control dies" (tokens included; Undead
    /// Augur: Zombie). The creature it was is checked as it last existed.
    ThisOrAnotherYouControlDiesWithSubtype(&'static str),
    /// "Whenever an artifact is put into a graveyard from the battlefield" (anyone's, tokens included).
    ArtifactToGraveyardFromBattlefield,
    /// "Whenever an artifact is put into an opponent's graveyard from the battlefield".
    ArtifactToOpponentsGraveyardFromBattlefield,
    /// "Whenever you discard a card" (cycling included: it discards the card). Slot 0, if its first
    /// filter is `triggering`, is the discarded card in your graveyard.
    YouDiscard,
    /// "Whenever you cast an enchantment spell" (from any zone); the spell is in `extra`.
    YouCastEnchantment,
    /// "Whenever you cast an artifact spell" (from any zone); the spell is in `extra`.
    YouCastArtifact,
    /// "Whenever one or more +1/+1 counters are put on this creature": every time (entering with
    /// them included, CR 122.6). `CountersPutOnThis` is the once-each-turn version.
    PlusCountersPutOnThis,
    /// "Whenever you lose life" (every time; the amount travels with the trigger).
    YouLoseLife,
    /// "Whenever you sacrifice a permanent" (each one).
    YouSacrifice,
    /// "Whenever a player sacrifices another permanent" (any player's, each one).
    PlayerSacrificesAnother,
    /// "Whenever you sacrifice a token".
    YouSacrificeToken,
    /// "Whenever you sacrifice a [subtype]" (each one; Rapacious Guest: Food).
    YouSacrificeSubtype(&'static str),
    /// "Whenever this creature or another creature or artifact you control dies" (tokens included;
    /// Al Bhed Salvagers).
    ThisOrAnotherCreatureOrArtifactYouControlDies,
    /// "Whenever another permanent you control enters" (Altar of the Brood).
    AnotherPermanentYouControlEnters,
    /// "Whenever another creature you control of this color enters" (a `Color` index: Ivy Lane
    /// Denizen, green).
    AnotherCreatureYouControlEntersWithColor(usize),
    /// "Whenever you cast a noncreature spell with mana value N or greater" (Jin-Gitaxias: 3).
    YouCastNoncreatureMvAtLeast(u32),
    /// "Whenever a player casts their first multicolored spell each turn", for its controller's own
    /// spell (Zenith Chronicler pairs it with `OpponentCastsFirstMulticolored`).
    YouCastFirstMulticolored,
    /// The opponent's first multicolored spell each turn (Zenith Chronicler).
    OpponentCastsFirstMulticolored,
    /// "Whenever an opponent casts their first noncreature spell each turn" (Shadow in the Warp).
    OpponentCastsFirstNoncreature,
    /// "Whenever you cast a legendary spell" (Rona, Herald of Invasion).
    YouCastLegendary,
    /// "Whenever you cast a [color] spell" (a `Color` index; K'rrik: black).
    YouCastColor(usize),
    /// "Whenever a player casts their second spell each turn" (Ledger Shredder).
    PlayerCastsSecondSpell,
    /// Magecraft: "Whenever you cast or copy an instant or sorcery spell" (the spell in `extra`).
    YouCastOrCopyInstantOrSorcery,
    /// "Whenever an opponent casts a spell from anywhere other than their hand" (Ghostly Pilferer).
    OpponentCastsNotFromHand,
    /// "Whenever a creature deals damage to you": that creature in slot 0 (No Mercy: destroy it).
    CreatureDealsDamageToYou,
    /// "Whenever a creature you control deals combat damage to a player": that creature in slot 0
    /// (Sphere Grid).
    YourCreatureDealsCombatDamageToPlayerIt,
    /// "Whenever one or more [subtype]s you control deal damage to a player": once for a combat
    /// damage step, once for each noncombat damage event (Francisco: Pirates).
    YourSubtypeDealDamageToPlayer(&'static str),
    /// "Whenever this creature deals damage to an opponent" (Looter il-Kor), combat or not.
    DealsDamageToOpponent,
    /// "Whenever a [subtype] attacks", anyone's: the attacker in slot 0 (Najeela: Warrior).
    SubtypeAttacks(&'static str),
    /// "Whenever this becomes untapped": in an untap step or by an effect (Ghostly Pilferer).
    BecomesUntapped,
    /// An Aura's grant: enchanted creature has "Whenever this creature becomes untapped, ... This
    /// ability triggers only once each turn." (Well Rested): on the Aura, once each turn.
    EnchantedBecomesUntappedOnceEachTurn,
    /// "Whenever a card is put into an opponent's graveyard from anywhere" (Bloodchief Ascension;
    /// tokens aren't cards).
    CardToOpponentsGraveyard,
    /// "Whenever ... a creature card is put into a graveyard from anywhere other than the
    /// battlefield" (Syr Konrad), anyone's.
    CreatureCardToGraveyardNotFromBattlefield,
    /// "Whenever ... a creature card leaves your graveyard" (Syr Konrad).
    CreatureCardLeavesYourGraveyard,
    /// "Whenever a creature of the chosen type dies" (Species Specialist: the source's
    /// `chosen_type`), anyone's, as it last existed.
    CreatureOfChosenTypeDies,
    /// A delayed "whenever a creature dies this turn" (Massacre Girl), for
    /// `Effect::DelayedThisTurn`.
    CreatureDiesThisTurn,
    /// "Whenever you sacrifice a [color] creature" (a `Color` index; Savra: black, green).
    YouSacrificeCreatureOfColor(usize),
    /// "Whenever you discard one or more cards": one trigger for the discards before the next
    /// priority, the count in its amount (Cryptcaller Chariot: "that many").
    YouDiscardOneOrMore,
    /// "Whenever you sacrifice one or more [subtype]s": one trigger for the sacrifices before the
    /// next priority, the count in its amount (Camellia: Foods).
    YouSacrificeOneOrMoreSubtype(&'static str),
    /// "Whenever a creature you control with flying enters": that creature in slot 0 when the first
    /// filter is `triggering` (Dragon Tempest).
    CreatureWithFlyingYouControlEnters,
    /// "Whenever a creature an opponent controls blocks" (Benefactor's Draught, delayed this turn).
    OpponentCreatureBlocks,
    /// "Whenever an opponent searches their library" (Wan Shi Tong).
    OpponentSearchesLibrary,
    /// "Whenever you put one or more counters on a permanent or player": once for each permanent or
    /// player, the number put in the trigger's amount (All Will Be One). "You" is the controller of
    /// the resolving spell or ability, else the permanent's controller.
    YouPutCounters,
    /// "Whenever you cast a multicolored spell" (Lobber Crew), every one.
    YouCastMulticolored,
    /// "Whenever an opponent is dealt noncombat damage" (Chandra's Spitfire), from any source.
    OpponentDealtNoncombatDamage,
    /// An Aura's "Whenever enchanted creature deals damage to an opponent" (Curiosity, Ophidian
    /// Eye), combat or not.
    EnchantedDealsDamageToOpponent,
    /// "Whenever a permanent you control enters tapped" (Amulet of Vigor): checked as triggers are
    /// next put on the stack, so "enters tapped" set by the effect that put it there counts. Slot 0
    /// (if `triggering`) is that permanent.
    PermanentYouControlEntersTapped,
    /// "Whenever a nonartifact permanent you control enters" (Cloudstone Curio); slot 0 (if
    /// `triggering`) is that permanent.
    NonartifactPermanentYouControlEnters,
    /// "Whenever an artifact you control enters" (Teething Wurmlet); slot 0 (if `triggering`) is
    /// that artifact.
    ArtifactYouControlEnters,
    /// "Whenever this creature attacks for the first time each turn" (Aurelia).
    AttacksFirstTimeEachTurn,
    /// "Whenever you sacrifice a nontoken permanent" (Skittering Precursor); slot 0 (if
    /// `triggering`) is that card.
    YouSacrificeNontoken,
    /// "Whenever an opponent sacrifices a nontoken permanent" (It That Betrays); slot 0 (if
    /// `triggering`) is that card where it went.
    OpponentSacrificesNontoken,
    /// "Whenever an opponent draws a card except the first one they draw in each of their draw
    /// steps" (Orcish Bowmasters).
    OpponentDrawsExceptFirstInDrawStep,
    /// "Whenever another colorless creature you control enters" (Glaring Fleshraker); slot 0 (if
    /// `triggering`) is that creature.
    AnotherColorlessCreatureYouControlEnters,
    /// "Whenever you cast a colorless spell with mana value N or greater" (Sanctum of Ugin: 7); the
    /// spell in `extra`.
    YouCastColorlessMvAtLeast(u32),
    /// "Whenever one or more creature cards are put into your graveyard from anywhere, ... This
    /// ability triggers only once each turn." (Sefris; tokens aren't cards).
    CreatureCardsToYourGraveyardOnceEachTurn,
    /// "Whenever another source you control deals exactly 1 damage to a permanent or player"
    /// (Ghyrson Starn); slot 0 (if `triggering`) is that permanent or player.
    AnotherSourceYouControlDealsExactlyOne,
    /// "Whenever one or more creatures an opponent controls attack you and aren't blocked"
    /// (Coveted Jewel), once blockers are declared.
    OpponentCreaturesAttackYouUnblocked,
    /// "Whenever you complete a dungeon" (Sefris; CR 309.7).
    YouCompleteDungeon,
    /// "Whenever another creature you control with flying enters" (Watcher of the Spheres); slot 0
    /// (if `triggering`) is that creature.
    AnotherCreatureWithFlyingYouControlEnters,
    /// "Whenever a [subtype] deals damage to you" (Mikaeus, the Unhallowed: Human), any source with
    /// the subtype; slot 0 (if `triggering`) is that source ("destroy it").
    SubtypeDealsDamageToYou(&'static str),
    /// "Whenever a [subtype] you control enters" (Dragon Tempest: Dragon), tokens included; slot 0
    /// (if `triggering`) is that permanent ("it deals ...").
    SubtypeYouControlEnters(&'static str),
    /// The delayed "when target creature is put into your graveyard this turn" of
    /// `Effect::DelayedWhenSlotToYourGraveyardThisTurn` (Saffi Eriksdotter); slot 0 (if
    /// `triggering`) is that card in the graveyard. Never fired by `fire()`.
    SlotCreatureToYourGraveyardThisTurn,
    /// "Whenever you cast an instant or sorcery spell", with that spell's mana value (X included) as
    /// the trigger's amount and the spell in `extra` (Metallurgic Summonings).
    YouCastInstantOrSorceryWithMv,
    /// "Whenever enchanted creature attacks" (Martial Impetus).
    EnchantedAttacks,
    /// "Whenever one or more [subtype]s you control deal combat damage to a player" (The Odd Acorn
    /// Gang: Squirrels).
    YourSubtypeDealCombatDamageToPlayer(&'static str),
    /// "When this Class becomes level N" (CR 716.2c).
    ClassBecomesLevel(u8),
    /// "When this creature is turned face up" (morph, manifest; Rattleclaw Mystic).
    TurnedFaceUp,
    /// "Whenever an artifact you control is put into a graveyard from the battlefield" (Marionette
    /// Master), its last controller's, this one included.
    ArtifactYouControlToGraveyardFromBattlefield,
    /// "Whenever another creature or artifact you control is put into a graveyard from the
    /// battlefield" (Marionette Apprentice).
    AnotherCreatureOrArtifactYouControlToGraveyardFromBattlefield,
    /// "When this [permanent] is put into a graveyard from the battlefield", tokens included (the
    /// Wicked Role).
    ThisPutIntoGraveyardFromBattlefield,
    /// "Whenever an opponent sacrifices a noncreature token" (Bloodroot Apothecary), fired from
    /// `Game::sacrificed`.
    OpponentSacrificesNoncreatureToken,
    /// "Whenever an Aura you control enters, if you cast it" (Light-Paws): fired as an Aura spell
    /// resolves onto the battlefield (`Game::cast_aura_entered`, game/zones.rs), with the Aura in a
    /// triggering first slot.
    CastAuraYouControlEnters,
    /// "Whenever [this] deals combat damage to a player for the first time each turn" (Raphael, Tag
    /// Team Tough), with the damage as the amount; see `Game::combat_damaged_player`.
    DealsCombatDamageToPlayerFirstTimeEachTurn,
    /// "Whenever you play a card from exile" (Prosper, Tome-Bound): a land played or a spell cast
    /// from exile, fired by `Game::played_from_exile` (game/zones.rs).
    YouPlayCardFromExile,
    /// "Whenever a [subtype] you control attacks" (Utvara Hellkite: Dragon), this one included; slot
    /// 0 (if `triggering`) is the attacker.
    SubtypeYouControlAttacks(&'static str),
    /// "Whenever a [subtype] is put into a graveyard from the battlefield" (Ygra, Eater of All: a
    /// Food), anyone's, tokens included; slot 0 (if `triggering`) is that card. The subtype is
    /// checked on the card in the graveyard.
    SubtypeToGraveyardFromBattlefield(&'static str),
    /// "When this [permanent] enters untapped" (Witch's Cottage): checked with the "enters tapped"
    /// triggers, once its enter replacements have applied.
    EntersUntapped,
    /// "Whenever the Ring tempts you" (CR 701.54), fired once the Ring-bearer is chosen or asked
    /// for.
    RingTemptsYou,
    /// "Whenever you choose a creature as your Ring-bearer" (Call of the Ring), the same one again
    /// included (CR 701.54c); slot 0 (if `triggering`) is that creature.
    YouChooseRingBearer,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trigger {
    pub when: When,
    pub ability: Ability,
}

/// A dungeon card (CR 309), in the order `game::d4::rooms` lists them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dungeon {
    /// Lost Mine of Phandelver.
    LostMine,
    /// Dungeon of the Mad Mage.
    MadMage,
    /// Tomb of Annihilation.
    Tomb,
    /// Undercity: only by venturing into Undercity (the initiative, CR 725.2).
    Undercity,
    /// Not a dungeon: the Ring emblem's triggered abilities (CR 701.54c), carried as
    /// `StackKind::Room` rows of `rooms(TheRing)` so they need no stack kind of their own. Nothing
    /// ventures into it.
    TheRing,
}

/// The d4 subsystems' effects ([`Effect::Subsystem`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubsystemEffect {
    /// "Venture into the dungeon" (CR 701.49a).
    Venture,
    /// "Venture into Undercity" (CR 701.49b, the initiative).
    VentureUndercity,
    /// "You take the initiative" (CR 725.1).
    TakeInitiative,
    /// A room ability's tokens: `n` of the token definition with this name (and P/T, for a
    /// creature token).
    RoomToken {
        name: &'static str,
        power: i32,
        toughness: i32,
        n: u8,
    },
    /// "Roll a d`sides`. You create a number of [tokens] equal to the result." (CR 706)
    RollDieCreateTokens { sides: u32, def: usize },
    /// "Target [player] gets an emblem with ..." (the player in target slot `slot`).
    EmblemForTarget { slot: u8, kind: EmblemKind },
    /// The end of a bottommost room's ability: the dungeon is completed once the last copy of it
    /// leaves the stack (CR 704.5t).
    CompleteDungeon,
    /// "This Saga gains ..." (Urza's Saga): the source has its chapter-`n` grants from now on, while
    /// it stays on the battlefield (`SubsystemExt::chapter_activated`, `chapter_mana`).
    GainChapterAbilities(u8),
    /// "Draw another card if you've completed a dungeon." (Imoen): `n` cards then.
    DrawIfCompletedDungeon(u8),
    /// "Create `n` [tokens]. Create `completed` of those tokens instead if you've completed a
    /// dungeon." (Safana: Treasures, 1 or 3)
    TokensByDungeon { def: usize, n: u8, completed: u8 },
    /// "Untap a creature you control. If you've completed a dungeon, untap all creatures you
    /// control instead." (White Plume Adventurer), as `Decision::UntapOwnCreature`.
    UntapOneOrAllByDungeon,
    /// "If that player lost less than `n` life this turn, they lose life equal to the difference."
    /// (Lolth's emblem, on the damaged opponent: the one opponent in 1v1)
    OpponentLosesDifferenceFrom(i32),
    /// "Flip a coin. If you lose the flip, this [source] deals `n` damage to you." (Mana Crypt)
    FlipCoinLoseDamageYou(i32),
    /// "The Ring tempts you" (CR 701.54a): choose your Ring-bearer, and your Ring emblem gains its
    /// next ability.
    RingTempts,
    /// The Ring's third ability: the blocker in the item's `extra` is sacrificed by its controller
    /// at end of combat, if it's still blocking.
    RingSacrificeBlocker,
    /// "Create a token that's a copy of [the creature in `slot`], except it's 1/1. The token enters
    /// tapped and attacking [that player]. Sacrifice it at the beginning of the next end step."
    /// (Echoing Assault; that player is the one opponent in 1v1.)
    AttackingOneOneCopy(u8),
    /// "Equip commander" (Commander's Plate): attach the source to a commander creature its
    /// controller controls. The ability is offered only while there is one
    /// (`Game::lacks_chapter_activated`), so it takes no target slot.
    AttachToYourCommander,
}
