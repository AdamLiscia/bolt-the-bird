//! The merger's own cards (scripts/pg_m.rs): attack restrictions (Ghostly Prison, Sphere of Safety, Promise
//! of Loyalty).
#![allow(dead_code, unused_imports)]

mod common;

use bolt_cards::set;
use bolt_core::{Action, Game, Keywords, ObjId, Pending, Step, Target, Zone};
use common::{act, obj, play, resolve_all};

fn cid(name: &str) -> usize {
    set("CMD")
        .unwrap()
        .index_of(name)
        .unwrap_or_else(|| panic!("{name} is not in the CMD pool"))
}

/// A Commander game on the CMD pool (99 Plains each), Bello vs. Ms. Bumbleflower, at player 0's first
/// main phase on turn 3, with player 0's `lands` and 5 of each basic for player 1.
fn game(lands: &[&str]) -> Game {
    game_with(lands, |_| {})
}

fn game_with(lands: &[&str], setup: impl FnOnce(&mut Game)) -> Game {
    let plains = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        set("CMD").unwrap().defs.clone(),
        [&plains, &plains],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    for &l in lands {
        put(&mut g, 0, l);
    }
    for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        for _ in 0..5 {
            put(&mut g, 1, land);
        }
    }
    setup(&mut g);
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g
}

/// Every basic five times for player 0.
fn table() -> Game {
    let mut lands = Vec::new();
    for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        lands.extend([land; 5]);
    }
    game(&lands)
}

fn put(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, cid(name), Zone::Battlefield)
}

fn hand(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, cid(name), Zone::Hand)
}

fn tokens(g: &Game, p: usize, name: &str) -> Vec<ObjId> {
    g.permanents(p)
        .filter(|&x| g.is_token(x) && g.def_of(x).name == name)
        .collect()
}

/// Passes toward player `p`'s declare-attackers step: true if it comes (some creature may attack), false
/// if the engine skips it (none can) and the turn reaches its second main phase.
fn reaches_attackers(g: &mut Game, p: usize) -> bool {
    loop {
        if g.pending == Pending::DeclareAttackers(p) {
            return true;
        }
        if g.step == Step::Main2 || g.step == Step::End {
            return false;
        }
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::Decide(_) => Action::No,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn ghostly_prison_taxes_each_attacker() {
    // No mana: the attacker can't be declared.
    let mut lions = ObjId(0);
    let mut g = game_with(&[], |g| {
        put(g, 1, "Ghostly Prison");
        lions = put(g, 0, "Savannah Lions");
    });
    assert!(!reaches_attackers(&mut g, 0), "can't pay {{2}}: no attack step");
    let _ = lions;
    // Two Plains: it attacks, and both are tapped to pay.
    let mut g = game_with(&["Plains", "Plains"], |g| {
        put(g, 1, "Ghostly Prison");
        lions = put(g, 0, "Savannah Lions");
    });
    assert!(reaches_attackers(&mut g, 0));
    act(&mut g, Action::Attack(lions));
    let untapped = g
        .permanents(0)
        .filter(|&x| g.def_of(x).name == "Plains" && !g.obj(x).tapped)
        .count();
    assert_eq!(untapped, 0, "the tax was paid");
}

#[test]
fn sphere_of_safety_taxes_per_enchantment() {
    let mut lions = ObjId(0);
    let mut g = game_with(&["Plains", "Plains"], |g| {
        put(g, 1, "Sphere of Safety");
        put(g, 1, "Ghostly Prison"); // a second enchantment: X = 2, and Ghostly Prison's own {2}
        lions = put(g, 0, "Savannah Lions");
    });
    assert_eq!(g.attack_tax(1), 4);
    assert!(
        !reaches_attackers(&mut g, 0),
        "{{4}} with two lands: no attack step"
    );
    let _ = lions;
}

#[test]
fn promise_of_loyalty_keeps_one_each_and_the_vowed_cant_attack_you() {
    let mut mine = [ObjId(0); 2];
    let mut theirs = [ObjId(0); 2];
    let mut lands = Vec::new();
    for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        lands.extend([land; 5]);
    }
    let mut g = game_with(&lands, |g| {
        mine = [put(g, 0, "Savannah Lions"), put(g, 0, "Savannah Lions")];
        theirs = [put(g, 1, "Savannah Lions"), put(g, 1, "Savannah Lions")];
    });
    let c = hand(&mut g, 0, "Promise of Loyalty");
    // You vow first, then the opponent; each keeps its first creature.
    play(
        &mut g,
        c,
        false,
        &[],
        &[Action::Pick(mine[0]), Action::Pick(theirs[0])],
    );
    let kept_mine = g.permanents(0).filter(|&x| g.types_of(x).creature).count();
    let kept: Vec<ObjId> = theirs
        .iter()
        .copied()
        .filter(|&x| g.obj(x).zone == Zone::Battlefield)
        .collect();
    assert_eq!((kept_mine, kept.len()), (1, 1), "one creature each");
    assert!(g.vowed_against(kept[0], 0));
    g.start_turn(4, 1);
    assert!(
        !reaches_attackers(&mut g, 1),
        "its only creature is vowed: it can't attack the caster"
    );
}
