//! Tier-4 cards, batch 1: prevention, scaled statics, multi-mana sources, cost reductions, extra
//! turns, player hexproof, and counting effects.

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
fn fleeting_flight_prevents_combat_damage() {
    let mut g = table();
    let ff = hand(&mut g, 0, "Fleeting Flight");
    let lions = put(&mut g, 0, "Savannah Lions");
    let blocker = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    play(&mut g, ff, false, &[obj(lions)], &[]);
    assert!(g.keywords(lions).has(Keywords::FLYING));
    attack_with(&mut g, &[lions]);
    while g.pending != Pending::DeclareBlockers(1) {
        act(&mut g, Action::PassPriority);
    }
    act(
        &mut g,
        Action::Block {
            blocker,
            attacker: lions,
        },
    );
    act(&mut g, Action::FinishBlocks);
    pass_to(&mut g, Step::EndCombat);
    assert_eq!(
        g.obj(lions).zone,
        Zone::Battlefield,
        "the Angel's damage was prevented"
    );
}

#[test]
fn tragic_banshee_is_much_worse_after_a_death() {
    let mut g = table();
    let b = hand(&mut g, 0, "Tragic Banshee");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    play(&mut g, b, false, &[], &[Action::Target(obj(angel))]);
    assert_eq!(g.power(angel), 3);

    let mut g = table();
    let b = hand(&mut g, 0, "Tragic Banshee");
    let stab = hand(&mut g, 0, "Stab");
    let angel = put(&mut g, 1, "Serra Angel");
    let lions = put(&mut g, 1, "Savannah Lions");
    begin(&mut g);
    play(&mut g, stab, false, &[obj(lions)], &[]);
    play(&mut g, b, false, &[], &[Action::Target(obj(angel))]);
    assert_eq!(g.obj(angel).zone, Zone::Graveyard);
}

#[test]
fn tolarian_terror_gets_cheaper() {
    let mut g = lands(&["Island", "Island"]);
    let t = hand(&mut g, 0, "Tolarian Terror"); // {6}{U}
    for _ in 0..5 {
        g.add_card(0, id("Stab"), Zone::Graveyard);
    }
    begin(&mut g);
    assert!(
        g.legal_actions().contains(&Action::Cast(t)),
        "five spells in the graveyard: {{1}}{{U}}"
    );
}

#[test]
fn dwynen_and_archdruid_lead_the_elves() {
    let mut g = lands(&[]);
    let dwynen = put(&mut g, 0, "Dwynen, Gilt-Leaf Daen");
    let druid = put(&mut g, 0, "Elvish Archdruid");
    let elf = put(&mut g, 0, "Llanowar Elves");
    begin(&mut g);
    assert_eq!(g.power(elf), 3, "+1/+1 from each lord");
    assert_eq!(g.power(dwynen), 4, "a 3/4: the Archdruid pumps Dwynen");
    let ceratops = hand(&mut g, 0, "Quakestrider Ceratops"); // {3}{G}{G}{G}: six mana
    assert!(
        !g.legal_actions().contains(&Action::Cast(ceratops)),
        "the Archdruid makes {{G}} per Elf (3) plus the Llanowar Elves' {{G}}: four"
    );
    attack_with(&mut g, &[dwynen, elf]);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 22, "two attacking Elves");
    let _ = druid;
}

#[test]
fn krenko_counts_goblins() {
    let mut g = table();
    let k = put(&mut g, 0, "Krenko, Mob Boss");
    put(&mut g, 0, "Courageous Goblin");
    begin(&mut g);
    act(&mut g, Action::Activate(k, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens_named(&g, 0, "Goblin"), 2);
}

#[test]
fn revenge_of_the_rats_makes_tapped_rats() {
    let mut g = table();
    let c = hand(&mut g, 0, "Revenge of the Rats");
    for _ in 0..3 {
        g.add_card(0, id("Savannah Lions"), Zone::Graveyard);
    }
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    let rats: Vec<ObjId> = g.creatures(0).filter(|&c| g.def_of(c).name == "Rat").collect();
    assert_eq!(rats.len(), 3);
    assert!(rats.iter().all(|&r| g.obj(r).tapped));
}

#[test]
fn lunar_insight_and_inspiring_call() {
    let mut g = table();
    let c = hand(&mut g, 0, "Lunar Insight");
    put(&mut g, 0, "Savannah Lions"); // 1
    put(&mut g, 0, "Serra Angel"); // 5
    put(&mut g, 0, "Healer's Hawk"); // 1
    begin(&mut g);
    let before = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[]);
    assert_eq!(
        g.players[0].hand.len(),
        before - 1 + 2,
        "two different mana values"
    );

    let mut g = table();
    let c = hand(&mut g, 0, "Inspiring Call");
    let a = put(&mut g, 0, "Savannah Lions");
    put(&mut g, 0, "Serra Angel");
    g.objects[a.0 as usize].counters = 1;
    begin(&mut g);
    let before = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), before);
    assert!(g.keywords(a).has(Keywords::INDESTRUCTIBLE));
}

