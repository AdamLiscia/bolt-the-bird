//! The d4 subsystems' own state and functions: dungeons, venture and the initiative (CR 309,
//! 701.49, 725), completed dungeons, emblems with triggered abilities (CR 114), dice (CR 706), the
//! Ring (CR 701.54), Saga chapter grants, and split cards' halves on the stack (CR 709.3).
//! The hooks other lanes' code calls are [`Game::fire_subsystems`] (from `fire()`), and, not called
//! yet, [`Game::ring_makes_legendary`] (d3), [`Game::ring_evasion_forbids_block`] (d1),
//! [`Game::ring_bearer_dealt_combat_damage`] (d2), [`Game::lacks_chapter_activated`] (merger) and
//! [`Game::lacks_chapter_mana`] (d1).

use super::*;
use crate::effect::{Dungeon, SubsystemEffect};

/// One room of a dungeon (CR 309.4): its room ability, and the rooms its arrows lead to.
pub struct Room {
    pub name: &'static str,
    pub ability: Ability,
    pub next: &'static [u8],
    /// False while a piece of the room ability is missing from the engine (its token or effect);
    /// every room is ready now.
    pub ready: bool,
}

fn room(name: &'static str, targets: Vec<Filter>, mut effects: Vec<Effect>, next: &'static [u8]) -> Room {
    if next.is_empty() {
        // The bottommost room: its ability leaving the stack completes the dungeon (CR 704.5t).
        effects.push(Effect::Subsystem(SubsystemEffect::CompleteDungeon));
    }
    Room {
        name,
        ability: Ability::new(targets, effects),
        next,
        ready: true,
    }
}

fn token(name: &'static str, power: i32, toughness: i32, n: u8) -> Effect {
    Effect::Subsystem(SubsystemEffect::RoomToken {
        name,
        power,
        toughness,
        n,
    })
}

fn scry(n: u8) -> Effect {
    Effect::Scry { n }
}

fn draw(n: u8) -> Effect {
    Effect::Draw { who: Who::You, n }
}

