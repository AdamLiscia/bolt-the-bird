//! Game start, turn structure, casting and state-based actions (CR 103, 117, 302, 305, 500-514, 704).

mod common;

use std::sync::Arc;

use bolt_core::{Action, Event, Game, Outcome, Pending, RulesError, Step, Zone};
use common::*;

fn deck() -> Vec<usize> {
    let mut d = vec![PLAINS; 17];
    d.extend([BEAR; 23]);
    d
}

/// A test board that stops at every priority, so intermediate states can be checked.
fn manual() -> Game {
    let mut g = board();
    g.auto_pass = false;
    g
}

fn pass_until(g: &mut Game, done: impl Fn(&Game) -> bool) {
    for _ in 0..200 {
        if done(g) {
            return;
        }
        act(g, Action::PassPriority);
    }
    panic!("condition never reached");
}

fn new_game(seed: u64) -> Game {
    let d = deck();
    Game::new(defs(), [&d, &d], seed, 0)
}

#[test]
fn opening_hands_and_libraries() {
    // CR 103.4-103.5.
    let g = new_game(1);
    for p in &g.players {
        assert_eq!((p.hand.len(), p.library.len(), p.life), (7, 33, 20));
    }
    assert_eq!(g.pending, Pending::Mulligan(0));
    g.check_invariants().unwrap();
}

#[test]
fn same_seed_same_game() {
    let (a, b) = (new_game(9), new_game(9));
    assert_eq!(a.players, b.players);
    assert_ne!(new_game(9).players[0].hand, new_game(10).players[0].hand);
}

#[test]
fn london_mulligan_draws_seven_and_bottoms_one() {
    // CR 103.5: shuffle, draw seven, then put one card on the bottom per mulligan.
    let mut g = new_game(2);
    act(&mut g, Action::Mulligan);
    assert_eq!(g.players[0].hand.len(), 7);
    act(&mut g, Action::Keep);
    assert_eq!(g.pending, Pending::Bottom(0, 1));
    let card = g.players[0].hand[0];
    act(&mut g, Action::Bottom(card));
    assert_eq!(g.players[0].hand.len(), 6);
    assert_eq!(g.players[0].library[0], card, "the bottom of the library");
    assert_eq!(g.pending, Pending::Mulligan(1));
    g.check_invariants().unwrap();
}

#[test]
fn the_player_on_the_play_skips_their_first_draw() {
    // CR 103.8a.
    let mut g = new_game(3);
    act(&mut g, Action::Keep);
    act(&mut g, Action::Keep);
    assert_eq!((g.turn, g.active), (1, 0));
    assert_eq!(g.players[0].hand.len(), 7);
    g.auto_pass = false;
    pass_until(&mut g, |g| g.turn == 2 && g.step == Step::Main1);
    assert_eq!(g.players[1].hand.len(), 8, "player 1 draws on turn 2");
}

#[test]
fn one_land_per_turn_at_sorcery_speed() {
    // CR 305.2, 505.6b.
    let mut g = manual();
    let l1 = g.add_card(0, PLAINS, Zone::Hand);
    let l2 = g.add_card(0, PLAINS, Zone::Hand);
    g.start_turn(3, 0);
    assert!(
        !g.legal_actions().contains(&Action::PlayLand(l1)),
        "not in upkeep"
    );
    pass_until(&mut g, |g| g.step == Step::Main1);
    act(&mut g, Action::PlayLand(l1));
    assert!(!g.legal_actions().contains(&Action::PlayLand(l2)));
    assert_eq!(
        g.apply(Action::PlayLand(l2)),
        Err(RulesError::Illegal(Action::PlayLand(l2)))
    );
}

