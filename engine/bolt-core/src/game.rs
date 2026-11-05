//! The game: state, legal actions, and the only function that changes state, [`Game::apply`].
//!
//! Rules are cited by Comprehensive Rules (CR) section. Simplifications are listed in
//! docs/engine/QUESTIONS.md, each with the reason and how to reverse it.

use std::sync::Arc;

use crate::card::{CardDef, Keywords};
use crate::mana::{self, ManaSource};
use crate::rng::Rng;

pub type PlayerId = usize;

/// Starting life total (CR 103.4).
pub const STARTING_LIFE: i32 = 20;
/// Opening hand size (CR 103.5).
pub const HAND_SIZE: usize = 7;
/// Maximum hand size, enforced in the cleanup step (CR 402.2, 514.1).
pub const MAX_HAND: usize = 7;

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug)]
pub struct Game {
    pub defs: Arc<Vec<CardDef>>,
    pub objects: Vec<Object>,
    pub players: [Player; 2],
    pub battlefield: Vec<ObjId>,
    pub stack: Vec<ObjId>,
    pub exile: Vec<ObjId>,
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
            exile: Vec::new(),
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

    pub fn keywords(&self, id: ObjId) -> Keywords {
        self.def_of(id).keywords
    }

    pub fn power(&self, id: ObjId) -> i32 {
        self.def_of(id).power
    }

