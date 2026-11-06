//! Tier-3 cards, batch 7: stun counters, "may pay" in triggers, ward paid in life, can't be
//! countered, affinity, "lose life unless you discard", reveal-until, counter doubling.

mod common;

use bolt_core::{Action, Game, Keywords, ObjId, Pending, Step, Target, Zone};
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
fn mischievous_pup_returns_another_permanent_even_a_land() {
    let mut g = table();
    let pup = hand(&mut g, 0, "Mischievous Pup");
    let land = g.permanents(0).find(|&c| g.def_of(c).name == "Plains").unwrap();
    begin(&mut g);
    act(&mut g, Action::Cast(pup));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(pup))),
        "another permanent"
    );
    resolve_all(&mut g, &[Action::Target(obj(land))]);
    assert_eq!(g.obj(land).zone, Zone::Hand);
}

#[test]
fn arahbo_pumps_cats_and_makes_more() {
    let mut g = table();
    let arahbo = hand(&mut g, 0, "Arahbo, the First Fang");
    let hunter = put(&mut g, 0, "Helpful Hunter"); // a Cat
    begin(&mut g);
    play(&mut g, arahbo, false, &[], &[]);
    assert_eq!(tokens_named(&g, 0, "Cat"), 1, "Arahbo itself entering counts");
    assert_eq!(g.power(hunter), 2, "other Cats get +1/+1");
    assert_eq!(g.power(arahbo), 2, "not itself");
}

#[test]
fn scavenging_ooze_grows_on_creature_cards() {
    let mut g = table();
    let ooze = put(&mut g, 0, "Scavenging Ooze");
    let dead = g.add_card(1, id("Serra Angel"), Zone::Graveyard);
    let spell = g.add_card(1, id("Stab"), Zone::Graveyard);
    begin(&mut g);
    act(&mut g, Action::Activate(ooze, 0));
    act(&mut g, Action::Target(obj(dead)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(ooze), g.players[0].life), (3, 21));
    act(&mut g, Action::Activate(ooze, 0));
    act(&mut g, Action::Target(obj(spell)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(ooze), g.players[0].life), (3, 21), "not a creature card");
}

#[test]
fn garruks_uprising() {
    let mut g = table();
    let c = hand(&mut g, 0, "Garruk's Uprising");
    put(&mut g, 0, "Serra Angel");
    let lions = put(&mut g, 0, "Savannah Lions");
    let big = hand(&mut g, 0, "Brazen Scourge"); // 3/3: no draw
    let huge = hand(&mut g, 0, "Quakestrider Ceratops"); // 12/8: draw
    begin(&mut g);
    let before = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), before, "cast it, drew one");
    assert!(g.keywords(lions).has(Keywords::TRAMPLE));
    play(&mut g, big, false, &[], &[]);
    let mid = g.players[0].hand.len();
    play(&mut g, huge, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), mid, "cast the Ceratops, drew one");
}

#[test]
fn armasaur_guide_needs_three_attackers() {
    let mut g = table();
    let guide = put(&mut g, 0, "Armasaur Guide");
    let a = put(&mut g, 0, "Savannah Lions");
    let b = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    attack_with(&mut g, &[guide, a, b]);
    resolve_all(&mut g, &[Action::Target(obj(a))]);
    assert_eq!(g.power(a), 3);
}

#[test]
fn grappling_kraken_stuns() {
    // CR 122.1d: the stunned creature stays tapped through one untap step.
    let mut g = lands(&[]);
    put(&mut g, 0, "Grappling Kraken");
    let victim = put(&mut g, 1, "Serra Angel");
    let land = hand(&mut g, 0, "Island");
    begin(&mut g);
    act(&mut g, Action::PlayLand(land));
    resolve_all(&mut g, &[Action::Target(obj(victim))]);
    assert!(g.obj(victim).tapped);
    while g.active == 0 {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(g.obj(victim).tapped, "the stun counter replaced its untap");
    assert_eq!(g.obj(victim).stun, 0);
}

#[test]
fn sylvan_scavenging_raccoon_needs_power_four() {
    let mut g = table();
    put(&mut g, 0, "Sylvan Scavenging");
    begin(&mut g);
    pass_to(&mut g, Step::End);
    resolve_all(&mut g, &[Action::Mode(1)]);
    assert_eq!(tokens_named(&g, 0, "Raccoon"), 0, "no creature with power 4");

    let mut g = table();
    put(&mut g, 0, "Sylvan Scavenging");
    put(&mut g, 0, "Serra Angel");
    begin(&mut g);
    pass_to(&mut g, Step::End);
    resolve_all(&mut g, &[Action::Mode(1)]);
    assert_eq!(tokens_named(&g, 0, "Raccoon"), 1);
}

#[test]
fn frenzied_goblin_may_pay_to_stop_a_blocker() {
    let mut g = lands(&["Mountain"]);
    let gob = put(&mut g, 0, "Frenzied Goblin");
    let wall = put(&mut g, 1, "Aegis Turtle");
    begin(&mut g);
    attack_with(&mut g, &[gob]);
    resolve_all(&mut g, &[Action::Target(obj(wall)), Action::Yes]);
    assert!(g.keywords(wall).has(Keywords::CANT_BLOCK));
}

#[test]
fn sire_of_seven_deaths_ward_costs_life() {
    let mut g = table();
    let sire = put(&mut g, 1, "Sire of Seven Deaths");
    let stab = hand(&mut g, 0, "Stab");
    begin(&mut g);
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(sire)));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.players[0].life, 13);
    assert_eq!(g.toughness(sire), 5);
}

