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
    assert!(
        !matches!(g.obj(c).zone, Zone::Hand | Zone::Stack),
        "cast and resolved"
    );
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

// ---- Najeela, the Blade-Blossom (Time) ----

#[test]
fn flooded_strand_fetches_a_plains_or_island() {
    let mut g = table();
    let f = put(&mut g, 0, "Flooded Strand");
    let bad = g.add_card(0, cid("Swamp"), Zone::Library);
    let good = g.add_card(0, cid("Island"), Zone::Library);
    let life = g.players[0].life;
    act(&mut g, Action::Activate(f, 0));
    assert_eq!(g.players[0].life, life - 1);
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(!g.legal_actions().contains(&Action::Pick(bad)));
    act(&mut g, Action::Pick(good));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(good).zone, Zone::Battlefield);
    assert_eq!(g.obj(f).zone, Zone::Graveyard);
}

#[test]
fn hallowed_fountain_pays_2_life_or_enters_tapped() {
    for pay in [true, false] {
        let mut g = game(&[]);
        let life = g.players[0].life;
        let c = hand(&mut g, 0, "Hallowed Fountain");
        act(&mut g, Action::PlayLand(c));
        act(&mut g, if pay { Action::Yes } else { Action::No });
        assert_eq!(g.obj(c).tapped, !pay);
        assert_eq!(g.players[0].life, if pay { life - 2 } else { life });
        if pay {
            assert_eq!(colors(&g, c), [true, true, false, false, false]);
        }
    }
}

#[test]
fn the_triomes_enter_tapped_and_make_their_three_colors() {
    for (name, mana) in [
        ("Ziatora's Proving Ground", [false, false, true, true, true]),
        ("Ketria Triome", [false, true, false, true, true]),
        ("Savai Triome", [true, false, true, true, false]),
    ] {
        let mut g = game(&[]);
        let c = hand(&mut g, 0, name);
        act(&mut g, Action::PlayLand(c));
        assert!(g.obj(c).tapped, "{name}");
        to_main1(&mut g, 5, 0);
        assert_eq!(colors(&g, c), mana, "{name}");
        let mut g = game(&["Plains", "Plains", "Plains"]);
        let c = hand(&mut g, 0, name);
        act(&mut g, Action::Activate(c, 0));
        resolve_all(&mut g, &[]);
        assert_eq!(g.obj(c).zone, Zone::Graveyard, "{name} cycles");
    }
}

#[test]
fn selesnya_signet_turns_one_mana_into_green_and_white() {
    signet_pays("Selesnya Signet", "Shanna, Sisay's Legacy"); // {G}{W}
}

#[test]
fn boros_charm_burns_protects_or_grants_double_strike() {
    let mut g = table();
    let bc = hand(&mut g, 0, "Boros Charm");
    act(&mut g, Action::CastModal(bc, 0));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 36);

    let mut g = table();
    let ring = put(&mut g, 0, "Sol Ring");
    let theirs = put(&mut g, 1, "Sol Ring");
    let bc = hand(&mut g, 0, "Boros Charm");
    act(&mut g, Action::CastModal(bc, 1));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(ring).has(Keywords::INDESTRUCTIBLE));
    assert!(!g.keywords(theirs).has(Keywords::INDESTRUCTIBLE));

    let mut g = table();
    let looter = put(&mut g, 0, "Merfolk Looter");
    let bc = hand(&mut g, 0, "Boros Charm");
    act(&mut g, Action::CastModal(bc, 2));
    act(&mut g, Action::Target(obj(looter)));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(looter).has(Keywords::DOUBLE_STRIKE));
}

#[test]
fn brainstorm_draws_three_and_puts_two_back_on_top() {
    let mut g = table();
    let a = hand(&mut g, 0, "Counterspell");
    let b = hand(&mut g, 0, "Entomb");
    let bs = hand(&mut g, 0, "Brainstorm");
    let cards = g.players[0].hand.len();
    play(&mut g, bs, false, &[], &[Action::Pick(a), Action::Pick(b)]);
    assert_eq!(g.players[0].hand.len(), cards - 1 + 3 - 2);
    let lib = &g.players[0].library;
    assert_eq!(&lib[lib.len() - 2..], &[a, b], "Entomb on top");
}

/// Attacks with `attacker` on turn 5 and passes until a decision comes up for player 0, or the turn
/// reaches its second main phase.
fn attack_until_decision(g: &mut Game, attacker: ObjId) {
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    act(g, Action::Attack(attacker));
    act(g, Action::FinishAttacks);
    while !matches!(g.pending, Pending::Decide(0)) && g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn edric_lets_you_draw_when_your_creature_hits_the_opponent() {
    let mut g = table();
    put(&mut g, 0, "Edric, Spymaster of Trest");
    let looter = put(&mut g, 0, "Merfolk Looter");
    g.start_turn(5, 0);
    let cards = g.players[0].hand.len();
    attack_until_decision(&mut g, looter);
    assert_eq!(g.pending, Pending::Decide(0), "may draw");
    act(&mut g, Action::Yes);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 39);
    assert_eq!(g.players[0].hand.len(), cards + 2, "the draw step and Edric");
}

#[test]
fn opt_scries_then_draws() {
    let mut g = table();
    let top = g.add_card(0, cid("Counterspell"), Zone::Library);
    let o = hand(&mut g, 0, "Opt");
    play(&mut g, o, false, &[], &[Action::Yes]); // the top card to the bottom
    assert_eq!(g.obj(top).zone, Zone::Library);
    assert_eq!(g.players[0].library[0], top, "on the bottom");
    let mut g = table();
    let top = g.add_card(0, cid("Counterspell"), Zone::Library);
    let o = hand(&mut g, 0, "Opt");
    play(&mut g, o, false, &[], &[Action::No]); // kept: then drawn
    assert_eq!(g.obj(top).zone, Zone::Hand);
}

#[test]
fn preordain_scries_two_then_draws() {
    let mut g = table();
    let second = g.add_card(0, cid("Entomb"), Zone::Library);
    let top = g.add_card(0, cid("Counterspell"), Zone::Library);
    let p = hand(&mut g, 0, "Preordain");
    play(&mut g, p, false, &[], &[Action::Yes, Action::No]); // bottom the top card, keep the next
    assert_eq!(g.obj(top).zone, Zone::Library);
    assert_eq!(g.obj(second).zone, Zone::Hand);
}

fn talisman(name: &str, pair: [bool; 5]) {
    let mut g = game(&[]);
    let t = put(&mut g, 0, name);
    assert_eq!(colors(&g, t), pair);
    assert!(g.mana_sources(0).iter().any(|&(x, m)| x == t && m.colorless));
}

#[test]
fn talisman_of_dominance_makes_colorless_or_blue_or_black() {
    talisman("Talisman of Dominance", [false, true, true, false, false]);
}

#[test]
fn talisman_of_curiosity_makes_colorless_or_green_or_blue() {
    talisman("Talisman of Curiosity", [false, true, false, false, true]);
}

#[test]
fn fleshbag_marauder_makes_each_player_sacrifice_a_creature() {
    let mut g = table();
    let looter = put(&mut g, 0, "Merfolk Looter");
    let theirs = put(&mut g, 1, "Merfolk Looter");
    let kept = put(&mut g, 1, "Thought Courier");
    let f = hand(&mut g, 0, "Fleshbag Marauder");
    play(
        &mut g,
        f,
        false,
        &[],
        &[Action::Pick(looter), Action::Pick(theirs)],
    );
    assert_eq!(g.obj(looter).zone, Zone::Graveyard);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.obj(kept).zone, Zone::Battlefield, "one each");
    assert_eq!(g.obj(f).zone, Zone::Battlefield);
}

#[test]
fn growth_spiral_draws_and_may_put_a_land_onto_the_battlefield() {
    let mut g = table();
    let forest = hand(&mut g, 0, "Forest");
    let gs = hand(&mut g, 0, "Growth Spiral");
    let cards = g.players[0].hand.len();
    play(&mut g, gs, false, &[], &[Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert_eq!(g.players[0].hand.len(), cards - 1 + 1 - 1);
}

#[test]
fn city_of_death_makes_a_treasure_then_copies_tokens() {
    let mut g = table();
    let c = hand(&mut g, 0, "City of Death");
    play(&mut g, c, false, &[], &[]);
    let t = tokens(&g, 0, "Treasure");
    assert_eq!(t.len(), 1);
    to_main1(&mut g, 5, 0);
    resolve_all(&mut g, &[Action::Target(obj(t[0]))]);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 2, "chapter II copies it");
}

#[test]
fn sylvan_caryatid_taps_for_any_color() {
    let mut g = game(&[]);
    let c = put(&mut g, 0, "Sylvan Caryatid");
    to_main1(&mut g, 5, 0);
    assert_eq!(colors(&g, c), [true; 5]);
    assert!(g.keywords(c).has(Keywords::DEFENDER) && g.keywords(c).has(Keywords::HEXPROOF));
}

// ---- Sheoldred, the Apocalypse (Spr3adsh33t) ----

#[test]
fn decanter_of_endless_water_taps_for_any_color() {
    let mut g = game(&[]);
    let d = put(&mut g, 0, "Decanter of Endless Water");
    assert_eq!(colors(&g, d), [true; 5]);
    assert!(g.def_of(d).no_max_hand_size);
}

#[test]
fn temple_bell_makes_each_player_draw_and_fate_unraveler_punishes_it() {
    let mut g = table();
    let bell = put(&mut g, 0, "Temple Bell");
    put(&mut g, 0, "Fate Unraveler");
    let (h0, h1) = (g.players[0].hand.len(), g.players[1].hand.len());
    act(&mut g, Action::Activate(bell, 0));
    resolve_all(&mut g, &[]);
    assert!(g.obj(bell).tapped);
    assert_eq!(
        (g.players[0].hand.len(), g.players[1].hand.len()),
        (h0 + 1, h1 + 1)
    );
    assert_eq!(g.players[1].life, 39, "Fate Unraveler: 1 for their draw");
    assert_eq!(g.players[0].life, 40);
}

#[test]
fn underworld_dreams_pings_the_opponent_for_each_draw() {
    let mut g = table();
    put(&mut g, 0, "Underworld Dreams");
    let m = put(&mut g, 0, "Mikokoro, Center of the Sea");
    assert!(g.mana_sources(0).iter().any(|&(x, s)| x == m && s.colorless));
    let (h0, h1) = (g.players[0].hand.len(), g.players[1].hand.len());
    act(&mut g, Action::Activate(m, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.players[0].hand.len(), g.players[1].hand.len()),
        (h0 + 1, h1 + 1)
    );
    assert_eq!(g.players[1].life, 39);
}

#[test]
fn charcoal_diamond_enters_tapped_and_taps_for_black() {
    let mut g = table();
    let c = hand(&mut g, 0, "Charcoal Diamond");
    play(&mut g, c, false, &[], &[]);
    assert!(g.obj(c).tapped);
    to_main1(&mut g, 5, 0);
    assert_eq!(colors(&g, c), [false, false, true, false, false]);
}

#[test]
fn font_of_mythos_gives_each_draw_step_two_more_cards() {
    let mut g = table();
    put(&mut g, 0, "Font of Mythos");
    let h0 = g.players[0].hand.len();
    to_main1(&mut g, 5, 0);
    assert_eq!(g.players[0].hand.len(), h0 + 3);
    let h1 = g.players[1].hand.len();
    to_main1(&mut g, 6, 1);
    assert_eq!(g.players[1].hand.len(), h1 + 3, "theirs too");
}

#[test]
fn greed_pays_b_and_2_life_for_a_card() {
    let mut g = table();
    let gr = put(&mut g, 0, "Greed");
    let h = g.players[0].hand.len();
    act(&mut g, Action::Activate(gr, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 38);
    assert_eq!(g.players[0].hand.len(), h + 1);
}

#[test]
fn infernal_grasp_destroys_a_creature_for_2_life() {
    let mut g = table();
    let t = put(&mut g, 1, "Merfolk Looter");
    let ig = hand(&mut g, 0, "Infernal Grasp");
    play(&mut g, ig, false, &[obj(t)], &[]);
    assert_eq!(g.obj(t).zone, Zone::Graveyard);
    assert_eq!(g.players[0].life, 38);
}

#[test]
fn lilianas_caress_drains_2_per_opponent_discard() {
    let mut g = table();
    put(&mut g, 0, "Liliana's Caress");
    let s = put(&mut g, 0, "Geier Reach Sanitarium");
    let mine = hand(&mut g, 0, "Counterspell");
    let theirs = hand(&mut g, 1, "Entomb");
    act(&mut g, Action::Activate(s, 0));
    resolve_all(&mut g, &[Action::Discard(mine), Action::Discard(theirs)]);
    assert_eq!(g.players[1].life, 38);
    assert_eq!(g.players[0].life, 40, "your own discard doesn't count");
}

#[test]
fn master_of_the_feast_feeds_the_opponent_on_your_upkeep() {
    let mut g = table();
    let m = put(&mut g, 0, "Master of the Feast");
    let h1 = g.players[1].hand.len();
    to_main1(&mut g, 5, 0);
    assert_eq!(g.players[1].hand.len(), h1 + 1);
    assert!(g.keywords(m).has(Keywords::FLYING));
}

#[test]
fn bloodthirsty_aerialist_and_epicure_of_blood_feed_on_lifegain() {
    let mut g = table();
    let a = put(&mut g, 0, "Bloodthirsty Aerialist");
    put(&mut g, 0, "Epicure of Blood");
    let mender = hand(&mut g, 0, "Circuit Mender"); // enters: gain 2 life
    play(&mut g, mender, false, &[], &[]);
    assert_eq!(g.players[0].life, 42);
    assert_eq!(g.obj(a).counters, 1);
    assert_eq!(g.players[1].life, 39);
}

#[test]
fn gixian_puppeteer_drains_on_the_second_draw_and_reanimates_on_death() {
    let mut g = table();
    let gp = put(&mut g, 0, "Gixian Puppeteer");
    let gr = put(&mut g, 0, "Greed");
    act(&mut g, Action::Activate(gr, 0)); // the second card this turn
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 38);
    assert_eq!(g.players[0].life, 40, "paid 2, gained 2");

    let small = g.add_card(0, cid("Merfolk Looter"), Zone::Graveyard);
    g.add_card(0, cid("Thought Courier"), Zone::Graveyard);
    let ig = hand(&mut g, 0, "Infernal Grasp");
    act(&mut g, Action::Cast(ig));
    act(&mut g, Action::Target(obj(gp)));
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(gp))),
        "another creature card"
    );
    resolve_all(&mut g, &[Action::Target(obj(small))]);
    assert_eq!(g.obj(small).zone, Zone::Battlefield);
}

#[test]
fn ob_nixilis_hate_twisted_kills_and_punishes_the_draws() {
    let mut g = table();
    let t = put(&mut g, 1, "Merfolk Looter");
    let ob = hand(&mut g, 0, "Ob Nixilis, the Hate-Twisted");
    play(&mut g, ob, false, &[], &[]);
    let h1 = g.players[1].hand.len();
    act(&mut g, Action::Loyalty(ob, 0));
    act(&mut g, Action::Target(obj(t)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(t).zone, Zone::Graveyard);
    assert_eq!(g.players[1].hand.len(), h1 + 2, "its controller draws two");
    assert_eq!(g.players[1].life, 38, "1 damage per card drawn");
}

// ---- Animar, Soul of Elements (Adam) ----

#[test]
fn consecrated_sphinx_draws_two_when_the_opponent_draws() {
    let mut g = table();
    put(&mut g, 0, "Consecrated Sphinx");
    let bell = put(&mut g, 0, "Temple Bell");
    let h0 = g.players[0].hand.len();
    act(&mut g, Action::Activate(bell, 0));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.players[0].hand.len(), h0 + 3, "the Bell's card and two more");
}

#[test]
fn fierce_guardianship_is_free_with_your_commander_out() {
    let mut g = game(&["Island"]);
    let opt = hand(&mut g, 0, "Opt");
    let fg = hand(&mut g, 0, "Fierce Guardianship");
    act(&mut g, Action::Cast(opt));
    assert!(
        !g.legal_actions().contains(&Action::CastAlternative(fg)),
        "no commander"
    );
    let c = put(&mut g, 0, "Merfolk Looter");
    g.commanders[0] = Some(c);
    act(&mut g, Action::CastAlternative(fg));
    act(&mut g, Action::Target(obj(opt)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(opt).zone, Zone::Graveyard);
    assert_eq!(g.obj(fg).zone, Zone::Graveyard);
}

#[test]
fn steam_vents_pays_2_life_or_enters_tapped() {
    for pay in [true, false] {
        let mut g = game(&[]);
        let c = hand(&mut g, 0, "Steam Vents");
        act(&mut g, Action::PlayLand(c));
        act(&mut g, if pay { Action::Yes } else { Action::No });
        assert_eq!(g.obj(c).tapped, !pay);
        assert_eq!(g.players[0].life, if pay { 38 } else { 40 });
        if pay {
            assert_eq!(colors(&g, c), [false, true, false, true, false]);
        }
    }
}

#[test]
fn arbor_elf_untaps_a_forest() {
    let mut g = game(&["Forest", "Plains"]);
    let elf = put(&mut g, 0, "Arbor Elf");
    to_main1(&mut g, 5, 0);
    let forest = g.permanents(0).find(|&x| g.def_of(x).name == "Forest").unwrap();
    let plains = g.permanents(0).find(|&x| g.def_of(x).name == "Plains").unwrap();
    let v = hand(&mut g, 0, "Elvish Visionary"); // {1}{G}: taps both lands
    play(&mut g, v, false, &[], &[]);
    assert!(g.obj(forest).tapped && g.obj(plains).tapped);
    act(&mut g, Action::Activate(elf, 0));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(plains))),
        "a Forest only"
    );
    act(&mut g, Action::Target(obj(forest)));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(forest).tapped);
    assert!(g.obj(plains).tapped);
}

/// With `islands` Islands, casts Consecrated Sphinx (6) and then `name`; counts the untapped lands.
fn untapped_after(name: &str, islands: usize) -> usize {
    let mut g = game(&vec!["Island"; islands]);
    let sphinx = hand(&mut g, 0, "Consecrated Sphinx");
    play(&mut g, sphinx, false, &[], &[]);
    let c = hand(&mut g, 0, name);
    play(&mut g, c, false, &[], &[]);
    g.permanents(0)
        .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
        .count()
}

#[test]
fn peregrine_drake_untaps_five_lands() {
    // 12 Islands: 6 for the Sphinx, 5 for the Drake, then 5 of the 11 tapped ones untap.
    assert_eq!(untapped_after("Peregrine Drake", 12), 6);
}

#[test]
fn cloud_of_faeries_untaps_two_lands_and_cycles() {
    // 8 Islands: 6 for the Sphinx, 2 for the Cloud, then 2 untap.
    assert_eq!(untapped_after("Cloud of Faeries", 8), 2);
    let mut g = game(&["Island", "Island"]);
    let c = hand(&mut g, 0, "Cloud of Faeries");
    let h = g.players[0].hand.len();
    act(&mut g, Action::Activate(c, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), h);
}

/// `name` makes any color, but only for creature spells.
fn creature_mana_only(name: &str) {
    let mut g = game(&[]);
    let src = put(&mut g, 0, name);
    assert_eq!(colors(&g, src), [true; 5]);
    let opt = hand(&mut g, 0, "Opt");
    assert!(
        !g.legal_actions().contains(&Action::Cast(opt)),
        "not for a noncreature spell"
    );
    let siren = hand(&mut g, 0, "Siren Stormtamer"); // {U}
    assert!(g.legal_actions().contains(&Action::Cast(siren)));
}

#[test]
fn ancient_ziggurat_makes_any_color_for_creature_spells() {
    creature_mana_only("Ancient Ziggurat");
}

#[test]
fn beastcaller_savant_has_haste_and_makes_creature_mana() {
    creature_mana_only("Beastcaller Savant"); // just put onto the battlefield: haste lets it tap
}

#[test]
fn elvish_visionary_draws_on_entering() {
    let mut g = table();
    let e = hand(&mut g, 0, "Elvish Visionary");
    let h = g.players[0].hand.len();
    play(&mut g, e, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), h);
}

#[test]
fn pestermite_may_tap_or_untap_a_permanent() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let p = hand(&mut g, 0, "Pestermite");
    play(&mut g, p, false, &[], &[Action::Target(obj(ring)), Action::Yes]);
    assert!(g.obj(ring).tapped);
}

#[test]
fn shared_summons_finds_two_creature_cards() {
    let mut g = table();
    let a = g.add_card(0, cid("Ashen Rider"), Zone::Library);
    let b = g.add_card(0, cid("Merfolk Looter"), Zone::Library);
    let ss = hand(&mut g, 0, "Shared Summons");
    play(&mut g, ss, false, &[], &[Action::Pick(a), Action::Pick(b)]);
    assert_eq!((g.obj(a).zone, g.obj(b).zone), (Zone::Hand, Zone::Hand));
}

#[test]
fn soul_of_the_harvest_draws_for_other_nontoken_creatures() {
    let mut g = table();
    put(&mut g, 0, "Soul of the Harvest");
    let l = hand(&mut g, 0, "Merfolk Looter");
    let h = g.players[0].hand.len();
    play(&mut g, l, false, &[], &[Action::Yes]);
    assert_eq!(g.players[0].hand.len(), h);
}

// ---- Atraxa, Praetors' Voice (Adam) ----

#[test]
fn gavony_township_puts_a_counter_on_each_of_your_creatures() {
    let mut g = table();
    let t = put(&mut g, 0, "Gavony Township");
    let a = put(&mut g, 0, "Merfolk Looter");
    let b = put(&mut g, 0, "Thought Courier");
    let theirs = put(&mut g, 1, "Merfolk Looter");
    act(&mut g, Action::Activate(t, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.obj(a).counters, g.obj(b).counters, g.obj(theirs).counters),
        (1, 1, 0)
    );
}

// ---- Lonis, Cryptozoologist (Spr3adsh33t) ----

#[test]
fn adrix_and_nev_double_your_tokens() {
    let mut g = table();
    let an = put(&mut g, 0, "Adrix and Nev, Twincasters");
    assert!(g.def_of(an).ward.is_some());
    let c = hand(&mut g, 0, "City of Death");
    play(&mut g, c, false, &[], &[]);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 2);
}

#[test]
fn displace_blinks_up_to_two_of_your_creatures() {
    let mut g = table();
    let a = put(&mut g, 0, "Elvish Visionary");
    let b = put(&mut g, 0, "Elvish Visionary");
    let d = hand(&mut g, 0, "Displace");
    let h = g.players[0].hand.len();
    play(&mut g, d, false, &[obj(a), obj(b)], &[]);
    assert_eq!(g.players[0].hand.len(), h - 1 + 2, "both entered again");
}

#[test]
fn ghostly_flicker_blinks_two_of_your_artifacts_creatures_or_lands() {
    let mut g = table();
    let v = put(&mut g, 0, "Elvish Visionary");
    let ring = put(&mut g, 0, "Sol Ring");
    let theirs = put(&mut g, 1, "Merfolk Looter");
    let gf = hand(&mut g, 0, "Ghostly Flicker");
    act(&mut g, Action::Cast(gf));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(theirs))),
        "yours only"
    );
    let h = g.players[0].hand.len();
    act(&mut g, Action::Target(obj(v)));
    act(&mut g, Action::Target(obj(ring)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), h + 1, "the Visionary entered again");
    assert_eq!(g.obj(ring).zone, Zone::Battlefield);
}

#[test]
fn blur_blinks_a_creature_and_draws() {
    let mut g = table();
    let v = put(&mut g, 0, "Elvish Visionary");
    let b = hand(&mut g, 0, "Blur");
    let h = g.players[0].hand.len();
    play(&mut g, b, false, &[obj(v)], &[]);
    assert_eq!(g.players[0].hand.len(), h - 1 + 2);
}

#[test]
fn botanical_sanctum_enters_tapped_after_three_other_lands() {
    let mut g = game(&["Plains", "Plains"]);
    let c = hand(&mut g, 0, "Botanical Sanctum");
    act(&mut g, Action::PlayLand(c));
    assert!(!g.obj(c).tapped);
    assert_eq!(colors(&g, c), [false, true, false, false, true]);
    let mut g = game(&["Plains", "Plains", "Plains"]);
    let c = hand(&mut g, 0, "Botanical Sanctum");
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped);
}

#[test]
fn vineglimmer_snarl_enters_untapped_only_revealing_a_forest_or_island() {
    let mut g = game(&[]);
    let c = hand(&mut g, 0, "Vineglimmer Snarl");
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped, "a hand of Plains");
    let mut g = game(&[]);
    hand(&mut g, 0, "Forest");
    let c = hand(&mut g, 0, "Vineglimmer Snarl");
    act(&mut g, Action::PlayLand(c));
    assert!(!g.obj(c).tapped);
    assert_eq!(colors(&g, c), [false, true, false, false, true]);
}

#[test]
fn simic_cluestone_taps_for_green_or_blue_and_cashes_in_for_a_card() {
    let mut g = table();
    let c = put(&mut g, 0, "Simic Cluestone");
    assert_eq!(colors(&g, c), [false, true, false, false, true]);
    let h = g.players[0].hand.len();
    act(&mut g, Action::Activate(c, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), h + 1);
}

#[test]
fn shrieking_drake_returns_a_creature_you_control() {
    let mut g = table();
    let v = put(&mut g, 0, "Elvish Visionary");
    let d = hand(&mut g, 0, "Shrieking Drake");
    play(&mut g, d, false, &[], &[Action::Pick(v)]);
    assert_eq!(g.obj(v).zone, Zone::Hand);
    assert_eq!(g.obj(d).zone, Zone::Battlefield);
}

#[test]
fn kioras_follower_untaps_another_permanent() {
    let mut g = table();
    let k = put(&mut g, 0, "Kiora's Follower");
    to_main1(&mut g, 5, 0);
    let opt = hand(&mut g, 0, "Opt");
    play(&mut g, opt, false, &[], &[Action::No]);
    let land = g
        .permanents(0)
        .find(|&x| g.types_of(x).land && g.obj(x).tapped)
        .expect("Opt tapped a land");
    act(&mut g, Action::Activate(k, 0));
    assert!(!g.legal_actions().contains(&Action::Target(obj(k))), "another");
    act(&mut g, Action::Target(obj(land)));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(land).tapped);
}

