//! Tier-3 cards, batch 5: anthems, mass effects, must-attack, enter-tapped for opponents,
//! can't-lose, counter doubling.

mod common;

use bolt_core::{Action, Game, Keywords, ObjId, Outcome, Pending, Step, Zone};
use common::*;

fn to_attackers(g: &mut Game) {
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
}

fn attack_with(g: &mut Game, attackers: &[ObjId]) {
    to_attackers(g);
    for &a in attackers {
        act(g, Action::Attack(a));
    }
    act(g, Action::FinishAttacks);
}

#[test]
fn anthem_of_champions() {
    let mut g = table();
    put(&mut g, 0, "Anthem of Champions");
    let lions = put(&mut g, 0, "Savannah Lions");
    let theirs = put(&mut g, 1, "Savannah Lions");
    begin(&mut g);
    assert_eq!((g.power(lions), g.toughness(lions), g.power(theirs)), (3, 2, 2));
}

#[test]
fn crackling_cyclops_and_spitfire_lagac() {
    let mut g = table();
    let cy = put(&mut g, 0, "Crackling Cyclops");
    let c = hand(&mut g, 0, "Boltwave");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.power(cy), 3);

    let mut g = lands(&[]);
    put(&mut g, 0, "Spitfire Lagac");
    let f = hand(&mut g, 0, "Forest");
    begin(&mut g);
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 19);
}

#[test]
fn preposterous_proportions() {
    let mut g = table();
    let c = hand(&mut g, 0, "Preposterous Proportions");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.power(lions), 12);
    assert!(g.keywords(lions).has(Keywords::VIGILANCE));
}

#[test]
fn goblin_bushwhacker_kicked() {
    let mut g = table();
    let c = hand(&mut g, 0, "Goblin Bushwhacker");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, c, true, &[], &[]);
    assert_eq!((g.power(lions), g.power(c)), (3, 2));
    assert!(g.keywords(c).has(Keywords::HASTE));
}

#[test]
fn burnished_hart_finds_up_to_two_basics() {
    let mut g = table();
    let hart = put(&mut g, 0, "Burnished Hart");
    let a = library_card(&mut g, 0, "Forest");
    let b = library_card(&mut g, 0, "Island");
    begin(&mut g);
    act(&mut g, Action::Activate(hart, 0));
    resolve_all(&mut g, &[Action::Pick(a), Action::Pick(b)]);
    assert!(g.obj(a).tapped && g.obj(b).tapped);
    assert_eq!(g.obj(hart).zone, Zone::Graveyard);
}

#[test]
fn rite_of_the_dragoncaller() {
    let mut g = table();
    put(&mut g, 0, "Rite of the Dragoncaller");
    let c = hand(&mut g, 0, "Boltwave");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    let dragon = g
        .creatures(0)
        .find(|&x| g.def_of(x).name == "Dragon")
        .expect("a Dragon");
    assert_eq!(g.power(dragon), 5);
}

#[test]
fn crypt_feaster_needs_threshold() {
    let mut g = table();
    let f = put(&mut g, 0, "Crypt Feaster");
    for _ in 0..7 {
        g.add_card(0, id("Stab"), Zone::Graveyard);
    }
    begin(&mut g);
    attack_with(&mut g, &[f]);
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(f), 5);
}

#[test]
fn paradise_druid_has_hexproof_while_untapped() {
    let mut g = table();
    let druid = put(&mut g, 0, "Paradise Druid");
    begin(&mut g);
    assert!(g.keywords(druid).has(Keywords::HEXPROOF));
    g.objects[druid.0 as usize].tapped = true;
    assert!(!g.keywords(druid).has(Keywords::HEXPROOF));
}

#[test]
fn condemn_and_aetherize_answer_attackers() {
    let mut g = table();
    let condemn = hand(&mut g, 1, "Condemn");
    let angel = put(&mut g, 0, "Serra Angel");
    begin(&mut g);
    attack_with(&mut g, &[angel]);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(condemn));
    act(&mut g, Action::Target(obj(angel)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].library[0], angel, "the bottom");
    assert_eq!(g.players[0].life, 24, "its controller gains its toughness");

    let mut g = table();
    let ae = hand(&mut g, 1, "Aetherize");
    let a = put(&mut g, 0, "Savannah Lions");
    let b = put(&mut g, 0, "Serra Angel");
    begin(&mut g);
    attack_with(&mut g, &[a, b]);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(ae));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(a).zone, g.obj(b).zone), (Zone::Hand, Zone::Hand));
}

#[test]
fn juggernaut_must_attack_and_dodges_walls() {
    let mut g = table();
    let j = put(&mut g, 0, "Juggernaut");
    put(&mut g, 1, "Rune-Sealed Wall");
    begin(&mut g);
    to_attackers(&mut g);
    assert!(
        !g.legal_actions().contains(&Action::FinishAttacks),
        "it attacks each combat if able"
    );
    act(&mut g, Action::Attack(j));
    act(&mut g, Action::FinishAttacks);
    assert_ne!(g.pending, Pending::DeclareBlockers(1), "a Wall can't block it");
}

#[test]
fn mass_reanimation() {
    let mut g = table();
    let c = hand(&mut g, 0, "Raise the Past");
    let cheap = g.add_card(0, id("Savannah Lions"), Zone::Graveyard);
    let big = g.add_card(0, id("Serra Angel"), Zone::Graveyard);
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(
        (g.obj(cheap).zone, g.obj(big).zone),
        (Zone::Battlefield, Zone::Graveyard)
    );

    let mut g = table();
    let c = hand(&mut g, 0, "Rise of the Dark Realms");
    let mine = g.add_card(0, id("Savannah Lions"), Zone::Graveyard);
    let theirs = g.add_card(1, id("Serra Angel"), Zone::Graveyard);
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Battlefield);
    assert_eq!(
        (g.obj(mine).controller, g.obj(theirs).controller),
        (0, 0),
        "under your control"
    );
}

#[test]
fn authority_of_the_consuls() {
    let mut g = table();
    put(&mut g, 0, "Authority of the Consuls");
    let theirs = hand(&mut g, 1, "Savannah Lions");
    let mine = hand(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, mine, false, &[], &[]);
    assert!(!g.obj(mine).tapped);
    while g.active == 0 {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    play(&mut g, theirs, false, &[], &[]);
    assert!(g.obj(theirs).tapped);
    assert_eq!(g.players[0].life, 21);
}

#[test]
fn mossborn_hydra_doubles() {
    let mut g = lands(&["Forest", "Forest", "Forest"]);
    let hydra = hand(&mut g, 0, "Mossborn Hydra");
    let land = hand(&mut g, 0, "Forest");
    begin(&mut g);
    play(&mut g, hydra, false, &[], &[]);
    assert_eq!(g.power(hydra), 1);
    act(&mut g, Action::PlayLand(land));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(hydra), 2);
}

#[test]
fn herald_of_eternal_dawn_keeps_you_alive() {
    let mut g = table();
    put(&mut g, 1, "Herald of Eternal Dawn");
    g.players[1].life = 2;
    let c = hand(&mut g, 0, "Boltwave");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.players[1].life, -1);
    assert_eq!(g.result, None);
    assert_ne!(g.result, Some(Outcome::Winner(0)));
}

#[test]
fn bulk_up_doubles_power() {
    let mut g = table();
    let c = hand(&mut g, 0, "Bulk Up");
    let angel = put(&mut g, 0, "Serra Angel");
    begin(&mut g);
    play(&mut g, c, false, &[obj(angel)], &[]);
    assert_eq!(g.power(angel), 8);
}