#[test]
fn koma_cant_be_countered_and_makes_coils() {
    let mut g = table();
    let koma = hand(&mut g, 0, "Koma, World-Eater");
    let refute = hand(&mut g, 1, "Refute");
    let junk = hand(&mut g, 1, "Forest");
    begin(&mut g);
    act(&mut g, Action::Cast(koma));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(refute));
    act(&mut g, Action::Target(obj(koma)));
    resolve_all(&mut g, &[Action::Discard(junk)]);
    assert_eq!(g.obj(koma).zone, Zone::Battlefield);
    assert!(g.keywords(koma).has(Keywords::TRAMPLE));

    let mut g = table();
    let koma = put(&mut g, 0, "Koma, World-Eater");
    begin(&mut g);
    attack_with(&mut g, &[koma]);
    pass_to(&mut g, Step::CombatDamage);
    resolve_all(&mut g, &[]);
    assert_eq!(tokens_named(&g, 0, "Koma's Coil"), 4);
}

#[test]
fn drakuseth_breathes_fire_at_up_to_three_targets() {
    let mut g = table();
    let d = put(&mut g, 0, "Drakuseth, Maw of Flames");
    let a = put(&mut g, 1, "Serra Angel");
    let b = put(&mut g, 1, "Savannah Lions");
    begin(&mut g);
    attack_with(&mut g, &[d]);
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::Target(obj(a)));
    act(&mut g, Action::Target(obj(b)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 16);
    assert_eq!(g.obj(a).damage, 3);
    assert_eq!(g.obj(b).zone, Zone::Graveyard);
}

#[test]
fn painful_quandary_taxes_spells() {
    let mut g = table();
    put(&mut g, 1, "Painful Quandary");
    let stab = hand(&mut g, 0, "Stab");
    let lions = put(&mut g, 1, "Savannah Lions");
    begin(&mut g);
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[Action::No]);
    assert_eq!(g.players[0].life, 15);
}

#[test]
fn exemplar_of_light_draws_once_a_turn() {
    let mut g = table();
    let ex = put(&mut g, 0, "Exemplar of Light");
    put(&mut g, 0, "Dazzling Angel");
    let a = hand(&mut g, 0, "Savannah Lions");
    let b = hand(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    let before = g.players[0].hand.len();
    play(&mut g, a, false, &[], &[]); // gain 1: counter: draw
    play(&mut g, b, false, &[], &[]); // gain 1: counter, but no second draw this turn
    assert_eq!(g.power(ex), 5);
    assert_eq!(g.players[0].hand.len(), before - 2 + 1);
}

#[test]
fn claws_out_has_affinity_for_cats() {
    let mut g = lands(&["Plains", "Plains"]);
    let c = hand(&mut g, 0, "Claws Out"); // {3}{W}{W}
    for _ in 0..3 {
        put(&mut g, 0, "Helpful Hunter");
    }
    begin(&mut g);
    assert!(
        g.legal_actions().contains(&Action::Cast(c)),
        "three Cats make it {{W}}{{W}}"
    );
}

#[test]
fn spinner_of_souls_digs_for_a_creature() {
    let mut g = table();
    put(&mut g, 0, "Spinner of Souls");
    let lions = put(&mut g, 0, "Savannah Lions");
    let stab = hand(&mut g, 1, "Stab");
    begin(&mut g);
    let before = g.players[0].hand.len();
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(
        g.players[0].hand.len(),
        before + 1,
        "the library is all creatures"
    );
}

#[test]
fn zimone_counts_and_doubles() {
    let mut g = table();
    let z = put(&mut g, 0, "Zimone, Paradox Sculptor");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    pass_to(&mut g, Step::BeginCombat);
    resolve_all(&mut g, &[Action::Target(obj(lions)), Action::No]);
    assert_eq!(g.power(lions), 3);
    pass_to(&mut g, Step::Main2);
    act(&mut g, Action::Activate(z, 0));
    act(&mut g, Action::Target(obj(lions)));
    act(&mut g, Action::No);
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(lions), 4);
}
