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

#[test]
fn damnation_beats_regeneration() {
    // "They can't be regenerated": a regeneration shield doesn't save Silvos.
    let mut g = table();
    let s = put(&mut g, 0, "Silvos, Rogue Elemental");
    act(&mut g, Action::Activate(s, 0));
    resolve_all(&mut g, &[]);
    let c = hand(&mut g, 0, "Damnation");
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(s).zone, Zone::Graveyard);
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

#[test]
fn deadeye_navigator_pairs_and_grants_a_flicker() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let nav = hand(&mut g, 0, "Deadeye Navigator");
    play(&mut g, nav, false, &[], &[Action::Yes, Action::Pick(ghoul)]);
    assert_eq!(g.paired_partner(nav), Some(ghoul));
    assert_eq!(g.paired_partner(ghoul), Some(nav));
    let n = g.def_of(ghoul).activated.len();
    let zcc = g.obj(ghoul).zcc;
    act(&mut g, Action::Activate(ghoul, n));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    assert!(g.obj(ghoul).zcc > zcc, "a new object");
    assert_eq!(g.paired_partner(nav), Some(ghoul), "paired again as it entered");
    let m = g.def_of(nav).activated.len();
    act(&mut g, Action::Activate(nav, m));
    resolve_all(&mut g, &[Action::Yes, Action::Pick(ghoul)]);
    assert_eq!(
        g.paired_partner(nav),
        Some(ghoul),
        "Navigator itself flickers and pairs on entering"
    );
}

// ---- Etali, Primal Conqueror (RuricMo) ----

#[test]
fn green_suns_zenith_fetches_a_green_creature_and_shuffles_itself_in() {
    let mut g = table();
    let big = g.add_card(0, cid("Avenger of Zendikar"), Zone::Library); // mana value 7
    let small = g.add_card(0, cid("Hungry Ghoul"), Zone::Library); // black
    let elf = g.add_card(0, cid("Circle of Dreams Druid"), Zone::Library);
    let gsz = hand(&mut g, 0, "Green Sun's Zenith");
    act(&mut g, Action::CastX(gsz, 3));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Pick(big)) && !legal.contains(&Action::Pick(small)));
    resolve_all(&mut g, &[Action::Pick(elf)]);
    assert_eq!(g.obj(elf).zone, Zone::Battlefield);
    assert_eq!(g.obj(gsz).zone, Zone::Library);
}

#[test]
fn bane_of_progress_destroys_artifacts_and_enchantments_and_grows() {
    let mut g = table();
    put(&mut g, 1, "Arcane Signet");
    put(&mut g, 1, "Exploration");
    put(&mut g, 0, "Sensei's Divining Top");
    let bane = hand(&mut g, 0, "Bane of Progress");
    play(&mut g, bane, false, &[], &[]);
    assert!(g
        .battlefield
        .iter()
        .all(|&x| !g.types_of(x).artifact && !g.types_of(x).enchantment));
    assert_eq!(g.power(bane), 5);
}

#[test]
fn silverback_elder_triggers_on_creature_spells() {
    let mut g = table();
    put(&mut g, 0, "Silverback Elder");
    let ghoul = hand(&mut g, 0, "Hungry Ghoul");
    let life = g.players[0].life;
    act(&mut g, Action::Cast(ghoul));
    resolve_all(&mut g, &[Action::Mode(2)]);
    assert_eq!(g.players[0].life, life + 4);
    let forest = top(&mut g, "Forest");
    let turtle = hand(&mut g, 0, "Aegis Turtle");
    act(&mut g, Action::Cast(turtle));
    resolve_all(&mut g, &[Action::Mode(1), Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(g.obj(forest).tapped);
}

#[test]
fn temple_of_the_false_god_needs_five_lands() {
    let mut g = game(&["Temple of the False God", "Forest", "Forest", "Forest"]);
    let t = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Temple of the False God")
        .unwrap();
    assert!(!g.mana_sources(0).iter().any(|&(x, _)| x == t));
    put(&mut g, 0, "Forest");
    assert_eq!(
        g.mana_sources(0)
            .iter()
            .filter(|&&(x, m)| x == t && m.colorless)
            .count(),
        2
    );
}

#[test]
fn circle_of_dreams_druid_taps_for_a_green_per_creature() {
    let g = game(&["Circle of Dreams Druid", "Hungry Ghoul", "Aegis Turtle"]);
    let c = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Circle of Dreams Druid")
        .unwrap();
    assert_eq!(g.mana_sources(0).iter().filter(|&&(x, _)| x == c).count(), 3);
}

#[test]
fn tribute_to_the_world_tree_draws_for_big_and_grows_small() {
    let mut g = table();
    put(&mut g, 0, "Tribute to the World Tree");
    let n = g.players[0].hand.len();
    let turtle = hand(&mut g, 0, "Aegis Turtle");
    play(&mut g, turtle, false, &[], &[]);
    assert_eq!(g.power(turtle), 2, "two +1/+1 counters");
    let pal = hand(&mut g, 0, "Palinchron");
    play(&mut g, pal, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), n + 1, "power 4: a card");
}

#[test]
fn parallel_lives_doubles_tokens() {
    let mut g = table();
    put(&mut g, 0, "Parallel Lives");
    let atw = hand(&mut g, 0, "Awaken the Woods");
    act(&mut g, Action::CastX(atw, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Forest Dryad").len(), 4);
}

#[test]
fn spire_garden_enters_tapped_and_taps_for_red_or_green() {
    let mut g = game(&[]);
    let sg = hand(&mut g, 0, "Spire Garden");
    act(&mut g, Action::PlayLand(sg));
    assert!(g.obj(sg).tapped);
    let g = game(&["Spire Garden"]);
    let sg = g.permanents(0).next().unwrap();
    assert_eq!(colors(&g, sg), [false, false, false, true, true]);
}

#[test]
fn worldspine_wurm_leaves_three_wurms_and_shuffles_itself_in() {
    let mut g = table();
    let w = put(&mut g, 0, "Worldspine Wurm");
    assert!(g.keywords(w).has(Keywords::TRAMPLE));
    let dam = hand(&mut g, 0, "Damnation");
    play(&mut g, dam, false, &[], &[]);
    assert_eq!(tokens(&g, 0, "Wurm").len(), 3);
    assert_eq!(g.obj(w).zone, Zone::Library);
}

#[test]
fn ulamog_the_infinite_gyre_destroys_on_cast_annihilates_and_reshuffles() {
    let mut g = table();
    let expl = put(&mut g, 1, "Exploration");
    let ul = hand(&mut g, 0, "Ulamog, the Infinite Gyre");
    act(&mut g, Action::Cast(ul));
    resolve_all(&mut g, &[Action::Target(obj(expl))]);
    assert_eq!(g.obj(expl).zone, Zone::Graveyard);
    assert!(g.keywords(ul).has(Keywords::INDESTRUCTIBLE));
    let mut g = game(&["Ulamog, the Infinite Gyre", "Hungry Ghoul", "Forest"]);
    let ul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Ulamog, the Infinite Gyre")
        .unwrap();
    let before = g.permanents(1).count();
    while g.step != Step::DeclareAttackers {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(ul));
    act(&mut g, Action::FinishAttacks);
    let theirs: Vec<ObjId> = g.permanents(1).take(4).collect();
    resolve_all(
        &mut g,
        &theirs.iter().map(|&x| Action::Pick(x)).collect::<Vec<_>>(),
    );
    assert_eq!(g.permanents(1).count(), before - 4, "annihilator 4");
    while g.step != Step::Main2 {
        act(&mut g, Action::PassPriority);
    }
    let ghoul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    let gone = g.add_card(0, cid("Forest"), Zone::Graveyard);
    act(&mut g, Action::ActivateWith(ghoul, 0, ul));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.obj(ul).zone,
        Zone::Library,
        "its owner shuffles their graveyard in"
    );
    assert_eq!(g.obj(gone).zone, Zone::Library);
}

#[test]
fn lurking_predators_puts_a_creature_off_the_top_when_an_opponent_casts() {
    let mut g = table();
    put(&mut g, 0, "Lurking Predators");
    let ghoul = top(&mut g, "Hungry Ghoul");
    to_main1(&mut g, 4, 1);
    let think = hand(&mut g, 1, "Think Twice");
    act(&mut g, Action::Cast(think));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    let land = top(&mut g, "Forest");
    let think = hand(&mut g, 1, "Think Twice");
    act(&mut g, Action::Cast(think));
    resolve_all(&mut g, &[Action::No]);
    assert_eq!(g.players[0].library[0], land, "not a creature: on the bottom");
}

#[test]
fn ulamog_the_defiler_exiles_half_a_library_and_grows_from_exile() {
    let mut g = table();
    g.add_card(1, cid("Avenger of Zendikar"), Zone::Exile); // mana value 7
    let n = g.players[1].library.len();
    let ul = hand(&mut g, 0, "Ulamog, the Defiler");
    act(&mut g, Action::Cast(ul));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].library.len(), n - n.div_ceil(2));
    assert_eq!(g.obj(ul).zone, Zone::Battlefield);
    assert_eq!(g.power(ul), 14, "7 + seven +1/+1 counters");
    assert_eq!(g.def_of(ul).ward_sacrifice, 2);
}

#[test]
fn homeward_path_returns_creatures_to_their_owners() {
    let mut g = table();
    let path = put(&mut g, 0, "Homeward Path");
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    g.objects[ghoul.0 as usize].controller = 0;
    act(&mut g, Action::Activate(path, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ghoul).controller, 1);
}

