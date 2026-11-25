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

#[test]
fn avenger_of_zendikar_makes_a_plant_per_land_and_grows_them_on_landfall() {
    let mut g = table();
    let lands = g.permanents(0).filter(|&x| g.types_of(x).land).count();
    let a = hand(&mut g, 0, "Avenger of Zendikar");
    play(&mut g, a, false, &[], &[]);
    let plants = tokens(&g, 0, "Plant");
    assert_eq!(plants.len(), lands);
    assert_eq!((g.power(plants[0]), g.toughness(plants[0])), (0, 1));
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!((g.power(plants[0]), g.toughness(plants[0])), (1, 2));
}

#[test]
fn scute_swarm_makes_insects_then_copies_itself_at_six_lands() {
    let mut g = game(&["Forest"; 4]);
    put(&mut g, 0, "Scute Swarm");
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[]);
    let bugs = tokens(&g, 0, "Insect");
    assert_eq!(bugs.len(), 1);
    assert!(!g.keywords(bugs[0]).has(Keywords::FLYING));
    let mut g = game(&["Forest"; 5]);
    put(&mut g, 0, "Scute Swarm");
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Scute Swarm").len(), 1, "six lands: a copy");
}

#[test]
fn wayward_swordtooth_needs_the_citys_blessing_to_attack() {
    let mut g = game(&[]);
    let w = put(&mut g, 0, "Wayward Swordtooth");
    put(&mut g, 1, "Morbid Opportunist");
    assert_eq!(land_drops(&mut g), 2);
    assert!(!g.city_blessing[0]);
    g.start_turn(5, 0);
    while g.step != Step::Main2 && g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.step, Step::Main2, "no blessing: nothing can attack");
    let _ = w;
    let mut g = table(); // 25 lands: the blessing
    let w = put(&mut g, 0, "Wayward Swordtooth");
    put(&mut g, 1, "Morbid Opportunist");
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    assert!(g.city_blessing[0]);
    assert!(g.legal_actions().contains(&Action::Attack(w)));
}

#[test]
fn retreat_to_coralhelm_taps_or_untaps_a_creature_or_scries_on_landfall() {
    let mut g = game(&[]);
    put(&mut g, 0, "Retreat to Coralhelm");
    let theirs = put(&mut g, 1, "Morbid Opportunist");
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[Action::Mode(0), Action::Target(obj(theirs))]);
    assert!(g.obj(theirs).tapped);
}

#[test]
fn guardian_project_draws_for_a_new_name_only() {
    let mut g = table();
    put(&mut g, 0, "Guardian Project");
    let n = g.players[0].hand.len();
    let a = hand(&mut g, 0, "Morbid Opportunist");
    play(&mut g, a, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), n + 1, "a new name: a card");
    let b = hand(&mut g, 0, "Morbid Opportunist");
    play(&mut g, b, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), n + 1, "the same name: no card");
}

#[test]
fn dryad_of_the_ilysian_grove_makes_lands_every_type() {
    let mut g = game(&["Forest"]);
    let f = g.permanents(0).find(|&x| g.types_of(x).land).unwrap();
    assert!(!g.has_subtype(f, "Island"));
    put(&mut g, 0, "Dryad of the Ilysian Grove");
    assert!(g.has_subtype(f, "Island") && g.has_subtype(f, "Mountain"));
    assert_eq!(colors(&g, f), [true; 5]);
    assert_eq!(land_drops(&mut g), 2);
}

#[test]
fn clever_impersonator_copies_a_nonland_permanent() {
    let mut g = table();
    let signet = put(&mut g, 1, "Arcane Signet");
    let c = hand(&mut g, 0, "Clever Impersonator");
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Pick(signet));
    resolve_all(&mut g, &[]);
    assert_eq!(g.def_of(c).name, "Arcane Signet");
}

#[test]
fn teval_mills_returns_a_land_and_makes_zombies_when_cards_leave_the_graveyard() {
    let mut g = table();
    let t = put(&mut g, 0, "Teval, the Balanced Scale");
    put(&mut g, 1, "Morbid Opportunist");
    g.start_turn(5, 0);
    for n in ["Forest", "Morbid Opportunist", "Arcane Signet"] {
        top(&mut g, n);
    }
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(t));
    act(&mut g, Action::FinishAttacks);
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    let land = g.players[0]
        .graveyard
        .iter()
        .copied()
        .find(|&c| g.types_of(c).land)
        .expect("milled");
    act(&mut g, Action::Pick(land));
    resolve_all(&mut g, &[]); // the Zombie Druid trigger
    assert!(g.obj(land).zone == Zone::Battlefield && g.obj(land).tapped);
    assert_eq!(tokens(&g, 0, "Zombie Druid").len(), 1);
}

#[test]
fn greensleeves_counts_lands_makes_badgers_and_has_protection_from_planeswalkers_and_wizards() {
    let mut g = game(&["Forest"; 5]);
    let gs = put(&mut g, 0, "Greensleeves, Maro-Sorcerer");
    assert_eq!(g.power(gs), 5);
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(gs), 6);
    let b = tokens(&g, 0, "Badger");
    assert_eq!(b.len(), 1);
    assert_eq!(g.power(b[0]), 3);
    let wizard = put(&mut g, 1, "Zimone, Mystery Unraveler"); // a Human Wizard
    let _ = wizard;
    let f2 = bolt_core::effect::Filter::CREATURE;
    assert!(
        g.target_ok(&f2, 1, Target::Object(gs)),
        "an ordinary source can target it"
    );
}

#[test]
fn the_gitrog_monster_eats_a_land_each_upkeep_and_draws_for_lands_in_the_graveyard() {
    let mut g = game(&["Forest", "Swamp"]);
    let gm = put(&mut g, 0, "The Gitrog Monster");
    let n = g.players[0].hand.len();
    g.start_turn(5, 0);
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    let land = g.permanents(0).find(|&x| g.types_of(x).land).unwrap();
    act(&mut g, Action::Pick(land));
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(land).zone, Zone::Graveyard);
    assert_eq!(g.obj(gm).zone, Zone::Battlefield);
    assert_eq!(
        g.players[0].hand.len(),
        n + 2,
        "the draw step and the land in the graveyard"
    );
    g.start_turn(7, 0);
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::No);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(gm).zone, Zone::Graveyard, "no land: the Monster goes");
}