/// The four dungeon cards' rooms, top to bottom (Adventures in the Forgotten Realms, and
/// Undercity from Commander Legends: Battle for Baldur's Gate).
pub fn rooms(d: Dungeon) -> &'static [Room] {
    if d == Dungeon::TheRing {
        return ring_abilities();
    }
    static ROOMS: std::sync::OnceLock<[Vec<Room>; 4]> = std::sync::OnceLock::new();
    let all = ROOMS.get_or_init(|| {
        let you = |n| Effect::GainLife { who: Who::You, n };
        let lose = |who, n| Effect::LoseLife { who, n };
        let treasure = || token("Treasure", 0, 0, 1);
        let creature = || vec![Filter::CREATURE];
        [
            vec![
                room("Cave Entrance", vec![], vec![scry(1)], &[1, 2]),
                room("Goblin Lair", vec![], vec![token("Goblin", 1, 1, 1)], &[3, 4]),
                room("Mine Tunnels", vec![], vec![treasure()], &[4, 5]),
                room(
                    "Storeroom",
                    creature(),
                    vec![Effect::Counters { slot: 0, n: 1 }],
                    &[6],
                ),
                room(
                    "Dark Pool",
                    vec![],
                    vec![lose(Who::EachOpponent, 1), you(1)],
                    &[6],
                ),
                // "Target creature gets -4/-0 until your next turn."
                room(
                    "Fungi Cavern",
                    creature(),
                    vec![Effect::PumpUntilYourNextTurn {
                        slot: 0,
                        power: -4,
                        toughness: 0,
                    }],
                    &[6],
                ),
                room("Temple of Dumathoin", vec![], vec![draw(1)], &[]),
            ],
            vec![
                room("Yawning Portal", vec![], vec![you(1)], &[1]),
                room("Dungeon Level", vec![], vec![scry(1)], &[2, 3]),
                room("Goblin Bazaar", vec![], vec![treasure()], &[4]),
                // "Target creature can't attack until your next turn."
                room(
                    "Twisted Caverns",
                    creature(),
                    vec![Effect::CantAttackUntilYourNextTurn(0)],
                    &[4],
                ),
                room("Lost Level", vec![], vec![scry(2)], &[5, 6]),
                // "Exile the top two cards of your library. You may play them." (this turn)
                room(
                    "Runestone Caverns",
                    vec![],
                    vec![Effect::ExileTopToPlayThisTurn(2)],
                    &[7],
                ),
                room(
                    "Muiral's Graveyard",
                    vec![],
                    vec![token("Skeleton", 1, 1, 2)],
                    &[7],
                ),
                room("Deep Mines", vec![], vec![scry(3)], &[8]),
                // "Draw three cards and reveal them. You may cast one of them without paying its
                // mana cost."
                room(
                    "Mad Wizard's Lair",
                    vec![],
                    vec![Effect::DrawThreeMayCastOneFree],
                    &[],
                ),
            ],
            vec![
                room(
                    "Trapped Entry",
                    vec![],
                    vec![lose(Who::You, 1), lose(Who::EachOpponent, 1)],
                    &[1, 2],
                ),
                // "Each player loses 2 life unless they discard a card."
                room(
                    "Veils of Fear",
                    vec![],
                    vec![Effect::EachPlayerLosesUnless {
                        life: 2,
                        sacrifice: false,
                    }],
                    &[3],
                ),
                // "Discard a card and sacrifice an artifact, a creature, and a land."
                room(
                    "Oubliette",
                    vec![],
                    vec![
                        Effect::Discard { who: Who::You, n: 1 },
                        Effect::SacrificeArtifactCreatureAndLand,
                    ],
                    &[4],
                ),
                // "Each player loses 2 life unless they sacrifice an artifact, a creature, or a land."
                room(
                    "Sandfall Cell",
                    vec![],
                    vec![Effect::EachPlayerLosesUnless {
                        life: 2,
                        sacrifice: true,
                    }],
                    &[4],
                ),
                room(
                    "Cradle of the Death God",
                    vec![],
                    vec![token("God Horror", 4, 4, 1)],
                    &[],
                ),
            ],
            vec![
                room(
                    "Secret Entrance",
                    vec![],
                    vec![Effect::Search {
                        what: SearchFor::BasicLand,
                        to: SearchTo::Hand,
                        n: 1,
                        kicked_n: None,
                    }],
                    &[1, 2],
                ),
                room(
                    "Forge",
                    creature(),
                    vec![Effect::Counters { slot: 0, n: 2 }],
                    &[3, 4],
                ),
                room("Lost Well", vec![], vec![scry(2)], &[4, 5]),
                room(
                    "Trap!",
                    vec![Filter::PLAYER],
                    vec![lose(Who::TargetPlayer(0), 5)],
                    &[6],
                ),
                // "Goad target creature."
                room("Arena", creature(), vec![Effect::Goad(0)], &[6, 7]),
                room("Stash", vec![], vec![treasure()], &[7]),
                room("Archives", vec![], vec![draw(1)], &[8]),
                room("Catacombs", vec![], vec![token("Skeleton", 4, 1, 1)], &[8]),
                // "Reveal the top ten cards of your library. Put a creature card from among them onto
                // the battlefield with three +1/+1 counters on it. It gains hexproof until your next
                // turn. Then shuffle."
                room(
                    "Throne of the Dead Three",
                    vec![],
                    vec![Effect::RevealTopCreatureOntoBattlefield {
                        n: 10,
                        counters: 3,
                        keywords: crate::Keywords::HEXPROOF,
                    }],
                    &[],
                ),
            ],
        ]
    });
    &all[d as usize]
}

/// The Ring emblem's triggered abilities (CR 701.54c), as `StackKind::Room` rows of
/// `Dungeon::TheRing`; its first ability is a static one (`Game::ring_makes_legendary`,
/// `Game::ring_evasion_forbids_block`).
pub mod ring {
    /// "Whenever your Ring-bearer attacks, draw a card, then discard a card." (level 2)
    pub const LOOT: u8 = 0;
    /// "Whenever your Ring-bearer becomes blocked by a creature, that creature's controller
    /// sacrifices it at end of combat." (level 3)
    pub const SACRIFICE_BLOCKER: u8 = 1;
    /// "Whenever your Ring-bearer deals combat damage to a player, each opponent loses 3 life."
    /// (level 4)
    pub const DRAIN: u8 = 2;
}