#[test]
fn purphoros_burns_on_each_creature_and_pumps() {
    let mut g = table();
    let pur = put(&mut g, 0, "Purphoros, God of the Forge");
    assert!(!g.types_of(pur).creature, "devotion to red below five");
    let ghoul = hand(&mut g, 0, "Hungry Ghoul");
    let life = g.players[1].life;
    play(&mut g, ghoul, false, &[], &[]);
    assert_eq!(g.players[1].life, life - 2);
    act(&mut g, Action::Activate(pur, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(ghoul), 3);
}

#[test]
fn conjurers_closet_flickers_a_creature_at_your_end_step() {
    let mut g = game(&["Conjurer's Closet", "Hungry Ghoul"]);
    let ghoul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    let zcc = g.obj(ghoul).zcc;
    while !matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    resolve_all(&mut g, &[Action::Target(obj(ghoul)), Action::Yes]);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    assert!(g.obj(ghoul).zcc > zcc);
}

#[test]
fn cavern_hoard_dragon_costs_less_and_makes_treasures() {
    let mut g = game(&["Mountain", "Mountain", "Forest", "Forest", "Forest", "Forest"]);
    for _ in 0..3 {
        put(&mut g, 1, "Arcane Signet");
    }
    let d = hand(&mut g, 0, "Cavern-Hoard Dragon");
    assert!(g.legal_actions().contains(&Action::Cast(d)), "{{4}}{{R}}{{R}}");
    play(&mut g, d, false, &[], &[]);
    assert!(g.keywords(d).has(Keywords::HASTE));
    while g.step != Step::DeclareAttackers {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(d));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(tokens(&g, 0, "Treasure").len(), 3);
}

#[test]
fn displaced_dinosaurs_makes_historic_permanents_seven_seven_dinosaurs() {
    let mut g = table();
    put(&mut g, 0, "Displaced Dinosaurs");
    let tiny = hand(&mut g, 0, "Tinybones, Bauble Burglar");
    play(&mut g, tiny, false, &[], &[]);
    assert_eq!((g.power(tiny), g.toughness(tiny)), (7, 7));
    assert!(g.has_subtype(tiny, "Dinosaur"));
    let signet = hand(&mut g, 0, "Arcane Signet");
    play(&mut g, signet, false, &[], &[]);
    assert!(g.types_of(signet).creature && g.types_of(signet).artifact);
    assert_eq!(g.power(signet), 7);
}

#[test]
fn vorinclex_doubles_your_land_mana_and_freezes_opponents_lands() {
    let mut g = game(&["Vorinclex, Voice of Hunger", "Forest"]);
    let f = g.permanents(0).find(|&x| g.def_of(x).name == "Forest").unwrap();
    assert_eq!(g.mana_sources(0).iter().filter(|&&(x, _)| x == f).count(), 2);
    to_main1(&mut g, 4, 1);
    let think = hand(&mut g, 1, "Think Twice");
    act(&mut g, Action::Cast(think));
    resolve_all(&mut g, &[]);
    let tapped: Vec<ObjId> = g.permanents(1).filter(|&x| g.obj(x).tapped).collect();
    assert_eq!(tapped.len(), 2);
    assert!(tapped.iter().all(|&x| g.obj(x).skip_untap));
}

#[test]
fn blightsteel_colossus_deals_poison() {
    let mut g = game(&["Blightsteel Colossus"]);
    let b = g.permanents(0).next().unwrap();
    assert!(g.keywords(b).has(Keywords::INDESTRUCTIBLE) && g.keywords(b).has(Keywords::TRAMPLE));
    while g.step != Step::DeclareAttackers {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(b));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 && !g.is_over() {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[1].poison, 11);
    assert_eq!(g.players[1].life, 40);
}

#[test]
fn etali_exiles_until_nonland_casts_free_and_transforms() {
    let mut g = table();
    let mine = top(&mut g, "Think Twice");
    let theirs = g.add_card(1, cid("Hungry Ghoul"), Zone::Library);
    let etali = hand(&mut g, 0, "Etali, Primal Conqueror // Etali, Primal Sickness");
    play(&mut g, etali, false, &[], &[]);
    assert_eq!(g.obj(mine).zone, Zone::Exile);
    assert_eq!(g.obj(theirs).zone, Zone::Exile);
    assert!(g.legal_actions().contains(&Action::Cast(theirs)), "cast it free");
    act(&mut g, Action::Cast(theirs));
    for _ in 0..20 {
        if g.items.is_empty() && !matches!(g.pending, Pending::Decide(_) | Pending::ChooseTarget(_)) {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(theirs).zone, Zone::Battlefield);
    assert_eq!(g.obj(theirs).controller, 0);
    let mut lands = vec!["Etali, Primal Conqueror // Etali, Primal Sickness"];
    lands.extend(["Forest"; 10]);
    let mut g = game(&lands);
    let etali = g.permanents(0).find(|&x| g.types_of(x).creature).unwrap();
    let life = g.players[0].life;
    act(&mut g, Action::Activate(etali, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, life - 2, "{{G/P}} paid with 2 life");
    assert_eq!(g.def_of(etali).name, "Etali, Primal Sickness");
    assert!(g.keywords(etali).has(Keywords::INDESTRUCTIBLE));
    while g.step != Step::DeclareAttackers {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(etali));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 && !g.is_over() {
        let a = if matches!(g.pending, Pending::Decide(_)) {
            g.legal_actions()[0]
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[1].poison, 11);
}

#[test]
fn sneak_attack_puts_a_hasty_creature_in_until_end_of_turn() {
    let mut g = game(&["Sneak Attack", "Mountain"]);
    let sa = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Sneak Attack")
        .unwrap();
    let pal = hand(&mut g, 0, "Palinchron");
    act(&mut g, Action::Activate(sa, 0));
    resolve_all(&mut g, &[Action::Pick(pal)]);
    assert_eq!(g.obj(pal).zone, Zone::Battlefield);
    assert!(g.keywords(pal).has(Keywords::HASTE));
    while g.turn == 3 {
        let a = match g.legal_actions()[0] {
            d @ Action::Discard(_) => d,
            _ if matches!(g.pending, Pending::DeclareAttackers(_)) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(pal).zone, Zone::Graveyard);
}

#[test]
fn terror_of_the_peaks_throws_entering_creatures_power() {
    let mut g = table();
    let terror = put(&mut g, 0, "Terror of the Peaks");
    assert_eq!(g.def_of(terror).targeted_life_tax, 3);
    let ghoul = hand(&mut g, 0, "Hungry Ghoul");
    let life = g.players[1].life;
    play(&mut g, ghoul, false, &[], &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].life, life - 2);
}

#[test]
fn xenagos_the_reveler_makes_mana_satyrs_and_digs() {
    let mut g = table();
    put(&mut g, 0, "Hungry Ghoul");
    put(&mut g, 0, "Aegis Turtle");
    let x = hand(&mut g, 0, "Xenagos, the Reveler");
    play(&mut g, x, false, &[], &[]);
    g.manual_mana[0] = true;
    act(&mut g, Action::Loyalty(x, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].pool.len(), 2);
    assert!(g.players[0].pool.iter().all(|m| m.colors[3] && m.colors[4]));
    to_main1(&mut g, 5, 0);
    act(&mut g, Action::Loyalty(x, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Satyr").len(), 1);
    g.objects[x.0 as usize].loyalty = 6;
    let land = top(&mut g, "Forest");
    let beast = top(&mut g, "Avenger of Zendikar");
    top(&mut g, "Think Twice");
    to_main1(&mut g, 7, 0);
    act(&mut g, Action::Loyalty(x, 2));
    resolve_all(
        &mut g,
        &[Action::Pick(beast), Action::Pick(land), Action::No, Action::Yes],
    ); // then Avenger's landfall
    assert_eq!(g.obj(beast).zone, Zone::Battlefield);
    assert_eq!(g.obj(land).zone, Zone::Battlefield);
}

#[test]
fn fauna_shaman_discards_a_creature_to_tutor_one() {
    let mut g = game(&["Fauna Shaman", "Forest"]);
    let fs = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Fauna Shaman")
        .unwrap();
    let fodder = hand(&mut g, 0, "Aegis Turtle");
    let want = g.add_card(0, cid("Palinchron"), Zone::Library);
    act(&mut g, Action::ActivateWith(fs, 0, fodder));
    resolve_all(&mut g, &[Action::Pick(want)]);
    assert_eq!(g.obj(fodder).zone, Zone::Graveyard);
    assert_eq!(g.obj(want).zone, Zone::Hand);
}

#[test]
fn ghalta_puts_any_number_of_creatures_from_hand() {
    let mut g = table();
    let a = hand(&mut g, 0, "Aegis Turtle");
    let b = hand(&mut g, 0, "Hungry Ghoul");
    let gh = hand(&mut g, 0, "Ghalta, Stampede Tyrant");
    play(&mut g, gh, false, &[], &[Action::Pick(a), Action::Pick(b)]);
    assert_eq!(g.obj(a).zone, Zone::Battlefield);
    assert_eq!(g.obj(b).zone, Zone::Battlefield);
}

#[test]
fn maze_of_ith_untaps_an_attacker_and_stops_its_damage() {
    let mut lands = vec!["Hungry Ghoul"];
    lands.extend(["Forest"; 2]);
    let mut g2 = game(&lands);
    let ghoul = g2
        .permanents(0)
        .find(|&x| g2.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    let maze2 = put(&mut g2, 1, "Maze of Ith");
    while g2.step != Step::DeclareAttackers {
        act(&mut g2, Action::PassPriority);
    }
    act(&mut g2, Action::Attack(ghoul));
    act(&mut g2, Action::FinishAttacks);
    while !g2.legal_actions().contains(&Action::Activate(maze2, 0)) {
        act(&mut g2, Action::PassPriority);
    }
    act(&mut g2, Action::Activate(maze2, 0));
    act(&mut g2, Action::Target(obj(ghoul)));
    while g2.step != Step::Main2 {
        let a = match g2.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g2, a);
    }
    assert!(!g2.obj(ghoul).tapped);
    assert_eq!(g2.players[1].life, 40, "its combat damage is prevented");
}

#[test]
fn ohran_frostfang_gives_attackers_deathtouch_and_draws() {
    let mut g = game(&["Ohran Frostfang", "Aegis Turtle", "Hungry Ghoul"]);
    let ghoul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    let n = g.players[0].hand.len();
    while g.step != Step::DeclareAttackers {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(ghoul));
    act(&mut g, Action::FinishAttacks);
    assert!(g.keywords(ghoul).has(Keywords::DEATHTOUCH));
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[0].hand.len(), n + 1);
}

#[test]
fn temur_sabertooth_bounces_another_for_indestructible() {
    let mut g = table();
    let st = put(&mut g, 0, "Temur Sabertooth");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    act(&mut g, Action::Activate(st, 0));
    resolve_all(&mut g, &[Action::Pick(ghoul)]);
    assert_eq!(g.obj(ghoul).zone, Zone::Hand);
    assert!(g.keywords(st).has(Keywords::INDESTRUCTIBLE));
}

#[test]
fn the_fire_crystal_discounts_red_hastes_and_copies() {
    let mut g = game(&[
        "The Fire Crystal",
        "Mountain",
        "Mountain",
        "Mountain",
        "Mountain",
        "Mountain",
        "Mountain",
    ]);
    let fc = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "The Fire Crystal")
        .unwrap();
    let sneak = hand(&mut g, 0, "Sneak Attack"); // {3}{R}
    play(&mut g, sneak, false, &[], &[]);
    let untapped = g
        .permanents(0)
        .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
        .count();
    assert_eq!(untapped, 3, "{{2}}{{R}} paid");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    assert!(g.keywords(ghoul).has(Keywords::HASTE));
    let mut g = table();
    let fc2 = put(&mut g, 0, "The Fire Crystal");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    act(&mut g, Action::Activate(fc2, 0));
    resolve_all(&mut g, &[Action::Target(obj(ghoul))]);
    assert_eq!(
        g.permanents(0)
            .filter(|&x| g.def_of(x).name == "Hungry Ghoul")
            .count(),
        2
    );
    let _ = fc;
}

#[test]
fn cultivator_colossus_chains_lands_from_hand_and_draws() {
    let mut g = table();
    let a = hand(&mut g, 0, "Forest");
    let b = hand(&mut g, 0, "Island");
    let cc = hand(&mut g, 0, "Cultivator Colossus");
    let n = g.players[0].hand.len();
    play(
        &mut g,
        cc,
        false,
        &[],
        &[Action::Pick(a), Action::Pick(b), Action::No],
    );
    assert!(g.obj(a).tapped && g.obj(b).zone == Zone::Battlefield);
    assert_eq!(
        g.players[0].hand.len(),
        n - 1 - 2 + 2,
        "two lands out, two cards drawn"
    );
    assert_eq!(
        g.power(cc) as usize,
        g.permanents(0).filter(|&x| g.types_of(x).land).count()
    );
}

#[test]
fn terrain_generator_puts_a_basic_land_from_hand() {
    let mut g = table();
    let tg = put(&mut g, 0, "Terrain Generator");
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::Activate(tg, 0));
    resolve_all(&mut g, &[Action::Pick(f)]);
    assert_eq!(g.obj(f).zone, Zone::Battlefield);
    assert!(g.obj(f).tapped);
}

#[test]
fn the_reaver_cleaver_makes_treasures_for_combat_damage() {
    let mut g = game(&["Hungry Ghoul", "The Reaver Cleaver", "Forest", "Forest", "Forest"]);
    let ghoul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    let rc = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "The Reaver Cleaver")
        .unwrap();
    act(&mut g, Action::Activate(rc, 0));
    resolve_all(&mut g, &[Action::Target(obj(ghoul))]);
    assert_eq!(g.power(ghoul), 3);
    assert!(g.keywords(ghoul).has(Keywords::TRAMPLE));
    while g.step != Step::DeclareAttackers {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(ghoul));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(tokens(&g, 0, "Treasure").len(), 3);
}

#[test]
fn apex_devastator_cascades_four_times() {
    let mut g = table();
    for name in ["Hungry Ghoul", "Aegis Turtle", "Palinchron", "Burglar Rat"] {
        top(&mut g, name);
    }
    let ad = hand(&mut g, 0, "Apex Devastator");
    act(&mut g, Action::Cast(ad));
    for _ in 0..200 {
        if g.items.is_empty() && !matches!(g.pending, Pending::Decide(_) | Pending::ChooseTarget(_)) {
            break;
        }
        let a = g
            .legal_actions()
            .into_iter()
            .find(|a| matches!(a, Action::Cast(_)))
            .unwrap_or(Action::PassPriority);
        let a = if matches!(g.pending, Pending::Decide(_) | Pending::ChooseTarget(_)) {
            g.legal_actions()[0]
        } else {
            a
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(ad).zone, Zone::Battlefield);
    let cast: usize = ["Hungry Ghoul", "Aegis Turtle", "Palinchron", "Burglar Rat"]
        .iter()
        .map(|n| g.permanents(0).filter(|&x| g.def_of(x).name == *n).count())
        .sum();
    assert_eq!(cast, 4, "four cascades");
}

#[test]
fn orthion_copies_another_creature_with_haste() {
    let mut g = game(&[
        "Orthion, Hero of Lavabrink",
        "Hungry Ghoul",
        "Mountain",
        "Mountain",
    ]);
    let o = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Orthion, Hero of Lavabrink")
        .unwrap();
    let ghoul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    act(&mut g, Action::Activate(o, 0));
    resolve_all(&mut g, &[Action::Target(obj(ghoul))]);
    let copies: Vec<ObjId> = g
        .permanents(0)
        .filter(|&x| g.def_of(x).name == "Hungry Ghoul" && x != ghoul)
        .collect();
    assert_eq!(copies.len(), 1);
    assert!(g.keywords(copies[0]).has(Keywords::HASTE));
}

#[test]
fn sarkhans_unsealing_burns_for_big_creature_spells() {
    let mut g = table();
    put(&mut g, 0, "Sarkhan's Unsealing");
    let ghoul = hand(&mut g, 0, "Hungry Ghoul");
    play(&mut g, ghoul, false, &[], &[]); // power 2: nothing
    assert_eq!(g.players[1].life, 40);
    let pal = hand(&mut g, 0, "Palinchron"); // power 4
    act(&mut g, Action::Cast(pal));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].life, 36);
    let theirs = put(&mut g, 1, "Hungry Ghoul");
    let wurm = hand(&mut g, 0, "Worldspine Wurm"); // power 15
    act(&mut g, Action::Cast(wurm));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 32);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
}