#[test]
fn midnight_snack_cashes_in_life_gained() {
    let mut g = table();
    let snack = put(&mut g, 0, "Midnight Snack");
    put(&mut g, 0, "Dazzling Angel");
    let a = hand(&mut g, 0, "Savannah Lions");
    let b = hand(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, a, false, &[], &[]);
    play(&mut g, b, false, &[], &[]);
    act(&mut g, Action::Activate(snack, 0));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 18, "gained 2 this turn");
}

#[test]
fn crystal_barricade_protects_you_and_your_creatures() {
    let mut g = table();
    put(&mut g, 1, "Crystal Barricade");
    let lions = put(&mut g, 1, "Savannah Lions");
    let hidetsugu = hand(&mut g, 0, "Hidetsugu's Second Rite");
    let storm = hand(&mut g, 0, "Slagstorm");
    begin(&mut g);
    act(&mut g, Action::Cast(hidetsugu));
    assert!(
        !g.legal_actions().contains(&Action::Target(Target::Player(1))),
        "you have hexproof"
    );
    act(&mut g, Action::Target(Target::Player(0)));
    resolve_all(&mut g, &[]);
    act(&mut g, Action::CastModal(storm, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield, "noncombat damage prevented");
}

#[test]
fn blanchwood_armor_counts_forests() {
    let mut g = lands(&["Forest", "Forest", "Forest"]);
    let armor = hand(&mut g, 0, "Blanchwood Armor");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, armor, false, &[obj(lions)], &[]);
    assert_eq!((g.power(lions), g.toughness(lions)), (5, 4));
}

#[test]
fn brasss_bounty_and_temporal_manipulation() {
    let mut g = lands(&["Mountain"; 7]);
    let c = hand(&mut g, 0, "Brass's Bounty");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(
        g.permanents(0)
            .filter(|&x| g.def_of(x).name == "Treasure")
            .count(),
        7
    );

    let mut g = table();
    let c = hand(&mut g, 0, "Temporal Manipulation");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    pass_to(&mut g, Step::End);
    while g.turn == 3 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!((g.turn, g.active), (4, 0), "player 0 takes the next turn too");
}

#[test]
fn ghalta_and_embercleave_get_cheaper() {
    let mut g = lands(&["Forest", "Forest"]);
    let ghalta = hand(&mut g, 0, "Ghalta, Primal Hunger");
    put(&mut g, 0, "Quakestrider Ceratops"); // 12 power
    begin(&mut g);
    assert!(
        g.legal_actions().contains(&Action::Cast(ghalta)),
        "{{10}}{{G}}{{G}} minus 12"
    );

    let mut g = lands(&["Mountain", "Mountain", "Mountain"]);
    let cleave = hand(&mut g, 0, "Embercleave"); // {4}{R}{R}
    let a = put(&mut g, 0, "Savannah Lions");
    let b = put(&mut g, 0, "Savannah Lions");
    let c = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    attack_with(&mut g, &[a, b, c]);
    act(&mut g, Action::Cast(cleave));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    resolve_all(&mut g, &[Action::Target(obj(a))]);
    assert_eq!(g.power(a), 3);
    assert!(g.keywords(a).has(Keywords::DOUBLE_STRIKE));
}

#[test]
fn ashroot_animist_shares_its_power() {
    let mut g = table();
    let animist = put(&mut g, 0, "Ashroot Animist");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    attack_with(&mut g, &[animist]);
    resolve_all(&mut g, &[Action::Target(obj(lions))]);
    assert_eq!(g.power(lions), 6);
    assert!(g.keywords(lions).has(Keywords::TRAMPLE));
}
