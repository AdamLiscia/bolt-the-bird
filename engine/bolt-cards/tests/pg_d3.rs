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
fn negate_counters_only_noncreature_spells() {
    let mut g = table();
    let opt = hand(&mut g, 0, "Opt");
    let n = hand(&mut g, 0, "Negate");
    act(&mut g, Action::Cast(opt));
    act(&mut g, Action::Cast(n));
    act(&mut g, Action::Target(obj(opt)));
    let cards = g.players[0].hand.len();
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(opt).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), cards, "Opt never resolved");

    let mut g = table();
    let looter = hand(&mut g, 0, "Merfolk Looter");
    let n = hand(&mut g, 0, "Negate");
    act(&mut g, Action::Cast(looter));
    assert!(
        !g.legal_actions().contains(&Action::Cast(n)),
        "no noncreature spell to target"
    );
}

#[test]
fn frantic_search_draws_two_discards_two_and_untaps_three_lands() {
    let mut g = game(&["Island", "Island", "Island", "Plains"]);
    let a = hand(&mut g, 0, "Counterspell");
    let b = hand(&mut g, 0, "Entomb");
    let fs = hand(&mut g, 0, "Frantic Search");
    let cards = g.players[0].hand.len();
    play(&mut g, fs, false, &[], &[Action::Discard(a), Action::Discard(b)]);
    assert_eq!(g.players[0].hand.len(), cards - 1);
    assert_eq!((g.obj(a).zone, g.obj(b).zone), (Zone::Graveyard, Zone::Graveyard));
    assert!(
        g.permanents(0)
            .filter(|&x| g.types_of(x).land)
            .all(|x| !g.obj(x).tapped),
        "the three lands it was cast with untap"
    );
}

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