#[test]
fn hellkite_courser_brings_out_the_commander_until_end_of_turn() {
    let mut g = table();
    let bello = g.commanders[0].expect("a commander");
    let hc = hand(&mut g, 0, "Hellkite Courser");
    play(&mut g, hc, false, &[], &[Action::Yes]);
    assert_eq!(g.obj(bello).zone, Zone::Battlefield);
    assert!(g.keywords(bello).has(Keywords::HASTE));
    while g.turn == 3 {
        let a = match g.legal_actions()[0] {
            d @ Action::Discard(_) => d,
            _ if matches!(g.pending, Pending::DeclareAttackers(_)) => Action::FinishAttacks,
            _ if matches!(g.pending, Pending::Decide(_)) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(bello).zone, Zone::Command);
}

#[test]
fn protean_hulk_finds_creatures_with_total_mana_value_six() {
    let mut g = table();
    let hulk = put(&mut g, 0, "Protean Hulk");
    let a = g.add_card(0, cid("Fauna Shaman"), Zone::Library); // 2
    let b = g.add_card(0, cid("Temur Sabertooth"), Zone::Library); // 4
    let c = g.add_card(0, cid("Hungry Ghoul"), Zone::Library); // 2
    let dam = hand(&mut g, 0, "Damnation");
    act(&mut g, Action::Cast(dam));
    resolve_all(&mut g, &[Action::Pick(a), Action::Pick(b)]);
    assert_eq!(g.obj(hulk).zone, Zone::Graveyard);
    assert_eq!(g.obj(a).zone, Zone::Battlefield);
    assert_eq!(g.obj(b).zone, Zone::Battlefield);
    assert_eq!(g.obj(c).zone, Zone::Library, "over the total");
}

#[test]
fn selvala_draws_for_the_biggest_and_makes_mana_by_power() {
    let mut g = table();
    let s = put(&mut g, 0, "Selvala, Heart of the Wilds");
    let n = g.players[0].hand.len();
    let pal = hand(&mut g, 0, "Palinchron");
    play(&mut g, pal, false, &[], &[]);
    assert_eq!(
        g.players[0].hand.len(),
        n + 1,
        "Palinchron cast (from n + 1), then a card: the biggest"
    );
    g.manual_mana[0] = true;
    act(&mut g, Action::Activate(s, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].pool.len(), 4, "Palinchron's power");
}

#[test]
fn vigor_turns_damage_to_your_other_creatures_into_counters() {
    let mut lands = vec!["Hungry Ghoul", "Vigor"];
    lands.extend(["Forest"; 3]);
    let mut g = game(&lands);
    let ghoul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    let theirs = put(&mut g, 1, "Hungry Ghoul");
    let k = hand(&mut g, 0, "Khalni Ambush // Khalni Territory");
    play(&mut g, k, false, &[obj(ghoul), obj(theirs)], &[]);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    assert_eq!(g.power(ghoul), 4, "2 damage prevented: two counters");
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
}

#[test]
fn molten_echoes_copies_creatures_of_the_chosen_type() {
    let mut g = table();
    let ghoul = hand(&mut g, 0, "Hungry Ghoul"); // a Zombie
    let me = hand(&mut g, 0, "Molten Echoes");
    act(&mut g, Action::Cast(me));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    let zombie = g
        .legal_actions()
        .into_iter()
        .find(|&a| {
            if let Action::Mode(m) = a {
                matches!(&g.decision, Some(bolt_core::Decision::ChooseCreatureType { options, .. }) if options[m as usize] == "Zombie")
            } else {
                false
            }
        })
        .expect("Zombie is an option");
    act(&mut g, zombie);
    play(&mut g, ghoul, false, &[], &[]);
    let copies: Vec<ObjId> = g
        .permanents(0)
        .filter(|&x| g.def_of(x).name == "Hungry Ghoul" && x != ghoul)
        .collect();
    assert_eq!(copies.len(), 1);
    assert!(g.keywords(copies[0]).has(Keywords::HASTE));
}

#[test]
fn primal_surge_keeps_putting_permanents_onto_the_battlefield() {
    let mut g = table();
    let think = top(&mut g, "Think Twice");
    let a = top(&mut g, "Hungry Ghoul");
    let b = top(&mut g, "Forest");
    let ps = hand(&mut g, 0, "Primal Surge");
    play(&mut g, ps, false, &[], &[Action::Yes, Action::Yes]);
    assert_eq!(g.obj(b).zone, Zone::Battlefield);
    assert_eq!(g.obj(a).zone, Zone::Battlefield);
    assert_eq!(g.obj(think).zone, Zone::Exile, "not a permanent: it stops");
}

#[test]
fn flameshadow_conjuring_copies_for_r() {
    let mut g = table();
    put(&mut g, 0, "Flameshadow Conjuring");
    let ghoul = hand(&mut g, 0, "Hungry Ghoul");
    play(&mut g, ghoul, false, &[], &[Action::Yes]);
    assert_eq!(
        g.permanents(0)
            .filter(|&x| g.def_of(x).name == "Hungry Ghoul")
            .count(),
        2
    );
}

#[test]
fn mirror_march_copies_for_each_coin_flip_won() {
    let mut g = table();
    put(&mut g, 0, "Mirror March");
    let ghoul = hand(&mut g, 0, "Hungry Ghoul");
    play(&mut g, ghoul, false, &[], &[]);
    let copies = g
        .permanents(0)
        .filter(|&x| g.def_of(x).name == "Hungry Ghoul" && x != ghoul)
        .count();
    assert!(copies < 50);
    assert!(g
        .permanents(0)
        .filter(|&x| g.is_token(x))
        .all(|x| g.keywords(x).has(Keywords::HASTE)));
}

#[test]
fn rionya_copies_another_creature_at_combat() {
    let mut g = game(&["Rionya, Fire Dancer", "Hungry Ghoul"]);
    let ghoul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        act(&mut g, Action::PassPriority);
    }
    resolve_all(&mut g, &[Action::Target(obj(ghoul))]);
    assert_eq!(
        g.permanents(0)
            .filter(|&x| g.def_of(x).name == "Hungry Ghoul")
            .count(),
        2
    );
}

#[test]
fn twinflame_strive_pays_more_per_extra_target() {
    let mut g = table();
    let a = put(&mut g, 0, "Hungry Ghoul");
    let b = put(&mut g, 0, "Aegis Turtle");
    let tf = hand(&mut g, 0, "Twinflame");
    let before = g
        .permanents(0)
        .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
        .count();
    act(&mut g, Action::CastModal(tf, 1));
    resolve_all(&mut g, &[Action::Target(obj(a)), Action::Target(obj(b))]);
    let after = g
        .permanents(0)
        .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
        .count();
    assert_eq!(before - after, 5, "{{1}}{{R}} plus {{2}}{{R}}");
    assert_eq!(g.permanents(0).filter(|&x| g.is_token(x)).count(), 2);
}