#[test]
fn thassa_is_no_creature_without_devotion_taps_creatures_and_blinks_at_end_step() {
    let mut g = table();
    let t = put(&mut g, 0, "Thassa, Deep-Dwelling");
    assert!(!g.types_of(t).creature, "devotion to blue 1");
    let theirs = put(&mut g, 1, "Merfolk Looter");
    act(&mut g, Action::Activate(t, 0));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert!(g.obj(theirs).tapped);

    let v = put(&mut g, 0, "Elvish Visionary");
    let h = g.players[0].hand.len();
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.step, Step::End);
    resolve_all(&mut g, &[Action::Target(obj(v))]);
    assert_eq!(g.players[0].hand.len(), h + 1, "the Visionary entered again");
}

#[test]
fn displacer_kitten_blinks_a_permanent_when_you_cast_a_noncreature_spell() {
    let mut g = table();
    put(&mut g, 0, "Displacer Kitten");
    let v = put(&mut g, 0, "Elvish Visionary");
    let opt = hand(&mut g, 0, "Opt");
    let h = g.players[0].hand.len();
    play(&mut g, opt, false, &[], &[Action::Target(obj(v)), Action::No]);
    assert_eq!(
        g.players[0].hand.len(),
        h - 1 + 1 + 1,
        "Opt's card and the Visionary's"
    );
}

// ---- Unblocked by lane d3's core theme (effects and statics), batch 1 ----

#[test]
fn altar_of_dementia_mills_the_sacrificed_creatures_power() {
    let mut g = table();
    let altar = put(&mut g, 0, "Altar of Dementia");
    let sphinx = put(&mut g, 0, "Consecrated Sphinx"); // power 4
    let lib = g.players[1].library.len();
    act(&mut g, Action::ActivateWith(altar, 0, sphinx));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(sphinx).zone, Zone::Graveyard);
    assert_eq!(g.players[1].library.len(), lib - 4);
}

#[test]
fn creeping_corrosion_destroys_every_artifact() {
    let mut g = table();
    let a = put(&mut g, 0, "Sol Ring");
    let b = put(&mut g, 1, "Temple Bell");
    let looter = put(&mut g, 1, "Merfolk Looter");
    let c = hand(&mut g, 0, "Creeping Corrosion");
    play(&mut g, c, false, &[], &[]);
    assert_eq!((g.obj(a).zone, g.obj(b).zone), (Zone::Graveyard, Zone::Graveyard));
    assert_eq!(g.obj(looter).zone, Zone::Battlefield);
}

#[test]
fn armageddon_destroys_every_land() {
    let mut g = table();
    let ring = put(&mut g, 0, "Sol Ring");
    let a = hand(&mut g, 0, "Armageddon");
    play(&mut g, a, false, &[], &[]);
    assert_eq!(
        g.permanents(0)
            .chain(g.permanents(1))
            .filter(|&x| g.types_of(x).land)
            .count(),
        0
    );
    assert_eq!(g.obj(ring).zone, Zone::Battlefield);
}

#[test]
fn damnable_pact_makes_a_player_draw_and_lose_x() {
    let mut g = table();
    let p = hand(&mut g, 0, "Damnable Pact");
    let h = g.players[1].hand.len();
    act(&mut g, Action::CastX(p, 4));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].hand.len(), h + 4);
    assert_eq!(g.players[1].life, 36);
}

#[test]
fn merciless_eviction_exiles_all_of_the_chosen_type() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let mine = put(&mut g, 0, "Merfolk Looter");
    let theirs = put(&mut g, 1, "Thought Courier");
    let m = hand(&mut g, 0, "Merciless Eviction");
    act(&mut g, Action::CastModal(m, 1)); // creatures
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(mine).zone, g.obj(theirs).zone), (Zone::Exile, Zone::Exile));
    assert_eq!(g.obj(ring).zone, Zone::Battlefield);
}

#[test]
fn dramatic_reversal_untaps_your_nonland_permanents() {
    let mut g = table();
    let bell = put(&mut g, 0, "Temple Bell");
    act(&mut g, Action::Activate(bell, 0));
    resolve_all(&mut g, &[]);
    assert!(g.obj(bell).tapped);
    let dr = hand(&mut g, 0, "Dramatic Reversal");
    play(&mut g, dr, false, &[], &[]);
    assert!(!g.obj(bell).tapped);
    assert!(
        g.permanents(0).any(|x| g.types_of(x).land && g.obj(x).tapped),
        "the lands that paid for it stay tapped"
    );
}

#[test]
fn venser_s_journal_gains_life_for_each_card_in_hand() {
    let mut g = table();
    let j = put(&mut g, 0, "Venser's Journal");
    assert!(g.def_of(j).no_max_hand_size);
    g.start_turn(5, 0);
    while g.step != Step::Draw {
        act(&mut g, Action::PassPriority);
    }
    // The upkeep trigger resolved before the draw: life went up by the hand size then.
    assert!(g.players[0].life > 40);
    let gained = g.players[0].life - 40;
    assert_eq!(
        gained as usize,
        g.players[0].hand.len() - 1,
        "the hand before this turn's draw"
    );
}

