//! Kellan's stages and Secluded Courtyard's restricted mana.

mod common;

use bolt_core::{Action, Game, Keywords, Pending, Step, Zone};
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

fn activate(g: &mut Game, id: bolt_core::ObjId, n: usize) {
    act(g, Action::Activate(id, n));
    resolve_all(g, &[]);
}

#[test]
fn kellan_goes_scout_to_detective_to_rogue_in_order() {
    let mut g = table();
    begin(&mut g);
    let k = put(&mut g, 0, "Kellan, Planar Trailblazer");
    activate(&mut g, k, 1); // "If Kellan is a Detective": nothing yet
    assert!(g.has_subtype(k, "Scout") && !g.has_subtype(k, "Rogue"));
    activate(&mut g, k, 0);
    assert!(g.has_subtype(k, "Detective") && g.has_subtype(k, "Faerie") && !g.has_subtype(k, "Scout"));
    assert_eq!((g.power(k), g.toughness(k)), (2, 1));
    activate(&mut g, k, 1);
    assert!(g.has_subtype(k, "Rogue") && !g.has_subtype(k, "Detective"));
    assert_eq!((g.power(k), g.toughness(k)), (3, 2));
    assert!(g.keywords(k).has(Keywords::DOUBLE_STRIKE));
}

#[test]
fn a_scout_kellan_has_no_combat_damage_trigger() {
    let mut g = table();
    begin(&mut g);
    let k = put(&mut g, 0, "Kellan, Planar Trailblazer");
    let top = g.add_card(0, id("Burst Lightning"), Zone::Library);
    pass_to(&mut g, Step::DeclareAttackers);
    act(&mut g, Action::Attack(k));
    act(&mut g, Action::FinishAttacks);
    pass_to(&mut g, Step::EndCombat);
    assert_eq!(g.players[1].life, 18);
    assert_eq!(g.obj(top).zone, Zone::Library);
}

#[test]
fn a_detective_kellan_exiles_the_top_card_to_play_this_turn() {
    let mut g = table();
    begin(&mut g);
    let k = put(&mut g, 0, "Kellan, Planar Trailblazer");
    activate(&mut g, k, 0);
    let top = g.add_card(0, id("Burst Lightning"), Zone::Library);
    pass_to(&mut g, Step::DeclareAttackers);
    act(&mut g, Action::Attack(k));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::EndCombat {
        let a = match g.pending {
            Pending::Decide(_) => Action::Pick(top),
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(top).zone, Zone::Exile);
    pass_to(&mut g, Step::Main2);
    assert!(g.legal_actions().contains(&Action::Cast(top)));
}

#[test]
fn secluded_courtyard_colored_mana_only_casts_the_chosen_type() {
    let mut g = lands(&[]);
    begin(&mut g);
    let yard = hand(&mut g, 0, "Secluded Courtyard");
    let lions = hand(&mut g, 0, "Savannah Lions"); // Cat, {W}
    let hawk = hand(&mut g, 0, "Healer's Hawk"); // Bird, {W}
    act(&mut g, Action::PlayLand(yard));
    assert!(matches!(g.pending, Pending::Decide(0)));
    let types = g.creature_type_options();
    let cat = types.iter().position(|t| t == "Cat").expect("Cat offered") as u8;
    act(&mut g, Action::Mode(cat));
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Cast(lions)), "{legal:?}");
    assert!(!legal.contains(&Action::Cast(hawk)));
}
