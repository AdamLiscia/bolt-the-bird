//! Shared helpers for building test positions.
#![allow(dead_code)]

use std::sync::Arc;

use bolt_core::{Action, CardDef, Color, Event, Game, Keywords, ObjId, Pending, Zone};

pub const PLAINS: usize = 0;
pub const MOUNTAIN: usize = 1;
pub const BEAR: usize = 2; // 2/2
pub const OGRE: usize = 3; // 5/5
pub const BIRD: usize = 4; // 1/1 flying
pub const SPIDER: usize = 5; // 1/3 reach
pub const KNIGHT: usize = 6; // 2/2 first strike
pub const DOUBLER: usize = 7; // 2/2 double strike
pub const ASP: usize = 8; // 1/1 deathtouch
pub const TRAMPLER: usize = 9; // 5/5 trample
pub const LINKER: usize = 10; // 2/2 lifelink
pub const WATCHER: usize = 11; // 2/2 vigilance
pub const RUNNER: usize = 12; // 2/2 haste
pub const BRUTE: usize = 13; // 3/3 menace
pub const WALL: usize = 14; // 0/4 defender
pub const GOLEM: usize = 15; // 3/3 indestructible
pub const LEGEND: usize = 16; // legendary 2/2
pub const STING: usize = 17; // 4/4 deathtouch, trample
pub const AMBUSHER: usize = 18; // 2/1 flash {1}{W}

pub fn defs() -> Arc<Vec<CardDef>> {
    let k = Keywords::NONE;
    let mut legend = CardDef::creature("Legend", "{1}{W}", 2, 2, k);
    legend.types.legendary = true;
    Arc::new(vec![
        CardDef::basic_land("Plains", Color::White),
        CardDef::basic_land("Mountain", Color::Red),
        CardDef::creature("Bear", "{1}{W}", 2, 2, k),
        CardDef::creature("Ogre", "{4}{W}", 5, 5, k),
        CardDef::creature("Bird", "{W}", 1, 1, Keywords::FLYING),
        CardDef::creature("Spider", "{1}{W}", 1, 3, Keywords::REACH),
        CardDef::creature("Knight", "{1}{W}", 2, 2, Keywords::FIRST_STRIKE),
        CardDef::creature("Doubler", "{2}{W}", 2, 2, Keywords::DOUBLE_STRIKE),
        CardDef::creature("Asp", "{W}", 1, 1, Keywords::DEATHTOUCH),
        CardDef::creature("Trampler", "{4}{W}", 5, 5, Keywords::TRAMPLE),
        CardDef::creature("Linker", "{1}{W}", 2, 2, Keywords::LIFELINK),
        CardDef::creature("Watcher", "{1}{W}", 2, 2, Keywords::VIGILANCE),
        CardDef::creature("Runner", "{1}{R}", 2, 2, Keywords::HASTE),
        CardDef::creature("Brute", "{2}{R}", 3, 3, Keywords::MENACE),
        CardDef::creature("Wall", "{1}{W}", 0, 4, Keywords::DEFENDER),
        CardDef::creature("Golem", "{3}", 3, 3, Keywords::INDESTRUCTIBLE),
        legend,
        CardDef::creature(
            "Sting",
            "{3}{R}",
            4,
            4,
            Keywords::DEATHTOUCH.with(Keywords::TRAMPLE),
        ),
        CardDef::creature("Ambusher", "{1}{W}", 2, 1, Keywords::FLASH),
    ])
}

/// An empty board with 10 uncastable Ogres in each library (there are no lands), so nobody decks out
/// and nobody has a choice to make until combat.
pub fn board() -> Game {
    let mut g = Game::empty(defs(), 1, 0);
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, OGRE, Zone::Library);
        }
    }
    g
}

pub fn put(g: &mut Game, p: usize, def: usize) -> ObjId {
    g.add_card(p, def, Zone::Battlefield)
}

/// Starts player 0's turn 3 and returns once attackers are being declared.
pub fn to_attacks(g: &mut Game) {
    g.start_turn(3, 0);
    assert_eq!(
        g.pending,
        Pending::DeclareAttackers(0),
        "expected to declare attackers"
    );
}

pub fn act(g: &mut Game, a: Action) -> Vec<Event> {
    g.apply(a)
        .unwrap_or_else(|e| panic!("{e}; legal: {:?}", g.legal_actions()))
}

/// Attacks with `attackers`, then blocks with `blocks` (blocker, attacker), then plays on until the
/// next decision. With nothing else to do, that is usually a later turn, so check results through
/// the returned events or through zones and life totals.
pub fn fight(g: &mut Game, attackers: &[ObjId], blocks: &[(ObjId, ObjId)]) -> Vec<Event> {
    to_attacks(g);
    let mut ev = Vec::new();
    for &a in attackers {
        ev.extend(act(g, Action::Attack(a)));
    }
    ev.extend(act(g, Action::FinishAttacks));
    if let Pending::DeclareBlockers(_) = g.pending {
        for &(blocker, attacker) in blocks {
            ev.extend(act(g, Action::Block { blocker, attacker }));
        }
        ev.extend(act(g, Action::FinishBlocks));
    } else {
        assert!(blocks.is_empty(), "no blocks were possible");
    }
    ev
}

/// Total combat damage dealt to `id` in `events`.
pub fn damage_to(events: &[Event], id: ObjId) -> i32 {
    events
        .iter()
        .map(|e| match e {
            Event::Damage {
                target: bolt_core::Target::Object(t),
                amount,
                ..
            } if *t == id => *amount,
            _ => 0,
        })
        .sum()
}

pub fn alive(g: &Game, id: ObjId) -> bool {
    g.obj(id).zone == Zone::Battlefield
}