#[test]
fn colossal_grave_reaver_mills_and_puts_a_milled_creature_onto_the_battlefield() {
    let mut g = table();
    for n in ["Forest", "Morbid Opportunist", "Plains"] {
        top(&mut g, n);
    }
    let r = hand(&mut g, 0, "Colossal Grave-Reaver");
    act(&mut g, Action::Cast(r));
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    let c = g.players[0]
        .graveyard
        .iter()
        .copied()
        .find(|&c| g.types_of(c).creature)
        .expect("milled");
    act(&mut g, Action::Pick(c));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).zone, Zone::Battlefield);
    assert!(g.keywords(r).has(Keywords::FLYING));
}

#[test]
fn wrenn_and_seven_digs_for_lands_drops_them_and_makes_a_treefolk() {
    let mut g = table();
    let w = hand(&mut g, 0, "Wrenn and Seven");
    play(&mut g, w, false, &[], &[]);
    for n in ["Forest", "Morbid Opportunist", "Island", "Arcane Signet"] {
        top(&mut g, n);
    }
    let n = g.players[0].hand.len();
    act(&mut g, Action::Loyalty(w, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n + 2, "two lands to hand");
    assert_eq!(g.obj(w).loyalty, 6);
    to_main1(&mut g, 5, 0);
    let lands = g.permanents(0).filter(|&x| g.types_of(x).land).count();
    let h: Vec<ObjId> = g.players[0]
        .hand
        .iter()
        .copied()
        .filter(|&c| g.types_of(c).land)
        .take(2)
        .collect();
    act(&mut g, Action::Loyalty(w, 1));
    resolve_all(&mut g, &[Action::Pick(h[0]), Action::Pick(h[1]), Action::No]);
    assert_eq!(g.permanents(0).filter(|&x| g.types_of(x).land).count(), lands + 2);
    assert!(g.obj(h[0]).tapped);
    to_main1(&mut g, 7, 0);
    act(&mut g, Action::Loyalty(w, 2));
    resolve_all(&mut g, &[]);
    let t = tokens(&g, 0, "Treefolk");
    assert_eq!(t.len(), 1);
    assert_eq!(g.power(t[0]) as usize, lands + 2);
    assert!(g.keywords(t[0]).has(Keywords::REACH));
}

#[test]
fn nissa_of_shadowed_boughs_grows_on_landfall_animates_a_land_and_cheats_a_creature() {
    let mut g = table();
    let n = hand(&mut g, 0, "Nissa of Shadowed Boughs");
    play(&mut g, n, false, &[], &[]);
    assert_eq!(g.obj(n).loyalty, 4);
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(n).loyalty, 5, "landfall");
    let tapped = g
        .permanents(0)
        .find(|&x| g.types_of(x).land && g.obj(x).tapped)
        .expect("paid for Nissa");
    act(&mut g, Action::Loyalty(n, 0));
    act(&mut g, Action::Target(obj(tapped)));
    resolve_all(&mut g, &[Action::Yes]);
    assert!(!g.obj(tapped).tapped);
    assert!(g.types_of(tapped).creature && g.types_of(tapped).land);
    assert_eq!(g.power(tapped), 3);
    assert_eq!(g.obj(n).loyalty, 6);
    let big = g.add_card(0, cid("Garruk's Packleader"), Zone::Graveyard);
    to_main1(&mut g, 5, 0);
    act(&mut g, Action::Loyalty(n, 1));
    resolve_all(&mut g, &[Action::Pick(big)]);
    assert_eq!(g.obj(big).zone, Zone::Battlefield);
    assert_eq!(g.power(big), 6, "two +1/+1 counters");
}

#[test]
fn delighted_halfling_makes_colored_mana_only_for_legendary_spells_which_cant_be_countered() {
    let mut g = game(&["Delighted Halfling", "Forest"]);
    let rat = hand(&mut g, 0, "Burglar Rat");
    let tiny = hand(&mut g, 0, "Tinybones, Bauble Burglar");
    let legal = g.legal_actions();
    assert!(
        !legal.contains(&Action::Cast(rat)),
        "{{B}} only for a legendary spell"
    );
    assert!(legal.contains(&Action::Cast(tiny)));
    act(&mut g, Action::Cast(tiny));
    assert!(g.obj(tiny).cast_uncounterable);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(tiny).zone, Zone::Battlefield);
}

#[test]
fn force_of_vigor_is_free_off_turn_by_exiling_a_green_card() {
    let mut g = table();
    let force = hand(&mut g, 0, "Force of Vigor");
    let green = hand(&mut g, 0, "Avenger of Zendikar");
    assert!(
        !g.legal_actions().contains(&Action::CastWith(force, green)),
        "not on your own turn"
    );
    let signet = put(&mut g, 1, "Arcane Signet");
    let expl = put(&mut g, 1, "Exploration");
    to_main1(&mut g, 4, 1);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::CastWith(force, green));
    resolve_all(&mut g, &[Action::Target(obj(signet)), Action::Target(obj(expl))]);
    assert_eq!(g.obj(green).zone, Zone::Exile);
    assert_eq!(g.obj(signet).zone, Zone::Graveyard);
    assert_eq!(g.obj(expl).zone, Zone::Graveyard);
    assert!(g.permanents(0).all(|x| !g.obj(x).tapped), "no mana paid");
}

#[test]
fn dread_return_flashes_back_by_sacrificing_three_creatures() {
    let mut g = table();
    let dr = g.add_card(0, cid("Dread Return"), Zone::Graveyard);
    let ghoul = g.add_card(0, cid("Vampire Gourmand"), Zone::Graveyard);
    let a = put(&mut g, 0, "Aegis Turtle");
    let b = put(&mut g, 0, "Aegis Turtle");
    assert!(
        !g.legal_actions().contains(&Action::CastFlashback(dr)),
        "two creatures"
    );
    let c = put(&mut g, 0, "Aegis Turtle");
    act(&mut g, Action::CastFlashback(dr));
    resolve_all(
        &mut g,
        &[
            Action::Target(obj(ghoul)),
            Action::Pick(a),
            Action::Pick(b),
            Action::Pick(c),
        ],
    );
    for x in [a, b, c] {
        assert_eq!(g.obj(x).zone, Zone::Graveyard);
    }
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    assert_eq!(g.obj(dr).zone, Zone::Exile);
}

