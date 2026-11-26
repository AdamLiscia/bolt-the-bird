//! Lane D4 (the desktop): each script's behavior in a 1v1 Commander game on the CMD pool.
#![allow(dead_code, unused_imports)]

mod common;

use bolt_cards::set;
use bolt_core::{Action, Game, Keywords, ObjId, Pending, Step, Target, Zone};
use common::{act, obj, play, resolve_all};

fn cid(name: &str) -> usize {
    set("CMD")
        .unwrap()
        .index_of(name)
        .unwrap_or_else(|| panic!("{name} is not in the CMD pool"))
}

/// A Commander game on the CMD pool (99 Plains each), Bello vs. Ms. Bumbleflower, at player 0's first
/// main phase on turn 3, with 5 of each basic for both players.
fn table() -> Game {
    let plains = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        set("CMD").unwrap().defs.clone(),
        [&plains, &plains],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    for p in 0..2 {
        for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            for _ in 0..5 {
                put(&mut g, p, land);
            }
        }
    }
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g
}

fn put(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, cid(name), Zone::Battlefield)
}

fn hand(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, cid(name), Zone::Hand)
}

fn lands(g: &Game, p: usize) -> usize {
    g.permanents(p).filter(|&x| g.def_of(x).types.land).count()
}

// ---- Split cards ----

#[test]
fn split_cards_are_supported() {
    let s = set("CMD").unwrap();
    for name in ["Boom // Bust", "Fire // Ice"] {
        let i = cid(name);
        assert!(s.supported[i], "{name}");
        assert!(s.defs[i].subsystems.split_card, "{name}");
    }
}

#[test]
fn fire_divides_two_damage_between_two_creatures() {
    let mut g = table();
    let a = put(&mut g, 1, "Llanowar Elves");
    let b = put(&mut g, 1, "Llanowar Elves");
    let c = hand(&mut g, 0, "Fire // Ice");
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(a)));
    act(&mut g, Action::Target(obj(b)));
    assert!(matches!(g.pending, Pending::Divide(0)));
    act(&mut g, Action::Amount(1));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.obj(a).zone, g.obj(b).zone),
        (Zone::Graveyard, Zone::Graveyard),
        "1 damage to each 1/1"
    );
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
}

#[test]
fn ice_taps_a_permanent_and_draws_then_goes_to_the_graveyard() {
    let mut g = table();
    let theirs = g.permanents(1).next().unwrap();
    let c = hand(&mut g, 0, "Fire // Ice");
    assert!(g.legal_actions().contains(&Action::CastOmen(c)));
    let n = g.players[0].hand.len();
    act(&mut g, Action::CastOmen(c));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert!(g.obj(theirs).tapped);
    assert_eq!(g.players[0].hand.len(), n, "cast it, drew one");
    assert_eq!(
        g.obj(c).zone,
        Zone::Graveyard,
        "a split card goes to the graveyard, not the library"
    );
}

#[test]
fn boom_destroys_a_land_of_each_player() {
    let mut g = table();
    let mine = g.permanents(0).next().unwrap();
    let theirs = g.permanents(1).next().unwrap();
    let c = hand(&mut g, 0, "Boom // Bust");
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(mine)));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(mine).zone, Zone::Graveyard);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!((lands(&g, 0), lands(&g, 1)), (24, 24));
}

#[test]
fn bust_destroys_all_lands_for_its_own_cost() {
    let mut g = table();
    let c = hand(&mut g, 0, "Boom // Bust");
    act(&mut g, Action::CastOmen(c));
    resolve_all(&mut g, &[]);
    assert_eq!((lands(&g, 0), lands(&g, 1)), (0, 0));
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
    assert_eq!(
        g.def_of(c).omen.as_ref().unwrap().cost.mana_value(),
        6,
        "{{5}}{{R}}"
    );
}