#[test]
fn mosswort_bridge_hides_a_card_and_plays_it_with_ten_power() {
    let mut g = table();
    let hidden = top(&mut g, "Palinchron");
    for _ in 0..3 {
        top(&mut g, "Forest");
    }
    let mb = hand(&mut g, 0, "Mosswort Bridge");
    act(&mut g, Action::PlayLand(mb));
    resolve_all(&mut g, &[Action::Pick(hidden)]);
    assert!(g.obj(mb).tapped);
    assert_eq!(g.obj(hidden).zone, Zone::Exile);
    put(&mut g, 0, "Worldspine Wurm");
    to_main1(&mut g, 5, 0);
    act(&mut g, Action::Activate(mb, 0));
    resolve_all(&mut g, &[]);
    assert!(g.legal_actions().contains(&Action::Cast(hidden)));
    play(&mut g, hidden, false, &[], &[]);
    assert_eq!(g.obj(hidden).zone, Zone::Battlefield);
}

#[test]
fn spinerock_knoll_plays_its_card_after_seven_damage() {
    let mut g = table();
    let hidden = top(&mut g, "Think Twice");
    let sk = hand(&mut g, 0, "Spinerock Knoll");
    act(&mut g, Action::PlayLand(sk));
    resolve_all(&mut g, &[Action::Pick(hidden)]);
    to_main1(&mut g, 5, 0);
    put(&mut g, 0, "Sarkhan's Unsealing");
    let pal = hand(&mut g, 0, "Palinchron"); // power 4: 4 damage
    act(&mut g, Action::Cast(pal));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    let pal2 = hand(&mut g, 0, "Palinchron");
    act(&mut g, Action::Cast(pal2));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].life, 32, "8 damage this turn");
    act(&mut g, Action::Activate(sk, 0));
    resolve_all(&mut g, &[]);
    assert!(g.legal_actions().contains(&Action::Cast(hidden)));
}

#[test]
fn cursed_mirror_copies_a_creature_until_end_of_turn_with_haste() {
    let mut g = table();
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let cm = hand(&mut g, 0, "Cursed Mirror");
    play(&mut g, cm, false, &[], &[Action::Pick(ghoul)]);
    assert_eq!(g.def_of(cm).name, "Hungry Ghoul");
    assert!(g.keywords(cm).has(Keywords::HASTE));
    while g.turn == 3 {
        let a = match g.legal_actions()[0] {
            d @ Action::Discard(_) => d,
            _ if matches!(g.pending, Pending::DeclareAttackers(_)) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.def_of(cm).name, "Cursed Mirror");
}

#[test]
fn wand_of_wonder_casts_an_opponents_instant_free() {
    let mut g = table();
    let wand = put(&mut g, 0, "Wand of Wonder");
    let theirs = g.add_card(1, cid("Think Twice"), Zone::Library);
    act(&mut g, Action::Activate(wand, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Exile);
    assert!(g.legal_actions().contains(&Action::Cast(theirs)));
}

#[test]
fn vivien_casts_creatures_from_the_top_and_tutors_a_smaller_one() {
    let mut g = table();
    let v = hand(&mut g, 0, "Vivien, Monsters' Advocate");
    play(&mut g, v, false, &[], &[]);
    let pal = top(&mut g, "Palinchron");
    assert!(
        g.legal_actions().contains(&Action::Cast(pal)),
        "creature spells from the top"
    );
    act(&mut g, Action::Loyalty(v, 0));
    resolve_all(&mut g, &[Action::Mode(2)]);
    let beast = tokens(&g, 0, "Beast");
    assert_eq!(beast.len(), 1);
    assert!(g.keywords(beast[0]).has(Keywords::TRAMPLE));
    to_main1(&mut g, 5, 0);
    let small = g.add_card(0, cid("Hungry Ghoul"), Zone::Library);
    act(&mut g, Action::Loyalty(v, 1));
    resolve_all(&mut g, &[]);
    let pal2 = hand(&mut g, 0, "Palinchron");
    act(&mut g, Action::Cast(pal2));
    resolve_all(&mut g, &[Action::Pick(small)]);
    assert_eq!(g.obj(small).zone, Zone::Battlefield);
}

#[test]
fn delina_makes_attacking_copies_exiled_at_end_of_combat() {
    let mut g = game(&["Delina, Wild Mage", "Hungry Ghoul"]);
    let delina = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Delina, Wild Mage")
        .unwrap();
    let ghoul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    while g.step != Step::DeclareAttackers {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(delina));
    act(&mut g, Action::FinishAttacks);
    act(&mut g, Action::Target(obj(ghoul)));
    let mut made = 0;
    while g.step == Step::DeclareAttackers && (!g.items.is_empty() || matches!(g.pending, Pending::Decide(_)))
    {
        let a = if matches!(g.pending, Pending::Decide(_)) {
            Action::No
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
    }
    made += g.permanents(0).filter(|&x| g.is_token(x)).count();
    assert!(made >= 1);
    let t = g.permanents(0).find(|&x| g.is_token(x)).unwrap();
    assert!(g.obj(t).tapped && !g.types_of(t).legendary);
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::Decide(_) => Action::No,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(g.permanents(0).all(|x| !g.is_token(x)), "exiled at end of combat");
}

#[test]
fn jaxis_copies_another_creature_and_has_blitz() {
    let mut g = game(&["Jaxis, the Troublemaker", "Hungry Ghoul", "Mountain"]);
    let jaxis = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Jaxis, the Troublemaker")
        .unwrap();
    let ghoul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    let card = hand(&mut g, 0, "Forest");
    act(&mut g, Action::ActivateWith(jaxis, 0, card));
    resolve_all(&mut g, &[Action::Target(obj(ghoul))]);
    assert_eq!(g.obj(card).zone, Zone::Graveyard, "discarded");
    let copy = g.permanents(0).find(|&x| g.is_token(x)).expect("a copy");
    assert!(g.keywords(copy).has(Keywords::HASTE));
    let mut g = table();
    let j = hand(&mut g, 0, "Jaxis, the Troublemaker");
    act(&mut g, Action::CastAlternative(j));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(j).has(Keywords::HASTE));
    assert!(g.obj(j).sacrifice_at_end);
}

#[test]
fn zopandrel_doubles_at_combat_and_eats_two_for_indestructible() {
    let mut g = game(&[
        "Zopandrel, Hunger Dominus",
        "Hungry Ghoul",
        "Aegis Turtle",
        "Forest",
        "Forest",
    ]);
    let z = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Zopandrel, Hunger Dominus")
        .unwrap();
    let ghoul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    act(&mut g, Action::ActivateWith(z, 0, ghoul));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ghoul).zone, Zone::Graveyard);
    assert_eq!(
        g.permanents(0).filter(|&x| g.types_of(x).creature).count(),
        1,
        "the turtle too"
    );
    assert!(g.keywords(z).has(Keywords::INDESTRUCTIBLE));
    while g.step != Step::BeginCombat {
        act(&mut g, Action::PassPriority);
    }
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(z), 8);
}

#[test]
fn mirage_phalanx_pairs_and_copies_both_at_combat() {
    let mut g = game(&[
        "Hungry Ghoul",
        "Mountain",
        "Mountain",
        "Mountain",
        "Mountain",
        "Mountain",
        "Mountain",
    ]);
    let ghoul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    let mp = hand(&mut g, 0, "Mirage Phalanx");
    play(&mut g, mp, false, &[], &[Action::Yes, Action::Pick(ghoul)]);
    assert_eq!(g.paired_partner(mp), Some(ghoul));
    while g.step != Step::BeginCombat {
        act(&mut g, Action::PassPriority);
    }
    while !g.items.is_empty() || matches!(g.pending, Pending::Decide(_)) {
        let a = if matches!(g.pending, Pending::Decide(_)) {
            Action::No
        } else {
            Action::PassPriority
        };
        act(&mut g, a); // the "may pair" triggers of the copies entering: already paired
    }
    let toks: Vec<ObjId> = g.permanents(0).filter(|&x| g.is_token(x)).collect();
    assert_eq!(toks.len(), 2);
    assert!(toks.iter().all(|&t| g.keywords(t).has(Keywords::HASTE)));
}

#[test]
fn ezuris_predation_makes_a_beast_to_fight_each_opponent_creature() {
    let mut g = table();
    let a = put(&mut g, 1, "Hungry Ghoul");
    let b = put(&mut g, 1, "Palinchron");
    let ep = hand(&mut g, 0, "Ezuri's Predation");
    play(&mut g, ep, false, &[], &[]);
    assert_eq!(g.obj(a).zone, Zone::Graveyard, "a 2/2 against a 4/4");
    assert_eq!(g.obj(b).zone, Zone::Battlefield, "Palinchron is 4/5");
    assert_eq!(
        tokens(&g, 0, "Phyrexian Beast").len(),
        1,
        "the one that fought Palinchron died"
    );
}

// ---- Malik, Grim Manipulator, batch 1 ----

