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
    /// Each creature and each planeswalker.
    EachCreatureAndPlaneswalker,
    /// The ability's controller ("deals 1 damage to you": City of Brass).
    You,
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
    /// Put a counter of these keywords on the source (Mondrak: an indestructible counter).
    KeywordCounterSelf(crate::Keywords),
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
    /// A creature card with exactly this mana value (Birthing Pod).
    CreatureMvExactly(u32),
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
}

#[derive(Clone, Debug, PartialEq)]
pub struct Trigger {
    pub when: When,
    pub ability: Ability,
}