fn ring_abilities() -> &'static [Room] {
    static RING: std::sync::OnceLock<Vec<Room>> = std::sync::OnceLock::new();
    RING.get_or_init(|| {
        let row = |name, effects| Room {
            name,
            ability: Ability::new(vec![], effects),
            next: &[],
            ready: true,
        };
        vec![
            row(
                "The Ring: loot",
                vec![draw(1), Effect::Discard { who: Who::You, n: 1 }],
            ),
            row(
                "The Ring: sacrifice the blocker",
                vec![Effect::Subsystem(SubsystemEffect::RingSacrificeBlocker)],
            ),
            row(
                "The Ring: each opponent loses 3 life",
                vec![Effect::LoseLife {
                    who: Who::EachOpponent,
                    n: 3,
                }],
            ),
        ]
    })
}

/// The room ability of the initiative (CR 725.2): "that player ventures into Undercity."
fn initiative_ability() -> &'static Ability {
    static A: std::sync::OnceLock<Ability> = std::sync::OnceLock::new();
    A.get_or_init(|| Ability::new(vec![], vec![Effect::Subsystem(SubsystemEffect::VentureUndercity)]))
}

/// The dungeons "venture into the dungeon" may choose, as `Mode` answers (CR 701.49a: not Undercity).
pub const VENTURE_CHOICES: [Dungeon; 3] = [Dungeon::LostMine, Dungeon::MadMage, Dungeon::Tomb];

impl Game {
    /// The ability a d4 stack entry carries out (`ability_of` for its own kinds).
    pub(super) fn subsystem_ability(&self, kind: StackKind) -> &Ability {
        match kind {
            StackKind::Room { dungeon, room } => &rooms(dungeon)[room as usize].ability,
            _ => initiative_ability(),
        }
    }

    /// A split card on the stack: the colors of the half being cast, from its mana cost (CR 709.3,
    /// 105.2), or None if its stack entry isn't found.
    pub(super) fn split_half_colors(&self, id: ObjId) -> Option<[bool; 5]> {
        let cost = self.split_half_cost(id)?;
        let mut c = [false; 5];
        for (i, x) in c.iter_mut().enumerate() {
            *x = cost.colored[i] > 0 || cost.twobrid[i] > 0;
        }
        for (k, &(a, b)) in crate::mana::PAIRS.iter().enumerate() {
            if cost.hybrid[k] > 0 {
                c[a] = true;
                c[b] = true;
            }
        }
        Some(c)
    }

    /// Rolls a die with `sides` sides (CR 706.1): 1 through `sides`.
    pub fn roll_die(&mut self, sides: u32) -> u32 {
        self.rng.below(sides.max(1) as usize) as u32 + 1
    }

    /// Flips a coin (CR 705): true if its flipper wins the flip (a call guessed right, 1 in 2).
    pub fn flip_coin(&mut self) -> bool {
        self.rng.below(2) == 0
    }

    /// Whether `p` has completed a dungeon this game (CR 309.7).
    pub fn completed_a_dungeon(&self, p: PlayerId) -> bool {
        self.completed_dungeons[p] > 0
    }

    /// How many times a room ability of `p`'s dungeon triggers: once, plus once for each "Room
    /// abilities of dungeons you own trigger an additional time" `p` has (Hama Pashar; Dungeon
    /// Delver's commander creatures).
    pub fn room_trigger_count(&self, p: PlayerId) -> usize {
        let mut n = 1;
        for x in self.battlefield.clone() {
            if self.lost_abilities(x) {
                continue;
            }
            let ext = &self.def_of(x).subsystems;
            let q = self.obj(x).controller;
            if ext.rooms_trigger_again && q == p {
                n += 1;
            }
            if ext.commanders_rooms_trigger_again {
                // Commander creatures q owns have the ability; it counts for their controller.
                for c in [self.commanders[q], self.partners[q]].into_iter().flatten() {
                    let o = self.obj(c);
                    if o.zone == Zone::Battlefield
                        && o.owner == q
                        && o.controller == p
                        && self.types_of(c).creature
                    {
                        n += 1;
                    }
                }
            }
        }
        n
    }

    /// `p`'s venture marker moves into `room` of `dungeon`: its room ability triggers (CR 309.4c),
    /// with `source` as the stack entry's source.
    fn enter_room(&mut self, p: PlayerId, source: ObjId, dungeon: Dungeon, room: u8) {
        self.venture[p] = Some((dungeon, room));
        for _ in 0..self.room_trigger_count(p) {
            self.triggered.push(StackItem {
                kind: StackKind::Room { dungeon, room },
                ..Game::casting(source, p)
            });
        }
    }

