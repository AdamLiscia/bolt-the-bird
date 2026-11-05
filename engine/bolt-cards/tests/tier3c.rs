//! Tier-3 cards, batch 4: ward, modal triggers, upkeep and draw triggers, cast triggers,
//! last-known power, and "that creature".

mod common;

use bolt_core::{Action, Event, Game, Keywords, Pending, Step, Zone};
use common::*;

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

fn attack_with(g: &mut Game, attackers: &[bolt_core::ObjId]) {
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    for &a in attackers {
        act(g, Action::Attack(a));
    }
    act(g, Action::FinishAttacks);
}

#[test]
fn diregraf_ghoul_enters_tapped() {
    let mut g = table();
    let c = hand(&mut g, 0, "Diregraf Ghoul");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert!(g.obj(c).tapped);
}

#[test]
fn dauntless_veteran_pumps_the_team_on_attack() {
    let mut g = table();
    let v = put(&mut g, 0, "Dauntless Veteran");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    attack_with(&mut g, &[v]);
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(v), g.power(lions)), (3, 3));
}

#[test]
fn shivan_dragon_firebreathes() {
    let mut g = table();
    let d = put(&mut g, 0, "Shivan Dragon");
    begin(&mut g);
    for _ in 0..3 {
        act(&mut g, Action::Activate(d, 0));
        resolve_all(&mut g, &[]);
    }
    assert_eq!(g.power(d), 8);
}

#[test]
fn zombify_and_day_of_judgment() {
    let mut g = table();
    let z = hand(&mut g, 0, "Zombify");
    let dead = g.add_card(0, id("Serra Angel"), Zone::Graveyard);
    begin(&mut g);
    play(&mut g, z, false, &[obj(dead)], &[]);
    assert_eq!(g.obj(dead).zone, Zone::Battlefield);

    let mut g = table();
    let doj = hand(&mut g, 0, "Day of Judgment");
    let a = put(&mut g, 0, "Savannah Lions");
    let b = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    play(&mut g, doj, false, &[], &[]);
    assert_eq!((g.obj(a).zone, g.obj(b).zone), (Zone::Graveyard, Zone::Graveyard));
}

#[test]
fn phyrexian_arena_draws_in_upkeep() {
    let mut g = table();
    put(&mut g, 0, "Phyrexian Arena");
    g.start_turn(3, 0);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 19);
    assert_eq!(
        g.players[0].hand.len(),
        1,
        "the Arena's card; the draw step comes next"
    );
}

#[test]
fn scrawling_crawler_punishes_opponents_draws() {
    let mut g = table();
    put(&mut g, 0, "Scrawling Crawler");
    g.start_turn(3, 0);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].hand.len(), 1, "each player draws");
    assert_eq!(g.players[1].life, 19, "and the opponent's draw costs 1 life");
    assert_eq!(g.players[0].life, 20);
}

#[test]
fn rune_sealed_wall_surveils() {
    let mut g = table();
    let w = put(&mut g, 0, "Rune-Sealed Wall");
    begin(&mut g);
    act(&mut g, Action::Activate(w, 0));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.players[0].graveyard.len(), 1);
}

#[test]
fn searslicer_goblin_needs_an_attack() {
    let mut g = table();
    let gob = put(&mut g, 0, "Searslicer Goblin");
    begin(&mut g);
    attack_with(&mut g, &[gob]);
    pass_to(&mut g, Step::End);
    resolve_all(&mut g, &[]);
    assert_eq!(tokens_named(&g, 0, "Goblin"), 1);
}

#[test]
fn good_fortune_unicorn_counters_the_newcomer() {
    let mut g = table();
    put(&mut g, 0, "Good-Fortune Unicorn");
    let lions = hand(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(g.power(lions), 3);
}

#[test]
fn clinquant_skymage_grows_on_each_draw() {
    let mut g = table();
    let s = put(&mut g, 0, "Clinquant Skymage");
    g.start_turn(3, 0);
    pass_to(&mut g, Step::Main1);
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(s), 2, "the draw-step card");
}

#[test]
fn archmage_of_runes_discounts_and_draws() {
    let mut g = lands(&["Island"]);
    put(&mut g, 0, "Archmage of Runes");
    let think = hand(&mut g, 0, "Think Twice"); // {1}{U} -> {U}
    begin(&mut g);
    let before = g.players[0].hand.len();
    play(&mut g, think, false, &[], &[]);
    assert_eq!(
        g.players[0].hand.len(),
        before - 1 + 1 + 1,
        "Think Twice's card and the Archmage's"
    );
}