#[test]
fn scapeshift_trades_lands_for_as_many_from_the_library() {
    let mut g = game(&["Forest", "Forest", "Forest", "Forest"]);
    let forests: Vec<ObjId> = g.permanents(0).collect();
    let isl = top(&mut g, "Island");
    let sw = top(&mut g, "Swamp");
    let sc = hand(&mut g, 0, "Scapeshift");
    play(
        &mut g,
        sc,
        false,
        &[],
        &[
            Action::Pick(forests[0]),
            Action::Pick(forests[1]),
            Action::No,
            Action::Pick(isl),
            Action::Pick(sw),
        ],
    );
    assert_eq!(g.obj(forests[0]).zone, Zone::Graveyard);
    assert_eq!(g.obj(forests[2]).zone, Zone::Battlefield);
    assert_eq!(g.obj(isl).zone, Zone::Battlefield);
    assert!(g.obj(sw).tapped);
}

#[test]
fn rise_of_the_witch_king_trades_a_creature_for_another_permanent_card() {
    let mut g = table();
    let mine = put(&mut g, 0, "Aegis Turtle");
    let theirs = put(&mut g, 1, "Aegis Turtle");
    put(&mut g, 1, "Hungry Ghoul");
    let expl = g.add_card(0, cid("Exploration"), Zone::Graveyard);
    let rise = hand(&mut g, 0, "Rise of the Witch-king");
    act(&mut g, Action::Cast(rise));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Pick(theirs));
    act(&mut g, Action::Pick(mine));
    assert!(
        !g.legal_actions().contains(&Action::Pick(mine)),
        "another permanent card"
    );
    resolve_all(&mut g, &[Action::Pick(expl)]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.obj(mine).zone, Zone::Graveyard);
    assert_eq!(g.obj(expl).zone, Zone::Battlefield);
}

#[test]
fn will_of_the_abzan_chooses_both_only_with_a_commander() {
    let mut g = table();
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let turtle = put(&mut g, 1, "Aegis Turtle");
    let back = g.add_card(0, cid("Vampire Gourmand"), Zone::Graveyard);
    let will = hand(&mut g, 0, "Will of the Abzan");
    assert!(g.legal_actions().contains(&Action::CastModal(will, 0)));
    assert!(
        !g.legal_actions().contains(&Action::CastModal(will, 2)),
        "no commander"
    );
    let bello = g.commanders[0].expect("a commander");
    act(&mut g, Action::Cast(bello));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(bello).zone, Zone::Battlefield);
    act(&mut g, Action::CastModal(will, 2));
    resolve_all(
        &mut g,
        &[Action::Target(Target::Player(1)), Action::Target(obj(back))],
    );
    assert_eq!(g.obj(ghoul).zone, Zone::Graveyard, "the greatest power");
    assert_eq!(g.obj(turtle).zone, Zone::Battlefield);
    assert_eq!(g.players[1].life, 37);
    assert_eq!(g.obj(back).zone, Zone::Battlefield);
}

#[test]
fn koma_makes_coils_each_upkeep_and_sacrifices_them_to_tap() {
    let mut g = table();
    let koma = put(&mut g, 0, "Koma, Cosmos Serpent");
    assert!(g.def_of(koma).uncounterable);
    assert!(
        !g.legal_actions().contains(&Action::ActivateWith(koma, 0, koma)),
        "another Serpent"
    );
    to_main1(&mut g, 4, 1);
    to_main1(&mut g, 5, 0);
    let coils = tokens(&g, 0, "Koma's Coil");
    assert_eq!(coils.len(), 2, "each upkeep");
    let land = g.permanents(1).next().unwrap();
    act(&mut g, Action::ActivateWith(koma, 0, coils[0]));
    resolve_all(&mut g, &[Action::Target(obj(land))]);
    assert!(g.obj(land).tapped);
    assert_eq!(g.obj(land).activations_stopped_turn, g.turn);
    act(&mut g, Action::ActivateWith(koma, 1, coils[1]));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(koma).has(Keywords::INDESTRUCTIBLE));
}

#[test]
fn senseis_divining_top_reorders_the_top_three_and_draws_then_goes_on_top() {
    let mut g = table();
    let t = put(&mut g, 0, "Sensei's Divining Top");
    let c = top(&mut g, "Forest");
    let b = top(&mut g, "Island");
    let a = top(&mut g, "Swamp");
    act(&mut g, Action::Activate(t, 0));
    resolve_all(&mut g, &[Action::Pick(c), Action::Pick(a)]);
    let lib = &g.players[0].library;
    assert_eq!(&lib[lib.len() - 3..], &[b, a, c], "c on top, then a, then b");
    act(&mut g, Action::Activate(t, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).zone, Zone::Hand);
    assert_eq!(g.players[0].library.last(), Some(&t));
}

#[test]
fn hedge_shredder_crews_attacks_and_puts_milled_lands_onto_the_battlefield() {
    let mut g = game(&["Hedge Shredder", "Aegis Turtle"]);
    let hs = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hedge Shredder")
        .unwrap();
    let turtle = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Aegis Turtle")
        .unwrap();
    assert!(!g.types_of(hs).creature);
    assert!(
        !g.legal_actions().contains(&Action::ActivateWith(hs, 0, turtle)),
        "Aegis Turtle has power 0"
    );
    let bird = put(&mut g, 0, "Strix Lookout");
    act(&mut g, Action::ActivateWith(hs, 0, bird));
    resolve_all(&mut g, &[]);
    assert!(g.obj(bird).tapped);
    assert!(g.types_of(hs).creature && g.types_of(hs).artifact);
    assert_eq!(g.power(hs), 5);
    let forest = top(&mut g, "Forest");
    top(&mut g, "Aegis Turtle");
    while g.step != Step::DeclareAttackers {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(hs));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(g.obj(forest).tapped);
    to_main1(&mut g, 4, 1);
    assert!(!g.types_of(hs).creature, "until end of turn");
}