#[test]
fn nissa_voice_of_zendikar_makes_plants_and_counters() {
    let mut g = table();
    let n = hand(&mut g, 0, "Nissa, Voice of Zendikar");
    play(&mut g, n, false, &[], &[]);
    act(&mut g, Action::Loyalty(n, 0));
    resolve_all(&mut g, &[]);
    let plant = tokens(&g, 0, "Plant");
    assert_eq!(plant.len(), 1);
    assert_eq!((g.power(plant[0]), g.toughness(plant[0])), (0, 1));
    to_main1(&mut g, 5, 0);
    act(&mut g, Action::Loyalty(n, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(plant[0]).counters, 1);
}

#[test]
fn academy_rector_exiles_itself_to_find_an_enchantment() {
    let mut g = table();
    let r = put(&mut g, 0, "Academy Rector");
    let study = g.add_card(0, cid("Rhystic Study"), Zone::Library);
    let ig = hand(&mut g, 0, "Infernal Grasp");
    play(&mut g, ig, false, &[obj(r)], &[Action::Yes, Action::Pick(study)]);
    assert_eq!(g.obj(r).zone, Zone::Exile);
    assert_eq!(g.obj(study).zone, Zone::Battlefield);
}

#[test]
fn seizan_makes_each_player_lose_2_and_draw_2_on_their_upkeep() {
    let mut g = table();
    put(&mut g, 0, "Seizan, Perverter of Truth");
    let h1 = g.players[1].hand.len();
    to_main1(&mut g, 4, 1);
    assert_eq!(g.players[1].life, 38);
    assert_eq!(g.players[1].hand.len(), h1 + 3, "two and the draw step");
    assert_eq!(g.players[0].life, 40);
    to_main1(&mut g, 5, 0);
    assert_eq!(g.players[0].life, 38);
}

fn tutor_to_top(name: &str, good: &str, bad: &str) {
    let mut g = table();
    let good = g.add_card(0, cid(good), Zone::Library);
    let bad = g.add_card(0, cid(bad), Zone::Library);
    let t = hand(&mut g, 0, name);
    act(&mut g, Action::Cast(t));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(!g.legal_actions().contains(&Action::Pick(bad)), "{name}");
    resolve_all(&mut g, &[Action::Pick(good)]);
    assert_eq!(g.players[0].library.last(), Some(&good), "{name}: on top");
}

#[test]
fn mystical_tutor_puts_an_instant_or_sorcery_on_top() {
    tutor_to_top("Mystical Tutor", "Armageddon", "Sol Ring");
}

#[test]
fn enlightened_tutor_puts_an_artifact_or_enchantment_on_top() {
    tutor_to_top("Enlightened Tutor", "Rhystic Study", "Counterspell");
}

#[test]
fn unmarked_grave_entombs_a_nonlegendary_card() {
    let mut g = table();
    let good = g.add_card(0, cid("Ashen Rider"), Zone::Library);
    let bad = g.add_card(0, cid("Razaketh, the Foulblooded"), Zone::Library);
    let u = hand(&mut g, 0, "Unmarked Grave");
    act(&mut g, Action::Cast(u));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(!g.legal_actions().contains(&Action::Pick(bad)), "legendary");
    resolve_all(&mut g, &[Action::Pick(good)]);
    assert_eq!(g.obj(good).zone, Zone::Graveyard);
}

#[test]
fn imperial_recruiter_finds_a_creature_with_power_2_or_less() {
    let mut g = table();
    let good = g.add_card(0, cid("Merfolk Looter"), Zone::Library);
    let bad = g.add_card(0, cid("Consecrated Sphinx"), Zone::Library);
    let r = hand(&mut g, 0, "Imperial Recruiter");
    act(&mut g, Action::Cast(r));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(!g.legal_actions().contains(&Action::Pick(bad)), "power 4");
    resolve_all(&mut g, &[Action::Pick(good)]);
    assert_eq!(g.obj(good).zone, Zone::Hand);
}

#[test]
fn avacyn_makes_your_other_permanents_indestructible() {
    let mut g = table();
    let a = put(&mut g, 0, "Avacyn, Angel of Hope");
    let looter = put(&mut g, 0, "Merfolk Looter");
    let ring = put(&mut g, 0, "Sol Ring");
    let ig = hand(&mut g, 0, "Infernal Grasp");
    play(&mut g, ig, false, &[obj(looter)], &[]);
    assert_eq!(g.obj(looter).zone, Zone::Battlefield);
    assert!(g.keywords(ring).has(Keywords::INDESTRUCTIBLE));
    assert!(g.keywords(a).has(Keywords::INDESTRUCTIBLE), "printed");
    let theirs = put(&mut g, 1, "Merfolk Looter");
    assert!(!g.keywords(theirs).has(Keywords::INDESTRUCTIBLE));
}

#[test]
fn elesh_norn_pumps_yours_and_shrinks_theirs() {
    let mut g = table();
    let n = put(&mut g, 0, "Elesh Norn, Grand Cenobite");
    let mine = put(&mut g, 0, "Thought Courier");
    let theirs = put(&mut g, 1, "Consecrated Sphinx");
    let small = put(&mut g, 1, "Merfolk Looter");
    assert_eq!((g.power(mine), g.toughness(mine)), (3, 3));
    assert_eq!((g.power(theirs), g.toughness(theirs)), (2, 4));
    assert_eq!((g.power(n), g.toughness(n)), (4, 7), "other");
    assert_eq!(g.toughness(small), -1, "a 1/1 of theirs is -1/-1");
}

#[test]
fn atraxa_proliferates_at_your_end_step() {
    let mut g = table();
    put(&mut g, 0, "Atraxa, Praetors' Voice");
    let t = put(&mut g, 0, "Gavony Township");
    let looter = put(&mut g, 0, "Merfolk Looter");
    act(&mut g, Action::Activate(t, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(looter).counters, 1);
    while g.step != Step::Cleanup && g.turn == 3 {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::Decide(_) => break,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
        if g.obj(looter).counters == 2 {
            break;
        }
    }
    assert_eq!(g.pending, Pending::Decide(0), "proliferate: choose");
    resolve_all(&mut g, &[Action::Pick(looter), Action::Yes]);
    assert_eq!(g.obj(looter).counters, 2);
}

#[test]
fn karns_bastion_proliferates() {
    let mut g = table();
    let b = put(&mut g, 0, "Karn's Bastion");
    assert!(g.mana_sources(0).iter().any(|&(x, m)| x == b && m.colorless));
    let t = put(&mut g, 0, "Gavony Township");
    let looter = put(&mut g, 0, "Merfolk Looter");
    act(&mut g, Action::Activate(t, 0));
    resolve_all(&mut g, &[]);
    act(&mut g, Action::Activate(b, 0));
    resolve_all(&mut g, &[Action::Pick(looter), Action::Yes]);
    assert_eq!(g.obj(looter).counters, 2);
}

// ---- Core theme d3, batch 2 ----

/// Attacks with `attacker` and plays the combat out to the second main phase; the opponent answers
/// any decision with its first legal action.
fn attack_to_main2(g: &mut Game, attacker: ObjId) {
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    act(g, Action::Attack(attacker));
    act(g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::Decide(1) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn wrath_of_god_destroys_all_creatures_through_regeneration() {
    let mut g = table();
    let mine = put(&mut g, 0, "Merfolk Looter");
    let theirs = put(&mut g, 1, "Consecrated Sphinx");
    let b = hand(&mut g, 0, "Blessing of Leeches");
    play(&mut g, b, false, &[obj(mine)], &[]);
    act(&mut g, Action::Activate(b, 0));
    resolve_all(&mut g, &[]);
    let w = hand(&mut g, 0, "Wrath of God");
    play(&mut g, w, false, &[], &[]);
    assert_eq!(
        (g.obj(mine).zone, g.obj(theirs).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
}

#[test]
fn sword_of_feast_and_famine_protects_and_untaps_your_lands_on_a_hit() {
    let mut g = table();
    let sword = put(&mut g, 0, "Sword of Feast and Famine");
    let looter = put(&mut g, 0, "Merfolk Looter");
    to_main1(&mut g, 5, 0);
    act(&mut g, Action::Activate(sword, 0));
    act(&mut g, Action::Target(obj(looter)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(looter), g.toughness(looter)), (3, 3));
    assert_eq!(g.protection(looter), [false, false, true, false, true]);
    assert!(
        g.permanents(0).any(|x| g.types_of(x).land && g.obj(x).tapped),
        "equip tapped lands"
    );
    let cards = g.players[1].hand.len();
    attack_to_main2(&mut g, looter);
    assert_eq!(g.players[1].life, 37);
    assert_eq!(g.players[1].hand.len(), cards - 1, "discarded");
    assert!(
        !g.permanents(0).any(|x| g.types_of(x).land && g.obj(x).tapped),
        "lands untapped"
    );
}

#[test]
fn regal_force_draws_for_each_green_creature() {
    let mut g = table();
    put(&mut g, 0, "Llanowar Elves");
    put(&mut g, 0, "Merfolk Looter");
    let f = hand(&mut g, 0, "Regal Force");
    let cards = g.players[0].hand.len();
    play(&mut g, f, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), cards - 1 + 2, "the Elves and the Force");
}

#[test]
fn loxodon_hierarch_gains_4_and_regenerates_your_creatures() {
    let mut g = table();
    let looter = put(&mut g, 0, "Merfolk Looter");
    let h = hand(&mut g, 0, "Loxodon Hierarch");
    play(&mut g, h, false, &[], &[]);
    assert_eq!(g.players[0].life, 44);
    act(&mut g, Action::Activate(h, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(h).zone, Zone::Graveyard, "sacrificed");
    let ig = hand(&mut g, 0, "Infernal Grasp");
    play(&mut g, ig, false, &[obj(looter)], &[]);
    assert_eq!(g.obj(looter).zone, Zone::Battlefield);
    assert!(g.obj(looter).tapped);
}

#[test]
fn blessing_of_leeches_costs_a_life_each_upkeep_and_regenerates() {
    let mut g = table();
    let looter = put(&mut g, 0, "Merfolk Looter");
    let b = hand(&mut g, 0, "Blessing of Leeches");
    play(&mut g, b, false, &[obj(looter)], &[]);
    act(&mut g, Action::Activate(b, 0));
    resolve_all(&mut g, &[]);
    let ig = hand(&mut g, 0, "Infernal Grasp");
    play(&mut g, ig, false, &[obj(looter)], &[]);
    assert_eq!(g.obj(looter).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, 38, "Infernal Grasp's 2");
    to_main1(&mut g, 5, 0);
    assert_eq!(g.players[0].life, 37);
}

#[test]
fn emissary_of_despair_drains_for_each_of_their_artifacts() {
    let mut g = table();
    let e = put(&mut g, 0, "Emissary of Despair");
    put(&mut g, 1, "Sol Ring");
    put(&mut g, 1, "Solemn Simulacrum");
    g.start_turn(5, 0);
    attack_to_main2(&mut g, e);
    assert_eq!(g.players[1].life, 40 - 2 - 2);
}

#[test]
fn return_of_the_wildspeaker_draws_or_pumps_non_humans() {
    let mut g = table();
    let sphinx = put(&mut g, 0, "Consecrated Sphinx");
    let human = put(&mut g, 0, "Eternal Witness");
    let r = hand(&mut g, 0, "Return of the Wildspeaker");
    let cards = g.players[0].hand.len();
    act(&mut g, Action::CastModal(r, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), cards - 1 + 4);
    let r = hand(&mut g, 0, "Return of the Wildspeaker");
    act(&mut g, Action::CastModal(r, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(sphinx), 7);
    assert_eq!(g.power(human), 2, "a Human");
}

#[test]
fn last_march_of_the_ents_draws_greatest_toughness_then_puts_creatures() {
    let mut g = table();
    put(&mut g, 0, "Consecrated Sphinx"); // toughness 6
    let lm = hand(&mut g, 0, "Last March of the Ents");
    assert!(g.def_of(lm).uncounterable);
    let a = hand(&mut g, 0, "Merfolk Looter");
    let b = hand(&mut g, 0, "Llanowar Elves");
    let cards = g.players[0].hand.len();
    play(&mut g, lm, false, &[], &[Action::Pick(a), Action::Pick(b)]);
    assert_eq!(
        (g.obj(a).zone, g.obj(b).zone),
        (Zone::Battlefield, Zone::Battlefield)
    );
    assert_eq!(g.players[0].hand.len(), cards - 1 + 6 - 2);
}

#[test]
fn tooth_and_nail_entwined_finds_two_and_puts_two() {
    let mut g = table();
    let a = g.add_card(0, cid("Consecrated Sphinx"), Zone::Library);
    let b = g.add_card(0, cid("Craterhoof Behemoth"), Zone::Library);
    let t = hand(&mut g, 0, "Tooth and Nail");
    act(&mut g, Action::CastModal(t, 2)); // entwined: {5}{G}{G} and {2}
    resolve_all(
        &mut g,
        &[Action::Pick(a), Action::Pick(b), Action::Pick(a), Action::Pick(b)],
    );
    assert_eq!(
        (g.obj(a).zone, g.obj(b).zone),
        (Zone::Battlefield, Zone::Battlefield)
    );
}

#[test]
fn tooth_and_nail_puts_up_to_two_from_hand() {
    let mut g = table();
    let a = hand(&mut g, 0, "Consecrated Sphinx");
    let b = hand(&mut g, 0, "Craterhoof Behemoth");
    let c = hand(&mut g, 0, "Merfolk Looter");
    let t = hand(&mut g, 0, "Tooth and Nail");
    act(&mut g, Action::CastModal(t, 1));
    resolve_all(&mut g, &[Action::Pick(a), Action::Pick(b)]);
    assert_eq!(
        (g.obj(a).zone, g.obj(b).zone, g.obj(c).zone),
        (Zone::Battlefield, Zone::Battlefield, Zone::Hand)
    );
}

#[test]
fn ponder_reorders_the_top_three_may_shuffle_and_draws() {
    let mut g = table();
    let a = g.add_card(0, cid("Sol Ring"), Zone::Library);
    let b = g.add_card(0, cid("Counterspell"), Zone::Library);
    let c = g.add_card(0, cid("Merfolk Looter"), Zone::Library);
    let p = hand(&mut g, 0, "Ponder");
    play(
        &mut g,
        p,
        false,
        &[],
        &[Action::Pick(a), Action::Pick(c), Action::No],
    );
    assert_eq!(g.obj(a).zone, Zone::Hand, "the card put on top is drawn");
    let lib = &g.players[0].library;
    assert_eq!(&lib[lib.len() - 2..], &[b, c]);
}

#[test]
fn dream_stalker_returns_a_permanent_even_a_land() {
    let mut g = table();
    let land = g.permanents(0).find(|&x| g.types_of(x).land).unwrap();
    let d = hand(&mut g, 0, "Dream Stalker");
    act(&mut g, Action::Cast(d));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(g.legal_actions().contains(&Action::Pick(land)));
    assert!(g.legal_actions().contains(&Action::Pick(d)), "itself");
    resolve_all(&mut g, &[Action::Pick(land)]);
    assert_eq!(g.obj(land).zone, Zone::Hand);
}

#[test]
fn ancestral_statue_returns_a_nonland_permanent() {
    let mut g = table();
    let ring = put(&mut g, 0, "Sol Ring");
    let land = g.permanents(0).find(|&x| g.types_of(x).land).unwrap();
    let s = hand(&mut g, 0, "Ancestral Statue");
    act(&mut g, Action::Cast(s));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(!g.legal_actions().contains(&Action::Pick(land)));
    resolve_all(&mut g, &[Action::Pick(ring)]);
    assert_eq!(g.obj(ring).zone, Zone::Hand);
    assert_eq!(g.obj(s).zone, Zone::Battlefield);
}

#[test]
fn weird_harvest_lets_each_player_search_for_x_creatures() {
    let mut g = table();
    let mine = g.add_card(0, cid("Consecrated Sphinx"), Zone::Library);
    let theirs = g.add_card(1, cid("Merfolk Looter"), Zone::Library);
    let w = hand(&mut g, 0, "Weird Harvest");
    act(&mut g, Action::CastX(w, 1));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.pending, Pending::Decide(0));
    act(&mut g, Action::Pick(mine));
    assert_eq!(g.pending, Pending::Decide(1), "then the opponent");
    resolve_all(&mut g, &[Action::Pick(theirs)]);
    assert_eq!((g.obj(mine).zone, g.obj(theirs).zone), (Zone::Hand, Zone::Hand));
}

#[test]
fn relentless_assault_untaps_attackers_and_adds_a_combat() {
    let mut g = table();
    let looter = put(&mut g, 0, "Merfolk Looter");
    let idle = put(&mut g, 0, "Consecrated Sphinx");
    g.start_turn(5, 0);
    attack_to_main2(&mut g, looter);
    assert!(g.obj(looter).tapped);
    g.objects[idle.0 as usize].tapped = true;
    let r = hand(&mut g, 0, "Relentless Assault");
    play(&mut g, r, false, &[], &[]);
    assert!(!g.obj(looter).tapped, "it attacked");
    assert!(g.obj(idle).tapped, "it didn't");
    attack_to_main2(&mut g, looter);
    assert_eq!(g.players[1].life, 38, "two combats");
}

#[test]
fn cyberdrive_awakener_animates_your_artifacts_and_gives_flying() {
    let mut g = table();
    let ring = put(&mut g, 0, "Sol Ring");
    let golem = put(&mut g, 0, "Solemn Simulacrum");
    let looter = put(&mut g, 0, "Merfolk Looter");
    let c = hand(&mut g, 0, "Cyberdrive Awakener");
    play(&mut g, c, false, &[], &[]);
    assert!(g.types_of(ring).creature);
    assert_eq!((g.power(ring), g.toughness(ring)), (4, 4));
    assert!(g.keywords(ring).has(Keywords::FLYING) && g.keywords(golem).has(Keywords::FLYING));
    assert!(!g.keywords(looter).has(Keywords::FLYING));
    to_main1(&mut g, 5, 0);
    assert!(!g.types_of(ring).creature, "until end of turn");
}

#[test]
fn oriq_loremage_entombs_and_grows_on_an_instant_or_sorcery() {
    let mut g = table();
    let o = put(&mut g, 0, "Oriq Loremage");
    to_main1(&mut g, 5, 0);
    let spell = g.add_card(0, cid("Counterspell"), Zone::Library);
    act(&mut g, Action::Activate(o, 0));
    resolve_all(&mut g, &[Action::Pick(spell)]);
    assert_eq!(g.obj(spell).zone, Zone::Graveyard);
    assert_eq!(g.obj(o).counters, 1);
    to_main1(&mut g, 7, 0);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    act(&mut g, Action::Activate(o, 0));
    resolve_all(&mut g, &[Action::Pick(ring)]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    assert_eq!(g.obj(o).counters, 1, "not for an artifact");
}

// ---- Core theme d3, batch 3: target filters and scaled statics ----

fn can_target(g: &Game, x: ObjId) -> bool {
    g.legal_actions().contains(&Action::Target(obj(x)))
}

#[test]
fn go_for_the_throat_kills_only_a_nonartifact_creature() {
    let mut g = table();
    let looter = put(&mut g, 1, "Merfolk Looter");
    let golem = put(&mut g, 1, "Solemn Simulacrum");
    let gft = hand(&mut g, 0, "Go for the Throat");
    act(&mut g, Action::Cast(gft));
    assert!(!can_target(&g, golem), "an artifact creature");
    act(&mut g, Action::Target(obj(looter)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(looter).zone, Zone::Graveyard);
}

#[test]
fn doom_blade_kills_only_a_nonblack_creature() {
    let mut g = table();
    let looter = put(&mut g, 1, "Merfolk Looter");
    let black = put(&mut g, 1, "Emissary of Despair");
    let db = hand(&mut g, 0, "Doom Blade");
    act(&mut g, Action::Cast(db));
    assert!(!can_target(&g, black));
    act(&mut g, Action::Target(obj(looter)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(looter).zone, Zone::Graveyard);
}

#[test]
fn red_elemental_blast_counters_a_blue_spell_or_destroys_a_blue_permanent() {
    let mut g = table();
    let elves = put(&mut g, 1, "Llanowar Elves");
    let theirs = put(&mut g, 1, "Merfolk Looter");
    let reb = hand(&mut g, 0, "Red Elemental Blast");
    act(&mut g, Action::CastModal(reb, 1));
    assert!(!can_target(&g, elves), "green");
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);

    let spell = hand(&mut g, 0, "Merfolk Looter");
    act(&mut g, Action::Cast(spell));
    let reb = hand(&mut g, 0, "Red Elemental Blast");
    act(&mut g, Action::CastModal(reb, 0));
    act(&mut g, Action::Target(obj(spell)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(spell).zone, Zone::Graveyard, "countered");
}

#[test]
fn pyroblast_targets_anything_but_hits_only_blue() {
    let mut g = table();
    let elves = hand(&mut g, 0, "Llanowar Elves");
    act(&mut g, Action::Cast(elves));
    let p = hand(&mut g, 0, "Pyroblast");
    act(&mut g, Action::CastModal(p, 0));
    act(&mut g, Action::Target(obj(elves)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield, "not blue: it resolves");
    let looter = put(&mut g, 1, "Merfolk Looter");
    let p = hand(&mut g, 0, "Pyroblast");
    act(&mut g, Action::CastModal(p, 1));
    assert!(can_target(&g, elves), "any permanent");
    act(&mut g, Action::Target(obj(looter)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(looter).zone, Zone::Graveyard);
}

#[test]
fn sultai_charm_kills_a_monocolored_creature_or_loots_two() {
    let mut g = table();
    let looter = put(&mut g, 1, "Merfolk Looter");
    let atraxa = put(&mut g, 1, "Atraxa, Praetors' Voice");
    let c = hand(&mut g, 0, "Sultai Charm");
    act(&mut g, Action::CastModal(c, 0));
    assert!(!can_target(&g, atraxa), "four colors");
    act(&mut g, Action::Target(obj(looter)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(looter).zone, Zone::Graveyard);
    let c = hand(&mut g, 0, "Sultai Charm");
    let junk = hand(&mut g, 0, "Sol Ring");
    let cards = g.players[0].hand.len();
    act(&mut g, Action::CastModal(c, 2));
    resolve_all(&mut g, &[Action::Discard(junk)]);
    assert_eq!(g.players[0].hand.len(), cards - 1 + 2 - 1);
    assert_eq!(g.obj(junk).zone, Zone::Graveyard);
}

#[test]
fn disdainful_stroke_counters_mana_value_4_or_more() {
    let mut g = table();
    let sphinx = hand(&mut g, 0, "Consecrated Sphinx");
    act(&mut g, Action::Cast(sphinx));
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    let ds = hand(&mut g, 0, "Disdainful Stroke");
    act(&mut g, Action::Cast(ds));
    assert!(!can_target(&g, bolt), "mana value 1");
    act(&mut g, Action::Target(obj(sphinx)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(sphinx).zone, Zone::Graveyard);
    assert_eq!(g.players[1].life, 37, "the Bolt resolved");
}

#[test]
fn strix_serenade_counters_and_gives_its_controller_a_bird() {
    let mut g = table();
    let ring = hand(&mut g, 0, "Sol Ring");
    act(&mut g, Action::Cast(ring));
    let s = hand(&mut g, 0, "Strix Serenade");
    act(&mut g, Action::Cast(s));
    act(&mut g, Action::Target(obj(ring)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    let bird = tokens(&g, 0, "Bird");
    assert_eq!(bird.len(), 1);
    assert_eq!((g.power(bird[0]), g.toughness(bird[0])), (2, 2));
    assert!(g.keywords(bird[0]).has(Keywords::FLYING));
}

#[test]
fn siren_stormtamer_counters_a_spell_targeting_your_creature() {
    let mut g = table();
    let st = put(&mut g, 0, "Siren Stormtamer");
    let looter = put(&mut g, 0, "Merfolk Looter");
    let theirs = put(&mut g, 1, "Merfolk Looter");
    let ig = hand(&mut g, 0, "Infernal Grasp");
    act(&mut g, Action::Cast(ig));
    act(&mut g, Action::Target(obj(theirs)));
    let ig2 = hand(&mut g, 0, "Infernal Grasp");
    act(&mut g, Action::Cast(ig2));
    act(&mut g, Action::Target(obj(looter)));
    act(&mut g, Action::Activate(st, 0));
    assert!(!can_target(&g, ig), "it targets their creature");
    act(&mut g, Action::Target(obj(ig2)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(st).zone, Zone::Graveyard, "sacrificed");
    assert_eq!(g.obj(looter).zone, Zone::Battlefield);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
}

#[test]
fn cranial_plating_counts_your_artifacts_and_moves_for_bb() {
    let mut g = table();
    let plating = put(&mut g, 0, "Cranial Plating");
    let looter = put(&mut g, 0, "Merfolk Looter");
    let elves = put(&mut g, 0, "Llanowar Elves");
    act(&mut g, Action::Activate(plating, 1)); // equip {1}
    act(&mut g, Action::Target(obj(looter)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(looter), g.toughness(looter)), (2, 1));
    put(&mut g, 0, "Sol Ring");
    put(&mut g, 1, "Sol Ring");
    assert_eq!(g.power(looter), 3, "your two artifacts");
    act(&mut g, Action::Activate(plating, 0)); // {B}{B}: attach
    act(&mut g, Action::Target(obj(elves)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(looter), g.power(elves)), (1, 3));
}

#[test]
fn ancestral_mask_grows_for_each_other_enchantment() {
    let mut g = table();
    let looter = put(&mut g, 0, "Merfolk Looter");
    put(&mut g, 0, "Rhystic Study");
    put(&mut g, 1, "Knowledge Is Power"); // no trigger on your spells
    let m = hand(&mut g, 0, "Ancestral Mask");
    play(&mut g, m, false, &[obj(looter)], &[]);
    assert_eq!((g.power(looter), g.toughness(looter)), (5, 5));
}

#[test]
fn knowledge_is_power_pumps_by_cards_drawn_this_turn() {
    let mut g = table();
    put(&mut g, 0, "Knowledge Is Power");
    let looter = put(&mut g, 0, "Merfolk Looter");
    let theirs = put(&mut g, 1, "Merfolk Looter");
    to_main1(&mut g, 5, 0);
    assert_eq!(
        (g.power(looter), g.toughness(looter)),
        (2, 2),
        "the draw step's card"
    );
    assert_eq!(g.power(theirs), 1);
    let p = hand(&mut g, 0, "Ponder");
    let top3: Vec<ObjId> = g.players[0].library.iter().rev().take(2).copied().collect();
    play(
        &mut g,
        p,
        false,
        &[],
        &[Action::Pick(top3[0]), Action::Pick(top3[1]), Action::No],
    );
    assert_eq!(g.power(looter), 3);
}

#[test]
fn multani_is_as_big_as_all_hands() {
    let mut g = table();
    let m = put(&mut g, 0, "Multani, Maro-Sorcerer");
    let n = (g.players[0].hand.len() + g.players[1].hand.len()) as i32;
    assert_eq!((g.power(m), g.toughness(m)), (n, n));
    hand(&mut g, 1, "Sol Ring");
    assert_eq!(g.power(m), n + 1);
    assert!(g.keywords(m).has(Keywords::SHROUD));
}

#[test]
fn pathbreaker_ibex_pumps_by_the_greatest_power() {
    let mut g = table();
    let ibex = put(&mut g, 0, "Pathbreaker Ibex");
    let sphinx = put(&mut g, 0, "Consecrated Sphinx");
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(ibex));
    act(&mut g, Action::FinishAttacks);
    while !g.items.is_empty() {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!((g.power(ibex), g.toughness(ibex)), (7, 7));
    assert_eq!(g.power(sphinx), 8);
    assert!(g.keywords(sphinx).has(Keywords::TRAMPLE));
}

// ---- Core theme d3, batch 4 ----

#[test]
fn mutilate_shrinks_all_creatures_per_swamp() {
    let mut g = table();
    let looter = put(&mut g, 1, "Merfolk Looter");
    let sphinx = put(&mut g, 0, "Consecrated Sphinx");
    let m = hand(&mut g, 0, "Mutilate");
    play(&mut g, m, false, &[], &[]);
    assert_eq!(g.obj(looter).zone, Zone::Graveyard);
    assert_eq!((g.power(sphinx), g.toughness(sphinx)), (-1, 1), "five Swamps");
}

#[test]
fn defile_shrinks_one_creature_per_swamp() {
    let mut g = table();
    let sphinx = put(&mut g, 1, "Consecrated Sphinx");
    let d = hand(&mut g, 0, "Defile");
    play(&mut g, d, false, &[obj(sphinx)], &[]);
    assert_eq!((g.power(sphinx), g.toughness(sphinx)), (-1, 1));
}

#[test]
fn seeds_of_innocence_destroys_artifacts_and_refunds_their_mana_value() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let plating = put(&mut g, 0, "Cranial Plating");
    let s = hand(&mut g, 0, "Seeds of Innocence");
    play(&mut g, s, false, &[], &[]);
    assert_eq!(
        (g.obj(ring).zone, g.obj(plating).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
    assert_eq!((g.players[0].life, g.players[1].life), (42, 41));
}

#[test]
fn trophy_mage_finds_an_artifact_with_mana_value_3() {
    let mut g = table();
    let good = g.add_card(0, cid("Sword of Feast and Famine"), Zone::Library);
    let bad = g.add_card(0, cid("Sol Ring"), Zone::Library);
    let t = hand(&mut g, 0, "Trophy Mage");
    act(&mut g, Action::Cast(t));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Yes);
    assert!(!g.legal_actions().contains(&Action::Pick(bad)));
    resolve_all(&mut g, &[Action::Pick(good)]);
    assert_eq!(g.obj(good).zone, Zone::Hand);
}

#[test]
fn jirina_exiles_a_graveyard_and_protects_humans() {
    let mut g = table();
    let dead = g.add_card(1, cid("Merfolk Looter"), Zone::Graveyard);
    let human = put(&mut g, 0, "Eternal Witness");
    let j = hand(&mut g, 0, "Jirina, Dauntless General");
    act(&mut g, Action::Cast(j));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert_eq!(g.obj(dead).zone, Zone::Exile);
    act(&mut g, Action::Activate(j, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(j).zone, Zone::Graveyard);
    assert!(g.keywords(human).has(Keywords::HEXPROOF) && g.keywords(human).has(Keywords::INDESTRUCTIBLE));
}

#[test]
fn privileged_position_gives_your_other_permanents_hexproof() {
    let mut g = table();
    let p = put(&mut g, 0, "Privileged Position");
    let ring = put(&mut g, 0, "Sol Ring");
    let theirs = put(&mut g, 1, "Sol Ring");
    assert!(g.keywords(ring).has(Keywords::HEXPROOF));
    assert!(!g.keywords(p).has(Keywords::HEXPROOF), "other");
    assert!(!g.keywords(theirs).has(Keywords::HEXPROOF));
}

#[test]
fn sterling_grove_shrouds_enchantments_and_tutors_one_to_the_top() {
    let mut g = table();
    let grove = put(&mut g, 0, "Sterling Grove");
    let study = put(&mut g, 0, "Rhystic Study");
    assert!(g.keywords(study).has(Keywords::SHROUD));
    assert!(!g.keywords(grove).has(Keywords::SHROUD), "other");
    let ench = g.add_card(0, cid("Knowledge Is Power"), Zone::Library);
    act(&mut g, Action::Activate(grove, 0));
    resolve_all(&mut g, &[Action::Pick(ench)]);
    assert_eq!(g.obj(grove).zone, Zone::Graveyard);
    assert_eq!(g.players[0].library.last(), Some(&ench));
}

#[test]
fn farewell_exiles_the_chosen_sets() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let looter = put(&mut g, 1, "Merfolk Looter");
    let study = put(&mut g, 1, "Knowledge Is Power");
    let dead = g.add_card(1, cid("Llanowar Elves"), Zone::Graveyard);
    let f = hand(&mut g, 0, "Farewell");
    // Mode 0b1010: creatures and graveyards.
    act(&mut g, Action::CastModal(f, 0b1010 - 1));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(looter).zone, g.obj(dead).zone), (Zone::Exile, Zone::Exile));
    assert_eq!(
        (g.obj(ring).zone, g.obj(study).zone),
        (Zone::Battlefield, Zone::Battlefield)
    );
    assert_eq!(
        g.obj(f).zone,
        Zone::Graveyard,
        "on the stack as graveyards are exiled"
    );
}

#[test]
fn gravitational_shift_pumps_fliers_and_shrinks_walkers() {
    let mut g = table();
    put(&mut g, 0, "Gravitational Shift");
    let birds = put(&mut g, 1, "Birds of Paradise");
    let looter = put(&mut g, 0, "Merfolk Looter");
    assert_eq!(g.power(birds), 2);
    assert_eq!(g.power(looter), -1);
}

#[test]
fn shield_of_the_oversoul_by_the_creatures_color() {
    let mut g = table();
    let elves = put(&mut g, 0, "Llanowar Elves");
    let s = hand(&mut g, 0, "Shield of the Oversoul");
    play(&mut g, s, false, &[obj(elves)], &[]);
    assert_eq!((g.power(elves), g.toughness(elves)), (2, 2));
    assert!(g.keywords(elves).has(Keywords::INDESTRUCTIBLE));
    assert!(!g.keywords(elves).has(Keywords::FLYING), "not white");
}

#[test]
fn sword_of_the_paruns_follows_the_equipped_creatures_tapped_state() {
    let mut g = table();
    let sword = put(&mut g, 0, "Sword of the Paruns");
    let looter = put(&mut g, 0, "Merfolk Looter");
    let other = put(&mut g, 0, "Llanowar Elves");
    act(&mut g, Action::Activate(sword, 1)); // equip {3}
    act(&mut g, Action::Target(obj(looter)));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.toughness(looter), g.toughness(other)),
        (3, 3),
        "untapped: +0/+2"
    );
    act(&mut g, Action::Activate(sword, 0)); // {3}: tap it
    resolve_all(&mut g, &[]);
    assert!(g.obj(looter).tapped);
    assert_eq!(
        (g.power(looter), g.toughness(looter)),
        (3, 1),
        "tapped: +2/+0 to tapped ones"
    );
    assert_eq!((g.power(other), g.toughness(other)), (1, 1));
}

#[test]
fn inexorable_tide_proliferates_on_each_spell() {
    let mut g = table();
    put(&mut g, 0, "Inexorable Tide");
    let t = put(&mut g, 0, "Gavony Township");
    let looter = put(&mut g, 0, "Merfolk Looter");
    act(&mut g, Action::Activate(t, 0));
    resolve_all(&mut g, &[]);
    let ring = hand(&mut g, 0, "Sol Ring");
    play(&mut g, ring, false, &[], &[Action::Pick(looter), Action::Yes]);
    assert_eq!(g.obj(looter).counters, 2);
}

#[test]
fn volt_charge_deals_3_and_proliferates() {
    let mut g = table();
    let t = put(&mut g, 0, "Gavony Township");
    let looter = put(&mut g, 0, "Merfolk Looter");
    act(&mut g, Action::Activate(t, 0));
    resolve_all(&mut g, &[]);
    let v = hand(&mut g, 0, "Volt Charge");
    play(
        &mut g,
        v,
        false,
        &[Target::Player(1)],
        &[Action::Pick(looter), Action::Yes],
    );
    assert_eq!(g.players[1].life, 37);
    assert_eq!(g.obj(looter).counters, 2);
}

// ---- Core theme d3, batch 5 ----

/// Passes priority (declaring no attackers) until a decision comes up, at most `limit` steps.
fn pass_until_decision(g: &mut Game, limit: usize) {
    for _ in 0..limit {
        if matches!(g.pending, Pending::Decide(_)) {
            return;
        }
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
    panic!("no decision came up");
}

#[test]
fn cataclysmic_gearhulk_each_player_keeps_one_nonland_of_each_type() {
    let mut g = table();
    let ring = put(&mut g, 0, "Sol Ring");
    let stone = put(&mut g, 0, "Mind Stone");
    let elves = put(&mut g, 0, "Llanowar Elves");
    let their_ring = put(&mut g, 1, "Sol Ring");
    let study = put(&mut g, 1, "Underworld Dreams");
    let looter = put(&mut g, 1, "Merfolk Looter");
    let mystic = put(&mut g, 1, "Elvish Mystic");
    let gh = hand(&mut g, 0, "Cataclysmic Gearhulk");
    // You: the artifact (Sol Ring) and the creature (itself); the opponent: the creature (Looter);
    // their lone artifact and enchantment are chosen for them.
    play(
        &mut g,
        gh,
        false,
        &[],
        &[Action::Pick(ring), Action::Pick(gh), Action::Pick(looter)],
    );
    let zone = |x: ObjId| g.obj(x).zone;
    assert_eq!(
        (zone(ring), zone(gh), zone(stone), zone(elves)),
        (
            Zone::Battlefield,
            Zone::Battlefield,
            Zone::Graveyard,
            Zone::Graveyard
        )
    );
    assert_eq!(
        (zone(their_ring), zone(study), zone(looter), zone(mystic)),
        (
            Zone::Battlefield,
            Zone::Battlefield,
            Zone::Battlefield,
            Zone::Graveyard
        )
    );
    assert_eq!(
        g.permanents(0).filter(|&x| g.types_of(x).land).count(),
        25,
        "lands stay"
    );
}

#[test]
fn teferis_puzzle_box_puts_the_hand_on_the_bottom_and_draws_that_many() {
    let mut g = table();
    put(&mut g, 0, "Teferi's Puzzle Box");
    g.start_turn(5, 0);
    let mut old: Vec<ObjId> = Vec::new();
    while g.step != Step::Main1 {
        if matches!(g.pending, Pending::Decide(_)) {
            if old.is_empty() {
                old = g.players[0].hand.clone(); // after the draw step's draw
            }
            let a = g.legal_actions()[0];
            act(&mut g, a);
        } else {
            act(&mut g, Action::PassPriority);
        }
    }
    let n = old.len();
    assert!(n >= 2);
    assert_eq!(g.players[0].hand.len(), n, "drew that many");
    assert!(old.iter().all(|c| !g.players[0].hand.contains(c)));
    assert!(
        old.iter().all(|c| g.players[0].library[..n].contains(c)),
        "on the bottom"
    );
}

#[test]
fn deadly_brew_each_sacrifices_then_you_may_return_another_permanent_card() {
    let mut g = table();
    let elves = put(&mut g, 0, "Llanowar Elves");
    let looter = put(&mut g, 1, "Merfolk Looter");
    let mystic = put(&mut g, 1, "Elvish Mystic");
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Graveyard);
    let brew = hand(&mut g, 0, "Deadly Brew");
    act(&mut g, Action::Cast(brew));
    pass_until_decision(&mut g, 10);
    assert_eq!(
        g.pending,
        Pending::Decide(1),
        "your lone creature is chosen; the opponent picks"
    );
    act(&mut g, Action::Pick(mystic));
    assert!(!g.legal_actions().contains(&Action::Pick(elves)), "another card");
    resolve_all(&mut g, &[Action::Pick(ring)]);
    assert_eq!(
        (g.obj(elves).zone, g.obj(mystic).zone, g.obj(looter).zone),
        (Zone::Graveyard, Zone::Graveyard, Zone::Battlefield)
    );
    assert_eq!(g.obj(ring).zone, Zone::Hand);
}

#[test]
fn wave_of_vitriol_sacrifices_and_fetches_tapped_basics_per_land() {
    let mut g = table();
    let ring = put(&mut g, 0, "Sol Ring");
    let study = put(&mut g, 0, "Rhystic Study");
    let tomb = put(&mut g, 0, "Overgrown Tomb");
    let grave = put(&mut g, 1, "Watery Grave");
    let stone = put(&mut g, 1, "Mind Stone");
    let elves = put(&mut g, 0, "Llanowar Elves");
    let mine = g.players[0].library[0];
    let theirs = g.players[1].library[0];
    let w = hand(&mut g, 0, "Wave of Vitriol");
    play(&mut g, w, false, &[], &[Action::Pick(mine), Action::Pick(theirs)]);
    for x in [ring, study, tomb, grave, stone] {
        assert_eq!(g.obj(x).zone, Zone::Graveyard);
    }
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
    for x in [mine, theirs] {
        assert_eq!(g.obj(x).zone, Zone::Battlefield);
        assert!(g.obj(x).tapped);
    }
}

#[test]
fn signal_the_clans_puts_one_of_three_different_creatures_into_your_hand() {
    let mut g = table();
    let a = g.add_card(0, cid("Consecrated Sphinx"), Zone::Library);
    let b = g.add_card(0, cid("Craterhoof Behemoth"), Zone::Library);
    let c = g.add_card(0, cid("Merfolk Looter"), Zone::Library);
    let s = hand(&mut g, 0, "Signal the Clans");
    play(
        &mut g,
        s,
        false,
        &[],
        &[Action::Pick(a), Action::Pick(b), Action::Pick(c)],
    );
    let in_hand = [a, b, c].iter().filter(|&&x| g.obj(x).zone == Zone::Hand).count();
    let in_library = [a, b, c]
        .iter()
        .filter(|&&x| g.obj(x).zone == Zone::Library)
        .count();
    assert_eq!((in_hand, in_library), (1, 2));
}

#[test]
fn moonsilver_key_finds_a_mana_artifact_or_a_basic_land() {
    let mut g = table();
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    let study = g.add_card(0, cid("Rhystic Study"), Zone::Library);
    let ingot = g.add_card(0, cid("Ornithopter"), Zone::Library);
    let key = put(&mut g, 0, "Moonsilver Key");
    act(&mut g, Action::Activate(key, 0));
    pass_until_decision(&mut g, 10);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(ring)));
    assert!(
        legal.contains(&Action::Pick(g.players[0].library[0])),
        "a basic land"
    );
    assert!(!legal.contains(&Action::Pick(study)));
    assert!(
        !legal.contains(&Action::Pick(ingot)),
        "an artifact without a mana ability"
    );
    resolve_all(&mut g, &[Action::Pick(ring)]);
    assert_eq!((g.obj(ring).zone, g.obj(key).zone), (Zone::Hand, Zone::Graveyard));
}

#[test]
fn god_eternal_bontu_sacrifices_any_number_and_draws_that_many() {
    let mut g = table();
    let ring = put(&mut g, 0, "Sol Ring");
    let stone = put(&mut g, 0, "Mind Stone");
    let b = hand(&mut g, 0, "God-Eternal Bontu");
    let before = g.players[0].hand.len() - 1;
    play(
        &mut g,
        b,
        false,
        &[],
        &[Action::Pick(ring), Action::Pick(stone), Action::No],
    );
    assert_eq!(
        (g.obj(ring).zone, g.obj(stone).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
    assert_eq!(g.players[0].hand.len(), before + 2);
    let s = hand(&mut g, 0, "Swords to Plowshares");
    play(&mut g, s, false, &[obj(b)], &[Action::Yes]);
    let lib = &g.players[0].library;
    assert_eq!(lib[lib.len() - 3], b, "third from the top");
}

#[test]
fn imps_mischief_changes_the_target_and_costs_its_mana_value_in_life() {
    let mut g = table();
    let looter = put(&mut g, 1, "Merfolk Looter");
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    let imp = hand(&mut g, 0, "Imp's Mischief");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::Cast(imp));
    act(&mut g, Action::Target(obj(bolt)));
    pass_until_decision(&mut g, 10);
    assert!(!g.legal_actions().contains(&Action::No), "the target must change");
    assert!(
        !g.legal_actions().contains(&Action::Target(Target::Player(1))),
        "a new target"
    );
    resolve_all(&mut g, &[Action::Target(obj(looter))]);
    assert_eq!(g.obj(looter).zone, Zone::Graveyard);
    assert_eq!((g.players[0].life, g.players[1].life), (39, 40));
}

#[test]
fn boseiju_destroys_a_nonbasic_land_and_its_controller_may_fetch_a_typed_land() {
    let mut g = table();
    let grave = put(&mut g, 1, "Watery Grave");
    let tomb = g.add_card(1, cid("Overgrown Tomb"), Zone::Library);
    let b = hand(&mut g, 0, "Boseiju, Who Endures");
    act(&mut g, Action::Activate(b, 0));
    act(&mut g, Action::Target(obj(grave)));
    resolve_all(&mut g, &[Action::Pick(tomb), Action::Yes]); // Yes: pay 2 life, untapped
    assert_eq!(
        (g.obj(grave).zone, g.obj(b).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
    assert_eq!(g.obj(tomb).zone, Zone::Battlefield);
    assert!(!g.obj(tomb).tapped);
}

#[test]
fn cindervines_destroys_and_damages_that_permanents_controller() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let c = put(&mut g, 0, "Cindervines");
    act(&mut g, Action::Activate(c, 0));
    act(&mut g, Action::Target(obj(ring)));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.obj(ring).zone, g.obj(c).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
    assert_eq!(g.players[1].life, 38);
}

#[test]
fn braids_sacrifice_then_the_opponent_shares_a_type_or_pays() {
    let mut g = table();
    put(&mut g, 0, "Braids, Arisen Nightmare");
    let ring = put(&mut g, 0, "Sol Ring");
    let stone = put(&mut g, 1, "Mind Stone");
    pass_until_decision(&mut g, 40);
    assert!(g.legal_actions().contains(&Action::No), "you may");
    act(&mut g, Action::Pick(ring));
    assert_eq!(g.pending, Pending::Decide(1));
    let hand_before = g.players[0].hand.len();
    act(&mut g, Action::No);
    assert_eq!(g.players[1].life, 38);
    assert_eq!(g.players[0].hand.len(), hand_before + 1);
    assert_eq!(
        (g.obj(ring).zone, g.obj(stone).zone),
        (Zone::Graveyard, Zone::Battlefield)
    );
}

#[test]
fn spellskite_redirects_a_target_to_itself() {
    let mut g = table();
    let skite = put(&mut g, 0, "Spellskite");
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::Activate(skite, 0));
    act(&mut g, Action::Target(obj(bolt)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 40);
    assert_eq!(g.obj(skite).damage, 3);
}

#[test]
fn defense_of_the_heart_fetches_two_creatures_when_they_have_three() {
    let mut g = table();
    let d = put(&mut g, 0, "Defense of the Heart");
    for name in ["Merfolk Looter", "Llanowar Elves", "Elvish Mystic"] {
        put(&mut g, 1, name);
    }
    let a = g.add_card(0, cid("Consecrated Sphinx"), Zone::Library);
    let b = g.add_card(0, cid("Craterhoof Behemoth"), Zone::Library);
    g.start_turn(5, 0);
    pass_until_decision(&mut g, 10);
    act(&mut g, Action::Pick(a));
    act(&mut g, Action::Pick(b));
    assert_eq!(g.obj(d).zone, Zone::Graveyard);
    assert_eq!(
        (g.obj(a).zone, g.obj(b).zone),
        (Zone::Battlefield, Zone::Battlefield)
    );
}

#[test]
fn gift_of_estates_searches_only_if_they_have_more_lands() {
    let mut g = table();
    let gift = hand(&mut g, 0, "Gift of Estates");
    let before = g.players[0].hand.len() - 1;
    play(&mut g, gift, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), before, "equal lands: nothing");

    let mut g = game(&["Plains", "Plains"]);
    let gift = hand(&mut g, 0, "Gift of Estates");
    let before = g.players[0].hand.len() - 1;
    let lib: Vec<ObjId> = g.players[0].library[..3].to_vec();
    play(
        &mut g,
        gift,
        false,
        &[],
        &[Action::Pick(lib[0]), Action::Pick(lib[1]), Action::Pick(lib[2])],
    );
    assert_eq!(g.players[0].hand.len(), before + 3);
}

#[test]
fn masked_vandal_is_every_creature_type_and_exiles_for_a_creature_card() {
    let mut g = table();
    let study = put(&mut g, 1, "Underworld Dreams");
    let looter = g.add_card(0, cid("Merfolk Looter"), Zone::Graveyard);
    let v = hand(&mut g, 0, "Masked Vandal");
    play(
        &mut g,
        v,
        false,
        &[],
        &[Action::Target(obj(study)), Action::Pick(looter)],
    );
    assert_eq!(
        (g.obj(study).zone, g.obj(looter).zone),
        (Zone::Exile, Zone::Exile)
    );
    assert!(g.has_subtype(v, "Elf") && g.has_subtype(v, "Wizard"));
}

// ---- Core theme d3, batch 6 ----

#[test]
fn genji_glove_gives_double_strike_and_untaps_for_an_extra_combat() {
    let mut g = table();
    let glove = put(&mut g, 0, "Genji Glove");
    let looter = put(&mut g, 0, "Merfolk Looter");
    g.objects[looter.0 as usize].since_turn = 0;
    act(&mut g, Action::Activate(glove, 0)); // equip {3}
    act(&mut g, Action::Target(obj(looter)));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(looter).has(Keywords::DOUBLE_STRIKE));
    while !g.legal_actions().contains(&Action::Attack(looter)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(looter));
    act(&mut g, Action::FinishAttacks);
    while !g.items.is_empty() || g.obj(looter).tapped {
        act(&mut g, Action::PassPriority);
    }
    assert!(!g.obj(looter).tapped, "untapped by the trigger");
    // Through the first combat's damage (double strike: 2) and into the extra combat.
    let mut attacked_again = false;
    for _ in 0..40 {
        let legal = g.legal_actions();
        if legal.contains(&Action::Attack(looter)) {
            attacked_again = true;
            break;
        }
        act(
            &mut g,
            if legal.contains(&Action::PassPriority) {
                Action::PassPriority
            } else {
                legal[0]
            },
        );
    }
    assert!(attacked_again, "an additional combat phase");
    assert_eq!(g.players[1].life, 38);
}

#[test]
fn rise_and_shine_animates_one_or_overloaded_each_noncreature_artifact() {
    let mut g = table();
    let ring = put(&mut g, 0, "Sol Ring");
    let stone = put(&mut g, 0, "Mind Stone");
    let r = hand(&mut g, 0, "Rise and Shine");
    play(&mut g, r, false, &[obj(ring)], &[]);
    assert!(g.types_of(ring).creature);
    assert_eq!((g.power(ring), g.toughness(ring)), (4, 4));
    assert!(!g.types_of(stone).creature);

    let mut g = table();
    let ring = put(&mut g, 0, "Sol Ring");
    let stone = put(&mut g, 0, "Mind Stone");
    let theirs = put(&mut g, 1, "Sol Ring");
    let r = hand(&mut g, 0, "Rise and Shine");
    play(&mut g, r, true, &[], &[]); // overloaded
    for x in [ring, stone] {
        assert_eq!((g.power(x), g.toughness(x)), (4, 4));
    }
    assert!(!g.types_of(theirs).creature, "only yours");
}

// ---- Core theme d3, batch 7 ----

#[test]
fn winds_of_abandon_exiles_one_or_overloaded_each_for_tapped_basics() {
    let mut g = table();
    let looter = put(&mut g, 1, "Merfolk Looter");
    let elves = put(&mut g, 1, "Llanowar Elves");
    let mine = put(&mut g, 0, "Elvish Mystic");
    let basic = g.players[1].library[0];
    let w = hand(&mut g, 0, "Winds of Abandon");
    play(&mut g, w, false, &[obj(looter)], &[Action::Pick(basic)]);
    assert_eq!(
        (g.obj(looter).zone, g.obj(elves).zone),
        (Zone::Exile, Zone::Battlefield)
    );
    assert!(g.obj(basic).tapped && g.obj(basic).zone == Zone::Battlefield);

    let more = put(&mut g, 1, "Merfolk Looter");
    let (a, b) = (g.players[1].library[0], g.players[1].library[1]);
    let w = hand(&mut g, 0, "Winds of Abandon");
    play(&mut g, w, true, &[], &[Action::Pick(a), Action::Pick(b)]); // overloaded
    assert_eq!(
        (g.obj(elves).zone, g.obj(more).zone, g.obj(mine).zone),
        (Zone::Exile, Zone::Exile, Zone::Battlefield)
    );
    assert!([a, b]
        .iter()
        .all(|&x| g.obj(x).zone == Zone::Battlefield && g.obj(x).tapped));
}

#[test]
fn tainted_pact_digs_until_you_take_one_or_a_name_repeats() {
    let mut g = table();
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    let study = g.add_card(0, cid("Rhystic Study"), Zone::Library);
    let p = hand(&mut g, 0, "Tainted Pact");
    play(&mut g, p, false, &[], &[Action::No, Action::Yes]);
    assert_eq!((g.obj(study).zone, g.obj(ring).zone), (Zone::Exile, Zone::Hand));

    // Plains, then Plains: the repeat stops it.
    let p = hand(&mut g, 0, "Tainted Pact");
    let top = *g.players[0].library.last().unwrap();
    play(&mut g, p, false, &[], &[Action::No]);
    assert_eq!(g.obj(top).zone, Zone::Exile);
    assert_eq!(g.players[0].graveyard.len(), 2, "both Pacts; nothing more asked");
}

#[test]
fn demonlord_belzenlok_repeats_on_mana_value_four_or_more() {
    let mut g = table();
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    g.add_card(0, cid("Plains"), Zone::Library);
    let sphinx = g.add_card(0, cid("Consecrated Sphinx"), Zone::Library);
    let b = hand(&mut g, 0, "Demonlord Belzenlok");
    play(&mut g, b, false, &[], &[]);
    assert_eq!((g.obj(sphinx).zone, g.obj(ring).zone), (Zone::Hand, Zone::Hand));
    assert_eq!(g.players[0].life, 38, "two cards into your hand");
}

// ---- Core theme d3, batch 8 ----

#[test]
fn benevolent_bodyguard_protects_from_the_chosen_color() {
    let mut g = table();
    let b = put(&mut g, 0, "Benevolent Bodyguard");
    let elves = put(&mut g, 0, "Llanowar Elves");
    act(&mut g, Action::Activate(b, 0));
    act(&mut g, Action::Target(obj(elves)));
    resolve_all(&mut g, &[Action::Mode(3)]); // red
    assert_eq!(g.protection(elves), [false, false, false, true, false]);
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    assert!(!g.legal_actions().contains(&Action::Target(obj(elves))));
}

#[test]
fn cartel_aristocrat_protects_itself() {
    let mut g = table();
    let a = put(&mut g, 0, "Cartel Aristocrat");
    let looter = put(&mut g, 0, "Merfolk Looter");
    act(&mut g, Action::ActivateWith(a, 0, looter));
    resolve_all(&mut g, &[Action::Mode(2)]); // black
    assert_eq!(g.obj(looter).zone, Zone::Graveyard);
    assert!(g.protection(a)[2]);
}

#[test]
fn akromas_will_protection_from_each_color() {
    let mut g = table();
    let elves = put(&mut g, 0, "Llanowar Elves");
    let w = hand(&mut g, 0, "Akroma's Will");
    act(&mut g, Action::CastModal(w, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.protection(elves), [true; 5]);
    assert!(g.keywords(elves).has(Keywords::INDESTRUCTIBLE) && g.keywords(elves).has(Keywords::LIFELINK));
    assert!(!g.keywords(elves).has(Keywords::FLYING), "one mode");
}

#[test]
fn serras_emissary_protects_you_and_your_creatures_from_the_chosen_type() {
    let mut g = table();
    let elves = put(&mut g, 0, "Llanowar Elves");
    let e = hand(&mut g, 0, "Serra's Emissary");
    play(&mut g, e, false, &[], &[Action::Mode(3)]); // instant
    assert_eq!(g.obj(e).chosen_card_type, Some(3));
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    let legal = g.legal_actions();
    for t in [Target::Player(0), obj(e), obj(elves)] {
        assert!(!legal.contains(&Action::Target(t)), "{t:?}");
    }
    assert!(legal.contains(&Action::Target(Target::Player(1))));
}

// ---- Core theme d3, batch 9 ----

#[test]
fn return_to_dust_exiles_two_in_your_main_phase() {
    let mut g = table();
    let a = put(&mut g, 1, "Sol Ring");
    let b = put(&mut g, 1, "Mind Stone");
    let r = hand(&mut g, 0, "Return to Dust");
    play(&mut g, r, false, &[obj(a), obj(b)], &[]);
    assert_eq!((g.obj(a).zone, g.obj(b).zone), (Zone::Exile, Zone::Exile));
}

#[test]
fn return_to_dust_takes_one_target_outside_your_main_phase() {
    let mut g = table();
    let a = put(&mut g, 1, "Sol Ring");
    let b = put(&mut g, 1, "Mind Stone");
    let r = hand(&mut g, 0, "Return to Dust");
    g.start_turn(4, 1);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::PassPriority); // the opponent passes in their main phase: you get priority
    assert_eq!(g.pending, Pending::Priority(0));
    act(&mut g, Action::Cast(r));
    act(&mut g, Action::Target(obj(a)));
    assert_eq!(g.legal_actions(), vec![Action::No], "no second target");
    resolve_all(&mut g, &[Action::No]);
    assert_eq!((g.obj(a).zone, g.obj(b).zone), (Zone::Exile, Zone::Battlefield));
}

#[test]
fn reconnaissance_removes_an_attacker_from_combat_and_untaps_it() {
    let mut g = table();
    let r = put(&mut g, 0, "Reconnaissance");
    let looter = put(&mut g, 0, "Merfolk Looter");
    g.objects[looter.0 as usize].since_turn = 0;
    while !g.legal_actions().contains(&Action::Attack(looter)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(looter));
    act(&mut g, Action::FinishAttacks);
    assert!(g.obj(looter).tapped);
    act(&mut g, Action::Activate(r, 0));
    act(&mut g, Action::Target(obj(looter)));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(looter).tapped);
    assert!(!g.combat.as_ref().unwrap().attackers.contains(&looter));
    while g.step != Step::Main2 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[1].life, 40, "no combat damage");
}

#[test]
fn raffine_connives_x_for_the_attackers() {
    let mut g = table();
    let raffine = put(&mut g, 0, "Raffine, Scheming Seer");
    let looter = put(&mut g, 0, "Merfolk Looter");
    for x in [raffine, looter] {
        g.objects[x.0 as usize].since_turn = 0;
    }
    while !g.legal_actions().contains(&Action::Attack(looter)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(raffine));
    act(&mut g, Action::Attack(looter));
    act(&mut g, Action::FinishAttacks);
    let ring = hand(&mut g, 0, "Sol Ring");
    let before = g.players[0].hand.len();
    // Target the Looter; draw 2 (Plains), then discard Sol Ring (nonland) and a Plains.
    let plains = *g.players[0]
        .hand
        .iter()
        .find(|&&c| g.def_of(c).types.land)
        .unwrap();
    resolve_all(
        &mut g,
        &[
            Action::Target(obj(looter)),
            Action::Pick(ring),
            Action::Pick(plains),
        ],
    );
    assert_eq!(g.players[0].hand.len(), before);
    assert_eq!(g.obj(looter).counters, 1, "one nonland card discarded");
    assert!(g.keywords(raffine).has(Keywords::FLYING));
}

// ---- batch 10: types and abilities ----

fn untap(g: &mut Game, id: ObjId) {
    g.objects[id.0 as usize].tapped = false;
}

#[test]
fn mirror_entity_makes_your_creatures_x_x_with_all_types_this_turn() {
    let mut g = table();
    let mirror = put(&mut g, 0, "Mirror Entity");
    let elves = put(&mut g, 0, "Llanowar Elves");
    let angel = put(&mut g, 0, "Serra Angel");
    let theirs = put(&mut g, 1, "Savannah Lions");
    act(
        &mut g,
        Action::ActivateX {
            id: mirror,
            n: 0,
            victim: None,
            x: 4,
        },
    );
    resolve_all(&mut g, &[]);
    for c in [mirror, elves, angel] {
        assert_eq!((g.power(c), g.toughness(c)), (4, 4));
    }
    assert!(g.has_subtype(elves, "Angel") && g.has_subtype(angel, "Elf"));
    assert!(!g.has_subtype(elves, "Forest"), "creature types only");
    assert_eq!((g.power(theirs), g.toughness(theirs)), (2, 1), "only yours");
    to_main1(&mut g, 4, 1);
    assert_eq!((g.power(elves), g.toughness(elves)), (1, 1));
    assert!(!g.has_subtype(elves, "Angel"));
}

#[test]
fn kormus_bell_makes_swamps_one_one_black_creatures() {
    let mut g = table();
    let swamp = g.permanents(0).find(|&x| g.def_of(x).name == "Swamp").unwrap();
    let forest = g.permanents(0).find(|&x| g.def_of(x).name == "Forest").unwrap();
    let theirs = g.permanents(1).find(|&x| g.def_of(x).name == "Swamp").unwrap();
    assert!(!g.types_of(swamp).creature);
    put(&mut g, 0, "Kormus Bell");
    for s in [swamp, theirs] {
        let t = g.types_of(s);
        assert!(t.creature && t.land, "a creature that's still a land");
        assert_eq!((g.power(s), g.toughness(s)), (1, 1));
        assert_eq!(g.colors_of(s), [false, false, true, false, false]);
    }
    assert!(!g.types_of(forest).creature);
    assert_eq!(
        colors(&g, swamp),
        [false, false, true, false, false],
        "still taps for black"
    );
    // With Urborg, every land is a Swamp.
    put(&mut g, 0, "Urborg, Tomb of Yawgmoth");
    assert!(g.types_of(forest).creature);
}

#[test]
fn song_of_the_dryads_makes_a_permanent_a_colorless_forest_land() {
    let mut g = table();
    let angel = put(&mut g, 1, "Serra Angel");
    let song = hand(&mut g, 0, "Song of the Dryads");
    play(&mut g, song, false, &[obj(angel)], &[]);
    assert_eq!(g.obj(song).zone, Zone::Battlefield);
    let t = g.types_of(angel);
    assert!(t.land && !t.creature && !t.enchantment);
    assert!(g.has_subtype(angel, "Forest") && !g.has_subtype(angel, "Angel"));
    assert_eq!(g.colors_of(angel), [false; 5]);
    assert!(!g.keywords(angel).has(Keywords::FLYING), "no abilities");
    let green: Vec<_> = g
        .mana_sources(1)
        .into_iter()
        .filter(|&(x, _)| x == angel)
        .map(|(_, m)| m.colors)
        .collect();
    assert_eq!(
        green,
        [[false, false, false, false, true]],
        "a Forest taps for green"
    );
}

#[test]
fn faerie_conclave_becomes_a_blue_faerie_flier_until_end_of_turn() {
    let mut g = table();
    let fc = put(&mut g, 0, "Faerie Conclave");
    untap(&mut g, fc);
    assert_eq!(colors(&g, fc), [false, true, false, false, false]);
    act(&mut g, Action::Activate(fc, 0));
    resolve_all(&mut g, &[]);
    let t = g.types_of(fc);
    assert!(t.creature && t.land);
    assert_eq!((g.power(fc), g.toughness(fc)), (2, 1));
    assert!(g.keywords(fc).has(Keywords::FLYING));
    assert_eq!(g.colors_of(fc), [false, true, false, false, false]);
    assert!(g.has_subtype(fc, "Faerie"));
    to_main1(&mut g, 4, 1);
    assert!(!g.types_of(fc).creature);
    assert_eq!(g.colors_of(fc), [false; 5]);
}

#[test]
fn raging_ravine_grows_when_it_attacks_as_a_creature() {
    let mut g = table();
    let rr = put(&mut g, 0, "Raging Ravine");
    act(&mut g, Action::Activate(rr, 0));
    resolve_all(&mut g, &[]);
    untap(&mut g, rr);
    assert_eq!(g.colors_of(rr), [false, false, false, true, true]);
    assert!(g.has_subtype(rr, "Elemental"));
    attack_until_decision(&mut g, rr);
    assert_eq!(g.obj(rr).counters, 1, "the gained attack trigger");
    assert_eq!(g.players[1].life, 36);
}

#[test]
fn blinkmoth_nexus_becomes_an_artifact_flier_and_pumps_a_blinkmoth() {
    let mut g = table();
    let b = put(&mut g, 0, "Blinkmoth Nexus");
    act(&mut g, Action::Activate(b, 0));
    resolve_all(&mut g, &[]);
    let t = g.types_of(b);
    assert!(t.creature && t.artifact && t.land);
    assert!(g.keywords(b).has(Keywords::FLYING) && g.has_subtype(b, "Blinkmoth"));
    untap(&mut g, b);
    act(&mut g, Action::Activate(b, 1));
    act(&mut g, Action::Target(obj(b)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(b), g.toughness(b)), (2, 2));
    to_main1(&mut g, 4, 1);
    assert!(!g.types_of(b).artifact && !g.types_of(b).creature);
}

#[test]
fn mishras_factory_becomes_an_assembly_worker_and_pumps_one() {
    let mut g = table();
    let m = put(&mut g, 0, "Mishra's Factory");
    act(&mut g, Action::Activate(m, 0));
    resolve_all(&mut g, &[]);
    assert!(g.types_of(m).artifact && g.has_subtype(m, "Assembly-Worker"));
    untap(&mut g, m);
    act(&mut g, Action::Activate(m, 1));
    act(&mut g, Action::Target(obj(m)));
    resolve_all(&mut g, &[]);
    assert!(g.obj(m).tapped);
    assert_eq!((g.power(m), g.toughness(m)), (3, 3));
}

#[test]
fn mutavault_becomes_a_two_two_with_all_creature_types() {
    let mut g = table();
    let m = put(&mut g, 0, "Mutavault");
    assert!(!g.has_subtype(m, "Goblin"));
    act(&mut g, Action::Activate(m, 0));
    resolve_all(&mut g, &[]);
    assert!(g.types_of(m).creature);
    assert_eq!((g.power(m), g.toughness(m)), (2, 2));
    assert!(g.has_subtype(m, "Goblin") && g.has_subtype(m, "Elf"));
    assert!(!g.has_subtype(m, "Forest"));
    to_main1(&mut g, 4, 1);
    assert!(!g.types_of(m).creature && !g.has_subtype(m, "Goblin"));
}

#[test]
fn abigale_takes_a_creatures_abilities_and_gives_it_three_counters() {
    let mut g = table();
    let angel = put(&mut g, 1, "Serra Angel");
    let a = hand(&mut g, 0, "Abigale, Eloquent First-Year");
    play(&mut g, a, false, &[], &[Action::Target(obj(angel))]);
    let k = g.keywords(angel);
    assert!(!k.has(Keywords::VIGILANCE), "lost");
    assert!(k.has(Keywords::FLYING) && k.has(Keywords::FIRST_STRIKE) && k.has(Keywords::LIFELINK));
    assert!(g.keywords(a).has(Keywords::LIFELINK), "not Abigale herself");
    // A mana creature loses its mana ability.
    let mut g = table();
    let elves = put(&mut g, 1, "Llanowar Elves");
    assert!(g.mana_sources(1).iter().any(|&(x, _)| x == elves));
    let a = hand(&mut g, 0, "Abigale, Eloquent First-Year");
    play(&mut g, a, false, &[], &[Action::Target(obj(elves))]);
    assert!(!g.mana_sources(1).iter().any(|&(x, _)| x == elves));
    assert!(
        g.types_of(elves).creature && g.has_subtype(elves, "Elf"),
        "keeps its types"
    );
}

// ---- batch 11 ----

#[test]
fn storm_of_souls_returns_your_creatures_as_one_one_flying_spirits() {
    let mut g = table();
    let sphinx = g.add_card(0, cid("Consecrated Sphinx"), Zone::Graveyard);
    let elves = g.add_card(0, cid("Llanowar Elves"), Zone::Graveyard);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Graveyard);
    let storm = hand(&mut g, 0, "Storm of Souls");
    play(&mut g, storm, false, &[], &[]);
    for c in [sphinx, elves] {
        assert_eq!(g.obj(c).zone, Zone::Battlefield);
        assert_eq!((g.power(c), g.toughness(c)), (1, 1));
        assert!(g.has_subtype(c, "Spirit") && g.keywords(c).has(Keywords::FLYING));
    }
    assert!(g.has_subtype(sphinx, "Sphinx"));
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    assert_eq!(g.obj(storm).zone, Zone::Exile);
}

// ---- Core theme d3, batch 13 ----

#[test]
fn massacre_girl_shrinks_the_others_again_whenever_a_creature_dies_this_turn() {
    let mut g = table();
    let elves = put(&mut g, 1, "Llanowar Elves"); // 1/1: dies to the first -1/-1
    let angel = put(&mut g, 1, "Serra Angel"); // 4/4
    let sphinx = put(&mut g, 0, "Consecrated Sphinx"); // 4/6
    let girl = hand(&mut g, 0, "Massacre Girl");
    play(&mut g, girl, false, &[], &[]);
    assert_eq!(g.obj(elves).zone, Zone::Graveyard);
    assert_eq!(g.power(angel), 2, "-1/-1, then -1/-1 again for the Elves");
    assert_eq!(g.toughness(sphinx), 4);
    assert_eq!(g.power(girl), 4, "not Massacre Girl");
}

#[test]
fn archfiend_of_ifnir_puts_a_minus_counter_on_their_creatures_when_you_discard() {
    let mut g = table();
    put(&mut g, 0, "Archfiend of Ifnir");
    let looter = put(&mut g, 0, "Merfolk Looter");
    let angel = put(&mut g, 1, "Serra Angel");
    let elves = put(&mut g, 1, "Llanowar Elves");
    let mine = put(&mut g, 0, "Llanowar Elves");
    to_main1(&mut g, 5, 0);
    let junk = hand(&mut g, 0, "Counterspell");
    act(&mut g, Action::Activate(looter, 0));
    resolve_all(&mut g, &[Action::Discard(junk)]);
    assert_eq!(g.obj(junk).zone, Zone::Graveyard);
    assert_eq!(g.power(angel), 3, "a -1/-1 counter");
    assert_eq!(g.obj(elves).zone, Zone::Graveyard);
    assert_eq!(g.obj(mine).zone, Zone::Battlefield, "only the opponent's");
    // Cycling {2} from the hand draws a card.
    let fiend = hand(&mut g, 0, "Archfiend of Ifnir");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(fiend, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(fiend).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n, "cycled one, drew one");
    assert_eq!(
        g.power(angel),
        2,
        "cycling discards: the Archfiend on the battlefield triggers"
    );
}

#[test]
fn the_meathook_massacre_shrinks_by_x_and_drains_on_deaths() {
    let mut g = table();
    let mine = put(&mut g, 0, "Llanowar Elves");
    let theirs = put(&mut g, 1, "Llanowar Elves");
    let angel = put(&mut g, 1, "Serra Angel");
    let m = hand(&mut g, 0, "The Meathook Massacre");
    act(&mut g, Action::CastX(m, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(m).zone, Zone::Battlefield);
    assert_eq!(
        (g.obj(mine).zone, g.obj(theirs).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
    assert_eq!(g.power(angel), 2, "-2/-2");
    assert_eq!(g.players[1].life, 39, "your creature died");
    assert_eq!(g.players[0].life, 41, "their creature died");
}

#[test]
fn wan_shi_tong_enters_with_x_counters_and_draws_half_x() {
    let mut g = table();
    let w = hand(&mut g, 0, "Wan Shi Tong, Librarian");
    let n = g.players[0].hand.len();
    act(&mut g, Action::CastX(w, 5));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(w).counters, 5);
    assert_eq!(g.power(w), 6);
    assert_eq!(g.players[0].hand.len(), n - 1 + 2, "half of 5, rounded down");
    assert!(g.keywords(w).has(Keywords::FLYING.with(Keywords::VIGILANCE)));
}

#[test]
fn korvold_sacrifices_another_permanent_and_grows_and_draws() {
    let mut g = table();
    let ring = put(&mut g, 0, "Sol Ring");
    let elves = put(&mut g, 0, "Llanowar Elves");
    let k = hand(&mut g, 0, "Korvold, Fae-Cursed King");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Cast(k));
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Pick(k)), "another permanent");
    assert!(legal.contains(&Action::Pick(elves)));
    resolve_all(&mut g, &[Action::Pick(ring)]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    assert_eq!(g.obj(k).counters, 1);
    assert_eq!(g.players[0].hand.len(), n - 1 + 1);
}

#[test]
fn savra_pays_2_life_for_an_edict_on_a_black_sacrifice_and_gains_on_a_green_one() {
    let mut g = table();
    put(&mut g, 0, "Savra, Queen of the Golgari");
    let altar = put(&mut g, 0, "Altar of Dementia");
    let entomber = put(&mut g, 0, "Vile Entomber"); // black
    let elves = put(&mut g, 0, "Llanowar Elves"); // green
    let angel = put(&mut g, 1, "Serra Angel");
    act(&mut g, Action::ActivateWith(altar, 0, entomber));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[Action::Yes]); // pay 2 life
    assert_eq!(g.players[0].life, 38);
    assert_eq!(g.obj(angel).zone, Zone::Graveyard, "their only creature");
    act(&mut g, Action::ActivateWith(altar, 0, elves));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[Action::Yes]); // gain 2 life
    assert_eq!(g.players[0].life, 40);
}

#[test]
fn benefactors_draught_untaps_all_creatures_and_draws_when_their_creature_blocks() {
    let mut g = table();
    let mine = put(&mut g, 0, "Llanowar Elves");
    let theirs = put(&mut g, 1, "Serra Angel");
    to_main1(&mut g, 5, 0);
    g.objects[mine.0 as usize].tapped = true;
    g.objects[theirs.0 as usize].tapped = true;
    let b = hand(&mut g, 0, "Benefactor's Draught");
    let n = g.players[0].hand.len();
    play(&mut g, b, false, &[], &[]);
    assert!(!g.obj(mine).tapped && !g.obj(theirs).tapped);
    assert_eq!(g.players[0].hand.len(), n, "cast one, drew one");
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(mine));
    act(&mut g, Action::FinishAttacks);
    while !matches!(g.pending, Pending::DeclareBlockers(1)) {
        act(&mut g, Action::PassPriority);
    }
    act(
        &mut g,
        Action::Block {
            blocker: theirs,
            attacker: mine,
        },
    );
    act(&mut g, Action::FinishBlocks);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n + 1, "their creature blocked");
}

#[test]
fn well_rested_grows_the_creature_and_gains_and_draws_when_it_untaps() {
    let mut g = table();
    let elves = put(&mut g, 0, "Llanowar Elves");
    let w = hand(&mut g, 0, "Well Rested");
    play(&mut g, w, false, &[obj(elves)], &[]);
    assert_eq!(g.obj(w).attached_to.map(|a| a.0), Some(elves));
    g.objects[elves.0 as usize].tapped = true;
    let n = g.players[0].hand.len();
    to_main1(&mut g, 5, 0);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elves).counters, 2);
    assert_eq!(g.players[0].life, 42);
    assert_eq!(g.players[0].hand.len(), n + 2, "the draw step and Well Rested");
}

#[test]
fn sphere_grid_counters_a_creature_that_hits_and_gives_countered_creatures_reach_and_trample() {
    let mut g = table();
    put(&mut g, 0, "Sphere Grid");
    let elves = put(&mut g, 0, "Llanowar Elves");
    g.objects[elves.0 as usize].since_turn = 0;
    assert!(!g.keywords(elves).has(Keywords::TRAMPLE));
    g.start_turn(5, 0);
    attack_until_decision(&mut g, elves);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 39);
    assert_eq!(g.obj(elves).counters, 1);
    assert!(g.keywords(elves).has(Keywords::REACH.with(Keywords::TRAMPLE)));
}