/// Resolves the stack, answering each decision with its first legal choice.
fn first_answers(g: &mut Game) {
    for _ in 0..40 {
        if g.items.is_empty() && !matches!(g.pending, Pending::Decide(_) | Pending::ChooseTarget(_)) {
            break;
        }
        let a = match g.pending {
            Pending::Decide(_) | Pending::ChooseTarget(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

fn enters_tapped(lands: &[&str], name: &str) -> bool {
    let mut g = game(lands);
    let c = hand(&mut g, 0, name);
    act(&mut g, Action::PlayLand(c));
    g.obj(c).tapped
}

#[test]
fn diabolic_tutor_finds_any_card() {
    let mut g = table();
    let ring = top(&mut g, "Sol Ring");
    let dt = hand(&mut g, 0, "Diabolic Tutor");
    play(&mut g, dt, false, &[], &[Action::Pick(ring)]);
    assert_eq!(g.obj(ring).zone, Zone::Hand);
}

#[test]
fn slow_lands_enter_untapped_with_two_other_lands() {
    for name in ["Sundown Pass", "Shattered Sanctum"] {
        assert!(enters_tapped(&["Plains"], name), "{name}");
        assert!(!enters_tapped(&["Plains", "Swamp"], name), "{name}");
    }
    let mut g = game(&[]);
    let c = put(&mut g, 0, "Shattered Sanctum");
    assert_eq!(colors(&g, c), [true, false, true, false, false]);
}

#[test]
fn inspiring_vantage_enters_untapped_with_two_or_fewer_other_lands() {
    assert!(!enters_tapped(&["Plains", "Mountain"], "Inspiring Vantage"));
    assert!(enters_tapped(
        &["Plains", "Mountain", "Plains"],
        "Inspiring Vantage"
    ));
}

#[test]
fn smoldering_marsh_enters_untapped_with_two_basics() {
    assert!(enters_tapped(&["Swamp"], "Smoldering Marsh"));
    assert!(!enters_tapped(&["Swamp", "Mountain"], "Smoldering Marsh"));
}

#[test]
fn the_pathways_tap_for_their_front_color() {
    let mut g = game(&[]);
    let b = put(&mut g, 0, "Blightstep Pathway // Searstep Pathway");
    let r = put(&mut g, 0, "Needleverge Pathway // Pillarverge Pathway");
    assert_eq!(colors(&g, b), [false, false, true, false, false]);
    assert_eq!(colors(&g, r), [false, false, false, true, false]);
}

#[test]
fn merciless_executioner_makes_each_player_sacrifice() {
    let mut g = table();
    let mine = put(&mut g, 0, "Hungry Ghoul");
    let theirs = put(&mut g, 1, "Palinchron");
    let me = hand(&mut g, 0, "Merciless Executioner");
    act(&mut g, Action::Cast(me));
    first_answers(&mut g);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    let left = [me, mine]
        .iter()
        .filter(|&&x| g.obj(x).zone == Zone::Battlefield)
        .count();
    assert_eq!(left, 1, "you sacrifice one of your two");
}

#[test]
fn massacre_wurm_shrinks_their_creatures_and_drains_for_each_death() {
    let mut g = table();
    put(&mut g, 1, "Hungry Ghoul"); // 1/1 dies
    let palin = put(&mut g, 1, "Palinchron"); // 4/5 lives
    let life = g.players[1].life;
    let mw = hand(&mut g, 0, "Massacre Wurm");
    play(&mut g, mw, false, &[], &[]);
    assert_eq!(g.players[1].life, life - 2);
    assert_eq!(g.obj(palin).zone, Zone::Battlefield);
    assert_eq!(g.power(palin), 2);
}

#[test]
fn butcher_of_malakir_makes_them_sacrifice_when_yours_die() {
    let mut g = table();
    put(&mut g, 0, "Butcher of Malakir");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let theirs = put(&mut g, 1, "Palinchron");
    let kill = hand(&mut g, 0, "Hero's Downfall");
    act(&mut g, Action::Cast(kill));
    act(&mut g, Action::Target(obj(ghoul)));
    first_answers(&mut g);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
}

#[test]
fn reckless_fireweaver_pings_for_each_artifact() {
    let mut g = table();
    put(&mut g, 0, "Reckless Fireweaver");
    let life = g.players[1].life;
    let ring = hand(&mut g, 0, "Sol Ring");
    play(&mut g, ring, false, &[], &[]);
    assert_eq!(g.players[1].life, life - 1);
}

#[test]
fn mayhem_devil_pings_when_a_permanent_is_sacrificed() {
    let mut g = table();
    put(&mut g, 0, "Mayhem Devil");
    let fh = put(&mut g, 0, "Floodhound");
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Activate(fh, 0));
    resolve_all(&mut g, &[]);
    let clue = tokens(&g, 0, "Clue")[0];
    let life = g.players[1].life;
    act(&mut g, Action::Activate(clue, 0));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].life, life - 1);
}

#[test]
fn teleportation_circle_blinks_at_your_end_step() {
    let mut g = table();
    put(&mut g, 0, "Teleportation Circle");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let zcc = g.obj(ghoul).zcc;
    let mut targeted = false;
    for _ in 0..40 {
        if g.step == Step::Cleanup || g.turn != 3 {
            break;
        }
        let a = match g.pending {
            Pending::ChooseTarget(_) if !targeted => {
                targeted = true;
                Action::Target(obj(ghoul))
            }
            Pending::Decide(_) | Pending::ChooseTarget(_) => g.legal_actions()[0],
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(targeted);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    assert!(g.obj(ghoul).zcc > zcc, "exiled and returned");
}

#[test]
fn crackling_doom_deals_two_and_takes_their_biggest() {
    let mut g = table();
    let small = put(&mut g, 1, "Hungry Ghoul");
    let big = put(&mut g, 1, "Palinchron");
    let life = g.players[1].life;
    let cd = hand(&mut g, 0, "Crackling Doom");
    play(&mut g, cd, false, &[], &[]);
    assert_eq!(g.players[1].life, life - 2);
    assert_eq!(g.obj(big).zone, Zone::Graveyard);
    assert_eq!(g.obj(small).zone, Zone::Battlefield);
}

#[test]
fn charming_prince_gains_three_life() {
    let mut g = table();
    let life = g.players[0].life;
    let cp = hand(&mut g, 0, "Charming Prince");
    play(&mut g, cp, false, &[], &[Action::Mode(1)]);
    assert_eq!(g.players[0].life, life + 3);
}

#[test]
fn ephemerate_blinks_and_rebounds() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let zcc = g.obj(ghoul).zcc;
    let e = hand(&mut g, 0, "Ephemerate");
    play(&mut g, e, false, &[obj(ghoul)], &[]);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    assert!(g.obj(ghoul).zcc > zcc);
    assert_eq!(g.obj(e).zone, Zone::Exile, "rebound");
}

#[test]
fn treasure_vault_makes_x_treasures() {
    let mut g = table();
    let tv = put(&mut g, 0, "Treasure Vault");
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(
        &mut g,
        Action::ActivateX {
            id: tv,
            n: 0,
            victim: None,
            x: 2,
        },
    );
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 2);
    assert_eq!(g.obj(tv).zone, Zone::Graveyard);
}

// ---- Malik, batch 2 ----

fn to_main(g: &mut Game, turn: u32, active: usize) {
    g.start_turn(turn, active);
    while g.step != Step::Main1 {
        let legal = g.legal_actions();
        let a = if legal.contains(&Action::PassPriority) {
            Action::PassPriority
        } else if legal.contains(&Action::FinishAttacks) {
            Action::FinishAttacks
        } else {
            legal[0]
        };
        act(g, a);
    }
}

#[test]
fn nahiris_resolve_pumps_and_exiles_until_your_next_upkeep() {
    let mut g = table();
    put(&mut g, 0, "Nahiri's Resolve");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    assert_eq!(g.power(ghoul), g.def_of(ghoul).power + 1);
    assert!(g.keywords(ghoul).has(Keywords::HASTE));
    let mut exiled = false;
    for _ in 0..40 {
        if g.turn != 3 {
            break;
        }
        let legal = g.legal_actions();
        let a = if legal.contains(&Action::Pick(ghoul)) && !exiled {
            exiled = true;
            Action::Pick(ghoul)
        } else if legal.contains(&Action::PassPriority) {
            Action::PassPriority
        } else if legal.contains(&Action::FinishAttacks) {
            Action::FinishAttacks
        } else {
            legal[0]
        };
        act(&mut g, a);
    }
    assert!(exiled);
    assert_eq!(g.obj(ghoul).zone, Zone::Exile);
    to_main(&mut g, 5, 0);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield, "back at your next upkeep");
}

#[test]
fn treasure_map_transforms_on_its_third_landmark_into_treasure_cove() {
    let mut g = table();
    let map = put(&mut g, 0, "Treasure Map // Treasure Cove");
    for turn in [5, 7, 9] {
        to_main(&mut g, turn, 0);
        act(&mut g, Action::Activate(map, 0));
        first_answers(&mut g);
    }
    assert_eq!(g.def_of(map).name, "Treasure Cove");
    let treasures = tokens(&g, 0, "Treasure");
    assert_eq!(treasures.len(), 3);
    to_main(&mut g, 11, 0);
    let n = g.players[0].hand.len();
    act(&mut g, Action::ActivateWith(map, 0, treasures[0]));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n + 1);
}

// ---- Bruna, Light of Alabaster, batch 1 ----

#[test]
fn supreme_verdict_destroys_all_creatures_and_cant_be_countered() {
    let mut g = table();
    let a = put(&mut g, 0, "Hungry Ghoul");
    let b = put(&mut g, 1, "Palinchron");
    let sv = hand(&mut g, 0, "Supreme Verdict");
    assert!(g.def_of(sv).uncounterable);
    play(&mut g, sv, false, &[], &[]);
    assert_eq!(g.obj(a).zone, Zone::Graveyard);
    assert_eq!(g.obj(b).zone, Zone::Graveyard);
}

#[test]
fn idyllic_tutor_finds_an_enchantment() {
    let mut g = table();
    let e = top(&mut g, "Nahiri's Resolve");
    let it = hand(&mut g, 0, "Idyllic Tutor");
    play(&mut g, it, false, &[], &[Action::Pick(e)]);
    assert_eq!(g.obj(e).zone, Zone::Hand);
}

#[test]
fn talisman_of_progress_and_deserted_beach_make_white_or_blue() {
    let mut g = game(&[]);
    let t = put(&mut g, 0, "Talisman of Progress");
    assert_eq!(colors(&g, t), [true, true, false, false, false]);
    assert!(enters_tapped(&["Plains"], "Deserted Beach"));
    assert!(!enters_tapped(&["Plains", "Island"], "Deserted Beach"));
}

#[test]
fn prismatic_vista_pays_a_life_for_a_basic() {
    let mut g = game(&["Prismatic Vista"]);
    let pv = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Prismatic Vista")
        .unwrap();
    let forest = top(&mut g, "Forest");
    let life = g.players[0].life;
    act(&mut g, Action::Activate(pv, 0));
    resolve_all(&mut g, &[Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(!g.obj(forest).tapped);
    assert_eq!(g.players[0].life, life - 1);
}

#[test]
fn dovins_veto_counters_a_noncreature_spell() {
    let mut g = table();
    let think = hand(&mut g, 0, "Think Twice");
    let dv = hand(&mut g, 0, "Dovin's Veto");
    act(&mut g, Action::Cast(think));
    act(&mut g, Action::Cast(dv));
    resolve_all(&mut g, &[Action::Target(obj(think))]);
    assert_eq!(g.obj(think).zone, Zone::Graveyard);
    assert!(g.def_of(dv).uncounterable);
}

#[test]
fn eel_umbra_pumps_and_saves_with_umbra_armor() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let eu = hand(&mut g, 0, "Eel Umbra");
    let before = g.power(ghoul);
    play(&mut g, eu, false, &[obj(ghoul)], &[]);
    assert_eq!(g.power(ghoul), before + 1);
    let kill = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, kill, false, &[obj(ghoul)], &[]);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield, "umbra armor");
    assert_eq!(g.obj(eu).zone, Zone::Graveyard);
}

#[test]
fn crashing_drawbridge_gives_your_creatures_haste() {
    let mut g = table();
    let cd = put(&mut g, 0, "Crashing Drawbridge");
    to_main(&mut g, 5, 0);
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    act(&mut g, Action::Activate(cd, 0));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(ghoul).has(Keywords::HASTE));
}

#[test]
fn heliods_pilgrim_may_find_an_aura() {
    let mut g = table();
    let aura = top(&mut g, "Eel Umbra");
    let hp = hand(&mut g, 0, "Heliod's Pilgrim");
    play(&mut g, hp, false, &[], &[Action::Yes, Action::Pick(aura)]);
    assert_eq!(g.obj(aura).zone, Zone::Hand);
}