#[test]
fn nissa_worldsoul_speaker_gets_energy_on_landfall_and_spends_eight_for_a_permanent() {
    let mut g = game(&["Nissa, Worldsoul Speaker"]);
    let nissa = g.permanents(0).next().unwrap();
    let avenger = hand(&mut g, 0, "Avenger of Zendikar");
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].energy, 2);
    assert!(!g.legal_actions().contains(&Action::CastWith(avenger, nissa)));
    g.players[0].energy = 8;
    act(&mut g, Action::CastWith(avenger, nissa));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(avenger).zone, Zone::Battlefield);
    assert_eq!(g.players[0].energy, 0);
    assert_eq!(g.obj(nissa).zone, Zone::Battlefield);
}

// ---- lane B: Phyrexian Metamorph, the Sakashimas, Zimone ----

fn lb_has(g: &Game, a: Action) -> bool {
    g.legal_actions().contains(&a)
}

/// Passes priority until a decision or target choice is pending.
fn lb_pass_until_asked(g: &mut Game) {
    for _ in 0..50 {
        if matches!(g.pending, Pending::Decide(_) | Pending::ChooseTarget(_)) {
            return;
        }
        act(g, Action::PassPriority);
    }
    panic!("never asked: {:?}", g.pending);
}

#[test]
fn phyrexian_metamorph_copies_an_artifact_or_creature_and_is_an_artifact() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let lions = put(&mut g, 1, "Savannah Lions");
    let meta = hand(&mut g, 0, "Phyrexian Metamorph");
    // {3}{U/P}: the Phyrexian pip may be paid with 2 life (the alternative cast).
    assert!(lb_has(&g, Action::Cast(meta)) && lb_has(&g, Action::CastAlternative(meta)));
    act(&mut g, Action::CastAlternative(meta));
    lb_pass_until_asked(&mut g);
    assert_eq!(g.players[0].life, 38);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(ring)) && legal.contains(&Action::Pick(lions)));
    act(&mut g, Action::Pick(lions));
    resolve_all(&mut g, &[]);
    assert_eq!(g.def_of(meta).name, "Savannah Lions");
    let t = g.types_of(meta);
    assert!(t.creature && t.artifact);
    assert_eq!(g.power(meta), 2);
    // Paid with mana: no life.
    let mut g = table();
    put(&mut g, 1, "Sol Ring");
    let meta = hand(&mut g, 0, "Phyrexian Metamorph");
    play(&mut g, meta, false, &[], &[Action::No]);
    assert_eq!(g.players[0].life, 40);
    assert_eq!(g.obj(meta).zone, Zone::Graveyard, "as itself it's a 0/0");
}

#[test]
fn sakashima_of_a_thousand_faces_copies_your_creature_and_ignores_the_legend_rule() {
    let mut g = table();
    let ojer = put(&mut g, 0, "Ojer Kaslem, Deepest Growth // Temple of Cultivation");
    let theirs = put(&mut g, 1, "Llanowar Elves");
    let saka = hand(&mut g, 0, "Sakashima of a Thousand Faces");
    act(&mut g, Action::Cast(saka));
    lb_pass_until_asked(&mut g);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(ojer)));
    assert!(
        !legal.contains(&Action::Pick(theirs)),
        "only creatures you control"
    );
    act(&mut g, Action::Pick(ojer));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.def_of(saka).name,
        "Ojer Kaslem, Deepest Growth // Temple of Cultivation"
    );
    assert_eq!(g.obj(saka).zone, Zone::Battlefield);
    assert_eq!(
        g.obj(ojer).zone,
        Zone::Battlefield,
        "the legend rule doesn't apply"
    );
}

#[test]
fn sakashima_the_impostor_keeps_its_name_and_can_bounce_itself() {
    let mut g = table();
    let ojer = put(&mut g, 0, "Ojer Kaslem, Deepest Growth // Temple of Cultivation");
    let imp = hand(&mut g, 0, "Sakashima the Impostor");
    act(&mut g, Action::Cast(imp));
    lb_pass_until_asked(&mut g);
    act(&mut g, Action::Pick(ojer));
    resolve_all(&mut g, &[]);
    // Named Sakashima the Impostor: no legend rule clash with Ojer Kaslem.
    assert_eq!(g.name_of(imp), "Sakashima the Impostor");
    assert!(g.types_of(imp).legendary && g.types_of(imp).creature);
    assert_eq!(g.power(imp), 6);
    assert_eq!(g.obj(ojer).zone, Zone::Battlefield);
    assert_eq!(g.obj(imp).zone, Zone::Battlefield);
    // "{2}{U}{U}: Return Sakashima the Impostor to its owner's hand at the beginning of the next end step."
    let n = g.def_of(imp).activated.len();
    assert!(lb_has(&g, Action::Activate(imp, n)));
    act(&mut g, Action::Activate(imp, n));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(imp).zone, Zone::Battlefield);
    for _ in 0..40 {
        if g.obj(imp).zone == Zone::Hand {
            break;
        }
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(imp).zone, Zone::Hand);
    assert!(g.step == Step::End || g.step == Step::Cleanup || g.turn == 3);
    // As itself (no copy), it doesn't have that ability.
    let mut g = table();
    let imp = put(&mut g, 0, "Sakashima the Impostor");
    assert!(!lb_has(&g, Action::Activate(imp, 0)));
}

#[test]
fn zimone_manifests_dread_then_turns_a_permanent_face_up() {
    let mut g = table();
    put(&mut g, 0, "Zimone, Mystery Unraveler");
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Library);
    let plains = g.add_card(0, cid("Plains"), Zone::Library);
    let land = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(land));
    lb_pass_until_asked(&mut g);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(lions)) && legal.contains(&Action::Pick(plains)));
    act(&mut g, Action::Pick(lions));
    resolve_all(&mut g, &[]);
    assert!(g.obj(lions).face_down);
    assert_eq!(g.obj(plains).zone, Zone::Graveyard);
    assert_eq!(g.def_of(lions).name, "");
    assert_eq!((g.power(lions), g.toughness(lions)), (2, 2));
    assert!(g.types_of(lions).creature && g.colors_of(lions) == [false; 5]);
    // The second landfall this turn: turn a permanent face up instead.
    let lore = hand(&mut g, 0, "Nature's Lore");
    play(
        &mut g,
        lore,
        false,
        &[],
        &[Action::Pick(forest), Action::Pick(lions)],
    );
    assert!(!g.obj(lions).face_down);
    assert_eq!(g.def_of(lions).name, "Savannah Lions");
    assert_eq!(g.power(lions), 2);
}

