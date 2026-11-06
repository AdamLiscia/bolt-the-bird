//! Planeswalkers (CR 306, 606): loyalty, loyalty abilities, damage, attacking them, emblems.

mod common;

use bolt_core::{Action, Game, Keywords, ObjId, Pending, Step, Target, Zone};
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

fn chandra(g: &mut Game) -> ObjId {
    walker(g, 0, "Chandra, Flameshaper")
}

#[test]
fn chandra_plus_two_adds_mana_that_empties_between_steps() {
    // CR 106.4, 500.4: mana in a pool empties at the end of each step and phase.
    let mut g = table();
    begin(&mut g);
    let c = chandra(&mut g);
    assert_eq!(g.obj(c).loyalty, 6);
    for _ in 0..3 {
        g.add_card(0, id("Aegis Turtle"), Zone::Library);
    }
    let bolt = g.add_card(0, id("Burst Lightning"), Zone::Library); // the top card
    act(&mut g, Action::Loyalty(c, 0));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Pick(bolt));
    assert_eq!(g.players[0].pool.len(), 3, "{{R}}{{R}}{{R}}");
    assert_eq!(g.obj(bolt).zone, Zone::Exile);
    // The pool pays first: casting the exiled Burst Lightning leaves every land as it was.
    let untapped = g.permanents(0).filter(|&x| !g.obj(x).tapped).count();
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 18);
    assert_eq!(g.players[0].pool.len(), 2);
    assert_eq!(g.permanents(0).filter(|&x| !g.obj(x).tapped).count(), untapped);
    pass_to(&mut g, Step::BeginCombat);
    assert!(g.players[0].pool.is_empty(), "emptied at the end of the step");
}

#[test]
fn chandra_plus_two_card_is_playable_only_this_turn() {
    let mut g = table();
    begin(&mut g);
    let c = chandra(&mut g);
    let raider = g.add_card(0, id("Gorehorn Raider"), Zone::Library);
    act(&mut g, Action::Loyalty(c, 0));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Pick(raider));
    assert!(g.legal_actions().contains(&Action::Cast(raider)));
    pass_to(&mut g, Step::End);
    pass_to(&mut g, Step::Main1); // the opponent's turn
    pass_to(&mut g, Step::End);
    pass_to(&mut g, Step::Main1); // our next turn
    assert_eq!(g.active, 0);
    assert_eq!(g.obj(raider).zone, Zone::Exile);
    assert!(!g.legal_actions().contains(&Action::Cast(raider)));
}

#[test]
fn chandra_plus_one_copies_a_creature_with_haste_until_the_end_step() {
    let mut g = table();
    begin(&mut g);
    let c = chandra(&mut g);
    let raider = put(&mut g, 0, "Gorehorn Raider");
    act(&mut g, Action::Loyalty(c, 1));
    act(&mut g, Action::Target(obj(raider)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).loyalty, 7);
    let copy = g
        .creatures(0)
        .find(|&x| x != raider && g.def_of(x).name == "Gorehorn Raider")
        .expect("the copy");
    assert!(g.keywords(copy).has(Keywords::HASTE));
    pass_to(&mut g, Step::Cleanup);
    assert_ne!(g.obj(copy).zone, Zone::Battlefield);
}

#[test]
fn chandra_minus_four_divides_eight_damage_as_she_activates() {
    // CR 601.2d: the division is chosen as the ability is put on the stack; each target gets at
    // least 1.
    let mut g = table();
    begin(&mut g);
    let c = chandra(&mut g);
    let turtle = put(&mut g, 1, "Aegis Turtle");
    let lions = put(&mut g, 1, "Savannah Lions");
    act(&mut g, Action::Loyalty(c, 2));
    act(&mut g, Action::Target(obj(turtle)));
    act(&mut g, Action::Target(obj(lions)));
    act(&mut g, Action::No); // no more targets
    assert!(matches!(g.pending, Pending::Divide(0)));
    let mut amounts: Vec<Action> = g.legal_actions();
    amounts.sort_by_key(|a| format!("{a:?}"));
    assert_eq!(
        amounts.len(),
        7,
        "1 to 7 for the turtle, the rest to the lions: {amounts:?}"
    );
    act(&mut g, Action::Amount(6));
    assert_eq!(g.obj(c).loyalty, 2, "costs paid after the division");
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(turtle).zone, Zone::Graveyard);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
}

#[test]
fn chandra_minus_four_single_target_takes_all_eight() {
    let mut g = table();
    begin(&mut g);
    let c = chandra(&mut g);
    let raider = put(&mut g, 1, "Gorehorn Raider");
    act(&mut g, Action::Loyalty(c, 2));
    act(&mut g, Action::Target(obj(raider)));
    act(&mut g, Action::No);
    assert!(
        matches!(g.pending, Pending::Priority(_)),
        "no division with one target"
    );
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert_eq!(g.obj(raider).zone, Zone::Graveyard);
}

#[test]
fn chandra_minus_four_damage_for_a_gone_target_is_not_dealt() {
    // CR 608.2b: an illegal target gets nothing; the others keep their share.
    let mut g = table();
    begin(&mut g);
    let c = chandra(&mut g);
    let lions = put(&mut g, 1, "Savannah Lions");
    let raider = put(&mut g, 1, "Gorehorn Raider");
    act(&mut g, Action::Loyalty(c, 2));
    act(&mut g, Action::Target(obj(lions)));
    act(&mut g, Action::Target(obj(raider)));
    act(&mut g, Action::No);
    act(&mut g, Action::Amount(5)); // 5 to the lions, 3 to the raider
    act(&mut g, Action::PassPriority);
    let bolt = hand(&mut g, 1, "Burst Lightning");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
    assert_eq!(g.obj(raider).zone, Zone::Battlefield);
    assert_eq!(g.obj(raider).damage, 3);
}