#[test]
fn illegal_actions_change_nothing() {
    let mut g = manual();
    let bear = g.add_card(0, BEAR, Zone::Hand);
    g.add_card(0, PLAINS, Zone::Hand);
    g.start_turn(3, 0);
    let before = format!("{g:?}");
    assert!(g.apply(Action::Cast(bear)).is_err(), "no untapped lands yet");
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn casting_a_creature_taps_lands_and_it_resolves() {
    // CR 601.2, 608.3.
    let mut g = manual();
    let p1 = put(&mut g, 0, PLAINS);
    let p2 = put(&mut g, 0, PLAINS);
    let bear = g.add_card(0, BEAR, Zone::Hand);
    g.start_turn(3, 0);
    pass_until(&mut g, |g| g.step == Step::Main1);
    let ev = act(&mut g, Action::Cast(bear));
    assert!(ev.contains(&Event::Cast { card: bear }));
    assert!(g.obj(p1).tapped && g.obj(p2).tapped);
    assert_eq!(g.obj(bear).zone, Zone::Stack);
    assert_eq!(
        g.pending,
        Pending::Priority(0),
        "CR 117.3c: the caster gets priority again"
    );
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert_eq!(g.obj(bear).zone, Zone::Battlefield);
    assert_eq!(g.obj(bear).since_turn, 3, "summoning sick");
}

#[test]
fn creatures_without_flash_wait_for_a_main_phase() {
    // CR 307.1 (sorcery timing) and 702.8a (flash): on the opponent's turn, only flash.
    let mut g = manual();
    put(&mut g, 1, PLAINS);
    put(&mut g, 1, PLAINS);
    let bear = g.add_card(1, BEAR, Zone::Hand);
    let amb = g.add_card(1, AMBUSHER, Zone::Hand);
    g.start_turn(3, 0);
    act(&mut g, Action::PassPriority);
    assert_eq!((g.step, g.pending), (Step::Upkeep, Pending::Priority(1)));
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Cast(amb)));
    assert!(!legal.contains(&Action::Cast(bear)));
}

#[test]
fn cleanup_discards_down_to_seven() {
    // CR 514.1.
    let mut g = manual();
    for _ in 0..9 {
        g.add_card(0, OGRE, Zone::Hand);
    }
    g.start_turn(3, 0);
    pass_until(&mut g, |g| matches!(g.pending, Pending::Discard(_)));
    assert_eq!(g.players[0].hand.len(), 10, "nine plus the draw");
    while let Pending::Discard(_) = g.pending {
        let a = g.legal_actions()[0];
        act(&mut g, a);
    }
    assert_eq!(g.players[0].hand.len(), 7);
    assert_eq!(g.players[0].graveyard.len(), 3);
}

#[test]
fn drawing_from_an_empty_library_loses() {
    // CR 704.5b.
    let mut g = Game::empty(defs(), 1, 0);
    g.add_card(1, OGRE, Zone::Library);
    g.start_turn(3, 0);
    assert_eq!(g.result, Some(Outcome::Winner(1)));
    assert_eq!(g.pending, Pending::GameOver);
    assert_eq!(g.apply(Action::PassPriority), Err(RulesError::GameOver));
}

#[test]
fn zero_life_loses() {
    // CR 704.5a.
    let mut g = board();
    g.players[1].life = 2;
    let bear = put(&mut g, 0, BEAR);
    fight(&mut g, &[bear], &[]);
    assert_eq!(g.result, Some(Outcome::Winner(0)));
}

#[test]
fn legend_rule_keeps_one() {
    // CR 704.5j.
    let mut g = manual();
    put(&mut g, 0, PLAINS);
    put(&mut g, 0, PLAINS);
    let first = put(&mut g, 0, LEGEND);
    let second = g.add_card(0, LEGEND, Zone::Hand);
    g.start_turn(3, 0);
    pass_until(&mut g, |g| g.step == Step::Main1);
    act(&mut g, Action::Cast(second));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert_eq!(g.obj(first).zone, Zone::Graveyard);
    assert_eq!(g.obj(second).zone, Zone::Battlefield);
}

#[test]
fn defs_are_shared_not_copied() {
    let g = new_game(1);
    let h = g.clone();
    assert!(Arc::ptr_eq(&g.defs, &h.defs));
}

#[test]
fn permanents_untap_in_the_untap_step() {
    // CR 502.3. A regression test: a bad edit once left this branch commented out, and nothing else
    // in the suite noticed.
    let mut g = manual();
    let land = put(&mut g, 0, PLAINS);
    let bear = put(&mut g, 0, BEAR);
    let theirs = put(&mut g, 1, BEAR);
    for id in [land, bear, theirs] {
        g.objects[id.0 as usize].tapped = true;
    }
    g.start_turn(3, 0);
    assert!(
        !g.obj(land).tapped && !g.obj(bear).tapped,
        "the active player's permanents untap"
    );
    assert!(g.obj(theirs).tapped, "the other player's don't");
}