#[test]
fn a_manifested_creature_card_turns_face_up_for_its_mana_cost() {
    let mut g = table();
    put(&mut g, 0, "Zimone, Mystery Unraveler");
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Library);
    g.add_card(0, cid("Plains"), Zone::Library);
    let land = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(land));
    lb_pass_until_asked(&mut g);
    act(&mut g, Action::Pick(lions));
    resolve_all(&mut g, &[]);
    assert!(g.obj(lions).face_down);
    // Special action: {W}, no stack.
    assert!(lb_has(&g, Action::Activate(lions, 0)));
    act(&mut g, Action::Activate(lions, 0));
    assert!(!g.obj(lions).face_down);
    assert!(g.items.is_empty());
    assert_eq!(g.def_of(lions).name, "Savannah Lions");
    // A manifested noncreature card can't be turned up that way.
    let mut g = table();
    put(&mut g, 0, "Zimone, Mystery Unraveler");
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    g.add_card(0, cid("Plains"), Zone::Library);
    let land = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(land));
    lb_pass_until_asked(&mut g);
    act(&mut g, Action::Pick(ring));
    resolve_all(&mut g, &[]);
    assert!(g.obj(ring).face_down);
    assert!(!lb_has(&g, Action::Activate(ring, 0)));
}

#[test]
fn jace_the_mind_sculptor_fateseals_brainstorms_and_bounces() {
    let mut g = table();
    let jace = hand(&mut g, 0, "Jace, the Mind Sculptor");
    play(&mut g, jace, false, &[], &[]);
    assert_eq!(g.obj(jace).loyalty, 3);
    let theirs = *g.players[1].library.last().unwrap();
    act(&mut g, Action::Loyalty(jace, 0));
    resolve_all(&mut g, &[Action::Target(Target::Player(1)), Action::No]);
    assert_eq!(g.players[1].library[0], theirs, "on the bottom");
    assert_eq!(g.obj(jace).loyalty, 5);
    to_main1(&mut g, 5, 0);
    let a = hand(&mut g, 0, "Forest");
    let b = hand(&mut g, 0, "Island");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Loyalty(jace, 1));
    resolve_all(&mut g, &[Action::Pick(a), Action::Pick(b)]);
    assert_eq!(g.players[0].hand.len(), n + 1, "three drawn, two back");
    let lib = &g.players[0].library;
    assert_eq!(&lib[lib.len() - 2..], &[a, b], "b on top");
    to_main1(&mut g, 7, 0);
    let bear = put(&mut g, 1, "Hungry Ghoul");
    act(&mut g, Action::Loyalty(jace, 2));
    resolve_all(&mut g, &[Action::Target(obj(bear))]);
    assert_eq!(g.obj(bear).zone, Zone::Hand);
    assert_eq!(g.obj(jace).loyalty, 4);
    g.objects[jace.0 as usize].loyalty = 12;
    to_main1(&mut g, 9, 0);
    let their_hand = g.players[1].hand.len();
    act(&mut g, Action::Loyalty(jace, 3));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert_eq!(
        g.players[1].library.len(),
        their_hand,
        "their hand became their library"
    );
    assert!(g.players[1].hand.is_empty());
}

#[test]
fn scroll_rack_swaps_cards_from_hand_with_the_top_of_the_library() {
    let mut g = table();
    let rack = put(&mut g, 0, "Scroll Rack");
    let x = hand(&mut g, 0, "Forest");
    let y = hand(&mut g, 0, "Island");
    let t1 = top(&mut g, "Swamp");
    let t2 = top(&mut g, "Mountain");
    act(&mut g, Action::Activate(rack, 0));
    resolve_all(
        &mut g,
        &[Action::Pick(x), Action::Pick(y), Action::No, Action::Pick(x)],
    );
    assert_eq!(g.obj(t1).zone, Zone::Hand);
    assert_eq!(g.obj(t2).zone, Zone::Hand);
    let lib = &g.players[0].library;
    assert_eq!(&lib[lib.len() - 2..], &[y, x], "x chosen for the top");
    assert!(g.obj(rack).tapped);
}

#[test]
fn conduit_of_worlds_casts_a_permanent_card_from_the_graveyard_as_the_only_spell() {
    let mut g = table();
    let conduit = put(&mut g, 0, "Conduit of Worlds");
    let land = g.add_card(0, cid("Forest"), Zone::Graveyard);
    assert!(g.legal_actions().contains(&Action::PlayLand(land)));
    let ghoul = g.add_card(0, cid("Hungry Ghoul"), Zone::Graveyard);
    let other = hand(&mut g, 0, "Burglar Rat");
    act(&mut g, Action::Activate(conduit, 0));
    resolve_all(&mut g, &[Action::Target(obj(ghoul))]);
    assert!(g.legal_actions().contains(&Action::Cast(ghoul)));
    act(&mut g, Action::Cast(ghoul));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    assert!(
        !g.legal_actions().contains(&Action::Cast(other)),
        "no additional spells this turn"
    );
}

#[test]
fn legolass_quick_reflexes_has_split_second_and_grants_a_tap_trigger() {
    let mut g = game(&["Hungry Ghoul", "Forest", "Island"]);
    let mine = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    let theirs = put(&mut g, 1, "Burglar Rat");
    g.objects[mine.0 as usize].tapped = true;
    let lql = hand(&mut g, 0, "Legolas's Quick Reflexes");
    act(&mut g, Action::Cast(lql));
    act(&mut g, Action::Target(obj(mine)));
    assert_eq!(g.legal_actions(), vec![Action::PassPriority], "split second");
    resolve_all(&mut g, &[]);
    assert!(!g.obj(mine).tapped);
    assert!(g.keywords(mine).has(Keywords::REACH) && g.keywords(mine).has(Keywords::HEXPROOF));
    while g.step != Step::DeclareAttackers {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(mine));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[Action::Target(obj(theirs))]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "2 damage to a 1/1");
}

