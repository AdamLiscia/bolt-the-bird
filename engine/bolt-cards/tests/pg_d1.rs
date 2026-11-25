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
