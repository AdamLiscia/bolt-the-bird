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
    assert!(
        !g.legal_actions().contains(&Action::Activate(s, 0)),
        "a creature is named"
    );
    act(&mut g, Action::ActivateWith(s, 0, z));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Food").len(), 1);
    assert!(
        g.obj(s).tapped && g.obj(z).tapped,
        "the Shire and the named creature"
    );
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

// ---- d1 core, batch 11: Minas Tirith, Watcher of the Spheres ----

#[test]
fn minas_tirith_draws_only_after_attacking_with_two() {
    let mut g = game(&[]);
    let s = hand(&mut g, 0, "Minas Tirith");
    act(&mut g, Action::PlayLand(s));
    resolve_all(&mut g, &[]);
    assert!(g.obj(s).tapped, "no legendary creature");
    let mut g = game(&["Plains", "Plains"]);
    let m = put(&mut g, 0, "Minas Tirith");
    let lions = [put(&mut g, 0, "Savannah Lions"), put(&mut g, 0, "Savannah Lions")];
    for &x in lions.iter().chain([&m]) {
        g.objects[x.0 as usize].since_turn = 0;
    }
    assert!(!legal(&g, Action::Activate(m, 0)), "no attack yet");
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    for &l in &lions {
        act(&mut g, Action::Attack(l));
    }
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        match g.pending {
            Pending::DeclareBlockers(_) => {
                act(&mut g, Action::FinishBlocks);
            }
            _ => {
                act(&mut g, Action::PassPriority);
            }
        }
    }
    let before = g.players[0].hand.len();
    act(&mut g, Action::Activate(m, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), before + 1);
}

#[test]
fn watcher_of_the_spheres_discounts_fliers_and_grows_when_one_enters() {
    let mut g = game(&["Plains", "Plains", "Plains", "Plains"]);
    let w = put(&mut g, 0, "Watcher of the Spheres");
    let angel = hand(&mut g, 0, "Serra Angel");
    act(&mut g, Action::Cast(angel));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.obj(angel).zone,
        Zone::Battlefield,
        "{{2}}{{W}}{{W}} from four Plains"
    );
    assert_eq!(g.power(w), 3, "+1/+1 for the Angel");
}

#[test]
fn desecrate_reality_returns_an_odd_permanent_only_with_three_colorless() {
    // Sol Ring's {C}{C}, plus Crystal Vein's {C} for the second cast.
    for (vein, returned) in [(false, false), (true, true)] {
        let mut lands = vec!["Sol Ring"];
        if vein {
            lands.push("Crystal Vein");
        }
        lands.extend(vec!["Plains"; 7 - 2 - vein as usize]);
        let mut g = game(&lands);
        let stone = put(&mut g, 1, "Mind Stone");
        let elves = g.add_card(0, cid("Llanowar Elves"), Zone::Graveyard);
        let c = hand(&mut g, 0, "Desecrate Reality");
        act(&mut g, Action::Cast(c));
        act(&mut g, Action::Target(obj(stone)));
        let pick = [Action::Pick(elves)];
        resolve_all(&mut g, if returned { &pick } else { &[] });
        assert_eq!(g.obj(stone).zone, Zone::Exile);
        let back = g.obj(elves).zone == Zone::Battlefield;
        assert_eq!(back, returned, "Crystal Vein: {vein}");
    }
}

#[test]
fn spire_of_industry_pays_life_for_color_only_with_an_artifact() {
    let mut g = game(&["Spire of Industry"]);
    let s = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Spire of Industry")
        .unwrap();
    let elves = hand(&mut g, 0, "Llanowar Elves");
    assert!(!legal(&g, Action::Cast(elves)), "no artifact: {{C}} only");
    let stone = put(&mut g, 0, "Mind Stone");
    g.objects[stone.0 as usize].tapped = true;
    let life = g.players[0].life;
    act(&mut g, Action::Cast(elves));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
    assert!(g.obj(s).tapped);
    assert_eq!(g.players[0].life, life - 1, "1 life for {{G}}");
    // Its {C} costs no life.
    let mut g = game(&["Spire of Industry"]);
    put(&mut g, 0, "Mind Stone");
    let ring = hand(&mut g, 0, "Sol Ring");
    let life = g.players[0].life;
    act(&mut g, Action::Cast(ring));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, life);
}

#[test]
fn chandras_incinerator_costs_less_after_noncombat_damage_and_redirects_it() {
    let mut g = game(&["Mountain"; 4]);
    let inc = hand(&mut g, 0, "Chandra's Incinerator");
    assert!(!legal(&g, Action::Cast(inc)), "{{5}}{{R}} from four Mountains");
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert!(legal(&g, Action::Cast(inc)), "{{3}} less: {{2}}{{R}}");
    act(&mut g, Action::Cast(inc));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(inc).zone, Zone::Battlefield);
    // A noncombat hit on the opponent: that much to their creature.
    let bear = put(&mut g, 1, "Savannah Lions");
    let shock = hand(&mut g, 0, "Lightning Bolt");
    put(&mut g, 0, "Mountain");
    act(&mut g, Action::Cast(shock));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[Action::Target(obj(bear))]);
    assert_eq!(g.obj(bear).zone, Zone::Graveyard);
}

#[test]
fn behold_the_multiverse_is_foretold_then_cast_for_its_foretell_cost_next_turn() {
    let mut g = game(&["Island", "Island", "Island", "Island"]);
    let c = hand(&mut g, 0, "Behold the Multiverse");
    act(&mut g, Action::Activate(c, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).zone, Zone::Exile, "foretold for {{2}}");
    assert!(!legal(&g, Action::Cast(c)), "not the turn it was foretold");
    // Player 0's next turn.
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let lands: Vec<ObjId> = g.permanents(0).filter(|&x| g.types_of(x).land).collect();
    for &l in &lands[2..] {
        g.objects[l.0 as usize].tapped = true;
    }
    let before = g.players[0].hand.len();
    act(&mut g, Action::Cast(c));
    resolve_all(&mut g, &[Action::Yes, Action::Yes]); // scry: both stay on top
    assert_eq!(g.obj(c).zone, Zone::Graveyard, "{{1}}{{U}} from two Islands");
    assert_eq!(g.players[0].hand.len(), before + 2);
}

#[test]
fn eye_of_ugin_discounts_colorless_eldrazi_spells() {
    let mut g = game(&["Eye of Ugin", "Plains", "Plains", "Plains", "Plains"]);
    let k = hand(&mut g, 0, "Conduit of Ruin"); // {6} colorless Eldrazi
    act(&mut g, Action::Cast(k));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.obj(k).zone,
        Zone::Battlefield,
        "{{4}}: four Plains (and the Eye's {{C}} spare)"
    );
}

#[test]
fn war_room_draws_for_life_equal_to_commander_colors() {
    let mut g = game(&["War Room", "Plains", "Plains", "Plains"]);
    let w = g.permanents(0).find(|&x| g.def_of(x).name == "War Room").unwrap();
    let n = g.commander_color_count(0);
    let (life, hand) = (g.players[0].life, g.players[0].hand.len());
    act(&mut g, Action::Activate(w, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, life - n);
    assert_eq!(g.players[0].hand.len(), hand + 1);
}

// ---- d1 core, batch 14 ----

#[test]
fn pride_of_the_clouds_grows_per_other_flier_and_forecasts_a_bird() {
    let mut g = game(&["Plains", "Plains", "Island", "Island"]);
    let pride = put(&mut g, 0, "Pride of the Clouds");
    assert_eq!(g.power(pride), 1);
    put(&mut g, 0, "Meloku the Clouded Mirror");
    put(&mut g, 1, "Watcher of the Spheres");
    assert_eq!(
        (g.power(pride), g.toughness(pride)),
        (3, 3),
        "two other fliers, anyone's"
    );
    // Forecast from hand, during your upkeep.
    let card = hand(&mut g, 0, "Pride of the Clouds");
    assert!(!legal(&g, Action::Activate(card, 0)), "main phase");
    g.start_turn(5, 0);
    while g.step != Step::Upkeep {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Activate(card, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(card).zone, Zone::Hand, "revealed");
    let birds = tokens(&g, 0, "Bird");
    assert_eq!(birds.len(), 1);
    let b = birds[0];
    assert!(g.keywords(b).has(Keywords::FLYING));
    assert_eq!(g.def_of(b).colors, [true, true, false, false, false]);
    assert_eq!((g.power(pride), g.toughness(pride)), (4, 4), "the Bird flies too");
}

#[test]
fn marionette_apprentice_fabricates_and_drains_when_another_creature_dies() {
    let mut g = game(&["Swamp", "Swamp"]);
    let m = hand(&mut g, 0, "Marionette Apprentice");
    play(&mut g, m, false, &[], &[Action::Yes]);
    assert_eq!(g.obj(m).counters, 1, "fabricate 1: a +1/+1 counter");
    let altar = put(&mut g, 0, "Ashnod's Altar");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    let life = g.players[1].life;
    act(&mut g, Action::ActivateWith(altar, 0, elf));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 1);
}

#[test]
fn marionette_master_makes_servos_and_drains_its_power_when_an_artifact_dies() {
    let mut g = table();
    let m = hand(&mut g, 0, "Marionette Master");
    play(&mut g, m, false, &[], &[Action::No]);
    let servos = tokens(&g, 0, "Servo");
    assert_eq!(servos.len(), 3, "fabricate 3: three Servos");
    assert_eq!(g.power(m), 1);
    let altar = put(&mut g, 0, "Ashnod's Altar");
    let life = g.players[1].life;
    act(&mut g, Action::ActivateWith(altar, 0, servos[0]));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].life, life - 1, "the Master's power");
}

#[test]
fn food_chain_exiles_a_creature_for_mana_only_creature_spells_spend() {
    let mut g = game(&[]);
    let chain = put(&mut g, 0, "Food Chain");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    g.objects[elf.0 as usize].tapped = true; // not its own mana
    act(&mut g, Action::ActivateWith(chain, 0, elf));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elf).zone, Zone::Exile);
    let sign = hand(&mut g, 0, "Sign in Blood");
    assert!(!legal(&g, Action::Cast(sign)), "not a creature spell");
    let seer = hand(&mut g, 0, "Viscera Seer");
    act(&mut g, Action::Cast(seer));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.obj(seer).zone,
        Zone::Battlefield,
        "two mana: 1 plus the Elves' mana value"
    );
}

#[test]
fn waves_of_aggression_retraces_from_the_graveyard_by_discarding_a_land() {
    let mut g = table();
    let w = g.add_card(0, cid("Waves of Aggression"), Zone::Graveyard);
    let land = hand(&mut g, 0, "Mountain");
    act(&mut g, Action::CastWith(w, land));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(land).zone, Zone::Graveyard, "discarded");
    assert_eq!(g.obj(w).zone, Zone::Graveyard, "retraced: back to the graveyard");
}

#[test]
fn darksteel_monolith_casts_a_colorless_spell_from_hand_for_zero_once_each_turn() {
    let mut g = game(&[]);
    put(&mut g, 0, "Darksteel Monolith");
    let a = hand(&mut g, 0, "Ashnod's Altar");
    let b = hand(&mut g, 0, "Phyrexian Altar");
    act(&mut g, Action::CastAlternative(a));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(a).zone, Zone::Battlefield, "{{3}} paid as {{0}}");
    assert!(!legal(&g, Action::CastAlternative(b)), "once each turn");
}

// ---- The Gitrog, Ravenous Ride (RuricMo) ----

#[test]
fn witchs_cottage_enters_tapped_unless_three_other_swamps_then_tops_a_creature_card() {
    let mut g = game(&["Swamp", "Swamp"]);
    let c = hand(&mut g, 0, "Witch's Cottage");
    act(&mut g, Action::PlayLand(c));
    resolve_all(&mut g, &[]);
    assert!(g.obj(c).tapped, "two other Swamps");
    let mut g = game(&["Swamp", "Swamp", "Swamp"]);
    let elves = g.add_card(0, cid("Llanowar Elves"), Zone::Graveyard);
    let c = hand(&mut g, 0, "Witch's Cottage");
    act(&mut g, Action::PlayLand(c));
    resolve_all(&mut g, &[Action::Target(Target::Object(elves)), Action::Yes]);
    assert!(!g.obj(c).tapped, "three other Swamps");
    assert_eq!(g.obj(elves).zone, Zone::Library);
    assert_eq!(*g.players[0].library.last().unwrap(), elves, "on top");
}

