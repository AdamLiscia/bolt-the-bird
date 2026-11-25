//! Playgroup decks, lane C: each script's behavior in a 1v1 Commander game on the CMD pool.
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

#[test]
fn the_cmd_pool_loads() {
    let g = table();
    assert_eq!(g.players[0].life, 40);
}

// ---- Glarb, Calamity's Augur (DonSpider) ----

fn colors(g: &Game, id: ObjId) -> [bool; 5] {
    g.mana_sources(0)
        .into_iter()
        .filter(|&(x, _)| x == id)
        .fold([false; 5], |acc, (_, m)| {
            std::array::from_fn(|i| acc[i] || m.colors[i])
        })
}

/// Activates a fetch land with `wanted` and `unwanted` cards in the library; checks it can find only
/// `wanted`, and takes it. Shock-land answers follow in `then`.
fn fetch(land: &str, wanted: &str, unwanted: &str, then: &[Action]) -> (Game, ObjId) {
    let mut g = table();
    let f = put(&mut g, 0, land);
    let bad = g.add_card(0, cid(unwanted), Zone::Library);
    let good = g.add_card(0, cid(wanted), Zone::Library);
    let life = g.players[0].life;
    act(&mut g, Action::Activate(f, 0));
    assert_eq!(g.players[0].life, life - 1);
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Pick(bad)),
        "{land} can't find {unwanted}"
    );
    act(&mut g, Action::Pick(good));
    resolve_all(&mut g, then);
    assert_eq!(g.obj(good).zone, Zone::Battlefield);
    (g, good)
}

#[test]
fn bloodstained_mire_fetches_a_swamp_or_mountain() {
    fetch("Bloodstained Mire", "Mountain", "Forest", &[]);
    fetch("Bloodstained Mire", "Overgrown Tomb", "Island", &[Action::Yes]); // a Swamp Forest
}

#[test]
fn polluted_delta_fetches_an_island_or_swamp() {
    let (g, grave) = fetch("Polluted Delta", "Watery Grave", "Forest", &[Action::No]);
    assert!(
        g.obj(grave).tapped,
        "a fetched shock land that isn't paid for enters tapped"
    );
}

#[test]
fn windswept_heath_fetches_a_forest_or_plains() {
    fetch("Windswept Heath", "Breeding Pool", "Swamp", &[Action::Yes]);
    fetch("Windswept Heath", "Plains", "Island", &[]);
}

#[test]
fn drowned_catacomb_enters_untapped_with_an_island_or_swamp() {
    let mut g = game(&["Forest"]);
    let c = hand(&mut g, 0, "Drowned Catacomb");
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped);
    assert_eq!(
        colors(&g, c),
        [false, false, false, false, false],
        "tapped: no mana now"
    );
    let mut g = game(&["Swamp"]);
    let c = hand(&mut g, 0, "Drowned Catacomb");
    act(&mut g, Action::PlayLand(c));
    assert!(!g.obj(c).tapped);
    assert_eq!(colors(&g, c), [false, true, true, false, false]);
}

#[test]
fn sunken_hollow_enters_untapped_with_two_basics() {
    let mut g = game(&["Forest"]);
    let c = hand(&mut g, 0, "Sunken Hollow");
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped);
    let mut g = game(&["Forest", "Island"]);
    let c = hand(&mut g, 0, "Sunken Hollow");
    act(&mut g, Action::PlayLand(c));
    assert!(!g.obj(c).tapped);
    assert_eq!(colors(&g, c), [false, true, true, false, false]);
    assert!(g.has_subtype(c, "Island") && g.has_subtype(c, "Swamp"));
}

#[test]
fn undergrowth_stadium_enters_tapped_against_one_opponent() {
    let mut g = game(&[]);
    let c = hand(&mut g, 0, "Undergrowth Stadium");
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(colors(&g, c), [false, false, true, false, true]);
}

fn surveil_land(name: &str, mana: [bool; 5]) {
    let mut g = game(&[]);
    let top = g.add_card(0, cid("Plains"), Zone::Library);
    let c = hand(&mut g, 0, name);
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped);
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Yes); // surveil 1: into the graveyard
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(top).zone, Zone::Graveyard);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(colors(&g, c), mana);
}

