//! Playgroup decks, lane D1 (the desktop): each script's behavior in a 1v1 Commander game on the CMD pool.
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

// ---- Ygra, Eater of All (Grande) ----

#[test]
fn ashnods_altar_sacrifices_a_creature_for_two_colorless() {
    let mut g = table();
    let altar = put(&mut g, 0, "Ashnod's Altar");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    act(&mut g, Action::ActivateWith(altar, 0, elf));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elf).zone, Zone::Graveyard);
    let pool = &g.players[0].pool;
    assert_eq!(pool.len(), 2, "{pool:?}");
    assert!(pool.iter().all(|m| m.colorless && m.colors == [false; 5]));
}

#[test]
fn eternal_witness_returns_a_card_from_your_graveyard() {
    let mut g = table();
    let card = g.add_card(0, cid("Night's Whisper"), Zone::Graveyard);
    let w = hand(&mut g, 0, "Eternal Witness");
    play(&mut g, w, false, &[], &[Action::Target(obj(card)), Action::Yes]);
    assert_eq!(g.obj(card).zone, Zone::Hand);
}

#[test]
fn worldly_tutor_puts_a_creature_on_top() {
    let mut g = game(&["Fyndhorn Elves"]); // its mana casts the Tutor
    let land = g.add_card(0, cid("Forest"), Zone::Library);
    let c = g.add_card(0, cid("Blood Artist"), Zone::Library);
    g.players[0].library.rotate_right(1); // the Artist at the bottom
    let t = hand(&mut g, 0, "Worldly Tutor");
    act(&mut g, Action::Cast(t));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Pick(land)),
        "a creature card"
    );
    act(&mut g, Action::Pick(c));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].library.last(), Some(&c));
}

#[test]
fn warren_soultrader_pays_life_and_sacrifices_another_for_a_treasure() {
    let mut g = table();
    let ws = put(&mut g, 0, "Warren Soultrader");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    assert!(
        !g.legal_actions().contains(&Action::ActivateWith(ws, 0, ws)),
        "another creature"
    );
    act(&mut g, Action::ActivateWith(ws, 0, elf));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 39);
    assert_eq!(g.obj(elf).zone, Zone::Graveyard);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1);
}

#[test]
fn blood_artist_drains_when_it_or_another_creature_dies() {
    let mut g = table();
    let artist = put(&mut g, 0, "Blood Artist");
    let seer = put(&mut g, 0, "Viscera Seer");
    let theirs = put(&mut g, 1, "Fyndhorn Elves");
    let decay = hand(&mut g, 0, "Abrupt Decay");
    play(
        &mut g,
        decay,
        false,
        &[obj(theirs)],
        &[Action::Target(Target::Player(1))],
    );
    assert_eq!(
        (g.players[0].life, g.players[1].life),
        (41, 39),
        "another creature died"
    );
    act(&mut g, Action::ActivateWith(seer, 0, artist));
    resolve_all(&mut g, &[Action::Target(Target::Player(1)), Action::No]);
    assert_eq!(
        (g.players[0].life, g.players[1].life),
        (42, 38),
        "the Artist itself died"
    );
}

#[test]
fn nights_whisper_draws_two_and_loses_two() {
    let mut g = table();
    let c = hand(&mut g, 0, "Night's Whisper");
    let n = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), n + 1);
    assert_eq!(g.players[0].life, 38);
}

#[test]
fn dictate_of_erebos_makes_the_opponent_sacrifice_when_your_creature_dies() {
    let mut g = table();
    put(&mut g, 0, "Dictate of Erebos");
    let altar = put(&mut g, 0, "Ashnod's Altar");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    let theirs = put(&mut g, 1, "Warren Soultrader");
    act(&mut g, Action::ActivateWith(altar, 0, elf));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
}

#[test]
fn phyrexian_altar_sacrifices_a_creature_for_one_mana_of_any_color() {
    let mut g = table();
    let altar = put(&mut g, 0, "Phyrexian Altar");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    act(&mut g, Action::ActivateWith(altar, 0, elf));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elf).zone, Zone::Graveyard);
    let pool = &g.players[0].pool;
    assert_eq!(pool.len(), 1, "{pool:?}");
    assert_eq!(pool[0].colors, [true; 5], "any color");
}

#[test]
fn sign_in_blood_makes_target_player_draw_two_and_lose_two() {
    let mut g = table();
    let c = hand(&mut g, 0, "Sign in Blood");
    let n = g.players[1].hand.len();
    play(&mut g, c, false, &[Target::Player(1)], &[]);
    assert_eq!(g.players[1].hand.len(), n + 2);
    assert_eq!(g.players[1].life, 38);
    assert_eq!(g.players[0].life, 40);
}

#[test]
fn viscera_seer_sacrifices_a_creature_to_scry_one() {
    let mut g = table();
    let seer = put(&mut g, 0, "Viscera Seer");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    let top = g.add_card(0, cid("Blood Artist"), Zone::Library);
    act(&mut g, Action::ActivateWith(seer, 0, elf));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.obj(elf).zone, Zone::Graveyard);
    assert_eq!(g.players[0].library[0], top, "scried to the bottom");
    act(&mut g, Action::ActivateWith(seer, 0, seer));
    resolve_all(&mut g, &[Action::No]);
    assert_eq!(g.obj(seer).zone, Zone::Graveyard, "it can sacrifice itself");
}

