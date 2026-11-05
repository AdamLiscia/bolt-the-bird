//! Shared helpers for card tests: a table with lands, casting, and resolving with scripted answers.
#![allow(dead_code)]

use bolt_cards::{fdn, index_of};
use bolt_core::{Action, Event, Game, ObjId, Pending, Step, Target, Zone};

pub fn id(name: &str) -> usize {
    index_of(name).unwrap_or_else(|| panic!("{name}"))
}

/// Player 0's first main phase on turn 3. Each player has 5 of each basic land, and libraries of
/// Aegis Turtles. Every priority stop is visible (no auto-pass).
pub fn table() -> Game {
    let mut g = Game::empty(fdn(), 1, 0);
    g.auto_pass = false;
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, id("Aegis Turtle"), Zone::Library);
        }
        for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            for _ in 0..5 {
                g.add_card(p, id(land), Zone::Battlefield);
            }
        }
    }
    g
}

pub fn begin(g: &mut Game) {
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(g, Action::PassPriority);
    }
}

pub fn act(g: &mut Game, a: Action) -> Vec<Event> {
    g.apply(a)
        .unwrap_or_else(|e| panic!("{e}; legal: {:?}", g.legal_actions()))
}

pub fn put(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, id(name), Zone::Battlefield)
}

pub fn hand(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, id(name), Zone::Hand)
}

/// Casts a card with the given targets, then passes until the stack is empty. Targets for
/// triggers that come up are taken from `then`.
pub fn cast(g: &mut Game, card: ObjId, targets: &[Target], then: &[Target]) -> Vec<Event> {
    let mut ev = act(g, Action::Cast(card));
    for &t in targets {
        ev.extend(act(g, Action::Target(t)));
    }
    let mut then = then.iter();
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_)) {
        if let Pending::ChooseTarget(_) = g.pending {
            ev.extend(act(
                g,
                Action::Target(*then.next().expect("a target for the trigger")),
            ));
        } else {
            ev.extend(act(g, Action::PassPriority));
        }
    }
    ev
}

pub fn obj(id: ObjId) -> Target {
    Target::Object(id)
}

pub fn tokens_named(g: &Game, p: usize, name: &str) -> usize {
    g.creatures(p)
        .filter(|&c| g.def_of(c).token && g.def_of(c).name == name)
        .count()
}

/// Casts `card` (kicked or not) with `targets`, then passes until the stack is empty, answering
/// trigger targets and resolution decisions from `answers` in order.
pub fn play(g: &mut Game, card: ObjId, kicked: bool, targets: &[Target], answers: &[Action]) {
    act(
        g,
        if kicked {
            Action::CastKicked(card)
        } else {
            Action::Cast(card)
        },
    );
    for &t in targets {
        act(g, Action::Target(t));
    }
    resolve_all(g, answers);
}

pub fn resolve_all(g: &mut Game, answers: &[Action]) {
    let mut answers = answers.iter();
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) | Pending::Decide(_) => *answers
                .next()
                .unwrap_or_else(|| panic!("no answer for {:?}", g.legal_actions())),
            _ => Action::PassPriority,
        };
        act(g, a);
    }
    assert!(answers.next().is_none(), "unused answers");
}

/// Adds a card to the bottom of a library, so the draw step doesn't draw it.
pub fn library_card(g: &mut Game, p: usize, name: &str) -> ObjId {
    let c = g.add_card(p, id(name), Zone::Library);
    g.players[p].library.rotate_right(1);
    c
}

pub fn token_of(g: &Game, p: usize, name: &str) -> ObjId {
    g.permanents(p)
        .find(|&c| g.def_of(c).token && g.def_of(c).name == name)
        .expect("the token")
}

/// A table where player 0 has exactly the lands named.
pub fn lands(names: &[&str]) -> Game {
    let mut g = Game::empty(fdn(), 1, 0);
    g.auto_pass = false;
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, id("Aegis Turtle"), Zone::Library);
        }
    }
    for n in names {
        put(&mut g, 0, n);
    }
    g
}
