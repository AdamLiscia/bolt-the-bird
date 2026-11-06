//! Tier 4, batch 8: Bloom Tender, Giada, Banner of Kinship.

mod common;

use bolt_core::{Action, Pending};
use common::*;

#[test]
fn bloom_tender_adds_one_mana_of_each_color_among_your_permanents() {
    let mut g = lands(&[]);
    begin(&mut g);
    put(&mut g, 0, "Bloom Tender"); // green
    put(&mut g, 0, "Savannah Lions"); // white
    let vindicator = hand(&mut g, 0, "Swiftblade Vindicator"); // {R}{W}
    assert!(
        !g.legal_actions().contains(&Action::Cast(vindicator)),
        "no red yet"
    );
    put(&mut g, 0, "Gorehorn Raider"); // red
    assert!(g.legal_actions().contains(&Action::Cast(vindicator)));
    play(&mut g, vindicator, false, &[], &[]);
    assert_eq!(g.players[0].pool.len(), 0);
}

#[test]
fn giadas_mana_only_casts_angels() {
    let mut g = lands(&["Mountain"]);
    begin(&mut g);
    put(&mut g, 0, "Giada, Font of Hope");
    let lions = hand(&mut g, 0, "Savannah Lions"); // {W}
    let valkyrie = hand(&mut g, 0, "Youthful Valkyrie"); // {1}{W}, an Angel
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Cast(lions)));
    assert!(legal.contains(&Action::Cast(valkyrie)));
}

#[test]
fn giada_gives_entering_angels_a_counter_per_angel_you_already_control() {
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Giada, Font of Hope");
    put(&mut g, 0, "Serra Angel");
    let angel = hand(&mut g, 0, "Dazzling Angel");
    play(&mut g, angel, false, &[], &[]);
    assert_eq!(g.obj(angel).counters, 2);
    let lions = hand(&mut g, 0, "Savannah Lions");
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(g.obj(lions).counters, 0, "not an Angel");
}

#[test]
fn banner_of_kinship_counts_the_chosen_type_and_pumps_it() {
    let mut g = table();
    begin(&mut g);
    let a = put(&mut g, 0, "Savannah Lions"); // Cat
    let b = put(&mut g, 0, "Savannah Lions");
    let raider = put(&mut g, 0, "Gorehorn Raider"); // Minotaur Pirate
    let banner = hand(&mut g, 0, "Banner of Kinship");
    act(&mut g, Action::Cast(banner));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert!(matches!(g.pending, Pending::Decide(0)));
    let types = g.creature_type_options();
    let cat = types.iter().position(|t| t == "Cat").expect("Cat offered") as u8;
    assert!(types.iter().any(|t| t == "Minotaur"));
    act(&mut g, Action::Mode(cat));
    assert_eq!(g.obj(banner).fellowship, 2);
    assert_eq!((g.power(a), g.toughness(b)), (4, 3));
    assert_eq!(g.power(raider), 4, "not a Cat");
    // A Cat that enters later gets the bonus too; the counters stay at two.
    let c = put(&mut g, 0, "Savannah Lions");
    assert_eq!(g.power(c), 4);
}