#[test]
fn blasting_station_pings_and_untaps_when_a_creature_enters() {
    let mut g = table();
    let st = put(&mut g, 0, "Blasting Station");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    act(&mut g, Action::ActivateWith(st, 0, elf));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].life, 39);
    assert!(g.obj(st).tapped);
    let e2 = hand(&mut g, 0, "Fyndhorn Elves");
    play(&mut g, e2, false, &[], &[Action::Yes]);
    assert!(!g.obj(st).tapped, "a creature entered");
}

#[test]
fn carrion_feeder_cant_block_and_grows_by_sacrificing() {
    let mut g = table();
    let f = put(&mut g, 0, "Carrion Feeder");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    assert!(g.def_of(f).cant_block);
    act(&mut g, Action::ActivateWith(f, 0, elf));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(f).counters, 1);
    assert_eq!(g.power(f), 2);
}

#[test]
fn darkbore_pathway_taps_for_black() {
    let mut g = game(&["Darkbore Pathway // Slitherbore Pathway"]);
    let seer = hand(&mut g, 0, "Viscera Seer");
    play(&mut g, seer, false, &[], &[]);
    assert_eq!(g.obj(seer).zone, Zone::Battlefield);
}

#[test]
fn fyndhorn_elves_taps_for_green() {
    let mut g = game(&["Fyndhorn Elves"]);
    let t = hand(&mut g, 0, "Worldly Tutor");
    assert!(g.legal_actions().contains(&Action::Cast(t)));
    let seer = hand(&mut g, 0, "Viscera Seer");
    assert!(!g.legal_actions().contains(&Action::Cast(seer)), "green only");
}

#[test]
fn krosan_grip_has_split_second_and_destroys_an_artifact() {
    let mut g = table();
    let altar = put(&mut g, 1, "Ashnod's Altar");
    let c = hand(&mut g, 0, "Krosan Grip");
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(altar)));
    assert_eq!(g.legal_actions(), vec![Action::PassPriority], "split second");
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(altar).zone, Zone::Graveyard);
}

#[test]
fn abrupt_decay_destroys_a_cheap_nonland_permanent_and_cant_be_countered() {
    let mut g = table();
    let cheap = put(&mut g, 1, "Warren Soultrader");
    let big = put(&mut g, 1, "Dictate of Erebos");
    let land = put(&mut g, 1, "Forest");
    let c = hand(&mut g, 0, "Abrupt Decay");
    assert!(g.def_of(c).uncounterable);
    act(&mut g, Action::Cast(c));
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Target(obj(big))), "mana value 5");
    assert!(!legal.contains(&Action::Target(obj(land))), "a land");
    act(&mut g, Action::Target(obj(cheap)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(cheap).zone, Zone::Graveyard);
}

#[test]
fn elves_of_deep_shadow_taps_for_black_and_pings_you() {
    let mut g = game(&["Elves of Deep Shadow"]);
    let seer = hand(&mut g, 0, "Viscera Seer");
    play(&mut g, seer, false, &[], &[]);
    assert_eq!(g.obj(seer).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, 39);
}

// ---- Raggadragga, Goreguts Boss (Adam) ----

fn tap(g: &mut Game, id: ObjId) {
    g.objects[id.0 as usize].tapped = true;
}

fn set_counters(g: &mut Game, id: ObjId, n: i32) {
    g.objects[id.0 as usize].counters = n;
}

/// The colors (WUBRG) and colorless mana a permanent of player 0's can make.
fn makes(g: &Game, id: ObjId) -> ([bool; 5], bool) {
    g.mana_sources(0)
        .into_iter()
        .filter(|&(x, _)| x == id)
        .fold(([false; 5], false), |acc, (_, m)| {
            (
                std::array::from_fn(|i| acc.0[i] || m.colors[i]),
                acc.1 || m.colorless,
            )
        })
}

fn activate(g: &mut Game, id: ObjId, n: usize, targets: &[Target]) {
    act(g, Action::Activate(id, n));
    for &t in targets {
        act(g, Action::Target(t));
    }
    resolve_all(g, &[]);
}

#[test]
fn beast_whisperer_draws_when_you_cast_a_creature_spell() {
    let mut g = table();
    put(&mut g, 0, "Beast Whisperer");
    let elf = hand(&mut g, 0, "Fyndhorn Elves");
    let n = g.players[0].hand.len();
    play(&mut g, elf, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), n, "cast one, drew one");
    let c = hand(&mut g, 0, "Night's Whisper");
    let n = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[]);
    assert_eq!(
        g.players[0].hand.len(),
        n + 1,
        "Night's Whisper's two, nothing for a sorcery"
    );
}

