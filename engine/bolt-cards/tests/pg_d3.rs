//! Playgroup decks, lane D3 (the desktop): each script's behavior in a 1v1 Commander game on the CMD pool.
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

fn to_main1(g: &mut Game, turn: u32, active: usize) {
    g.start_turn(turn, active);
    while g.step != Step::Main1 {
        act(g, Action::PassPriority);
    }
}

/// The colors a permanent can make right now.
fn colors(g: &Game, id: ObjId) -> [bool; 5] {
    g.mana_sources(0)
        .into_iter()
        .filter(|&(x, _)| x == id)
        .fold([false; 5], |acc, (_, m)| {
            std::array::from_fn(|i| acc[i] || m.colors[i])
        })
}

const WUB: [bool; 5] = [true, true, true, false, false];

/// Like `game`, but player 0's library (and so its opening hand) is 99 of `library`.
fn game_drawing(library: &str, lands: &[&str]) -> Game {
    let mine = vec![cid(library); 99];
    let plains = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        set("CMD").unwrap().defs.clone(),
        [&mine, &plains],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    for &l in lands {
        put(&mut g, 0, l);
    }
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g
}

// ---- Sefris of the Hidden Ways (Spr3adsh33t) ----

#[test]
fn isolated_chapel_enters_untapped_with_a_plains_or_swamp() {
    let mut g = game(&["Island"]);
    let c = hand(&mut g, 0, "Isolated Chapel");
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped);
    for other in ["Plains", "Swamp"] {
        let mut g = game(&[other]);
        let c = hand(&mut g, 0, "Isolated Chapel");
        act(&mut g, Action::PlayLand(c));
        assert!(!g.obj(c).tapped, "{other}");
        assert_eq!(colors(&g, c), [true, false, true, false, false]);
    }
}

/// With one Plains and `signet`, `spell` (costing exactly the signet's two colors) can be cast;
/// with the signet alone it can't.
fn signet_pays(signet: &str, spell: &str) {
    let mut g = game(&["Plains"]);
    put(&mut g, 0, signet);
    let c = hand(&mut g, 0, spell);
    assert!(
        g.legal_actions().contains(&Action::Cast(c)),
        "{signet} makes the two colors"
    );
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(c).zone, Zone::Battlefield);
    let mut g = game(&[]);
    put(&mut g, 0, signet);
    let c = hand(&mut g, 0, spell);
    assert!(
        !g.legal_actions().contains(&Action::Cast(c)),
        "its {{1}} needs another source"
    );
}

#[test]
fn dimir_signet_turns_one_mana_into_blue_and_black() {
    signet_pays("Dimir Signet", "Vohar, Vodalian Desecrator"); // {U}{B}
}

#[test]
fn orzhov_signet_turns_one_mana_into_white_and_black() {
    signet_pays("Orzhov Signet", "Cartel Aristocrat"); // {W}{B}
}

#[test]
fn darkwater_catacombs_turns_one_mana_into_blue_and_black() {
    signet_pays("Darkwater Catacombs", "Vohar, Vodalian Desecrator");
}

#[test]
fn raffines_tower_enters_tapped_makes_three_colors_and_cycles() {
    let mut g = game(&["Plains", "Plains", "Plains"]);
    let c = hand(&mut g, 0, "Raffine's Tower");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(c, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).zone, Zone::Graveyard, "cycled");
    assert_eq!(g.players[0].hand.len(), n, "and drew a card");
    let mut g = game(&[]);
    let c = hand(&mut g, 0, "Raffine's Tower");
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped);
    to_main1(&mut g, 5, 0);
    assert_eq!(colors(&g, c), WUB);
}