    /// Venture into the dungeon (CR 701.49), or into Undercity with `undercity`: enter the first room
    /// of a dungeon (chosen, unless it's Undercity), or advance to the next room (chosen if there are
    /// two).
    pub fn venture_into(&mut self, p: PlayerId, source: ObjId, undercity: bool) {
        match self.venture[p] {
            Some((d, r)) if !rooms(d)[r as usize].next.is_empty() => {
                let next = rooms(d)[r as usize].next;
                if next.len() == 1 {
                    self.enter_room(p, source, d, next[0]);
                } else {
                    self.ask(
                        p,
                        Decision::ChooseRoom {
                            dungeon: d,
                            from: r,
                            source,
                        },
                    );
                }
            }
            current => {
                if let Some((d, r)) = current {
                    // On the bottommost room still: that dungeon is done (CR 704.5t), then a new one.
                    self.complete_dungeon(p, d, r);
                }
                if undercity {
                    self.enter_room(p, source, Dungeon::Undercity, 0);
                } else {
                    self.ask(p, Decision::ChooseDungeon { source });
                }
            }
        }
    }

    /// CR 704.5t, 309.7: `p` completes the dungeon if its marker is still on that bottommost room.
    fn complete_dungeon(&mut self, p: PlayerId, d: Dungeon, r: u8) {
        if self.venture[p] == Some((d, r)) && rooms(d)[r as usize].next.is_empty() {
            self.venture[p] = None;
            self.completed_dungeons[p] = self.completed_dungeons[p].saturating_add(1);
            self.fire(p, When::YouCompleteDungeon, None); // lane d2's emit (Sefris)
        }
    }

    /// `p` takes the initiative (CR 725.1), and then ventures into Undercity (CR 725.2, a triggered
    /// ability).
    pub fn take_initiative(&mut self, p: PlayerId, source: ObjId) {
        self.initiative = Some((p, source));
        self.triggered.push(StackItem {
            kind: StackKind::InitiativeVenture,
            ..Game::casting(source, p)
        });
    }

    /// The subsystems' share of `fire(p, when)`: emblems' triggered abilities, the initiative's
    /// upkeep venture, and taking the initiative by combat damage (CR 725.2). Called by `fire()`.
    pub fn fire_subsystems(&mut self, p: PlayerId, when: When) {
        for (owner, kind, card) in self.emblems.clone() {
            if let EmblemKind::Triggered { when: w, trigger } = kind {
                if owner == p && w == when {
                    self.triggered.push(StackItem {
                        kind: StackKind::Trigger(trigger),
                        ..Game::casting(card, owner)
                    });
                }
            }
        }
        self.fire_ring(p, when);
        match (when, self.initiative) {
            (When::Upkeep { your_turn_only: true }, Some((q, source))) if q == p => {
                self.triggered.push(StackItem {
                    kind: StackKind::InitiativeVenture,
                    ..Game::casting(source, p)
                });
            }
            // "Whenever one or more creatures a player controls deal combat damage to the player who
            // has the initiative, that creature's controller takes the initiative."
            (When::YourCreaturesDealCombatDamageToPlayer, Some((q, source))) if q == opponent(p) => {
                self.take_initiative(p, source);
            }
            _ => {}
        }
    }