#[test]
fn balmor_pumps_on_instants_and_sorceries() {
    let mut g = table();
    let b = put(&mut g, 0, "Balmor, Battlemage Captain");
    let c = hand(&mut g, 0, "Boltwave");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.power(b), 2);
    assert!(g.keywords(b).has(Keywords::TRAMPLE));
}

#[test]
fn firespitter_whelp_counts_dragons_too() {
    let mut g = table();
    put(&mut g, 0, "Firespitter Whelp");
    let dragon = hand(&mut g, 0, "Shivan Dragon");
    let lions = hand(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(g.players[1].life, 20, "not a Dragon");
    play(&mut g, dragon, false, &[], &[]);
    assert_eq!(g.players[1].life, 19);
}

#[test]
fn sower_of_chaos_stops_a_blocker() {
    let mut g = table();
    let sower = put(&mut g, 0, "Sower of Chaos");
    let wall = put(&mut g, 1, "Aegis Turtle");
    begin(&mut g);
    act(&mut g, Action::Activate(sower, 0));
    act(&mut g, Action::Target(obj(wall)));
    resolve_all(&mut g, &[]);
    attack_with(&mut g, &[sower]);
    assert_ne!(
        g.pending,
        Pending::DeclareBlockers(1),
        "its only blocker can't block"
    );
}

#[test]
fn heartfire_immolator_uses_last_known_power() {
    // CR 608.2h: prowess made it 3/3; it deals 3 after being sacrificed.
    let mut g = table();
    let imm = put(&mut g, 0, "Heartfire Immolator");
    let c = hand(&mut g, 0, "Boltwave");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    act(&mut g, Action::Activate(imm, 0));
    act(&mut g, Action::Target(obj(angel)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(angel).damage, 3);
}

#[test]
fn ward_counters_unless_paid() {
    // CR 702.21a. Stab targets Cackling Prowler; its ward asks for {2}.
    let mut g = lands(&["Swamp"]);
    let stab = hand(&mut g, 0, "Stab");
    let prowler = put(&mut g, 1, "Cackling Prowler");
    begin(&mut g);
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(prowler)));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority); // the ward trigger resolves
    assert_eq!(g.pending, Pending::Decide(0));
    assert_eq!(
        g.legal_actions(),
        vec![Action::No],
        "can't pay {{2}}: only one land, and it's tapped"
    );
    let ev = act(&mut g, Action::No);
    assert!(ev.contains(&Event::Fizzled { source: stab }));
    assert_eq!(g.obj(stab).zone, Zone::Graveyard);
    assert_eq!(g.toughness(prowler), 3);

    // With mana to spare, pay and Stab resolves.
    let mut g = lands(&["Swamp", "Swamp", "Swamp"]);
    let stab = hand(&mut g, 0, "Stab");
    let prowler = put(&mut g, 1, "Cackling Prowler");
    begin(&mut g);
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(prowler)));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.toughness(prowler), 1);
}

#[test]
fn ward_ignores_its_controllers_own_spells() {
    let mut g = table();
    let growth = hand(&mut g, 0, "Giant Growth");
    let prowler = put(&mut g, 0, "Cackling Prowler");
    begin(&mut g);
    play(&mut g, growth, false, &[obj(prowler)], &[]);
    assert_eq!(g.power(prowler), 7);
}

#[test]
fn apothecary_stomper_chooses_a_mode() {
    let mut g = table();
    let c = hand(&mut g, 0, "Apothecary Stomper");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert_eq!(g.legal_actions(), vec![Action::Mode(0), Action::Mode(1)]);
    act(&mut g, Action::Mode(1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 24);

    let mut g = table();
    let c = hand(&mut g, 0, "Apothecary Stomper");
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Mode(0), Action::Target(obj(c))]);
    assert_eq!(g.power(c), 6);
}

#[test]
fn wardens_of_the_cycle_is_a_morbid_modal_trigger() {
    let mut g = table();
    put(&mut g, 0, "Wardens of the Cycle");
    let lions = put(&mut g, 1, "Savannah Lions");
    let stab = hand(&mut g, 0, "Stab");
    begin(&mut g);
    play(&mut g, stab, false, &[obj(lions)], &[]);
    pass_to(&mut g, Step::End);
    resolve_all(&mut g, &[Action::Mode(0)]);
    assert_eq!(g.players[0].life, 22);
}

#[test]
fn ruby_daring_tracker() {
    let mut g = table();
    let ruby = put(&mut g, 0, "Ruby, Daring Tracker");
    put(&mut g, 0, "Serra Angel");
    begin(&mut g);
    attack_with(&mut g, &[ruby]);
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(ruby), 3);
    let m = g.def_of(ruby).mana.unwrap();
    assert!(m.colors[3] && m.colors[4]);
}