#[test]
fn counterspell_counters_a_spell() {
    let mut g = table();
    let looter = hand(&mut g, 0, "Merfolk Looter");
    let cs = hand(&mut g, 0, "Counterspell");
    act(&mut g, Action::Cast(looter));
    act(&mut g, Action::Cast(cs));
    act(&mut g, Action::Target(obj(looter)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(looter).zone, Zone::Graveyard);
    assert_eq!(g.obj(cs).zone, Zone::Graveyard);
}

#[test]
fn entomb_puts_any_card_from_the_library_into_the_graveyard() {
    let mut g = table();
    let rider = g.add_card(0, cid("Ashen Rider"), Zone::Library);
    let e = hand(&mut g, 0, "Entomb");
    play(&mut g, e, false, &[], &[Action::Pick(rider)]);
    assert_eq!(g.obj(rider).zone, Zone::Graveyard);
}

#[test]
fn buried_alive_puts_up_to_three_creature_cards_into_the_graveyard() {
    let mut g = table();
    let cards: Vec<ObjId> = ["Ashen Rider", "Merfolk Looter", "Razaketh, the Foulblooded"]
        .iter()
        .map(|n| g.add_card(0, cid(n), Zone::Library))
        .collect();
    let spell = g.add_card(0, cid("Counterspell"), Zone::Library);
    let b = hand(&mut g, 0, "Buried Alive");
    act(&mut g, Action::Cast(b));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Pick(spell)),
        "creature cards only"
    );
    resolve_all(
        &mut g,
        &cards.iter().map(|&c| Action::Pick(c)).collect::<Vec<_>>(),
    );
    for c in cards {
        assert_eq!(g.obj(c).zone, Zone::Graveyard);
    }
    assert_eq!(g.obj(spell).zone, Zone::Library);

    // "Up to": it may stop after one.
    let mut g = table();
    let one = g.add_card(0, cid("Ashen Rider"), Zone::Library);
    let other = g.add_card(0, cid("Merfolk Looter"), Zone::Library);
    let b = hand(&mut g, 0, "Buried Alive");
    play(&mut g, b, false, &[], &[Action::Pick(one), Action::No]);
    assert_eq!(g.obj(one).zone, Zone::Graveyard);
    assert_eq!(g.obj(other).zone, Zone::Library);
}

#[test]
fn geier_reach_sanitarium_makes_each_player_loot() {
    let mut g = game(&["Plains", "Plains"]);
    let s = put(&mut g, 0, "Geier Reach Sanitarium");
    assert!(g.mana_sources(0).iter().any(|&(x, m)| x == s && m.colorless));
    let mine = hand(&mut g, 0, "Counterspell");
    let theirs = hand(&mut g, 1, "Entomb");
    let (h0, h1) = (g.players[0].hand.len(), g.players[1].hand.len());
    act(&mut g, Action::Activate(s, 0));
    resolve_all(&mut g, &[Action::Discard(mine), Action::Discard(theirs)]);
    assert!(g.obj(s).tapped);
    assert_eq!(g.obj(mine).zone, Zone::Graveyard);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!((g.players[0].hand.len(), g.players[1].hand.len()), (h0, h1));
}

#[test]
fn razaketh_pays_2_life_and_a_creature_to_tutor_any_card() {
    let mut g = table();
    let r = put(&mut g, 0, "Razaketh, the Foulblooded");
    let elder = put(&mut g, 0, "Merfolk Looter");
    let card = g.add_card(0, cid("Entomb"), Zone::Library);
    assert!(
        !g.legal_actions().contains(&Action::ActivateWith(r, 0, r)),
        "another creature"
    );
    let life = g.players[0].life;
    act(&mut g, Action::ActivateWith(r, 0, elder));
    resolve_all(&mut g, &[Action::Pick(card)]);
    assert_eq!(g.players[0].life, life - 2);
    assert_eq!(g.obj(elder).zone, Zone::Graveyard);
    assert_eq!(g.obj(card).zone, Zone::Hand);
}

#[test]
fn ashen_rider_exiles_a_permanent_on_entering_and_on_dying() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let land = put(&mut g, 1, "Island");
    let r = hand(&mut g, 0, "Ashen Rider");
    play(&mut g, r, false, &[], &[Action::Target(obj(ring))]);
    assert_eq!(g.obj(ring).zone, Zone::Exile);
    let bw = hand(&mut g, 0, "Beast Within");
    play(&mut g, bw, false, &[obj(r)], &[Action::Target(obj(land))]);
    assert_eq!(g.obj(r).zone, Zone::Graveyard);
    assert_eq!(g.obj(land).zone, Zone::Exile, "a land is a permanent too");
}

#[test]
fn arcane_sanctum_enters_tapped_and_makes_three_colors() {
    let mut g = game(&[]);
    let c = hand(&mut g, 0, "Arcane Sanctum");
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped);
    to_main1(&mut g, 5, 0);
    assert_eq!(colors(&g, c), WUB);
}