#[test]
fn jarad_drains_the_sacrificed_power_and_returns_for_a_swamp_and_a_forest() {
    let mut g = table();
    let jarad = put(&mut g, 0, "Jarad, Golgari Lich Lord");
    let elves = put(&mut g, 0, "Llanowar Elves");
    g.add_card(0, cid("Llanowar Elves"), Zone::Graveyard);
    assert_eq!(g.power(jarad), 3, "one creature card in the graveyard");
    let life = g.players[1].life;
    act(&mut g, Action::ActivateWith(jarad, 0, elves));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 1, "the Elves' power");
    // From the graveyard: a Swamp and a Forest.
    let mut g2 = table();
    let jarad = g2.add_card(0, cid("Jarad, Golgari Lich Lord"), Zone::Graveyard);
    let swamps = |g: &Game| g.permanents(0).filter(|&x| g.def_of(x).name == "Swamp").count();
    let forests = |g: &Game| g.permanents(0).filter(|&x| g.def_of(x).name == "Forest").count();
    let (s, f) = (swamps(&g2), forests(&g2));
    let ret = g2
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a, Action::Activate(c, _) if *c == jarad))
        .expect("its graveyard ability");
    act(&mut g2, ret);
    resolve_all(&mut g2, &[]);
    assert_eq!(g2.obj(jarad).zone, Zone::Hand);
    assert_eq!((swamps(&g2), forests(&g2)), (s - 1, f - 1));
}

#[test]
fn sunbirds_invocation_reveals_the_spells_mana_value_and_casts_one_free() {
    let mut g = table();
    put(&mut g, 0, "Sunbird's Invocation");
    let elves = g.add_card(0, cid("Llanowar Elves"), Zone::Library);
    let forest = g.add_card(0, cid("Forest"), Zone::Library); // the top card
    let deep = g.players[0].library[g.players[0].library.len() - 3];
    let signet = hand(&mut g, 0, "Arcane Signet");
    act(&mut g, Action::Cast(signet));
    while !matches!(g.pending, Pending::FreeCast(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(deep).zone, Zone::Library, "X is 2: only the top two");
    assert_eq!(g.players[0].library[0], forest, "the land to the bottom");
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Cast(elves)), "{legal:?}");
    act(&mut g, Action::Cast(elves));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.obj(elves).zone,
        Zone::Battlefield,
        "cast free from among the revealed"
    );
    assert_eq!(g.obj(signet).zone, Zone::Battlefield);
}

/// The Gitrog saddled by two creatures, attacking and connecting: at the "you may" decision.
fn gitrog_connects(g: &mut Game) -> (ObjId, ObjId, ObjId) {
    let gitrog = put(g, 0, "The Gitrog, Ravenous Ride");
    let small = put(g, 0, "Llanowar Elves"); // power 1
    let big = put(g, 0, "Reclamation Sage"); // power 2
    for r in [small, big] {
        act(g, Action::ActivateWith(gitrog, 0, r));
        resolve_all(g, &[]);
        assert!(g.obj(r).tapped, "saddling taps it");
    }
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    act(g, Action::Attack(gitrog));
    act(g, Action::FinishAttacks);
    for _ in 0..40 {
        if matches!(g.pending, Pending::Decide(0)) {
            break;
        }
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
    (gitrog, small, big)
}

#[test]
fn the_gitrog_may_sacrifice_a_chosen_rider_then_puts_up_to_x_lands() {
    // Declined: nothing happens.
    let mut g = table();
    let (_, small, big) = gitrog_connects(&mut g);
    let legal = g.legal_actions();
    assert!(
        legal.contains(&Action::Pick(small))
            && legal.contains(&Action::Pick(big))
            && legal.contains(&Action::No)
    );
    let hand = g.players[0].hand.len();
    act(&mut g, Action::No);
    assert_eq!(
        (g.obj(small).zone, g.obj(big).zone),
        (Zone::Battlefield, Zone::Battlefield)
    );
    assert_eq!(g.players[0].hand.len(), hand);

    // The smaller rider chosen (not the most powerful): draw 1, then no lands put down.
    let mut g = table();
    let (_, small, big) = gitrog_connects(&mut g);
    g.add_card(0, cid("Forest"), Zone::Hand);
    let hand = g.players[0].hand.len();
    act(&mut g, Action::Pick(small));
    assert_eq!(g.obj(small).zone, Zone::Graveyard);
    assert_eq!(g.obj(big).zone, Zone::Battlefield);
    assert_eq!(g.players[0].hand.len(), hand + 1, "drew one");
    assert!(g.legal_actions().contains(&Action::Amount(1)));
    act(&mut g, Action::Amount(0));
    assert_eq!(g.players[0].hand.len(), hand + 1, "chose fewer lands: none");

    // The bigger one: draw 2, put one land of the two allowed.
    let mut g = table();
    let (_, _, big) = gitrog_connects(&mut g);
    let forest = g.add_card(0, cid("Forest"), Zone::Hand);
    g.add_card(0, cid("Forest"), Zone::Hand);
    act(&mut g, Action::Pick(big));
    act(&mut g, Action::Amount(1));
    act(&mut g, Action::Pick(forest));
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(g.obj(forest).tapped);
    assert!(!matches!(g.pending, Pending::Decide(0)), "only one");
}

#[test]
fn karador_costs_less_per_creature_card_and_casts_one_creature_from_the_graveyard_each_turn() {
    let mut g = game(&["Plains", "Swamp", "Forest", "Forest"]);
    let k = hand(&mut g, 0, "Karador, Ghost Chieftain");
    for _ in 0..3 {
        g.add_card(0, cid("Fyndhorn Elves"), Zone::Graveyard);
    }
    assert!(
        !g.legal_actions().contains(&Action::Cast(k)),
        "{{2}}{{W}}{{B}}{{G}} with four lands"
    );
    g.add_card(0, cid("Fyndhorn Elves"), Zone::Graveyard);
    assert!(
        g.legal_actions().contains(&Action::Cast(k)),
        "{{1}}{{W}}{{B}}{{G}}"
    );

    let mut g = table();
    put(&mut g, 0, "Karador, Ghost Chieftain");
    let a = g.add_card(0, cid("Llanowar Elves"), Zone::Graveyard);
    let b = g.add_card(0, cid("Fyndhorn Elves"), Zone::Graveyard);
    let karador = Action::CastFromGraveyardAs(a, bolt_core::game::KARADOR_CAST);
    assert!(g.legal_actions().contains(&karador));
    act(&mut g, karador);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(a).zone, Zone::Battlefield);
    assert!(
        !g.legal_actions()
            .contains(&Action::CastFromGraveyardAs(b, bolt_core::game::KARADOR_CAST)),
        "once each turn"
    );
}

// ---- Animar, Soul of Elements ----

#[test]
fn walking_ballista_enters_with_x_and_pings_by_removing_counters() {
    let mut g = table();
    let b = hand(&mut g, 0, "Walking Ballista");
    act(&mut g, Action::CastX(b, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(b).counters, 1, "X = 1");
    let life = g.players[1].life;
    act(&mut g, Action::Activate(b, 1));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 1);
    assert_eq!(g.obj(b).zone, Zone::Graveyard, "a 0/0 with no counters");
}

#[test]
fn somberwald_sage_makes_three_mana_of_one_color_for_creature_spells_only() {
    let mut g = game(&[]);
    let sage = put(&mut g, 0, "Somberwald Sage");
    g.objects[sage.0 as usize].since_turn = 0;
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    let gw = hand(&mut g, 0, "Jukai Naturalist"); // {G}{W}
    let green = hand(&mut g, 0, "Reclamation Sage"); // {2}{G}
    act(&mut g, Action::Activate(sage, 4)); // green (WUBRG order)
    resolve_all(&mut g, &[]);
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Cast(bolt)), "not for an instant");
    assert!(
        !legal.contains(&Action::Cast(gw)),
        "green mana can't pay the {{W}}"
    );
    assert!(
        legal.contains(&Action::Cast(green)),
        "{{2}}{{G}} all paid in green: {legal:?}"
    );
    act(&mut g, Action::Cast(green));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(green).zone, Zone::Battlefield);
}

#[test]
fn vizier_of_the_menagerie_spends_any_mana_on_creature_spells() {
    let mut g = game(&["Plains"]);
    put(&mut g, 0, "Vizier of the Menagerie");
    let elves = hand(&mut g, 0, "Llanowar Elves");
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Cast(elves)), "{{G}} paid with a Plains");
    assert!(!legal.contains(&Action::Cast(bolt)), "creature spells only");
    let top = g.add_card(0, cid("Fyndhorn Elves"), Zone::Library);
    assert!(
        g.legal_actions().contains(&Action::Cast(top)),
        "from the top of the library"
    );
}

// ---- Lonis, Oloro, Korvold ----

#[test]
fn argent_sphinx_flickers_itself_with_metalcraft() {
    let mut g = table();
    let sphinx = put(&mut g, 0, "Argent Sphinx");
    assert!(
        !g.legal_actions().contains(&Action::Activate(sphinx, 0)),
        "two artifacts aren't enough"
    );
    for _ in 0..2 {
        put(&mut g, 0, "Sol Ring");
    }
    assert!(!g.legal_actions().contains(&Action::Activate(sphinx, 0)));
    put(&mut g, 0, "Sol Ring");
    act(&mut g, Action::Activate(sphinx, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(sphinx).zone, Zone::Exile);
    for _ in 0..30 {
        if g.obj(sphinx).zone == Zone::Battlefield {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(sphinx).zone, Zone::Battlefield, "back at the end step");
}

#[test]
fn mystic_gate_filters_hybrid_into_two() {
    let mut g = game(&["Mystic Gate", "Plains"]);
    let spell = hand(&mut g, 0, "Pollywog Prodigy"); // {1}{U}: the Plains pays {W/U}, the Gate makes two
    assert!(g.legal_actions().contains(&Action::Cast(spell)));
    let mut g = game(&["Mystic Gate"]);
    let spell = hand(&mut g, 0, "Pollywog Prodigy");
    assert!(
        !g.legal_actions().contains(&Action::Cast(spell)),
        "nothing pays the {{W/U}}"
    );
}

#[test]
fn karlov_grows_on_life_gain_and_spends_six_counters_to_exile() {
    let mut g = table();
    let k = put(&mut g, 0, "Karlov of the Ghost Council");
    let bear = put(&mut g, 1, "Llanowar Elves");
    g.objects[k.0 as usize].counters = 4;
    assert!(
        !g.legal_actions().contains(&Action::Activate(k, 0)),
        "four counters"
    );
    g.objects[k.0 as usize].counters = 6;
    act(&mut g, Action::Activate(k, 0));
    act(&mut g, Action::Target(Target::Object(bear)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(bear).zone, Zone::Exile);
    assert_eq!(g.obj(k).counters, 0);
}

#[test]
fn miren_gains_the_sacrificed_creatures_toughness() {
    let mut g = table();
    let miren = put(&mut g, 0, "Miren, the Moaning Well");
    let wall = put(&mut g, 0, "Fyndhorn Elves");
    let life = g.players[0].life;
    act(&mut g, Action::ActivateWith(miren, 0, wall));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, life + g.def_of(wall).toughness);
    assert_eq!(g.obj(wall).zone, Zone::Graveyard);
}

#[test]
fn topiary_lecturer_increments_and_taps_for_its_power() {
    let mut g = table();
    let t = put(&mut g, 0, "Topiary Lecturer");
    g.objects[t.0 as usize].since_turn = 0;
    let p0 = g.power(t);
    let ring = hand(&mut g, 0, "Sol Ring"); // one mana: not more than its power or toughness
    act(&mut g, Action::Cast(ring));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(t), p0);
    let elves = hand(&mut g, 0, "Reclamation Sage"); // three mana: more
    act(&mut g, Action::Cast(elves));
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) | Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.power(t), p0 + 1, "incremented");
    let n = g
        .legal_actions()
        .iter()
        .filter(|a| matches!(a, Action::Activate(c, 0) if *c == t))
        .count();
    assert_eq!(n, 1, "its mana ability");
}

