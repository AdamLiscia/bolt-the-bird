//! Tier-3 cards, batch 3: life-gain payoffs, landfall, attack and death triggers, end-step and
//! combat-step triggers, threshold, and spell discounts.

mod common;

use bolt_core::{Action, Game, Keywords, Pending, Step, Target, Zone};
use common::*;

fn to_attack(g: &mut Game) {
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
}

/// Passes until `step`, declaring no attackers or blockers along the way.
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
fn sanguine_syphoner_drains_on_attack() {
    let mut g = table();
    let s = put(&mut g, 0, "Sanguine Syphoner");
    begin(&mut g);
    to_attack(&mut g);
    act(&mut g, Action::Attack(s));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[]);
    assert_eq!((g.players[0].life, g.players[1].life), (21, 19));
}

#[test]
fn ajanis_pridemate_and_blight_priest_count_every_life_gain() {
    let mut g = table();
    let mate = put(&mut g, 0, "Ajani's Pridemate");
    put(&mut g, 0, "Marauding Blight-Priest");
    let a = hand(&mut g, 0, "Joust Through");
    put(&mut g, 0, "Dazzling Angel");
    let lions = hand(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, lions, false, &[], &[]); // Dazzling Angel: gain 1
    assert_eq!(g.power(mate), 3);
    assert_eq!(g.players[1].life, 19);
    let _ = a;
}

#[test]
fn landfall_payoffs() {
    let mut g = lands(&[]);
    put(&mut g, 0, "Elfsworn Giant");
    put(&mut g, 0, "Tatyova, Benthic Druid");
    let forest = hand(&mut g, 0, "Forest");
    begin(&mut g);
    let before = g.players[0].hand.len();
    act(&mut g, Action::PlayLand(forest));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens_named(&g, 0, "Elf Warrior"), 1);
    assert_eq!(g.players[0].life, 21);
    assert_eq!(g.players[0].hand.len(), before, "played one, drew one");
}

#[test]
fn eager_trufflesnout_makes_food_on_a_hit() {
    let mut g = table();
    let boar = put(&mut g, 0, "Eager Trufflesnout");
    begin(&mut g);
    to_attack(&mut g);
    act(&mut g, Action::Attack(boar));
    act(&mut g, Action::FinishAttacks);
    pass_to(&mut g, Step::CombatDamage);
    resolve_all(&mut g, &[]);
    assert_eq!(g.permanents(0).filter(|&c| g.def_of(c).name == "Food").count(), 1);
}

#[test]
fn elvish_regrower_returns_any_permanent_card() {
    let mut g = table();
    let c = hand(&mut g, 0, "Elvish Regrower");
    let land = g.add_card(0, id("Forest"), Zone::Graveyard);
    let spell = g.add_card(0, id("Stab"), Zone::Graveyard);
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    let legal = g.legal_actions();
    assert!(
        legal.contains(&Action::Target(obj(land))),
        "lands are permanent cards"
    );
    assert!(!legal.contains(&Action::Target(obj(spell))));
    act(&mut g, Action::Target(obj(land)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(land).zone, Zone::Hand);
}

#[test]
fn erudite_wizard_grows_on_the_second_draw() {
    let mut g = table();
    let w = put(&mut g, 0, "Erudite Wizard");
    let sailor = put(&mut g, 0, "Spectral Sailor");
    begin(&mut g);
    act(&mut g, Action::Activate(sailor, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(w), 3);
}

#[test]
fn fanatical_firebrand_pings() {
    let mut g = table();
    let f = put(&mut g, 0, "Fanatical Firebrand");
    begin(&mut g);
    act(&mut g, Action::Activate(f, 0));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 19);
    assert_eq!(g.obj(f).zone, Zone::Graveyard);
}

#[test]
fn axgard_cavalry_grants_haste() {
    let mut g = table();
    let cav = put(&mut g, 0, "Axgard Cavalry");
    let fresh = hand(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, fresh, false, &[], &[]);
    act(&mut g, Action::Activate(cav, 0));
    act(&mut g, Action::Target(obj(fresh)));
    resolve_all(&mut g, &[]);
    to_attack(&mut g);
    assert!(g.legal_actions().contains(&Action::Attack(fresh)));
}

#[test]
fn sure_strike() {
    let mut g = table();
    let c = hand(&mut g, 0, "Sure Strike");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, c, false, &[obj(lions)], &[]);
    assert_eq!(g.power(lions), 5);
    assert!(g.keywords(lions).has(Keywords::FIRST_STRIKE));
}

#[test]
fn overrun_and_heroic_reinforcements() {
    let mut g = table();
    let c = hand(&mut g, 0, "Overrun");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.power(lions), 5);
    assert!(g.keywords(lions).has(Keywords::TRAMPLE));

    let mut g = table();
    let c = hand(&mut g, 0, "Heroic Reinforcements");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    let soldiers: Vec<_> = g
        .creatures(0)
        .filter(|&x| g.def_of(x).name == "Soldier")
        .collect();
    assert_eq!(soldiers.len(), 2);
    assert!(
        soldiers
            .iter()
            .all(|&s| g.power(s) == 2 && g.keywords(s).has(Keywords::HASTE)),
        "tokens too"
    );
}

#[test]
fn battlesong_berserker_triggers_once_per_attack() {
    let mut g = table();
    put(&mut g, 0, "Battlesong Berserker");
    let a = put(&mut g, 0, "Savannah Lions");
    let b = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    to_attack(&mut g);
    act(&mut g, Action::Attack(a));
    act(&mut g, Action::Attack(b));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[Action::Target(obj(a))]);
    assert_eq!((g.power(a), g.power(b)), (3, 2));
    assert!(g.keywords(a).has(Keywords::MENACE));
}