#[test]
fn galadriels_dismissal_phases_out_a_creature_or_kicked_all_of_a_players() {
    let mut g = table();
    let a = put(&mut g, 1, "Hungry Ghoul");
    let b = put(&mut g, 1, "Palinchron");
    let gd = hand(&mut g, 0, "Galadriel's Dismissal");
    play(&mut g, gd, false, &[obj(a)], &[]);
    assert!(
        !g.battlefield.contains(&a) && g.obj(a).zone == Zone::Battlefield,
        "phased out"
    );
    assert!(g.battlefield.contains(&b));
    let gd2 = hand(&mut g, 0, "Galadriel's Dismissal");
    play(&mut g, gd2, true, &[Target::Player(1)], &[]);
    assert!(!g.battlefield.contains(&b), "kicked: all of their creatures");
    to_main(&mut g, 4, 1);
    assert!(
        g.battlefield.contains(&a) && g.battlefield.contains(&b),
        "back on their untap step"
    );
}

#[test]
fn mirrormade_copies_an_artifact() {
    let mut g = table();
    put(&mut g, 1, "Sol Ring");
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let mm = hand(&mut g, 0, "Mirrormade");
    act(&mut g, Action::Cast(mm));
    for _ in 0..10 {
        if matches!(g.pending, Pending::Decide(_)) {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Pick(ghoul)),
        "not a creature"
    );
    let ring = g.permanents(1).find(|&x| g.def_of(x).name == "Sol Ring").unwrap();
    act(&mut g, Action::Pick(ring));
    resolve_all(&mut g, &[]);
    assert_eq!(g.def_of(mm).name, "Sol Ring");
}

// ---- Olivia, Crimson Bride, batch 1 ----

#[test]
fn rakdos_signet_pays_for_a_black_red_spell() {
    // {B}{R} from a Swamp and the Signet: the Swamp pays the Signet's {1}.
    let mut g = game(&["Swamp", "Rakdos Signet"]);
    let fl = hand(&mut g, 0, "Faithless Looting"); // {R}: only the Signet makes red
    assert!(g.legal_actions().contains(&Action::Cast(fl)));
}

#[test]
fn wayfarers_bauble_fetches_a_basic_tapped() {
    let mut g = table();
    let wb = put(&mut g, 0, "Wayfarer's Bauble");
    let forest = top(&mut g, "Forest");
    act(&mut g, Action::Activate(wb, 0));
    resolve_all(&mut g, &[Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(g.obj(forest).tapped);
}

#[test]
fn faithless_looting_draws_two_discards_two_and_has_flashback() {
    let mut g = table();
    let fl = hand(&mut g, 0, "Faithless Looting");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Cast(fl));
    first_answers(&mut g);
    assert_eq!(g.players[0].hand.len(), n - 1);
    assert!(g.legal_actions().contains(&Action::CastFlashback(fl)));
}

#[test]
fn the_discard_to_draw_spells_draw_two() {
    for name in [
        "Tormenting Voice",
        "Electric Revelation",
        "Seize the Spoils",
        "Unexpected Windfall",
    ] {
        let mut g = table();
        let c = hand(&mut g, 0, name);
        let junk = hand(&mut g, 0, "Forest");
        let n = g.players[0].hand.len();
        act(&mut g, Action::CastWith(c, junk));
        resolve_all(&mut g, &[]);
        assert_eq!(g.obj(junk).zone, Zone::Graveyard, "{name}");
        assert_eq!(
            g.players[0].hand.len(),
            n,
            "{name}: two cast or discarded, two drawn"
        );
    }
    let mut g = table();
    let c = hand(&mut g, 0, "Unexpected Windfall");
    let junk = hand(&mut g, 0, "Forest");
    act(&mut g, Action::CastWith(c, junk));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 2);
}

#[test]
fn cathartic_pyre_deals_three_or_cycles_two() {
    let mut g = table();
    let palin = put(&mut g, 1, "Palinchron");
    let cp = hand(&mut g, 0, "Cathartic Pyre");
    act(&mut g, Action::CastModal(cp, 0));
    act(&mut g, Action::Target(obj(palin)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(palin).damage, 3);
}

#[test]
fn foreboding_ruins_enters_untapped_revealing_a_swamp() {
    let mut g = table();
    hand(&mut g, 0, "Swamp");
    let fr = hand(&mut g, 0, "Foreboding Ruins");
    act(&mut g, Action::PlayLand(fr));
    first_answers(&mut g);
    assert!(!g.obj(fr).tapped);
}

// ---- Vaevictis Asmadi, the Dire, batch 1 ----

#[test]
fn tranquility_destroys_all_enchantments() {
    let mut g = table();
    let a = put(&mut g, 0, "Nahiri's Resolve");
    let b = put(&mut g, 1, "Mirrormade");
    let t = hand(&mut g, 0, "Tranquility");
    play(&mut g, t, false, &[], &[]);
    assert_eq!(g.obj(a).zone, Zone::Graveyard);
    assert_eq!(g.obj(b).zone, Zone::Graveyard);
}

#[test]
fn vesuva_enters_tapped_as_a_copy_of_a_land_or_untapped_as_itself() {
    let mut g = table();
    let v = hand(&mut g, 0, "Vesuva");
    let pool = g.permanents(1).find(|&x| g.def_of(x).name == "Island").unwrap();
    act(&mut g, Action::PlayLand(v));
    assert!(matches!(g.pending, Pending::Decide(_)));
    act(&mut g, Action::Pick(pool));
    assert_eq!(g.def_of(v).name, "Island");
    assert!(g.obj(v).tapped);
    let mut g = table();
    let v = hand(&mut g, 0, "Vesuva");
    act(&mut g, Action::PlayLand(v));
    act(&mut g, Action::No);
    assert_eq!(g.def_of(v).name, "Vesuva");
    assert!(!g.obj(v).tapped);
}

// ---- Olivia, batch 2: the Wicked Role ----

#[test]
fn charming_scoundrel_makes_a_wicked_role_that_drains_when_it_goes() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let before = g.power(ghoul);
    let cs = hand(&mut g, 0, "Charming Scoundrel");
    play(
        &mut g,
        cs,
        false,
        &[],
        &[Action::Mode(2), Action::Target(obj(ghoul))],
    );
    let role = tokens(&g, 0, "Wicked Role");
    assert_eq!(role.len(), 1);
    assert_eq!(g.power(ghoul), before + 1);
    let life = g.players[1].life;
    let kill = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, kill, false, &[obj(ghoul)], &[]);
    assert_eq!(g.players[1].life, life - 1, "the Role went to the graveyard");
}

#[test]
fn witchs_mark_loots_two_and_a_second_role_replaces_the_first() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let junk = hand(&mut g, 0, "Forest");
    let wm = hand(&mut g, 0, "Witch's Mark");
    let n = g.players[0].hand.len();
    play(&mut g, wm, false, &[obj(ghoul)], &[Action::Discard(junk)]);
    assert_eq!(
        g.players[0].hand.len(),
        n - 2 + 2,
        "cast, discarded one, drew two"
    );
    let life = g.players[1].life;
    let wm2 = hand(&mut g, 0, "Witch's Mark");
    play(&mut g, wm2, false, &[obj(ghoul)], &[Action::No]);
    assert_eq!(
        tokens(&g, 0, "Wicked Role").len(),
        1,
        "one Role of yours per creature"
    );
    assert_eq!(g.players[1].life, life - 1, "the old Role drained as it went");
}

// ---- Mac C: Auras put onto the battlefield attached ----

fn to_attackers(g: &mut Game) {
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
}

fn picks(g: &Game) -> Vec<ObjId> {
    g.legal_actions()
        .into_iter()
        .filter_map(|a| match a {
            Action::Pick(c) => Some(c),
            _ => None,
        })
        .collect()
}

#[test]
fn bruna_attaches_auras_from_the_battlefield_graveyard_and_hand_when_she_attacks() {
    let mut g = table();
    let bruna = put(&mut g, 0, "Bruna, Light of Alabaster");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let eel = hand(&mut g, 0, "Eel Umbra");
    let gz = g.obj(ghoul).zcc;
    assert!(g.put_aura_attached(eel, ghoul, gz, 0));
    let ws = hand(&mut g, 0, "Wingspan Stride");
    let armor = g.add_card(0, cid("Blanchwood Armor"), Zone::Graveyard);
    let land = hand(&mut g, 0, "Forest");
    to_attackers(&mut g);
    act(&mut g, Action::Attack(bruna));
    act(&mut g, Action::FinishAttacks);
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    let offered = picks(&g);
    for c in [eel, ws, armor] {
        assert!(offered.contains(&c), "every Aura that could enchant her");
    }
    assert!(!offered.contains(&land));
    let bz = g.obj(bruna).zcc;
    resolve_all(
        &mut g,
        &[Action::Pick(eel), Action::Pick(armor), Action::Pick(ws)],
    );
    for c in [eel, ws, armor] {
        assert_eq!(g.obj(c).attached_to, Some((bruna, bz)));
    }
    assert_eq!(g.obj(ws).zone, Zone::Battlefield);
    assert_eq!(g.obj(armor).zone, Zone::Battlefield);
    assert_eq!(
        g.power(bruna),
        5 + 1 + 1 + 5,
        "Eel Umbra, Wingspan Stride, and a Forest each"
    );
}

#[test]
fn light_paws_finds_a_cheaper_aura_with_a_new_name_only_for_a_cast_aura() {
    let mut g = table();
    let lp = put(&mut g, 0, "Light-Paws, Emperor's Voice");
    let ws = top(&mut g, "Wingspan Stride");
    let armor = top(&mut g, "Blanchwood Armor");
    let eel2 = top(&mut g, "Eel Umbra");
    let eel = hand(&mut g, 0, "Eel Umbra");
    act(&mut g, Action::Cast(eel));
    act(&mut g, Action::Target(obj(lp)));
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    let offered = picks(&g);
    assert_eq!(
        offered,
        vec![ws],
        "mana value 2 or less, and not a second Eel Umbra"
    );
    assert!(!offered.contains(&armor) && !offered.contains(&eel2));
    // The Wingspan Stride it puts onto the battlefield wasn't cast: no second search.
    resolve_all(&mut g, &[Action::Pick(ws)]);
    let z = g.obj(lp).zcc;
    assert_eq!(g.obj(ws).attached_to, Some((lp, z)));
    assert!(g.keywords(lp).has(Keywords::FLYING));
    assert_eq!(g.obj(armor).zone, Zone::Library);
}

#[test]
fn sovereigns_of_lost_alara_exalts_and_attaches_an_aura_to_a_lone_attacker() {
    let mut g = table();
    put(&mut g, 0, "Sovereigns of Lost Alara");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let before = g.power(ghoul);
    to_attackers(&mut g);
    let ws = top(&mut g, "Wingspan Stride"); // after the draw step
    act(&mut g, Action::Attack(ghoul));
    act(&mut g, Action::FinishAttacks);
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(picks(&g).contains(&ws));
    resolve_all(&mut g, &[Action::Pick(ws)]);
    let z = g.obj(ghoul).zcc;
    assert_eq!(g.obj(ws).attached_to, Some((ghoul, z)));
    assert_eq!(g.power(ghoul), before + 2, "exalted and Wingspan Stride");
    assert!(g.keywords(ghoul).has(Keywords::FLYING));
}

