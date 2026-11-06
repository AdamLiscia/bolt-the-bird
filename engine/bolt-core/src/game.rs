//! The game: state, legal actions, and the only function that changes state, [`Game::apply`].
//!
//! Rules are cited by Comprehensive Rules (CR) section. Simplifications are listed in
//! docs/engine/QUESTIONS.md, each with the reason and how to reverse it.

use std::sync::Arc;

use crate::card::{CardDef, ExtraCost, Keywords, Reduction};
use crate::effect::{EmblemKind, LookRest, LookTo, Pick, Scale, Scope, Static, StaticCondition};
use std::collections::VecDeque;

use crate::effect::{
    Ability, Condition, Controller, Effect, Filter, Recipient, SearchFor, SearchTo, When, Who,
};
use crate::mana::{self, ManaCost, ManaSource};
use crate::rng::Rng;

pub type PlayerId = usize;

/// Starting life total (CR 103.4).
pub const STARTING_LIFE: i32 = 20;
/// Opening hand size (CR 103.5).
pub const HAND_SIZE: usize = 7;
/// Maximum hand size, enforced in the cleanup step (CR 402.2, 514.1).
pub const MAX_HAND: usize = 7;
/// The most work (zone changes and state-based-action passes) one action may do. Real actions
/// stay far below it, even with a thousand permanents; reaching it means an engine loop that never
/// ends, so the action panics rather than eat the machine's memory.
pub const WORK_BUDGET: u64 = 1_000_000;

/// A game object: one physical card, wherever it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Zone {
    Library,
    Hand,
    Battlefield,
    Graveyard,
    Stack,
    Exile,
    /// Tokens that left the battlefield cease to exist (CR 111.7, 704.5d).
    Gone,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    pub id: ObjId,
    /// Index into [`Game::defs`].
    pub def: usize,
    pub owner: PlayerId,
    pub controller: PlayerId,
    pub zone: Zone,
    pub tapped: bool,
    /// Damage marked this turn (CR 120.3e). Removed in cleanup (CR 514.2).
    pub damage: i32,
    /// Dealt damage by a source with deathtouch since the last check (CR 704.5h).
    pub deathtouched: bool,
    /// The turn this object came under its controller's control on the battlefield. A creature can
    /// attack only if this is before the current turn, or it has haste (CR 302.6).
    pub since_turn: u32,
    /// Zone changes so far. A card that changes zones is a new object (CR 400.7), so a target
    /// remembers this number and stops matching once it changes.
    pub zcc: u32,
    /// +1/+1 counters (CR 122.1a).
    pub counters: i32,
    /// Cast with kicker; remembered while it's on the stack and the battlefield.
    pub kicked: bool,
    /// X as cast, remembered like kicker.
    pub x: u8,
    /// The object this Equipment or Aura is attached to, pinned to its zone-change count.
    pub attached_to: Option<(ObjId, u32)>,
    /// Power when it last left the battlefield (last known information, CR 608.2h).
    pub last_power: i32,
    /// Stun counters: each one replaces an untap (CR 122.1d).
    pub stun: i32,
    /// The turn a once-each-turn "counters put on this" trigger last fired.
    pub counter_trigger_turn: u32,
    /// Incubation counters.
    pub incubation: i32,
    /// Cast from its owner's hand (for "cast spells from your hand" effects).
    pub from_hand: bool,
    /// Exiled instead if it would die during this turn, while it is this object: (zcc, turn).
    pub exile_if_dies: Option<(u32, u32)>,
    /// Creature types it has gained, a type that replaced its printed ones, and keywords gained for
    /// as long as it's on the battlefield.
    pub extra_subtypes: Vec<&'static str>,
    pub subtype_override: Option<&'static str>,
    pub granted: Keywords,
    /// Gained types when it last left the battlefield (last known information).
    pub last_subtypes: Vec<&'static str>,
    /// An "activate only once" ability has been used.
    pub used_once: bool,
    /// Flashback granted until end of turn.
    pub flashback_eot: bool,
    /// Soul counters, and how many it had when it last left the battlefield.
    pub soul: i32,
    pub last_soul: i32,
    /// Revival counters, and how many when it last left the battlefield.
    pub revival: i32,
    pub last_revival: i32,
    /// The color chosen as it entered (index into WUBRG).
    pub chosen_color: Option<u8>,
    /// The creature type chosen as it entered (CR 614.12).
    pub chosen_type: Option<String>,
    pub fellowship: i32,
    /// Who may play this card from exile, and through which turn.
    pub playable: Option<(PlayerId, u32)>,
    /// Bait counters (Fishing Pole) and a stash counter (Tinybones).
    pub bait: i32,
    pub stash: bool,
    /// A token that's a copy of a card (its definition isn't a token's).
    pub copy_token: bool,
    /// Sacrifice it at the beginning of the next end step (Electroduplicate).
    pub sacrifice_at_end: bool,
    /// The turn it was last put into a graveyard.
    pub graveyard_turn: u32,
    /// Loyalty (CR 306.5), and the turn a loyalty ability was last activated (CR 606.3).
    pub loyalty: i32,
    pub loyalty_turn: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub life: i32,
    /// Top of the library is the last element.
    pub library: Vec<ObjId>,
    pub hand: Vec<ObjId>,
    pub graveyard: Vec<ObjId>,
    pub lands_played: u8,
    pub mulligans: u8,
    pub drew_from_empty: bool,
    pub lost: bool,
    /// Mana added by an effect, spent before any permanent is tapped; empties at the end of each
    /// step and phase (CR 106.4, 500.4).
    pub pool: Vec<ManaSource>,
}

impl Player {
    fn new() -> Self {
        Player {
            life: STARTING_LIFE,
            library: Vec::new(),
            hand: Vec::new(),
            graveyard: Vec::new(),
            lands_played: 0,
            mulligans: 0,
            pool: Vec::new(),
            drew_from_empty: false,
            lost: false,
        }
    }
}

/// Steps and phases of a turn (CR 500-514).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Step {
    /// Before turn 1: opening hands and mulligans.
    PreGame,
    Untap,
    Upkeep,
    Draw,
    Main1,
    BeginCombat,
    DeclareAttackers,
    DeclareBlockers,
    FirstStrikeDamage,
    CombatDamage,
    EndCombat,
    Main2,
    End,
    Cleanup,
}

/// Whose decision the game is waiting on, and what kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pending {
    Mulligan(PlayerId),
    /// After keeping, put this many cards from hand on the bottom (CR 103.5, London mulligan).
    Bottom(PlayerId, u8),
    Priority(PlayerId),
    DeclareAttackers(PlayerId),
    DeclareBlockers(PlayerId),
    /// Discard down to the maximum hand size (CR 514.1).
    Discard(PlayerId),
    /// Choose the next target of the spell or ability being put on the stack (CR 601.2c, 603.3d).
    ChooseTarget(PlayerId),
    /// A decision in the middle of a resolving spell or ability; see [`Game::decision`].
    Decide(PlayerId),
    /// Divide the damage of the spell or ability being put on the stack: how much the next target
    /// gets (CR 601.2d).
    Divide(PlayerId),
    GameOver,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    Keep,
    Mulligan,
    Bottom(ObjId),
    PassPriority,
    PlayLand(ObjId),
    Cast(ObjId),
    /// Activate ability number `n` of a permanent (CR 602).
    Activate(ObjId, usize),
    /// Activate, sacrificing this other creature as part of the cost.
    ActivateWith(ObjId, usize, ObjId),
    /// Cast with its kicker paid (CR 702.33).
    CastKicked(ObjId),
    /// Cast paying an additional cost with this creature (sacrifice) or card (discard).
    CastWith(ObjId, ObjId),
    /// Cast from the graveyard for its flashback cost (CR 702.34).
    CastFlashback(ObjId),
    /// Cast a modal spell choosing mode `n` (CR 700.2a).
    CastModal(ObjId, u8),
    /// Cast paying its alternative cost (CR 118.9).
    CastAlternative(ObjId),
    /// Cast with X chosen (CR 601.2b).
    CastX(ObjId, u8),
    /// Attack a planeswalker instead of the defending player (CR 508.1b).
    AttackPlaneswalker(ObjId, ObjId),
    /// Activate loyalty ability number `n` (CR 606).
    Loyalty(ObjId, usize),
    /// Activate an ability with {X} in its cost, optionally sacrificing another creature.
    ActivateX {
        id: ObjId,
        n: usize,
        victim: Option<ObjId>,
        x: u8,
    },
    Target(Target),
    Yes,
    No,
    /// Choose a card for the current decision (a search, or a card from a revealed hand).
    Pick(ObjId),
    /// Choose mode `n` of a modal triggered ability.
    Mode(u8),
    Attack(ObjId),
    FinishAttacks,
    Block {
        blocker: ObjId,
        attacker: ObjId,
    },
    /// Undo every block declared so far this step. Keeps menace from dead-ending a declaration.
    ResetBlocks,
    FinishBlocks,
    Discard(ObjId),
    /// How much of a divided amount the next target gets.
    Amount(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Player(PlayerId),
    Object(ObjId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    TurnBegan {
        turn: u32,
        active: PlayerId,
    },
    Drew {
        player: PlayerId,
        card: ObjId,
    },
    Moved {
        card: ObjId,
        from: Zone,
        to: Zone,
    },
    Cast {
        card: ObjId,
    },
    Resolved {
        card: ObjId,
    },
    /// A spell or ability left the stack without resolving because every target was illegal
    /// (CR 608.2b).
    Fizzled {
        source: ObjId,
    },
    Triggered {
        source: ObjId,
    },
    Damage {
        source: ObjId,
        target: Target,
        amount: i32,
    },
    LifeGained {
        player: PlayerId,
        amount: i32,
    },
    LifeLost {
        player: PlayerId,
        amount: i32,
    },
    Lost {
        player: PlayerId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Winner(PlayerId),
    Draw,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RulesError {
    Illegal(Action),
    GameOver,
}

impl std::fmt::Display for RulesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RulesError::Illegal(a) => write!(f, "illegal action: {a:?}"),
            RulesError::GameOver => write!(f, "the game is over"),
        }
    }
}

impl std::error::Error for RulesError {}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Combat {
    pub attackers: Vec<ObjId>,
    /// (blocker, attacker) in declaration order.
    pub blocks: Vec<(ObjId, ObjId)>,
    pub first_strike_step: bool,
    /// Attackers attacking a planeswalker instead of the player: (attacker, planeswalker).
    pub at_planeswalker: Vec<(ObjId, ObjId)>,
}

impl Combat {
    pub fn is_blocked(&self, attacker: ObjId) -> bool {
        self.blocks.iter().any(|&(_, a)| a == attacker)
    }

    pub fn blockers_of(&self, attacker: ObjId) -> impl Iterator<Item = ObjId> + '_ {
        self.blocks
            .iter()
            .filter(move |&&(_, a)| a == attacker)
            .map(|&(b, _)| b)
    }

    pub fn is_blocking(&self, blocker: ObjId) -> bool {
        self.blocks.iter().any(|&(b, _)| b == blocker)
    }
}

/// What a stack entry is (CR 405).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackKind {
    Spell,
    /// Triggered ability number `n` of the source's card.
    Trigger(usize),
    /// Activated ability number `n` of the source's card.
    Activated(usize),
    /// A ward trigger of the source: counter the spell or ability from `against` unless its
    /// controller pays `cost` (CR 702.21a).
    Ward {
        cost: ManaCost,
        against: ObjId,
    },
    /// Ward—pay life.
    WardLife {
        life: i32,
        against: ObjId,
    },
    /// Loyalty ability number `n` of the source.
    Loyalty(usize),
}

/// The ability of a ward trigger, which has no targets or effects of its own.
static WARD_ABILITY: Ability = Ability {
    targets: Vec::new(),
    effects: Vec::new(),
    condition: None,
    optional: false,
    min_targets: None,
    same_graveyard: false,
    modes: Vec::new(),
    divide: 0,
};

/// A chosen target, pinned to the object it was when chosen (CR 400.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chosen {
    Player(PlayerId),
    Object(ObjId, u32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StackItem {
    pub source: ObjId,
    pub controller: PlayerId,
    pub kind: StackKind,
    pub targets: Vec<Chosen>,
    pub kicked: bool,
    /// Cast with flashback: exiled when it leaves the stack (CR 702.34a).
    pub flashback: bool,
    /// The creature sacrificed or card discarded for an additional cost.
    pub extra: Option<ObjId>,
    /// The chosen mode of a modal spell.
    pub mode: Option<u8>,
    /// For triggers that refer to "that much" or "that many": the amount.
    pub amount: i32,
    /// Cast for its alternative cost.
    pub alt: bool,
    /// The value chosen for X (CR 107.3).
    pub x: u8,
    /// How divided damage is split among the targets, in target order (CR 601.2d).
    pub division: Vec<u8>,
}

/// How a spell is being cast.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CastMode {
    pub x: u8,
    pub alt: bool,
    pub kicked: bool,
    pub flashback: bool,
    pub extra: Option<ObjId>,
    pub mode: Option<u8>,
}

impl CastMode {
    fn with(x: ObjId) -> CastMode {
        CastMode {
            extra: Some(x),
            ..CastMode::default()
        }
    }
}

/// A stack entry whose targets are still being chosen.
#[derive(Clone, Debug, PartialEq)]
pub struct Choosing {
    pub item: StackItem,
    /// Who gets priority once the entry is on the stack.
    pub then: PlayerId,
    pub cast: bool,
    /// For an activated ability: its costs, paid once targets are chosen (CR 602.2b), as
    /// (ability number, creature to sacrifice).
    pub pay: Option<(usize, Option<ObjId>)>,
    /// For a loyalty ability: its loyalty cost, paid once targets are chosen.
    pub loyalty: Option<(ObjId, i32)>,
}

/// Until-end-of-turn effects, kept per object so a lookup touches only that object's effects (a
/// board can hold thousands of creatures, each pumped by "creatures you control get +1/+1").
#[derive(Clone, Debug, Default)]
pub struct Modifiers(std::collections::HashMap<ObjId, Vec<Modifier>>);

impl Modifiers {
    pub fn push(&mut self, m: Modifier) {
        self.0.entry(m.obj).or_default().push(m);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// The effects on one object, in the order they were created.
    pub fn on(&self, id: ObjId) -> impl Iterator<Item = &Modifier> + '_ {
        self.0.get(&id).into_iter().flatten()
    }
}

/// A spell or ability part-way through resolving (CR 608.2).
#[derive(Clone, Debug, PartialEq)]
pub struct Resolving {
    pub item: StackItem,
    pub legal: Vec<bool>,
    pub remaining: VecDeque<Effect>,
    /// False if the spell fizzled or its condition failed: it does nothing.
    pub acts: bool,
}

/// A choice made while a spell or ability resolves.
#[derive(Clone, Debug, PartialEq)]
pub enum Decision {
    /// "You may": `Yes` or `No`.
    May,
    /// Discard `left` more cards. Optional ones also allow `No`, and `then_draw` draws after.
    Discard {
        left: u8,
        optional: bool,
        then_draw: bool,
    },
    /// Choose a nonland card from `victim`'s hand for them to discard.
    PickDiscard { victim: PlayerId },
    /// Surveil: `Yes` puts the top of `cards` into the graveyard, `No` keeps it on top.
    Surveil { cards: Vec<ObjId> },
    /// `Pick` a matching library card, or `No` to find nothing (CR 701.19b).
    Search { what: SearchFor, to: SearchTo, left: u8 },
    /// `Yes` for the top of the library, `No` for the bottom.
    Tuck { card: ObjId },
    /// Choose a mode (`Mode(n)`) for the modal trigger in `Game::mode_pending`.
    ChooseMode,
    /// `Pick` a creature to sacrifice, `left` more to go.
    SacrificeCreatures { left: u8 },
    /// Genesis Wave: `Pick` permanent cards with mana value at most `x` one at a time, `No` to stop;
    /// the rest of `cards` go to the graveyard.
    GenesisPick { cards: Vec<ObjId>, x: u32 },
    /// Choose a color for `card` with `Mode(n)`, n in WUBRG order.
    ChooseColor { card: ObjId },
    /// Choose a creature type for `card` from `options` (by index).
    ChooseCreatureType { card: ObjId, options: Vec<String> },
    /// Put the first of `rest` into pile A (`Yes`) or B (`No`).
    SplitPiles {
        rest: Vec<ObjId>,
        a: Vec<ObjId>,
        b: Vec<ObjId>,
        owner: PlayerId,
    },
    /// The opponent takes pile A (`Yes`) or B (`No`) for the owner's hand.
    ChoosePile {
        a: Vec<ObjId>,
        b: Vec<ObjId>,
        owner: PlayerId,
    },
    /// Keep one permanent of permanent type `ty` (`Pick`); `kept` so far.
    KeepOfType { ty: u8, kept: Vec<ObjId> },
    /// "You may pay {cost}": `Yes` (if affordable) or `No`.
    MayPay { cost: ManaCost },
    /// `Discard` a card, or `No` to lose `n` life.
    LoseOrDiscard { n: i32, sacrifice: bool },
    /// Ward—pay life: `Yes` pays `life`, `No` lets the spell or ability from `against` be countered.
    PayWardLife { life: i32, against: ObjId },
    /// `Pick` another creature to sacrifice, or `No`.
    SacrificeAnother { source: ObjId },
    /// `Pick` one of `cards` that matches, or `No`; then `cards` that weren't picked go to `rest`.
    LookPick {
        cards: Vec<ObjId>,
        pick: Pick,
        to: LookTo,
        rest: Option<LookRest>,
    },
    /// Ward: `Yes` pays `cost`, `No` lets the spell or ability from `against` be countered.
    PayWard { cost: ManaCost, against: ObjId },
}

/// An until-end-of-turn change to one object (CR 611.2, 613).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Modifier {
    pub obj: ObjId,
    pub zcc: u32,
    pub power: i32,
    pub toughness: i32,
    pub keywords: Keywords,
    /// A granted "when this creature dies" trigger: (card that has the trigger, its index, controller).
    pub dies: Option<(ObjId, usize, PlayerId)>,
}

#[derive(Clone, Debug)]
pub struct Game {
    pub defs: Arc<Vec<CardDef>>,
    pub objects: Vec<Object>,
    pub players: [Player; 2],
    pub battlefield: Vec<ObjId>,
    /// Cards on the stack (the zone). The stack's order and abilities are in `items`.
    pub stack: Vec<ObjId>,
    /// The stack, bottom first: spells and abilities (CR 405.1).
    pub items: Vec<StackItem>,
    pub exile: Vec<ObjId>,
    /// Tokens that have ceased to exist.
    pub gone: Vec<ObjId>,
    /// Whether each player attacked with a creature this turn (raid).
    pub attacked_this_turn: [bool; 2],
    /// Life-gain events and cards drawn by each player this turn.
    pub life_gains_this_turn: [u32; 2],
    pub draws_this_turn: [u32; 2],
    /// Morbid: whether a creature died this turn.
    pub creature_died_this_turn: bool,
    /// Whether each player lost life this turn.
    pub life_lost_this_turn: [bool; 2],
    /// True while combat damage is being dealt.
    pub in_combat_damage: bool,
    /// Cards to return at the beginning of the next end step: (card, its zone-change count, revival
    /// counters to return with).
    pub delayed_returns: Vec<(ObjId, u32, i32)>,
    /// Set by "end the turn" while it resolves.
    pub end_turn: bool,
    /// The last token created, for "other" in the same effect.
    pub last_created: Vec<ObjId>,
    /// Permanents controlled until end of turn: (object, zcc, who controlled it before).
    pub control_until_eot: Vec<(ObjId, u32, PlayerId)>,
    /// Work done by the current action (zone changes and state-based-action passes). An engine
    /// bug that loops forever would otherwise grow the event list until the machine runs out of
    /// memory; past [`WORK_BUDGET`] the action panics instead.
    pub work: u64,
    /// How many permanents with a "loses all abilities" static are on the battlefield. Zero almost
    /// always, which lets ability-loss checks skip a battlefield scan.
    pub ability_loss_sources: u32,
    /// Permanents on the battlefield whose cards have static abilities, so static lookups don't scan
    /// the whole battlefield (a board can hold hundreds of tokens).
    pub static_sources: Vec<ObjId>,
    /// Creatures with combat damage to them prevented this turn.
    pub combat_damage_prevented: Vec<(ObjId, u32)>,
    /// Life gained by each player this turn.
    pub life_gained_this_turn: [i32; 2],
    /// Players with extra turns to take, last first (CR 500.7).
    pub extra_turns: Vec<PlayerId>,
    /// Emblems: (owner, kind, the card that made it) (CR 114).
    pub emblems: Vec<(PlayerId, EmblemKind, ObjId)>,
    /// Triggered abilities waiting to be put on the stack (CR 603.3).
    pub triggered: Vec<StackItem>,
    pub choosing: Option<Choosing>,
    /// The spell or ability that is resolving, while it waits on a decision.
    pub resolving: Option<Resolving>,
    /// The decision a resolving spell or ability is waiting on.
    pub decision: Option<Decision>,
    /// A modal trigger waiting for its mode, and who gets priority afterwards.
    pub mode_pending: Option<(StackItem, PlayerId)>,
    pub until_eot: Modifiers,
    /// "Until this leaves the battlefield" exiles: (source, its zone-change count, exiled card).
    pub links: Vec<(ObjId, u32, ObjId)>,
    pub turn: u32,
    pub active: PlayerId,
    /// The player who took the first turn.
    pub first: PlayerId,
    pub step: Step,
    pub pending: Pending,
    pub combat: Option<Combat>,
    pub result: Option<Outcome>,
    /// Pass priority automatically when passing is the only legal action (on by default). Off is for
    /// tests and step-by-step viewers that want to see every priority stop.
    pub auto_pass: bool,
    /// Consecutive priority passes with no action between them (CR 117.4).
    passes: u8,
    rng: Rng,
    events: Vec<Event>,
}

pub fn opponent(p: PlayerId) -> PlayerId {
    1 - p
}

impl Game {
    /// Starts a game: shuffles both decks, draws opening hands, and asks `first` about a mulligan.
    /// `decks` hold indices into `defs`.
    pub fn new(defs: Arc<Vec<CardDef>>, decks: [&[usize]; 2], seed: u64, first: PlayerId) -> Game {
        let mut g = Game::empty(defs, seed, first);
        for (p, deck) in decks.iter().enumerate() {
            for &d in deck.iter() {
                g.add_card(p, d, Zone::Library);
            }
            let mut lib = std::mem::take(&mut g.players[p].library);
            g.rng.shuffle(&mut lib);
            g.players[p].library = lib;
            for _ in 0..HAND_SIZE {
                g.draw(p);
            }
        }
        g.events.clear();
        g.pending = Pending::Mulligan(first);
        g
    }

