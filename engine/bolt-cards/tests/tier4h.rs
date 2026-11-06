//! Tier 4, batch 10: Thousand-Year Storm and copies of spells (CR 707.10).

mod common;

use bolt_core::{Action, Target, Zone};
use common::*;

fn bolt_face(g: &mut bolt_core::Game, p: usize, at: usize) {
    let b = hand(g, p, "Burst Lightning");
    play(g, b, false, &[Target::Player(at)], &[]);
}

#[test]
fn thousand_year_storm_copies_for_each_earlier_instant_or_sorcery() {
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Thousand-Year Storm");
    bolt_face(&mut g, 0, 1); // first: no copies
    assert_eq!(g.players[1].life, 18);
    bolt_face(&mut g, 0, 1); // second: one copy
    assert_eq!(g.players[1].life, 14);
    bolt_face(&mut g, 0, 1); // third: two copies
    assert_eq!(g.players[1].life, 8);
}

#[test]
fn copies_cease_to_exist_instead_of_going_to_the_graveyard() {
    // CR 707.10a, 704.5e.
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Thousand-Year Storm");
    bolt_face(&mut g, 0, 1);
    bolt_face(&mut g, 0, 1);
    assert_eq!(g.players[0].graveyard.len(), 2);
    assert!(g.items.is_empty());
}

#[test]
fn spells_cast_before_the_storm_still_count() {
    // "each other instant and sorcery spell you've cast before it this turn", Storm or not.
    let mut g = table();
    begin(&mut g);
    bolt_face(&mut g, 0, 1);
    put(&mut g, 0, "Thousand-Year Storm");
    bolt_face(&mut g, 0, 1); // one copy
    assert_eq!(g.players[1].life, 14);
}

#[test]
fn copies_keep_the_originals_targets_and_kicker() {
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Thousand-Year Storm");
    bolt_face(&mut g, 0, 1);
    let turtle = put(&mut g, 1, "Aegis Turtle"); // 0/5
    let b = hand(&mut g, 0, "Burst Lightning");
    play(&mut g, b, true, &[obj(turtle)], &[]); // kicked: 4, plus a kicked copy
    assert_eq!(g.obj(turtle).zone, Zone::Graveyard);
    assert_eq!(g.players[1].life, 18);
}

#[test]
fn opponents_spells_dont_count_or_copy() {
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Thousand-Year Storm");
    bolt_face(&mut g, 0, 1);
    act(&mut g, Action::PassPriority);
    let b = hand(&mut g, 1, "Burst Lightning");
    act(&mut g, Action::Cast(b));
    act(&mut g, Action::Target(Target::Player(0)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 18, "no copy for the opponent");
}