// ---- Mac C batch 2 ----

#[test]
fn olivia_returns_a_creature_attacking_that_is_exiled_without_a_legendary_vampire() {
    let mut g = table();
    let olivia = put(&mut g, 0, "Olivia, Crimson Bride");
    let ghoul = g.add_card(0, cid("Hungry Ghoul"), Zone::Graveyard);
    to_attackers(&mut g);
    act(&mut g, Action::Attack(olivia));
    act(&mut g, Action::FinishAttacks);
    while !matches!(g.pending, Pending::ChooseTarget(0)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Target(obj(ghoul)));
    while !g.items.is_empty() {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    assert!(g.obj(ghoul).tapped);
    assert!(
        g.combat.as_ref().unwrap().attackers.contains(&ghoul),
        "tapped and attacking"
    );
    let kill = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, kill, false, &[obj(olivia)], &[]);
    assert_ne!(g.obj(olivia).zone, Zone::Battlefield);
    assert_eq!(g.obj(ghoul).zone, Zone::Exile, "no legendary Vampire: exiled");
}

#[test]
fn dragonhawk_exiles_per_big_creature_and_burns_for_each_card_left_at_the_end_step() {
    let mut g = table();
    put(&mut g, 0, "Sovereigns of Lost Alara"); // power 4: X is 2 with Dragonhawk
    let ghoul = top(&mut g, "Hungry Ghoul");
    let forest = top(&mut g, "Forest");
    let dh = hand(&mut g, 0, "Dragonhawk, Fate's Tempest");
    play(&mut g, dh, false, &[], &[]);
    assert_eq!(g.obj(ghoul).zone, Zone::Exile);
    assert_eq!(g.obj(forest).zone, Zone::Exile);
    assert!(
        g.legal_actions().contains(&Action::PlayLand(forest)),
        "playable from exile"
    );
    act(&mut g, Action::PlayLand(forest));
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    let life = g.players[1].life;
    while g.turn == 3 && !g.is_over() {
        let legal = g.legal_actions();
        let a = if legal.contains(&Action::PassPriority) {
            Action::PassPriority
        } else if legal.contains(&Action::FinishAttacks) {
            Action::FinishAttacks
        } else {
            legal[0] // a cleanup discard
        };
        act(&mut g, a);
    }
    assert_eq!(
        g.players[1].life,
        life - 2,
        "2 damage for the one card still exiled"
    );
}

#[test]
fn glimpse_of_tomorrow_is_suspended_then_puts_your_permanents_back_auras_last() {
    let mut g = game(&["Mountain", "Mountain"]);
    let gl = hand(&mut g, 0, "Glimpse of Tomorrow");
    assert!(
        !g.legal_actions().contains(&Action::Cast(gl)),
        "no mana cost: not cast from hand"
    );
    act(&mut g, Action::Activate(gl, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(gl).zone, Zone::Exile);
    assert_eq!(g.obj(gl).time_counters, 3);
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let ws = hand(&mut g, 0, "Wingspan Stride");
    let z = g.obj(ghoul).zcc;
    assert!(g.put_aura_attached(ws, ghoul, z, 0));
    g.start_turn(5, 0);
    g.start_turn(7, 0);
    assert_eq!(g.obj(gl).time_counters, 1);
    g.start_turn(9, 0);
    while !matches!(g.pending, Pending::FreeCast(0)) {
        act(&mut g, Action::PassPriority);
    }
    // Only its own permanents in the library, so "that many" reveals exactly them.
    g.players[0].library.clear();
    act(&mut g, Action::Cast(gl));
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield, "the non-Auras first");
    assert_eq!(g.permanents(0).filter(|&x| g.types_of(x).land).count(), 2);
    act(&mut g, Action::Pick(ghoul));
    let z = g.obj(ghoul).zcc;
    assert_eq!(g.obj(ws).attached_to, Some((ghoul, z)));
    assert_eq!(g.obj(gl).zone, Zone::Graveyard);
}

// ---- Malik and Olivia: small pieces ----

#[test]
fn moria_scavenger_loots_and_amasses_only_for_a_creature_card() {
    let mut g = table();
    let ms = put(&mut g, 0, "Moria Scavenger");
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let land = hand(&mut g, 0, "Forest");
    let n = g.players[0].hand.len();
    act(&mut g, Action::ActivateWith(ms, 0, land));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n, "discarded one, drew one");
    assert!(tokens(&g, 0, "Orc Army").is_empty(), "a land: no amass");
    g.start_turn(7, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let ghoul = hand(&mut g, 0, "Hungry Ghoul");
    act(&mut g, Action::ActivateWith(ms, 0, ghoul));
    resolve_all(&mut g, &[]);
    let army = tokens(&g, 0, "Orc Army");
    assert_eq!(army.len(), 1);
    assert_eq!(g.power(army[0]), 1);
}

#[test]
fn professional_face_breaker_makes_treasure_on_damage_and_cashes_it_for_a_card() {
    let mut g = table();
    let fb = put(&mut g, 0, "Professional Face-Breaker");
    to_attackers(&mut g);
    act(&mut g, Action::Attack(fb));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        let legal = g.legal_actions();
        let a = if legal.contains(&Action::FinishBlocks) {
            Action::FinishBlocks
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
    }
    let treasure = tokens(&g, 0, "Treasure");
    assert_eq!(treasure.len(), 1);
    let card = top(&mut g, "Forest");
    act(&mut g, Action::ActivateWith(fb, 0, treasure[0]));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(card).zone, Zone::Exile);
    assert!(g.legal_actions().contains(&Action::PlayLand(card)));
}

// ---- Bruna's Auras ----

fn enchant(g: &mut Game, aura: &str, on: ObjId) -> ObjId {
    let a = hand(g, 0, aura);
    let z = g.obj(on).zcc;
    assert!(g.put_aura_attached(a, on, z, 0));
    a
}

#[test]
fn steel_of_the_godhead_checks_the_enchanted_creatures_colors() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let before = g.power(ghoul);
    enchant(&mut g, "Steel of the Godhead", ghoul);
    assert_eq!(g.power(ghoul), before, "black: neither");
    let bruna = put(&mut g, 0, "Bruna, Light of Alabaster");
    enchant(&mut g, "Steel of the Godhead", bruna);
    assert_eq!(g.power(bruna), 5 + 2, "white and blue: both");
    assert!(g.keywords(bruna).has(Keywords::LIFELINK));
    assert!(g.keywords(bruna).has(Keywords::UNBLOCKABLE));
}

#[test]
fn face_of_divinity_adds_first_strike_and_lifelink_with_another_aura() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let before = g.power(ghoul);
    enchant(&mut g, "Face of Divinity", ghoul);
    assert_eq!(g.power(ghoul), before + 2);
    assert!(!g.keywords(ghoul).has(Keywords::FIRST_STRIKE));
    enchant(&mut g, "Wingspan Stride", ghoul);
    assert!(g.keywords(ghoul).has(Keywords::FIRST_STRIKE));
    assert!(g.keywords(ghoul).has(Keywords::LIFELINK));
}

#[test]
fn shielded_by_faith_may_move_to_a_creature_that_enters() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let sf = enchant(&mut g, "Shielded by Faith", ghoul);
    assert!(g.keywords(ghoul).has(Keywords::INDESTRUCTIBLE));
    let next = hand(&mut g, 0, "Hungry Ghoul");
    play(&mut g, next, false, &[], &[Action::Yes]);
    let z = g.obj(next).zcc;
    assert_eq!(g.obj(sf).attached_to, Some((next, z)));
    assert!(g.keywords(next).has(Keywords::INDESTRUCTIBLE));
    assert!(!g.keywords(ghoul).has(Keywords::INDESTRUCTIBLE));
}

#[test]
fn eldrazi_conscription_pumps_and_annihilates_two() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let before = g.power(ghoul);
    enchant(&mut g, "Eldrazi Conscription", ghoul);
    assert_eq!(g.power(ghoul), before + 10);
    assert!(g.keywords(ghoul).has(Keywords::TRAMPLE));
    let theirs = g.permanents(1).count();
    to_attackers(&mut g);
    act(&mut g, Action::Attack(ghoul));
    act(&mut g, Action::FinishAttacks);
    while g.step == Step::DeclareAttackers {
        let legal = g.legal_actions();
        let a = if legal.contains(&Action::PassPriority) {
            Action::PassPriority
        } else {
            legal[0]
        };
        act(&mut g, a);
    }
    assert_eq!(
        g.permanents(1).count(),
        theirs - 2,
        "defending player sacrificed two"
    );
}

#[test]
fn gisa_exiles_dying_opponent_creatures_and_returns_them_decayed() {
    let mut g = table();
    put(&mut g, 0, "Gisa, Glorious Resurrector");
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let kill = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, kill, false, &[obj(ghoul)], &[]);
    assert_eq!(g.obj(ghoul).zone, Zone::Exile, "exiled instead of dying");
    g.start_turn(5, 0);
    while g.step == Step::Upkeep || !g.items.is_empty() {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    assert_eq!(g.obj(ghoul).controller, 0);
    assert!(g.obj(ghoul).decayed, "it gains decayed");
}

// ---- Mac C: modal double-faced back faces cast; Vohar ----