#[test]
fn staff_of_domination_untaps_gains_taps_and_draws() {
    let mut g = table();
    let staff = put(&mut g, 0, "Staff of Domination");
    let mine = put(&mut g, 0, "Fyndhorn Elves");
    let theirs = put(&mut g, 1, "Fyndhorn Elves");
    tap(&mut g, mine);
    activate(&mut g, staff, 2, &[obj(mine)]);
    assert!(!g.obj(mine).tapped && g.obj(staff).tapped);
    activate(&mut g, staff, 0, &[]);
    assert!(!g.obj(staff).tapped, "{{1}}: untap");
    activate(&mut g, staff, 1, &[]);
    assert_eq!(g.players[0].life, 41);
    activate(&mut g, staff, 0, &[]);
    activate(&mut g, staff, 3, &[obj(theirs)]);
    assert!(g.obj(theirs).tapped);
    activate(&mut g, staff, 0, &[]);
    let n = g.players[0].hand.len();
    activate(&mut g, staff, 4, &[]);
    assert_eq!(g.players[0].hand.len(), n + 1);
}

#[test]
fn fires_of_yavimaya_gives_haste_and_sacrifices_for_a_pump() {
    let mut g = table();
    let fires = put(&mut g, 0, "Fires of Yavimaya");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    let theirs = put(&mut g, 1, "Fyndhorn Elves");
    assert!(g.keywords(elf).has(Keywords::HASTE));
    assert!(!g.keywords(theirs).has(Keywords::HASTE), "only yours");
    activate(&mut g, fires, 0, &[obj(elf)]);
    assert_eq!(g.obj(fires).zone, Zone::Graveyard);
    assert_eq!(g.power(elf), 3);
}

#[test]
fn wirewood_lodge_taps_for_colorless_and_untaps_an_elf() {
    let mut g = table();
    let lodge = put(&mut g, 0, "Wirewood Lodge");
    assert_eq!(makes(&g, lodge), ([false; 5], true));
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    let seer = put(&mut g, 0, "Viscera Seer");
    tap(&mut g, elf);
    tap(&mut g, seer);
    act(&mut g, Action::Activate(lodge, 0));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(seer))),
        "not an Elf"
    );
    act(&mut g, Action::Target(obj(elf)));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(elf).tapped);
}

#[test]
fn boreal_druid_taps_for_colorless() {
    let mut g = table();
    let d = put(&mut g, 0, "Boreal Druid");
    g.start_turn(5, 0); // no longer summoning sick
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(makes(&g, d), ([false; 5], true));
}

#[test]
fn cragcrown_pathway_taps_for_red() {
    let mut g = table();
    let p = put(&mut g, 0, "Cragcrown Pathway // Timbercrown Pathway");
    assert_eq!(makes(&g, p), ([false, false, false, true, false], false));
}

#[test]
fn marwyn_grows_with_elves_and_taps_for_her_power_in_green() {
    let mut g = table();
    let m = put(&mut g, 0, "Marwyn, the Nurturer");
    let elf = hand(&mut g, 0, "Fyndhorn Elves");
    play(&mut g, elf, false, &[], &[]);
    assert_eq!(g.obj(m).counters, 1, "another Elf entered");
    let seer = hand(&mut g, 0, "Viscera Seer");
    play(&mut g, seer, false, &[], &[]);
    assert_eq!(g.obj(m).counters, 1, "not an Elf");
    let mut g = game(&["Marwyn, the Nurturer"]);
    let m = named(&g, 0, "Marwyn, the Nurturer");
    let w = hand(&mut g, 0, "Eternal Witness"); // {1}{G}{G}
    assert!(!g.legal_actions().contains(&Action::Cast(w)), "one {{G}}");
    set_counters(&mut g, m, 2);
    assert!(g.legal_actions().contains(&Action::Cast(w)), "three {{G}}");
}

fn named(g: &Game, p: usize, name: &str) -> ObjId {
    g.permanents(p)
        .find(|&x| g.def_of(x).name == name)
        .expect("on the battlefield")
}

#[test]
fn natures_claim_destroys_and_its_controller_gains_4() {
    let mut g = table();
    let altar = put(&mut g, 1, "Ashnod's Altar");
    let c = hand(&mut g, 0, "Nature's Claim");
    play(&mut g, c, false, &[obj(altar)], &[]);
    assert_eq!(g.obj(altar).zone, Zone::Graveyard);
    assert_eq!((g.players[0].life, g.players[1].life), (40, 44));
}

#[test]
fn viridian_joiner_taps_for_its_power_in_green() {
    let mut g = game(&["Viridian Joiner"]);
    let j = named(&g, 0, "Viridian Joiner");
    let w = hand(&mut g, 0, "Eternal Witness");
    assert!(!g.legal_actions().contains(&Action::Cast(w)));
    set_counters(&mut g, j, 2);
    assert!(g.legal_actions().contains(&Action::Cast(w)));
}

#[test]
fn vitalize_untaps_your_creatures_only() {
    let mut g = table();
    let a = put(&mut g, 0, "Fyndhorn Elves");
    let b = put(&mut g, 0, "Viscera Seer");
    let theirs = put(&mut g, 1, "Fyndhorn Elves");
    for x in [a, b, theirs] {
        tap(&mut g, x);
    }
    let c = hand(&mut g, 0, "Vitalize");
    play(&mut g, c, false, &[], &[]);
    assert!(!g.obj(a).tapped && !g.obj(b).tapped);
    assert!(g.obj(theirs).tapped);
}

// ---- Atla Palani, Nest Tender (RuricMo) ----