// ---- Aesi, Tyrant of Gyre Strait (Time) ----

#[test]
fn swan_song_counters_an_instant_and_its_controller_gets_a_bird() {
    let mut g = table();
    let think = hand(&mut g, 0, "Think Twice");
    let swan = hand(&mut g, 0, "Swan Song");
    act(&mut g, Action::Cast(think));
    act(&mut g, Action::Cast(swan));
    resolve_all(&mut g, &[Action::Target(obj(think))]);
    assert_eq!(g.obj(think).zone, Zone::Graveyard);
    let birds = tokens(&g, 0, "Bird");
    assert_eq!(birds.len(), 1);
    assert_eq!(g.power(birds[0]), 2);
    assert!(g.keywords(birds[0]).has(Keywords::FLYING));
}

#[test]
fn biorhythm_sets_each_life_total_to_that_players_creature_count() {
    let mut g = table();
    put(&mut g, 0, "Aegis Turtle");
    put(&mut g, 0, "Hungry Ghoul");
    let bio = hand(&mut g, 0, "Biorhythm");
    play(&mut g, bio, false, &[], &[]);
    assert_eq!(g.players[0].life, 2);
    assert_eq!(g.players[1].life, 0);
}

#[test]
fn sunder_returns_all_lands_to_hand() {
    let mut g = table();
    let sunder = hand(&mut g, 0, "Sunder");
    play(&mut g, sunder, false, &[], &[]);
    assert!(g.battlefield.iter().all(|&x| !g.types_of(x).land));
    assert!(g.players[1].hand.iter().filter(|&&x| g.types_of(x).land).count() >= 25);
}

#[test]
fn ghost_town_returns_to_hand_only_off_turn() {
    let mut g = game(&["Ghost Town"]);
    let gt = g.permanents(0).next().unwrap();
    assert!(colors(&g, gt) == [false; 5] && g.mana_sources(0).iter().any(|&(x, m)| x == gt && m.colorless));
    assert!(!g.legal_actions().contains(&Action::Activate(gt, 0)), "your turn");
    to_main1(&mut g, 4, 1);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Activate(gt, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(gt).zone, Zone::Hand);
}

