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

#[test]
fn a_pathway_is_played_as_either_face() {
    for (face, colors) in [
        (0u8, [false, false, true, false, false]),
        (1, [false, false, false, false, true]),
    ] {
        let mut g = table();
        let p = hand(&mut g, 0, "Darkbore Pathway // Slitherbore Pathway");
        act(&mut g, Action::PlayLand(p));
        assert_eq!(
            g.legal_actions(),
            vec![Action::Mode(0), Action::Mode(1)],
            "front or back"
        );
        act(&mut g, Action::Mode(face));
        assert_eq!(g.obj(p).zone, Zone::Battlefield);
        assert_eq!(
            makes(&g, p),
            (colors, false),
            "face {face}: black front, green back"
        );
    }
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

/// Passes until the stack is empty, targeting player 1 when asked and saying yes to decisions.
fn resolve_yes_at_opponent(g: &mut Game) {
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let legal = g.legal_actions();
        let a = match g.pending {
            Pending::ChooseTarget(_) if legal.contains(&Action::Target(Target::Player(1))) => {
                Action::Target(Target::Player(1))
            }
            Pending::Decide(_) if legal.contains(&Action::Yes) => Action::Yes,
            Pending::ChooseTarget(_) | Pending::Decide(_) => legal[0],
            // A free cast (cascade) is offered as a cast at priority.
            _ => legal
                .iter()
                .copied()
                .find(|a| matches!(a, Action::Cast(_)))
                .unwrap_or(Action::PassPriority),
        };
        act(g, a);
    }
}

#[test]
fn noise_marine_cascades_and_deals_damage_per_spell_cast() {
    let mut g = table();
    let nw = g.add_card(0, cid("Night's Whisper"), Zone::Library); // mana value 2: cascaded into
    let m = hand(&mut g, 0, "Noise Marine");
    act(&mut g, Action::Cast(m));
    resolve_yes_at_opponent(&mut g);
    assert_eq!(g.obj(nw).zone, Zone::Graveyard, "cast free and resolved");
    assert_eq!(g.players[0].life, 38, "Night's Whisper's 2 life");
    assert_eq!(g.players[1].life, 38, "two spells cast this turn");
    assert_eq!(g.obj(m).zone, Zone::Battlefield);
}

#[test]
fn baleful_mastery_exiles_and_the_cheap_cost_gifts_a_card() {
    let mut g = table();
    let a = put(&mut g, 1, "Blood Artist");
    let c = hand(&mut g, 0, "Baleful Mastery");
    let n = g.players[1].hand.len();
    act(&mut g, Action::CastAlternative(c));
    act(&mut g, Action::Target(obj(a)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(a).zone, Zone::Exile);
    assert_eq!(
        g.players[1].hand.len(),
        n + 1,
        "{{1}}{{B}} paid: the opponent draws"
    );
    assert_eq!(g.permanents(0).filter(|&x| g.obj(x).tapped).count(), 2);
    let b = put(&mut g, 1, "Blood Artist");
    let c = hand(&mut g, 0, "Baleful Mastery");
    let n = g.players[1].hand.len();
    play(&mut g, c, false, &[obj(b)], &[]);
    assert_eq!(g.obj(b).zone, Zone::Exile);
    assert_eq!(g.players[1].hand.len(), n, "the mana cost: no card");
}

#[test]
fn natures_will_taps_their_lands_and_untaps_yours_on_combat_damage() {
    let mut g = game(&["Fyndhorn Elves", "Forest", "Forest"]);
    put(&mut g, 0, "Nature's Will");
    let elf = named(&g, 0, "Fyndhorn Elves");
    let mine: Vec<ObjId> = g
        .permanents(0)
        .filter(|&x| g.def_of(x).name == "Forest")
        .collect();
    for &f in &mine {
        tap(&mut g, f);
    }
    attack_with(&mut g, &[elf]);
    assert_eq!(g.players[1].life, 39);
    assert!(
        g.permanents(1)
            .filter(|&x| g.types_of(x).land)
            .all(|x| g.obj(x).tapped),
        "theirs tapped"
    );
    assert!(mine.iter().all(|&f| !g.obj(f).tapped), "yours untapped");
}

#[test]
fn jeskas_will_adds_red_per_card_in_their_hand_or_exiles_three_to_play() {
    let mut g = table();
    let c = hand(&mut g, 0, "Jeska's Will");
    assert!(
        !g.legal_actions().contains(&Action::CastModal(c, 2)),
        "both only with a commander out"
    );
    let n = g.players[1].hand.len();
    act(&mut g, Action::CastModal(c, 0));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(pooled(&g, 3), n);
    let mut g = table();
    let top: Vec<ObjId> = (0..3)
        .map(|_| g.add_card(0, cid("Forest"), Zone::Library))
        .collect();
    let c = hand(&mut g, 0, "Jeska's Will");
    act(&mut g, Action::CastModal(c, 1));
    resolve_all(&mut g, &[]);
    assert!(top.iter().all(|&x| g.obj(x).zone == Zone::Exile));
    assert!(
        g.legal_actions().contains(&Action::PlayLand(top[0])),
        "playable this turn"
    );
}

#[test]
fn karmic_guide_reanimates_and_has_echo() {
    let mut g = table();
    let artist = g.add_card(0, cid("Viscera Seer"), Zone::Graveyard);
    let k = hand(&mut g, 0, "Karmic Guide");
    play(&mut g, k, false, &[], &[Action::Target(obj(artist))]);
    assert_eq!(g.obj(artist).zone, Zone::Battlefield);
    assert!(g.keywords(k).has(Keywords::FLYING));
    g.start_turn(5, 0);
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::No); // don't pay echo
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(k).zone, Zone::Graveyard, "sacrificed");
}

// ---- d1 core, batch 3: static cost reductions ----

fn tapped_lands(g: &Game) -> usize {
    g.permanents(0)
        .filter(|&x| g.obj(x).tapped && g.types_of(x).land)
        .count()
}

#[test]
fn jukai_naturalist_makes_enchantment_spells_cost_1_less() {
    let mut g = table();
    put(&mut g, 0, "Jukai Naturalist");
    let c = hand(&mut g, 0, "Sylvan Library");
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(c).zone, Zone::Battlefield);
    assert_eq!(tapped_lands(&g), 1, "{{1}}{{G}} less {{1}}");
}

#[test]
fn bontus_monument_discounts_black_creatures_and_drains_on_creature_casts() {
    let mut g = table();
    put(&mut g, 0, "Bontu's Monument");
    let c = hand(&mut g, 0, "Gray Merchant of Asphodel");
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(c).zone, Zone::Battlefield);
    assert_eq!(tapped_lands(&g), 4, "{{3}}{{B}}{{B}} less {{1}}");
    // The Monument drains 1, then Gray Merchant drains its devotion (2).
    assert_eq!(g.players[1].life, 40 - 1 - 2);
    assert_eq!(g.players[0].life, 40 + 1 + 2);
    // A green creature isn't discounted, but still drains.
    let elf = hand(&mut g, 0, "Llanowar Elves");
    play(&mut g, elf, false, &[], &[]);
    assert_eq!(tapped_lands(&g), 5);
    assert_eq!(g.players[1].life, 36);
}