#[test]
fn cavalier_of_night_trades_a_creature_for_theirs_and_reanimates_on_death() {
    let mut g = table();
    let looter = put(&mut g, 0, "Merfolk Looter");
    let bear = put(&mut g, 1, "Cartel Aristocrat");
    let small = g.add_card(0, cid("Thought Courier"), Zone::Graveyard); // mana value 2
    let big = g.add_card(0, cid("Ashen Rider"), Zone::Graveyard); // 8
    let c = hand(&mut g, 0, "Cavalier of Night");
    play(
        &mut g,
        c,
        false,
        &[],
        &[Action::Pick(looter), Action::Target(obj(bear))],
    );
    assert_eq!(g.obj(looter).zone, Zone::Graveyard);
    assert_eq!(g.obj(bear).zone, Zone::Graveyard);
    assert!(g.keywords(c).has(Keywords::LIFELINK));

    let bw = hand(&mut g, 0, "Beast Within");
    act(&mut g, Action::Cast(bw));
    act(&mut g, Action::Target(obj(c)));
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(big))),
        "mana value 3 or less"
    );
    resolve_all(&mut g, &[Action::Target(obj(small))]);
    assert_eq!(g.obj(small).zone, Zone::Battlefield);
    assert_eq!(g.obj(big).zone, Zone::Graveyard);

    // Declining the sacrifice destroys nothing.
    let mut g = table();
    put(&mut g, 0, "Merfolk Looter");
    let bear = put(&mut g, 1, "Cartel Aristocrat");
    let c = hand(&mut g, 0, "Cavalier of Night");
    play(&mut g, c, false, &[], &[Action::No]);
    assert_eq!(g.obj(bear).zone, Zone::Battlefield);
}

#[test]
fn choked_estuary_enters_untapped_only_revealing_an_island_or_swamp() {
    let mut g = game(&[]); // a hand of Plains
    let c = hand(&mut g, 0, "Choked Estuary");
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped, "nothing to reveal");
    let mut g = game(&[]);
    hand(&mut g, 0, "Swamp");
    let c = hand(&mut g, 0, "Choked Estuary");
    act(&mut g, Action::PlayLand(c));
    assert!(!g.obj(c).tapped, "revealed a Swamp");
    assert_eq!(colors(&g, c), [false, true, true, false, false]);
}

#[test]
fn port_town_enters_untapped_only_revealing_a_plains_or_island() {
    let mut g = game_drawing("Swamp", &[]); // a hand of Swamps
    let c = hand(&mut g, 0, "Port Town");
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped, "nothing to reveal");
    let mut g = game_drawing("Swamp", &[]);
    hand(&mut g, 0, "Island");
    let c = hand(&mut g, 0, "Port Town");
    act(&mut g, Action::PlayLand(c));
    assert!(!g.obj(c).tapped, "revealed an Island");
    assert_eq!(colors(&g, c), [true, true, false, false, false]);
}

#[test]
fn darkslick_shores_enters_tapped_after_three_other_lands() {
    let mut g = game(&["Plains", "Plains"]);
    let c = hand(&mut g, 0, "Darkslick Shores");
    act(&mut g, Action::PlayLand(c));
    assert!(!g.obj(c).tapped);
    assert_eq!(colors(&g, c), [false, true, true, false, false]);
    let mut g = game(&["Plains", "Plains", "Plains"]);
    let c = hand(&mut g, 0, "Darkslick Shores");
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped);
}

fn loots(name: &str) {
    let mut g = table();
    let l = put(&mut g, 0, name);
    to_main1(&mut g, 5, 0);
    let junk = hand(&mut g, 0, "Counterspell");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(l, 0));
    resolve_all(&mut g, &[Action::Discard(junk)]);
    assert!(g.obj(l).tapped);
    assert_eq!(g.obj(junk).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n, "drew one, discarded one");
}

#[test]
fn merfolk_looter_taps_to_loot() {
    loots("Merfolk Looter");
}

#[test]
fn thought_courier_taps_to_loot() {
    loots("Thought Courier");
}

#[test]
fn vile_entomber_entombs_on_entering() {
    let mut g = table();
    let card = g.add_card(0, cid("Razaketh, the Foulblooded"), Zone::Library);
    let v = hand(&mut g, 0, "Vile Entomber");
    play(&mut g, v, false, &[], &[Action::Pick(card)]);
    assert_eq!(g.obj(card).zone, Zone::Graveyard);
    assert!(g.keywords(v).has(Keywords::DEATHTOUCH));
}