#[test]
fn champion_of_lambholt_keeps_smaller_creatures_from_blocking() {
    let mut g = table();
    let champ = put(&mut g, 0, "Champion of Lambholt");
    g.objects[champ.0 as usize].counters = 2; // 3/3
    let elves = put(&mut g, 0, "Llanowar Elves");
    for c in [champ, elves] {
        g.objects[c.0 as usize].since_turn = 0;
    }
    let small = put(&mut g, 1, "Fyndhorn Elves");
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(elves));
    act(&mut g, Action::FinishAttacks);
    let mut asked = false;
    for _ in 0..30 {
        if matches!(g.pending, Pending::DeclareBlockers(_)) {
            asked = true;
            assert!(
                !g.legal_actions()
                    .iter()
                    .any(|a| matches!(a, Action::Block { blocker, .. } if *blocker == small)),
                "power 1 is less than the Champion's 3"
            );
            break;
        }
        if g.step == Step::Main2 {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    let _ = asked;
    assert!(g.players[1].life < 40, "unblocked");
}

#[test]
fn will_discounts_white_and_blue_spells_by_life_gained() {
    let mut g = game(&["Plains"]);
    let will = put(&mut g, 0, "Will, Scion of Peace");
    g.objects[will.0 as usize].since_turn = 0;
    g.life_gained_this_turn[0] = 2;
    let hawk = hand(&mut g, 0, "Tempest Hawk"); // {2}{W}
    assert!(!g.legal_actions().contains(&Action::Cast(hawk)), "one Plains");
    act(&mut g, Action::Activate(will, 0));
    resolve_all(&mut g, &[]);
    assert!(
        g.legal_actions().contains(&Action::Cast(hawk)),
        "{{2}} less: {{W}}"
    );
}

#[test]
fn high_tide_makes_islands_add_another_blue() {
    let mut g = game(&["Island", "Island"]);
    let tide = hand(&mut g, 0, "High Tide");
    act(&mut g, Action::Cast(tide));
    resolve_all(&mut g, &[]);
    let island = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Island" && !g.obj(x).tapped)
        .unwrap();
    let n = g.mana_sources(0).iter().filter(|&&(x, _)| x == island).count();
    assert_eq!(n, 2, "its {{U}} and High Tide's");
}

#[test]
fn pyromatics_is_copied_for_each_replicate_paid() {
    let mut g = table();
    let p = hand(&mut g, 0, "Pyromatics");
    let life = g.players[1].life;
    act(&mut g, Action::CastX(p, 2)); // replicated twice: three in all
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 3);
}

#[test]
fn rith_s_grove_returns_a_land_or_is_sacrificed() {
    let mut g = game(&["Forest"]);
    let grove = hand(&mut g, 0, "Rith's Grove");
    act(&mut g, Action::PlayLand(grove));
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) | Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(grove).zone, Zone::Battlefield, "the Forest went back");
    assert!(g.permanents(0).all(|x| g.def_of(x).name != "Forest"));
}

#[test]
fn momo_discounts_a_flier_and_grows_when_one_enters() {
    let mut g = game(&["Plains", "Plains"]);
    let momo = put(&mut g, 0, "Momo, Friendly Flier");
    let hawk = hand(&mut g, 0, "Tempest Hawk"); // {2}{W}, flying: {1}{W}
    assert!(g.legal_actions().contains(&Action::Cast(hawk)));
    let p0 = g.power(momo);
    act(&mut g, Action::Cast(hawk));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(momo), p0 + 1);
}

// ---- idea decks, Najeela ----

#[test]
fn path_of_ancestry_scries_when_its_mana_casts_a_creature_of_the_commanders_type() {
    let mut g = game(&[]);
    let path = put(&mut g, 0, "Path of Ancestry");
    let raccoon = hand(&mut g, 0, "Trailtracker Scout"); // {1}{G}, a Raccoon like Bello
    g.add_card(0, cid("Forest"), Zone::Battlefield);
    act(&mut g, Action::Cast(raccoon));
    assert!(g.obj(path).tapped);
    assert!(
        g.items.iter().any(|i| i.source == path),
        "the scry trigger, above the Raccoon"
    );
}

#[test]
fn rain_of_riches_gives_the_first_treasure_spell_cascade() {
    let mut g = game(&[]);
    let rain = put(&mut g, 0, "Rain of Riches");
    let treasure = g
        .defs
        .iter()
        .position(|d| d.token && d.name == "Treasure")
        .expect("Treasure");
    g.add_card(0, treasure, Zone::Battlefield);
    g.add_card(0, treasure, Zone::Battlefield);
    let top = g.add_card(0, cid("Ornithopter"), Zone::Library); // mana value 0
    let elves = hand(&mut g, 0, "Llanowar Elves");
    act(&mut g, Action::Cast(elves));
    assert!(g.items.iter().any(|i| i.source == rain), "cascade");
    while !matches!(g.pending, Pending::FreeCast(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(g.legal_actions().contains(&Action::Cast(top)));
    act(&mut g, Action::PassPriority);
    resolve_all(&mut g, &[]);
    let fyndhorn = hand(&mut g, 0, "Fyndhorn Elves");
    let top2 = g.add_card(0, cid("Ornithopter"), Zone::Library);
    act(&mut g, Action::Cast(fyndhorn));
    while !g.items.is_empty() {
        assert!(
            !matches!(g.pending, Pending::FreeCast(_)),
            "only the first each turn"
        );
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(top2).zone, Zone::Library, "no cascade");
}

#[test]
fn make_disappear_with_casualty_is_copied() {
    let mut g = table();
    let elves = put(&mut g, 0, "Llanowar Elves");
    let tiny = put(&mut g, 0, "Ornithopter"); // power 0: not enough
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    let md = hand(&mut g, 0, "Make Disappear");
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Cast(md)), "casualty is optional");
    assert!(!legal.contains(&Action::CastWith(md, tiny)), "power 1 or greater");
    act(&mut g, Action::CastWith(md, elves));
    act(&mut g, Action::Target(Target::Object(bolt)));
    assert_eq!(g.obj(elves).zone, Zone::Graveyard, "sacrificed as it was cast");
    let before = g.items.len();
    // The casualty trigger resolves into a copy.
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert!(g.items.len() >= before, "a copy on the stack");
    for _ in 0..40 {
        if g.items.is_empty() && !matches!(g.pending, Pending::Decide(_) | Pending::ChooseTarget(_)) {
            break;
        }
        let a = match g.pending {
            Pending::Decide(_) | Pending::ChooseTarget(_) => {
                let l = g.legal_actions();
                if l.contains(&Action::No) {
                    Action::No
                } else {
                    l[0]
                }
            }
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[1].life, 40, "the Bolt was countered");
}

// ---- idea decks: lands, rituals, mana creatures ----

#[test]
fn idea_deck_lands_tap_for_their_colors_and_vernal_fen_checks_basics() {
    let mut g = game(&[]);
    for (name, n) in [
        ("Tropical Island", 2),
        ("Underground Sea", 2),
        ("Volcanic Island", 2),
    ] {
        let l = put(&mut g, 0, name);
        let m = g
            .mana_sources(0)
            .into_iter()
            .find(|&(x, _)| x == l)
            .expect("a source")
            .1;
        assert_eq!(m.colors.iter().filter(|&&b| b).count(), n, "{name}");
    }
    let fen = hand(&mut g, 0, "Vernal Fen");
    act(&mut g, Action::PlayLand(fen));
    resolve_all(&mut g, &[]);
    assert!(g.obj(fen).tapped, "no basic lands");
}

#[test]
fn dark_ritual_and_culling_the_weak_add_black() {
    let mut g = game(&["Swamp"]);
    let ritual = hand(&mut g, 0, "Dark Ritual");
    act(&mut g, Action::Cast(ritual));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].pool.len(), 3);
    let mut g = game(&["Swamp"]);
    let elves = put(&mut g, 0, "Llanowar Elves");
    let cull = hand(&mut g, 0, "Culling the Weak");
    act(&mut g, Action::CastWith(cull, elves));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elves).zone, Zone::Graveyard);
    assert_eq!(g.players[0].pool.len(), 4);
}

#[test]
fn blood_pet_sacrifices_for_black_and_unexpected_assistance_convokes() {
    let mut g = game(&[]);
    let pet = put(&mut g, 0, "Blood Pet");
    let elves = hand(&mut g, 0, "Llanowar Elves"); // can't: {G}
    let ritual = hand(&mut g, 0, "Dark Ritual"); // {B}: the Pet pays
    let _ = elves;
    act(&mut g, Action::Cast(ritual));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(pet).zone, Zone::Graveyard, "sacrificed for its {{B}}");

    let mut g = game(&["Island", "Island"]);
    for _ in 0..4 {
        let c = put(&mut g, 0, "Llanowar Elves");
        g.objects[c.0 as usize].since_turn = 0;
    }
    let ua = hand(&mut g, 0, "Unexpected Assistance");
    assert!(
        g.legal_actions().contains(&Action::Cast(ua)),
        "convoked with four Elves and two Islands"
    );
}

// ---- Ral, Atraxa ----

#[test]
fn emry_has_affinity_mills_and_lets_you_cast_an_artifact_from_the_graveyard() {
    let mut g = game(&["Island"]);
    for _ in 0..2 {
        put(&mut g, 0, "Ornithopter");
    }
    let emry = hand(&mut g, 0, "Emry, Lurker of the Loch"); // {2}{U}, two artifacts: {U}
    let library = g.players[0].library.len();
    act(&mut g, Action::Cast(emry));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(emry).zone, Zone::Battlefield);
    assert_eq!(g.players[0].library.len(), library - 4, "milled four");
    g.objects[emry.0 as usize].since_turn = 0;
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Graveyard);
    act(&mut g, Action::Activate(emry, 0));
    act(&mut g, Action::Target(Target::Object(ring)));
    resolve_all(&mut g, &[]);
    g.add_card(0, cid("Island"), Zone::Battlefield);
    assert!(
        g.legal_actions().contains(&Action::Cast(ring)),
        "castable from the graveyard this turn"
    );
}

#[test]
fn nissa_makes_a_horror_as_big_as_her_loyalty() {
    let mut g = table();
    let nissa = put(&mut g, 0, "Nissa, Ascended Animist");
    g.objects[nissa.0 as usize].loyalty = 7;
    let plus = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a, Action::Loyalty(c, 0) if *c == nissa))
        .expect("+1");
    act(&mut g, plus);
    resolve_all(&mut g, &[]);
    let horrors = tokens(&g, 0, "Phyrexian Horror");
    assert_eq!(horrors.len(), 1);
    assert_eq!(g.power(horrors[0]), 8, "X is her loyalty after +1");
    assert_eq!(g.toughness(horrors[0]), 8);
}

#[test]
fn wash_away_targets_only_spells_not_cast_from_hand_unless_cleaved() {
    // Uncleaved: a spell cast from hand isn't a legal target, so Wash Away can't be cast at all.
    let mut g = table();
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    let wash = hand(&mut g, 0, "Wash Away");
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Cast(wash)), "uncleaved: no legal target");
    assert!(
        legal.contains(&Action::CastAlternative(wash)),
        "cleaved: any spell"
    );
    // Cleaved: the spell cast from hand is a legal target, and it's countered.
    act(&mut g, Action::CastAlternative(wash));
    act(&mut g, Action::Target(Target::Object(bolt)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 40, "cleaved: countered");

    // Uncleaved: a spell cast from the graveyard (flashback) is a legal target.
    let mut g = table();
    let think = g.add_card(0, cid("Think Twice"), Zone::Graveyard);
    act(&mut g, Action::CastFlashback(think));
    let wash = hand(&mut g, 0, "Wash Away");
    assert!(g.legal_actions().contains(&Action::Cast(wash)));
    act(&mut g, Action::Cast(wash));
    act(&mut g, Action::Target(Target::Object(think)));
    let cards = g.players[0].hand.len();
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), cards, "Think Twice countered: no draw");
}

#[test]
fn the_crossroads_lands_sacrifice_and_fetch_a_basic_tapped() {
    let mut g = game(&[]);
    g.add_card(0, cid("Forest"), Zone::Library);
    let life = g.players[0].life;
    let l = hand(&mut g, 0, "Riveteers Overlook");
    act(&mut g, Action::PlayLand(l));
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) | Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(l).zone, Zone::Graveyard);
    let forest = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Forest")
        .expect("a Forest");
    assert!(g.obj(forest).tapped);
    assert_eq!(g.players[0].life, life + 1);
}