#[test]
fn rapacious_guest_makes_food_grows_on_food_and_drains_its_power_when_it_leaves() {
    let mut g = table();
    let guest = put(&mut g, 0, "Rapacious Guest");
    g.objects[guest.0 as usize].since_turn = 0;
    g.start_turn(5, 0);
    attack_until_decision(&mut g, guest);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 38);
    let food = tokens(&g, 0, "Food");
    assert_eq!(food.len(), 1);
    while g.step != Step::Main2 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Activate(food[0], 0)); // {2}, {T}, sacrifice: gain 3
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(guest).counters, 1);
    let grasp = hand(&mut g, 0, "Infernal Grasp");
    play(
        &mut g,
        grasp,
        false,
        &[obj(guest)],
        &[Action::Target(Target::Player(1))],
    );
    assert_eq!(g.obj(guest).zone, Zone::Graveyard);
    assert_eq!(g.players[1].life, 35, "its power: 3");
}

#[test]
fn klothys_exiles_a_graveyard_card_for_mana_or_a_drain() {
    let mut g = table();
    let k = put(&mut g, 0, "Klothys, God of Destiny");
    assert!(!g.types_of(k).creature, "devotion 2 of 7");
    let bolt = g.add_card(1, cid("Counterspell"), Zone::Graveyard);
    let land = g.add_card(1, cid("Forest"), Zone::Graveyard);
    to_main1(&mut g, 5, 0);
    resolve_all(&mut g, &[Action::Target(obj(bolt))]);
    assert_eq!(g.obj(bolt).zone, Zone::Exile);
    assert_eq!((g.players[0].life, g.players[1].life), (42, 38));
    to_main1(&mut g, 7, 0);
    let pool = g.players[0].pool.len();
    resolve_all(&mut g, &[Action::Target(obj(land))]);
    assert_eq!(g.obj(land).zone, Zone::Exile);
    assert_eq!(g.players[0].pool.len(), pool + 1, "{{R}} or {{G}}");
    assert_eq!(g.players[0].life, 42, "a land: no drain");
}

