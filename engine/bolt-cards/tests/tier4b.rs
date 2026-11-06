//! Tier-4 cards, batch 2: X costs (CR 107.3) and hybrid mana.

mod common;

use bolt_core::{Action, Zone};
use common::*;

#[test]
fn wildwood_scourge_enters_with_x_and_grows() {
    let mut g = lands(&["Forest"; 4]);
    let w = hand(&mut g, 0, "Wildwood Scourge");
    let veil = hand(&mut g, 0, "Snakeskin Veil");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    assert!(g.legal_actions().contains(&Action::CastX(w, 3)));
    assert!(
        !g.legal_actions().contains(&Action::CastX(w, 4)),
        "{{X}}{{G}} with four Forests: X up to 3"
    );
    act(&mut g, Action::CastX(w, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(w), 2);
    play(&mut g, veil, false, &[obj(lions)], &[]);
    assert_eq!(g.power(w), 3, "a counter on another non-Hydra creature");
}

#[test]
fn goblin_negotiation_turns_excess_damage_into_goblins() {
    let mut g = lands(&["Mountain"; 7]);
    let c = hand(&mut g, 0, "Goblin Negotiation");
    let lions = put(&mut g, 1, "Savannah Lions"); // toughness 1
    begin(&mut g);
    act(&mut g, Action::CastX(c, 5));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
    assert_eq!(tokens_named(&g, 0, "Goblin"), 4, "5 damage, 1 of it lethal");
}

#[test]
fn exsanguinate_drains_x() {
    let mut g = lands(&["Swamp"; 6]);
    let c = hand(&mut g, 0, "Exsanguinate");
    begin(&mut g);
    act(&mut g, Action::CastX(c, 4));
    resolve_all(&mut g, &[]);
    assert_eq!((g.players[0].life, g.players[1].life), (24, 16));
}

#[test]
fn genesis_wave_puts_cheap_permanents_onto_the_battlefield() {
    let mut g = lands(&["Forest"; 6]);
    let c = hand(&mut g, 0, "Genesis Wave"); // {X}{G}{G}{G}
    begin(&mut g);
    // Put known cards on top: a 1-drop, a 5-drop and an instant.
    let lions = g.add_card(0, id("Savannah Lions"), Zone::Library);
    let angel = g.add_card(0, id("Serra Angel"), Zone::Library);
    let stab = g.add_card(0, id("Stab"), Zone::Library);
    act(&mut g, Action::CastX(c, 3));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(lions)));
    assert!(!legal.contains(&Action::Pick(angel)), "mana value 5 > X");
    assert!(!legal.contains(&Action::Pick(stab)), "not a permanent card");
    act(&mut g, Action::Pick(lions));
    act(&mut g, Action::No);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield);
    assert_eq!(
        (g.obj(angel).zone, g.obj(stab).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
}

#[test]
fn fiend_artisan_has_hybrid_costs_and_tutors_for_x() {
    let mut g = lands(&["Swamp", "Forest", "Swamp"]);
    let f = hand(&mut g, 0, "Fiend Artisan"); // {B/G}{B/G}
    let food = put(&mut g, 0, "Savannah Lions");
    for _ in 0..2 {
        g.add_card(0, id("Stab"), Zone::Graveyard);
    }
    g.add_card(0, id("Serra Angel"), Zone::Graveyard);
    let target = library_card(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, f, false, &[], &[]);
    assert_eq!(g.power(f), 2, "one creature card in the graveyard");
    g.objects[f.0 as usize].since_turn = 0; // pretend it's been here since last turn
    for land in g
        .permanents(0)
        .filter(|&x| g.def_of(x).types.land)
        .collect::<Vec<_>>()
    {
        g.objects[land.0 as usize].tapped = false;
    }
    let act_x = Action::ActivateX {
        id: f,
        n: 0,
        victim: Some(food),
        x: 1,
    };
    assert!(g.legal_actions().contains(&act_x));
    act(&mut g, act_x);
    resolve_all(&mut g, &[Action::Pick(target)]);
    assert_eq!(g.obj(target).zone, Zone::Battlefield);
}

#[test]
fn a_cost_reduction_needs_a_legal_target_to_count() {
    // Found by the fuzzer (seed 5858): Luminous Rebuke is cheap only against a tapped creature. The
    // only tapped creature had hexproof from instants, so the cheap cost wasn't really available.
    let mut g = lands(&["Plains", "Plains"]);
    let rebuke = hand(&mut g, 0, "Luminous Rebuke");
    let elenda = put(&mut g, 1, "Elenda, Saint of Dusk");
    g.objects[elenda.0 as usize].tapped = true;
    begin(&mut g);
    assert!(!g.legal_actions().contains(&Action::Cast(rebuke)));
}