#[test]
fn sorcerer_class_creatures_tap_for_instants_and_level_three_burns() {
    let mut g = game(&[]);
    let class = put(&mut g, 0, "Sorcerer Class");
    let elves = put(&mut g, 0, "Llanowar Elves");
    g.objects[elves.0 as usize].since_turn = 0;
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    let fyndhorn = hand(&mut g, 0, "Fyndhorn Elves");
    assert!(!g.legal_actions().contains(&Action::Cast(bolt)), "level 1");
    g.objects[class.0 as usize].class_level = 2;
    let legal = g.legal_actions();
    assert!(
        legal.contains(&Action::Cast(bolt)),
        "the Elves tap for {{R}}: {legal:?}"
    );
    assert!(
        legal.contains(&Action::Cast(fyndhorn)),
        "its own {{G}} still pays a creature"
    );
    g.objects[class.0 as usize].class_level = 3;
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.players[1].life,
        40 - 3 - 1,
        "the Bolt, then one instant or sorcery this turn"
    );
}

// ---- idea decks: blight, Lavaleaper, Lumberjack ----

#[test]
fn blight_or_pay_puts_minus_counters_or_costs_mana() {
    let mut g = table();
    let elves = put(&mut g, 0, "Llanowar Elves");
    let bear = put(&mut g, 1, "Fyndhorn Elves");
    let e = hand(&mut g, 0, "Bogslither's Embrace");
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Cast(e)) && legal.contains(&Action::CastWith(e, elves)));
    act(&mut g, Action::CastWith(e, elves));
    act(&mut g, Action::Target(Target::Object(bear)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(bear).zone, Zone::Exile);
    assert_eq!(
        g.obj(elves).zone,
        Zone::Graveyard,
        "a 1/1 with a -1/-1 counter dies"
    );
}

#[test]
fn lavaleaper_basics_add_one_more_and_everything_has_haste() {
    let mut g = game(&["Forest"]);
    let forest = g.permanents(0).find(|&x| g.def_of(x).name == "Forest").unwrap();
    assert_eq!(g.mana_sources(0).iter().filter(|&&(x, _)| x == forest).count(), 1);
    put(&mut g, 1, "Lavaleaper");
    assert_eq!(
        g.mana_sources(0).iter().filter(|&&(x, _)| x == forest).count(),
        2,
        "any player's basics"
    );
    let elves = put(&mut g, 0, "Llanowar Elves");
    assert!(g.keywords(elves).has(Keywords::HASTE));
}

#[test]
fn orcish_lumberjack_sacrifices_a_forest_for_three() {
    let mut g = game(&["Forest"]);
    let lj = put(&mut g, 0, "Orcish Lumberjack");
    g.objects[lj.0 as usize].since_turn = 0;
    let forest = g.permanents(0).find(|&x| g.def_of(x).name == "Forest").unwrap();
    act(&mut g, Action::ActivateWith(lj, 1, forest)); // {R}{R}{G}
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(forest).zone, Zone::Graveyard);
    assert_eq!(g.players[0].pool.len(), 3);
}

#[test]
fn disruptor_has_flash_and_vinelasher_offers_offspring() {
    let g = table();
    let d = g
        .defs
        .iter()
        .position(|x| x.name == "Disruptor of Currents")
        .unwrap();
    assert!(g.defs[d].keywords.has(Keywords::FLASH) && g.defs[d].mana_ext.convoke);
    let mut g = table();
    let v = hand(&mut g, 0, "Iridescent Vinelasher");
    assert!(
        g.legal_actions().contains(&Action::CastKicked(v)),
        "offspring {{2}}"
    );
    let core = put(&mut g, 0, "Firdoch Core");
    act(&mut g, Action::Activate(core, 0));
    resolve_all(&mut g, &[]);
    assert!(g.types_of(core).creature, "a 4/4 until end of turn");
    assert_eq!(g.power(core), 4);
}

#[test]
fn roaring_earth_channels_from_hand_into_a_spirit_land() {
    let mut g = table();
    let re = hand(&mut g, 0, "Roaring Earth");
    let forest = g.permanents(0).find(|&x| g.def_of(x).name == "Forest").unwrap();
    let channel = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a, Action::ActivateX { id, x: 2, .. } if *id == re))
        .expect("channel from hand with X = 2");
    act(&mut g, channel);
    act(&mut g, Action::Target(Target::Object(forest)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(re).zone, Zone::Graveyard, "discarded");
    assert!(g.types_of(forest).creature && g.types_of(forest).land);
    assert_eq!(g.power(forest), 2);
}

#[test]
fn cabal_ritual_adds_five_with_threshold() {
    for (cards, mana) in [(0, 3), (7, 5)] {
        let mut g = game(&["Swamp", "Swamp"]); // {1}{B}
        for _ in 0..cards {
            g.add_card(0, cid("Plains"), Zone::Graveyard);
        }
        let r = hand(&mut g, 0, "Cabal Ritual");
        act(&mut g, Action::Cast(r));
        resolve_all(&mut g, &[]);
        assert_eq!(g.players[0].pool.len(), mana, "{cards} cards in the graveyard");
    }
}

#[test]
fn crop_rotation_trades_a_land_and_volraths_stronghold_tops_a_creature() {
    let mut g = game(&["Forest", "Swamp", "Swamp"]);
    let forest = g.permanents(0).find(|&x| g.def_of(x).name == "Forest").unwrap();
    let vs = g.add_card(0, cid("Volrath's Stronghold"), Zone::Library);
    let cr = hand(&mut g, 0, "Crop Rotation");
    act(&mut g, Action::CastWith(cr, forest));
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::Decide(_) if g.legal_actions().contains(&Action::Pick(vs)) => Action::Pick(vs),
            Pending::ChooseTarget(_) | Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(forest).zone, Zone::Graveyard);
    assert_eq!(g.obj(vs).zone, Zone::Battlefield);
    let elves = g.add_card(0, cid("Llanowar Elves"), Zone::Graveyard);
    act(&mut g, Action::Activate(vs, 0));
    act(&mut g, Action::Target(Target::Object(elves)));
    resolve_all(&mut g, &[]);
    assert_eq!(*g.players[0].library.last().unwrap(), elves);
}

/// Answers every decision with its first option until the stack is empty.
fn settle(g: &mut Game) {
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) | Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn snuff_out_pays_4_life_with_a_swamp_and_kills_a_nonblack_creature() {
    let mut g = table();
    let elf = put(&mut g, 1, "Llanowar Elves");
    let snuff = hand(&mut g, 0, "Snuff Out");
    act(&mut g, Action::CastAlternative(snuff));
    act(&mut g, Action::Target(Target::Object(elf)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 36);
    assert_eq!(g.obj(elf).zone, Zone::Graveyard);

    let mut g = game(&["Forest", "Forest", "Forest", "Forest"]);
    put(&mut g, 1, "Llanowar Elves");
    let snuff = hand(&mut g, 0, "Snuff Out");
    assert!(
        !g.legal_actions().contains(&Action::CastAlternative(snuff)),
        "no Swamp"
    );
}

#[test]
fn spymasters_vault_enters_tapped_without_a_swamp_and_connives_per_death() {
    let mut g = game(&["Forest"]);
    let v = hand(&mut g, 0, "Spymaster's Vault");
    act(&mut g, Action::PlayLand(v));
    assert!(g.obj(v).tapped, "no Swamp: tapped");

    let mut g = table();
    let elf = put(&mut g, 0, "Llanowar Elves");
    let v = hand(&mut g, 0, "Spymaster's Vault");
    act(&mut g, Action::PlayLand(v));
    assert!(!g.obj(v).tapped, "a Swamp: untapped");
    g.creatures_died = (g.turn, 2);
    let cards = g.players[0].hand.len();
    let yard = g.players[0].graveyard.len();
    act(&mut g, Action::Activate(v, 0));
    act(&mut g, Action::Target(Target::Object(elf)));
    settle(&mut g);
    assert_eq!(g.players[0].hand.len(), cards, "drew 2, discarded 2");
    assert_eq!(g.players[0].graveyard.len(), yard + 2);
}

#[test]
fn the_black_gate_asks_3_life_and_makes_a_creature_unblockable_at_tied_life() {
    let mut g = table();
    let elf = put(&mut g, 0, "Llanowar Elves");
    let gate = hand(&mut g, 0, "The Black Gate");
    act(&mut g, Action::PlayLand(gate));
    assert!(matches!(g.pending, Pending::Decide(_)));
    act(&mut g, Action::Yes);
    assert_eq!(g.players[0].life, 37);
    assert!(!g.obj(gate).tapped);
    g.players[1].life = 37; // tied for the most life
    act(&mut g, Action::Activate(gate, 0));
    act(&mut g, Action::Target(Target::Object(elf)));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(elf).has(Keywords::UNBLOCKABLE));

    let mut g = table();
    let elf = put(&mut g, 0, "Llanowar Elves");
    let gate = hand(&mut g, 0, "The Black Gate");
    act(&mut g, Action::PlayLand(gate));
    act(&mut g, Action::No);
    assert!(g.obj(gate).tapped, "declined: tapped");
    g.objects[gate.0 as usize].tapped = false;
    g.players[1].life = 20; // you have the most life: your own creatures are "that player's"
    act(&mut g, Action::Activate(gate, 0));
    act(&mut g, Action::Target(Target::Object(elf)));
    resolve_all(&mut g, &[]);
    assert!(!g.keywords(elf).has(Keywords::UNBLOCKABLE));
}

#[test]
fn escape_tunnel_fetches_a_basic_or_makes_a_small_creature_unblockable() {
    let mut g = table();
    let elf = put(&mut g, 0, "Llanowar Elves");
    let tunnel = put(&mut g, 0, "Escape Tunnel");
    act(&mut g, Action::Activate(tunnel, 1));
    act(&mut g, Action::Target(Target::Object(elf)));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(elf).has(Keywords::UNBLOCKABLE));
    assert_eq!(g.obj(tunnel).zone, Zone::Graveyard);

    let mut g = table();
    let tunnel = put(&mut g, 0, "Escape Tunnel");
    let lands = g.permanents(0).filter(|&x| g.types_of(x).land).count();
    act(&mut g, Action::Activate(tunnel, 0));
    settle(&mut g);
    assert_eq!(g.obj(tunnel).zone, Zone::Graveyard);
    assert_eq!(
        g.permanents(0).filter(|&x| g.types_of(x).land).count(),
        lands,
        "one for one"
    );
    assert!(
        g.permanents(0)
            .filter(|&x| g.types_of(x).land)
            .any(|x| g.obj(x).tapped),
        "fetched tapped"
    );
}