#[test]
fn aura_shards_may_destroy_an_artifact_when_your_creature_enters() {
    let mut g = table();
    put(&mut g, 0, "Aura Shards");
    let altar = put(&mut g, 1, "Ashnod's Altar");
    let elf = hand(&mut g, 0, "Fyndhorn Elves");
    play(
        &mut g,
        elf,
        false,
        &[],
        &[Action::Target(obj(altar)), Action::Yes],
    );
    assert_eq!(g.obj(altar).zone, Zone::Graveyard);
}

#[test]
fn impact_tremors_pings_when_your_creature_enters() {
    let mut g = table();
    put(&mut g, 0, "Impact Tremors");
    let elf = hand(&mut g, 0, "Fyndhorn Elves");
    play(&mut g, elf, false, &[], &[]);
    assert_eq!(g.players[1].life, 39);
    put(&mut g, 1, "Fyndhorn Elves");
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 39, "not the opponent's creature");
}

#[test]
fn goblin_bombardment_sacrifices_a_creature_for_1_damage() {
    let mut g = table();
    let gb = put(&mut g, 0, "Goblin Bombardment");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    act(&mut g, Action::ActivateWith(gb, 0, elf));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].life, 39);
    assert_eq!(g.obj(elf).zone, Zone::Graveyard);
}

#[test]
fn high_market_taps_for_colorless_and_sacrifices_for_life() {
    let mut g = table();
    let hm = put(&mut g, 0, "High Market");
    assert_eq!(makes(&g, hm), ([false; 5], true));
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    act(&mut g, Action::ActivateWith(hm, 0, elf));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 41);
    assert!(g.obj(hm).tapped);
}

/// Plays a tapped tri-land; checks it's tapped and its three colors next turn.
fn tri_land(name: &str) -> (Game, ObjId) {
    let mut g = game(&[]);
    let c = hand(&mut g, 0, name);
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(makes(&g, c), ([true, false, false, true, true], false));
    (g, c)
}

#[test]
fn jetmirs_garden_enters_tapped_makes_three_colors_and_cycles() {
    tri_land("Jetmir's Garden");
    let mut g = game(&["Plains", "Plains", "Plains"]);
    let c = hand(&mut g, 0, "Jetmir's Garden");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(c, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).zone, Zone::Graveyard, "cycled");
    assert_eq!(g.players[0].hand.len(), n);
}

#[test]
fn jungle_shrine_enters_tapped_and_makes_three_colors() {
    tri_land("Jungle Shrine");
}

#[test]
fn fanatical_devotion_sacrifices_to_regenerate() {
    let mut g = table();
    let fd = put(&mut g, 0, "Fanatical Devotion");
    let seer = put(&mut g, 0, "Viscera Seer");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    act(&mut g, Action::ActivateWith(fd, 0, seer));
    resolve_all(&mut g, &[Action::Target(obj(elf))]);
    assert_eq!(g.obj(seer).zone, Zone::Graveyard);
    let decay = hand(&mut g, 0, "Abrupt Decay");
    play(&mut g, decay, false, &[obj(elf)], &[]);
    assert_eq!(g.obj(elf).zone, Zone::Battlefield, "regenerated");
    assert!(g.obj(elf).tapped);
}

#[test]
fn samut_gives_others_haste_and_untaps_another_creature() {
    let mut g = table();
    let samut = put(&mut g, 0, "Samut, Voice of Dissent");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    let theirs = put(&mut g, 1, "Fyndhorn Elves");
    assert!(g.keywords(elf).has(Keywords::HASTE));
    assert!(!g.keywords(theirs).has(Keywords::HASTE));
    tap(&mut g, elf);
    act(&mut g, Action::Activate(samut, 0));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(samut))),
        "another creature"
    );
    act(&mut g, Action::Target(obj(elf)));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(elf).tapped && g.obj(samut).tapped);
}

#[test]
fn sheltering_landscape_fetches_a_basic_tapped_and_cycles() {
    let mut g = table();
    let sl = put(&mut g, 0, "Sheltering Landscape");
    assert_eq!(makes(&g, sl), ([false; 5], true));
    let island = g.add_card(0, cid("Island"), Zone::Library);
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    act(&mut g, Action::Activate(sl, 0));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(!g.legal_actions().contains(&Action::Pick(island)));
    act(&mut g, Action::Pick(forest));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(sl).zone, Zone::Graveyard);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(g.obj(forest).tapped);
    let c = hand(&mut g, 0, "Sheltering Landscape");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(c, 1));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(c).zone, g.players[0].hand.len()), (Zone::Graveyard, n));
}

// ---- Karador, Ghost Chieftain (Grande) ----

/// Casts or activates until a search decision comes up, checks `bad` can't be picked, and picks `good`.
fn pick(g: &mut Game, good: ObjId, bad: ObjId) {
    while !matches!(g.pending, Pending::Decide(_)) {
        act(g, Action::PassPriority);
    }
    assert!(!g.legal_actions().contains(&Action::Pick(bad)), "can't find that");
    act(g, Action::Pick(good));
    resolve_all(g, &[]);
}