#[test]
fn ripples_of_undeath_mills_three_and_pays_to_keep_one() {
    let mut g = table();
    put(&mut g, 0, "Ripples of Undeath");
    let a = g.add_card(0, cid("Counterspell"), Zone::Library);
    let b = g.add_card(0, cid("Entomb"), Zone::Library);
    let c = g.add_card(0, cid("Opt"), Zone::Library);
    g.add_card(0, cid("Plains"), Zone::Library); // the draw step's
    to_main1(&mut g, 5, 0);
    resolve_all(&mut g, &[Action::Yes, Action::Pick(b)]);
    assert_eq!(g.obj(b).zone, Zone::Hand);
    assert_eq!((g.obj(a).zone, g.obj(c).zone), (Zone::Graveyard, Zone::Graveyard));
    assert_eq!(g.players[0].life, 37);
    // Declining keeps the life.
    to_main1(&mut g, 7, 0);
    resolve_all(&mut g, &[Action::No]);
    assert_eq!(g.players[0].life, 37);
}

// ---- Core theme d3, batch 14 ----

#[test]
fn professor_onyx_magecraft_drains_and_her_abilities_work() {
    let mut g = table();
    let elves = put(&mut g, 1, "Llanowar Elves");
    let angel = put(&mut g, 1, "Serra Angel");
    let a = g.add_card(0, cid("Counterspell"), Zone::Library);
    let b = g.add_card(0, cid("Entomb"), Zone::Library);
    let c = g.add_card(0, cid("Opt"), Zone::Library);
    let onyx = hand(&mut g, 0, "Professor Onyx");
    play(&mut g, onyx, false, &[], &[]);
    act(&mut g, Action::Loyalty(onyx, 0)); // +1
    resolve_all(&mut g, &[Action::Pick(c)]);
    assert_eq!(g.players[0].life, 39);
    assert_eq!(g.obj(c).zone, Zone::Hand);
    assert_eq!((g.obj(a).zone, g.obj(b).zone), (Zone::Graveyard, Zone::Graveyard));
    play(&mut g, c, false, &[], &[Action::No]); // Opt: keep the top card
    assert_eq!((g.players[0].life, g.players[1].life), (41, 38), "magecraft");
    to_main1(&mut g, 5, 0);
    act(&mut g, Action::Loyalty(onyx, 1)); // −3
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(angel).zone, Zone::Graveyard, "the greatest power");
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
}

/// Passes priority and answers every question with No (or the only answer) until the stack is empty.
fn settle_declining(g: &mut Game) {
    while !g.items.is_empty() || matches!(g.pending, Pending::Decide(_) | Pending::ChooseTarget(_)) {
        let legal = g.legal_actions();
        let a = match g.pending {
            Pending::Decide(_) | Pending::ChooseTarget(_) if legal.contains(&Action::No) => Action::No,
            Pending::Decide(_) | Pending::ChooseTarget(_) => legal[0],
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn flusterstorm_counters_only_instants_and_sorceries_unless_they_pay() {
    let mut g = table();
    let elves = hand(&mut g, 0, "Llanowar Elves");
    let f = hand(&mut g, 0, "Flusterstorm");
    act(&mut g, Action::Cast(elves));
    assert!(
        !g.legal_actions().contains(&Action::Cast(f)),
        "a creature spell isn't a legal target"
    );
    resolve_all(&mut g, &[]);
    let opt = hand(&mut g, 0, "Opt");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Cast(opt));
    act(&mut g, Action::Cast(f));
    act(&mut g, Action::Target(obj(opt)));
    settle_declining(&mut g); // don't pay {1}
    assert_eq!(
        (g.obj(opt).zone, g.obj(f).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
    assert_eq!(g.players[0].hand.len(), n - 2, "Opt was countered: no draw");
}

#[test]
fn end_the_festivities_and_tectonic_hazard_ping_the_opponent_and_their_creatures() {
    for name in ["End the Festivities", "Tectonic Hazard"] {
        let mut g = table();
        let theirs = put(&mut g, 1, "Llanowar Elves");
        let angel = put(&mut g, 1, "Serra Angel");
        let mine = put(&mut g, 0, "Llanowar Elves");
        let c = hand(&mut g, 0, name);
        play(&mut g, c, false, &[], &[]);
        assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "{name}");
        assert_eq!(g.obj(angel).damage, 1);
        assert_eq!(g.obj(mine).zone, Zone::Battlefield, "only theirs");
        assert_eq!((g.players[0].life, g.players[1].life), (40, 39));
    }
}

#[test]
fn smash_to_dust_destroys_an_artifact_or_a_defender_or_pings_their_creatures() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let wall = put(&mut g, 1, "Wall of Reverence");
    let angel = put(&mut g, 1, "Serra Angel");
    let elves = put(&mut g, 1, "Llanowar Elves");
    let c = hand(&mut g, 0, "Smash to Dust");
    act(&mut g, Action::CastModal(c, 0));
    act(&mut g, Action::Target(obj(ring)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    let c = hand(&mut g, 0, "Smash to Dust");
    act(&mut g, Action::CastModal(c, 1));
    let legal = g.legal_actions();
    assert!(
        legal.contains(&Action::Target(obj(wall))) && !legal.contains(&Action::Target(obj(angel))),
        "defender only"
    );
    act(&mut g, Action::Target(obj(wall)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(wall).zone, Zone::Graveyard);
    let c = hand(&mut g, 0, "Smash to Dust");
    act(&mut g, Action::CastModal(c, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elves).zone, Zone::Graveyard);
    assert_eq!((g.obj(angel).damage, g.players[1].life), (1, 40));
}

#[test]
fn debt_to_the_deathless_drains_two_times_x() {
    let mut g = table();
    let c = hand(&mut g, 0, "Debt to the Deathless");
    act(&mut g, Action::CastX(c, 3));
    resolve_all(&mut g, &[]);
    assert_eq!((g.players[0].life, g.players[1].life), (46, 34));
}

#[test]
fn wall_of_reverence_gains_life_equal_to_a_creatures_power_at_your_end_step() {
    let mut g = table();
    put(&mut g, 0, "Wall of Reverence");
    let angel = put(&mut g, 0, "Serra Angel");
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.step, Step::End);
    resolve_all(&mut g, &[Action::Target(obj(angel)), Action::Yes]);
    assert_eq!(g.players[0].life, 44);
}

#[test]
fn buried_ruin_returns_an_artifact_card_from_your_graveyard() {
    let mut g = table();
    let ruin = put(&mut g, 0, "Buried Ruin");
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Graveyard);
    let elves = g.add_card(0, cid("Llanowar Elves"), Zone::Graveyard);
    act(&mut g, Action::Activate(ruin, 0));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(elves))),
        "artifact cards only"
    );
    act(&mut g, Action::Target(obj(ring)));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.obj(ring).zone, g.obj(ruin).zone),
        (Zone::Hand, Zone::Graveyard)
    );
}

#[test]
fn tomb_of_the_spirit_dragon_gains_one_per_colorless_creature() {
    let mut g = table();
    let tomb = put(&mut g, 0, "Tomb of the Spirit Dragon");
    put(&mut g, 0, "Spellskite");
    put(&mut g, 0, "Spellskite");
    put(&mut g, 0, "Llanowar Elves");
    act(&mut g, Action::Activate(tomb, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 42);
}

#[test]
fn zhalfirin_void_scries_1_on_entering_and_taps_for_colorless() {
    let mut g = table();
    let top = g.add_card(0, cid("Counterspell"), Zone::Library);
    let v = hand(&mut g, 0, "Zhalfirin Void");
    act(&mut g, Action::PlayLand(v));
    resolve_all(&mut g, &[Action::Yes]); // to the bottom
    assert_eq!(g.players[0].library[0], top);
    assert_eq!(colors(&g, v), [false; 5], "colorless");
}

#[test]
fn all_is_dust_sacrifices_every_colored_permanent() {
    let mut g = table();
    let ring = put(&mut g, 0, "Sol Ring");
    let mine = put(&mut g, 0, "Llanowar Elves");
    let theirs = put(&mut g, 1, "Serra Angel");
    let c = hand(&mut g, 0, "All Is Dust");
    play(&mut g, c, false, &[], &[]);
    assert_eq!(
        (g.obj(mine).zone, g.obj(theirs).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
    assert_eq!(g.obj(ring).zone, Zone::Battlefield, "colorless");
    assert_eq!(
        g.permanents(1).filter(|&x| g.types_of(x).land).count(),
        25,
        "lands are colorless"
    );
}

#[test]
fn bane_of_bala_ged_makes_the_defender_exile_two_permanents() {
    let mut g = table();
    let bane = put(&mut g, 0, "Bane of Bala Ged");
    g.objects[bane.0 as usize].since_turn = 0;
    let angel = put(&mut g, 1, "Serra Angel");
    let ring = put(&mut g, 1, "Sol Ring");
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(bane));
    act(&mut g, Action::FinishAttacks);
    while g.pending != Pending::Decide(1) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Pick(angel));
    act(&mut g, Action::Pick(ring));
    assert_eq!((g.obj(angel).zone, g.obj(ring).zone), (Zone::Exile, Zone::Exile));
}

#[test]
fn portal_to_phyrexia_edicts_three_and_reanimates_each_upkeep_as_a_phyrexian() {
    let mut g = table();
    let a = put(&mut g, 1, "Serra Angel");
    let b = put(&mut g, 1, "Llanowar Elves");
    let c = put(&mut g, 1, "Consecrated Sphinx");
    let portal = hand(&mut g, 0, "Portal to Phyrexia");
    play(&mut g, portal, false, &[], &[]);
    assert!([a, b, c].iter().all(|&x| g.obj(x).zone == Zone::Graveyard));
    g.start_turn(5, 0);
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        act(&mut g, Action::PassPriority);
    }
    resolve_all(&mut g, &[Action::Target(obj(a))]);
    assert_eq!(g.obj(a).zone, Zone::Battlefield);
    assert_eq!(g.obj(a).controller, 0);
    assert!(g.has_subtype(a, "Phyrexian") && g.has_subtype(a, "Angel"));
}

// ---- Core theme d3, batch 15 ----

#[test]
fn fade_from_history_gives_bears_to_artifact_and_enchantment_players_then_destroys_them() {
    let mut g = table();
    let grove = put(&mut g, 0, "Sterling Grove");
    let ring = put(&mut g, 1, "Sol Ring");
    let fade = hand(&mut g, 0, "Fade from History");
    play(&mut g, fade, false, &[], &[]);
    assert_eq!(
        (g.obj(grove).zone, g.obj(ring).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
    for p in 0..2 {
        let bears = tokens(&g, p, "Bear");
        assert_eq!(bears.len(), 1);
        assert_eq!((g.power(bears[0]), g.toughness(bears[0])), (2, 2));
        assert_eq!(g.colors_of(bears[0]), [false, false, false, false, true], "green");
    }
    let fade = hand(&mut g, 0, "Fade from History");
    play(&mut g, fade, false, &[], &[]);
    assert_eq!(
        (tokens(&g, 0, "Bear").len(), tokens(&g, 1, "Bear").len()),
        (1, 1),
        "nobody has one now"
    );
}

#[test]
fn warping_wail_exiles_a_small_creature_counters_a_sorcery_or_makes_a_scion() {
    let mut g = table();
    put(&mut g, 0, "Sol Ring");
    put(&mut g, 0, "Sol Ring");
    let skite = put(&mut g, 1, "Spellskite");
    let angel = put(&mut g, 1, "Serra Angel");
    let wail = hand(&mut g, 0, "Warping Wail");
    act(&mut g, Action::CastModal(wail, 0));
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Target(obj(angel))));
    act(&mut g, Action::Target(obj(skite)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(skite).zone, Zone::Exile, "toughness 4 but power 0");

    let wail = hand(&mut g, 0, "Warping Wail");
    act(&mut g, Action::CastModal(wail, 2));
    resolve_all(&mut g, &[]);
    let scions = tokens(&g, 0, "Eldrazi Scion");
    assert_eq!(scions.len(), 1);
    let s = scions[0];
    assert_eq!((g.power(s), g.toughness(s)), (1, 1));
    assert_eq!(g.colors_of(s), [false; 5]);
    g.manual_mana[0] = true;
    act(&mut g, Action::TapForMana(s, 5));
    assert!(g.players[0].pool.iter().any(|m| m.colorless));
    assert!(tokens(&g, 0, "Eldrazi Scion").is_empty(), "sacrificed");
}

#[test]
fn warping_wail_counters_only_sorcery_spells() {
    let mut g = table();
    put(&mut g, 0, "Sol Ring");
    let wail = hand(&mut g, 0, "Warping Wail");
    g.start_turn(4, 1);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let ponder = g.add_card(1, cid("Ponder"), Zone::Hand);
    act(&mut g, Action::Cast(ponder));
    act(&mut g, Action::PassPriority);
    assert!(g.legal_actions().contains(&Action::CastModal(wail, 1)));
    act(&mut g, Action::CastModal(wail, 1));
    act(&mut g, Action::Target(obj(ponder)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ponder).zone, Zone::Graveyard, "countered: no look");
}

#[test]
fn exalted_flamer_returns_a_random_spell_each_upkeep_and_pings_on_spells() {
    let mut g = table();
    let flamer = put(&mut g, 0, "Exalted Flamer of Tzeentch");
    let life = g.players[1].life;
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, bolt, false, &[Target::Object(flamer)], &[]);
    // The bolt hit the Flamer (2/4): still alive; the trigger dealt 1 to the opponent.
    assert_eq!(g.players[1].life, life - 1);
    assert_eq!(g.obj(flamer).zone, Zone::Battlefield);
    let elves = g.add_card(0, cid("Llanowar Elves"), Zone::Graveyard);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(bolt).zone, Zone::Hand, "the only instant or sorcery");
    assert_eq!(g.obj(elves).zone, Zone::Graveyard);
}

#[test]
fn expressive_iteration_hand_bottom_and_exile_to_play_this_turn() {
    let mut g = table();
    let angel = g.add_card(0, cid("Serra Angel"), Zone::Library);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    let it = hand(&mut g, 0, "Expressive Iteration");
    play(&mut g, it, false, &[], &[Action::Pick(angel), Action::Pick(ring)]);
    assert_eq!(g.obj(angel).zone, Zone::Hand);
    assert_eq!(g.players[0].library[0], ring);
    assert_eq!(g.obj(forest).zone, Zone::Exile);
    act(&mut g, Action::PlayLand(forest));
    assert_eq!(
        g.obj(forest).zone,
        Zone::Battlefield,
        "the exiled land is playable"
    );
}

#[test]
fn mission_briefing_surveils_then_lets_you_cast_a_spell_from_the_graveyard_once() {
    let mut g = table();
    let bolt = g.add_card(0, cid("Lightning Bolt"), Zone::Library);
    let top = g.add_card(0, cid("Serra Angel"), Zone::Library);
    let mb = hand(&mut g, 0, "Mission Briefing");
    act(&mut g, Action::Cast(mb));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    // Surveil 2: both into the graveyard, then choose the Bolt.
    let mut guard = 0;
    while g.obj(bolt).zone == Zone::Library && guard < 4 {
        let legal = g.legal_actions();
        let a = legal
            .iter()
            .find(|a| matches!(a, Action::Pick(_)))
            .copied()
            .unwrap_or(legal[0]);
        act(&mut g, a);
        guard += 1;
    }
    assert_eq!(
        (g.obj(bolt).zone, g.obj(top).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
    resolve_all(&mut g, &[Action::Pick(bolt)]);
    let life = g.players[1].life;
    act(&mut g, Action::CastFlashback(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 3);
    assert_eq!(g.obj(bolt).zone, Zone::Exile);
}

#[test]
fn mizzixs_mastery_casts_a_copy_free_and_overloaded_copies_each() {
    let mut g = table();
    let b1 = g.add_card(0, cid("Lightning Bolt"), Zone::Graveyard);
    let b2 = g.add_card(0, cid("Lightning Bolt"), Zone::Graveyard);
    let mm = hand(&mut g, 0, "Mizzix's Mastery");
    act(&mut g, Action::Cast(mm));
    act(&mut g, Action::Target(obj(b1)));
    while !matches!(g.pending, Pending::FreeCast(_)) {
        act(&mut g, Action::PassPriority);
    }
    let copies = free_casts(&g);
    assert_eq!(copies.len(), 1);
    let life = g.players[1].life;
    act(&mut g, Action::Cast(copies[0]));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 3);
    assert_eq!(
        (g.obj(b1).zone, g.obj(b2).zone, g.obj(mm).zone),
        (Zone::Exile, Zone::Graveyard, Zone::Exile)
    );

    // Overload {5}{R}{R}{R}: each instant and sorcery card.
    g.add_card(0, cid("Lightning Bolt"), Zone::Graveyard);
    for _ in 0..3 {
        put(&mut g, 0, "Mountain");
    }
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let mm = hand(&mut g, 0, "Mizzix's Mastery");
    act(&mut g, Action::CastKicked(mm));
    while !matches!(g.pending, Pending::FreeCast(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(free_casts(&g).len(), 2);
}

fn free_casts(g: &Game) -> Vec<ObjId> {
    g.legal_actions()
        .into_iter()
        .filter_map(|a| match a {
            Action::Cast(c) => Some(c),
            _ => None,
        })
        .collect()
}

// ---- Core theme d3, batch 18 ----

#[test]
fn minamo_untaps_a_legendary_permanent() {
    let mut g = table();
    let minamo = put(&mut g, 0, "Minamo, School at Water's Edge");
    let avacyn = put(&mut g, 0, "Avacyn, Angel of Hope");
    let angel = put(&mut g, 0, "Serra Angel");
    g.objects[avacyn.0 as usize].tapped = true;
    g.objects[angel.0 as usize].tapped = true;
    act(&mut g, Action::Activate(minamo, 0));
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Target(obj(angel))), "not legendary");
    act(&mut g, Action::Target(obj(avacyn)));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(avacyn).tapped && g.obj(minamo).tapped);
}

#[test]
fn torment_of_hailfire_repeats_x_times() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let life = g.players[1].life;
    let t = hand(&mut g, 0, "Torment of Hailfire");
    act(&mut g, Action::CastX(t, 3));
    while g.pending != Pending::Decide(1) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Pick(ring));
    while g.pending == Pending::Decide(1) {
        act(&mut g, Action::No);
    }
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    assert_eq!(g.players[1].life, life - 6, "two of the three rounds in life");
}

#[test]
fn ensnare_taps_all_creatures_for_two_islands() {
    let mut g = table();
    let elves = put(&mut g, 0, "Llanowar Elves");
    let angel = put(&mut g, 1, "Serra Angel");
    let e = hand(&mut g, 0, "Ensnare");
    let islands = |g: &Game| g.permanents(0).filter(|&x| g.def_of(x).name == "Island").count();
    let before = islands(&g);
    act(&mut g, Action::CastAlternative(e));
    resolve_all(&mut g, &[]);
    assert!(g.obj(elves).tapped && g.obj(angel).tapped);
    assert_eq!(islands(&g), before - 2);
}

#[test]
fn kinnan_doubles_nonland_mana_and_digs_for_a_non_human() {
    let mut g = table();
    let kinnan = put(&mut g, 0, "Kinnan, Bonder Prodigy");
    let ring = put(&mut g, 0, "Sol Ring");
    g.manual_mana[0] = true;
    act(&mut g, Action::TapForMana(ring, 5));
    assert_eq!(
        g.players[0].pool.iter().filter(|m| m.colorless).count(),
        3,
        "{{C}}{{C}} and one more"
    );
    g.manual_mana[0] = false;
    g.players[0].pool.clear();
    let human = g.add_card(0, cid("Imperial Recruiter"), Zone::Library);
    let angel = g.add_card(0, cid("Serra Angel"), Zone::Library);
    act(&mut g, Action::Activate(kinnan, 0));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(!g.legal_actions().contains(&Action::Pick(human)));
    resolve_all(&mut g, &[Action::Pick(angel)]);
    assert_eq!(g.obj(angel).zone, Zone::Battlefield);
    assert_eq!(g.obj(human).zone, Zone::Library);
}

#[test]
fn orcish_bowmasters_pings_and_amasses_on_entering_and_on_extra_draws() {
    let mut g = table();
    let life = g.players[1].life;
    let b = hand(&mut g, 0, "Orcish Bowmasters");
    play(&mut g, b, false, &[], &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].life, life - 1);
    let army = tokens(&g, 0, "Orc Army");
    assert_eq!(army.len(), 1);
    assert_eq!(g.power(army[0]), 1);
    // The opponent's turn: the draw step's card doesn't trigger, Opt's does.
    to_main1(&mut g, 4, 1);
    assert_eq!(g.players[1].life, life - 1, "the first draw in their draw step");
    let opt = g.add_card(1, cid("Opt"), Zone::Hand);
    act(&mut g, Action::Cast(opt));
    let mut guard = 0;
    while (!g.items.is_empty() || !matches!(g.pending, Pending::Priority(_))) && guard < 40 {
        let a = match g.pending {
            Pending::ChooseTarget(_) => Action::Target(Target::Player(1)),
            Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
        guard += 1;
    }
    assert_eq!(g.players[1].life, life - 2);
    assert_eq!(g.power(army[0]), 2, "the same Army grew");
}

