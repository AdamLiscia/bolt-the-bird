//! Shared helpers for Tarkir card tests (tests/tdm*.rs): `mod tdm_common;` then `use tdm_common::*;`.
#![allow(dead_code)]

use bolt_cards::set;
use bolt_core::{Action, Game, ObjId, Pending, Zone};

pub fn tid(name: &str) -> usize {
    set("TDM")
        .unwrap()
        .index_of(name)
        .unwrap_or_else(|| panic!("{name} is not in TDM"))
}

/// Player 0's first main phase on turn 3 (after `begin`) with Tarkir cards: 5 of each basic land each,
/// libraries of Plains. Every priority stop is visible.
pub fn table() -> Game {
    let mut g = Game::empty(set("TDM").unwrap().defs.clone(), 1, 0);
    g.auto_pass = false;
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, tid("Plains"), Zone::Library);
        }
        for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            for _ in 0..5 {
                g.add_card(p, tid(land), Zone::Battlefield);
            }
        }
    }
    g
}

/// A table where player 0 has exactly these lands.
pub fn tdm_lands(names: &[&str]) -> Game {
    let mut g = Game::empty(set("TDM").unwrap().defs.clone(), 1, 0);
    g.auto_pass = false;
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, tid("Plains"), Zone::Library);
        }
    }
    for n in names {
        put(&mut g, 0, n);
    }
    g
}

pub fn put(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, tid(name), Zone::Battlefield)
}

pub fn hand(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, tid(name), Zone::Hand)
}

pub fn tokens(g: &Game, p: usize, name: &str) -> Vec<ObjId> {
    g.permanents(p)
        .filter(|&x| g.def_of(x).token && g.def_of(x).name == name)
        .collect()
}

pub fn untapped_lands(g: &Game, p: usize) -> usize {
    g.permanents(p)
        .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
        .count()
}

/// Lands player 0 tapped while `f` ran.
pub fn paid(g: &mut Game, f: impl FnOnce(&mut Game)) -> usize {
    let before = untapped_lands(g, 0);
    f(g);
    before - untapped_lands(g, 0)
}

/// Attack with `attacker` on turn 5 (not summoning sick); returns after attack triggers resolve.
pub fn attack_with(g: &mut Game, attacker: ObjId) {
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        g.apply(Action::PassPriority).unwrap();
    }
    g.apply(Action::Attack(attacker)).unwrap();
    g.apply(Action::FinishAttacks).unwrap();
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_)) {
        if matches!(g.pending, Pending::ChooseTarget(_)) {
            break;
        }
        g.apply(Action::PassPriority).unwrap();
    }
}