#[test]
fn invigorate_with_a_forest_gives_the_opponent_3_life_instead_of_mana() {
    let mut g = game(&["Forest"]);
    let elf = put(&mut g, 0, "Llanowar Elves");
    let inv = hand(&mut g, 0, "Invigorate");
    act(&mut g, Action::CastAlternative(inv));
    act(&mut g, Action::Target(Target::Object(elf)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 43);
    assert_eq!(g.power(elf), 5);

    let mut g = game(&["Island"]);
    let inv = hand(&mut g, 0, "Invigorate");
    put(&mut g, 0, "Llanowar Elves");
    assert!(
        !g.legal_actions().contains(&Action::CastAlternative(inv)),
        "no Forest"
    );
}

#[test]
fn pyrrhic_strike_chooses_one_or_both_when_blighted() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let witness = put(&mut g, 1, "Eternal Witness");
    let mine = put(&mut g, 0, "Eternal Witness");
    let strike = hand(&mut g, 0, "Pyrrhic Strike");
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::CastModal(strike, 0)));
    assert!(legal.contains(&Action::CastModal(strike, 1)));
    assert!(
        !legal.contains(&Action::CastModal(strike, 2)),
        "both only when blighted"
    );
    act(&mut g, Action::CastWith(strike, mine));
    act(&mut g, Action::Target(Target::Object(ring)));
    act(&mut g, Action::Target(Target::Object(witness)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    assert_eq!(g.obj(witness).zone, Zone::Graveyard);
    assert_eq!(g.obj(mine).zone, Zone::Graveyard, "blighted 2: a 2/1 dies");

    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let strike = hand(&mut g, 0, "Pyrrhic Strike");
    act(&mut g, Action::CastModal(strike, 0));
    act(&mut g, Action::Target(Target::Object(ring)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
}

#[test]
fn tifas_limit_break_pumps_doubles_or_triples_by_tier() {
    for (mode, power) in [(0u8, 3), (1, 2), (2, 3)] {
        let mut g = table();
        let elf = put(&mut g, 0, "Llanowar Elves");
        let tifa = hand(&mut g, 0, "Tifa's Limit Break");
        act(&mut g, Action::CastModal(tifa, mode));
        act(&mut g, Action::Target(Target::Object(elf)));
        resolve_all(&mut g, &[]);
        assert_eq!(g.power(elf), power, "mode {mode}");
        if mode == 2 {
            assert_eq!(g.toughness(elf), 3);
        }
    }
    // Final Heaven costs {6}{G} more: {G}{6}{G} is 8 mana.
    let mut g = game(&[
        "Forest", "Forest", "Forest", "Forest", "Forest", "Forest", "Forest",
    ]);
    put(&mut g, 1, "Llanowar Elves"); // the target, not a mana source of ours
    let tifa = hand(&mut g, 0, "Tifa's Limit Break");
    assert!(g.legal_actions().contains(&Action::CastModal(tifa, 1)));
    assert!(
        !g.legal_actions().contains(&Action::CastModal(tifa, 2)),
        "7 lands: no Final Heaven"
    );
}

/// The mana in player 0's pool.
fn pool_len(g: &Game) -> usize {
    g.players[0].pool.len()
}

#[test]
fn chrome_mox_imprints_a_card_and_taps_for_its_colors() {
    let mut g = game(&[]);
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    let mox = hand(&mut g, 0, "Chrome Mox");
    act(&mut g, Action::Cast(mox));
    resolve_all(&mut g, &[Action::Pick(bolt)]);
    assert_eq!(g.obj(bolt).zone, Zone::Exile, "imprinted");
    let shock = hand(&mut g, 0, "Lightning Bolt");
    assert!(
        g.legal_actions().contains(&Action::Cast(shock)),
        "red mana from the Mox"
    );
    let giant = hand(&mut g, 0, "Giant Growth");
    assert!(!g.legal_actions().contains(&Action::Cast(giant)), "no green");

    let mut g = game(&[]);
    let mox = hand(&mut g, 0, "Chrome Mox");
    hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(mox));
    resolve_all(&mut g, &[Action::No]);
    let shock = hand(&mut g, 0, "Lightning Bolt");
    assert!(
        !g.legal_actions().contains(&Action::Cast(shock)),
        "nothing imprinted: no mana"
    );
}

#[test]
fn mox_diamond_stays_only_if_you_discard_a_land() {
    // Reckless Fireweaver: "Whenever an artifact you control enters, ... 1 damage to each opponent."
    let mut g = game(&[]);
    put(&mut g, 0, "Reckless Fireweaver");
    let forest = hand(&mut g, 0, "Forest");
    let mox = hand(&mut g, 0, "Mox Diamond");
    act(&mut g, Action::Cast(mox));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert!(matches!(g.pending, Pending::Decide(_)));
    assert_eq!(g.obj(mox).zone, Zone::Stack, "asked before it enters");
    resolve_all(&mut g, &[Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Graveyard, "discarded");
    assert_eq!(g.obj(mox).zone, Zone::Battlefield);
    assert_eq!(g.players[1].life, 39, "it entered: the trigger fired");

    let mut g = game(&[]);
    put(&mut g, 0, "Reckless Fireweaver");
    hand(&mut g, 0, "Forest");
    let mox = hand(&mut g, 0, "Mox Diamond");
    act(&mut g, Action::Cast(mox));
    resolve_all(&mut g, &[Action::No]);
    assert_eq!(g.obj(mox).zone, Zone::Graveyard, "no land discarded");
    assert_eq!(g.players[1].life, 40, "it never entered: no trigger");
}

#[test]
fn elvish_spirit_guide_exiles_from_hand_for_green() {
    let mut g = game(&[]);
    let esg = hand(&mut g, 0, "Elvish Spirit Guide");
    act(&mut g, Action::Activate(esg, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(esg).zone, Zone::Exile);
    assert_eq!(pool_len(&g), 1);
    let giant = hand(&mut g, 0, "Giant Growth");
    put(&mut g, 0, "Llanowar Elves");
    assert!(
        g.legal_actions().contains(&Action::Cast(giant)),
        "{{G}} in the pool"
    );
}

#[test]
fn lions_eye_diamond_discards_the_hand_for_three_of_one_color() {
    let mut g = game(&[]);
    let led = put(&mut g, 0, "Lion's Eye Diamond");
    let cards = g.players[0].hand.len();
    assert!(cards > 0);
    let yard = g.players[0].graveyard.len();
    act(&mut g, Action::Activate(led, 0));
    resolve_all(&mut g, &[Action::Mode(3)]);
    assert!(g.players[0].hand.is_empty(), "hand discarded");
    assert_eq!(
        g.players[0].graveyard.len(),
        yard + cards + 1,
        "the hand and the Diamond"
    );
    assert_eq!(pool_len(&g), 3);
    assert!(g.players[0]
        .pool
        .iter()
        .all(|m| m.colors == [false, false, false, true, false]));
}

#[test]
fn instill_energy_lets_a_new_creature_attack_and_untaps_it_once() {
    let mut g = table();
    let elf = put(&mut g, 0, "Llanowar Elves");
    let ie = hand(&mut g, 0, "Instill Energy");
    act(&mut g, Action::Cast(ie));
    act(&mut g, Action::Target(Target::Object(elf)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ie).attached_to, Some((elf, g.obj(elf).zcc)));
    g.objects[elf.0 as usize].tapped = true;
    act(&mut g, Action::Activate(ie, 0));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(elf).tapped, "untapped");
    assert!(
        !g.legal_actions().contains(&Action::Activate(ie, 0)),
        "once each turn"
    );
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        g.legal_actions().contains(&Action::Attack(elf)),
        "attacks as though it had haste"
    );
}

#[test]
fn westvale_abbey_makes_clerics_and_flips_on_five_picked_sacrifices() {
    let mut g = table();
    let abbey = put(&mut g, 0, "Westvale Abbey // Ormendahl, Profane Prince");
    act(&mut g, Action::Activate(abbey, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 39);
    assert_eq!(
        g.creatures(0)
            .filter(|&c| g.def_of(c).name == "Human Cleric")
            .count(),
        1
    );

    let mut g = table();
    let abbey = put(&mut g, 0, "Westvale Abbey // Ormendahl, Profane Prince");
    let elves: Vec<ObjId> = (0..6).map(|_| put(&mut g, 0, "Llanowar Elves")).collect();
    act(&mut g, Action::Activate(abbey, 1));
    assert!(matches!(g.pending, Pending::Decide(_)));
    for &e in &elves[..5] {
        act(&mut g, Action::Pick(e));
    }
    resolve_all(&mut g, &[]);
    assert!(
        elves[..5].iter().all(|&e| g.obj(e).zone == Zone::Graveyard),
        "the five picked"
    );
    assert_eq!(g.obj(elves[5]).zone, Zone::Battlefield, "the sixth stays");
    assert!(g.obj(abbey).transformed && !g.obj(abbey).tapped);
    assert_eq!(g.power(abbey), 9);

    let mut g = table();
    let abbey = put(&mut g, 0, "Westvale Abbey // Ormendahl, Profane Prince");
    for _ in 0..4 {
        put(&mut g, 0, "Llanowar Elves");
    }
    assert!(
        !g.legal_actions().contains(&Action::Activate(abbey, 1)),
        "four creatures: can't"
    );
}

#[test]
fn tekuthal_removes_three_picked_counters_for_an_indestructible_counter() {
    let mut g = table();
    let tek = put(&mut g, 0, "Tekuthal, Inquiry Dominus");
    let a = put(&mut g, 0, "Llanowar Elves");
    g.objects[a.0 as usize].counters = 3;
    act(&mut g, Action::Activate(tek, 0));
    for _ in 0..3 {
        act(&mut g, Action::Pick(a));
    }
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(a).counters, 0);
    assert!(g.keywords(tek).has(Keywords::INDESTRUCTIBLE));
    assert!(g.def_of(tek).proliferate_twice);
}

#[test]
fn deathrite_shaman_exiles_graveyard_cards_by_type() {
    let mut g = table();
    let ds = put(&mut g, 0, "Deathrite Shaman");
    g.objects[ds.0 as usize].since_turn = 0;
    let bolt = g.add_card(1, cid("Lightning Bolt"), Zone::Graveyard);
    let forest = g.add_card(1, cid("Forest"), Zone::Graveyard);
    let elf = g.add_card(1, cid("Llanowar Elves"), Zone::Graveyard);
    act(&mut g, Action::Activate(ds, 0));
    assert!(
        !g.legal_actions().contains(&Action::Target(Target::Object(bolt))),
        "a land card only"
    );
    act(&mut g, Action::Target(Target::Object(forest)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(forest).zone, Zone::Exile);
    assert_eq!(g.players[0].pool.len(), 1, "one mana of any color");

    let mut g = table();
    let ds = put(&mut g, 0, "Deathrite Shaman");
    g.objects[ds.0 as usize].since_turn = 0;
    let bolt = g.add_card(1, cid("Lightning Bolt"), Zone::Graveyard);
    act(&mut g, Action::Activate(ds, 1));
    act(&mut g, Action::Target(Target::Object(bolt)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(bolt).zone, Zone::Exile);
    assert_eq!(g.players[1].life, 38);
    let _ = elf;
}

#[test]
fn braid_of_fire_adds_red_per_age_counter_or_is_sacrificed() {
    let mut g = table();
    let braid = put(&mut g, 0, "Braid of Fire");
    // Through the opponent's turn to player 0's next upkeep.
    while !matches!(g.pending, Pending::Decide(_)) {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ if !g.legal_actions().contains(&Action::PassPriority) => g.legal_actions()[0], // cleanup discard
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!((g.active, g.step), (0, Step::Upkeep));
    act(&mut g, Action::Yes);
    assert_eq!(g.players[0].pool.len(), 1);
    assert!(g.players[0]
        .pool
        .iter()
        .all(|m| m.colors == [false, false, false, true, false]));
    assert_eq!(g.obj(braid).zone, Zone::Battlefield);
}

#[test]
fn glen_elendra_guardian_enters_shrunk_and_removes_its_counter_to_counter_a_spell() {
    let mut g = table();
    let gg = hand(&mut g, 0, "Glen Elendra Guardian");
    act(&mut g, Action::Cast(gg));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(gg), g.toughness(gg)), (2, 3), "a -1/-1 counter");
    let elf = hand(&mut g, 0, "Llanowar Elves");
    act(&mut g, Action::Cast(elf));
    assert!(
        !g.legal_actions().contains(&Action::Activate(gg, 0)),
        "a creature spell isn't a target"
    );
    resolve_all(&mut g, &[]);
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    let cards = g.players[0].hand.len();
    act(&mut g, Action::Activate(gg, 0));
    act(&mut g, Action::Target(Target::Object(bolt)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(bolt).zone, Zone::Graveyard, "countered");
    assert_eq!(g.players[1].life, 40);
    assert_eq!(g.players[0].hand.len(), cards + 1, "its controller draws");
    assert_eq!((g.power(gg), g.toughness(gg)), (3, 4), "the counter is gone");
    assert!(
        !g.legal_actions().contains(&Action::Activate(gg, 0)),
        "no counter left"
    );
}

#[test]
fn comet_storm_hits_one_target_plus_one_per_kick() {
    let mut g = table();
    let elf = put(&mut g, 1, "Llanowar Elves");
    let storm = hand(&mut g, 0, "Comet Storm");
    act(&mut g, Action::CastX(storm, 2));
    assert!(matches!(g.pending, Pending::Decide(_)), "the kick count");
    act(&mut g, Action::Amount(1));
    act(&mut g, Action::Target(Target::Player(1)));
    assert!(
        !g.legal_actions().contains(&Action::No),
        "kicked once: a second target"
    );
    act(&mut g, Action::Target(Target::Object(elf)));
    assert!(
        !g.legal_actions().iter().any(|a| matches!(a, Action::Target(_))),
        "two targets, no more"
    );
    settle(&mut g);
    assert_eq!(g.players[1].life, 38);
    assert_eq!(g.obj(elf).zone, Zone::Graveyard);
}

#[test]
fn changeling_wayfinder_fetches_a_basic_to_hand() {
    let mut g = table();
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    let w = hand(&mut g, 0, "Changeling Wayfinder");
    act(&mut g, Action::Cast(w));
    settle(&mut g);
    assert!(g.has_subtype(w, "Elf"), "changeling");
    assert!(
        g.players[0].hand.iter().any(|&c| g.types_of(c).basic),
        "a basic in hand"
    );
    let _ = forest;
}

#[test]
fn reaping_willow_spends_its_two_counters_to_reanimate_a_small_creature() {
    let mut g = table();
    let willow = hand(&mut g, 0, "Reaping Willow");
    act(&mut g, Action::Cast(willow));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.power(willow), g.toughness(willow)),
        (1, 4),
        "two -1/-1 counters"
    );
    let elf = g.add_card(0, cid("Llanowar Elves"), Zone::Graveyard);
    act(&mut g, Action::Activate(willow, 0));
    act(&mut g, Action::Target(Target::Object(elf)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elf).zone, Zone::Battlefield);
    assert_eq!(
        (g.power(willow), g.toughness(willow)),
        (3, 6),
        "both counters removed"
    );
    assert!(
        !g.legal_actions().contains(&Action::Activate(willow, 0)),
        "no counters left"
    );
}

#[test]
fn flitterwing_nuisance_draws_for_each_creature_that_connects_this_turn() {
    let mut g = table();
    let fw = hand(&mut g, 0, "Flitterwing Nuisance");
    act(&mut g, Action::Cast(fw));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(fw), 1, "a -1/-1 counter");
    let elves = [put(&mut g, 0, "Llanowar Elves"), put(&mut g, 0, "Llanowar Elves")];
    for &e in &elves {
        g.objects[e.0 as usize].since_turn = 0;
    }
    act(&mut g, Action::Activate(fw, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(fw), 2, "the counter is gone");
    let cards = g.players[0].hand.len();
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    for &e in &elves {
        act(&mut g, Action::Attack(e));
    }
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::ChooseTarget(_) | Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(
        g.players[0].hand.len(),
        cards + 2,
        "one draw per creature that connected"
    );
}

#[test]
fn forbidden_orchard_gives_the_opponent_a_spirit_when_tapped() {
    let mut g = game(&[]);
    let orchard = put(&mut g, 0, "Forbidden Orchard");
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    settle(&mut g);
    assert!(g.obj(orchard).tapped);
    assert_eq!(g.players[1].life, 37);
    assert_eq!(
        g.creatures(1).filter(|&c| g.def_of(c).name == "Spirit").count(),
        1
    );
}

#[test]
fn three_tree_city_names_a_type_as_it_enters_and_taps_for_one_color_per_creature() {
    let mut g = table();
    for _ in 0..3 {
        put(&mut g, 0, "Llanowar Elves");
    }
    let city = hand(&mut g, 0, "Three Tree City");
    act(&mut g, Action::PlayLand(city));
    let elf = match &g.decision {
        Some(bolt_core::Decision::ChooseCreatureType { options, .. }) => {
            options.iter().position(|o| o == "Elf").expect("Elf is an option")
        }
        d => panic!("expected the creature type choice, got {d:?}"),
    };
    act(&mut g, Action::Mode(elf as u8));
    assert_eq!(g.obj(city).chosen_type.as_deref(), Some("Elf"));
    act(&mut g, Action::Activate(city, 0));
    resolve_all(&mut g, &[Action::Mode(4)]);
    assert_eq!(g.players[0].pool.len(), 3, "three Elves: three green");
    assert!(g.players[0]
        .pool
        .iter()
        .all(|m| m.colors == [false, false, false, false, true]));
}

#[test]
fn flare_of_malice_sacrifices_a_picked_nontoken_black_creature_instead_of_mana() {
    let mut g = game(&[]);
    let mine = put(&mut g, 0, "Deathrite Shaman"); // black (and green), nontoken
    let small = put(&mut g, 1, "Llanowar Elves");
    let big = put(&mut g, 1, "Eternal Witness");
    let flare = hand(&mut g, 0, "Flare of Malice");
    assert!(!g.legal_actions().contains(&Action::Cast(flare)), "no mana");
    act(&mut g, Action::CastWith(flare, mine));
    settle(&mut g);
    assert_eq!(g.obj(mine).zone, Zone::Graveyard, "sacrificed for the cost");
    assert_eq!(g.obj(big).zone, Zone::Graveyard, "the greatest mana value");
    assert_eq!(g.obj(small).zone, Zone::Battlefield);
}

/// Passes to player 0's next turn until a decision comes up.
fn to_next_decision(g: &mut Game) {
    while !matches!(g.pending, Pending::Decide(_)) {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ if !g.legal_actions().contains(&Action::PassPriority) => g.legal_actions()[0], // cleanup discard
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn scuzzback_scrounger_may_blight_one_for_a_treasure_each_first_main_phase() {
    let mut g = table();
    let scuzz = put(&mut g, 0, "Scuzzback Scrounger");
    let elf = put(&mut g, 0, "Llanowar Elves");
    to_next_decision(&mut g);
    assert_eq!((g.active, g.step), (0, Step::Main1));
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(scuzz)) && legal.contains(&Action::Pick(elf)));
    assert!(legal.contains(&Action::No));
    act(&mut g, Action::Pick(scuzz));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(scuzz), g.toughness(scuzz)), (2, 1), "a -1/-1 counter");
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1);
    // Next time, decline: no Treasure.
    to_next_decision(&mut g);
    act(&mut g, Action::No);
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1);
    assert_eq!(g.obj(elf).zone, Zone::Battlefield);
}

