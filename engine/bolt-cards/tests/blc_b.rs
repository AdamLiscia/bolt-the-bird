//! Bloomburrow Commander (BLC), lane B: each script's behavior in a 1v1 Commander game.

mod common;

use bolt_cards::set;
use bolt_core::{Action, Game, Keywords, ObjId, Pending, Step, Target, Zone};
use common::{act, obj, play, resolve_all};

fn bid(name: &str) -> usize {
    set("BLC")
        .unwrap()
        .index_of(name)
        .unwrap_or_else(|| panic!("{name} is not in BLC"))
}

/// A Commander game (99 Plains each) with these commanders, at player 0's first main phase with
/// `lands` for player 0 and 5 of each basic for player 1.
fn game(commanders: [&str; 2], lands: &[&str]) -> Game {
    let plains = vec![bid("Plains"); 99];
    let mut g = Game::new_commander(
        set("BLC").unwrap().defs.clone(),
        [&plains, &plains],
        [bid(commanders[0]), bid(commanders[1])],
        7,
        0,
    );
    g.auto_pass = false;
    for &l in lands {
        put(&mut g, 0, l);
    }
    for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        for _ in 0..5 {
            put(&mut g, 1, land);
        }
    }
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g
}

/// Every basic five times for player 0, Bello vs. Ms. Bumbleflower.
fn table() -> Game {
    let mut lands = Vec::new();
    for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        lands.extend([land; 5]);
    }
    game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &lands)
}

fn put(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, bid(name), Zone::Battlefield)
}

fn hand(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, bid(name), Zone::Hand)
}

fn colors_of(g: &Game, id: ObjId) -> Vec<[bool; 5]> {
    g.mana_sources(0)
        .into_iter()
        .filter(|&(x, _)| x == id)
        .map(|(_, m)| m.colors)
        .collect()
}

#[test]
fn arcane_signet_and_command_tower_make_your_commanders_colors() {
    let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[]);
    let signet = put(&mut g, 0, "Arcane Signet");
    let tower = put(&mut g, 0, "Command Tower");
    let rg = [false, false, false, true, true];
    assert_eq!(colors_of(&g, signet), [rg]);
    assert_eq!(colors_of(&g, tower), [rg]);

    // No commander: no mana.
    let mut g = Game::empty(set("BLC").unwrap().defs.clone(), 1, 0);
    let tower = g.add_card(0, bid("Command Tower"), Zone::Battlefield);
    assert!(colors_of(&g, tower).is_empty());
}

#[test]
fn exotic_orchard_and_fellwar_stone_copy_opponents_land_colors() {
    let plains = vec![bid("Plains"); 99];
    let mut g = Game::new_commander(
        set("BLC").unwrap().defs.clone(),
        [&plains, &plains],
        [bid("Bello, Bard of the Brambles"), bid("Ms. Bumbleflower")],
        7,
        0,
    );
    let orchard = put(&mut g, 0, "Exotic Orchard");
    let stone = put(&mut g, 0, "Fellwar Stone");
    assert!(colors_of(&g, orchard).is_empty(), "the opponent has no lands");
    put(&mut g, 1, "Island");
    put(&mut g, 1, "Swamp");
    put(&mut g, 0, "Forest"); // yours don't count
    let ub = [false, true, true, false, false];
    assert_eq!(colors_of(&g, orchard), [ub]);
    assert_eq!(colors_of(&g, stone), [ub]);
}

#[test]
fn sol_ring_makes_two_colorless() {
    let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[]);
    let ring = put(&mut g, 0, "Sol Ring");
    let sources: Vec<_> = g
        .mana_sources(0)
        .into_iter()
        .filter(|&(x, _)| x == ring)
        .collect();
    assert_eq!(sources.len(), 2);
    assert!(sources.iter().all(|(_, m)| m.colorless));
    let signet = hand(&mut g, 0, "Arcane Signet"); // {2}
    act(&mut g, Action::Cast(signet));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.obj(signet).zone,
        Zone::Battlefield,
        "paid with one Sol Ring tap"
    );
    assert!(g.obj(ring).tapped);
}

#[test]
fn terramorphic_expanse_fetches_a_basic_tapped() {
    let mut g = table();
    let t = put(&mut g, 0, "Terramorphic Expanse");
    let basic = *g.players[0].library.last().unwrap();
    act(&mut g, Action::Activate(t, 0));
    resolve_all(&mut g, &[Action::Pick(basic)]);
    assert_eq!(g.obj(t).zone, Zone::Graveyard);
    assert_eq!(g.obj(basic).zone, Zone::Battlefield);
    assert!(g.obj(basic).tapped);
}

#[test]
fn mind_stone_taps_for_colorless_and_cashes_in_for_a_card() {
    let mut g = table();
    let stone = put(&mut g, 0, "Mind Stone");
    assert!(g.mana_sources(0).iter().any(|&(x, m)| x == stone && m.colorless));
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(stone, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(stone).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n + 1);
}

#[test]
fn glacial_fortress_enters_tapped_unless_you_have_a_plains_or_island() {
    for (have, tapped) in [(&[][..], true), (&["Island"][..], false)] {
        let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], have);
        let f = hand(&mut g, 0, "Glacial Fortress");
        act(&mut g, Action::PlayLand(f));
        assert_eq!(g.obj(f).tapped, tapped);
        assert_eq!(
            g.def_of(f).mana.unwrap().colors,
            [true, true, false, false, false]
        );
    }
}