    /// A game with no cards, for building test positions with [`Game::add_card`] and
    /// [`Game::start_turn`].
    pub fn empty(defs: Arc<Vec<CardDef>>, seed: u64, first: PlayerId) -> Game {
        Game {
            defs,
            objects: Vec::new(),
            players: [Player::new(), Player::new()],
            battlefield: Vec::new(),
            stack: Vec::new(),
            items: Vec::new(),
            exile: Vec::new(),
            gone: Vec::new(),
            attacked_this_turn: [false; 2],
            life_gains_this_turn: [0; 2],
            draws_this_turn: [0; 2],
            creature_died_this_turn: false,
            life_lost_this_turn: [false; 2],
            in_combat_damage: false,
            delayed_returns: Vec::new(),
            end_turn: false,
            last_created: Vec::new(),
            control_until_eot: Vec::new(),
            work: 0,
            ability_loss_sources: 0,
            static_sources: Vec::new(),
            combat_damage_prevented: Vec::new(),
            life_gained_this_turn: [0; 2],
            extra_turns: Vec::new(),
            emblems: Vec::new(),
            triggered: Vec::new(),
            choosing: None,
            resolving: None,
            decision: None,
            mode_pending: None,
            until_eot: Modifiers::default(),
            links: Vec::new(),
            turn: 0,
            active: first,
            first,
            step: Step::PreGame,
            pending: Pending::Mulligan(first),
            combat: None,
            result: None,
            auto_pass: true,
            passes: 0,
            rng: Rng::new(seed),
            events: Vec::new(),
        }
    }

    /// Creates a card owned by `owner` in `zone`. Library cards go on top. Permanents created on the
    /// battlefield have been there since before turn 1, so they can attack.
    pub fn add_card(&mut self, owner: PlayerId, def: usize, zone: Zone) -> ObjId {
        assert!(def < self.defs.len(), "no card definition {def}");
        if zone == Zone::Battlefield && self.defs[def].statics.iter().any(|s| s.lose_abilities) {
            self.ability_loss_sources += 1;
        }
        if zone == Zone::Battlefield && !self.defs[def].statics.is_empty() {
            self.static_sources.push(ObjId(self.objects.len() as u32));
        }
        let id = ObjId(self.objects.len() as u32);
        self.objects.push(Object {
            id,
            def,
            owner,
            controller: owner,
            zone,
            tapped: false,
            damage: 0,
            deathtouched: false,
            since_turn: 0,
            zcc: 0,
            counters: 0,
            kicked: false,
            x: 0,
            attached_to: None,
            last_power: 0,
            stun: 0,
            counter_trigger_turn: 0,
            incubation: 0,
            from_hand: false,
            exile_if_dies: None,
            extra_subtypes: Vec::new(),
            subtype_override: None,
            granted: Keywords::NONE,
            last_subtypes: Vec::new(),
            used_once: false,
            flashback_eot: false,
            soul: 0,
            last_soul: 0,
            revival: 0,
            last_revival: 0,
            chosen_color: None,
            chosen_type: None,
            fellowship: 0,
            playable: None,
            bait: 0,
            stash: false,
            copy_token: false,
            sacrifice_at_end: false,
            graveyard_turn: 0,
            loyalty: 0,
            loyalty_turn: 0,
        });
        self.zone_list(owner, zone).push(id);
        id
    }

    /// Jumps to the start of turn `turn` for `active`, skipping mulligans. For test positions.
    pub fn start_turn(&mut self, turn: u32, active: PlayerId) {
        self.turn = turn - 1;
        self.active = opponent(active);
        self.begin_next_turn();
        self.settle();
    }

    pub fn obj(&self, id: ObjId) -> &Object {
        &self.objects[id.0 as usize]
    }

    fn obj_mut(&mut self, id: ObjId) -> &mut Object {
        &mut self.objects[id.0 as usize]
    }

    pub fn def_of(&self, id: ObjId) -> &CardDef {
        &self.defs[self.obj(id).def]
    }