#[test]
fn cloudstone_curio_may_return_a_permanent_sharing_a_type() {
    let mut g = table();
    put(&mut g, 0, "Cloudstone Curio");
    let angel = put(&mut g, 0, "Serra Angel");
    let elves = hand(&mut g, 0, "Llanowar Elves");
    act(&mut g, Action::Cast(elves));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    resolve_all(&mut g, &[Action::Pick(angel)]);
    assert_eq!(g.obj(angel).zone, Zone::Hand);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
}

#[test]
fn teething_wurmlet_gains_life_per_artifact_and_grows_once_a_turn() {
    let mut g = table();
    let w = put(&mut g, 0, "Teething Wurmlet");
    let life = g.players[0].life;
    for _ in 0..3 {
        let r = hand(&mut g, 0, "Sol Ring");
        play(&mut g, r, false, &[], &[]);
    }
    assert_eq!(g.players[0].life, life + 3);
    assert_eq!(g.power(w), 2, "one counter this turn");
    assert!(g.keywords(w).has(Keywords::DEATHTOUCH), "three artifacts");
}

#[test]
fn sanctum_of_ugin_sacrifices_to_find_a_colorless_creature() {
    let mut g = table();
    let sanctum = put(&mut g, 0, "Sanctum of Ugin");
    let skite = g.add_card(0, cid("Spellskite"), Zone::Library);
    let dust = hand(&mut g, 0, "All Is Dust");
    act(&mut g, Action::Cast(dust));
    let mut guard = 0;
    while g.obj(skite).zone == Zone::Library && guard < 20 {
        let legal = g.legal_actions();
        let a = if legal.contains(&Action::Yes) {
            Action::Yes
        } else if legal.contains(&Action::Pick(skite)) {
            Action::Pick(skite)
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
        guard += 1;
    }
    assert_eq!(g.obj(sanctum).zone, Zone::Graveyard);
    assert_eq!(g.obj(skite).zone, Zone::Hand);
}

#[test]
fn rishkars_expertise_draws_by_power_then_casts_one_free() {
    let mut g = table();
    put(&mut g, 0, "Serra Angel");
    let elves = hand(&mut g, 0, "Llanowar Elves");
    let r = hand(&mut g, 0, "Rishkar's Expertise");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Cast(r));
    while !matches!(g.pending, Pending::FreeCast(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[0].hand.len(), n - 1 + 4, "Serra Angel's power");
    act(&mut g, Action::Cast(elves));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
}

// ---- Core theme d3, batch 20 ----

#[test]
fn dragon_tempest_hastes_the_flier_and_the_dragon_burns_per_dragon() {
    let mut g = table();
    put(&mut g, 0, "Dragon Tempest");
    put(&mut g, 0, "Shivan Dragon");
    let d = hand(&mut g, 0, "Shivan Dragon");
    let life = g.players[1].life;
    act(&mut g, Action::Cast(d));
    let mut guard = 0;
    while (!g.items.is_empty() || !matches!(g.pending, Pending::Priority(_))) && guard < 40 {
        let legal = g.legal_actions();
        let a = if legal.contains(&Action::Target(Target::Player(1))) {
            Action::Target(Target::Player(1))
        } else if matches!(g.pending, Pending::Priority(_)) {
            Action::PassPriority
        } else {
            legal[0]
        };
        act(&mut g, a);
        guard += 1;
    }
    assert_eq!(g.players[1].life, life - 2, "two Dragons");
    assert!(g.keywords(d).has(Keywords::HASTE));
}

#[test]
fn ledger_shredder_connives_on_the_second_spell() {
    let mut g = table();
    let s = put(&mut g, 0, "Ledger Shredder");
    let angel = hand(&mut g, 0, "Serra Angel");
    for _ in 0..2 {
        let e = hand(&mut g, 0, "Llanowar Elves");
        act(&mut g, Action::Cast(e));
        let mut guard = 0;
        while (!g.items.is_empty() || matches!(g.pending, Pending::Decide(_))) && guard < 20 {
            let a = if matches!(g.pending, Pending::Decide(_)) {
                Action::Pick(angel)
            } else {
                Action::PassPriority
            };
            act(&mut g, a);
            guard += 1;
        }
    }
    assert_eq!(g.obj(angel).zone, Zone::Graveyard, "discarded");
    assert_eq!(g.power(s), 2, "a nonland card: a counter");
}

#[test]
fn ancient_brass_dragon_reanimates_from_either_graveyard() {
    let mut g = table();
    let brass = put(&mut g, 0, "Ancient Brass Dragon");
    let theirs = g.add_card(1, cid("Llanowar Elves"), Zone::Graveyard);
    let land = g.add_card(1, cid("Forest"), Zone::Graveyard);
    g.start_turn(5, 0);
    attack_until_decision(&mut g, brass);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(theirs)) && legal.contains(&Action::No));
    assert!(!legal.contains(&Action::Pick(land)));
    act(&mut g, Action::Pick(theirs));
    assert_eq!(g.obj(theirs).zone, Zone::Battlefield);
    assert_eq!(g.obj(theirs).controller, 0, "under your control");
}

// ---- Core theme d3, batch 21 ----

#[test]
fn lord_of_extinction_counts_every_graveyard() {
    let mut g = table();
    let lord = put(&mut g, 0, "Lord of Extinction");
    for (p, c) in [(0, "Opt"), (1, "Forest"), (1, "Serra Angel")] {
        g.add_card(p, cid(c), Zone::Graveyard);
    }
    let n = (g.players[0].graveyard.len() + g.players[1].graveyard.len()) as i32;
    assert_eq!((g.power(lord), g.toughness(lord)), (n, n));
}

#[test]
fn accursed_marauder_makes_each_player_sacrifice_a_nontoken_creature() {
    let mut g = table();
    let elves = put(&mut g, 0, "Llanowar Elves");
    let angel = put(&mut g, 1, "Serra Angel");
    let m = hand(&mut g, 0, "Accursed Marauder");
    act(&mut g, Action::Cast(m));
    let mut guard = 0;
    while (!g.items.is_empty() || matches!(g.pending, Pending::Decide(_))) && guard < 20 {
        let a = match g.pending {
            Pending::Decide(0) => Action::Pick(elves),
            Pending::Decide(_) => Action::Pick(angel),
            _ => Action::PassPriority,
        };
        act(&mut g, a);
        guard += 1;
    }
    assert_eq!(
        (g.obj(elves).zone, g.obj(angel).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
    assert_eq!(g.obj(m).zone, Zone::Battlefield);
}

#[test]
fn drana_counters_each_attacker_after_first_strike_damage() {
    let mut g = table();
    let drana = put(&mut g, 0, "Drana, Liberator of Malakir");
    g.start_turn(5, 0);
    attack_to_main2(&mut g, drana);
    assert_eq!(g.obj(drana).counters, 1);
    assert_eq!(
        g.players[1].life, 38,
        "first strike: 2 damage, before the counter"
    );
}

#[test]
fn canopy_gargantuan_doubles_toughness_in_counters_at_upkeep() {
    let mut g = table();
    put(&mut g, 0, "Canopy Gargantuan");
    let angel = put(&mut g, 0, "Serra Angel");
    let theirs = put(&mut g, 1, "Serra Angel");
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!((g.obj(angel).counters, g.obj(theirs).counters), (4, 0));
}

#[test]
fn vampire_hexmage_removes_all_counters() {
    let mut g = table();
    let hex = put(&mut g, 0, "Vampire Hexmage");
    let angel = put(&mut g, 1, "Serra Angel");
    g.objects[angel.0 as usize].counters = 3;
    act(&mut g, Action::Activate(hex, 0));
    act(&mut g, Action::Target(obj(angel)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(angel).counters, g.obj(hex).zone), (0, Zone::Graveyard));
}

#[test]
fn takenuma_mills_three_then_returns_a_creature_card() {
    let mut g = table();
    let angel = g.add_card(0, cid("Serra Angel"), Zone::Library);
    let t = hand(&mut g, 0, "Takenuma, Abandoned Mire");
    act(&mut g, Action::Activate(t, 0));
    resolve_all(&mut g, &[Action::Pick(angel)]);
    assert_eq!((g.obj(angel).zone, g.obj(t).zone), (Zone::Hand, Zone::Graveyard));
}

// ---- Core theme d3, batch 22 ----

#[test]
fn organic_extinction_destroys_only_nonartifact_creatures() {
    let mut g = table();
    let elves = put(&mut g, 0, "Llanowar Elves");
    let angel = put(&mut g, 1, "Serra Angel");
    let thopter = put(&mut g, 1, "Ornithopter");
    let c = hand(&mut g, 0, "Organic Extinction");
    assert!(g.def_of(c).mana_ext.improvise);
    act(&mut g, Action::Cast(c));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.obj(elves).zone, g.obj(angel).zone, g.obj(thopter).zone),
        (Zone::Graveyard, Zone::Graveyard, Zone::Battlefield)
    );
}

#[test]
fn camellia_makes_squirrels_from_foods_and_forages_counters_onto_them() {
    let mut g = table();
    let cam = put(&mut g, 0, "Camellia, the Seedmiser");
    let food = (0..g.defs.len())
        .find(|&i| g.defs[i].token && g.defs[i].name == "Food")
        .expect("Food");
    let meal = g.add_card(0, food, Zone::Battlefield);
    act(&mut g, Action::Activate(cam, 0));
    resolve_all(&mut g, &[]);
    assert_ne!(g.obj(meal).zone, Zone::Battlefield, "foraged the Food");
    let squirrels = tokens(&g, 0, "Squirrel");
    assert_eq!(squirrels.len(), 1, "the Food sacrifice made a Squirrel");
    let sq = squirrels[0];
    assert!(
        g.keywords(sq).has(Keywords::MENACE),
        "other Squirrels have menace"
    );
    for _ in 0..3 {
        g.add_card(0, cid("Opt"), Zone::Graveyard);
    }
    act(&mut g, Action::Activate(cam, 0));
    resolve_all(&mut g, &[]);
    // The Squirrel trigger resolved before the first ability, so it got a counter then too.
    assert_eq!((g.obj(sq).counters, g.obj(cam).counters), (2, 0));
}

#[test]
fn dig_through_time_delves_and_takes_two_of_seven() {
    let mut g = table();
    let top: Vec<ObjId> = (0..7).map(|_| g.add_card(0, cid("Opt"), Zone::Library)).collect();
    let gy: Vec<ObjId> = (0..6)
        .map(|_| g.add_card(0, cid("Forest"), Zone::Graveyard))
        .collect();
    let c = hand(&mut g, 0, "Dig Through Time");
    let lib = g.players[0].library.len();
    act(&mut g, Action::Cast(c));
    if let Pending::Decide(_) = g.pending {
        act(&mut g, Action::Amount(6));
    }
    assert!(gy.iter().all(|&x| g.obj(x).zone == Zone::Exile), "delved six");
    let picks: Vec<Action> = [0, 3, 1, 2, 4, 5].iter().map(|&i| Action::Pick(top[i])).collect();
    resolve_all(&mut g, &picks);
    assert_eq!((g.obj(top[0]).zone, g.obj(top[3]).zone), (Zone::Hand, Zone::Hand));
    assert_eq!(g.players[0].library.len(), lib - 2);
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
}

// ---- Core theme d3, batch 27 ----

#[test]
fn mikaeus_pumps_non_humans_with_undying_and_destroys_a_human_that_hits_you() {
    let mut g = table();
    let m = put(&mut g, 1, "Mikaeus, the Unhallowed");
    assert!(g.keywords(m).has(Keywords::INTIMIDATE));
    let elves = put(&mut g, 1, "Llanowar Elves");
    let their_human = put(&mut g, 1, "Thought Courier");
    assert_eq!((g.power(elves), g.toughness(elves)), (2, 2));
    assert!(g.keywords(elves).has(Keywords::UNDYING));
    assert_eq!(g.power(their_human), 1, "not a Human's");
    assert_eq!(g.power(m), 5, "other");
    let pilgrim = put(&mut g, 0, "Avacyn's Pilgrim");
    attack_to_main2(&mut g, pilgrim);
    assert_eq!(g.players[1].life, 39);
    assert_eq!(
        g.obj(pilgrim).zone,
        Zone::Graveyard,
        "a Human dealt damage to Mikaeus's controller"
    );
}

#[test]
fn skyscribing_each_player_draws_x_and_forecasts_a_card_each() {
    let mut g = table();
    let c = hand(&mut g, 0, "Skyscribing");
    let f = hand(&mut g, 0, "Skyscribing");
    let (h0, h1) = (g.players[0].hand.len(), g.players[1].hand.len());
    act(&mut g, Action::CastX(c, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), h0 - 1 + 2);
    assert_eq!(g.players[1].hand.len(), h1 + 2);
    assert!(!g.legal_actions().contains(&Action::Activate(f, 0)), "main phase");
    g.start_turn(5, 0);
    while g.step != Step::Upkeep {
        act(&mut g, Action::PassPriority);
    }
    let (h0, h1) = (g.players[0].hand.len(), g.players[1].hand.len());
    act(&mut g, Action::Activate(f, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(f).zone, Zone::Hand, "revealed");
    assert_eq!(g.players[0].hand.len(), h0 + 1);
    assert_eq!(g.players[1].hand.len(), h1 + 1);
    assert!(
        !g.legal_actions().contains(&Action::Activate(f, 0)),
        "once each turn"
    );
}

/// Activates Fabled Passage beside `others` and returns the basic it found.
fn fabled_passage_with(others: &[&str]) -> (Game, ObjId) {
    let mut lands = vec!["Fabled Passage"];
    lands.extend_from_slice(others);
    let mut g = game(&lands);
    let fp = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Fabled Passage")
        .unwrap();
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    act(&mut g, Action::Activate(fp, 0));
    assert_eq!(g.obj(fp).zone, Zone::Graveyard, "sacrificed as a cost");
    resolve_all(&mut g, &[Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    (g, forest)
}

#[test]
fn fabled_passage_untaps_the_land_with_four_lands() {
    let (g, forest) = fabled_passage_with(&["Plains", "Plains", "Plains"]);
    assert!(
        !g.obj(forest).tapped,
        "four lands (the found one included): untapped"
    );
}

#[test]
fn fabled_passage_leaves_the_land_tapped_with_three_lands() {
    let (g, forest) = fabled_passage_with(&["Plains", "Plains"]);
    assert!(g.obj(forest).tapped, "only three lands: stays tapped");
}

fn green_sources(g: &Game, id: ObjId) -> bool {
    g.mana_sources(0).into_iter().any(|(x, m)| x == id && m.colors[4])
}

#[test]
fn malcolm_loots_and_casts_the_discarded_card_free_at_four_chorus_counters() {
    let mut g = table();
    let malcolm = put(&mut g, 0, "Malcolm, Alluring Scoundrel");
    assert!(g.keywords(malcolm).has(Keywords::FLASH) && g.keywords(malcolm).has(Keywords::FLYING));
    g.objects[malcolm.0 as usize].kind_counters = 3;
    let elves = hand(&mut g, 0, "Llanowar Elves");
    g.start_turn(5, 0);
    attack_until_decision(&mut g, malcolm);
    assert_eq!(g.obj(malcolm).kind_counters, 4, "a chorus counter");
    assert_eq!(g.pending, Pending::Decide(0), "discard");
    act(&mut g, Action::Pick(elves));
    assert_eq!(g.obj(elves).zone, Zone::Graveyard);
    assert!(
        g.legal_actions().contains(&Action::Cast(elves)),
        "may cast it free"
    );
    act(&mut g, Action::Cast(elves));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
}

#[test]
fn malcolm_below_four_chorus_counters_only_loots() {
    let mut g = table();
    let malcolm = put(&mut g, 0, "Malcolm, Alluring Scoundrel");
    let elves = hand(&mut g, 0, "Llanowar Elves");
    g.start_turn(5, 0);
    attack_until_decision(&mut g, malcolm);
    assert_eq!(g.obj(malcolm).kind_counters, 1);
    act(&mut g, Action::Pick(elves));
    assert_eq!(g.obj(elves).zone, Zone::Graveyard);
    assert!(!g.legal_actions().contains(&Action::Cast(elves)));
}

#[test]
fn brightcap_badger_makes_saprolings_that_tap_for_green() {
    let mut g = table();
    let badger = hand(&mut g, 0, "Brightcap Badger // Fungus Frolic");
    act(&mut g, Action::CastOmen(badger));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(badger).zone, Zone::Exile, "on an adventure");
    let saps = tokens(&g, 0, "Saproling");
    assert_eq!(saps.len(), 2);
    g.start_turn(5, 0); // no longer summoning sick
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert!(!green_sources(&g, saps[0]), "no Badger yet");
    play(&mut g, badger, false, &[], &[]);
    assert_eq!(g.obj(badger).zone, Zone::Battlefield, "cast from exile");
    assert!(green_sources(&g, saps[0]) && green_sources(&g, saps[1]));
    assert!(!green_sources(&g, badger), "only Fungus and Saprolings");
    while g.step != Step::End {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    resolve_all(&mut g, &[]);
    assert_eq!(
        tokens(&g, 0, "Saproling").len(),
        3,
        "a Saproling at your end step"
    );
}

#[test]
fn shanna_grows_per_creature_and_dodges_opponents_abilities() {
    let mut g = table();
    let shanna = put(&mut g, 1, "Shanna, Sisay's Legacy");
    assert_eq!((g.power(shanna), g.toughness(shanna)), (1, 1), "herself");
    let elves = put(&mut g, 1, "Llanowar Elves");
    assert_eq!((g.power(shanna), g.toughness(shanna)), (2, 2));
    let thassa = put(&mut g, 0, "Thassa, Deep-Dwelling");
    act(&mut g, Action::Activate(thassa, 0));
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Target(obj(elves))));
    assert!(
        !legal.contains(&Action::Target(obj(shanna))),
        "an opponent's ability"
    );
    act(&mut g, Action::Target(obj(elves)));
    resolve_all(&mut g, &[]);
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    assert!(
        g.legal_actions().contains(&Action::Target(obj(shanna))),
        "spells still can"
    );
}

#[test]
fn chandra_dressed_to_kill_pings_exiles_red_and_makes_her_emblem() {
    let mut g = table();
    let chandra = put(&mut g, 0, "Chandra, Dressed to Kill");
    g.objects[chandra.0 as usize].loyalty = 7;
    let life = g.players[1].life;
    act(&mut g, Action::Loyalty(chandra, 0));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 1, "1 damage");
    assert_eq!(g.obj(chandra).loyalty, 8);
    // The -7 next turn: the top five exiled, red ones castable, and the emblem.
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let elves = g.add_card(0, cid("Llanowar Elves"), Zone::Library);
    let bolt = g.add_card(0, cid("Lightning Bolt"), Zone::Library);
    act(&mut g, Action::Loyalty(chandra, 2));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(bolt).zone, g.obj(elves).zone), (Zone::Exile, Zone::Exile));
    assert!(g.obj(bolt).playable.is_some() && g.obj(elves).playable.is_none());
    assert!(g.emblems.iter().any(|e| e.0 == 0), "an emblem");
    let life = g.players[1].life;
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].life, life - 4, "the Bolt's 3 and the emblem's 1");
}

#[test]
fn adept_watershaper_makes_other_tapped_creatures_indestructible() {
    let mut g = table();
    let w = put(&mut g, 0, "Adept Watershaper");
    let elves = put(&mut g, 0, "Llanowar Elves");
    let theirs = put(&mut g, 1, "Llanowar Elves");
    assert!(!g.keywords(elves).has(Keywords::INDESTRUCTIBLE), "untapped");
    g.objects[elves.0 as usize].tapped = true;
    g.objects[w.0 as usize].tapped = true;
    g.objects[theirs.0 as usize].tapped = true;
    assert!(g.keywords(elves).has(Keywords::INDESTRUCTIBLE));
    assert!(!g.keywords(w).has(Keywords::INDESTRUCTIBLE), "other");
    assert!(!g.keywords(theirs).has(Keywords::INDESTRUCTIBLE), "yours");
}

#[test]
fn riverguards_reflexes_pumps_grants_first_strike_and_untaps() {
    let mut g = table();
    let elves = put(&mut g, 0, "Llanowar Elves");
    g.objects[elves.0 as usize].tapped = true;
    let r = hand(&mut g, 0, "Riverguard's Reflexes");
    play(&mut g, r, false, &[obj(elves)], &[]);
    assert_eq!((g.power(elves), g.toughness(elves)), (3, 3));
    assert!(g.keywords(elves).has(Keywords::FIRST_STRIKE) && !g.obj(elves).tapped);
}

#[test]
fn keep_out_hits_a_tapped_creature_or_destroys_an_enchantment() {
    let mut g = table();
    let tapped = put(&mut g, 1, "Savannah Lions");
    let untapped = put(&mut g, 1, "Llanowar Elves");
    g.objects[tapped.0 as usize].tapped = true;
    let k = hand(&mut g, 0, "Keep Out");
    act(&mut g, Action::CastModal(k, 0));
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Target(obj(tapped))) && !legal.contains(&Action::Target(obj(untapped))));
    act(&mut g, Action::Target(obj(tapped)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(tapped).zone, Zone::Graveyard);
    let tide = put(&mut g, 1, "Inexorable Tide");
    let k = hand(&mut g, 0, "Keep Out");
    act(&mut g, Action::CastModal(k, 1));
    act(&mut g, Action::Target(obj(tide)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(tide).zone, Zone::Graveyard);
}

#[test]
fn terminate_sear_and_boulder_dash() {
    let mut g = table();
    let a = put(&mut g, 1, "Savannah Lions");
    let t = hand(&mut g, 0, "Terminate");
    play(&mut g, t, false, &[obj(a)], &[]);
    assert_eq!(g.obj(a).zone, Zone::Graveyard);
    let b = put(&mut g, 1, "Savannah Lions");
    let sear = hand(&mut g, 0, "Sear");
    act(&mut g, Action::Cast(sear));
    assert!(
        !g.legal_actions().contains(&Action::Target(Target::Player(1))),
        "no players"
    );
    act(&mut g, Action::Target(obj(b)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(b).zone, Zone::Graveyard);
    let c = put(&mut g, 1, "Savannah Lions");
    let life = g.players[1].life;
    let dash = hand(&mut g, 0, "Boulder Dash");
    play(&mut g, dash, false, &[obj(c), Target::Player(1)], &[]);
    assert_eq!(g.obj(c).zone, Zone::Graveyard, "2 damage");
    assert_eq!(g.players[1].life, life - 1, "1 damage to the other target");
}

#[test]
fn goatnap_steals_untaps_and_hastes() {
    let mut g = table();
    let theirs = put(&mut g, 1, "Savannah Lions");
    g.objects[theirs.0 as usize].tapped = true;
    let gn = hand(&mut g, 0, "Goatnap");
    play(&mut g, gn, false, &[obj(theirs)], &[]);
    assert_eq!(g.obj(theirs).controller, 0);
    assert!(!g.obj(theirs).tapped && g.keywords(theirs).has(Keywords::HASTE));
    assert_eq!(g.power(theirs), 2, "not a Goat");
}

#[test]
fn crib_swap_exiles_and_gives_its_controller_a_changeling() {
    let mut g = table();
    let theirs = put(&mut g, 1, "Savannah Lions");
    let cs = hand(&mut g, 0, "Crib Swap");
    assert!(g.def_of(cs).changeling, "every creature type");
    play(&mut g, cs, false, &[obj(theirs)], &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Exile);
    let shifters = tokens(&g, 1, "Shapeshifter");
    assert_eq!(shifters.len(), 1, "its controller's");
    let x = shifters[0];
    assert_eq!((g.power(x), g.toughness(x)), (1, 1));
    assert!(g.has_subtype(x, "Goat") && g.colors_of(x) == [false; 5]);
}

#[test]
fn kinscaer_sentry_puts_a_cheap_creature_in_attacking() {
    let mut g = table();
    let sentry = put(&mut g, 0, "Kinscaer Sentry");
    let cheap = hand(&mut g, 0, "Llanowar Elves");
    let big = hand(&mut g, 0, "Pippin, Guard of the Citadel");
    g.start_turn(5, 0);
    attack_until_decision(&mut g, sentry);
    let legal = g.legal_actions();
    assert!(
        legal.contains(&Action::Pick(cheap)) && !legal.contains(&Action::Pick(big)),
        "X is 1"
    );
    act(&mut g, Action::Pick(cheap));
    assert_eq!(g.obj(cheap).zone, Zone::Battlefield);
    assert!(g.obj(cheap).tapped);
    assert!(
        g.combat.as_ref().is_some_and(|c| c.attackers.contains(&cheap)),
        "attacking"
    );
}

#[test]
fn alesha_returns_a_small_creature_attacking_if_you_pay() {
    let mut g = table();
    let alesha = put(&mut g, 0, "Alesha, Who Smiles at Death");
    let elves = g.add_card(0, cid("Llanowar Elves"), Zone::Graveyard);
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(alesha));
    act(&mut g, Action::FinishAttacks);
    while !matches!(g.pending, Pending::ChooseTarget(0) | Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    resolve_all(&mut g, &[Action::Target(obj(elves)), Action::Yes]);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
    assert!(g.combat.as_ref().is_some_and(|c| c.attackers.contains(&elves)));
}

#[test]
fn ride_the_shoopuf_grows_creatures_and_becomes_a_beast() {
    let mut g = table();
    let ride = put(&mut g, 0, "Ride the Shoopuf");
    let elves = put(&mut g, 0, "Llanowar Elves");
    let forest = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(forest));
    resolve_all(&mut g, &[Action::Target(obj(elves))]);
    assert_eq!(g.obj(elves).counters, 1, "landfall");
    act(&mut g, Action::Activate(ride, 0));
    resolve_all(&mut g, &[]);
    assert!(g.types_of(ride).creature && g.types_of(ride).enchantment);
    assert_eq!((g.power(ride), g.toughness(ride)), (7, 7));
    assert!(g.has_subtype(ride, "Beast"));
    g.start_turn(5, 0);
    assert!(g.types_of(ride).creature, "for as long as it stays");
}

#[test]
fn kodama_puts_a_cheaper_permanent_but_not_from_its_own_put() {
    let mut g = table();
    put(&mut g, 0, "Kodama of the East Tree");
    let elves = hand(&mut g, 0, "Llanowar Elves");
    let lions = hand(&mut g, 0, "Savannah Lions");
    let pippin = hand(&mut g, 0, "Pippin, Guard of the Citadel");
    act(&mut g, Action::Cast(elves));
    for _ in 0..10 {
        if g.pending == Pending::Decide(0) {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    // Llanowar Elves (1) entered: a 1-drop may follow, not Pippin (2).
    assert_eq!(g.pending, Pending::Decide(0));
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(lions)) && !legal.contains(&Action::Pick(pippin)));
    act(&mut g, Action::Pick(lions));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield);
    assert!(g.obj(lions).put_by_kodama);
    assert_ne!(g.pending, Pending::Decide(0), "the Lions don't trigger it again");
}

