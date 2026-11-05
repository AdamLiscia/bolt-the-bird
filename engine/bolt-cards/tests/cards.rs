//! One rules test per supported card, written from its printed text.

use bolt_cards::{fdn, index_of};
use bolt_core::{CardDef, Keywords};

fn card(name: &str) -> CardDef {
    fdn()[index_of(name).unwrap_or_else(|| panic!("{name} is in FDN"))].clone()
}

fn check(name: &str, cost_mv: u32, pt: (i32, i32), keywords: &[Keywords]) {
    let c = card(name);
    assert!(c.types.creature, "{name} is a creature");
    assert_eq!(c.cost.mana_value(), cost_mv, "{name} mana value");
    assert_eq!((c.power, c.toughness), pt, "{name} P/T");
    let mut expected = Keywords::NONE;
    for &k in keywords {
        expected = expected.with(k);
    }
    assert_eq!(c.keywords, expected, "{name} keywords");
}

#[test]
fn aegis_turtle() {
    check("Aegis Turtle", 1, (0, 5), &[]);
}

#[test]
fn brazen_scourge() {
    check("Brazen Scourge", 3, (3, 3), &[Keywords::HASTE]);
}

#[test]
fn healers_hawk() {
    check(
        "Healer's Hawk",
        1,
        (1, 1),
        &[Keywords::FLYING, Keywords::LIFELINK],
    );
}

#[test]
fn quakestrider_ceratops() {
    check("Quakestrider Ceratops", 6, (12, 8), &[]);
}

#[test]
fn savannah_lions() {
    check("Savannah Lions", 1, (2, 1), &[]);
}

#[test]
fn serra_angel() {
    check("Serra Angel", 5, (4, 4), &[Keywords::FLYING, Keywords::VIGILANCE]);
}

#[test]
fn swiftblade_vindicator() {
    check(
        "Swiftblade Vindicator",
        2,
        (1, 1),
        &[Keywords::DOUBLE_STRIKE, Keywords::VIGILANCE, Keywords::TRAMPLE],
    );
}

#[test]
fn vampire_nighthawk() {
    check(
        "Vampire Nighthawk",
        3,
        (2, 3),
        &[Keywords::FLYING, Keywords::DEATHTOUCH, Keywords::LIFELINK],
    );
}

#[test]
fn basic_lands_tap_for_their_color() {
    for (name, color) in [
        ("Plains", 0),
        ("Island", 1),
        ("Swamp", 2),
        ("Mountain", 3),
        ("Forest", 4),
    ] {
        let c = card(name);
        assert!(c.types.basic && c.types.land);
        let m = c.mana.expect("a mana source");
        assert!(
            m.colors[color] && m.colors.iter().filter(|&&b| b).count() == 1,
            "{name}"
        );
    }
}

// ---------------------------------------------------------------- the cards in play

use bolt_core::fuzz::random_game;
use bolt_core::rng::Rng;
use bolt_core::{Action, Game, Pending, Zone};

fn board() -> Game {
    let mut g = Game::empty(fdn(), 1, 0);
    let turtle = index_of("Aegis Turtle").unwrap(); // uncastable: no lands
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, turtle, Zone::Library);
        }
    }
    g
}

fn act(g: &mut Game, a: Action) {
    g.apply(a)
        .unwrap_or_else(|e| panic!("{e}; legal: {:?}", g.legal_actions()));
}

#[test]
fn vampire_nighthawk_blocks_serra_angel() {
    // Deathtouch kills the 4/4 angel; lifelink gains 2 for the Nighthawk's controller.
    let mut g = board();
    let angel = g.add_card(0, index_of("Serra Angel").unwrap(), Zone::Battlefield);
    let hawk = g.add_card(1, index_of("Vampire Nighthawk").unwrap(), Zone::Battlefield);
    g.start_turn(3, 0);
    act(&mut g, Action::Attack(angel));
    assert!(!g.obj(angel).tapped, "vigilance");
    act(&mut g, Action::FinishAttacks);
    act(
        &mut g,
        Action::Block {
            blocker: hawk,
            attacker: angel,
        },
    );
    act(&mut g, Action::FinishBlocks);
    assert_eq!(g.obj(angel).zone, Zone::Graveyard);
    assert_eq!(g.obj(hawk).zone, Zone::Graveyard, "4 damage kills the 2/3");
    assert_eq!(g.players[1].life, 22);
}

#[test]
fn swiftblade_vindicator_tramples_over_on_the_second_strike() {
    // First strike: 1 damage kills Savannah Lions. Regular damage: the blocker is gone, so trample
    // sends all of it to the player (CR 702.19e).
    let mut g = board();
    let vin = g.add_card(0, index_of("Swiftblade Vindicator").unwrap(), Zone::Battlefield);
    let lions = g.add_card(1, index_of("Savannah Lions").unwrap(), Zone::Battlefield);
    g.start_turn(3, 0);
    act(&mut g, Action::Attack(vin));
    act(&mut g, Action::FinishAttacks);
    act(
        &mut g,
        Action::Block {
            blocker: lions,
            attacker: vin,
        },
    );
    act(&mut g, Action::FinishBlocks);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
    assert_eq!(
        g.obj(vin).zone,
        Zone::Battlefield,
        "the lions died before dealing damage"
    );
    assert_eq!(g.players[1].life, 19);
}

#[test]
fn brazen_scourge_attacks_the_turn_it_arrives() {
    let mut g = board();
    g.auto_pass = false;
    let mountain = index_of("Mountain").unwrap();
    for _ in 0..3 {
        g.add_card(0, mountain, Zone::Battlefield);
    }
    let scourge = g.add_card(0, index_of("Brazen Scourge").unwrap(), Zone::Hand);
    g.start_turn(3, 0);
    while !g.legal_actions().contains(&Action::Cast(scourge)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Cast(scourge));
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    assert!(g.legal_actions().contains(&Action::Attack(scourge)));
}

#[test]
fn random_games_with_supported_fdn_cards() {
    let defs = fdn();
    let mask = bolt_cards::supported_mask();
    let lands: Vec<usize> = (0..defs.len())
        .filter(|&i| mask[i] && defs[i].types.land)
        .collect();
    let spells: Vec<usize> = (0..defs.len())
        .filter(|&i| mask[i] && !defs[i].types.land)
        .collect();
    for seed in 0..300 {
        let mut rng = Rng::new(seed);
        let a = bolt_core::fuzz::random_deck(&lands, &spells, &mut rng);
        let b = bolt_core::fuzz::random_deck(&lands, &spells, &mut rng);
        random_game(defs.clone(), [&a, &b], seed, true).unwrap_or_else(|e| panic!("{e}"));
    }
}
