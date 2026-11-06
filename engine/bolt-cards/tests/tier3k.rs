//! The type layer (CR 205.1, 613.1d): Imprisoned in the Moon and Soulstone Sanctuary change what a
//! permanent is.

mod common;

use bolt_core::{Action, Game, Keywords, Pending, Step};
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

#[test]
fn imprisoned_creature_is_a_colorless_land_that_taps_for_colorless() {
    let mut g = table();
    begin(&mut g);
    let raider = put(&mut g, 1, "Gorehorn Raider");
    let moon = hand(&mut g, 0, "Imprisoned in the Moon");
    play(&mut g, moon, false, &[obj(raider)], &[]);
    let t = g.types_of(raider);
    assert!(t.land && !t.creature, "{t:?}");
    assert_eq!(g.colors_of(raider), [false; 5]);
    assert!(!g.creatures(1).any(|c| c == raider));
    assert!(g
        .mana_sources(1)
        .iter()
        .any(|&(id, m)| id == raider && m.colorless));
    // It can't attack: it isn't a creature.
    pass_to(&mut g, Step::End);
    pass_to(&mut g, Step::DeclareAttackers);
    assert_eq!(g.active, 1);
    assert!(!g.legal_actions().contains(&Action::Attack(raider)));
}

#[test]
fn an_imprisoned_planeswalker_is_no_longer_a_planeswalker() {
    let mut g = table();
    begin(&mut g);
    let ajani = hand(&mut g, 1, "Ajani, Caller of the Pride");
    g.start_turn(4, 1);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    play(&mut g, ajani, false, &[], &[]);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let moon = hand(&mut g, 0, "Imprisoned in the Moon");
    play(&mut g, moon, false, &[obj(ajani)], &[]);
    assert!(!g.types_of(ajani).planeswalker);
    assert_eq!(g.obj(ajani).zone, bolt_core::Zone::Battlefield);
}

#[test]
fn soulstone_sanctuary_becomes_a_three_three_creature_land_with_every_type() {
    let mut g = table();
    begin(&mut g);
    let ss = put(&mut g, 0, "Soulstone Sanctuary");
    assert!(!g.types_of(ss).creature);
    act(&mut g, Action::Activate(ss, 0));
    resolve_all(&mut g, &[]);
    let t = g.types_of(ss);
    assert!(t.creature && t.land);
    assert_eq!((g.power(ss), g.toughness(ss)), (3, 3));
    assert!(g.keywords(ss).has(Keywords::VIGILANCE));
    assert!(g.has_subtype(ss, "Cat") && g.has_subtype(ss, "Dragon"));
    assert!(!g.has_subtype(ss, "Forest"), "creature types only");
    // It stays a creature (no duration), and vigilance keeps it untapped as it attacks.
    pass_to(&mut g, Step::DeclareAttackers);
    act(&mut g, Action::Attack(ss));
    act(&mut g, Action::FinishAttacks);
    assert!(!g.obj(ss).tapped);
}

#[test]
fn witness_protection_makes_the_creature_green_and_white() {
    let mut g = table();
    begin(&mut g);
    let raider = put(&mut g, 1, "Gorehorn Raider"); // red
    let wp = hand(&mut g, 0, "Witness Protection");
    play(&mut g, wp, false, &[obj(raider)], &[]);
    assert_eq!(g.colors_of(raider), [true, false, false, false, true]);
    assert!(g.types_of(raider).creature);
}