#[test]
fn mina_and_denn_returns_a_land_to_give_trample_and_allows_an_extra_land() {
    let mut g = table();
    let md = put(&mut g, 0, "Mina and Denn, Wildborn");
    let elf = put(&mut g, 0, "Llanowar Elves");
    let land = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Plains")
        .expect("a Plains");
    act(&mut g, Action::ActivateWith(md, 0, land));
    act(&mut g, Action::Target(Target::Object(elf)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(land).zone, Zone::Hand, "returned for the cost");
    assert!(g.keywords(elf).has(Keywords::TRAMPLE));
    // Two land plays this turn.
    let a = hand(&mut g, 0, "Forest");
    let b = hand(&mut g, 0, "Mountain");
    act(&mut g, Action::PlayLand(a));
    assert!(g.legal_actions().contains(&Action::PlayLand(b)));
}

#[test]
fn wild_growth_adds_an_additional_green() {
    let mut g = table();
    let island = first_named(&g, 0, "Island");
    enchant_land(&mut g, "Wild Growth", island);
    let made: Vec<_> = g.mana_sources(0).into_iter().filter(|s| s.0 == island).collect();
    assert_eq!(made.len(), 2);
    assert!(made
        .iter()
        .any(|s| s.1.colors == [false, false, false, false, true]));
}

#[test]
fn utopia_sprawl_enchants_a_forest_and_adds_the_chosen_color() {
    let mut g = table();
    let island = first_named(&g, 0, "Island");
    let forest = first_named(&g, 0, "Forest");
    let c = hand(&mut g, 0, "Utopia Sprawl");
    act(&mut g, Action::Cast(c));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(island))),
        "only a Forest"
    );
    act(&mut g, Action::Target(obj(forest)));
    resolve_all(&mut g, &[Action::Mode(2)]); // black
    assert_eq!(g.obj(c).attached_to.map(|a| a.0), Some(forest));
    g.objects[forest.0 as usize].tapped = false;
    let made: Vec<_> = g.mana_sources(0).into_iter().filter(|s| s.0 == forest).collect();
    assert_eq!(made.len(), 2);
    assert!(made
        .iter()
        .any(|s| s.1.colors == [false, false, true, false, false]));
}

#[test]
fn shimmerwilds_growth_adds_the_chosen_color() {
    let mut g = table();
    let plains = first_named(&g, 0, "Plains");
    let c = hand(&mut g, 0, "Shimmerwilds Growth");
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(plains)));
    resolve_all(&mut g, &[Action::Mode(3)]); // red
    g.objects[plains.0 as usize].tapped = false;
    let made: Vec<_> = g.mana_sources(0).into_iter().filter(|s| s.0 == plains).collect();
    assert!(made
        .iter()
        .any(|s| s.1.colors == [false, false, false, true, false]));
}

#[test]
fn concordant_crossroads_gives_every_creature_haste() {
    let mut g = table();
    put(&mut g, 0, "Concordant Crossroads");
    let mine = put(&mut g, 0, "Llanowar Elves");
    let theirs = put(&mut g, 1, "Llanowar Elves");
    assert!(g.keywords(mine).has(Keywords::HASTE));
    assert!(g.keywords(theirs).has(Keywords::HASTE));
}

#[test]
fn quirion_ranger_returns_a_forest_to_untap_a_creature_once_each_turn() {
    let mut g = table();
    let ranger = put(&mut g, 0, "Quirion Ranger");
    let elf = put(&mut g, 0, "Llanowar Elves");
    g.objects[elf.0 as usize].tapped = true;
    let island = first_named(&g, 0, "Island");
    let forest = first_named(&g, 0, "Forest");
    assert!(
        !g.legal_actions()
            .contains(&Action::ActivateWith(ranger, 0, island)),
        "a Forest only"
    );
    act(&mut g, Action::ActivateWith(ranger, 0, forest));
    act(&mut g, Action::Target(obj(elf)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(forest).zone, Zone::Hand);
    assert!(!g.obj(elf).tapped);
    let other = first_named(&g, 0, "Forest");
    assert!(
        !g.legal_actions()
            .contains(&Action::ActivateWith(ranger, 0, other)),
        "once each turn"
    );
}

#[test]
fn wirewood_symbiote_returns_an_elf_and_scryb_ranger_has_protection_from_blue() {
    let mut g = table();
    let sym = put(&mut g, 0, "Wirewood Symbiote");
    let elf = put(&mut g, 0, "Llanowar Elves");
    let scryb = put(&mut g, 0, "Scryb Ranger");
    g.objects[scryb.0 as usize].tapped = true;
    act(&mut g, Action::ActivateWith(sym, 0, elf));
    act(&mut g, Action::Target(obj(scryb)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elf).zone, Zone::Hand);
    assert!(!g.obj(scryb).tapped);
    assert!(g.keywords(scryb).has(Keywords::FLYING));
    assert!(g.keywords(scryb).has(Keywords::FLASH));
}

#[test]
fn wolfwillow_haven_adds_green_and_sacrifices_for_a_wolf_on_your_turn() {
    let mut g = table();
    let island = first_named(&g, 0, "Island");
    let haven = enchant_land(&mut g, "Wolfwillow Haven", island);
    let made: Vec<_> = g.mana_sources(0).into_iter().filter(|s| s.0 == island).collect();
    assert!(made
        .iter()
        .any(|s| s.1.colors == [false, false, false, false, true]));
    act(&mut g, Action::Activate(haven, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(haven).zone, Zone::Graveyard);
    let w = tokens(&g, 0, "Wolf");
    assert_eq!(w.len(), 1);
    assert_eq!((g.power(w[0]), g.toughness(w[0])), (2, 2));
    assert!(g.colors_of(w[0])[4], "green");
}

#[test]
fn earthcraft_taps_the_named_creature_to_untap_a_basic_land() {
    let mut g = table();
    let ec = put(&mut g, 0, "Earthcraft");
    let elf = put(&mut g, 0, "Llanowar Elves"); // summoning sick: still fine
    let bear = put(&mut g, 0, "Eternal Witness");
    let forest = first_named(&g, 0, "Forest");
    g.objects[forest.0 as usize].tapped = true;
    let legal = g.legal_actions();
    assert!(
        legal.contains(&Action::ActivateWith(ec, 0, elf))
            && legal.contains(&Action::ActivateWith(ec, 0, bear))
    );
    act(&mut g, Action::ActivateWith(ec, 0, elf));
    act(&mut g, Action::Target(obj(forest)));
    resolve_all(&mut g, &[]);
    assert!(g.obj(elf).tapped, "the creature the player named");
    assert!(!g.obj(bear).tapped);
    assert!(!g.obj(forest).tapped);
}

#[test]
fn earthcraft_targets_only_basic_lands() {
    let mut g = table();
    let ec = put(&mut g, 0, "Earthcraft");
    let elf = put(&mut g, 0, "Llanowar Elves");
    let castle = put(&mut g, 0, "Castle Garenbrig");
    act(&mut g, Action::ActivateWith(ec, 0, elf));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(castle))),
        "not basic"
    );
}

