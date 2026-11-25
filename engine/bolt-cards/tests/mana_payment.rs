//! Paying mana well: the smart auto-payer keeps the colors your hand still needs, and manual mana lets
//! a player tap their own sources into the pool (Bolt service games; 2026-11-24, Adam kept an Island
//! up for a counterspell and the old payer spent it).

mod common;

use bolt_core::{Action, Game, Pending, Step, Zone};
use common::*;

/// Player 0's main phase with exactly these lands, and these cards in hand.
fn main_with(lands_named: &[&str], hand_cards: &[&str]) -> (Game, Vec<bolt_core::ObjId>) {
    let mut g = lands(lands_named);
    let ids = hand_cards.iter().map(|n| hand(&mut g, 0, n)).collect();
    begin(&mut g);
    assert_eq!(g.step, Step::Main1);
    (g, ids)
}

fn untapped(g: &Game, name: &str) -> usize {
    g.permanents(0)
        .filter(|&x| g.def_of(x).name == name && !g.obj(x).tapped)
        .count()
}

#[test]
fn the_smart_payer_keeps_an_island_up_for_the_blue_card_in_hand() {
    // Island first, so the old payer (narrowest, then oldest) spends it on Gnarlid Colony's {1}.
    let (mut g, ids) = main_with(
        &["Island", "Forest", "Forest"],
        &["Gnarlid Colony", "Essence Scatter"],
    );
    g.smart_pay = true;
    act(&mut g, Action::Cast(ids[0]));
    assert_eq!(
        untapped(&g, "Island"),
        1,
        "the Island stays up for Essence Scatter"
    );
    assert_eq!(untapped(&g, "Forest"), 0);
}

#[test]
fn research_games_keep_the_old_payer() {
    let (mut g, ids) = main_with(
        &["Island", "Forest", "Forest"],
        &["Gnarlid Colony", "Essence Scatter"],
    );
    assert!(
        !g.smart_pay,
        "off by default, so stored results replay identically"
    );
    act(&mut g, Action::Cast(ids[0]));
    assert_eq!(untapped(&g, "Island"), 0, "the old payer's choice, unchanged");
}

#[test]
fn manual_mana_taps_into_the_pool_and_casting_spends_it_first() {
    let (mut g, ids) = main_with(&["Island", "Forest", "Forest"], &["Llanowar Elves"]);
    assert!(
        !g.legal_actions()
            .iter()
            .any(|a| matches!(a, Action::TapForMana(..))),
        "off: no tap moves"
    );
    g.manual_mana[0] = true;
    let moves: Vec<Action> = g
        .legal_actions()
        .into_iter()
        .filter(|a| matches!(a, Action::TapForMana(..)))
        .collect();
    assert_eq!(moves.len(), 3, "one move per land and color: {moves:?}");
    let forest = g.permanents(0).find(|&x| g.def_of(x).name == "Forest").unwrap();
    act(&mut g, Action::TapForMana(forest, 4)); // {G}
    assert!(g.obj(forest).tapped);
    assert_eq!(g.players[0].pool.len(), 1);
    assert_eq!(g.pending, Pending::Priority(0), "a mana ability keeps priority");
    act(&mut g, Action::Cast(ids[0]));
    assert!(g.players[0].pool.is_empty(), "the floating {{G}} paid for it");
    assert_eq!(untapped(&g, "Forest"), 1, "no other land tapped");
    assert_eq!(untapped(&g, "Island"), 1);
}

#[test]
fn a_dual_land_offers_both_colors() {
    let (mut g, _) = main_with(&["Bloodfell Caves"], &[]);
    g.manual_mana[0] = true;
    let colors: Vec<u8> = g
        .legal_actions()
        .into_iter()
        .filter_map(|a| match a {
            Action::TapForMana(_, c) => Some(c),
            _ => None,
        })
        .collect();
    assert_eq!(colors, vec![2, 3], "{{B}} and {{R}}");
}

#[test]
fn floating_mana_empties_at_the_end_of_the_step() {
    let (mut g, _) = main_with(&["Forest", "Forest"], &[]);
    g.manual_mana[0] = true;
    let forest = g.permanents(0).find(|&x| g.def_of(x).name == "Forest").unwrap();
    act(&mut g, Action::TapForMana(forest, 4));
    assert_eq!(g.players[0].pool.len(), 1);
    while g.step == Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert!(g.players[0].pool.is_empty(), "CR 500.4");
}

#[test]
fn manual_casting_still_auto_pays_the_rest() {
    let (mut g, ids) = main_with(&["Forest", "Forest"], &["Gnarlid Colony"]);
    g.manual_mana[0] = true;
    assert!(
        g.legal_actions().contains(&Action::Cast(ids[0])),
        "castable from untapped lands alone"
    );
    act(&mut g, Action::Cast(ids[0]));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ids[0]).zone, Zone::Battlefield);
    assert_eq!(untapped(&g, "Forest"), 0);
}

#[test]
fn tap_moves_alone_dont_stop_auto_pass() {
    let mut g = Game::empty(bolt_cards::fdn(), 1, 0);
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, id("Aegis Turtle"), Zone::Library);
        }
    }
    put(&mut g, 0, "Forest");
    g.manual_mana[0] = true;
    g.start_turn(3, 0);
    // With auto-pass on, priority stops where player 0 could only pass (or tap a land for nothing)
    // are skipped, as before.
    let mut stops = 0;
    while g.turn == 3 && stops < 50 {
        let real = g
            .legal_actions()
            .iter()
            .filter(|a| !matches!(a, Action::TapForMana(..)))
            .count();
        assert!(
            real > 1 || !matches!(g.pending, Pending::Priority(_)),
            "stopped with nothing to do"
        );
        let first = g.legal_actions()[0];
        act(&mut g, first);
        stops += 1;
    }
}