#[test]
fn wood_elves_puts_a_forest_onto_the_battlefield_untapped() {
    let mut g = table();
    let island = g.add_card(0, cid("Island"), Zone::Library);
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    let w = hand(&mut g, 0, "Wood Elves");
    act(&mut g, Action::Cast(w));
    pick(&mut g, forest, island);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(!g.obj(forest).tapped);
}

#[test]
fn acidic_slime_destroys_an_artifact_enchantment_or_land() {
    let mut g = table();
    let land = put(&mut g, 1, "Forest");
    let elf = put(&mut g, 1, "Fyndhorn Elves");
    let c = hand(&mut g, 0, "Acidic Slime");
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(elf))),
        "not a creature"
    );
    act(&mut g, Action::Target(obj(land)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(land).zone, Zone::Graveyard);
}

#[test]
fn eladamris_call_tutors_a_creature_to_hand() {
    let mut g = table();
    let land = g.add_card(0, cid("Forest"), Zone::Library);
    let artist = g.add_card(0, cid("Blood Artist"), Zone::Library);
    let c = hand(&mut g, 0, "Eladamri's Call");
    act(&mut g, Action::Cast(c));
    pick(&mut g, artist, land);
    assert_eq!(g.obj(artist).zone, Zone::Hand);
}

#[test]
fn avacyns_pilgrim_taps_for_white() {
    let g = game(&["Avacyn's Pilgrim"]);
    let p = named(&g, 0, "Avacyn's Pilgrim");
    assert_eq!(makes(&g, p), ([true, false, false, false, false], false));
}

#[test]
fn utter_end_exiles_a_nonland_permanent() {
    let mut g = table();
    let d = put(&mut g, 1, "Dictate of Erebos");
    let c = hand(&mut g, 0, "Utter End");
    play(&mut g, c, false, &[obj(d)], &[]);
    assert_eq!(g.obj(d).zone, Zone::Exile);
}

#[test]
fn wight_of_the_reliquary_grows_and_sacrifices_for_a_land() {
    let mut g = table();
    let w = put(&mut g, 0, "Wight of the Reliquary");
    g.add_card(0, cid("Blood Artist"), Zone::Graveyard);
    g.add_card(0, cid("Night's Whisper"), Zone::Graveyard);
    assert_eq!(g.power(w), 3, "one creature card");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    assert!(
        !g.legal_actions().contains(&Action::ActivateWith(w, 0, w)),
        "another creature"
    );
    let spell = g.add_card(0, cid("Night's Whisper"), Zone::Library);
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    act(&mut g, Action::ActivateWith(w, 0, elf));
    pick(&mut g, forest, spell);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(g.obj(forest).tapped && g.obj(w).tapped);
    assert_eq!(g.power(w), 4, "the Elves joined the graveyard");
}

// ---- Ral, Monsoon Mage // Ral, Leyline Prodigy (Time) ----

#[test]
fn gilded_lotus_makes_three_mana_of_one_color() {
    let mut g = game(&[]);
    put(&mut g, 0, "Gilded Lotus");
    let w = hand(&mut g, 0, "Eternal Witness"); // {1}{G}{G}
    let decay = hand(&mut g, 0, "Abrupt Decay"); // {B}{G}
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Cast(w)), "three green");
    assert!(!legal.contains(&Action::Cast(decay)), "one color only");
}

#[test]
fn ash_barrens_taps_for_colorless_and_landcycles() {
    let mut g = table();
    let a = put(&mut g, 0, "Ash Barrens");
    assert_eq!(makes(&g, a), ([false; 5], true));
    let spell = g.add_card(0, cid("Night's Whisper"), Zone::Library);
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    let c = hand(&mut g, 0, "Ash Barrens");
    act(&mut g, Action::Activate(c, 0));
    pick(&mut g, forest, spell);
    assert_eq!((g.obj(c).zone, g.obj(forest).zone), (Zone::Graveyard, Zone::Hand));
}

/// Player 0 casts Night's Whisper and then `counter` targeting it, answering decisions with `answers`.
fn counter_whisper(counter: &str, answers: &[Action]) -> (Game, ObjId, ObjId) {
    let mut g = table();
    let nw = hand(&mut g, 0, "Night's Whisper");
    let c = hand(&mut g, 0, counter);
    act(&mut g, Action::Cast(nw));
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(nw)));
    resolve_all(&mut g, answers);
    (g, nw, c)
}

#[test]
fn dismiss_counters_and_draws() {
    let (g, nw, _) = counter_whisper("Dismiss", &[]);
    assert_eq!(g.obj(nw).zone, Zone::Graveyard);
    assert_eq!(g.players[0].life, 40, "Night's Whisper was countered");
    let (g2, _, _) = counter_whisper("Neutralize", &[]);
    assert_eq!(
        g.players[0].hand.len(),
        g2.players[0].hand.len() + 1,
        "Dismiss's card"
    );
}

#[test]
fn exclude_counters_a_creature_spell_and_draws() {
    let mut g = table();
    let nw = hand(&mut g, 0, "Night's Whisper");
    let c = hand(&mut g, 0, "Exclude");
    act(&mut g, Action::Cast(nw));
    assert!(
        !g.legal_actions().contains(&Action::Cast(c)),
        "no creature spell to target"
    );
    resolve_all(&mut g, &[]);
    let elf = hand(&mut g, 0, "Fyndhorn Elves");
    act(&mut g, Action::Cast(elf));
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(elf)));
    let n = g.players[0].hand.len();
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elf).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n + 1, "Exclude's card");
}