#[test]
fn animar_grows_on_creature_casts_and_discounts_per_counter() {
    let mut g = table();
    let a = put(&mut g, 0, "Animar, Soul of Elements");
    g.objects[a.0 as usize].counters = 2;
    let c = hand(&mut g, 0, "Gray Merchant of Asphodel");
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(c).zone, Zone::Battlefield);
    assert_eq!(tapped_lands(&g), 3, "{{3}}{{B}}{{B}} less {{2}}");
    assert_eq!(g.obj(a).counters, 3);
}

#[test]
fn rise_of_the_eldrazi_needs_colorless_mana_and_destroys_draws_and_takes_a_turn() {
    let mut g = table();
    let ring = put(&mut g, 0, "Sol Ring");
    let rise = hand(&mut g, 0, "Rise of the Eldrazi");
    assert!(
        !g.legal_actions().contains(&Action::Cast(rise)),
        "Sol Ring's {{C}}{{C}} can't pay {{C}}{{C}}{{C}}"
    );
    let dynamo = put(&mut g, 0, "Thran Dynamo");
    let forest = g.permanents(1).find(|&x| g.def_of(x).name == "Forest").unwrap();
    let n = g.players[0].hand.len();
    play(&mut g, rise, false, &[obj(forest), Target::Player(0)], &[]);
    assert_eq!(g.obj(forest).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n - 1 + 4);
    assert_eq!(g.extra_turns, vec![0]);
    assert_eq!(g.obj(rise).zone, Zone::Exile);
    assert!(
        g.obj(dynamo).tapped && g.obj(ring).tapped,
        "the {{C}} came from the rocks"
    );
}

#[test]
fn shizo_gives_a_legendary_creature_fear() {
    let mut g = table();
    let shizo = put(&mut g, 0, "Shizo, Death's Storehouse");
    let mut lands = 0;
    for x in g.permanents(0).collect::<Vec<_>>() {
        if g.def_of(x).name == "Swamp" && lands < 1 {
            lands += 1;
        } else if g.types_of(x).land && x != shizo {
            tap(&mut g, x); // only a Swamp left to pay {B}
        }
    }
    let animar = put(&mut g, 0, "Animar, Soul of Elements");
    assert!(makes(&g, shizo).0[2], "{{T}}: Add {{B}}");
    activate(&mut g, shizo, 0, &[obj(animar)]);
    assert!(g.keywords(animar).has(Keywords::FEAR));
    assert!(g.obj(shizo).tapped);
}

#[test]
fn haunted_ridge_enters_tapped_unless_you_control_two_other_lands() {
    let mut g = game(&["Swamp"]);
    let early = hand(&mut g, 0, "Haunted Ridge");
    act(&mut g, Action::PlayLand(early));
    assert!(g.obj(early).tapped, "one other land");
    let mut g = game(&["Swamp", "Forest"]);
    let late = hand(&mut g, 0, "Overgrown Farmland");
    act(&mut g, Action::PlayLand(late));
    assert!(!g.obj(late).tapped, "two other lands");
    assert_eq!(makes(&g, late), ([true, false, false, false, true], false));
}

#[test]
fn toxic_deluge_pays_x_life_and_shrinks_every_creature() {
    let mut g = table();
    let mine = put(&mut g, 0, "Llanowar Elves");
    let animar = put(&mut g, 0, "Animar, Soul of Elements");
    set_counters(&mut g, animar, 3); // a 4/4
    let theirs = put(&mut g, 1, "Gray Merchant of Asphodel"); // 2/4
    let c = hand(&mut g, 0, "Toxic Deluge");
    assert!(g.legal_actions().contains(&Action::CastX(c, 30)));
    act(&mut g, Action::CastX(c, 3));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 37, "paid 3 life");
    assert_eq!(g.obj(mine).zone, Zone::Graveyard);
    assert_eq!(g.obj(theirs).zone, Zone::Battlefield, "a 2/4 lives at -1/1");
    assert_eq!(g.obj(animar).zone, Zone::Battlefield);
    assert_eq!(g.power(animar), 1);
}

fn tap_for(g: &mut Game, id: ObjId, color: u8) {
    g.manual_mana[0] = true;
    act(g, Action::TapForMana(id, color));
}

/// Casts the Aura `name` on player 0's land `land`.
fn enchant_land(g: &mut Game, name: &str, land: ObjId) -> ObjId {
    let c = hand(g, 0, name);
    act(g, Action::Cast(c));
    act(g, Action::Target(obj(land)));
    resolve_all(g, &[]);
    assert_eq!(g.obj(c).attached_to.map(|a| a.0), Some(land));
    g.objects[land.0 as usize].tapped = false; // it may have paid for the Aura
    c
}

fn first_named(g: &Game, p: usize, name: &str) -> ObjId {
    g.permanents(p).find(|&x| g.def_of(x).name == name).unwrap()
}

#[test]
fn fertile_ground_adds_one_mana_of_any_color() {
    let mut g = table();
    let forest = first_named(&g, 0, "Forest");
    enchant_land(&mut g, "Fertile Ground", forest);
    let made: Vec<_> = g.mana_sources(0).into_iter().filter(|s| s.0 == forest).collect();
    assert_eq!(made.len(), 2);
    assert!(made.iter().any(|s| s.1.colors == [true; 5]));
    tap_for(&mut g, forest, 4);
    assert_eq!(pooled(&g, 4), 2);
}

#[test]
fn trace_of_abundance_gives_shroud_and_extra_mana() {
    let mut g = table();
    let island = first_named(&g, 0, "Island");
    enchant_land(&mut g, "Trace of Abundance", island);
    assert!(g.keywords(island).has(Keywords::SHROUD));
    assert_eq!(g.mana_sources(0).into_iter().filter(|s| s.0 == island).count(), 2);
}

#[test]
fn buried_in_the_garden_exiles_until_it_leaves_and_adds_mana() {
    let mut g = table();
    let theirs = put(&mut g, 1, "Sol Ring");
    let forest = first_named(&g, 0, "Forest");
    let c = hand(&mut g, 0, "Buried in the Garden");
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(forest)));
    resolve_all(&mut g, &[Action::Target(obj(theirs))]);
    g.objects[forest.0 as usize].tapped = false; // it may have paid for the Aura
    assert_eq!(g.obj(theirs).zone, Zone::Exile);
    assert_eq!(g.mana_sources(0).into_iter().filter(|s| s.0 == forest).count(), 2);
}