#[test]
fn a_split_card_has_both_halves_mana_value_off_the_stack_and_pays_only_its_half() {
    let mut g = table();
    let c = hand(&mut g, 0, "Fire // Ice");
    assert_eq!(
        g.def_of(c).cost.mana_value(),
        4,
        "{{1}}{{R}} + {{1}}{{U}} (CR 709.4)"
    );
    assert_eq!(
        g.colors_of(c),
        [false, true, false, true, false],
        "both halves' colors in hand"
    );
    let untapped = |g: &Game| g.permanents(0).filter(|&x| !g.obj(x).tapped).count();
    let before = untapped(&g);
    act(&mut g, Action::CastOmen(c));
    let theirs = g.permanents(1).next().unwrap();
    act(&mut g, Action::Target(obj(theirs)));
    assert_eq!(
        g.colors_of(c),
        [false, true, false, false, false],
        "Ice on the stack is blue"
    );
    assert_eq!(before - untapped(&g), 2, "Ice's {{1}}{{U}}");
    resolve_all(&mut g, &[]);
    let c2 = hand(&mut g, 0, "Fire // Ice");
    let before = untapped(&g);
    act(&mut g, Action::Cast(c2));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::No); // one target
    assert_eq!(
        g.colors_of(c2),
        [false, false, false, true, false],
        "Fire on the stack is red"
    );
    assert_eq!(before - untapped(&g), 2, "Fire's {{1}}{{R}}, not both halves");
    resolve_all(&mut g, &[]);
}

#[test]
fn a_split_cards_right_half_is_cast_only_from_hand() {
    let mut g = table();
    let c = g.add_card(0, cid("Fire // Ice"), Zone::Graveyard);
    assert!(!g.legal_actions().contains(&Action::CastOmen(c)));
}

// ---- Dice ----

#[test]
fn ancient_gold_dragon_rolls_a_d20_for_faerie_dragons() {
    let mut g = table();
    assert!(set("CMD").unwrap().supported[cid("Ancient Gold Dragon")]);
    let d = put(&mut g, 0, "Ancient Gold Dragon");
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(d));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    let faeries: Vec<ObjId> = g
        .permanents(0)
        .filter(|&x| g.is_token(x) && g.def_of(x).name == "Faerie Dragon")
        .collect();
    assert!((1..=20).contains(&faeries.len()), "{}", faeries.len());
    let f = faeries[0];
    assert_eq!((g.power(f), g.toughness(f)), (1, 1));
    assert!(g.keywords(f).has(Keywords::FLYING));
    assert!(g.has_subtype(f, "Dragon"), "a Faerie Dragon is a Dragon");
    assert_eq!(g.colors_of(f), [false, true, false, false, false], "blue");
}

// ---- Dungeons ----

#[test]
fn hama_pashar_doubles_your_room_abilities() {
    let mut g = table();
    assert!(set("CMD").unwrap().supported[cid("Hama Pashar, Ruin Seeker")]);
    assert_eq!(g.room_trigger_count(0), 1);
    put(&mut g, 0, "Hama Pashar, Ruin Seeker");
    assert_eq!((g.room_trigger_count(0), g.room_trigger_count(1)), (2, 1));
}

#[test]
fn dungeon_delver_doubles_them_through_your_commander_creature() {
    let mut g = table();
    assert!(set("CMD").unwrap().supported[cid("Dungeon Delver")]);
    put(&mut g, 0, "Dungeon Delver");
    assert_eq!(g.room_trigger_count(0), 1);
    // A commander creature you own on the battlefield.
    g.commanders[0] = Some(put(&mut g, 0, "Bello, Bard of the Brambles"));
    assert_eq!(g.room_trigger_count(0), 2);
}

// ---- Choose a Background ----

#[test]
fn a_background_is_a_legal_second_commander_for_a_creature_that_chooses_one() {
    let s = set("CMD").unwrap();
    let deck = vec![cid("Swamp"); 98];
    let problems = bolt_cards::commander_deck_problems_for(
        s,
        &[cid("Safana, Calimport Cutthroat"), cid("Dungeon Delver")],
        &deck,
    );
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_background_needs_a_commander_that_chooses_one() {
    let s = set("CMD").unwrap();
    let deck = vec![cid("Island"); 98];
    let problems = bolt_cards::commander_deck_problems_for(
        s,
        &[cid("Hama Pashar, Ruin Seeker"), cid("Dungeon Delver")],
        &deck,
    );
    assert!(
        problems.iter().any(|p| p.contains("isn't a legendary creature")),
        "{problems:?}"
    );
    assert!(
        problems.iter().any(|p| p.contains("aren't partners")),
        "{problems:?}"
    );
}
