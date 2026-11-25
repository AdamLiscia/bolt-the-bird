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