#[test]
fn noggle_the_mind_makes_a_colorless_1_1_noggle() {
    let mut g = table();
    let elder = put(&mut g, 1, "Merfolk Looter");
    let n = hand(&mut g, 0, "Noggle the Mind");
    play(&mut g, n, false, &[obj(elder)], &[]);
    assert_eq!((g.power(elder), g.toughness(elder)), (1, 1));
    assert_eq!(g.colors_of(elder), [false; 5]);
    assert!(g.has_subtype(elder, "Noggle") && !g.has_subtype(elder, "Merfolk"));
}

#[test]
fn maskwood_nexus_makes_your_creatures_every_type_and_shapeshifters() {
    let mut g = table();
    let nexus = put(&mut g, 0, "Maskwood Nexus");
    let elves = put(&mut g, 0, "Llanowar Elves");
    let in_hand = hand(&mut g, 0, "Savannah Lions");
    let theirs = put(&mut g, 1, "Llanowar Elves");
    assert!(g.has_subtype(elves, "Dragon") && g.has_subtype(in_hand, "Goblin"));
    assert!(!g.has_subtype(theirs, "Dragon"), "yours only");
    assert!(!g.has_subtype(nexus, "Dragon"), "creatures only");
    act(&mut g, Action::Activate(nexus, 0));
    resolve_all(&mut g, &[]);
    let shifters = tokens(&g, 0, "Shapeshifter");
    assert_eq!(shifters.len(), 1);
    let x = shifters[0];
    assert_eq!((g.power(x), g.toughness(x)), (2, 2));
    assert_eq!(g.colors_of(x), [false, true, false, false, false]);
}

#[test]
fn jin_gitaxias_progress_tyrant_copies_yours_and_counters_theirs_once_each_turn() {
    let mut g = table();
    put(&mut g, 0, "Jin-Gitaxias, Progress Tyrant");
    let life = g.players[1].life;
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].life, life - 6, "the Bolt and its copy");
    let second = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(second));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 9, "once each turn: no second copy");
    // The opponent's first artifact, instant or sorcery spell each turn is countered.
    g.start_turn(4, 1);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let theirs = hand(&mut g, 1, "Sol Ring");
    act(&mut g, Action::Cast(theirs));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "countered");
    let again = hand(&mut g, 1, "Sol Ring");
    act(&mut g, Action::Cast(again));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(again).zone, Zone::Battlefield, "once each turn");
}

#[test]
fn silence_stops_the_opponents_spells_this_turn() {
    let mut g = table();
    let s = hand(&mut g, 0, "Silence");
    play(&mut g, s, false, &[], &[]);
    assert_eq!(g.no_more_spells_turn[1], g.turn);
    assert_ne!(g.no_more_spells_turn[0], g.turn, "yours still");
}

#[test]
fn drannith_magistrate_stops_casting_from_anywhere_but_the_hand() {
    let mut g = table();
    put(&mut g, 0, "Drannith Magistrate");
    g.start_turn(4, 1);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let turn = g.turn;
    let grave = g.add_card(1, cid("Llanowar Elves"), Zone::Graveyard);
    g.objects[grave.0 as usize].playable = Some((1, turn));
    let from_hand = hand(&mut g, 1, "Llanowar Elves");
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Cast(from_hand)));
    assert!(!legal.contains(&Action::Cast(grave)), "not from the graveyard");
}

#[test]
fn vizier_of_remedies_puts_one_fewer_minus_counter_on_yours() {
    let mut g = table();
    put(&mut g, 0, "Vizier of Remedies");
    let mine = put(&mut g, 0, "Pippin, Guard of the Citadel");
    let theirs = put(&mut g, 1, "Pippin, Guard of the Citadel");
    // Proliferate a -1/-1 counter on each: yours gets none (one minus one).
    g.objects[mine.0 as usize].counters = -1;
    g.objects[theirs.0 as usize].counters = -1;
    let karn = put(&mut g, 0, "Karn's Bastion");
    act(&mut g, Action::Activate(karn, 0));
    resolve_all(&mut g, &[Action::Pick(mine), Action::Pick(theirs), Action::Yes]);
    assert_eq!(g.obj(mine).counters, -1, "yours: one fewer, none");
    assert_eq!(
        g.obj(theirs).zone,
        Zone::Graveyard,
        "theirs: a second -1/-1 counter kills the 2/2"
    );
}

#[test]
fn mental_misstep_counters_only_mana_value_one() {
    let mut g = table();
    let ring = hand(&mut g, 0, "Sol Ring");
    act(&mut g, Action::Cast(ring));
    let mm = hand(&mut g, 0, "Mental Misstep");
    act(&mut g, Action::Cast(mm));
    act(&mut g, Action::Target(obj(ring)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard, "countered");
    let pippin = hand(&mut g, 0, "Pippin, Guard of the Citadel");
    act(&mut g, Action::Cast(pippin));
    let mm = hand(&mut g, 0, "Mental Misstep");
    assert!(
        !g.legal_actions().contains(&Action::Cast(mm)),
        "no mana value 1 spell to target"
    );
}

#[test]
fn spell_pierce_and_dismember() {
    let mut g = table();
    let ring = hand(&mut g, 0, "Sol Ring");
    act(&mut g, Action::Cast(ring));
    let sp = hand(&mut g, 0, "Spell Pierce");
    act(&mut g, Action::Cast(sp));
    act(&mut g, Action::Target(obj(ring)));
    resolve_all(&mut g, &[Action::No]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard, "didn't pay");
    let theirs = put(&mut g, 1, "Savannah Lions");
    let dm = hand(&mut g, 0, "Dismember");
    play(&mut g, dm, false, &[obj(theirs)], &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
}

#[test]
fn opposition_agent_exiles_what_the_opponent_finds_for_you_to_play() {
    let mut g = table();
    put(&mut g, 0, "Opposition Agent");
    let wilds = put(&mut g, 1, "Evolving Wilds");
    g.start_turn(4, 1);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let forest = g.add_card(1, cid("Forest"), Zone::Library); // after the draw
    act(&mut g, Action::Activate(wilds, 0));
    resolve_all(&mut g, &[Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Exile, "exiled instead");
    assert_eq!(g.obj(forest).playable.map(|p| p.0), Some(0), "you may play it");
}

#[test]
fn ram_through_tramples_over() {
    let mut g = table();
    let big = put(&mut g, 0, "Gigantosaurus"); // 10/10, no trample
    let theirs = put(&mut g, 1, "Llanowar Elves");
    let life = g.players[1].life;
    let r = hand(&mut g, 0, "Ram Through");
    play(&mut g, r, false, &[obj(big), obj(theirs)], &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.players[1].life, life, "no trample, no excess");
    let other = put(&mut g, 1, "Llanowar Elves");
    let t = hand(&mut g, 0, "Ram Through");
    g.objects[big.0 as usize].granted = g.objects[big.0 as usize].granted.with(Keywords::TRAMPLE);
    play(&mut g, t, false, &[obj(big), obj(other)], &[]);
    assert_eq!(g.obj(other).zone, Zone::Graveyard);
    assert_eq!(g.players[1].life, life - 9, "trample: 1 to the Elves, 9 over");
}

#[test]
fn scale_up_and_its_overload() {
    let mut g = table();
    let a = put(&mut g, 0, "Llanowar Elves");
    let b = put(&mut g, 0, "Savannah Lions");
    let s = hand(&mut g, 0, "Scale Up");
    play(&mut g, s, false, &[obj(a)], &[]);
    assert_eq!((g.power(a), g.toughness(a)), (6, 4));
    assert!(g.has_subtype(a, "Wurm"));
    assert_eq!((g.power(b), g.toughness(b)), (2, 1));
    let s = hand(&mut g, 0, "Scale Up");
    act(&mut g, Action::CastAlternative(s));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(b), g.toughness(b)), (6, 4), "overload: each");
    g.start_turn(5, 0);
    assert_eq!(g.power(a), 1, "until end of turn");
}

#[test]
fn roiling_regrowth_and_planar_engineering_trade_lands() {
    let mut g = table();
    let land = g.permanents(0).find(|&x| g.types_of(x).land).unwrap();
    let lands = g.permanents(0).filter(|&x| g.types_of(x).land).count();
    let f1 = g.add_card(0, cid("Forest"), Zone::Library);
    let f2 = g.add_card(0, cid("Forest"), Zone::Library);
    let r = hand(&mut g, 0, "Roiling Regrowth");
    act(&mut g, Action::Cast(r));
    resolve_all(&mut g, &[Action::Pick(land), Action::Pick(f1), Action::Pick(f2)]);
    assert_eq!(g.obj(land).zone, Zone::Graveyard);
    assert!(g.obj(f1).tapped && g.obj(f2).zone == Zone::Battlefield);
    assert_eq!(g.permanents(0).filter(|&x| g.types_of(x).land).count(), lands + 1);
}

#[test]
fn entish_restoration_finds_three_with_a_big_creature() {
    let mut g = table();
    put(&mut g, 0, "Gigantosaurus"); // power 10
    let land = g.permanents(0).find(|&x| g.types_of(x).land).unwrap();
    let fs: Vec<ObjId> = (0..3)
        .map(|_| g.add_card(0, cid("Forest"), Zone::Library))
        .collect();
    let e = hand(&mut g, 0, "Entish Restoration");
    act(&mut g, Action::Cast(e));
    resolve_all(
        &mut g,
        &[
            Action::Pick(land),
            Action::Pick(fs[0]),
            Action::Pick(fs[1]),
            Action::Pick(fs[2]),
        ],
    );
    assert!(fs.iter().all(|&f| g.obj(f).zone == Zone::Battlefield), "three");
}

#[test]
fn archdruids_charm_modes() {
    let mut g = table();
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    let c = hand(&mut g, 0, "Archdruid's Charm");
    act(&mut g, Action::CastModal(c, 0));
    resolve_all(&mut g, &[Action::Pick(forest)]);
    assert!(
        g.obj(forest).zone == Zone::Battlefield && g.obj(forest).tapped,
        "a land: tapped"
    );
    let mine = put(&mut g, 0, "Savannah Lions");
    let theirs = put(&mut g, 1, "Savannah Lions"); // no ward
    for x in g.permanents(0).collect::<Vec<_>>() {
        g.objects[x.0 as usize].tapped = false; // mana for the next {G}{G}{G}
    }
    let c = hand(&mut g, 0, "Archdruid's Charm");
    act(&mut g, Action::CastModal(c, 1));
    act(&mut g, Action::Target(obj(mine)));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(mine).counters, 1);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "3 damage to the 2/1");
    let tide = put(&mut g, 1, "Inexorable Tide");
    for x in g.permanents(0).collect::<Vec<_>>() {
        g.objects[x.0 as usize].tapped = false; // mana for the next {G}{G}{G}
    }
    let c = hand(&mut g, 0, "Archdruid's Charm");
    act(&mut g, Action::CastModal(c, 2));
    act(&mut g, Action::Target(obj(tide)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(tide).zone, Zone::Exile);
}

#[test]
fn ad_nauseam_draws_and_pays_life_until_you_stop() {
    let mut g = table();
    let a = g.add_card(0, cid("Lightning Bolt"), Zone::Library);
    let b = g.add_card(0, cid("Pippin, Guard of the Citadel"), Zone::Library);
    let life = g.players[0].life;
    let ad = hand(&mut g, 0, "Ad Nauseam");
    act(&mut g, Action::Cast(ad));
    resolve_all(&mut g, &[Action::Yes, Action::No]);
    assert_eq!((g.obj(b).zone, g.obj(a).zone), (Zone::Hand, Zone::Hand));
    assert_eq!(g.players[0].life, life - 3, "2 for Pippin, 1 for the Bolt");
}

#[test]
fn vraska_betrayals_sting_draws_proliferates_and_makes_a_treasure() {
    let mut g = table();
    let v = put(&mut g, 0, "Vraska, Betrayal's Sting");
    g.objects[v.0 as usize].loyalty = 6;
    let (life, cards) = (g.players[0].life, g.players[0].hand.len());
    act(&mut g, Action::Loyalty(v, 0));
    resolve_all(&mut g, &[Action::Pick(v), Action::Yes]);
    assert_eq!(
        (g.players[0].life, g.players[0].hand.len()),
        (life - 1, cards + 1)
    );
    assert_eq!(g.obj(v).loyalty, 7, "proliferated");
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let theirs = put(&mut g, 1, "Llanowar Elves");
    act(&mut g, Action::Loyalty(v, 1));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert!(g.treasure_form(theirs) && !g.types_of(theirs).creature);
    assert_eq!(g.obj(v).loyalty, 5);
}

// ---- d3 core, batch 38 (Raffine) ----

#[test]
fn mausoleum_wanderer_grows_with_spirits_and_counters_unless_they_pay_its_power() {
    let mut g = table();
    let w = put(&mut g, 0, "Mausoleum Wanderer");
    let sailor = hand(&mut g, 0, "Spectral Sailor");
    act(&mut g, Action::Cast(sailor));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(w), g.toughness(w)), (2, 2), "another Spirit entered");
    let creature = hand(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::Cast(creature));
    assert!(
        !g.legal_actions().contains(&Action::Activate(w, 0)),
        "a creature spell isn't an instant or sorcery"
    );
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(w), 2, "the Lions aren't a Spirit");
    let life = g.players[0].life;
    let nw = hand(&mut g, 0, "Night's Whisper");
    act(&mut g, Action::Cast(nw));
    act(&mut g, Action::Activate(w, 0));
    act(&mut g, Action::Target(obj(nw)));
    assert_eq!(g.obj(w).zone, Zone::Graveyard, "sacrificed as the cost");
    resolve_all(&mut g, &[Action::No]);
    assert_eq!(g.players[0].life, life, "didn't pay {{2}}: countered");
}

#[test]
fn skycat_sovereign_grows_per_other_flier_you_control_and_makes_cat_birds() {
    let mut g = table();
    let cat = put(&mut g, 0, "Skycat Sovereign");
    put(&mut g, 1, "Meloku the Clouded Mirror");
    put(&mut g, 0, "Savannah Lions");
    assert_eq!(
        (g.power(cat), g.toughness(cat)),
        (1, 1),
        "not theirs, not a nonflier"
    );
    put(&mut g, 0, "Spectral Sailor");
    assert_eq!((g.power(cat), g.toughness(cat)), (2, 2));
    act(&mut g, Action::Activate(cat, 0));
    resolve_all(&mut g, &[]);
    let birds = tokens(&g, 0, "Cat Bird");
    assert_eq!(birds.len(), 1);
    let b = birds[0];
    assert!(g.keywords(b).has(Keywords::FLYING));
    assert_eq!(g.def_of(b).colors, [true, false, false, false, false]);
    assert_eq!((g.power(b), g.toughness(b)), (1, 1));
    assert!(g.has_subtype(b, "Cat") && g.has_subtype(b, "Bird"));
    assert_eq!((g.power(cat), g.toughness(cat)), (3, 3), "the Cat Bird flies too");
}

#[test]
fn underdark_rift_rolls_a_d10_and_tucks_the_target_beneath_that_many() {
    let mut g = table();
    let rift = put(&mut g, 0, "Underdark Rift");
    let theirs = put(&mut g, 1, "Savannah Lions");
    for _ in 0..12 {
        g.add_card(1, cid("Forest"), Zone::Library);
    }
    act(&mut g, Action::Activate(rift, 0));
    act(&mut g, Action::Target(obj(theirs)));
    assert_eq!(g.obj(rift).zone, Zone::Exile, "exiled as the cost");
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Library);
    let lib = &g.players[1].library;
    let above = lib.len() - 1 - lib.iter().position(|&x| x == theirs).unwrap();
    assert!(
        (1..=10).contains(&above),
        "beneath the top X, X from 1 to 10: {above}"
    );
}

#[test]
fn crux_of_fate_destroys_the_dragons_or_everything_else() {
    for (mode, dragon_dies) in [(0, true), (1, false)] {
        let mut g = table();
        let dragon = put(&mut g, 1, "Shivan Dragon");
        let lions = put(&mut g, 0, "Savannah Lions");
        let crux = hand(&mut g, 0, "Crux of Fate");
        act(&mut g, Action::CastModal(crux, mode));
        resolve_all(&mut g, &[]);
        let dead = |x| g.obj(x).zone == Zone::Graveyard;
        assert_eq!(
            (dead(dragon), dead(lions)),
            (dragon_dies, !dragon_dies),
            "mode {mode}"
        );
    }
}

#[test]
fn apocalypse_exiles_all_permanents_and_you_discard_your_hand() {
    let mut g = table();
    let mine = put(&mut g, 0, "Sol Ring");
    let theirs = put(&mut g, 1, "Savannah Lions");
    let card = hand(&mut g, 0, "Savannah Lions");
    let theirs_in_hand = hand(&mut g, 1, "Lightning Bolt");
    let a = hand(&mut g, 0, "Apocalypse");
    act(&mut g, Action::Cast(a));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(mine).zone, g.obj(theirs).zone), (Zone::Exile, Zone::Exile));
    assert!(
        g.permanents(0).next().is_none() && g.permanents(1).next().is_none(),
        "lands too"
    );
    assert_eq!(g.obj(card).zone, Zone::Graveyard, "you discard");
    assert_eq!(g.obj(theirs_in_hand).zone, Zone::Hand, "only you");
    assert!(g.players[0].hand.is_empty());
}

#[test]
fn karrthus_takes_and_untaps_all_dragons_and_gives_them_haste() {
    let mut g = table();
    let theirs = put(&mut g, 1, "Shivan Dragon");
    g.objects[theirs.0 as usize].tapped = true;
    let mine = put(&mut g, 0, "Utvara Hellkite");
    g.objects[mine.0 as usize].tapped = true;
    let lions = put(&mut g, 1, "Savannah Lions");
    let k = hand(&mut g, 0, "Karrthus, Tyrant of Jund");
    act(&mut g, Action::Cast(k));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).controller, 0, "gained control");
    assert_eq!(g.obj(lions).controller, 1, "not a Dragon");
    assert!(!g.obj(theirs).tapped && !g.obj(mine).tapped, "untapped");
    assert!(g.keywords(theirs).has(Keywords::HASTE) && g.keywords(mine).has(Keywords::HASTE));
    assert!(!g.keywords(lions).has(Keywords::HASTE));
}

#[test]
fn vaevictis_makes_each_player_sacrifice_a_target_and_flip_a_permanent_onto_the_battlefield() {
    let mut g = table();
    let v = put(&mut g, 0, "Vaevictis Asmadi, the Dire");
    let ring = put(&mut g, 0, "Sol Ring");
    let theirs = put(&mut g, 1, "Savannah Lions");
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    let my_top = g.add_card(0, cid("Savannah Lions"), Zone::Library);
    let their_top = g.add_card(1, cid("Lightning Bolt"), Zone::Library);
    act(&mut g, Action::Attack(v));
    act(&mut g, Action::FinishAttacks);
    act(&mut g, Action::Target(obj(ring)));
    act(&mut g, Action::Target(obj(theirs)));
    while !g.items.is_empty() {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(
        (g.obj(ring).zone, g.obj(theirs).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
    assert_eq!(
        g.obj(my_top).zone,
        Zone::Battlefield,
        "a permanent card: onto the battlefield"
    );
    assert_eq!(g.obj(my_top).controller, 0);
    assert_eq!(g.obj(their_top).zone, Zone::Library, "an instant stays");
}

#[test]
fn kindred_summons_puts_x_creatures_of_the_chosen_type_onto_the_battlefield() {
    let mut g = table();
    put(&mut g, 0, "Shivan Dragon");
    put(&mut g, 0, "Utvara Hellkite");
    put(&mut g, 0, "Savannah Lions");
    let a = g.add_card(0, cid("Goldspan Dragon"), Zone::Library);
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Library);
    let b = g.add_card(0, cid("Shivan Dragon"), Zone::Library);
    let c = hand(&mut g, 0, "Kindred Summons");
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    let dragon = g
        .creature_types_known_to(0)
        .iter()
        .position(|t| t == "Dragon")
        .unwrap();
    resolve_all(&mut g, &[Action::Mode(dragon as u8)]);
    assert_eq!(
        (g.obj(a).zone, g.obj(b).zone),
        (Zone::Battlefield, Zone::Battlefield),
        "two Dragons you control: two"
    );
    assert_eq!(g.obj(lions).zone, Zone::Library, "not a Dragon: shuffled back");
}

#[test]
fn shadowspear_equips_and_strips_opponents_hexproof_and_indestructible() {
    let mut g = table();
    let spear = put(&mut g, 0, "Shadowspear");
    let lions = put(&mut g, 0, "Savannah Lions");
    let shield = Keywords::HEXPROOF.with(Keywords::INDESTRUCTIBLE);
    g.objects[lions.0 as usize].granted = shield;
    let theirs = put(&mut g, 1, "Savannah Lions");
    g.objects[theirs.0 as usize].granted = shield;
    let equip = g.def_of(spear).activated.len() - 1;
    act(&mut g, Action::Activate(spear, equip));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(lions), g.toughness(lions)), (3, 2));
    assert!(g.keywords(lions).has(Keywords::TRAMPLE.with(Keywords::LIFELINK)));
    act(&mut g, Action::Activate(spear, 0));
    resolve_all(&mut g, &[]);
    assert!(!g.keywords(theirs).has(Keywords::HEXPROOF) && !g.keywords(theirs).has(Keywords::INDESTRUCTIBLE));
    assert!(g.keywords(lions).has(shield), "yours keep them");
    g.start_turn(5, 1);
    assert!(g.keywords(theirs).has(shield), "until end of turn");
}

#[test]
fn brain_freeze_mills_three_per_copy() {
    let mut g = table();
    for _ in 0..12 {
        g.add_card(1, cid("Forest"), Zone::Library);
    }
    let before = g.players[1].graveyard.len();
    let ring = hand(&mut g, 0, "Sol Ring");
    play(&mut g, ring, false, &[], &[]);
    let bf = hand(&mut g, 0, "Brain Freeze");
    act(&mut g, Action::Cast(bf));
    act(&mut g, Action::Target(Target::Player(1)));
    // The storm copy (one spell before it) may choose a new target: keep the opponent.
    let mut n = 0;
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) => Action::Target(Target::Player(1)),
            Pending::Decide(_) => {
                n += 1;
                g.legal_actions()[0]
            }
            _ => Action::PassPriority,
        };
        act(&mut g, a);
        assert!(n < 10);
    }
    assert_eq!(
        g.players[1].graveyard.len() - before,
        6,
        "the spell and one storm copy"
    );
}

#[test]
fn blackblade_reforged_grows_per_land_and_equips_legends_cheaply() {
    let mut g = game(&[
        "Forest", "Forest", "Forest", "Forest", "Forest", "Forest", "Forest",
    ]);
    let blade = put(&mut g, 0, "Blackblade Reforged");
    let legend = put(&mut g, 0, "Pippin, Guard of the Citadel");
    let lions = put(&mut g, 0, "Savannah Lions");
    let base = g.power(legend);
    act(&mut g, Action::Activate(blade, 0));
    act(&mut g, Action::Target(obj(legend)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(legend), base + 7, "seven lands");
    act(&mut g, Action::Activate(blade, 0));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(lions))),
        "the {{3}} equip is for legendary creatures"
    );
}

#[test]
fn darksteel_mutation_makes_an_indestructible_0_1_insect_artifact_creature() {
    let mut g = table();
    let courser = put(&mut g, 1, "Courser of Kruphix");
    let lions = put(&mut g, 1, "Savannah Lions");
    let m = hand(&mut g, 0, "Darksteel Mutation");
    play(&mut g, m, false, &[obj(courser)], &[]);
    assert_eq!(g.obj(m).zone, Zone::Battlefield);
    let t = g.types_of(courser);
    assert!(t.artifact && t.creature && !t.enchantment && !t.land);
    assert_eq!((g.power(courser), g.toughness(courser)), (0, 1));
    assert!(g.has_subtype(courser, "Insect") && !g.has_subtype(courser, "Centaur"));
    assert!(g.keywords(courser).has(Keywords::INDESTRUCTIBLE));
    let wrath = hand(&mut g, 0, "Wrath of God");
    play(&mut g, wrath, false, &[], &[]);
    assert_eq!(g.obj(courser).zone, Zone::Battlefield, "indestructible");
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
}

#[test]
fn ashaya_makes_your_nontoken_creatures_forest_lands_and_counts_them() {
    let mut g = table();
    let ashaya = put(&mut g, 0, "Ashaya, Soul of the Wild");
    let elves = put(&mut g, 0, "Llanowar Elves");
    let lions = put(&mut g, 0, "Savannah Lions");
    let theirs = put(&mut g, 1, "Savannah Lions");
    // 25 lands, plus Ashaya, the Elves and the Lions.
    assert_eq!((g.power(ashaya), g.toughness(ashaya)), (28, 28));
    let t = g.types_of(lions);
    assert!(t.land && t.creature);
    assert!(g.has_subtype(lions, "Forest") && g.has_subtype(lions, "Cat"));
    assert!(
        !g.types_of(theirs).land && !g.has_subtype(theirs, "Forest"),
        "yours only"
    );
    to_main1(&mut g, 5, 0);
    assert_eq!(
        colors(&g, lions),
        [false, false, false, false, true],
        "a Forest taps for green"
    );
    assert_eq!(colors(&g, elves), [false, false, false, false, true]);
}

