//! Muldrotha, the Gravetide: one land and one permanent spell of each permanent type from your
//! graveyard during each of your turns.

mod common;

use bolt_core::{Action, Game, Pending, Step, Zone};
use common::*;

/// Permanent types, as Muldrotha's action numbers them.
const ARTIFACT: u8 = 0;
const CREATURE: u8 = 1;

fn grave(g: &mut Game, name: &str) -> bolt_core::ObjId {
    g.add_card(0, id(name), Zone::Graveyard)
}

#[test]
fn one_permanent_spell_of_each_type_from_the_graveyard() {
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Muldrotha, the Gravetide");
    let a = grave(&mut g, "Savannah Lions");
    let b = grave(&mut g, "Healer's Hawk");
    assert!(g
        .legal_actions()
        .contains(&Action::CastFromGraveyardAs(a, CREATURE)));
    act(&mut g, Action::CastFromGraveyardAs(a, CREATURE));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(a).zone, Zone::Battlefield);
    assert!(
        !g.legal_actions()
            .contains(&Action::CastFromGraveyardAs(b, CREATURE)),
        "one creature a turn"
    );
}

#[test]
fn an_artifact_creature_uses_the_type_you_choose() {
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Muldrotha, the Gravetide");
    let guide = grave(&mut g, "Campus Guide");
    let lions = grave(&mut g, "Savannah Lions");
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::CastFromGraveyardAs(guide, ARTIFACT)));
    assert!(legal.contains(&Action::CastFromGraveyardAs(guide, CREATURE)));
    act(&mut g, Action::CastFromGraveyardAs(guide, ARTIFACT));
    resolve_all(&mut g, &[Action::No]); // its enters trigger: decline
    assert!(g
        .legal_actions()
        .contains(&Action::CastFromGraveyardAs(lions, CREATURE)));
}

#[test]
fn a_land_from_the_graveyard_uses_the_land_drop() {
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Muldrotha, the Gravetide");
    let forest = grave(&mut g, "Forest");
    assert!(g.legal_actions().contains(&Action::PlayLand(forest)));
    act(&mut g, Action::PlayLand(forest));
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    let other = hand(&mut g, 0, "Plains");
    assert!(
        !g.legal_actions().contains(&Action::PlayLand(other)),
        "the land drop is used"
    );
}

#[test]
fn only_during_your_turn() {
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Muldrotha, the Gravetide");
    let hawk = grave(&mut g, "Resolute Reinforcements"); // flash: timing alone allows it
    while !(g.active == 1 && g.step == Step::Main1) {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    // Player 0 gets priority on the opponent's turn after they pass.
    act(&mut g, Action::PassPriority);
    assert!(matches!(g.pending, Pending::Priority(0)));
    assert!(!g
        .legal_actions()
        .contains(&Action::CastFromGraveyardAs(hawk, CREATURE)));
}