#[test]
fn castle_garenbrig_makes_six_green_for_creature_spells_only() {
    let mut g = table();
    let castle = put(&mut g, 0, "Castle Garenbrig");
    assert!(!g.obj(castle).tapped, "a Forest is in play");
    act(&mut g, Action::Activate(castle, 0));
    resolve_all(&mut g, &[]);
    assert!(g.obj(castle).tapped);
    assert_eq!(g.creature_ability_color_mana[0].0[4], 6);
    // Tap out the rest, so only the six {G} are left.
    for l in g.permanents(0).collect::<Vec<_>>() {
        if g.types_of(l).land {
            g.objects[l.0 as usize].tapped = true;
        }
    }
    let nw = hand(&mut g, 0, "Night's Whisper");
    assert!(
        !g.legal_actions().contains(&Action::Cast(nw)),
        "not for a noncreature spell"
    );
    let elves = hand(&mut g, 0, "Llanowar Elves");
    act(&mut g, Action::Cast(elves));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
    assert_eq!(g.creature_ability_color_mana[0].0[4], 5);
}

#[test]
fn gravecrawler_casts_from_the_graveyard_only_while_you_control_a_zombie() {
    let mut g = table();
    let gc = g.add_card(0, cid("Gravecrawler"), Zone::Graveyard);
    assert!(!g.legal_actions().contains(&Action::Cast(gc)), "no Zombie");
    let other = put(&mut g, 0, "Gravecrawler");
    assert!(g.legal_actions().contains(&Action::Cast(gc)), "a Zombie");
    act(&mut g, Action::Cast(gc));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(gc).zone, Zone::Battlefield);
    assert!(g.def_of(other).cant_block);
}

#[test]
fn timeline_culler_warps_from_the_graveyard_for_b_and_two_life() {
    let mut g = table();
    let tc = g.add_card(0, cid("Timeline Culler"), Zone::Graveyard);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::CastAlternative(tc)), "its warp");
    assert!(
        !legal.contains(&Action::Cast(tc)),
        "not its mana cost from the graveyard"
    );
    let life = g.players[0].life;
    act(&mut g, Action::CastAlternative(tc));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(tc).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, life - 2);
    assert!(g.obj(tc).warped, "exiled at the next end step");
    while g.step != Step::End {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(tc).zone, Zone::Exile);
    // With 1 life, the warp can't be paid.
    let mut g = table();
    let tc = g.add_card(0, cid("Timeline Culler"), Zone::Graveyard);
    g.players[0].life = 1;
    assert!(!g.legal_actions().contains(&Action::CastAlternative(tc)));
    // From the hand: either cost.
    let h = hand(&mut g, 0, "Timeline Culler");
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::CastAlternative(h)), "1 life");
    assert!(legal.contains(&Action::Cast(h)));
}

/// Through the opponent's turn to player 0's next upkeep decision.
fn to_my_upkeep_decision(g: &mut Game) {
    while !matches!(g.pending, Pending::Decide(_)) {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ if !g.legal_actions().contains(&Action::PassPriority) => g.legal_actions()[0], // cleanup discard
            _ => Action::PassPriority,
        };
        act(g, a);
    }
    assert_eq!((g.active, g.step), (0, Step::Upkeep));
}

#[test]
fn herald_of_leshrac_takes_a_land_per_age_counter_and_gives_them_back_when_it_leaves() {
    let mut g = table();
    let h = put(&mut g, 0, "Herald of Leshrac");
    assert!(g.keywords(h).has(Keywords::FLYING));
    assert_eq!((g.power(h), g.toughness(h)), (2, 4));
    to_my_upkeep_decision(&mut g);
    let theirs: Vec<ObjId> = g.permanents(1).filter(|&x| g.types_of(x).land).collect();
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(theirs[0])) && legal.contains(&Action::No));
    let mine = g.permanents(0).find(|&x| g.types_of(x).land).expect("a land");
    assert!(
        !legal.contains(&Action::Pick(mine)),
        "only a land you don't control"
    );
    act(&mut g, Action::Pick(theirs[0]));
    assert_eq!(g.obj(theirs[0]).controller, 0, "for good");
    assert_eq!(
        (g.power(h), g.toughness(h)),
        (3, 5),
        "a land you control but don't own"
    );
    // It leaves: each land goes back to its owner.
    g.move_card(h, Zone::Graveyard);
    act(&mut g, Action::PassPriority); // the trigger goes on the stack
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs[0]).controller, 1);
}

#[test]
fn herald_of_leshrac_is_sacrificed_when_not_paid() {
    let mut g = table();
    let h = put(&mut g, 0, "Herald of Leshrac");
    to_my_upkeep_decision(&mut g);
    act(&mut g, Action::No);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(h).zone, Zone::Graveyard);
}

#[test]
fn gemstone_caverns_taps_for_colorless_or_any_color_with_a_luck_counter() {
    let mut g = table();
    let gc = put(&mut g, 0, "Gemstone Caverns");
    assert!(g.def_of(gc).begins_game_with_luck_counter);
    assert_eq!(makes(&g, gc), ([false; 5], true), "{{C}}");
    g.objects[gc.0 as usize].kind_counters = 1; // a luck counter
    assert_eq!(makes(&g, gc), ([true; 5], false), "any color instead");
}

const RAL: &str = "Ral, Monsoon Mage // Ral, Leyline Prodigy";

/// Taps every land of player 0's but `keep` Swamps.
fn tap_all_but_swamps(g: &mut Game, keep: usize) {
    let mut kept = 0;
    for l in g.permanents(0).collect::<Vec<_>>() {
        if g.types_of(l).land {
            if g.def_of(l).name == "Swamp" && kept < keep {
                kept += 1;
            } else {
                g.objects[l.0 as usize].tapped = true;
            }
        }
    }
}

#[test]
fn ral_monsoon_mage_discounts_and_flips_into_ral_leyline_prodigy() {
    let mut g = table();
    let ral = put(&mut g, 0, RAL);
    tap_all_but_swamps(&mut g, 1);
    let nw = hand(&mut g, 0, "Night's Whisper");
    assert!(
        g.legal_actions().contains(&Action::Cast(nw)),
        "{{1}}{{B}} for {{B}}"
    );
    // Cast instants and sorceries until a flip is won (each lost flip deals Ral's 1 damage).
    let mut transformed = false;
    for _ in 0..30 {
        for l in g.permanents(0).collect::<Vec<_>>() {
            g.objects[l.0 as usize].tapped = false;
        }
        let nw = hand(&mut g, 0, "Night's Whisper");
        let life = g.players[0].life;
        act(&mut g, Action::Cast(nw));
        let mut asked = false;
        while !g.items.is_empty() || matches!(g.pending, Pending::Decide(_)) {
            if matches!(g.pending, Pending::Decide(_)) {
                asked = true;
                act(&mut g, Action::Yes);
            } else {
                act(&mut g, Action::PassPriority);
            }
        }
        if asked {
            transformed = true;
            break;
        }
        assert_eq!(
            g.players[0].life,
            life - 2 - 1,
            "Night's Whisper's 2, the lost flip's 1"
        );
    }
    assert!(transformed, "a won flip in 30");
    let ral = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Ral, Leyline Prodigy")
        .unwrap_or(ral);
    assert_eq!(g.def_of(ral).name, "Ral, Leyline Prodigy");
    assert!(
        g.obj(ral).loyalty > 2,
        "an additional loyalty counter per instant and sorcery"
    );
}

#[test]
fn ral_leyline_prodigy_minus_two_divides_damage_and_draws_with_a_blue_permanent() {
    let mut g = table();
    let ral = put(&mut g, 0, RAL);
    g.objects[ral.0 as usize].transformed = true;
    g.objects[ral.0 as usize].loyalty = 8;
    assert_eq!(g.def_of(ral).name, "Ral, Leyline Prodigy");
    let hand_before = g.players[0].hand.len();
    let life = g.players[1].life;
    act(&mut g, Action::Loyalty(ral, 1));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::No);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 2, "both to one target");
    assert_eq!(
        g.players[0].hand.len(),
        hand_before,
        "Ral is his own blue permanent: no draw"
    );
}

#[test]
fn ral_leyline_prodigy_minus_eight_casts_exiled_instants_and_sorceries_free() {
    let mut g = table();
    let ral = put(&mut g, 0, RAL);
    g.objects[ral.0 as usize].transformed = true;
    g.objects[ral.0 as usize].loyalty = 8;
    for _ in 0..8 {
        g.add_card(0, cid("Night's Whisper"), Zone::Library);
    }
    act(&mut g, Action::Loyalty(ral, 2));
    resolve_all(&mut g, &[]);
    let exiled: Vec<ObjId> = g.exile.iter().copied().filter(|&c| g.obj(c).owner == 0).collect();
    assert_eq!(exiled.len(), 8);
    for l in g.permanents(0).collect::<Vec<_>>() {
        if g.types_of(l).land {
            g.objects[l.0 as usize].tapped = true;
        }
    }
    assert!(
        g.legal_actions().contains(&Action::Cast(exiled[0])),
        "free, from exile, this turn"
    );
}

/// Taps every land of player 0's except `keep`.
fn tap_lands_except(g: &mut Game, keep: &[ObjId]) {
    for l in g.permanents(0).collect::<Vec<_>>() {
        if g.types_of(l).land && !keep.contains(&l) {
            g.objects[l.0 as usize].tapped = true;
        }
    }
}

#[test]
fn urza_s_incubator_discounts_creature_spells_of_the_chosen_type() {
    let mut g = table();
    let inc = put(&mut g, 0, "Urza's Incubator");
    g.objects[inc.0 as usize].chosen_type = Some("Shaman".to_string());
    let forests: Vec<ObjId> = g
        .permanents(0)
        .filter(|&x| g.def_of(x).name == "Forest")
        .take(2)
        .collect();
    tap_lands_except(&mut g, &forests);
    let witness = hand(&mut g, 0, "Eternal Witness");
    assert!(
        g.legal_actions().contains(&Action::Cast(witness)),
        "{{1}}{{G}}{{G}} less {{2}}: {{G}}{{G}}"
    );
    g.objects[inc.0 as usize].chosen_type = Some("Elf".to_string());
    assert!(!g.legal_actions().contains(&Action::Cast(witness)), "not an Elf");
}

#[test]
fn haven_of_the_spirit_dragon_colors_only_dragon_creature_spells_and_returns_a_dragon() {
    let mut g = table();
    let haven = put(&mut g, 0, "Haven of the Spirit Dragon");
    let plains: Vec<ObjId> = g
        .permanents(0)
        .filter(|&x| g.def_of(x).name == "Plains")
        .take(2)
        .collect();
    tap_lands_except(&mut g, &[haven, plains[0], plains[1]]);
    let gadrak = hand(&mut g, 0, "Gadrak, the Crown-Scourge");
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    let legal = g.legal_actions();
    assert!(
        legal.contains(&Action::Cast(gadrak)),
        "{{R}} from the Haven for a Dragon creature"
    );
    assert!(!legal.contains(&Action::Cast(bolt)), "not for a non-Dragon spell");
    // {2}, {T}, sacrifice: a Dragon card back to hand.
    let mut g = table();
    let haven = put(&mut g, 0, "Haven of the Spirit Dragon");
    let dead = g.add_card(0, cid("Gadrak, the Crown-Scourge"), Zone::Graveyard);
    let elf = g.add_card(0, cid("Llanowar Elves"), Zone::Graveyard);
    act(&mut g, Action::Activate(haven, 0));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(elf))),
        "not a Dragon"
    );
    act(&mut g, Action::Target(obj(dead)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(dead).zone, Zone::Hand);
    assert_eq!(g.obj(haven).zone, Zone::Graveyard, "sacrificed");
}

#[test]
fn crypt_of_agadeem_enters_tapped_and_adds_black_per_black_creature_card() {
    let mut g = table();
    let c = hand(&mut g, 0, "Crypt of Agadeem");
    act(&mut g, Action::PlayLand(c));
    resolve_all(&mut g, &[]);
    assert!(g.obj(c).tapped);
    let mut g = table();
    let crypt = put(&mut g, 0, "Crypt of Agadeem");
    g.add_card(0, cid("Blood Artist"), Zone::Graveyard);
    g.add_card(0, cid("Viscera Seer"), Zone::Graveyard);
    g.add_card(0, cid("Llanowar Elves"), Zone::Graveyard);
    act(&mut g, Action::Activate(crypt, 0));
    resolve_all(&mut g, &[]);
    let black = g.players[0]
        .pool
        .iter()
        .filter(|m| m.colors == [false, false, true, false, false])
        .count();
    assert_eq!(black, 2, "{:?}", g.players[0].pool);
}