#[test]
fn lorans_escape_gives_hexproof_and_indestructible_then_scries() {
    let mut g = table();
    let ring = put(&mut g, 0, "Sol Ring");
    let top = g.add_card(0, cid("Counterspell"), Zone::Library);
    let e = hand(&mut g, 0, "Loran's Escape");
    play(&mut g, e, false, &[obj(ring)], &[Action::Yes]); // the top card to the bottom
    let k = g.keywords(ring);
    assert!(k.has(Keywords::HEXPROOF) && k.has(Keywords::INDESTRUCTIBLE));
    assert_eq!(g.players[0].library[0], top, "scried to the bottom");
}

#[test]
fn kenriths_transformation_draws_and_makes_a_green_3_3_elk() {
    let mut g = table();
    let courser = put(&mut g, 1, "Courser of Kruphix");
    let k = hand(&mut g, 0, "Kenrith's Transformation");
    let cards = g.players[0].hand.len();
    play(&mut g, k, false, &[obj(courser)], &[]);
    assert_eq!(g.players[0].hand.len(), cards, "cast one, drew one");
    let t = g.types_of(courser);
    assert!(t.creature && !t.enchantment);
    assert_eq!((g.power(courser), g.toughness(courser)), (3, 3));
    assert_eq!(g.colors_of(courser), [false, false, false, false, true]);
    assert!(g.has_subtype(courser, "Elk") && !g.has_subtype(courser, "Centaur"));
}

#[test]
fn the_battle_of_bywater_destroys_the_big_ones_then_makes_food() {
    let mut g = table();
    let lions = put(&mut g, 0, "Savannah Lions");
    let courser = put(&mut g, 0, "Courser of Kruphix");
    let angel = put(&mut g, 1, "Serra Angel");
    let b = hand(&mut g, 0, "The Battle of Bywater");
    play(&mut g, b, false, &[], &[]);
    assert_eq!(g.obj(angel).zone, Zone::Graveyard, "power 4");
    assert_eq!(
        (g.obj(lions).zone, g.obj(courser).zone),
        (Zone::Battlefield, Zone::Battlefield)
    );
    assert_eq!(tokens(&g, 0, "Food").len(), 2, "one per creature you control");
}

#[test]
fn dakkon_blackblade_counts_your_lands() {
    let mut g = table();
    let d = put(&mut g, 0, "Dakkon Blackblade");
    put(&mut g, 1, "Forest");
    assert_eq!((g.power(d), g.toughness(d)), (25, 25));
}

#[test]
fn scion_of_oona_pumps_and_shrouds_other_faeries() {
    let mut g = table();
    let scion = put(&mut g, 0, "Scion of Oona");
    let cloud = put(&mut g, 0, "Cloud of Faeries");
    let lions = put(&mut g, 0, "Savannah Lions");
    assert_eq!((g.power(cloud), g.toughness(cloud)), (2, 2));
    assert!(g.keywords(cloud).has(Keywords::SHROUD));
    assert_eq!((g.power(scion), g.toughness(scion)), (1, 1), "other Faeries only");
    assert!(!g.keywords(scion).has(Keywords::SHROUD));
    assert_eq!(g.power(lions), 2);
}

#[test]
fn sword_of_light_and_shadow_protects_gains_and_returns_a_creature() {
    let mut g = table();
    let sword = put(&mut g, 0, "Sword of Light and Shadow");
    let looter = put(&mut g, 0, "Merfolk Looter");
    let dead = g.add_card(0, cid("Savannah Lions"), Zone::Graveyard);
    to_main1(&mut g, 5, 0);
    act(&mut g, Action::Activate(sword, 0));
    act(&mut g, Action::Target(obj(looter)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(looter), g.toughness(looter)), (3, 3));
    assert_eq!(g.protection(looter), [true, false, true, false, false]);
    let life = g.players[0].life;
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(looter));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::ChooseTarget(0) => Action::Target(obj(dead)),
            Pending::Decide(1) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[1].life, 37);
    assert_eq!(g.players[0].life, life + 3);
    assert_eq!(g.obj(dead).zone, Zone::Hand);
}

#[test]
fn reckless_endeavor_rolls_two_d12_for_damage_and_treasures() {
    let mut g = table();
    let lions = put(&mut g, 1, "Savannah Lions");
    let r = hand(&mut g, 0, "Reckless Endeavor");
    act(&mut g, Action::Cast(r));
    while !g.items.is_empty() && !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    let treasures = match g.decision.clone() {
        Some(bolt_core::game::Decision::ChooseDamageRoll { rolls, .. }) => {
            assert_eq!(g.legal_actions(), vec![Action::Mode(0), Action::Mode(1)]);
            act(&mut g, Action::Mode(1)); // the second result is the damage
            rolls[0] as usize
        }
        _ => tokens(&g, 0, "Treasure").len(), // a tie: no choice
    };
    resolve_all(&mut g, &[]);
    assert!((1..=12).contains(&treasures));
    assert_eq!(tokens(&g, 0, "Treasure").len(), treasures);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard, "at least 1 damage");
}

#[test]
fn garruk_primal_hunter_makes_beasts_draws_and_makes_wurms() {
    let mut g = table();
    let garruk = hand(&mut g, 0, "Garruk, Primal Hunter");
    play(&mut g, garruk, false, &[], &[]);
    act(&mut g, Action::Loyalty(garruk, 0));
    resolve_all(&mut g, &[]);
    let beasts = tokens(&g, 0, "Beast");
    assert_eq!(beasts.len(), 1);
    assert_eq!((g.power(beasts[0]), g.toughness(beasts[0])), (3, 3));
    to_main1(&mut g, 5, 0);
    let cards = g.players[0].hand.len();
    act(&mut g, Action::Loyalty(garruk, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), cards + 3, "the Beast's power");
    g.objects[garruk.0 as usize].loyalty = 6;
    to_main1(&mut g, 7, 0);
    act(&mut g, Action::Loyalty(garruk, 2));
    resolve_all(&mut g, &[]);
    let wurms = tokens(&g, 0, "Wurm");
    assert_eq!(wurms.len(), 25);
    assert_eq!((g.power(wurms[0]), g.toughness(wurms[0])), (6, 6));
    assert_eq!(g.colors_of(wurms[0]), [false, false, false, false, true]);
}

#[test]
fn zur_the_enchanter_finds_a_small_enchantment_on_attack() {
    let mut g = table();
    let zur = put(&mut g, 0, "Zur the Enchanter");
    to_main1(&mut g, 5, 0);
    let small = g.add_card(0, cid("Darksteel Mutation"), Zone::Library);
    let big = g.add_card(0, cid("Greed"), Zone::Library);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(zur));
    act(&mut g, Action::FinishAttacks);
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Yes); // search
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(small)) && !legal.contains(&Action::Pick(big)));
    act(&mut g, Action::Pick(small));
    assert_eq!(g.obj(small).zone, Zone::Battlefield);
}

#[test]
fn nim_deathmantle_makes_a_black_zombie_and_brings_back_the_dead_for_4() {
    let mut g = table();
    let nim = put(&mut g, 0, "Nim Deathmantle");
    let lions = put(&mut g, 0, "Savannah Lions");
    to_main1(&mut g, 5, 0);
    act(&mut g, Action::Activate(nim, 0));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(lions), g.toughness(lions)), (4, 3));
    assert!(g.keywords(lions).has(Keywords::INTIMIDATE));
    assert_eq!(g.colors_of(lions), [false, false, true, false, false]);
    assert!(g.has_subtype(lions, "Zombie") && !g.has_subtype(lions, "Cat"));
    for _ in 0..4 {
        put(&mut g, 0, "Plains"); // untapped white for the Wraths
    }
    let wrath = hand(&mut g, 0, "Wrath of God");
    play(&mut g, wrath, false, &[], &[Action::Yes]); // pay {4}
    assert_eq!(g.obj(lions).zone, Zone::Battlefield, "returned");
    let zcc = g.obj(lions).zcc;
    assert_eq!(g.obj(nim).attached_to, Some((lions, zcc)), "and equipped");
    assert_eq!(g.power(lions), 4);
    // Declined: it stays in the graveyard.
    let wrath = hand(&mut g, 0, "Wrath of God");
    play(&mut g, wrath, false, &[], &[Action::No]);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
}

/// Attacks with `attacker` in the first combat (declaring no attackers in any extra combat), to the
/// second main phase; returns how many declare-attackers steps player 0 got.
fn attack_once_count_combats(g: &mut Game, attacker: ObjId) -> usize {
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    act(g, Action::Attack(attacker));
    act(g, Action::FinishAttacks);
    let mut combats = 1;
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareAttackers(0) => {
                combats += 1;
                Action::FinishAttacks
            }
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::Decide(1) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(g, a);
    }
    combats
}

#[test]
fn scourge_of_valkas_burns_for_each_dragon_as_dragons_enter() {
    let mut g = table();
    put(&mut g, 0, "Atarka, World Render");
    let v = hand(&mut g, 0, "Scourge of Valkas");
    play(&mut g, v, false, &[], &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].life, 38, "two Dragons");
    put(&mut g, 0, "Mountain");
    let t = hand(&mut g, 0, "Scourge of the Throne"); // not legendary: no legend rule
    play(&mut g, t, false, &[], &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].life, 35, "another Dragon: three");
    act(&mut g, Action::Activate(v, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(v), 5);
}

#[test]
fn atarka_gives_attacking_dragons_double_strike() {
    let mut g = table();
    put(&mut g, 0, "Atarka, World Render");
    let v = put(&mut g, 0, "Scourge of Valkas");
    to_main1(&mut g, 5, 0);
    attack_once_count_combats(&mut g, v);
    assert_eq!(g.players[1].life, 32, "4 double strike");
}

#[test]
fn lathliss_makes_a_dragon_for_each_nontoken_dragon_and_pumps_dragons() {
    let mut g = table();
    let l = put(&mut g, 0, "Lathliss, Dragon Queen");
    let lions = hand(&mut g, 0, "Savannah Lions");
    play(&mut g, lions, false, &[], &[]);
    assert!(tokens(&g, 0, "Dragon").is_empty(), "not a Dragon");
    let a = hand(&mut g, 0, "Atarka, World Render");
    play(&mut g, a, false, &[], &[]);
    let dragons = tokens(&g, 0, "Dragon");
    assert_eq!(dragons.len(), 1, "not again for the token");
    assert_eq!((g.power(dragons[0]), g.toughness(dragons[0])), (5, 5));
    assert_eq!(
        g.colors_of(dragons[0]),
        [false, false, false, true, false],
        "a red Dragon"
    );
    act(&mut g, Action::Activate(l, 0));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(a), g.power(dragons[0]), g.power(l)), (7, 6, 7));
}

#[test]
fn scourge_of_the_throne_dethrones_and_attacks_again() {
    let mut g = table();
    let s = put(&mut g, 0, "Scourge of the Throne");
    to_main1(&mut g, 5, 0);
    assert_eq!(attack_once_count_combats(&mut g, s), 2, "an additional combat");
    assert_eq!(g.obj(s).counters, 1, "dethrone");
    assert!(!g.obj(s).tapped, "untapped");
    let mut g = table();
    let s = put(&mut g, 0, "Scourge of the Throne");
    to_main1(&mut g, 5, 0);
    g.players[1].life = 30;
    assert_eq!(attack_once_count_combats(&mut g, s), 1, "not the most life");
    assert_eq!(g.obj(s).counters, 0);
}

/// Broodcaller Scourge's put-from-hand, on a stand-in trigger until d2's damage amount lands: the
/// limit is the trigger's `amount`, the pick is the player's, and Kodama still sees the card enter.
fn broodcaller_put(amount: i32) -> (Game, [ObjId; 3]) {
    use bolt_core::effect::{Ability, Effect, Trigger, When};
    use bolt_core::game::{StackItem, StackKind};
    use std::sync::Arc;
    let mut g = table();
    let carrier = put(&mut g, 0, "Savannah Lions");
    let def = g.obj(carrier).def;
    Arc::make_mut(&mut g.defs)[def].triggers.insert(
        0,
        Trigger {
            when: When::YourSubtypeDealCombatDamageToPlayer("Dragon"),
            ability: Ability::new(vec![], vec![Effect::MayPutPermanentFromHandMvAtMostAmount]),
        },
    );
    let elves = hand(&mut g, 0, "Llanowar Elves");
    let pippin = hand(&mut g, 0, "Pippin, Guard of the Citadel");
    let kodama = hand(&mut g, 0, "Kodama of the East Tree");
    g.triggered.push(StackItem {
        source: carrier,
        controller: 0,
        kind: StackKind::Trigger(0),
        targets: Vec::new(),
        kicked: false,
        flashback: false,
        extra: None,
        mode: None,
        amount,
        alt: false,
        x: 0,
        division: Vec::new(),
        source_zcc: None,
    });
    for _ in 0..6 {
        if matches!(g.pending, Pending::Decide(_)) || g.triggered.is_empty() && g.items.is_empty() {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    (g, [elves, pippin, kodama])
}

#[test]
fn broodcaller_puts_a_permanent_with_mana_value_up_to_the_damage() {
    let (mut g, [elves, pippin, kodama]) = broodcaller_put(2);
    assert_eq!(g.pending, Pending::Decide(0));
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(elves)) && legal.contains(&Action::Pick(pippin)));
    assert!(!legal.contains(&Action::Pick(kodama)), "mana value 5 is over 2");
    assert!(legal.contains(&Action::No), "a may");
    act(&mut g, Action::Pick(pippin));
    assert_eq!(g.obj(pippin).zone, Zone::Battlefield);
    assert!(!g.obj(pippin).put_by_kodama, "Kodama can still trigger on it");
    assert_eq!(g.obj(elves).zone, Zone::Hand);
}

#[test]
fn broodcaller_may_decline_and_asks_nothing_without_damage() {
    let (mut g, [elves, pippin, _]) = broodcaller_put(2);
    assert_eq!(g.pending, Pending::Decide(0));
    act(&mut g, Action::No);
    assert_eq!((g.obj(elves).zone, g.obj(pippin).zone), (Zone::Hand, Zone::Hand));
    let (g, [elves, ..]) = broodcaller_put(0);
    assert!(!matches!(g.pending, Pending::Decide(_)));
    assert_eq!(g.obj(elves).zone, Zone::Hand);
}

// ---- d3 batch 45: Veil of Summer, Allosaurus Shepherd, Finale of Devastation, Song of Totentanz,
// Urza's {5} ----

/// Player 0 casts `first` (with Veil of Summer resolved first if `veil`) and passes; player 1 then
/// casts `answer`. Returns the game with player 1 choosing a target.
fn veil_then_opponent_answers(veil: bool, first: &str, answer: &str) -> (Game, ObjId) {
    let mut g = table();
    if veil {
        let v = hand(&mut g, 0, "Veil of Summer");
        play(&mut g, v, false, &[], &[]);
    }
    let f = hand(&mut g, 0, first);
    act(&mut g, Action::Cast(f));
    while g.pending != Pending::Priority(1) {
        act(&mut g, Action::PassPriority);
    }
    let a = hand(&mut g, 1, answer);
    act(&mut g, Action::Cast(a));
    (g, f)
}

#[test]
fn veil_of_summer_gives_you_and_yours_hexproof_from_blue_and_black() {
    for veil in [false, true] {
        let mut g = table();
        let lions = put(&mut g, 0, "Savannah Lions");
        if veil {
            let v = hand(&mut g, 0, "Veil of Summer");
            play(&mut g, v, false, &[], &[]);
        }
        let div = hand(&mut g, 0, "Night's Whisper");
        act(&mut g, Action::Cast(div));
        while g.pending != Pending::Priority(1) {
            act(&mut g, Action::PassPriority);
        }
        let db = hand(&mut g, 1, "Doom Blade");
        // Doom Blade is black: with Veil it has no legal target, so it can't be cast.
        assert_eq!(
            g.legal_actions().contains(&Action::Cast(db)),
            !veil,
            "veil: {veil}"
        );
        if !veil {
            act(&mut g, Action::Cast(db));
            assert!(can_target(&g, lions));
        }
    }
    let mut g = table();
    let lions = put(&mut g, 0, "Savannah Lions");
    let v = hand(&mut g, 0, "Veil of Summer");
    play(&mut g, v, false, &[], &[]);
    let div = hand(&mut g, 0, "Night's Whisper");
    act(&mut g, Action::Cast(div));
    while g.pending != Pending::Priority(1) {
        act(&mut g, Action::PassPriority);
    }
    let bolt = hand(&mut g, 1, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    assert!(can_target(&g, lions), "a red spell still can");
}

#[test]
fn veil_of_summer_spells_you_control_cant_be_countered() {
    for veil in [false, true] {
        let (mut g, div) = veil_then_opponent_answers(veil, "Night's Whisper", "Counterspell");
        let before = g.players[0].hand.len();
        act(&mut g, Action::Target(obj(div)));
        resolve_all(&mut g, &[]);
        let drawn = g.players[0].hand.len() - before;
        assert_eq!(drawn, if veil { 2 } else { 0 }, "veil: {veil}");
    }
}

#[test]
fn veil_of_summer_draws_only_if_an_opponent_cast_blue_or_black() {
    for (color, draws) in [(None, 0), (Some(0), 0), (Some(1), 1), (Some(2), 1)] {
        let mut g = table();
        if let Some(k) = color {
            g.color_cast_turn[1][k] = g.turn;
        }
        let v = hand(&mut g, 0, "Veil of Summer");
        let before = g.players[0].hand.len();
        play(&mut g, v, false, &[], &[]);
        assert_eq!(g.players[0].hand.len() + 1 - before, draws, "color {color:?}");
    }
    // The cast path records each spell's colors.
    let mut g = table();
    let div = hand(&mut g, 0, "Night's Whisper");
    play(&mut g, div, false, &[], &[]);
    assert_eq!(g.color_cast_turn[0][2], g.turn);
    assert_eq!(g.color_cast_turn[0][1], 0);
}

#[test]
fn allosaurus_shepherd_makes_elves_5_5_dinosaurs_until_end_of_turn() {
    let mut g = table();
    let shep = put(&mut g, 0, "Allosaurus Shepherd");
    let elves = put(&mut g, 0, "Llanowar Elves");
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::Activate(shep, 0));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(elves), g.toughness(elves)), (5, 5));
    assert_eq!((g.power(shep), g.toughness(shep)), (5, 5), "it's an Elf too");
    assert!(g.has_subtype(elves, "Dinosaur") && g.has_subtype(elves, "Elf"));
    assert_eq!((g.power(lions), g.toughness(lions)), (2, 1), "not an Elf");
    g.start_turn(5, 0);
    assert_eq!((g.power(elves), g.toughness(elves)), (1, 1));
    assert!(!g.has_subtype(elves, "Dinosaur"));
}

#[test]
fn allosaurus_shepherd_green_spells_cant_be_countered() {
    let mut g = table();
    put(&mut g, 0, "Allosaurus Shepherd");
    let elves = hand(&mut g, 0, "Llanowar Elves");
    act(&mut g, Action::Cast(elves));
    while g.pending != Pending::Priority(1) {
        act(&mut g, Action::PassPriority);
    }
    let cs = hand(&mut g, 1, "Counterspell");
    act(&mut g, Action::Cast(cs));
    act(&mut g, Action::Target(obj(elves)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
}

fn finale(x: u8) -> (Game, [ObjId; 4]) {
    let mut g = table();
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Library);
    let kodama = g.add_card(0, cid("Kodama of the East Tree"), Zone::Library);
    let pippin = g.add_card(0, cid("Pippin, Guard of the Citadel"), Zone::Graveyard);
    let f = hand(&mut g, 0, "Finale of Devastation");
    act(&mut g, Action::CastX(f, x));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    (g, [lions, kodama, pippin, f])
}

#[test]
fn finale_of_devastation_searches_library_and_graveyard() {
    let (mut g, [lions, kodama, pippin, _]) = finale(2);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(lions)) && legal.contains(&Action::Pick(pippin)));
    assert!(!legal.contains(&Action::Pick(kodama)), "mana value 5");
    resolve_all(&mut g, &[Action::Pick(pippin)]);
    assert_eq!(g.obj(pippin).zone, Zone::Battlefield);
    assert_eq!(g.obj(lions).zone, Zone::Library);
    assert!(!g.keywords(pippin).has(Keywords::HASTE), "X is under 10");
}

#[test]
fn finale_of_devastation_with_x_10_pumps_and_hastes() {
    let (mut g, [lions, ..]) = finale(10);
    resolve_all(&mut g, &[Action::Pick(lions)]);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield);
    assert_eq!((g.power(lions), g.toughness(lions)), (12, 11));
    assert!(g.keywords(lions).has(Keywords::HASTE));
}

#[test]
fn song_of_totentanz_makes_x_rats_that_cant_block_and_hastes() {
    let mut g = table();
    let lions = put(&mut g, 0, "Savannah Lions");
    let song = hand(&mut g, 0, "Song of Totentanz");
    act(&mut g, Action::CastX(song, 3));
    resolve_all(&mut g, &[]);
    let rats = tokens(&g, 0, "Rat");
    assert_eq!(rats.len(), 3);
    for &r in &rats {
        assert!(g.def_of(r).cant_block);
        assert_eq!(g.colors_of(r), [false, false, true, false, false]);
        assert_eq!((g.power(r), g.toughness(r)), (1, 1));
        assert!(g.keywords(r).has(Keywords::HASTE));
    }
    assert!(g.keywords(lions).has(Keywords::HASTE));
}

/// Resolves `effect` from a stand-in trigger on a Savannah Lions of player 0's.
fn stand_in(g: &mut Game, effect: bolt_core::effect::Effect) {
    use bolt_core::effect::{Ability, Trigger, When};
    use bolt_core::game::{StackItem, StackKind};
    use std::sync::Arc;
    let carrier = put(g, 0, "Savannah Lions");
    let def = g.obj(carrier).def;
    Arc::make_mut(&mut g.defs)[def].triggers.insert(
        0,
        Trigger {
            when: When::Never,
            ability: Ability::new(vec![], vec![effect]),
        },
    );
    g.triggered.push(StackItem {
        source: carrier,
        controller: 0,
        kind: StackKind::Trigger(0),
        targets: Vec::new(),
        kicked: false,
        flashback: false,
        extra: None,
        mode: None,
        amount: 0,
        alt: false,
        x: 0,
        division: Vec::new(),
        source_zcc: None,
    });
    for _ in 0..6 {
        if matches!(g.pending, Pending::Decide(_)) || g.triggered.is_empty() && g.items.is_empty() {
            break;
        }
        act(g, Action::PassPriority);
    }
}

#[test]
fn urzas_five_exiles_the_top_card_to_play_free_this_turn() {
    let mut g = table();
    g.players[0].library.clear();
    let elves = g.add_card(0, cid("Llanowar Elves"), Zone::Library);
    stand_in(&mut g, bolt_core::effect::Effect::ShuffleExileTopPlayFreeThisTurn);
    assert_eq!(g.obj(elves).zone, Zone::Exile);
    assert_eq!(g.obj(elves).playable, Some((0, g.turn)));
    assert!(g.obj(elves).cast_free);
    while g.pending != Pending::Priority(0) {
        act(&mut g, Action::PassPriority);
    }
    assert!(g.legal_actions().contains(&Action::Cast(elves)));
}

// ---- d3 batch 46: Archon of Cruelty, Papalymo Totolymo ----

#[test]
fn archon_of_cruelty_enters_opponent_sacrifices_discards_and_loses() {
    let mut g = table();
    let elves = put(&mut g, 1, "Llanowar Elves");
    let lions = put(&mut g, 1, "Savannah Lions");
    let card = hand(&mut g, 1, "Lightning Bolt");
    let archon = hand(&mut g, 0, "Archon of Cruelty");
    let (life0, life1, hand0) = (g.players[0].life, g.players[1].life, g.players[0].hand.len());
    act(&mut g, Action::Cast(archon));
    for _ in 0..20 {
        match g.pending {
            Pending::ChooseTarget(0) => {
                act(&mut g, Action::Target(Target::Player(1)));
            }
            Pending::Decide(1) => {
                let legal = g.legal_actions();
                if legal.contains(&Action::Pick(lions)) {
                    assert!(legal.contains(&Action::Pick(elves)), "their choice");
                    act(&mut g, Action::Pick(lions));
                } else {
                    act(&mut g, Action::Discard(card));
                }
            }
            _ if g.items.is_empty() && g.triggered.is_empty() && g.obj(archon).zone == Zone::Battlefield => {
                break
            }
            _ => {
                act(&mut g, Action::PassPriority);
            }
        }
    }
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
    assert_eq!(g.obj(card).zone, Zone::Graveyard, "discarded");
    assert_eq!(g.players[1].life, life1 - 3);
    assert_eq!(g.players[0].life, life0 + 3);
    assert_eq!(g.players[0].hand.len(), hand0, "cast the Archon, drew one");
}

#[test]
fn papalymo_pings_on_noncreature_casts_and_sacrifices_the_biggest() {
    let mut g = table();
    let pap = put(&mut g, 0, "Papalymo Totolymo");
    let elves = put(&mut g, 1, "Llanowar Elves");
    let lions = put(&mut g, 1, "Savannah Lions");
    // No life lost yet: nothing is sacrificed.
    let mut h = g.clone();
    act(&mut h, Action::Activate(pap, 0));
    resolve_all(&mut h, &[]);
    assert_eq!(
        (h.obj(elves).zone, h.obj(lions).zone),
        (Zone::Battlefield, Zone::Battlefield)
    );
    // A noncreature spell: 1 damage to the opponent, 1 life for you.
    let (life0, life1) = (g.players[0].life, g.players[1].life);
    let nw = hand(&mut g, 0, "Night's Whisper");
    play(&mut g, nw, false, &[], &[]);
    assert_eq!(g.players[1].life, life1 - 1);
    assert_eq!(g.players[0].life, life0 - 2 + 1);
    act(&mut g, Action::Activate(pap, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(pap).zone, Zone::Graveyard, "sacrificed as a cost");
    assert_eq!(g.obj(lions).zone, Zone::Graveyard, "the greatest power");
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
}