#[test]
fn crypt_ghast_doubles_swamps_and_extorts() {
    let mut g = table();
    put(&mut g, 0, "Crypt Ghast");
    let swamp = first_named(&g, 0, "Swamp");
    tap_for(&mut g, swamp, 2);
    assert_eq!(pooled(&g, 2), 2, "an additional {{B}}");
    let forest = first_named(&g, 0, "Forest");
    assert_eq!(g.mana_sources(0).into_iter().filter(|s| s.0 == forest).count(), 1);
    // Extort: cast a spell, pay {W/B} from the pool, drain 1.
    let elf = hand(&mut g, 0, "Llanowar Elves");
    play(&mut g, elf, false, &[], &[Action::Yes]);
    assert_eq!(g.obj(elf).zone, Zone::Battlefield);
    assert_eq!(g.players[1].life, 39);
    assert_eq!(g.players[0].life, 41);
}

#[test]
fn chromatic_lantern_lets_lands_tap_for_any_color() {
    let mut g = table();
    let plains = first_named(&g, 0, "Plains");
    assert_eq!(makes(&g, plains).0, [true, false, false, false, false]);
    let lantern = put(&mut g, 0, "Chromatic Lantern");
    assert_eq!(makes(&g, plains).0, [true; 5]);
    assert_eq!(makes(&g, lantern).0, [true; 5]);
}

#[test]
fn grove_of_the_burnwillows_gives_life_only_for_colored_mana() {
    let mut g = game(&["Grove of the Burnwillows", "Grove of the Burnwillows"]);
    let groves: Vec<_> = g.permanents(0).collect();
    assert_eq!(makes(&g, groves[0]), ([false, false, false, true, true], true));
    tap_for(&mut g, groves[0], 5);
    assert_eq!(g.players[1].life, 40);
    tap_for(&mut g, groves[1], 3);
    assert_eq!(g.players[1].life, 41);
}

#[test]
fn mulldrifter_draws_two_and_evoked_is_sacrificed() {
    let mut g = table();
    let c = hand(&mut g, 0, "Mulldrifter");
    let n = g.players[0].hand.len();
    act(&mut g, Action::CastAlternative(c));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n - 1 + 2);
    assert_eq!(g.obj(c).zone, Zone::Graveyard, "evoked: sacrificed");
    assert_eq!(tapped_lands(&g), 3, "{{2}}{{U}}");
    // Cast for its mana cost, it stays.
    let c = hand(&mut g, 0, "Mulldrifter");
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(c).zone, Zone::Battlefield);
    assert!(g.keywords(c).has(Keywords::FLYING));
}

#[test]
fn reveillark_evoked_returns_two_small_creatures() {
    let mut g = table();
    let a = g.add_card(0, cid("Llanowar Elves"), Zone::Graveyard);
    let b = g.add_card(0, cid("Fyndhorn Elves"), Zone::Graveyard);
    let big = g.add_card(0, cid("Gray Merchant of Asphodel"), Zone::Graveyard);
    let c = hand(&mut g, 0, "Reveillark");
    act(&mut g, Action::CastAlternative(c));
    resolve_all(&mut g, &[Action::Target(obj(a)), Action::Target(obj(b))]);
    assert_eq!(g.obj(c).zone, Zone::Graveyard, "evoked: sacrificed");
    assert_eq!(g.obj(a).zone, Zone::Battlefield);
    assert_eq!(g.obj(b).zone, Zone::Battlefield);
    assert_eq!(g.obj(big).zone, Zone::Graveyard, "power 2 or less only");
}

#[test]
fn chord_of_calling_convokes_with_creatures_and_fetches_up_to_x() {
    // Only three Forests: {X}{G}{G}{G} with X = 2 needs two creatures to tap.
    let mut g = game(&["Forest", "Forest", "Forest"]);
    let a = put(&mut g, 0, "Gray Merchant of Asphodel");
    let b = put(&mut g, 0, "Animar, Soul of Elements");
    g.objects[b.0 as usize].since_turn = g.turn; // summoning sick still convokes
    let target = g.add_card(0, cid("Fyndhorn Elves"), Zone::Library);
    let c = hand(&mut g, 0, "Chord of Calling");
    assert!(g.legal_actions().contains(&Action::CastX(c, 2)));
    act(&mut g, Action::CastX(c, 2));
    resolve_all(&mut g, &[Action::Pick(target)]);
    assert_eq!(g.obj(target).zone, Zone::Battlefield);
    assert!(g.obj(a).tapped && g.obj(b).tapped, "both creatures convoked");
}

#[test]
fn force_of_will_pitches_a_blue_card_and_1_life_to_counter() {
    let mut g = game(&["Forest"]);
    let fow = hand(&mut g, 0, "Force of Will");
    let blue = hand(&mut g, 0, "Brainstorm");
    let elf = hand(&mut g, 0, "Llanowar Elves");
    act(&mut g, Action::Cast(elf));
    assert!(g.legal_actions().contains(&Action::CastWith(fow, blue)));
    act(&mut g, Action::CastWith(fow, blue));
    act(&mut g, Action::Target(obj(elf)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elf).zone, Zone::Graveyard, "countered");
    assert_eq!(g.obj(blue).zone, Zone::Exile);
    assert_eq!(g.obj(fow).zone, Zone::Graveyard);
    assert_eq!(g.players[0].life, 39);
}

#[test]
fn unfathomable_truths_draws_three_and_makes_a_spawn_that_sacrifices_for_colorless() {
    let mut g = table();
    let c = hand(&mut g, 0, "Unfathomable Truths");
    assert_eq!(g.colors_of(c), [false; 5], "devoid");
    let n = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), n - 1 + 3);
    let spawn = tokens(&g, 0, "Eldrazi Spawn");
    assert_eq!(spawn.len(), 1);
    let s = spawn[0];
    assert_eq!((g.power(s), g.toughness(s)), (0, 1));
    assert_eq!(g.colors_of(s), [false; 5]);
    // Just made, and no {T} in its cost: it can still be sacrificed for {C}.
    assert_eq!(makes(&g, s), ([false; 5], true));
    g.manual_mana[0] = true;
    act(&mut g, Action::TapForMana(s, 5));
    assert!(g.players[0].pool.iter().any(|m| m.colorless));
    assert!(tokens(&g, 0, "Eldrazi Spawn").is_empty(), "sacrificed");
}

#[test]
fn magnus_the_red_discounts_spells_per_creature_token_and_makes_spawn() {
    let mut g = table();
    let magnus = put(&mut g, 0, "Magnus the Red");
    let truths = hand(&mut g, 0, "Unfathomable Truths");
    play(&mut g, truths, false, &[], &[]);
    assert_eq!(tapped_lands(&g), 5, "no tokens yet: {{4}}{{U}}");
    // One Spawn now: {1} less. Keep it out of the payment by counting it.
    let c = hand(&mut g, 0, "Unfathomable Truths");
    let before = tapped_lands(&g);
    play(&mut g, c, false, &[], &[]);
    let spawns_left = tokens(&g, 0, "Eldrazi Spawn").len();
    let used = tapped_lands(&g) - before + (2 - spawns_left);
    assert_eq!(used, 4, "{{4}}{{U}} less {{1}} for the one creature token");
    // Blade of Magnus.
    attack_with(&mut g, &[magnus]);
    assert_eq!(tokens(&g, 0, "Spawn").len(), 1);
    let spawn = tokens(&g, 0, "Spawn")[0];
    assert_eq!((g.power(spawn), g.toughness(spawn)), (3, 3));
}