#[test]
fn palinchron_untaps_seven_lands_and_returns_to_hand() {
    let mut g = table();
    let pal = hand(&mut g, 0, "Palinchron");
    play(&mut g, pal, false, &[], &[]);
    assert!(g.keywords(pal).has(Keywords::FLYING));
    let tapped = g
        .permanents(0)
        .filter(|&x| g.types_of(x).land && g.obj(x).tapped)
        .count();
    assert_eq!(tapped, 0, "the seven lands it cost are untapped");
    act(&mut g, Action::Activate(pal, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(pal).zone, Zone::Hand);
}

#[test]
fn broken_bond_destroys_an_artifact_and_puts_a_land_from_hand() {
    let mut g = table();
    let signet = put(&mut g, 1, "Arcane Signet");
    let forest = hand(&mut g, 0, "Forest");
    let bb = hand(&mut g, 0, "Broken Bond");
    play(&mut g, bb, false, &[obj(signet)], &[Action::Pick(forest)]);
    assert_eq!(g.obj(signet).zone, Zone::Graveyard);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(!g.obj(forest).tapped);
}

#[test]
fn collector_ouphe_stops_artifact_abilities() {
    let mut g = table();
    let top_ = put(&mut g, 0, "Sensei's Divining Top");
    assert!(g.legal_actions().contains(&Action::Activate(top_, 0)));
    put(&mut g, 1, "Collector Ouphe");
    assert!(!g.legal_actions().contains(&Action::Activate(top_, 0)));
    assert!(!g.legal_actions().contains(&Action::Activate(top_, 1)));
}

#[test]
fn mox_amber_makes_the_colors_of_legendary_creatures() {
    let mut g = game(&["Mox Amber"]);
    let mox = g.permanents(0).next().unwrap();
    assert_eq!(colors(&g, mox), [false; 5], "no legendary creature: no mana");
    put(&mut g, 0, "Tinybones, Bauble Burglar");
    assert_eq!(colors(&g, mox), [false, false, true, false, false]);
}

#[test]
fn voidslime_counters_an_activated_ability() {
    let mut g = table();
    let t = put(&mut g, 0, "Sensei's Divining Top");
    let vs = hand(&mut g, 0, "Voidslime");
    act(&mut g, Action::Activate(t, 0));
    assert_eq!(g.items.len(), 1);
    act(&mut g, Action::Cast(vs));
    resolve_all(&mut g, &[Action::Target(obj(t))]);
    assert!(g.items.is_empty());
    assert_eq!(g.obj(t).zone, Zone::Battlefield);
    assert_eq!(g.obj(vs).zone, Zone::Graveyard);
}

#[test]
fn awaken_the_woods_makes_x_forest_dryad_land_creatures() {
    let mut g = table();
    let atw = hand(&mut g, 0, "Awaken the Woods");
    act(&mut g, Action::CastX(atw, 3));
    resolve_all(&mut g, &[]);
    let dryads = tokens(&g, 0, "Forest Dryad");
    assert_eq!(dryads.len(), 3);
    let t = g.types_of(dryads[0]);
    assert!(t.land && t.creature);
    assert!(g.has_subtype(dryads[0], "Forest"));
}

#[test]
fn sylvan_advocate_grows_with_six_lands() {
    let mut g = game(&[
        "Sylvan Advocate",
        "Forest",
        "Forest",
        "Forest",
        "Forest",
        "Forest",
    ]);
    let adv = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Sylvan Advocate")
        .unwrap();
    assert_eq!(g.power(adv), 2);
    assert!(g.keywords(adv).has(Keywords::VIGILANCE));
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    assert_eq!((g.power(adv), g.toughness(adv)), (4, 5));
}

#[test]
fn omnath_keeps_unspent_green_mana_and_grows_with_it() {
    let mut g = game(&["Omnath, Locus of Mana", "Forest", "Island"]);
    let om = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Omnath, Locus of Mana")
        .unwrap();
    let forest = g.permanents(0).find(|&x| g.def_of(x).name == "Forest").unwrap();
    let island = g.permanents(0).find(|&x| g.def_of(x).name == "Island").unwrap();
    g.manual_mana[0] = true;
    act(&mut g, Action::TapForMana(forest, 4));
    act(&mut g, Action::TapForMana(island, 1));
    assert_eq!(g.power(om), 2);
    let step = g.step;
    while g.step == step {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(
        g.players[0].pool.len(),
        1,
        "the blue mana empties, the green stays"
    );
    assert_eq!((g.power(om), g.toughness(om)), (2, 2));
}

#[test]
fn cyclone_summoner_cast_from_hand_bounces_all_but_giants_wizards_and_lands() {
    let mut g = table();
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let signet = put(&mut g, 0, "Arcane Signet");
    let cs = hand(&mut g, 0, "Cyclone Summoner");
    play(&mut g, cs, false, &[], &[]);
    assert_eq!(g.obj(ghoul).zone, Zone::Hand);
    assert_eq!(g.obj(signet).zone, Zone::Hand);
    assert_eq!(g.obj(cs).zone, Zone::Battlefield, "a Giant");
    assert_eq!(g.permanents(0).filter(|&x| g.types_of(x).land).count(), 25);
}

#[test]
fn roil_elemental_steals_on_landfall_while_you_control_it() {
    let mut g = table();
    let roil = put(&mut g, 0, "Roil Elemental");
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[Action::Target(obj(ghoul)), Action::Yes]);
    assert_eq!(g.obj(ghoul).controller, 0);
    let ota = hand(&mut g, 0, "Otawara, Soaring City");
    act(&mut g, Action::Activate(ota, 0));
    resolve_all(&mut g, &[Action::Target(obj(roil))]);
    assert_eq!(g.obj(roil).zone, Zone::Hand);
    assert_eq!(g.obj(ghoul).controller, 1, "control ends with Roil Elemental");
}

#[test]
fn yavimaya_dryad_has_forestwalk_and_gives_a_forest_to_a_player() {
    let mut g = table();
    let forest = top(&mut g, "Forest");
    let yd = hand(&mut g, 0, "Yavimaya Dryad");
    play(
        &mut g,
        yd,
        false,
        &[],
        &[
            Action::Target(Target::Player(1)),
            Action::Yes,
            Action::Pick(forest),
        ],
    );
    assert!(g.keywords(yd).has(Keywords::FORESTWALK));
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert_eq!(g.obj(forest).controller, 1);
    assert!(g.obj(forest).tapped);
}

#[test]
fn otawara_channels_a_bounce_for_less_per_legendary_creature() {
    let mut g = table();
    put(&mut g, 0, "Tinybones, Bauble Burglar");
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let ota = hand(&mut g, 0, "Otawara, Soaring City");
    act(&mut g, Action::Activate(ota, 0));
    resolve_all(&mut g, &[Action::Target(obj(ghoul))]);
    assert_eq!(g.obj(ghoul).zone, Zone::Hand);
    assert_eq!(g.obj(ota).zone, Zone::Graveyard, "discarded");
    let tapped = g.permanents(0).filter(|&x| g.obj(x).tapped).count();
    assert_eq!(tapped, 3, "{{2}}{{U}}: one less for Tinybones");
}

#[test]
fn scatter_to_the_winds_awakens_a_land_when_cast_for_its_awaken_cost() {
    let mut g = table();
    let think = hand(&mut g, 0, "Think Twice");
    let land = g.permanents(0).find(|&x| g.def_of(x).name == "Forest").unwrap();
    let stw = hand(&mut g, 0, "Scatter to the Winds");
    act(&mut g, Action::Cast(think));
    act(&mut g, Action::CastKicked(stw));
    resolve_all(&mut g, &[Action::Target(obj(think)), Action::Target(obj(land))]);
    assert_eq!(g.obj(think).zone, Zone::Graveyard);
    assert!(g.types_of(land).creature && g.types_of(land).land);
    assert_eq!((g.power(land), g.toughness(land)), (3, 3));
    assert!(g.keywords(land).has(Keywords::HASTE));
    to_main1(&mut g, 5, 0);
    assert!(g.types_of(land).creature, "for good");
}

#[test]
fn three_steps_ahead_pays_for_each_chosen_mode() {
    let mut g = table();
    let think = hand(&mut g, 0, "Think Twice");
    let turtle = put(&mut g, 0, "Aegis Turtle");
    let tsa = hand(&mut g, 0, "Three Steps Ahead");
    act(&mut g, Action::Cast(think));
    let untapped = g
        .permanents(0)
        .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
        .count();
    act(&mut g, Action::CastModal(tsa, 6));
    act(&mut g, Action::Target(obj(think)));
    act(&mut g, Action::Target(obj(turtle)));
    let after = g
        .permanents(0)
        .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
        .count();
    assert_eq!(untapped - after, 8, "{{U}} plus {{1}}{{U}}, {{3}} and {{2}}");
    let n = g.players[0].hand.len();
    let discard = g.players[0].hand[0];
    resolve_all(&mut g, &[Action::Discard(discard)]);
    assert_eq!(g.obj(think).zone, Zone::Graveyard);
    assert_eq!(
        g.permanents(0)
            .filter(|&x| g.def_of(x).name == "Aegis Turtle")
            .count(),
        2
    );
    assert_eq!(g.players[0].hand.len(), n + 1);
}

#[test]
fn plaza_of_heroes_makes_any_color_for_legends_and_protects_one() {
    let mut g = game(&["Plaza of Heroes", "Forest"]);
    let plaza = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Plaza of Heroes")
        .unwrap();
    let rat = hand(&mut g, 0, "Burglar Rat");
    let tiny = hand(&mut g, 0, "Tinybones, Bauble Burglar");
    assert!(!g.legal_actions().contains(&Action::Cast(rat)));
    assert!(g.legal_actions().contains(&Action::Cast(tiny)));
    play(&mut g, tiny, false, &[], &[]);
    assert_eq!(g.obj(tiny).zone, Zone::Battlefield);
    let _ = plaza;
    let mut g = table();
    let plaza = put(&mut g, 0, "Plaza of Heroes");
    assert_eq!(colors(&g, plaza), [false; 5]);
    let tiny = put(&mut g, 0, "Tinybones, Bauble Burglar");
    assert_eq!(
        colors(&g, plaza),
        [false, false, true, false, false],
        "Tinybones is black"
    );
    act(&mut g, Action::Activate(plaza, 0));
    resolve_all(&mut g, &[Action::Target(obj(tiny))]);
    assert_eq!(g.obj(plaza).zone, Zone::Exile);
    assert!(g.keywords(tiny).has(Keywords::HEXPROOF) && g.keywords(tiny).has(Keywords::INDESTRUCTIBLE));
}

#[test]
fn bear_umbra_pumps_untaps_lands_on_attack_and_saves_its_creature() {
    let mut lands = vec!["Hungry Ghoul"];
    for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        lands.extend([land; 5]);
    }
    let mut g = game(&lands);
    let ghoul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    let umbra = hand(&mut g, 0, "Bear Umbra");
    play(&mut g, umbra, false, &[obj(ghoul)], &[]);
    assert_eq!(g.power(ghoul), 4);
    assert!(g.permanents(0).any(|x| g.obj(x).tapped));
    while g.step != Step::DeclareAttackers {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(ghoul));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[]);
    assert!(
        g.permanents(0)
            .filter(|&x| g.types_of(x).land)
            .all(|x| !g.obj(x).tapped),
        "untap all lands you control"
    );
    while g.step != Step::Main2 {
        act(&mut g, Action::PassPriority);
    }
    let dam = hand(&mut g, 0, "Damnation");
    play(&mut g, dam, false, &[], &[]);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield, "umbra armor");
    assert_eq!(g.obj(umbra).zone, Zone::Graveyard);
}

