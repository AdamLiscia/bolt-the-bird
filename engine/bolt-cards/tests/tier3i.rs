//! Tier-3 cards, batch 9b: delayed returns, ending the turn, ability loss, chosen colors, playing
//! from exile, and piles.

mod common;

use bolt_core::{Action, Game, Keywords, ObjId, Pending, Step, Zone};
use common::*;

fn attack_with(g: &mut Game, attackers: &[ObjId]) {
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    for &a in attackers {
        act(g, Action::Attack(a));
    }
    act(g, Action::FinishAttacks);
}

fn pass_to(g: &mut Game, step: Step) {
    while g.step != step {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn kykar_flickers_until_the_end_step() {
    let mut g = table();
    put(&mut g, 0, "Kykar, Zephyr Awakener");
    let lions = put(&mut g, 0, "Savannah Lions");
    let c = hand(&mut g, 0, "Boltwave");
    begin(&mut g);
    play(
        &mut g,
        c,
        false,
        &[],
        &[Action::Mode(0), Action::Target(obj(lions))],
    );
    assert_eq!(g.obj(lions).zone, Zone::Exile);
    pass_to(&mut g, Step::End);
    assert_eq!(
        g.obj(lions).zone,
        Zone::Battlefield,
        "back at the beginning of the end step"
    );

    let mut g = table();
    put(&mut g, 0, "Kykar, Zephyr Awakener");
    let c = hand(&mut g, 0, "Boltwave");
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Mode(1)]);
    assert_eq!(tokens_named(&g, 0, "Spirit"), 1);
}

#[test]
fn nine_lives_familiar_keeps_coming_back() {
    let mut g = table();
    let cat = hand(&mut g, 0, "Nine-Lives Familiar");
    let stab = hand(&mut g, 0, "Stab");
    begin(&mut g);
    play(&mut g, cat, false, &[], &[]);
    assert_eq!(g.obj(cat).revival, 8);
    play(&mut g, stab, false, &[obj(cat)], &[]);
    assert_eq!(g.obj(cat).zone, Zone::Graveyard);
    pass_to(&mut g, Step::End);
    assert_eq!(g.obj(cat).zone, Zone::Battlefield);
    assert_eq!(g.obj(cat).revival, 7);
}

#[test]
fn time_stop_ends_the_turn() {
    // CR 724: the stack is exiled and the turn goes to cleanup.
    let mut g = table();
    let lions = put(&mut g, 0, "Savannah Lions");
    let ts = hand(&mut g, 1, "Time Stop");
    begin(&mut g);
    attack_with(&mut g, &[lions]);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(ts));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert_eq!(g.players[1].life, 20, "no combat damage");
    assert_eq!(g.active, 1, "the next turn has begun");
}

#[test]
fn witness_protection_makes_a_vanilla_one_one() {
    let mut g = table();
    let aura = hand(&mut g, 0, "Witness Protection");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    play(&mut g, aura, false, &[obj(angel)], &[]);
    assert_eq!((g.power(angel), g.toughness(angel)), (1, 1));
    assert!(!g.keywords(angel).has(Keywords::FLYING));
    assert!(g.has_subtype(angel, "Citizen") && !g.has_subtype(angel, "Angel"));
}

#[test]
fn heraldic_banner_picks_a_color() {
    let mut g = lands(&["Plains", "Plains", "Plains"]);
    let banner = hand(&mut g, 0, "Heraldic Banner");
    let lions = put(&mut g, 0, "Savannah Lions");
    let scourge = put(&mut g, 0, "Brazen Scourge");
    begin(&mut g);
    play(&mut g, banner, false, &[], &[Action::Mode(0)]);
    assert_eq!((g.power(lions), g.power(scourge)), (3, 3), "white creatures only");
    assert_eq!(g.obj(banner).chosen_color, Some(0));
}

#[test]
fn strongbox_raider_plays_a_card_from_exile() {
    let mut g = table();
    let lions = put(&mut g, 0, "Savannah Lions");
    let raider = hand(&mut g, 0, "Strongbox Raider");
    begin(&mut g);
    attack_with(&mut g, &[lions]);
    pass_to(&mut g, Step::Main2);
    let forest = g.add_card(0, id("Forest"), Zone::Library); // on top, after the draw
    let top2: Vec<ObjId> = g.players[0].library.iter().rev().take(2).copied().collect();
    assert!(top2.contains(&forest));
    play(&mut g, raider, false, &[], &[Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Exile);
    assert!(
        g.legal_actions().contains(&Action::PlayLand(forest)),
        "playable from exile"
    );
    act(&mut g, Action::PlayLand(forest));
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
}

#[test]
fn curator_of_destinies_splits_into_piles() {
    let mut g = table();
    let curator = hand(&mut g, 0, "Curator of Destinies");
    begin(&mut g);
    let before = g.players[0].hand.len();
    act(&mut g, Action::Cast(curator));
    resolve_all(
        &mut g,
        &[
            Action::Yes,
            Action::Yes,
            Action::No,
            Action::No,
            Action::No,
            Action::Yes,
        ], // A gets 2; opponent takes A
    );
    assert_eq!(g.players[0].hand.len(), before - 1 + 2);
    assert_eq!(g.players[0].graveyard.len(), 3);
}
