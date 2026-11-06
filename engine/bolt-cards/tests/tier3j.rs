//! Tier-3 cards, batch 10a: protection, repeated mill, bait counters, and stashed cards.

mod common;

use bolt_core::{Action, Game, Keywords, ObjId, Pending, Zone};
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

#[test]
fn akromas_memorial_grants_keywords_and_protection() {
    // CR 702.16: protection from black and red stops targeting, blocking and damage.
    let mut g = table();
    put(&mut g, 0, "Akroma's Memorial");
    let lions = put(&mut g, 0, "Savannah Lions");
    let stab = hand(&mut g, 1, "Stab"); // black
    put(&mut g, 1, "Vampire Nighthawk"); // a black flier
    begin(&mut g);
    let k = g.keywords(lions);
    assert!(k.has(Keywords::FLYING) && k.has(Keywords::FIRST_STRIKE) && k.has(Keywords::HASTE));
    attack_with(&mut g, &[lions]);
    assert_ne!(
        g.pending,
        Pending::DeclareBlockers(1),
        "can't be blocked by a black creature"
    );
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(stab));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(lions))),
        "can't be targeted by black spells"
    );

    let mut g = table();
    put(&mut g, 0, "Akroma's Memorial");
    let lions = put(&mut g, 0, "Savannah Lions");
    let storm = hand(&mut g, 1, "Slagstorm"); // red, 3 damage to each creature
    begin(&mut g);
    act(&mut g, Action::PassPriority);

    // Cast it on the opponent's own turn instead: give them a turn.
    while g.active == 0 {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    while g.step != bolt_core::Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::CastModal(storm, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield, "red damage is prevented");
}

#[test]
fn sphinxs_tutelage_keeps_milling_while_colors_match() {
    let mut g = table();
    put(&mut g, 0, "Sphinx's Tutelage");
    g.start_turn(3, 0);
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        act(&mut g, Action::PassPriority);
    }
    resolve_all(&mut g, &[bolt_core::Action::Target(bolt_core::Target::Player(1))]);
    // The opponent's library is all blue Aegis Turtles: every pair shares blue, so it keeps going.
    assert!(g.players[1].library.is_empty());
}

#[test]
fn fishing_pole_catches_a_fish() {
    let mut g = table();
    let pole = put(&mut g, 0, "Fishing Pole");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    act(&mut g, Action::Activate(pole, 1)); // equip {2}
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    act(&mut g, Action::Activate(pole, 0)); // {1}, tap the creature and the Pole
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(pole).bait, 1);
    assert!(g.obj(lions).tapped && g.obj(pole).tapped);
    while g.active == 0 || g.turn < 5 {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    resolve_all(&mut g, &[]);
    assert_eq!(
        tokens_named(&g, 0, "Fish"),
        1,
        "the creature untapped and caught a Fish"
    );
    assert_eq!(g.obj(pole).bait, 0);
}

#[test]
fn tinybones_steals_discarded_cards() {
    let mut g = table();
    put(&mut g, 0, "Tinybones, Bauble Burglar");
    let rat = hand(&mut g, 0, "Burglar Rat");
    let lions = hand(&mut g, 1, "Savannah Lions");
    begin(&mut g);
    play(&mut g, rat, false, &[], &[Action::Discard(lions)]);
    assert_eq!(g.obj(lions).zone, Zone::Exile);
    assert!(g.obj(lions).stash);
    assert!(
        g.legal_actions().contains(&Action::Cast(lions)),
        "play it with any mana"
    );
    act(&mut g, Action::Cast(lions));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.obj(lions).zone, g.obj(lions).controller, g.obj(lions).owner),
        (Zone::Battlefield, 0, 1)
    );
}

#[test]
fn an_ability_with_only_protected_targets_cant_be_activated() {
    // Found by the fuzzer (seed 6555): Heartfire Immolator is red, and every creature it could
    // target had protection from red, so activating it left a target choice with no options.
    let mut g = table();
    put(&mut g, 0, "Akroma's Memorial");
    put(&mut g, 0, "Savannah Lions");
    let imm = put(&mut g, 0, "Heartfire Immolator");
    begin(&mut g);
    // Every creature, the Immolator included, has protection from red.
    assert!(
        !g.legal_actions().contains(&Action::Activate(imm, 0)),
        "no legal target"
    );
}