#[test]
fn freyalise_makes_elf_druids_destroys_and_draws_per_green_creature() {
    let mut g = table();
    let f = put(&mut g, 0, "Freyalise, Llanowar's Fury");
    let start = g.obj(f).loyalty;
    act(&mut g, Action::Loyalty(f, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(f).loyalty, start + 2);
    let elf = tokens(&g, 0, "Elf Druid");
    assert_eq!(elf.len(), 1);
    assert_eq!((g.power(elf[0]), g.toughness(elf[0])), (1, 1));
    assert_eq!(
        g.def_of(elf[0]).subtypes,
        vec!["Elf".to_string(), "Druid".to_string()]
    );
    assert_eq!(
        makes(&g, elf[0]),
        ([false, false, false, false, false], false),
        "summoning sick"
    );
    g.objects[elf[0].0 as usize].since_turn = 0;
    assert_eq!(
        makes(&g, elf[0]),
        ([false, false, false, false, true], false),
        "{{T}}: Add {{G}}"
    );
}

// ---- d1 core, batch 4 ----

#[test]
fn ruby_and_jet_medallions_discount_their_color() {
    let mut g = table();
    put(&mut g, 0, "Jet Medallion");
    put(&mut g, 0, "Ruby Medallion");
    let c = hand(&mut g, 0, "Gray Merchant of Asphodel");
    play(&mut g, c, false, &[], &[]);
    assert_eq!(tapped_lands(&g), 4, "black: {{3}}{{B}}{{B}} less {{1}}");
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    assert_eq!(tapped_lands(&g), 5, "red {{R}}: no generic to take off");
    let ench = hand(&mut g, 0, "Sylvan Library");
    play(&mut g, ench, false, &[], &[]);
    assert_eq!(tapped_lands(&g), 7, "green: full price");
}

#[test]
fn stormscape_familiar_discounts_white_or_black_once() {
    let mut g = table();
    put(&mut g, 0, "Stormscape Familiar");
    let c = hand(&mut g, 0, "Gray Merchant of Asphodel");
    play(&mut g, c, false, &[], &[]);
    assert_eq!(tapped_lands(&g), 4);
}

#[test]
fn zendikar_resurgent_doubles_land_mana_and_draws_on_creature_casts() {
    let mut g = table();
    put(&mut g, 0, "Zendikar Resurgent");
    let forest = first_named(&g, 0, "Forest");
    assert_eq!(g.mana_sources(0).into_iter().filter(|s| s.0 == forest).count(), 2);
    let n = g.players[0].hand.len();
    let elf = hand(&mut g, 0, "Llanowar Elves");
    play(&mut g, elf, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), n + 1, "cast one, drew one");
}

#[test]
fn dictate_of_karametra_doubles_every_players_lands() {
    let mut g = table();
    put(&mut g, 1, "Dictate of Karametra");
    let forest = first_named(&g, 0, "Forest");
    assert_eq!(g.mana_sources(0).into_iter().filter(|s| s.0 == forest).count(), 2);
    let theirs = first_named(&g, 1, "Forest");
    assert_eq!(g.mana_sources(1).into_iter().filter(|s| s.0 == theirs).count(), 2);
}

#[test]
fn leyline_of_abundance_adds_g_for_creatures_and_pumps_everything() {
    let mut g = table();
    let l = put(&mut g, 0, "Leyline of Abundance");
    let elf = put(&mut g, 0, "Llanowar Elves");
    assert_eq!(g.mana_sources(0).into_iter().filter(|s| s.0 == elf).count(), 2);
    let forest = first_named(&g, 0, "Forest");
    assert_eq!(
        g.mana_sources(0).into_iter().filter(|s| s.0 == forest).count(),
        1,
        "lands don't"
    );
    activate(&mut g, l, 0, &[]);
    assert_eq!(g.obj(elf).counters, 1);
}

#[test]
fn woe_strider_makes_a_goat_scries_and_escapes_with_two_counters() {
    let mut g = table();
    let w = hand(&mut g, 0, "Woe Strider");
    play(&mut g, w, false, &[], &[]);
    let goat = tokens(&g, 0, "Goat");
    assert_eq!(goat.len(), 1);
    assert_eq!((g.power(goat[0]), g.toughness(goat[0])), (0, 1));
    // Sacrifice another creature: Scry 1.
    act(&mut g, Action::ActivateWith(w, 0, goat[0]));
    while !g.items.is_empty() || matches!(g.pending, Pending::Decide(_)) {
        let a = match g.pending {
            Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(tokens(&g, 0, "Goat").is_empty(), "sacrificed");
    // A second one in the graveyard with only three other cards: no escape.
    let w = g.add_card(0, cid("Woe Strider"), Zone::Graveyard);
    for _ in 0..3 {
        g.add_card(0, cid("Forest"), Zone::Graveyard);
    }
    assert!(!g.legal_actions().contains(&Action::CastAlternative(w)));
    let fourth = g.add_card(0, cid("Forest"), Zone::Graveyard);
    assert!(g.legal_actions().contains(&Action::CastAlternative(w)));
    let before = tapped_lands(&g);
    act(&mut g, Action::CastAlternative(w));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(w).zone, Zone::Battlefield);
    assert_eq!(g.obj(w).counters, 2, "escapes with two +1/+1 counters");
    assert_eq!(tapped_lands(&g) - before, 5, "{{3}}{{B}}{{B}}");
    assert_eq!(g.obj(fourth).zone, Zone::Exile);
    assert_eq!(g.players[0].graveyard.len(), 0, "four other cards exiled");
}

// ---- d1 core, batch 6 ----

#[test]
fn demon_of_deaths_gate_pays_6_life_and_three_black_creatures() {
    let mut g = table();
    let striders = [put(&mut g, 0, "Woe Strider"), put(&mut g, 0, "Woe Strider")];
    let elf = put(&mut g, 0, "Llanowar Elves");
    let d = hand(&mut g, 0, "Demon of Death's Gate");
    assert!(
        !g.legal_actions().contains(&Action::CastAlternative(d)),
        "two black creatures"
    );
    let third = put(&mut g, 0, "Woe Strider");
    act(&mut g, Action::CastAlternative(d));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(d).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, 34);
    for s in striders.into_iter().chain([third]) {
        assert_eq!(g.obj(s).zone, Zone::Graveyard);
    }
    assert_eq!(g.obj(elf).zone, Zone::Battlefield, "not black");
    assert_eq!(tapped_lands(&g), 0);
}