    pub fn toughness(&self, id: ObjId) -> i32 {
        self.def_of(id).toughness
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
        }
    }

    /// Moves a card between zones. Leaving the battlefield clears its per-permanent state (CR 400.7).
    fn move_to(&mut self, id: ObjId, to: Zone) {
        let (owner, from) = (self.obj(id).owner, self.obj(id).zone);
        let list = self.zone_list(owner, from);
        let pos = list.iter().position(|&x| x == id).expect("object is in its zone");
        list.remove(pos);
        self.zone_list(owner, to).push(id);
        let turn = self.turn;
        let o = self.obj_mut(id);
        o.zone = to;
        o.tapped = false;
        o.damage = 0;
        o.deathtouched = false;
        if to != Zone::Battlefield && to != Zone::Stack {
            o.controller = o.owner;
        }
        if to == Zone::Battlefield {
            o.since_turn = turn;
        }
        if from == Zone::Battlefield {
            if let Some(c) = &mut self.combat {
                c.attackers.retain(|&a| a != id);
            }
        }
        self.events.push(Event::Moved { card: id, from, to });
    }

    /// Draws a card. Drawing from an empty library loses at the next check (CR 704.5b).
    fn draw(&mut self, p: PlayerId) {
        match self.players[p].library.last().copied() {
            Some(card) => {
                self.move_to(card, Zone::Hand);
                self.events.push(Event::Drew { player: p, card });
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
                let mut v: Vec<Action> = self.attack_candidates(p).map(Action::Attack).collect();
                v.push(Action::FinishAttacks);
                v
            }
            Pending::DeclareBlockers(p) => self.block_actions(p),
        }
    }

    fn priority_actions(&self, p: PlayerId) -> Vec<Action> {
        let mut v = vec![Action::PassPriority];
        let sorcery_timing =
            p == self.active && matches!(self.step, Step::Main1 | Step::Main2) && self.stack.is_empty();
        let sources = self.untapped_sources(p);
        for &c in &self.players[p].hand {
            let def = self.def_of(c);
            if def.types.land {
                // CR 305.2: one land per turn, at sorcery timing.
                if sorcery_timing && self.players[p].lands_played == 0 {
                    v.push(Action::PlayLand(c));
                }
            } else if def.types.creature
                && (sorcery_timing || def.keywords.has(Keywords::FLASH))
                && mana::pay(&def.cost, &sources.iter().map(|s| s.1).collect::<Vec<_>>()).is_some()
            {
                v.push(Action::Cast(c));
            }
        }
        v
    }

    fn untapped_sources(&self, p: PlayerId) -> Vec<(ObjId, ManaSource)> {
        self.permanents(p)
            .filter(|&id| !self.obj(id).tapped)
            .filter_map(|id| self.def_of(id).mana.map(|m| (id, m)))
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
        if !self.legal_actions().contains(&action) {
            return Err(RulesError::Illegal(action));
        }
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
            (Pending::Priority(p), Action::Cast(c)) => self.cast(p, c),
            (Pending::DeclareAttackers(_), Action::Attack(c)) => {
                // CR 508.1f: attacking taps the creature unless it has vigilance.
                if !self.keywords(c).has(Keywords::VIGILANCE) {
                    self.obj_mut(c).tapped = true;
                }
                self.combat.as_mut().expect("in combat").attackers.push(c);
            }
            (Pending::DeclareAttackers(_), Action::FinishAttacks) => {
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
            self.turn = 0;
            self.active = opponent(self.first);
            self.begin_next_turn();
        }
    }

    /// CR 601.2: move to the stack, pay, then the caster gets priority again (CR 117.3c).
    fn cast(&mut self, p: PlayerId, card: ObjId) {
        let sources = self.untapped_sources(p);
        let cost = self.def_of(card).cost;
        let paid = mana::pay(&cost, &sources.iter().map(|s| s.1).collect::<Vec<_>>())
            .expect("legal casts are payable");
        for i in paid {
            self.obj_mut(sources[i].0).tapped = true;
        }
        self.move_to(card, Zone::Stack);
        self.obj_mut(card).controller = p;
        self.events.push(Event::Cast { card });
        self.passes = 0;
        self.give_priority(p);
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
        if let Some(&top) = self.stack.last() {
            self.resolve(top);
            self.give_priority(self.active);
        } else {
            self.end_step();
        }
    }

    fn resolve(&mut self, card: ObjId) {
        let controller = self.obj(card).controller;
        if self.def_of(card).types.is_permanent() {
            self.move_to(card, Zone::Battlefield);
            self.obj_mut(card).controller = controller;
        } else {
            self.move_to(card, Zone::Graveyard);
        }
        self.events.push(Event::Resolved { card });
    }

    /// State-based actions run before any player would receive priority (CR 117.5, 704.3).
    fn give_priority(&mut self, p: PlayerId) {
        self.state_based_actions();
        if !self.is_over() {
            self.pending = Pending::Priority(p);
        }
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
        self.turn += 1;
        self.active = opponent(self.active);
        self.events.push(Event::TurnBegan {
            turn: self.turn,
            active: self.active,
        });
        self.enter(Step::Untap);
    }

    fn enter(&mut self, step: Step) {
        self.step = step;
        let a = self.active;
        match step {
            Step::PreGame => unreachable!("the game never returns to pre-game"),
            Step::Untap => {
                // CR 502.3: untap everything the active player controls. No priority (CR 502.4).
                for id in self.permanents(a).collect::<Vec<_>>() {
                    self.obj_mut(id).tapped = false;
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
                let combat = self.combat.as_mut().expect("in combat");
                combat.first_strike_step = combat.attackers.iter().any(|&id| {
                    let k = self.defs[self.objects[id.0 as usize].def].keywords;
                    k.has(Keywords::FIRST_STRIKE) || k.has(Keywords::DOUBLE_STRIKE)
                });
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
                if self.players[a].hand.len() > MAX_HAND {
                    self.pending = Pending::Discard(a);
                } else {
                    self.finish_cleanup();
                }
            }
            Step::Upkeep | Step::Main1 | Step::BeginCombat | Step::Main2 | Step::End => {
                self.give_priority(a);
            }
        }
    }

    /// CR 514.2: damage wears off. No player gets priority in cleanup (CR 514.3).
    fn finish_cleanup(&mut self) {
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
                assigned.push((a, Target::Player(defender), power));
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
                assigned.push((a, Target::Player(defender), left));
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
        for (source, target, amount) in assigned {
            self.deal_damage(source, target, amount);
        }
        self.state_based_actions();
    }

    /// CR 120.3: damage to a player is life loss; to a creature it is marked. Lifelink gains life
    /// for the source's controller (CR 702.15b).
    fn deal_damage(&mut self, source: ObjId, target: Target, amount: i32) {
        let k = self.keywords(source);
        match target {
            Target::Player(p) => {
                self.players[p].life -= amount;
                self.events.push(Event::LifeLost { player: p, amount });
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
            self.players[c].life += amount;
            self.events.push(Event::LifeGained { player: c, amount });
        }
    }

    // ---------------------------------------------------------------- state-based actions

    /// CR 704: repeat until none apply.
    fn state_based_actions(&mut self) {
        loop {
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
            dies.sort_unstable();
            dies.dedup();
            let mut lost = false;
            for p in 0..2 {
                let pl = &self.players[p];
                if !pl.lost && (pl.life <= 0 || pl.drew_from_empty) {
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
        if let Some(i) = seen.iter().position(|&n| n != 1) {
            return Err(format!("object {i} is listed in {} zones", seen[i]));
        }
        for o in &self.objects {
            if o.damage < 0 {
                return Err(format!("negative damage on {o:?}"));
            }
            if o.zone != Zone::Battlefield && (o.tapped || o.damage != 0) {
                return Err(format!("off-battlefield object carries state: {o:?}"));
            }
        }
        if self.is_over() != (self.pending == Pending::GameOver) {
            return Err("result and pending disagree".into());
        }
        Ok(())
    }
}