    /// Legal answers to the subsystems' decisions.
    pub(super) fn subsystem_decision_actions(&self, d: &Decision) -> Vec<Action> {
        match *d {
            Decision::ChooseDungeon { .. } => (0..VENTURE_CHOICES.len() as u8).map(Action::Mode).collect(),
            Decision::ChooseRoom { dungeon, from, .. } => (0..rooms(dungeon)[from as usize].next.len() as u8)
                .map(Action::Mode)
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Carries out an answer to one of the subsystems' decisions.
    pub(super) fn subsystem_decide(&mut self, p: PlayerId, d: Decision, a: Action) {
        match (d, a) {
            (Decision::ChooseDungeon { source }, Action::Mode(m)) => {
                let d = VENTURE_CHOICES[(m as usize).min(VENTURE_CHOICES.len() - 1)];
                self.enter_room(p, source, d, 0);
            }
            (
                Decision::ChooseRoom {
                    dungeon,
                    from,
                    source,
                },
                Action::Mode(m),
            ) => {
                let next = rooms(dungeon)[from as usize].next;
                self.enter_room(p, source, dungeon, next[(m as usize).min(next.len() - 1)]);
            }
            (Decision::UntapOwnCreature, Action::Pick(c)) => self.untap_by_effect(c),
            (Decision::ChooseRingBearer, Action::Pick(c)) => self.choose_ring_bearer(p, c),
            (d, a) => unreachable!("legal action {a:?} unhandled for {d:?}"),
        }
    }

    /// Resolves one of the subsystems' effects.
    pub(super) fn subsystem_effect(&mut self, item: &StackItem, legal: &[bool], e: SubsystemEffect) {
        let p = item.controller;
        match e {
            SubsystemEffect::Venture => self.venture_into(p, item.source, false),
            SubsystemEffect::VentureUndercity => self.venture_into(p, item.source, true),
            SubsystemEffect::TakeInitiative => self.take_initiative(p, item.source),
            SubsystemEffect::RoomToken {
                name,
                power,
                toughness,
                n,
            } => {
                let def = self.defs.iter().position(|d| {
                    d.token
                        && d.name == name
                        && (!d.types.creature || (d.power == power && d.toughness == toughness))
                });
                if let Some(def) = def {
                    for _ in 0..n {
                        self.create_token(p, def);
                    }
                }
            }
            SubsystemEffect::RollDieCreateTokens { sides, def } => {
                let roll = self.roll_die(sides);
                for _ in 0..roll {
                    self.create_token(p, def);
                }
            }
            SubsystemEffect::EmblemForTarget { slot, kind } => {
                if legal.get(slot as usize).copied().unwrap_or(false) {
                    if let Some(&Chosen::Player(q)) = item.targets.get(slot as usize) {
                        self.emblems.push((q, kind, item.source));
                    }
                }
            }
            // A bottommost room's ability leaving the stack completes the dungeon (CR 704.5t), once
            // the last copy of it has resolved.
            SubsystemEffect::CompleteDungeon => {
                if let StackKind::Room { dungeon, room } = item.kind {
                    let pending = self
                        .items
                        .iter()
                        .chain(self.triggered.iter())
                        .any(|t| t.controller == p && t.kind == item.kind);
                    if !pending {
                        self.complete_dungeon(p, dungeon, room);
                    }
                }
            }
            SubsystemEffect::DrawIfCompletedDungeon(n) => {
                if self.completed_a_dungeon(p) {
                    for _ in 0..n {
                        self.draw(p);
                    }
                }
            }
            SubsystemEffect::TokensByDungeon { def, n, completed } => {
                let k = if self.completed_a_dungeon(p) { completed } else { n };
                for _ in 0..k {
                    self.create_token(p, def);
                }
            }
            SubsystemEffect::UntapOneOrAllByDungeon => {
                if self.completed_a_dungeon(p) {
                    for c in self.creatures(p).collect::<Vec<_>>() {
                        self.untap_by_effect(c);
                    }
                } else {
                    // Untapping an untapped creature does nothing, so only tapped ones are offered.
                    match self.untap_own_creature_options(p)[..] {
                        [] => {}
                        [Action::Pick(c)] => self.untap_by_effect(c),
                        _ => self.ask(p, Decision::UntapOwnCreature),
                    }
                }
            }
            SubsystemEffect::FlipCoinLoseDamageYou(n) => {
                if !self.flip_coin() {
                    self.deal_damage(item.source, Target::Player(p), n);
                }
            }
            SubsystemEffect::OpponentLosesDifferenceFrom(n) => {
                let q = opponent(p);
                let (turn, lost) = self.life_lost_turn[q];
                let lost = if turn == self.turn { lost } else { 0 };
                if lost < n {
                    self.lose_life(q, n - lost);
                }
            }
            SubsystemEffect::GainChapterAbilities(n) => {
                let id = item.source;
                let o = self.obj(id);
                if o.zone == Zone::Battlefield {
                    let zcc = o.zcc;
                    self.chapter_grants
                        .retain(|&(x, z, _)| self.objects[x.0 as usize].zcc == z);
                    self.chapter_grants.push((id, zcc, n));
                }
            }
            SubsystemEffect::RingTempts => self.ring_tempts(p),
            SubsystemEffect::RingSacrificeBlocker => {
                if let Some(b) = item.extra {
                    let blocking = self.combat.as_ref().is_some_and(|c| c.is_blocking(b));
                    if blocking && self.obj(b).zone == Zone::Battlefield {
                        self.obj_mut(b).sacrifice_end_combat = true;
                    }
                }
            }
        }
    }

    /// A split card's mana cost now (CR 709.3b, 709.4): on the stack, only the half being cast;
    /// anywhere else (and for every other card), its `cost`. Mana value checks on spells read this.
    pub fn stack_mana_cost(&self, id: ObjId) -> ManaCost {
        let def = self.def_of(id);
        if def.subsystems.split_card && self.obj(id).zone == Zone::Stack {
            if let Some(half) = self.split_half_cost(id) {
                return half;
            }
        }
        def.cost
    }

    /// `stack_mana_cost` for a spell being cast as `item` (its stack entry may not be on the
    /// stack yet): a split card's cast half, by `item.mode` (CR 709.3b).
    pub(super) fn cast_mana_cost(&self, item: &StackItem) -> ManaCost {
        let def = self.def_of(item.source);
        if def.subsystems.split_card {
            if let Some(half) = self.split_half_cost_by_mode(item.source, item.mode) {
                return half;
            }
        }
        def.cost
    }

    /// A split card on the stack: the mana cost of the half being cast, or None if its stack
    /// entry isn't found.
    fn split_half_cost(&self, id: ObjId) -> Option<ManaCost> {
        let item = self
            .items
            .iter()
            .chain(self.resolving.as_ref().map(|r| &r.item))
            .chain(self.choosing.as_ref().map(|c| &c.item))
            .find(|i| i.source == id && matches!(i.kind, StackKind::Spell | StackKind::SpellCopy))?;
        self.split_half_cost_by_mode(id, item.mode)
    }

    /// A split card's half for a spell cast with `mode`: the right half's (an Omen-mode cast) or
    /// the left half's.
    fn split_half_cost_by_mode(&self, id: ObjId, mode: Option<u8>) -> Option<ManaCost> {
        let def = self.def_of(id);
        if mode == Some(OMEN_MODE) {
            Some(def.omen.as_ref()?.cost)
        } else {
            def.subsystems.left_cost
        }
    }

    // ---- The Ring (CR 701.54) ----

    /// `p`'s Ring-bearer: the creature last chosen, while it's still that object on the
    /// battlefield under `p`'s control (CR 701.54a, 701.54c).
    pub fn ring_bearer(&self, p: PlayerId) -> Option<ObjId> {
        let (c, zcc) = self.ring_bearer[p]?;
        let o = self.obj(c);
        (o.zone == Zone::Battlefield && o.zcc == zcc && o.controller == p && self.types_of(c).creature)
            .then_some(c)
    }

    /// Whether `id` is its controller's Ring-bearer.
    pub fn is_ring_bearer(&self, id: ObjId) -> bool {
        self.ring_bearer(self.obj(id).controller) == Some(id)
    }

    /// How many of the Ring's abilities `p`'s emblem has (0 if the Ring has never tempted them).
    pub fn ring_level(&self, p: PlayerId) -> u8 {
        self.ring_level[p]
    }

    /// Hook for the type pass (d3's `types_of`): the Ring's first ability, "Your Ring-bearer is
    /// legendary". Not called yet.
    pub fn ring_makes_legendary(&self, id: ObjId) -> bool {
        self.is_ring_bearer(id) && self.ring_level(self.obj(id).controller) >= 1
    }

    /// Hook for block legality (d1's `can_block`): the Ring's first ability, "[your Ring-bearer]
    /// can't be blocked by creatures with greater power". Not called yet.
    pub fn ring_evasion_forbids_block(&self, blocker: ObjId, attacker: ObjId) -> bool {
        self.ring_makes_legendary(attacker) && self.power(blocker) > self.power(attacker)
    }

    /// `Decision::ChooseRingBearer`'s answers: the creatures `p` controls.
    pub(super) fn ring_bearer_options(&self, p: PlayerId) -> Vec<Action> {
        self.creatures(p).map(Action::Pick).collect()
    }

    /// The Ring tempts `p` (CR 701.54a): they choose a creature they control as their Ring-bearer
    /// (asked only if there's more than one), and their emblem gains its next ability. Tempting
    /// with no creature still gains the ability.
    pub fn ring_tempts(&mut self, p: PlayerId) {
        self.ring_tempted[p] = self.ring_tempted[p].saturating_add(1);
        self.ring_level[p] = (self.ring_level[p] + 1).min(4);
        match self.ring_bearer_options(p)[..] {
            [] => {}
            [Action::Pick(c)] => self.choose_ring_bearer(p, c),
            _ => self.ask(p, Decision::ChooseRingBearer),
        }
        // Hook for d2: "Whenever the Ring tempts you" triggers fire here.
    }

    /// `c` becomes `p`'s Ring-bearer (another creature stops being it; CR 701.54a).
    fn choose_ring_bearer(&mut self, p: PlayerId, c: ObjId) {
        self.ring_bearer[p] = Some((c, self.obj(c).zcc));
        // Hook for d2: "Whenever you choose a creature as your Ring-bearer" (Call of the Ring)
        // triggers fire here, with `c` as the chosen creature.
    }

    /// Hook for combat damage (d2): the Ring's fourth ability, "Whenever your Ring-bearer deals
    /// combat damage to a player, each opponent loses 3 life", for attacker `a` that just dealt
    /// combat damage to a player. Not called yet.
    pub fn ring_bearer_dealt_combat_damage(&mut self, a: ObjId) {
        let p = self.obj(a).controller;
        if self.ring_bearer(p) == Some(a) && self.ring_level(p) >= 4 {
            self.push_ring_ability(p, a, ring::DRAIN, None);
        }
    }

    fn push_ring_ability(&mut self, p: PlayerId, bearer: ObjId, n: u8, extra: Option<ObjId>) {
        self.triggered.push(StackItem {
            kind: StackKind::Room {
                dungeon: Dungeon::TheRing,
                room: n,
            },
            extra,
            ..Game::casting(bearer, p)
        });
    }

    /// The Ring's triggered abilities on events `fire()` already has: the bearer attacking (on
    /// `YouAttack`, once attackers are declared) and becoming blocked (on `OpponentCreatureBlocks`,
    /// once blockers are; once per blocker).
    fn fire_ring(&mut self, p: PlayerId, when: When) {
        let Some(bearer) = self.ring_bearer(p) else {
            return;
        };
        let level = self.ring_level(p);
        match when {
            When::YouAttack if level >= 2 => {
                if self
                    .combat
                    .as_ref()
                    .is_some_and(|c| c.attackers.contains(&bearer))
                {
                    self.push_ring_ability(p, bearer, ring::LOOT, None);
                }
            }
            When::OpponentCreatureBlocks if level >= 3 => {
                let blockers: Vec<ObjId> = self
                    .combat
                    .as_ref()
                    .map(|c| c.blockers_of(bearer).collect())
                    .unwrap_or_default();
                let turn = self.turn;
                self.ring_blocks_seen.retain(|&(_, _, t)| t == turn);
                for b in blockers {
                    let zcc = self.obj(b).zcc;
                    if !self.ring_blocks_seen.contains(&(b, zcc, turn)) {
                        self.ring_blocks_seen.push((b, zcc, turn));
                        self.push_ring_ability(p, bearer, ring::SACRIFICE_BLOCKER, Some(b));
                    }
                }
            }
            _ => {}
        }
    }

    /// `Decision::UntapOwnCreature`'s answers: `p`'s tapped creatures.
    pub(super) fn untap_own_creature_options(&self, p: PlayerId) -> Vec<Action> {
        self.creatures(p)
            .filter(|&c| self.obj(c).tapped)
            .map(Action::Pick)
            .collect()
    }

    /// Whether the Saga `id` has its chapter-`n` grants now: that chapter's grant resolved while
    /// it's been on the battlefield (CR 714; Urza's Saga).
    pub fn has_chapter_grant(&self, id: ObjId, n: u8) -> bool {
        let o = self.obj(id);
        o.zone == Zone::Battlefield
            && self
                .chapter_grants
                .iter()
                .any(|&(x, zcc, c)| x == id && zcc == o.zcc && c == n)
    }

    /// Whether activated ability `n` of `id` is a chapter grant it doesn't have yet
    /// (`SubsystemExt::chapter_activated`): the activation check skips it.
    pub fn lacks_chapter_activated(&self, id: ObjId, n: usize) -> bool {
        self.def_of(id)
            .subsystems
            .chapter_activated
            .iter()
            .any(|&(c, i)| i == n && !self.has_chapter_grant(id, c))
    }

    /// Whether `id`'s `mana` ability is a chapter grant it doesn't have yet
    /// (`SubsystemExt::chapter_mana`): it isn't a mana source until then.
    pub fn lacks_chapter_mana(&self, id: ObjId) -> bool {
        self.def_of(id)
            .subsystems
            .chapter_mana
            .is_some_and(|c| !self.has_chapter_grant(id, c))
    }
}