#[test]
fn sephara_taps_four_fliers_and_gives_other_fliers_indestructible() {
    let mut g = table();
    let mut angels: Vec<ObjId> = (0..3).map(|_| put(&mut g, 0, "Serra Angel")).collect();
    let s = hand(&mut g, 0, "Sephara, Sky's Blade");
    assert!(!g.legal_actions().contains(&Action::CastAlternative(s)));
    let birds = put(&mut g, 0, "Birds of Paradise");
    g.objects[birds.0 as usize].since_turn = g.turn; // tapping for a cost isn't {T}
    angels.push(birds);
    act(&mut g, Action::CastAlternative(s));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(s).zone, Zone::Battlefield);
    assert_eq!(tapped_lands(&g), 1, "{{W}}");
    assert!(angels.iter().all(|&a| g.obj(a).tapped));
    assert!(g.keywords(angels[0]).has(Keywords::INDESTRUCTIBLE));
    assert!(!g.keywords(s).has(Keywords::INDESTRUCTIBLE), "other creatures");
}

#[test]
fn sephara_fliers_tapped_for_the_cost_cant_also_make_its_white() {
    // No lands: the {W} must come from a flier that isn't one of the four tapped.
    let mut g = game(&[]);
    let mut fliers: Vec<ObjId> = (0..3).map(|_| put(&mut g, 0, "Serra Angel")).collect();
    let birds = put(&mut g, 0, "Birds of Paradise");
    let s = hand(&mut g, 0, "Sephara, Sky's Blade");
    assert!(
        !g.legal_actions().contains(&Action::CastAlternative(s)),
        "Birds can't both tap for the cost and make {{W}}"
    );
    fliers.push(put(&mut g, 0, "Serra Angel"));
    assert!(g.legal_actions().contains(&Action::CastAlternative(s)));
    act(&mut g, Action::CastAlternative(s));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(s).zone, Zone::Battlefield);
    assert!(fliers.iter().all(|&a| g.obj(a).tapped), "the four Angels");
    assert!(g.obj(birds).tapped, "the Birds made the {{W}}");
}

#[test]
fn zirda_makes_abilities_cost_2_less_but_not_below_one_mana() {
    let mut g = table();
    let z = put(&mut g, 0, "Zirda, the Dawnwaker");
    let sower = put(&mut g, 0, "Sower of Chaos");
    let elf = put(&mut g, 1, "Llanowar Elves");
    activate(&mut g, sower, 0, &[Target::Object(elf)]);
    assert_eq!(tapped_lands(&g), 1, "{{2}}{{R}} less {{2}}");
    assert!(g.keywords(elf).has(Keywords::CANT_BLOCK));
    let bird = put(&mut g, 1, "Birds of Paradise");
    activate(&mut g, z, 0, &[Target::Object(bird)]);
    assert_eq!(tapped_lands(&g), 2, "{{1}} stays {{1}}");
    assert!(g.keywords(bird).has(Keywords::CANT_BLOCK));
}

#[test]
fn umbral_mantle_untaps_the_equipped_creature_to_pump_it() {
    let mut g = table();
    let m = put(&mut g, 0, "Umbral Mantle");
    let elf = put(&mut g, 0, "Llanowar Elves");
    activate(&mut g, m, 1, &[Target::Object(elf)]);
    assert!(
        !g.legal_actions().contains(&Action::Activate(m, 0)),
        "{{Q}} needs it tapped"
    );
    g.objects[elf.0 as usize].tapped = true;
    activate(&mut g, m, 0, &[]);
    assert!(!g.obj(elf).tapped);
    assert_eq!((g.power(elf), g.toughness(elf)), (3, 3));
    assert_eq!(tapped_lands(&g), 3);
}

#[test]
fn survival_of_the_fittest_discards_a_creature_to_tutor_one() {
    let mut g = table();
    let s = put(&mut g, 0, "Survival of the Fittest");
    let fodder = hand(&mut g, 0, "Llanowar Elves");
    let want = g.add_card(0, cid("Serra Angel"), Zone::Library);
    act(&mut g, Action::ActivateWith(s, 0, fodder));
    resolve_all(&mut g, &[Action::Pick(want)]);
    assert_eq!(g.obj(fodder).zone, Zone::Graveyard);
    assert_eq!(g.obj(want).zone, Zone::Hand);
}

#[test]
fn selhoff_entomber_discards_a_creature_to_draw() {
    let mut g = table();
    let s = put(&mut g, 0, "Selhoff Entomber");
    let fodder = hand(&mut g, 0, "Llanowar Elves");
    let plains = hand(&mut g, 0, "Plains");
    assert!(!g.legal_actions().contains(&Action::ActivateWith(s, 0, plains)));
    let n = g.players[0].hand.len();
    act(&mut g, Action::ActivateWith(s, 0, fodder));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(fodder).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n, "discarded one, drew one");
    assert!(g.obj(s).tapped);
}