/// Resolves the stack, answering player 0's decisions with `pick` when offered (else Yes, else the
/// first choice) and the opponent's with their first choice.
fn resolve_picking(g: &mut Game, pick: ObjId) {
    while !g.items.is_empty() || matches!(g.pending, Pending::Decide(_) | Pending::ChooseTarget(_)) {
        let legal = g.legal_actions();
        let a = match g.pending {
            Pending::Decide(0) if legal.contains(&Action::Pick(pick)) => Action::Pick(pick),
            Pending::Decide(0) if legal.contains(&Action::Yes) => Action::Yes,
            Pending::Decide(_) | Pending::ChooseTarget(_) => legal[0],
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn tergrid_takes_what_an_opponent_sacrifices() {
    let mut g = table();
    put(&mut g, 0, "Tergrid, God of Fright // Tergrid's Lantern");
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let exe = hand(&mut g, 0, "Merciless Executioner");
    act(&mut g, Action::Cast(exe));
    resolve_picking(&mut g, exe);
    assert_eq!(g.obj(exe).zone, Zone::Graveyard);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield, "back from their graveyard");
    assert_eq!(g.obj(ghoul).controller, 0);
}

#[test]
fn tergrids_lantern_is_cast_as_the_back_face_and_taxes_a_player() {
    let mut g = table();
    let t = hand(&mut g, 0, "Tergrid, God of Fright // Tergrid's Lantern");
    assert!(g.legal_actions().contains(&Action::Cast(t)), "the front");
    assert!(g.legal_actions().contains(&Action::CastModal(t, 1)), "the back");
    act(&mut g, Action::CastModal(t, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(t).zone, Zone::Battlefield);
    assert_eq!(g.def_of(t).name, "Tergrid's Lantern");
    assert!(!g.types_of(t).creature && g.types_of(t).artifact);
    let life = g.players[1].life;
    act(&mut g, Action::Activate(t, 0));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[Action::No]);
    assert_eq!(g.players[1].life, life - 3, "didn't sacrifice or discard");
    assert!(g.obj(t).tapped);
    act(&mut g, Action::Activate(t, 1));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(t).tapped, "{{3}}{{B}}: untap");
}

#[test]
fn a_back_face_spell_that_is_countered_is_its_front_again() {
    let mut g = table();
    let t = hand(&mut g, 0, "Tergrid, God of Fright // Tergrid's Lantern");
    act(&mut g, Action::CastModal(t, 1));
    assert_eq!(
        g.def_of(t).name,
        "Tergrid's Lantern",
        "on the stack as its back face"
    );
    act(&mut g, Action::PassPriority);
    let veto = hand(&mut g, 1, "Dovin's Veto");
    act(&mut g, Action::Cast(veto));
    act(&mut g, Action::Target(obj(t)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(t).zone, Zone::Graveyard);
    assert_eq!(g.def_of(t).name, "Tergrid, God of Fright // Tergrid's Lantern");
}

#[test]
fn vohar_loots_drains_on_a_discarded_sorcery_and_grants_a_graveyard_cast() {
    let mut g = table();
    let v = put(&mut g, 0, "Vohar, Vodalian Desecrator");
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let sorcery = hand(&mut g, 0, "Tranquility");
    let (mine, theirs) = (g.players[0].life, g.players[1].life);
    act(&mut g, Action::Activate(v, 0));
    resolve_all(&mut g, &[Action::Pick(sorcery)]);
    assert_eq!(g.obj(sorcery).zone, Zone::Graveyard);
    assert_eq!((g.players[0].life, g.players[1].life), (mine + 1, theirs - 1));
    act(&mut g, Action::Activate(v, 1));
    act(&mut g, Action::Target(obj(sorcery)));
    resolve_all(&mut g, &[]);
    assert_ne!(g.obj(v).zone, Zone::Battlefield, "sacrificed");
    assert!(
        g.legal_actions().contains(&Action::CastFlashback(sorcery)),
        "castable from the graveyard this turn"
    );
}

// ---- Script-only pass: Bruna and Olivia ----

/// Passes (finishing attacks and blocks with none) until `done` holds.
fn pass_until(g: &mut Game, done: impl Fn(&Game) -> bool) {
    for _ in 0..200 {
        if done(g) {
            return;
        }
        let legal = g.legal_actions();
        let a = [Action::PassPriority, Action::FinishAttacks, Action::FinishBlocks]
            .into_iter()
            .find(|a| legal.contains(a))
            .unwrap_or(legal[0]);
        act(g, a);
    }
    panic!("never got there");
}

#[test]
fn lavaspur_boots_gives_plus_one_and_haste() {
    let mut g = table();
    let boots = put(&mut g, 0, "Lavaspur Boots");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let before = g.power(ghoul);
    act(&mut g, Action::Activate(boots, 0));
    act(&mut g, Action::Target(obj(ghoul)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(ghoul), before + 1);
    assert!(g.keywords(ghoul).has(Keywords::HASTE));
    pass_until(&mut g, |g| g.pending == Pending::DeclareAttackers(0));
    assert!(
        g.legal_actions().contains(&Action::Attack(ghoul)),
        "haste: it attacks the turn it came in"
    );
}

#[test]
fn mother_of_runes_gives_protection_from_a_chosen_color() {
    let mut g = table();
    let m = put(&mut g, 0, "Mother of Runes");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    g.start_turn(5, 0);
    pass_until(&mut g, |g| g.step == Step::Main1);
    act(&mut g, Action::Activate(m, 0));
    act(&mut g, Action::Target(obj(ghoul)));
    resolve_all(&mut g, &[Action::Mode(2)]);
    assert!(g.protection(ghoul)[2], "protection from black");
    assert!(g.obj(m).tapped);
}

#[test]
fn port_razer_untaps_and_adds_a_combat_it_cannot_attack_in() {
    let mut g = table();
    let razer = put(&mut g, 0, "Port Razer");
    let _other = put(&mut g, 0, "Hungry Ghoul"); // stays home, so the extra combat asks for attackers
    to_attackers(&mut g);
    act(&mut g, Action::Attack(razer));
    act(&mut g, Action::FinishAttacks);
    let life = g.players[1].life;
    pass_until(&mut g, |g| g.players[1].life < life && g.items.is_empty());
    pass_until(&mut g, |g| g.pending == Pending::DeclareAttackers(0));
    assert!(!g.obj(razer).tapped, "untapped");
    assert!(
        !g.legal_actions().contains(&Action::Attack(razer)),
        "it already attacked that player"
    );
}

#[test]
fn breath_of_fury_trades_its_creature_for_another_and_an_extra_combat() {
    let mut g = table();
    let a = put(&mut g, 0, "Hungry Ghoul");
    let b = put(&mut g, 0, "Hungry Ghoul");
    let breath = hand(&mut g, 0, "Breath of Fury");
    let z = g.obj(a).zcc;
    assert!(g.put_aura_attached(breath, a, z, 0));
    to_attackers(&mut g);
    act(&mut g, Action::Attack(a));
    act(&mut g, Action::FinishAttacks);
    pass_until(&mut g, |g| matches!(g.pending, Pending::Decide(0)));
    assert!(g.legal_actions().contains(&Action::Pick(b)));
    act(&mut g, Action::Pick(b));
    assert_eq!(g.obj(a).zone, Zone::Graveyard, "sacrificed");
    let zb = g.obj(b).zcc;
    assert_eq!(g.obj(breath).attached_to, Some((b, zb)));
    pass_until(&mut g, |g| g.pending == Pending::DeclareAttackers(0));
    assert!(
        g.legal_actions().contains(&Action::Attack(b)),
        "an additional combat"
    );
}

// ---- Lane B's decks, Mac C pieces ----

#[test]
fn kaalia_puts_a_dragon_from_hand_onto_the_battlefield_attacking() {
    let mut g = table();
    let kaalia = put(&mut g, 0, "Kaalia of the Vast");
    to_attackers(&mut g);
    let dragon = hand(&mut g, 0, "Terror of the Peaks");
    let ghoul = hand(&mut g, 0, "Hungry Ghoul");
    act(&mut g, Action::Attack(kaalia));
    act(&mut g, Action::FinishAttacks);
    pass_until(&mut g, |g| matches!(g.pending, Pending::Decide(0)));
    assert!(
        !g.legal_actions().contains(&Action::Pick(ghoul)),
        "a Zombie isn't offered"
    );
    act(&mut g, Action::Pick(dragon));
    assert_eq!(g.obj(dragon).zone, Zone::Battlefield);
    assert!(g.obj(dragon).tapped);
    assert!(
        g.combat.as_ref().unwrap().attackers.contains(&dragon),
        "tapped and attacking"
    );
}

#[test]
fn command_beacon_puts_your_commander_into_your_hand() {
    let mut g = table();
    let beacon = put(&mut g, 0, "Command Beacon");
    let bello = g
        .command
        .iter()
        .copied()
        .find(|&c| g.obj(c).owner == 0)
        .expect("the commander");
    act(&mut g, Action::Activate(beacon, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(bello).zone, Zone::Hand);
    assert_eq!(g.obj(beacon).zone, Zone::Graveyard);
}

#[test]
fn mystic_forge_casts_artifacts_from_the_top_and_exiles_the_top_for_life() {
    let mut g = table();
    let forge = put(&mut g, 0, "Mystic Forge");
    let ring = top(&mut g, "Sol Ring");
    assert!(
        g.legal_actions().contains(&Action::Cast(ring)),
        "an artifact from the top"
    );
    let ghoul = top(&mut g, "Hungry Ghoul");
    assert!(
        !g.legal_actions().contains(&Action::Cast(ghoul)),
        "not a black creature"
    );
    let life = g.players[0].life;
    act(&mut g, Action::Activate(forge, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ghoul).zone, Zone::Exile);
    assert_eq!(g.players[0].life, life - 1);
}

#[test]
fn lithoform_engine_copies_an_ability_you_control() {
    let mut g = table();
    let engine = put(&mut g, 0, "Lithoform Engine");
    let forge = put(&mut g, 0, "Mystic Forge");
    let a = top(&mut g, "Hungry Ghoul");
    let b = top(&mut g, "Hungry Ghoul");
    let life = g.players[0].life;
    act(&mut g, Action::Activate(forge, 0));
    act(&mut g, Action::Activate(engine, 0));
    act(&mut g, Action::Target(obj(forge)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(a).zone, Zone::Exile, "the copy");
    assert_eq!(g.obj(b).zone, Zone::Exile, "the original");
    assert_eq!(g.players[0].life, life - 1, "the copy's cost isn't paid again");
}

#[test]
fn lithoform_engine_copies_an_instant_with_new_targets() {
    let mut g = table();
    let engine = put(&mut g, 0, "Lithoform Engine");
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let life = g.players[1].life;
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::Activate(engine, 1));
    act(&mut g, Action::Target(obj(bolt)));
    resolve_all(&mut g, &[Action::Target(obj(ghoul))]);
    assert_eq!(g.obj(ghoul).zone, Zone::Graveyard, "the copy's new target");
    assert_eq!(g.players[1].life, life - 3, "the original");
}

#[test]
fn lithoform_engine_copies_a_permanent_spell_as_a_token() {
    let mut g = table();
    let engine = put(&mut g, 0, "Lithoform Engine");
    let ghoul = hand(&mut g, 0, "Hungry Ghoul");
    act(&mut g, Action::Cast(ghoul));
    act(&mut g, Action::Activate(engine, 2));
    act(&mut g, Action::Target(obj(ghoul)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    let copies: Vec<ObjId> = g
        .creatures(0)
        .filter(|&c| c != ghoul && g.def_of(c).name == "Hungry Ghoul")
        .collect();
    assert_eq!(copies.len(), 1, "the copy");
    assert!(g.is_token(copies[0]), "it became a token");
}

#[test]
fn thespians_stage_becomes_a_copy_of_a_land_and_keeps_its_ability() {
    let mut g = table();
    let stage = put(&mut g, 0, "Thespian's Stage");
    let beacon = put(&mut g, 1, "Command Beacon");
    assert_eq!(g.def_of(stage).activated.len(), 2);
    assert!(
        !g.legal_actions().contains(&Action::Activate(stage, 1)),
        "the copy-only duplicate isn't the card's own"
    );
    act(&mut g, Action::Activate(stage, 0));
    act(&mut g, Action::Target(obj(beacon)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.name_of(stage), "Command Beacon");
    g.start_turn(5, 0);
    pass_until(&mut g, |g| g.step == Step::Main1);
    let legal = g.legal_actions();
    assert!(
        legal.contains(&Action::Activate(stage, 0)),
        "Command Beacon's ability"
    );
    assert!(
        legal.contains(&Action::Activate(stage, 1)),
        "and its own copy ability"
    );
}
