//! The merger's pieces done by lane B: type-changing land statics (Blood Moon, Urborg), haste for
//! creatures' abilities (Thousand-Year Elixir), and ward—sacrifice nonland permanents (Valgavoth).
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
/// main phase on turn 3, with player 0's `lands` and 5 of each basic for player 1.
fn game(lands: &[&str]) -> Game {
    let plains = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        set("CMD").unwrap().defs.clone(),
        [&plains, &plains],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
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

/// Every basic five times for player 0.
fn table() -> Game {
    let mut lands = Vec::new();
    for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        lands.extend([land; 5]);
    }
    game(&lands)
}

fn put(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, cid(name), Zone::Battlefield)
}

fn hand(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, cid(name), Zone::Hand)
}

fn tokens(g: &Game, p: usize, name: &str) -> Vec<ObjId> {
    g.permanents(p)
        .filter(|&x| g.is_token(x) && g.def_of(x).name == name)
        .collect()
}

fn colors(g: &Game, id: ObjId) -> Vec<[bool; 5]> {
    g.mana_sources(g.obj(id).controller)
        .into_iter()
        .filter(|&(x, _)| x == id)
        .map(|(_, m)| m.colors)
        .collect()
}

#[test]
fn blood_moon_makes_nonbasic_lands_mountains() {
    let mut g = game(&[]);
    let caves = put(&mut g, 0, "Caves of Koilos");
    let forest = put(&mut g, 0, "Forest");
    let temple = put(&mut g, 0, "Temple Garden");
    assert!(g.has_subtype(temple, "Forest"));
    put(&mut g, 1, "Blood Moon");
    assert_eq!(
        colors(&g, caves),
        [[false, false, false, true, false]],
        "only red"
    );
    assert!(g.has_subtype(temple, "Mountain") && !g.has_subtype(temple, "Forest"));
    assert_eq!(
        colors(&g, forest),
        [[false, false, false, false, true]],
        "basics keep theirs"
    );
}

#[test]
fn urborg_makes_every_land_a_swamp() {
    let mut g = game(&[]);
    let forest = put(&mut g, 0, "Forest");
    let theirs = put(&mut g, 1, "Plains");
    put(&mut g, 0, "Urborg, Tomb of Yawgmoth");
    assert!(g.has_subtype(forest, "Swamp") && g.has_subtype(forest, "Forest"));
    assert!(g.has_subtype(theirs, "Swamp"));
    assert_eq!(colors(&g, forest), [[false, false, true, false, true]]);
    // Blood Moon turns Urborg itself into a Mountain without its ability.
    let urborg = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Urborg, Tomb of Yawgmoth")
        .unwrap();
    put(&mut g, 1, "Blood Moon");
    assert!(!g.has_subtype(forest, "Swamp"));
    assert_eq!(colors(&g, urborg), [[false, false, false, true, false]]);
}

#[test]
fn thousand_year_elixir_lets_new_creatures_tap() {
    let mut g = game(&["Forest"]);
    let elves = put(&mut g, 0, "Llanowar Elves");
    g.objects[elves.0 as usize].since_turn = g.turn; // just arrived
    assert!(colors(&g, elves).is_empty(), "summoning sick");
    put(&mut g, 0, "Thousand-Year Elixir");
    assert_eq!(colors(&g, elves).len(), 1);
}

#[test]
fn ward_sacrifice_nonland_permanents() {
    // Valgavoth's ward on a test definition (Savannah Lions: "Ward—Sacrifice three nonland permanents.").
    let mut defs = (*set("CMD").unwrap().defs).clone();
    let lions = cid("Savannah Lions");
    defs[lions].ward_sacrifice = 3;
    defs[lions].ward_sacrifice_nonland = true;
    let plains = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        std::sync::Arc::new(defs),
        [&plains, &plains],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        for _ in 0..5 {
            put(&mut g, 0, land);
            put(&mut g, 1, land);
        }
    }
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let warded = put(&mut g, 1, "Savannah Lions");
    // Player 0 controls only two nonland permanents: the ward can't be paid, so the spell is countered.
    put(&mut g, 0, "Sol Ring");
    put(&mut g, 0, "Elvish Mystic");
    let bolt = hand(&mut g, 0, "Burst Lightning");
    play(&mut g, bolt, false, &[Target::Object(warded)], &[]);
    assert_eq!(g.obj(warded).zone, Zone::Battlefield);
    assert_eq!(g.obj(bolt).zone, Zone::Graveyard);
    // With three, the lands aren't offered.
    let third = put(&mut g, 0, "Llanowar Elves");
    let bolt = hand(&mut g, 0, "Burst Lightning");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Object(warded)));
    for _ in 0..10 {
        if matches!(g.pending, Pending::Decide(0)) {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(third)));
    assert!(!legal
        .iter()
        .any(|a| matches!(a, Action::Pick(x) if g.types_of(*x).land)));
}