#[test]
fn undercity_sewers_enters_tapped_and_surveils() {
    surveil_land("Undercity Sewers", [false, true, true, false, false]);
}

#[test]
fn underground_mortuary_enters_tapped_and_surveils() {
    surveil_land("Underground Mortuary", [false, false, true, false, true]);
}

#[test]
fn hedge_maze_enters_tapped_and_surveils() {
    surveil_land("Hedge Maze", [false, true, false, false, true]);
}

#[test]
fn zagoth_triome_enters_tapped_makes_three_colors_and_cycles() {
    let mut g = game(&["Plains", "Plains", "Plains"]);
    let c = hand(&mut g, 0, "Zagoth Triome");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(c, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).zone, Zone::Graveyard, "cycled");
    assert_eq!(g.players[0].hand.len(), n);
    let mut g = game(&[]);
    let c = hand(&mut g, 0, "Zagoth Triome");
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(colors(&g, c), [false, true, true, false, true]);
}

/// Plays a shock land, paying 2 life or not.
fn shock(name: &str, pay: bool, mana: [bool; 5]) {
    let mut g = game(&[]);
    let life = g.players[0].life;
    let c = hand(&mut g, 0, name);
    act(&mut g, Action::PlayLand(c));
    assert_eq!(g.pending, Pending::Decide(0), "pay 2 life?");
    act(&mut g, if pay { Action::Yes } else { Action::No });
    assert_eq!(g.obj(c).tapped, !pay);
    assert_eq!(g.players[0].life, if pay { life - 2 } else { life });
    if pay {
        assert_eq!(colors(&g, c), mana);
    }
}

#[test]
fn breeding_pool_pays_2_life_or_enters_tapped() {
    shock("Breeding Pool", true, [false, true, false, false, true]);
    shock("Breeding Pool", false, [false; 5]);
}

#[test]
fn overgrown_tomb_pays_2_life_or_enters_tapped() {
    shock("Overgrown Tomb", true, [false, false, true, false, true]);
    shock("Overgrown Tomb", false, [false; 5]);
}

#[test]
fn watery_grave_pays_2_life_or_enters_tapped() {
    shock("Watery Grave", true, [false, true, true, false, false]);
    shock("Watery Grave", false, [false; 5]);
}

#[test]
fn damnation_destroys_all_creatures() {
    let mut g = table();
    let a = put(&mut g, 0, "Azusa, Lost but Seeking");
    let b = put(&mut g, 1, "Aesi, Tyrant of Gyre Strait");
    let c = hand(&mut g, 0, "Damnation");
    play(&mut g, c, false, &[], &[]);
    assert_eq!((g.obj(a).zone, g.obj(b).zone), (Zone::Graveyard, Zone::Graveyard));
}

/// How many lands player 0 can play this turn, from a hand of Forests.
fn land_drops(g: &mut Game) -> usize {
    let mut n = 0;
    loop {
        let f = hand(g, 0, "Forest");
        if !g.legal_actions().contains(&Action::PlayLand(f)) {
            return n;
        }
        act(g, Action::PlayLand(f));
        resolve_all(g, &[Action::No; 0]);
        n += 1;
    }
}

#[test]
fn exploration_gives_an_extra_land_drop() {
    let mut g = game(&[]);
    put(&mut g, 0, "Exploration");
    assert_eq!(land_drops(&mut g), 2);
}

#[test]
fn azusa_gives_two_extra_land_drops() {
    let mut g = game(&[]);
    put(&mut g, 0, "Azusa, Lost but Seeking");
    assert_eq!(land_drops(&mut g), 3);
}

#[test]
fn aesi_gives_a_land_drop_and_draws_on_landfall() {
    let mut g = game(&[]);
    put(&mut g, 0, "Aesi, Tyrant of Gyre Strait");
    let n = g.players[0].hand.len();
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.players[0].hand.len(), n + 1, "played one, drew one");
    let f2 = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f2));
    resolve_all(&mut g, &[Action::No]);
    let f3 = hand(&mut g, 0, "Forest");
    assert!(
        !g.legal_actions().contains(&Action::PlayLand(f3)),
        "two land drops"
    );
}
