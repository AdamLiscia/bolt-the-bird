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

/// A card on top of player 0's library.
fn top(g: &mut Game, name: &str) -> ObjId {
    g.add_card(0, cid(name), Zone::Library)
}

#[test]
fn glarb_plays_lands_and_big_spells_from_the_top_and_surveils() {
    let mut g = table();
    let glarb = put(&mut g, 0, "Glarb, Calamity's Augur");
    assert!(g.keywords(glarb).has(Keywords::DEATHTOUCH));
    let land = top(&mut g, "Forest");
    assert!(g.legal_actions().contains(&Action::PlayLand(land)));
    act(&mut g, Action::PlayLand(land));
    let small = top(&mut g, "Morbid Opportunist"); // mana value 3
    assert!(!g.legal_actions().contains(&Action::Cast(small)));
    let big = top(&mut g, "Garruk's Packleader"); // mana value 5
    play(&mut g, big, false, &[], &[]);
    assert_eq!(g.obj(big).zone, Zone::Battlefield);
    to_main1(&mut g, 5, 0);
    act(&mut g, Action::Activate(glarb, 0));
    resolve_all(&mut g, &[Action::Yes, Action::Yes]);
    assert!(g.obj(glarb).tapped);
}

fn to_main1(g: &mut Game, turn: u32, active: usize) {
    g.start_turn(turn, active);
    while g.step != Step::Main1 {
        act(g, Action::PassPriority);
    }
}

#[test]
fn oracle_of_mul_daya_plays_two_lands_including_from_the_top() {
    let mut g = game(&[]);
    put(&mut g, 0, "Oracle of Mul Daya");
    let a = top(&mut g, "Forest");
    act(&mut g, Action::PlayLand(a));
    let b = top(&mut g, "Island");
    act(&mut g, Action::PlayLand(b));
    let c = top(&mut g, "Swamp");
    assert!(
        !g.legal_actions().contains(&Action::PlayLand(c)),
        "two land drops"
    );
    let spell = top(&mut g, "Garruk's Packleader");
    assert!(!g.legal_actions().contains(&Action::Cast(spell)), "lands only");
}

#[test]
fn traveling_chocobo_casts_birds_from_the_top_and_doubles_landfall() {
    let mut g = table();
    put(&mut g, 0, "Traveling Chocobo");
    let bird = top(&mut g, "Traveling Chocobo");
    assert!(g.legal_actions().contains(&Action::Cast(bird)));
    put(&mut g, 0, "Aesi, Tyrant of Gyre Strait");
    let n = g.players[0].hand.len();
    let f = top(&mut g, "Forest");
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[Action::Yes, Action::Yes]);
    assert_eq!(g.players[0].hand.len(), n + 2, "Aesi's landfall twice");
}

#[test]
fn crucible_of_worlds_plays_lands_from_the_graveyard() {
    let mut g = game(&[]);
    let land = g.add_card(0, cid("Forest"), Zone::Graveyard);
    assert!(!g.legal_actions().contains(&Action::PlayLand(land)));
    put(&mut g, 0, "Crucible of Worlds");
    act(&mut g, Action::PlayLand(land));
    assert_eq!(g.obj(land).zone, Zone::Battlefield);
}

#[test]
fn icetill_explorer_plays_extra_lands_from_the_graveyard_and_mills_on_landfall() {
    let mut g = game(&[]);
    put(&mut g, 0, "Icetill Explorer");
    let a = g.add_card(0, cid("Forest"), Zone::Graveyard);
    let milled = top(&mut g, "Plains");
    act(&mut g, Action::PlayLand(a));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(milled).zone, Zone::Graveyard, "landfall: mill a card");
    act(&mut g, Action::PlayLand(milled));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.obj(milled).zone,
        Zone::Battlefield,
        "the second land drop, from the graveyard"
    );
}

#[test]
fn ancient_greenwarden_plays_graveyard_lands_and_doubles_land_triggers() {
    let mut g = game(&[]);
    put(&mut g, 0, "Ancient Greenwarden");
    put(&mut g, 0, "Aesi, Tyrant of Gyre Strait");
    let land = g.add_card(0, cid("Forest"), Zone::Graveyard);
    let n = g.players[0].hand.len();
    act(&mut g, Action::PlayLand(land));
    resolve_all(&mut g, &[Action::Yes, Action::Yes]);
    assert_eq!(g.players[0].hand.len(), n + 2);
}

#[test]
fn yarok_doubles_enter_triggers() {
    let mut g = table();
    put(&mut g, 0, "Yarok, the Desecrated");
    let n = g.players[0].hand.len();
    let c = hand(&mut g, 0, "Spirited Companion"); // when it enters, draw a card
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), n + 2);
}

#[test]
fn seedborn_muse_untaps_your_permanents_on_the_opponents_untap_step() {
    let mut g = table();
    put(&mut g, 0, "Seedborn Muse");
    let big = hand(&mut g, 0, "Garruk's Packleader");
    play(&mut g, big, false, &[], &[]);
    assert!(g.permanents(0).any(|x| g.obj(x).tapped));
    to_main1(&mut g, 4, 1);
    assert!(!g.permanents(0).any(|x| g.obj(x).tapped));
}