#[test]
fn shifting_woodland_becomes_a_copy_of_a_graveyard_card_with_delirium() {
    let mut g = game(&[]);
    let sw = hand(&mut g, 0, "Shifting Woodland");
    act(&mut g, Action::PlayLand(sw));
    resolve_all(&mut g, &[]);
    assert!(g.obj(sw).tapped, "no Forest");
    let mut g = game(&["Forest", "Forest", "Forest", "Forest"]);
    let sw = hand(&mut g, 0, "Shifting Woodland");
    act(&mut g, Action::PlayLand(sw));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(sw).tapped);
    let ghoul = g.add_card(0, cid("Hungry Ghoul"), Zone::Graveyard);
    g.add_card(0, cid("Forest"), Zone::Graveyard);
    g.add_card(0, cid("Arcane Signet"), Zone::Graveyard);
    assert!(
        !g.legal_actions().contains(&Action::Activate(sw, 0)),
        "three card types"
    );
    g.add_card(0, cid("Think Twice"), Zone::Graveyard);
    act(&mut g, Action::Activate(sw, 0));
    resolve_all(&mut g, &[Action::Target(obj(ghoul))]);
    assert_eq!(g.def_of(sw).name, "Hungry Ghoul");
    assert!(g.types_of(sw).creature);
    while g.turn == 3 {
        let a = match g.legal_actions()[0] {
            d @ Action::Discard(_) => d, // cleanup: down to seven
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.def_of(sw).name, "Shifting Woodland");
}

#[test]
fn khalni_ambush_fights_or_is_played_as_khalni_territory() {
    let mut g = table();
    let k = hand(&mut g, 0, "Khalni Ambush // Khalni Territory");
    assert!(g.legal_actions().contains(&Action::PlayLand(k)));
    act(&mut g, Action::PlayLand(k));
    resolve_all(&mut g, &[]);
    assert_eq!(g.def_of(k).name, "Khalni Territory");
    assert!(g.types_of(k).land && g.obj(k).tapped);
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let rat = put(&mut g, 1, "Burglar Rat");
    let k = hand(&mut g, 0, "Khalni Ambush // Khalni Territory");
    play(&mut g, k, false, &[obj(ghoul), obj(rat)], &[]);
    assert_eq!(g.obj(rat).zone, Zone::Graveyard);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    assert_eq!(g.obj(k).zone, Zone::Graveyard);
}

#[test]
fn nissa_vastwood_seer_finds_a_forest_and_flips_into_a_planeswalker_at_seven_lands() {
    let mut g = table();
    let forest = top(&mut g, "Forest");
    let nissa = hand(&mut g, 0, "Nissa, Vastwood Seer // Nissa, Sage Animist");
    play(&mut g, nissa, false, &[], &[Action::Yes, Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Hand);
    act(&mut g, Action::PlayLand(forest));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(nissa).zone, Zone::Battlefield);
    assert_eq!(g.def_of(nissa).name, "Nissa, Sage Animist");
    assert_eq!(g.obj(nissa).loyalty, 3);
    let land = top(&mut g, "Island");
    act(&mut g, Action::Loyalty(nissa, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.obj(land).zone,
        Zone::Battlefield,
        "a land: onto the battlefield"
    );
    to_main1(&mut g, 5, 0);
    act(&mut g, Action::Loyalty(nissa, 1));
    resolve_all(&mut g, &[]);
    let ashaya = tokens(&g, 0, "Ashaya, the Awoken World");
    assert_eq!(ashaya.len(), 1);
    assert!(g.types_of(ashaya[0]).legendary);
    assert_eq!(g.power(ashaya[0]), 4);
    g.objects[nissa.0 as usize].loyalty = 7;
    to_main1(&mut g, 7, 0);
    let six: Vec<ObjId> = g.permanents(0).filter(|&x| g.types_of(x).land).take(6).collect();
    act(&mut g, Action::Loyalty(nissa, 2));
    let targets: Vec<Action> = six.iter().map(|&x| Action::Target(obj(x))).collect();
    resolve_all(&mut g, &targets);
    for &x in &six {
        assert!(g.types_of(x).creature && g.types_of(x).land);
        assert_eq!(g.power(x), 6);
    }
    to_main1(&mut g, 9, 0);
    assert!(g.types_of(six[0]).creature, "for good");
}