#[test]
fn tortured_existence_trades_a_discarded_creature_for_one_in_the_graveyard() {
    let mut g = table();
    let t = put(&mut g, 0, "Tortured Existence");
    let back = g.add_card(0, cid("Serra Angel"), Zone::Graveyard);
    let fodder = hand(&mut g, 0, "Llanowar Elves");
    act(&mut g, Action::ActivateWith(t, 0, fodder));
    act(&mut g, Action::Target(Target::Object(back)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(fodder).zone, Zone::Graveyard);
    assert_eq!(g.obj(back).zone, Zone::Hand);
}

#[test]
fn mind_over_matter_discards_to_tap_or_untap() {
    let mut g = table();
    let m = put(&mut g, 0, "Mind Over Matter");
    let elf = put(&mut g, 1, "Llanowar Elves");
    let c = hand(&mut g, 0, "Plains");
    act(&mut g, Action::ActivateWith(m, 0, c));
    act(&mut g, Action::Target(Target::Object(elf)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
    assert!(g.obj(elf).tapped);
    let c = hand(&mut g, 0, "Plains");
    act(&mut g, Action::ActivateWith(m, 0, c));
    act(&mut g, Action::Target(Target::Object(elf)));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(elf).tapped, "untapped");
}

#[test]
fn looter_il_kor_has_shadow_and_loots_on_damage() {
    let mut g = table();
    let l = put(&mut g, 0, "Looter il-Kor");
    put(&mut g, 1, "Llanowar Elves");
    assert!(g.keywords(l).has(Keywords::SHADOW));
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(l));
    act(&mut g, Action::FinishAttacks);
    let n = g.players[0].hand.len();
    while g.step != Step::Main2 {
        let legal = g.legal_actions();
        assert!(!legal.iter().any(|a| matches!(a, Action::Block { .. })), "shadow");
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::Decide(_) => legal[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[1].life, 39);
    assert_eq!(g.players[0].hand.len(), n, "drew one, discarded one");
    assert_eq!(g.players[0].graveyard.len(), 1);
}

#[test]
fn shadow_in_the_warp_discounts_the_first_creature_spell_each_turn() {
    let mut g = table();
    put(&mut g, 0, "Shadow in the Warp");
    let a = hand(&mut g, 0, "Serra Angel");
    play(&mut g, a, false, &[], &[]);
    assert_eq!(tapped_lands(&g), 3, "{{3}}{{W}}{{W}} less {{2}}");
    let b = hand(&mut g, 0, "Serra Angel");
    play(&mut g, b, false, &[], &[]);
    assert_eq!(tapped_lands(&g), 8, "the second one pays in full");
}

// ---- d1 core, batch 7 ----

fn islands(g: &Game) -> Vec<ObjId> {
    g.permanents(0)
        .filter(|&x| g.def_of(x).name == "Island")
        .collect()
}

#[test]
fn gush_returns_two_islands_tapped_ones_first_and_draws_two() {
    let mut g = table();
    let isl = islands(&g);
    g.objects[isl[4].0 as usize].tapped = true;
    let c = hand(&mut g, 0, "Gush");
    let n = g.players[0].hand.len();
    act(&mut g, Action::CastAlternative(c));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
    assert_eq!(g.obj(isl[4]).zone, Zone::Hand, "the tapped Island");
    assert_eq!(islands(&g).len(), 3);
    assert_eq!(g.players[0].hand.len(), n - 1 + 2 + 2);
    assert_eq!(tapped_lands(&g), 0, "no mana paid");
    let mut g = game(&["Island"]);
    let c = hand(&mut g, 0, "Gush");
    assert!(
        !g.legal_actions().contains(&Action::CastAlternative(c)),
        "one Island"
    );
}

#[test]
fn thwart_returns_three_islands_to_counter_a_spell() {
    let mut g = table();
    let nw = hand(&mut g, 0, "Night's Whisper");
    let c = hand(&mut g, 0, "Thwart");
    act(&mut g, Action::Cast(nw));
    let tapped = tapped_lands(&g);
    act(&mut g, Action::CastAlternative(c));
    act(&mut g, Action::Target(obj(nw)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(nw).zone, Zone::Graveyard);
    assert_eq!(g.players[0].life, 40, "countered");
    assert_eq!(islands(&g).len(), 2);
    assert!(tapped_lands(&g) <= tapped, "no mana for Thwart");
}

#[test]
fn harrow_sacrifices_a_land_and_puts_two_basics_onto_the_battlefield_untapped() {
    let mut g = table();
    let forest = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Forest")
        .expect("a Forest");
    let a = g.add_card(0, cid("Island"), Zone::Library);
    let b = g.add_card(0, cid("Swamp"), Zone::Library);
    let c = hand(&mut g, 0, "Harrow");
    assert!(
        !g.legal_actions().contains(&Action::Cast(c)),
        "a land must be sacrificed"
    );
    act(&mut g, Action::CastWith(c, forest));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    resolve_all(&mut g, &[Action::Pick(a), Action::Pick(b)]);
    assert_eq!(g.obj(forest).zone, Zone::Graveyard);
    for x in [a, b] {
        assert_eq!(g.obj(x).zone, Zone::Battlefield);
        assert!(!g.obj(x).tapped);
    }
}

#[test]
fn trade_routes_returns_a_land_and_cycles_land_cards() {
    let mut g = table();
    let tr = put(&mut g, 0, "Trade Routes");
    let isl = islands(&g)[0];
    let theirs = put(&mut g, 1, "Forest");
    act(&mut g, Action::Activate(tr, 0));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(theirs))),
        "you control"
    );
    act(&mut g, Action::Target(obj(isl)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(isl).zone, Zone::Hand);
    let elf = hand(&mut g, 0, "Fyndhorn Elves");
    assert!(
        !g.legal_actions().contains(&Action::ActivateWith(tr, 1, elf)),
        "a land card"
    );
    let n = g.players[0].hand.len();
    act(&mut g, Action::ActivateWith(tr, 1, isl));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(isl).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n, "discarded one, drew one");
}

#[test]
fn murmuring_bosk_enters_tapped_without_a_treefolk_and_only_its_w_or_b_hurts() {
    let mut g = game(&[]);
    let b = hand(&mut g, 0, "Murmuring Bosk");
    act(&mut g, Action::PlayLand(b));
    resolve_all(&mut g, &[]);
    assert!(g.obj(b).tapped, "no Treefolk to reveal");
    let mut g = game(&[]);
    hand(&mut g, 0, "Faeburrow Elder");
    let b = hand(&mut g, 0, "Murmuring Bosk");
    act(&mut g, Action::PlayLand(b));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(b).tapped, "revealed a Treefolk");
    let elf = hand(&mut g, 0, "Fyndhorn Elves");
    play(&mut g, elf, false, &[], &[]);
    assert_eq!(g.players[0].life, 40, "{{G}}: no damage");
    let mut g = game(&["Murmuring Bosk"]);
    let seer = hand(&mut g, 0, "Viscera Seer");
    play(&mut g, seer, false, &[], &[]);
    assert_eq!(g.obj(seer).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, 39, "{{B}}: 1 damage");
}

#[test]
fn natural_order_sacrifices_a_green_creature_for_a_green_creature_card() {
    let mut g = table();
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    let seer = put(&mut g, 0, "Viscera Seer");
    let big = g.add_card(0, cid("Avacyn's Pilgrim"), Zone::Library);
    let black = g.add_card(0, cid("Blood Artist"), Zone::Library);
    let c = hand(&mut g, 0, "Natural Order");
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::CastWith(c, seer)), "not green");
    assert!(!legal.contains(&Action::Cast(c)));
    act(&mut g, Action::CastWith(c, elf));
    pick(&mut g, big, black);
    assert_eq!(g.obj(elf).zone, Zone::Graveyard);
    assert_eq!(g.obj(big).zone, Zone::Battlefield);
}

#[test]
fn sylvan_safekeeper_sacrifices_a_land_to_give_shroud() {
    let mut g = table();
    let s = put(&mut g, 0, "Sylvan Safekeeper");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    let theirs = put(&mut g, 1, "Fyndhorn Elves");
    let land = islands(&g)[0];
    act(&mut g, Action::ActivateWith(s, 0, land));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(theirs))),
        "you control"
    );
    act(&mut g, Action::Target(obj(elf)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(land).zone, Zone::Graveyard);
    assert!(g.keywords(elf).has(Keywords::SHROUD));
}

#[test]
fn once_upon_a_time_is_free_only_as_the_first_spell_this_game() {
    let mut g = table();
    let c = hand(&mut g, 0, "Once Upon a Time");
    assert!(g.legal_actions().contains(&Action::CastAlternative(c)));
    act(&mut g, Action::CastAlternative(c));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    let first = g.legal_actions()[0];
    act(&mut g, first);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
    assert_eq!(tapped_lands(&g), 0, "free");
    let c2 = hand(&mut g, 0, "Once Upon a Time");
    assert!(
        !g.legal_actions().contains(&Action::CastAlternative(c2)),
        "not the first spell"
    );
    assert!(g.legal_actions().contains(&Action::Cast(c2)));
}