#[test]
fn lumra_mills_four_and_returns_the_lands() {
    let mut g = table();
    for n in ["Forest", "Plains", "Morbid Opportunist", "Island"] {
        top(&mut g, n);
    }
    let lands = g.permanents(0).filter(|&x| g.types_of(x).land).count() as i32;
    let l = hand(&mut g, 0, "Lumra, Bellow of the Woods");
    play(&mut g, l, false, &[], &[]);
    assert_eq!(g.power(l), lands + 3, "three lands came back");
    assert_eq!(g.players[0].graveyard.len(), 1, "the creature stays");
    let k = g.keywords(l);
    assert!(k.has(Keywords::REACH) && k.has(Keywords::VIGILANCE));
}

#[test]
fn ob_nixilis_drains_three_and_grows_on_landfall() {
    let mut g = game(&[]);
    let ob = put(&mut g, 0, "Ob Nixilis, the Fallen");
    let life = g.players[1].life;
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[Action::Target(Target::Player(1)), Action::Yes]);
    assert_eq!(g.players[1].life, life - 3);
    assert_eq!(g.power(ob), 6);
}

#[test]
fn doom_whisperer_pays_two_life_to_surveil_two() {
    let mut g = game(&[]);
    let d = put(&mut g, 0, "Doom Whisperer");
    let k = g.keywords(d);
    assert!(k.has(Keywords::FLYING) && k.has(Keywords::TRAMPLE));
    let a = top(&mut g, "Forest");
    let life = g.players[0].life;
    act(&mut g, Action::Activate(d, 0));
    assert_eq!(g.players[0].life, life - 2);
    resolve_all(&mut g, &[Action::Yes, Action::No]);
    assert_eq!(g.obj(a).zone, Zone::Graveyard);
}

#[test]
fn matoya_draws_whenever_you_surveil() {
    let mut g = game(&[]);
    put(&mut g, 0, "Matoya, Archon Elder");
    let d = put(&mut g, 0, "Doom Whisperer");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(d, 0));
    resolve_all(&mut g, &[Action::No, Action::No]);
    assert_eq!(g.players[0].hand.len(), n + 1);
}

#[test]
fn lightning_greaves_gives_haste_and_shroud_for_free() {
    let mut g = game(&[]);
    let gr = put(&mut g, 0, "Lightning Greaves");
    let c = put(&mut g, 0, "Morbid Opportunist");
    act(&mut g, Action::Activate(gr, 0));
    act(&mut g, Action::Target(obj(c)));
    resolve_all(&mut g, &[]);
    let k = g.keywords(c);
    assert!(k.has(Keywords::HASTE) && k.has(Keywords::SHROUD));
    let f = bolt_core::effect::Filter::CREATURE;
    assert!(
        !g.target_ok(&f, 0, Target::Object(c)),
        "not even its controller can target it"
    );
}

#[test]
fn assassins_trophy_destroys_and_the_controller_may_fetch_a_basic() {
    let mut g = table();
    let signet = put(&mut g, 1, "Arcane Signet");
    let lands = g.permanents(1).filter(|&x| g.types_of(x).land).count();
    let c = hand(&mut g, 0, "Assassin's Trophy");
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(signet)));
    while !matches!(g.pending, Pending::Decide(1)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(signet).zone, Zone::Graveyard);
    let pick = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a, Action::Pick(_)))
        .expect("a basic");
    act(&mut g, pick);
    resolve_all(&mut g, &[]);
    assert_eq!(g.permanents(1).filter(|&x| g.types_of(x).land).count(), lands + 1);
}

#[test]
fn cyclonic_rift_bounces_one_or_overloaded_every_nonland_permanent_you_dont_control() {
    let mut g = table();
    let a = put(&mut g, 1, "Arcane Signet");
    let b = put(&mut g, 1, "Morbid Opportunist");
    let mine = put(&mut g, 0, "Arcane Signet");
    let c = hand(&mut g, 0, "Cyclonic Rift");
    play(&mut g, c, false, &[obj(a)], &[]);
    assert_eq!((g.obj(a).zone, g.obj(b).zone), (Zone::Hand, Zone::Battlefield));
    let c2 = hand(&mut g, 0, "Cyclonic Rift");
    play(&mut g, c2, true, &[], &[]);
    assert_eq!(g.obj(b).zone, Zone::Hand);
    assert_eq!(g.obj(mine).zone, Zone::Battlefield);
    assert!(g.permanents(1).all(|x| g.types_of(x).land), "lands stay");
}

#[test]
fn drag_to_the_roots_costs_two_less_with_delirium() {
    let mut g = game(&["Swamp", "Forest"]);
    let target = put(&mut g, 1, "Arcane Signet");
    let c = hand(&mut g, 0, "Drag to the Roots");
    assert!(!g.legal_actions().contains(&Action::Cast(c)));
    for n in ["Forest", "Morbid Opportunist", "Arcane Signet", "Deadly Dispute"] {
        g.add_card(0, cid(n), Zone::Graveyard);
    }
    play(&mut g, c, false, &[obj(target)], &[]);
    assert_eq!(g.obj(target).zone, Zone::Graveyard);
}

#[test]
fn rite_of_replication_copies_a_creature_once_or_five_times() {
    let mut g = table();
    let t = put(&mut g, 1, "Morbid Opportunist");
    let c = hand(&mut g, 0, "Rite of Replication");
    play(&mut g, c, false, &[obj(t)], &[]);
    let copies = |g: &Game| {
        g.permanents(0)
            .filter(|&x| g.is_token(x) && g.def_of(x).name == "Morbid Opportunist")
            .count()
    };
    assert_eq!(copies(&g), 1);
    to_main1(&mut g, 5, 0);
    let c2 = hand(&mut g, 0, "Rite of Replication");
    play(&mut g, c2, true, &[obj(t)], &[]);
    assert_eq!(copies(&g), 6);
}