#[test]
fn gitaxian_probe_draws_a_card() {
    let mut g = table();
    let c = hand(&mut g, 0, "Gitaxian Probe");
    let n = g.players[0].hand.len();
    play(&mut g, c, false, &[Target::Player(1)], &[]);
    assert_eq!(g.players[0].hand.len(), n);
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
}

#[test]
fn jwari_disruption_counters_unless_its_controller_pays_1() {
    let (g, nw, _) = counter_whisper("Jwari Disruption // Jwari Ruins", &[Action::No]);
    assert_eq!(g.obj(nw).zone, Zone::Graveyard);
    assert_eq!(g.players[0].life, 40, "not paid: countered");
    let (g, _, _) = counter_whisper("Jwari Disruption // Jwari Ruins", &[Action::Yes]);
    assert_eq!(g.players[0].life, 38, "paid: it resolves");
}

#[test]
fn lightning_bolt_deals_3() {
    let mut g = table();
    let c = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, c, false, &[Target::Player(1)], &[]);
    assert_eq!(g.players[1].life, 37);
}

#[test]
fn neutralize_counters_and_cycles() {
    let (g, nw, _) = counter_whisper("Neutralize", &[]);
    assert_eq!(g.obj(nw).zone, Zone::Graveyard);
    assert_eq!(g.players[0].life, 40);
    let mut g = table();
    let c = hand(&mut g, 0, "Neutralize");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(c, 0));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(c).zone, g.players[0].hand.len()), (Zone::Graveyard, n));
}

#[test]
fn opportunity_makes_target_player_draw_four() {
    let mut g = table();
    let c = hand(&mut g, 0, "Opportunity");
    let n = g.players[1].hand.len();
    play(&mut g, c, false, &[Target::Player(1)], &[]);
    assert_eq!(g.players[1].hand.len(), n + 4);
}

#[test]
fn prophetic_bolt_deals_4_and_takes_one_of_the_top_four() {
    let mut g = table();
    let top: Vec<ObjId> = ["Blood Artist", "Viscera Seer", "Lightning Bolt", "Dismiss"]
        .iter()
        .map(|n| g.add_card(0, cid(n), Zone::Library))
        .collect();
    let c = hand(&mut g, 0, "Prophetic Bolt");
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(Target::Player(1)));
    let mut first = true;
    while !g.items.is_empty() || matches!(g.pending, Pending::Decide(_)) {
        if matches!(g.pending, Pending::Decide(_)) {
            let a = if first {
                Action::Pick(top[1])
            } else {
                *g.legal_actions()
                    .iter()
                    .find(|a| matches!(a, Action::Pick(_)))
                    .expect("a pick")
            };
            first = false;
            act(&mut g, a);
        } else {
            act(&mut g, Action::PassPriority);
        }
    }
    assert_eq!(g.players[1].life, 36);
    assert_eq!(g.obj(top[1]).zone, Zone::Hand);
    let lib = &g.players[0].library;
    for x in [top[0], top[2], top[3]] {
        assert!(lib[..3].contains(&x), "the rest on the bottom");
    }
}

#[test]
fn rewind_counters_and_untaps_four_lands() {
    let (g, nw, _) = counter_whisper("Rewind", &[]);
    assert_eq!(g.obj(nw).zone, Zone::Graveyard);
    let tapped = g.permanents(0).filter(|&x| g.obj(x).tapped).count();
    assert_eq!(tapped, 2, "six tapped for the two spells, four untapped");
}

// ---- Zacama, Primal Calamity (DonSpider) ----

#[test]
fn karametra_isnt_a_creature_without_devotion_and_ramps_on_creature_spells() {
    let mut g = table();
    let k = put(&mut g, 0, "Karametra, God of Harvests");
    assert!(!g.types_of(k).creature, "devotion 2");
    let island = g.add_card(0, cid("Island"), Zone::Library);
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    let elf = hand(&mut g, 0, "Fyndhorn Elves");
    act(&mut g, Action::Cast(elf));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(g.legal_actions().contains(&Action::No), "a may search");
    act(&mut g, Action::Yes);
    pick(&mut g, forest, island);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(g.obj(forest).tapped);
    assert_eq!(g.obj(elf).zone, Zone::Battlefield);
}

#[test]
fn stony_silence_stops_artifact_abilities() {
    let mut g = table();
    let altar = put(&mut g, 0, "Ashnod's Altar");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    assert!(g.legal_actions().contains(&Action::ActivateWith(altar, 0, elf)));
    put(&mut g, 1, "Stony Silence");
    assert!(!g.legal_actions().contains(&Action::ActivateWith(altar, 0, elf)));
}

#[test]
fn sunhome_taps_for_colorless_and_grants_double_strike() {
    let mut g = table();
    let s = put(&mut g, 0, "Sunhome, Fortress of the Legion");
    assert_eq!(makes(&g, s), ([false; 5], true));
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    activate(&mut g, s, 0, &[obj(elf)]);
    assert!(g.keywords(elf).has(Keywords::DOUBLE_STRIKE));
}