#[test]
fn mana_confluence_pays_1_life_for_any_color() {
    let mut g = game(&["Mana Confluence"]);
    let elf = hand(&mut g, 0, "Fyndhorn Elves");
    play(&mut g, elf, false, &[], &[]);
    assert_eq!(g.obj(elf).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, 39);
    let mut g = game(&["Mana Confluence"]);
    g.players[0].life = 0;
    let elf = hand(&mut g, 0, "Fyndhorn Elves");
    assert!(
        !g.legal_actions().contains(&Action::Cast(elf)),
        "can't pay the life"
    );
}

#[test]
fn urza_lands_make_seven_together() {
    let mut g = game(&["Urza's Mine", "Urza's Power Plant"]);
    let big = hand(&mut g, 0, "Gilded Lotus"); // {5}
    assert!(
        !g.legal_actions().contains(&Action::Cast(big)),
        "two lands: {{C}}{{C}}"
    );
    let mut g = game(&["Urza's Mine", "Urza's Power Plant", "Urza's Tower"]);
    let big = hand(&mut g, 0, "Gilded Lotus");
    play(&mut g, big, false, &[], &[]);
    assert_eq!(g.obj(big).zone, Zone::Battlefield, "2 + 2 + 3");
    let mut g = game(&["Urza's Mine", "Urza's Power Plant", "Urza's Tower"]);
    let big = hand(&mut g, 0, "Kozilek, Butcher of Truth"); // {10}
    assert!(!g.legal_actions().contains(&Action::Cast(big)), "seven, not ten");
}

// ---- d1 core, batch 8 ----

#[test]
fn krrik_pays_2_life_for_a_black_pip_mana_cant_pay_and_grows_on_black_spells() {
    let mut g = game(&["Swamp"]);
    let k = put(&mut g, 0, "K'rrik, Son of Yawgmoth");
    let nw = hand(&mut g, 0, "Night's Whisper");
    play(&mut g, nw, false, &[], &[]);
    assert_eq!(
        g.obj(nw).zone,
        Zone::Graveyard,
        "{{1}} from the Swamp, {{B}} for 2 life"
    );
    assert_eq!(g.players[0].life, 40 - 2 - 2);
    assert_eq!(g.obj(k).counters, 1, "a black spell");
    let mut g = table();
    put(&mut g, 0, "K'rrik, Son of Yawgmoth");
    let nw = hand(&mut g, 0, "Night's Whisper");
    play(&mut g, nw, false, &[], &[]);
    assert_eq!(g.players[0].life, 38, "mana first");
}

#[test]
fn meloku_returns_a_land_for_a_flying_illusion() {
    let mut g = table();
    let m = put(&mut g, 0, "Meloku the Clouded Mirror");
    let isl = islands(&g)[0];
    act(&mut g, Action::ActivateWith(m, 0, isl));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(isl).zone, Zone::Hand);
    let t = tokens(&g, 0, "Illusion");
    assert_eq!(t.len(), 1);
    assert!(g.keywords(t[0]).has(Keywords::FLYING));
    assert!(g.colors_of(t[0])[1], "blue");
    assert_eq!((g.power(t[0]), g.toughness(t[0])), (1, 1));
}

#[test]
fn the_shire_enters_tapped_without_a_legend_and_taps_a_creature_for_food() {
    let mut g = game(&[]);
    let s = hand(&mut g, 0, "The Shire");
    act(&mut g, Action::PlayLand(s));
    resolve_all(&mut g, &[]);
    assert!(g.obj(s).tapped, "no legendary creature");
    let mut g = table();
    put(&mut g, 0, "Zirda, the Dawnwaker");
    let s = hand(&mut g, 0, "The Shire");
    act(&mut g, Action::PlayLand(s));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(s).tapped, "a legendary creature");
    g.objects[s.0 as usize].since_turn = 0;
    let z = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Zirda, the Dawnwaker")
        .expect("Zirda");
    act(&mut g, Action::Activate(s, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Food").len(), 1);
    assert!(g.obj(s).tapped && g.obj(z).tapped, "the Shire and a creature");
}

// ---- d1 core, batch 9: improvise, multikicker, forage, restricted and conditional mana ----

fn legal(g: &Game, a: Action) -> bool {
    g.legal_actions().contains(&a)
}

#[test]
fn universal_surveillance_improvises_with_untapped_artifacts() {
    let mut g = game(&["Island", "Island", "Island"]);
    let racks = [put(&mut g, 0, "Scroll Rack"), put(&mut g, 0, "Scroll Rack")];
    let c = hand(&mut g, 0, "Universal Surveillance");
    assert!(
        legal(&g, Action::CastX(c, 2)),
        "{{2}}{{U}}{{U}}{{U}}: three Islands and two artifacts"
    );
    assert!(!legal(&g, Action::CastX(c, 3)));
    let before = g.players[0].hand.len();
    act(&mut g, Action::CastX(c, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), before - 1 + 2, "drew X = 2");
    assert!(
        racks.iter().all(|&r| g.obj(r).tapped),
        "both artifacts tapped for {{1}} each"
    );
}

#[test]
fn inspiring_statuary_gives_nonartifact_spells_improvise() {
    let mut g = game(&["Swamp"]);
    let statuary = put(&mut g, 0, "Inspiring Statuary");
    let whisper = hand(&mut g, 0, "Night's Whisper");
    let altar = hand(&mut g, 0, "Ashnod's Altar");
    put(&mut g, 0, "Scroll Rack");
    assert!(
        !legal(&g, Action::Cast(altar)),
        "an artifact spell doesn't improvise: {{3}} from one Swamp"
    );
    act(&mut g, Action::Cast(whisper));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(whisper).zone, Zone::Graveyard);
    let tapped = g
        .permanents(0)
        .filter(|&x| g.types_of(x).artifact && g.obj(x).tapped)
        .count();
    assert_eq!(tapped, 1, "one artifact paid the {{1}}");
    assert!(g.obj(statuary).zone == Zone::Battlefield);
}

#[test]
fn kappa_cannoneer_improvises_and_grows_unblockable_when_artifacts_enter() {
    let mut g = game(&["Island", "Swamp"]);
    for _ in 0..4 {
        put(&mut g, 0, "Scroll Rack");
    }
    let k = hand(&mut g, 0, "Kappa Cannoneer");
    act(&mut g, Action::Cast(k));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.obj(k).zone,
        Zone::Battlefield,
        "{{5}}{{U}}: two lands and four artifacts"
    );
    assert_eq!(g.obj(k).counters, 1, "it's an artifact entering itself");
    assert!(g.keywords(k).has(Keywords::UNBLOCKABLE));
    assert!(g.def_of(k).ward.is_some());
}

#[test]
fn drift_of_phantasms_transmutes_for_a_three_drop() {
    let mut g = game(&["Island", "Island", "Island"]);
    let target = g.add_card(0, cid("Inspiring Statuary"), Zone::Library);
    let drift = hand(&mut g, 0, "Drift of Phantasms");
    act(&mut g, Action::Activate(drift, 0));
    resolve_all(&mut g, &[Action::Pick(target)]);
    assert_eq!(g.obj(drift).zone, Zone::Graveyard);
    assert_eq!(g.obj(target).zone, Zone::Hand);
}