#[test]
fn youthful_valkyrie_likes_angels() {
    let mut g = table();
    let v = put(&mut g, 0, "Youthful Valkyrie");
    let lions = hand(&mut g, 0, "Savannah Lions");
    let angel = hand(&mut g, 0, "Serra Angel");
    begin(&mut g);
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(g.power(v), 1);
    play(&mut g, angel, false, &[], &[]);
    assert_eq!(g.power(v), 2);
}

#[test]
fn vengeful_bloodwitch_drains_on_any_of_your_creatures_dying() {
    let mut g = table();
    let witch = put(&mut g, 0, "Vengeful Bloodwitch");
    let lions = put(&mut g, 0, "Savannah Lions");
    let s1 = hand(&mut g, 0, "Stab");
    let s2 = hand(&mut g, 0, "Stab");
    begin(&mut g);
    play(
        &mut g,
        s1,
        false,
        &[obj(lions)],
        &[Action::Target(Target::Player(1))],
    );
    assert_eq!((g.players[0].life, g.players[1].life), (21, 19));
    play(
        &mut g,
        s2,
        false,
        &[obj(witch)],
        &[Action::Target(Target::Player(1))],
    );
    assert_eq!(
        (g.players[0].life, g.players[1].life),
        (22, 18),
        "its own death counts"
    );
}

#[test]
fn nessian_hornbeetle_needs_another_big_creature() {
    let mut g = table();
    let bug = put(&mut g, 0, "Nessian Hornbeetle");
    put(&mut g, 0, "Serra Angel");
    begin(&mut g);
    pass_to(&mut g, Step::BeginCombat);
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(bug), 3);
}

#[test]
fn brineborn_cutthroat_grows_on_the_opponents_turn() {
    let mut g = table();
    let cut = put(&mut g, 1, "Brineborn Cutthroat");
    let stab = hand(&mut g, 1, "Stab");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(cut), 3);
}

#[test]
fn firebrand_archer_pings_on_noncreature_spells() {
    let mut g = table();
    put(&mut g, 0, "Firebrand Archer");
    let c = hand(&mut g, 0, "Boltwave");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.players[1].life, 16, "3 from Boltwave, 1 from the Archer");
}

#[test]
fn empyrean_eagle_pumps_other_fliers() {
    let mut g = table();
    let eagle = put(&mut g, 0, "Empyrean Eagle");
    let hawk = put(&mut g, 0, "Healer's Hawk");
    let lions = put(&mut g, 0, "Savannah Lions");
    let theirs = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    assert_eq!(
        (g.power(eagle), g.power(hawk), g.power(lions), g.power(theirs)),
        (2, 2, 2, 4)
    );
}

#[test]
fn mocking_sprite_discounts_instants_and_sorceries() {
    let mut g = lands(&["Island"]);
    put(&mut g, 0, "Mocking Sprite");
    let think = hand(&mut g, 0, "Think Twice"); // {1}{U} -> {U}
    let lions = hand(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    assert!(g.legal_actions().contains(&Action::Cast(think)));
    let _ = lions;
}

#[test]
fn slumbering_cerberus_untaps_only_after_a_death() {
    let mut g = table();
    let dog = put(&mut g, 0, "Slumbering Cerberus");
    g.objects[dog.0 as usize].tapped = true;
    begin(&mut g);
    assert!(g.obj(dog).tapped, "doesn't untap in the untap step");
    pass_to(&mut g, Step::End);
    resolve_all(&mut g, &[]);
    assert!(g.obj(dog).tapped, "nothing died");

    let mut g = table();
    let dog = put(&mut g, 0, "Slumbering Cerberus");
    let lions = put(&mut g, 1, "Savannah Lions");
    let stab = hand(&mut g, 0, "Stab");
    g.objects[dog.0 as usize].tapped = true;
    begin(&mut g);
    play(&mut g, stab, false, &[obj(lions)], &[]);
    pass_to(&mut g, Step::End);
    resolve_all(&mut g, &[]);
    assert!(!g.obj(dog).tapped);
}

#[test]
fn needletooth_pack_is_morbid() {
    let mut g = table();
    let pack = put(&mut g, 0, "Needletooth Pack");
    let lions = put(&mut g, 1, "Savannah Lions");
    let stab = hand(&mut g, 0, "Stab");
    begin(&mut g);
    play(&mut g, stab, false, &[obj(lions)], &[]);
    pass_to(&mut g, Step::End);
    resolve_all(&mut g, &[Action::Target(obj(pack))]);
    assert_eq!(g.power(pack), 6);
}

#[test]
fn threshold_creatures() {
    let mut g = table();
    let c = hand(&mut g, 0, "Billowing Shriekmass");
    let bird = put(&mut g, 0, "Dreadwing Scavenger");
    for _ in 0..4 {
        g.add_card(0, id("Stab"), Zone::Graveyard);
    }
    begin(&mut g);
    assert_eq!(g.power(bird), 2, "four cards: no threshold");
    play(&mut g, c, false, &[], &[]); // mills three: seven
    assert_eq!(g.players[0].graveyard.len(), 7);
    assert_eq!((g.power(c), g.toughness(c)), (4, 4));
    assert_eq!(g.power(bird), 3);
    assert!(g.keywords(bird).has(Keywords::DEATHTOUCH));
}

#[test]
fn dreadwing_scavenger_loots_on_entering() {
    let mut g = table();
    let c = hand(&mut g, 0, "Dreadwing Scavenger");
    let junk = hand(&mut g, 0, "Forest");
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Discard(junk)]);
    assert_eq!(g.obj(junk).zone, Zone::Graveyard);
}
