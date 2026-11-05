//! Tier-3 cards, batch 6: optional sacrifices, graveyard abilities, looking at the top of the
//! library, targets limited by the source's power, and subtype filters.

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
fn vampire_gourmand_may_sacrifice_to_draw_and_slip_past() {
    let mut g = table();
    let v = put(&mut g, 0, "Vampire Gourmand");
    let food = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    let before = g.players[0].hand.len();
    attack_with(&mut g, &[v]);
    resolve_all(&mut g, &[Action::Pick(food)]);
    assert_eq!(g.obj(food).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), before + 1);
    assert!(g.keywords(v).has(Keywords::UNBLOCKABLE));

    // Declining does nothing.
    let mut g = table();
    let v = put(&mut g, 0, "Vampire Gourmand");
    put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    let before = g.players[0].hand.len();
    attack_with(&mut g, &[v]);
    resolve_all(&mut g, &[Action::No]);
    assert_eq!(g.players[0].hand.len(), before);
    assert!(!g.keywords(v).has(Keywords::UNBLOCKABLE));
}

#[test]
fn high_society_hunter_draws_when_nontoken_creatures_die() {
    let mut g = table();
    put(&mut g, 0, "High-Society Hunter");
    let theirs = put(&mut g, 1, "Savannah Lions");
    let stab = hand(&mut g, 0, "Stab");
    begin(&mut g);
    let before = g.players[0].hand.len();
    play(&mut g, stab, false, &[obj(theirs)], &[]);
    assert_eq!(
        g.players[0].hand.len(),
        before - 1 + 1,
        "any player's creature counts"
    );
}

#[test]
fn reassembling_skeleton_comes_back_from_the_graveyard() {
    let mut g = table();
    let sk = g.add_card(0, id("Reassembling Skeleton"), Zone::Graveyard);
    begin(&mut g);
    act(&mut g, Action::Activate(sk, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(sk).zone, Zone::Battlefield);
    assert!(g.obj(sk).tapped);
    assert!(
        !g.legal_actions().contains(&Action::Activate(sk, 0)),
        "only from the graveyard"
    );
}

#[test]
fn gutless_plunderer_keeps_one_on_top() {
    let mut g = table();
    let lions = put(&mut g, 0, "Savannah Lions");
    let c = hand(&mut g, 0, "Gutless Plunderer");
    begin(&mut g);
    attack_with(&mut g, &[lions]);
    pass_to(&mut g, Step::Main2);
    let top3: Vec<ObjId> = g.players[0].library.iter().rev().take(3).copied().collect();
    play(&mut g, c, false, &[], &[Action::Pick(top3[2])]);
    assert_eq!(*g.players[0].library.last().unwrap(), top3[2]);
    assert!(g.obj(top3[0]).zone == Zone::Graveyard && g.obj(top3[1]).zone == Zone::Graveyard);
}

#[test]
fn squad_rallier_finds_a_small_creature() {
    let mut g = table();
    let r = put(&mut g, 0, "Squad Rallier");
    begin(&mut g);
    // The library is Aegis Turtles (0/5): power 0, so they qualify.
    let top: Vec<ObjId> = g.players[0].library.iter().rev().take(4).copied().collect();
    act(&mut g, Action::Activate(r, 0));
    resolve_all(&mut g, &[Action::Pick(top[0])]);
    assert_eq!(g.obj(top[0]).zone, Zone::Hand);
    let lib = &g.players[0].library;
    assert!(
        top[1..].iter().all(|c| lib[..3].contains(c)),
        "the rest go to the bottom"
    );
}

#[test]
fn inspiration_from_beyond_mills_then_returns_a_spell() {
    let mut g = table();
    let c = hand(&mut g, 0, "Inspiration from Beyond");
    let stab = g.add_card(0, id("Stab"), Zone::Graveyard);
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Pick(stab)]);
    assert_eq!(g.obj(stab).zone, Zone::Hand);
    assert_eq!(g.players[0].graveyard.len(), 4, "three milled, plus itself");
}

#[test]
fn alesha_returns_creatures_up_to_her_power() {
    let mut g = table();
    let alesha = put(&mut g, 0, "Alesha, Who Laughs at Fate");
    let small = g.add_card(0, id("Savannah Lions"), Zone::Graveyard); // mana value 1
    let big = g.add_card(0, id("Serra Angel"), Zone::Graveyard); // mana value 5
    begin(&mut g);
    attack_with(&mut g, &[alesha]);
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(alesha), 3);
    pass_to(&mut g, Step::End);
    assert_eq!(
        g.legal_actions(),
        vec![Action::Target(obj(small))],
        "power 3: the Angel costs 5"
    );
    act(&mut g, Action::Target(obj(small)));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.obj(small).zone, g.obj(big).zone),
        (Zone::Battlefield, Zone::Graveyard)
    );
}

#[test]
fn fiendish_panda_brings_back_a_cheaper_creature() {
    let mut g = table();
    let panda = put(&mut g, 0, "Fiendish Panda");
    let small = g.add_card(0, id("Savannah Lions"), Zone::Graveyard);
    let stab = hand(&mut g, 1, "Stab");
    begin(&mut g);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(panda)));
    resolve_all(&mut g, &[Action::Target(obj(small))]);
    assert_eq!(g.obj(small).zone, Zone::Battlefield);
    assert_eq!(g.obj(panda).zone, Zone::Graveyard, "another target: not itself");
}

#[test]
fn kiora_loots_and_may_make_a_scion() {
    let mut g = table();
    let k = hand(&mut g, 0, "Kiora, the Rising Tide");
    let a = hand(&mut g, 0, "Forest");
    let b = hand(&mut g, 0, "Island");
    begin(&mut g);
    play(&mut g, k, false, &[], &[Action::Discard(a), Action::Discard(b)]);
    assert_eq!(g.players[0].graveyard.len(), 2);
    for _ in 0..5 {
        g.add_card(0, id("Stab"), Zone::Graveyard);
    }
    g.objects[k.0 as usize].since_turn = 0;
    attack_with(&mut g, &[k]);
    resolve_all(&mut g, &[Action::Yes]);
    let scion = g
        .creatures(0)
        .find(|&c| g.def_of(c).name == "Scion of the Deep")
        .expect("the Scion");
    assert_eq!(g.power(scion), 8);
}

#[test]
fn cephalid_inkmage_is_unblockable_with_threshold() {
    let mut g = table();
    let c = hand(&mut g, 0, "Cephalid Inkmage");
    for _ in 0..4 {
        g.add_card(0, id("Stab"), Zone::Graveyard);
    }
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Yes, Action::Yes, Action::Yes]);
    assert_eq!(g.players[0].graveyard.len(), 7);
    assert!(g.keywords(c).has(Keywords::UNBLOCKABLE));
}

#[test]
fn stromkirk_bloodthief_needs_an_opponent_to_lose_life() {
    let mut g = table();
    let thief = put(&mut g, 0, "Stromkirk Bloodthief");
    let bolt = hand(&mut g, 0, "Boltwave");
    begin(&mut g);
    play(&mut g, bolt, false, &[], &[]);
    pass_to(&mut g, Step::End);
    resolve_all(&mut g, &[Action::Target(obj(thief))]);
    assert_eq!(g.power(thief), 3);
}