#[test]
fn nesting_grounds_moves_a_counter_and_asks_the_kind() {
    let mut g = table();
    let ng = put(&mut g, 0, "Nesting Grounds");
    let a = put(&mut g, 0, "Llanowar Elves");
    let b = put(&mut g, 1, "Llanowar Elves");
    set_counters(&mut g, a, 2);
    act(&mut g, Action::Activate(ng, 0));
    act(&mut g, Action::Target(obj(a)));
    act(&mut g, Action::Target(obj(b)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(a).counters, g.obj(b).counters), (1, 1));
    // Two kinds: the player picks (here a stun counter).
    let mut g = table();
    let ng = put(&mut g, 0, "Nesting Grounds");
    let a = put(&mut g, 0, "Llanowar Elves");
    let b = put(&mut g, 1, "Llanowar Elves");
    set_counters(&mut g, a, 1);
    g.objects[a.0 as usize].stun = 1;
    act(&mut g, Action::Activate(ng, 0));
    act(&mut g, Action::Target(obj(a)));
    act(&mut g, Action::Target(obj(b)));
    resolve_all(&mut g, &[Action::Mode(2)]);
    assert_eq!((g.obj(a).stun, g.obj(b).stun), (0, 1));
    assert_eq!(g.obj(a).counters, 1);
}

#[test]
fn inkmoth_nexus_becomes_a_flying_infect_artifact_creature() {
    let mut g = table();
    let ink = put(&mut g, 0, "Inkmoth Nexus");
    act(&mut g, Action::Activate(ink, 0));
    resolve_all(&mut g, &[]);
    let t = g.types_of(ink);
    assert!(t.creature && t.artifact && t.land);
    assert_eq!((g.power(ink), g.toughness(ink)), (1, 1));
    assert!(g.keywords(ink).has(Keywords::FLYING));
    assert!(g.has_subtype(ink, "Blinkmoth") && g.has_subtype(ink, "Phyrexian"));
    assert!(
        g.infect_until_eot.iter().any(|e| e.0 == ink),
        "infect until end of turn"
    );
}

#[test]
fn hushwood_verge_and_tainted_field_make_their_conditional_colors() {
    let mut g = table();
    let hv = put(&mut g, 0, "Hushwood Verge");
    let tf = put(&mut g, 0, "Tainted Field");
    let (c, _) = makes(&g, hv);
    assert!(c[4] && c[0], "{{G}}, and {{W}} with a Forest or Plains");
    let (c, colorless) = makes(&g, tf);
    assert!(
        colorless && c[0] && c[2],
        "{{C}}, and {{W}} or {{B}} with a Swamp"
    );
}

#[test]
fn gaeas_cradle_adds_green_per_creature() {
    let mut g = table();
    let cradle = put(&mut g, 0, "Gaea's Cradle");
    for _ in 0..3 {
        put(&mut g, 0, "Llanowar Elves");
    }
    tap_lands_except(&mut g, &[cradle]);
    for e in g
        .permanents(0)
        .filter(|&x| g.types_of(x).creature)
        .collect::<Vec<_>>()
    {
        g.objects[e.0 as usize].tapped = true; // no Elf mana
    }
    let witness = hand(&mut g, 0, "Eternal Witness");
    assert!(
        g.legal_actions().contains(&Action::Cast(witness)),
        "three creatures: {{G}}{{G}}{{G}}"
    );
}

#[test]
fn thalia_taxes_noncreature_spells_for_everyone() {
    let mut g = table();
    put(&mut g, 1, "Thalia, Guardian of Thraben");
    tap_all_but_swamps(&mut g, 2);
    let nw = hand(&mut g, 0, "Night's Whisper");
    let seer = hand(&mut g, 0, "Viscera Seer");
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Cast(nw)), "{{1}}{{B}} plus {{1}}");
    assert!(
        legal.contains(&Action::Cast(seer)),
        "a creature spell isn't taxed"
    );
    let s = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Swamp" && g.obj(x).tapped)
        .expect("a Swamp");
    g.objects[s.0 as usize].tapped = false;
    assert!(g.legal_actions().contains(&Action::Cast(nw)), "three mana");
}

#[test]
fn oakhame_adversary_costs_two_less_against_a_green_permanent() {
    let mut g = table();
    let forests: Vec<ObjId> = g
        .permanents(0)
        .filter(|&x| g.def_of(x).name == "Forest")
        .take(2)
        .collect();
    tap_lands_except(&mut g, &forests);
    let oa = hand(&mut g, 0, "Oakhame Adversary");
    assert!(
        !g.legal_actions().contains(&Action::Cast(oa)),
        "{{3}}{{G}} with two lands"
    );
    put(&mut g, 1, "Llanowar Elves");
    assert!(
        g.legal_actions().contains(&Action::Cast(oa)),
        "{{1}}{{G}}: an opponent's green permanent"
    );
}

#[test]
fn nogi_discounts_dragon_spells() {
    let mut g = table();
    put(&mut g, 0, "Nogi, Draco-Zealot");
    let mountains: Vec<ObjId> = g
        .permanents(0)
        .filter(|&x| g.def_of(x).name == "Mountain")
        .take(2)
        .collect();
    tap_lands_except(&mut g, &mountains);
    let gadrak = hand(&mut g, 0, "Gadrak, the Crown-Scourge");
    assert!(
        g.legal_actions().contains(&Action::Cast(gadrak)),
        "{{2}}{{R}} less {{1}}"
    );
}

#[test]
fn klauth_adds_spell_only_mana_that_lasts_the_turn() {
    let mut g = table();
    let k = put(&mut g, 0, "Klauth, Unrivaled Ancient");
    g.objects[k.0 as usize].since_turn = 0;
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(k));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[]);
    let x = g.power(k) as u32;
    assert_eq!(g.spell_mana_eot[0], (x, g.turn), "X = the attackers' total power");
    // Still there after combat, and it pays for a spell.
    while g.step != Step::Main2 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.spell_mana_eot[0].0, x);
    tap_lands_except(&mut g, &[]);
    let nw = hand(&mut g, 0, "Night's Whisper");
    assert!(g.legal_actions().contains(&Action::Cast(nw)));
    act(&mut g, Action::Cast(nw));
    assert_eq!(g.spell_mana_eot[0].0, x - 2);
}

#[test]
fn dragon_broodmother_makes_a_dragon_that_devours_at_each_upkeep() {
    let mut g = table();
    put(&mut g, 0, "Dragon Broodmother");
    let elf = put(&mut g, 0, "Llanowar Elves");
    let seer = put(&mut g, 0, "Viscera Seer");
    // Through the opponent's upkeep: its trigger makes player 0 a Dragon, and the devour asks.
    while !matches!(g.pending, Pending::Decide(0)) {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ if !g.legal_actions().contains(&Action::PassPriority) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    let dragon = tokens(&g, 0, "Dragon");
    assert_eq!(dragon.len(), 1);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(elf)) && legal.contains(&Action::No));
    act(&mut g, Action::Pick(elf));
    act(&mut g, Action::Pick(seer));
    assert_eq!(g.obj(elf).zone, Zone::Graveyard);
    assert_eq!(g.obj(seer).zone, Zone::Graveyard);
    assert_eq!(g.obj(dragon[0]).counters, 4, "devour 2: two creatures");
    assert!(g.keywords(dragon[0]).has(Keywords::FLYING));
}

#[test]
fn cradle_clearcutter_casts_as_a_prototype_and_taps_for_its_power() {
    let mut g = table();
    let forests: Vec<ObjId> = g
        .permanents(0)
        .filter(|&x| g.def_of(x).name == "Forest")
        .take(3)
        .collect();
    tap_lands_except(&mut g, &forests);
    let cc = hand(&mut g, 0, "Cradle Clearcutter");
    let legal = g.legal_actions();
    assert!(
        legal.contains(&Action::CastAlternative(cc)),
        "{{2}}{{G}} prototype"
    );
    assert!(!legal.contains(&Action::Cast(cc)), "{{6}}: three lands");
    act(&mut g, Action::CastAlternative(cc));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(cc).zone, Zone::Battlefield);
    assert_eq!((g.power(cc), g.toughness(cc)), (1, 3));
    assert!(g.colors_of(cc)[4], "green");
    // Full size: 3/6, colorless; it taps for {G} per power.
    let mut g = table();
    let cc = put(&mut g, 0, "Cradle Clearcutter");
    g.objects[cc.0 as usize].since_turn = 0;
    assert_eq!((g.power(cc), g.toughness(cc)), (3, 6));
    assert!(!g.colors_of(cc).iter().any(|&k| k));
    act(&mut g, Action::Activate(cc, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].pool.len(), 3);
}

#[test]
fn valeria_discounts_noncreature_spells_and_draws_on_the_first() {
    let mut g = table();
    put(&mut g, 0, "Valeria Richards, Precocious");
    tap_all_but_swamps(&mut g, 1);
    let nw = hand(&mut g, 0, "Night's Whisper");
    assert!(
        g.legal_actions().contains(&Action::Cast(nw)),
        "{{1}}{{B}} less {{1}}"
    );
    let hand_before = g.players[0].hand.len();
    act(&mut g, Action::Cast(nw));
    resolve_all(&mut g, &[]);
    // Night's Whisper draws two, Valeria's trigger one; the cast card left the hand.
    assert_eq!(g.players[0].hand.len(), hand_before - 1 + 2 + 1);
}

#[test]
fn ayara_drains_on_black_entries_and_sacrifices_a_named_black_creature() {
    let mut g = table();
    let life = (g.players[0].life, g.players[1].life);
    let ayara = hand(&mut g, 0, "Ayara, First of Locthwain");
    act(&mut g, Action::Cast(ayara));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.players[0].life, g.players[1].life),
        (life.0 + 1, life.1 - 1),
        "Ayara itself"
    );
    let seer = hand(&mut g, 0, "Viscera Seer");
    act(&mut g, Action::Cast(seer));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life.1 - 2, "another black creature");
    let elf = put(&mut g, 0, "Llanowar Elves");
    g.objects[ayara.0 as usize].since_turn = 0;
    let legal = g.legal_actions();
    assert!(
        legal.contains(&Action::ActivateWith(ayara, 0, seer)),
        "a black creature"
    );
    assert!(!legal.contains(&Action::ActivateWith(ayara, 0, elf)), "not green");
    let hand_before = g.players[0].hand.len();
    act(&mut g, Action::ActivateWith(ayara, 0, seer));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(seer).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), hand_before + 1);
}

#[test]
fn weftstalker_ardent_pings_for_creatures_and_artifacts_and_has_warp() {
    let mut g = table();
    let w = hand(&mut g, 0, "Weftstalker Ardent");
    assert!(
        g.legal_actions().contains(&Action::CastAlternative(w)),
        "warp {{R}}"
    );
    act(&mut g, Action::CastAlternative(w));
    resolve_all(&mut g, &[]);
    let life = g.players[1].life;
    let altar = hand(&mut g, 0, "Ashnod's Altar");
    act(&mut g, Action::Cast(altar));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 1, "an artifact");
    let elf = hand(&mut g, 0, "Llanowar Elves");
    act(&mut g, Action::Cast(elf));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 2, "a creature");
}

#[test]
fn liege_of_the_hollows_lets_each_player_pay_for_squirrels() {
    let mut g = table();
    let liege = put(&mut g, 0, "Liege of the Hollows");
    g.move_card(liege, Zone::Graveyard);
    act(&mut g, Action::PassPriority); // the trigger goes on the stack
                                       // Player 0 (active) pays 2, then player 1 pays 1.
    resolve_all(&mut g, &[Action::Amount(2), Action::Amount(1)]);
    assert_eq!(tokens(&g, 0, "Squirrel").len(), 2);
    assert_eq!(tokens(&g, 1, "Squirrel").len(), 1);
}

#[test]
fn nahiris_lithoforming_sacrifices_draws_and_lands_enter_tapped() {
    let mut g = table();
    let hand0 = g.players[0].hand.len();
    let nl = hand(&mut g, 0, "Nahiri's Lithoforming");
    act(&mut g, Action::CastX(nl, 1));
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    let pick = *g
        .legal_actions()
        .iter()
        .find(|a| matches!(a, Action::Pick(_)))
        .expect("a land");
    resolve_all(&mut g, &[pick]);
    assert_eq!(
        g.players[0].hand.len(),
        hand0 + 1,
        "one land sacrificed, one card drawn"
    );
    assert_eq!(g.extra_lands_this_turn[0], 1);
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[]);
    assert!(g.obj(f).tapped, "lands enter tapped this turn");
}