    fn modifiers(&self, id: ObjId) -> impl Iterator<Item = &Modifier> + '_ {
        let zcc = self.obj(id).zcc;
        self.until_eot.on(id).filter(move |m| m.zcc == zcc)
    }

    /// Keywords after granting effects (layer 6).
    /// An Aura's "loses all abilities" effect on `id`, if any, with its base power and toughness.
    /// Looked up directly (not through `statics_on`) so it can be used while computing statics.
    fn ability_loss(&self, id: ObjId) -> Option<(i32, i32)> {
        if self.ability_loss_sources == 0 {
            return None;
        }
        let zcc = self.obj(id).zcc;
        self.static_sources.iter().find_map(|&src| {
            if self.obj(src).attached_to != Some((id, zcc)) {
                return None;
            }
            self.def_of(src)
                .statics
                .iter()
                .find(|s| s.lose_abilities)
                .map(|s| s.base_pt.unwrap_or((0, 0)))
        })
    }

    fn lost_abilities(&self, id: ObjId) -> bool {
        self.obj(id).zone == Zone::Battlefield && self.ability_loss(id).is_some()
    }

    /// Whether `id` is a token: a token card, or a copy token (CR 111.1).
    pub fn is_token(&self, id: ObjId) -> bool {
        self.def_of(id).token || self.obj(id).copy_token
    }

    /// Printed or gained creature types, ignoring types from static abilities (used while working
    /// out static abilities, so they don't depend on each other).
    fn has_base_subtype(&self, id: ObjId, s: &str) -> bool {
        if self.lost_abilities(id) {
            return s == "Citizen"; // Witness Protection
        }
        let o = self.obj(id);
        if let Some(only) = o.subtype_override {
            return only == s;
        }
        self.def_of(id).subtypes.iter().any(|x| x == s) || o.extra_subtypes.contains(&s)
    }

    /// Whether `id` has creature type `s` (CR 205.3).
    pub fn has_subtype(&self, id: ObjId, s: &str) -> bool {
        self.has_base_subtype(id, s) || self.statics_on(id).any(|st| st.subtype == Some(s))
    }

    /// Static abilities on the battlefield that apply to `id` right now (CR 604.2, 611.3).
    fn statics_on(&self, id: ObjId) -> impl Iterator<Item = Static> + '_ {
        let o = self.obj(id);
        let on_battlefield = o.zone == Zone::Battlefield;
        let creature = self.def_of(id).types.creature;
        self.static_sources
            .iter()
            .filter(move |_| on_battlefield)
            .filter(move |&&src| !self.lost_abilities(src))
            .flat_map(move |&src| {
                let so = self.obj(src);
                self.def_of(src)
                    .statics
                    .iter()
                    .copied()
                    .filter(move |s| {
                        let mine = so.controller == o.controller;
                        let applies = match s.scope {
                            Scope::This => src == id,
                            Scope::YourCreatures => mine && creature,
                            Scope::YourCreaturesWithCounters => mine && creature && o.counters > 0,
                            Scope::Attached => so.attached_to == Some((id, o.zcc)),
                            // Printed or granted-until-end-of-turn flying; static grants are left out
                            // so statics don't depend on each other (no FDN static grants flying).
                            Scope::YourCreaturesOfChosenType => {
                                mine && creature
                                    && so
                                        .chosen_type
                                        .as_deref()
                                        .is_some_and(|t| self.has_base_subtype(id, t))
                            }
                            Scope::YourCreaturesOfChosenColor => {
                                mine && creature
                                    && so
                                        .chosen_color
                                        .is_some_and(|c| self.def_of(id).colors[c as usize])
                            }
                            Scope::OtherYourSubtype(sub) => {
                                mine && creature && src != id && self.has_base_subtype(id, sub)
                            }
                            Scope::OtherYourFliers => {
                                mine && creature
                                    && src != id
                                    && self
                                        .modifiers(id)
                                        .fold(self.def_of(id).keywords, |k, m| k.with(m.keywords))
                                        .has(Keywords::FLYING)
                            }
                        };
                        let condition_ok = match s.condition {
                            None => true,
                            Some(StaticCondition::Threshold) => {
                                self.players[so.controller].graveyard.len() >= 7
                            }
                            Some(StaticCondition::Untapped) => !so.tapped,
                            Some(StaticCondition::CountersAtLeast(n)) => so.counters >= n,
                            Some(StaticCondition::LifeAtLeast(n)) => self.players[so.controller].life >= n,
                        };
                        applies && condition_ok && (!s.your_turn_only || self.active == so.controller)
                    })
                    .map(move |mut s| {
                        if let Some(Scale::PerFellowshipCounter) = s.scale {
                            s.power *= so.fellowship;
                            s.toughness *= so.fellowship;
                        }
                        if let Some(Scale::PerYouControlSubtype(sub)) = s.scale {
                            let n = self
                                .permanents(so.controller)
                                .filter(|&x| self.has_base_subtype(x, sub))
                                .count() as i32;
                            s.power *= n;
                            s.toughness *= n;
                        }
                        s
                    })
            })
    }

    /// Keywords after granting effects (layer 6).
    pub fn keywords(&self, id: ObjId) -> Keywords {
        self.keywords_inner(id)
    }

    /// Colors `id` has protection from.
    pub fn protection(&self, id: ObjId) -> [bool; 5] {
        let mut p = [false; 5];
        for s in self.statics_on(id) {
            for (i, x) in s.protection.iter().enumerate() {
                p[i] |= *x;
            }
        }
        p
    }

    /// Whether `id` has protection from any of `source`'s colors.
    fn protected_from(&self, id: ObjId, source: ObjId) -> bool {
        if self.def_of(id).protection_from_everything
            && self.obj(id).zone == Zone::Battlefield
            && !self.lost_abilities(id)
        {
            return true;
        }
        let p = self.protection(id);
        self.def_of(source).colors.iter().zip(p).any(|(c, pr)| *c && pr)
    }

    fn keywords_inner(&self, id: ObjId) -> Keywords {
        let printed = if self.lost_abilities(id) {
            Keywords::NONE
        } else {
            self.def_of(id).keywords.with(self.obj(id).granted)
        };
        let mut k = self.modifiers(id).fold(printed, |k, m| k.with(m.keywords));
        if self.emblem_anthem(id) {
            k = k
                .with(Keywords::VIGILANCE)
                .with(Keywords::TRAMPLE)
                .with(Keywords::INDESTRUCTIBLE);
        }
        self.statics_on(id).fold(k, |k, s| k.with(s.keywords))
    }

    /// Power after counters, static effects and pump effects (layer 7).
    fn emblem_anthem(&self, id: ObjId) -> bool {
        let o = self.obj(id);
        o.zone == Zone::Battlefield
            && self.def_of(id).types.creature
            && self
                .emblems
                .iter()
                .any(|&(q, k, _)| q == o.controller && k == EmblemKind::VivienAnthem)
    }

    fn opponents_graveyard_bonus(&self, id: ObjId) -> i32 {
        let d = self.def_of(id);
        let mut b = if self.emblem_anthem(id) { 2 } else { 0 };
        if d.pt_opponents_graveyards {
            b += self.players[opponent(self.obj(id).controller)].graveyard.len() as i32;
        }
        if d.pt_per_graveyard_creature {
            let p = self.obj(id).controller;
            b += self.players[p]
                .graveyard
                .iter()
                .filter(|&&c| self.def_of(c).types.creature)
                .count() as i32;
        }
        b
    }

    pub fn power(&self, id: ObjId) -> i32 {
        self.ability_loss(id).map_or(self.def_of(id).power, |pt| pt.0)
            + self.opponents_graveyard_bonus(id)
            + self.obj(id).counters
            + self.modifiers(id).map(|m| m.power).sum::<i32>()
            + self.statics_on(id).map(|s| s.power).sum::<i32>()
    }

    pub fn toughness(&self, id: ObjId) -> i32 {
        self.ability_loss(id).map_or(self.def_of(id).toughness, |pt| pt.1)
            + self.opponents_graveyard_bonus(id)
            + self.obj(id).counters
            + self.modifiers(id).map(|m| m.toughness).sum::<i32>()
            + self.statics_on(id).map(|s| s.toughness).sum::<i32>()
    }

    /// Puts triggers of `p`'s permanents that match `when` into the waiting list.
    fn fire(&mut self, p: PlayerId, when: When, except: Option<ObjId>) {
        for id in self.permanents(p).collect::<Vec<_>>() {
            if Some(id) == except || self.lost_abilities(id) {
                continue;
            }
            let whens: Vec<When> = self.def_of(id).triggers.iter().map(|tr| tr.when).collect();
            for (n, w) in whens.into_iter().enumerate() {
                if w == when {
                    self.triggered.push(StackItem {
                        source: id,
                        controller: p,
                        kind: StackKind::Trigger(n),
                        targets: Vec::new(),
                        kicked: false,
                        flashback: false,
                        extra: None,
                        mode: None,
                        amount: 0,
                        alt: false,
                        x: 0,
                        division: Vec::new(),
                    });
                }
            }
        }
    }

    /// CR 119.3: losing life, with "whenever an opponent loses life" triggers.
    fn lose_life(&mut self, p: PlayerId, n: i32) {
        self.players[p].life -= n;
        self.life_lost_this_turn[p] = true;
        self.events.push(Event::LifeLost { player: p, amount: n });
        let before = self.triggered.len();
        self.fire(opponent(p), When::OpponentLosesLife, None);
        for t in &mut self.triggered[before..] {
            t.amount = n;
        }
    }

    /// CR 119.3: gaining life, with "first time each turn" triggers.
    fn gain_life(&mut self, p: PlayerId, n: i32) {
        self.players[p].life += n;
        self.events.push(Event::LifeGained { player: p, amount: n });
        self.life_gains_this_turn[p] += 1;
        self.life_gained_this_turn[p] += n;
        self.fire(p, When::YouGainLife, None);
        if self.life_gains_this_turn[p] == 1 {
            self.fire(
                p,
                When::YouGainLifeFirstTime {
                    your_turn_only: false,
                },
                None,
            );
            if self.active == p {
                self.fire(p, When::YouGainLifeFirstTime { your_turn_only: true }, None);
            }
        }
    }

    pub fn is_over(&self) -> bool {
        self.result.is_some()
    }

    /// Permanents on the battlefield controlled by `p`.
    pub fn permanents(&self, p: PlayerId) -> impl Iterator<Item = ObjId> + '_ {
        self.battlefield
            .iter()
            .copied()
            .filter(move |&id| self.obj(id).controller == p)
    }

    pub fn creatures(&self, p: PlayerId) -> impl Iterator<Item = ObjId> + '_ {
        self.permanents(p).filter(|&id| self.def_of(id).types.creature)
    }

    /// Events since the last call, oldest first.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    fn zone_list(&mut self, owner: PlayerId, zone: Zone) -> &mut Vec<ObjId> {
        match zone {
            Zone::Library => &mut self.players[owner].library,
            Zone::Hand => &mut self.players[owner].hand,
            Zone::Graveyard => &mut self.players[owner].graveyard,
            Zone::Battlefield => &mut self.battlefield,
            Zone::Stack => &mut self.stack,
            Zone::Exile => &mut self.exile,
            Zone::Gone => &mut self.gone,
        }
    }

    /// Moves a card between zones. Leaving the battlefield clears its per-permanent state (CR 400.7).
    /// Counts one unit of work toward [`WORK_BUDGET`].
    fn work(&mut self) {
        self.work += 1;
        assert!(
            self.work <= WORK_BUDGET,
            "runaway action: over {WORK_BUDGET} zone changes or state-based-action passes \
             (an engine loop that never ends)"
        );
    }

    fn move_to(&mut self, id: ObjId, to: Zone) {
        self.work();
        let (owner, from) = (self.obj(id).owner, self.obj(id).zone);
        // Replacement effects on going to a graveyard (CR 614.6).
        let mut to = to;
        let mut shuffle = false;
        if to == Zone::Graveyard {
            let o = self.obj(id);
            if self.def_of(id).shuffle_instead_of_graveyard {
                to = Zone::Library;
                shuffle = true;
            } else if from == Zone::Battlefield
                && self.def_of(id).types.creature
                && o.exile_if_dies == Some((o.zcc, self.turn))
            {
                to = Zone::Exile;
            }
        }
        if !self.def_of(id).statics.is_empty() {
            if from == Zone::Battlefield {
                self.static_sources.retain(|&x| x != id);
            }
            if to == Zone::Battlefield {
                self.static_sources.push(id);
            }
        }
        if self.def_of(id).statics.iter().any(|s| s.lose_abilities) {
            if from == Zone::Battlefield {
                self.ability_loss_sources -= 1;
            }
            if to == Zone::Battlefield {
                self.ability_loss_sources += 1;
            }
        }
        if from == Zone::Battlefield {
            let p = self.power(id);
            self.obj_mut(id).last_power = p;
        }
        let last_controller = self.obj(id).controller;
        let list = self.zone_list(owner, from);
        let pos = list.iter().position(|&x| x == id).expect("object is in its zone");
        list.remove(pos);
        self.zone_list(owner, to).push(id);
        let turn = self.turn;
        let o = self.obj_mut(id);
        o.zone = to;
        o.zcc += 1;
        if to != Zone::Exile {
            o.playable = None;
        }
        if to != Zone::Exile && to != Zone::Stack {
            o.stash = false;
        }
        if from == Zone::Battlefield {
            o.bait = 0;
            o.sacrifice_at_end = false;
        }
        if to == Zone::Graveyard {
            o.graveyard_turn = turn;
        }
        o.tapped = false;
        o.damage = 0;
        o.deathtouched = false;
        o.counters = 0;
        o.incubation = 0;
        if from == Zone::Battlefield {
            let mut last = std::mem::take(&mut o.extra_subtypes);
            last.extend(o.subtype_override.take());
            o.last_subtypes = last;
            o.granted = Keywords::NONE;
            o.used_once = false;
            o.last_soul = o.soul;
            o.last_revival = o.revival;
            o.revival = 0;
            o.chosen_color = None;
            o.chosen_type = None;
            o.fellowship = 0;
            o.soul = 0;
        }
        o.attached_to = None;
        if to != Zone::Battlefield && to != Zone::Stack {
            o.controller = o.owner;
        }
        if to == Zone::Battlefield {
            o.since_turn = turn;
        } else if to != Zone::Stack {
            o.kicked = false;
            o.x = 0;
        }
        if from == Zone::Battlefield {
            if let Some(c) = &mut self.combat {
                c.attackers.retain(|&a| a != id);
            }
        }
        if shuffle {
            let mut lib = std::mem::take(&mut self.players[owner].library);
            self.rng.shuffle(&mut lib);
            self.players[owner].library = lib;
        }
        self.events.push(Event::Moved { card: id, from, to });
        self.zone_triggers(id, from, to, last_controller);
    }

    /// CR 603.6a, 603.6c, 603.10a: enters and dies triggers.
    fn zone_triggers(&mut self, id: ObjId, from: Zone, to: Zone, last_controller: PlayerId) {
        let dies = from == Zone::Battlefield && to == Zone::Graveyard;
        if from == Zone::Hand && to == Zone::Graveyard {
            // A card going from hand to graveyard was discarded (CR 701.9).
            let owner = self.obj(id).owner;
            let now = self.obj(id).zcc;
            let before = self.triggered.len();
            self.fire(opponent(owner), When::OpponentDiscards, None);
            for t in &mut self.triggered[before..] {
                t.targets = vec![Chosen::Object(id, now)];
            }
        }
        if dies && self.def_of(id).types.creature {
            self.creature_died_this_turn = true;
            self.fire(last_controller, When::CreatureYouControlDies, Some(id));
            if !self.is_token(id) {
                let was = |g: &Game, s: &str| {
                    g.def_of(id).subtypes.iter().any(|x| x == s) || g.obj(id).last_subtypes.contains(&s)
                };
                let now = self.obj(id).zcc;
                for any in self.permanents(last_controller).collect::<Vec<_>>() {
                    let whens: Vec<When> = self.def_of(any).triggers.iter().map(|tr| tr.when).collect();
                    for (n, w) in whens.into_iter().enumerate() {
                        if let When::YourNontokenCreatureDiesWithout(sub) = w {
                            if !was(self, sub) {
                                self.triggered.push(StackItem {
                                    source: any,
                                    controller: last_controller,
                                    kind: StackKind::Trigger(n),
                                    targets: vec![Chosen::Object(id, now)],
                                    kicked: false,
                                    flashback: false,
                                    extra: None,
                                    mode: None,
                                    amount: 0,
                                    alt: false,
                                    x: 0,
                                    division: Vec::new(),
                                });
                            }
                        }
                    }
                }
                self.fire(
                    last_controller,
                    When::AnotherNontokenCreatureYouControlDies,
                    Some(id),
                );
                self.fire(0, When::AnotherNontokenCreatureDies, Some(id));
                self.fire(1, When::AnotherNontokenCreatureDies, Some(id));
            }
        }
        if to == Zone::Battlefield {
            // Replacement effects on entering (CR 614.1c-d).
            let def = self.def_of(id).clone();
            let controller_now = self.obj(id).controller;
            let opponent_forces_tapped = def.types.creature
                && self
                    .permanents(opponent(controller_now))
                    .any(|c| self.def_of(c).opponents_creatures_enter_tapped);
            if def.enters_tapped || opponent_forces_tapped {
                self.obj_mut(id).tapped = true;
            }
            if def.types.creature {
                self.fire(
                    opponent(controller_now),
                    When::CreatureOpponentControlsEnters,
                    None,
                );
            }
            if let Some(l) = def.loyalty {
                let l = l * self.doubling(self.obj(id).controller);
                self.obj_mut(id).loyalty = l; // CR 306.5b
            }
            if def.enters_with_x_counters {
                let x = self.obj(id).x as i32;
                self.add_counters(id, x);
            }
            // Giada: each other Angel you control enters with a counter per Angel already there.
            let controller = self.obj(id).controller;
            let bonus: i32 = self
                .permanents(controller)
                .filter(|&x| x != id)
                .filter_map(|x| self.def_of(x).other_subtype_enter_bonus)
                .filter(|&sub| self.has_base_subtype(id, sub))
                .map(|sub| {
                    self.permanents(controller)
                        .filter(|&x| x != id && self.has_base_subtype(x, sub))
                        .count() as i32
                })
                .sum();
            if bonus > 0 {
                let n = bonus * self.doubling(controller);
                self.obj_mut(id).counters += n;
            }
            if let Some((n, cond)) = def.enters_counters {
                let controller = self.obj(id).controller;
                let yes = match cond {
                    Condition::Kicked => self.obj(id).kicked,
                    Condition::Attacked => self.attacked_this_turn[controller],
                    Condition::Always => true,
                    Condition::OpponentLostLifeThisTurn => false,
                    Condition::SourceWasNot(_) => false,
                    Condition::HadRevival => false,
                    _ => false,
                };
                if yes {
                    let n = n * self.doubling(controller);
                    self.obj_mut(id).counters += n;
                }
            }
            if def.types.land {
                let controller = self.obj(id).controller;
                self.fire(controller, When::LandYouControlEnters, None);
            }
            if def.types.creature {
                let controller = self.obj(id).controller;
                let before = self.triggered.len();
                self.fire(controller, When::AnotherCreatureYouControlEnters, Some(id));
                // "that creature": fill in the entering creature for abilities that refer to it.
                let zcc = self.obj(id).zcc;
                for i in before..self.triggered.len() {
                    let refers = self
                        .ability_of(&self.triggered[i])
                        .targets
                        .first()
                        .is_some_and(|f| f.triggering);
                    if refers {
                        self.triggered[i].targets = vec![Chosen::Object(id, zcc)];
                    }
                }
                let subs = def.subtypes.clone();
                let entering_power = self.power(id);
                let entering_token = self.is_token(id);
                for any in self.permanents(controller).collect::<Vec<_>>() {
                    let whens: Vec<When> = self.def_of(any).triggers.iter().map(|tr| tr.when).collect();
                    for (n, w) in whens.into_iter().enumerate() {
                        let hit = match w {
                            When::NontokenYouControlEntersWithSubtype(sub) => {
                                !entering_token && subs.iter().any(|s| s == sub)
                            }
                            When::YourCreatureEntersWithPowerAtLeast(p) => entering_power >= p,
                            _ => false,
                        };
                        if hit {
                            self.triggered.push(StackItem {
                                source: any,
                                controller,
                                kind: StackKind::Trigger(n),
                                targets: Vec::new(),
                                kicked: false,
                                flashback: false,
                                extra: None,
                                mode: None,
                                amount: 0,
                                alt: false,
                                x: 0,
                                division: Vec::new(),
                            });
                        }
                    }
                }
                for other in self
                    .permanents(controller)
                    .filter(|&o| o != id)
                    .collect::<Vec<_>>()
                {
                    let whens: Vec<When> = self.def_of(other).triggers.iter().map(|tr| tr.when).collect();
                    for (n, w) in whens.into_iter().enumerate() {
                        if let When::AnotherYouControlEntersWithSubtype(sub) = w {
                            if subs.iter().any(|s| s == sub) {
                                self.triggered.push(StackItem {
                                    source: other,
                                    controller,
                                    kind: StackKind::Trigger(n),
                                    targets: Vec::new(),
                                    kicked: false,
                                    flashback: false,
                                    extra: None,
                                    mode: None,
                                    amount: 0,
                                    alt: false,
                                    x: 0,
                                    division: Vec::new(),
                                });
                            }
                        }
                    }
                }
            }
        }
        if from == Zone::Battlefield {
            // CR 610.3: cards exiled "until this leaves the battlefield" come back.
            let left_zcc = self.obj(id).zcc - 1;
            let back: Vec<ObjId> = self
                .links
                .iter()
                .filter(|&&(src, z, _)| src == id && z == left_zcc)
                .map(|&(_, _, card)| card)
                .collect();
            self.links.retain(|&(src, z, _)| !(src == id && z == left_zcc));
            for card in back {
                if self.obj(card).zone == Zone::Exile {
                    self.move_to(card, Zone::Battlefield);
                }
            }
        }
        if dies {
            // Granted "when this creature dies" triggers, from before it changed zones.
            let was = self.obj(id).zcc - 1;
            let granted: Vec<(ObjId, usize, PlayerId)> = self
                .until_eot
                .on(id)
                .filter(|m| m.zcc == was)
                .filter_map(|m| m.dies)
                .collect();
            let now = self.obj(id).zcc;
            for (card, n, controller) in granted {
                self.triggered.push(StackItem {
                    source: card,
                    controller,
                    kind: StackKind::Trigger(n),
                    targets: vec![Chosen::Object(id, now)],
                    kicked: false,
                    flashback: false,
                    extra: None,
                    mode: None,
                    amount: 0,
                    alt: false,
                    x: 0,
                    division: Vec::new(),
                });
            }
        }
        let enters = to == Zone::Battlefield;
        let controller = if dies {
            last_controller
        } else {
            self.obj(id).controller
        };
        let whens: Vec<When> = self.def_of(id).triggers.iter().map(|tr| tr.when).collect();
        for (n, when) in whens.into_iter().enumerate() {
            let own_death = when == When::CreatureYouControlDies && dies && self.def_of(id).types.creature;
            if (when == When::Dies && dies) || (when == When::Enters && enters) || own_death {
                self.triggered.push(StackItem {
                    source: id,
                    controller,
                    kind: StackKind::Trigger(n),
                    targets: Vec::new(),
                    kicked: false,
                    flashback: false,
                    extra: None,
                    mode: None,
                    amount: 0,
                    alt: false,
                    x: 0,
                    division: Vec::new(),
                });
            }
        }
    }

    /// Draws a card. Drawing from an empty library loses at the next check (CR 704.5b).
    fn draw(&mut self, p: PlayerId) {
        match self.players[p].library.last().copied() {
            Some(card) => {
                self.move_to(card, Zone::Hand);
                self.events.push(Event::Drew { player: p, card });
                self.draws_this_turn[p] += 1;
                self.fire(p, When::YouDraw, None);
                self.fire(opponent(p), When::OpponentDraws, None);
                if self.draws_this_turn[p] == 2 {
                    self.fire(p, When::YouDrawSecond, None);
                }
            }
            None => self.players[p].drew_from_empty = true,
        }
    }

    // ---------------------------------------------------------------- legal actions

    /// Every action the deciding player may take now. Empty only when the game is over.
    pub fn legal_actions(&self) -> Vec<Action> {
        match self.pending {
            Pending::GameOver => Vec::new(),
            Pending::Mulligan(p) => {
                let mut v = vec![Action::Keep];
                if (self.players[p].mulligans as usize) < HAND_SIZE {
                    v.push(Action::Mulligan);
                }
                v
            }
            Pending::Bottom(p, _) => self.players[p].hand.iter().map(|&c| Action::Bottom(c)).collect(),
            Pending::Discard(p) => self.players[p].hand.iter().map(|&c| Action::Discard(c)).collect(),
            Pending::Priority(p) => self.priority_actions(p),
            Pending::DeclareAttackers(p) => {
                let walkers: Vec<ObjId> = self
                    .permanents(opponent(p))
                    .filter(|&w| self.def_of(w).types.planeswalker)
                    .collect();
                let mut v: Vec<Action> = Vec::new();
                for c in self.attack_candidates(p) {
                    v.push(Action::Attack(c));
                    v.extend(walkers.iter().map(|&w| Action::AttackPlaneswalker(c, w)));
                }
                // CR 508.1d: creatures that attack each combat if able must be declared.
                if !self.attack_candidates(p).any(|c| self.def_of(c).must_attack) {
                    v.push(Action::FinishAttacks);
                }
                v
            }
            Pending::DeclareBlockers(p) => self.block_actions(p),
            Pending::Decide(p) => self.decision_actions(p),
            Pending::Divide(_) => {
                let c = self.choosing.as_ref().expect("dividing");
                let total = self.ability_of(&c.item).divide;
                let given: u8 = c.item.division.iter().sum();
                // Every target after this one still needs at least 1 (CR 601.2d).
                let after = (c.item.targets.len() - c.item.division.len() - 1) as u8;
                (1..=total - given - after).map(Action::Amount).collect()
            }
            Pending::ChooseTarget(_) => {
                let c = self.choosing.as_ref().expect("choosing targets");
                let filter = self.ability_of(&c.item).targets[c.item.targets.len()];
                let ability = self.ability_of(&c.item);
                let mut options = self.candidates(&filter, c.item.controller);
                // CR 115.3: one object can't be chosen twice; "other" excludes the source.
                options.retain(|&t| match t {
                    Target::Object(x) => {
                        let is_source = !self.slot_ok(&c.item, &filter, Target::Object(x));
                        let taken = c.item.targets.contains(&Chosen::Object(x, self.obj(x).zcc));
                        !is_source && !taken
                    }
                    Target::Player(_) => true,
                });
                if ability.same_graveyard {
                    if let Some(&Chosen::Object(first, _)) = c.item.targets.first() {
                        let owner = self.obj(first).owner;
                        options.retain(|&t| matches!(t, Target::Object(x) if self.obj(x).owner == owner));
                    }
                }
                let can_stop = c.item.targets.len() >= ability.min_targets.unwrap_or(usize::MAX);
                // A cost reduction that depends on the target: only offer targets you can afford.
                let reduces = matches!(
                    self.def_of(c.item.source).reduction,
                    Some(Reduction::IfTargetsTapped(_))
                );
                if c.cast && reduces && c.item.targets.is_empty() {
                    let it = &c.item;
                    let mode = CastMode {
                        x: it.x,
                        alt: it.alt,
                        kicked: it.kicked,
                        flashback: it.flashback,
                        extra: it.extra,
                        mode: it.mode,
                    };
                    options.retain(|&t| {
                        let chosen = match t {
                            Target::Player(q) => Chosen::Player(q),
                            Target::Object(x) => Chosen::Object(x, self.obj(x).zcc),
                        };
                        self.can_pay_for(
                            it.controller,
                            &self.spell_cost(it.controller, it.source, mode, &[chosen]),
                            None,
                            Some(it.source),
                        )
                    });
                }
                let mut v: Vec<Action> = options.into_iter().map(Action::Target).collect();
                if can_stop {
                    v.push(Action::No); // "up to": stop choosing
                }
                v
            }
        }
    }

    /// The ability a stack entry carries out.
    pub fn ability_of(&self, item: &StackItem) -> &Ability {
        let def = self.def_of(item.source);
        match item.kind {
            StackKind::Spell => match item.mode {
                Some(m) => &def.modes[m as usize],
                None if item.kicked && def.kicked_spell.is_some() => {
                    def.kicked_spell.as_ref().expect("kicked")
                }
                None => def.spell.as_ref().expect("a spell with an ability"),
            },
            StackKind::Trigger(n) => match item.mode {
                Some(m) => &def.triggers[n].ability.modes[m as usize],
                None => &def.triggers[n].ability,
            },
            StackKind::Ward { .. } | StackKind::WardLife { .. } => &WARD_ABILITY,
            StackKind::Loyalty(n) => &def.loyalty_abilities[n].1,
            StackKind::Activated(n) => &def.activated[n].ability,
        }
    }

    fn controller_ok(c: Controller, controller: PlayerId, of: PlayerId) -> bool {
        match c {
            Controller::Any => true,
            Controller::You => of == controller,
            Controller::Opponent => of != controller,
        }
    }

    /// Whether `t` is a legal target for `f` for an ability controlled by `controller` (CR 115).
    pub fn target_ok(&self, f: &Filter, controller: PlayerId, t: Target) -> bool {
        match t {
            Target::Player(p) => {
                f.players
                    && Game::controller_ok(f.controller, controller, p)
                    && (p == controller || !self.permanents(p).any(|x| self.def_of(x).player_hexproof))
            }
            Target::Object(id) => {
                let o = self.obj(id);
                if !Game::controller_ok(f.controller, controller, o.controller) {
                    return false;
                }
                if f.subtype.is_some_and(|s| !self.has_subtype(id, s))
                    || f.not_subtype.is_some_and(|s| self.has_subtype(id, s))
                {
                    return false;
                }
                if o.zone == Zone::Stack {
                    // A spell, already cast: one being cast isn't on the stack yet (CR 115.5).
                    let is_spell = self
                        .items
                        .iter()
                        .any(|i| i.kind == StackKind::Spell && i.source == id);
                    return f.spells
                        && is_spell
                        && f.spell_creature
                            .is_none_or(|c| c == self.def_of(id).types.creature);
                }
                if let Some(creature_only) = f.graveyard {
                    let d = self.def_of(id);
                    return o.zone == Zone::Graveyard
                        && (!creature_only || d.types.creature)
                        && (!f.nonland_permanents || (d.types.is_permanent() && !d.types.land))
                        && (!f.permanent_cards || d.types.is_permanent())
                        && (!f.instant_or_sorcery || d.types.instant || d.types.sorcery)
                        && (!f.put_there_this_turn || o.graveyard_turn == self.turn)
                        && f.max_mana_value.is_none_or(|m| d.cost.mana_value() <= m);
                }
                if o.zone != Zone::Battlefield {
                    return false;
                }
                // CR 702.11b: hexproof stops opponents' spells and abilities from targeting it.
                if o.controller != controller && !f.triggering && self.keywords(id).has(Keywords::HEXPROOF) {
                    return false;
                }
                let ty = &self.def_of(id).types;
                let in_combat = self
                    .combat
                    .as_ref()
                    .is_some_and(|c| c.attackers.contains(&id) || c.is_blocking(id));
                let creature_ok = ty.creature
                    && f.creatures
                    && (!f.creature_needs_flying || self.keywords(id).has(Keywords::FLYING))
                    && f.creature_min_power.is_none_or(|m| self.power(id) >= m)
                    && (!f.in_combat || in_combat);
                creature_ok
                    || (ty.planeswalker && f.planeswalkers)
                    || (ty.artifact && f.artifacts)
                    || (ty.enchantment && f.enchantments)
                    || (!ty.land && f.nonland_permanents)
                    || f.any_permanent
            }
        }
    }

    /// Every legal target for `f`, players first.
    pub fn candidates(&self, f: &Filter, controller: PlayerId) -> Vec<Target> {
        let graveyards = self
            .players
            .iter()
            .flat_map(|pl| pl.graveyard.iter())
            .filter(|_| f.graveyard.is_some());
        (0..2)
            .map(Target::Player)
            .chain(self.battlefield.iter().map(|&id| Target::Object(id)))
            .chain(
                self.stack
                    .iter()
                    .filter(|_| f.spells)
                    .map(|&id| Target::Object(id)),
            )
            .chain(graveyards.map(|&id| Target::Object(id)))
            .filter(|&t| self.target_ok(f, controller, t))
            .collect()
    }

    /// Target legality that depends on the stack entry itself: "other" and "mana value at most the
    /// source's power".
    fn slot_ok(&self, item: &StackItem, f: &Filter, t: Target) -> bool {
        let Target::Object(x) = t else { return true };
        if self.protected_from(x, item.source) {
            return false; // CR 702.16c
        }
        let from_instant = item.kind == StackKind::Spell && self.def_of(item.source).types.instant;
        if from_instant && self.def_of(x).hexproof_from_instants && self.obj(x).controller != item.controller
        {
            return false;
        }
        if f.other && x == item.source {
            return false;
        }
        if f.attached_to_first_target {
            let Some(&Chosen::Object(c, zcc)) = item.targets.first() else {
                return false;
            };
            if self.obj(x).attached_to != Some((c, zcc)) {
                return false;
            }
        }
        if f.mana_value_at_most_source_power {
            let s = self.obj(item.source);
            let power = if s.zone == Zone::Battlefield {
                self.power(item.source)
            } else {
                s.last_power
            };
            if self.def_of(x).cost.mana_value() as i32 > power {
                return false;
            }
        }
        true
    }

    /// A stack entry for a spell about to be cast, for target checks.
    fn casting(card: ObjId, p: PlayerId) -> StackItem {
        StackItem {
            source: card,
            controller: p,
            kind: StackKind::Spell,
            targets: Vec::new(),
            kicked: false,
            flashback: false,
            extra: None,
            mode: None,
            amount: 0,
            alt: false,
            x: 0,
            division: Vec::new(),
        }
    }

    fn trigger_can_target(&self, item: &StackItem, ability: &Ability) -> bool {
        let required = ability.min_targets.unwrap_or(ability.targets.len());
        ability.targets[..required].iter().all(|f| {
            self.candidates(f, item.controller)
                .into_iter()
                .any(|t| self.slot_ok(item, f, t))
        })
    }

    fn priority_actions(&self, p: PlayerId) -> Vec<Action> {
        let mut v = vec![Action::PassPriority];
        let sorcery_timing =
            p == self.active && matches!(self.step, Step::Main1 | Step::Main2) && self.items.is_empty();
        let hand = self.players[p].hand.iter().map(|&c| (c, false));
        let flashback = self.players[p].graveyard.iter().filter(|&&c| {
            let d = self.def_of(c);
            d.flashback.is_some() || (self.obj(c).flashback_eot && (d.types.instant || d.types.sorcery))
        });
        let turn = self.turn;
        let from_exile = self.exile.iter().copied().filter(move |&c| {
            let o = self.obj(c);
            let stashed = o.stash
                && o.owner != p
                && self.active == p
                && self.permanents(p).any(|x| self.def_of(x).stash_player);
            stashed || o.playable.is_some_and(|(q, until)| q == p && turn <= until)
        });
        // Cards an effect lets you cast from your graveyard this turn (Zul Ashur).
        let from_graveyard = self.players[p].graveyard.iter().copied().filter(move |&c| {
            self.obj(c)
                .playable
                .is_some_and(|(q, until)| q == p && turn <= until)
        });
        let candidates: Vec<(ObjId, bool)> = hand
            .chain(flashback.map(|&c| (c, true)))
            .chain(from_exile.map(|c| (c, false)))
            .chain(from_graveyard.map(|c| (c, false)))
            .collect();
        for (c, from_graveyard) in candidates {
            let def = self.def_of(c);
            if def.types.land {
                // CR 305.2: one land per turn, at sorcery timing.
                let drops = 1 + self
                    .permanents(p)
                    .map(|x| self.def_of(x).extra_land_drops)
                    .sum::<u8>();
                if sorcery_timing && self.players[p].lands_played < drops {
                    v.push(Action::PlayLand(c));
                }
                continue;
            }
            // CR 307.1, 302.1, 304.1, 702.8a: only instants and flash ignore sorcery timing.
            let flash_all = self.permanents(p).any(|x| self.def_of(x).grants_flash);
            let timing =
                sorcery_timing || flash_all || def.types.instant || def.keywords.has(Keywords::FLASH);
            let castable = def.types.is_permanent() || def.spell.is_some() || !def.modes.is_empty();
            if !timing || !castable {
                continue;
            }
            if let Some(ability) = &def.spell {
                if !self.trigger_can_target(&Game::casting(c, p), ability) {
                    continue; // CR 601.2c: a spell needs a legal choice for every target.
                }
            }
            for (action, mode) in self.cast_modes(p, c, from_graveyard) {
                if self.mode_affordable(p, c, mode) {
                    v.push(action);
                }
            }
        }
        v.extend(self.activations(p));
        v.extend(self.loyalty_actions(p, sorcery_timing));
        v
    }

    /// The ways to cast `c`: plain, kicked, flashback, or paying an additional cost with each
    /// eligible creature or card.
    fn cast_modes(&self, p: PlayerId, c: ObjId, from_graveyard: bool) -> Vec<(Action, CastMode)> {
        let def = self.def_of(c);
        if def.cost.x > 0 && !from_graveyard {
            // One mode per affordable X (capped by the mana available).
            let max = (self.untapped_sources(p).len() + self.players[p].pool.len()).min(30) as u8;
            return (0..=max)
                .map(|x| {
                    (
                        Action::CastX(c, x),
                        CastMode {
                            x,
                            ..CastMode::default()
                        },
                    )
                })
                .collect();
        }
        if from_graveyard {
            return vec![(
                Action::CastFlashback(c),
                CastMode {
                    flashback: true,
                    ..CastMode::default()
                },
            )];
        }
        if !def.modes.is_empty() {
            return (0..def.modes.len())
                .filter(|&m| self.trigger_can_target(&Game::casting(c, p), &def.modes[m]))
                .map(|m| {
                    (
                        Action::CastModal(c, m as u8),
                        CastMode {
                            mode: Some(m as u8),
                            ..CastMode::default()
                        },
                    )
                })
                .collect();
        }
        let creatures = || {
            self.creatures(p)
                .map(|x| (Action::CastWith(c, x), CastMode::with(x)))
        };
        match &def.extra_cost {
            Some(ExtraCost::SacrificeCreature) => creatures().collect(),
            Some(ExtraCost::Discard) => self.players[p]
                .hand
                .iter()
                .filter(|&&x| x != c)
                .map(|&x| (Action::CastWith(c, x), CastMode::with(x)))
                .collect(),
            Some(ExtraCost::SacrificeCreatureOrPay(_)) => {
                std::iter::once((Action::Cast(c), CastMode::default()))
                    .chain(creatures())
                    .collect()
            }
            None => {
                let mut v = vec![(Action::Cast(c), CastMode::default())];
                if let Some((_, needed)) = def.alt_cost {
                    let creatures = self
                        .battlefield
                        .iter()
                        .filter(|&&x| self.def_of(x).types.creature)
                        .count();
                    if creatures >= needed as usize {
                        v.push((
                            Action::CastAlternative(c),
                            CastMode {
                                alt: true,
                                ..CastMode::default()
                            },
                        ));
                    }
                }
                if def.kicker.is_some() {
                    v.push((
                        Action::CastKicked(c),
                        CastMode {
                            kicked: true,
                            ..CastMode::default()
                        },
                    ));
                }
                v
            }
        }
    }

    /// The mana a spell costs in `mode` with these targets (CR 601.2f).
    fn spell_cost(&self, p: PlayerId, c: ObjId, mode: CastMode, targets: &[Chosen]) -> ManaCost {
        let def = self.def_of(c);
        let o = self.obj(c);
        let in_hand = o.zone == Zone::Hand || (o.zone == Zone::Stack && o.from_hand);
        // CR 107.3b, 118.9: cast without paying its mana cost, X is 0. Choosing a bigger X means
        // paying the cost.
        let free = in_hand
            && mode.x == 0
            && !mode.alt
            && !mode.flashback
            && self.permanents(p).any(|x| self.def_of(x).free_from_hand);
        let mut cost = if free {
            ManaCost::default()
        } else if mode.alt {
            def.alt_cost.expect("has an alternative cost").0
        } else if mode.flashback {
            def.flashback.unwrap_or(def.cost)
        } else {
            def.cost.with_x(mode.x)
        };
        if mode.kicked {
            cost = add_costs(&cost, &def.kicker.expect("has kicker"));
        }
        if let (Some(ExtraCost::SacrificeCreatureOrPay(m)), None) = (&def.extra_cost, mode.extra) {
            cost = add_costs(&cost, m);
        }
        let less = match &def.reduction {
            Some(Reduction::IfYouControl(sub, n)) => {
                let yes = self.permanents(p).any(|x| self.has_subtype(x, sub));
                if yes {
                    *n
                } else {
                    0
                }
            }
            Some(Reduction::PerInstantSorceryInGraveyard) => self.players[p]
                .graveyard
                .iter()
                .filter(|&&x| self.def_of(x).types.instant || self.def_of(x).types.sorcery)
                .count()
                .min(255) as u8,
            Some(Reduction::TotalPowerOfYourCreatures) => self
                .creatures(p)
                .map(|x| self.power(x).max(0))
                .sum::<i32>()
                .min(255) as u8,
            Some(Reduction::PerAttackingCreature) => self.combat.as_ref().map_or(0, |c| {
                c.attackers
                    .iter()
                    .filter(|&&a| self.obj(a).controller == p)
                    .count()
            }) as u8,
            Some(Reduction::PerYouControl(sub)) => {
                self.permanents(p).filter(|&x| self.has_subtype(x, sub)).count() as u8
            }
            Some(Reduction::IfTargetsTapped(n)) => match targets.first() {
                Some(&Chosen::Object(x, _)) if self.obj(x).tapped && self.def_of(x).types.creature => *n,
                _ => 0,
            },
            None => 0,
        };
        let discount: u8 = if def.types.instant || def.types.sorcery {
            self.permanents(p).map(|x| self.def_of(x).spell_discount).sum()
        } else {
            0
        };
        cost.generic = cost.generic.saturating_sub(less + discount);
        if self.obj(c).stash {
            // "Mana of any type can be spent": colored pips become generic.
            cost.generic += cost.colored.iter().sum::<u8>();
            cost.colored = [0; 5];
        }
        cost
    }

    /// Whether some legal choice of targets makes the spell affordable in `mode`.
    fn mode_affordable(&self, p: PlayerId, c: ObjId, mode: CastMode) -> bool {
        if self.can_pay_for(p, &self.spell_cost(p, c, mode, &[]), None, Some(c)) {
            return true;
        }
        if let Some(Reduction::IfTargetsTapped(_)) = self.def_of(c).reduction {
            let f = self
                .def_of(c)
                .spell
                .as_ref()
                .and_then(|s| s.targets.first().copied());
            return f.is_some_and(|f| {
                let casting = Game::casting(c, p);
                self.candidates(&f, p)
                    .into_iter()
                    .filter(|&t| self.slot_ok(&casting, &f, t))
                    .any(|t| match t {
                        Target::Object(x) => {
                            let chosen = [Chosen::Object(x, self.obj(x).zcc)];
                            self.can_pay_for(p, &self.spell_cost(p, c, mode, &chosen), None, Some(c))
                        }
                        Target::Player(_) => false,
                    })
            });
        }
        false
    }

    /// Whether a permanent can pay a {T} cost: creatures need to have been under your control since
    /// your turn began, or haste (CR 302.6).
    fn can_tap(&self, id: ObjId) -> bool {
        let o = self.obj(id);
        !o.tapped
            && (!self.def_of(id).types.creature
                || o.since_turn < self.turn
                || self.keywords(id).has(Keywords::HASTE))
    }

    /// Taps (and sacrifices, for Treasure-like sources) what pays `cost`. Never uses `exclude`.
    fn pay_mana(&mut self, p: PlayerId, cost: &ManaCost, exclude: Option<ObjId>) {
        self.pay_mana_for(p, cost, exclude, None);
    }

    /// Pays `cost`, for casting `spell` if given (restricted mana can then help).
    fn pay_mana_for(&mut self, p: PlayerId, cost: &ManaCost, exclude: Option<ObjId>, spell: Option<ObjId>) {
        let sources = self.payment_sources(p, exclude, spell);
        let pool = self.players[p].pool.clone();
        let all: Vec<ManaSource> = pool.iter().copied().chain(sources.iter().map(|s| s.1)).collect();
        let mut paid = mana::pay(cost, &all).expect("payable");
        paid.sort_unstable_by(|a, b| b.cmp(a));
        for i in paid {
            if i < pool.len() {
                self.players[p].pool.remove(i);
                continue;
            }
            let id = sources[i - pool.len()].0;
            self.obj_mut(id).tapped = true;
            if self.def_of(id).sac_for_mana {
                self.move_to(id, Zone::Graveyard);
            }
        }
    }

    fn can_pay(&self, p: PlayerId, cost: &ManaCost, exclude: Option<ObjId>) -> bool {
        self.can_pay_for(p, cost, exclude, None)
    }

    /// The sources that may pay: not `exclude`, and restricted mana only for a spell it allows.
    fn payment_sources(
        &self,
        p: PlayerId,
        exclude: Option<ObjId>,
        spell: Option<ObjId>,
    ) -> Vec<(ObjId, ManaSource)> {
        self.untapped_sources(p)
            .into_iter()
            .filter(|s| Some(s.0) != exclude)
            .filter(|s| match self.def_of(s.0).mana_only_for_subtype {
                None => true,
                Some(sub) => spell.is_some_and(|c| self.has_base_subtype(c, sub)),
            })
            .collect()
    }

    fn can_pay_for(
        &self,
        p: PlayerId,
        cost: &ManaCost,
        exclude: Option<ObjId>,
        spell: Option<ObjId>,
    ) -> bool {
        let sources: Vec<ManaSource> = self
            .payment_sources(p, exclude, spell)
            .into_iter()
            .map(|s| s.1)
            .collect();
        let all: Vec<ManaSource> = self.players[p].pool.iter().copied().chain(sources).collect();
        mana::pay(cost, &all).is_some()
    }

    /// Loyalty abilities `p` can activate: sorcery timing, once per planeswalker per turn, and not
    /// below zero loyalty (CR 606.3-606.6).
    fn loyalty_actions(&self, p: PlayerId, sorcery_timing: bool) -> Vec<Action> {
        let mut v = Vec::new();
        if !sorcery_timing {
            return v;
        }
        for id in self.permanents(p) {
            let o = self.obj(id);
            if o.loyalty_turn == self.turn && self.turn > 0 {
                continue;
            }
            for (n, (cost, ability)) in self.def_of(id).loyalty_abilities.iter().enumerate() {
                let item = StackItem {
                    kind: StackKind::Loyalty(n),
                    ..Game::casting(id, p)
                };
                if o.loyalty + cost >= 0 && self.trigger_can_target(&item, ability) {
                    v.push(Action::Loyalty(id, n));
                }
            }
        }
        v
    }

    fn pay_loyalty(&mut self, id: ObjId, cost: i32) {
        // CR 606.4: a plus cost puts counters on, so Doubling Season doubles it.
        let cost = if cost > 0 {
            cost * self.doubling(self.obj(id).controller)
        } else {
            cost
        };
        let turn = self.turn;
        let o = self.obj_mut(id);
        o.loyalty += cost;
        o.loyalty_turn = turn;
    }

    fn activate_loyalty(&mut self, p: PlayerId, id: ObjId, n: usize) {
        let (cost, ability) = self.def_of(id).loyalty_abilities[n].clone();
        let item = StackItem {
            kind: StackKind::Loyalty(n),
            ..Game::casting(id, p)
        };
        self.passes = 0;
        if ability.targets.is_empty() {
            self.pay_loyalty(id, cost);
            self.items.push(item);
            self.give_priority(p);
        } else {
            self.choosing = Some(Choosing {
                item,
                then: p,
                cast: false,
                pay: None,
                loyalty: Some((id, cost)),
            });
            self.pending = Pending::ChooseTarget(p);
        }
    }

    /// Untapped creatures `p` controls with this subtype, other than `except`.
    fn untapped_with(&self, p: PlayerId, sub: &str, except: ObjId) -> Vec<ObjId> {
        self.creatures(p)
            .filter(|&c| c != except && !self.obj(c).tapped && self.has_subtype(c, sub))
            .collect()
    }

    /// Activated abilities `p` can activate now (CR 602.2, 602.5).
    fn activations(&self, p: PlayerId) -> Vec<Action> {
        let mut v = Vec::new();
        let graveyard = self.players[p].graveyard.iter().copied();
        for id in self.permanents(p).chain(graveyard) {
            let in_graveyard = self.obj(id).zone == Zone::Graveyard;
            for (n, a) in self.def_of(id).activated.iter().enumerate() {
                if a.from_graveyard != in_graveyard || (!in_graveyard && self.lost_abilities(id)) {
                    continue;
                }
                let uses_self = a.tap || a.sacrifice_self;
                let sorcery_ok = !a.sorcery
                    || (p == self.active
                        && matches!(self.step, Step::Main1 | Step::Main2)
                        && self.items.is_empty());
                if (a.tap && !self.can_tap(id))
                    || self.obj(id).incubation < a.remove_incubation
                    || (a.once && self.obj(id).used_once)
                    || (a.tap_attached
                        && !self
                            .obj(id)
                            .attached_to
                            .is_some_and(|(c, z)| self.obj(c).zcc == z && self.can_tap(c)))
                    || a.tap_untapped
                        .is_some_and(|(sub, n)| self.untapped_with(p, sub, id).len() < n as usize)
                    || !sorcery_ok
                    || !self.can_pay(p, &a.mana, uses_self.then_some(id))
                    || !self.trigger_can_target(
                        &StackItem {
                            kind: StackKind::Activated(n),
                            ..Game::casting(id, p)
                        },
                        &a.ability,
                    )
                {
                    continue;
                }
                if a.mana.x > 0 {
                    let max = (self.untapped_sources(p).len() + self.players[p].pool.len()).min(30) as u8;
                    let victims: Vec<Option<ObjId>> = if a.sacrifice_other_creature {
                        self.creatures(p).filter(|&c| c != id).map(Some).collect()
                    } else {
                        vec![None]
                    };
                    for victim in victims {
                        for x in 0..=max {
                            if self.can_pay(p, &a.mana.with_x(x), uses_self.then_some(id)) {
                                v.push(Action::ActivateX { id, n, victim, x });
                            }
                        }
                    }
                } else if a.sacrifice_other_creature {
                    v.extend(
                        self.creatures(p)
                            .filter(|&c| c != id)
                            .map(|c| Action::ActivateWith(id, n, c)),
                    );
                } else {
                    v.push(Action::Activate(id, n));
                }
            }
        }
        v
    }

    /// CR 602.2: pay the costs, then the ability goes on the stack (targets chosen as a decision).
    /// CR 602.2: announce, choose targets, then pay the costs.
    fn activate(&mut self, p: PlayerId, id: ObjId, n: usize, victim: Option<ObjId>) {
        let item = StackItem {
            source: id,
            controller: p,
            kind: StackKind::Activated(n),
            targets: Vec::new(),
            kicked: false,
            flashback: false,
            extra: None,
            mode: None,
            amount: 0,
            alt: false,
            x: 0,
            division: Vec::new(),
        };
        self.passes = 0;
        if self.def_of(id).activated[n].ability.targets.is_empty() {
            self.pay_activation(p, id, n, victim, 0);
            self.items.push(item);
            self.give_priority(p);
        } else {
            self.choosing = Some(Choosing {
                item,
                then: p,
                cast: false,
                pay: Some((n, victim)),
                loyalty: None,
            });
            self.pending = Pending::ChooseTarget(p);
        }
    }

    /// Activates an ability with {X} in its cost.
    fn activate_x(&mut self, p: PlayerId, id: ObjId, n: usize, victim: Option<ObjId>, x: u8) {
        let item = StackItem {
            kind: StackKind::Activated(n),
            x,
            ..Game::casting(id, p)
        };
        self.passes = 0;
        if self.def_of(id).activated[n].ability.targets.is_empty() {
            self.pay_activation(p, id, n, victim, x);
            self.items.push(item);
            self.give_priority(p);
        } else {
            self.choosing = Some(Choosing {
                item,
                then: p,
                cast: false,
                pay: Some((n, victim)),
                loyalty: None,
            });
            self.pending = Pending::ChooseTarget(p);
        }
    }

    fn pay_activation(&mut self, p: PlayerId, id: ObjId, n: usize, victim: Option<ObjId>, x: u8) {
        let a = self.def_of(id).activated[n].clone();
        let uses_self = a.tap || a.sacrifice_self;
        // Mana first: a creature can tap for mana and then be sacrificed for the same cost
        // (CR 602.2b). The fuzzer found the reverse order failing.
        self.pay_mana(p, &a.mana.with_x(x), uses_self.then_some(id));
        if let Some(v) = victim {
            if self.obj(v).zone == Zone::Battlefield {
                self.move_to(v, Zone::Graveyard);
            }
        }
        if a.tap {
            self.obj_mut(id).tapped = true;
        }
        self.obj_mut(id).incubation -= a.remove_incubation;
        if a.once {
            self.obj_mut(id).used_once = true;
        }
        if a.tap_attached {
            if let Some((c, _)) = self.obj(id).attached_to {
                self.obj_mut(c).tapped = true;
            }
        }
        if let Some((sub, n)) = a.tap_untapped {
            for c in self.untapped_with(p, sub, id).into_iter().take(n as usize) {
                self.obj_mut(c).tapped = true;
            }
        }
        if a.sacrifice_self {
            self.move_to(id, Zone::Graveyard);
        }
    }

    /// Creature types `p` could sensibly name: those of creatures they control and of creature
    /// cards they own (a player knows their own deck), sorted. Public for tests and agents.
    pub fn creature_types_known_to(&self, p: PlayerId) -> Vec<String> {
        let mut v: Vec<String> = self
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.owner == p || (o.zone == Zone::Battlefield && o.controller == p))
            .filter(|(_, o)| o.zone != Zone::Gone)
            .filter(|(i, _)| self.def_of(ObjId(*i as u32)).types.creature)
            .flat_map(|(i, _)| self.def_of(ObjId(i as u32)).subtypes.iter().cloned())
            .collect();
        v.sort();
        v.dedup();
        v.truncate(64);
        v
    }

    /// The options of a pending creature-type choice.
    pub fn creature_type_options(&self) -> Vec<String> {
        match &self.decision {
            Some(Decision::ChooseCreatureType { options, .. }) => options.clone(),
            _ => Vec::new(),
        }
    }

    /// Untapped mana sources, one entry per mana each can make.
    fn untapped_sources(&self, p: PlayerId) -> Vec<(ObjId, ManaSource)> {
        let base = self.single_sources(p);
        let mut out = Vec::with_capacity(base.len());
        for (id, m) in base {
            if self.def_of(id).vivid_mana {
                // One mana of each color among permanents you control.
                for c in 0..5 {
                    if self.permanents(p).any(|x| self.def_of(x).colors[c]) {
                        out.push((id, ManaSource::of(&[crate::Color::ALL[c]])));
                    }
                }
                continue;
            }
            // One activation makes this many mana; each is offered to the payer separately.
            let n = self.def_of(id).mana_per_subtype.map_or(1, |sub| {
                self.permanents(p)
                    .filter(|&x| self.has_subtype(x, sub))
                    .count()
                    .max(1)
            });
            out.extend(std::iter::repeat_n((id, m), n));
        }
        out
    }

    fn single_sources(&self, p: PlayerId) -> Vec<(ObjId, ManaSource)> {
        self.permanents(p)
            .filter(|&id| self.can_tap(id))
            .filter(|&id| !self.lost_abilities(id))
            .filter_map(|id| {
                let d = self.def_of(id);
                if d.vivid_mana {
                    return Some((id, ManaSource::default()));
                }
                if d.mana_of_chosen_color {
                    let c = self.obj(id).chosen_color?;
                    return Some((id, ManaSource::of(&[crate::Color::ALL[c as usize]])));
                }
                d.mana.map(|m| (id, m))
            })
            .collect()
    }

    /// Creatures that can be declared as attackers (CR 508.1a, 302.6, 702.3b).
    fn attack_candidates(&self, p: PlayerId) -> impl Iterator<Item = ObjId> + '_ {
        let declared = self
            .combat
            .as_ref()
            .map(|c| c.attackers.clone())
            .unwrap_or_default();
        self.creatures(p).filter(move |&id| {
            let o = self.obj(id);
            let k = self.keywords(id);
            !o.tapped
                && !k.has(Keywords::DEFENDER)
                && (o.since_turn < self.turn || k.has(Keywords::HASTE))
                && !declared.contains(&id)
        })
    }

    /// Whether `blocker` may block `attacker`, ignoring menace (CR 509.1b, 702.9b, 702.17b).
    fn can_block(&self, blocker: ObjId, attacker: ObjId) -> bool {
        if self.protected_from(attacker, blocker) {
            return false; // CR 702.16f
        }
        if self.def_of(blocker).cant_block
            || self.keywords(blocker).has(Keywords::CANT_BLOCK)
            || self.keywords(attacker).has(Keywords::UNBLOCKABLE)
            || self
                .def_of(attacker)
                .not_blocked_by
                .is_some_and(|sub| self.has_subtype(blocker, sub))
        {
            return false;
        }
        let b = self.keywords(blocker);
        !self.keywords(attacker).has(Keywords::FLYING) || b.has(Keywords::FLYING) || b.has(Keywords::REACH)
    }

    fn block_actions(&self, p: PlayerId) -> Vec<Action> {
        let combat = self.combat.as_ref().expect("blocks happen in combat");
        let free: Vec<ObjId> = self
            .creatures(p)
            .filter(|&id| !self.obj(id).tapped && !combat.is_blocking(id))
            .collect();
        let mut v = Vec::new();
        for &a in &combat.attackers {
            let able: Vec<ObjId> = free.iter().copied().filter(|&b| self.can_block(b, a)).collect();
            // CR 702.111b: menace needs two or more blockers. Only offer the first block if a second
            // blocker is available.
            let menace_first = self.keywords(a).has(Keywords::MENACE) && !combat.is_blocked(a);
            if menace_first && able.len() < 2 {
                continue;
            }
            v.extend(able.into_iter().map(|b| Action::Block {
                blocker: b,
                attacker: a,
            }));
        }
        if !combat.blocks.is_empty() {
            v.push(Action::ResetBlocks);
        }
        if self.blocks_are_legal(combat) {
            v.push(Action::FinishBlocks);
        }
        v
    }

    /// Whether `action` is legal now. Same answer as `legal_actions().contains(&action)`, with fast
    /// paths for declarations, whose full lists can run to hundreds of thousands of actions.
    fn is_legal(&self, action: Action) -> bool {
        match (self.pending, action) {
            (Pending::DeclareBlockers(p), Action::Block { blocker, attacker }) => {
                let combat = self.combat.as_ref().expect("blocks happen in combat");
                let free = |b: ObjId| {
                    let o = self.obj(b);
                    o.zone == Zone::Battlefield
                        && o.controller == p
                        && self.def_of(b).types.creature
                        && !o.tapped
                        && !combat.is_blocking(b)
                };
                if !combat.attackers.contains(&attacker)
                    || !free(blocker)
                    || !self.can_block(blocker, attacker)
                {
                    return false;
                }
                // CR 702.111b: a first block on a menace attacker needs a second blocker available.
                let menace_first =
                    self.keywords(attacker).has(Keywords::MENACE) && !combat.is_blocked(attacker);
                !menace_first
                    || self
                        .creatures(p)
                        .filter(|&b| b != blocker && free(b) && self.can_block(b, attacker))
                        .count()
                        >= 1
            }
            (Pending::DeclareAttackers(p), Action::Attack(c)) => self.attack_candidates(p).any(|x| x == c),
            _ => self.legal_actions().contains(&action),
        }
    }

    fn blocks_are_legal(&self, combat: &Combat) -> bool {
        combat
            .attackers
            .iter()
            .all(|&a| !self.keywords(a).has(Keywords::MENACE) || combat.blockers_of(a).count() != 1)
    }

    // ---------------------------------------------------------------- applying actions

    /// Applies one action. Illegal actions are errors and change nothing.
    pub fn apply(&mut self, action: Action) -> Result<Vec<Event>, RulesError> {
        if self.is_over() {
            return Err(RulesError::GameOver);
        }
        if !self.is_legal(action) {
            return Err(RulesError::Illegal(action));
        }
        self.work = 0;
        match (self.pending, action) {
            (Pending::Mulligan(p), Action::Keep) => {
                let n = self.players[p].mulligans;
                if n > 0 {
                    self.pending = Pending::Bottom(p, n);
                } else {
                    self.after_keep(p);
                }
            }
            (Pending::Mulligan(p), Action::Mulligan) => {
                for c in self.players[p].hand.clone() {
                    self.move_to(c, Zone::Library);
                }
                let mut lib = std::mem::take(&mut self.players[p].library);
                self.rng.shuffle(&mut lib);
                self.players[p].library = lib;
                for _ in 0..HAND_SIZE {
                    self.draw(p);
                }
                self.players[p].mulligans += 1;
            }
            (Pending::Bottom(p, n), Action::Bottom(c)) => {
                self.move_to(c, Zone::Library);
                let lib = &mut self.players[p].library;
                let top = lib.pop().expect("just moved");
                lib.insert(0, top);
                if n > 1 {
                    self.pending = Pending::Bottom(p, n - 1);
                } else {
                    self.after_keep(p);
                }
            }
            (Pending::Priority(p), Action::PassPriority) => self.pass(p),
            (Pending::Priority(p), Action::PlayLand(c)) => {
                self.move_to(c, Zone::Battlefield);
                self.players[p].lands_played += 1;
                self.passes = 0;
                self.give_priority(p);
            }
            (Pending::Priority(p), Action::Cast(c)) => self.cast(p, c, CastMode::default()),
            (Pending::Priority(p), Action::CastKicked(c)) => self.cast(
                p,
                c,
                CastMode {
                    kicked: true,
                    ..CastMode::default()
                },
            ),
            (Pending::Priority(p), Action::CastWith(c, x)) => self.cast(p, c, CastMode::with(x)),
            (Pending::Priority(p), Action::Loyalty(id, n)) => self.activate_loyalty(p, id, n),
            (Pending::DeclareAttackers(_), Action::AttackPlaneswalker(c, pw)) => {
                self.apply(Action::Attack(c)).expect("the attacker is legal");
                self.combat
                    .as_mut()
                    .expect("in combat")
                    .at_planeswalker
                    .push((c, pw));
                return Ok(self.take_events());
            }
            (Pending::Priority(p), Action::CastX(c, x)) => self.cast(
                p,
                c,
                CastMode {
                    x,
                    ..CastMode::default()
                },
            ),
            (Pending::Priority(p), Action::ActivateX { id, n, victim, x }) => {
                self.activate_x(p, id, n, victim, x)
            }
            (Pending::Priority(p), Action::CastAlternative(c)) => self.cast(
                p,
                c,
                CastMode {
                    alt: true,
                    ..CastMode::default()
                },
            ),
            (Pending::Priority(p), Action::CastModal(c, m)) => self.cast(
                p,
                c,
                CastMode {
                    mode: Some(m),
                    ..CastMode::default()
                },
            ),
            (Pending::Priority(p), Action::CastFlashback(c)) => self.cast(
                p,
                c,
                CastMode {
                    flashback: true,
                    ..CastMode::default()
                },
            ),
            (Pending::Priority(p), Action::Activate(id, n)) => self.activate(p, id, n, None),
            (Pending::Priority(p), Action::ActivateWith(id, n, victim)) => {
                self.activate(p, id, n, Some(victim))
            }
            (Pending::ChooseTarget(_), Action::Target(target)) => self.choose_target(target),
            (Pending::ChooseTarget(_), Action::No) => self.finish_choosing(),
            (Pending::Divide(_), Action::Amount(n)) => {
                self.choosing.as_mut().expect("dividing").item.division.push(n);
                self.finish_choosing();
            }
            (Pending::Decide(p), a) => self.decide(p, a),
            (Pending::DeclareAttackers(_), Action::Attack(c)) => {
                // CR 508.1f: attacking taps the creature unless it has vigilance.
                if !self.keywords(c).has(Keywords::VIGILANCE) {
                    self.obj_mut(c).tapped = true;
                }
                self.combat.as_mut().expect("in combat").attackers.push(c);
                self.attacked_this_turn[self.active] = true;
                // CR 508.3a: "whenever this creature attacks".
                let whens: Vec<When> = self.def_of(c).triggers.iter().map(|tr| tr.when).collect();
                for (n, w) in whens.into_iter().enumerate() {
                    if w == When::Attacks {
                        self.triggered.push(StackItem {
                            source: c,
                            controller: self.active,
                            kind: StackKind::Trigger(n),
                            targets: Vec::new(),
                            kicked: false,
                            flashback: false,
                            extra: None,
                            mode: None,
                            amount: 0,
                            alt: false,
                            x: 0,
                            division: Vec::new(),
                        });
                    }
                }
            }
            (Pending::DeclareAttackers(_), Action::FinishAttacks) => {
                if self.combat.as_ref().is_some_and(|c| !c.attackers.is_empty()) {
                    self.fire(self.active, When::YouAttack, None);
                }
                let count = self.combat.as_ref().map_or(0, |c| c.attackers.len()) as u8;
                for n in 1..=count {
                    self.fire(self.active, When::YouAttackWithAtLeast(n), None);
                }
                self.passes = 0;
                self.give_priority(self.active);
            }
            (Pending::DeclareBlockers(_), Action::Block { blocker, attacker }) => {
                self.combat
                    .as_mut()
                    .expect("in combat")
                    .blocks
                    .push((blocker, attacker));
            }
            (Pending::DeclareBlockers(_), Action::ResetBlocks) => {
                self.combat.as_mut().expect("in combat").blocks.clear();
            }
            (Pending::DeclareBlockers(_), Action::FinishBlocks) => {
                self.passes = 0;
                self.give_priority(self.active);
            }
            (Pending::Discard(p), Action::Discard(c)) => {
                self.move_to(c, Zone::Graveyard);
                if self.players[p].hand.len() <= MAX_HAND {
                    self.finish_cleanup();
                }
            }
            (pending, action) => unreachable!("legal action {action:?} unhandled in {pending:?}"),
        }
        self.settle();
        Ok(self.take_events())
    }

    fn after_keep(&mut self, p: PlayerId) {
        if p == self.first {
            self.pending = Pending::Mulligan(opponent(p));
        } else {
            // CR 103.6a: leylines may begin the game on the battlefield (always taken; QUESTIONS).
            for q in 0..2 {
                let leylines: Vec<ObjId> = self.players[q]
                    .hand
                    .iter()
                    .copied()
                    .filter(|&c| self.def_of(c).leyline)
                    .collect();
                for c in leylines {
                    self.move_to(c, Zone::Battlefield);
                }
            }
            self.triggered.clear();
            self.turn = 0;
            self.active = opponent(self.first);
            self.begin_next_turn();
        }
    }

    /// CR 601.2: move to the stack, choose targets, pay, then the caster gets priority again
    /// (CR 117.3c).
    fn cast(&mut self, p: PlayerId, card: ObjId, mode: CastMode) {
        self.obj_mut(card).from_hand = self.obj(card).zone == Zone::Hand;
        self.move_to(card, Zone::Stack);
        self.obj_mut(card).controller = p;
        self.obj_mut(card).kicked = mode.kicked;
        self.obj_mut(card).x = mode.x;
        let item = StackItem {
            source: card,
            controller: p,
            kind: StackKind::Spell,
            targets: Vec::new(),
            kicked: mode.kicked,
            flashback: mode.flashback,
            extra: mode.extra,
            mode: mode.mode,
            amount: 0,
            alt: mode.alt,
            x: mode.x,
            division: Vec::new(),
        };
        let def = self.def_of(card);
        let ability = match mode.mode {
            Some(m) => Some(&def.modes[m as usize]),
            None if mode.kicked && def.kicked_spell.is_some() => def.kicked_spell.as_ref(),
            None => def.spell.as_ref(),
        };
        let has_targets = ability.is_some_and(|s| !s.targets.is_empty());
        if has_targets {
            self.choosing = Some(Choosing {
                item,
                then: p,
                cast: true,
                pay: None,
                loyalty: None,
            });
            self.pending = Pending::ChooseTarget(p);
        } else {
            self.finish_cast(item);
        }
    }

    fn finish_cast(&mut self, item: StackItem) {
        let (p, card) = (item.controller, item.source);
        let mode = CastMode {
            x: item.x,
            alt: item.alt,
            kicked: item.kicked,
            flashback: item.flashback,
            extra: item.extra,
            mode: item.mode,
        };
        let cost = self.spell_cost(p, card, mode, &item.targets);
        // CR 601.2h: additional costs are paid with the mana. Mana first, so a creature can tap for
        // mana and then be sacrificed.
        self.pay_mana_for(p, &cost, None, Some(card));
        if let Some(x) = item.extra {
            if self.obj(x).zone != Zone::Graveyard {
                self.move_to(x, Zone::Graveyard);
            }
        }
        let noncreature = !self.def_of(card).types.creature;
        if p != self.active {
            self.fire(p, When::YouCastOnOpponentsTurn, None);
        }
        self.fire(opponent(p), When::OpponentCasts, None);
        for (owner, kind, card) in self.emblems.clone() {
            if let EmblemKind::OnAnySpellCast { trigger } = kind {
                self.triggered.push(StackItem {
                    kind: StackKind::Trigger(trigger),
                    ..Game::casting(card, owner)
                });
            }
        }
        self.fire(p, When::YouCast, None);
        self.ward_check(&item);
        let instant_or_sorcery = self.def_of(card).types.instant || self.def_of(card).types.sorcery;
        self.items.push(item);
        self.events.push(Event::Cast { card });
        if instant_or_sorcery {
            self.fire(p, When::YouCastInstantOrSorcery, None);
        }
        // "Whenever you cast a noncreature or [subtype] spell".
        let (creature, subs) = (
            self.def_of(card).types.creature,
            self.def_of(card).subtypes.clone(),
        );
        for id in self.permanents(p).collect::<Vec<_>>() {
            let whens: Vec<When> = self.def_of(id).triggers.iter().map(|tr| tr.when).collect();
            for (n, w) in whens.into_iter().enumerate() {
                if let When::YouCastNoncreatureOrSubtype(sub) = w {
                    if !creature || subs.iter().any(|s| s == sub) {
                        self.triggered.push(StackItem {
                            source: id,
                            controller: p,
                            kind: StackKind::Trigger(n),
                            targets: Vec::new(),
                            kicked: false,
                            flashback: false,
                            extra: None,
                            mode: None,
                            amount: 0,
                            alt: false,
                            x: 0,
                            division: Vec::new(),
                        });
                    }
                }
            }
        }
        if noncreature {
            // Prowess and other cast triggers of the caster's permanents.
            for id in self.permanents(p).collect::<Vec<_>>() {
                let whens: Vec<When> = self.def_of(id).triggers.iter().map(|tr| tr.when).collect();
                for (n, w) in whens.into_iter().enumerate() {
                    if w == When::YouCastNoncreature {
                        self.triggered.push(StackItem {
                            source: id,
                            controller: p,
                            kind: StackKind::Trigger(n),
                            targets: Vec::new(),
                            kicked: false,
                            flashback: false,
                            extra: None,
                            mode: None,
                            amount: 0,
                            alt: false,
                            x: 0,
                            division: Vec::new(),
                        });
                    }
                }
            }
        }
        self.passes = 0;
        self.give_priority(p);
    }

    fn choose_target(&mut self, target: Target) {
        let c = self.choosing.as_mut().expect("choosing targets");
        let chosen = match target {
            Target::Player(p) => Chosen::Player(p),
            Target::Object(id) => Chosen::Object(id, self.objects[id.0 as usize].zcc),
        };
        c.item.targets.push(chosen);
        let c = self.choosing.as_ref().expect("choosing targets");
        let done = c.item.targets.len() == self.ability_of(&c.item).targets.len();
        if done {
            self.finish_choosing();
        }
    }

    fn finish_choosing(&mut self) {
        let mut c = self.choosing.take().expect("choosing targets");
        let total = self.ability_of(&c.item).divide;
        let k = c.item.targets.len();
        if total > 0 && k > 0 && c.item.division.len() < k {
            if c.item.division.len() + 1 < k {
                self.pending = Pending::Divide(c.item.controller);
                self.choosing = Some(c);
                return;
            }
            let given: u8 = c.item.division.iter().sum();
            c.item.division.push(total - given);
        }
        if c.cast {
            self.finish_cast(c.item);
        } else {
            if let Some((pw, cost)) = c.loyalty {
                self.pay_loyalty(pw, cost);
            }
            if let Some((n, victim)) = c.pay {
                self.pay_activation(c.item.controller, c.item.source, n, victim, c.item.x);
            }
            self.ward_check(&c.item);
            self.items.push(c.item);
            self.give_priority(c.then);
        }
    }

    /// CR 117.4: if both players pass in succession, the top of the stack resolves, or with an
    /// empty stack the step ends.
    fn pass(&mut self, p: PlayerId) {
        self.passes += 1;
        if self.passes < 2 {
            self.give_priority(opponent(p));
            return;
        }
        self.passes = 0;
        if let Some(top) = self.items.pop() {
            self.resolve(top);
        } else {
            self.end_step();
        }
    }

    /// Whether a chosen target is still legal (CR 608.2b).
    fn still_legal(&self, item: &StackItem, slot: usize) -> bool {
        let f = &self.ability_of(item).targets[slot];
        match item.targets[slot] {
            Chosen::Player(p) => {
                !self.players[p].lost && self.target_ok(f, item.controller, Target::Player(p))
            }
            Chosen::Object(id, zcc) => {
                self.obj(id).zcc == zcc
                    && self.slot_ok(item, f, Target::Object(id))
                    && self.target_ok(f, item.controller, Target::Object(id))
            }
        }
    }

    /// CR 702.21a: whenever a permanent with ward becomes the target of a spell or ability an
    /// opponent controls, its ward triggers.
    fn ward_check(&mut self, item: &StackItem) {
        for &t in &item.targets {
            if let Chosen::Object(id, _) = t {
                let (zone, controller) = (self.obj(id).zone, self.obj(id).controller);
                if zone != Zone::Battlefield || controller == item.controller {
                    continue;
                }
                if let Some(life) = self.def_of(id).ward_life {
                    self.triggered.push(StackItem {
                        source: id,
                        controller,
                        kind: StackKind::WardLife {
                            life,
                            against: item.source,
                        },
                        targets: Vec::new(),
                        kicked: false,
                        flashback: false,
                        extra: None,
                        mode: None,
                        amount: 0,
                        alt: false,
                        x: 0,
                        division: Vec::new(),
                    });
                }
                if let Some(cost) = self.def_of(id).ward {
                    self.triggered.push(StackItem {
                        source: id,
                        controller,
                        kind: StackKind::Ward {
                            cost,
                            against: item.source,
                        },
                        targets: Vec::new(),
                        kicked: false,
                        flashback: false,
                        extra: None,
                        mode: None,
                        amount: 0,
                        alt: false,
                        x: 0,
                        division: Vec::new(),
                    });
                }
            }
        }
    }

    /// Counters the topmost spell or ability on the stack from `source` (CR 701.5).
    fn counter_from(&mut self, source: ObjId) {
        if let Some(pos) = self.items.iter().rposition(|i| i.source == source) {
            if self.items[pos].kind == StackKind::Spell && self.def_of(source).uncounterable {
                return; // "This spell can't be countered."
            }
            let countered = self.items.remove(pos);
            if countered.kind == StackKind::Spell {
                self.leave_stack(source, countered.flashback);
            }
            self.events.push(Event::Fizzled { source });
        }
    }

    /// CR 608.2: check targets, then carry out the effects in order. Pauses on decisions.
    fn resolve(&mut self, item: StackItem) {
        let source = item.source;
        if let StackKind::WardLife { life, against } = item.kind {
            self.resolving = Some(Resolving {
                item,
                legal: Vec::new(),
                remaining: VecDeque::new(),
                acts: true,
            });
            if let Some(payer) = self
                .items
                .iter()
                .rev()
                .find(|i| i.source == against)
                .map(|i| i.controller)
            {
                self.ask(payer, Decision::PayWardLife { life, against });
                return;
            }
            self.continue_resolving();
            return;
        }
        if let StackKind::Ward { cost, against } = item.kind {
            self.resolving = Some(Resolving {
                item,
                legal: Vec::new(),
                remaining: VecDeque::new(),
                acts: true,
            });
            if self.items.iter().any(|i| i.source == against) {
                let payer = self
                    .items
                    .iter()
                    .rev()
                    .find(|i| i.source == against)
                    .expect("on the stack")
                    .controller;
                self.ask(payer, Decision::PayWard { cost, against });
                return;
            }
            self.continue_resolving();
            return;
        }
        let legal: Vec<bool> = (0..item.targets.len())
            .map(|i| self.still_legal(&item, i))
            .collect();
        let fizzled = !legal.is_empty() && !legal.iter().any(|&b| b);
        let is_trigger = matches!(item.kind, StackKind::Trigger(_));
        let condition_failed =
            is_trigger && !self.condition_holds(self.ability_of(&item), item.controller, source);
        if fizzled && !condition_failed {
            self.events.push(Event::Fizzled { source });
        }
        let has_ability =
            item.kind != StackKind::Spell || self.def_of(source).spell.is_some() || item.mode.is_some();
        let acts = has_ability && !fizzled && !condition_failed;
        let (remaining, optional) = if acts {
            let a = self.ability_of(&item);
            (a.effects.iter().copied().collect(), a.optional)
        } else {
            (VecDeque::new(), false)
        };
        let fizzled_spell = fizzled && item.kind == StackKind::Spell;
        let controller = item.controller;
        self.resolving = Some(Resolving {
            item,
            legal,
            remaining,
            acts: !fizzled_spell,
        });
        if optional {
            self.decision = Some(Decision::May);
            self.pending = Pending::Decide(controller);
            return;
        }
        self.continue_resolving();
    }

    /// Runs effects until one needs a decision, or until done.
    fn continue_resolving(&mut self) {
        loop {
            if self.decision.is_some() || self.is_over() {
                return;
            }
            let r = self.resolving.as_mut().expect("resolving");
            let Some(e) = r.remaining.pop_front() else { break };
            let (item, legal) = (r.item.clone(), r.legal.clone());
            self.run_effect(&item, &legal, e);
        }
        let r = self.resolving.take().expect("resolving");
        let source = r.item.source;
        if r.item.kind == StackKind::Spell {
            if r.acts && self.def_of(source).types.is_permanent() {
                self.move_to(source, Zone::Battlefield);
                self.obj_mut(source).controller = r.item.controller;
                if self.def_of(source).revival_on_cast {
                    let n = 8 * self.doubling(r.item.controller);
                    self.obj_mut(source).revival = n;
                }
                // CR 303.4f: an Aura spell enters attached to its target.
                if self.def_of(source).aura {
                    if let Some(&Chosen::Object(t, z)) = r.item.targets.first() {
                        self.obj_mut(source).attached_to = Some((t, z));
                    }
                }
            } else if self.obj(source).zone == Zone::Stack {
                self.leave_stack(source, r.item.flashback);
            }
            if r.acts {
                self.events.push(Event::Resolved { card: source });
            }
            // "As this enters, choose a color" (CR 614.12).
            if r.acts && self.def_of(source).choose_color && self.obj(source).zone == Zone::Battlefield {
                self.ask(r.item.controller, Decision::ChooseColor { card: source });
                return;
            }
            if r.acts
                && self.def_of(source).choose_creature_type
                && self.obj(source).zone == Zone::Battlefield
            {
                // With no creature cards to name, any choice does nothing; skip it.
                let options = self.creature_types_known_to(r.item.controller);
                if !options.is_empty() {
                    self.ask(
                        r.item.controller,
                        Decision::ChooseCreatureType {
                            card: source,
                            options,
                        },
                    );
                    return;
                }
            }
        }
        if self.end_turn {
            self.end_the_turn();
            return;
        }
        self.give_priority(self.active);
    }

    /// CR 724.1: exile every object on the stack, remove creatures from combat, and go to cleanup.
    fn end_the_turn(&mut self) {
        self.end_turn = false;
        let spells: Vec<ObjId> = self
            .items
            .drain(..)
            .filter(|i| i.kind == StackKind::Spell)
            .map(|i| i.source)
            .collect();
        for s in spells {
            self.move_to(s, Zone::Exile);
        }
        self.triggered.clear();
        self.combat = None;
        self.enter(Step::Cleanup);
    }

    /// A spell leaves the stack: to the graveyard, or to exile if it was cast with flashback.
    fn leave_stack(&mut self, id: ObjId, flashback: bool) {
        self.move_to(id, if flashback { Zone::Exile } else { Zone::Graveyard });
    }

    /// Puts +1/+1 counters on a permanent, with "whenever you put counters on this" triggers.
    fn add_counters(&mut self, id: ObjId, n: i32) {
        if n <= 0 || self.obj(id).zone != Zone::Battlefield {
            return;
        }
        let n = n * self.doubling(self.obj(id).controller);
        self.obj_mut(id).counters += n;
        if self.def_of(id).types.creature && !self.has_subtype(id, "Hydra") {
            let controller = self.obj(id).controller;
            self.fire(controller, When::CountersOnAnotherNonHydraYouControl, Some(id));
        }
        if self.obj(id).counter_trigger_turn != self.turn {
            let before = self.triggered.len();
            self.fire_source(id, When::CountersPutOnThis);
            if self.triggered.len() > before {
                self.obj_mut(id).counter_trigger_turn = self.turn;
            }
        }
    }

    /// Liliana's −9: walk the permanent types in order, asking which one to keep of each.
    fn next_keep(&mut self, p: PlayerId, mut ty: u8, kept: Vec<ObjId>) {
        while ty < 5 {
            let any = self.permanents(p).any(|x| Game::has_type(self.def_of(x), ty));
            if any {
                self.ask(p, Decision::KeepOfType { ty, kept });
                return;
            }
            ty += 1;
        }
        let doomed: Vec<ObjId> = self.permanents(p).filter(|x| !kept.contains(x)).collect();
        for x in doomed {
            self.move_to(x, Zone::Graveyard);
        }
    }

    /// Permanent types in order: artifact, creature, enchantment, land, planeswalker.
    fn has_type(d: &CardDef, ty: u8) -> bool {
        match ty {
            0 => d.types.artifact,
            1 => d.types.creature,
            2 => d.types.enchantment,
            3 => d.types.land,
            _ => d.types.planeswalker,
        }
    }

    fn picks(&self, c: ObjId, pick: Pick) -> bool {
        let d = self.def_of(c);
        match pick {
            Pick::Any => true,
            Pick::CreaturePowerAtMost(n) => d.types.creature && d.power <= n,
            Pick::InstantOrSorcery => d.types.instant || d.types.sorcery,
            Pick::CreatureOrLand => d.types.creature || d.types.land,
            Pick::CreatureMvAtMostLands => {
                let owner = self.obj(c).owner;
                let lands = self
                    .permanents(owner)
                    .filter(|&x| self.def_of(x).types.land)
                    .count() as u32;
                d.types.creature && d.cost.mana_value() <= lands
            }
        }
    }

    fn ask(&mut self, p: PlayerId, d: Decision) {
        self.decision = Some(d);
        self.pending = Pending::Decide(p);
    }

    fn library_matches(&self, p: PlayerId, what: SearchFor) -> Vec<ObjId> {
        self.players[p]
            .library
            .iter()
            .copied()
            .filter(|&c| {
                let d = self.def_of(c);
                match what {
                    SearchFor::Any => true,
                    SearchFor::BasicLand => d.types.basic && d.types.land,
                    SearchFor::InstantOrSorceryWithManaValue(mv) => {
                        (d.types.instant || d.types.sorcery) && d.cost.mana_value() == mv
                    }
                    SearchFor::CreatureMvAtMost(mv) => d.types.creature && d.cost.mana_value() <= mv,
                    SearchFor::CreatureMvAtMostX => false, // resolved to CreatureMvAtMost first
                }
            })
            .collect()
    }

    fn decision_actions(&self, p: PlayerId) -> Vec<Action> {
        let hand = &self.players[p].hand;
        match self.decision.as_ref().expect("a decision") {
            Decision::May | Decision::Surveil { .. } | Decision::Tuck { .. } => vec![Action::Yes, Action::No],
            Decision::SacrificeCreatures { .. } => self.creatures(p).map(Action::Pick).collect(),
            Decision::ChooseColor { .. } => (0..5).map(Action::Mode).collect(),
            Decision::ChooseCreatureType { options, .. } => {
                (0..options.len() as u8).map(Action::Mode).collect()
            }
            Decision::KeepOfType { ty, .. } => self
                .permanents(p)
                .filter(|&x| Game::has_type(self.def_of(x), *ty))
                .map(Action::Pick)
                .collect(),
            Decision::GenesisPick { cards, x } => {
                let mut v: Vec<Action> = cards
                    .iter()
                    .copied()
                    .filter(|&c| {
                        let d = self.def_of(c);
                        d.types.is_permanent() && d.cost.mana_value() <= *x
                    })
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::SplitPiles { .. } | Decision::ChoosePile { .. } => vec![Action::Yes, Action::No],
            Decision::MayPay { cost } => {
                let mut v = Vec::new();
                if self.can_pay(p, cost, None) {
                    v.push(Action::Yes);
                }
                v.push(Action::No);
                v
            }
            Decision::LoseOrDiscard { sacrifice, .. } => {
                let mut v: Vec<Action> = hand.iter().map(|&c| Action::Discard(c)).collect();
                if *sacrifice {
                    v.extend(
                        self.permanents(p)
                            .filter(|&x| !self.def_of(x).types.land)
                            .map(Action::Pick),
                    );
                }
                v.push(Action::No);
                v
            }
            Decision::PayWardLife { life, .. } => {
                let mut v = Vec::new();
                if self.players[p].life >= *life {
                    v.push(Action::Yes); // CR 119.4
                }
                v.push(Action::No);
                v
            }
            Decision::SacrificeAnother { source } => {
                let mut v: Vec<Action> = self
                    .creatures(p)
                    .filter(|&c| c != *source)
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::LookPick { cards, pick, .. } => {
                let mut v: Vec<Action> = cards
                    .iter()
                    .copied()
                    .filter(|&c| self.picks(c, *pick))
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::ChooseMode => {
                let (item, _) = self.mode_pending.as_ref().expect("a modal trigger");
                let StackKind::Trigger(n) = item.kind else {
                    unreachable!("only triggers are modal here")
                };
                let modes = &self.def_of(item.source).triggers[n].ability.modes;
                (0..modes.len())
                    .filter(|&m| self.trigger_can_target(item, &modes[m]))
                    .map(|m| Action::Mode(m as u8))
                    .collect()
            }
            Decision::PayWard { cost, .. } => {
                let mut v = Vec::new();
                if self.can_pay(p, cost, None) {
                    v.push(Action::Yes);
                }
                v.push(Action::No);
                v
            }
            Decision::Discard { optional, .. } => {
                let mut v: Vec<Action> = hand.iter().map(|&c| Action::Discard(c)).collect();
                if *optional {
                    v.push(Action::No);
                }
                v
            }
            Decision::PickDiscard { victim } => self.players[*victim]
                .hand
                .iter()
                .filter(|&&c| !self.def_of(c).types.land)
                .map(|&c| Action::Pick(c))
                .collect(),
            Decision::Search { what, .. } => {
                let mut v: Vec<Action> = self
                    .library_matches(p, *what)
                    .into_iter()
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
        }
    }

    fn shuffle_library(&mut self, p: PlayerId) {
        let mut lib = std::mem::take(&mut self.players[p].library);
        self.rng.shuffle(&mut lib);
        self.players[p].library = lib;
    }

    fn decide(&mut self, p: PlayerId, a: Action) {
        let d = self.decision.take().expect("a decision");
        if let (Decision::ChooseCreatureType { card, options }, Action::Mode(i)) = (&d, a) {
            let t = options[i as usize].clone();
            let (card, owner) = (*card, self.obj(*card).controller);
            let n = self
                .creatures(owner)
                .filter(|&x| self.has_base_subtype(x, &t))
                .count() as i32;
            let n = n * self.doubling(owner);
            let o = self.obj_mut(card);
            o.chosen_type = Some(t);
            o.fellowship = n;
            if self.end_turn {
                self.end_the_turn();
            } else {
                self.give_priority(self.active);
            }
            return;
        }
        if let (Decision::ChooseColor { card }, Action::Mode(c)) = (&d, a) {
            self.obj_mut(*card).chosen_color = Some(c);
            if self.end_turn {
                self.end_the_turn();
            } else {
                self.give_priority(self.active);
            }
            return;
        }
        if let (Decision::ChooseMode, Action::Mode(m)) = (&d, a) {
            let (mut item, then) = self.mode_pending.take().expect("a modal trigger");
            item.mode = Some(m);
            self.triggered.insert(0, item);
            self.give_priority(then);
            return;
        }
        match (d, a) {
            (Decision::PayWard { cost, .. }, Action::Yes) => self.pay_mana(p, &cost, None),
            (Decision::SacrificeAnother { .. }, Action::Pick(c)) => self.move_to(c, Zone::Graveyard),
            (Decision::MayPay { cost }, Action::Yes) => self.pay_mana(p, &cost, None),
            (Decision::KeepOfType { ty, mut kept }, Action::Pick(c)) => {
                if !kept.contains(&c) {
                    kept.push(c);
                }
                self.next_keep(p, ty + 1, kept);
            }
            (Decision::GenesisPick { mut cards, x }, Action::Pick(c)) => {
                cards.retain(|&k| k != c);
                self.move_to(c, Zone::Battlefield);
                if !cards.is_empty() {
                    self.ask(p, Decision::GenesisPick { cards, x });
                }
            }
            (Decision::GenesisPick { cards, .. }, Action::No) => {
                for c in cards {
                    self.move_to(c, Zone::Graveyard);
                }
            }
            (
                Decision::SplitPiles {
                    mut rest,
                    mut a,
                    mut b,
                    owner,
                },
                choice,
            ) => {
                let c = rest.remove(0);
                if choice == Action::Yes {
                    a.push(c)
                } else {
                    b.push(c)
                }
                if rest.is_empty() {
                    self.ask(opponent(owner), Decision::ChoosePile { a, b, owner });
                } else {
                    self.ask(p, Decision::SplitPiles { rest, a, b, owner });
                }
            }
            (Decision::ChoosePile { a, b, .. }, choice) => {
                let (keep, lose) = if choice == Action::Yes { (a, b) } else { (b, a) };
                for c in keep {
                    self.move_to(c, Zone::Hand);
                }
                for c in lose {
                    self.move_to(c, Zone::Graveyard);
                }
            }
            (Decision::SacrificeCreatures { left }, Action::Pick(c)) => {
                self.move_to(c, Zone::Graveyard);
                if left > 1 && self.creatures(p).next().is_some() {
                    self.ask(p, Decision::SacrificeCreatures { left: left - 1 });
                }
            }
            (Decision::MayPay { .. }, Action::No) => {
                self.resolving.as_mut().expect("resolving").remaining.clear();
            }
            (Decision::LoseOrDiscard { .. }, Action::Discard(c)) => self.move_to(c, Zone::Graveyard),
            (Decision::LoseOrDiscard { .. }, Action::Pick(c)) => self.move_to(c, Zone::Graveyard),
            (Decision::LoseOrDiscard { n, .. }, Action::No) => {
                self.lose_life(p, n);
            }
            (Decision::PayWardLife { life, .. }, Action::Yes) => {
                self.lose_life(p, life);
            }
            (Decision::PayWardLife { against, .. }, Action::No) => self.counter_from(against),
            (Decision::SacrificeAnother { .. }, Action::No) => {
                self.resolving.as_mut().expect("resolving").remaining.clear();
            }
            (Decision::LookPick { cards, to, rest, .. }, choice) => {
                let chosen = match choice {
                    Action::Pick(c) => Some(c),
                    _ => None,
                };
                if let Some(c) = chosen {
                    match to {
                        LookTo::Hand => self.move_to(c, Zone::Hand),
                        LookTo::Battlefield => self.move_to(c, Zone::Battlefield),
                        LookTo::PlayFromExileThisTurn => {
                            self.obj_mut(c).playable = Some((p, self.turn));
                        }
                        LookTo::PlayFromExile => {
                            // Until the end of your next turn (CR 611.2c).
                            let until = if self.active == p {
                                self.turn + 2
                            } else {
                                self.turn + 1
                            };
                            self.obj_mut(c).playable = Some((p, until));
                        }
                        LookTo::Top => {
                            let lib = &mut self.players[p].library;
                            let pos = lib.iter().position(|&x| x == c).expect("in library");
                            let card = lib.remove(pos);
                            lib.push(card);
                        }
                    }
                }
                let mut others: Vec<ObjId> = cards.into_iter().filter(|&c| Some(c) != chosen).collect();
                match rest {
                    Some(LookRest::Graveyard) => {
                        for c in others {
                            self.move_to(c, Zone::Graveyard);
                        }
                    }
                    Some(LookRest::BottomRandom) => {
                        self.rng.shuffle(&mut others);
                        let lib = &mut self.players[p].library;
                        lib.retain(|x| !others.contains(x));
                        for c in others {
                            lib.insert(0, c);
                        }
                    }
                    None => {}
                }
            }
            (Decision::PayWard { against, .. }, Action::No) => self.counter_from(against),
            (Decision::May, Action::Yes) => {}
            (Decision::May, Action::No) => {
                self.resolving.as_mut().expect("resolving").remaining.clear();
            }
            (
                Decision::Discard {
                    left,
                    optional,
                    then_draw,
                },
                Action::Discard(c),
            ) => {
                self.move_to(c, Zone::Graveyard);
                if left > 1 && !self.players[p].hand.is_empty() {
                    self.ask(
                        p,
                        Decision::Discard {
                            left: left - 1,
                            optional,
                            then_draw,
                        },
                    );
                } else if then_draw {
                    self.draw(p);
                }
            }
            (Decision::Discard { .. }, Action::No) => {}
            (Decision::PickDiscard { .. }, Action::Pick(c)) => self.move_to(c, Zone::Graveyard),
            (Decision::Surveil { mut cards }, choice) => {
                let card = cards.remove(0);
                if choice == Action::Yes {
                    self.move_to(card, Zone::Graveyard);
                }
                if !cards.is_empty() {
                    self.ask(p, Decision::Surveil { cards });
                } else {
                    // Kept cards stay on top in their original order.
                }
            }
            (Decision::Search { what, to, left }, Action::Pick(c)) => {
                match to {
                    SearchTo::Hand => self.move_to(c, Zone::Hand),
                    SearchTo::Battlefield { tapped } => {
                        self.move_to(c, Zone::Battlefield);
                        self.obj_mut(c).tapped = tapped;
                    }
                    SearchTo::Top => {}
                }
                if left > 1 && !self.library_matches(p, what).is_empty() {
                    self.ask(
                        p,
                        Decision::Search {
                            what,
                            to,
                            left: left - 1,
                        },
                    );
                } else {
                    self.finish_search(p, to, Some(c));
                }
            }
            (Decision::Search { to, .. }, Action::No) => self.finish_search(p, to, None),
            (Decision::Tuck { card }, choice) => {
                let owner = self.obj(card).owner;
                self.move_to(card, Zone::Library);
                if choice == Action::No {
                    let lib = &mut self.players[owner].library;
                    let c = lib.pop().expect("just moved");
                    lib.insert(0, c);
                }
            }
            (d, a) => unreachable!("legal action {a:?} unhandled for {d:?}"),
        }
        if self.decision.is_none() {
            self.continue_resolving();
        }
    }

    /// CR 701.19: shuffle after searching. A card found "to the top" goes on top after the shuffle.
    fn finish_search(&mut self, p: PlayerId, to: SearchTo, found: Option<ObjId>) {
        self.shuffle_library(p);
        if let (SearchTo::Top, Some(c)) = (to, found) {
            let lib = &mut self.players[p].library;
            let pos = lib.iter().position(|&x| x == c).expect("in library");
            let card = lib.remove(pos);
            lib.push(card);
        }
    }

    fn players_for(&self, item: &StackItem, legal: &[bool], who: Who) -> Vec<PlayerId> {
        let slot_player = |n: u8| match item.targets.get(n as usize) {
            Some(&Chosen::Player(p)) if legal[n as usize] => Some(p),
            Some(&Chosen::Object(id, _)) if legal[n as usize] => Some(self.obj(id).controller),
            _ => None,
        };
        match who {
            Who::You => vec![item.controller],
            Who::EachOpponent => vec![opponent(item.controller)],
            Who::TargetPlayer(n) | Who::ControllerOf(n) => slot_player(n).into_iter().collect(),
        }
    }

    fn slot_object(item: &StackItem, legal: &[bool], n: u8) -> Option<ObjId> {
        match item.targets.get(n as usize) {
            Some(&Chosen::Object(id, _)) if legal[n as usize] => Some(id),
            _ => None,
        }
    }

    /// CR 608.2c: effects happen in the order written; illegal targets are skipped (CR 608.2b).
    fn run_effect(&mut self, item: &StackItem, legal: &[bool], e: Effect) {
        let src = item.source;
        match e {
            Effect::Damage { amount, kicked, to } => {
                let amount = if item.kicked {
                    kicked.unwrap_or(amount)
                } else {
                    amount
                };
                let targets: Vec<Target> = match to {
                    Recipient::Target(n) => match item.targets.get(n as usize) {
                        _ if !legal.get(n as usize).copied().unwrap_or(false) => vec![],
                        Some(&Chosen::Player(p)) => vec![Target::Player(p)],
                        Some(&Chosen::Object(id, _)) => vec![Target::Object(id)],
                        None => vec![],
                    },
                    Recipient::EachOpponent => vec![Target::Player(opponent(item.controller))],
                    Recipient::EachPlayer => vec![
                        Target::Player(item.controller),
                        Target::Player(opponent(item.controller)),
                    ],
                    Recipient::EachCreature { without_flying } => self
                        .battlefield
                        .iter()
                        .copied()
                        .filter(|&id| self.def_of(id).types.creature)
                        .filter(|&id| !without_flying || !self.keywords(id).has(Keywords::FLYING))
                        .map(Target::Object)
                        .collect(),
                };
                for t in targets {
                    self.deal_damage(src, t, amount);
                }
            }
            Effect::DamageIfLifeIs { slot, life, amount } => {
                if let Some(&Chosen::Player(p)) = item.targets.get(slot as usize) {
                    if legal[slot as usize] && self.players[p].life == life {
                        self.deal_damage(src, Target::Player(p), amount);
                    }
                }
            }
            Effect::Destroy(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    self.destroy(id);
                }
            }
            Effect::Draw { who, n } => {
                for p in self.players_for(item, legal, who) {
                    for _ in 0..n {
                        self.draw(p);
                    }
                }
            }
            Effect::GainLife { who, n } => {
                for p in self.players_for(item, legal, who) {
                    self.gain_life(p, n);
                }
            }
            Effect::LoseLife { who, n } => {
                for p in self.players_for(item, legal, who) {
                    self.lose_life(p, n);
                }
            }
            Effect::Pump {
                slot,
                power,
                toughness,
            } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let zcc = self.obj(id).zcc;
                    self.until_eot.push(Modifier {
                        obj: id,
                        zcc,
                        power,
                        toughness,
                        keywords: Keywords::NONE,
                        dies: None,
                    });
                }
            }
            Effect::Grant { slot, keywords } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let zcc = self.obj(id).zcc;
                    self.until_eot.push(Modifier {
                        obj: id,
                        zcc,
                        power: 0,
                        toughness: 0,
                        keywords,
                        dies: None,
                    });
                }
            }
            Effect::Counters { slot, n } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    self.add_counters(id, n);
                }
            }
            Effect::CreateToken { who, def, n } => {
                for p in self.players_for(item, legal, who) {
                    for _ in 0..n {
                        self.create_token(p, def);
                    }
                }
            }
            Effect::PumpSelf { power, toughness } => {
                if self.obj(src).zone == Zone::Battlefield {
                    let zcc = self.obj(src).zcc;
                    self.until_eot.push(Modifier {
                        obj: src,
                        zcc,
                        power,
                        toughness,
                        keywords: Keywords::NONE,
                        dies: None,
                    });
                }
            }
            Effect::CreateTokenPerNamesake { def } => {
                let name = &self.def_of(src).name;
                let n = self
                    .creatures(item.controller)
                    .filter(|&c| c != src && &self.def_of(c).name == name)
                    .count();
                for _ in 0..n {
                    self.create_token(item.controller, def);
                }
            }
            Effect::Discard { who, n } => {
                if let Some(p) = self.players_for(item, legal, who).into_iter().next() {
                    if !self.players[p].hand.is_empty() {
                        self.ask(
                            p,
                            Decision::Discard {
                                left: n,
                                optional: false,
                                then_draw: false,
                            },
                        );
                    }
                }
            }
            Effect::MayDiscardToDraw => {
                let p = item.controller;
                if !self.players[p].hand.is_empty() {
                    self.ask(
                        p,
                        Decision::Discard {
                            left: 1,
                            optional: true,
                            then_draw: true,
                        },
                    );
                }
            }
            Effect::ChooseDiscard { slot } => {
                if let Some(victim) = self
                    .players_for(item, legal, Who::TargetPlayer(slot))
                    .into_iter()
                    .next()
                {
                    let any = self.players[victim]
                        .hand
                        .iter()
                        .any(|&c| !self.def_of(c).types.land);
                    if any {
                        self.ask(item.controller, Decision::PickDiscard { victim });
                    }
                }
            }
            Effect::Surveil { n } => {
                let lib = &self.players[item.controller].library;
                let cards: Vec<ObjId> = lib.iter().rev().take(n as usize).copied().collect();
                if !cards.is_empty() {
                    self.ask(item.controller, Decision::Surveil { cards });
                }
            }
            Effect::Search {
                what,
                to,
                n,
                kicked_n,
            } => {
                let n = if item.kicked { kicked_n.unwrap_or(n) } else { n };
                let what = match what {
                    SearchFor::CreatureMvAtMostX => SearchFor::CreatureMvAtMost(item.x as u32),
                    w => w,
                };
                self.ask(item.controller, Decision::Search { what, to, left: n });
            }
            Effect::Counter(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    if let Some(pos) = self
                        .items
                        .iter()
                        .position(|i| i.kind == StackKind::Spell && i.source == id)
                    {
                        if self.def_of(id).uncounterable {
                            return;
                        }
                        let countered = self.items.remove(pos);
                        self.leave_stack(id, countered.flashback);
                    }
                }
            }
            Effect::CountersSelf { n } => {
                self.add_counters(src, n);
            }
            Effect::PumpAll {
                who,
                power,
                toughness,
            } => {
                let players = self.players_for(item, legal, who);
                let ids: Vec<ObjId> = players
                    .into_iter()
                    .flat_map(|p| self.creatures(p).collect::<Vec<_>>())
                    .collect();
                for id in ids {
                    let zcc = self.obj(id).zcc;
                    self.until_eot.push(Modifier {
                        obj: id,
                        zcc,
                        power,
                        toughness,
                        keywords: Keywords::NONE,
                        dies: None,
                    });
                }
            }
            Effect::GrantAll { who, keywords } => {
                let players = self.players_for(item, legal, who);
                let ids: Vec<ObjId> = players
                    .into_iter()
                    .flat_map(|p| self.creatures(p).collect::<Vec<_>>())
                    .collect();
                for id in ids {
                    let zcc = self.obj(id).zcc;
                    self.until_eot.push(Modifier {
                        obj: id,
                        zcc,
                        power: 0,
                        toughness: 0,
                        keywords,
                        dies: None,
                    });
                }
            }
            Effect::Mill { who, n } => {
                for p in self.players_for(item, legal, who) {
                    for _ in 0..n {
                        if let Some(&c) = self.players[p].library.last() {
                            self.move_to(c, Zone::Graveyard);
                        }
                    }
                }
            }
            Effect::PumpAttached { power, toughness } => {
                if let Some((id, zcc)) = self.obj(src).attached_to {
                    if self.obj(id).zcc == zcc && self.obj(id).zone == Zone::Battlefield {
                        self.until_eot.push(Modifier {
                            obj: id,
                            zcc,
                            power,
                            toughness,
                            keywords: Keywords::NONE,
                            dies: None,
                        });
                    }
                }
            }
            Effect::Attach { slot } => {
                // CR 701.3: attach the source if it's still on the battlefield.
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if self.obj(src).zone == Zone::Battlefield {
                        let zcc = self.obj(id).zcc;
                        self.obj_mut(src).attached_to = Some((id, zcc));
                    }
                }
            }
            Effect::FightBetween { a, b } => {
                if let (Some(x), Some(y)) = (
                    Game::slot_object(item, legal, a),
                    Game::slot_object(item, legal, b),
                ) {
                    let (px, py) = (self.power(x), self.power(y));
                    if px > 0 {
                        self.deal_damage(x, Target::Object(y), px);
                    }
                    if py > 0 {
                        self.deal_damage(y, Target::Object(x), py);
                    }
                }
            }
            Effect::GrantSelf { keywords } => {
                if self.obj(src).zone == Zone::Battlefield {
                    let zcc = self.obj(src).zcc;
                    self.until_eot.push(Modifier {
                        obj: src,
                        zcc,
                        power: 0,
                        toughness: 0,
                        keywords,
                        dies: None,
                    });
                }
            }
            Effect::Reanimate(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    if self.obj(id).zone == Zone::Graveyard {
                        self.move_to(id, Zone::Battlefield);
                        self.obj_mut(id).controller = item.controller;
                    }
                }
            }
            Effect::ExileUntilSourceLeaves(n) => {
                // CR 610.3c: if the source already left, nothing is exiled.
                if let Some(id) = Game::slot_object(item, legal, n) {
                    if self.obj(src).zone == Zone::Battlefield {
                        self.move_to(id, Zone::Exile);
                        self.links.push((src, self.obj(src).zcc, id));
                    }
                }
            }
            Effect::DrainIfCreatureExiled { n } => {
                let exiled_creature = (0..item.targets.len() as u8).any(|i| {
                    Game::slot_object(item, legal, i)
                        .is_some_and(|x| self.obj(x).zone == Zone::Exile && self.def_of(x).types.creature)
                });
                if exiled_creature {
                    let opp = opponent(item.controller);
                    self.lose_life(opp, n);
                    self.gain_life(item.controller, n);
                }
            }
            Effect::DestroyAllCreatures => {
                let all: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| self.def_of(c).types.creature)
                    .collect();
                for c in all {
                    self.destroy(c);
                }
            }
            Effect::DamageEqualSourcePower(n) => {
                let power = if self.obj(src).zone == Zone::Battlefield {
                    self.power(src)
                } else {
                    self.obj(src).last_power
                };
                if let Some(id) = Game::slot_object(item, legal, n) {
                    if power > 0 {
                        self.deal_damage(src, Target::Object(id), power);
                    }
                }
            }
            Effect::BottomAndGainToughness(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    let (controller, toughness, owner) =
                        (self.obj(id).controller, self.toughness(id), self.obj(id).owner);
                    self.move_to(id, Zone::Library);
                    let lib = &mut self.players[owner].library;
                    let c = lib.pop().expect("just moved");
                    lib.insert(0, c);
                    if toughness > 0 {
                        self.gain_life(controller, toughness);
                    }
                }
            }
            Effect::BounceAttackers => {
                let attackers = self
                    .combat
                    .as_ref()
                    .map(|c| c.attackers.clone())
                    .unwrap_or_default();
                for a in attackers {
                    if self.obj(a).zone == Zone::Battlefield {
                        self.move_to(a, Zone::Hand);
                    }
                }
            }
            Effect::ReanimateAll {
                all_graveyards,
                max_mana_value,
            } => {
                let players: Vec<PlayerId> = if all_graveyards {
                    vec![0, 1]
                } else {
                    vec![item.controller]
                };
                let cards: Vec<ObjId> = players
                    .iter()
                    .flat_map(|&p| self.players[p].graveyard.clone())
                    .filter(|&c| {
                        let d = self.def_of(c);
                        d.types.creature && max_mana_value.is_none_or(|m| d.cost.mana_value() <= m)
                    })
                    .collect();
                for c in cards {
                    self.move_to(c, Zone::Battlefield);
                    self.obj_mut(c).controller = item.controller;
                }
            }
            Effect::DoubleCountersSelf => {
                if self.obj(src).zone == Zone::Battlefield {
                    self.obj_mut(src).counters *= 2;
                }
            }
            Effect::DoublePower(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    let (power, zcc) = (self.power(id), self.obj(id).zcc);
                    self.until_eot.push(Modifier {
                        obj: id,
                        zcc,
                        power,
                        toughness: 0,
                        keywords: Keywords::NONE,
                        dies: None,
                    });
                }
            }
            Effect::MaySacrificeAnother => {
                let any = self.creatures(item.controller).any(|c| c != src);
                if any {
                    self.ask(item.controller, Decision::SacrificeAnother { source: src });
                } else {
                    self.resolving.as_mut().expect("resolving").remaining.clear();
                }
            }
            Effect::ReturnSelfTapped => {
                if self.obj(src).zone == Zone::Graveyard {
                    self.move_to(src, Zone::Battlefield);
                    self.obj_mut(src).tapped = true;
                }
            }
            Effect::LookAtTop {
                n,
                pick,
                chosen_to,
                rest_to,
            } => {
                let lib = &self.players[item.controller].library;
                let cards: Vec<ObjId> = lib.iter().rev().take(n as usize).copied().collect();
                if !cards.is_empty() {
                    self.ask(
                        item.controller,
                        Decision::LookPick {
                            cards,
                            pick,
                            to: chosen_to,
                            rest: Some(rest_to),
                        },
                    );
                }
            }
            Effect::ReturnFromGraveyard { pick } => {
                let cards: Vec<ObjId> = self.players[item.controller]
                    .graveyard
                    .iter()
                    .copied()
                    .filter(|&c| self.picks(c, pick))
                    .collect();
                if !cards.is_empty() {
                    self.ask(
                        item.controller,
                        Decision::LookPick {
                            cards,
                            pick,
                            to: LookTo::Hand,
                            rest: None,
                        },
                    );
                }
            }
            Effect::CounterAndGainIfCreatureExiled { life } => {
                let exiled = Game::slot_object(item, legal, 0)
                    .is_some_and(|x| self.obj(x).zone == Zone::Exile && self.def_of(x).types.creature);
                if exiled {
                    self.add_counters(src, 1);
                    self.gain_life(item.controller, life);
                }
            }
            Effect::TapAndStun(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    self.obj_mut(id).tapped = true;
                    let n = self.doubling(self.obj(id).controller);
                    self.obj_mut(id).stun += n;
                }
            }
            Effect::GateControlPowerAtLeast(n) => {
                if !self.creatures(item.controller).any(|c| self.power(c) >= n) {
                    self.resolving.as_mut().expect("resolving").remaining.clear();
                }
            }
            Effect::MayPay(cost) => self.ask(item.controller, Decision::MayPay { cost }),
            Effect::LoseUnlessDiscard { n } => {
                let opp = opponent(item.controller);
                if self.players[opp].hand.is_empty() {
                    self.lose_life(opp, n);
                } else {
                    self.ask(opp, Decision::LoseOrDiscard { n, sacrifice: false });
                }
            }
            Effect::RevealUntilCreature => {
                let p = item.controller;
                let mut revealed = Vec::new();
                let mut found = None;
                while let Some(&c) = self.players[p].library.last() {
                    self.players[p].library.pop();
                    if self.def_of(c).types.creature {
                        found = Some(c);
                        break;
                    }
                    revealed.push(c);
                }
                if let Some(c) = found {
                    self.players[p].library.push(c);
                    self.move_to(c, Zone::Hand);
                }
                self.rng.shuffle(&mut revealed);
                for c in revealed {
                    self.players[p].library.insert(0, c);
                }
            }
            Effect::DoubleCounters(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    // "each kind of counter": +1/+1 and stun counters.
                    let c = self.obj(id).counters;
                    self.obj_mut(id).stun *= 2;
                    self.add_counters(id, c);
                }
            }
            Effect::GainLifeAmount => {
                if item.amount > 0 {
                    self.gain_life(item.controller, item.amount);
                }
            }
            Effect::DrawAmount => {
                for _ in 0..item.amount {
                    self.draw(item.controller);
                }
            }
            Effect::CreateTokensAmount { def } => {
                for _ in 0..item.amount {
                    self.create_token(item.controller, def);
                }
            }
            Effect::IncubationAmount => {
                if self.obj(src).zone == Zone::Battlefield {
                    let n = item.amount * self.doubling(self.obj(src).controller);
                    self.obj_mut(src).incubation += n;
                }
            }
            Effect::OpponentsMillUntilLand => {
                // Reveal until a land, then put every revealed card into the graveyard (so a card
                // that goes elsewhere instead, like Progenitus, is not revealed again).
                let opp = opponent(item.controller);
                let lib = &self.players[opp].library;
                let n = lib
                    .iter()
                    .rev()
                    .position(|&c| self.def_of(c).types.land)
                    .map_or(lib.len(), |i| i + 1);
                let revealed: Vec<ObjId> = lib.iter().rev().take(n).copied().collect();
                for c in revealed {
                    self.move_to(c, Zone::Graveyard);
                }
            }
            Effect::ReturnWith {
                slot,
                counters,
                subtype,
                keywords,
            } => {
                let card = match slot {
                    Some(n) => Game::slot_object(item, legal, n),
                    None => Some(src),
                };
                if let Some(c) = card.filter(|&c| self.obj(c).zone == Zone::Graveyard) {
                    self.move_to(c, Zone::Battlefield);
                    let o = self.obj_mut(c);
                    o.controller = o.owner;
                    o.extra_subtypes.extend(subtype);
                    o.granted = o.granted.with(keywords);
                    self.add_counters(c, counters);
                }
            }
            Effect::BecomeSubtype(s) => {
                if self.obj(src).zone == Zone::Battlefield {
                    let o = self.obj_mut(src);
                    o.subtype_override = Some(s);
                    o.extra_subtypes.clear();
                }
            }
            Effect::LoseUnlessSacrificeOrDiscard { n } => {
                let opp = opponent(item.controller);
                self.ask(opp, Decision::LoseOrDiscard { n, sacrifice: true });
            }
            Effect::SacrificeCreatures { who, n } => {
                for p in self.players_for(item, legal, who) {
                    let mine: Vec<ObjId> = self.creatures(p).collect();
                    if mine.len() <= n as usize {
                        for c in mine {
                            self.move_to(c, Zone::Graveyard);
                        }
                    } else {
                        self.ask(p, Decision::SacrificeCreatures { left: n });
                        return;
                    }
                }
            }
            Effect::GrantFlashback(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    self.obj_mut(id).flashback_eot = true;
                }
            }
            Effect::SoulCounterSelf => {
                if self.obj(src).zone == Zone::Battlefield {
                    self.obj_mut(src).soul += 1;
                }
            }
            Effect::OpponentsLoseSoul => {
                let o = self.obj(src);
                let n = if o.zone == Zone::Battlefield {
                    o.soul
                } else {
                    o.last_soul
                };
                if n > 0 {
                    self.lose_life(opponent(item.controller), n);
                }
            }
            Effect::Flicker(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    self.move_to(id, Zone::Exile);
                    let zcc = self.obj(id).zcc;
                    self.delayed_returns.push((id, zcc, 0));
                }
            }
            Effect::ReturnNextEndStepWithFewerRevival => {
                let o = self.obj(src);
                if o.zone == Zone::Graveyard && o.last_revival > 0 {
                    self.delayed_returns.push((src, o.zcc, o.last_revival - 1));
                }
            }
            Effect::EndTheTurn => self.end_turn = true,
            Effect::GainControlUntilEot(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    let (before, zcc) = (self.obj(id).controller, self.obj(id).zcc);
                    if before != item.controller {
                        self.control_until_eot.push((id, zcc, before));
                        let turn = self.turn;
                        let o = self.obj_mut(id);
                        o.controller = item.controller;
                        o.since_turn = turn; // CR 302.6
                    }
                }
            }
            Effect::UntapTarget(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    self.obj_mut(id).tapped = false;
                }
            }
            Effect::ExileIfDiesThisTurn(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    let (zcc, turn) = (self.obj(id).zcc, self.turn);
                    self.obj_mut(id).exile_if_dies = Some((zcc, turn));
                }
            }
            Effect::PlayableThisTurn(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    let turn = self.turn;
                    self.obj_mut(id).playable = Some((item.controller, turn));
                }
            }
            Effect::AddMana { color, n } => {
                let m = ManaSource::of(&[color]);
                self.players[item.controller]
                    .pool
                    .extend(std::iter::repeat_n(m, n as usize));
            }
            Effect::DividedDamage => {
                for (i, &n) in item.division.iter().enumerate() {
                    if let Some(id) = Game::slot_object(item, legal, i as u8) {
                        self.deal_damage(src, Target::Object(id), n as i32);
                    }
                }
            }
            Effect::ExileTopChooseOneToPlay { n, this_turn } => {
                let p = item.controller;
                let mut cards = Vec::new();
                for _ in 0..n {
                    if let Some(&c) = self.players[p].library.last() {
                        self.move_to(c, Zone::Exile);
                        cards.push(c);
                    }
                }
                if !cards.is_empty() {
                    self.ask(
                        p,
                        Decision::LookPick {
                            cards,
                            pick: Pick::Any,
                            to: if this_turn {
                                LookTo::PlayFromExileThisTurn
                            } else {
                                LookTo::PlayFromExile
                            },
                            rest: None,
                        },
                    );
                }
            }
            Effect::Piles { n } => {
                let p = item.controller;
                let rest: Vec<ObjId> = self.players[p]
                    .library
                    .iter()
                    .rev()
                    .take(n as usize)
                    .copied()
                    .collect();
                if !rest.is_empty() {
                    self.ask(
                        p,
                        Decision::SplitPiles {
                            rest,
                            a: Vec::new(),
                            b: Vec::new(),
                            owner: p,
                        },
                    );
                }
            }
            Effect::TutelageMill(n) => {
                if let Some(Chosen::Player(q)) = item.targets.get(n as usize).copied() {
                    loop {
                        // The two are milled together; a card that goes somewhere else instead
                        // (Progenitus) wasn't milled (CR 701.13), so every repeat shrinks the
                        // library by two and the loop ends.
                        let lib = &self.players[q].library;
                        let top: Vec<ObjId> = lib.iter().rev().take(2).copied().collect();
                        for &c in &top {
                            self.move_to(c, Zone::Graveyard);
                        }
                        let milled: Vec<ObjId> = top
                            .into_iter()
                            .filter(|&c| self.obj(c).zone == Zone::Graveyard)
                            .collect();
                        let nonland: Vec<[bool; 5]> = milled
                            .iter()
                            .filter(|&&c| !self.def_of(c).types.land)
                            .map(|&c| self.def_of(c).colors)
                            .collect();
                        let share = nonland.len() == 2 && (0..5).any(|i| nonland[0][i] && nonland[1][i]);
                        if !share || milled.len() < 2 {
                            break;
                        }
                    }
                }
            }
            Effect::BaitCounterSelf => {
                if self.obj(src).zone == Zone::Battlefield {
                    self.obj_mut(src).bait += 1;
                }
            }
            Effect::RemoveBaitForToken { def } => {
                if self.obj(src).bait > 0 {
                    self.obj_mut(src).bait -= 1;
                    self.create_token(item.controller, def);
                }
            }
            Effect::ExileWithStash(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    if self.obj(id).zone == Zone::Graveyard {
                        self.move_to(id, Zone::Exile);
                        self.obj_mut(id).stash = true;
                    }
                }
            }
            Effect::PreventCombatDamageTo(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    let zcc = self.obj(id).zcc;
                    self.combat_damage_prevented.push((id, zcc));
                }
            }
            Effect::PumpMorbid {
                slot,
                power,
                toughness,
                morbid_power,
                morbid_toughness,
            } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let (p, t) = if self.creature_died_this_turn {
                        (morbid_power, morbid_toughness)
                    } else {
                        (power, toughness)
                    };
                    let zcc = self.obj(id).zcc;
                    self.until_eot.push(Modifier {
                        obj: id,
                        zcc,
                        power: p,
                        toughness: t,
                        keywords: Keywords::NONE,
                        dies: None,
                    });
                }
            }
            Effect::GainLifePerAttacking(sub) => {
                let n = self.combat.as_ref().map_or(0, |c| {
                    c.attackers
                        .iter()
                        .filter(|&&a| self.obj(a).controller == item.controller && self.has_subtype(a, sub))
                        .count()
                }) as i32;
                if n > 0 {
                    self.gain_life(item.controller, n);
                }
            }
            Effect::CreateTokensPerYouControl { def, sub } => {
                let n = self
                    .permanents(item.controller)
                    .filter(|&x| self.has_subtype(x, sub))
                    .count();
                for _ in 0..n {
                    self.create_token(item.controller, def);
                }
            }
            Effect::CreateTokensPerGraveyardCreature { def, tapped } => {
                let n = self.players[item.controller]
                    .graveyard
                    .iter()
                    .filter(|&&c| self.def_of(c).types.creature)
                    .count();
                for _ in 0..n {
                    self.create_token(item.controller, def);
                    for t in self.last_created.clone() {
                        self.obj_mut(t).tapped = tapped;
                    }
                }
            }
            Effect::CreateTokensPerLand { def } => {
                let n = self
                    .permanents(item.controller)
                    .filter(|&x| self.def_of(x).types.land)
                    .count();
                for _ in 0..n {
                    self.create_token(item.controller, def);
                }
            }
            Effect::DrawPerDistinctManaValue => {
                let mut mvs: Vec<u32> = self
                    .permanents(item.controller)
                    .filter(|&x| !self.def_of(x).types.land)
                    .map(|x| self.def_of(x).cost.mana_value())
                    .collect();
                mvs.sort_unstable();
                mvs.dedup();
                for _ in 0..mvs.len() {
                    self.draw(item.controller);
                }
            }
            Effect::InspiringCall => {
                let with: Vec<ObjId> = self
                    .creatures(item.controller)
                    .filter(|&c| self.obj(c).counters > 0)
                    .collect();
                for _ in 0..with.len() {
                    self.draw(item.controller);
                }
                for c in with {
                    let zcc = self.obj(c).zcc;
                    self.until_eot.push(Modifier {
                        obj: c,
                        zcc,
                        power: 0,
                        toughness: 0,
                        keywords: Keywords::INDESTRUCTIBLE,
                        dies: None,
                    });
                }
            }
            Effect::LoseLifeEqualGainedThisTurn(n) => {
                let x = self.life_gained_this_turn[item.controller];
                if let Some(&Chosen::Player(q)) = item.targets.get(n as usize) {
                    if x > 0 && legal[n as usize] {
                        self.lose_life(q, x);
                    }
                }
            }
            Effect::ExtraTurn => self.extra_turns.push(item.controller),
            Effect::PumpBySourcePower(n) => {
                let x = self.power(src);
                if let Some(id) = Game::slot_object(item, legal, n) {
                    let zcc = self.obj(id).zcc;
                    self.until_eot.push(Modifier {
                        obj: id,
                        zcc,
                        power: x,
                        toughness: x,
                        keywords: Keywords::NONE,
                        dies: None,
                    });
                }
            }
            Effect::DamageX { to } => {
                self.run_effect(
                    item,
                    legal,
                    Effect::Damage {
                        amount: item.x as i32,
                        kicked: None,
                        to,
                    },
                );
            }
            Effect::DrainX => {
                let opp = opponent(item.controller);
                let x = item.x as i32;
                if x > 0 {
                    self.lose_life(opp, x);
                    self.gain_life(item.controller, x);
                }
            }
            Effect::DamageXExcessTokens { slot, def } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let x = item.x as i32;
                    // CR 120.10: excess damage is damage beyond what was lethal.
                    let lethal = (self.toughness(id) - self.obj(id).damage).max(0);
                    let excess = (x - lethal).max(0);
                    if x > 0 {
                        self.deal_damage(src, Target::Object(id), x);
                    }
                    for _ in 0..excess {
                        self.create_token(item.controller, def);
                    }
                }
            }
            Effect::GenesisWave => {
                let p = item.controller;
                let x = item.x as usize;
                let cards: Vec<ObjId> = self.players[p].library.iter().rev().take(x).copied().collect();
                if !cards.is_empty() {
                    self.ask(
                        p,
                        Decision::GenesisPick {
                            cards,
                            x: item.x as u32,
                        },
                    );
                }
            }
            Effect::CopyToken {
                slot,
                haste_and_sacrifice,
                extra_subtype,
            } => {
                let original = match slot {
                    Some(n) => Game::slot_object(item, legal, n),
                    None => Some(src),
                };
                if let Some(o) = original {
                    // CR 707.2: the copy takes the original's printed values (its card), not its
                    // counters or other effects.
                    let def = self.obj(o).def;
                    self.last_created.clear();
                    for _ in 0..self.doubling(item.controller) {
                        let id = self.add_card(item.controller, def, Zone::Hand);
                        self.obj_mut(id).copy_token = true;
                        self.move_to(id, Zone::Battlefield);
                        self.last_created.push(id);
                        let ob = self.obj_mut(id);
                        ob.extra_subtypes.extend(extra_subtype);
                        if haste_and_sacrifice {
                            ob.granted = ob.granted.with(Keywords::HASTE);
                            ob.sacrifice_at_end = true;
                        }
                    }
                }
            }
            Effect::ExileOtherTokensWithSubtype(sub) => {
                let keep = self.last_created.clone();
                let others: Vec<ObjId> = self
                    .permanents(item.controller)
                    .filter(|&c| !keep.contains(&c) && self.is_token(c) && self.has_subtype(c, sub))
                    .collect();
                for c in others {
                    self.move_to(c, Zone::Exile);
                }
            }
            Effect::LoyaltySelf(n) => {
                if self.obj(src).zone == Zone::Battlefield {
                    let n = if n > 0 {
                        n * self.doubling(self.obj(src).controller)
                    } else {
                        n
                    };
                    self.obj_mut(src).loyalty += n;
                }
            }
            Effect::CreateTokensPerLife { def } => {
                let n = self.players[item.controller].life.max(0);
                for _ in 0..n {
                    self.create_token(item.controller, def);
                }
            }
            Effect::Emblem(kind) => self.emblems.push((item.controller, kind, src)),
            Effect::KeepOneOfEachType => {
                let opp = opponent(item.controller);
                self.next_keep(opp, 0, Vec::new());
            }
            Effect::UntapSelf => {
                if self.obj(src).zone == Zone::Battlefield {
                    self.obj_mut(src).tapped = false;
                }
            }
            Effect::Tap(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    self.obj_mut(id).tapped = true;
                }
            }
            Effect::Reflexive(n) => {
                self.triggered.push(StackItem {
                    source: src,
                    controller: item.controller,
                    kind: StackKind::Trigger(n),
                    targets: Vec::new(),
                    kicked: false,
                    flashback: false,
                    extra: None,
                    mode: None,
                    amount: 0,
                    alt: false,
                    x: 0,
                    division: Vec::new(),
                });
            }
            Effect::GrantDiesTrigger { slot, trigger } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let zcc = self.obj(id).zcc;
                    self.until_eot.push(Modifier {
                        obj: id,
                        zcc,
                        power: 0,
                        toughness: 0,
                        keywords: Keywords::NONE,
                        dies: Some((src, trigger, item.controller)),
                    });
                }
            }
            Effect::ReturnTapped(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    if self.obj(id).zone == Zone::Graveyard {
                        self.move_to(id, Zone::Battlefield);
                        self.obj_mut(id).tapped = true;
                    }
                }
            }
            Effect::Exile(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    self.move_to(id, Zone::Exile);
                }
            }
            Effect::Bounce(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    self.move_to(id, Zone::Hand);
                }
            }
            Effect::Fight(n) => {
                // CR 701.12b: if either creature is gone, no damage is dealt.
                let me = src;
                if let Some(them) = Game::slot_object(item, legal, n) {
                    if self.obj(me).zone == Zone::Battlefield {
                        let (a, b) = (self.power(me), self.power(them));
                        if a > 0 {
                            self.deal_damage(me, Target::Object(them), a);
                        }
                        if b > 0 {
                            self.deal_damage(them, Target::Object(me), b);
                        }
                    }
                }
            }
            Effect::Tuck(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    let owner = self.obj(id).owner;
                    self.ask(owner, Decision::Tuck { card: id });
                }
            }
            Effect::Bite { from, to } => {
                let (Some(a), Some(b)) = (
                    Game::slot_object(item, legal, from),
                    Game::slot_object(item, legal, to),
                ) else {
                    return; // CR 701.12b-style: no damage if either is gone
                };
                let power = self.power(a);
                if power > 0 {
                    self.deal_damage(a, Target::Object(b), power);
                }
            }
            Effect::ExileGraveyard { who } => {
                for p in self.players_for(item, legal, who) {
                    for c in self.players[p].graveyard.clone() {
                        self.move_to(c, Zone::Exile);
                    }
                }
            }
        }
    }

    fn condition_holds(&self, ability: &Ability, controller: PlayerId, source: ObjId) -> bool {
        match &ability.condition {
            None => true,
            Some(Condition::Attacked) => self.attacked_this_turn[controller],
            Some(Condition::ControlPowerAtLeast(n)) => {
                self.creatures(controller).any(|c| self.power(c) >= *n)
            }
            Some(Condition::Kicked) => self.obj(source).kicked,
            Some(Condition::ControlAnotherPowerAtLeast(n)) => self
                .creatures(controller)
                .any(|c| c != source && self.power(c) >= *n),
            Some(Condition::CreatureDiedThisTurn) => self.creature_died_this_turn,
            Some(Condition::Always) => true,
            Some(Condition::OpponentLostLifeThisTurn) => self.life_lost_this_turn[opponent(controller)],
            Some(Condition::HadRevival) => self.obj(source).last_revival > 0,
            Some(Condition::SourceWasNot(sub)) => {
                let o = self.obj(source);
                !self.def_of(source).subtypes.iter().any(|x| x == sub) && !o.last_subtypes.contains(sub)
            }
            Some(Condition::Threshold) => self.players[controller].graveyard.len() >= 7,
            Some(Condition::ControlAnother(sub)) => self
                .creatures(controller)
                .any(|id| id != source && self.has_subtype(id, sub)),
        }
    }

    /// Creates a token on the battlefield. It is summoning sick like any new permanent.
    /// Doubling Season doubles tokens and counters (CR 614.1a).
    fn create_token(&mut self, p: PlayerId, def: usize) {
        self.last_created.clear();
        for _ in 0..self.doubling(p) {
            let id = self.add_card(p, def, Zone::Battlefield);
            self.last_created.push(id);
            self.obj_mut(id).since_turn = self.turn;
            self.zone_triggers(id, Zone::Gone, Zone::Battlefield, p);
        }
    }

    /// How many times over tokens and counters for `p` are made: 2 per Doubling Season.
    fn doubling(&self, p: PlayerId) -> i32 {
        let n = self
            .permanents(p)
            .filter(|&x| self.def_of(x).doubles_tokens_and_counters)
            .count();
        1 << n.min(10)
    }

    /// CR 701.8: destroy moves a permanent to its owner's graveyard, unless it is indestructible.
    fn destroy(&mut self, id: ObjId) {
        if !self.keywords(id).has(Keywords::INDESTRUCTIBLE) {
            self.move_to(id, Zone::Graveyard);
        }
    }

    /// CR 603.3: triggered abilities go on the stack the next time a player would get priority,
    /// active player's first (APNAP, CR 101.4). Returns false if a target choice is now pending.
    fn put_triggers_on_stack(&mut self, then: PlayerId) -> bool {
        let active = self.active;
        self.triggered.sort_by_key(|t| t.controller != active);
        while !self.triggered.is_empty() {
            let item = self.triggered.remove(0);
            let ability = self.ability_of(&item).clone();
            if !self.condition_holds(&ability, item.controller, item.source) {
                continue;
            }
            if item.mode.is_none() && !ability.modes.is_empty() {
                // CR 700.2b: the mode is chosen as the trigger goes on the stack.
                let any = ability.modes.iter().any(|m| self.trigger_can_target(&item, m));
                if any {
                    self.events.push(Event::Triggered { source: item.source });
                    let who = item.controller;
                    self.mode_pending = Some((item, then));
                    self.decision = Some(Decision::ChooseMode);
                    self.pending = Pending::Decide(who);
                    return false;
                }
                continue;
            }
            if item.mode.is_none() {
                self.events.push(Event::Triggered { source: item.source });
            }
            if ability.targets.len() == item.targets.len() {
                self.items.push(item);
            } else if self.trigger_can_target(&item, &ability) {
                let who = item.controller;
                self.choosing = Some(Choosing {
                    item,
                    then,
                    cast: false,
                    pay: None,
                    loyalty: None,
                });
                self.pending = Pending::ChooseTarget(who);
                return false;
            }
            // CR 603.3d: with no legal targets, the ability is removed from the stack.
        }
        true
    }

    /// State-based actions run before any player would receive priority (CR 117.5, 704.3).
    fn give_priority(&mut self, p: PlayerId) {
        self.state_based_actions();
        if self.is_over() || !self.put_triggers_on_stack(p) {
            return;
        }
        self.passes = if self.triggered.is_empty() { self.passes } else { 0 };
        self.pending = Pending::Priority(p);
    }

    /// Passes priority automatically when passing is the only legal action. Nothing is hidden by
    /// this: the player had no choice.
    fn settle(&mut self) {
        if !self.auto_pass {
            return;
        }
        while let Pending::Priority(p) = self.pending {
            if self.legal_actions().len() != 1 {
                break;
            }
            self.pass(p);
        }
    }

    // ---------------------------------------------------------------- turn structure

    fn end_step(&mut self) {
        let next = match self.step {
            Step::PreGame => Step::Untap,
            Step::Untap => Step::Upkeep,
            Step::Upkeep => Step::Draw,
            Step::Draw => Step::Main1,
            Step::Main1 => Step::BeginCombat,
            Step::BeginCombat => Step::DeclareAttackers,
            Step::DeclareAttackers => {
                // CR 508.8: no attackers, so skip declare blockers and combat damage.
                if self.combat.as_ref().is_some_and(|c| c.attackers.is_empty()) {
                    Step::EndCombat
                } else {
                    Step::DeclareBlockers
                }
            }
            Step::DeclareBlockers => {
                if self.combat.as_ref().is_some_and(|c| c.first_strike_step) {
                    Step::FirstStrikeDamage
                } else {
                    Step::CombatDamage
                }
            }
            Step::FirstStrikeDamage => Step::CombatDamage,
            Step::CombatDamage => Step::EndCombat,
            Step::EndCombat => Step::Main2,
            Step::Main2 => Step::End,
            Step::End => Step::Cleanup,
            Step::Cleanup => {
                self.begin_next_turn();
                return;
            }
        };
        self.enter(next);
    }

    fn begin_next_turn(&mut self) {
        self.attacked_this_turn = [false; 2];
        self.life_gains_this_turn = [0; 2];
        self.life_gained_this_turn = [0; 2];
        self.draws_this_turn = [0; 2];
        self.creature_died_this_turn = false;
        self.life_lost_this_turn = [false; 2];
        self.turn += 1;
        self.active = self.extra_turns.pop().unwrap_or_else(|| opponent(self.active));
        self.events.push(Event::TurnBegan {
            turn: self.turn,
            active: self.active,
        });
        self.enter(Step::Untap);
    }

    fn enter(&mut self, step: Step) {
        self.step = step;
        // CR 500.4: mana empties from each pool as a step or phase ends.
        for pl in &mut self.players {
            pl.pool.clear();
        }
        let a = self.active;
        match step {
            Step::PreGame => unreachable!("the game never returns to pre-game"),
            Step::Untap => {
                // CR 502.3: untap everything the active player controls. No priority (CR 502.4).
                for id in self.permanents(a).collect::<Vec<_>>() {
                    if self.obj(id).stun > 0 && self.obj(id).tapped {
                        self.obj_mut(id).stun -= 1; // CR 122.1d: a stun counter is removed instead
                    } else if !self.def_of(id).doesnt_untap {
                        let was_tapped = self.obj(id).tapped;
                        self.obj_mut(id).tapped = false;
                        if was_tapped && self.def_of(id).types.creature {
                            let zcc = self.obj(id).zcc;
                            let gear: Vec<ObjId> = self
                                .battlefield
                                .iter()
                                .copied()
                                .filter(|&e| self.obj(e).attached_to == Some((id, zcc)))
                                .collect();
                            for e in gear {
                                self.fire_source(e, When::EquippedBecomesUntapped);
                            }
                        }
                    }
                }
                self.players[a].lands_played = 0;
                self.enter(Step::Upkeep);
            }
            Step::Draw => {
                // CR 103.8a: the player who goes first skips their first draw.
                if self.turn > 1 {
                    self.draw(a);
                }
                self.give_priority(a);
            }
            Step::DeclareAttackers => {
                self.combat = Some(Combat::default());
                if self.attack_candidates(a).next().is_some() {
                    self.pending = Pending::DeclareAttackers(a);
                } else {
                    self.give_priority(a);
                }
            }
            Step::DeclareBlockers => {
                let attackers = self.combat.as_ref().expect("in combat").attackers.clone();
                let fs = attackers.iter().any(|&id| {
                    let k = self.keywords(id);
                    k.has(Keywords::FIRST_STRIKE) || k.has(Keywords::DOUBLE_STRIKE)
                });
                self.combat.as_mut().expect("in combat").first_strike_step = fs;
                let d = opponent(a);
                if self.block_actions(d) != [Action::FinishBlocks] {
                    self.pending = Pending::DeclareBlockers(d);
                } else {
                    self.give_priority(a);
                }
            }
            Step::FirstStrikeDamage => {
                self.combat_damage(true);
                self.give_priority(a);
            }
            Step::CombatDamage => {
                self.combat_damage(false);
                self.give_priority(a);
            }
            Step::EndCombat => {
                self.combat = None;
                self.give_priority(a);
            }
            Step::Cleanup => {
                if self.players[a].hand.len() > MAX_HAND
                    && !self.permanents(a).any(|c| self.def_of(c).no_max_hand_size)
                {
                    self.pending = Pending::Discard(a);
                } else {
                    self.finish_cleanup();
                }
            }
            Step::BeginCombat => {
                self.fire(a, When::BeginCombatYourTurn, None);
                for c in self.players[a].graveyard.clone() {
                    let whens: Vec<When> = self.def_of(c).triggers.iter().map(|tr| tr.when).collect();
                    for (n, w) in whens.into_iter().enumerate() {
                        if w == When::BeginCombatYourTurnFromGraveyard {
                            self.triggered.push(StackItem {
                                source: c,
                                controller: a,
                                kind: StackKind::Trigger(n),
                                targets: Vec::new(),
                                kicked: false,
                                flashback: false,
                                extra: None,
                                mode: None,
                                amount: 0,
                                alt: false,
                                x: 0,
                                division: Vec::new(),
                            });
                        }
                    }
                }
                self.give_priority(a);
            }
            Step::End => {
                // "At the beginning of the end step, sacrifice this token."
                let doomed: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| self.obj(c).sacrifice_at_end)
                    .collect();
                for c in doomed {
                    self.move_to(c, Zone::Graveyard);
                }
                // Delayed "at the beginning of the next end step" returns (CR 603.7).
                for (c, zcc, revival) in std::mem::take(&mut self.delayed_returns) {
                    let o = self.obj(c);
                    if o.zcc == zcc && matches!(o.zone, Zone::Exile | Zone::Graveyard) {
                        self.move_to(c, Zone::Battlefield);
                        let n = revival * self.doubling(self.obj(c).owner);
                        let o = self.obj_mut(c);
                        o.controller = o.owner;
                        o.revival = n;
                    }
                }
                // CR 513.1: "at the beginning of the end step" triggers.
                self.fire(a, When::EndStep { your_turn_only: true }, None);
                self.fire(
                    a,
                    When::EndStep {
                        your_turn_only: false,
                    },
                    None,
                );
                self.fire(
                    opponent(a),
                    When::EndStep {
                        your_turn_only: false,
                    },
                    None,
                );
                self.give_priority(a);
            }
            Step::Upkeep => {
                self.fire(a, When::Upkeep { your_turn_only: true }, None);
                self.fire(
                    a,
                    When::Upkeep {
                        your_turn_only: false,
                    },
                    None,
                );
                self.fire(
                    opponent(a),
                    When::Upkeep {
                        your_turn_only: false,
                    },
                    None,
                );
                self.give_priority(a);
            }
            Step::Main1 | Step::Main2 => {
                self.give_priority(a);
            }
        }
    }

    /// CR 514.2: damage wears off. No player gets priority in cleanup (CR 514.3).
    fn finish_cleanup(&mut self) {
        self.until_eot.clear(); // CR 514.2
        for (id, zcc, before) in std::mem::take(&mut self.control_until_eot) {
            if self.obj(id).zcc == zcc && self.obj(id).zone == Zone::Battlefield {
                let turn = self.turn;
                let o = self.obj_mut(id);
                o.controller = before;
                o.since_turn = turn;
            }
        }
        self.combat_damage_prevented.clear();
        for o in &mut self.objects {
            o.flashback_eot = false;
        }
        for id in self.battlefield.clone() {
            let o = self.obj_mut(id);
            o.damage = 0;
            o.deathtouched = false;
        }
        self.end_step();
    }

    // ---------------------------------------------------------------- combat damage

    /// Whether a creature deals damage in this combat damage step (CR 510.4, 702.7b, 702.4b).
    fn deals_damage_now(&self, id: ObjId, first_strike_step: bool) -> bool {
        let k = self.keywords(id);
        let (fs, ds) = (k.has(Keywords::FIRST_STRIKE), k.has(Keywords::DOUBLE_STRIKE));
        if first_strike_step {
            fs || ds
        } else {
            ds || !fs
        }
    }

    /// Damage that is lethal to `id` from a source (CR 702.2c: any damage from deathtouch).
    fn lethal_damage(&self, id: ObjId, deathtouch: bool) -> i32 {
        let left = (self.toughness(id) - self.obj(id).damage).max(0);
        if deathtouch {
            left.min(1)
        } else {
            left
        }
    }

    /// CR 510.1-510.2: every creature assigns its combat damage, then all of it is dealt at once.
    fn combat_damage(&mut self, first_strike_step: bool) {
        let combat = self.combat.clone().expect("in combat");
        let defender = opponent(self.active);
        let mut assigned: Vec<(ObjId, Target, i32)> = Vec::new();
        for &a in &combat.attackers {
            let power = self.power(a);
            if power <= 0 || !self.deals_damage_now(a, first_strike_step) {
                continue;
            }
            let k = self.keywords(a);
            let blockers: Vec<ObjId> = combat
                .blockers_of(a)
                .filter(|&b| self.obj(b).zone == Zone::Battlefield)
                .collect();
            if !combat.is_blocked(a) {
                match combat.at_planeswalker.iter().find(|&&(x, _)| x == a) {
                    // CR 506.4c: if the planeswalker left combat, the attacker deals no damage.
                    Some(&(_, w)) if self.obj(w).zone == Zone::Battlefield => {
                        assigned.push((a, Target::Object(w), power))
                    }
                    Some(_) => {}
                    None => assigned.push((a, Target::Player(defender), power)),
                }
                continue;
            }
            // CR 509.1h: a blocked creature stays blocked; with no blockers left it deals no damage
            // unless it has trample (CR 702.19e).
            let mut left = power;
            for (i, &b) in blockers.iter().enumerate() {
                let last = i + 1 == blockers.len();
                let amount = if last && !k.has(Keywords::TRAMPLE) {
                    left
                } else {
                    left.min(self.lethal_damage(b, k.has(Keywords::DEATHTOUCH)))
                };
                if amount > 0 {
                    assigned.push((a, Target::Object(b), amount));
                }
                left -= amount;
            }
            if left > 0 && k.has(Keywords::TRAMPLE) {
                match combat.at_planeswalker.iter().find(|&&(x, _)| x == a) {
                    Some(&(_, w)) if self.obj(w).zone == Zone::Battlefield => {
                        assigned.push((a, Target::Object(w), left))
                    }
                    Some(_) => {}
                    None => assigned.push((a, Target::Player(defender), left)),
                }
            }
        }
        for &(b, a) in &combat.blocks {
            let power = self.power(b);
            if power > 0
                && self.obj(b).zone == Zone::Battlefield
                && self.obj(a).zone == Zone::Battlefield
                && self.deals_damage_now(b, first_strike_step)
            {
                assigned.push((b, Target::Object(a), power));
            }
        }
        let mut hit_player = Vec::new();
        self.in_combat_damage = true;
        for (source, target, amount) in assigned {
            self.deal_damage(source, target, amount);
            if matches!(target, Target::Player(_)) && combat.attackers.contains(&source) {
                hit_player.push((source, amount));
            }
        }
        self.in_combat_damage = false;
        // CR 510.3a: "deals combat damage to a player" triggers.
        for &(a, _) in &hit_player {
            let controller = self.obj(a).controller;
            self.fire(controller, When::YourCreatureDealsCombatDamageToPlayer, None);
        }
        for (a, amount) in hit_player {
            let before = self.triggered.len();
            self.fire_source(a, When::DealsCombatDamageToPlayer);
            for t in &mut self.triggered[before..] {
                t.amount = amount;
            }
            let zcc = self.obj(a).zcc;
            let gear: Vec<ObjId> = self
                .battlefield
                .iter()
                .copied()
                .filter(|&e| self.obj(e).attached_to == Some((a, zcc)))
                .collect();
            for e in gear {
                self.fire_source(e, When::EquippedDealsCombatDamageToPlayer);
            }
        }
        self.state_based_actions();
    }

    /// Puts the triggers of one permanent that match `when` into the waiting list.
    fn fire_source(&mut self, id: ObjId, when: When) {
        if self.lost_abilities(id) {
            return;
        }
        let controller = self.obj(id).controller;
        let whens: Vec<When> = self.def_of(id).triggers.iter().map(|tr| tr.when).collect();
        for (n, w) in whens.into_iter().enumerate() {
            if w == when {
                self.triggered.push(StackItem {
                    source: id,
                    controller,
                    kind: StackKind::Trigger(n),
                    targets: Vec::new(),
                    kicked: false,
                    flashback: false,
                    extra: None,
                    mode: None,
                    amount: 0,
                    alt: false,
                    x: 0,
                    division: Vec::new(),
                });
            }
        }
    }

    /// CR 120.3: damage to a player is life loss; to a creature it is marked. Lifelink gains life
    /// for the source's controller (CR 702.15b).
    /// Twinflame Tyrant: 2 per copy its controller has when damage goes to an opponent or a
    /// permanent they control.
    fn damage_doubling(&self, source: ObjId, target: Target) -> i32 {
        let c = self.obj(source).controller;
        let to_opponent = match target {
            Target::Player(p) => p != c,
            Target::Object(id) => self.obj(id).controller != c,
        };
        if !to_opponent {
            return 1;
        }
        let n = self
            .permanents(c)
            .filter(|&x| self.def_of(x).doubles_damage_to_opponents)
            .count();
        1 << n.min(10)
    }

    fn deal_damage(&mut self, source: ObjId, target: Target, amount: i32) {
        if let Target::Object(id) = target {
            if self.protected_from(id, source) {
                return; // CR 702.16e: damage from a source of that color is prevented
            }
            let zcc = self.obj(id).zcc;
            if self.in_combat_damage && self.combat_damage_prevented.contains(&(id, zcc)) {
                return;
            }
            let controller = self.obj(id).controller;
            if !self.in_combat_damage
                && self
                    .permanents(controller)
                    .any(|b| b != id && self.def_of(b).prevent_noncombat_to_others)
            {
                return;
            }
        }
        let amount = amount * self.damage_doubling(source, target);
        let k = self.keywords(source);
        match target {
            Target::Player(p) => {
                self.lose_life(p, amount);
                let controller = self.obj(source).controller;
                if !self.in_combat_damage && controller != p {
                    let before = self.triggered.len();
                    self.fire(controller, When::YourSourceDealsNoncombatDamageToOpponent, None);
                    for t in &mut self.triggered[before..] {
                        t.amount = amount;
                    }
                }
            }
            Target::Object(id) if self.def_of(id).types.planeswalker && !self.def_of(id).types.creature => {
                self.obj_mut(id).loyalty -= amount; // CR 120.3c
            }
            Target::Object(id) => {
                let o = self.obj_mut(id);
                o.damage += amount;
                o.deathtouched |= k.has(Keywords::DEATHTOUCH);
            }
        }
        self.events.push(Event::Damage {
            source,
            target,
            amount,
        });
        if k.has(Keywords::LIFELINK) {
            let c = self.obj(source).controller;
            self.gain_life(c, amount);
        }
    }

    // ---------------------------------------------------------------- state-based actions

    /// CR 704: repeat until none apply.
    fn state_based_actions(&mut self) {
        loop {
            self.work();
            let mut dies = Vec::new();
            for &id in &self.battlefield {
                if !self.defs[self.objects[id.0 as usize].def].types.creature {
                    continue;
                }
                let o = self.obj(id);
                let t = self.toughness(id);
                let indestructible = self.keywords(id).has(Keywords::INDESTRUCTIBLE);
                if t <= 0 {
                    dies.push(id); // CR 704.5f
                } else if !indestructible && (o.damage >= t || (o.deathtouched && o.damage > 0)) {
                    dies.push(id); // CR 704.5g, 704.5h
                }
            }
            dies.extend(self.legend_rule_losers());
            // CR 704.5i: a planeswalker with no loyalty goes to the graveyard.
            dies.extend(
                self.battlefield
                    .iter()
                    .copied()
                    .filter(|&w| self.def_of(w).types.planeswalker && self.obj(w).loyalty <= 0),
            );
            // CR 704.5m-n: an Aura attached to nothing legal goes to the graveyard; an Equipment
            // becomes unattached.
            for &id in &self.battlefield.clone() {
                if let Some((t, z)) = self.obj(id).attached_to {
                    let o = self.obj(t);
                    let ok = o.zone == Zone::Battlefield && o.zcc == z && self.def_of(t).types.creature;
                    if !ok {
                        if self.def_of(id).aura {
                            dies.push(id);
                        } else {
                            self.obj_mut(id).attached_to = None;
                        }
                    }
                }
            }
            // CR 704.5d: a token outside the battlefield ceases to exist.
            let ceased: Vec<ObjId> = self
                .objects
                .iter()
                .filter(|o| {
                    (self.defs[o.def].token || o.copy_token)
                        && !matches!(o.zone, Zone::Battlefield | Zone::Gone)
                })
                .map(|o| o.id)
                .collect();
            for id in ceased {
                self.move_to(id, Zone::Gone);
            }
            dies.sort_unstable();
            dies.dedup();
            let mut lost = false;
            for p in 0..2 {
                let pl = &self.players[p];
                // "You can't lose the game" (Herald of Eternal Dawn).
                let protected = self.permanents(p).any(|c| self.def_of(c).cant_lose);
                if !pl.lost && !protected && (pl.life <= 0 || pl.drew_from_empty) {
                    self.players[p].lost = true; // CR 704.5a, 704.5b
                    self.events.push(Event::Lost { player: p });
                    lost = true;
                }
            }
            if lost {
                self.result = Some(match (self.players[0].lost, self.players[1].lost) {
                    (true, true) => Outcome::Draw,
                    (true, false) => Outcome::Winner(1),
                    _ => Outcome::Winner(0),
                });
                self.pending = Pending::GameOver;
                return;
            }
            if dies.is_empty() {
                return;
            }
            for id in dies {
                self.move_to(id, Zone::Graveyard);
            }
        }
    }

    /// CR 704.5j: of two legendary permanents with the same name and controller, one goes to the
    /// graveyard. The player would choose; the engine keeps the newest (docs/engine/QUESTIONS.md).
    fn legend_rule_losers(&self) -> Vec<ObjId> {
        let mut out = Vec::new();
        for (i, &a) in self.battlefield.iter().enumerate() {
            let da = self.def_of(a);
            if !da.types.legendary {
                continue;
            }
            let newer = self.battlefield[i + 1..]
                .iter()
                .any(|&b| self.obj(b).controller == self.obj(a).controller && self.def_of(b).name == da.name);
            if newer {
                out.push(a);
            }
        }
        out
    }

    // ---------------------------------------------------------------- invariants

    /// Checks that the state is internally consistent. The fuzzer calls this after every action.
    pub fn check_invariants(&self) -> Result<(), String> {
        let mut seen = vec![0u8; self.objects.len()];
        let mut mark = |ids: &[ObjId], zone: Zone, owner: Option<PlayerId>| -> Result<(), String> {
            for &id in ids {
                let o = &self.objects[id.0 as usize];
                if o.zone != zone || owner.is_some_and(|p| p != o.owner) {
                    return Err(format!("{id:?} listed in {zone:?} of {owner:?} but is {o:?}"));
                }
                seen[id.0 as usize] += 1;
            }
            Ok(())
        };
        for (p, pl) in self.players.iter().enumerate() {
            mark(&pl.library, Zone::Library, Some(p))?;
            mark(&pl.hand, Zone::Hand, Some(p))?;
            mark(&pl.graveyard, Zone::Graveyard, Some(p))?;
        }
        mark(&self.battlefield, Zone::Battlefield, None)?;
        mark(&self.stack, Zone::Stack, None)?;
        mark(&self.exile, Zone::Exile, None)?;
        mark(&self.gone, Zone::Gone, None)?;
        if let Some(i) = seen.iter().position(|&n| n != 1) {
            return Err(format!("object {i} is listed in {} zones", seen[i]));
        }
        for o in &self.objects {
            if o.damage < 0 {
                return Err(format!("negative damage on {o:?}"));
            }
            if o.zone != Zone::Battlefield && (o.tapped || o.damage != 0 || o.counters != 0) {
                return Err(format!("off-battlefield object carries state: {o:?}"));
            }
        }
        let spells: Vec<ObjId> = self
            .items
            .iter()
            .filter(|i| i.kind == StackKind::Spell)
            .map(|i| i.source)
            .collect();
        let casting = self.choosing.as_ref().filter(|c| c.cast).map(|c| c.item.source);
        let resolving = self
            .resolving
            .as_ref()
            .filter(|r| r.item.kind == StackKind::Spell)
            .map(|r| r.item.source);
        let mut on_stack = self.stack.clone();
        on_stack.retain(|id| Some(*id) != casting && Some(*id) != resolving);
        if on_stack != spells {
            return Err(format!(
                "stack cards {:?} disagree with spell items {spells:?}",
                self.stack
            ));
        }
        let loss = self
            .battlefield
            .iter()
            .filter(|&&id| self.def_of(id).statics.iter().any(|s| s.lose_abilities))
            .count() as u32;
        let mut with_statics: Vec<ObjId> = self
            .battlefield
            .iter()
            .copied()
            .filter(|&id| !self.def_of(id).statics.is_empty())
            .collect();
        let mut listed = self.static_sources.clone();
        with_statics.sort_unstable();
        listed.sort_unstable();
        if with_statics != listed {
            return Err(format!(
                "static sources {listed:?} but {with_statics:?} on the battlefield"
            ));
        }
        if loss != self.ability_loss_sources {
            return Err(format!(
                "ability-loss count {} but {loss} on the battlefield",
                self.ability_loss_sources
            ));
        }
        if self.is_over() != (self.pending == Pending::GameOver) {
            return Err("result and pending disagree".into());
        }
        Ok(())
    }
}

/// The total of two mana costs (kicker on top of the base cost).
fn add_costs(a: &ManaCost, b: &ManaCost) -> ManaCost {
    let mut c = *a;
    c.generic += b.generic;
    for i in 0..5 {
        c.colored[i] += b.colored[i];
    }
    c
}