#[test]
fn reliquary_tower_and_thought_vessel_lift_the_hand_size_limit() {
    for name in ["Reliquary Tower", "Thought Vessel"] {
        let mut g = table();
        let x = put(&mut g, 0, name);
        assert!(g.def_of(x).no_max_hand_size);
        assert!(g.mana_sources(0).iter().any(|&(y, m)| y == x && m.colorless));
        for _ in 0..10 {
            hand(&mut g, 0, "Plains");
        }
        // To the cleanup step: no discarding down to seven.
        while g.step != Step::End {
            let a = match g.pending {
                Pending::DeclareAttackers(_) => Action::FinishAttacks,
                _ => Action::PassPriority,
            };
            act(&mut g, a);
        }
        let n = g.players[0].hand.len();
        while g.turn == 3 {
            assert!(!matches!(g.pending, Pending::Discard(_)), "{name}");
            act(&mut g, Action::PassPriority);
        }
        assert_eq!(g.players[0].hand.len(), n);
    }
}

#[test]
fn temple_of_enlightenment_enters_tapped_and_scries() {
    let mut g = table();
    let t = hand(&mut g, 0, "Temple of Enlightenment");
    let top = *g.players[0].library.last().unwrap();
    act(&mut g, Action::PlayLand(t));
    resolve_all(&mut g, &[Action::Yes]);
    assert!(g.obj(t).tapped);
    assert_eq!(g.players[0].library[0], top, "scried to the bottom");
}

#[test]
fn cultivate_puts_one_basic_onto_the_battlefield_and_one_in_hand() {
    let mut g = table();
    let c = hand(&mut g, 0, "Cultivate");
    let lib = g.players[0].library.clone();
    let (a, b) = (lib[lib.len() - 1], lib[lib.len() - 2]);
    play(&mut g, c, false, &[], &[Action::Pick(a), Action::Pick(b)]);
    assert_eq!(g.obj(a).zone, Zone::Battlefield);
    assert!(g.obj(a).tapped);
    assert_eq!(g.obj(b).zone, Zone::Hand);
}

#[test]
fn farseek_finds_a_plains_island_swamp_or_mountain() {
    let mut g = table();
    let c = hand(&mut g, 0, "Farseek");
    let forest = g.add_card(0, bid("Forest"), Zone::Library);
    let island = g.add_card(0, bid("Island"), Zone::Library);
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(!g.legal_actions().contains(&Action::Pick(forest)), "not a Forest");
    resolve_all(&mut g, &[Action::Pick(island)]);
    assert_eq!(g.obj(island).zone, Zone::Battlefield);
    assert!(g.obj(island).tapped);
}

#[test]
fn ms_bumbleflower_gives_cards_and_counters_and_draws_two_on_the_second() {
    let mut g = table();
    let flower = put(&mut g, 0, "Ms. Bumbleflower");
    assert!(g.keywords(flower).has(Keywords::VIGILANCE));
    let spells: Vec<ObjId> = (0..3).map(|_| hand(&mut g, 0, "Arcane Signet")).collect();
    let mut mine = g.players[0].hand.len();
    for (i, &s) in spells.iter().enumerate() {
        let theirs = g.players[1].hand.len();
        act(&mut g, Action::Cast(s));
        resolve_all(
            &mut g,
            &[Action::Target(Target::Player(1)), Action::Target(obj(flower))],
        );
        mine -= 1;
        if i == 1 {
            mine += 2; // the second resolution this turn
        }
        assert_eq!(g.players[1].hand.len(), theirs + 1, "target opponent draws");
        assert_eq!(g.players[0].hand.len(), mine, "spell {i}");
        assert_eq!(g.obj(flower).counters, i as i32 + 1);
        assert!(g.keywords(flower).has(Keywords::FLYING));
    }
}

#[test]
fn bello_animates_your_big_artifacts_and_enchantments_on_your_turn() {
    let mut g = table();
    put(&mut g, 0, "Bello, Bard of the Brambles");
    let stash = put(&mut g, 0, "Bootleggers' Stash"); // an artifact, mana value 6
    let onslaught = put(&mut g, 0, "Berserkers' Onslaught"); // an enchantment, 5
    let signet = put(&mut g, 0, "Arcane Signet"); // mana value 2
    let boots = put(&mut g, 0, "Swiftfoot Boots"); // Equipment
    for x in [stash, onslaught] {
        assert!(g.types_of(x).creature && g.types_of(x).artifact == g.def_of(x).types.artifact);
        assert_eq!((g.power(x), g.toughness(x)), (4, 4));
        let k = g.keywords(x);
        assert!(k.has(Keywords::INDESTRUCTIBLE) && k.has(Keywords::HASTE));
        assert!(g.has_subtype(x, "Elemental"));
    }
    assert!(!g.types_of(signet).creature, "mana value 2");
    assert!(!g.types_of(boots).creature, "Equipment");
    // It attacks (haste) and draws on combat damage.
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(stash));
    act(&mut g, Action::FinishAttacks);
    let n = g.players[0].hand.len();
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[1].life, 36);
    assert_eq!(g.players[0].hand.len(), n + 1);
    // On the opponent's turn, just an artifact.
    g.start_turn(4, 1);
    assert!(!g.types_of(stash).creature);
}