#[test]
fn crystal_vein_taps_for_one_or_sacrifices_for_two() {
    let mut g = game(&[]);
    let v = put(&mut g, 0, "Crystal Vein");
    assert_eq!(makes(&g, v), ([false; 5], true));
    act(&mut g, Action::Activate(v, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(v).zone, Zone::Graveyard);
    assert_eq!(g.players[0].pool.len(), 2);
}

#[test]
fn eldrazi_temple_makes_two_for_colorless_eldrazi_only() {
    let mut g = game(&["Plains"]);
    let t = put(&mut g, 0, "Eldrazi Temple");
    assert_eq!(makes(&g, t), ([false; 5], true));
    let raker = hand(&mut g, 0, "Glaring Fleshraker");
    let altar = hand(&mut g, 0, "Ashnod's Altar");
    assert!(
        !legal(&g, Action::Cast(altar)),
        "{{3}} for a non-Eldrazi: two mana"
    );
    act(&mut g, Action::Cast(raker));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.obj(raker).zone,
        Zone::Battlefield,
        "{{2}}{{C}}: the Temple's {{C}}{{C}} and a Plains"
    );
}

#[test]
fn shrine_of_the_forsaken_gods_makes_two_for_colorless_spells_with_seven_lands() {
    let mut g = game(&["Plains"; 6]);
    put(&mut g, 0, "Shrine of the Forsaken Gods");
    let p = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Plains")
        .expect("a Plains");
    tap(&mut g, p);
    let sire = hand(&mut g, 0, "Sire of Seven Deaths");
    act(&mut g, Action::Cast(sire));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.obj(sire).zone,
        Zone::Battlefield,
        "{{7}}: five Plains and the Shrine's {{C}}{{C}}"
    );
    let mut g2 = game(&["Plains"; 5]);
    put(&mut g2, 0, "Shrine of the Forsaken Gods");
    let sire = hand(&mut g2, 0, "Sire of Seven Deaths");
    assert!(!legal(&g2, Action::Cast(sire)), "six lands: only {{C}}");
}

#[test]
fn nimbus_maze_adds_white_with_an_island_and_blue_with_a_plains() {
    let mut g = game(&[]);
    let m = put(&mut g, 0, "Nimbus Maze");
    assert_eq!(makes(&g, m), ([false; 5], true));
    put(&mut g, 0, "Island");
    assert_eq!(makes(&g, m), ([true, false, false, false, false], true));
    put(&mut g, 0, "Plains");
    assert_eq!(makes(&g, m), ([true, true, false, false, false], true));
}

#[test]
fn everflowing_chalice_enters_with_a_charge_counter_per_kick() {
    let mut g = game(&["Plains"; 4]);
    let c = hand(&mut g, 0, "Everflowing Chalice");
    assert!(legal(&g, Action::CastX(c, 2)));
    assert!(!legal(&g, Action::CastX(c, 3)), "three kicks cost {{6}}");
    act(&mut g, Action::CastX(c, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).kind_counters, 2);
    let n = g.mana_sources(0).iter().filter(|s| s.0 == c).count();
    assert_eq!(n, 2, "{{C}} for each charge counter");
    let mut g = game(&[]);
    let c = hand(&mut g, 0, "Everflowing Chalice");
    act(&mut g, Action::CastX(c, 0));
    resolve_all(&mut g, &[]);
    assert!(g.mana_sources(0).iter().all(|s| s.0 != c), "no counters, no mana");
}

#[test]
fn hall_of_the_bandit_lord_costs_three_life_and_gives_its_creature_haste() {
    let mut g = game(&["Swamp"]);
    let hall = put(&mut g, 0, "Hall of the Bandit Lord");
    let artist = hand(&mut g, 0, "Blood Artist");
    act(&mut g, Action::Cast(artist));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(artist).zone, Zone::Battlefield);
    assert!(g.obj(hall).tapped);
    assert_eq!(g.players[0].life, 37);
    assert!(
        g.keywords(artist).has(Keywords::HASTE),
        "its mana paid for a creature spell"
    );
    let mut g = game(&[]);
    let h = hand(&mut g, 0, "Hall of the Bandit Lord");
    act(&mut g, Action::PlayLand(h));
    assert!(g.obj(h).tapped, "enters tapped");
}

#[test]
fn feed_the_cycle_forages_or_pays_black() {
    let mut g = game(&["Swamp", "Plains"]);
    let elf = put(&mut g, 1, "Fyndhorn Elves");
    let c = hand(&mut g, 0, "Feed the Cycle");
    assert!(!legal(&g, Action::Cast(c)), "{{1}}{{B}} and {{B}} from two lands");
    let gy: Vec<ObjId> = (0..3)
        .map(|_| g.add_card(0, cid("Plains"), Zone::Graveyard))
        .collect();
    let named = g
        .legal_actions()
        .into_iter()
        .find_map(|a| match a {
            Action::CastWith(x, y) if x == c => Some(y),
            _ => None,
        })
        .expect("forage by exiling three");
    assert!(gy.contains(&named));
    act(&mut g, Action::CastWith(c, named));
    act(&mut g, Action::Target(obj(elf)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elf).zone, Zone::Graveyard);
    assert!(
        gy.iter().all(|&x| g.obj(x).zone == Zone::Exile),
        "foraged: three cards exiled"
    );
    let mut g = game(&["Swamp", "Swamp", "Swamp"]);
    let elf = put(&mut g, 1, "Fyndhorn Elves");
    let c = hand(&mut g, 0, "Feed the Cycle");
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(elf)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elf).zone, Zone::Graveyard, "paid {{1}}{{B}}{{B}}");
}

#[test]
fn verges_add_their_second_color_with_either_land_type() {
    for (name, first, second, types) in [
        ("Blazemire Verge", 2, 3, ["Swamp", "Mountain"]),
        ("Bleachbone Verge", 2, 0, ["Plains", "Swamp"]),
        ("Floodfarm Verge", 0, 1, ["Plains", "Island"]),
        ("Gloomlake Verge", 1, 2, ["Island", "Swamp"]),
        ("Wastewood Verge", 4, 2, ["Swamp", "Forest"]),
    ] {
        let mut g = game(&[]);
        let v = put(&mut g, 0, name);
        let mut only = [false; 5];
        only[first] = true;
        assert_eq!(makes(&g, v), (only, false), "{name} alone");
        for t in types {
            let mut g = game(&[]);
            let v = put(&mut g, 0, name);
            put(&mut g, 0, t);
            let mut both = only;
            both[second] = true;
            assert_eq!(makes(&g, v), (both, false), "{name} with a {t}");
        }
    }
}

#[test]
fn tainted_peak_adds_black_or_red_only_with_a_swamp() {
    let mut g = game(&[]);
    let t = put(&mut g, 0, "Tainted Peak");
    assert_eq!(makes(&g, t), ([false; 5], true));
    put(&mut g, 0, "Swamp");
    assert_eq!(makes(&g, t), ([false, false, true, true, false], true));
}
