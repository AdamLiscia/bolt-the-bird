//! The game: state, legal actions, and the only function that changes state, [`Game::apply`].
//!
//! Rules are cited by Comprehensive Rules (CR) section. Simplifications are listed in
//! docs/engine/QUESTIONS.md, each with the reason and how to reverse it.

use std::sync::Arc;

use crate::card::{Activated, Behold, CardDef, EntryDoubler, ExtraCost, Keywords, Reduction, Types};
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
/// Commander (CR 903.7): each player starts at 40.
pub const COMMANDER_LIFE: i32 = 40;
/// Commander (CR 903.10a, 704.6c): 21 or more combat damage from one commander loses the game.
pub const COMMANDER_DAMAGE_LOSS: i32 = 21;
/// Opening hand size (CR 103.5).
pub const HAND_SIZE: usize = 7;
/// Maximum hand size, enforced in the cleanup step (CR 402.2, 514.1).
pub const MAX_HAND: usize = 7;
/// The most work (zone changes and state-based-action passes) one action may do. Real actions
/// stay far below it, even with a thousand permanents; reaching it means an engine loop that never
/// ends, so the action panics rather than eat the machine's memory.
pub const WORK_BUDGET: u64 = 1_000_000;

/// Spells and abilities one turn may resolve before the game is a draw (CR 104.4b: a loop of
/// mandatory actions with no way to stop). Real turns resolve a few dozen.
pub const RESOLUTIONS_PER_TURN: u32 = 10_000;

/// What mana is being spent on, for mana that can only pay for some things.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PayFor {
    Spell(ObjId),
    Ability(ObjId),
    Other,
    /// A card cast as its Omen half: an Omen spell (not a Dragon or other creature spell).
    OmenSpell(ObjId),
}

/// What a spell's mana pays for: the card, or the card cast as its Omen half.
fn pay_for_spell(c: ObjId, mode: Option<u8>) -> PayFor {
    if mode == Some(OMEN_MODE) {
        PayFor::OmenSpell(c)
    } else {
        PayFor::Spell(c)
    }
}

/// Land subtypes in FDN (CR 205.3i); every other subtype is a creature type.
const LAND_TYPES: [&str; 5] = ["Plains", "Island", "Swamp", "Mountain", "Forest"];

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
    /// Commander: where each player's commander starts and returns to (CR 408, 903.6).
    Command,
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
    /// Commander: cast from the command zone, and the tax that cast paid (CR 903.8).
    pub from_command: bool,
    pub commander_tax: u8,
    /// Exiled instead if it would die during this turn, while it is this object: (zcc, turn).
    pub exile_if_dies: Option<(u32, u32)>,
    /// May be cast without paying its mana cost, during an open free-cast window (Etali).
    pub cast_free: bool,
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
    /// Became a creature with this base power and toughness (Soulstone Sanctuary).
    pub animated: Option<(i32, i32)>,
    /// The stage it has become (Kellan); 0 is as printed.
    pub stage: u8,
    pub fellowship: i32,
    /// Who may play this card from exile, and through which turn.
    pub playable: Option<(PlayerId, u32)>,
    /// Bait counters (Fishing Pole) and a stash counter (Tinybones).
    pub bait: i32,
    pub stash: bool,
    /// The turn this object's once-each-turn mana ability (the Devotees) was last used.
    pub filter_used_turn: u32,
    /// Keyword counters on it (CR 122.1b), kept apart from other granted keywords.
    pub keyword_counters: Keywords,
    /// Lore counters on a Saga (CR 714).
    pub lore: u8,
    /// A token that's a copy of a card (its definition isn't a token's).
    pub copy_token: bool,
    /// Sacrifice it at the beginning of the next end step (Electroduplicate).
    pub sacrifice_at_end: bool,
    /// The turn it was last put into a graveyard.
    pub graveyard_turn: u32,
    /// Loyalty (CR 306.5), and the turn a loyalty ability was last activated (CR 606.3).
    pub loyalty: i32,
    pub loyalty_turn: u32,
    /// "Can't be countered", from an effect as it was cast (Mistrise Village).
    pub cast_uncounterable: bool,
    /// It entered the battlefield as a resolving spell: "if you cast it".
    pub was_cast: bool,
    /// It has dealt damage since it entered the battlefield.
    pub dealt_damage: bool,
    /// The turn it may attack as though it didn't have defender (Krotiq Nestguard).
    pub defender_ignored_turn: u32,
    /// The mode chosen as it entered (the Sieges); kept after it leaves, for triggers it left behind.
    pub chosen_mode: Option<u8>,
    /// The turn its "triggers only once each turn" ability last triggered (Hollowmurk Siege).
    pub once_each_turn: u32,
    /// The turn counters were last put on it (for "the first time counters have been put on that
    /// creature this turn").
    pub counters_put_turn: u32,
    /// The counters it had when it last left the battlefield: +1/+1, stun and keyword counters.
    pub last_counters: (i32, i32, Keywords),
    /// The card an ability of this permanent exiled and refers back to (Severance Priest).
    pub linked_card: Option<ObjId>,
    /// A finality counter: if it would die, it's exiled instead.
    pub finality: bool,
    /// Cast from the graveyard with "it enters with a finality counter on it".
    pub enters_with_finality: bool,
    /// It's a creature until end of turn of this turn, in addition to its other types: (turn, base
    /// power, base toughness, creature type, keywords) (Reverberating Summons).
    pub eot_creature: Option<(u32, i32, i32, &'static str, Keywords)>,
    /// Gained harmonize until end of turn, for its mana cost (Songcrafter Mage).
    pub harmonize_eot: bool,
    /// A commander just put into a graveyard or exile whose owner hasn't chosen yet (CR 903.9a).
    pub commander_choice_pending: bool,
    /// How many times this object's abilities resolved this turn: (turn, count).
    pub resolved_this_turn: (u32, u32),
    /// How many times this commander has been cast from the command zone (its own tax, CR 903.8).
    pub command_zone_casts: u8,
    /// A creature type it has until end of turn: (turn, type) (Sarkhan, Dragon Ascendant).
    pub eot_subtype: Option<(u32, &'static str)>,
    /// A copy of a card (CR 707.12): it exists only on the stack, on the battlefield (as a token),
    /// or in exile while it waits to be cast.
    pub card_copy: bool,
    /// Its offspring cost was paid as it was cast (it makes a 1/1 copy when it enters).
    pub offspring_paid: bool,
    /// A token copy with its own base power and toughness (offspring's 1/1 copies).
    pub copy_base_pt: Option<(i32, i32)>,
    /// A decayed counter (CR 702.147).
    pub decayed: bool,
    /// Sacrifice it at end of combat (decayed, after it attacked).
    pub sacrifice_end_combat: bool,
    /// Generic mana its delve pays while it's being cast: that many graveyard cards are exiled.
    pub delve: u8,
    /// Its own card while it's a copy of another (CR 707.2): restored when it leaves the battlefield.
    pub printed_def: Option<usize>,
    /// Time counters on a suspended card in exile (CR 702.62).
    pub time_counters: u8,
    /// Cast from suspend: a creature has haste while it stays on the battlefield (CR 702.62a).
    pub suspend_haste: bool,
    /// It doesn't untap during its controller's next untap step.
    pub skip_untap: bool,
    /// Quest counters (Beastmaster Ascension).
    pub quest: i32,
    /// The turn an "Activate only once each turn" ability of it was last activated.
    pub activated_turn: Option<u32>,
    /// Counters of the card's own named kind (hoofprint, acorn), which nothing else reads.
    pub kind_counters: i32,
    /// A copy that isn't legendary (Helm of the Host: "except the token isn't legendary").
    pub not_legendary: bool,
    /// A shock land that entered and hasn't had its "pay 2 life" choice yet.
    pub shock_pending: bool,
    /// The turn it was last exerted (CR 701.39).
    pub exerted_turn: Option<u32>,
    /// Regeneration shields this turn: (turn, how many) (CR 701.15).
    pub regen_shields: (u32, u8),
    /// The turn it was last put into a graveyard from a library.
    pub milled_turn: u32,
    /// An eternalized token: black, with no mana cost (CR 702.129a).
    pub eternalized: bool,
    /// Times it has attacked this turn: (turn, count).
    pub attacks_turn: (u32, u8),
    /// The turn its activated abilities can't be activated (Koma: "Tap target permanent. Its
    /// activated abilities can't be activated this turn."); 0: never.
    pub activations_stopped_turn: u32,
    /// A transforming double-faced permanent with its back face up (CR 712.8e): its characteristics
    /// are its definition's `back`. Front face up again once it leaves the battlefield (CR 711.8).
    pub transformed: bool,
    /// It entered as a copy with its own card's exceptions (`CardDef::copy_artifact`,
    /// `copy_keeps_name`, `copy_keeps_abilities` of `printed_def`); cleared when it leaves.
    pub copy_exceptions: bool,
    /// Face down (CR 708): a 2/2 creature with no name, abilities, types other than creature, color or
    /// mana cost (manifest, CR 701.40). Face up again when it leaves the battlefield.
    pub face_down: bool,
    /// Conduit of Worlds: the turn it may be cast from its owner's graveyard (if no spell was cast).
    pub conduit_turn: u32,
    /// Soulbond: the creature it's paired with (CR 702.95), and that creature's zcc.
    pub paired_with: Option<(ObjId, u32)>,
    /// Returned with unearth: exiled at the next end step or instead of leaving the battlefield.
    pub unearthed: bool,
    /// Exile it at the beginning of the next end step (hasty token copies: Molten Echoes, Rionya).
    pub exile_at_end: bool,
    /// Return it to the command zone at the beginning of the next end step (Hellkite Courser).
    pub command_at_end: bool,
    /// A token copy that's an enchantment and no other card type (Myrkul).
    pub copy_enchantment_only: bool,
    /// Its hideaway card (exiled face down), if any.
    pub hideaway: Option<ObjId>,
    /// Exile it at end of combat (Delina's and Mirage Phalanx's tokens).
    pub exile_end_combat: bool,
    /// A copy that "loses soulbond".
    pub lost_soulbond: bool,
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
    /// Poison counters (CR 122.1f): ten or more and the player loses (CR 704.5c).
    pub poison: i32,
    /// Energy counters (CR 122.1).
    pub energy: u32,
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
            poison: 0,
            lost: false,
            energy: 0,
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
    /// Remove a counter from a creature you control, as a cost of the spell being cast.
    RemoveCounters(PlayerId),
    /// Cast any of [`Game::free_casts`] without paying mana costs, or pass to stop (Etali).
    FreeCast(PlayerId),
    GameOver,
}

impl Pending {
    /// The player who has to act, or `None` once the game is over.
    pub fn player(&self) -> Option<PlayerId> {
        match *self {
            Pending::Mulligan(p)
            | Pending::Bottom(p, _)
            | Pending::Priority(p)
            | Pending::DeclareAttackers(p)
            | Pending::DeclareBlockers(p)
            | Pending::Discard(p)
            | Pending::ChooseTarget(p)
            | Pending::Decide(p)
            | Pending::Divide(p)
            | Pending::RemoveCounters(p)
            | Pending::FreeCast(p) => Some(p),
            Pending::GameOver => None,
        }
    }
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
    /// Cast from the graveyard by removing counters (Quilled Greatwurm).
    CastFromGraveyard(ObjId),
    /// Cast a permanent spell from your graveyard as permanent type `t` (0 artifact, 1 creature,
    /// 2 enchantment, 4 planeswalker) with Muldrotha.
    CastFromGraveyardAs(ObjId, u8),
    /// Tap (or sacrifice) a mana source for mana of color `c` (0-4 WUBRG, 5 colorless), into the pool.
    /// Offered only to players with manual mana on, at priority; it doesn't use the stack (CR 605.3).
    TapForMana(ObjId, u8),
    /// Cast with harmonize from the graveyard, tapping this creature to pay {X} less (X its power).
    CastHarmonize(ObjId, ObjId),
    /// Cast the card's Omen half (Tarkir: Dragonstorm).
    CastOmen(ObjId),
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
    /// A copy of a spell (CR 707.10): its source is the copied card, and it ceases to exist when
    /// it leaves the stack instead of moving the card.
    SpellCopy,
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
    /// Offspring's "when this creature enters, create a 1/1 token copy of it" (CR 702.175).
    Offspring,
    /// Ward—discard a card.
    WardDiscard {
        against: ObjId,
    },
    /// Ward—sacrifice `n` permanents (Ulamog, the Defiler).
    WardSacrifice {
        n: u8,
        against: ObjId,
    },
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
    divide_lands: false,
    targets_exactly_x: false,
};

/// Offspring's triggered ability (CR 702.175a): a 1/1 token copy of the creature.
fn offspring_ability() -> &'static Ability {
    static A: std::sync::OnceLock<Ability> = std::sync::OnceLock::new();
    A.get_or_init(|| Ability::new(Vec::new(), vec![Effect::OffspringCopy]))
}

/// What a face-down permanent is (CR 708.2a): a 2/2 creature with no name, text, subtypes, color
/// or mana cost.
pub fn face_down_def() -> &'static CardDef {
    static D: std::sync::OnceLock<CardDef> = std::sync::OnceLock::new();
    D.get_or_init(|| {
        let mut d = CardDef::creature("", "", 2, 2, Keywords::NONE);
        d.colors = [false; 5];
        d
    })
}

/// "{1}{U}: Exile this creature, then return it to the battlefield under your control." (granted by
/// Deadeye Navigator's soulbond).
fn soulbond_flicker_ability() -> &'static Activated {
    static A: std::sync::OnceLock<Activated> = std::sync::OnceLock::new();
    A.get_or_init(|| Activated {
        mana: ManaCost::parse("{1}{U}").expect("cost"),
        ability: Ability::new(Vec::new(), vec![Effect::ExileSelfReturn { transformed: false }]),
        ..Activated::default()
    })
}

/// "{1}: This creature fights target creature you don't control." (granted by The Grand Evolution).
fn granted_fight_ability() -> &'static Activated {
    static A: std::sync::OnceLock<Activated> = std::sync::OnceLock::new();
    A.get_or_init(|| Activated {
        mana: ManaCost {
            generic: 1,
            ..ManaCost::default()
        },
        ability: Ability::new(
            vec![Filter::CREATURE.controlled_by(Controller::Opponent)],
            vec![Effect::Fight(0)],
        ),
        ..Activated::default()
    })
}

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

/// `CastMode::mode` (and `StackItem::mode`) for a spell cast as its Omen half.
pub const OMEN_MODE: u8 = 255;

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
    /// Counters still to remove from creatures you control as an additional cost.
    pub remove_counters: u8,
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
    Discard { left: u8, optional: bool, then_draw: u8 },
    /// Choose a nonland card from `victim`'s hand for them to discard.
    PickDiscard { victim: PlayerId },
    /// Surveil: `Yes` puts the top of `cards` into the graveyard, `No` keeps it on top.
    Surveil { cards: Vec<ObjId> },
    /// Scry: `Yes` puts the top of `cards` on the bottom of the library, `No` keeps it on top.
    Scry { cards: Vec<ObjId>, kept: Vec<ObjId> },
    /// Scry, then order: `Pick` the card to go on top next, from those kept (CR 701.22a "in any order").
    ScryOrder { kept: Vec<ObjId>, placed: Vec<ObjId> },
    /// Endure `n`: `Yes` puts the counters on the source, `No` makes an `n`/`n` Spirit token.
    Endure { n: u8, spirit: usize },
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
    /// "Counter target spell unless its controller pays": `Yes` pays `cost`, `No` lets the spell from
    /// `against` be countered.
    PayOrCountered { cost: ManaCost, against: ObjId },
    /// `Pick` a creature you control to exile.
    ExileCreature,
    /// "As this enters, choose [A] or [B]": `Mode(0)` or `Mode(1)` (the Sieges).
    ChooseEnterMode { card: ObjId },
    /// Choose a nonland card from `victim`'s hand to exile.
    PickExile { victim: PlayerId },
    /// Exile `left` more cards from your graveyard (creature cards only, if `creature_only`);
    /// before the first, `No` declines and skips the rest of the effects.
    ExileFromGraveyard {
        left: u8,
        creature_only: bool,
        started: bool,
    },
    /// `Pick` a nonland permanent you control to sacrifice, or `No`.
    SacrificeNonland,
    /// `Pick` cards from `cards` to put onto the battlefield (`left` more; `u8::MAX`: any number),
    /// or `No` to stop; the rest go to your hand or to the bottom in a random order.
    PutFromAmong {
        cards: Vec<ObjId>,
        left: u8,
        noncreature_nonland: bool,
        max_mv: Option<u32>,
        rest_to_hand: bool,
    },
    /// `Pick` a permanent card in your graveyard whose name isn't in `names` to return, or `No`.
    ReturnDifferentNames { names: Vec<String> },
    /// Discard a card (`No` stops); then draw one for each discarded.
    DiscardUpToDraw { left: u8, discarded: u8 },
    /// You may `Pick` a nonland card from `victim`'s hand to exile, linked to `source`; or `No`.
    PickExileLinked { victim: PlayerId, source: ObjId },
    /// `Pick` a card from your graveyard to put on top of your library, `left` more to go.
    PutOnTopFromGraveyard { left: u8 },
    /// `Pick` a Dragon you control of color `color` (WUBRG) for a +1/+1 counter; `placed` so far.
    DragonForColor { color: u8, placed: Vec<ObjId> },
    /// `Discard` a card, or `No` to stop the rest of the effect ("You may discard a card. When you
    /// do, ...").
    MayDiscardOrStop,
    /// `Pick` the card of `cards` (in the library) to put on the bottom next; each goes under the
    /// one before.
    BottomOrder { cards: Vec<ObjId> },
    /// `Pick` a source for `card`'s damage shield (trigger number `trigger`).
    ChooseDamageSource { card: ObjId, trigger: usize },
    /// Discard a card or `No`; a nonland card discarded triggers `source`'s trigger `trigger`.
    DiscardReflexive { source: ObjId, trigger: usize },
    /// Discard a creature card, or two cards (`second`: one non-creature already discarded).
    DiscardTwoUnlessCreature { second: bool },
    /// `Yes`: the creature `target` gets `n` +1/+1 counters; `No`: an `n`/`n` Spirit (endure, for a
    /// creature other than the source).
    EndureOther { target: ObjId, n: u8, spirit: usize },
    /// Commander (CR 903.9a): `Yes` moves `card` from its graveyard or exile to the command zone,
    /// `No` leaves it there.
    CommanderToCommandZone { card: ObjId },
    /// `Pick` a permanent card in your graveyard to return to your hand (`left` more), or `No`.
    GraveyardToHand { left: u8 },
    /// `Pick` a card in `owner`'s graveyard to shuffle into their library (`left` more), or `No`.
    GraveyardShuffle { owner: PlayerId, left: u8, picked: bool },
    /// `Pick` a monocolored card in your library whose name isn't among `picked`'s, or `No`.
    SearchMonoNames { picked: Vec<ObjId> },
    /// The opponent `Pick`s one of `cards` (exiled) to shuffle into `caster`'s library.
    OpponentShufflesOne { cards: Vec<ObjId>, caster: PlayerId },
    /// Ward—discard a card: `Discard` one, or `No` lets the spell or ability from `against` be
    /// countered.
    PayWardDiscard { against: ObjId },
    /// Choose X for a spell cast with `mode` (an Omen or harmonize cast whose cost has {X}):
    /// `Amount(x)` for each X you can pay, 0 through `max`.
    ChooseCastX { card: ObjId, mode: CastMode, max: u8 },
    /// Delve: `Amount(n)` exiles n cards from your graveyard to pay {n} of the spell's generic cost
    /// (0 through `max`, only amounts that leave the rest payable).
    ChooseDelve { card: ObjId, mode: CastMode, max: u8 },
    /// "You may have this creature enter as a copy of any creature on the battlefield": `Pick` one,
    /// or `No`.
    EnterAsCopy { card: ObjId },
    /// `Pick` a creature you control to return to its owner's hand.
    ReturnOwnCreature,
    /// `Pick` a land you control to return to its owner's hand.
    ReturnOwnLand,
    /// `Pick` a creature or planeswalker you control to sacrifice.
    SacrificeCreatureOrPlaneswalker,
    /// `Pick` a token you control to sacrifice, or `No`.
    SacrificeToken,
    /// A tempting offer: `Yes` to search for a land too (the caster then gets another), or `No`.
    TemptOffer,
    /// A shock land entered: `Yes` pays 2 life, `No` leaves it tapped.
    ShockLand { card: ObjId },
    /// `Pick` a creature card in hand to put onto the battlefield tapped and attacking, or `No`.
    PutCreatureAttacking,
    /// Unleash: `Yes` for a +1/+1 counter on it, or `No`.
    Unleash { card: ObjId },
    /// `Pick` a land to sacrifice, or `No` to sacrifice `source` (The Gitrog Monster).
    SacrificeLandOrThis { source: ObjId },
    /// `Pick` a land card from your hand to put onto the battlefield tapped, or `No` to stop.
    LandsFromHand,
    /// "Flashback—Sacrifice N creatures": `Pick` a creature to sacrifice, `left` more to go; then the
    /// caster gets priority (Dread Return).
    FlashbackSacrifice { left: u8 },
    /// Scapeshift: `Pick` a land to sacrifice (`n` so far), or `No` to search for up to `n` lands.
    SacrificeLandsThenSearch { n: u8 },
    /// `Pick` a creature to sacrifice from `cards` (tied for the greatest power).
    SacrificeOneOf { cards: Vec<ObjId> },
    /// `Pick` a creature to sacrifice; then you may return another permanent card from your graveyard
    /// to the battlefield (Rise of the Witch-king).
    SacrificeThenReturnOther,
    /// `Pick` a permanent card in your graveyard other than `except` to return to the battlefield, or
    /// `No`.
    ReturnOtherPermanent { except: ObjId },
    /// `Pick` one of `cards` to put onto the battlefield (`left` more; creature cards only, or with
    /// `creature_and_land` one creature card and one land card), or `No` to stop; then, with
    /// `rest_bottom`, the rest go on the bottom of your library in a random order.
    PutPicksOntoBattlefield {
        cards: Vec<ObjId>,
        picked: Vec<ObjId>,
        left: u8,
        creature_and_land: bool,
        rest_bottom: bool,
    },
    /// Choose a new target for target number `slot` of the stack entry `items[item]` (CR 115.7):
    /// `Target` a legal new one, or `No` to keep it.
    ChooseNewTarget { item: usize, slot: u8 },
    /// Manifest dread (CR 701.62): `Pick` which of `cards` (the top two) to manifest; the other
    /// goes to the graveyard.
    ManifestDread { cards: Vec<ObjId> },
    /// `Pick` a face-down permanent you control to turn face up, or `No` (Zimone).
    TurnFaceUp,
    /// `Pick` a card in your hand to put on top of your library, `left` more to go.
    HandToTop { left: u8 },
    /// Scroll Rack: `Pick` a card in your hand to exile face down (`exiled` so far), or `No` to stop.
    ScrollRackExile { exiled: Vec<ObjId> },
    /// `Pick` a land card from your hand to put onto the battlefield, or `No`.
    LandFromHand,
    /// Soulbond: `Pick` another unpaired creature you control to pair with `source`, or `No`.
    SoulbondPair { source: ObjId },
    /// `Pick` a land card in your hand to put onto the battlefield tapped, or `No` (once).
    LandFromHandTapped,
    /// "Unless that player pays": `Yes` pays `cost` and stops the rest of the effects, `No` lets them
    /// happen.
    PayToStop { cost: ManaCost },
    /// Cumulative upkeep: `Yes` pays `cost`, `No` sacrifices `source`.
    PayUpkeep { cost: ManaCost, source: ObjId },
    /// `Pick` a creature you control for the next of `left` +1/+1 counters.
    DistributeCounters { left: u8 },
    /// `Pick` a permanent you control to sacrifice, `left` more to go.
    SacrificePermanents { left: u8 },
    /// Ward—sacrifice permanents: `Pick` one (`left` more, `picked` so far), or before the first, `No`
    /// lets the spell or ability from `against` be countered.
    PayWardSacrifice { left: u8, against: ObjId, picked: bool },
    /// Sneak Attack: `Pick` a creature card in your hand to put onto the battlefield, or `No`.
    SneakAttack,
    /// `Pick` one of `cards` (exiled) to put onto the battlefield, or `No` to stop; the rest stay.
    ExiledToBattlefield { cards: Vec<ObjId> },
    /// `Pick` a creature card in your hand to put onto the battlefield, or `No` to stop (Ghalta).
    CreaturesFromHand,
    /// `Pick` another creature you control to return to its owner's hand, or `No` (Temur
    /// Sabertooth).
    ReturnAnotherForIndestructible { source: ObjId },
    /// `Pick` a land card in your hand to put onto the battlefield tapped (then draw and repeat), or
    /// `No` (Cultivator Colossus).
    LandTappedDrawRepeat,
    /// `Pick` a basic land card in your hand to put onto the battlefield tapped, or `No`.
    BasicFromHandTapped,
    /// Protean Hulk: `Pick` a creature card with mana value at most `budget`, or `No` to stop.
    HulkSearch { budget: u32 },
    /// Primal Surge: `Yes` puts `card` (a permanent card) onto the battlefield and repeats, `No`
    /// stops.
    PrimalSurge { card: ObjId },
    /// Hellkite Courser: `Yes` puts your commander onto the battlefield, `No` doesn't.
    CommanderCourser { card: ObjId },
    /// Hideaway: `Pick` one of `cards` (the top ones) to exile face down for `source`.
    HideawayPick { source: ObjId, cards: Vec<ObjId> },
    /// Choose a counter for `token`: `Mode(0)` reach, `Mode(1)` vigilance, `Mode(2)` trample.
    KeywordCounterChoice { token: ObjId },
    /// Delina rolled 15 or more: `Yes` rolls again for `target`, `No` stops.
    DelinaAgain { target: ObjId },
}

/// Options for [`Game::new_commander_with`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommanderOptions {
    /// Each player's starting life (CR 903.7: 40).
    pub starting_life: i32,
    /// With `Game::commander_choice` on, these seats' commanders go to the command zone without
    /// asking.
    pub auto_command_zone: [bool; 2],
}

impl Default for CommanderOptions {
    fn default() -> Self {
        CommanderOptions {
            starting_life: COMMANDER_LIFE,
            auto_command_zone: [false; 2],
        }
    }
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
    /// Stack entries resolved this turn, for [`RESOLUTIONS_PER_TURN`].
    pub resolutions_this_turn: u32,
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
    /// Instants and sorceries each player has cast this turn.
    pub instants_sorceries_cast: [u32; 2],
    /// Spells each player has cast this turn (all kinds).
    pub spells_cast: [u32; 2],
    /// Permanent types (artifact, creature, enchantment, land, planeswalker) each player has
    /// played from their graveyard with Muldrotha this turn.
    pub graveyard_types_used: [[bool; 5]; 2],
    /// Each instant or sorcery cast this turn as it was put on the stack, for copying it.
    pub cast_snapshots: Vec<StackItem>,
    /// Cards that may be cast for free right after the ability that exiled them resolves, and who
    /// may cast them (CR 608.2g: spells cast during resolution go on the stack afterward).
    pub free_casts: Vec<ObjId>,
    pub free_caster: PlayerId,
    /// Set just before casting from the graveyard by removing counters (Quilled Greatwurm).
    removal_to_cast: u8,
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
    /// Pay with the smart auto-payer: floating mana first, then keep the colors the rest of the hand
    /// needs (Bolt service games). Off for research runs, so stored results replay identically.
    pub smart_pay: bool,
    /// Players who tap their own mana sources (TapForMana moves at priority). Casting still auto-pays
    /// whatever the pool doesn't cover.
    pub manual_mana: [bool; 2],
    /// Consecutive priority passes with no action between them (CR 117.4).
    passes: u8,
    rng: Rng,
    events: Vec<Event>,
    /// Commander (CR 903): the command zone, each player's commander, how many times each player
    /// has cast theirs from the command zone (the tax), and the combat damage each player has taken
    /// from the opponent's commander. Empty, `None` and zero in a Limited game.
    pub command: Vec<ObjId>,
    pub commanders: [Option<ObjId>; 2],
    pub commander_casts: [u8; 2],
    pub commander_damage: [i32; 2],
    /// The turn each player made their next spell this turn uncounterable (0: never).
    pub next_spell_uncounterable: [u32; 2],
    /// Whether a creature died under each player's control this turn.
    pub creature_died_under: [bool; 2],
    /// Whether each player has cast a creature spell, and a noncreature spell, this turn.
    pub cast_creature_noncreature: [[bool; 2]; 2],
    /// Delayed "whenever you attack this turn" triggers: (source, trigger number, controller, turn).
    pub delayed_on_attack: Vec<(ObjId, usize, PlayerId, u32)>,
    /// "When it dies under your control this turn": (creature, its zcc, source, trigger, controller,
    /// turn).
    pub dies_under_control: Vec<(ObjId, u32, ObjId, usize, PlayerId, u32)>,
    /// Granted "whenever this creature deals combat damage to a player" until end of turn:
    /// (creature, its zcc, source, trigger, turn).
    pub granted_combat_damage: Vec<(ObjId, u32, ObjId, usize, u32)>,
    /// Keywords granted "until your next turn": (creature, its zcc, keywords, whose next turn).
    pub until_your_next_turn: Vec<(ObjId, u32, Keywords, PlayerId)>,
    /// Delayed "when you next attack this turn" triggers, removed as they trigger.
    pub delayed_on_next_attack: Vec<(ObjId, usize, PlayerId, u32)>,
    /// Additional combat phases still to come this turn, each with the step to go to after the
    /// additional main phase that follows it (CR 500.8).
    pub extra_combats: Vec<Step>,
    /// The step after the additional main phase of the combat in progress, and of the main phase
    /// in progress.
    pub extra_combat_resume: Option<Step>,
    pub extra_main_resume: Option<Step>,
    /// Delayed "copy the next spell you cast this turn" triggers: (source, trigger, controller, turn).
    pub copy_next_spell: Vec<(ObjId, usize, PlayerId, u32)>,
    /// Spell copies waiting to go on the stack once the ability that made them finishes resolving
    /// (their controller chooses new targets then).
    pub pending_copies: Vec<StackItem>,
    /// Delayed "this turn" triggers: (event, source, trigger number, controller, turn).
    pub delayed_this_turn: Vec<(When, ObjId, usize, PlayerId, u32)>,
    /// Free casts not taken go to their owner's hand instead of staying in exile.
    pub free_casts_to_hand: bool,
    /// "The next time a source of your choice would deal damage to you this turn": (protected player,
    /// source, the card, its trigger number, turn).
    pub damage_shields: Vec<(PlayerId, ObjId, ObjId, usize, u32)>,
    /// Whether a card left each player's graveyard this turn.
    pub graveyard_left_this_turn: [bool; 2],
    /// Cards left each player's graveyard during their turn since triggers were last put on the stack.
    pub graveyard_leave_pending: [bool; 2],
    /// Commander: when on, a commander put into a graveyard or exile stays there until its owner
    /// chooses, as a state-based action, whether to move it to the command zone (CR 903.9a), so
    /// "dies" triggers see it die. Off (the default): it goes straight to the command zone.
    pub commander_choice: bool,
    /// Who gets priority once a pending commander choice is made.
    commander_choice_then: PlayerId,
    /// Who gets priority after a shock land's choice.
    shock_then: PlayerId,
    /// Each seat's second commander, if it has partners (CR 702.124).
    pub partners: [Option<ObjId>; 2],
    /// Combat damage dealt to a player by one commander object (CR 903.10a): (commander, player,
    /// damage). A mirror match's two commanders are two objects, so two entries.
    pub commander_damage_from: Vec<(ObjId, PlayerId, i32)>,
    /// With `commander_choice` on, this seat's commanders go to the command zone without asking.
    pub commander_auto_return: [bool; 2],
    /// Additional land plays each player has this turn (Explore).
    pub extra_lands_this_turn: [u8; 2],
    /// "Becomes a copy ... until end of turn": (object, its zcc, the card it was before).
    pub copy_until_eot: Vec<(ObjId, u32, usize)>,
    /// The turn each player made their creature spells uncounterable for the turn (Domri).
    pub creatures_uncounterable_turn: [u32; 2],
    /// The turn each player's creatures must attack if able (Bident of Thassa).
    pub must_attack_turn: [u32; 2],
    /// The turn each player last created a token.
    pub token_created_turn: [u32; 2],
    /// The city's blessing (ascend), for the rest of the game.
    pub city_blessing: [bool; 2],
    /// Mana each player spent on spells this turn: (turn, amount) (expend).
    pub mana_spent_on_spells: [(u32, u32); 2],
    /// "Prevent all combat damage that would be dealt this turn": the turn it applies to (Spore Frog).
    pub combat_damage_prevented_turn: Option<u32>,
    /// "Prevent all damage that would be dealt to you this turn", per player (Riot Control).
    pub damage_to_player_prevented_turn: [Option<u32>; 2],
    /// Creatures that attacked this turn, anyone's (Rowdy Research).
    pub attacked_creatures_this_turn: Vec<ObjId>,
    /// An opponent accepted a tempting offer that is resolving.
    pub tempt_accepted: bool,
    /// Free casts not taken go to the bottom of the library in a random order (cascade).
    pub free_casts_to_bottom: bool,
    /// Additional combat phases to come right after the current one (CR 500.8).
    pub combats_after_this_phase: u8,
    /// Combat phases begun this turn.
    pub combats_this_turn: u8,
    /// Creatures to return to their owners' hands at the beginning of the next end step (Ilharg).
    pub return_to_hand_at_end: Vec<(ObjId, u32)>,
    /// Creatures with infect until end of turn: (object, its zcc, turn) (Triumph of the Hordes).
    pub infect_until_eot: Vec<(ObjId, u32, u32)>,
    /// Cards left each player's graveyard since triggers were last put on the stack (any turn).
    pub graveyard_leave_any_pending: [bool; 2],
    /// Land cards went to each player's graveyard / creature cards were milled, since triggers were
    /// last put on the stack.
    pub land_to_graveyard_pending: [bool; 2],
    pub creatures_milled_pending: [bool; 2],
    /// Creatures that must be blocked each combat this turn if able: (object, zcc, turn).
    pub must_be_blocked: Vec<(ObjId, u32, u32)>,
    /// Entries of `extra_combats` (by index) that are a combat with no main phase after it and that
    /// untap the active player's creatures as they begin (Moraug).
    pub extra_combats_untap_only: Vec<usize>,
    /// The current additional combat is one of those.
    pub combat_untap_only: bool,
    /// Additional combats (each followed by a main phase) to come after this turn's second main
    /// phase (World at War).
    pub combats_after_main2: u8,
    /// Untap the creatures that attacked this turn as the next combat begins (World at War).
    pub untap_attackers_at_combat: bool,
    /// Rebound cards in exile, castable free at their owner's next upkeep: (card, its zcc).
    pub rebound: Vec<(ObjId, u32)>,
    /// How many waiting triggers (from the front of `triggered`) Roaming Throne has already looked at.
    pub triggered_scanned: usize,
    /// Creatures that have "{1}: This creature fights target creature you don't control." until end
    /// of turn: (object, zcc, turn) (The Grand Evolution). Activated as ability number
    /// `activated.len()` of the creature (see `Game::activated_of`).
    pub granted_fight: Vec<(ObjId, u32, u32)>,
    /// Land cards went from a library to each player's graveyard since triggers were last put on the
    /// stack (Hedge Shredder).
    pub lands_milled_pending: [bool; 2],
    /// "Whenever this creature becomes tapped" granted until end of turn: (creature, zcc, card with
    /// the trigger, its index, turn) (Legolas's Quick Reflexes).
    pub granted_tapped: Vec<(ObjId, u32, ObjId, usize, u32)>,
    /// The turn each player can't cast additional spells (Conduit of Worlds); 0: never.
    pub no_more_spells_turn: [u32; 2],
    /// "For as long as you control [source]" control changes: (permanent, its zcc, source, source's
    /// zcc, new controller, controller before) (Roil Elemental).
    pub control_while: Vec<(ObjId, u32, ObjId, u32, PlayerId, PlayerId)>,
    /// Draws at the beginning of the next turn's upkeep: (player, cards) (Arcane Denial).
    pub next_upkeep_draws: Vec<(PlayerId, u8)>,
    /// Snow mana ({S}) spent so far on the spell being paid for (Blessing of Frost).
    pub snow_paid: u32,
    /// Creatures whose combat damage dealt is prevented this turn (Maze of Ith), with their zcc.
    pub combat_damage_by_prevented: Vec<(ObjId, u32)>,
    /// Each player's starting life total (Myrkul's "half your starting life total").
    pub starting_life: i32,
    /// Noncreature spells each player has cast this turn: (turn, count) (Esper Sentinel).
    pub noncreature_casts: [(u32, u8); 2],
    /// Damage dealt to each player this turn: (turn, total) (Spinerock Knoll).
    pub damage_to_player_turn: [(u32, i32); 2],
    /// "When you next cast a creature spell this turn": (source, trigger, controller, turn) (Vivien).
    pub next_creature_spell: Vec<(ObjId, usize, PlayerId, u32)>,
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

    /// A 1v1 Commander game (CR 903): 40 life, each player's commander in the command zone and
    /// the rest of the deck (99 cards) shuffled as the library. Deck legality (singleton, color
    /// identity, 100 cards) is the caller's job.
    pub fn new_commander(
        defs: Arc<Vec<CardDef>>,
        decks: [&[usize]; 2],
        commanders: [usize; 2],
        seed: u64,
        first: PlayerId,
    ) -> Game {
        Game::new_commander_with(
            defs,
            decks,
            [&[commanders[0]], &[commanders[1]]],
            CommanderOptions::default(),
            seed,
            first,
        )
    }

    /// A 1v1 Commander game with options: one or two commanders per seat (partners, each with its
    /// own tax), the starting life, and whether each seat's commanders always go to the command zone
    /// when `commander_choice` is on.
    pub fn new_commander_with(
        defs: Arc<Vec<CardDef>>,
        decks: [&[usize]; 2],
        commanders: [&[usize]; 2],
        options: CommanderOptions,
        seed: u64,
        first: PlayerId,
    ) -> Game {
        assert!(
            commanders.iter().all(|c| (1..=2).contains(&c.len())),
            "one or two commanders a seat"
        );
        let mut g = Game::empty(defs, seed, first);
        g.commander_auto_return = options.auto_command_zone;
        for (p, deck) in decks.iter().enumerate() {
            g.players[p].life = options.starting_life;
            g.starting_life = options.starting_life;
            let c = g.add_card(p, commanders[p][0], Zone::Command);
            g.commanders[p] = Some(c);
            if let Some(&second) = commanders[p].get(1) {
                let c2 = g.add_card(p, second, Zone::Command);
                g.partners[p] = Some(c2);
            }
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

    /// Moves a card to another zone as an effect would (replacements apply; for tests and tools).
    pub fn move_card(&mut self, id: ObjId, to: Zone) {
        self.move_to(id, to);
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
            resolutions_this_turn: 0,
            creature_died_this_turn: false,
            life_lost_this_turn: [false; 2],
            in_combat_damage: false,
            delayed_returns: Vec::new(),
            end_turn: false,
            last_created: Vec::new(),
            control_until_eot: Vec::new(),
            work: 0,
            instants_sorceries_cast: [0; 2],
            spells_cast: [0; 2],
            graveyard_types_used: [[false; 5]; 2],
            cast_snapshots: Vec::new(),
            free_casts: Vec::new(),
            free_caster: 0,
            removal_to_cast: 0,
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
            smart_pay: false,
            manual_mana: [false; 2],
            passes: 0,
            rng: Rng::new(seed),
            events: Vec::new(),
            command: Vec::new(),
            commanders: [None, None],
            commander_casts: [0, 0],
            commander_damage: [0, 0],
            next_spell_uncounterable: [0, 0],
            creature_died_under: [false; 2],
            cast_creature_noncreature: [[false; 2]; 2],
            delayed_on_attack: Vec::new(),
            dies_under_control: Vec::new(),
            granted_combat_damage: Vec::new(),
            until_your_next_turn: Vec::new(),
            delayed_on_next_attack: Vec::new(),
            copy_until_eot: Vec::new(),
            creatures_uncounterable_turn: [0, 0],
            must_attack_turn: [0, 0],
            token_created_turn: [0, 0],
            city_blessing: [false, false],
            mana_spent_on_spells: [(0, 0); 2],
            extra_combats: Vec::new(),
            extra_combat_resume: None,
            extra_main_resume: None,
            copy_next_spell: Vec::new(),
            pending_copies: Vec::new(),
            delayed_this_turn: Vec::new(),
            free_casts_to_hand: false,
            damage_shields: Vec::new(),
            graveyard_left_this_turn: [false; 2],
            graveyard_leave_pending: [false; 2],
            graveyard_leave_any_pending: [false; 2],
            land_to_graveyard_pending: [false; 2],
            creatures_milled_pending: [false; 2],
            commander_choice: false,
            commander_choice_then: 0,
            shock_then: 0,
            partners: [None, None],
            commander_damage_from: Vec::new(),
            commander_auto_return: [false; 2],
            extra_lands_this_turn: [0; 2],
            combat_damage_prevented_turn: None,
            damage_to_player_prevented_turn: [None; 2],
            attacked_creatures_this_turn: Vec::new(),
            tempt_accepted: false,
            free_casts_to_bottom: false,
            combats_after_this_phase: 0,
            combats_this_turn: 0,
            return_to_hand_at_end: Vec::new(),
            infect_until_eot: Vec::new(),
            must_be_blocked: Vec::new(),
            extra_combats_untap_only: Vec::new(),
            combat_untap_only: false,
            combats_after_main2: 0,
            untap_attackers_at_combat: false,
            rebound: Vec::new(),
            triggered_scanned: 0,
            granted_fight: Vec::new(),
            lands_milled_pending: [false; 2],
            granted_tapped: Vec::new(),
            no_more_spells_turn: [0; 2],
            control_while: Vec::new(),
            next_upkeep_draws: Vec::new(),
            snow_paid: 0,
            combat_damage_by_prevented: Vec::new(),
            starting_life: STARTING_LIFE,
            noncreature_casts: [(0, 0); 2],
            damage_to_player_turn: [(0, 0); 2],
            next_creature_spell: Vec::new(),
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
            from_command: false,
            commander_tax: 0,
            exile_if_dies: None,
            cast_free: false,
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
            animated: None,
            stage: 0,
            fellowship: 0,
            playable: None,
            bait: 0,
            stash: false,
            filter_used_turn: 0,
            keyword_counters: Keywords::NONE,
            lore: 0,
            copy_token: false,
            sacrifice_at_end: false,
            graveyard_turn: 0,
            loyalty: 0,
            loyalty_turn: 0,
            cast_uncounterable: false,
            was_cast: false,
            dealt_damage: false,
            defender_ignored_turn: 0,
            chosen_mode: None,
            once_each_turn: 0,
            counters_put_turn: 0,
            last_counters: (0, 0, Keywords::NONE),
            linked_card: None,
            finality: false,
            enters_with_finality: false,
            eot_creature: None,
            harmonize_eot: false,
            commander_choice_pending: false,
            resolved_this_turn: (0, 0),
            command_zone_casts: 0,
            eot_subtype: None,
            card_copy: false,
            offspring_paid: false,
            copy_base_pt: None,
            decayed: false,
            sacrifice_end_combat: false,
            delve: 0,
            printed_def: None,
            time_counters: 0,
            suspend_haste: false,
            skip_untap: false,
            quest: 0,
            activated_turn: None,
            kind_counters: 0,
            not_legendary: false,
            milled_turn: 0,
            activations_stopped_turn: 0,
            shock_pending: false,
            exerted_turn: None,
            regen_shields: (0, 0),
            eternalized: false,
            attacks_turn: (0, 0),
            transformed: false,
            copy_exceptions: false,
            face_down: false,
            conduit_turn: 0,
            paired_with: None,
            unearthed: false,
            exile_at_end: false,
            command_at_end: false,
            copy_enchantment_only: false,
            hideaway: None,
            exile_end_combat: false,
            lost_soulbond: false,
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
        let o = self.obj(id);
        if o.face_down {
            return face_down_def();
        }
        let d = &self.defs[o.def];
        match &d.back {
            Some(back) if o.transformed => back,
            _ => d,
        }
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

    /// Its card types now (CR 205.1, layer 4): printed, except on the battlefield, where it may
    /// have become a creature (Soulstone Sanctuary) or only a land (Imprisoned in the Moon).
    pub fn types_of(&self, id: ObjId) -> Types {
        let o = self.obj(id);
        let mut t = self.def_of(id).types;
        if o.copy_enchantment_only {
            // Myrkul's tokens: an enchantment, and no other card types.
            t = Types {
                enchantment: true,
                legendary: t.legendary,
                snow: t.snow,
                ..Types::default()
            };
        }
        if let Some(p) = self.copy_printed(id) {
            t.artifact |= p.copy_artifact; // "except it's an artifact in addition"
            t.legendary |= p.copy_keeps_name; // "it's legendary in addition"
        }
        if o.not_legendary {
            t.legendary = false;
        }
        if let Some((colors, n)) = self.def_of(id).creature_unless_devotion {
            if o.zone == Zone::Battlefield && self.devotion(o.controller, colors) < n as u32 {
                t.creature = false; // the Theros gods
            }
        }
        // The common case: nothing on the battlefield changes types.
        if self.bello_animates(id) {
            t.creature = true;
        }
        if o.zone != Zone::Battlefield
            || (o.animated.is_none() && o.eot_creature.is_none() && self.ability_loss_sources == 0)
        {
            return t;
        }
        if o.animated.is_some() || self.eot_creature(id).is_some() {
            t.creature = true;
        }
        if self.attached_static(id).is_some_and(|s| s.becomes_land) {
            t = Types {
                land: true,
                legendary: t.legendary,
                basic: t.basic,
                ..Types::default()
            };
        }
        t
    }

    /// The clone's own card, while it's a copy with that card's exceptions (`Object::copy_exceptions`).
    pub fn copy_printed(&self, id: ObjId) -> Option<&CardDef> {
        let o = self.obj(id);
        if !o.copy_exceptions {
            return None;
        }
        o.printed_def.map(|d| &self.defs[d])
    }

    /// Its name now: its card's, or a copy's that keeps its own name (Sakashima the Impostor).
    pub fn name_of(&self, id: ObjId) -> &str {
        match self.copy_printed(id) {
            Some(p) if p.copy_keeps_name => &p.name,
            _ => &self.def_of(id).name,
        }
    }

    /// Whether `p` controls a permanent that turns the legend rule off for them.
    fn legend_rule_off(&self, p: PlayerId) -> bool {
        self.permanents(p).any(|x| {
            self.def_of(x).no_legend_rule
                || self
                    .copy_printed(x)
                    .is_some_and(|d| d.copy_keeps_abilities && d.no_legend_rule)
        })
    }

    /// Activated abilities its own card lends a copy that keeps them (`Activated::copy_only`).
    fn kept_activated<'a>(&'a self, id: ObjId) -> impl Iterator<Item = &'a Activated> + 'a {
        self.copy_printed(id)
            .filter(|d| d.copy_keeps_abilities || d.copy_keeps_name)
            .into_iter()
            .flat_map(|d| d.activated.iter().filter(|a| a.copy_only))
    }

    /// Its colors now (CR 105, layer 5).
    pub fn colors_of(&self, id: ObjId) -> [bool; 5] {
        if self.obj(id).eternalized {
            return [false, false, true, false, false]; // a black Zombie
        }
        let printed = self.def_of(id).colors;
        if self.obj(id).zone != Zone::Battlefield {
            return printed;
        }
        self.attached_static(id)
            .and_then(|s| s.set_colors)
            .unwrap_or(printed)
    }

    /// The ability-removing static of an Aura attached to `id`, if any (Witness Protection,
    /// Imprisoned in the Moon). Cheap when no such Aura is on the battlefield.
    fn attached_static(&self, id: ObjId) -> Option<Static> {
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
                .copied()
                .find(|s| s.lose_abilities)
        })
    }

    /// Mana sources `p` could tap now, one entry per mana. Public for tests and agents.
    pub fn mana_sources(&self, p: PlayerId) -> Vec<(ObjId, ManaSource)> {
        self.untapped_sources(p)
    }

    /// An Aura that only takes abilities away (Fresh Start): no base power and toughness, types or
    /// colors of its own, so the creature keeps those.
    fn loses_abilities_only(&self, id: ObjId) -> bool {
        self.attached_static(id)
            .is_some_and(|s| s.base_pt.is_none() && !s.becomes_land && s.set_colors.is_none())
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
        let o = self.obj(id);
        // Roaming Throne: "This creature is the chosen type in addition to its other types."
        if o.zone == Zone::Battlefield
            && self.def_of(id).doubles_chosen_type_triggers
            && o.chosen_type.as_deref() == Some(s)
        {
            return true;
        }
        if o.zone == Zone::Battlefield && o.eot_subtype.is_some_and(|(t, sub)| t == self.turn && sub == s) {
            return true;
        }
        if self.eot_creature(id).is_some_and(|e| e.3 == s) {
            return true;
        }
        if s == "Elemental" && self.bello_animates(id) {
            return true;
        }
        if self.lost_abilities(id) && !self.loses_abilities_only(id) {
            return s == "Citizen"; // Witness Protection
        }
        let o = self.obj(id);
        if let Some(only) = o.subtype_override {
            return only == s;
        }
        if o.animated.is_some() && o.zone == Zone::Battlefield && !LAND_TYPES.contains(&s) {
            return true; // "all creature types"
        }
        if o.stage > 0 {
            let st = &self.def_of(id).stages[o.stage as usize - 1];
            return st.subtypes.contains(&s) || o.extra_subtypes.contains(&s);
        }
        self.def_of(id).subtypes.iter().any(|x| x == s) || o.extra_subtypes.contains(&s)
    }

    /// Whether `id` has creature type `s` (CR 205.3).
    pub fn has_subtype(&self, id: ObjId, s: &str) -> bool {
        self.has_base_subtype(id, s)
            || self.statics_on(id).any(|st| st.subtype == Some(s))
            || (matches!(s, "Plains" | "Island" | "Swamp" | "Mountain" | "Forest")
                && self.every_basic_type(id))
    }

    /// Dryad of the Ilysian Grove: a land on the battlefield whose controller has it.
    fn every_basic_type(&self, id: ObjId) -> bool {
        let o = self.obj(id);
        o.zone == Zone::Battlefield
            && self.types_of(id).land
            && self
                .permanents(o.controller)
                .any(|x| self.def_of(x).lands_every_basic_type && !self.lost_abilities(x))
    }

    /// Static abilities on the battlefield that apply to `id` right now (CR 604.2, 611.3).
    fn statics_on(&self, id: ObjId) -> impl Iterator<Item = Static> + '_ {
        let o = self.obj(id);
        let on_battlefield = o.zone == Zone::Battlefield;
        let creature = self.types_of(id).creature;
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
                            Scope::YourAttackingTokens => {
                                mine && creature
                                    && self.is_token(id)
                                    && self.combat.as_ref().is_some_and(|c| c.attackers.contains(&id))
                            }
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
                                    && so.chosen_color.is_some_and(|c| self.colors_of(id)[c as usize])
                            }
                            Scope::OtherYourSubtype(sub) => {
                                mine && creature && src != id && self.has_base_subtype(id, sub)
                            }
                            Scope::YourUntappedSubtype(sub) => {
                                mine && creature && !o.tapped && self.has_base_subtype(id, sub)
                            }
                            Scope::YourAttackingCreatures => {
                                mine && creature
                                    && self.combat.as_ref().is_some_and(|c| c.attackers.contains(&id))
                            }
                            Scope::YourArtifactCreatures => mine && creature && self.types_of(id).artifact,
                            Scope::OtherYourCreatures => mine && creature && src != id,
                            Scope::EachLand => self.types_of(id).land,
                            Scope::YourLandCreatures => mine && creature && self.types_of(id).land,
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
                            Some(StaticCondition::ControlCreatureWithCounter) => {
                                self.controls_creature_with_counter(so.controller)
                            }
                            Some(StaticCondition::HasntDealtDamage) => !so.dealt_damage,
                            Some(StaticCondition::ChosenMode(m)) => so.chosen_mode == Some(m),
                            Some(StaticCondition::CounterKindsAmongYoursAtLeast(n)) => {
                                self.counter_kinds(so.controller) >= n as u32
                            }
                            Some(StaticCondition::QuestCountersAtLeast(n)) => so.quest >= n,
                            Some(StaticCondition::CitysBlessing) => self.city_blessing[so.controller],
                            Some(StaticCondition::ControlLandsAtLeast(n)) => {
                                self.battlefield
                                    .iter()
                                    .filter(|&&x| {
                                        self.obj(x).controller == so.controller && self.def_of(x).types.land
                                    })
                                    .count()
                                    >= n as usize
                            }
                        };
                        applies && condition_ok && (!s.your_turn_only || self.active == so.controller)
                    })
                    .map(move |mut s| {
                        if let Some(Scale::PerYourCreatureBasePowerOrToughnessOne) = s.scale {
                            let n = self
                                .creatures(so.controller)
                                .filter(|&c| {
                                    let (bp, bt) = self.base_pt(c);
                                    bp == 1 || bt == 1
                                })
                                .count() as i32;
                            s.power *= n;
                            s.toughness *= n;
                        }
                        if let Some(Scale::PerOtherYourCreatureBasePowerOne) = s.scale {
                            let n = self
                                .creatures(so.controller)
                                .filter(|&c| c != src && self.base_pt(c).0 == 1)
                                .count() as i32;
                            s.power *= n;
                            s.toughness *= n;
                        }
                        if let Some(Scale::PerKindCounter) = s.scale {
                            let n = so.kind_counters;
                            s.power *= n;
                            s.toughness *= n;
                        }
                        if let Some(Scale::PerColorAmongYourPermanents) = s.scale {
                            let n = (0..5)
                                .filter(|&c| self.permanents(so.controller).any(|x| self.colors_of(x)[c]))
                                .count() as i32;
                            s.power *= n;
                            s.toughness *= n;
                        }
                        if let Some(Scale::PerCreatureYouControl) = s.scale {
                            let n = self.creatures(so.controller).count() as i32;
                            s.power *= n;
                            s.toughness *= n;
                        }
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
        let k = self.keywords_inner(id);
        let o = self.obj(id);
        if o.suspend_haste && o.zone == Zone::Battlefield {
            return k.with(Keywords::HASTE);
        }
        k
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
        if let Some(sub) = self.def_of(id).protection_from_planeswalkers_and {
            if self.obj(id).zone == Zone::Battlefield
                && !self.lost_abilities(id)
                && (self.types_of(source).planeswalker || self.has_subtype(source, sub))
            {
                return true;
            }
        }
        if let Some(sub) = self.def_of(id).protection_from_subtype {
            if self.obj(id).zone == Zone::Battlefield
                && !self.lost_abilities(id)
                && self.has_subtype(source, sub)
            {
                return true; // Yawgmoth: protection from Humans
            }
        }
        let p = self.protection(id);
        self.colors_of(source).iter().zip(p).any(|(c, pr)| *c && pr)
    }

    fn keywords_inner(&self, id: ObjId) -> Keywords {
        let printed = if self.lost_abilities(id) {
            Keywords::NONE
        } else {
            self.def_of(id)
                .keywords
                .with(self.obj(id).granted)
                .with(self.obj(id).keyword_counters)
        };
        let mut k = self.modifiers(id).fold(printed, |k, m| k.with(m.keywords));
        if !self.until_your_next_turn.is_empty() {
            let zcc = self.obj(id).zcc;
            for e in &self.until_your_next_turn {
                if e.0 == id && e.1 == zcc {
                    k = k.with(e.2);
                }
            }
        }
        if self.obj(id).animated.is_some() {
            k = k.with(Keywords::VIGILANCE);
        }
        if let Some((_, _, _, _, kw)) = self.eot_creature(id) {
            k = k.with(kw);
        }
        if self.bello_animates(id) {
            k = k.with(Keywords::INDESTRUCTIBLE).with(Keywords::HASTE);
        }
        if !self.lost_abilities(id) {
            for s in &self.def_of(id).stages[..self.obj(id).stage as usize] {
                k = k.with(s.keywords);
            }
        }
        if self.has_emblem(id, EmblemKind::ElspethAnthem) {
            k = k.with(Keywords::FLYING);
        }
        let o = self.obj(id);
        if o.zone == Zone::Battlefield
            && self.def_of(id).indestructible_at_half_life
            && !self.lost_abilities(id)
            && self.players[o.controller].life * 2 <= self.starting_life
        {
            k = k.with(Keywords::INDESTRUCTIBLE); // Myrkul
        }
        if o.zone == Zone::Battlefield
            && self.types_of(id).land
            && self
                .emblems
                .iter()
                .any(|&(q, k, _)| q == o.controller && k == EmblemKind::LandsIndestructible)
        {
            k = k.with(Keywords::INDESTRUCTIBLE);
        }
        if self.has_emblem(id, EmblemKind::GarrukAnthem) {
            k = k.with(Keywords::TRAMPLE);
        }
        if self.has_emblem(id, EmblemKind::VivienAnthem) {
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
            && self.types_of(id).creature
            && self.emblems.iter().any(|&(q, k, _)| {
                q == o.controller && matches!(k, EmblemKind::VivienAnthem | EmblemKind::ElspethAnthem)
            })
    }

    /// Devotion to these colors (CR 700.5): the mana symbols of those colors in the mana costs of
    /// permanents `p` controls (a hybrid symbol counts once).
    pub fn devotion(&self, p: PlayerId, colors: [bool; 5]) -> u32 {
        self.permanents(p)
            .filter(|&x| !self.obj(x).eternalized) // no mana cost
            .map(|x| {
                let c = self.def_of(x).cost;
                let mono: u32 = (0..5)
                    .filter(|&i| colors[i])
                    .map(|i| c.colored[i] as u32 + c.twobrid[i] as u32)
                    .sum();
                let hybrid: u32 = mana::PAIRS
                    .iter()
                    .enumerate()
                    .filter(|(_, &(a, b))| colors[a] || colors[b])
                    .map(|(k, _)| c.hybrid[k] as u32)
                    .sum();
                mono + hybrid
            })
            .sum()
    }

    /// Garruk, Cursed Huntsman's emblem: +3/+3 for each one its controller has.
    fn garruk_emblems(&self, id: ObjId) -> i32 {
        let o = self.obj(id);
        if o.zone != Zone::Battlefield || !self.types_of(id).creature {
            return 0;
        }
        3 * self
            .emblems
            .iter()
            .filter(|&&(q, k, _)| q == o.controller && k == EmblemKind::GarrukAnthem)
            .count() as i32
    }

    fn has_emblem(&self, id: ObjId, kind: EmblemKind) -> bool {
        let o = self.obj(id);
        o.zone == Zone::Battlefield
            && self.types_of(id).creature
            && self
                .emblems
                .iter()
                .any(|&(q, k, _)| q == o.controller && k == kind)
    }

    /// A token copy of `of` for `p` with these flags, made after the original's printed card.
    fn token_copy_of(&mut self, p: PlayerId, of: ObjId) -> Option<ObjId> {
        if self.obj(of).zone != Zone::Battlefield {
            return None;
        }
        let def = self.obj(of).def;
        let id = self.add_card(p, def, Zone::Hand);
        self.obj_mut(id).copy_token = true;
        self.move_to(id, Zone::Battlefield);
        Some(id)
    }

    /// Delina: roll a d20; a tapped and attacking nonlegendary copy of `target`, exiled at end of
    /// combat; on 15 or more, you may roll again.
    fn delina_roll(&mut self, p: PlayerId, target: ObjId) {
        let roll = self.rng.below(20) + 1;
        for _ in 0..self.token_doubling(p) {
            if let Some(id) = self.token_copy_of(p, target) {
                let o = self.obj_mut(id);
                o.tapped = true;
                o.not_legendary = true;
                o.exile_end_combat = true;
                if let Some(c) = self.combat.as_mut() {
                    c.attackers.push(id);
                }
            }
        }
        if roll >= 15 && self.obj(target).zone == Zone::Battlefield {
            self.ask(p, Decision::DelinaAgain { target });
        }
    }

    /// Primal Surge: exile the top card; a permanent card may go onto the battlefield (and repeat).
    fn primal_surge(&mut self, p: PlayerId) {
        if let Some(&c) = self.players[p].library.last() {
            self.move_to(c, Zone::Exile);
            if self.def_of(c).types.is_permanent() {
                self.ask(p, Decision::PrimalSurge { card: c });
            }
        }
    }

    /// Hasty token copies of `of` that are exiled at the next end step.
    fn copies_exile_at_end(&mut self, item: &StackItem, legal: &[bool], slot: u8, n: u32) {
        for _ in 0..n {
            let copy = Effect::CopyToken {
                slot: Some(slot),
                haste_and_sacrifice: false,
                extra_subtype: None,
            };
            self.run_effect(item, legal, copy);
            for id in self.last_created.clone() {
                let o = self.obj_mut(id);
                o.granted = o.granted.with(Keywords::HASTE);
                o.exile_at_end = true;
            }
        }
    }

    /// `p` sacrifices `n` permanents of their choice: all of them if they have `n` or fewer.
    fn sacrifice_permanents(&mut self, p: PlayerId, n: u8) {
        let mine: Vec<ObjId> = self
            .permanents(p)
            .filter(|&x| !self.cant_be_sacrificed(x))
            .collect();
        if mine.len() <= n as usize {
            for x in mine {
                self.move_to(x, Zone::Graveyard);
            }
        } else {
            self.ask(p, Decision::SacrificePermanents { left: n });
        }
    }

    /// How many card types are among cards in `p`'s graveyard (delirium).
    fn graveyard_card_types(&self, p: PlayerId) -> usize {
        let mut kinds = [false; 7];
        for &c in &self.players[p].graveyard {
            let t = self.def_of(c).types;
            let flags = [
                t.land,
                t.creature,
                t.artifact,
                t.enchantment,
                t.instant,
                t.sorcery,
                t.planeswalker,
            ];
            for (k, f) in kinds.iter_mut().zip(flags) {
                *k |= f;
            }
        }
        kinds.iter().filter(|&&k| k).count()
    }

    /// Umbra armor: an Aura with it on `id` is destroyed instead, and `id` loses all damage.
    /// Returns whether one did.
    fn use_umbra_armor(&mut self, id: ObjId) -> bool {
        if self.obj(id).zone != Zone::Battlefield {
            return false;
        }
        let zcc = self.obj(id).zcc;
        let aura = self.battlefield.iter().copied().find(|&a| {
            self.obj(a).attached_to == Some((id, zcc))
                && self.def_of(a).umbra_armor
                && !self.lost_abilities(a)
        });
        let Some(aura) = aura else { return false };
        let o = self.obj_mut(id);
        o.damage = 0;
        o.deathtouched = false;
        self.destroy(aura);
        true
    }

    /// How many legendary creatures `p` controls (Otawara's discount).
    fn legendary_creatures(&self, p: PlayerId) -> u8 {
        self.creatures(p)
            .filter(|&c| self.types_of(c).legendary)
            .count()
            .min(255) as u8
    }

    /// One green mana in a pool (Omnath).
    fn is_green_mana(m: &ManaSource) -> bool {
        m.colors == [false, false, false, false, true] && !m.colorless
    }

    fn opponents_graveyard_bonus(&self, id: ObjId) -> i32 {
        let d = self.def_of(id);
        let mut b = if self.emblem_anthem(id) { 2 } else { 0 };
        if d.keeps_green_mana {
            let p = self.obj(id).controller;
            b += self.players[p]
                .pool
                .iter()
                .filter(|m| Game::is_green_mana(m))
                .count() as i32;
        }
        b += self.garruk_emblems(id);
        if d.pt_opponents_graveyards {
            b += self.players[opponent(self.obj(id).controller)].graveyard.len() as i32;
        }
        if d.pt_lands {
            let p = self.obj(id).controller;
            b += self.permanents(p).filter(|&x| self.types_of(x).land).count() as i32;
        }
        if d.pt_hand_size {
            b += self.players[self.obj(id).controller].hand.len() as i32;
        }
        if d.pt_half_highest_opponent_life {
            let life = self.players[opponent(self.obj(id).controller)].life.max(0);
            b += (life + 1) / 2;
        }
        if d.pt_per_graveyard_creature {
            let p = self.obj(id).controller;
            b += self.players[p]
                .graveyard
                .iter()
                .filter(|&&c| self.types_of(c).creature)
                .count() as i32;
        }
        b
    }

    /// Base power and toughness: printed, or set by becoming a creature or a stage.
    fn base_pt(&self, id: ObjId) -> (i32, i32) {
        let (p, t) = self.base_pt_set(id);
        // Maha: "Creatures your opponents control have base toughness 1."
        let o = self.obj(id);
        if o.zone == Zone::Battlefield
            && self
                .permanents(opponent(o.controller))
                .any(|x| self.def_of(x).opponents_creatures_base_toughness_one && !self.lost_abilities(x))
        {
            return (p, 1);
        }
        (p, t)
    }

    fn base_pt_set(&self, id: ObjId) -> (i32, i32) {
        let o = self.obj(id);
        let d = self.def_of(id);
        if let Some((_, p, t, _, _)) = self.eot_creature(id) {
            return (p, t);
        }
        if self.bello_animates(id) {
            return (4, 4);
        }
        if let Some(pt) = o.animated {
            return pt;
        }
        // "During your turn, this creature has base power and toughness 5/2." (Snowmelt Stag)
        if let Some(pt) = self
            .def_of(id)
            .statics
            .iter()
            .find(|s| s.scope == Scope::This && !s.lose_abilities && s.base_pt.is_some())
            .filter(|s| !s.your_turn_only || self.active == o.controller)
            .filter(|_| o.zone == Zone::Battlefield && !self.lost_abilities(id))
            .and_then(|s| s.base_pt)
        {
            return pt;
        }
        (1..=o.stage as usize)
            .rev()
            .find_map(|s| d.stages[s - 1].base_pt)
            .unwrap_or(o.copy_base_pt.unwrap_or((d.power, d.toughness)))
    }

    /// Moraug: +1/+0 for each time a creature you control has attacked this turn.
    fn attack_count_bonus(&self, id: ObjId) -> i32 {
        let o = self.obj(id);
        if o.zone != Zone::Battlefield || o.attacks_turn.0 != self.turn || o.attacks_turn.1 == 0 {
            return 0;
        }
        let moraugs = self
            .permanents(o.controller)
            .filter(|&x| self.def_of(x).power_per_attack_this_turn && !self.lost_abilities(x))
            .count() as i32;
        moraugs * o.attacks_turn.1 as i32
    }

    pub fn power(&self, id: ObjId) -> i32 {
        self.attack_count_bonus(id)
            + self
                .ability_loss(id)
                .filter(|_| !self.loses_abilities_only(id))
                .map_or(self.base_pt(id).0, |pt| pt.0)
            + self.opponents_graveyard_bonus(id)
            + self.obj(id).counters
            + self.modifiers(id).map(|m| m.power).sum::<i32>()
            + self.statics_on(id).map(|s| s.power).sum::<i32>()
    }

    pub fn toughness(&self, id: ObjId) -> i32 {
        self.ability_loss(id)
            .filter(|_| !self.loses_abilities_only(id))
            .map_or(self.base_pt(id).1, |pt| pt.1)
            + self.opponents_graveyard_bonus(id)
            + self.obj(id).counters
            + self.modifiers(id).map(|m| m.toughness).sum::<i32>()
            + self.statics_on(id).map(|s| s.toughness).sum::<i32>()
    }

    /// Taps a permanent; if it was untapped, its "whenever this becomes tapped" triggers fire.
    fn become_tapped(&mut self, id: ObjId) {
        if self.obj(id).tapped {
            return;
        }
        self.obj_mut(id).tapped = true;
        if self.obj(id).zone == Zone::Battlefield {
            self.fire_source(id, When::BecomesTapped);
            let (zcc, turn) = (self.obj(id).zcc, self.turn);
            for (c, z, src, n, t) in self.granted_tapped.clone() {
                if c == id && z == zcc && t == turn {
                    let controller = self.obj(id).controller;
                    self.triggered.push(StackItem {
                        kind: StackKind::Trigger(n),
                        extra: Some(id),
                        ..Game::casting(src, controller)
                    });
                }
            }
        }
    }

    /// An effect untaps a permanent: a tapped one with a stun counter loses the counter instead
    /// (CR 122.1d).
    fn untap_by_effect(&mut self, id: ObjId) {
        let o = self.obj_mut(id);
        if o.tapped && o.stun > 0 {
            o.stun -= 1;
        } else {
            o.tapped = false;
        }
    }

    /// An Aura on it says "Enchanted creature doesn't untap during its controller's untap step."
    fn enchanted_doesnt_untap(&self, id: ObjId) -> bool {
        let zcc = self.obj(id).zcc;
        self.battlefield.iter().any(|&a| {
            self.obj(a).attached_to == Some((id, zcc))
                && self.def_of(a).enchanted_doesnt_untap
                && !self.lost_abilities(a)
        })
    }

    /// Total toughness of the creatures `p` controls.
    fn total_toughness(&self, p: PlayerId) -> i32 {
        self.creatures(p).map(|c| self.toughness(c)).sum()
    }

    /// Abilities that Auras grant the creatures `p` controls ("Enchanted creature has ..."), as
    /// activations of the Aura's ability.
    fn granted_activations(&self, p: PlayerId) -> Vec<Action> {
        let mut v = Vec::new();
        for &aura in &self.battlefield {
            let Some((c, zcc)) = self.obj(aura).attached_to else {
                continue;
            };
            if self.obj(c).zone != Zone::Battlefield
                || self.obj(c).zcc != zcc
                || self.obj(c).controller != p
                || self.lost_abilities(aura)
                || self.lost_abilities(c)
                || self.activation_stopped(c)
            {
                continue;
            }
            for (n, a) in self.def_of(aura).activated.iter().enumerate() {
                if a.granted_to_attached
                    && (!a.tap_attached || self.can_tap(c))
                    && self.can_pay_for(p, &a.mana, None, PayFor::Ability(c))
                {
                    v.push(Action::Activate(aura, n));
                }
            }
        }
        v
    }

    /// Whether the permanent has a counter of any kind on it (CR 122).
    pub fn has_counter(&self, id: ObjId) -> bool {
        let o = self.obj(id);
        o.counters != 0
            || o.stun > 0
            || o.incubation > 0
            || o.soul > 0
            || o.revival > 0
            || o.fellowship > 0
            || o.bait > 0
            || o.loyalty > 0
            || o.keyword_counters != Keywords::NONE
            || o.finality
            || o.decayed
    }

    /// "Can't be countered": printed, or from an effect as it was cast.
    fn uncounterable(&self, id: ObjId) -> bool {
        self.def_of(id).uncounterable || self.obj(id).cast_uncounterable
    }

    /// Clarion Conqueror: activated abilities of this artifact, creature or planeswalker on the
    /// battlefield can't be activated (mana abilities included).
    fn activation_stopped(&self, id: ObjId) -> bool {
        if self.obj(id).zone != Zone::Battlefield {
            return false;
        }
        if self.turn > 0 && self.obj(id).activations_stopped_turn == self.turn {
            return true; // Koma
        }
        let t = self.types_of(id);
        if t.artifact
            && self
                .battlefield
                .iter()
                .any(|&x| self.def_of(x).stops_artifact_abilities && !self.lost_abilities(x))
        {
            return true; // Collector Ouphe
        }
        (t.artifact || t.creature || t.planeswalker)
            && self
                .battlefield
                .iter()
                .any(|&x| self.def_of(x).stops_activated_abilities && !self.lost_abilities(x))
    }

    /// "Whenever one or more cards leave your graveyard during your turn" triggers, once for the cards
    /// that left since triggers were last put on the stack.
    fn flush_graveyard_leaves(&mut self) {
        for p in 0..2 {
            if std::mem::take(&mut self.graveyard_leave_any_pending[p]) {
                self.fire(p, When::CardsLeaveYourGraveyard, None);
            }
            if std::mem::take(&mut self.land_to_graveyard_pending[p]) {
                self.fire(p, When::LandCardsToYourGraveyard, None);
            }
            if std::mem::take(&mut self.creatures_milled_pending[p]) {
                self.fire(p, When::CreatureCardsMilledToYourGraveyard, None);
            }
            if std::mem::take(&mut self.lands_milled_pending[p]) {
                self.fire(p, When::LandCardsMilledToYourGraveyard, None);
            }
        }
        for p in 0..2 {
            if !std::mem::take(&mut self.graveyard_leave_pending[p]) {
                continue;
            }
            self.fire(p, When::CardsLeaveYourGraveyardDuringYourTurn, None);
            let before = self.triggered.len();
            self.fire(p, When::CardLeavesYourGraveyardOnceEachTurn, None);
            let turn = self.turn;
            let mut fresh = Vec::new();
            for t in self.triggered.drain(before..).collect::<Vec<_>>() {
                if self.obj(t.source).once_each_turn != turn {
                    self.obj_mut(t.source).once_each_turn = turn;
                    fresh.push(t);
                }
            }
            self.triggered.extend(fresh);
        }
    }

    /// CR 903.9a: a commander put into a graveyard or exile since state-based actions were last
    /// checked: its owner chooses whether to move it to the command zone. Asks, and returns true,
    /// if there's such a choice to make before `then` gets priority.
    fn ask_commander_choice(&mut self, then: PlayerId) -> bool {
        let Some(c) = self
            .commanders
            .iter()
            .chain(&self.partners)
            .flatten()
            .copied()
            .find(|&c| {
                self.obj(c).commander_choice_pending
                    && matches!(self.obj(c).zone, Zone::Graveyard | Zone::Exile)
            })
        else {
            return false;
        };
        self.obj_mut(c).commander_choice_pending = false;
        let owner = self.obj(c).owner;
        if self.commander_auto_return[owner] {
            self.move_to(c, Zone::Command);
            return self.ask_commander_choice(then);
        }
        self.commander_choice_then = then;
        self.ask(owner, Decision::CommanderToCommandZone { card: c });
        true
    }

    /// Bello: the Bello that makes `id` a 4/4 Elemental creature right now, if any: `id` is a
    /// non-Equipment artifact or non-Aura enchantment with mana value 4 or greater on the
    /// battlefield, and its controller controls Bello during their own turn.
    fn bello_for(&self, id: ObjId) -> Option<ObjId> {
        let o = self.obj(id);
        if o.zone != Zone::Battlefield || self.active != o.controller {
            return None;
        }
        let d = self.def_of(id);
        let sub = |s: &str| d.subtypes.iter().any(|x| x == s);
        let fits = (d.types.artifact && !sub("Equipment")) || (d.types.enchantment && !sub("Aura"));
        if !fits || d.cost.mana_value() < 4 {
            return None;
        }
        self.battlefield.iter().copied().find(|&b| {
            self.obj(b).controller == o.controller
                && self.def_of(b).animates_big_noncreatures_your_turn
                && self.ability_loss(b).is_none()
        })
    }

    fn bello_animates(&self, id: ObjId) -> bool {
        self.bello_for(id).is_some()
    }

    /// The offspring cost `p` may pay casting `c`: printed, or granted to creature spells (Zinnia).
    fn offspring_cost(&self, p: PlayerId, c: ObjId) -> Option<ManaCost> {
        let d = self.def_of(c);
        if d.kicker.is_some() {
            return None;
        }
        d.offspring.or_else(|| {
            if !d.types.creature {
                return None;
            }
            self.permanents(p)
                .filter(|&x| !self.lost_abilities(x))
                .find_map(|x| self.def_of(x).grants_offspring)
        })
    }

    /// Untapped tokens `p` controls, other than `except`.
    fn untapped_tokens(&self, p: PlayerId, except: ObjId) -> Vec<ObjId> {
        self.permanents(p)
            .filter(|&x| x != except && self.is_token(x) && !self.obj(x).tapped)
            .collect()
    }

    /// Whether `id` is a commander (either of a seat's partners).
    pub fn is_commander(&self, id: ObjId) -> bool {
        self.commanders.contains(&Some(id)) || self.partners.contains(&Some(id))
    }

    /// `p`'s maximum hand size: seven, or the largest a permanent of theirs sets (Twenty-Toed Toad).
    fn max_hand(&self, p: PlayerId) -> usize {
        self.permanents(p)
            .map(|c| self.def_of(c).max_hand_size as usize)
            .max()
            .unwrap_or(0)
            .max(MAX_HAND)
    }

    /// Whether `p` controls a creature with a counter on it.
    fn controls_creature_with_counter(&self, p: PlayerId) -> bool {
        self.creatures(p).any(|c| self.has_counter(c))
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

    /// "If a creature attacking causes a triggered ability of a permanent you control to trigger,
    /// that ability triggers an additional time" (Windcrag Siege, Mardu): the active player's
    /// triggers added since `before` trigger once more for each such Siege.
    fn attack_triggers_again(&mut self, before: usize) {
        let p = self.active;
        let k = self
            .permanents(p)
            .filter(|&x| {
                self.def_of(x)
                    .attack_triggers_again_in_mode
                    .is_some_and(|m| self.obj(x).chosen_mode == Some(m))
            })
            .count();
        let new: Vec<StackItem> = self.triggered[before..]
            .iter()
            .filter(|t| t.controller == p && self.obj(t.source).controller == p)
            .cloned()
            .collect();
        for _ in 0..k {
            self.triggered.extend(new.iter().cloned());
        }
    }

    /// The creature Dragonbroods' Relic taps as part of its mana cost: an untapped creature `p`
    /// controls that isn't a mana source itself (so it can't be needed for the same payment), not
    /// `exclude`, lowest power first.
    fn relic_partner(&self, p: PlayerId, exclude: Option<ObjId>) -> Option<ObjId> {
        self.creatures(p)
            .filter(|&c| !self.obj(c).tapped && Some(c) != exclude)
            .filter(|&c| self.def_of(c).mana.is_none() && self.def_of(c).filter_mana.is_none())
            .min_by_key(|&c| self.power(c))
    }

    /// The amount a divided ability divides: fixed, or the lands its controller controls.
    fn divide_total(&self, item: &StackItem) -> u8 {
        let a = self.ability_of(item);
        if a.divide_lands {
            let lands = self
                .permanents(item.controller)
                .filter(|&x| self.types_of(x).land)
                .count();
            lands.min(u8::MAX as usize) as u8
        } else {
            a.divide
        }
    }

    /// Whether `c` may be picked by `Decision::PutPicksOntoBattlefield`: a creature card, or with
    /// `creature_and_land` a creature card or a land card of a kind not picked yet.
    fn pick_onto_battlefield_ok(&self, c: ObjId, picked: &[ObjId], creature_and_land: bool) -> bool {
        let t = self.def_of(c).types;
        if !creature_and_land {
            return t.creature;
        }
        // One creature card and one land card: a card that's both can fill either slot.
        let creature_taken = picked
            .iter()
            .any(|&x| self.def_of(x).types.creature && !self.def_of(x).types.land);
        let land_taken = picked
            .iter()
            .any(|&x| self.def_of(x).types.land && !self.def_of(x).types.creature);
        if picked.len() >= 2 {
            return false;
        }
        (t.creature && !creature_taken)
            || (t.land && !land_taken)
            || (picked.is_empty() && (t.creature || t.land))
    }

    /// Whether a looked-at card may be put onto the battlefield by `Decision::PutFromAmong`.
    fn put_among_ok(&self, c: ObjId, noncreature_nonland: bool, max_mv: Option<u32>) -> bool {
        let d = self.def_of(c);
        d.types.is_permanent()
            && (!noncreature_nonland || (!d.types.creature && !d.types.land))
            && max_mv.is_none_or(|m| d.cost.mana_value() <= m)
    }

    fn put_among_rest(&mut self, p: PlayerId, mut cards: Vec<ObjId>, to_hand: bool) {
        if to_hand {
            for c in cards {
                self.move_to(c, Zone::Hand);
            }
        } else {
            self.rng.shuffle(&mut cards);
            let lib = &mut self.players[p].library;
            lib.retain(|x| !cards.contains(x));
            for c in cards {
                lib.insert(0, c);
            }
        }
    }

    /// Decayed, printed or from a decayed counter (CR 702.147).
    fn is_decayed(&self, id: ObjId) -> bool {
        self.obj(id).zone == Zone::Battlefield && (self.def_of(id).decayed || self.obj(id).decayed)
    }

    /// Emergent Ultimatum: a monocolored card whose name differs from those already picked.
    fn mono_search_ok(&self, c: ObjId, picked: &[ObjId]) -> bool {
        let d = self.def_of(c);
        d.colors.iter().filter(|&&b| b).count() == 1 && !picked.iter().any(|&k| self.def_of(k).name == d.name)
    }

    /// Emergent Ultimatum: exile what was found and shuffle; an opponent picks one to shuffle back.
    fn finish_mono_search(&mut self, p: PlayerId, picked: Vec<ObjId>) {
        for &c in &picked {
            self.move_to(c, Zone::Exile);
        }
        self.shuffle_library(p);
        if !picked.is_empty() {
            self.ask(
                opponent(p),
                Decision::OpponentShufflesOne {
                    cards: picked,
                    caster: p,
                },
            );
        }
    }

    /// The equip cost after "{1} less for each color of the creature it targets", for target `t`.
    fn color_discounted(&self, a: &crate::Activated, t: Option<ObjId>) -> ManaCost {
        let mut cost = a.mana;
        if a.less_per_target_color {
            let colors = t.map_or(0, |t| self.colors_of(t).iter().filter(|&&b| b).count()) as u8;
            cost.generic = cost.generic.saturating_sub(colors);
        }
        cost
    }

    /// Zurgo, Thunder's Decree: during its controller's end step, their Warrior tokens can't be
    /// sacrificed.
    fn cant_be_sacrificed(&self, id: ObjId) -> bool {
        let p = self.obj(id).controller;
        self.step == Step::End
            && self.active == p
            && self.is_token(id)
            && self.has_subtype(id, "Warrior")
            && self
                .permanents(p)
                .any(|x| self.def_of(x).warrior_tokens_unsacrificeable_end_step)
    }

    /// Voice of Victory: `p` can't cast spells while an opponent with it takes their turn.
    fn cant_cast_now(&self, p: PlayerId) -> bool {
        (self.turn > 0 && self.no_more_spells_turn[p] == self.turn)
            || self.active != p
                && self
                    .permanents(self.active)
                    .any(|x| self.def_of(x).opponents_cant_cast_your_turn && !self.lost_abilities(x))
    }

    /// Glacierwood Siege (Sultai): `p` may play lands from their graveyard.
    fn siege_graveyard_lands(&self, p: PlayerId) -> bool {
        self.permanents(p).any(|x| {
            self.def_of(x)
                .graveyard_lands_in_mode
                .is_some_and(|m| self.obj(x).chosen_mode == Some(m))
                || (self.def_of(x).lands_from_graveyard && !self.lost_abilities(x))
        })
    }

    /// A counter of any kind was put on `id` (CR 122.6 includes entering with counters):
    /// "whenever a counter is put on a creature you control" triggers, once each turn per source.
    fn counter_put_on(&mut self, id: ObjId) {
        if !self.types_of(id).creature || self.obj(id).zone != Zone::Battlefield {
            return;
        }
        let p = self.obj(id).controller;
        let turn = self.turn;
        if self.obj(id).counters_put_turn != turn {
            self.obj_mut(id).counters_put_turn = turn;
            let before = self.triggered.len();
            self.fire(p, When::FirstCountersOnYourCreatureThisTurn, None);
            let zcc = self.obj(id).zcc;
            for t in &mut self.triggered[before..] {
                t.targets = vec![Chosen::Object(id, zcc)];
            }
        }
        let before = self.triggered.len();
        self.fire(p, When::CounterPutOnYourCreatureOnceEachTurn, None);
        let turn = self.turn;
        let mut fresh = Vec::new();
        for t in self.triggered.drain(before..).collect::<Vec<_>>() {
            if self.obj(t.source).once_each_turn != turn {
                self.obj_mut(t.source).once_each_turn = turn;
                fresh.push(t);
            }
        }
        self.triggered.extend(fresh);
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

    /// Replaces what `viewer` can't see with a guess, for search: the opponent's hand and library
    /// are redealt from the same cards, `viewer`'s own library is reshuffled, and the game's random
    /// numbers are reseeded. Everything `viewer` can see stays as it is (hand sizes, library sizes,
    /// every public zone). Cards that were revealed earlier are redealt too.
    pub fn determinize(&mut self, viewer: PlayerId, seed: u64) {
        let mut rng = Rng::new(seed);
        let opp = opponent(viewer);
        // A hidden card something still refers to (a trigger it put on the stack, a link) keeps its
        // identity: the rules already know which card it is.
        let items = self
            .items
            .iter()
            .chain(&self.triggered)
            .chain(&self.cast_snapshots)
            .chain(self.choosing.as_ref().map(|c| &c.item))
            .chain(self.mode_pending.as_ref().map(|m| &m.0));
        let mut pinned: Vec<ObjId> = items.flat_map(|i| [Some(i.source), i.extra]).flatten().collect();
        pinned.extend(&self.free_casts);
        pinned.extend(self.links.iter().flat_map(|&(a, _, b)| [a, b]));
        pinned.extend(self.emblems.iter().map(|e| e.2));
        let mut redeal = |g: &mut Game, mut ids: Vec<ObjId>| {
            ids.retain(|c| !pinned.contains(c));
            let mut defs: Vec<usize> = ids.iter().map(|&c| g.obj(c).def).collect();
            rng.shuffle(&mut defs);
            for (c, d) in ids.into_iter().zip(defs) {
                g.obj_mut(c).def = d;
            }
        };
        let hidden: Vec<ObjId> = self.players[opp]
            .hand
            .iter()
            .chain(&self.players[opp].library)
            .copied()
            .collect();
        redeal(self, hidden);
        let mine = self.players[viewer].library.clone();
        redeal(self, mine);
        self.rng = Rng::new(rng.next_u64());
    }

    /// Permanents on the battlefield controlled by `p`.
    pub fn permanents(&self, p: PlayerId) -> impl Iterator<Item = ObjId> + '_ {
        self.battlefield
            .iter()
            .copied()
            .filter(move |&id| self.obj(id).controller == p)
    }

    pub fn creatures(&self, p: PlayerId) -> impl Iterator<Item = ObjId> + '_ {
        self.permanents(p).filter(|&id| self.types_of(id).creature)
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
            Zone::Command => &mut self.command,
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
        // Unearth (CR 702.84a): if it would leave the battlefield, it's exiled instead.
        if from == Zone::Battlefield && to != Zone::Battlefield && self.obj(id).unearthed {
            to = Zone::Exile;
        }
        let mut shuffle = false;
        // CR 903.9: a commander that would go to a graveyard, exile, a hand or a library goes to the
        // command zone instead. (The owner may choose; always choosing it is what players do.)
        let choose_later = self.commander_choice && matches!(to, Zone::Graveyard | Zone::Exile);
        if self.is_commander(id)
            && matches!(to, Zone::Graveyard | Zone::Exile | Zone::Hand | Zone::Library)
            && !choose_later
        {
            to = Zone::Command;
        }
        if to == Zone::Graveyard {
            let o = self.obj(id);
            if self.def_of(id).shuffle_instead_of_graveyard {
                to = Zone::Library;
                shuffle = true;
            } else if from == Zone::Battlefield
                && self.types_of(id).creature
                && (o.exile_if_dies == Some((o.zcc, self.turn)) || o.finality)
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
        if to == Zone::Graveyard && from != Zone::Graveyard && !self.is_token(id) {
            if self.def_of(id).types.land {
                self.land_to_graveyard_pending[owner] = true;
            }
            if from == Zone::Library && self.def_of(id).types.creature {
                self.creatures_milled_pending[owner] = true;
            }
            if from == Zone::Library && self.def_of(id).types.land {
                self.lands_milled_pending[owner] = true;
            }
        }
        if to == Zone::Graveyard && from != Zone::Graveyard && !self.is_token(id) {
            self.fire_source(id, When::PutIntoGraveyardFromAnywhere);
        }
        if from == Zone::Graveyard && to != Zone::Graveyard && !self.is_token(id) {
            self.graveyard_left_this_turn[owner] = true;
            self.graveyard_leave_any_pending[owner] = true;
            if self.active == owner {
                self.graveyard_leave_pending[owner] = true;
            }
        }
        let commander_waits =
            self.commander_choice && matches!(to, Zone::Graveyard | Zone::Exile) && self.is_commander(id);
        let o = self.obj_mut(id);
        o.zone = to;
        o.commander_choice_pending = commander_waits;
        o.zcc += 1;
        if to != Zone::Stack {
            o.cast_free = false;
        }
        if to != Zone::Exile {
            o.playable = None;
        }
        if to != Zone::Exile && to != Zone::Stack {
            o.stash = false;
        }
        if from == Zone::Battlefield {
            o.bait = 0;
            o.sacrifice_at_end = false;
            o.exile_at_end = false;
            o.command_at_end = false;
        }
        if to == Zone::Graveyard {
            o.graveyard_turn = turn;
            if from == Zone::Library {
                o.milled_turn = turn;
            }
        }
        if from == Zone::Battlefield {
            o.last_counters = (o.counters, o.stun, o.keyword_counters);
            o.stun = 0;
            o.skip_untap = false;
            o.time_counters = 0;
            o.quest = 0;
            o.finality = false;
            o.eot_creature = None;
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
            o.keyword_counters = Keywords::NONE;
            o.lore = 0;
            o.defender_ignored_turn = 0;
            o.eot_subtype = None;
            o.decayed = false;
            o.sacrifice_end_combat = false;
            o.used_once = false;
            o.last_soul = o.soul;
            o.last_revival = o.revival;
            o.revival = 0;
            o.chosen_color = None;
            o.chosen_type = None;
            o.animated = None;
            o.stage = 0;
            o.fellowship = 0;
            o.soul = 0;
        }
        o.attached_to = None;
        if to != Zone::Battlefield && to != Zone::Stack {
            o.controller = o.owner;
        }
        if to == Zone::Battlefield {
            o.since_turn = turn;
            o.was_cast = false;
            o.dealt_damage = false;
        }
        if from == Zone::Stack {
            o.cast_uncounterable = false;
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
    fn is_saga(&self, id: ObjId) -> bool {
        self.def_of(id).subtypes.iter().any(|s| s == "Saga")
    }

    /// The Saga's last chapter number (CR 714.2c).
    fn final_chapter(&self, id: ObjId) -> u8 {
        self.def_of(id)
            .triggers
            .iter()
            .filter_map(|t| match t.when {
                When::Chapter(n) => Some(n),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }

    /// A lore counter on a Saga: its chapter ability for that number triggers (CR 714.2b).
    fn add_lore(&mut self, id: ObjId) {
        let n = self.obj(id).lore.saturating_add(1);
        self.obj_mut(id).lore = n;
        let p = self.obj(id).controller;
        let chapters: Vec<usize> = self
            .def_of(id)
            .triggers
            .iter()
            .enumerate()
            .filter(|(_, t)| t.when == When::Chapter(n))
            .map(|(i, _)| i)
            .collect();
        for i in chapters {
            self.triggered.push(StackItem {
                kind: StackKind::Trigger(i),
                ..Game::casting(id, p)
            });
        }
    }

    fn zone_triggers(&mut self, id: ObjId, from: Zone, to: Zone, last_controller: PlayerId) {
        let before = self.triggered.len();
        self.zone_triggers_once(id, from, to, last_controller);
        if to != Zone::Battlefield || from == Zone::Battlefield || self.obj(id).zone != Zone::Battlefield {
            return;
        }
        // Yarok, Ancient Greenwarden, Traveling Chocobo: the entering's triggers trigger again.
        let land = self.types_of(id).land;
        let doublers: Vec<PlayerId> = self
            .battlefield
            .iter()
            .copied()
            .filter(|&x| !self.lost_abilities(x))
            .filter(|&x| match self.def_of(x).entering_triggers_again {
                Some(EntryDoubler::Permanent) => true,
                Some(EntryDoubler::Land) => land,
                Some(EntryDoubler::LandOrSubtype(sub)) => {
                    (land || self.has_subtype(id, sub)) && self.obj(id).controller == self.obj(x).controller
                }
                None => false,
            })
            .map(|x| self.obj(x).controller)
            .collect();
        let new: Vec<StackItem> = self.triggered[before..].to_vec();
        for p in doublers {
            for t in &new {
                let src = self.obj(t.source);
                if t.controller == p && src.controller == p && src.zone == Zone::Battlefield {
                    self.triggered.push(t.clone());
                }
            }
        }
    }

    fn zone_triggers_once(&mut self, id: ObjId, from: Zone, to: Zone, last_controller: PlayerId) {
        if to == Zone::Battlefield && from != Zone::Battlefield && self.is_saga(id) {
            self.add_lore(id); // CR 714.3a: a Saga enters with a lore counter
        }
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
        if from == Zone::Battlefield && to != Zone::Battlefield {
            self.fire_source(id, When::LeavesBattlefield);
            if self.types_of(id).creature {
                self.fire(
                    last_controller,
                    When::CreatureYouControlLeavesBattlefield,
                    Some(id),
                );
            }
            if self.is_token(id) {
                self.fire(last_controller, When::TokenYouControlLeaves, Some(id));
            }
        }
        if to == Zone::Battlefield && from != Zone::Battlefield && self.is_token(id) {
            let c = self.obj(id).controller;
            self.token_created_turn[c] = self.turn;
            self.fire(c, When::TokenYouControlEnters, Some(id));
        }
        if to == Zone::Battlefield && from != Zone::Battlefield && self.types_of(id).enchantment {
            let c = self.obj(id).controller;
            self.fire(c, When::EnchantmentYouControlEnters, None); // constellation
        }
        if to == Zone::Battlefield && from != Zone::Battlefield {
            let c = self.obj(id).controller;
            for other in self.permanents(c).filter(|&o| o != id).collect::<Vec<_>>() {
                if self.lost_abilities(other) {
                    continue;
                }
                let whens: Vec<When> = self.def_of(other).triggers.iter().map(|tr| tr.when).collect();
                for (n, w) in whens.into_iter().enumerate() {
                    if let When::AnotherYouControlEntersWithAnySubtype(subs) = w {
                        if subs.iter().any(|s| self.has_subtype(id, s)) {
                            let refers = self.def_of(other).triggers[n]
                                .ability
                                .targets
                                .first()
                                .is_some_and(|f| f.triggering);
                            let zcc = self.obj(id).zcc;
                            self.triggered.push(StackItem {
                                kind: StackKind::Trigger(n),
                                targets: if refers {
                                    vec![Chosen::Object(id, zcc)]
                                } else {
                                    Vec::new()
                                },
                                ..Game::casting(other, c)
                            });
                        }
                    }
                }
            }
        }
        if dies && self.types_of(id).creature {
            // "Whenever equipped creature dies": Equipment still attached until state-based actions.
            let gear: Vec<ObjId> = self
                .battlefield
                .iter()
                .copied()
                .filter(|&e| self.obj(e).attached_to.is_some_and(|(c, _)| c == id))
                .collect();
            for e in gear {
                self.fire_source(e, When::EquippedDies);
            }
            self.fire(0, When::AnotherCreatureDies, Some(id));
            self.fire(1, When::AnotherCreatureDies, Some(id));
            let before = self.triggered.len();
            self.fire(0, When::OtherCreaturesDieOnceEachTurn, Some(id));
            self.fire(1, When::OtherCreaturesDieOnceEachTurn, Some(id));
            let turn = self.turn;
            let mut fresh = Vec::new();
            for t in self.triggered.drain(before..).collect::<Vec<_>>() {
                if self.obj(t.source).once_each_turn != turn {
                    self.obj_mut(t.source).once_each_turn = turn;
                    fresh.push(t);
                }
            }
            self.triggered.extend(fresh);
            self.creature_died_this_turn = true;
            self.creature_died_under[last_controller] = true;
            // "Whenever a creature you control dies while this card is in your graveyard": cards
            // already there (not the one that just died).
            for g in self.players[last_controller].graveyard.clone() {
                if g == id || self.obj(g).owner != last_controller {
                    continue;
                }
                let whens: Vec<When> = self.def_of(g).triggers.iter().map(|tr| tr.when).collect();
                for (n, w) in whens.into_iter().enumerate() {
                    if w == When::CreatureYouControlDiesFromGraveyard {
                        self.triggered.push(StackItem {
                            source: g,
                            controller: last_controller,
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
            self.fire(last_controller, When::CreatureYouControlDies, Some(id));
            // "a creature you control without flying": as it last existed (printed and counters).
            let flew = self
                .def_of(id)
                .keywords
                .with(self.obj(id).last_counters.2)
                .has(Keywords::FLYING);
            if !flew {
                let before = self.triggered.len();
                self.fire(
                    last_controller,
                    When::CreatureYouControlWithoutFlyingDies,
                    Some(id),
                );
                for t in &mut self.triggered[before..] {
                    t.extra = Some(id);
                }
            }
            let (plus, stun, kw) = self.obj(id).last_counters;
            if plus != 0 || stun > 0 || kw != Keywords::NONE {
                let before = self.triggered.len();
                self.fire(
                    last_controller,
                    When::CreatureYouControlWithCountersDies,
                    Some(id),
                );
                for t in &mut self.triggered[before..] {
                    t.extra = Some(id);
                }
            }
            let was = self.obj(id).zcc - 1;
            let turn = self.turn;
            for (c, z, src, n, controller, t) in self.dies_under_control.clone() {
                if c == id && z == was && controller == last_controller && t == turn {
                    self.triggered.push(StackItem {
                        kind: StackKind::Trigger(n),
                        ..Game::casting(src, controller)
                    });
                }
            }
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
                let before = self.triggered.len();
                self.fire(
                    last_controller,
                    When::AnotherNontokenCreatureYouControlDies,
                    Some(id),
                );
                // "that creature" / "it": the card that died, for abilities that refer to it (Myrkul).
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
                self.fire(0, When::AnotherNontokenCreatureDies, Some(id));
                self.fire(1, When::AnotherNontokenCreatureDies, Some(id));
            }
        }
        if to == Zone::Battlefield {
            self.obj_mut(id).chosen_mode = None; // a new object chooses again
            self.obj_mut(id).linked_card = None;
            // Replacement effects on entering (CR 614.1c-d).
            let def = self.def_of(id).clone();
            let controller_now = self.obj(id).controller;
            let opponent_forces_tapped = def.types.creature
                && self
                    .permanents(opponent(controller_now))
                    .any(|c| self.def_of(c).opponents_creatures_enter_tapped);
            let unless_ok = def.enters_tapped_unless.is_some_and(|subs| {
                self.permanents(controller_now)
                    .any(|x| x != id && subs.iter().any(|&sub| self.has_subtype(x, sub)))
            });
            let lands_other: Vec<ObjId> = self
                .permanents(controller_now)
                .filter(|&x| x != id && self.types_of(x).land)
                .collect();
            let few_basics = def.enters_tapped_unless_basics > 0
                && lands_other.iter().filter(|&&x| self.types_of(x).basic).count()
                    < def.enters_tapped_unless_basics as usize;
            let many_lands = def
                .enters_tapped_unless_at_most_lands
                .is_some_and(|n| lands_other.len() > n as usize);
            if def.enters_tapped
                || opponent_forces_tapped
                || (def.enters_tapped_unless.is_some() && !unless_ok)
                || def.enters_tapped_unless_reveal.is_some_and(|subs| {
                    !self.players[controller_now]
                        .hand
                        .iter()
                        .any(|&x| subs.iter().any(|&sub| self.has_subtype(x, sub)))
                })
                || few_basics
                || many_lands
            {
                self.obj_mut(id).tapped = true;
            }
            if def.shock_land {
                self.obj_mut(id).shock_pending = true;
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
            // Displaced Dinosaurs: a historic permanent you control becomes a 7/7 Dinosaur.
            let historic =
                def.types.artifact || def.types.legendary || def.subtypes.iter().any(|s| s == "Saga");
            let controller = self.obj(id).controller;
            if historic
                && self
                    .permanents(controller)
                    .any(|x| x != id && self.def_of(x).historic_become_dinosaurs && !self.lost_abilities(x))
            {
                self.obj_mut(id).eot_creature = Some((u32::MAX, 7, 7, "Dinosaur", Keywords::NONE));
            }
            if def.enters_counters_greatest_exiled_mv {
                let mv = self
                    .exile
                    .iter()
                    .map(|&c| self.def_of(c).cost.mana_value())
                    .max()
                    .unwrap_or(0);
                self.add_counters(id, mv as i32);
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
            // Dragonstorm Globe: "Each Dragon you control enters with an additional +1/+1 counter on it."
            let flat = self
                .permanents(controller)
                .filter(|&x| x != id)
                .filter_map(|x| self.def_of(x).subtype_enters_with_counter)
                .filter(|&sub| self.has_base_subtype(id, sub))
                .count() as i32;
            // Grumgully: "Each other non-Human creature you control enters with an additional +1/+1
            // counter on it."
            let non = if self.types_of(id).creature {
                self.permanents(controller)
                    .filter(|&x| x != id)
                    .filter_map(|x| self.def_of(x).other_non_subtype_enters_with_counter)
                    .filter(|&sub| !self.has_base_subtype(id, sub))
                    .count() as i32
            } else {
                0
            };
            let chauffeur = if def.enters_counters_one_plus_others {
                1 + self.creatures(controller).filter(|&x| x != id).count() as i32
            } else {
                0
            };
            if def.vanishing > 0 {
                self.obj_mut(id).time_counters = def.vanishing; // CR 702.63a
            }
            let bonus = bonus + flat + non + chauffeur;
            if bonus > 0 {
                let n = bonus * self.doubling(controller);
                self.obj_mut(id).counters += n;
                self.counter_put_on(id); // CR 122.6: entering with counters counts
            }
            if let Some((n, cond)) = def.enters_counters {
                let controller = self.obj(id).controller;
                let yes = match cond {
                    Condition::Kicked => self.obj(id).kicked,
                    Condition::Attacked => self.attacked_this_turn[controller],
                    Condition::Always => true,
                    Condition::CastTwoSpellsThisTurn => self.spells_cast[controller] >= 2,
                    Condition::OpponentLostLifeThisTurn => false,
                    Condition::SourceWasNot(_) => false,
                    Condition::HadRevival => false,
                    _ => false,
                };
                if yes {
                    let n = n * self.doubling(controller);
                    self.obj_mut(id).counters += n;
                    self.counter_put_on(id);
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
                let turn = self.turn;
                for (w, src, n, who, t) in self.delayed_this_turn.clone() {
                    if w == When::CreatureYouControlEntersThisTurn && who == controller && t == turn {
                        self.triggered.push(StackItem {
                            kind: StackKind::Trigger(n),
                            ..Game::casting(src, who)
                        });
                    }
                }
                if !self.is_token(id) {
                    self.fire(
                        controller,
                        When::AnotherNontokenCreatureYouControlEnters,
                        Some(id),
                    );
                }
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
                            When::AnotherCreatureYouControlEntersWithPowerAtLeast(p) => {
                                any != id && entering_power >= p
                            }
                            When::AnotherCreatureYouControlEntersWithBasePower(p) => {
                                any != id && self.base_pt(id).0 == p
                            }
                            When::AnotherCreatureYouControlEntersNotCast => {
                                any != id && (from != Zone::Stack || self.obj(id).card_copy)
                            }
                            _ => false,
                        };
                        if hit {
                            let refers = self.def_of(any).triggers[n]
                                .ability
                                .targets
                                .first()
                                .is_some_and(|f| f.triggering);
                            let zcc = self.obj(id).zcc;
                            self.triggered.push(StackItem {
                                source: any,
                                controller,
                                kind: StackKind::Trigger(n),
                                targets: if refers {
                                    vec![Chosen::Object(id, zcc)]
                                } else {
                                    Vec::new()
                                },
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
            let own_death = when == When::CreatureYouControlDies && dies && self.types_of(id).creature;
            let (plus, stun, kw) = self.obj(id).last_counters;
            let had_counters = plus != 0 || stun > 0 || kw != Keywords::NONE;
            if when == When::CreatureYouControlWithCountersDies
                && dies
                && self.types_of(id).creature
                && had_counters
            {
                self.triggered.push(StackItem {
                    kind: StackKind::Trigger(n),
                    extra: Some(id),
                    ..Game::casting(id, controller)
                });
            }
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
        if from == Zone::Battlefield && to != Zone::Battlefield {
            self.obj_mut(id).suspend_haste = false;
        }
        // A copy leaving the battlefield is its own card again (CR 707.2). Its leave triggers were
        // collected from the copied card; they'd now point at the wrong abilities, so they're dropped.
        if from == Zone::Battlefield && to != Zone::Battlefield {
            if let Some(d) = self.obj_mut(id).printed_def.take() {
                self.obj_mut(id).def = d;
                self.triggered
                    .retain(|t| !(t.source == id && matches!(t.kind, StackKind::Trigger(_))));
            }
            // CR 711.8: a transformed permanent that leaves is front face up again; leave triggers
            // collected from its back face would point at the front's abilities, so they're dropped.
            self.obj_mut(id).copy_exceptions = false;
            self.obj_mut(id).face_down = false;
            self.obj_mut(id).unearthed = false;
            if self.obj(id).transformed {
                self.obj_mut(id).transformed = false;
                self.triggered
                    .retain(|t| !(t.source == id && matches!(t.kind, StackKind::Trigger(_))));
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
            Pending::Priority(p) => {
                let mut v = self.priority_actions(p);
                if self.manual_mana[p] {
                    v.extend(self.mana_moves(p));
                }
                v
            }
            Pending::DeclareAttackers(p) => {
                let walkers: Vec<ObjId> = self
                    .permanents(opponent(p))
                    .filter(|&w| self.types_of(w).planeswalker)
                    .collect();
                let mut v: Vec<Action> = Vec::new();
                for c in self.attack_candidates(p) {
                    v.push(Action::Attack(c));
                    v.extend(walkers.iter().map(|&w| Action::AttackPlaneswalker(c, w)));
                }
                // CR 508.1d: creatures that attack each combat if able must be declared.
                let forced = self.must_attack_turn[p] == self.turn && self.turn > 0;
                if !self
                    .attack_candidates(p)
                    .any(|c| self.def_of(c).must_attack || forced)
                {
                    v.push(Action::FinishAttacks);
                }
                v
            }
            Pending::DeclareBlockers(p) => self.block_actions(p),
            Pending::Decide(p) => self.decision_actions(p),
            Pending::FreeCast(p) => {
                let mut v = vec![Action::PassPriority];
                v.extend(self.free_cast_actions(p));
                v
            }
            Pending::RemoveCounters(p) => self
                .creatures(p)
                .filter(|&c| Game::removable_counters(self.obj(c)) > 0)
                .map(Action::Pick)
                .collect(),
            Pending::Divide(_) => {
                let c = self.choosing.as_ref().expect("dividing");
                let total = self.divide_total(&c.item);
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
                    // CR 115.3: one player can't fill two slots of a single "divided" target word.
                    Target::Player(p) => {
                        self.divide_total(&c.item) == 0 || !c.item.targets.contains(&Chosen::Player(p))
                    }
                });
                // Each target of a divided amount gets at least 1, so no more targets than the total.
                if ability.divide_lands && c.item.targets.len() >= self.divide_total(&c.item) as usize {
                    options.clear();
                }
                if ability.same_graveyard {
                    if let Some(&Chosen::Object(first, _)) = c.item.targets.first() {
                        let owner = self.obj(first).owner;
                        options.retain(|&t| matches!(t, Target::Object(x) if self.obj(x).owner == owner));
                    }
                }
                let can_stop = c.item.targets.len() >= ability.min_targets.unwrap_or(usize::MAX)
                    && (!ability.targets_exactly_x || c.item.targets.len() == c.item.x as usize);
                // Dragonfire Blade's equip: only targets whose discount makes it affordable.
                if let (false, StackKind::Activated(n)) = (c.cast, c.item.kind) {
                    let a = &self.activated_of(c.item.source, n);
                    if a.less_per_target_color {
                        let uses_self = a.tap || a.sacrifice_self;
                        options.retain(|&t| match t {
                            Target::Object(x) => self.can_pay_for(
                                c.item.controller,
                                &self.color_discounted(a, Some(x)),
                                uses_self.then_some(c.item.source),
                                PayFor::Ability(c.item.source),
                            ),
                            Target::Player(_) => false,
                        });
                    }
                }
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
                            pay_for_spell(it.source, mode.mode),
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

    /// Activated ability number `n` of `id`: its card's, or one past them the fight ability The
    /// Grand Evolution granted it this turn (`Game::granted_fight`).
    pub fn activated_of(&self, id: ObjId, n: usize) -> &Activated {
        let own = &self.def_of(id).activated;
        if n < own.len() {
            return &own[n];
        }
        let kept: Option<&Activated> = self.kept_activated(id).nth(n - own.len());
        if let Some(a) = kept {
            return a;
        }
        // After its own and kept abilities: the granted fight (if it has it now), then soulbond's.
        let k = n - own.len() - self.kept_activated(id).count();
        let fight_now = self.granted_fight_of(id).is_some();
        if k == 0 && fight_now {
            return granted_fight_ability();
        }
        if k == usize::from(fight_now) && self.soulbond_granted_of(id).is_some() {
            return soulbond_flicker_ability();
        }
        // Its source changed since it was activated: the granted ability it was.
        if self.granted_fight.iter().any(|e| e.0 == id) {
            granted_fight_ability()
        } else {
            soulbond_flicker_ability()
        }
    }

    /// The creature `id` is paired with, if the pairing still holds: both on the battlefield under
    /// one controller, each paired with the other (CR 702.95c).
    pub fn paired_partner(&self, id: ObjId) -> Option<ObjId> {
        let o = self.obj(id);
        let (x, zcc) = o.paired_with?;
        let other = self.obj(x);
        let back = other.paired_with == Some((id, o.zcc));
        (o.zone == Zone::Battlefield
            && other.zone == Zone::Battlefield
            && other.zcc == zcc
            && other.controller == o.controller
            && back)
            .then_some(x)
    }

    /// Deadeye Navigator's granted ability, if `id` has it now (paired with a soulbond source).
    fn soulbond_granted_of(&self, id: ObjId) -> Option<&'static Activated> {
        let x = self.paired_partner(id)?;
        (self.def_of(id).soulbond_flicker || self.def_of(x).soulbond_flicker)
            .then(soulbond_flicker_ability)
            .filter(|_| !self.obj(id).lost_soulbond && !self.obj(x).lost_soulbond)
    }

    /// Pairs two creatures (soulbond).
    fn pair(&mut self, a: ObjId, b: ObjId) {
        let (za, zb) = (self.obj(a).zcc, self.obj(b).zcc);
        self.obj_mut(a).paired_with = Some((b, zb));
        self.obj_mut(b).paired_with = Some((a, za));
    }

    /// The granted fight ability, if `id` has it now.
    fn granted_fight_of(&self, id: ObjId) -> Option<&'static Activated> {
        let o = self.obj(id);
        let has = o.zone == Zone::Battlefield
            && self
                .granted_fight
                .iter()
                .any(|&(x, zcc, turn)| x == id && zcc == o.zcc && turn == self.turn);
        has.then(granted_fight_ability)
    }

    /// The ability a stack entry carries out.
    pub fn ability_of(&self, item: &StackItem) -> &Ability {
        let def = self.def_of(item.source);
        match item.kind {
            StackKind::Spell | StackKind::SpellCopy => match item.mode {
                Some(OMEN_MODE) => &def.omen.as_ref().expect("an Omen").spell,
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
            StackKind::Ward { .. }
            | StackKind::WardLife { .. }
            | StackKind::WardDiscard { .. }
            | StackKind::WardSacrifice { .. } => &WARD_ABILITY,
            // A transformed planeswalker that left (its −N took the last loyalty) is front face up
            // again; its ability still resolves from the back face it was activated on.
            StackKind::Loyalty(n) => match def.loyalty_abilities.get(n) {
                Some(a) => &a.1,
                None => {
                    let printed = &self.defs[self.obj(item.source).def];
                    &printed.back.as_ref().expect("a back face").loyalty_abilities[n].1
                }
            },
            StackKind::Offspring => offspring_ability(),
            StackKind::Activated(n) => &self.activated_of(item.source, n).ability,
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
                if f.abilities && self.ability_on_stack(id).is_some() && o.zone != Zone::Stack {
                    return true; // "target ... ability": an ability of this source on the stack
                }
                if !Game::controller_ok(f.controller, controller, o.controller) {
                    return false;
                }
                if f.colored && o.zone == Zone::Battlefield && !self.colors_of(id).iter().any(|&c| c) {
                    return false; // "a permanent that's one or more colors"
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
                    let ty = self.types_of(id);
                    return f.spells
                        && is_spell
                        && f.spell_creature.is_none_or(|c| c == self.types_of(id).creature)
                        && (!f.spells_enchantment_instant_sorcery || {
                            let t = self.types_of(id);
                            t.enchantment || t.instant || t.sorcery
                        })
                        && f.spell_creature.is_none_or(|c| c == ty.creature)
                        && (!f.spell_artifact_or_creature || ty.artifact || ty.creature)
                        && f.max_mana_value
                            .is_none_or(|m| self.def_of(id).cost.mana_value() <= m);
                }
                if let Some(creature_only) = f.graveyard {
                    let d = self.def_of(id);
                    return o.zone == Zone::Graveyard
                        && (!creature_only || d.types.creature)
                        && (!f.nonland_permanents || (d.types.is_permanent() && !d.types.land))
                        && (!f.permanent_cards || d.types.is_permanent())
                        && (!f.instant_or_sorcery || d.types.instant || d.types.sorcery)
                        && (!f.noncreature_nonland || (!d.types.creature && !d.types.land))
                        && (!f.creature_or_land || d.types.creature || d.types.land)
                        && (!f.put_there_this_turn || o.graveyard_turn == self.turn)
                        && f.max_mana_value.is_none_or(|m| d.cost.mana_value() <= m)
                        && f.creature_max_power.is_none_or(|m| d.power <= m)
                        && (!f.nonland_cards || !d.types.land);
                }
                if o.zone != Zone::Battlefield {
                    return false;
                }
                // CR 702.18a: shroud stops all spells and abilities from targeting it.
                if !f.triggering && self.keywords(id).has(Keywords::SHROUD) {
                    return false;
                }
                // CR 702.11b: hexproof stops opponents' spells and abilities from targeting it.
                if o.controller != controller && !f.triggering && self.keywords(id).has(Keywords::HEXPROOF) {
                    return false;
                }
                let ty = &self.types_of(id);
                let in_combat = self
                    .combat
                    .as_ref()
                    .is_some_and(|c| c.attackers.contains(&id) || c.is_blocking(id));
                let creature_ok = ty.creature
                    && f.creatures
                    && (!f.creature_needs_flying || self.keywords(id).has(Keywords::FLYING))
                    && (!f.creature_without_flying || !self.keywords(id).has(Keywords::FLYING))
                    && f.creature_min_power.is_none_or(|m| self.power(id) >= m)
                    && f.creature_max_power.is_none_or(|m| self.power(id) <= m)
                    && (!f.in_combat || in_combat)
                    && (!f.was_dealt_damage || o.damage > 0);
                let kind_ok = creature_ok
                    || (ty.planeswalker && f.planeswalkers)
                    || (ty.artifact && f.artifacts)
                    || (ty.land && f.lands)
                    || (ty.enchantment && f.enchantments)
                    || (!ty.land && f.nonland_permanents)
                    || f.any_permanent;
                let kind_ok = kind_ok
                    && (!f.noncreature || !ty.creature)
                    && (!f.nonbasic || !ty.basic)
                    && (!f.nonlegendary || !ty.legendary)
                    && (!f.legendary_only || ty.legendary)
                    && (!f.tokens_only || self.is_token(id))
                    && (!f.entered_this_turn || o.since_turn == self.turn)
                    && (!f.nontoken || !self.is_token(id))
                    && f.max_mana_value
                        .is_none_or(|m| self.def_of(id).cost.mana_value() <= m);
                // Kin-Tree Severance: "permanent with mana value 3 or greater".
                kind_ok
                    && f.min_mana_value
                        .is_none_or(|m| self.def_of(id).cost.mana_value() >= m)
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
            .chain(self.ability_sources().filter(|_| f.abilities).map(Target::Object))
            .filter(|&t| self.target_ok(f, controller, t))
            .collect()
    }

    /// Sources of abilities on the stack that aren't themselves on the stack, each once.
    fn ability_sources(&self) -> impl Iterator<Item = ObjId> + '_ {
        let mut seen: Vec<ObjId> = Vec::new();
        for i in &self.items {
            let spell = matches!(i.kind, StackKind::Spell | StackKind::SpellCopy);
            if !spell && self.obj(i.source).zone != Zone::Stack && !seen.contains(&i.source) {
                seen.push(i.source);
            }
        }
        seen.into_iter()
    }

    /// The topmost activated or triggered ability on the stack from `source`, by index in `items`.
    fn ability_on_stack(&self, source: ObjId) -> Option<usize> {
        self.items
            .iter()
            .rposition(|i| i.source == source && !matches!(i.kind, StackKind::Spell | StackKind::SpellCopy))
    }

    /// The stack entry a "target spell or ability" chose: the spell if `id` is on the stack, else
    /// the topmost ability of that source.
    fn targeted_item(&self, id: ObjId) -> Option<usize> {
        if self.obj(id).zone == Zone::Stack {
            self.items
                .iter()
                .rposition(|i| i.source == id && matches!(i.kind, StackKind::Spell | StackKind::SpellCopy))
        } else {
            self.ability_on_stack(id)
        }
    }

    /// Legal new targets for target number `slot` of `items[item]` (CR 115.7): not its current
    /// target, not one its other targets already have, and never the spell itself (CR 115.5).
    fn new_target_options(&self, item: usize, slot: usize) -> Vec<Target> {
        let it = &self.items[item];
        let a = self.ability_of(it);
        let Some(&f) = a.targets.get(slot) else {
            return Vec::new();
        };
        if f.triggering || slot >= it.targets.len() {
            return Vec::new(); // "that creature" isn't a target
        }
        let chosen = |t: Target| match t {
            Target::Player(p) => Chosen::Player(p),
            Target::Object(x) => Chosen::Object(x, self.obj(x).zcc),
        };
        self.candidates(&f, it.controller)
            .into_iter()
            .filter(|&t| self.slot_ok(it, &f, t))
            .filter(|&t| !it.targets.contains(&chosen(t)))
            .filter(|&t| t != Target::Object(it.source))
            .collect()
    }

    /// Manifest `card` (CR 701.40a): put it onto the battlefield face down as a 2/2 creature.
    fn manifest(&mut self, p: PlayerId, card: ObjId) {
        self.obj_mut(card).face_down = true;
        self.move_to(card, Zone::Battlefield);
        if self.obj(card).zone == Zone::Battlefield {
            self.obj_mut(card).controller = p;
        } else {
            self.obj_mut(card).face_down = false;
        }
    }

    /// Whether an effect can turn `x` face up: a face-down permanent card (CR 701.40c: an instant or
    /// sorcery stays face down).
    fn can_turn_face_up(&self, x: ObjId) -> bool {
        self.obj(x).face_down && self.defs[self.obj(x).def].types.is_permanent()
    }

    /// Asks `p` about the next of `items[item]`'s targets, from `slot` on, that can change.
    fn ask_new_target(&mut self, p: PlayerId, item: usize, from: usize) {
        let n = self.items.get(item).map_or(0, |i| i.targets.len());
        for slot in from..n {
            if !self.new_target_options(item, slot).is_empty() {
                self.ask(
                    p,
                    Decision::ChooseNewTarget {
                        item,
                        slot: slot as u8,
                    },
                );
                return;
            }
        }
    }

    /// Target legality that depends on the stack entry itself: "other" and "mana value at most the
    /// source's power".
    fn slot_ok(&self, item: &StackItem, f: &Filter, t: Target) -> bool {
        let Target::Object(x) = t else { return true };
        if self.protected_from(x, item.source) {
            return false; // CR 702.16c
        }
        let from_instant = matches!(item.kind, StackKind::Spell | StackKind::SpellCopy)
            && self.types_of(item.source).instant;
        if from_instant && self.def_of(x).hexproof_from_instants && self.obj(x).controller != item.controller
        {
            return false;
        }
        if f.other && x == item.source {
            return false;
        }
        // Dragonfire Blade: "hexproof from monocolored" (CR 702.11d).
        let o = self.obj(x);
        if o.zone == Zone::Battlefield
            && o.controller != item.controller
            && self.colors_of(item.source).iter().filter(|&&b| b).count() == 1
            && self.battlefield.iter().any(|&e| {
                self.def_of(e).equipped_hexproof_from_monocolored
                    && self.obj(e).attached_to == Some((x, o.zcc))
            })
        {
            return false;
        }
        if f.mana_value_is_x && self.def_of(x).cost.mana_value() != item.x as u32 {
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
        // Split second: only mana abilities (added by the caller) while it's on the stack.
        if self
            .items
            .iter()
            .any(|i| i.kind == StackKind::Spell && self.def_of(i.source).split_second)
        {
            return vec![Action::PassPriority];
        }
        let mut v = vec![Action::PassPriority];
        let sorcery_timing =
            p == self.active && matches!(self.step, Step::Main1 | Step::Main2) && self.items.is_empty();
        let hand = self.players[p].hand.iter().map(|&c| (c, false));
        let flashback = self.players[p].graveyard.iter().filter(|&&c| {
            let d = self.def_of(c);
            d.flashback.is_some()
                || ((self.obj(c).flashback_eot || self.obj(c).harmonize_eot)
                    && (d.types.instant || d.types.sorcery))
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
        let conduit_ok = self.spells_cast[p] == 0;
        let from_graveyard = self.players[p].graveyard.iter().copied().filter(move |&c| {
            self.obj(c)
                .playable
                .is_some_and(|(q, until)| q == p && turn <= until)
                || (conduit_ok && turn > 0 && self.obj(c).conduit_turn == turn)
        });
        // Muldrotha: a land from your graveyard, during your turn.
        let muldrotha = p == self.active && self.permanents(p).any(|x| self.def_of(x).graveyard_permanents);
        // Glacierwood Siege (Sultai): "You may play lands from your graveyard", with your land drops.
        let siege_lands = self.siege_graveyard_lands(p);
        let grave_land = self.players[p].graveyard.iter().copied().filter(move |&c| {
            (siege_lands || (muldrotha && !self.graveyard_types_used[p][3])) && self.def_of(c).types.land
        });
        let command = self
            .command
            .iter()
            .copied()
            .filter(move |&c| self.obj(c).owner == p);
        // The top card of your library (Glarb, Oracle of Mul Daya, Traveling Chocobo).
        let top = self.players[p].library.last().copied().filter(|&c| {
            let d = self.def_of(c);
            self.permanents(p).filter(|&x| !self.lost_abilities(x)).any(|x| {
                let s = self.def_of(x);
                if d.types.land {
                    s.lands_from_top
                } else {
                    s.top_spells_min_mana_value
                        .is_some_and(|m| d.cost.mana_value() >= m)
                        || s.top_spells_subtype
                            .is_some_and(|sub| d.subtypes.iter().any(|t| t == sub))
                        || (s.top_spells_creatures && d.types.creature)
                }
            })
        });
        let candidates: Vec<(ObjId, bool)> = hand
            .chain(top.map(|c| (c, false)))
            .chain(command.map(|c| (c, false)))
            .chain(flashback.map(|&c| (c, true)))
            .chain(from_exile.map(|c| (c, false)))
            .chain(from_graveyard.map(|c| (c, false)))
            .chain(grave_land.map(|c| (c, false)))
            .collect();
        let cant_cast = self.cant_cast_now(p);
        for (c, from_graveyard) in candidates {
            let def = self.def_of(c);
            // A modal double-faced card with a land back: played as that land, or cast as its front.
            let mdfc_land =
                def.modal_back && !def.types.land && def.back.as_ref().is_some_and(|b| b.types.land);
            if def.types.land || mdfc_land {
                // CR 305.2: one land per turn, at sorcery timing.
                let drops = 1
                    + self.extra_lands_this_turn[p]
                    + self
                        .permanents(p)
                        .map(|x| self.def_of(x).extra_land_drops)
                        .sum::<u8>()
                    + self
                        .battlefield
                        .iter()
                        .filter(|&&x| self.def_of(x).extra_land_drop_each_player && !self.lost_abilities(x))
                        .count() as u8;
                if sorcery_timing && self.players[p].lands_played < drops {
                    v.push(Action::PlayLand(c));
                }
                if def.types.land {
                    continue;
                }
            }
            if cant_cast {
                continue; // Voice of Victory
            }
            // CR 307.1, 302.1, 304.1, 702.8a: only instants and flash ignore sorcery timing.
            let flash_all = self.permanents(p).any(|x| self.def_of(x).grants_flash);
            // An Omen half from hand, with its own type's timing and targets.
            if let (Some(o), Zone::Hand) = (&def.omen, self.obj(c).zone) {
                let omen_mode = CastMode {
                    mode: Some(OMEN_MODE),
                    ..CastMode::default()
                };
                let flash_sd = self
                    .permanents(p)
                    .any(|x| self.def_of(x).flash_sorceries_and_dragons);
                if (sorcery_timing || flash_all || o.instant || flash_sd)
                    && self.trigger_can_target(&Game::casting(c, p), &o.spell)
                    && self.mode_affordable(p, c, omen_mode)
                {
                    v.push(Action::CastOmen(c));
                }
            }
            // Whirlwing Stormbrood: sorcery spells and Dragon spells as though they had flash.
            let flash_sd = self
                .permanents(p)
                .any(|x| self.def_of(x).flash_sorceries_and_dragons);
            let normal_timing = sorcery_timing
                || flash_all
                || def.types.instant
                || def.keywords.has(Keywords::FLASH)
                || (flash_sd && (def.types.sorcery || def.subtypes.iter().any(|s| s == "Dragon")));
            // Molten Exhale: as though it had flash, if a Dragon is beheld (then only that way).
            let behold_flash = !normal_timing && def.behold == Behold::MayForFlash && self.can_behold(p, c);
            let timing = normal_timing || behold_flash;
            let castable = def.types.is_permanent() || def.spell.is_some() || !def.modes.is_empty();
            if !timing || !castable {
                continue;
            }
            let in_combat = matches!(
                self.step,
                Step::BeginCombat
                    | Step::DeclareAttackers
                    | Step::DeclareBlockers
                    | Step::FirstStrikeDamage
                    | Step::CombatDamage
                    | Step::EndCombat
            );
            if def.cast_only_during_your_combat && !(p == self.active && in_combat) {
                continue; // Savage Beating
            }
            // An X spell's targets can depend on X (Stolen by the Fae), and a kicked spell can have
            // its own targets (Tear Asunder): those are checked per way of casting below.
            let per_mode = def.cost.x > 0 || def.kicked_spell.is_some();
            if let Some(ability) = &def.spell {
                if !per_mode && !self.trigger_can_target(&Game::casting(c, p), ability) {
                    continue; // CR 601.2c: a spell needs a legal choice for every target.
                }
                // CR 601.2a: the card is on the stack by the time targets are chosen. Leaving a
                // graveyard can shrink a target (Consuming Aberration counts that graveyard), so
                // check again from there. Rare enough that a scratch copy is cheap.
                if self.obj(c).zone == Zone::Graveyard && !ability.targets.is_empty() {
                    let mut after = self.clone();
                    after.move_to(c, Zone::Stack);
                    if !after.trigger_can_target(&Game::casting(c, p), ability) {
                        continue;
                    }
                }
            }
            for (action, mode) in self.cast_modes(p, c, from_graveyard) {
                if behold_flash && !mode.kicked {
                    continue;
                }
                if per_mode && mode.mode.is_none() {
                    let ability = if mode.kicked {
                        def.kicked_spell.as_ref().or(def.spell.as_ref())
                    } else {
                        def.spell.as_ref()
                    };
                    let item = StackItem {
                        x: mode.x,
                        kicked: mode.kicked,
                        ..Game::casting(c, p)
                    };
                    if ability.is_some_and(|a| !self.trigger_can_target(&item, a)) {
                        continue;
                    }
                }
                if self.mode_affordable(p, c, mode) {
                    v.push(action);
                }
            }
        }
        // Muldrotha: one permanent spell of each permanent type from your graveyard.
        if muldrotha {
            let flash_all = self.permanents(p).any(|x| self.def_of(x).grants_flash);
            for &c in &self.players[p].graveyard {
                let def = self.def_of(c);
                let timing = sorcery_timing || flash_all || def.keywords.has(Keywords::FLASH);
                if def.types.land || !def.types.is_permanent() || !timing {
                    continue;
                }
                if let Some(ability) = &def.spell {
                    if !self.trigger_can_target(&Game::casting(c, p), ability) {
                        continue; // an Aura needs something to enchant
                    }
                }
                if !self.mode_affordable(p, c, CastMode::default()) {
                    continue;
                }
                for t in [0u8, 1, 2, 4] {
                    if Game::has_type(def.types, t) && !self.graveyard_types_used[p][t as usize] {
                        v.push(Action::CastFromGraveyardAs(c, t));
                    }
                }
            }
        }
        // Cast from the graveyard by removing counters from your creatures.
        let available: i32 = self
            .creatures(p)
            .map(|c| Game::removable_counters(self.obj(c)))
            .sum();
        for &c in &self.players[p].graveyard {
            let n = self.def_of(c).graveyard_cast_remove_counters;
            if n > 0
                && sorcery_timing
                && available >= n as i32
                && self.mode_affordable(p, c, CastMode::default())
            {
                v.push(Action::CastFromGraveyard(c));
            }
            if self.def_of(c).cast_from_graveyard_finality
                && sorcery_timing
                && self.mode_affordable(p, c, CastMode::default())
            {
                v.push(Action::CastFromGraveyard(c));
            }
        }
        v.extend(self.activations(p));
        v.extend(self.granted_activations(p));
        v.extend(self.loyalty_actions(p, sorcery_timing));
        v
    }

    /// Counters a cost like "remove six counters from among creatures you control" can use.
    fn removable_counters(o: &Object) -> i32 {
        o.counters.max(0) + o.stun.max(0) + o.revival.max(0)
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
            // Dread Return: "Flashback—Sacrifice three creatures" needs that many to sacrifice.
            let victims = self.creatures(p).filter(|&x| !self.cant_be_sacrificed(x)).count();
            if (def.flashback_sacrifice as usize) > victims {
                return Vec::new();
            }
            let mut v = vec![(
                Action::CastFlashback(c),
                CastMode {
                    flashback: true,
                    ..CastMode::default()
                },
            )];
            if def.harmonize || self.obj(c).harmonize_eot {
                for cr in self
                    .creatures(p)
                    .filter(|&x| !self.obj(x).tapped)
                    .collect::<Vec<_>>()
                {
                    v.push((
                        Action::CastHarmonize(c, cr),
                        CastMode {
                            flashback: true,
                            extra: Some(cr),
                            ..CastMode::default()
                        },
                    ));
                }
            }
            return v;
        }
        if !def.modes.is_empty() {
            // Will of the Abzan: both modes only while you control a commander.
            let commander = self.permanents(p).any(|x| self.is_commander(x));
            let last = def.modes.len() - 1;
            return (0..def.modes.len())
                .filter(|&m| !(def.last_mode_needs_commander && m == last && !commander))
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
                .filter(|&x| !self.cant_be_sacrificed(x))
                .map(|x| (Action::CastWith(c, x), CastMode::with(x)))
        };
        match &def.extra_cost {
            Some(ExtraCost::SacrificeCreature) => creatures().collect(),
            Some(ExtraCost::SacrificeArtifactOrCreature) => self
                .permanents(p)
                .filter(|&x| {
                    (self.types_of(x).creature || self.types_of(x).artifact) && !self.cant_be_sacrificed(x)
                })
                .map(|x| (Action::CastWith(c, x), CastMode::with(x)))
                .collect(),
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
                // Defiler of Vigor: a green permanent spell may cost {G} less for 2 life.
                // Phyrexian mana (CR 107.4f): every Phyrexian pip paid with 2 life.
                let life_pips: u8 = def.phyrexian.iter().sum();
                if life_pips > 0 && def.alt_cost.is_none() && self.players[p].life >= 2 * life_pips as i32 {
                    v.push((
                        Action::CastAlternative(c),
                        CastMode {
                            alt: true,
                            ..CastMode::default()
                        },
                    ));
                }
                let defiler = def.alt_cost.is_none()
                    && life_pips == 0
                    && def.types.is_permanent()
                    && def.colors[4]
                    && def.cost.colored[4] > 0
                    && self.players[p].life >= 2
                    && self
                        .permanents(p)
                        .any(|x| self.def_of(x).green_permanents_pay_life_for_g && !self.lost_abilities(x));
                if defiler {
                    v.push((
                        Action::CastAlternative(c),
                        CastMode {
                            alt: true,
                            ..CastMode::default()
                        },
                    ));
                }
                // Nissa, Worldsoul Speaker: pay eight {E} rather than the mana cost of a permanent spell.
                if def.types.is_permanent() && !def.types.land {
                    let energy = self.players[p].energy;
                    if let Some(x) = self.permanents(p).find(|&x| {
                        let n = self.def_of(x).permanents_for_energy;
                        n > 0 && energy >= n as u32 && !self.lost_abilities(x)
                    }) {
                        v.push((
                            Action::CastWith(c, x),
                            CastMode {
                                alt: true,
                                extra: Some(x),
                                ..CastMode::default()
                            },
                        ));
                    }
                }
                // Force of Vigor: exile a card of that color from your hand instead, not on your turn.
                if let (Some(color), false) = (def.pitch_not_your_turn, p == self.active) {
                    for &x in &self.players[p].hand {
                        if x != c && self.def_of(x).colors[color as usize] {
                            v.push((Action::CastWith(c, x), CastMode::with(x)));
                        }
                    }
                }
                let commander_ok =
                    !def.alt_cost_needs_commander || self.permanents(p).any(|x| self.is_commander(x));
                if let (Some((_, needed)), true) = (def.alt_cost, commander_ok) {
                    let creatures = self
                        .battlefield
                        .iter()
                        .filter(|&&x| self.types_of(x).creature)
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
                let buyback = def.buyback_sacrifice.is_some_and(|(sub, n)| {
                    self.permanents(p)
                        .filter(|&x| self.has_subtype(x, sub) && !self.cant_be_sacrificed(x))
                        .count()
                        >= n as usize
                });
                if (def.kicker.is_some() && def.buyback_sacrifice.is_none())
                    || buyback
                    || (def.behold != Behold::No && self.can_behold(p, c))
                    || self.offspring_cost(p, c).is_some()
                {
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

    /// Whether `p` can behold a Dragon for `c`: a Dragon they control, or another Dragon card in hand.
    fn can_behold(&self, p: PlayerId, c: ObjId) -> bool {
        self.permanents(p).any(|x| self.has_subtype(x, "Dragon"))
            || self.players[p]
                .hand
                .iter()
                .any(|&x| x != c && self.def_of(x).subtypes.iter().any(|s| s == "Dragon"))
    }

    /// The mana a spell costs in `mode` with these targets (CR 601.2f).
    fn spell_cost(&self, p: PlayerId, c: ObjId, mode: CastMode, targets: &[Chosen]) -> ManaCost {
        let def = self.def_of(c);
        let o = self.obj(c);
        let in_hand = o.zone == Zone::Hand || (o.zone == Zone::Stack && o.from_hand);
        // CR 107.3b, 118.9: cast without paying its mana cost, X is 0. Choosing a bigger X means
        // paying the cost.
        // Dracogenesis: "You may cast Dragon spells without paying their mana costs" (not an Omen half).
        let free_subtype = mode.mode != Some(OMEN_MODE)
            && self.permanents(p).any(|x| {
                self.def_of(x)
                    .free_subtype_spells
                    .is_some_and(|sub| def.subtypes.iter().any(|s| s == sub))
            });
        let free = (o.cast_free
            || free_subtype
            || (in_hand && self.permanents(p).any(|x| self.def_of(x).free_from_hand)))
            && mode.x == 0
            && !mode.alt
            && !mode.flashback;
        let pitched = def.pitch_not_your_turn.is_some() && mode.extra.is_some() && !mode.flashback;
        let energized = mode.alt
            && mode
                .extra
                .is_some_and(|x| self.def_of(x).permanents_for_energy > 0);
        let mut cost = if free || pitched || energized {
            ManaCost::default()
        } else if mode.alt {
            match def.alt_cost {
                Some((c, _)) => c,
                None if def.phyrexian.iter().any(|&n| n > 0) => {
                    let mut c = def.cost.with_x(mode.x);
                    for (k, &n) in def.phyrexian.iter().enumerate() {
                        c.colored[k] = c.colored[k].saturating_sub(n);
                    }
                    c
                }
                None => {
                    // Defiler of Vigor: 2 life paid, {G} less.
                    let mut c = def.cost.with_x(mode.x);
                    c.colored[4] = c.colored[4].saturating_sub(1);
                    c
                }
            }
        } else if mode.flashback {
            def.flashback.unwrap_or(def.cost).with_x(mode.x)
        } else if mode.mode == Some(OMEN_MODE) {
            def.omen.as_ref().expect("an Omen").cost.with_x(mode.x)
        } else {
            def.cost.with_x(mode.x)
        };
        // Spree: each chosen + mode's additional cost.
        if let Some(extra) = mode.mode.and_then(|m| def.mode_costs.get(m as usize)) {
            cost = add_costs(&cost, extra);
        }
        // Entwine: the last mode is every mode, for the entwine cost more.
        if let (Some(e), Some(m)) = (def.entwine, mode.mode) {
            if m as usize + 1 == def.modes.len() {
                cost = add_costs(&cost, &e);
            }
        }
        if mode.kicked {
            if let Some(k) = def.kicker {
                cost = add_costs(&cost, &k);
            } else if let Some(k) = self.offspring_cost(p, c) {
                cost = add_costs(&cost, &k);
            }
        }
        // Caustic Exhale: "behold a Dragon or pay {1}": not beheld, pay it.
        if let (Behold::OrPay(extra), false) = (def.behold, mode.kicked) {
            cost = add_costs(&cost, &extra);
        }
        // CR 903.8: commander tax.
        let tax = if o.zone == Zone::Command {
            o.command_zone_casts.saturating_mul(2)
        } else if o.zone == Zone::Stack && o.from_command {
            o.commander_tax
        } else {
            0
        };
        cost.generic = cost.generic.saturating_add(tax);
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
                .filter(|&&x| self.types_of(x).instant || self.types_of(x).sorcery)
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
            Some(Reduction::IfCastAnotherSpell(n)) => {
                if self.spells_cast[p] > 0 {
                    *n
                } else {
                    0
                }
            }
            Some(Reduction::PerCreatureYouControl) => self.creatures(p).count().min(255) as u8,
            Some(Reduction::Delirium(n)) => {
                let mut kinds = [false; 7];
                for &c in &self.players[p].graveyard {
                    let t = self.def_of(c).types;
                    let flags = [
                        t.land,
                        t.creature,
                        t.artifact,
                        t.enchantment,
                        t.instant,
                        t.sorcery,
                        t.planeswalker,
                    ];
                    for (k, f) in kinds.iter_mut().zip(flags) {
                        *k |= f;
                    }
                }
                if kinds.iter().filter(|&&k| k).count() >= 4 {
                    *n
                } else {
                    0
                }
            }
            Some(Reduction::PerTokenYouControl) => {
                self.permanents(p).filter(|&x| self.is_token(x)).count().min(255) as u8
            }
            Some(Reduction::PerCreatureYouControlPowerAtLeast(n)) => self
                .creatures(p)
                .filter(|&c| self.power(c) >= *n)
                .count()
                .min(255) as u8,
            Some(Reduction::PerCreatureOnBattlefield) => {
                (self.creatures(0).count() + self.creatures(1).count()).min(255) as u8
            }
            Some(Reduction::GreatestPowerAmongYourCreatures) => self
                .creatures(p)
                .map(|c| self.power(c))
                .max()
                .unwrap_or(0)
                .clamp(0, 255) as u8,
            Some(Reduction::PerCreatureAttackedThisTurn) => {
                self.attacked_creatures_this_turn.len().min(255) as u8
            }
            Some(Reduction::PerAnyAttackingCreature) => {
                self.combat.as_ref().map_or(0, |c| c.attackers.len()).min(255) as u8
            }
            Some(Reduction::MoreIfTargetsSubtype(..)) => 0,
            Some(Reduction::GreatestArtifactsAnOpponentControls) => self
                .permanents(opponent(p))
                .filter(|&x| self.types_of(x).artifact)
                .count()
                .min(255) as u8,
            Some(Reduction::IfTargetsTapped(n)) => match targets.first() {
                Some(&Chosen::Object(x, _)) if self.obj(x).tapped && self.types_of(x).creature => *n,
                _ => 0,
            },
            None => 0,
        };
        let discount: u8 = if def.types.instant || def.types.sorcery {
            self.permanents(p).map(|x| self.def_of(x).spell_discount).sum()
        } else if def.types.creature && mode.mode != Some(OMEN_MODE) {
            let big = if def.power >= 4 {
                self.permanents(p)
                    .map(|x| self.def_of(x).big_creature_spell_discount)
                    .sum::<u8>()
            } else {
                0
            };
            self.permanents(p)
                .map(|x| self.def_of(x).creature_spell_discount)
                .sum::<u8>()
                + big
        } else {
            0
        };
        // Harmonize: tapping a creature pays {X} less, X its power.
        let harmonizes = def.harmonize || o.harmonize_eot;
        if let (true, true, Some(cr)) = (mode.flashback, harmonizes, mode.extra) {
            cost.generic = cost.generic.saturating_sub(self.power(cr).clamp(0, 255) as u8);
        }
        // Highspire Bell-Ringer: "The second spell you cast each turn costs {1} less to cast."
        let second: u8 = if self.spells_cast[p] == 1 {
            self.permanents(p)
                .map(|x| self.def_of(x).second_spell_discount)
                .sum()
        } else {
            0
        };
        // Goblin Anarchomancer: "Each spell you cast that's red or green costs {1} less to cast."
        let second = second
            + if def.colors[3] || def.colors[4] {
                self.permanents(p)
                    .map(|x| self.def_of(x).red_green_spell_discount)
                    .sum::<u8>()
            } else {
                0
            }
            + if def.colors[3] {
                self.permanents(p)
                    .map(|x| self.def_of(x).red_spell_discount)
                    .sum::<u8>()
            } else {
                0
            };
        // Emerald Medallion: "Green spells you cast cost {1} less to cast."
        let second = second
            + if def.colors[4] {
                self.permanents(p)
                    .map(|x| self.def_of(x).green_spell_discount)
                    .sum::<u8>()
            } else {
                0
            };
        // Temur Battlecrier: during your turn, {1} less for each creature you control with power 4+.
        let battle: u8 = if self.active == p {
            let crier = self
                .permanents(p)
                .filter(|&x| self.def_of(x).your_turn_discount_per_big_creature && !self.lost_abilities(x))
                .count();
            if crier > 0 {
                let big = self.creatures(p).filter(|&x| self.power(x) >= 4).count();
                (crier * big).min(255) as u8
            } else {
                0
            }
        } else {
            0
        };
        cost.generic = cost.generic.saturating_sub(less + discount + second + battle);
        // Dragon's Prey: "This spell costs {2} more to cast if it targets a Dragon."
        if let Some(Reduction::MoreIfTargetsSubtype(sub, n)) = def.reduction {
            let hits = targets
                .iter()
                .any(|t| matches!(t, &Chosen::Object(x, _) if self.has_subtype(x, sub)));
            if hits {
                cost.generic = cost.generic.saturating_add(n);
            }
        }
        cost.generic = cost.generic.saturating_sub(self.obj(c).delve);
        if self.obj(c).stash {
            // "Mana of any type can be spent": colored pips become generic.
            cost.generic += cost.colored.iter().sum::<u8>();
            cost.colored = [0; 5];
        }
        cost
    }

    /// Whether some legal choice of targets makes the spell affordable in `mode`.
    fn mode_affordable(&self, p: PlayerId, c: ObjId, mode: CastMode) -> bool {
        let mut cost = self.spell_cost(p, c, mode, &[]);
        cost.generic = cost.generic.saturating_sub(self.delve_available(p, c));
        if self.can_pay_for(p, &cost, None, pay_for_spell(c, mode.mode)) {
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
                            self.can_pay_for(
                                p,
                                &self.spell_cost(p, c, mode, &chosen),
                                None,
                                pay_for_spell(c, mode.mode),
                            )
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
            && (!self.types_of(id).creature
                || o.since_turn < self.turn
                || self.keywords(id).has(Keywords::HASTE))
    }

    /// Taps (and sacrifices, for Treasure-like sources) what pays `cost`. Never uses `exclude`.
    fn pay_mana(&mut self, p: PlayerId, cost: &ManaCost, exclude: Option<ObjId>) {
        self.pay_mana_for(p, cost, exclude, PayFor::Other);
    }

    /// Pays `cost`, for casting `spell` if given (restricted mana can then help).
    fn pay_mana_for(&mut self, p: PlayerId, cost: &ManaCost, exclude: Option<ObjId>, spell: PayFor) {
        if !self.can_pay_direct(p, cost, exclude, spell)
            && self.devotee_fix(p, cost, exclude, spell).is_none()
        {
            if let Some((used, bill, all)) = self.paid_mana_plan(p, cost, exclude, spell) {
                let plain: Vec<ManaSource> = all.iter().map(|s| s.1).collect();
                let mut paid = mana::pay(&bill, &plain).expect("planned");
                paid.sort_unstable_by(|a, b| b.cmp(a));
                let pool_len = self.players[p].pool.len();
                // Output mana the bill didn't need floats in the pool (CR 106.4), like any other.
                let spare: Vec<ManaSource> = (0..all.len())
                    .filter(|&i| i >= pool_len && all[i].0.is_none() && !paid.contains(&i))
                    .map(|i| all[i].1)
                    .collect();
                for &f in &used {
                    self.become_tapped(f);
                }
                for i in paid {
                    if i < pool_len {
                        self.players[p].pool.remove(i);
                    } else if let (Some(id), _) = all[i] {
                        if self.types_of(id).snow {
                            self.snow_paid += 1;
                        }
                        self.become_tapped(id);
                        if self.def_of(id).sac_for_mana {
                            self.move_to(id, Zone::Graveyard);
                        }
                    }
                    // Otherwise a two-mana output, paid for by tapping its Signet or filter land above.
                }
                self.players[p].pool.extend(spare);
                return;
            }
        }
        let mut fixed = *cost;
        if !self.can_pay_direct(p, cost, exclude, spell) {
            // A Devotee turns one mana into one of its colors (once each turn).
            if let Some((devotee, k)) = self.devotee_fix(p, cost, exclude, spell) {
                self.obj_mut(devotee).filter_used_turn = self.turn;
                fixed = k;
            }
        }
        let cost = &fixed;
        let sources = self.payment_sources(p, exclude, spell);
        let pool = self.players[p].pool.clone();
        let all: Vec<ManaSource> = pool.iter().copied().chain(sources.iter().map(|s| s.1)).collect();
        let mut paid = if self.smart_pay {
            let keep = self.keep_weights(p, &all, spell);
            mana::pay_ranked(cost, &all, pool.len(), &keep).expect("payable")
        } else {
            mana::pay(cost, &all).expect("payable")
        };
        paid.sort_unstable_by(|a, b| b.cmp(a));
        // Pain lands: 1 damage for each one whose colored mana the payment needed, i.e.
        // the same sources couldn't pay with that land making only {C}.
        let mut pained = Vec::new();
        for &i in &paid {
            if i >= pool.len() && self.def_of(sources[i - pool.len()].0).pain_colored {
                let alt: Vec<ManaSource> = paid
                    .iter()
                    .map(|&j| {
                        if j == i {
                            ManaSource {
                                colorless: true,
                                ..ManaSource::default()
                            }
                        } else {
                            all[j]
                        }
                    })
                    .collect();
                if mana::pay(cost, &alt).is_none() {
                    pained.push(sources[i - pool.len()].0);
                }
            }
        }
        for land in pained {
            self.deal_damage(land, Target::Player(p), 1);
        }
        for i in paid {
            if i < pool.len() {
                self.players[p].pool.remove(i);
                continue;
            }
            let id = sources[i - pool.len()].0;
            if self.types_of(id).snow {
                self.snow_paid += 1; // {S}: mana from a snow source (CR 107.4h)
            }
            // Delighted Halfling: its mana for a legendary spell is the restricted ability's
            // (always the better choice), so "that spell can't be countered".
            if let PayFor::Spell(c) = spell {
                if self.def_of(id).any_color_for_legendary && self.def_of(c).types.legendary {
                    self.obj_mut(c).cast_uncounterable = true;
                }
            }
            let tomb = self.def_of(id).mana_tap_damage;
            if tomb > 0 && !self.obj(id).tapped {
                self.deal_damage(id, Target::Player(p), tomb as i32); // Ancient Tomb
            }
            let henge = self.def_of(id).mana_tap_life;
            if henge > 0 && !self.obj(id).tapped {
                self.gain_life(p, henge); // The Great Henge
            }
            self.become_tapped(id);
            self.land_tapped_for_mana(p, id);
            if self.def_of(id).mana_taps_creature {
                if let Some(c) = self.relic_partner(p, exclude) {
                    self.become_tapped(c);
                }
            }
            if self.def_of(id).sac_for_mana {
                self.move_to(id, Zone::Graveyard);
            }
        }
    }

    /// How much `p` wants each source kept untapped: the colored mana the rest of their hand and
    /// their permanents' activated abilities ask for, summed over the colors the source can make.
    /// Floating mana (already in the pool) scores 0.
    fn keep_weights(&self, p: PlayerId, sources: &[ManaSource], spell: PayFor) -> Vec<u32> {
        let mut demand = [0u32; 5];
        let mut add = |c: &ManaCost| {
            for (i, d) in demand.iter_mut().enumerate() {
                *d += c.colored[i] as u32;
            }
            for (k, &(a, b)) in mana::PAIRS.iter().enumerate() {
                demand[a] += c.hybrid[k] as u32;
                demand[b] += c.hybrid[k] as u32;
            }
        };
        for &c in &self.players[p].hand {
            if matches!(spell, PayFor::Spell(s) | PayFor::OmenSpell(s) if s == c) {
                continue;
            }
            add(&self.def_of(c).cost);
            if let Some(o) = &self.def_of(c).omen {
                add(&o.cost); // an Omen card might be cast as either half
            }
        }
        for id in self.permanents(p).collect::<Vec<_>>() {
            for a in &self.def_of(id).activated {
                add(&a.mana);
            }
        }
        let floating = self.players[p].pool.len();
        sources
            .iter()
            .enumerate()
            .map(|(i, s)| {
                if i < floating {
                    return 0;
                }
                (0..5).filter(|&c| s.colors[c]).map(|c| demand[c]).sum()
            })
            .collect()
    }

    /// Manual mana: the TapForMana moves `p` has now, one per source and color it can make.
    /// Sources whose mana is restricted (only for some spells) stay automatic.
    fn mana_moves(&self, p: PlayerId) -> Vec<Action> {
        let mut seen: Vec<ObjId> = Vec::new();
        let mut v = Vec::new();
        for (id, m) in self.untapped_sources(p) {
            let d = self.def_of(id);
            if seen.contains(&id)
                || d.mana_only_for_subtype.is_some()
                || d.any_color_for_chosen_type
                || d.mana_only_for_creatures
                || d.any_color_for_dragon_or_omen
                || d.any_color_for_legendary
                || d.plaza_mana
            {
                continue;
            }
            seen.push(id);
            let colors: Vec<u8> = if d.vivid_mana {
                vec![5] // one move: it makes all its mana at once
            } else {
                (0..5u8)
                    .filter(|&c| m.colors[c as usize])
                    .chain(m.colorless.then_some(5))
                    .collect()
            };
            v.extend(colors.into_iter().map(|c| Action::TapForMana(id, c)));
        }
        v
    }

    /// Vorinclex: an opponent's land tapped for mana doesn't untap during its controller's next
    /// untap step.
    fn land_tapped_for_mana(&mut self, p: PlayerId, id: ObjId) {
        if self.types_of(id).land
            && self
                .permanents(opponent(p))
                .any(|x| self.def_of(x).vorinclex && !self.lost_abilities(x))
        {
            self.obj_mut(id).skip_untap = true;
        }
    }

    fn tap_for_mana(&mut self, p: PlayerId, id: ObjId, c: u8) {
        let made: Vec<ManaSource> = self
            .untapped_sources(p)
            .into_iter()
            .filter(|s| s.0 == id)
            .map(|s| s.1)
            .collect();
        let vivid = self.def_of(id).vivid_mana;
        for m in made {
            let one = if vivid {
                m
            } else if c == 5 {
                ManaSource {
                    colorless: true,
                    ..ManaSource::default()
                }
            } else {
                ManaSource::of(&[crate::Color::ALL[c as usize]])
            };
            self.players[p].pool.push(one);
        }
        if self.def_of(id).pain_colored && c != 5 {
            self.deal_damage(id, Target::Player(p), 1);
        }
        let tomb = self.def_of(id).mana_tap_damage;
        if tomb > 0 && !self.obj(id).tapped {
            self.deal_damage(id, Target::Player(p), tomb as i32); // Ancient Tomb
        }
        let henge = self.def_of(id).mana_tap_life;
        if henge > 0 && !self.obj(id).tapped {
            self.gain_life(p, henge); // The Great Henge
        }
        self.become_tapped(id);
        self.land_tapped_for_mana(p, id);
        if self.def_of(id).mana_taps_creature {
            if let Some(partner) = self.relic_partner(p, None) {
                self.become_tapped(partner);
            }
        }
        if self.def_of(id).sac_for_mana {
            self.move_to(id, Zone::Graveyard);
        }
        self.pending = Pending::Priority(p);
    }

    fn can_pay(&self, p: PlayerId, cost: &ManaCost, exclude: Option<ObjId>) -> bool {
        self.can_pay_for(p, cost, exclude, PayFor::Other)
    }

    /// The sources that may pay: not `exclude`, and restricted mana only for a spell it allows.
    fn payment_sources(
        &self,
        p: PlayerId,
        exclude: Option<ObjId>,
        spell: PayFor,
    ) -> Vec<(ObjId, ManaSource)> {
        self.untapped_sources(p)
            .into_iter()
            .filter(|s| Some(s.0) != exclude)
            .filter(|s| {
                !self.def_of(s.0).mana_only_for_creatures
                    || matches!(spell, PayFor::Spell(c) if self.def_of(c).types.creature)
            })
            .filter(|s| match self.def_of(s.0).mana_only_for_subtype {
                None => true,
                // Giada: "Spend this mana only to cast an Angel spell."
                Some(sub) => matches!(spell, PayFor::Spell(c) if self.has_base_subtype(c, sub)),
            })
            .map(|(id, m)| {
                // Plaza of Heroes: any color for a legendary spell.
                if self.def_of(id).plaza_mana
                    && matches!(spell, PayFor::Spell(c) if self.def_of(c).types.legendary)
                {
                    let mut any = ManaSource::of(&crate::Color::ALL);
                    any.colorless = true;
                    return (id, any);
                }
                // Delighted Halfling: any color, only for a legendary spell.
                if self.def_of(id).any_color_for_legendary {
                    if matches!(spell, PayFor::Spell(c) if self.def_of(c).types.legendary) {
                        let mut any = ManaSource::of(&crate::Color::ALL);
                        any.colorless = true;
                        return (id, any);
                    }
                    return (id, m);
                }
                // Maelstrom of the Spirit Dragon: any color, only for a Dragon spell or an Omen spell.
                if self.def_of(id).any_color_for_dragon_or_omen {
                    let allowed = match spell {
                        PayFor::Spell(c) => self.has_base_subtype(c, "Dragon"),
                        PayFor::OmenSpell(_) => true,
                        _ => false,
                    };
                    if allowed {
                        let mut any = ManaSource::of(&crate::Color::ALL);
                        any.colorless = true;
                        return (id, any);
                    }
                    return (id, m);
                }
                // Secluded Courtyard: any color, but only for its chosen type.
                if !self.def_of(id).any_color_for_chosen_type {
                    return (id, m);
                }
                let Some(t) = self.obj(id).chosen_type.as_deref() else {
                    return (id, m);
                };
                let allowed = match spell {
                    PayFor::Spell(c) => self.def_of(c).types.creature && self.has_base_subtype(c, t),
                    PayFor::Ability(src) => self.types_of(src).creature && self.has_subtype(src, t),
                    PayFor::Other | PayFor::OmenSpell(_) => false,
                };
                if allowed {
                    let mut any = ManaSource::of(&crate::Color::ALL);
                    any.colorless = true;
                    (id, any)
                } else {
                    (id, m)
                }
            })
            .collect()
    }

    fn can_pay_for(&self, p: PlayerId, cost: &ManaCost, exclude: Option<ObjId>, spell: PayFor) -> bool {
        self.can_pay_direct(p, cost, exclude, spell)
            || self.devotee_fix(p, cost, exclude, spell).is_some()
            || self.paid_mana_plan(p, cost, exclude, spell).is_some()
    }

    /// Signets and filter lands ("{1}, {T}: Add {W}{U}"): the smallest set of them that makes `cost`
    /// payable, solved jointly with the other sources: each one used takes itself out of the normal
    /// sources, adds its own cost to the bill, and adds its two mana. Returns the ones to use, the
    /// total bill, and the sources (the pool first, then lands and others, then the two-mana outputs).
    #[allow(clippy::type_complexity)]
    fn paid_mana_plan(
        &self,
        p: PlayerId,
        cost: &ManaCost,
        exclude: Option<ObjId>,
        spell: PayFor,
    ) -> Option<(Vec<ObjId>, ManaCost, Vec<(Option<ObjId>, ManaSource)>)> {
        let filters: Vec<ObjId> = self
            .permanents(p)
            .filter(|&x| {
                self.def_of(x).paid_mana.is_some()
                    && !self.obj(x).tapped
                    && Some(x) != exclude
                    && !self.lost_abilities(x)
                    && !self.activation_stopped(x)
            })
            .take(5)
            .collect();
        if filters.is_empty() {
            return None;
        }
        let normal = self.payment_sources(p, exclude, spell);
        let mut masks: Vec<u32> = (1..(1u32 << filters.len())).collect();
        masks.sort_by_key(|m| m.count_ones());
        for mask in masks {
            let used: Vec<ObjId> = (0..filters.len())
                .filter(|&k| mask & (1 << k) != 0)
                .map(|k| filters[k])
                .collect();
            let mut bill = *cost;
            let mut all: Vec<(Option<ObjId>, ManaSource)> =
                self.players[p].pool.iter().map(|&m| (None, m)).collect();
            all.extend(
                normal
                    .iter()
                    .filter(|s| !used.contains(&s.0))
                    .map(|&(id, m)| (Some(id), m)),
            );
            for &f in &used {
                let (input, outputs) = self.def_of(f).paid_mana.expect("a two-mana source");
                bill = add_costs(&bill, &input);
                all.extend(outputs.iter().map(|&m| (None, m)));
            }
            let plain: Vec<ManaSource> = all.iter().map(|s| s.1).collect();
            if mana::pay(&bill, &plain).is_some() {
                return Some((used, bill, all));
            }
        }
        None
    }

    /// "{1}: Add {A}, {B}, or {C}. Activate only once each turn." (the Devotees): a Devotee not used
    /// this turn lets one mana of any kind pay a colored pip of its colors. Returns the Devotee and
    /// the cost with that pip turned into {1}, if that makes it payable.
    fn devotee_fix(
        &self,
        p: PlayerId,
        cost: &ManaCost,
        exclude: Option<ObjId>,
        spell: PayFor,
    ) -> Option<(ObjId, ManaCost)> {
        for id in self.permanents(p) {
            let Some(colors) = self.def_of(id).filter_mana else {
                continue;
            };
            if self.obj(id).filter_used_turn == self.turn
                || Some(id) == exclude
                || self.lost_abilities(id)
                || self.activation_stopped(id)
            {
                continue;
            }
            for (c, &makes) in colors.iter().enumerate() {
                if makes && cost.colored[c] > 0 {
                    let mut k = *cost;
                    k.colored[c] -= 1;
                    k.generic += 1;
                    if self.can_pay_direct(p, &k, exclude, spell) {
                        return Some((id, k));
                    }
                }
            }
        }
        None
    }

    fn can_pay_direct(&self, p: PlayerId, cost: &ManaCost, exclude: Option<ObjId>, spell: PayFor) -> bool {
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
            if (o.loyalty_turn == self.turn && self.turn > 0) || self.activation_stopped(id) {
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
                remove_counters: 0,
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
        let hand = self.players[p].hand.iter().copied();
        for id in self.permanents(p).chain(graveyard).chain(hand) {
            let in_graveyard = self.obj(id).zone == Zone::Graveyard;
            let in_hand = self.obj(id).zone == Zone::Hand;
            for (n, a) in self
                .def_of(id)
                .activated
                .iter()
                .chain(self.kept_activated(id))
                .chain(self.granted_fight_of(id))
                .chain(self.soulbond_granted_of(id))
                .enumerate()
                .filter(|_| !self.obj(id).face_down)
            {
                if a.copy_only && n < self.def_of(id).activated.len() {
                    continue; // only a copy that keeps it has it
                }
                if self.abolished(p, id) {
                    continue;
                }
                if a.from_hand_discard != in_hand
                    || (!in_hand && a.from_graveyard != in_graveyard)
                    || (!in_graveyard && self.lost_abilities(id))
                    || self.activation_stopped(id)
                    || a.granted_to_attached
                {
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
                    || (a.once_each_turn && self.obj(id).activated_turn == Some(self.turn))
                    || self.obj(id).kind_counters < a.remove_kind_counters
                    || (a.tap_attached
                        && !self
                            .obj(id)
                            .attached_to
                            .is_some_and(|(c, z)| self.obj(c).zcc == z && self.can_tap(c)))
                    || a.tap_untapped
                        .is_some_and(|(sub, n)| self.untapped_with(p, sub, id).len() < n as usize)
                    || !sorcery_ok
                    || self.permanents(p).count() < a.min_permanents as usize
                    || (a.opponent_min_lands > 0
                        && self
                            .permanents(opponent(p))
                            .filter(|&x| self.types_of(x).land)
                            .count()
                            < a.opponent_min_lands as usize)
                    || (a.your_turn_only && p != self.active)
                    || (a.not_your_turn_only && p == self.active)
                    || (a.needs_delirium && self.graveyard_card_types(p) < 4)
                    || (a.if_card_left_graveyard && !self.graveyard_left_this_turn[p])
                    || (a.if_created_token_this_turn && self.token_created_turn[p] != self.turn)
                    || self.players[p].life < a.life
                    || !self.can_pay_for(
                        p,
                        &if a.less_per_legendary_creature {
                            let mut c = a.mana;
                            c.generic = c.generic.saturating_sub(self.legendary_creatures(p));
                            c
                        } else if a.less_per_target_color {
                            // The best target: the most colors among your creatures.
                            let best = self
                                .creatures(p)
                                .max_by_key(|&c| self.colors_of(c).iter().filter(|&&b| b).count());
                            self.color_discounted(a, best)
                        } else {
                            a.mana
                        },
                        uses_self.then_some(id),
                        PayFor::Ability(id),
                    )
                    || (!a.sacrifice_mv_x
                        && !self.trigger_can_target(
                            &StackItem {
                                kind: StackKind::Activated(n),
                                ..Game::casting(id, p)
                            },
                            &a.ability,
                        ))
                {
                    continue;
                }
                if a.sacrifice_mv_x {
                    // Sidisi: each other creature you control, with X its mana value plus 1.
                    for c in self.creatures(p).filter(|&c| c != id).collect::<Vec<_>>() {
                        let x = (self.def_of(c).cost.mana_value() + 1).min(u8::MAX as u32) as u8;
                        let item = StackItem {
                            kind: StackKind::Activated(n),
                            x,
                            ..Game::casting(id, p)
                        };
                        if self.trigger_can_target(&item, &a.ability) {
                            v.push(Action::ActivateX {
                                id,
                                n,
                                victim: Some(c),
                                x,
                            });
                        }
                    }
                    continue;
                }
                if a.tap_x_untapped_tokens {
                    let tokens = self.untapped_tokens(p, id).len().min(30) as u8;
                    for x in 0..=tokens {
                        if self.can_pay_for(p, &a.mana, uses_self.then_some(id), PayFor::Ability(id)) {
                            v.push(Action::ActivateX {
                                id,
                                n,
                                victim: None,
                                x,
                            });
                        }
                    }
                } else if a.mana.x > 0 {
                    let max = (self.untapped_sources(p).len() + self.players[p].pool.len()).min(30) as u8;
                    let victims: Vec<Option<ObjId>> = if a.sacrifice_other_creature {
                        self.creatures(p).filter(|&c| c != id).map(Some).collect()
                    } else {
                        vec![None]
                    };
                    for victim in victims {
                        for x in 0..=max {
                            if a.life_x && self.players[p].life < x as i32 {
                                continue; // "Pay X life"
                            }
                            if a.ability.targets_exactly_x {
                                // "X target creatures": X can't exceed the legal targets.
                                let f = a.ability.targets[0];
                                let item = StackItem {
                                    kind: StackKind::Activated(n),
                                    ..Game::casting(id, p)
                                };
                                let legal = self
                                    .candidates(&f, p)
                                    .into_iter()
                                    .filter(|&t| self.slot_ok(&item, &f, t))
                                    .count();
                                if x as usize > legal {
                                    continue;
                                }
                            }
                            if self.can_pay_for(
                                p,
                                &a.mana.with_x(x),
                                uses_self.then_some(id),
                                PayFor::Ability(id),
                            ) {
                                v.push(Action::ActivateX { id, n, victim, x });
                            }
                        }
                    }
                } else if a.exile_creature_card {
                    v.extend(
                        self.players[p]
                            .graveyard
                            .iter()
                            .copied()
                            .filter(|&c| c != id && self.def_of(c).types.creature)
                            .map(|c| Action::ActivateWith(id, n, c)),
                    );
                } else if a.discard_any_card {
                    v.extend(
                        self.players[p]
                            .hand
                            .iter()
                            .copied()
                            .map(|c| Action::ActivateWith(id, n, c)),
                    );
                } else if a.sacrifice_two_others {
                    let others: Vec<ObjId> = self
                        .creatures(p)
                        .filter(|&c| c != id && !self.cant_be_sacrificed(c))
                        .collect();
                    if others.len() >= 2 {
                        v.extend(others.into_iter().map(|c| Action::ActivateWith(id, n, c)));
                    }
                } else if a.discard_creature_card {
                    v.extend(
                        self.players[p]
                            .hand
                            .iter()
                            .copied()
                            .filter(|&c| self.def_of(c).types.creature)
                            .map(|c| Action::ActivateWith(id, n, c)),
                    );
                } else if a.crew > 0 {
                    // Crew: tap an untapped creature with enough power (summoning sickness doesn't matter).
                    v.extend(
                        self.creatures(p)
                            .filter(|&c| c != id && !self.obj(c).tapped && self.power(c) >= a.crew as i32)
                            .map(|c| Action::ActivateWith(id, n, c)),
                    );
                } else if a.discard_cost {
                    v.extend(
                        self.players[p]
                            .hand
                            .iter()
                            .copied()
                            .filter(|&c| c != id)
                            .map(|c| Action::ActivateWith(id, n, c)),
                    );
                } else if a.sacrifice_other_creature {
                    v.extend(
                        self.creatures(p)
                            .filter(|&c| c != id && !self.cant_be_sacrificed(c))
                            .map(|c| Action::ActivateWith(id, n, c)),
                    );
                } else if let Some(sub) = a.sacrifice_subtype {
                    // "Sacrifice a [subtype]": the source too, if it has the type.
                    let victims: Vec<ObjId> = self
                        .permanents(p)
                        .filter(|&c| self.has_subtype(c, sub) && !self.cant_be_sacrificed(c))
                        .filter(|&c| !(a.sacrifice_another && c == id))
                        .collect();
                    v.extend(
                        victims
                            .into_iter()
                            .filter(|&c| {
                                uses_self || self.can_pay_for(p, &a.mana, Some(c), PayFor::Ability(id))
                            })
                            .map(|c| Action::ActivateWith(id, n, c)),
                    );
                } else if a.sacrifice_creature {
                    v.extend(
                        self.creatures(p)
                            .filter(|&c| !self.cant_be_sacrificed(c))
                            .map(|c| Action::ActivateWith(id, n, c)),
                    );
                } else if a.sacrifice_token {
                    // "Sacrifice a token": any token you control, kept out of the mana payment.
                    let tokens: Vec<ObjId> = self
                        .permanents(p)
                        .filter(|&c| self.is_token(c) && !self.cant_be_sacrificed(c))
                        .collect();
                    v.extend(
                        tokens
                            .into_iter()
                            .filter(|&c| {
                                uses_self || self.can_pay_for(p, &a.mana, Some(c), PayFor::Ability(id))
                            })
                            .map(|c| Action::ActivateWith(id, n, c)),
                    );
                } else {
                    v.push(Action::Activate(id, n));
                }
            }
        }
        // A face-down creature card may be turned face up for its mana cost (CR 701.40b).
        for id in self.permanents(p).collect::<Vec<_>>() {
            let o = self.obj(id);
            if o.face_down && self.defs[o.def].types.creature {
                let cost = self.defs[o.def].cost;
                if cost.x == 0 && self.can_pay_for(p, &cost, None, PayFor::Ability(id)) {
                    v.push(Action::Activate(id, 0));
                }
            }
        }
        v
    }

    /// CR 602.2: pay the costs, then the ability goes on the stack (targets chosen as a decision).
    /// CR 602.2: announce, choose targets, then pay the costs.
    fn activate(&mut self, p: PlayerId, id: ObjId, n: usize, victim: Option<ObjId>) {
        if self.obj(id).face_down {
            // CR 701.40b: turning a manifested creature card face up for its mana cost is a special
            // action; it doesn't use the stack.
            let cost = self.defs[self.obj(id).def].cost;
            self.pay_mana_for(p, &cost, None, PayFor::Ability(id));
            self.obj_mut(id).face_down = false;
            self.passes = 0;
            self.give_priority(p);
            return;
        }
        let granted = self.activated_of(id, n).granted_to_attached;
        let extra = if granted {
            self.obj(id).attached_to.map(|(c, _)| c)
        } else {
            None
        };
        let item = StackItem {
            source: id,
            controller: p,
            kind: StackKind::Activated(n),
            targets: Vec::new(),
            kicked: false,
            flashback: false,
            extra,
            mode: None,
            amount: 0,
            alt: false,
            x: 0,
            division: Vec::new(),
        };
        let mut item = item;
        if let (true, Some(v)) = (self.activated_of(id, n).amount_from_victim_power, victim) {
            item.amount = self.power(v); // Greater Good: the power as it was sacrificed
        }
        self.passes = 0;
        if self.activated_of(id, n).ability.targets.is_empty() {
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
                remove_counters: 0,
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
        let a = &self.activated_of(id, n).ability;
        if a.targets.is_empty() || (a.targets_exactly_x && x == 0) {
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
                remove_counters: 0,
            });
            self.pending = Pending::ChooseTarget(p);
        }
    }

    fn pay_activation(&mut self, p: PlayerId, id: ObjId, n: usize, victim: Option<ObjId>, x: u8) {
        let a = self.activated_of(id, n).clone();
        let uses_self = a.tap || a.sacrifice_self;
        // A sacrificed token (a Treasure) can't also pay the mana.
        let exclude = if uses_self {
            Some(id)
        } else {
            victim.filter(|_| a.sacrifice_token || a.sacrifice_subtype.is_some())
        };
        // Mana first: a creature can tap for mana and then be sacrificed for the same cost
        // (CR 602.2b). The fuzzer found the reverse order failing.
        let mut cost = a.mana.with_x(x);
        if a.less_per_legendary_creature {
            cost.generic = cost.generic.saturating_sub(self.legendary_creatures(p));
        }
        if a.less_per_target_color {
            cost = a.mana;
            cost.generic = cost.generic.saturating_sub(x);
        }
        self.pay_mana_for(p, &cost, exclude, PayFor::Ability(id));
        if let Some(v) = victim {
            if a.crew > 0 {
                self.become_tapped(v); // crew taps, it doesn't sacrifice
            } else if a.sacrifice_two_others {
                if self.obj(v).zone == Zone::Battlefield {
                    self.move_to(v, Zone::Graveyard);
                }
                let second = self
                    .creatures(p)
                    .filter(|&c| c != id && !self.cant_be_sacrificed(c))
                    .min_by_key(|&c| self.power(c));
                if let Some(c) = second {
                    self.move_to(c, Zone::Graveyard);
                }
            } else if a.discard_creature_card || a.discard_any_card {
                if self.obj(v).zone == Zone::Hand {
                    self.move_to(v, Zone::Graveyard); // discarded
                }
            } else if a.exile_creature_card {
                if self.obj(v).zone == Zone::Graveyard {
                    self.move_to(v, Zone::Exile);
                }
            } else if a.discard_cost {
                if self.obj(v).zone == Zone::Hand {
                    self.move_to(v, Zone::Graveyard); // discarded
                }
            } else if self.obj(v).zone == Zone::Battlefield {
                self.move_to(v, Zone::Graveyard);
            }
        }
        if a.minus_counter_cost && self.obj(id).zone == Zone::Battlefield {
            self.obj_mut(id).counters -= 1; // Devoted Druid: a -1/-1 counter
        }
        if a.tap_x_untapped_tokens {
            // The payer picks which tokens to tap: the first X untapped ones.
            for t in self.untapped_tokens(p, id).into_iter().take(x as usize) {
                self.become_tapped(t);
            }
        }
        if a.discard_hand {
            for c in self.players[p].hand.clone() {
                self.move_to(c, Zone::Graveyard);
            }
        }
        if a.life > 0 {
            self.lose_life(p, a.life);
        }
        if a.life_x && x > 0 {
            self.lose_life(p, x as i32);
        }
        if a.tap {
            self.become_tapped(id);
        }
        self.obj_mut(id).incubation -= a.remove_incubation;
        if a.once {
            self.obj_mut(id).used_once = true;
        }
        if a.once_each_turn {
            self.obj_mut(id).activated_turn = Some(self.turn);
        }
        self.obj_mut(id).kind_counters -= a.remove_kind_counters;
        if a.tap_attached {
            if let Some((c, _)) = self.obj(id).attached_to {
                self.become_tapped(c);
            }
        }
        if let Some((sub, n)) = a.tap_untapped {
            for c in self.untapped_with(p, sub, id).into_iter().take(n as usize) {
                self.become_tapped(c);
            }
        }
        if a.sacrifice_self {
            self.move_to(id, Zone::Graveyard);
        }
        if a.exile_self && self.obj(id).zone == Zone::Graveyard {
            self.move_to(id, Zone::Exile); // renew: "Exile this card from your graveyard"
        } else if a.exile_self && self.obj(id).zone == Zone::Battlefield {
            self.move_to(id, Zone::Exile); // "Exile this creature"
        }
        if a.from_hand_discard && self.obj(id).zone == Zone::Hand {
            self.move_to(id, Zone::Graveyard); // cycling: "Discard this card"
            self.fire_source(id, When::YouCycleThis);
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
            .filter(|(i, _)| self.types_of(ObjId(*i as u32)).creature)
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
                    if self.permanents(p).any(|x| self.colors_of(x)[c]) {
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
            // "an amount of {G} equal to this creature's power"
            let n = if self.def_of(id).mana_equal_power {
                self.power(id).max(0) as usize
            } else if self.def_of(id).mana_per_creature {
                self.creatures(p).count() // Circle of Dreams Druid
            } else {
                n
            };
            let n = n.max(self.def_of(id).mana_amount as usize); // Sol Ring: {C}{C}
                                                                 // Vorinclex: "Whenever you tap a land for mana, add one mana of any type that land produced."
            let n = if self.types_of(id).land && self.permanents(p).any(|x| self.def_of(x).vorinclex) {
                n * 2
            } else {
                n
            };
            // Serra's Sanctum, Sanctum Weaver: one mana for each enchantment you control.
            let n = if self.def_of(id).mana_per_enchantment {
                self.permanents(p)
                    .filter(|&x| self.types_of(x).enchantment)
                    .count()
            } else {
                n
            };
            let fero = self.def_of(id).ferocious_mana_amount as usize;
            let n = if fero > 0 && self.creatures(p).any(|c| self.power(c) >= 4) {
                n.max(fero) // Ferocious: "{T}: Add {G}{G}{G}{G}"
            } else {
                n
            };
            out.extend(std::iter::repeat_n((id, m), n));
            // Nissa, Who Shakes the World: a Forest tapped for mana adds an additional {G}.
            if self.has_subtype(id, "Forest") {
                let nissas = self
                    .permanents(p)
                    .filter(|&x| self.def_of(x).forests_add_g && !self.lost_abilities(x))
                    .count();
                out.extend(std::iter::repeat_n(
                    (id, ManaSource::of(&[crate::Color::Green])),
                    nissas,
                ));
            }
        }
        out
    }

    /// Grand Abolisher: during its controller's turn, `p` (an opponent) can't activate abilities of
    /// artifacts, creatures, or enchantments.
    fn abolished(&self, p: PlayerId, id: ObjId) -> bool {
        if p == self.active {
            return false;
        }
        let t = self.types_of(id);
        (t.artifact || t.creature || t.enchantment)
            && self
                .permanents(self.active)
                .any(|x| self.def_of(x).opponents_cant_activate_your_turn && !self.lost_abilities(x))
    }

    fn single_sources(&self, p: PlayerId) -> Vec<(ObjId, ManaSource)> {
        self.permanents(p)
            .filter(|&id| self.can_tap(id) && !self.activation_stopped(id) && !self.abolished(p, id))
            .filter_map(|id| {
                if self.lost_abilities(id) {
                    // Imprisoned in the Moon grants "{T}: Add {C}"; other ability loss leaves none.
                    let land = self.attached_static(id).is_some_and(|s| s.becomes_land);
                    return land.then(|| {
                        (
                            id,
                            ManaSource {
                                colorless: true,
                                ..ManaSource::default()
                            },
                        )
                    });
                }
                let d = self.def_of(id);
                let need = d.mana_only_with_lands as usize;
                if need > 0 && self.permanents(p).filter(|&x| self.types_of(x).land).count() < need {
                    return None; // Temple of the False God
                }
                if d.mana_taps_creature && self.relic_partner(p, None).is_none() {
                    return None; // "Tap an untapped creature you control"
                }
                if d.mana_only_if_you_control
                    .is_some_and(|sub| !self.permanents(p).any(|x| self.has_subtype(x, sub)))
                {
                    return None; // "Activate only if you control a Dragon"
                }
                if d.vivid_mana {
                    return Some((id, ManaSource::default()));
                }
                if d.mana_commander_identity {
                    // CR 903.4f: no commander (or a colorless one) means no mana.
                    let mut identity = self.commanders[p].map(|c| self.def_of(c).color_identity)?;
                    if let Some(c2) = self.partners[p] {
                        for (i, has) in identity.iter_mut().zip(self.def_of(c2).color_identity) {
                            *i |= has;
                        }
                    }
                    let colors: Vec<crate::Color> = crate::Color::ALL
                        .into_iter()
                        .filter(|&c| identity[c as usize])
                        .collect();
                    return (!colors.is_empty()).then(|| (id, ManaSource::of(&colors)));
                }
                if d.plaza_mana {
                    // {C}, or a color among legendary permanents you control.
                    let mut m = ManaSource {
                        colorless: true,
                        ..ManaSource::default()
                    };
                    for l in self.permanents(p).filter(|&l| self.types_of(l).legendary) {
                        for (c, has) in m.colors.iter_mut().zip(self.colors_of(l)) {
                            *c |= has;
                        }
                    }
                    return Some((id, m));
                }
                if d.mana_legendary_colors {
                    let mut could = [false; 5];
                    for l in self.permanents(p).filter(|&l| {
                        let t = self.types_of(l);
                        t.legendary && (t.creature || t.planeswalker)
                    }) {
                        for (c, has) in could.iter_mut().zip(self.colors_of(l)) {
                            *c |= has;
                        }
                    }
                    let colors: Vec<crate::Color> = crate::Color::ALL
                        .into_iter()
                        .filter(|&c| could[c as usize])
                        .collect();
                    return (!colors.is_empty()).then(|| (id, ManaSource::of(&colors)));
                }
                if d.mana_opponent_land_colors {
                    let mut could = [false; 5];
                    for l in self.permanents(opponent(p)).filter(|&l| self.types_of(l).land) {
                        if let Some(m) = self.def_of(l).mana {
                            for (c, has) in could.iter_mut().zip(m.colors) {
                                *c |= has;
                            }
                        }
                    }
                    let colors: Vec<crate::Color> = crate::Color::ALL
                        .into_iter()
                        .filter(|&c| could[c as usize])
                        .collect();
                    return (!colors.is_empty()).then(|| (id, ManaSource::of(&colors)));
                }
                if d.mana_of_chosen_color {
                    let c = self.obj(id).chosen_color?;
                    return Some((id, ManaSource::of(&[crate::Color::ALL[c as usize]])));
                }
                if let Some(fixed) = d.thriving_color {
                    let mut colors = vec![fixed];
                    colors.extend(self.obj(id).chosen_color.map(|c| crate::Color::ALL[c as usize]));
                    return Some((id, ManaSource::of(&colors)));
                }
                if d.mana_own_land_colors {
                    // Reflecting Pool: what its controller's other lands could produce.
                    let mut m = ManaSource::default();
                    for l in self.permanents(p).filter(|&l| l != id && self.types_of(l).land) {
                        if let Some(lm) = self.def_of(l).mana {
                            for (c, has) in m.colors.iter_mut().zip(lm.colors) {
                                *c |= has;
                            }
                            m.colorless |= lm.colorless;
                        }
                    }
                    return (m.colorless || m.colors.iter().any(|&c| c)).then_some((id, m));
                }
                if let Some((extra, subs)) = d.extra_color_if_you_control {
                    let mut m = d.mana.unwrap_or_default();
                    if self
                        .permanents(p)
                        .any(|x| subs.iter().any(|s| self.has_subtype(x, s)))
                    {
                        m.colors[extra as usize] = true; // Thornspire Verge
                    }
                    return Some((id, m));
                }
                if d.colored_mana_only_if_you_control
                    .is_some_and(|sub| !self.permanents(p).any(|x| self.has_subtype(x, sub)))
                {
                    // Tainted Wood: only its {C} without a Swamp.
                    return d.mana.map(|m| {
                        (
                            id,
                            ManaSource {
                                colorless: m.colorless,
                                ..ManaSource::default()
                            },
                        )
                    });
                }
                if self.every_basic_type(id) {
                    // Every basic land type: each one's mana ability (CR 305.6).
                    let mut m = d.mana.unwrap_or_default();
                    m.colors = [true; 5];
                    return Some((id, m));
                }
                if d.mana.is_none()
                    && self.types_of(id).creature
                    && self.has_counter(id)
                    && self
                        .permanents(p)
                        .any(|x| self.def_of(x).counter_creatures_tap_for_green && !self.lost_abilities(x))
                {
                    return Some((id, ManaSource::of(&[crate::Color::Green]))); // Rishkar
                }
                let forests = self.types_of(id).land
                    && self
                        .battlefield
                        .iter()
                        .any(|&x| self.def_of(x).lands_are_forests && !self.lost_abilities(x));
                d.mana.map(|mut m| {
                    if forests {
                        m.colors[crate::Color::Green as usize] = true; // Yavimaya: "{T}: Add {G}"
                    }
                    (id, m)
                })
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
                && (!k.has(Keywords::DEFENDER) || o.defender_ignored_turn == self.turn)
                && (o.since_turn < self.turn || k.has(Keywords::HASTE))
                && !declared.contains(&id)
                && !(self.def_of(id).cant_attack_twice_per_turn
                    && self.attacked_creatures_this_turn.contains(&id))
                && (!self.def_of(id).fights_only_with_blessing || self.city_blessing[o.controller])
        })
    }

    /// Whether `blocker` may block `attacker`, ignoring menace (CR 509.1b, 702.9b, 702.17b).
    fn can_block(&self, blocker: ObjId, attacker: ObjId) -> bool {
        if self.protected_from(attacker, blocker) {
            return false; // CR 702.16f
        }
        if self.def_of(blocker).cant_block
            || (self.def_of(blocker).fights_only_with_blessing
                && !self.city_blessing[self.obj(blocker).controller])
            || self.is_decayed(blocker)
            || self.keywords(blocker).has(Keywords::CANT_BLOCK)
            || self.keywords(attacker).has(Keywords::UNBLOCKABLE)
            || (self.keywords(attacker).has(Keywords::FORESTWALK)
                && self
                    .permanents(self.obj(blocker).controller)
                    .any(|x| self.types_of(x).land && self.has_subtype(x, "Forest")))
            || (self.keywords(attacker).has(Keywords::ISLANDWALK)
                && self
                    .permanents(self.obj(blocker).controller)
                    .any(|x| self.types_of(x).land && self.has_subtype(x, "Island")))
            || (self
                .permanents(self.obj(attacker).controller)
                .any(|x| self.def_of(x).small_creatures_unblockable && !self.lost_abilities(x))
                && (self.power(attacker) <= 1 || self.toughness(attacker) <= 1))
            || self
                .def_of(attacker)
                .not_blocked_by
                .is_some_and(|sub| self.has_subtype(blocker, sub))
            || (self.def_of(attacker).not_blocked_by_lesser_power
                && !self.lost_abilities(attacker)
                && self.power(blocker) < self.power(attacker))
            || self
                .def_of(attacker)
                .not_blocked_by_power_at_most
                .is_some_and(|n| !self.lost_abilities(attacker) && self.power(blocker) <= n)
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
                        && self.types_of(b).creature
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
        let menace_ok = combat
            .attackers
            .iter()
            .all(|&a| !self.keywords(a).has(Keywords::MENACE) || combat.blockers_of(a).count() != 1);
        // "Must be blocked if able" (Anzrag): some untapped creature that could block it does.
        let turn = self.turn;
        let lure_ok = combat.attackers.iter().all(|&a| {
            let lured = self
                .must_be_blocked
                .iter()
                .any(|&(c, z, t)| c == a && z == self.obj(a).zcc && t == turn);
            !lured
                || combat.blockers_of(a).count() > 0
                || !self
                    .creatures(opponent(self.obj(a).controller))
                    .any(|b| !self.obj(b).tapped && self.can_block(b, a))
        });
        menace_ok && lure_ok
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
        // In a free-cast window, casting works as it does with priority.
        let pending = match (self.pending, action) {
            (Pending::FreeCast(p), a) if a != Action::PassPriority => Pending::Priority(p),
            (pending, _) => pending,
        };
        match (pending, action) {
            (Pending::FreeCast(_), Action::PassPriority) => self.close_free_casts(),
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
            (Pending::Priority(p), Action::TapForMana(id, c)) => self.tap_for_mana(p, id, c),
            (Pending::Priority(p), Action::PlayLand(c)) => {
                if self.obj(c).zone == Zone::Graveyard && !self.siege_graveyard_lands(p) {
                    self.graveyard_types_used[p][3] = true; // Muldrotha's land
                }
                if self.def_of(c).modal_back && !self.def_of(c).types.land {
                    self.obj_mut(c).transformed = true; // its land face (CR 712.12)
                }
                self.move_to(c, Zone::Battlefield);
                self.players[p].lands_played += 1;
                self.passes = 0;
                for q in 0..2 {
                    self.fire(q, When::PlayerPlaysLand, None); // Horn of Greed
                }
                // "As this land enters, choose a creature type" (Secluded Courtyard).
                let options = if self.def_of(c).choose_creature_type {
                    self.creature_types_known_to(p)
                } else {
                    Vec::new()
                };
                if self.def_of(c).choose_color {
                    // "As it enters, choose a color" (the Thriving lands).
                    self.ask(p, Decision::ChooseColor { card: c });
                } else if options.is_empty() {
                    self.give_priority(p);
                } else {
                    self.ask(p, Decision::ChooseCreatureType { card: c, options });
                }
            }
            (Pending::Priority(p), Action::Cast(c)) => self.cast(p, c, CastMode::default()),
            (Pending::Priority(p), Action::CastFromGraveyardAs(c, t)) => {
                self.graveyard_types_used[p][t as usize] = true;
                self.cast(p, c, CastMode::default());
            }
            (Pending::Priority(p), Action::CastFromGraveyard(c)) => {
                self.removal_to_cast = self.def_of(c).graveyard_cast_remove_counters;
                let finality = self.def_of(c).cast_from_graveyard_finality;
                self.cast(p, c, CastMode::default());
                if finality {
                    self.obj_mut(c).enters_with_finality = true;
                }
            }
            (Pending::RemoveCounters(_), Action::Pick(c)) => {
                // Stun counters first (losing one helps you), then +1/+1, then revival.
                let o = self.obj_mut(c);
                if o.stun > 0 {
                    o.stun -= 1;
                } else if o.counters > 0 {
                    o.counters -= 1;
                } else {
                    o.revival -= 1;
                }
                let ch = self.choosing.as_mut().expect("casting");
                ch.remove_counters -= 1;
                if ch.remove_counters == 0 {
                    let ch = self.choosing.take().expect("casting");
                    self.finish_cast(ch.item);
                }
            }
            (Pending::Priority(p), Action::CastKicked(c)) => self.cast(
                p,
                c,
                CastMode {
                    kicked: true,
                    ..CastMode::default()
                },
            ),
            (Pending::Priority(p), Action::CastWith(c, x)) => {
                // Nissa, Worldsoul Speaker: casting "with" her pays energy instead of mana.
                let energy = self.def_of(x).permanents_for_energy > 0
                    && self.obj(x).zone == Zone::Battlefield
                    && self.def_of(c).extra_cost.is_none();
                let mode = CastMode {
                    alt: energy,
                    ..CastMode::with(x)
                };
                self.cast(p, c, mode)
            }
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
            (Pending::Priority(p), Action::CastHarmonize(c, cr)) => self.cast_maybe_x(
                p,
                c,
                CastMode {
                    flashback: true,
                    extra: Some(cr),
                    ..CastMode::default()
                },
            ),
            (Pending::Priority(p), Action::CastOmen(c)) => self.cast_maybe_x(
                p,
                c,
                CastMode {
                    mode: Some(OMEN_MODE),
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
                    self.become_tapped(c);
                }
                self.combat.as_mut().expect("in combat").attackers.push(c);
                self.attacked_this_turn[self.active] = true;
                let turn = self.turn;
                let (t, n) = self.obj(c).attacks_turn;
                self.obj_mut(c).attacks_turn = (turn, if t == turn { n.saturating_add(1) } else { 1 });
                if !self.attacked_creatures_this_turn.contains(&c) {
                    self.attacked_creatures_this_turn.push(c);
                }
                if self.is_decayed(c) {
                    self.obj_mut(c).sacrifice_end_combat = true; // decayed (CR 702.147)
                }
                let before = self.triggered.len();
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
                // "Whenever equipped creature attacks".
                let zcc = self.obj(c).zcc;
                let gear: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&e| self.obj(e).attached_to == Some((c, zcc)))
                    .collect();
                for e in gear {
                    self.fire_source(e, When::EquippedAttacks);
                }
                self.attack_triggers_again(before);
            }
            (Pending::DeclareAttackers(_), Action::FinishAttacks) => {
                let before = self.triggered.len();
                if self.combat.as_ref().is_some_and(|c| !c.attackers.is_empty()) {
                    self.fire(self.active, When::YouAttack, None);
                    let n = self.combat.as_ref().map_or(0, |c| c.attackers.len());
                    for _ in 0..n {
                        self.fire(self.active, When::CreatureYouControlAttacks, None);
                    }
                    let (active, turn) = (self.active, self.turn);
                    for (source, n, controller, t) in self.delayed_on_attack.clone() {
                        if controller == active && t == turn {
                            self.triggered.push(StackItem {
                                kind: StackKind::Trigger(n),
                                ..Game::casting(source, controller)
                            });
                        }
                    }
                    // "When you next attack this turn": once.
                    let (now, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.delayed_on_next_attack)
                        .into_iter()
                        .partition(|&(_, _, controller, t)| controller == active && t == turn);
                    self.delayed_on_next_attack = later;
                    for (source, n, controller, _) in now {
                        self.triggered.push(StackItem {
                            kind: StackKind::Trigger(n),
                            ..Game::casting(source, controller)
                        });
                    }
                }
                let count = self.combat.as_ref().map_or(0, |c| c.attackers.len()) as u8;
                for n in 1..=count {
                    self.fire(self.active, When::YouAttackWithAtLeast(n), None);
                }
                if count >= 2 {
                    self.fire(opponent(self.active), When::OpponentAttacksWithAtLeastTwo, None);
                }
                self.attack_triggers_again(before);
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
                let blockers: Vec<ObjId> = self
                    .combat
                    .as_ref()
                    .map(|c| c.blocks.iter().map(|b| b.0).collect())
                    .unwrap_or_default();
                let mut seen = Vec::new();
                for b in blockers {
                    if !seen.contains(&b) {
                        seen.push(b);
                        self.fire_source(b, When::Blocks);
                    }
                }
                // CR 509.3c: "whenever this creature becomes blocked".
                let blocked: Vec<ObjId> = self
                    .combat
                    .as_ref()
                    .map(|c| c.attackers.iter().copied().filter(|&a| c.is_blocked(a)).collect())
                    .unwrap_or_default();
                for a in blocked {
                    self.fire_source(a, When::BecomesBlocked);
                }
                self.passes = 0;
                self.give_priority(self.active);
            }
            (Pending::Discard(p), Action::Discard(c)) => {
                self.move_to(c, Zone::Graveyard);
                if self.players[p].hand.len() <= self.max_hand(p) {
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
            self.triggered_scanned = 0;
            self.turn = 0;
            self.active = opponent(self.first);
            self.begin_next_turn();
        }
    }

    /// CR 601.2: move to the stack, choose targets, pay, then the caster gets priority again
    /// (CR 117.3c).
    /// Cast with `mode`, first asking for X when that mode's cost has {X} (an Omen or harmonize
    /// cast: no action carries X there, so it's a decision answered with `Amount`).
    fn cast_maybe_x(&mut self, p: PlayerId, card: ObjId, mode: CastMode) {
        let def = self.def_of(card);
        let has_x = if mode.mode == Some(OMEN_MODE) {
            def.omen.as_ref().is_some_and(|o| o.cost.x > 0)
        } else {
            mode.flashback && def.flashback.is_some_and(|f| f.x > 0)
        };
        if !has_x {
            return self.cast(p, card, mode);
        }
        let max = (self.untapped_sources(p).len() + self.players[p].pool.len()).min(30) as u8;
        self.ask(p, Decision::ChooseCastX { card, mode, max });
    }

    /// `id` becomes a copy of card definition `of` (CR 707.2: the copiable values are the printed
    /// card). Its own card is kept, to come back when it leaves the battlefield.
    /// What a clone entering as a copy may copy: a creature, or with `enter_as_copy_nonland` any
    /// nonland permanent.
    fn copy_option(&self, clone: ObjId, x: ObjId) -> bool {
        let t = self.types_of(x);
        let d = self.def_of(clone);
        if d.copy_artifact {
            return t.artifact || t.creature;
        }
        if d.copy_only_yours {
            return t.creature && self.obj(x).controller == self.obj(clone).controller;
        }
        if self.def_of(clone).enter_as_copy_nonland {
            !t.land
        } else {
            t.creature
        }
    }

    fn become_copy(&mut self, id: ObjId, of: usize) {
        let o = self.obj_mut(id);
        if o.printed_def.is_none() {
            o.printed_def = Some(o.def);
        }
        o.def = of;
    }

    /// Graveyard cards delve could exile for `card` (Teval: "Spells you cast have delve").
    fn delve_available(&self, p: PlayerId, card: ObjId) -> u8 {
        let teval = self
            .permanents(p)
            .any(|x| self.def_of(x).spells_have_delve && !self.lost_abilities(x));
        if !teval {
            return 0;
        }
        self.players[p]
            .graveyard
            .iter()
            .filter(|&&x| x != card)
            .count()
            .min(255) as u8
    }

    /// Cast, first asking how many cards to delve when delve could pay part of the cost.
    fn cast(&mut self, p: PlayerId, card: ObjId, mode: CastMode) {
        let avail = self.delve_available(p, card);
        if avail > 0 && self.obj(card).delve == 0 {
            let generic = self.spell_cost(p, card, mode, &[]).generic;
            let max = avail.min(generic);
            if max > 0 {
                self.ask(p, Decision::ChooseDelve { card, mode, max });
                return;
            }
        }
        self.cast_now(p, card, mode);
    }

    /// Exile the `n` graveyard cards delve paid with: cards with no graveyard use of their own
    /// first (no flashback, harmonize or graveyard ability), oldest first. The engine chooses
    /// which; only how many is the player's choice.
    fn exile_for_delve(&mut self, p: PlayerId, card: ObjId, n: u8) {
        let mut cards: Vec<ObjId> = self.players[p]
            .graveyard
            .iter()
            .copied()
            .filter(|&x| x != card)
            .collect();
        cards.sort_by_key(|&x| {
            let d = self.def_of(x);
            d.flashback.is_some() || d.harmonize || d.activated.iter().any(|a| a.from_graveyard)
        });
        for x in cards.into_iter().take(n as usize) {
            self.move_to(x, Zone::Exile);
        }
    }

    fn cast_now(&mut self, p: PlayerId, card: ObjId, mode: CastMode) {
        // Offspring is paid "as you cast" (CR 702.175a): fixed now, even if Zinnia leaves.
        let offspring = mode.kicked && self.offspring_cost(p, card).is_some();
        self.obj_mut(card).offspring_paid = offspring;
        self.obj_mut(card).from_hand = self.obj(card).zone == Zone::Hand;
        let from_command = self.obj(card).zone == Zone::Command;
        self.obj_mut(card).from_command = from_command;
        if from_command {
            // CR 903.8: {2} more for each earlier cast from the command zone.
            let owner = self.obj(card).owner;
            let casts = self.obj(card).command_zone_casts;
            self.obj_mut(card).commander_tax = casts.saturating_mul(2);
            self.obj_mut(card).command_zone_casts = casts.saturating_add(1);
            self.commander_casts[owner] = self.commander_casts[owner].saturating_add(1);
        }
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
            Some(OMEN_MODE) => def.omen.as_ref().map(|o| &o.spell),
            Some(m) => Some(&def.modes[m as usize]),
            None if mode.kicked && def.kicked_spell.is_some() => def.kicked_spell.as_ref(),
            None => def.spell.as_ref(),
        };
        let has_targets = ability.is_some_and(|s| !s.targets.is_empty());
        let removal = std::mem::take(&mut self.removal_to_cast);
        if removal > 0 {
            // Quilled Greatwurm has no targets, so costs come next (CR 601.2h).
            debug_assert!(!has_targets);
            self.choosing = Some(Choosing {
                item,
                then: p,
                cast: true,
                pay: None,
                loyalty: None,
                remove_counters: removal,
            });
            self.pending = Pending::RemoveCounters(p);
        } else if has_targets {
            self.choosing = Some(Choosing {
                item,
                then: p,
                cast: true,
                pay: None,
                loyalty: None,
                remove_counters: 0,
            });
            self.pending = Pending::ChooseTarget(p);
        } else {
            self.finish_cast(item);
        }
    }

    fn finish_cast(&mut self, item: StackItem) {
        let (p, card) = (item.controller, item.source);
        let flashback_cast = item.flashback;
        // Terror of the Peaks: "cost an additional 3 life" for each of its controller's such targets.
        let tax: i32 = item
            .targets
            .iter()
            .filter_map(|t| match *t {
                Chosen::Object(x, _)
                    if self.obj(x).zone == Zone::Battlefield && self.obj(x).controller != p =>
                {
                    Some(self.def_of(x).targeted_life_tax)
                }
                _ => None,
            })
            .sum();
        if tax > 0 {
            self.lose_life(p, tax);
        }
        if self.turn > 0 && self.obj(card).conduit_turn == self.turn {
            self.obj_mut(card).conduit_turn = 0;
            self.no_more_spells_turn[p] = self.turn; // Conduit of Worlds
        }
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
        // Harmonize: the creature tapped to lower the cost can't also tap for mana to pay it.
        let harmonized = item
            .extra
            .filter(|_| item.flashback && (self.def_of(card).harmonize || self.obj(card).harmonize_eot));
        self.snow_paid = 0;
        self.pay_mana_for(p, &cost, harmonized, pay_for_spell(card, item.mode));
        let mut item = item;
        if self.def_of(card).x_is_snow_spent {
            item.x = self.snow_paid.min(u8::MAX as u32) as u8;
        }
        // Buyback—sacrifice N permanents of a subtype (Walk the Aeons): tapped ones first.
        if let (true, Some((sub, n))) = (item.kicked, self.def_of(card).buyback_sacrifice) {
            let mut victims: Vec<ObjId> = self
                .permanents(p)
                .filter(|&x| self.has_subtype(x, sub) && !self.cant_be_sacrificed(x))
                .collect();
            victims.sort_by_key(|&x| !self.obj(x).tapped);
            for v in victims.into_iter().take(n as usize) {
                self.move_to(v, Zone::Graveyard);
            }
        }
        let energy_by = item
            .extra
            .filter(|&x| item.alt && self.def_of(x).permanents_for_energy > 0);
        if let Some(x) = energy_by {
            let n = self.def_of(x).permanents_for_energy as u32;
            self.players[p].energy = self.players[p].energy.saturating_sub(n);
        } else if item.alt && self.def_of(card).alt_cost.is_none() {
            let pips: u8 = self.def_of(card).phyrexian.iter().sum();
            // Phyrexian pips paid with life, or Defiler of Vigor's optional additional cost.
            self.lose_life(p, if pips > 0 { 2 * pips as i32 } else { 2 });
        }
        if self.types_of(card).is_permanent() && self.def_of(card).colors[4] {
            self.fire(p, When::YouCastGreenPermanent, None);
        }
        // Expend: "as you spend your Nth total mana to cast spells during a turn".
        let spent = cost.mana_value();
        let turn = self.turn;
        let (t, before) = self.mana_spent_on_spells[p];
        let before = if t == turn { before } else { 0 };
        self.mana_spent_on_spells[p] = (turn, before + spent);
        for k in [4u8, 8] {
            if before < k as u32 && before + spent >= k as u32 {
                self.fire(p, When::Expend(k), None);
            }
        }
        if self.types_of(card).enchantment && self.obj(card).from_hand {
            let before = self.triggered.len();
            self.fire(p, When::YouCastEnchantmentFromHand, None);
            for t in &mut self.triggered[before..] {
                t.extra = Some(card);
            }
        }
        let delved = std::mem::take(&mut self.obj_mut(card).delve);
        if delved > 0 {
            self.exile_for_delve(p, card, delved);
        }
        if let Some(x) = item.extra.filter(|_| energy_by.is_none()) {
            if harmonized.is_some() {
                self.become_tapped(x); // harmonize taps the creature; it isn't sacrificed
            } else if self.def_of(card).pitch_not_your_turn.is_some() && !item.flashback {
                self.move_to(x, Zone::Exile); // Force of Vigor: the pitched card is exiled
            } else if self.obj(x).zone != Zone::Graveyard {
                self.move_to(x, Zone::Graveyard);
            }
        }
        let noncreature = !self.types_of(card).creature;
        if p != self.active {
            self.fire(p, When::YouCastOnOpponentsTurn, None);
        }
        self.fire(opponent(p), When::OpponentCasts, None);
        if noncreature {
            self.fire(opponent(p), When::OpponentCastsNoncreature, None);
            // Pollywog Prodigy: only for a spell with mana value less than its power.
            let cost = self.def_of(card).cost;
            let mv = (cost.mana_value() + cost.x as u32 * item.x as u32) as i32;
            let before = self.triggered.len();
            self.fire(opponent(p), When::OpponentCastsNoncreatureMvBelowPower, None);
            let new = self.triggered.split_off(before);
            for t in new {
                if mv < self.power(t.source) {
                    self.triggered.push(t);
                }
            }
        }
        if self.types_of(card).instant || self.types_of(card).sorcery {
            self.fire(opponent(p), When::OpponentCastsInstantOrSorcery, None);
        }
        for (owner, kind, card) in self.emblems.clone() {
            if let EmblemKind::OnAnySpellCast { trigger } = kind {
                self.triggered.push(StackItem {
                    kind: StackKind::Trigger(trigger),
                    ..Game::casting(card, owner)
                });
            }
        }
        self.spells_cast[p] += 1;
        let is_creature = self.types_of(card).creature;
        if !is_creature {
            let (t, n) = self.noncreature_casts[p];
            self.noncreature_casts[p] = (self.turn, if t == self.turn { n.saturating_add(1) } else { 1 });
        }
        self.cast_creature_noncreature[p][usize::from(!is_creature)] = true;
        if self.creatures_uncounterable_turn[p] == self.turn && self.turn > 0 && self.types_of(card).creature
        {
            self.obj_mut(card).cast_uncounterable = true;
        }
        if self.next_spell_uncounterable[p] == self.turn && self.turn > 0 {
            self.next_spell_uncounterable[p] = 0;
            self.obj_mut(card).cast_uncounterable = true;
        }
        if self.types_of(card).creature {
            let before = self.triggered.len();
            self.fire(p, When::YouCastCreature, None);
            let power = self.def_of(card).power;
            let new = self.triggered.split_off(before);
            // Vivien's −2: "When you next cast a creature spell this turn" (once).
            let turn = self.turn;
            let (now, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.next_creature_spell)
                .into_iter()
                .partition(|&(_, _, q, t)| q == p && t == turn);
            self.next_creature_spell = later;
            for (source, n, q, _) in now {
                self.triggered.push(StackItem {
                    kind: StackKind::Trigger(n),
                    extra: Some(card),
                    ..Game::casting(source, q)
                });
            }
            for mut t in new {
                t.extra = Some(card); // "that spell" (Sarkhan's Unsealing)
                let cond = match t.kind {
                    StackKind::Trigger(n) => self.def_of(t.source).triggers[n].ability.condition.clone(),
                    _ => None,
                };
                if let Some(Condition::CastPowerBetween(lo, hi)) = cond {
                    if power < lo || power > hi {
                        continue;
                    }
                }
                self.triggered.push(t);
            }
        }
        let before = self.triggered.len();
        self.fire(p, When::YouCast, None);
        for t in &mut self.triggered[before..] {
            t.extra = t.extra.or(Some(card)); // "that spell" (Teval)
        }
        if self.spells_cast[p] == 2 {
            self.fire(opponent(p), When::OpponentCastsSecondSpell, None);
            let before = self.triggered.len();
            self.fire(p, When::YouCastSecondSpell, None);
            for t in &mut self.triggered[before..] {
                t.extra = t.extra.or(Some(card)); // "copy it" (Taigam)
            }
        }
        self.ward_check(&item);
        let instant_or_sorcery = self.types_of(card).instant || self.types_of(card).sorcery;
        let casts_this = self
            .def_of(card)
            .triggers
            .iter()
            .any(|t| t.when == When::YouCastThis);
        if instant_or_sorcery || casts_this {
            self.cast_snapshots.push(item.clone());
        }
        self.items.push(item);
        if casts_this {
            // Storm counts the spells cast before this one, as it's cast (CR 702.40a).
            let before = self.triggered.len();
            self.fire_source(card, When::YouCastThis);
            let earlier = (self.spells_cast[0] + self.spells_cast[1]).saturating_sub(1) as i32;
            for t in &mut self.triggered[before..] {
                t.amount = earlier;
            }
        }
        self.events.push(Event::Cast { card });
        // "When you cast this spell" (Ugin): triggers from the stack.
        let whens: Vec<When> = self.def_of(card).triggers.iter().map(|tr| tr.when).collect();
        for (n, w) in whens.into_iter().enumerate() {
            if w == When::CastThisSpell {
                self.triggered.push(StackItem {
                    kind: StackKind::Trigger(n),
                    extra: Some(card), // "this spell" (cascade)
                    ..Game::casting(card, p)
                });
            }
        }
        if !self.colors_of(card).iter().any(|&c| c) {
            self.fire(p, When::YouCastColorless, None);
        }
        // "Copy the next spell you cast this turn when you cast it" (Flamehold Grappler).
        let turn = self.turn;
        let (now, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.copy_next_spell)
            .into_iter()
            .partition(|&(_, _, q, t)| q == p && t == turn);
        self.copy_next_spell = later;
        for (source, n, controller, _) in now {
            self.triggered.push(StackItem {
                kind: StackKind::Trigger(n),
                extra: Some(card),
                ..Game::casting(source, controller)
            });
        }
        if instant_or_sorcery {
            self.fire(p, When::YouCastInstantOrSorcery, None);
            let earlier = self.instants_sorceries_cast[p];
            self.instants_sorceries_cast[p] += 1;
            let before = self.triggered.len();
            self.fire(p, When::YouCastInstantOrSorceryCounted, None);
            for t in &mut self.triggered[before..] {
                t.amount = earlier as i32;
                t.extra = Some(card);
            }
        }
        // "Whenever you cast a noncreature or [subtype] spell".
        let (creature, subs) = (self.types_of(card).creature, self.def_of(card).subtypes.clone());
        for id in self.permanents(p).collect::<Vec<_>>() {
            let whens: Vec<When> = self.def_of(id).triggers.iter().map(|tr| tr.when).collect();
            for (n, w) in whens.into_iter().enumerate() {
                let hit = match w {
                    When::YouCastNoncreatureOrSubtype(sub) => !creature || subs.iter().any(|s| s == sub),
                    When::YouCastSubtype(sub) => subs.iter().any(|s| s == sub),
                    _ => false,
                };
                if hit {
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
        if noncreature {
            let turn = self.turn;
            for (w, src, n, who, t) in self.delayed_this_turn.clone() {
                if w == When::YouCastNoncreature && who == p && t == turn {
                    self.triggered.push(StackItem {
                        kind: StackKind::Trigger(n),
                        ..Game::casting(src, who)
                    });
                }
            }
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
        let sacrifice = self.def_of(card).flashback_sacrifice;
        if flashback_cast && sacrifice > 0 {
            // Dread Return's flashback cost, paid as it's cast (CR 601.2h), before priority.
            self.ask(p, Decision::FlashbackSacrifice { left: sacrifice });
            return;
        }
        if self.free_casts.is_empty() {
            self.give_priority(p);
        } else {
            self.open_free_casts();
        }
    }

    /// The casts open in a free-cast window: spells only (no lands), any timing (CR 608.2g).
    fn free_cast_actions(&self, p: PlayerId) -> Vec<Action> {
        let mut v = Vec::new();
        if self.cant_cast_now(p) {
            return v; // Voice of Victory
        }
        for &c in &self.free_casts {
            let def = self.def_of(c);
            if self.obj(c).zone != Zone::Exile || def.types.land {
                continue;
            }
            let castable = def.types.is_permanent() || def.spell.is_some() || !def.modes.is_empty();
            if !castable {
                continue;
            }
            if let Some(ability) = &def.spell {
                if !self.trigger_can_target(&Game::casting(c, p), ability) {
                    continue;
                }
            }
            for (action, mode) in self.cast_modes(p, c, false) {
                if self.mode_affordable(p, c, mode) {
                    v.push(action);
                }
            }
        }
        v
    }

    /// Offers the free casts, or closes the window if nothing can be cast.
    fn open_free_casts(&mut self) {
        let p = self.free_caster;
        if self.free_cast_actions(p).is_empty() {
            self.close_free_casts();
        } else {
            self.pending = Pending::FreeCast(p);
        }
    }

    /// Cards not cast stay in exile; priority goes to the active player (CR 117.3b).
    fn close_free_casts(&mut self) {
        let to_hand = std::mem::take(&mut self.free_casts_to_hand);
        let to_bottom = std::mem::take(&mut self.free_casts_to_bottom);
        for c in std::mem::take(&mut self.free_casts) {
            self.obj_mut(c).cast_free = false;
            if to_hand && self.obj(c).zone == Zone::Exile {
                self.move_to(c, Zone::Hand); // Breaching Dragonstorm: "If you don't, put that card into your hand."
            }
            if to_bottom && self.obj(c).zone == Zone::Exile {
                // Cascade: a card not cast goes to the bottom.
                let owner = self.obj(c).owner;
                self.move_to(c, Zone::Library);
                self.put_among_rest(owner, vec![c], false);
            }
        }
        self.give_priority(self.active);
    }

    fn choose_target(&mut self, target: Target) {
        let c = self.choosing.as_mut().expect("choosing targets");
        let chosen = match target {
            Target::Player(p) => Chosen::Player(p),
            Target::Object(id) => Chosen::Object(id, self.objects[id.0 as usize].zcc),
        };
        c.item.targets.push(chosen);
        let c = self.choosing.as_ref().expect("choosing targets");
        let exact = self.ability_of(&c.item).targets_exactly_x && c.item.targets.len() >= c.item.x as usize;
        let done = c.item.targets.len() == self.ability_of(&c.item).targets.len() || exact;
        if done {
            self.finish_choosing();
        }
    }

    fn finish_choosing(&mut self) {
        let mut c = self.choosing.take().expect("choosing targets");
        let total = self.divide_total(&c.item);
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
                if self.activated_of(c.item.source, n).less_per_target_color {
                    // Pass the discount as X (the equip cost has no {X}).
                    if let Some(&Chosen::Object(t, _)) = c.item.targets.first() {
                        c.item.x = self.colors_of(t).iter().filter(|&&b| b).count() as u8;
                    }
                }
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
        // "Whenever a creature you control or a creature spell you control becomes the target of a
        // spell or ability an opponent controls" (Surrak).
        for &t in &item.targets {
            if let Chosen::Object(id, _) = t {
                let o = self.obj(id);
                let creature = self.types_of(id).creature;
                let on_field = o.zone == Zone::Battlefield;
                let spell = o.zone == Zone::Stack
                    && self
                        .items
                        .iter()
                        .any(|i| i.kind == StackKind::Spell && i.source == id);
                if creature && (on_field || spell) && o.controller != item.controller {
                    let p = o.controller;
                    self.fire(p, When::YourCreatureTargetedByOpponent, None);
                }
            }
        }
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
                if self.def_of(id).ward_discard {
                    self.triggered.push(StackItem {
                        kind: StackKind::WardDiscard { against: item.source },
                        ..Game::casting(id, controller)
                    });
                }
                let n = self.def_of(id).ward_sacrifice;
                if n > 0 {
                    self.triggered.push(StackItem {
                        kind: StackKind::WardSacrifice {
                            n,
                            against: item.source,
                        },
                        ..Game::casting(id, controller)
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
            if self.items[pos].kind == StackKind::Spell && self.uncounterable(source) {
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
        self.resolutions_this_turn += 1;
        if self.resolutions_this_turn > RESOLUTIONS_PER_TURN {
            // CR 104.4b: an endless loop of mandatory actions is a draw.
            self.result = Some(Outcome::Draw);
            self.pending = Pending::GameOver;
            return;
        }
        let source = item.source;
        if let StackKind::WardDiscard { against } = item.kind {
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
                if self.players[payer].hand.is_empty() {
                    self.counter_from(against);
                } else {
                    self.ask(payer, Decision::PayWardDiscard { against });
                    return;
                }
            }
            self.continue_resolving();
            return;
        }
        if let StackKind::WardSacrifice { n, against } = item.kind {
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
                let have = self
                    .permanents(payer)
                    .filter(|&x| !self.cant_be_sacrificed(x))
                    .count();
                if have < n as usize {
                    self.counter_from(against);
                } else {
                    self.ask(
                        payer,
                        Decision::PayWardSacrifice {
                            left: n,
                            against,
                            picked: false,
                        },
                    );
                    return;
                }
            }
            self.continue_resolving();
            return;
        }
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
        let has_ability = !matches!(item.kind, StackKind::Spell | StackKind::SpellCopy)
            || self.def_of(source).spell.is_some()
            || item.mode.is_some();
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
        // CR 707.10f: a copy of a permanent spell becomes a token as it resolves.
        if r.item.kind == StackKind::SpellCopy && r.acts && self.types_of(source).is_permanent() {
            let def = self.obj(source).def;
            let id = self.add_card(r.item.controller, def, Zone::Hand);
            self.obj_mut(id).copy_token = true;
            self.move_to(id, Zone::Battlefield);
            self.obj_mut(id).controller = r.item.controller;
            if self.def_of(id).aura {
                if let Some(&Chosen::Object(t, z)) = r.item.targets.first() {
                    self.obj_mut(id).attached_to = Some((t, z));
                }
            }
        }
        if r.item.kind == StackKind::Spell {
            let omen = r.item.mode == Some(OMEN_MODE);
            let shuffles = self.def_of(source).shuffle_on_resolve;
            if (omen || shuffles) && self.obj(source).zone == Zone::Stack {
                // An Omen: "Then shuffle this card into its owner's library."
                let owner = self.obj(source).owner;
                self.move_to(source, Zone::Library);
                self.shuffle_library(owner);
            } else if r.acts && self.types_of(source).is_permanent() {
                self.move_to(source, Zone::Battlefield);
                self.obj_mut(source).controller = r.item.controller;
                self.obj_mut(source).was_cast = true;
                // Blitz: haste, "When this creature dies, draw a card", sacrificed at the next end step.
                if let (true, Some(n)) = (r.item.alt, self.def_of(source).blitz) {
                    if self.obj(source).zone == Zone::Battlefield {
                        let zcc = self.obj(source).zcc;
                        self.until_eot.push(Modifier {
                            obj: source,
                            zcc,
                            power: 0,
                            toughness: 0,
                            keywords: Keywords::HASTE,
                            dies: Some((source, n, r.item.controller)),
                        });
                        self.obj_mut(source).sacrifice_at_end = true;
                    }
                }
                if self.obj(source).card_copy {
                    self.obj_mut(source).copy_token = true; // CR 707.10f: a copy becomes a token
                }
                // "Whenever a creature you control enters, if you cast it" (The Sibsig Ceremony).
                if self.types_of(source).creature && self.obj(source).zone == Zone::Battlefield {
                    let controller = r.item.controller;
                    let before = self.triggered.len();
                    self.fire(controller, When::CastCreatureYouControlEnters, Some(source));
                    let zcc = self.obj(source).zcc;
                    for t in &mut self.triggered[before..] {
                        t.targets = vec![Chosen::Object(source, zcc)];
                    }
                }
                if std::mem::take(&mut self.obj_mut(source).offspring_paid) {
                    self.triggered.push(StackItem {
                        kind: StackKind::Offspring,
                        ..Game::casting(source, r.item.controller)
                    });
                }
                if std::mem::take(&mut self.obj_mut(source).enters_with_finality) {
                    self.obj_mut(source).finality = true;
                    self.counter_put_on(source);
                }
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
                if r.acts && r.item.kicked && self.def_of(source).buyback_sacrifice.is_some() {
                    self.move_to(source, Zone::Hand); // buyback
                } else {
                    self.leave_stack(source, r.item.flashback);
                }
            }
            if r.acts {
                self.events.push(Event::Resolved { card: source });
            }
            // "You may have this creature enter as a copy of any creature" (Naga Fleshcrafter).
            if r.acts
                && self.def_of(source).enter_as_copy
                && self.obj(source).zone == Zone::Battlefield
                && self
                    .battlefield
                    .iter()
                    .any(|&x| x != source && self.copy_option(source, x))
            {
                self.ask(r.item.controller, Decision::EnterAsCopy { card: source });
                return;
            }
            // "As this enters, choose [A] or [B]" (the Sieges). Asked as the spell resolves, like the
            // color choice below: a Siege put onto the battlefield another way gets no mode.
            if r.acts && self.def_of(source).unleash && self.obj(source).zone == Zone::Battlefield {
                self.ask(r.item.controller, Decision::Unleash { card: source });
                return;
            }
            if r.acts && self.def_of(source).enter_modes && self.obj(source).zone == Zone::Battlefield {
                self.ask(r.item.controller, Decision::ChooseEnterMode { card: source });
                return;
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
        if !self.free_casts.is_empty() {
            self.open_free_casts();
            return;
        }
        // Spell copies: onto the stack, choosing new targets if it has any (CR 707.10c).
        while !self.pending_copies.is_empty() {
            let copy = self.pending_copies.remove(0);
            let d = self.def_of(copy.source);
            let targeted = d.spell.is_some() || copy.mode.is_some();
            if targeted && !self.ability_of(&copy).targets.is_empty() {
                let mut fresh = copy.clone();
                fresh.targets.clear();
                fresh.division.clear();
                let ability = self.ability_of(&fresh).clone();
                if self.trigger_can_target(&fresh, &ability) {
                    let p = copy.controller;
                    self.choosing = Some(Choosing {
                        item: fresh,
                        then: self.active,
                        cast: false,
                        pay: None,
                        loyalty: None,
                        remove_counters: 0,
                    });
                    self.pending = Pending::ChooseTarget(p);
                    return;
                }
            }
            self.ward_check(&copy);
            self.items.push(copy);
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
        self.triggered_scanned = 0;
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
        self.counter_put_on(id);
        if self.types_of(id).creature && !self.has_subtype(id, "Hydra") {
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
            let any = self.permanents(p).any(|x| Game::has_type(self.types_of(x), ty));
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
    fn has_type(t: Types, ty: u8) -> bool {
        match ty {
            0 => t.artifact,
            1 => t.creature,
            2 => t.enchantment,
            3 => t.land,
            _ => t.planeswalker,
        }
    }

    fn picks(&self, c: ObjId, pick: Pick) -> bool {
        let d = self.def_of(c);
        match pick {
            Pick::Any => true,
            Pick::CreaturePowerAtMost(n) => d.types.creature && d.power <= n,
            Pick::InstantOrSorcery => d.types.instant || d.types.sorcery,
            Pick::CreatureOrLand => d.types.creature || d.types.land,
            Pick::Land => d.types.land,
            Pick::Enchantment => d.types.enchantment,
            Pick::InstantSorceryOrSubtype(sub) => {
                d.types.instant || d.types.sorcery || d.subtypes.iter().any(|s| s == sub)
            }
            Pick::CreatureMvAtMostLands => {
                let owner = self.obj(c).owner;
                let lands = self.permanents(owner).filter(|&x| self.types_of(x).land).count() as u32;
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
                    SearchFor::LandTypes { types, basic } => {
                        const NAMES: [&str; 5] = ["Plains", "Island", "Swamp", "Mountain", "Forest"];
                        d.types.land
                            && (!basic || d.types.basic)
                            && (0..5).any(|i| types[i] && d.subtypes.iter().any(|s| s == NAMES[i]))
                    }
                    SearchFor::InstantOrSorceryWithManaValue(mv) => {
                        (d.types.instant || d.types.sorcery) && d.cost.mana_value() == mv
                    }
                    SearchFor::CreatureMvAtMost(mv) => d.types.creature && d.cost.mana_value() <= mv,
                    SearchFor::CreatureMvAtMostX => false, // resolved to CreatureMvAtMost first
                    SearchFor::Named(names) => names.contains(&d.name.as_str()),
                    SearchFor::Colored(c) => d.colors[c as usize],
                    SearchFor::Subtype(sub) => d.subtypes.iter().any(|s| s == sub),
                    SearchFor::ColorlessNonland => !d.types.land && !d.colors.iter().any(|&c| c),
                    SearchFor::Land => d.types.land,
                    SearchFor::CreatureMvAtLeast(mv) => d.types.creature && d.cost.mana_value() >= mv,
                    SearchFor::BasicLandSharingType => d.types.basic && d.types.land,
                    SearchFor::ManaValue(mv) => d.cost.mana_value() == mv,
                    SearchFor::GreenCreatureMvAtMostX => false, // resolved first
                    SearchFor::ColoredCreatureMvAtMost(c, mv) => {
                        d.types.creature && d.colors[c as usize] && d.cost.mana_value() <= mv
                    }
                }
            })
            .collect()
    }

    fn decision_actions(&self, p: PlayerId) -> Vec<Action> {
        let hand = &self.players[p].hand;
        match self.decision.as_ref().expect("a decision") {
            Decision::ScryOrder { kept, .. } => kept.iter().copied().map(Action::Pick).collect(),
            Decision::May
            | Decision::Surveil { .. }
            | Decision::Scry { .. }
            | Decision::Tuck { .. }
            | Decision::Endure { .. } => {
                vec![Action::Yes, Action::No]
            }
            Decision::SacrificeCreatures { .. } => self
                .creatures(p)
                .filter(|&c| !self.cant_be_sacrificed(c))
                .map(Action::Pick)
                .collect(),
            Decision::ExileCreature => self.creatures(p).map(Action::Pick).collect(),
            Decision::ChooseColor { card } => {
                // The Thriving lands: "a color other than" their own.
                let not = self.def_of(*card).thriving_color.map(|c| c as u8);
                (0..5).filter(|&c| Some(c) != not).map(Action::Mode).collect()
            }
            Decision::ChooseEnterMode { .. } => vec![Action::Mode(0), Action::Mode(1)],
            Decision::ChooseCreatureType { options, .. } => {
                (0..options.len() as u8).map(Action::Mode).collect()
            }
            Decision::KeepOfType { ty, .. } => self
                .permanents(p)
                .filter(|&x| Game::has_type(self.types_of(x), *ty))
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
                            .filter(|&x| !self.types_of(x).land)
                            .map(Action::Pick),
                    );
                }
                v.push(Action::No);
                v
            }
            Decision::PayWardDiscard { .. } => {
                let mut v: Vec<Action> = self.players[p]
                    .hand
                    .iter()
                    .copied()
                    .map(Action::Discard)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::EnterAsCopy { card } => {
                let mut v: Vec<Action> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&x| x != *card && self.copy_option(*card, x))
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::ChooseDelve { card, mode, max } => (0..=*max)
                .filter(|&n| {
                    let mut cost = self.spell_cost(p, *card, *mode, &[]);
                    cost.generic = cost.generic.saturating_sub(n);
                    self.can_pay_for(p, &cost, None, pay_for_spell(*card, mode.mode))
                })
                .map(Action::Amount)
                .collect(),
            Decision::ChooseCastX { card, mode, max } => (0..=*max)
                .filter(|&x| self.mode_affordable(p, *card, CastMode { x, ..*mode }))
                .map(Action::Amount)
                .collect(),
            Decision::PayWardLife { life, .. } => {
                let mut v = Vec::new();
                if self.players[p].life >= *life {
                    v.push(Action::Yes); // CR 119.4
                }
                v.push(Action::No);
                v
            }
            Decision::CommanderToCommandZone { .. } => vec![Action::Yes, Action::No],
            Decision::ShockLand { .. } => vec![Action::Yes, Action::No],
            Decision::SacrificeLandOrThis { .. } => {
                let mut v: Vec<Action> = self
                    .permanents(p)
                    .filter(|&x| self.types_of(x).land && !self.cant_be_sacrificed(x))
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::FlashbackSacrifice { .. } | Decision::SacrificeThenReturnOther => self
                .creatures(p)
                .filter(|&c| !self.cant_be_sacrificed(c))
                .map(Action::Pick)
                .collect(),
            Decision::SacrificeLandsThenSearch { .. } => {
                let mut v: Vec<Action> = self
                    .permanents(p)
                    .filter(|&x| self.types_of(x).land && !self.cant_be_sacrificed(x))
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::PutPicksOntoBattlefield {
                cards,
                picked,
                creature_and_land,
                ..
            } => {
                let mut v: Vec<Action> = cards
                    .iter()
                    .copied()
                    .filter(|&c| self.pick_onto_battlefield_ok(c, picked, *creature_and_land))
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::SacrificeOneOf { cards } => cards.iter().copied().map(Action::Pick).collect(),
            Decision::HandToTop { .. } => hand.iter().copied().map(Action::Pick).collect(),
            Decision::SoulbondPair { source } => {
                let mut v: Vec<Action> = self
                    .creatures(p)
                    .filter(|&c| c != *source && self.paired_partner(c).is_none())
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::SacrificePermanents { .. } => self
                .permanents(p)
                .filter(|&x| !self.cant_be_sacrificed(x))
                .map(Action::Pick)
                .collect(),
            Decision::PayWardSacrifice { picked, .. } => {
                let mut v: Vec<Action> = self
                    .permanents(p)
                    .filter(|&x| !self.cant_be_sacrificed(x))
                    .map(Action::Pick)
                    .collect();
                if !picked {
                    v.push(Action::No);
                }
                v
            }
            Decision::SneakAttack => {
                let mut v: Vec<Action> = hand
                    .iter()
                    .copied()
                    .filter(|&x| self.def_of(x).types.creature)
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::ExiledToBattlefield { cards } => {
                let mut v: Vec<Action> = cards.iter().copied().map(Action::Pick).collect();
                v.push(Action::No);
                v
            }
            Decision::CreaturesFromHand => {
                let mut v: Vec<Action> = hand
                    .iter()
                    .copied()
                    .filter(|&x| self.def_of(x).types.creature)
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::ReturnAnotherForIndestructible { source } => {
                let mut v: Vec<Action> = self
                    .creatures(p)
                    .filter(|&c| c != *source)
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::LandTappedDrawRepeat | Decision::BasicFromHandTapped => {
                let basic = matches!(self.decision, Some(Decision::BasicFromHandTapped));
                let mut v: Vec<Action> = hand
                    .iter()
                    .copied()
                    .filter(|&x| {
                        let t = self.def_of(x).types;
                        t.land && (!basic || t.basic)
                    })
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::HulkSearch { budget } => {
                let mut v: Vec<Action> = self.players[p]
                    .library
                    .iter()
                    .copied()
                    .filter(|&c| self.def_of(c).types.creature && self.def_of(c).cost.mana_value() <= *budget)
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::PrimalSurge { .. } | Decision::CommanderCourser { .. } => vec![Action::Yes, Action::No],
            Decision::HideawayPick { cards, .. } => cards.iter().copied().map(Action::Pick).collect(),
            Decision::KeywordCounterChoice { .. } => vec![Action::Mode(0), Action::Mode(1), Action::Mode(2)],
            Decision::DelinaAgain { .. } => vec![Action::Yes, Action::No],
            Decision::LandFromHand => {
                let mut v: Vec<Action> = hand
                    .iter()
                    .copied()
                    .filter(|&x| self.def_of(x).types.land)
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::ScrollRackExile { .. } => {
                let mut v: Vec<Action> = hand.iter().copied().map(Action::Pick).collect();
                v.push(Action::No);
                v
            }
            Decision::ReturnOtherPermanent { except } => {
                let mut v: Vec<Action> = self.players[p]
                    .graveyard
                    .iter()
                    .copied()
                    .filter(|&c| c != *except && self.def_of(c).types.is_permanent())
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::PayToStop { cost } | Decision::PayUpkeep { cost, .. } => {
                let mut v = Vec::new();
                if self.can_pay_for(p, cost, None, PayFor::Other) {
                    v.push(Action::Yes);
                }
                v.push(Action::No);
                v
            }
            Decision::DistributeCounters { .. } => self.creatures(p).map(Action::Pick).collect(),
            Decision::LandFromHandTapped => {
                let mut v: Vec<Action> = self.players[p]
                    .hand
                    .iter()
                    .copied()
                    .filter(|&x| self.def_of(x).types.land)
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::ManifestDread { cards } => cards.iter().map(|&c| Action::Pick(c)).collect(),
            Decision::TurnFaceUp => {
                let mut v: Vec<Action> = self
                    .permanents(p)
                    .filter(|&x| self.can_turn_face_up(x))
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::ChooseNewTarget { item, slot } => {
                let mut v: Vec<Action> = self
                    .new_target_options(*item, *slot as usize)
                    .into_iter()
                    .map(Action::Target)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::LandsFromHand => {
                let mut v: Vec<Action> = self.players[p]
                    .hand
                    .iter()
                    .copied()
                    .filter(|&x| self.def_of(x).types.land)
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::MayDiscardOrStop => {
                let mut v: Vec<Action> = self.players[p]
                    .hand
                    .iter()
                    .copied()
                    .map(Action::Discard)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::BottomOrder { cards } => cards.iter().copied().map(Action::Pick).collect(),
            Decision::ChooseDamageSource { .. } => self
                .battlefield
                .iter()
                .copied()
                .chain(
                    self.items
                        .iter()
                        .filter(|i| i.kind == StackKind::Spell)
                        .map(|i| i.source),
                )
                .map(Action::Pick)
                .collect(),
            Decision::PutOnTopFromGraveyard { .. } => self.players[p]
                .graveyard
                .iter()
                .copied()
                .map(Action::Pick)
                .collect(),
            Decision::DragonForColor { color, .. } => self
                .creatures(p)
                .filter(|&c| self.has_subtype(c, "Dragon") && self.colors_of(c)[*color as usize])
                .map(Action::Pick)
                .collect(),
            Decision::SacrificeNonland => {
                let mut v: Vec<Action> = self
                    .permanents(p)
                    .filter(|&c| !self.types_of(c).land)
                    .map(Action::Pick)
                    .collect();
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
            Decision::LookPick {
                cards, pick, rest, ..
            } => {
                let mut v: Vec<Action> = cards
                    .iter()
                    .copied()
                    .filter(|&c| self.picks(c, *pick))
                    .map(Action::Pick)
                    .collect();
                // "Put one of them into your hand": taking one isn't optional when one can be taken.
                let must = matches!(
                    rest,
                    Some(
                        LookRest::GraveyardTakeOne
                            | LookRest::GraveyardTakeTwo
                            | LookRest::BottomAnyOrderTakeOne
                    )
                );
                if v.is_empty() || !must {
                    v.push(Action::No);
                }
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
            Decision::PayWard { cost, .. } | Decision::PayOrCountered { cost, .. } => {
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
            Decision::PutFromAmong {
                cards,
                noncreature_nonland,
                max_mv,
                ..
            } => {
                let mut v: Vec<Action> = cards
                    .iter()
                    .copied()
                    .filter(|&c| self.put_among_ok(c, *noncreature_nonland, *max_mv))
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::ReturnDifferentNames { names } => {
                let mut v: Vec<Action> = self.players[p]
                    .graveyard
                    .iter()
                    .copied()
                    .filter(|&c| self.def_of(c).types.is_permanent() && !names.contains(&self.def_of(c).name))
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::PickExileLinked { victim, .. } => {
                let mut v: Vec<Action> = self.players[*victim]
                    .hand
                    .iter()
                    .filter(|&&c| !self.types_of(c).land)
                    .map(|&c| Action::Pick(c))
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::EndureOther { .. } => vec![Action::Yes, Action::No],
            Decision::ReturnOwnCreature => self.creatures(p).map(Action::Pick).collect(),
            Decision::TemptOffer => vec![Action::Yes, Action::No],
            Decision::Unleash { .. } => vec![Action::Yes, Action::No],
            Decision::PutCreatureAttacking => {
                let mut v: Vec<Action> = self.players[p]
                    .hand
                    .iter()
                    .copied()
                    .filter(|&c| self.def_of(c).types.creature)
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::SacrificeToken => {
                let mut v: Vec<Action> = self
                    .permanents(p)
                    .filter(|&c| self.is_token(c) && !self.cant_be_sacrificed(c))
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::ReturnOwnLand => self
                .permanents(p)
                .filter(|&c| self.types_of(c).land)
                .map(Action::Pick)
                .collect(),
            Decision::SacrificeCreatureOrPlaneswalker => self
                .permanents(p)
                .filter(|&c| self.types_of(c).creature || self.types_of(c).planeswalker)
                .filter(|&c| !self.cant_be_sacrificed(c))
                .map(Action::Pick)
                .collect(),
            Decision::SearchMonoNames { picked } => {
                let mut v: Vec<Action> = self.players[p]
                    .library
                    .iter()
                    .copied()
                    .filter(|&c| self.mono_search_ok(c, picked))
                    .map(Action::Pick)
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::OpponentShufflesOne { cards, .. } => cards.iter().map(|&c| Action::Pick(c)).collect(),
            Decision::GraveyardToHand { .. } => {
                let mut v: Vec<Action> = self.players[p]
                    .graveyard
                    .iter()
                    .filter(|&&c| self.def_of(c).types.is_permanent())
                    .map(|&c| Action::Pick(c))
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::GraveyardShuffle { owner, .. } => {
                let mut v: Vec<Action> = self.players[*owner]
                    .graveyard
                    .iter()
                    .map(|&c| Action::Pick(c))
                    .collect();
                v.push(Action::No);
                v
            }
            Decision::DiscardReflexive { .. } => {
                let mut v: Vec<Action> = hand.iter().map(|&c| Action::Discard(c)).collect();
                v.push(Action::No);
                v
            }
            Decision::DiscardTwoUnlessCreature { .. } => hand.iter().map(|&c| Action::Discard(c)).collect(),
            Decision::DiscardUpToDraw { .. } => {
                let mut v: Vec<Action> = hand.iter().map(|&c| Action::Discard(c)).collect();
                v.push(Action::No);
                v
            }
            Decision::PickExile { victim } => self.players[*victim]
                .hand
                .iter()
                .filter(|&&c| !self.types_of(c).land)
                .map(|&c| Action::Pick(c))
                .collect(),
            Decision::ExileFromGraveyard {
                creature_only,
                started,
                ..
            } => {
                let mut v: Vec<Action> = self.players[p]
                    .graveyard
                    .iter()
                    .filter(|&&c| !*creature_only || self.def_of(c).types.creature)
                    .map(|&c| Action::Pick(c))
                    .collect();
                if !*started {
                    v.push(Action::No);
                }
                v
            }
            Decision::PickDiscard { victim } => self.players[*victim]
                .hand
                .iter()
                .filter(|&&c| !self.types_of(c).land)
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
        if let Decision::EnterAsCopy { card } = d {
            if let Action::Pick(of) = a {
                let def = self.obj(of).def;
                let printed = self.obj(card).def;
                let eot = self.defs[printed].copy_until_eot_haste;
                self.become_copy(card, def);
                self.obj_mut(card).copy_exceptions = true;
                if eot {
                    // Cursed Mirror: until end of turn, with haste.
                    let zcc = self.obj(card).zcc;
                    self.copy_until_eot.push((card, zcc, printed));
                    self.until_eot.push(Modifier {
                        obj: card,
                        zcc,
                        power: 0,
                        toughness: 0,
                        keywords: Keywords::HASTE,
                        dies: None,
                    });
                }
                if let Some(l) = self.defs[def].loyalty {
                    self.obj_mut(card).loyalty = l; // a copied planeswalker enters with its loyalty
                }
                // It entered as the copy: the copied creature's "when this enters" triggers apply.
                let enters: Vec<usize> = self
                    .def_of(card)
                    .triggers
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.when == When::Enters)
                    .map(|(n, _)| n)
                    .collect();
                let controller = self.obj(card).controller;
                for n in enters {
                    self.triggered.push(StackItem {
                        kind: StackKind::Trigger(n),
                        ..Game::casting(card, controller)
                    });
                }
            }
            if self.end_turn {
                self.end_the_turn();
            } else {
                self.give_priority(self.active);
            }
            return;
        }
        if let (Decision::ChooseDelve { card, mode, .. }, Action::Amount(n)) = (&d, a) {
            let (card, mode) = (*card, *mode);
            self.obj_mut(card).delve = n;
            return self.cast_now(p, card, mode);
        }
        if let (Decision::ChooseCastX { card, mode, .. }, Action::Amount(x)) = (&d, a) {
            let (card, mode) = (*card, *mode);
            return self.cast(p, card, CastMode { x, ..mode });
        }
        if let Decision::Unleash { card } = d {
            if a == Action::Yes {
                self.add_counters(card, 1);
            }
            if self.end_turn {
                self.end_the_turn();
            } else {
                self.give_priority(self.active);
            }
            return;
        }
        if let (Decision::FlashbackSacrifice { left }, Action::Pick(c)) = (&d, a) {
            self.move_to(c, Zone::Graveyard);
            let more = self.creatures(p).any(|x| !self.cant_be_sacrificed(x));
            if *left > 1 && more {
                self.ask(p, Decision::FlashbackSacrifice { left: left - 1 });
            } else if self.free_casts.is_empty() {
                self.give_priority(p);
            } else {
                self.open_free_casts();
            }
            return;
        }
        if let Decision::ShockLand { card } = d {
            if a == Action::Yes {
                self.lose_life(p, 2);
            } else {
                self.obj_mut(card).tapped = true;
            }
            let then = self.shock_then;
            self.give_priority(then);
            return;
        }
        if let Decision::CommanderToCommandZone { card } = d {
            if a == Action::Yes && matches!(self.obj(card).zone, Zone::Graveyard | Zone::Exile) {
                self.move_to(card, Zone::Command);
            }
            let then = self.commander_choice_then;
            self.give_priority(then);
            return;
        }
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
        if let (Decision::ChooseEnterMode { card }, Action::Mode(m)) = (&d, a) {
            self.obj_mut(*card).chosen_mode = Some(m);
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
            self.triggered_scanned += 1; // already looked at by Roaming Throne
            self.give_priority(then);
            return;
        }
        match (d, a) {
            (Decision::PayWard { cost, .. } | Decision::PayOrCountered { cost, .. }, Action::Yes) => {
                self.pay_mana(p, &cost, None)
            }
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
            (Decision::ExileCreature, Action::Pick(c)) => self.move_to(c, Zone::Exile),
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
            (Decision::PayWardDiscard { .. }, Action::Discard(c)) => self.move_to(c, Zone::Graveyard),
            (Decision::PayWardDiscard { against }, Action::No) => self.counter_from(against),
            (Decision::SacrificeNonland, Action::Pick(c)) => self.move_to(c, Zone::Graveyard),
            (Decision::MayDiscardOrStop, Action::Discard(c)) => self.move_to(c, Zone::Graveyard),
            (Decision::MayDiscardOrStop, Action::No) => {
                self.resolving.as_mut().expect("resolving").remaining.clear();
            }
            (Decision::BottomOrder { cards }, Action::Pick(c)) => {
                let rest: Vec<ObjId> = cards.into_iter().filter(|&x| x != c).collect();
                let lib = &mut self.players[p].library;
                if let Some(pos) = lib.iter().position(|&x| x == c) {
                    let card = lib.remove(pos);
                    lib.insert(0, card);
                }
                self.bottom_in_order(p, rest);
            }
            (Decision::ChooseDamageSource { card, trigger }, Action::Pick(c)) => {
                let entry = (p, c, card, trigger, self.turn);
                self.damage_shields.push(entry);
            }
            (Decision::PutOnTopFromGraveyard { left }, Action::Pick(c)) => {
                self.put_on_top(c);
                if left > 1 && !self.players[p].graveyard.is_empty() {
                    self.ask(p, Decision::PutOnTopFromGraveyard { left: left - 1 });
                }
            }
            (Decision::DragonForColor { color, mut placed }, Action::Pick(c)) => {
                self.add_counters(c, 1);
                if !placed.contains(&c) {
                    placed.push(c);
                }
                self.next_dragon_color(p, color + 1, placed);
            }
            (Decision::SacrificeNonland, Action::No) => {
                self.resolving.as_mut().expect("resolving").remaining.clear();
            }
            (Decision::SacrificeAnother { .. }, Action::No) => {
                self.resolving.as_mut().expect("resolving").remaining.clear();
            }
            (
                Decision::LookPick {
                    cards,
                    pick,
                    to,
                    rest,
                },
                choice,
            ) => {
                let chosen = match choice {
                    Action::Pick(c) => Some(c),
                    _ => None,
                };
                if let Some(c) = chosen {
                    match to {
                        LookTo::Hand => self.move_to(c, Zone::Hand),
                        LookTo::Battlefield => self.move_to(c, Zone::Battlefield),
                        LookTo::BattlefieldTwoCounters => {
                            self.move_to(c, Zone::Battlefield);
                            self.add_counters(c, 2);
                        }
                        LookTo::BattlefieldTapped => {
                            self.move_to(c, Zone::Battlefield);
                            self.obj_mut(c).tapped = true;
                        }
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
                if rest == Some(LookRest::GraveyardTakeTwo) && chosen.is_some() && !others.is_empty() {
                    // "Put two of them into your hand": the second pick, from the rest.
                    self.ask(
                        p,
                        Decision::LookPick {
                            cards: others,
                            pick,
                            to,
                            rest: Some(LookRest::GraveyardTakeOne),
                        },
                    );
                    return;
                }
                if rest == Some(LookRest::GraveyardElseCounter) && chosen.is_none() {
                    // Ainok Wayfarer: "If you don't, put a +1/+1 counter on this creature."
                    if let Some(src) = self.resolving.as_ref().map(|r| r.item.source) {
                        if self.obj(src).zone == Zone::Battlefield {
                            self.add_counters(src, 1);
                        }
                    }
                }
                match rest {
                    Some(
                        LookRest::Graveyard
                        | LookRest::GraveyardTakeOne
                        | LookRest::GraveyardTakeTwo
                        | LookRest::GraveyardElseCounter,
                    ) => {
                        for c in others {
                            self.move_to(c, Zone::Graveyard);
                        }
                    }
                    Some(LookRest::BottomAnyOrderTakeOne) => self.bottom_in_order(p, others),
                    Some(LookRest::MayGraveyard) => {
                        if !others.is_empty() {
                            self.ask(p, Decision::Surveil { cards: others });
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
            (Decision::PayWard { against, .. } | Decision::PayOrCountered { against, .. }, Action::No) => {
                self.counter_from(against)
            }
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
                } else {
                    for _ in 0..then_draw {
                        self.draw(p);
                    }
                }
            }
            (Decision::Discard { .. }, Action::No) => {}
            (Decision::PickDiscard { .. }, Action::Pick(c)) => self.move_to(c, Zone::Graveyard),
            (Decision::PickExile { .. }, Action::Pick(c)) => self.move_to(c, Zone::Exile),
            (
                Decision::PutFromAmong {
                    mut cards,
                    left,
                    noncreature_nonland,
                    max_mv,
                    rest_to_hand,
                },
                Action::Pick(c),
            ) => {
                cards.retain(|&k| k != c);
                self.move_to(c, Zone::Battlefield);
                let left = if left == u8::MAX { left } else { left - 1 };
                let more = left > 0
                    && cards
                        .iter()
                        .any(|&k| self.put_among_ok(k, noncreature_nonland, max_mv));
                if more {
                    self.ask(
                        p,
                        Decision::PutFromAmong {
                            cards,
                            left,
                            noncreature_nonland,
                            max_mv,
                            rest_to_hand,
                        },
                    );
                } else {
                    self.put_among_rest(p, cards, rest_to_hand);
                }
            }
            (
                Decision::PutFromAmong {
                    cards, rest_to_hand, ..
                },
                Action::No,
            ) => {
                self.put_among_rest(p, cards, rest_to_hand);
            }
            (Decision::ReturnDifferentNames { mut names }, Action::Pick(c)) => {
                names.push(self.def_of(c).name.clone());
                self.move_to(c, Zone::Battlefield);
                let more = self.players[p]
                    .graveyard
                    .iter()
                    .any(|&k| self.def_of(k).types.is_permanent() && !names.contains(&self.def_of(k).name));
                if more {
                    self.ask(p, Decision::ReturnDifferentNames { names });
                }
            }
            (Decision::ReturnDifferentNames { .. }, Action::No) => {}
            (Decision::DiscardReflexive { source, trigger }, Action::Discard(c)) => {
                self.move_to(c, Zone::Graveyard);
                if !self.def_of(c).types.land {
                    self.triggered.push(StackItem {
                        kind: StackKind::Trigger(trigger),
                        ..Game::casting(source, p)
                    });
                }
            }
            (Decision::DiscardReflexive { .. }, Action::No) => {}
            (Decision::ReturnOwnCreature, Action::Pick(c)) => self.move_to(c, Zone::Hand),
            (Decision::ReturnOwnLand, Action::Pick(c)) => self.move_to(c, Zone::Hand),
            (Decision::SacrificeToken, Action::Pick(c)) => self.move_to(c, Zone::Graveyard),
            (Decision::TemptOffer, Action::Yes) => {
                self.tempt_accepted = true;
                let land = Decision::Search {
                    what: SearchFor::Land,
                    to: SearchTo::Battlefield { tapped: false },
                    left: 1,
                };
                self.ask(p, land);
            }
            (Decision::TemptOffer, Action::No) => self.tempt_accepted = false,
            (Decision::PutCreatureAttacking, Action::Pick(c)) => {
                self.move_to(c, Zone::Battlefield);
                self.obj_mut(c).tapped = true;
                if let Some(combat) = self.combat.as_mut() {
                    combat.attackers.push(c); // CR 506.3a: attacking without being declared
                }
                let zcc = self.obj(c).zcc;
                self.return_to_hand_at_end.push((c, zcc));
            }
            (Decision::PutCreatureAttacking, Action::No) => {}
            (Decision::SacrificeToken, Action::No) => {
                self.resolving.as_mut().expect("resolving").remaining.clear();
            }
            (Decision::SacrificeCreatureOrPlaneswalker, Action::Pick(c)) => self.move_to(c, Zone::Graveyard),
            (Decision::SearchMonoNames { mut picked }, Action::Pick(c)) => {
                picked.push(c);
                let more = self.players[p]
                    .library
                    .iter()
                    .any(|&k| self.mono_search_ok(k, &picked));
                if picked.len() < 3 && more {
                    self.ask(p, Decision::SearchMonoNames { picked });
                } else {
                    self.finish_mono_search(p, picked);
                }
            }
            (Decision::SearchMonoNames { picked }, Action::No) => self.finish_mono_search(p, picked),
            (Decision::OpponentShufflesOne { cards, caster }, Action::Pick(c)) => {
                self.move_to(c, Zone::Library);
                self.shuffle_library(caster);
                for k in cards.into_iter().filter(|&k| k != c) {
                    if !self.types_of(k).land {
                        self.obj_mut(k).cast_free = true;
                        self.free_casts.push(k);
                    }
                }
                self.free_caster = caster;
            }
            (Decision::GraveyardToHand { left }, Action::Pick(c)) => {
                self.move_to(c, Zone::Hand);
                let more = self.players[p]
                    .graveyard
                    .iter()
                    .any(|&k| self.def_of(k).types.is_permanent());
                if left > 1 && more {
                    self.ask(p, Decision::GraveyardToHand { left: left - 1 });
                }
            }
            (Decision::GraveyardToHand { .. }, Action::No) => {}
            (Decision::GraveyardShuffle { owner, left, .. }, Action::Pick(c)) => {
                self.move_to(c, Zone::Library);
                if left > 1 && !self.players[owner].graveyard.is_empty() {
                    self.ask(
                        p,
                        Decision::GraveyardShuffle {
                            owner,
                            left: left - 1,
                            picked: true,
                        },
                    );
                } else {
                    self.shuffle_library(owner);
                }
            }
            (Decision::GraveyardShuffle { owner, picked, .. }, Action::No) => {
                if picked {
                    self.shuffle_library(owner);
                }
            }
            (Decision::EndureOther { target, n, spirit }, choice) => {
                if choice == Action::Yes && self.obj(target).zone == Zone::Battlefield {
                    self.add_counters(target, n as i32);
                } else {
                    self.endure_token(p, n, spirit);
                }
            }
            (Decision::DiscardTwoUnlessCreature { second }, Action::Discard(c)) => {
                self.move_to(c, Zone::Graveyard);
                let creature = self.def_of(c).types.creature;
                if !second && !creature && !self.players[p].hand.is_empty() {
                    self.ask(p, Decision::DiscardTwoUnlessCreature { second: true });
                }
            }
            (Decision::PickExileLinked { source, .. }, Action::Pick(c)) => {
                self.move_to(c, Zone::Exile);
                self.obj_mut(source).linked_card = Some(c);
            }
            (Decision::PickExileLinked { .. }, Action::No) => {}
            (Decision::DiscardUpToDraw { left, discarded }, Action::Discard(c)) => {
                self.move_to(c, Zone::Graveyard);
                let discarded = discarded + 1;
                if left > 1 && !self.players[p].hand.is_empty() {
                    self.ask(
                        p,
                        Decision::DiscardUpToDraw {
                            left: left - 1,
                            discarded,
                        },
                    );
                } else {
                    for _ in 0..discarded {
                        self.draw(p);
                    }
                }
            }
            (Decision::DiscardUpToDraw { discarded, .. }, Action::No) => {
                for _ in 0..discarded {
                    self.draw(p);
                }
            }
            (
                Decision::ExileFromGraveyard {
                    left, creature_only, ..
                },
                Action::Pick(c),
            ) => {
                self.move_to(c, Zone::Exile);
                if left > 1 {
                    self.ask(
                        p,
                        Decision::ExileFromGraveyard {
                            left: left - 1,
                            creature_only,
                            started: true,
                        },
                    );
                }
            }
            (Decision::ExileFromGraveyard { .. }, Action::No) => {
                self.resolving.as_mut().expect("resolving").remaining.clear();
            }
            (Decision::Endure { n, spirit }, choice) => {
                let src = self.resolving.as_ref().map(|r| r.item.source);
                match src {
                    Some(s) if choice == Action::Yes && self.obj(s).zone == Zone::Battlefield => {
                        self.add_counters(s, n as i32);
                    }
                    _ => self.endure_token(p, n, spirit),
                }
            }
            (Decision::Scry { mut cards, mut kept }, choice) => {
                let card = cards.remove(0);
                if choice == Action::Yes {
                    let lib = &mut self.players[p].library;
                    if let Some(pos) = lib.iter().position(|&x| x == card) {
                        let c = lib.remove(pos);
                        lib.insert(0, c);
                    }
                } else {
                    kept.push(card);
                }
                if !cards.is_empty() {
                    self.ask(p, Decision::Scry { cards, kept });
                } else if kept.len() > 1 {
                    self.ask(
                        p,
                        Decision::ScryOrder {
                            kept,
                            placed: Vec::new(),
                        },
                    );
                }
            }
            (Decision::ScryOrder { mut kept, mut placed }, Action::Pick(c)) => {
                kept.retain(|&x| x != c);
                placed.push(c);
                if kept.len() > 1 {
                    self.ask(p, Decision::ScryOrder { kept, placed });
                } else {
                    placed.extend(kept);
                    // `placed` runs from the top down; the library's top is its end.
                    let lib = &mut self.players[p].library;
                    lib.retain(|x| !placed.contains(x));
                    lib.extend(placed.iter().rev());
                }
            }
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
                    SearchTo::Top | SearchTo::TopOrTappedIfYouControl(_) => {}
                    SearchTo::Graveyard => self.move_to(c, Zone::Graveyard),
                    SearchTo::BattlefieldTappedStunned => {
                        self.move_to(c, Zone::Battlefield);
                        let n = self.doubling(p);
                        let o = self.obj_mut(c);
                        o.tapped = true;
                        o.stun += n;
                        self.counter_put_on(c);
                    }
                    SearchTo::BattlefieldTappedUnder(q) => {
                        self.move_to(c, Zone::Battlefield);
                        let turn = self.turn;
                        let o = self.obj_mut(c);
                        o.tapped = true;
                        if o.zone == Zone::Battlefield {
                            o.controller = q as PlayerId;
                            o.since_turn = turn;
                        }
                    }
                    SearchTo::ExileCastFreeThisTurn => {
                        self.move_to(c, Zone::Exile);
                        let turn = self.turn;
                        let o = self.obj_mut(c);
                        o.playable = Some((p, turn));
                        o.cast_free = true;
                    }
                }
                // Myriad Landscape: the rest share a basic land type with the first found.
                let what = if what == SearchFor::BasicLandSharingType {
                    const NAMES: [&str; 5] = ["Plains", "Island", "Swamp", "Mountain", "Forest"];
                    let subs = &self.def_of(c).subtypes;
                    SearchFor::LandTypes {
                        types: NAMES.map(|n| subs.iter().any(|s| s == n)),
                        basic: true,
                    }
                } else {
                    what
                };
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
            (Decision::SacrificeLandOrThis { source }, choice) => match choice {
                Action::Pick(land) => self.move_to(land, Zone::Graveyard),
                _ => {
                    if self.obj(source).zone == Zone::Battlefield && !self.cant_be_sacrificed(source) {
                        self.move_to(source, Zone::Graveyard);
                    }
                }
            },
            (Decision::SacrificeLandsThenSearch { n }, choice) => match choice {
                Action::Pick(land) => {
                    self.move_to(land, Zone::Graveyard);
                    let more = self
                        .permanents(p)
                        .any(|x| self.types_of(x).land && !self.cant_be_sacrificed(x));
                    if more {
                        self.ask(p, Decision::SacrificeLandsThenSearch { n: n + 1 });
                    } else {
                        self.scapeshift_search(p, n + 1);
                    }
                }
                _ => self.scapeshift_search(p, n),
            },
            (Decision::SacrificeOneOf { .. }, Action::Pick(c)) => self.move_to(c, Zone::Graveyard),
            (Decision::SacrificeThenReturnOther, Action::Pick(c)) => {
                self.move_to(c, Zone::Graveyard);
                self.ask_return_other(p, c);
            }
            (Decision::ReturnOtherPermanent { .. }, Action::Pick(c)) => self.move_to(c, Zone::Battlefield),
            (Decision::ReturnOtherPermanent { .. }, Action::No) => {}
            (Decision::HandToTop { left }, Action::Pick(c)) => {
                self.move_to(c, Zone::Library); // on top; the next one goes over it
                if left > 1 && !self.players[p].hand.is_empty() {
                    self.ask(p, Decision::HandToTop { left: left - 1 });
                }
            }
            (Decision::ScrollRackExile { mut exiled }, Action::Pick(c)) => {
                self.move_to(c, Zone::Exile);
                exiled.push(c);
                if self.players[p].hand.is_empty() {
                    self.scroll_rack_finish(p, exiled);
                } else {
                    self.ask(p, Decision::ScrollRackExile { exiled });
                }
            }
            (Decision::ScrollRackExile { exiled }, Action::No) => self.scroll_rack_finish(p, exiled),
            (Decision::LandFromHand, Action::Pick(c)) => self.move_to(c, Zone::Battlefield),
            (Decision::LandFromHand, Action::No) => {}
            (Decision::HideawayPick { source, cards }, Action::Pick(c)) => {
                self.move_to(c, Zone::Exile);
                self.obj_mut(source).hideaway = Some(c);
                let rest: Vec<ObjId> = cards.into_iter().filter(|&x| x != c).collect();
                self.put_among_rest(p, rest, false);
            }
            (Decision::DelinaAgain { target }, Action::Yes) => self.delina_roll(p, target),
            (Decision::DelinaAgain { .. }, Action::No) => {}
            (Decision::KeywordCounterChoice { token }, Action::Mode(m)) => {
                let k = [Keywords::REACH, Keywords::VIGILANCE, Keywords::TRAMPLE][m.min(2) as usize];
                if self.obj(token).zone == Zone::Battlefield {
                    let o = self.obj_mut(token);
                    o.keyword_counters = o.keyword_counters.with(k);
                    self.counter_put_on(token);
                }
            }
            (Decision::HulkSearch { budget }, Action::Pick(c)) => {
                let mv = self.def_of(c).cost.mana_value();
                self.move_to(c, Zone::Battlefield);
                let budget = budget.saturating_sub(mv);
                let more = self.players[p]
                    .library
                    .iter()
                    .any(|&x| self.def_of(x).types.creature && self.def_of(x).cost.mana_value() <= budget);
                if more {
                    self.ask(p, Decision::HulkSearch { budget });
                } else {
                    self.shuffle_library(p);
                }
            }
            (Decision::HulkSearch { .. }, Action::No) => self.shuffle_library(p),
            (Decision::PrimalSurge { card }, Action::Yes) => {
                self.move_to(card, Zone::Battlefield);
                self.primal_surge(p);
            }
            (Decision::PrimalSurge { .. }, Action::No) => {}
            (Decision::CommanderCourser { card }, Action::Yes) => {
                if self.obj(card).zone == Zone::Command {
                    self.move_to(card, Zone::Battlefield);
                    let zcc = self.obj(card).zcc;
                    self.until_eot.push(Modifier {
                        obj: card,
                        zcc,
                        power: 0,
                        toughness: 0,
                        keywords: Keywords::HASTE,
                        dies: None,
                    });
                    self.obj_mut(card).command_at_end = true;
                }
            }
            (Decision::CommanderCourser { .. }, Action::No) => {}
            (Decision::CreaturesFromHand, Action::Pick(c)) => {
                self.move_to(c, Zone::Battlefield);
                if self.players[p]
                    .hand
                    .iter()
                    .any(|&x| self.def_of(x).types.creature)
                {
                    self.ask(p, Decision::CreaturesFromHand);
                }
            }
            (Decision::CreaturesFromHand, Action::No) => {}
            (Decision::ReturnAnotherForIndestructible { source }, Action::Pick(c)) => {
                self.move_to(c, Zone::Hand);
                if self.obj(source).zone == Zone::Battlefield {
                    let zcc = self.obj(source).zcc;
                    self.until_eot.push(Modifier {
                        obj: source,
                        zcc,
                        power: 0,
                        toughness: 0,
                        keywords: Keywords::INDESTRUCTIBLE,
                        dies: None,
                    });
                }
            }
            (Decision::ReturnAnotherForIndestructible { .. }, Action::No) => {}
            (Decision::LandTappedDrawRepeat, Action::Pick(c)) => {
                self.move_to(c, Zone::Battlefield);
                self.obj_mut(c).tapped = true;
                self.draw(p);
                if self.players[p].hand.iter().any(|&x| self.def_of(x).types.land) {
                    self.ask(p, Decision::LandTappedDrawRepeat);
                }
            }
            (Decision::LandTappedDrawRepeat, Action::No) => {}
            (Decision::BasicFromHandTapped, Action::Pick(c)) => {
                self.move_to(c, Zone::Battlefield);
                self.obj_mut(c).tapped = true;
            }
            (Decision::BasicFromHandTapped, Action::No) => {}
            (Decision::SneakAttack, Action::Pick(c)) => {
                self.move_to(c, Zone::Battlefield);
                if self.obj(c).zone == Zone::Battlefield {
                    let zcc = self.obj(c).zcc;
                    self.until_eot.push(Modifier {
                        obj: c,
                        zcc,
                        power: 0,
                        toughness: 0,
                        keywords: Keywords::HASTE,
                        dies: None,
                    });
                    self.obj_mut(c).sacrifice_at_end = true;
                }
            }
            (Decision::SneakAttack, Action::No) => {}
            (Decision::ExiledToBattlefield { mut cards }, Action::Pick(c)) => {
                cards.retain(|&x| x != c);
                self.move_to(c, Zone::Battlefield);
                if !cards.is_empty() {
                    self.ask(p, Decision::ExiledToBattlefield { cards });
                }
            }
            (Decision::ExiledToBattlefield { .. }, Action::No) => {}
            (Decision::SacrificePermanents { left }, Action::Pick(c)) => {
                self.move_to(c, Zone::Graveyard);
                if left > 1 && self.permanents(p).any(|x| !self.cant_be_sacrificed(x)) {
                    self.ask(p, Decision::SacrificePermanents { left: left - 1 });
                }
            }
            (Decision::PayWardSacrifice { left, against, .. }, Action::Pick(c)) => {
                self.move_to(c, Zone::Graveyard);
                if left > 1 {
                    self.ask(
                        p,
                        Decision::PayWardSacrifice {
                            left: left - 1,
                            against,
                            picked: true,
                        },
                    );
                }
            }
            (Decision::PayWardSacrifice { against, .. }, Action::No) => self.counter_from(against),
            (Decision::SoulbondPair { source }, Action::Pick(c)) => {
                if self.obj(source).zone == Zone::Battlefield {
                    self.pair(source, c);
                }
            }
            (Decision::SoulbondPair { .. }, Action::No) => {}
            (Decision::LandsFromHand, choice) => {
                if let Action::Pick(land) = choice {
                    self.move_to(land, Zone::Battlefield);
                    self.obj_mut(land).tapped = true;
                    if self.players[p].hand.iter().any(|&x| self.def_of(x).types.land) {
                        self.ask(p, Decision::LandsFromHand);
                    }
                }
            }
            (
                Decision::PutPicksOntoBattlefield {
                    mut cards,
                    mut picked,
                    left,
                    creature_and_land,
                    rest_bottom,
                },
                choice,
            ) => {
                let mut more = false;
                if let Action::Pick(c) = choice {
                    cards.retain(|&x| x != c);
                    picked.push(c);
                    self.move_to(c, Zone::Battlefield);
                    let left = left.saturating_sub(1);
                    more = left > 0
                        && cards
                            .iter()
                            .any(|&x| self.pick_onto_battlefield_ok(x, &picked, creature_and_land));
                    if more {
                        self.ask(
                            p,
                            Decision::PutPicksOntoBattlefield {
                                cards: cards.clone(),
                                picked,
                                left,
                                creature_and_land,
                                rest_bottom,
                            },
                        );
                    }
                }
                if !more && rest_bottom {
                    let rest: Vec<ObjId> = cards
                        .into_iter()
                        .filter(|&x| self.obj(x).zone == Zone::Library)
                        .collect();
                    self.put_among_rest(p, rest, false);
                }
            }
            (Decision::PayToStop { cost }, Action::Yes) => {
                self.pay_mana(p, &cost, None);
                self.resolving.as_mut().expect("resolving").remaining.clear();
            }
            (Decision::PayToStop { .. }, _) => {}
            (Decision::PayUpkeep { cost, .. }, Action::Yes) => self.pay_mana(p, &cost, None),
            (Decision::PayUpkeep { source, .. }, _) => {
                if self.obj(source).zone == Zone::Battlefield && !self.cant_be_sacrificed(source) {
                    self.move_to(source, Zone::Graveyard);
                }
            }
            (Decision::DistributeCounters { left }, Action::Pick(c)) => {
                self.add_counters(c, 1);
                if left > 1 && self.creatures(p).next().is_some() {
                    self.ask(p, Decision::DistributeCounters { left: left - 1 });
                }
            }
            (Decision::LandFromHandTapped, Action::Pick(c)) => {
                self.move_to(c, Zone::Battlefield);
                self.obj_mut(c).tapped = true;
            }
            (Decision::LandFromHandTapped, _) => {}
            (Decision::ManifestDread { cards }, Action::Pick(c)) => {
                self.manifest(p, c);
                for x in cards {
                    if x != c && self.obj(x).zone == Zone::Library {
                        self.move_to(x, Zone::Graveyard);
                    }
                }
            }
            (Decision::TurnFaceUp, Action::Pick(c)) => self.obj_mut(c).face_down = false,
            (Decision::TurnFaceUp, _) => {}
            (Decision::ChooseNewTarget { item, slot }, choice) => {
                if let Action::Target(t) = choice {
                    let chosen = match t {
                        Target::Player(q) => Chosen::Player(q),
                        Target::Object(x) => Chosen::Object(x, self.obj(x).zcc),
                    };
                    self.items[item].targets[slot as usize] = chosen;
                }
                self.ask_new_target(p, item, slot as usize + 1);
            }
            (d, a) => unreachable!("legal action {a:?} unhandled for {d:?}"),
        }
        if self.decision.is_none() {
            self.continue_resolving();
        }
    }

    /// Scroll Rack: that many cards from the top of the library into the hand, then the exiled cards
    /// on top in any order (`ScryOrder`).
    fn scroll_rack_finish(&mut self, p: PlayerId, exiled: Vec<ObjId>) {
        for _ in 0..exiled.len() {
            if let Some(&c) = self.players[p].library.last() {
                self.move_to(c, Zone::Hand);
            }
        }
        let mut back = Vec::new();
        for c in exiled {
            if self.obj(c).zone == Zone::Exile {
                self.move_to(c, Zone::Library);
                back.push(c);
            }
        }
        if back.len() > 1 {
            back.reverse(); // the last one put back is on top
            self.ask(
                p,
                Decision::ScryOrder {
                    kept: back,
                    placed: Vec::new(),
                },
            );
        }
    }

    /// Scapeshift: search for up to `n` land cards, onto the battlefield tapped, then shuffle.
    fn scapeshift_search(&mut self, p: PlayerId, n: u8) {
        let to = SearchTo::Battlefield { tapped: true };
        if n == 0 {
            self.finish_search(p, to, None);
        } else {
            self.ask(
                p,
                Decision::Search {
                    what: SearchFor::Land,
                    to,
                    left: n,
                },
            );
        }
    }

    /// Rise of the Witch-king: you may return a permanent card from your graveyard other than the
    /// creature you sacrificed (`except`).
    fn ask_return_other(&mut self, p: PlayerId, except: ObjId) {
        let any = self.players[p]
            .graveyard
            .iter()
            .any(|&c| c != except && self.def_of(c).types.is_permanent());
        if any {
            self.ask(p, Decision::ReturnOtherPermanent { except });
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
                    Recipient::You => vec![Target::Player(item.controller)],
                    Recipient::EachPlayer => vec![
                        Target::Player(item.controller),
                        Target::Player(opponent(item.controller)),
                    ],
                    Recipient::EachCreatureAndPlaneswalker => self
                        .battlefield
                        .iter()
                        .copied()
                        .filter(|&id| self.types_of(id).creature || self.types_of(id).planeswalker)
                        .map(Target::Object)
                        .collect(),
                    Recipient::EachCreature { without_flying } => self
                        .battlefield
                        .iter()
                        .copied()
                        .filter(|&id| self.types_of(id).creature)
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
            Effect::Mobilize { def, n } => {
                let p = item.controller;
                let n = n.map_or_else(
                    || {
                        self.players[p]
                            .graveyard
                            .iter()
                            .filter(|&&c| self.def_of(c).types.creature)
                            .count()
                    },
                    |n| n as usize,
                );
                for _ in 0..n {
                    self.create_token(p, def);
                    for id in self.last_created.clone() {
                        let o = self.obj_mut(id);
                        o.tapped = true;
                        o.sacrifice_at_end = true;
                        if let Some(c) = self.combat.as_mut() {
                            c.attackers.push(id); // CR 506.3a: attacking without being declared
                        }
                    }
                }
            }
            Effect::CreateTokensPowerTapped { def } => {
                let power = if self.obj(src).zone == Zone::Battlefield {
                    self.power(src)
                } else {
                    self.obj(src).last_power
                };
                let mut made = Vec::new();
                for _ in 0..power.max(0) {
                    self.create_token(item.controller, def);
                    made.extend(self.last_created.iter().copied());
                }
                for &t in &made {
                    self.obj_mut(t).tapped = true;
                }
                self.last_created = made;
            }
            Effect::CreateToken { who, def, n } => {
                // Every token this effect makes is "just created" (for "They gain ..."), not only the last.
                let mut made = Vec::new();
                for p in self.players_for(item, legal, who) {
                    for _ in 0..n {
                        self.create_token(p, def);
                        made.extend(self.last_created.iter().copied());
                    }
                }
                self.last_created = made;
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
                                then_draw: 0,
                            },
                        );
                    }
                }
            }
            Effect::DiscardThenDraw(n) => {
                let p = item.controller;
                if !self.players[p].hand.is_empty() {
                    self.ask(
                        p,
                        Decision::Discard {
                            left: 1,
                            optional: false,
                            then_draw: n,
                        },
                    );
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
                            then_draw: 1,
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
                    let any = self.players[victim].hand.iter().any(|&c| !self.types_of(c).land);
                    if any {
                        self.ask(item.controller, Decision::PickDiscard { victim });
                    }
                }
            }
            Effect::Endure { n, spirit } => {
                if self.obj(src).zone == Zone::Battlefield {
                    self.ask(item.controller, Decision::Endure { n, spirit });
                } else {
                    self.endure_token(item.controller, n, spirit); // CR 701.62b: gone, so a Spirit
                }
            }
            Effect::Scry { n } => {
                self.fire(item.controller, When::YouScryOrSurveil, None);
                let lib = &self.players[item.controller].library;
                let cards: Vec<ObjId> = lib.iter().rev().take(n as usize).copied().collect();
                if !cards.is_empty() {
                    self.ask(
                        item.controller,
                        Decision::Scry {
                            cards,
                            kept: Vec::new(),
                        },
                    );
                }
            }
            Effect::Surveil { n } => {
                self.fire(item.controller, When::YouScryOrSurveil, None);
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
                    SearchFor::GreenCreatureMvAtMostX => SearchFor::ColoredCreatureMvAtMost(4, item.x as u32),
                    w => w,
                };
                let to = match to {
                    SearchTo::TopOrTappedIfYouControl(sub) => {
                        if self.permanents(item.controller).any(|x| self.has_subtype(x, sub)) {
                            SearchTo::Battlefield { tapped: true }
                        } else {
                            SearchTo::Top
                        }
                    }
                    t => t,
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
                        if self.uncounterable(id) {
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
                        .is_some_and(|x| self.obj(x).zone == Zone::Exile && self.types_of(x).creature)
                });
                if exiled_creature {
                    let opp = opponent(item.controller);
                    self.lose_life(opp, n);
                    self.gain_life(item.controller, n);
                }
            }
            Effect::ChooseExileFromHand { slot } => {
                if let Some(victim) = self
                    .players_for(item, legal, Who::TargetPlayer(slot))
                    .into_iter()
                    .next()
                {
                    if self.players[victim].hand.iter().any(|&c| !self.types_of(c).land) {
                        self.ask(item.controller, Decision::PickExile { victim });
                    }
                }
            }
            Effect::BounceDrawIfToken(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    if self.obj(id).zone == Zone::Battlefield {
                        let token = self.is_token(id);
                        self.move_to(id, Zone::Hand);
                        if token {
                            self.draw(item.controller);
                        }
                    }
                }
            }
            Effect::MillSourcePower(n) => {
                let power = if self.obj(src).zone == Zone::Battlefield {
                    self.power(src)
                } else {
                    self.obj(src).last_power
                };
                for p in self.players_for(item, legal, Who::TargetPlayer(n)) {
                    for _ in 0..power.max(0) {
                        if let Some(&c) = self.players[p].library.last() {
                            self.move_to(c, Zone::Graveyard);
                        }
                    }
                }
            }
            Effect::MayExileFromGraveyard { n, creature_only } => {
                let p = item.controller;
                let have = self.players[p]
                    .graveyard
                    .iter()
                    .filter(|&&c| !creature_only || self.def_of(c).types.creature)
                    .count();
                if have < n as usize {
                    self.resolving.as_mut().expect("resolving").remaining.clear();
                } else {
                    self.ask(
                        p,
                        Decision::ExileFromGraveyard {
                            left: n,
                            creature_only,
                            started: false,
                        },
                    );
                }
            }
            Effect::TopCardsOntoBattlefield {
                n,
                up_to,
                noncreature_nonland,
                max_mv,
                rest_to_hand,
            } => {
                let p = item.controller;
                let cards: Vec<ObjId> = self.players[p]
                    .library
                    .iter()
                    .rev()
                    .take(n as usize)
                    .copied()
                    .collect();
                if !cards.is_empty() {
                    let left = if up_to == 0 { u8::MAX } else { up_to };
                    self.ask(
                        p,
                        Decision::PutFromAmong {
                            cards,
                            left,
                            noncreature_nonland,
                            max_mv,
                            rest_to_hand,
                        },
                    );
                }
            }
            Effect::ChooseExileFromHandLinked { slot } => {
                if let Some(victim) = self
                    .players_for(item, legal, Who::TargetPlayer(slot))
                    .into_iter()
                    .next()
                {
                    if self.players[victim].hand.iter().any(|&c| !self.types_of(c).land) {
                        self.ask(item.controller, Decision::PickExileLinked { victim, source: src });
                    }
                }
            }
            Effect::LinkedOwnerCreatesSpirit { spirit } => {
                if let Some(c) = self.obj(src).linked_card {
                    let owner = self.obj(c).owner;
                    let x = self.def_of(c).cost.mana_value();
                    self.endure_token(owner, x as u8, spirit);
                }
            }
            Effect::ExileSourceSpell => {
                if self.obj(src).zone == Zone::Stack {
                    self.move_to(src, Zone::Exile);
                }
            }
            Effect::ReturnPermanentsDifferentNames => {
                let p = item.controller;
                if self.players[p]
                    .graveyard
                    .iter()
                    .any(|&c| self.def_of(c).types.is_permanent())
                {
                    self.ask(p, Decision::ReturnDifferentNames { names: Vec::new() });
                }
            }
            Effect::DiscardUpToThenDrawThatMany(n) => {
                let p = item.controller;
                if !self.players[p].hand.is_empty() {
                    self.ask(
                        p,
                        Decision::DiscardUpToDraw {
                            left: n,
                            discarded: 0,
                        },
                    );
                }
            }
            Effect::ExileRandomFromGraveyardPlayable => {
                let p = item.controller;
                let n = self.players[p].graveyard.len();
                if n > 0 {
                    let c = self.players[p].graveyard[self.rng.below(n)];
                    self.move_to(c, Zone::Exile);
                    let turn = self.turn;
                    self.obj_mut(c).playable = Some((p, turn));
                }
            }
            Effect::ExileTopOfDamagedCastFree => {
                let p = item.controller;
                let q = opponent(p);
                let x = item.amount.max(0);
                for _ in 0..x {
                    if let Some(&c) = self.players[q].library.last() {
                        self.move_to(c, Zone::Exile);
                        if !self.types_of(c).land && self.def_of(c).cost.mana_value() <= x as u32 {
                            self.obj_mut(c).cast_free = true;
                            self.free_casts.push(c);
                        }
                    }
                }
                self.free_caster = p;
            }
            Effect::GateAttackingAtLeast(n) => {
                let p = item.controller;
                let attacking = self.combat.as_ref().map_or(0, |c| {
                    c.attackers
                        .iter()
                        .filter(|&&a| self.obj(a).controller == p)
                        .count()
                });
                if attacking < n as usize {
                    self.resolving.as_mut().expect("resolving").remaining.clear();
                }
            }
            Effect::GrantAttached(keywords) => {
                if let Some((id, zcc)) = self.obj(src).attached_to {
                    if self.obj(id).zcc == zcc && self.obj(id).zone == Zone::Battlefield {
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
            }
            Effect::AttackDespiteDefenderSelf => {
                if self.obj(src).zone == Zone::Battlefield {
                    let turn = self.turn;
                    self.obj_mut(src).defender_ignored_turn = turn;
                }
            }
            Effect::CountersAll { who, n } => {
                for p in self.players_for(item, legal, who) {
                    for c in self.creatures(p).collect::<Vec<_>>() {
                        self.add_counters(c, n);
                    }
                }
            }
            Effect::ExileTopToPlayAll(n) => {
                let p = item.controller;
                // Until the end of your next turn (CR 611.2c).
                let until = if self.active == p {
                    self.turn + 2
                } else {
                    self.turn + 1
                };
                for _ in 0..n {
                    if let Some(&c) = self.players[p].library.last() {
                        self.move_to(c, Zone::Exile);
                        self.obj_mut(c).playable = Some((p, until));
                    }
                }
            }
            Effect::SacrificeCreatedAtEnd => {
                for id in self.last_created.clone() {
                    if self.obj(id).zone == Zone::Battlefield {
                        self.obj_mut(id).sacrifice_at_end = true;
                    }
                }
            }
            Effect::DiscardIfPlayerTarget(slot) => {
                if let Some(&Chosen::Player(p)) = item.targets.get(slot as usize) {
                    if legal.get(slot as usize).copied().unwrap_or(false) && !self.players[p].hand.is_empty()
                    {
                        self.ask(
                            p,
                            Decision::Discard {
                                left: 1,
                                optional: false,
                                then_draw: 0,
                            },
                        );
                    }
                }
            }
            Effect::StunSelf(n) => {
                if self.obj(src).zone == Zone::Battlefield {
                    let n = n as i32 * self.doubling(self.obj(src).controller);
                    self.obj_mut(src).stun += n;
                    self.counter_put_on(src);
                }
            }
            Effect::AdditionalCombatAndMain => {
                // After this main phase: the combat, then a main phase, then what came next.
                let resume = if self.step == Step::Main1 {
                    Step::BeginCombat
                } else {
                    Step::End
                };
                if matches!(self.step, Step::Main1 | Step::Main2) {
                    let resume = self.extra_main_resume.unwrap_or(resume);
                    self.extra_combats.push(resume);
                }
            }
            Effect::DelayedOnNextAttackThisTurn(trigger) => {
                let entry = (src, trigger, item.controller, self.turn);
                self.delayed_on_next_attack.push(entry);
            }
            Effect::DamageEqualCreaturesYouControl(slot) => {
                let n = self.creatures(item.controller).count() as i32;
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if n > 0 {
                        self.deal_damage(src, Target::Object(id), n);
                    }
                }
            }
            Effect::MillLandsYouControl => {
                let p = item.controller;
                let n = self.permanents(p).filter(|&x| self.types_of(x).land).count();
                for _ in 0..n {
                    if let Some(&c) = self.players[p].library.last() {
                        self.move_to(c, Zone::Graveyard);
                    }
                }
            }
            Effect::CountersPerLandCardInGraveyard(slot) => {
                let n = self.players[item.controller]
                    .graveyard
                    .iter()
                    .filter(|&&c| self.def_of(c).types.land)
                    .count() as i32;
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    self.add_counters(id, n);
                }
            }
            Effect::SlotEnduresSourceCounters { slot, spirit } => {
                let o = self.obj(src);
                let kinds = Keywords::ALL
                    .iter()
                    .filter(|(k, _)| o.keyword_counters.has(*k))
                    .count() as i32;
                let n = (o.counters.max(0) + o.stun.max(0) + kinds).min(255) as u8;
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if self.obj(id).zone == Zone::Battlefield {
                        self.ask(
                            item.controller,
                            Decision::EndureOther {
                                target: id,
                                n,
                                spirit,
                            },
                        );
                    } else {
                        self.endure_token(item.controller, n, spirit); // CR 701.62b
                    }
                }
            }
            Effect::MayContinue => self.ask(item.controller, Decision::May),
            Effect::AttachToCreated => {
                let made = self.last_created.last().copied();
                if let Some(c) = made {
                    if self.obj(src).zone == Zone::Battlefield && self.obj(c).zone == Zone::Battlefield {
                        let zcc = self.obj(c).zcc;
                        self.obj_mut(src).attached_to = Some((c, zcc));
                    }
                }
            }
            Effect::ReturnUpToPermanentCardsToHand(n) => {
                let p = item.controller;
                if self.players[p]
                    .graveyard
                    .iter()
                    .any(|&c| self.def_of(c).types.is_permanent())
                {
                    self.ask(p, Decision::GraveyardToHand { left: n });
                }
            }
            Effect::ShuffleUpToFromGraveyard { slot, n } => {
                if let Some(owner) = self
                    .players_for(item, legal, Who::TargetPlayer(slot))
                    .into_iter()
                    .next()
                {
                    if !self.players[owner].graveyard.is_empty() {
                        self.ask(
                            item.controller,
                            Decision::GraveyardShuffle {
                                owner,
                                left: n,
                                picked: false,
                            },
                        );
                    }
                }
            }
            Effect::GrantHarmonize(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    self.obj_mut(id).harmonize_eot = true;
                }
            }
            Effect::CounterUnlessPaysX(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let target = self
                        .items
                        .iter()
                        .rev()
                        .find(|i| i.kind == StackKind::Spell && i.source == id)
                        .map(|t| t.controller);
                    if let Some(payer) = target {
                        if !self.uncounterable(id) {
                            let cost = ManaCost {
                                generic: item.x,
                                ..ManaCost::default()
                            };
                            self.ask(payer, Decision::PayOrCountered { cost, against: id });
                        }
                    }
                }
            }
            Effect::MayDiscardReflexiveIfNonland(trigger) => {
                let p = item.controller;
                if !self.players[p].hand.is_empty() {
                    self.ask(p, Decision::DiscardReflexive { source: src, trigger });
                }
            }
            Effect::GrantYourCreaturesWithCounters(keywords) => {
                for c in self.creatures(item.controller).collect::<Vec<_>>() {
                    if self.has_counter(c) {
                        let zcc = self.obj(c).zcc;
                        self.until_eot.push(Modifier {
                            obj: c,
                            zcc,
                            power: 0,
                            toughness: 0,
                            keywords,
                            dies: None,
                        });
                    }
                }
            }
            Effect::DiscardTwoUnlessCreature => {
                let p = item.controller;
                if !self.players[p].hand.is_empty() {
                    self.ask(p, Decision::DiscardTwoUnlessCreature { second: false });
                }
            }
            Effect::EndureX { spirit } => {
                let n = item.x;
                if self.obj(src).zone == Zone::Battlefield {
                    self.ask(item.controller, Decision::Endure { n, spirit });
                } else {
                    self.endure_token(item.controller, n, spirit); // CR 701.62b
                }
            }
            Effect::AddColorless(n) => {
                for _ in 0..n {
                    self.players[item.controller].pool.push(ManaSource {
                        colorless: true,
                        ..ManaSource::default()
                    });
                }
            }
            Effect::BlinkSlot(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if self.obj(id).zone == Zone::Battlefield {
                        self.move_to(id, Zone::Exile);
                        self.move_to(id, Zone::Battlefield);
                        self.obj_mut(id).controller = item.controller;
                    }
                }
            }
            Effect::DrawPerCreatureYouControl => {
                let n = self.creatures(item.controller).count();
                for _ in 0..n {
                    self.draw(item.controller);
                }
            }
            Effect::GainLifePerCreaturePowerAtLeast { per, power } => {
                let n = self
                    .creatures(item.controller)
                    .filter(|&c| self.power(c) >= power)
                    .count() as i32;
                if n > 0 {
                    self.gain_life(item.controller, per * n);
                }
            }
            Effect::CreateTokensX { def } => {
                for _ in 0..item.x {
                    self.create_token(item.controller, def);
                }
            }
            Effect::ReturnCreatureYouControl => {
                if self.creatures(item.controller).next().is_some() {
                    self.ask(item.controller, Decision::ReturnOwnCreature);
                }
            }
            Effect::BounceAllCreatures { tokens } => {
                let all: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| self.types_of(c).creature && self.is_token(c) == tokens)
                    .collect();
                for c in all {
                    self.move_to(c, Zone::Hand);
                }
            }
            Effect::DecayedCounter(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if self.obj(id).zone == Zone::Battlefield {
                        self.obj_mut(id).decayed = true;
                        self.counter_put_on(id);
                    }
                }
            }
            Effect::MayBeholdThen => {
                let p = item.controller;
                if self.can_behold(p, src) {
                    self.ask(p, Decision::May);
                } else {
                    self.resolving.as_mut().expect("resolving").remaining.clear();
                }
            }
            Effect::SubtypeUntilEotSelf(sub) => {
                if self.obj(src).zone == Zone::Battlefield {
                    let turn = self.turn;
                    self.obj_mut(src).eot_subtype = Some((turn, sub));
                }
            }
            Effect::CopyLinkedExileAttacking => {
                let zcc = self.obj(src).zcc;
                let linked: Vec<ObjId> = self
                    .links
                    .iter()
                    .filter(|&&(s, z, card)| s == src && z == zcc && self.obj(card).zone == Zone::Exile)
                    .map(|&(_, _, card)| card)
                    .collect();
                for card in linked {
                    let def = self.obj(card).def;
                    let id = self.add_card(item.controller, def, Zone::Hand);
                    self.obj_mut(id).copy_token = true;
                    self.move_to(id, Zone::Battlefield);
                    let o = self.obj_mut(id);
                    o.tapped = true;
                    o.sacrifice_at_end = true;
                    if let Some(c) = self.combat.as_mut() {
                        c.attackers.push(id); // CR 506.3a: attacking without being declared
                    }
                }
            }
            Effect::ExileCastCopyFree(slot) => {
                if let Some(card) = Game::slot_object(item, legal, slot) {
                    if self.obj(card).zone == Zone::Graveyard {
                        self.move_to(card, Zone::Exile);
                        let def = self.obj(card).def;
                        let copy = self.add_card(item.controller, def, Zone::Exile);
                        let o = self.obj_mut(copy);
                        o.card_copy = true;
                        o.cast_free = true;
                        self.free_casts.push(copy);
                        self.free_caster = item.controller;
                    }
                }
            }
            Effect::SearchMonocoloredOpponentPicks => {
                let p = item.controller;
                if self.players[p]
                    .library
                    .iter()
                    .any(|&c| self.mono_search_ok(c, &[]))
                {
                    self.ask(p, Decision::SearchMonoNames { picked: Vec::new() });
                } else {
                    self.shuffle_library(p);
                }
            }
            Effect::CopyNextSpellThisTurn(trigger) => {
                let entry = (src, trigger, item.controller, self.turn);
                self.copy_next_spell.push(entry);
            }
            Effect::CopySpellNewTargets => {
                let spell = item.extra.and_then(|c| {
                    self.items
                        .iter()
                        .rev()
                        .find(|i| i.kind == StackKind::Spell && i.source == c)
                        .cloned()
                });
                if let Some(spell) = spell {
                    self.pending_copies.push(StackItem {
                        kind: StackKind::SpellCopy,
                        controller: item.controller,
                        ..spell
                    });
                }
            }
            Effect::UntapCreaturesYouControl => {
                for c in self.creatures(item.controller).collect::<Vec<_>>() {
                    self.untap_by_effect(c);
                }
            }
            Effect::DelayedOnAttackThisTurn(trigger) => {
                let entry = (src, trigger, item.controller, self.turn);
                self.delayed_on_attack.push(entry);
            }
            Effect::DelayedThisTurn { when, trigger } => {
                let entry = (when, src, trigger, item.controller, self.turn);
                self.delayed_this_turn.push(entry);
            }
            Effect::MayDiscardThen => {
                if self.players[item.controller].hand.is_empty() {
                    self.resolving.as_mut().expect("resolving").remaining.clear();
                } else {
                    self.ask(item.controller, Decision::MayDiscardOrStop);
                }
            }
            Effect::ExileUntilNonlandCastFree { max_mv } => {
                let p = item.controller;
                while let Some(&c) = self.players[p].library.last() {
                    self.move_to(c, Zone::Exile);
                    if self.def_of(c).types.land {
                        continue;
                    }
                    if self.def_of(c).cost.mana_value() <= max_mv {
                        self.obj_mut(c).cast_free = true;
                        self.free_casts.push(c);
                        self.free_caster = p;
                        self.free_casts_to_hand = true;
                    } else {
                        self.move_to(c, Zone::Hand);
                    }
                    break;
                }
            }
            Effect::BecomeCreatureUntilEot {
                power,
                toughness,
                subtype,
                keywords,
            } => {
                if self.obj(src).zone == Zone::Battlefield {
                    let turn = self.turn;
                    self.obj_mut(src).eot_creature = Some((turn, power, toughness, subtype, keywords));
                }
            }
            Effect::PreventNextFromChosenSource { trigger } => {
                self.ask(
                    item.controller,
                    Decision::ChooseDamageSource { card: src, trigger },
                );
            }
            Effect::DamageExtrasControllerAndDrawAmount => {
                let n = item.amount;
                if let Some(s) = item.extra {
                    let them = self.obj(s).controller;
                    if n > 0 {
                        self.deal_damage(src, Target::Player(them), n);
                    }
                }
                for _ in 0..n.max(0) {
                    self.draw(item.controller);
                }
            }
            Effect::CopySourceSpell { storm } => {
                let n = if storm { item.amount.max(0) as u32 } else { 1 };
                let snap = self
                    .cast_snapshots
                    .iter()
                    .rev()
                    .find(|s| s.source == src)
                    .cloned();
                if let Some(snap) = snap {
                    for _ in 0..n {
                        let copy = StackItem {
                            kind: StackKind::SpellCopy,
                            controller: item.controller,
                            ..snap.clone()
                        };
                        self.items.push(copy);
                    }
                }
            }
            Effect::DestroyLandControllerFetchesStunned(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let p = self.obj(id).controller;
                    self.destroy(id);
                    self.ask(
                        p,
                        Decision::Search {
                            what: SearchFor::BasicLand,
                            to: SearchTo::BattlefieldTappedStunned,
                            left: 1,
                        },
                    );
                }
            }
            Effect::StillnessCheck => {
                let p = item.controller;
                if self.players[p].library.is_empty() {
                    if self.obj(src).zone == Zone::Battlefield {
                        self.move_to(src, Zone::Exile);
                    }
                    let k = self.players[p].graveyard.len().min(5) as u8;
                    if k > 0 {
                        self.ask(p, Decision::PutOnTopFromGraveyard { left: k });
                    }
                }
            }
            Effect::SpiritDragonsUpkeep => self.next_dragon_color(item.controller, 0, Vec::new()),
            Effect::GrantAllUntilYourNextTurn(keywords) => {
                let p = item.controller;
                for c in self.creatures(p).collect::<Vec<_>>() {
                    let zcc = self.obj(c).zcc;
                    self.until_your_next_turn.push((c, zcc, keywords, p));
                }
            }
            Effect::OffspringCopy => {
                let def = self.obj(src).def;
                self.last_created.clear();
                for _ in 0..self.token_doubling(item.controller) {
                    let id = self.add_card(item.controller, def, Zone::Hand);
                    let o = self.obj_mut(id);
                    o.copy_token = true;
                    o.copy_base_pt = Some((1, 1));
                    self.move_to(id, Zone::Battlefield);
                    self.last_created.push(id);
                }
            }
            Effect::AddManaAnyColorsX => {
                let m = ManaSource::of(&crate::Color::ALL);
                self.players[item.controller]
                    .pool
                    .extend(std::iter::repeat_n(m, item.x as usize));
            }
            Effect::CopyTokenTwiceIfSubtype { slot, sub } => {
                if let Some(t) = Game::slot_object(item, legal, slot) {
                    let n = if self.has_subtype(t, sub) { 2 } else { 1 };
                    let def = self.obj(t).def;
                    let copy = self.obj(t).copy_token || !self.defs[def].token;
                    let base = self.obj(t).copy_base_pt;
                    for _ in 0..n {
                        for _ in 0..self.token_doubling(item.controller) {
                            let id = self.add_card(item.controller, def, Zone::Hand);
                            let o = self.obj_mut(id);
                            o.copy_token = copy;
                            o.copy_base_pt = base;
                            self.move_to(id, Zone::Battlefield);
                        }
                    }
                }
            }
            Effect::ExileControllerMayFetchBasic(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let p = self.obj(id).controller;
                    self.move_to(id, Zone::Exile);
                    // "may search": the search itself allows finding nothing (CR 701.19b).
                    self.ask(
                        p,
                        Decision::Search {
                            what: SearchFor::BasicLand,
                            to: SearchTo::Battlefield { tapped: true },
                            left: 1,
                        },
                    );
                }
            }
            Effect::DoubleCountersEachYours => {
                for c in self.creatures(item.controller).collect::<Vec<_>>() {
                    let n = self.obj(c).counters;
                    self.add_counters(c, n);
                }
            }
            Effect::DestroyAllCreaturesPowerAtLeast(n) => {
                let all: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| self.types_of(c).creature && self.power(c) >= n)
                    .collect();
                for c in all {
                    self.destroy(c);
                }
            }
            Effect::PumpOthersAndGrant {
                power,
                toughness,
                keywords,
            } => {
                let others: Vec<ObjId> = self.creatures(item.controller).filter(|&c| c != src).collect();
                for id in others {
                    let zcc = self.obj(id).zcc;
                    self.until_eot.push(Modifier {
                        obj: id,
                        zcc,
                        power,
                        toughness,
                        keywords,
                        dies: None,
                    });
                }
            }
            Effect::CopyThenSuspendSpell => {
                let spell = item.extra.and_then(|c| {
                    self.items
                        .iter()
                        .rev()
                        .find(|i| i.kind == StackKind::Spell && i.source == c)
                        .cloned()
                });
                if let Some(spell) = spell {
                    let c = spell.source;
                    self.items.push(StackItem {
                        kind: StackKind::SpellCopy,
                        controller: item.controller,
                        ..spell
                    });
                    self.items
                        .retain(|i| !(i.kind == StackKind::Spell && i.source == c));
                    self.move_to(c, Zone::Exile);
                    self.obj_mut(c).time_counters = 4;
                }
            }
            Effect::OthersBecomeCopyOfTarget(slot) => {
                if let Some(t) = Game::slot_object(item, legal, slot) {
                    let def = self.obj(t).def;
                    let others: Vec<ObjId> = self.creatures(item.controller).filter(|&x| x != t).collect();
                    for id in others {
                        let (zcc, before) = (self.obj(id).zcc, self.obj(id).def);
                        self.copy_until_eot.push((id, zcc, before));
                        self.become_copy(id, def);
                    }
                }
            }
            Effect::LoseLifeCastSpellManaValue => {
                if let Some(s) = item.extra {
                    let o = self.obj(s);
                    let mv = self.def_of(s).cost.with_x(o.x).mana_value() as i32;
                    self.lose_life(item.controller, mv);
                }
            }
            Effect::ShrinkAllWithoutSubtypeX(sub) => {
                let x = item.x as i32;
                let ids: Vec<ObjId> = (0..2)
                    .flat_map(|q| self.creatures(q).collect::<Vec<_>>())
                    .filter(|&c| !self.has_subtype(c, sub))
                    .collect();
                for id in ids {
                    let zcc = self.obj(id).zcc;
                    self.until_eot.push(Modifier {
                        obj: id,
                        zcc,
                        power: -x,
                        toughness: -x,
                        keywords: Keywords::NONE,
                        dies: None,
                    });
                }
            }
            Effect::ExileControllerGainsPower(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let (controller, power) = (self.obj(id).controller, self.power(id));
                    self.move_to(id, Zone::Exile);
                    if power > 0 {
                        self.gain_life(controller, power);
                    }
                }
            }
            Effect::DestroyAllCreaturesIfXAtLeast(n) => {
                if item.x >= n {
                    let all: Vec<ObjId> = self
                        .battlefield
                        .iter()
                        .copied()
                        .filter(|&c| self.types_of(c).creature)
                        .collect();
                    for c in all {
                        self.destroy(c);
                    }
                }
            }
            Effect::UntapAllNonland => {
                let all: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| !self.types_of(c).land)
                    .collect();
                for c in all {
                    self.untap_by_effect(c);
                }
            }
            Effect::DestroyControllerMayFetchBasic(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let p = self.obj(id).controller;
                    self.destroy(id);
                    self.ask(
                        p,
                        Decision::Search {
                            what: SearchFor::BasicLand,
                            to: SearchTo::Battlefield { tapped: false },
                            left: 1,
                        },
                    );
                }
            }
            Effect::BounceOpponentsNonland => {
                let theirs: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&x| self.obj(x).controller != item.controller && !self.types_of(x).land)
                    .collect();
                for x in theirs {
                    self.move_to(x, Zone::Hand);
                }
            }
            Effect::CountersOnYourSubtype(sub) => {
                for c in self.creatures(item.controller).collect::<Vec<_>>() {
                    if self.has_subtype(c, sub) {
                        self.add_counters(c, 1);
                    }
                }
            }
            Effect::ReturnLandsFromGraveyardTapped => {
                let p = item.controller;
                let lands: Vec<ObjId> = self.players[p]
                    .graveyard
                    .iter()
                    .copied()
                    .filter(|&c| self.def_of(c).types.land)
                    .collect();
                for c in lands {
                    self.move_to(c, Zone::Battlefield);
                    self.obj_mut(c).tapped = true;
                }
            }
            Effect::InsectOrCopyWithSixLands { def } => {
                let p = item.controller;
                let lands = self.permanents(p).filter(|&x| self.types_of(x).land).count();
                if lands >= 6 {
                    let original = self.obj(src).def;
                    for _ in 0..self.token_doubling(p) {
                        let id = self.add_card(p, original, Zone::Hand);
                        self.obj_mut(id).copy_token = true;
                        self.move_to(id, Zone::Battlefield);
                    }
                } else {
                    self.create_token(p, def);
                }
            }
            Effect::Scapeshift => {
                let p = item.controller;
                if self
                    .permanents(p)
                    .any(|x| self.types_of(x).land && !self.cant_be_sacrificed(x))
                {
                    self.ask(p, Decision::SacrificeLandsThenSearch { n: 0 });
                } else {
                    self.scapeshift_search(p, 0);
                }
            }
            Effect::SacrificeGreatestPower(who) => {
                for p in self.players_for(item, legal, who) {
                    let mine: Vec<ObjId> = self
                        .creatures(p)
                        .filter(|&c| !self.cant_be_sacrificed(c))
                        .collect();
                    let Some(best) = mine.iter().map(|&c| self.power(c)).max() else {
                        continue;
                    };
                    let cards: Vec<ObjId> = mine.into_iter().filter(|&c| self.power(c) == best).collect();
                    if cards.len() == 1 {
                        self.move_to(cards[0], Zone::Graveyard);
                    } else {
                        self.ask(p, Decision::SacrificeOneOf { cards });
                        return;
                    }
                }
            }
            Effect::SacrificeThenReturnOther => {
                let p = item.controller;
                if self.creatures(p).any(|c| !self.cant_be_sacrificed(c)) {
                    self.ask(p, Decision::SacrificeThenReturnOther);
                }
            }
            Effect::TapStopActivations(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    self.become_tapped(id);
                    self.obj_mut(id).activations_stopped_turn = self.turn;
                }
            }
            Effect::LookTopReorder(n) => {
                let p = item.controller;
                let lib = &self.players[p].library;
                let kept: Vec<ObjId> = lib.iter().rev().take(n as usize).copied().collect();
                if kept.len() > 1 {
                    self.ask(
                        p,
                        Decision::ScryOrder {
                            kept,
                            placed: Vec::new(),
                        },
                    );
                }
            }
            Effect::TransformSelf => {
                let d = &self.defs[self.obj(src).def];
                if self.obj(src).zone == Zone::Battlefield && d.back.is_some() {
                    let o = self.obj_mut(src);
                    o.transformed = !o.transformed;
                }
            }
            Effect::ReturnSelfTappedTransformed => {
                if self.obj(src).zone == Zone::Graveyard && self.defs[self.obj(src).def].back.is_some() {
                    self.obj_mut(src).transformed = true;
                    self.move_to(src, Zone::Battlefield);
                    if self.obj(src).zone == Zone::Battlefield {
                        self.obj_mut(src).tapped = true;
                    } else {
                        self.obj_mut(src).transformed = false;
                    }
                }
            }
            Effect::ExileSelfReturn { transformed } => {
                if self.obj(src).zone == Zone::Battlefield {
                    self.move_to(src, Zone::Exile);
                    // CR 712.14: only a double-faced card can return transformed.
                    if self.obj(src).zone == Zone::Exile
                        && (!transformed || self.defs[self.obj(src).def].back.is_some())
                    {
                        self.obj_mut(src).transformed = transformed;
                        self.move_to(src, Zone::Battlefield);
                        if self.obj(src).zone != Zone::Battlefield {
                            self.obj_mut(src).transformed = false;
                        }
                    }
                }
            }
            Effect::RevealAmountPutCreatureAndLand => {
                let p = item.controller;
                let cards: Vec<ObjId> = self.players[p]
                    .library
                    .iter()
                    .rev()
                    .take(item.amount.max(0) as usize)
                    .copied()
                    .collect();
                if !cards.is_empty() {
                    self.ask(
                        p,
                        Decision::PutPicksOntoBattlefield {
                            cards,
                            picked: Vec::new(),
                            left: 2,
                            creature_and_land: true,
                            rest_bottom: true,
                        },
                    );
                }
            }
            Effect::MillPutCreatures { n, up_to } => {
                let p = item.controller;
                let mut milled = Vec::new();
                for _ in 0..n {
                    if let Some(&c) = self.players[p].library.last() {
                        self.move_to(c, Zone::Graveyard);
                        if self.obj(c).zone == Zone::Graveyard {
                            milled.push(c);
                        }
                    }
                }
                let cards: Vec<ObjId> = milled
                    .into_iter()
                    .filter(|&c| self.def_of(c).types.creature)
                    .collect();
                if !cards.is_empty() && up_to > 0 {
                    self.ask(
                        p,
                        Decision::PutPicksOntoBattlefield {
                            cards,
                            picked: Vec::new(),
                            left: up_to,
                            creature_and_land: false,
                            rest_bottom: false,
                        },
                    );
                }
            }
            Effect::CrewedSelf => {
                if self.obj(src).zone == Zone::Battlefield {
                    let d = self.def_of(src);
                    let (pw, t) = (d.power, d.toughness);
                    let turn = self.turn;
                    self.obj_mut(src).eot_creature = Some((turn, pw, t, "Vehicle", Keywords::NONE));
                }
            }
            Effect::MilledLandsToBattlefieldTapped => {
                let p = item.controller;
                let turn = self.turn;
                let lands: Vec<ObjId> = self.players[p]
                    .graveyard
                    .iter()
                    .copied()
                    .filter(|&c| self.def_of(c).types.land && self.obj(c).milled_turn == turn)
                    .collect();
                for c in lands {
                    self.move_to(c, Zone::Battlefield);
                    self.obj_mut(c).tapped = true;
                }
            }
            Effect::Energy(n) => {
                let p = item.controller;
                self.players[p].energy += n as u32;
            }
            Effect::CounterSpellOrAbility(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    if let Some(pos) = self.targeted_item(id) {
                        let it = self.items[pos].clone();
                        if it.kind == StackKind::Spell {
                            if !self.uncounterable(id) {
                                self.items.remove(pos);
                                self.leave_stack(id, it.flashback);
                            }
                        } else {
                            self.items.remove(pos); // a copy or an ability: it just leaves the stack
                        }
                    }
                }
            }
            Effect::LifeBecomesCreatureCount => {
                for q in [item.controller, opponent(item.controller)] {
                    let n = self.creatures(q).count() as i32;
                    let life = self.players[q].life;
                    if n > life {
                        self.gain_life(q, n - life);
                    } else if n < life {
                        self.lose_life(q, life - n);
                    }
                }
            }
            Effect::BounceAllLands => {
                let lands: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&x| self.types_of(x).land)
                    .collect();
                for l in lands {
                    self.move_to(l, Zone::Hand);
                }
            }
            Effect::UntapLands(n) => {
                let lands: Vec<ObjId> = self
                    .permanents(item.controller)
                    .filter(|&x| self.types_of(x).land && self.obj(x).tapped)
                    .take(n as usize)
                    .collect();
                for l in lands {
                    self.untap_by_effect(l);
                }
            }
            Effect::MayLandFromHand => {
                let p = item.controller;
                if self.players[p].hand.iter().any(|&x| self.def_of(x).types.land) {
                    self.ask(p, Decision::LandFromHand);
                }
            }
            Effect::BounceAllExceptLandsAndSubtypes(subs) => {
                let all: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&x| !self.types_of(x).land && !subs.iter().any(|s| self.has_subtype(x, s)))
                    .collect();
                for x in all {
                    self.move_to(x, Zone::Hand);
                }
            }
            Effect::GainControlWhileSource(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    let p = item.controller;
                    let before = self.obj(id).controller;
                    if self.obj(src).zone == Zone::Battlefield && self.obj(src).controller == p && before != p
                    {
                        let entry = (id, self.obj(id).zcc, src, self.obj(src).zcc, p, before);
                        self.control_while.push(entry);
                        let turn = self.turn;
                        let o = self.obj_mut(id);
                        o.controller = p;
                        o.since_turn = turn; // CR 302.6
                    }
                }
            }
            Effect::SearchForestForPlayer(n) => {
                if let Some(q) = self
                    .players_for(item, legal, Who::TargetPlayer(n))
                    .first()
                    .copied()
                {
                    let what = SearchFor::LandTypes {
                        types: [false, false, false, false, true],
                        basic: false,
                    };
                    self.ask(
                        item.controller,
                        Decision::Search {
                            what,
                            to: SearchTo::BattlefieldTappedUnder(q as u8),
                            left: 1,
                        },
                    );
                }
            }
            Effect::Awaken { slot, n } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if self.obj(id).zone == Zone::Battlefield {
                        self.obj_mut(id).eot_creature = Some((u32::MAX, 0, 0, "Elemental", Keywords::HASTE));
                        self.add_counters(id, n as i32);
                    }
                }
            }
            Effect::SlotBecomesCreatureForGood {
                slot,
                power,
                toughness,
                subtype,
            } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if self.obj(id).zone == Zone::Battlefield {
                        self.obj_mut(id).eot_creature =
                            Some((u32::MAX, power, toughness, subtype, Keywords::NONE));
                    }
                }
            }
            Effect::SoulbondPairOnEnter => {
                let p = item.controller;
                let open = self
                    .creatures(p)
                    .any(|c| c != src && self.paired_partner(c).is_none());
                if self.obj(src).zone == Zone::Battlefield
                    && self.paired_partner(src).is_none()
                    && open
                    && !self.obj(src).lost_soulbond
                {
                    self.ask(p, Decision::SoulbondPair { source: src });
                }
            }
            Effect::SoulbondPairWith(n) => {
                if let Some(c) = Game::slot_object(item, legal, n) {
                    let both = !self.obj(src).lost_soulbond
                        && self.obj(src).zone == Zone::Battlefield
                        && self.obj(c).zone == Zone::Battlefield
                        && self.obj(c).controller == item.controller
                        && self.paired_partner(src).is_none()
                        && self.paired_partner(c).is_none();
                    if both {
                        self.pair(src, c);
                    }
                }
            }
            Effect::DestroyArtifactsEnchantmentsCountersSelf => {
                let all: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&x| {
                        let t = self.types_of(x);
                        t.artifact || t.enchantment
                    })
                    .collect();
                let mut n = 0;
                for x in all {
                    self.destroy(x);
                    if self.obj(x).zone != Zone::Battlefield {
                        n += 1;
                    }
                }
                self.add_counters(src, n);
            }
            Effect::DrawIfPowerThreeElseTwoCounters(n) => {
                if let Some(c) = Game::slot_object(item, legal, n) {
                    if self.power(c) >= 3 {
                        self.draw(item.controller);
                    } else {
                        self.add_counters(c, 2);
                    }
                }
            }
            Effect::SacrificePermanents { who, n } => {
                for q in self.players_for(item, legal, who) {
                    self.sacrifice_permanents(q, n);
                    if self.decision.is_some() {
                        return;
                    }
                }
            }
            Effect::AnnihilatorPerCounters => {
                let n = self.obj(src).counters.clamp(0, 255) as u8;
                if n > 0 {
                    self.sacrifice_permanents(opponent(item.controller), n);
                }
            }
            Effect::OwnerShufflesGraveyardIntoLibrary => {
                let owner = self.obj(src).owner;
                for c in self.players[owner].graveyard.clone() {
                    self.move_to(c, Zone::Library);
                }
                self.shuffle_library(owner);
            }
            Effect::RevealTopCreatureToBattlefieldElseMayBottom => {
                let p = item.controller;
                if let Some(&c) = self.players[p].library.last() {
                    if self.def_of(c).types.creature {
                        self.move_to(c, Zone::Battlefield);
                    } else {
                        self.ask(p, Decision::Tuck { card: c });
                    }
                }
            }
            Effect::CreaturesToOwners => {
                let turn = self.turn;
                for c in self.battlefield.clone() {
                    let o = self.obj(c);
                    if self.types_of(c).creature && o.controller != o.owner {
                        let o = self.obj_mut(c);
                        o.controller = o.owner;
                        o.since_turn = turn;
                    }
                }
            }
            Effect::ExileTopHalfOfLibrary(n) => {
                if let Some(q) = self
                    .players_for(item, legal, Who::TargetPlayer(n))
                    .first()
                    .copied()
                {
                    let half = self.players[q].library.len().div_ceil(2);
                    for _ in 0..half {
                        if let Some(&c) = self.players[q].library.last() {
                            self.move_to(c, Zone::Exile);
                        }
                    }
                }
            }
            Effect::TreasuresPerOpponentArtifact { def } => {
                let p = item.controller;
                let n = self
                    .permanents(opponent(p))
                    .filter(|&x| self.types_of(x).artifact)
                    .count();
                for _ in 0..n {
                    self.create_token(p, def);
                }
            }
            Effect::EachExileUntilNonlandCastFree => {
                let p = item.controller;
                for q in [p, opponent(p)] {
                    while let Some(&c) = self.players[q].library.last() {
                        self.move_to(c, Zone::Exile);
                        if !self.types_of(c).land {
                            self.obj_mut(c).cast_free = true;
                            self.free_casts.push(c);
                            break;
                        }
                    }
                }
                self.free_caster = p;
            }
            Effect::PoisonAmountToOpponent => {
                let q = opponent(item.controller);
                self.players[q].poison += item.amount.max(0);
            }
            Effect::SneakAttack => {
                let p = item.controller;
                if self.players[p]
                    .hand
                    .iter()
                    .any(|&x| self.def_of(x).types.creature)
                {
                    self.ask(p, Decision::SneakAttack);
                }
            }
            Effect::AddRedOrGreenPerCreature => {
                let p = item.controller;
                let n = self.creatures(p).count();
                for _ in 0..n {
                    self.players[p]
                        .pool
                        .push(ManaSource::of(&[crate::Color::Red, crate::Color::Green]));
                }
            }
            Effect::ExileTopPutCreaturesAndLands(n) => {
                let p = item.controller;
                let mut cards = Vec::new();
                for _ in 0..n {
                    if let Some(&c) = self.players[p].library.last() {
                        self.move_to(c, Zone::Exile);
                        let t = self.def_of(c).types;
                        if t.creature || t.land {
                            cards.push(c);
                        }
                    }
                }
                if !cards.is_empty() {
                    self.ask(p, Decision::ExiledToBattlefield { cards });
                }
            }
            Effect::CreaturesFromHand => {
                let p = item.controller;
                if self.players[p]
                    .hand
                    .iter()
                    .any(|&x| self.def_of(x).types.creature)
                {
                    self.ask(p, Decision::CreaturesFromHand);
                }
            }
            Effect::PreventCombatDamageToAndBy(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    let zcc = self.obj(id).zcc;
                    self.combat_damage_prevented.push((id, zcc));
                    self.combat_damage_by_prevented.push((id, zcc));
                }
            }
            Effect::MayReturnAnotherThenIndestructible => {
                let p = item.controller;
                if self.creatures(p).any(|c| c != src) {
                    self.ask(p, Decision::ReturnAnotherForIndestructible { source: src });
                }
            }
            Effect::LandFromHandTappedDrawRepeat => {
                let p = item.controller;
                if self.players[p].hand.iter().any(|&x| self.def_of(x).types.land) {
                    self.ask(p, Decision::LandTappedDrawRepeat);
                }
            }
            Effect::BasicLandFromHandTapped => {
                let p = item.controller;
                if self.players[p].hand.iter().any(|&x| {
                    let t = self.def_of(x).types;
                    t.land && t.basic
                }) {
                    self.ask(p, Decision::BasicFromHandTapped);
                }
            }
            Effect::CopiesHasteSacrifice { slot, n } => {
                for _ in 0..n {
                    let copy = Effect::CopyToken {
                        slot: Some(slot),
                        haste_and_sacrifice: true,
                        extra_subtype: None,
                    };
                    self.run_effect(item, legal, copy);
                }
            }
            Effect::UnsealingDamage(n) => {
                let Some(spell) = item.extra else { return };
                let power = self.def_of(spell).power;
                if (4..=6).contains(&power) {
                    if let Some(&t) = item.targets.get(n as usize) {
                        if legal.get(n as usize) == Some(&true) {
                            let to = match t {
                                Chosen::Player(q) => Target::Player(q),
                                Chosen::Object(x, _) => Target::Object(x),
                            };
                            self.deal_damage(src, to, 4);
                        }
                    }
                } else if power >= 7 {
                    let q = opponent(item.controller);
                    self.deal_damage(src, Target::Player(q), 4);
                    let theirs: Vec<ObjId> = self
                        .permanents(q)
                        .filter(|&x| {
                            let t = self.types_of(x);
                            t.creature || t.planeswalker
                        })
                        .collect();
                    for x in theirs {
                        self.deal_damage(src, Target::Object(x), 4);
                    }
                }
            }
            Effect::CopiesHasteExileAtEnd { slot, n } => {
                let n = if n == 0 {
                    1 + self.instants_sorceries_cast[item.controller]
                } else {
                    n as u32
                };
                self.copies_exile_at_end(item, legal, slot, n);
            }
            Effect::CopyIfChosenTypeExileAtEnd(slot) => {
                if let Some(c) = Game::slot_object(item, legal, slot) {
                    let chosen = self.obj(src).chosen_type.clone();
                    if chosen.is_some_and(|t| self.has_subtype(c, &t)) {
                        self.copies_exile_at_end(item, legal, slot, 1);
                    }
                }
            }
            Effect::CoinFlipCopiesExileAtEnd(slot) => {
                let mut wins = 0;
                while self.rng.below(2) == 0 && wins < 50 {
                    wins += 1;
                }
                self.copies_exile_at_end(item, legal, slot, wins);
            }
            Effect::CommanderFromCommandZoneUntilEnd => {
                let p = item.controller;
                let card = self
                    .command
                    .iter()
                    .copied()
                    .find(|&c| self.obj(c).owner == p && self.is_commander(c));
                if let Some(card) = card {
                    self.ask(p, Decision::CommanderCourser { card });
                }
            }
            Effect::SearchCreaturesTotalManaValue(n) => {
                let p = item.controller;
                let any = self.players[p]
                    .library
                    .iter()
                    .any(|&x| self.def_of(x).types.creature && self.def_of(x).cost.mana_value() <= n);
                if any {
                    self.ask(p, Decision::HulkSearch { budget: n });
                } else {
                    self.shuffle_library(p);
                }
            }
            Effect::DrawIfGreatestPower(slot) => {
                if let Some(c) = Game::slot_object(item, legal, slot) {
                    let power = self.power(c);
                    let others = self
                        .battlefield
                        .iter()
                        .copied()
                        .filter(|&x| x != c && self.types_of(x).creature)
                        .all(|x| self.power(x) < power);
                    if others {
                        let q = self.obj(c).controller;
                        self.draw(q);
                    }
                }
            }
            Effect::AddAnyColorsGreatestPower => {
                let p = item.controller;
                let n = self.creatures(p).map(|c| self.power(c)).max().unwrap_or(0).max(0);
                for _ in 0..n {
                    self.players[p].pool.push(ManaSource::of(&crate::Color::ALL));
                }
            }
            Effect::PrimalSurge => self.primal_surge(item.controller),
            Effect::Hideaway(n) => {
                let p = item.controller;
                let cards: Vec<ObjId> = self.players[p]
                    .library
                    .iter()
                    .rev()
                    .take(n as usize)
                    .copied()
                    .collect();
                if !cards.is_empty() {
                    self.ask(p, Decision::HideawayPick { source: src, cards });
                }
            }
            Effect::PlayHideaway(cond) => {
                let p = item.controller;
                let ok = match cond {
                    0 => self.creatures(p).map(|c| self.power(c).max(0)).sum::<i32>() >= 10,
                    _ => {
                        let q = opponent(p);
                        let (t, total) = self.damage_to_player_turn[q];
                        t == self.turn && total >= 7
                    }
                };
                if let (true, Some(c)) = (ok, self.obj(src).hideaway) {
                    if self.obj(c).zone == Zone::Exile {
                        let turn = self.turn;
                        let o = self.obj_mut(c);
                        o.playable = Some((p, turn));
                        o.cast_free = true;
                    }
                }
            }
            Effect::WandOfWonder => {
                let p = item.controller;
                let roll = self.rng.below(20) + 1;
                let x = if roll == 20 {
                    3
                } else if roll >= 10 {
                    2
                } else {
                    1
                };
                let q = opponent(p);
                let mut hits = Vec::new();
                let mut rest = Vec::new();
                while let Some(&c) = self.players[q].library.last() {
                    self.move_to(c, Zone::Exile);
                    let t = self.def_of(c).types;
                    if t.instant || t.sorcery {
                        hits.push(c);
                        break;
                    }
                    rest.push(c);
                }
                for c in rest {
                    self.move_to(c, Zone::Library);
                }
                self.shuffle_library(q);
                for c in hits.into_iter().take(x) {
                    self.obj_mut(c).cast_free = true;
                    self.free_casts.push(c);
                }
                self.free_caster = p;
            }
            Effect::CreateTokenWithKeywordCounterChoice { def } => {
                let p = item.controller;
                self.create_token(p, def);
                if let Some(&t) = self.last_created.first() {
                    self.ask(p, Decision::KeywordCounterChoice { token: t });
                }
            }
            Effect::NextCreatureSpellThisTurn(trigger) => {
                let entry = (src, trigger, item.controller, self.turn);
                self.next_creature_spell.push(entry);
            }
            Effect::SearchCreatureLesserManaValue => {
                if let Some(spell) = item.extra {
                    let mv = self.def_of(spell).cost.mana_value();
                    if mv > 0 {
                        let what = SearchFor::CreatureMvAtMost(mv - 1);
                        self.ask(
                            item.controller,
                            Decision::Search {
                                what,
                                to: SearchTo::Battlefield { tapped: false },
                                left: 1,
                            },
                        );
                    }
                }
            }
            Effect::DelinaRoll(slot) => {
                if let Some(c) = Game::slot_object(item, legal, slot) {
                    self.delina_roll(item.controller, c);
                }
            }
            Effect::CopyHasteDiesDrawSacrifice { slot, trigger } => {
                if let Some(c) = Game::slot_object(item, legal, slot) {
                    let p = item.controller;
                    for _ in 0..self.token_doubling(p) {
                        if let Some(id) = self.token_copy_of(p, c) {
                            let zcc = self.obj(id).zcc;
                            self.until_eot.push(Modifier {
                                obj: id,
                                zcc,
                                power: 0,
                                toughness: 0,
                                keywords: Keywords::HASTE,
                                dies: Some((src, trigger, p)),
                            });
                            self.obj_mut(id).sacrifice_at_end = true;
                        }
                    }
                }
            }
            Effect::KeywordCounterSelf(k) => {
                if self.obj(src).zone == Zone::Battlefield {
                    let o = self.obj_mut(src);
                    o.keyword_counters = o.keyword_counters.with(k);
                    self.counter_put_on(src);
                }
            }
            Effect::PhalanxCopies => {
                let p = item.controller;
                if let Some(partner) = self.paired_partner(src) {
                    for of in [src, partner] {
                        for _ in 0..self.token_doubling(p) {
                            if let Some(id) = self.token_copy_of(p, of) {
                                let o = self.obj_mut(id);
                                o.granted = o.granted.with(Keywords::HASTE);
                                o.exile_end_combat = true;
                                o.lost_soulbond = true;
                            }
                        }
                    }
                }
            }
            Effect::TokenFightsEachOpponentCreature { def } => {
                let p = item.controller;
                let theirs: Vec<ObjId> = self.creatures(opponent(p)).collect();
                let mut made = Vec::new();
                for _ in 0..theirs.len() {
                    self.create_token(p, def);
                    made.extend(self.last_created.iter().copied());
                }
                for (t, c) in made.into_iter().zip(theirs) {
                    if self.obj(t).zone == Zone::Battlefield && self.obj(c).zone == Zone::Battlefield {
                        let (pt, pc) = (self.power(t), self.power(c));
                        self.deal_damage(t, Target::Object(c), pt);
                        self.deal_damage(c, Target::Object(t), pc);
                    }
                }
            }
            Effect::BecomeCopyOfSlotUntilEot(n) => {
                if let Some(card) = Game::slot_object(item, legal, n) {
                    if self.obj(src).zone == Zone::Battlefield {
                        let (zcc, before, of) = (self.obj(src).zcc, self.obj(src).def, self.obj(card).def);
                        self.copy_until_eot.push((src, zcc, before));
                        self.become_copy(src, of);
                    }
                }
            }
            Effect::LookTopMayBottom(n) => {
                if let Some(q) = self
                    .players_for(item, legal, Who::TargetPlayer(n))
                    .first()
                    .copied()
                {
                    if let Some(&card) = self.players[q].library.last() {
                        self.ask(item.controller, Decision::Tuck { card });
                    }
                }
            }
            Effect::DrawThenPutBack { draw, back } => {
                let p = item.controller;
                for _ in 0..draw {
                    self.draw(p);
                }
                if back > 0 && !self.players[p].hand.is_empty() {
                    self.ask(p, Decision::HandToTop { left: back });
                }
            }
            Effect::ExileLibraryShuffleHandIn(n) => {
                if let Some(q) = self
                    .players_for(item, legal, Who::TargetPlayer(n))
                    .first()
                    .copied()
                {
                    for c in self.players[q].library.clone() {
                        self.move_to(c, Zone::Exile);
                    }
                    for c in self.players[q].hand.clone() {
                        self.move_to(c, Zone::Library);
                    }
                    self.shuffle_library(q);
                }
            }
            Effect::ScrollRack => {
                let p = item.controller;
                if !self.players[p].hand.is_empty() {
                    self.ask(p, Decision::ScrollRackExile { exiled: Vec::new() });
                }
            }
            Effect::ConduitCast(n) => {
                let p = item.controller;
                if self.spells_cast[p] == 0 {
                    if let Some(id) = Game::slot_object(item, legal, n) {
                        if self.obj(id).zone == Zone::Graveyard {
                            self.obj_mut(id).conduit_turn = self.turn;
                        }
                    }
                }
            }
            Effect::GrantTappedTrigger { slot, trigger } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let entry = (id, self.obj(id).zcc, src, trigger, self.turn);
                    self.granted_tapped.push(entry);
                }
            }
            Effect::ExtraDealsDamageEqualPower(n) => {
                if let (Some(c), Some(&t)) = (item.extra, item.targets.get(n as usize)) {
                    if legal.get(n as usize) == Some(&true) {
                        let amount = if self.obj(c).zone == Zone::Battlefield {
                            self.power(c)
                        } else {
                            self.obj(c).last_power
                        };
                        let to = match t {
                            Chosen::Player(q) => Target::Player(q),
                            Chosen::Object(x, _) => Target::Object(x),
                        };
                        if amount > 0 {
                            self.deal_damage(c, to, amount);
                        }
                    }
                }
            }
            Effect::DrawThenSourceOnTop => {
                let p = item.controller;
                self.draw(p);
                if self.obj(src).zone == Zone::Battlefield {
                    self.move_to(src, Zone::Library);
                }
            }
            Effect::GrantFightAbilityEot => {
                // CR 611.2c: the creatures you control as it resolves.
                let turn = self.turn;
                let mine: Vec<ObjId> = self.creatures(item.controller).collect();
                for c in mine {
                    let zcc = self.obj(c).zcc;
                    self.granted_fight.push((c, zcc, turn));
                }
            }
            Effect::ReturnSelfToHandAtNextEnd => {
                if self.obj(src).zone == Zone::Battlefield {
                    let zcc = self.obj(src).zcc;
                    self.return_to_hand_at_end.push((src, zcc));
                }
            }
            Effect::ManifestDreadFirstElseTurnFaceUp => {
                let turn = self.turn;
                let o = self.obj_mut(src);
                o.resolved_this_turn = if o.resolved_this_turn.0 == turn {
                    (turn, o.resolved_this_turn.1 + 1)
                } else {
                    (turn, 1)
                };
                let p = item.controller;
                if self.obj(src).resolved_this_turn.1 == 1 {
                    let cards: Vec<ObjId> = self.players[p].library.iter().rev().take(2).copied().collect();
                    match cards.len() {
                        0 => {}
                        1 => self.manifest(p, cards[0]),
                        _ => self.ask(p, Decision::ManifestDread { cards }),
                    }
                } else if self.permanents(p).any(|x| self.can_turn_face_up(x)) {
                    self.ask(p, Decision::TurnFaceUp);
                }
            }
            Effect::UntapLandsUpTo(n) => {
                let mut lands: Vec<ObjId> = self
                    .permanents(item.controller)
                    .filter(|&l| self.types_of(l).land && self.obj(l).tapped)
                    .collect();
                // The lands that make the most colors first.
                lands.sort_by_key(|&l| {
                    std::cmp::Reverse(
                        self.def_of(l)
                            .mana
                            .map_or(0, |m| m.colors.iter().filter(|&&b| b).count()),
                    )
                });
                for l in lands.into_iter().take(n as usize) {
                    self.untap_by_effect(l);
                }
            }
            Effect::MayPutLandFromHandTapped => {
                let p = item.controller;
                if self.players[p].hand.iter().any(|&x| self.def_of(x).types.land) {
                    self.ask(p, Decision::LandFromHandTapped);
                }
            }
            Effect::DrawMoreIfLands { n, more, lands } => {
                let p = item.controller;
                let have = self.permanents(p).filter(|&x| self.types_of(x).land).count();
                for _ in 0..if have >= lands as usize { more } else { n } {
                    self.draw(p);
                }
            }
            Effect::SearchBasicsPerAttackersTapped => {
                let p = item.controller;
                let attacking = self.combat.as_ref().map_or(0, |c| {
                    c.attackers
                        .iter()
                        .filter(|&&a| self.obj(a).controller != p)
                        .count()
                });
                if attacking > 0 {
                    let left = attacking.min(u8::MAX as usize) as u8;
                    self.ask(
                        p,
                        Decision::Search {
                            what: SearchFor::BasicLand,
                            to: SearchTo::Battlefield { tapped: true },
                            left,
                        },
                    );
                }
            }
            Effect::UnlessOpponentPays(n) => {
                let payer = opponent(item.controller);
                let cost = ManaCost {
                    generic: n,
                    ..ManaCost::default()
                };
                self.ask(payer, Decision::PayToStop { cost });
            }
            Effect::CumulativeUpkeep(n) => {
                if self.obj(src).zone == Zone::Battlefield {
                    self.obj_mut(src).kind_counters += 1; // an age counter
                    let age = self.obj(src).kind_counters.clamp(0, 255) as u8;
                    let cost = ManaCost {
                        generic: n.saturating_mul(age),
                        ..ManaCost::default()
                    };
                    self.ask(item.controller, Decision::PayUpkeep { cost, source: src });
                }
            }
            Effect::NextUpkeepDraws { slot, theirs, yours } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let them = self.obj(id).controller;
                    self.next_upkeep_draws.push((them, theirs));
                }
                self.next_upkeep_draws.push((item.controller, yours));
            }
            Effect::Unearth => {
                if self.obj(src).zone == Zone::Graveyard {
                    self.move_to(src, Zone::Battlefield);
                    if self.obj(src).zone == Zone::Battlefield {
                        let o = self.obj_mut(src);
                        o.unearthed = true;
                        o.suspend_haste = true; // "It gains haste."
                    }
                }
            }
            Effect::ExtraTurnFor(slot) => {
                if let Some(Chosen::Player(q)) = item.targets.get(slot as usize) {
                    if legal.get(slot as usize).copied().unwrap_or(false) {
                        self.extra_turns.push(*q);
                    }
                }
            }
            Effect::AnimateLandElemental(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if self.obj(id).zone == Zone::Battlefield {
                        self.add_counters(id, 3);
                        self.untap_by_effect(id);
                        let o = self.obj_mut(id);
                        o.animated = Some((0, 0));
                        o.granted = o.granted.with(Keywords::VIGILANCE).with(Keywords::HASTE);
                        if !o.extra_subtypes.contains(&"Elemental") {
                            o.extra_subtypes.push("Elemental");
                        }
                    }
                }
            }
            Effect::DistributeCountersX => {
                let p = item.controller;
                if item.x > 0 && self.creatures(p).next().is_some() {
                    self.ask(p, Decision::DistributeCounters { left: item.x });
                }
            }
            Effect::DrawPerCreaturePowerAtLeast(n) => {
                let p = item.controller;
                let k = self.creatures(p).filter(|&c| self.power(c) >= n).count();
                for _ in 0..k {
                    self.draw(p);
                }
            }
            Effect::MinusCounter(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if self.obj(id).zone == Zone::Battlefield {
                        self.obj_mut(id).counters -= 1;
                    }
                }
            }
            Effect::Proliferate => {
                let p = item.controller;
                for x in self.battlefield.clone() {
                    let o = self.obj(x);
                    let mine = o.controller == p;
                    if mine && o.counters > 0 {
                        self.add_counters(x, 1);
                    } else if !mine && o.counters < 0 {
                        self.obj_mut(x).counters -= 1;
                    }
                    if mine && self.types_of(x).planeswalker {
                        self.obj_mut(x).loyalty += 1;
                    }
                }
            }
            Effect::ExileMakeEnchantmentCopy(slot) => {
                if let Some(card) = Game::slot_object(item, legal, slot) {
                    if self.obj(card).zone == Zone::Graveyard {
                        self.move_to(card, Zone::Exile);
                        if self.obj(card).zone == Zone::Exile {
                            let def = self.obj(card).def;
                            for _ in 0..self.token_doubling(item.controller) {
                                let id = self.add_card(item.controller, def, Zone::Hand);
                                let o = self.obj_mut(id);
                                o.copy_token = true;
                                o.copy_enchantment_only = true;
                                self.move_to(id, Zone::Battlefield);
                            }
                        }
                    }
                }
            }
            Effect::UnlessOpponentPaysPower => {
                let payer = opponent(item.controller);
                // Esper Sentinel: only their first noncreature spell this turn.
                let (t, n) = self.noncreature_casts[payer];
                if t != self.turn || n != 1 {
                    self.resolving.as_mut().expect("resolving").remaining.clear();
                } else {
                    let power = if self.obj(src).zone == Zone::Battlefield {
                        self.power(src)
                    } else {
                        self.obj(src).last_power
                    };
                    let cost = ManaCost {
                        generic: power.clamp(0, 255) as u8,
                        ..ManaCost::default()
                    };
                    self.ask(payer, Decision::PayToStop { cost });
                }
            }
            Effect::ChangeTargets(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if let Some(i) = self.targeted_item(id) {
                        self.ask_new_target(item.controller, i, 0);
                    }
                }
            }
            Effect::CreatureFromHandOrGraveyardMvAtMostLands => {
                let p = item.controller;
                let cards: Vec<ObjId> = self.players[p]
                    .hand
                    .iter()
                    .chain(&self.players[p].graveyard)
                    .copied()
                    .filter(|&c| self.picks(c, Pick::CreatureMvAtMostLands))
                    .collect();
                if !cards.is_empty() {
                    self.ask(
                        p,
                        Decision::LookPick {
                            cards,
                            pick: Pick::CreatureMvAtMostLands,
                            to: LookTo::BattlefieldTwoCounters,
                            rest: None,
                        },
                    );
                }
            }
            Effect::SacrificeLandOrSource => {
                let p = item.controller;
                if self
                    .permanents(p)
                    .any(|x| self.types_of(x).land && !self.cant_be_sacrificed(x))
                {
                    self.ask(p, Decision::SacrificeLandOrThis { source: src });
                } else if self.obj(src).zone == Zone::Battlefield && !self.cant_be_sacrificed(src) {
                    self.move_to(src, Zone::Graveyard);
                }
            }
            Effect::MilledCreatureToBattlefield => {
                let p = item.controller;
                let turn = self.turn;
                let cards: Vec<ObjId> = self.players[p]
                    .graveyard
                    .iter()
                    .copied()
                    .filter(|&c| self.def_of(c).types.creature && self.obj(c).milled_turn == turn)
                    .collect();
                if !cards.is_empty() {
                    self.ask(
                        p,
                        Decision::LookPick {
                            cards,
                            pick: Pick::Any,
                            to: LookTo::Battlefield,
                            rest: None, // they are already in the graveyard; taking one is offered (optional)
                        },
                    );
                }
            }
            Effect::RevealTopLandsToHandRestGraveyard(n) => {
                let p = item.controller;
                let top: Vec<ObjId> = self.players[p]
                    .library
                    .iter()
                    .rev()
                    .take(n as usize)
                    .copied()
                    .collect();
                for c in top {
                    if self.def_of(c).types.land {
                        self.move_to(c, Zone::Hand);
                    } else {
                        self.move_to(c, Zone::Graveyard);
                    }
                }
            }
            Effect::LandsFromHandTapped => {
                let p = item.controller;
                if self.players[p].hand.iter().any(|&x| self.def_of(x).types.land) {
                    self.ask(p, Decision::LandsFromHand);
                }
            }
            Effect::ReturnPermanentCardsToHand => {
                let p = item.controller;
                let cards: Vec<ObjId> = self.players[p]
                    .graveyard
                    .iter()
                    .copied()
                    .filter(|&c| self.def_of(c).types.is_permanent())
                    .collect();
                for c in cards {
                    self.move_to(c, Zone::Hand);
                }
            }
            Effect::TapOrUntap(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if self.obj(id).tapped {
                        self.untap_by_effect(id);
                    } else {
                        self.become_tapped(id);
                    }
                }
            }
            Effect::DrawIfUniqueName(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let p = item.controller;
                    let name = self.def_of(id).name.clone();
                    let twin = self.creatures(p).any(|c| c != id && self.def_of(c).name == name)
                        || self.players[p]
                            .graveyard
                            .iter()
                            .any(|&c| self.def_of(c).types.creature && self.def_of(c).name == name);
                    if !twin {
                        self.draw(p);
                    }
                }
            }
            Effect::ReturnLandFromGraveyardTapped => {
                let p = item.controller;
                let cards: Vec<ObjId> = self.players[p]
                    .graveyard
                    .iter()
                    .copied()
                    .filter(|&c| self.def_of(c).types.land)
                    .collect();
                if !cards.is_empty() {
                    self.ask(
                        p,
                        Decision::LookPick {
                            cards,
                            pick: Pick::Land,
                            to: LookTo::BattlefieldTapped,
                            rest: None,
                        },
                    );
                }
            }
            Effect::OpponentMay => {
                self.ask(opponent(item.controller), Decision::May);
            }
            Effect::Explore(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let p = self.obj(id).controller;
                    if let Some(&top) = self.players[p].library.last() {
                        if self.def_of(top).types.land {
                            self.move_to(top, Zone::Hand);
                            self.resolving.as_mut().expect("resolving").remaining.clear();
                        } else {
                            self.add_counters(id, 1);
                            // "put the card back or put it into your graveyard": Yes mills it.
                            self.ask(p, Decision::May);
                        }
                    } else {
                        self.add_counters(id, 1);
                        self.resolving.as_mut().expect("resolving").remaining.clear();
                    }
                }
            }
            Effect::QuestCounterSelf => {
                if self.obj(src).zone == Zone::Battlefield {
                    self.obj_mut(src).quest += 1;
                }
            }
            Effect::OpponentsAttackThisTurn => {
                self.must_attack_turn[opponent(item.controller)] = self.turn;
            }
            Effect::CreateTokensPerLastCounters { def } => {
                let n = if self.obj(src).zone == Zone::Battlefield {
                    self.obj(src).counters
                } else {
                    self.obj(src).last_counters.0
                };
                for _ in 0..n.max(0) {
                    self.create_token(item.controller, def);
                }
            }
            Effect::CountersSelfPerOtherWithSubtypes(subs) => {
                let n = self
                    .permanents(item.controller)
                    .filter(|&x| x != src && subs.iter().any(|s| self.has_subtype(x, s)))
                    .count() as i32;
                self.add_counters(src, n);
            }
            Effect::CopyEachTokenYouControl => {
                let toks: Vec<ObjId> = self
                    .permanents(item.controller)
                    .filter(|&x| self.is_token(x))
                    .collect();
                for o in toks {
                    // CR 707.2: the copiable values: its card or token definition, and offspring's 1/1.
                    let def = self.obj(o).def;
                    let base = self.obj(o).copy_base_pt;
                    let card_copy = self.obj(o).copy_token;
                    for _ in 0..self.token_doubling(item.controller) {
                        let id = self.add_card(item.controller, def, Zone::Hand);
                        self.obj_mut(id).copy_token = card_copy || !self.defs[def].token;
                        self.obj_mut(id).copy_base_pt = base;
                        self.move_to(id, Zone::Battlefield);
                    }
                }
            }
            Effect::ExileLinked(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if self.obj(id).zone == Zone::Battlefield {
                        self.move_to(id, Zone::Exile);
                        self.obj_mut(src).linked_card = Some(id);
                    }
                }
            }
            Effect::LinkedOwnerCreatesXx { def } => {
                if let Some(c) = self.obj(src).linked_card {
                    let owner = self.obj(c).owner;
                    let x = self.def_of(c).cost.mana_value() as i32;
                    self.create_token(owner, def);
                    for t in self.last_created.clone() {
                        self.obj_mut(t).copy_base_pt = Some((x, x));
                    }
                }
            }
            Effect::GiftCard => {
                let opp = opponent(item.controller);
                self.draw(opp);
                self.fire(item.controller, When::YouGiveGift, None);
            }
            Effect::CountersOnGreenEnteredThisTurn => {
                let turn = self.turn;
                let fresh: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| {
                        self.types_of(c).creature
                            && self.colors_of(c)[crate::Color::Green as usize]
                            && self.obj(c).since_turn == turn
                    })
                    .collect();
                for c in fresh {
                    self.add_counters(c, 1);
                }
            }
            Effect::CountersEqualPower(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let n = self.power(id);
                    self.add_counters(id, n);
                }
            }
            Effect::WinIfCountersOrHandAtLeast(n) => {
                let p = item.controller;
                let o = self.obj(src);
                let counters = o.counters + o.stun + o.time_counters as i32;
                if counters >= n || self.players[p].hand.len() as i32 >= n {
                    let opp = opponent(p);
                    let protected = self.permanents(opp).any(|c| self.def_of(c).cant_lose);
                    if !protected && !self.players[opp].lost {
                        self.players[opp].lost = true;
                        self.events.push(Event::Lost { player: opp });
                        self.result = Some(Outcome::Winner(p));
                        self.pending = Pending::GameOver;
                    }
                }
            }
            Effect::AddManaEither(a, b) => {
                self.players[item.controller].pool.push(ManaSource::of(&[a, b]));
            }
            Effect::CreatureSpellsUncounterableThisTurn => {
                self.creatures_uncounterable_turn[item.controller] = self.turn;
            }
            Effect::ReturnDiedWithFlyingCounter => {
                if let Some(c) = item.extra {
                    if self.obj(c).zone == Zone::Graveyard {
                        let owner = self.obj(c).owner;
                        self.move_to(c, Zone::Battlefield);
                        self.obj_mut(c).controller = owner;
                        let o = self.obj_mut(c);
                        o.keyword_counters = o.keyword_counters.with(Keywords::FLYING);
                        self.counter_put_on(c);
                    }
                }
            }
            Effect::VanishingTick => {
                if self.obj(src).zone == Zone::Battlefield && self.obj(src).time_counters > 0 {
                    self.obj_mut(src).time_counters -= 1;
                    if self.obj(src).time_counters == 0 && !self.cant_be_sacrificed(src) {
                        self.move_to(src, Zone::Graveyard); // CR 702.63a: sacrifice it
                    }
                }
            }
            Effect::TapFreeze(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    self.become_tapped(id);
                    self.obj_mut(id).skip_untap = true;
                }
            }
            Effect::PumpYoursPowerAtLeastAndGrant {
                min,
                power,
                toughness,
                keywords,
            } => {
                let big: Vec<ObjId> = self
                    .creatures(item.controller)
                    .filter(|&c| self.power(c) >= min)
                    .collect();
                for id in big {
                    let zcc = self.obj(id).zcc;
                    self.until_eot.push(Modifier {
                        obj: id,
                        zcc,
                        power,
                        toughness,
                        keywords,
                        dies: None,
                    });
                }
            }
            Effect::PumpAttackersPerAttacker => {
                let attackers: Vec<ObjId> = self
                    .combat
                    .as_ref()
                    .map(|c| c.attackers.clone())
                    .unwrap_or_default();
                let x = attackers.len() as i32;
                for id in attackers {
                    if self.obj(id).controller == item.controller && self.obj(id).zone == Zone::Battlefield {
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
            }
            Effect::ExtraLandThisTurn => {
                let p = item.controller;
                self.extra_lands_this_turn[p] = self.extra_lands_this_turn[p].saturating_add(1);
            }
            Effect::RevealTopLandToBattlefieldElseHand => {
                let p = item.controller;
                if let Some(&c) = self.players[p].library.last() {
                    let to = if self.def_of(c).types.land {
                        Zone::Battlefield
                    } else {
                        Zone::Hand
                    };
                    self.move_to(c, to);
                }
            }
            Effect::DestroyWithSameName(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let name = self.def_of(id).name.clone();
                    let others: Vec<ObjId> = self
                        .battlefield
                        .iter()
                        .copied()
                        .filter(|&x| x != id && self.def_of(x).name == name)
                        .collect();
                    self.destroy(id);
                    for x in others {
                        self.destroy(x);
                    }
                }
            }
            Effect::DrawX => {
                for _ in 0..item.x {
                    self.draw(item.controller);
                }
            }
            Effect::AddManaAnyColor => {
                self.players[item.controller]
                    .pool
                    .push(ManaSource::of(&crate::Color::ALL));
            }
            Effect::DiscardUnlessAttacked => {
                let p = item.controller;
                if !self.attacked_this_turn[p] && !self.players[p].hand.is_empty() {
                    self.ask(
                        p,
                        Decision::Discard {
                            left: 1,
                            optional: false,
                            then_draw: 0,
                        },
                    );
                }
            }
            Effect::DrawIfSecondResolutionThisTurn(n) => {
                let turn = self.turn;
                let o = self.obj_mut(src);
                o.resolved_this_turn = if o.resolved_this_turn.0 == turn {
                    (turn, o.resolved_this_turn.1 + 1)
                } else {
                    (turn, 1)
                };
                if o.resolved_this_turn.1 == 2 {
                    for _ in 0..n {
                        self.draw(item.controller);
                    }
                }
            }
            Effect::TapAttached => {
                if let Some((c, zcc)) = self.obj(src).attached_to {
                    if self.obj(c).zcc == zcc && self.obj(c).zone == Zone::Battlefield {
                        self.become_tapped(c);
                    }
                }
            }
            Effect::UntapExtra => {
                if let Some(c) = item.extra {
                    if self.obj(c).zone == Zone::Battlefield {
                        self.untap_by_effect(c);
                    }
                }
            }
            Effect::GrantDiesUnderYourControl { slot, trigger } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let entry = (id, self.obj(id).zcc, src, trigger, item.controller, self.turn);
                    self.dies_under_control.push(entry);
                }
            }
            Effect::PutDeadCountersOn(slot) => {
                let (Some(dead), Some(id)) = (item.extra, Game::slot_object(item, legal, slot)) else {
                    return;
                };
                if self.obj(id).zone != Zone::Battlefield {
                    return;
                }
                let (plus, stun, kw) = self.obj(dead).last_counters;
                if plus > 0 {
                    self.add_counters(id, plus);
                }
                let extra = stun > 0 || kw != Keywords::NONE;
                if stun > 0 {
                    let n = stun * self.doubling(self.obj(id).controller);
                    self.obj_mut(id).stun += n;
                }
                if kw != Keywords::NONE {
                    let o = self.obj_mut(id);
                    o.keyword_counters = o.keyword_counters.with(kw);
                }
                if extra {
                    self.counter_put_on(id);
                }
            }
            Effect::GrantCombatDamageTrigger { slot, trigger } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let entry = (id, self.obj(id).zcc, src, trigger, self.turn);
                    self.granted_combat_damage.push(entry);
                }
            }
            Effect::BounceSpellOrPermanent(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if let Some(pos) = self
                        .items
                        .iter()
                        .position(|i| i.kind == StackKind::Spell && i.source == id)
                    {
                        self.items.remove(pos);
                    }
                    self.move_to(id, Zone::Hand);
                }
            }
            Effect::MaySacrificeNonland => {
                let any = self.permanents(item.controller).any(|c| !self.types_of(c).land);
                if any {
                    self.ask(item.controller, Decision::SacrificeNonland);
                } else {
                    self.resolving.as_mut().expect("resolving").remaining.clear();
                }
            }
            Effect::DiscardHandDrawPerSpells => {
                let p = item.controller;
                for c in self.players[p].hand.clone() {
                    self.move_to(c, Zone::Graveyard);
                }
                for _ in 0..self.spells_cast[p] {
                    self.draw(p);
                }
            }
            Effect::BetorEndStep => {
                let p = item.controller;
                self.draw(p);
                if self.total_toughness(p) >= 20 {
                    for c in self.creatures(p).collect::<Vec<_>>() {
                        self.untap_by_effect(c);
                    }
                }
                if self.total_toughness(p) >= 40 {
                    let opp = opponent(p);
                    let life = self.players[opp].life;
                    if life > 0 {
                        self.lose_life(opp, (life + 1) / 2);
                    }
                }
            }
            Effect::DestroyOpponentsNonland => {
                let opp = opponent(item.controller);
                let all: Vec<ObjId> = self.permanents(opp).filter(|&c| !self.types_of(c).land).collect();
                for c in all {
                    self.destroy(c);
                }
            }
            Effect::DestroyCreaturesAndEnchantmentsDraw => {
                let all: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| self.types_of(c).creature || self.types_of(c).enchantment)
                    .collect();
                let mut destroyed = 0;
                for c in all {
                    self.destroy(c);
                    if self.obj(c).zone != Zone::Battlefield {
                        destroyed += 1;
                    }
                }
                for _ in 0..destroyed {
                    self.draw(item.controller);
                }
            }
            Effect::DestroyAllCreatures => {
                let all: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| self.types_of(c).creature)
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
            Effect::DoublePowerToughness(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    let (power, toughness, zcc) = (self.power(id), self.toughness(id), self.obj(id).zcc);
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
            Effect::KeywordCounter { slot, keywords } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if self.obj(id).zone == Zone::Battlefield {
                        let o = self.obj_mut(id);
                        o.keyword_counters = o.keyword_counters.with(keywords);
                        self.counter_put_on(id);
                    }
                }
            }
            Effect::RemoveAllCounters(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if self.obj(id).zone == Zone::Battlefield {
                        let o = self.obj_mut(id);
                        o.counters = 0;
                        o.stun = 0;
                        o.keyword_counters = Keywords::NONE;
                    }
                }
            }
            Effect::DoublePowerToughnessAll => {
                for id in self.creatures(item.controller).collect::<Vec<_>>() {
                    let (power, toughness, zcc) = (self.power(id), self.toughness(id), self.obj(id).zcc);
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
            Effect::BounceSelf => {
                if self.obj(src).zone == Zone::Battlefield {
                    self.move_to(src, Zone::Hand);
                }
            }
            Effect::GrantCreated(keywords) => {
                for id in self.last_created.clone() {
                    if self.obj(id).zone == Zone::Battlefield {
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
            }
            Effect::SpiritGreatestToughness { spirit } => {
                let p = item.controller;
                let x = self
                    .creatures(p)
                    .map(|c| self.toughness(c))
                    .max()
                    .unwrap_or(0)
                    .max(0);
                self.endure_token(p, x.min(255) as u8, spirit);
            }
            Effect::ReturnSelfToHand => {
                if self.obj(src).zone == Zone::Graveyard {
                    self.move_to(src, Zone::Hand);
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
                    .is_some_and(|x| self.obj(x).zone == Zone::Exile && self.types_of(x).creature);
                if exiled {
                    self.add_counters(src, 1);
                    self.gain_life(item.controller, life);
                }
            }
            Effect::TapAndStun(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    self.become_tapped(id);
                    let n = self.doubling(self.obj(id).controller);
                    self.obj_mut(id).stun += n;
                    self.counter_put_on(id);
                }
            }
            Effect::GateControlPowerAtLeast(n) => {
                if !self.creatures(item.controller).any(|c| self.power(c) >= n) {
                    self.resolving.as_mut().expect("resolving").remaining.clear();
                }
            }
            Effect::MayPay(cost) => self.ask(item.controller, Decision::MayPay { cost }),
            Effect::CounterUnlessPays(slot, n, m) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let target = self
                        .items
                        .iter()
                        .rev()
                        .find(|i| i.kind == StackKind::Spell && i.source == id);
                    if let Some(t) = target {
                        if self.uncounterable(id) {
                            return;
                        }
                        let generic = if item.kicked { m } else { n };
                        let cost = ManaCost {
                            generic,
                            ..ManaCost::default()
                        };
                        let payer = t.controller;
                        self.ask(payer, Decision::PayOrCountered { cost, against: id });
                    }
                }
            }
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
                    if self.types_of(c).creature {
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
                    .position(|&c| self.types_of(c).land)
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
                    let mine: Vec<ObjId> = self
                        .creatures(p)
                        .filter(|&c| !self.cant_be_sacrificed(c))
                        .collect();
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
                    self.untap_by_effect(id);
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
            Effect::AdvanceStage { if_subtype, to } => {
                if self.obj(src).zone == Zone::Battlefield && self.has_base_subtype(src, if_subtype) {
                    self.obj_mut(src).stage = to;
                }
            }
            Effect::BecomeCreatureSelf { power, toughness } => {
                if self.obj(src).zone == Zone::Battlefield {
                    self.obj_mut(src).animated = Some((power, toughness));
                }
            }
            Effect::ExileTopOfEachCastFree => {
                let p = item.controller;
                for q in [p, opponent(p)] {
                    if let Some(&c) = self.players[q].library.last() {
                        self.move_to(c, Zone::Exile);
                        if !self.types_of(c).land {
                            self.obj_mut(c).cast_free = true;
                            self.free_casts.push(c);
                        }
                    }
                }
                self.free_caster = p;
            }
            Effect::CopySpellPerAmount => {
                let snap = item
                    .extra
                    .and_then(|c| self.cast_snapshots.iter().rev().find(|s| s.source == c))
                    .cloned();
                if let Some(snap) = snap {
                    for _ in 0..item.amount.max(0) {
                        let copy = StackItem {
                            kind: StackKind::SpellCopy,
                            controller: item.controller,
                            ..snap.clone()
                        };
                        self.ward_check(&copy);
                        self.items.push(copy);
                    }
                }
            }
            Effect::CountersAmount(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    self.add_counters(id, item.amount);
                }
            }
            Effect::AddMana { color, n } => {
                let m = ManaSource::of(&[color]);
                self.players[item.controller]
                    .pool
                    .extend(std::iter::repeat_n(m, n as usize));
            }
            Effect::DividedCounters => {
                for (i, &n) in item.division.iter().enumerate() {
                    if let Some(id) = Game::slot_object(item, legal, i as u8) {
                        self.add_counters(id, n as i32);
                    }
                }
            }
            Effect::DividedDamage => {
                for (i, &n) in item.division.iter().enumerate() {
                    if let Some(id) = Game::slot_object(item, legal, i as u8) {
                        self.deal_damage(src, Target::Object(id), n as i32);
                    } else if let Some(&Chosen::Player(p)) = item.targets.get(i) {
                        if legal[i] {
                            self.deal_damage(src, Target::Player(p), n as i32); // "any target" (Twin Bolt)
                        }
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
                            .filter(|&&c| !self.types_of(c).land)
                            .map(|&c| self.colors_of(c))
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
                    .filter(|&&c| self.types_of(c).creature)
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
                    .filter(|&x| self.types_of(x).land)
                    .count();
                for _ in 0..n {
                    self.create_token(item.controller, def);
                }
            }
            Effect::DrawPerDistinctManaValue => {
                let mut mvs: Vec<u32> = self
                    .permanents(item.controller)
                    .filter(|&x| !self.types_of(x).land)
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
                    for _ in 0..self.token_doubling(item.controller) {
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
                    self.untap_by_effect(src);
                }
            }
            Effect::Tap(n) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    self.become_tapped(id);
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
            Effect::ToLibrary(n, top) => {
                if let Some(id) = Game::slot_object(item, legal, n) {
                    let owner = self.obj(id).owner;
                    self.move_to(id, Zone::Library);
                    let lib = &mut self.players[owner].library;
                    if let Some(pos) = lib.iter().position(|&x| x == id) {
                        let c = lib.remove(pos);
                        if top {
                            lib.push(c);
                        } else {
                            lib.insert(0, c);
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
            Effect::PumpPerCreatures { slot, attacking } => {
                let x = if attacking {
                    self.combat.as_ref().map_or(0, |c| c.attackers.len()) as i32
                } else {
                    self.creatures(item.controller).count() as i32
                };
                if let Some(id) = Game::slot_object(item, legal, slot) {
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
            Effect::DamagePerCreatures(slot) => {
                let x = self.creatures(item.controller).count() as i32;
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if x > 0 {
                        self.deal_damage(src, Target::Object(id), x);
                    }
                }
            }
            Effect::PumpYoursPerCreatures => {
                let ids: Vec<ObjId> = self.creatures(item.controller).collect();
                let x = ids.len() as i32;
                for id in ids {
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
            Effect::NextSpellUncounterable => {
                self.next_spell_uncounterable[item.controller] = self.turn;
            }
            Effect::TapSelf => {
                if self.obj(src).zone == Zone::Battlefield {
                    self.become_tapped(src);
                }
            }
            Effect::PlayerExilesCreature(slot) => {
                if let Some(p) = self
                    .players_for(item, legal, Who::TargetPlayer(slot))
                    .into_iter()
                    .next()
                {
                    let mine: Vec<ObjId> = self.creatures(p).collect();
                    match mine.len() {
                        0 => {}
                        1 => self.move_to(mine[0], Zone::Exile),
                        _ => self.ask(p, Decision::ExileCreature),
                    }
                }
            }
            Effect::DamageEqualCardPower { card, to } => {
                let (Some(c), Some(t)) = (
                    Game::slot_object(item, legal, card),
                    Game::slot_object(item, legal, to),
                ) else {
                    return;
                };
                let power = if self.obj(c).zone == Zone::Battlefield {
                    self.power(c)
                } else {
                    self.def_of(c).power
                };
                if power > 0 {
                    self.deal_damage(src, Target::Object(t), power);
                }
            }
            Effect::DrawIfCounterElseCounterSelf => {
                if self.controls_creature_with_counter(item.controller) {
                    self.draw(item.controller);
                } else if self.obj(src).zone == Zone::Battlefield {
                    self.add_counters(src, 1);
                }
            }
            Effect::ExileGraveyard { who } => {
                for p in self.players_for(item, legal, who) {
                    for c in self.players[p].graveyard.clone() {
                        self.move_to(c, Zone::Exile);
                    }
                }
            }
            Effect::DoublePowerSelf => {
                if self.obj(src).zone == Zone::Battlefield {
                    let (power, zcc) = (self.power(src), self.obj(src).zcc);
                    self.until_eot.push(Modifier {
                        obj: src,
                        zcc,
                        power,
                        toughness: 0,
                        keywords: Keywords::NONE,
                        dies: None,
                    });
                }
            }
            Effect::CreateTokensAmountTapped { def } => {
                for _ in 0..item.amount {
                    self.create_token(item.controller, def);
                    for t in self.last_created.clone() {
                        self.obj_mut(t).tapped = true;
                    }
                }
            }
            Effect::WinTheGame => {
                let opp = opponent(item.controller);
                let protected = self.permanents(opp).any(|c| self.def_of(c).cant_lose);
                if !protected && !self.players[opp].lost {
                    self.players[opp].lost = true;
                    self.events.push(Event::Lost { player: opp });
                    self.result = Some(Outcome::Winner(item.controller));
                    self.pending = Pending::GameOver;
                }
            }
            Effect::CreateTokensPerSpellCastThisTurn { def } => {
                for _ in 0..self.spells_cast[item.controller] {
                    self.create_token(item.controller, def);
                }
            }
            Effect::ReturnLandYouControl => {
                let p = item.controller;
                if self.permanents(p).any(|c| self.types_of(c).land) {
                    self.ask(p, Decision::ReturnOwnLand);
                }
            }
            Effect::DamageEqualSlotPower { from, to } => {
                let Some(Chosen::Object(a, _)) = item.targets.get(from as usize).copied() else {
                    return;
                };
                let power = if self.obj(a).zone == Zone::Battlefield {
                    self.power(a)
                } else {
                    self.obj(a).last_power // CR 608.2h: last known information
                };
                let target = match item.targets.get(to as usize) {
                    Some(&Chosen::Object(b, _)) if legal[to as usize] => Target::Object(b),
                    Some(&Chosen::Player(q)) if legal[to as usize] => Target::Player(q),
                    _ => return,
                };
                if power > 0 {
                    self.deal_damage(a, target, power);
                }
            }
            Effect::ActivePlayerDraws(n) => {
                for _ in 0..n {
                    self.draw(self.active);
                }
            }
            Effect::LoyaltyEachYouControlSubtype(sub) => {
                let p = item.controller;
                for w in self.permanents(p).collect::<Vec<_>>() {
                    if self.types_of(w).planeswalker && self.has_subtype(w, sub) {
                        self.obj_mut(w).loyalty += self.doubling(p);
                    }
                }
            }
            Effect::SacrificeCreatureOrPlaneswalkerElseDiscard { who } => {
                for p in self.players_for(item, legal, who) {
                    let mine: Vec<ObjId> = self
                        .permanents(p)
                        .filter(|&c| self.types_of(c).creature || self.types_of(c).planeswalker)
                        .filter(|&c| !self.cant_be_sacrificed(c))
                        .collect();
                    match mine.len() {
                        0 => {
                            if !self.players[p].hand.is_empty() {
                                self.ask(
                                    p,
                                    Decision::Discard {
                                        left: 1,
                                        optional: false,
                                        then_draw: 0,
                                    },
                                );
                                return;
                            }
                        }
                        1 => self.move_to(mine[0], Zone::Graveyard),
                        _ => {
                            self.ask(p, Decision::SacrificeCreatureOrPlaneswalker);
                            return;
                        }
                    }
                }
            }
            Effect::ChaosWarp(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let owner = self.obj(id).owner;
                    self.move_to(id, Zone::Library);
                    self.shuffle_library(owner);
                    if let Some(&top) = self.players[owner].library.last() {
                        if self.types_of(top).is_permanent() {
                            self.move_to(top, Zone::Battlefield);
                        }
                    }
                }
            }
            Effect::DestroyAllCreaturesDrawPerDestroyed => {
                let all: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| self.types_of(c).creature)
                    .collect();
                let mut n = 0;
                for c in all {
                    self.destroy_no_regen(c); // "They can't be regenerated."
                    if self.obj(c).zone != Zone::Battlefield {
                        n += 1;
                    }
                }
                for _ in 0..n {
                    self.draw(item.controller);
                }
            }
            Effect::PreventAllCombatDamageThisTurn => self.combat_damage_prevented_turn = Some(self.turn),
            Effect::PreventAllDamageToYouThisTurn => {
                self.damage_to_player_prevented_turn[item.controller] = Some(self.turn)
            }
            Effect::GainLifePerOpponentCreature => {
                let n = self.creatures(opponent(item.controller)).count() as i32;
                self.gain_life(item.controller, n);
            }
            Effect::PumpByOwnPowerAndGrant { slot, keywords } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let (x, zcc) = (self.power(id), self.obj(id).zcc);
                    self.until_eot.push(Modifier {
                        obj: id,
                        zcc,
                        power: x,
                        toughness: x,
                        keywords,
                        dies: None,
                    });
                }
            }
            Effect::RevealTopCreatureToHandElseMay => {
                let p = item.controller;
                match self.players[p].library.last().copied() {
                    Some(c) if self.def_of(c).types.creature => {
                        self.move_to(c, Zone::Hand);
                        self.resolving.as_mut().expect("resolving").remaining.clear();
                    }
                    Some(_) => self.ask(p, Decision::May),
                    None => self.resolving.as_mut().expect("resolving").remaining.clear(),
                }
            }
            Effect::DamageAmountToEachOpponentCreature => {
                let n = item.amount;
                if n > 0 {
                    for c in self.creatures(opponent(item.controller)).collect::<Vec<_>>() {
                        self.deal_damage(src, Target::Object(c), n);
                    }
                }
            }
            Effect::DamageAmountTo(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if item.amount > 0 {
                        self.deal_damage(src, Target::Object(id), item.amount);
                    }
                }
            }
            Effect::ShuffleSourceFromGraveyardIntoLibrary => {
                if self.obj(src).zone == Zone::Graveyard {
                    let owner = self.obj(src).owner;
                    self.move_to(src, Zone::Library);
                    self.shuffle_library(owner);
                }
            }
            Effect::DestroyOpponentsArtifacts => {
                for a in self.permanents(opponent(item.controller)).collect::<Vec<_>>() {
                    if self.types_of(a).artifact {
                        self.destroy(a);
                    }
                }
            }
            Effect::PumpXAndGrant { slot, keywords } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let zcc = self.obj(id).zcc;
                    self.until_eot.push(Modifier {
                        obj: id,
                        zcc,
                        power: item.x as i32,
                        toughness: 0,
                        keywords,
                        dies: None,
                    });
                }
            }
            Effect::RegenerateSelf => {
                if self.obj(src).zone == Zone::Battlefield {
                    let turn = self.turn;
                    let (t, n) = self.obj(src).regen_shields;
                    self.obj_mut(src).regen_shields = (turn, if t == turn { n + 1 } else { 1 });
                }
            }
            Effect::Regenerate(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    let turn = self.turn;
                    let (t, n) = self.obj(id).regen_shields;
                    self.obj_mut(id).regen_shields = (turn, if t == turn { n + 1 } else { 1 });
                }
            }
            Effect::DestroyNoRegen(slot) => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    self.destroy_no_regen(id);
                }
            }
            Effect::PumpAllGrantInfect {
                power,
                toughness,
                keywords,
            } => {
                let turn = self.turn;
                for id in self.creatures(item.controller).collect::<Vec<_>>() {
                    let zcc = self.obj(id).zcc;
                    self.until_eot.push(Modifier {
                        obj: id,
                        zcc,
                        power,
                        toughness,
                        keywords,
                        dies: None,
                    });
                    self.infect_until_eot.push((id, zcc, turn));
                }
            }
            Effect::EternalizeSelf => {
                let def = self.obj(src).def;
                self.last_created.clear();
                for _ in 0..self.token_doubling(item.controller) {
                    let id = self.add_card(item.controller, def, Zone::Hand);
                    self.obj_mut(id).copy_token = true;
                    self.move_to(id, Zone::Battlefield);
                    self.last_created.push(id);
                    let ob = self.obj_mut(id);
                    ob.copy_base_pt = Some((4, 4));
                    ob.eternalized = true;
                    ob.extra_subtypes.push("Zombie");
                }
            }
            Effect::AdditionalCombatAfterSecondMainUntapAttackers => {
                self.combats_after_main2 = self.combats_after_main2.saturating_add(1);
            }
            Effect::Rebound => {
                if self.obj(src).zone == Zone::Stack && self.obj(src).from_hand {
                    self.move_to(src, Zone::Exile);
                    let zcc = self.obj(src).zcc;
                    self.rebound.push((src, zcc));
                }
            }
            Effect::AdditionalCombatAfterMainUntap => {
                if matches!(self.step, Step::Main1 | Step::Main2) {
                    let resume = if self.step == Step::Main1 {
                        Step::BeginCombat
                    } else {
                        Step::End
                    };
                    let resume = self.extra_main_resume.unwrap_or(resume);
                    self.extra_combats_untap_only.push(self.extra_combats.len());
                    self.extra_combats.push(resume);
                }
            }
            Effect::MustBeBlockedThisTurnSelf => {
                if self.obj(src).zone == Zone::Battlefield {
                    let entry = (src, self.obj(src).zcc, self.turn);
                    self.must_be_blocked.push(entry);
                }
            }
            Effect::AdditionalCombatAfterThisPhase => {
                self.combats_after_this_phase = self.combats_after_this_phase.saturating_add(1);
            }
            Effect::UntapAttackersAndGrant(keywords) => {
                let attackers = self
                    .combat
                    .as_ref()
                    .map(|c| c.attackers.clone())
                    .unwrap_or_default();
                for a in attackers {
                    if self.obj(a).zone == Zone::Battlefield {
                        self.untap_by_effect(a);
                        let zcc = self.obj(a).zcc;
                        self.until_eot.push(Modifier {
                            obj: a,
                            zcc,
                            power: 0,
                            toughness: 0,
                            keywords,
                            dies: None,
                        });
                    }
                }
            }
            Effect::ExertSelfUntapOthers => {
                if self.obj(src).zone == Zone::Battlefield {
                    let turn = self.turn;
                    let o = self.obj_mut(src);
                    o.exerted_turn = Some(turn);
                    o.skip_untap = true;
                }
                for c in self.creatures(item.controller).collect::<Vec<_>>() {
                    if c != src {
                        self.untap_by_effect(c);
                    }
                }
            }
            Effect::GainControlOfOpponentsArtifacts => {
                let me = item.controller;
                let turn = self.turn;
                for a in self.permanents(opponent(me)).collect::<Vec<_>>() {
                    if self.types_of(a).artifact {
                        let o = self.obj_mut(a);
                        o.controller = me;
                        o.since_turn = turn; // CR 302.6
                    }
                }
            }
            Effect::CreateTokenWithCountersAmount { def } => {
                let n = item.amount;
                self.create_token(item.controller, def);
                for t in self.last_created.clone() {
                    self.add_counters(t, n);
                }
            }
            Effect::PutCreatureFromHandAttackingUntilEnd => {
                let p = item.controller;
                if self.players[p]
                    .hand
                    .iter()
                    .any(|&c| self.def_of(c).types.creature)
                {
                    self.ask(p, Decision::PutCreatureAttacking);
                }
            }
            Effect::SourceThirdFromTop => {
                if matches!(self.obj(src).zone, Zone::Graveyard | Zone::Exile) {
                    let owner = self.obj(src).owner;
                    self.move_to(src, Zone::Library);
                    let lib = &mut self.players[owner].library;
                    if let Some(pos) = lib.iter().position(|&x| x == src) {
                        let card = lib.remove(pos);
                        let at = lib.len().saturating_sub(2);
                        lib.insert(at, card); // the top is the end: third from the top
                    }
                }
            }
            Effect::GrantAllPermanentsYouControl(keywords) => {
                for id in self.permanents(item.controller).collect::<Vec<_>>() {
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
            Effect::CopyAttachedNonlegendaryHaste => {
                let Some((c, zcc)) = self.obj(src).attached_to else {
                    return;
                };
                if self.obj(c).zcc != zcc || self.obj(c).zone != Zone::Battlefield {
                    return;
                }
                let def = self.obj(c).def;
                self.last_created.clear();
                for _ in 0..self.token_doubling(item.controller) {
                    let id = self.add_card(item.controller, def, Zone::Hand);
                    self.obj_mut(id).copy_token = true;
                    self.obj_mut(id).not_legendary = true;
                    self.move_to(id, Zone::Battlefield);
                    self.last_created.push(id);
                    let ob = self.obj_mut(id);
                    ob.granted = ob.granted.with(Keywords::HASTE);
                }
            }
            Effect::TemptOpponentsLand => {
                self.tempt_accepted = false;
                self.ask(opponent(item.controller), Decision::TemptOffer);
            }
            Effect::SearchLandIfTempted => {
                if std::mem::take(&mut self.tempt_accepted) {
                    let land = Decision::Search {
                        what: SearchFor::Land,
                        to: SearchTo::Battlefield { tapped: false },
                        left: 1,
                    };
                    self.ask(item.controller, land);
                }
            }
            Effect::Cascade => {
                let Some(spell) = item.extra else {
                    return;
                };
                let mv = self.def_of(spell).cost.mana_value();
                let p = item.controller;
                let mut exiled = Vec::new();
                let mut hit = None;
                while let Some(&c) = self.players[p].library.last() {
                    self.move_to(c, Zone::Exile);
                    if !self.def_of(c).types.land && self.def_of(c).cost.mana_value() < mv {
                        hit = Some(c);
                        break;
                    }
                    exiled.push(c);
                }
                for &c in &exiled {
                    self.move_to(c, Zone::Library);
                }
                self.put_among_rest(p, exiled, false);
                if let Some(c) = hit {
                    self.obj_mut(c).cast_free = true;
                    self.free_casts.push(c);
                    self.free_caster = p;
                    self.free_casts_to_bottom = true;
                }
            }
            Effect::YourCreaturesBasePtHandSize => {
                let p = item.controller;
                let x = self.players[p].hand.len() as i32;
                let turn = self.turn;
                for c in self.creatures(p).collect::<Vec<_>>() {
                    // Keep what an earlier "becomes a creature" gave it (its type and keywords).
                    let (sub, kw) = self.eot_creature(c).map_or(("", Keywords::NONE), |e| (e.3, e.4));
                    self.obj_mut(c).eot_creature = Some((turn, x, x, sub, kw));
                }
            }
            Effect::GainLifePerCreatureCardInGraveyard => {
                let p = item.controller;
                let n = self.players[p]
                    .graveyard
                    .iter()
                    .filter(|&&c| self.def_of(c).types.creature)
                    .count() as i32;
                self.gain_life(p, n);
            }
            Effect::ExileSourceFromGraveyard => {
                if self.obj(src).zone == Zone::Graveyard {
                    self.move_to(src, Zone::Exile);
                }
            }
            Effect::Evolve(slot) => {
                if let Some(c) = Game::slot_object(item, legal, slot) {
                    if self.obj(src).zone == Zone::Battlefield
                        && (self.power(c) > self.power(src) || self.toughness(c) > self.toughness(src))
                    {
                        self.add_counters(src, 1);
                    }
                }
            }
            Effect::KindCounterSelf(n) => {
                if self.obj(src).zone == Zone::Battlefield {
                    let n = n * self.doubling(self.obj(src).controller);
                    self.obj_mut(src).kind_counters += n;
                }
            }
            Effect::MaySacrificeToken => {
                let p = item.controller;
                if self
                    .permanents(p)
                    .any(|c| self.is_token(c) && !self.cant_be_sacrificed(c))
                {
                    self.ask(p, Decision::SacrificeToken);
                } else {
                    self.resolving.as_mut().expect("resolving").remaining.clear();
                }
            }
            Effect::UntapLandsYouControl => {
                for l in self.permanents(item.controller).collect::<Vec<_>>() {
                    if self.types_of(l).land {
                        self.obj_mut(l).tapped = false;
                    }
                }
            }
            Effect::CreateTokensPower { def } => {
                let power = if self.obj(src).zone == Zone::Battlefield {
                    self.power(src)
                } else {
                    self.obj(src).last_power
                };
                let mut made = Vec::new();
                for _ in 0..power.max(0) {
                    self.create_token(item.controller, def);
                    made.extend(self.last_created.iter().copied());
                }
                self.last_created = made;
            }
            Effect::SelvalaParley => {
                let nonland = (0..2)
                    .filter(|&q| {
                        self.players[q]
                            .library
                            .last()
                            .is_some_and(|&c| !self.types_of(c).land)
                    })
                    .count();
                let p = item.controller;
                let green = ManaSource::of(&[crate::Color::Green]);
                self.players[p].pool.extend(std::iter::repeat_n(green, nonland));
                self.gain_life(p, nonland as i32);
                let active = self.active;
                for q in [active, opponent(active)] {
                    self.draw(q);
                }
            }
            Effect::ShrinkAllWithoutSubtypesPerYours(subs) => {
                let kind = |g: &Game, c: ObjId| subs.iter().any(|s| g.has_subtype(c, s));
                let x = self.creatures(item.controller).filter(|&c| kind(self, c)).count() as i32;
                if x > 0 {
                    let all: Vec<ObjId> = self
                        .battlefield
                        .iter()
                        .copied()
                        .filter(|&c| self.types_of(c).creature && !kind(self, c))
                        .collect();
                    for c in all {
                        let zcc = self.obj(c).zcc;
                        self.until_eot.push(Modifier {
                            obj: c,
                            zcc,
                            power: -x,
                            toughness: -x,
                            keywords: Keywords::NONE,
                            dies: None,
                        });
                    }
                }
            }
            Effect::SlotBecomesCreatureUntilEot {
                slot,
                power,
                toughness,
                subtype,
                keywords,
            } => {
                if let Some(id) = Game::slot_object(item, legal, slot) {
                    if self.obj(id).zone == Zone::Battlefield {
                        let turn = self.turn;
                        self.obj_mut(id).eot_creature = Some((turn, power, toughness, subtype, keywords));
                    }
                }
            }
        }
    }

    fn condition_holds(&self, ability: &Ability, controller: PlayerId, source: ObjId) -> bool {
        match &ability.condition {
            None => true,
            Some(Condition::Attacked) => self.attacked_this_turn[controller],
            Some(Condition::StageAtLeast(_)) => true, // checked as it triggered
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
            Some(Condition::ControlCreatureWithCounter) => self.controls_creature_with_counter(controller),
            Some(Condition::CastIt) => self.obj(source).was_cast,
            Some(Condition::CastFromHand) => self.obj(source).was_cast && self.obj(source).from_hand,
            Some(Condition::CastPowerBetween(..)) => true, // checked as it triggered
            Some(Condition::ControlLandsAtLeast(n)) => {
                self.permanents(controller)
                    .filter(|&x| self.types_of(x).land)
                    .count()
                    >= *n as usize
            }
            Some(Condition::CastTwoSpellsThisTurn) => self.spells_cast[controller] >= 2,
            Some(Condition::ChosenMode(m)) => self.obj(source).chosen_mode == Some(*m),
            Some(Condition::CreatureDiedUnderYourControlThisTurn) => self.creature_died_under[controller],
            Some(Condition::TotalToughnessAtLeast(n)) => self.total_toughness(controller) >= *n,
            Some(Condition::YourMainPhase) => {
                self.active == controller && matches!(self.step, Step::Main1 | Step::Main2)
            }
            Some(Condition::ControlGreatestPower) => {
                let all: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| self.types_of(c).creature)
                    .collect();
                let max = all.iter().map(|&c| self.power(c)).max();
                max.is_some_and(|m| self.creatures(controller).any(|c| self.power(c) == m))
            }
            Some(Condition::CastCreatureAndNoncreatureThisTurn) => {
                self.cast_creature_noncreature[controller] == [true, true]
            }
            Some(Condition::HandSizeExactly(n)) => self.players[controller].hand.len() == *n as usize,
            Some(Condition::XAtLeast(n)) => self.obj(source).x >= *n,
            Some(Condition::FirstCombatOfTurn) => self.combats_this_turn <= 1,
            Some(Condition::NotExertedThisTurn) => self.obj(source).exerted_turn != Some(self.turn),
            Some(Condition::ControlArtifactsAtLeast(n)) => {
                self.permanents(controller)
                    .filter(|&x| self.types_of(x).artifact)
                    .count() as u32
                    >= *n
            }
            Some(Condition::SourceInGraveyardOrExile) => {
                matches!(self.obj(source).zone, Zone::Graveyard | Zone::Exile)
            }
            Some(Condition::OpponentControlsMoreLands) => {
                let lands = |q: PlayerId| self.permanents(q).filter(|&x| self.types_of(x).land).count();
                lands(opponent(controller)) > lands(controller)
            }
            Some(Condition::Both(a, b)) => {
                let one = |c: &Condition| Ability {
                    condition: Some(c.clone()),
                    ..Ability::default()
                };
                self.condition_holds(&one(a), controller, source)
                    && self.condition_holds(&one(b), controller, source)
            }
        }
    }

    /// Creates a token on the battlefield. It is summoning sick like any new permanent.
    /// Doubling Season doubles tokens and counters (CR 614.1a).
    /// Endure's Spirit: an `n`/`n` token (a 0/0 Spirit with `n` +1/+1 counters).
    fn endure_token(&mut self, p: PlayerId, n: u8, spirit: usize) {
        self.create_token(p, spirit);
        for id in self.last_created.clone() {
            self.obj_mut(id).counters += n as i32;
            self.counter_put_on(id);
        }
    }

    fn create_token(&mut self, p: PlayerId, def: usize) {
        self.last_created.clear();
        for _ in 0..self.token_doubling(p) {
            let id = self.add_card(p, def, Zone::Battlefield);
            self.last_created.push(id);
            self.obj_mut(id).since_turn = self.turn;
            self.zone_triggers(id, Zone::Gone, Zone::Battlefield, p);
        }
    }

    /// How many times over tokens for `p` are made: Doubling Season and token-only doublers (Elspeth).
    fn token_doubling(&self, p: PlayerId) -> i32 {
        let n = self
            .permanents(p)
            .filter(|&x| self.def_of(x).doubles_tokens && !self.lost_abilities(x))
            .count();
        self.doubling(p) << n.min(10)
    }

    /// Puts a card on top of its owner's library.
    fn put_on_top(&mut self, c: ObjId) {
        let owner = self.obj(c).owner;
        self.move_to(c, Zone::Library);
        let lib = &mut self.players[owner].library;
        if let Some(pos) = lib.iter().position(|&x| x == c) {
            let card = lib.remove(pos);
            lib.push(card);
        }
    }

    /// Call the Spirit Dragons: ask for the Dragon of the next color that has one; after the last
    /// color, five different Dragons win the game.
    fn next_dragon_color(&mut self, p: PlayerId, from: u8, placed: Vec<ObjId>) {
        for color in from..5 {
            let any = self
                .creatures(p)
                .any(|c| self.has_subtype(c, "Dragon") && self.colors_of(c)[color as usize]);
            if any {
                self.ask(p, Decision::DragonForColor { color, placed });
                return;
            }
        }
        if placed.len() >= 5 {
            let opp = opponent(p);
            let protected = self.permanents(opp).any(|c| self.def_of(c).cant_lose);
            if !protected && !self.players[opp].lost {
                self.players[opp].lost = true;
                self.events.push(Event::Lost { player: opp });
                self.result = Some(Outcome::Winner(p));
                self.pending = Pending::GameOver;
            }
        }
    }

    /// Its until-end-of-turn creature form (Reverberating Summons), if it's this turn.
    fn eot_creature(&self, id: ObjId) -> Option<(u32, i32, i32, &'static str, Keywords)> {
        let o = self.obj(id);
        // `u32::MAX`: for good (awaken).
        o.eot_creature
            .filter(|e| (e.0 == self.turn || e.0 == u32::MAX) && o.zone == Zone::Battlefield)
    }

    /// Put `cards` (in `p`'s library) on the bottom in the order `p` chooses.
    fn bottom_in_order(&mut self, p: PlayerId, cards: Vec<ObjId>) {
        match cards.len() {
            0 => {}
            1 => {
                let lib = &mut self.players[p].library;
                if let Some(pos) = lib.iter().position(|&x| x == cards[0]) {
                    let card = lib.remove(pos);
                    lib.insert(0, card);
                }
            }
            _ => self.ask(p, Decision::BottomOrder { cards }),
        }
    }

    /// Different kinds of counters among creatures `p` controls (CR 122.1).
    fn counter_kinds(&self, p: PlayerId) -> u32 {
        let (mut plus, mut stun, mut fin, mut other) = (false, false, false, 0u32);
        let mut kw = Keywords::NONE;
        let mut others = [false; 6];
        for c in self.creatures(p) {
            let o = self.obj(c);
            plus |= o.counters > 0;
            stun |= o.stun > 0;
            fin |= o.finality;
            kw = kw.with(o.keyword_counters);
            for (i, n) in [o.incubation, o.soul, o.revival, o.fellowship, o.bait, o.loyalty]
                .into_iter()
                .enumerate()
            {
                others[i] |= n > 0;
            }
        }
        other += others.iter().filter(|&&b| b).count() as u32;
        let kws = Keywords::ALL.iter().filter(|(k, _)| kw.has(*k)).count() as u32;
        plus as u32 + stun as u32 + fin as u32 + other + kws
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
        if self.keywords(id).has(Keywords::INDESTRUCTIBLE) {
            return;
        }
        if !self.use_regen_shield(id) && !self.use_umbra_armor(id) {
            self.move_to(id, Zone::Graveyard);
        }
    }

    /// "Destroy ... It can't be regenerated."
    fn destroy_no_regen(&mut self, id: ObjId) {
        if !self.keywords(id).has(Keywords::INDESTRUCTIBLE) && !self.use_umbra_armor(id) {
            self.move_to(id, Zone::Graveyard);
        }
    }

    /// Regeneration (CR 701.15): if `id` has a shield this turn, use it: tap it, remove all damage
    /// from it and remove it from combat. Returns whether it regenerated.
    fn use_regen_shield(&mut self, id: ObjId) -> bool {
        let turn = self.turn;
        let (t, n) = self.obj(id).regen_shields;
        if t != turn || n == 0 || self.obj(id).zone != Zone::Battlefield {
            return false;
        }
        let o = self.obj_mut(id);
        o.regen_shields = (turn, n - 1);
        o.tapped = true;
        o.damage = 0;
        o.deathtouched = false;
        if let Some(c) = self.combat.as_mut() {
            c.attackers.retain(|&a| a != id);
            c.blocks.retain(|b| b.0 != id && b.1 != id);
        }
        true
    }

    /// Whether `id` has infect (Triumph of the Hordes grants it until end of turn).
    fn has_infect(&self, id: ObjId) -> bool {
        let (zcc, turn) = (self.obj(id).zcc, self.turn);
        (self.def_of(id).infect && !self.lost_abilities(id))
            || self
                .infect_until_eot
                .iter()
                .any(|&(c, z, t)| c == id && z == zcc && t == turn)
    }

    /// Roaming Throne: each waiting trigger of another creature its controller controls of the chosen
    /// type triggers an additional time. Each waiting trigger is looked at once.
    fn double_for_thrones(&mut self) {
        let len = self.triggered.len();
        let start = self.triggered_scanned.min(len);
        let thrones: Vec<(ObjId, PlayerId, String)> = self
            .battlefield
            .iter()
            .copied()
            .filter(|&t| self.def_of(t).doubles_chosen_type_triggers && !self.lost_abilities(t))
            .filter_map(|t| {
                self.obj(t)
                    .chosen_type
                    .clone()
                    .map(|ty| (t, self.obj(t).controller, ty))
            })
            .collect();
        if !thrones.is_empty() {
            for i in start..len {
                let item = self.triggered[i].clone();
                if !matches!(item.kind, StackKind::Trigger(_)) {
                    continue;
                }
                let s = item.source;
                let creature = self.def_of(s).types.creature || self.types_of(s).creature;
                let n = thrones
                    .iter()
                    .filter(|(t, c, ty)| {
                        *t != s && *c == item.controller && creature && self.has_subtype(s, ty)
                    })
                    .count();
                for _ in 0..n {
                    self.triggered.push(item.clone());
                }
            }
        }
        self.triggered_scanned = self.triggered.len();
    }

    /// CR 603.3: triggered abilities go on the stack the next time a player would get priority,
    /// active player's first (APNAP, CR 101.4). Returns false if a target choice is now pending.
    fn put_triggers_on_stack(&mut self, then: PlayerId) -> bool {
        self.flush_graveyard_leaves();
        self.double_for_thrones();
        let active = self.active;
        self.triggered.sort_by_key(|t| t.controller != active);
        while !self.triggered.is_empty() {
            let item = self.triggered.remove(0);
            self.triggered_scanned = self.triggered_scanned.saturating_sub(1);
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
                    remove_counters: 0,
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
        if !self.is_over() && self.ask_commander_choice(p) {
            return;
        }
        // Shock lands: "As this land enters, you may pay 2 life. If you don't, it enters tapped." The
        // choice is asked before anyone can act again, so nothing can use the land in between.
        if !self.is_over() {
            let shock = self
                .battlefield
                .iter()
                .copied()
                .find(|&x| self.obj(x).shock_pending);
            if let Some(c) = shock {
                self.obj_mut(c).shock_pending = false;
                self.shock_then = p;
                let owner = self.obj(c).controller;
                self.ask(owner, Decision::ShockLand { card: c });
                return;
            }
        }
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
            // Mana moves alone aren't a reason to stop: nothing to spend them on.
            let real = self
                .legal_actions()
                .iter()
                .filter(|a| !matches!(a, Action::TapForMana(..)))
                .count();
            if real != 1 {
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
            // CR 500.8: an additional combat phase comes right after this main phase.
            Step::Main1 | Step::Main2 if !self.extra_combats.is_empty() => {
                self.extra_combat_resume = self.extra_combats.pop();
                self.extra_main_resume = None;
                let at = self.extra_combats.len();
                if let Some(k) = self.extra_combats_untap_only.iter().position(|&i| i == at) {
                    self.extra_combats_untap_only.remove(k);
                    self.combat_untap_only = true;
                }
                Step::BeginCombat
            }
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
            Step::EndCombat if self.combat_untap_only && self.combats_after_this_phase == 0 => {
                // Moraug's combat: no additional main phase; on to what came next.
                self.combat_untap_only = false;
                self.extra_combat_resume.take().unwrap_or(Step::Main2)
            }
            Step::EndCombat if self.combats_after_this_phase > 0 => {
                self.combats_after_this_phase -= 1;
                Step::BeginCombat // "After this phase, there is an additional combat phase."
            }
            Step::EndCombat => {
                // After an additional combat: its additional main phase (a Main2), then onward.
                self.extra_main_resume = self.extra_combat_resume.take();
                Step::Main2
            }
            Step::Main2 if self.extra_main_resume.is_none() && self.combats_after_main2 > 0 => {
                // World at War: after the second main phase, a combat and then a main phase.
                self.combats_after_main2 -= 1;
                self.extra_combat_resume = Some(Step::End);
                self.untap_attackers_at_combat = true;
                Step::BeginCombat
            }
            Step::Main2 => self.extra_main_resume.take().unwrap_or(Step::End),
            Step::End => Step::Cleanup,
            Step::Cleanup => {
                self.begin_next_turn();
                return;
            }
        };
        self.enter(next);
    }

    fn begin_next_turn(&mut self) {
        self.instants_sorceries_cast = [0; 2];
        self.spells_cast = [0; 2];
        self.cast_creature_noncreature = [[false; 2]; 2];
        self.delayed_on_attack.clear();
        self.dies_under_control.clear();
        self.granted_combat_damage.clear();
        self.granted_tapped.clear();
        self.delayed_on_next_attack.clear();
        self.extra_combats.clear();
        self.extra_combat_resume = None;
        self.extra_main_resume = None;
        self.combats_after_this_phase = 0;
        self.combats_this_turn = 0;
        self.extra_combats_untap_only.clear();
        self.combat_untap_only = false;
        self.combats_after_main2 = 0;
        self.untap_attackers_at_combat = false;
        self.copy_next_spell.clear();
        self.delayed_this_turn.clear();
        self.damage_shields.clear();
        self.graveyard_left_this_turn = [false; 2];
        self.graveyard_leave_pending = [false; 2];
        self.extra_lands_this_turn = [0; 2];
        self.graveyard_types_used = [[false; 5]; 2];
        self.cast_snapshots.clear();
        self.attacked_this_turn = [false; 2];
        self.attacked_creatures_this_turn.clear();
        self.life_gains_this_turn = [0; 2];
        self.life_gained_this_turn = [0; 2];
        self.draws_this_turn = [0; 2];
        self.resolutions_this_turn = 0;
        self.creature_died_this_turn = false;
        self.creature_died_under = [false; 2];
        self.life_lost_this_turn = [false; 2];
        self.turn += 1;
        self.active = self.extra_turns.pop().unwrap_or_else(|| opponent(self.active));
        let active = self.active;
        self.until_your_next_turn.retain(|e| e.3 != active);
        self.events.push(Event::TurnBegan {
            turn: self.turn,
            active: self.active,
        });
        self.enter(Step::Untap);
    }

    fn enter(&mut self, step: Step) {
        self.step = step;
        // CR 500.4: mana empties from each pool as a step or phase ends.
        let omnath: Vec<bool> = (0..2)
            .map(|q| {
                self.permanents(q)
                    .any(|x| self.def_of(x).keeps_green_mana && !self.lost_abilities(x))
            })
            .collect();
        for (q, pl) in self.players.iter_mut().enumerate() {
            if omnath[q] {
                pl.pool.retain(Game::is_green_mana);
            } else {
                pl.pool.clear();
            }
        }
        let a = self.active;
        match step {
            Step::PreGame => unreachable!("the game never returns to pre-game"),
            Step::Untap => {
                // Seedborn Muse: "Untap all permanents you control during each other player's untap step."
                let o = opponent(a);
                if self
                    .permanents(o)
                    .any(|x| self.def_of(x).untaps_on_others_untap && !self.lost_abilities(x))
                {
                    for id in self.permanents(o).collect::<Vec<_>>() {
                        if std::mem::take(&mut self.obj_mut(id).skip_untap) {
                        } else if self.obj(id).stun > 0 && self.obj(id).tapped {
                            self.obj_mut(id).stun -= 1;
                        } else if !self.def_of(id).doesnt_untap && !self.enchanted_doesnt_untap(id) {
                            self.obj_mut(id).tapped = false;
                        }
                    }
                }
                // CR 502.3: untap everything the active player controls. No priority (CR 502.4).
                for id in self.permanents(a).collect::<Vec<_>>() {
                    if std::mem::take(&mut self.obj_mut(id).skip_untap) {
                        // "It doesn't untap during its controller's next untap step."
                    } else if self.obj(id).stun > 0 && self.obj(id).tapped {
                        self.obj_mut(id).stun -= 1; // CR 122.1d: a stun counter is removed instead
                    } else if !self.def_of(id).doesnt_untap && !self.enchanted_doesnt_untap(id) {
                        let was_tapped = self.obj(id).tapped;
                        self.obj_mut(id).tapped = false;
                        if was_tapped && self.types_of(id).creature {
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
                self.fire(a, When::EachDrawStep, None);
                self.fire(opponent(a), When::EachDrawStep, None);
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
                // Decayed: "sacrifice it at end of combat".
                let doomed: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| self.obj(c).sacrifice_end_combat && !self.cant_be_sacrificed(c))
                    .collect();
                for c in doomed {
                    self.move_to(c, Zone::Graveyard);
                }
                // "At end of combat, exile this token." (Delina, Mirage Phalanx)
                let gone: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| self.obj(c).exile_end_combat)
                    .collect();
                for c in gone {
                    self.move_to(c, Zone::Exile);
                }
                self.combat = None;
                self.give_priority(a);
            }
            Step::Cleanup => {
                if self.players[a].hand.len() > self.max_hand(a)
                    && !self.permanents(a).any(|c| self.def_of(c).no_max_hand_size)
                    && !self
                        .emblems
                        .iter()
                        .any(|&(q, k, _)| q == a && k == EmblemKind::NoMaxHandSize)
                {
                    self.pending = Pending::Discard(a);
                } else {
                    self.finish_cleanup();
                }
            }
            Step::BeginCombat => {
                self.combats_this_turn = self.combats_this_turn.saturating_add(1);
                if std::mem::take(&mut self.untap_attackers_at_combat) {
                    // World at War: "untap all creatures that attacked this turn".
                    for c in self.attacked_creatures_this_turn.clone() {
                        if self.obj(c).zone == Zone::Battlefield {
                            self.untap_by_effect(c);
                        }
                    }
                }
                if self.combat_untap_only {
                    // Moraug: "At the beginning of that combat, untap all creatures you control."
                    for c in self.creatures(a).collect::<Vec<_>>() {
                        self.untap_by_effect(c);
                    }
                }
                self.fire(a, When::BeginCombatYourTurn, None);
                self.fire(a, When::BeginCombatEachTurn, None);
                self.fire(opponent(a), When::BeginCombatEachTurn, None);
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
                // Ilharg: "Return that creature to your hand at the beginning of the next end step."
                // Unearth: exiled at the beginning of the next end step.
                let unearthed: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| self.obj(c).unearthed)
                    .collect();
                for c in unearthed {
                    self.move_to(c, Zone::Exile);
                }
                for (c, zcc) in std::mem::take(&mut self.return_to_hand_at_end) {
                    if self.obj(c).zcc == zcc && self.obj(c).zone == Zone::Battlefield {
                        self.move_to(c, Zone::Hand);
                    }
                }
                // "At the beginning of the end step, sacrifice this token."
                let doomed: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| self.obj(c).sacrifice_at_end && !self.cant_be_sacrificed(c))
                    .collect();
                for c in doomed {
                    self.move_to(c, Zone::Graveyard);
                }
                // "Exile them at the beginning of the next end step."
                let exiled: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| self.obj(c).exile_at_end)
                    .collect();
                for c in exiled {
                    self.move_to(c, Zone::Exile);
                }
                // Hellkite Courser: "Return it to the command zone".
                let back: Vec<ObjId> = self
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|&c| self.obj(c).command_at_end)
                    .collect();
                for c in back {
                    self.move_to(c, Zone::Command);
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
                // Arcane Denial: "at the beginning of the next turn's upkeep".
                for (q, n) in std::mem::take(&mut self.next_upkeep_draws) {
                    for _ in 0..n {
                        self.draw(q);
                    }
                }
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
                // Suspend (CR 702.62a): remove a time counter; with the last one gone, cast it free.
                let suspended: Vec<ObjId> = self
                    .exile
                    .iter()
                    .copied()
                    .filter(|&c| self.obj(c).owner == a && self.obj(c).time_counters > 0)
                    .collect();
                let mut ready = Vec::new();
                for c in suspended {
                    self.obj_mut(c).time_counters -= 1;
                    if self.obj(c).time_counters == 0 {
                        ready.push(c);
                    }
                }
                // Rebound: "you may cast this card from exile without paying its mana cost."
                let (now, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.rebound)
                    .into_iter()
                    .partition(|&(c, _)| self.obj(c).owner == a);
                self.rebound = later;
                let rebounds: Vec<ObjId> = now
                    .into_iter()
                    .filter(|&(c, z)| self.obj(c).zcc == z && self.obj(c).zone == Zone::Exile)
                    .map(|(c, _)| c)
                    .collect();
                if !ready.is_empty() || !rebounds.is_empty() {
                    for c in ready {
                        let o = self.obj_mut(c);
                        o.cast_free = true;
                        o.suspend_haste = true;
                        self.free_casts.push(c);
                    }
                    for c in rebounds {
                        self.obj_mut(c).cast_free = true;
                        self.free_casts.push(c);
                    }
                    self.free_caster = a;
                    self.open_free_casts();
                    return;
                }
                self.give_priority(a);
            }
            Step::Main1 => {
                // CR 714.3b: as your precombat main phase begins, a lore counter on each Saga you control.
                for id in self.permanents(a).collect::<Vec<_>>() {
                    if self.is_saga(id) {
                        self.add_lore(id);
                    }
                }
                self.give_priority(a);
            }
            Step::Main2 => {
                self.give_priority(a);
            }
        }
    }

    /// CR 514.2: damage wears off. No player gets priority in cleanup (CR 514.3).
    fn finish_cleanup(&mut self) {
        self.until_eot.clear(); // CR 514.2
        for (id, zcc, before) in std::mem::take(&mut self.copy_until_eot) {
            if self.obj(id).zcc == zcc && self.obj(id).zone == Zone::Battlefield {
                self.obj_mut(id).def = before;
                if self.obj(id).printed_def == Some(before) {
                    self.obj_mut(id).printed_def = None;
                }
            }
        }
        for (id, zcc, before) in std::mem::take(&mut self.control_until_eot) {
            if self.obj(id).zcc == zcc && self.obj(id).zone == Zone::Battlefield {
                let turn = self.turn;
                let o = self.obj_mut(id);
                o.controller = before;
                o.since_turn = turn;
            }
        }
        self.combat_damage_prevented.clear();
        self.combat_damage_by_prevented.clear();
        for o in &mut self.objects {
            o.flashback_eot = false;
            o.harmonize_eot = false;
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

    /// Zilortha: lethal damage to `id` is determined by its power.
    fn lethal_by_power(&self, id: ObjId) -> bool {
        let p = self.obj(id).controller;
        self.permanents(p)
            .any(|x| self.def_of(x).lethal_damage_by_power && !self.lost_abilities(x))
    }

    /// Damage that is lethal to `id` from a source (CR 702.2c: any damage from deathtouch).
    fn lethal_damage(&self, id: ObjId, deathtouch: bool) -> i32 {
        let size = if self.lethal_by_power(id) {
            self.power(id).max(1)
        } else {
            self.toughness(id)
        };
        let left = (size - self.obj(id).damage).max(0);
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
        let events_before = self.events.len();
        self.in_combat_damage = true;
        for (source, target, amount) in assigned {
            let before = match target {
                Target::Player(d) => self.players[d].life,
                Target::Object(_) => 0,
            };
            self.deal_damage(source, target, amount);
            if let Target::Player(d) = target {
                // CR 903.10a: combat damage from the opponent's commander.
                if self.is_commander(source) && self.obj(source).owner != d {
                    let dealt = (before - self.players[d].life).max(0);
                    self.commander_damage[d] += dealt;
                    match self
                        .commander_damage_from
                        .iter_mut()
                        .find(|e| e.0 == source && e.1 == d)
                    {
                        Some(e) => e.2 += dealt,
                        None => self.commander_damage_from.push((source, d, dealt)),
                    }
                }
            }
            if matches!(target, Target::Player(_)) && combat.attackers.contains(&source) {
                hit_player.push((source, amount));
            }
        }
        self.in_combat_damage = false;
        // "Whenever a creature you control deals combat damage during your turn": once per creature
        // per step, with the total it dealt (CR 510.3a).
        let active = self.active;
        let watching = self.permanents(active).any(|x| {
            self.def_of(x)
                .triggers
                .iter()
                .any(|t| t.when == When::YourCreatureDealsCombatDamageOnYourTurn)
        });
        let mut dealt: Vec<(ObjId, i32)> = Vec::new();
        for e in self.events[events_before..].iter().filter(|_| watching) {
            if let Event::Damage { source, amount, .. } = *e {
                match dealt.iter_mut().find(|d| d.0 == source) {
                    Some(d) => d.1 += amount,
                    None => dealt.push((source, amount)),
                }
            }
        }
        for (source, amount) in dealt {
            if self.obj(source).controller != active || !self.types_of(source).creature {
                continue;
            }
            let zcc = self.obj(source).zcc;
            let before = self.triggered.len();
            self.fire(active, When::YourCreatureDealsCombatDamageOnYourTurn, None);
            for t in &mut self.triggered[before..] {
                t.targets = vec![Chosen::Object(source, zcc)];
                t.amount = amount;
            }
        }
        // CR 510.3a: "deals combat damage to a player" triggers.
        for &(a, _) in &hit_player {
            if self.has_counter(a) {
                let controller = self.obj(a).controller;
                self.fire(
                    controller,
                    When::YourCreatureWithCounterDealsCombatDamageToPlayer,
                    None,
                );
            }
        }
        let mut hitters: Vec<PlayerId> = hit_player.iter().map(|&(a, _)| self.obj(a).controller).collect();
        hitters.sort_unstable();
        hitters.dedup();
        for p in hitters {
            self.fire(p, When::YourCreaturesDealCombatDamageToPlayer, None);
        }
        for &(a, _) in &hit_player {
            let controller = self.obj(a).controller;
            self.fire(controller, When::YourCreatureDealsCombatDamageToPlayer, None);
            if self.is_token(a) {
                self.fire(controller, When::YourCreatureTokenDealsCombatDamageToPlayer, None);
            }
        }
        for p in 0..2 {
            let tramplers: i32 = hit_player
                .iter()
                .filter(|&&(a, _)| self.obj(a).controller == p && self.keywords(a).has(Keywords::TRAMPLE))
                .map(|&(_, n)| n)
                .sum();
            if tramplers > 0 {
                let before = self.triggered.len();
                self.fire(p, When::YourTramplersDealCombatDamageToPlayer, None);
                for t in &mut self.triggered[before..] {
                    t.amount = tramplers;
                }
            }
        }
        for &(a, amount) in &hit_player {
            let controller = self.obj(a).controller;
            let before = self.triggered.len();
            self.fire(
                controller,
                When::YourCreatureDealsCombatDamageToPlayerAmount,
                None,
            );
            for t in &mut self.triggered[before..] {
                t.amount = amount;
            }
        }
        for (a, amount) in hit_player {
            let before = self.triggered.len();
            self.fire_source(a, When::DealsCombatDamageToPlayer);
            for t in &mut self.triggered[before..] {
                t.amount = amount;
            }
            if let Some(bello) = self.bello_for(a) {
                let controller = self.obj(a).controller;
                self.triggered.push(StackItem {
                    kind: StackKind::Trigger(0),
                    ..Game::casting(bello, controller)
                });
            }
            let zcc = self.obj(a).zcc;
            let turn = self.turn;
            for (c, z, src, n, t) in self.granted_combat_damage.clone() {
                if c == a && z == zcc && t == turn {
                    let controller = self.obj(a).controller;
                    self.triggered.push(StackItem {
                        kind: StackKind::Trigger(n),
                        amount,
                        ..Game::casting(src, controller)
                    });
                }
            }
            let gear: Vec<ObjId> = self
                .battlefield
                .iter()
                .copied()
                .filter(|&e| self.obj(e).attached_to == Some((a, zcc)))
                .collect();
            for e in gear {
                let before = self.triggered.len();
                self.fire_source(e, When::EquippedDealsCombatDamageToPlayer);
                for t in &mut self.triggered[before..] {
                    t.amount = amount; // "that many" (The Reaver Cleaver)
                }
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
        let stage = self.obj(id).stage;
        let whens: Vec<(When, Option<Condition>)> = self
            .def_of(id)
            .triggers
            .iter()
            .map(|tr| (tr.when, tr.ability.condition.clone()))
            .collect();
        for (n, (w, cond)) in whens.into_iter().enumerate() {
            if let Some(Condition::StageAtLeast(s)) = cond {
                if stage < s {
                    continue; // not gained yet
                }
            }
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
        let o = self.obj(source);
        let new_creature =
            o.zone == Zone::Battlefield && o.since_turn == self.turn && self.types_of(source).creature;
        let neriv = if new_creature {
            let n = self
                .permanents(c)
                .filter(|&x| self.def_of(x).doubles_damage_of_new_creatures && !self.lost_abilities(x))
                .count();
            1 << n.min(10)
        } else {
            1
        };
        let violence = if self.types_of(source).creature && o.zone == Zone::Battlefield {
            let n = self
                .permanents(c)
                .filter(|&x| self.def_of(x).doubles_your_creatures_damage && !self.lost_abilities(x))
                .count();
            1 << n.min(10)
        } else {
            1
        };
        let fiery = self
            .permanents(c)
            .filter(|&x| self.def_of(x).triples_your_damage && !self.lost_abilities(x))
            .count() as u32;
        neriv * violence * 3i32.pow(fiery.min(10)) * self.opponent_damage_doubling(source, target)
    }

    fn opponent_damage_doubling(&self, source: ObjId, target: Target) -> i32 {
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
        // Malignus; Questing Beast ("combat damage ... by creatures you control can't be prevented").
        let unpreventable = self.def_of(source).damage_cant_be_prevented
            || (self.in_combat_damage
                && self.types_of(source).creature
                && self
                    .permanents(self.obj(source).controller)
                    .any(|x| self.def_of(x).your_combat_damage_cant_be_prevented && !self.lost_abilities(x)));
        if !unpreventable {
            if self.in_combat_damage && self.combat_damage_prevented_turn == Some(self.turn) {
                return; // Spore Frog
            }
            if self.in_combat_damage
                && self
                    .combat_damage_by_prevented
                    .contains(&(source, self.obj(source).zcc))
            {
                return; // Maze of Ith
            }
            if let Target::Player(p) = target {
                if self.damage_to_player_prevented_turn[p] == Some(self.turn) {
                    return; // Riot Control
                }
            }
        }
        if let (false, Target::Object(id)) = (unpreventable, target) {
            if self.protected_from(id, source) {
                return; // CR 702.16e: damage from a source of that color is prevented
            }
            let zcc = self.obj(id).zcc;
            if self.in_combat_damage && self.combat_damage_prevented.contains(&(id, zcc)) {
                return;
            }
            // Vigor: damage to another creature you control becomes +1/+1 counters.
            let controller = self.obj(id).controller;
            if amount > 0
                && self.obj(id).zone == Zone::Battlefield
                && self.types_of(id).creature
                && self.permanents(controller).any(|v| {
                    v != id && self.def_of(v).damage_to_yours_becomes_counters && !self.lost_abilities(v)
                })
            {
                self.add_counters(id, amount);
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
        if let Target::Player(p) = target {
            let turn = self.turn;
            if let Some(pos) = self
                .damage_shields
                .iter()
                .position(|e| e.0 == p && e.1 == source && e.4 == turn)
            {
                // Prevented (CR 615); "When damage is prevented this way, ..." (CR 603.12).
                let (_, _, card, n, _) = self.damage_shields.remove(pos);
                if amount > 0 {
                    self.triggered.push(StackItem {
                        kind: StackKind::Trigger(n),
                        amount,
                        extra: Some(source),
                        ..Game::casting(card, p)
                    });
                }
                return;
            }
        }
        if amount > 0 && self.obj(source).zone == Zone::Battlefield {
            self.obj_mut(source).dealt_damage = true;
        }
        let k = self.keywords(source);
        let infect = self.has_infect(source);
        match target {
            Target::Player(p) if infect => {
                self.players[p].poison += amount; // CR 702.90b: poison counters, not life loss
            }
            Target::Object(id) if infect && self.types_of(id).creature => {
                self.obj_mut(id).counters -= amount; // CR 702.90c: -1/-1 counters
            }
            Target::Player(p) => {
                let turn = self.turn;
                let (t, total) = self.damage_to_player_turn[p];
                self.damage_to_player_turn[p] = (turn, if t == turn { total } else { 0 } + amount);
                self.lose_life(p, amount);
                let controller = self.obj(source).controller;
                if amount >= 5 {
                    self.fire(controller, When::YourSourceDealsFiveOrMoreToPlayer, None);
                    // Dragonborn Champion
                }
                if amount >= 6 && controller != p {
                    self.fire_source(source, When::DealsSixOrMoreToOpponent); // Deus of Calamity
                }
                if !self.in_combat_damage && controller != p {
                    let before = self.triggered.len();
                    self.fire(controller, When::YourSourceDealsNoncombatDamageToOpponent, None);
                    for t in &mut self.triggered[before..] {
                        t.amount = amount;
                    }
                }
            }
            Target::Object(id) if self.types_of(id).planeswalker && !self.types_of(id).creature => {
                self.obj_mut(id).loyalty -= amount; // CR 120.3c
            }
            Target::Object(id) => {
                let o = self.obj_mut(id);
                o.damage += amount;
                o.deathtouched |= k.has(Keywords::DEATHTOUCH);
                if amount > 0 {
                    let before = self.triggered.len();
                    self.fire_source(id, When::DealtDamage);
                    for t in &mut self.triggered[before..] {
                        t.amount = amount;
                    }
                }
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
        // Roil Elemental: control lasts only as long as you control the source.
        if !self.control_while.is_empty() {
            let mut keep = Vec::new();
            for e in std::mem::take(&mut self.control_while) {
                let (id, zcc, src, src_zcc, p, before) = e;
                let o = self.obj(id);
                if o.zcc != zcc || o.zone != Zone::Battlefield {
                    continue; // it left: the effect ends
                }
                let s = self.obj(src);
                if s.zcc == src_zcc && s.zone == Zone::Battlefield && s.controller == p {
                    keep.push(e);
                } else {
                    let turn = self.turn;
                    let o = self.obj_mut(id);
                    o.controller = before;
                    o.since_turn = turn;
                }
            }
            self.control_while = keep;
        }
        loop {
            self.work();
            // Ascend (CR 702.131b): ten or more permanents and an ascend permanent.
            for p in 0..2 {
                if !self.city_blessing[p]
                    && self.permanents(p).count() >= 10
                    && self
                        .permanents(p)
                        .any(|x| self.def_of(x).ascend && !self.lost_abilities(x))
                {
                    self.city_blessing[p] = true;
                }
            }
            let mut dies = Vec::new();
            for &id in &self.battlefield {
                if !self.types_of(id).creature {
                    continue;
                }
                let o = self.obj(id);
                let t = self.toughness(id);
                let indestructible = self.keywords(id).has(Keywords::INDESTRUCTIBLE);
                if t <= 0 {
                    dies.push(id); // CR 704.5f
                } else if !indestructible
                    && ((o.damage >= t && !self.lethal_by_power(id))
                        || (self.lethal_by_power(id) && o.damage > 0 && o.damage >= self.power(id))
                        || (o.deathtouched && o.damage > 0))
                {
                    dies.push(id); // CR 704.5g, 704.5h
                }
            }
            // Destruction by lethal damage can be regenerated (CR 701.15); zero toughness can't.
            let regenerate: Vec<ObjId> = dies
                .iter()
                .copied()
                .filter(|&id| {
                    self.toughness(id) > 0 && {
                        let (t, n) = self.obj(id).regen_shields;
                        t == self.turn && n > 0
                    }
                })
                .collect();
            for id in regenerate {
                self.use_regen_shield(id);
                dies.retain(|&d| d != id);
            }
            // Umbra armor saves a creature destroyed by lethal damage (not by zero toughness).
            for id in dies.clone() {
                if self.toughness(id) > 0 && self.use_umbra_armor(id) {
                    dies.retain(|&d| d != id);
                }
            }
            dies.extend(self.legend_rule_losers());
            // CR 714.4: a Saga with lore counters at least its final chapter, and no chapter ability
            // of it waiting or on the stack, is sacrificed.
            for &id in &self.battlefield {
                let fin = self.final_chapter(id);
                if self.is_saga(id) && fin > 0 && self.obj(id).lore >= fin {
                    let pending_chapter = self
                        .items
                        .iter()
                        .chain(self.triggered.iter())
                        .any(|i| i.source == id && matches!(i.kind, StackKind::Trigger(_)));
                    let resolving = self.resolving.as_ref().is_some_and(|r| r.item.source == id);
                    if !pending_chapter && !resolving {
                        dies.push(id);
                    }
                }
            }
            // CR 704.5i: a planeswalker with no loyalty goes to the graveyard.
            dies.extend(
                self.battlefield
                    .iter()
                    .copied()
                    .filter(|&w| self.types_of(w).planeswalker && self.obj(w).loyalty <= 0),
            );
            // CR 704.5m-n: an Aura attached to nothing legal goes to the graveyard; an Equipment
            // becomes unattached.
            for &id in &self.battlefield.clone() {
                if let Some((t, z)) = self.obj(id).attached_to {
                    let o = self.obj(t);
                    let ty = self.types_of(t);
                    // An Aura checks its own "enchant" restriction (CR 303.4d); Equipment needs a
                    // creature (CR 301.5c).
                    let fits = match self.def_of(id).spell.as_ref().and_then(|s| s.targets.first()) {
                        Some(f) if self.def_of(id).aura => {
                            (ty.creature && f.creatures)
                                || (ty.land && f.lands)
                                || (ty.planeswalker && f.planeswalkers)
                        }
                        _ => ty.creature,
                    };
                    let ok = o.zone == Zone::Battlefield && o.zcc == z && fits;
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
                    ((self.defs[o.def].token || o.copy_token)
                        && !matches!(o.zone, Zone::Battlefield | Zone::Gone))
                        || (o.card_copy
                            && !matches!(o.zone, Zone::Battlefield | Zone::Stack | Zone::Gone)
                            && !(o.zone == Zone::Exile && self.free_casts.contains(&o.id)))
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
                // CR 704.6c: 21 combat damage from one commander. `commander_damage` is the total from
                // the opponent's commanders: the same thing when they have only one.
                let one_commander = self.partners[opponent(p)].is_none();
                let commander_dead = self
                    .commander_damage_from
                    .iter()
                    .any(|e| e.1 == p && e.2 >= COMMANDER_DAMAGE_LOSS)
                    || (one_commander && self.commander_damage[p] >= COMMANDER_DAMAGE_LOSS);
                let poisoned = pl.poison >= 10; // CR 704.5c
                if !pl.lost
                    && !protected
                    && (pl.life <= 0 || pl.drew_from_empty || commander_dead || poisoned)
                {
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
            if !self.types_of(a).legendary || self.legend_rule_off(self.obj(a).controller) {
                continue;
            }
            let name = self.name_of(a);
            let newer = self.battlefield[i + 1..].iter().any(|&b| {
                self.obj(b).controller == self.obj(a).controller
                    && self.name_of(b) == name
                    && self.types_of(b).legendary
            });
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
        c.twobrid[i] += b.twobrid[i];
    }
    for i in 0..c.hybrid.len() {
        c.hybrid[i] += b.hybrid[i];
    }
    c
}