#[test]
fn wilderness_reclamation_untaps_your_lands_at_your_end_step() {
    let mut g = table();
    put(&mut g, 0, "Wilderness Reclamation");
    let lands: Vec<ObjId> = g.permanents(0).filter(|&x| g.types_of(x).land).collect();
    for &l in &lands {
        tap(&mut g, l);
    }
    let theirs = g.permanents(1).find(|&x| g.types_of(x).land).unwrap();
    tap(&mut g, theirs);
    while g.step != Step::End {
        act(&mut g, Action::PassPriority);
    }
    resolve_all(&mut g, &[]);
    assert!(lands.iter().all(|&l| !g.obj(l).tapped));
    assert!(g.obj(theirs).tapped, "only yours");
}

#[test]
fn zacama_untaps_lands_when_cast_and_has_three_abilities() {
    let mut g = table();
    let theirs = put(&mut g, 1, "Warren Soultrader");
    let altar = put(&mut g, 1, "Ashnod's Altar");
    let z = hand(&mut g, 0, "Zacama, Primal Calamity");
    play(&mut g, z, false, &[], &[]);
    assert_eq!(
        g.permanents(0).filter(|&x| g.obj(x).tapped).count(),
        0,
        "lands untapped"
    );
    activate(&mut g, z, 0, &[obj(theirs)]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    activate(&mut g, z, 1, &[obj(altar)]);
    assert_eq!(g.obj(altar).zone, Zone::Graveyard);
    activate(&mut g, z, 2, &[]);
    assert_eq!(g.players[0].life, 43);
}

// ---- core theme d1 (mana, costs and keywords) ----

/// How many mana of WUBRG color `c` are in player 0's pool.
fn pooled(g: &Game, c: usize) -> usize {
    g.players[0].pool.iter().filter(|m| m.colors[c]).count()
}

/// Passes until player 0 declares attackers, attacks with `attackers`, then plays on (answering any
/// decision with its first legal action) until the second main phase.
fn attack_with(g: &mut Game, attackers: &[ObjId]) {
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    for &a in attackers {
        act(g, Action::Attack(a));
    }
    act(g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::Decide(_) | Pending::ChooseTarget(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn ignoble_hierarch_has_exalted_and_taps_for_black_red_or_green() {
    let mut g = game(&["Ignoble Hierarch", "Fyndhorn Elves"]);
    let h = named(&g, 0, "Ignoble Hierarch");
    let elf = named(&g, 0, "Fyndhorn Elves");
    assert_eq!(makes(&g, h), ([false, false, true, true, true], false));
    attack_with(&mut g, &[elf]);
    assert_eq!(g.players[1].life, 38, "a lone attacker: 1 + 1");
}

#[test]
fn glen_elendra_archmage_counters_a_noncreature_spell_and_persists() {
    let mut g = table();
    let a = put(&mut g, 0, "Glen Elendra Archmage");
    let nw = hand(&mut g, 0, "Night's Whisper");
    act(&mut g, Action::Cast(nw));
    act(&mut g, Action::Activate(a, 0));
    act(&mut g, Action::Target(obj(nw)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(nw).zone, Zone::Graveyard);
    assert_eq!(g.players[0].life, 40, "countered");
    assert_eq!(g.obj(a).zone, Zone::Battlefield, "persist");
    assert_eq!((g.power(a), g.obj(a).counters), (1, -1));
    let elf = hand(&mut g, 0, "Fyndhorn Elves");
    act(&mut g, Action::Cast(elf));
    assert!(
        !g.legal_actions().contains(&Action::Activate(a, 0)),
        "no noncreature spell to target"
    );
    resolve_all(&mut g, &[]);
    let nw2 = hand(&mut g, 0, "Night's Whisper");
    act(&mut g, Action::Cast(nw2));
    act(&mut g, Action::Activate(a, 0));
    act(&mut g, Action::Target(obj(nw2)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(a).zone, Zone::Graveyard, "it had a -1/-1 counter");
}

#[test]
fn gray_merchant_drains_your_devotion_to_black() {
    let mut g = table();
    put(&mut g, 0, "Dictate of Erebos"); // {3}{B}{B}
    put(&mut g, 1, "Dictate of Erebos"); // the opponent's: not yours
    let m = hand(&mut g, 0, "Gray Merchant of Asphodel");
    play(&mut g, m, false, &[], &[]);
    assert_eq!((g.players[0].life, g.players[1].life), (44, 36));
}

#[test]
fn nykthos_taps_for_colorless_or_devotion_to_a_chosen_color() {
    let mut g = table();
    let n = put(&mut g, 0, "Nykthos, Shrine to Nyx");
    assert_eq!(makes(&g, n), ([false; 5], true));
    put(&mut g, 0, "Dictate of Erebos");
    put(&mut g, 0, "Eternal Witness"); // {1}{G}{G}
    activate(&mut g, n, 2, &[]); // black
    assert_eq!(pooled(&g, 2), 2);
    let mut g = table();
    let n = put(&mut g, 0, "Nykthos, Shrine to Nyx");
    put(&mut g, 0, "Eternal Witness");
    activate(&mut g, n, 4, &[]); // green
    assert_eq!(pooled(&g, 4), 2);
}

#[test]
fn cabal_coffers_adds_black_for_each_swamp() {
    let mut g = table();
    let c = put(&mut g, 0, "Cabal Coffers");
    activate(&mut g, c, 0, &[]);
    assert_eq!(pooled(&g, 2), 5, "five Swamps");
    assert!(g.obj(c).tapped);
}

#[test]
fn cabal_stronghold_adds_black_for_each_basic_swamp() {
    let mut g = table();
    let c = put(&mut g, 0, "Cabal Stronghold");
    assert_eq!(makes(&g, c), ([false; 5], true));
    put(&mut g, 0, "Overgrown Tomb"); // a nonbasic Swamp
    activate(&mut g, c, 0, &[]);
    assert_eq!(pooled(&g, 2), 5, "the five basic Swamps only");
}

#[test]
fn priest_of_titania_adds_green_for_each_elf_on_the_battlefield() {
    let mut g = game(&["Priest of Titania", "Fyndhorn Elves"]);
    let p = named(&g, 0, "Priest of Titania");
    put(&mut g, 1, "Fyndhorn Elves");
    activate(&mut g, p, 0, &[]);
    assert_eq!(pooled(&g, 4), 3, "itself, your Elves and the opponent's");
}

#[test]
fn heronblade_elite_grows_with_humans_and_taps_for_its_power_in_one_color() {
    let mut g = table();
    let e = put(&mut g, 0, "Heronblade Elite");
    let w = hand(&mut g, 0, "Eternal Witness"); // a Human
    play(&mut g, w, false, &[], &[]);
    assert_eq!(g.obj(e).counters, 1);
    let mut g = game(&["Heronblade Elite"]);
    let e = named(&g, 0, "Heronblade Elite");
    set_counters(&mut g, e, 1);
    activate(&mut g, e, 3, &[]); // red
    assert_eq!(pooled(&g, 3), 2);
    assert_eq!(g.players[0].pool.len(), 2);
}

#[test]
fn into_the_story_costs_3_less_with_seven_cards_in_an_opponents_graveyard() {
    let mut g = table();
    for _ in 0..7 {
        g.add_card(1, cid("Forest"), Zone::Graveyard);
    }
    let c = hand(&mut g, 0, "Into the Story");
    let n = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), n + 3);
    let tapped = g.permanents(0).filter(|&x| g.obj(x).tapped).count();
    assert_eq!(tapped, 4, "{{2}}{{U}}{{U}}");
}

#[test]
fn phyrexian_tower_sacrifices_a_creature_for_two_black() {
    let mut g = table();
    let t = put(&mut g, 0, "Phyrexian Tower");
    assert_eq!(makes(&g, t), ([false; 5], true));
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    act(&mut g, Action::ActivateWith(t, 0, elf));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elf).zone, Zone::Graveyard);
    assert_eq!(pooled(&g, 2), 2);
}

#[test]
fn sunbaked_canyon_pays_life_for_red_or_white_and_cycles_itself() {
    let mut g = table();
    let c = put(&mut g, 0, "Sunbaked Canyon");
    activate(&mut g, c, 0, &[]);
    assert_eq!(g.players[0].life, 39);
    let pool = &g.players[0].pool;
    assert_eq!(pool.len(), 1);
    assert!(
        pool[0].colors[3] && pool[0].colors[0] && !pool[0].colors[4],
        "{{R}} or {{W}}"
    );
    let c2 = put(&mut g, 0, "Sunbaked Canyon");
    let n = g.players[0].hand.len();
    activate(&mut g, c2, 1, &[]);
    assert_eq!(g.obj(c2).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n + 1);
}

#[test]
fn wild_cantor_sacrifices_itself_for_one_mana_of_any_color() {
    let mut g = table();
    let w = put(&mut g, 0, "Wild Cantor"); // no {T}: usable while summoning sick
    activate(&mut g, w, 0, &[]);
    assert_eq!(g.obj(w).zone, Zone::Graveyard);
    assert_eq!(g.players[0].pool.len(), 1);
    assert_eq!(g.players[0].pool[0].colors, [true; 5]);
}

#[test]
fn kozilek_draws_four_on_cast_annihilates_and_shuffles_the_graveyard_back() {
    let mut g = table();
    for _ in 0..4 {
        g.add_card(0, cid("Plains"), Zone::Library);
    }
    let k = hand(&mut g, 0, "Kozilek, Butcher of Truth");
    let n = g.players[0].hand.len();
    play(&mut g, k, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), n - 1 + 4, "cast: draw four");
    let altar = put(&mut g, 0, "Ashnod's Altar");
    let gy = g.add_card(0, cid("Night's Whisper"), Zone::Graveyard);
    act(&mut g, Action::ActivateWith(altar, 0, k));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.obj(k).zone, g.obj(gy).zone),
        (Zone::Library, Zone::Library),
        "the graveyard shuffled back"
    );

    let mut g = game(&["Kozilek, Butcher of Truth"]);
    let k = named(&g, 0, "Kozilek, Butcher of Truth");
    let before = g.permanents(1).count();
    attack_with(&mut g, &[k]);
    assert_eq!(g.permanents(1).count(), before - 4, "annihilator 4");
    assert_eq!(g.players[1].life, 28);
}
