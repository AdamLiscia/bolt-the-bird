//! Tier 4, batch 6: replacement effects that double (Doubling Season, Twinflame Tyrant) and casting
//! without paying mana costs (Omniscience).

mod common;

use bolt_core::{Action, Target, Zone};
use common::*;

#[test]
fn doubling_season_doubles_tokens() {
    // CR 614.1a, 111.1.
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Doubling Season");
    let r = hand(&mut g, 0, "Resolute Reinforcements");
    play(&mut g, r, false, &[], &[]);
    assert_eq!(tokens_named(&g, 0, "Soldier"), 2);
}

#[test]
fn doubling_season_doubles_counters_and_starting_loyalty() {
    // CR 122.6, 306.5b: a planeswalker entering with loyalty counters gets twice as many; so does
    // a plus loyalty cost.
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Doubling Season");
    let ajani = hand(&mut g, 0, "Ajani, Caller of the Pride");
    play(&mut g, ajani, false, &[], &[]);
    assert_eq!(g.obj(ajani).loyalty, 8);
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::Loyalty(ajani, 0));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ajani).loyalty, 10);
    assert_eq!(g.power(lions), 4);
}

#[test]
fn doubling_season_only_doubles_for_its_controller() {
    let mut g = table();
    begin(&mut g);
    put(&mut g, 1, "Doubling Season");
    let r = hand(&mut g, 0, "Resolute Reinforcements");
    play(&mut g, r, false, &[], &[]);
    assert_eq!(tokens_named(&g, 0, "Soldier"), 1);
}

#[test]
fn twinflame_tyrant_doubles_damage_to_opponents_and_their_permanents() {
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Twinflame Tyrant");
    let bolt = hand(&mut g, 0, "Burst Lightning");
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    assert_eq!(g.players[1].life, 16);
    let turtle = put(&mut g, 1, "Aegis Turtle"); // 0/5
    let bolt = hand(&mut g, 0, "Burst Lightning");
    play(&mut g, bolt, false, &[obj(turtle)], &[]);
    assert_eq!(g.obj(turtle).damage, 4);
    let bolt = hand(&mut g, 0, "Burst Lightning");
    play(&mut g, bolt, false, &[Target::Player(0)], &[]);
    assert_eq!(g.players[0].life, 18, "not to yourself");
}

#[test]
fn omniscience_casts_from_hand_for_free() {
    let mut g = lands(&[]);
    begin(&mut g);
    put(&mut g, 0, "Omniscience");
    let raider = hand(&mut g, 0, "Gorehorn Raider");
    assert!(g.legal_actions().contains(&Action::Cast(raider)));
    play(&mut g, raider, false, &[], &[]);
    assert_eq!(g.obj(raider).zone, Zone::Battlefield);
}

#[test]
fn omniscience_makes_x_zero() {
    // CR 107.3b: cast without paying its mana cost, X can only be 0.
    let mut g = lands(&[]);
    begin(&mut g);
    put(&mut g, 0, "Omniscience");
    let ex = hand(&mut g, 0, "Exsanguinate");
    assert!(
        !g.legal_actions()
            .iter()
            .any(|a| matches!(a, Action::CastX(c, x) if *c == ex && *x > 0)),
        "{:?}",
        g.legal_actions()
    );
}
