//! Planeswalkers (CR 306, 606): loyalty, loyalty abilities, damage, attacking them, emblems.

mod common;

use bolt_core::{Action, Game, Keywords, ObjId, Pending, Step, Zone};
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

fn walker(g: &mut Game, p: usize, name: &str) -> ObjId {
    // Cast it for real so it enters with its starting loyalty.
    let w = hand(g, p, name);
    play(g, w, false, &[], &[]);
    w
}

#[test]
fn a_planeswalker_enters_with_loyalty_and_uses_one_ability_a_turn() {
    // CR 306.5b, 606.3.
    let mut g = table();
    begin(&mut g);
    let ajani = walker(&mut g, 0, "Ajani, Caller of the Pride");
    assert_eq!(g.obj(ajani).loyalty, 4);
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::Loyalty(ajani, 0));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(ajani).loyalty, g.power(lions)), (5, 3));
    assert!(
        !g.legal_actions().iter().any(|a| matches!(a, Action::Loyalty(..))),
        "once per turn"
    );
}

#[test]
fn a_minus_ability_cant_go_below_zero() {
    // CR 606.6.
    let mut g = table();
    begin(&mut g);
    let ajani = walker(&mut g, 0, "Ajani, Caller of the Pride");
    assert!(
        !g.legal_actions().contains(&Action::Loyalty(ajani, 2)),
        "−8 with 4 loyalty"
    );
    assert!(g.legal_actions().contains(&Action::Loyalty(ajani, 1)) || g.creatures(0).count() == 0);
}

#[test]
fn damage_removes_loyalty_and_zero_loyalty_dies() {
    // CR 120.3c, 704.5i.
    let mut g = table();
    begin(&mut g);
    let kaito = walker(&mut g, 0, "Kaito, Cunning Infiltrator"); // 3 loyalty
    let bolt = hand(&mut g, 1, "Burst Lightning");
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(obj(kaito)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(kaito).loyalty, 1);
    let bolt2 = hand(&mut g, 1, "Burst Lightning");
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(bolt2));
    act(&mut g, Action::Target(obj(kaito)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(kaito).zone, Zone::Graveyard);
}

#[test]
fn creatures_can_attack_a_planeswalker() {
    // CR 508.1b.
    let mut g = table();
    let w = hand(&mut g, 1, "Vivien Reid");
    let lions = put(&mut g, 0, "Savannah Lions");
    g.start_turn(3, 1);
    pass_to(&mut g, Step::Main1);
    play(&mut g, w, false, &[], &[]);
    while g.active == 1 {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::AttackPlaneswalker(lions, w));
    act(&mut g, Action::FinishAttacks);
    pass_to(&mut g, Step::EndCombat);
    assert_eq!(g.obj(w).loyalty, 3, "5 minus 2");
    assert_eq!(g.players[1].life, 20);
}

#[test]
fn liliana_draws_on_deaths_and_makes_zombies() {
    let mut g = table();
    begin(&mut g);
    let lili = walker(&mut g, 0, "Liliana, Dreadhorde General");
    act(&mut g, Action::Loyalty(lili, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens_named(&g, 0, "Zombie"), 1);
    let zombie = g.creatures(0).find(|&c| g.def_of(c).name == "Zombie").unwrap();
    let stab = hand(&mut g, 0, "Stab");
    let before = g.players[0].hand.len();
    play(&mut g, stab, false, &[obj(zombie)], &[]);
    assert_eq!(g.players[0].hand.len(), before - 1 + 1, "a creature died: draw");
}

#[test]
fn liliana_ultimate_keeps_one_of_each_type() {
    let mut g = table();
    begin(&mut g);
    let lili = walker(&mut g, 0, "Liliana, Dreadhorde General");
    g.objects[lili.0 as usize].loyalty = 9;
    let a = put(&mut g, 1, "Savannah Lions");
    let b = put(&mut g, 1, "Serra Angel");
    let land = g.permanents(1).find(|&x| g.def_of(x).types.land).unwrap();
    act(&mut g, Action::Loyalty(lili, 2));
    // The opponent keeps the Angel (creature) and one land.
    resolve_all(&mut g, &[Action::Pick(b), Action::Pick(land)]);
    assert_eq!(g.obj(a).zone, Zone::Graveyard);
    assert_eq!(g.obj(b).zone, Zone::Battlefield);
    assert_eq!(g.permanents(1).count(), 2);
}

#[test]
fn kaito_grows_from_combat_damage_and_makes_ninjas() {
    let mut g = table();
    begin(&mut g);
    let kaito = walker(&mut g, 0, "Kaito, Cunning Infiltrator");
    let lions = put(&mut g, 0, "Savannah Lions");
    let junk = hand(&mut g, 0, "Forest");
    act(&mut g, Action::Loyalty(kaito, 0));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[Action::Discard(junk)]);
    assert!(g.keywords(lions).has(Keywords::UNBLOCKABLE));
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(lions));
    act(&mut g, Action::FinishAttacks);
    pass_to(&mut g, Step::CombatDamage);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(kaito).loyalty, 5, "3, +1, then +1 for the hit");
}

#[test]
fn vivien_emblem_pumps_your_creatures() {
    let mut g = table();
    begin(&mut g);
    let viv = walker(&mut g, 0, "Vivien Reid");
    g.objects[viv.0 as usize].loyalty = 8;
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::Loyalty(viv, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(lions), 4);
    let k = g.keywords(lions);
    assert!(k.has(Keywords::VIGILANCE) && k.has(Keywords::TRAMPLE) && k.has(Keywords::INDESTRUCTIBLE));
}
