//! Playgroup decks, lane D2 (the desktop): each script's behavior in a 1v1 Commander game on the CMD pool.
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

/// Attack with `attackers` on turn 5; stops after attackers are declared.
fn attack(g: &mut Game, attackers: &[ObjId]) {
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    for &a in attackers {
        act(g, Action::Attack(a));
    }
    act(g, Action::FinishAttacks);
}

fn in_graveyard(g: &Game, p: usize, x: ObjId) -> bool {
    g.players[p].graveyard.contains(&x)
}

fn colors_of(g: &Game, id: ObjId) -> Vec<[bool; 5]> {
    g.mana_sources(0)
        .into_iter()
        .filter(|&(x, _)| x == id)
        .map(|(_, m)| m.colors)
        .collect()
}

fn commander_on_field(g: &mut Game, name: &str) -> ObjId {
    let c = put(g, 0, name);
    g.commanders[0] = Some(c);
    c
}

fn has(g: &Game, a: Action) -> bool {
    g.legal_actions().contains(&a)
}

// ---- The Gitrog, Ravenous Ride ----

#[test]
fn demonic_tutor_puts_any_card_into_your_hand() {
    let mut g = table();
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    let t = hand(&mut g, 0, "Demonic Tutor");
    play(&mut g, t, false, &[], &[Action::Pick(ring)]);
    assert_eq!(g.obj(ring).zone, Zone::Hand);
    assert!(in_graveyard(&g, 0, t));
}

#[test]
fn exquisite_blood_gains_what_the_opponent_loses() {
    let mut g = table();
    put(&mut g, 0, "Exquisite Blood");
    let b = hand(&mut g, 0, "Boltwave");
    play(&mut g, b, false, &[], &[]);
    assert_eq!(g.players[1].life, 37);
    assert_eq!(g.players[0].life, 43);
}

#[test]
fn goldvein_hydra_enters_with_x_and_dies_into_tapped_treasures() {
    let mut g = table();
    let h = hand(&mut g, 0, "Goldvein Hydra");
    act(&mut g, Action::CastX(h, 3));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(h), g.toughness(h)), (3, 3));
    let k = g.keywords(h);
    assert!(k.has(Keywords::VIGILANCE) && k.has(Keywords::TRAMPLE) && k.has(Keywords::HASTE));
    let kill = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, kill, false, &[obj(h)], &[]);
    let t = tokens(&g, 0, "Treasure");
    assert_eq!(t.len(), 3, "one per point of its last power");
    assert!(t.iter().all(|&x| g.obj(x).tapped));
}

#[test]
fn grave_pact_makes_the_opponent_sacrifice_when_yours_dies() {
    let mut g = table();
    put(&mut g, 0, "Grave Pact");
    let mine = put(&mut g, 0, "Savannah Lions");
    let lions = put(&mut g, 1, "Savannah Lions");
    let angel = put(&mut g, 1, "Serra Angel");
    let kill = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, kill, false, &[obj(mine)], &[Action::Pick(angel)]);
    assert!(in_graveyard(&g, 1, angel), "their choice");
    assert_eq!(g.obj(lions).zone, Zone::Battlefield);
}

#[test]
fn rotting_regisaur_discards_at_your_upkeep() {
    let mut g = table();
    put(&mut g, 0, "Rotting Regisaur");
    let card = hand(&mut g, 0, "Sol Ring");
    let before = g.players[0].graveyard.len();
    g.start_turn(5, 0);
    let mut asked = false;
    while g.step != Step::Main1 {
        if let Pending::Decide(_) = g.pending {
            asked = true;
            act(&mut g, Action::Discard(card));
        } else {
            act(&mut g, Action::PassPriority);
        }
    }
    assert!(asked, "a discard at upkeep");
    assert!(in_graveyard(&g, 0, card));
    assert_eq!(g.players[0].graveyard.len(), before + 1);
}

#[test]
fn voracious_hydra_doubles_its_counters_or_fights() {
    let mut g = table();
    let h = hand(&mut g, 0, "Voracious Hydra");
    act(&mut g, Action::CastX(h, 2));
    resolve_all(&mut g, &[Action::Mode(0)]);
    assert_eq!(g.obj(h).counters, 4);
    assert_eq!((g.power(h), g.toughness(h)), (4, 5));

    let mut g = table();
    let lions = put(&mut g, 1, "Savannah Lions"); // 2/1
    let h = hand(&mut g, 0, "Voracious Hydra");
    act(&mut g, Action::CastX(h, 2));
    resolve_all(&mut g, &[Action::Mode(1), Action::Target(obj(lions))]);
    assert!(in_graveyard(&g, 1, lions));
    assert_eq!(g.obj(h).zone, Zone::Battlefield, "a 2/3 survives 2 damage");
    assert_eq!(g.obj(h).counters, 2);
}

#[test]
fn daemogoth_titan_sacrifices_a_creature_when_it_attacks() {
    let mut g = table();
    let titan = put(&mut g, 0, "Daemogoth Titan");
    let lions = put(&mut g, 0, "Savannah Lions");
    assert_eq!((g.power(titan), g.toughness(titan)), (11, 10));
    attack(&mut g, &[titan]);
    resolve_all(&mut g, &[Action::Pick(lions)]);
    assert!(in_graveyard(&g, 0, lions));
    assert_eq!(g.obj(titan).zone, Zone::Battlefield);
}

#[test]
fn daemogoth_titan_sacrifices_when_it_blocks_too() {
    let mut g = table();
    let titan = put(&mut g, 0, "Daemogoth Titan");
    let lions = put(&mut g, 1, "Savannah Lions");
    g.start_turn(6, 1);
    while g.pending != Pending::DeclareAttackers(1) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(lions));
    act(&mut g, Action::FinishAttacks);
    while !matches!(g.pending, Pending::DeclareBlockers(_)) {
        act(&mut g, Action::PassPriority);
    }
    act(
        &mut g,
        Action::Block {
            blocker: titan,
            attacker: lions,
        },
    );
    act(&mut g, Action::FinishBlocks);
    resolve_all(&mut g, &[]);
    assert!(in_graveyard(&g, 0, titan), "its only creature: itself");
}

#[test]
fn fleetfeather_sandals_give_flying_and_haste() {
    let mut g = table();
    let s = put(&mut g, 0, "Fleetfeather Sandals");
    let l = put(&mut g, 0, "Aegis Turtle");
    assert!(!g.keywords(l).has(Keywords::FLYING));
    act(&mut g, Action::Activate(s, 0));
    act(&mut g, Action::Target(obj(l)));
    resolve_all(&mut g, &[]);
    let k = g.keywords(l);
    assert!(k.has(Keywords::FLYING) && k.has(Keywords::HASTE));
}

#[test]
fn lantern_of_the_lost_exiles_a_card_then_all_graveyards() {
    let mut g = table();
    let theirs = g.add_card(1, cid("Savannah Lions"), Zone::Graveyard);
    let lantern = hand(&mut g, 0, "Lantern of the Lost");
    play(&mut g, lantern, false, &[], &[Action::Target(obj(theirs))]);
    assert_eq!(g.obj(theirs).zone, Zone::Exile);
    g.add_card(0, cid("Savannah Lions"), Zone::Graveyard);
    g.add_card(1, cid("Serra Angel"), Zone::Graveyard);
    let hand_before = g.players[0].hand.len();
    act(&mut g, Action::Activate(lantern, 0));
    resolve_all(&mut g, &[]);
    assert!(g.players[0].graveyard.is_empty() && g.players[1].graveyard.is_empty());
    assert_eq!(g.obj(lantern).zone, Zone::Exile, "exiled as the cost");
    assert_eq!(g.players[0].hand.len(), hand_before + 1);
}

// ---- Kaalia of the Vast ----

const B_R: [bool; 5] = [false, false, true, true, false];
const R_W: [bool; 5] = [true, false, false, true, false];

#[test]
fn shock_lands_pay_two_life_or_enter_tapped() {
    for (name, colors) in [("Blood Crypt", B_R), ("Sacred Foundry", R_W)] {
        let mut g = game(&[]);
        let s = hand(&mut g, 0, name);
        act(&mut g, Action::PlayLand(s));
        act(&mut g, Action::Yes);
        assert!(!g.obj(s).tapped, "{name}");
        assert_eq!(g.players[0].life, 38, "{name}");
        assert_eq!(colors_of(&g, s), [colors], "{name}");
        let mut g = game(&[]);
        let s = hand(&mut g, 0, name);
        act(&mut g, Action::PlayLand(s));
        act(&mut g, Action::No);
        assert!(g.obj(s).tapped, "{name}");
        assert_eq!(g.players[0].life, 40, "{name}");
    }
}

#[test]
fn deadly_rollick_and_flawless_maneuver_are_free_with_a_commander() {
    let mut g = game(&[]);
    let theirs = put(&mut g, 1, "Savannah Lions");
    let mine = put(&mut g, 0, "Aegis Turtle");
    let rollick = hand(&mut g, 0, "Deadly Rollick");
    let maneuver = hand(&mut g, 0, "Flawless Maneuver");
    assert!(!has(&g, Action::CastAlternative(rollick)), "no commander");
    assert!(!has(&g, Action::Cast(rollick)), "no lands");
    commander_on_field(&mut g, "Bello, Bard of the Brambles");
    play_alt(&mut g, rollick, &[obj(theirs)]);
    assert_eq!(g.obj(theirs).zone, Zone::Exile);
    play_alt(&mut g, maneuver, &[]);
    assert!(g.keywords(mine).has(Keywords::INDESTRUCTIBLE));
}

fn play_alt(g: &mut Game, card: ObjId, targets: &[Target]) {
    act(g, Action::CastAlternative(card));
    for &t in targets {
        act(g, Action::Target(t));
    }
    resolve_all(g, &[]);
}

#[test]
fn filter_lands_turn_one_hybrid_into_two_colored() {
    let mut g = game(&["Fetid Heath", "Plains"]);
    let blood = put(&mut g, 1, "Exquisite Blood");
    let m = hand(&mut g, 0, "Mortify"); // {1}{W}{B}
    assert!(!has(&g, Action::Cast(m)), "two lands, no black");
    put(&mut g, 0, "Plains"); // Plains pays {W/B}; the Heath makes {W}{B}; the other Plains {1}
    play(&mut g, m, false, &[obj(blood)], &[]);
    assert!(in_graveyard(&g, 1, blood));

    let mut g = game(&["Graven Cairns"]);
    let t = hand(&mut g, 0, "Vampiric Tutor"); // {B}
    put(&mut g, 0, "Mountain"); // pays {B/R}: the Cairns makes {B}{B}; {C} + {R} alone has no {B}
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    play(&mut g, t, false, &[], &[Action::Pick(ring)]);
    assert_eq!(g.players[0].library.last(), Some(&ring));
}

#[test]
fn dragonskull_summit_checks_for_a_swamp_or_mountain() {
    let mut g = game(&[]);
    let s = hand(&mut g, 0, "Dragonskull Summit");
    act(&mut g, Action::PlayLand(s));
    assert!(g.obj(s).tapped);
    let mut g = game(&["Swamp"]);
    let s = hand(&mut g, 0, "Dragonskull Summit");
    act(&mut g, Action::PlayLand(s));
    assert!(!g.obj(s).tapped);
    assert_eq!(colors_of(&g, s), [B_R]);
}

#[test]
fn vampiric_tutor_puts_a_card_on_top_for_two_life() {
    let mut g = table();
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    g.players[0].library.rotate_right(1); // not on top to begin with
    let t = hand(&mut g, 0, "Vampiric Tutor");
    play(&mut g, t, false, &[], &[Action::Pick(ring)]);
    assert_eq!(g.players[0].library.last(), Some(&ring));
    assert_eq!(g.players[0].life, 38);
}

#[test]
fn battlebond_lands_enter_tapped_one_on_one() {
    for (name, colors) in [("Luxury Suite", B_R), ("Spectator Seating", R_W)] {
        let mut g = game(&[]);
        let s = hand(&mut g, 0, name);
        act(&mut g, Action::PlayLand(s));
        assert!(g.obj(s).tapped, "{name}");
        assert_eq!(g.def_of(s).mana.map(|m| m.colors), Some(colors), "{name}");
    }
}

#[test]
fn talismans_and_sulfurous_springs_hurt_for_colored_mana() {
    for name in [
        "Talisman of Indulgence",
        "Talisman of Hierarchy",
        "Sulfurous Springs",
    ] {
        let mut g = game(&[name]);
        let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
        let t = hand(&mut g, 0, "Vampiric Tutor"); // {B}
        play(&mut g, t, false, &[], &[Action::Pick(ring)]);
        assert_eq!(g.players[0].life, 37, "{name}: 1 damage, then 2 life");
        let mut g = game(&[name]);
        let ring = hand(&mut g, 0, "Sol Ring"); // {1}: the free {C}
        play(&mut g, ring, false, &[], &[]);
        assert_eq!(g.players[0].life, 40, "{name}");
    }
}

#[test]
fn mortify_destroys_a_creature_or_an_enchantment() {
    let mut g = table();
    let blood = put(&mut g, 1, "Exquisite Blood");
    let lions = put(&mut g, 1, "Savannah Lions");
    let m = hand(&mut g, 0, "Mortify");
    play(&mut g, m, false, &[obj(blood)], &[]);
    let m = hand(&mut g, 0, "Mortify");
    play(&mut g, m, false, &[obj(lions)], &[]);
    assert!(in_graveyard(&g, 1, blood) && in_graveyard(&g, 1, lions));
}

#[test]
fn pestilence_demon_pings_every_creature_and_player() {
    let mut g = table();
    let demon = put(&mut g, 0, "Pestilence Demon");
    let lions = put(&mut g, 1, "Savannah Lions");
    act(&mut g, Action::Activate(demon, 0));
    resolve_all(&mut g, &[]);
    assert!(in_graveyard(&g, 1, lions));
    assert_eq!(g.obj(demon).zone, Zone::Battlefield);
    assert_eq!((g.players[0].life, g.players[1].life), (39, 39));
}

#[test]
fn akroma_angel_of_wrath_has_its_keywords_and_protection() {
    let mut g = table();
    let a = put(&mut g, 1, "Akroma, Angel of Wrath");
    let k = g.keywords(a);
    for kw in [
        Keywords::FLYING,
        Keywords::FIRST_STRIKE,
        Keywords::VIGILANCE,
        Keywords::TRAMPLE,
        Keywords::HASTE,
    ] {
        assert!(k.has(kw));
    }
    let lions = put(&mut g, 1, "Savannah Lions");
    let downfall = hand(&mut g, 0, "Hero's Downfall"); // black
    act(&mut g, Action::Cast(downfall));
    assert!(!has(&g, Action::Target(obj(a))), "protection from black");
    assert!(has(&g, Action::Target(obj(lions))));
}

#[test]
fn flagstones_of_trokair_fetches_a_plains_when_destroyed() {
    let mut g = table();
    let flag = put(&mut g, 0, "Flagstones of Trokair");
    assert_eq!(colors_of(&g, flag), [[true, false, false, false, false]]);
    let plains = g.add_card(0, cid("Plains"), Zone::Library);
    let spine = hand(&mut g, 0, "Spine of Ish Sah");
    play(
        &mut g,
        spine,
        false,
        &[],
        &[Action::Target(obj(flag)), Action::Yes, Action::Pick(plains)],
    );
    assert!(in_graveyard(&g, 0, flag));
    assert_eq!(g.obj(plains).zone, Zone::Battlefield);
    assert!(g.obj(plains).tapped);
}

#[test]
fn shinka_gives_a_legendary_creature_first_strike() {
    let mut g = table();
    let shinka = put(&mut g, 0, "Shinka, the Bloodsoaked Keep");
    let legend = put(&mut g, 0, "Ms. Bumbleflower");
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::Activate(shinka, 0));
    assert!(!has(&g, Action::Target(obj(lions))), "not legendary");
    act(&mut g, Action::Target(obj(legend)));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(legend).has(Keywords::FIRST_STRIKE));
}

// ---- Henrika Domnathi ----

#[test]
fn diabolic_intent_sacrifices_a_creature_to_tutor() {
    let mut g = table();
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    let t = hand(&mut g, 0, "Diabolic Intent");
    assert!(!has(&g, Action::Cast(t)), "the sacrifice is required");
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::CastWith(t, lions));
    assert!(in_graveyard(&g, 0, lions));
    resolve_all(&mut g, &[Action::Pick(ring)]);
    assert_eq!(g.obj(ring).zone, Zone::Hand);
}

#[test]
fn cordial_vampire_grows_vampires_whenever_a_creature_dies() {
    let mut g = table();
    let cordial = put(&mut g, 0, "Cordial Vampire");
    let hexmage = put(&mut g, 0, "Vampire Hexmage"); // a Vampire
    let lions = put(&mut g, 1, "Savannah Lions");
    let kill = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, kill, false, &[obj(lions)], &[]);
    assert_eq!((g.obj(cordial).counters, g.obj(hexmage).counters), (1, 1));
    let kill = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, kill, false, &[obj(cordial)], &[]);
    assert_eq!(g.obj(hexmage).counters, 2, "its own death counts too");
}

#[test]
fn sheoldred_the_apocalypse_gains_on_your_draws_and_drains_theirs() {
    let mut g = table();
    put(&mut g, 0, "Sheoldred, the Apocalypse");
    g.start_turn(4, 1);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[1].life, 38, "the opponent's draw step");
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[0].life, 42, "your draw step");
}

#[test]
fn diabolic_edict_makes_the_target_player_sacrifice() {
    let mut g = table();
    let lions = put(&mut g, 1, "Savannah Lions");
    let angel = put(&mut g, 1, "Serra Angel");
    let e = hand(&mut g, 0, "Diabolic Edict");
    play(&mut g, e, false, &[Target::Player(1)], &[Action::Pick(lions)]);
    assert!(in_graveyard(&g, 1, lions));
    assert_eq!(g.obj(angel).zone, Zone::Battlefield);
}

#[test]
fn diresight_surveils_two_draws_two_and_costs_two_life() {
    let mut g = table();
    let a = g.add_card(0, cid("Sol Ring"), Zone::Library);
    let b = g.add_card(0, cid("Savannah Lions"), Zone::Library); // on top
    let d = hand(&mut g, 0, "Diresight");
    play(&mut g, d, false, &[], &[Action::Yes, Action::No]);
    assert!(in_graveyard(&g, 0, b), "surveiled away");
    assert_eq!(g.obj(a).zone, Zone::Hand, "kept on top, then drawn");
    assert_eq!(g.players[0].life, 38);
}

#[test]
fn kokusho_drains_five_when_it_dies() {
    let mut g = table();
    let k = put(&mut g, 0, "Kokusho, the Evening Star");
    let kill = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, kill, false, &[obj(k)], &[]);
    assert_eq!((g.players[0].life, g.players[1].life), (45, 35));
}

// ---- Korvold, Fae-Cursed King ----

#[test]
fn deserted_temple_untaps_a_land() {
    let mut g = table();
    let temple = put(&mut g, 0, "Deserted Temple");
    let lands = hand(&mut g, 0, "Savage Lands");
    act(&mut g, Action::PlayLand(lands));
    assert!(g.obj(lands).tapped, "enters tapped");
    act(&mut g, Action::Activate(temple, 0));
    act(&mut g, Action::Target(obj(lands)));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(lands).tapped);
    assert!(g.obj(temple).tapped);
    assert_eq!(
        g.def_of(lands).mana.map(|m| m.colors),
        Some([false, false, true, true, true]),
        "Savage Lands: {{B}}, {{R}}, or {{G}}"
    );
}

#[test]
fn rakdos_carnarium_bounces_a_land_and_makes_two() {
    let mut g = game(&["Swamp"]);
    let swamp = g.permanents(0).find(|&x| g.def_of(x).name == "Swamp").unwrap();
    let c = hand(&mut g, 0, "Rakdos Carnarium");
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped);
    resolve_all(&mut g, &[Action::Pick(swamp)]);
    assert_eq!(g.obj(swamp).zone, Zone::Hand);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    // {B}{R} from one land: a {1}{B} spell.
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    let t = hand(&mut g, 0, "Demonic Tutor");
    play(&mut g, t, false, &[], &[Action::Pick(ring)]);
    assert_eq!(g.obj(ring).zone, Zone::Hand);
}

#[test]
fn shadowblood_ridge_turns_one_into_black_and_red() {
    let mut g = game(&["Shadowblood Ridge", "Plains"]);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    let t = hand(&mut g, 0, "Vampiric Tutor"); // {B}: only the Ridge makes black
    play(&mut g, t, false, &[], &[Action::Pick(ring)]);
    assert_eq!(g.players[0].library.last(), Some(&ring));
}

#[test]
fn souls_fire_deals_your_creatures_power_to_any_target() {
    let mut g = table();
    let angel = put(&mut g, 0, "Serra Angel"); // 4/4
    let f = hand(&mut g, 0, "Soul's Fire");
    play(&mut g, f, false, &[obj(angel), Target::Player(1)], &[]);
    assert_eq!(g.players[1].life, 36);
}

#[test]
fn thrors_map_finds_a_basic_and_loots() {
    let mut g = table();
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    let map = hand(&mut g, 0, "Thrór's Map");
    play(&mut g, map, false, &[], &[Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Hand);
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(map, 0));
    resolve_all(&mut g, &[Action::Discard(forest)]);
    assert!(in_graveyard(&g, 0, forest));
    assert_eq!(g.players[0].hand.len(), n, "drew one, discarded one");
}

// ---- Damia, Sage of Stone ----

#[test]
fn rejuvenating_springs_and_morphic_pool_enter_tapped_one_on_one() {
    let g_u = [false, true, false, false, true];
    let u_b = [false, true, true, false, false];
    for (name, colors) in [("Rejuvenating Springs", g_u), ("Morphic Pool", u_b)] {
        let mut g = game(&[]);
        let s = hand(&mut g, 0, name);
        act(&mut g, Action::PlayLand(s));
        assert!(g.obj(s).tapped, "{name}");
        assert_eq!(g.def_of(s).mana.map(|m| m.colors), Some(colors), "{name}");
    }
}

#[test]
fn simic_signet_turns_one_into_green_and_blue() {
    let mut g = game(&["Simic Signet", "Plains"]);
    let elves = hand(&mut g, 0, "Llanowar Elves"); // {G}: only the Signet makes green
    play(&mut g, elves, false, &[], &[]);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
}

#[test]
fn underground_river_hurts_for_colored_mana() {
    let mut g = game(&["Underground River"]);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    let t = hand(&mut g, 0, "Vampiric Tutor");
    play(&mut g, t, false, &[], &[Action::Pick(ring)]);
    assert_eq!(g.players[0].life, 37, "1 damage, then 2 life");
}

#[test]
fn sunken_ruins_filters_blue_into_black() {
    // The Island pays {U/B}; the Ruins makes {B}{B}. {C} and {U} alone have no {B}.
    let mut g = game(&["Sunken Ruins", "Island"]);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    let t = hand(&mut g, 0, "Vampiric Tutor");
    play(&mut g, t, false, &[], &[Action::Pick(ring)]);
    assert_eq!(g.players[0].library.last(), Some(&ring));
}

#[test]
fn leyline_of_anticipation_starts_in_play_and_gives_flash() {
    let leylines = vec![cid("Leyline of Anticipation"); 99];
    let plains = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        set("CMD").unwrap().defs.clone(),
        [&leylines, &plains],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    act(&mut g, Action::Keep);
    act(&mut g, Action::Keep);
    let n = g
        .permanents(0)
        .filter(|&c| g.def_of(c).name == "Leyline of Anticipation")
        .count();
    assert_eq!(n, 7, "every one in the opening hand");

    let mut g = table();
    put(&mut g, 1, "Leyline of Anticipation");
    let lions = hand(&mut g, 1, "Savannah Lions");
    act(&mut g, Action::PassPriority);
    assert!(has(&g, Action::Cast(lions)), "on the opponent's turn");
}

#[test]
fn okina_pumps_a_legendary_creature() {
    let mut g = table();
    let okina = put(&mut g, 0, "Okina, Temple to the Grandfathers");
    let legend = put(&mut g, 0, "Ms. Bumbleflower");
    let lions = put(&mut g, 0, "Savannah Lions");
    let p = g.power(legend);
    act(&mut g, Action::Activate(okina, 0));
    assert!(!has(&g, Action::Target(obj(lions))), "not legendary");
    act(&mut g, Action::Target(obj(legend)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(legend), p + 1);
}

// ---- Raffine, Scheming Seer ----

#[test]
fn bitterbloom_bearer_makes_a_faerie_each_upkeep_for_a_life() {
    let mut g = table();
    let b = put(&mut g, 0, "Bitterbloom Bearer");
    assert!(g.keywords(b).has(Keywords::FLASH) && g.keywords(b).has(Keywords::FLYING));
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[0].life, 39);
    let t = tokens(&g, 0, "Faerie");
    assert_eq!(t.len(), 1);
    assert!(g.keywords(t[0]).has(Keywords::FLYING));
}

#[test]
fn queza_drains_one_whenever_you_draw() {
    let mut g = table();
    put(&mut g, 0, "Queza, Augur of Agonies");
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        if let Pending::ChooseTarget(_) = g.pending {
            act(&mut g, Action::Target(Target::Player(1)));
        } else {
            act(&mut g, Action::PassPriority);
        }
    }
    assert_eq!((g.players[0].life, g.players[1].life), (41, 39));
}

#[test]
fn vault_of_the_archangel_gives_deathtouch_and_lifelink() {
    let mut g = table();
    let vault = put(&mut g, 0, "Vault of the Archangel");
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::Activate(vault, 0));
    resolve_all(&mut g, &[]);
    let k = g.keywords(lions);
    assert!(k.has(Keywords::DEATHTOUCH) && k.has(Keywords::LIFELINK));
}

// ---- Core: triggers, batch 1 ----

/// Passes to `turn`'s first main phase, answering any yes/no along the way with `yes`.
fn to_main_answering(g: &mut Game, turn: u32, active: usize, yes: bool) {
    g.start_turn(turn, active);
    while g.step != Step::Main1 {
        if let Pending::Decide(_) = g.pending {
            act(g, if yes { Action::Yes } else { Action::No });
        } else {
            act(g, Action::PassPriority);
        }
    }
}

#[test]
fn lifeblood_hydra_dies_into_life_and_cards_equal_to_its_power() {
    let mut g = table();
    let h = hand(&mut g, 0, "Lifeblood Hydra");
    act(&mut g, Action::CastX(h, 3));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(h), 3);
    assert!(g.keywords(h).has(Keywords::TRAMPLE));
    let kill = hand(&mut g, 0, "Hero's Downfall");
    let n = g.players[0].hand.len();
    play(&mut g, kill, false, &[obj(h)], &[]);
    assert_eq!(g.players[0].life, 43);
    assert_eq!(g.players[0].hand.len(), n - 1 + 3);
}

#[test]
fn mana_vault_makes_three_stays_tapped_and_hurts_on_your_draw_step() {
    let mut g = game(&[]);
    let vault = put(&mut g, 0, "Mana Vault");
    assert_eq!(g.mana_sources(0).iter().filter(|&&(x, _)| x == vault).count(), 3);
    g.manual_mana[0] = true;
    act(&mut g, Action::TapForMana(vault, 5)); // {C}{C}{C}
    g.manual_mana[0] = false;
    for _ in 0..4 {
        put(&mut g, 0, "Plains");
    }
    to_main_answering(&mut g, 5, 0, false);
    assert!(g.obj(vault).tapped, "declined the {{4}}");
    assert_eq!(g.players[0].life, 39, "1 damage in your draw step");
    to_main_answering(&mut g, 6, 1, false);
    assert_eq!(g.players[0].life, 39, "not on the opponent's draw step");
    to_main_answering(&mut g, 7, 0, true);
    assert!(!g.obj(vault).tapped, "paid {{4}} to untap");
    assert_eq!(g.players[0].life, 39, "untapped by the draw step");
}

#[test]
fn howling_mine_gives_each_player_an_extra_draw_while_untapped() {
    let mut g = table();
    put(&mut g, 0, "Howling Mine");
    let (a, b) = (g.players[0].hand.len(), g.players[1].hand.len());
    to_main_answering(&mut g, 4, 1, false);
    assert_eq!(g.players[1].hand.len(), b + 2);
    to_main_answering(&mut g, 5, 0, false);
    assert_eq!(g.players[0].hand.len(), a + 2);
}

#[test]
fn loyal_drake_draws_at_combat_only_with_your_commander_out() {
    let mut g = table();
    put(&mut g, 0, "Loyal Drake");
    let combat = |g: &mut Game| {
        while g.step != Step::BeginCombat {
            act(g, Action::PassPriority);
        }
        resolve_all(g, &[]);
    };
    let n = g.players[0].hand.len();
    combat(&mut g);
    assert_eq!(g.players[0].hand.len(), n, "no commander");
    commander_on_field(&mut g, "Bello, Bard of the Brambles");
    to_main_answering(&mut g, 5, 0, false);
    let n = g.players[0].hand.len();
    combat(&mut g);
    assert_eq!(g.players[0].hand.len(), n + 1, "lieutenant");
}

#[test]
fn aetherflux_reservoir_gains_per_spell_and_fires_for_fifty_life() {
    let mut g = table();
    let r = put(&mut g, 0, "Aetherflux Reservoir");
    for _ in 0..2 {
        let ring = hand(&mut g, 0, "Sol Ring");
        play(&mut g, ring, false, &[], &[]);
    }
    assert_eq!(g.players[0].life, 43, "1, then 2");
    assert!(!has(&g, Action::Activate(r, 0)), "43 life can't pay 50");
    g.players[0].life = 60;
    act(&mut g, Action::Activate(r, 0));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 10);
    assert!(g.players[1].life <= 0);
}

#[test]
fn hydroid_krasis_gains_and_draws_half_x_as_its_cast() {
    let mut g = table();
    let h = hand(&mut g, 0, "Hydroid Krasis");
    let n = g.players[0].hand.len();
    act(&mut g, Action::CastX(h, 5));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 42);
    assert_eq!(g.players[0].hand.len(), n - 1 + 2);
    assert_eq!(g.obj(h).counters, 5);
    let k = g.keywords(h);
    assert!(k.has(Keywords::FLYING) && k.has(Keywords::TRAMPLE));
}

#[test]
fn malakir_rebirth_returns_the_creature_tapped_when_it_dies() {
    let mut g = table();
    let lions = put(&mut g, 0, "Savannah Lions");
    let r = hand(&mut g, 0, "Malakir Rebirth // Malakir Mire");
    play(&mut g, r, false, &[obj(lions)], &[]);
    assert_eq!(g.players[0].life, 38);
    let kill = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, kill, false, &[obj(lions)], &[]);
    let back = g
        .creatures(0)
        .find(|&c| g.def_of(c).name == "Savannah Lions")
        .expect("returned");
    assert!(g.obj(back).tapped);

    // The back face: Malakir Mire, a land that enters tapped and taps for {B}.
    let mut g = game(&[]);
    let m = hand(&mut g, 0, "Malakir Rebirth // Malakir Mire");
    act(&mut g, Action::PlayLand(m));
    assert_eq!(g.obj(m).zone, Zone::Battlefield);
    assert!(g.obj(m).tapped);
    assert_eq!(
        g.def_of(m).mana.map(|x| x.colors),
        Some([false, false, true, false, false])
    );
}

#[test]
fn breenas_trigger_never_happens_one_on_one() {
    let mut g = table();
    let b = put(&mut g, 0, "Breena, the Demagogue");
    let lions = put(&mut g, 0, "Savannah Lions");
    assert!(g.keywords(b).has(Keywords::FLYING));
    attack(&mut g, &[lions]);
    let n = g.players[0].hand.len();
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n);
    assert!(g.creatures(0).all(|c| g.obj(c).counters == 0));
}

#[test]
fn burgeoning_puts_a_land_in_when_the_opponent_plays_one() {
    let mut g = table();
    put(&mut g, 0, "Burgeoning");
    let mine = hand(&mut g, 0, "Forest");
    let other = hand(&mut g, 0, "Island");
    act(&mut g, Action::PlayLand(mine));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(other).zone, Zone::Hand, "not on your own land play");
    g.start_turn(4, 1);
    while !(g.step == Step::Main1 && g.pending == Pending::Priority(1)) {
        act(&mut g, Action::PassPriority);
    }
    let theirs = hand(&mut g, 1, "Forest");
    act(&mut g, Action::PlayLand(theirs));
    resolve_all(&mut g, &[Action::Pick(other)]); // a land card from hand, or No
    assert_eq!(g.obj(other).zone, Zone::Battlefield);
}

// ---- Core: triggers, batch 2 ----

#[test]
fn reanimate_takes_any_graveyards_creature_for_its_mana_value() {
    let mut g = table();
    let angel = g.add_card(1, cid("Serra Angel"), Zone::Graveyard); // mana value 5
    let r = hand(&mut g, 0, "Reanimate");
    play(&mut g, r, false, &[obj(angel)], &[]);
    assert_eq!(g.obj(angel).zone, Zone::Battlefield);
    assert_eq!(g.obj(angel).controller, 0);
    assert_eq!(g.players[0].life, 35);
}

#[test]
fn feed_the_swarm_destroys_and_costs_its_mana_value() {
    let mut g = table();
    let blood = put(&mut g, 1, "Exquisite Blood"); // mana value 5
    let mine = put(&mut g, 0, "Savannah Lions");
    let f = hand(&mut g, 0, "Feed the Swarm");
    act(&mut g, Action::Cast(f));
    assert!(!has(&g, Action::Target(obj(mine))), "an opponent's only");
    act(&mut g, Action::Target(obj(blood)));
    resolve_all(&mut g, &[]);
    assert!(in_graveyard(&g, 1, blood));
    assert_eq!(g.players[0].life, 35);
}

#[test]
fn darkstar_augur_takes_the_top_card_each_upkeep_and_has_offspring() {
    let mut g = table();
    put(&mut g, 0, "Darkstar Augur");
    let top = g.add_card(0, cid("Serra Angel"), Zone::Library);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(top).zone, Zone::Hand);
    assert_eq!(g.players[0].life, 35);

    let mut g = table();
    let a = hand(&mut g, 0, "Darkstar Augur");
    play(&mut g, a, true, &[], &[]);
    let copies = tokens(&g, 0, "Darkstar Augur");
    assert_eq!(copies.len(), 1, "offspring paid");
    assert_eq!(g.power(copies[0]), 1);
}

#[test]
fn castle_locthwain_draws_then_costs_life_per_card_in_hand() {
    let mut g = game(&[]);
    let c = hand(&mut g, 0, "Castle Locthwain");
    act(&mut g, Action::PlayLand(c));
    assert!(g.obj(c).tapped, "no Swamp");
    let mut g = game(&["Swamp", "Swamp", "Swamp"]);
    let c = hand(&mut g, 0, "Castle Locthwain");
    act(&mut g, Action::PlayLand(c));
    assert!(!g.obj(c).tapped, "a Swamp");
    let n = g.players[0].hand.len() as i32;
    act(&mut g, Action::Activate(c, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 40 - (n + 1), "drew first, then counted");
}

#[test]
fn eldritch_pact_draws_and_drains_by_graveyard_size() {
    let mut g = table();
    for _ in 0..4 {
        g.add_card(0, cid("Plains"), Zone::Graveyard);
    }
    let p = hand(&mut g, 0, "Eldritch Pact");
    let n = g.players[0].hand.len();
    play(&mut g, p, false, &[Target::Player(0)], &[]);
    assert_eq!(g.players[0].hand.len(), n - 1 + 4);
    assert_eq!(g.players[0].life, 36);
}

#[test]
fn dark_deal_redraws_each_hand_one_short() {
    let mut g = table();
    let d = hand(&mut g, 0, "Dark Deal");
    let (a, b) = (g.players[0].hand.len() - 1, g.players[1].hand.len());
    play(&mut g, d, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), a - 1);
    assert_eq!(g.players[1].hand.len(), b - 1);
}

#[test]
fn graveborn_muse_draws_and_drains_per_zombie() {
    let mut g = table();
    put(&mut g, 0, "Graveborn Muse"); // a Zombie itself
    put(&mut g, 0, "Undead Augur"); // Zombie Wizard
    let n = g.players[0].hand.len();
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(
        g.players[0].hand.len(),
        n + 2 + 1,
        "two Zombies, then the draw step"
    );
    assert_eq!(g.players[0].life, 38);
}

#[test]
fn disciple_of_bolas_sacrifices_another_for_life_and_cards() {
    let mut g = table();
    let angel = put(&mut g, 0, "Serra Angel"); // 4/4
    let d = hand(&mut g, 0, "Disciple of Bolas");
    let n = g.players[0].hand.len();
    play(&mut g, d, false, &[], &[Action::Pick(angel)]);
    assert!(in_graveyard(&g, 0, angel));
    assert_eq!(g.obj(d).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, 44);
    assert_eq!(g.players[0].hand.len(), n - 1 + 4);
}

#[test]
fn darigaazs_caldera_bounces_a_non_lair_land_or_is_sacrificed() {
    let mut g = game(&["Swamp"]);
    let swamp = g.permanents(0).find(|&x| g.def_of(x).name == "Swamp").unwrap();
    let c = hand(&mut g, 0, "Darigaaz's Caldera");
    act(&mut g, Action::PlayLand(c));
    resolve_all(&mut g, &[Action::Pick(swamp)]);
    assert_eq!(g.obj(swamp).zone, Zone::Hand);
    assert_eq!(g.obj(c).zone, Zone::Battlefield);
    assert_eq!(
        g.def_of(c).mana.map(|m| m.colors),
        Some([false, false, true, true, true])
    );
    let mut g = game(&[]);
    let c = hand(&mut g, 0, "Darigaaz's Caldera");
    act(&mut g, Action::PlayLand(c));
    resolve_all(&mut g, &[Action::No]);
    assert!(in_graveyard(&g, 0, c), "no other land to return");
}

// ---- Core: triggers, batch 3 (dies, discard, life and cast emits) ----

/// Passes until the stack is empty, choosing `target` for every trigger and answering `Yes` to
/// every "you may".
fn settle(g: &mut Game, target: Target) {
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) => Action::Target(target),
            Pending::Decide(_) => Action::Yes,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

/// Hero's Downfall (with three more Swamps for it) on `target`, then settles toward player 1.
fn kill(g: &mut Game, target: ObjId) {
    for _ in 0..3 {
        put(g, 0, "Swamp");
    }
    let d = hand(g, 0, "Hero's Downfall");
    act(g, Action::Cast(d));
    act(g, Action::Target(obj(target)));
    settle(g, Target::Player(1));
    assert_eq!(g.obj(target).zone, Zone::Graveyard);
}

/// Casts `name` from hand and settles toward player 1.
fn cast_settle(g: &mut Game, name: &str) -> ObjId {
    let c = hand(g, 0, name);
    act(g, Action::Cast(c));
    settle(g, Target::Player(1));
    c
}

#[test]
fn sanguine_bond_vito_and_defiant_bloodlord_drain_what_you_gain() {
    for name in [
        "Sanguine Bond",
        "Vito, Thorn of the Dusk Rose",
        "Defiant Bloodlord",
    ] {
        let mut g = table();
        put(&mut g, 0, name);
        let kokusho = put(&mut g, 0, "Kokusho, the Evening Star");
        kill(&mut g, kokusho); // each opponent loses 5, you gain 5
        assert_eq!(g.players[0].life, 45, "{name}");
        assert_eq!(g.players[1].life, 30, "{name}: then loses that much more");
    }
}

#[test]
fn vito_gives_your_creatures_lifelink() {
    let mut g = table();
    let vito = put(&mut g, 0, "Vito, Thorn of the Dusk Rose");
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::Activate(vito, 0));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(lions).has(Keywords::LIFELINK));
}

#[test]
fn pitiless_plunderer_makes_a_treasure_when_another_of_yours_dies() {
    let mut g = table();
    let p = put(&mut g, 0, "Pitiless Plunderer");
    let lions = put(&mut g, 0, "Savannah Lions");
    let theirs = put(&mut g, 1, "Savannah Lions");
    kill(&mut g, lions);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1);
    kill(&mut g, theirs);
    kill(&mut g, p);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1, "not theirs, not itself");
}

#[test]
fn elas_il_kor_gains_on_enter_and_drains_on_death() {
    let mut g = table();
    put(&mut g, 0, "Elas il-Kor, Sadistic Pilgrim");
    let lions = cast_settle(&mut g, "Savannah Lions");
    assert_eq!(g.players[0].life, 41);
    kill(&mut g, lions);
    assert_eq!(g.players[1].life, 39);
}

#[test]
fn undead_augur_draws_and_loses_when_it_or_another_zombie_dies() {
    let mut g = table();
    let augur = put(&mut g, 0, "Undead Augur");
    let lions = put(&mut g, 0, "Savannah Lions");
    let n = g.players[0].hand.len();
    kill(&mut g, lions);
    assert_eq!(g.players[0].hand.len(), n, "not a Zombie");
    kill(&mut g, augur);
    assert_eq!(g.players[0].hand.len(), n + 1);
    assert_eq!(g.players[0].life, 39);
}

#[test]
fn disciple_of_the_vault_drains_when_any_artifact_goes_to_a_graveyard() {
    let mut g = table();
    put(&mut g, 0, "Disciple of the Vault");
    for p in [1, 0] {
        let ring = put(&mut g, p, "Sol Ring");
        let decay = hand(&mut g, 0, "Abrupt Decay");
        act(&mut g, Action::Cast(decay));
        act(&mut g, Action::Target(obj(ring)));
        settle(&mut g, Target::Player(1));
    }
    assert_eq!(g.players[1].life, 38, "theirs and yours");
}

#[test]
fn viridian_revel_draws_when_an_opponents_artifact_goes_to_the_graveyard() {
    let mut g = table();
    put(&mut g, 0, "Viridian Revel");
    let n = g.players[0].hand.len();
    for p in [1, 0] {
        let ring = put(&mut g, p, "Sol Ring");
        let decay = hand(&mut g, 0, "Abrupt Decay");
        act(&mut g, Action::Cast(decay));
        act(&mut g, Action::Target(obj(ring)));
        settle(&mut g, Target::Player(1));
    }
    assert_eq!(g.players[0].hand.len(), n + 1, "only the opponent's");
}

/// Thrór's Map's "{2}, {T}: Draw a card, then discard a card", discarding `card`; triggers then
/// settle toward player 1.
fn loot_away(g: &mut Game, card: ObjId) {
    let map = put(g, 0, "Thrór's Map");
    act(g, Action::Activate(map, 0));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(g, Action::PassPriority);
    }
    act(g, Action::Discard(card));
    settle(g, Target::Player(1));
}

#[test]
fn feast_of_sanity_pings_and_gains_on_each_discard() {
    let mut g = table();
    put(&mut g, 0, "Feast of Sanity");
    let c = hand(&mut g, 0, "Sol Ring");
    loot_away(&mut g, c);
    assert!(in_graveyard(&g, 0, c));
    assert_eq!(g.players[1].life, 39);
    assert_eq!(g.players[0].life, 41);
}

#[test]
fn shadowstorm_vizier_grows_when_you_discard() {
    let mut g = table();
    let v = put(&mut g, 0, "Shadowstorm Vizier");
    let c = hand(&mut g, 0, "Sol Ring");
    loot_away(&mut g, c);
    assert_eq!(g.power(v), 2);
}

#[test]
fn drake_haven_pays_one_for_a_drake_on_each_discard() {
    let mut g = table();
    put(&mut g, 0, "Drake Haven");
    let c = hand(&mut g, 0, "Sol Ring");
    loot_away(&mut g, c);
    assert_eq!(tokens(&g, 0, "Drake").len(), 1);
}

#[test]
fn enchantresses_draw_when_you_cast_an_enchantment() {
    for name in [
        "Enchantress's Presence",
        "Argothian Enchantress",
        "Satyr Enchanter",
        "Mesa Enchantress",
        "Verduran Enchantress",
        "Sythis, Harvest's Hand",
    ] {
        let mut g = table();
        put(&mut g, 0, name);
        let n = g.players[0].hand.len();
        cast_settle(&mut g, "Sol Ring");
        assert_eq!(
            g.players[0].hand.len(),
            n,
            "{name}: an artifact isn't an enchantment"
        );
        cast_settle(&mut g, "Viridian Revel");
        assert_eq!(g.players[0].hand.len(), n + 1, "{name}");
        let gained = if name.starts_with("Sythis") { 1 } else { 0 };
        assert_eq!(g.players[0].life, 40 + gained, "{name}");
    }
}

#[test]
fn mirran_spy_untaps_a_creature_when_you_cast_an_artifact() {
    let mut g = table();
    put(&mut g, 0, "Mirran Spy");
    let lions = put(&mut g, 0, "Savannah Lions");
    attack(&mut g, &[lions]);
    while g.step != Step::Main2 {
        act(&mut g, Action::PassPriority);
    }
    assert!(g.obj(lions).tapped);
    let ring = hand(&mut g, 0, "Sol Ring");
    act(&mut g, Action::Cast(ring));
    settle(&mut g, obj(lions));
    assert!(!g.obj(lions).tapped);
}

#[test]
fn revel_in_riches_makes_treasures_and_wins_with_ten() {
    let mut g = table();
    put(&mut g, 0, "Revel in Riches");
    let theirs = put(&mut g, 1, "Savannah Lions");
    kill(&mut g, theirs);
    let treasure = tokens(&g, 0, "Treasure");
    assert_eq!(treasure.len(), 1);
    g.start_turn(4, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert!(g.result.is_none(), "one Treasure");
    let def = g.obj(treasure[0]).def;
    for _ in 0..9 {
        g.add_card(0, def, Zone::Battlefield);
    }
    g.start_turn(5, 0);
    while g.result.is_none() && g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.result, Some(bolt_core::Outcome::Winner(0)));
}

#[test]
fn yahenni_grows_when_theirs_die_and_sacrifices_for_indestructible() {
    let mut g = table();
    let y = put(&mut g, 0, "Yahenni, Undying Partisan");
    let theirs = put(&mut g, 1, "Savannah Lions");
    let mine = put(&mut g, 0, "Savannah Lions");
    kill(&mut g, theirs);
    assert_eq!(g.power(y), 3);
    act(&mut g, Action::ActivateWith(y, 0, mine));
    resolve_all(&mut g, &[]);
    assert!(in_graveyard(&g, 0, mine));
    assert!(g.keywords(y).has(Keywords::INDESTRUCTIBLE));
    assert_eq!(g.power(y), 3, "your own creature doesn't count");
}

/// Treetop Snarespinner's counter on `target`, twice.
fn two_counters(g: &mut Game, target: ObjId) {
    let s = put(g, 0, "Treetop Snarespinner");
    for _ in 0..2 {
        act(g, Action::Activate(s, 0));
        act(g, Action::Target(obj(target)));
        settle(g, Target::Player(1));
    }
}

#[test]
fn herd_baloth_makes_a_beast_each_time_counters_go_on_it() {
    let mut g = table();
    let b = put(&mut g, 0, "Herd Baloth");
    two_counters(&mut g, b);
    assert_eq!(tokens(&g, 0, "Beast").len(), 2, "every time, not once a turn");
}

#[test]
fn scurry_oak_evolves_into_squirrels() {
    let mut g = table();
    let oak = put(&mut g, 0, "Scurry Oak");
    cast_settle(&mut g, "Savannah Lions"); // 2/1: greater power
    assert_eq!(g.power(oak), 2);
    assert_eq!(tokens(&g, 0, "Squirrel").len(), 1);
}

#[test]
fn vilis_draws_for_the_life_you_lose_and_shrinks_for_two_life() {
    let mut g = table();
    let vilis = put(&mut g, 0, "Vilis, Broker of Blood");
    let lions = put(&mut g, 1, "Savannah Lions");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(vilis, 0));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 38);
    assert_eq!(g.players[0].hand.len(), n + 2, "paying 2 life is losing 2");
    assert!(in_graveyard(&g, 1, lions), "a 2/1 with -1/-1");
}

// ---- Core: triggers, batch 4 (suppressed and doubled triggers) ----

#[test]
fn hushbringer_stops_creatures_entering_or_dying_from_triggering() {
    let mut g = table();
    put(&mut g, 1, "Hushbringer");
    put(&mut g, 0, "Elas il-Kor, Sadistic Pilgrim");
    let lions = cast_settle(&mut g, "Savannah Lions");
    kill(&mut g, lions);
    assert_eq!(g.players[0].life, 40);
    assert_eq!(g.players[1].life, 40);
}

#[test]
fn doorkeeper_thrull_stops_entering_but_not_dying() {
    let mut g = table();
    put(&mut g, 1, "Doorkeeper Thrull");
    put(&mut g, 0, "Elas il-Kor, Sadistic Pilgrim");
    let lions = cast_settle(&mut g, "Savannah Lions");
    assert_eq!(g.players[0].life, 40);
    kill(&mut g, lions);
    assert_eq!(g.players[1].life, 39);
}

#[test]
fn elesh_norn_doubles_your_entering_triggers_and_stops_theirs() {
    let mut g = table();
    put(&mut g, 0, "Elesh Norn, Mother of Machines");
    put(&mut g, 0, "Elas il-Kor, Sadistic Pilgrim");
    put(&mut g, 1, "Elas il-Kor, Sadistic Pilgrim");
    cast_settle(&mut g, "Savannah Lions");
    assert_eq!(g.players[0].life, 42, "twice");
    g.start_turn(4, 1);
    while !(g.step == Step::Main1 && g.pending == Pending::Priority(1)) {
        act(&mut g, Action::PassPriority);
    }
    let theirs = hand(&mut g, 1, "Savannah Lions");
    act(&mut g, Action::Cast(theirs));
    settle(&mut g, Target::Player(0));
    assert_eq!(g.obj(theirs).zone, Zone::Battlefield);
    assert_eq!(g.players[1].life, 40, "their Elas doesn't trigger");
}

#[test]
fn harmonic_prodigy_doubles_another_wizards_triggers() {
    let mut g = table();
    let prodigy = put(&mut g, 0, "Harmonic Prodigy");
    let augur = put(&mut g, 0, "Undead Augur"); // a Zombie Wizard
    let n = g.players[0].hand.len();
    kill(&mut g, augur);
    assert_eq!(g.players[0].hand.len(), n + 2, "twice");
    assert_eq!(g.players[0].life, 38);
    let before = g.power(prodigy);
    cast_settle(&mut g, "Sol Ring");
    assert_eq!(g.power(prodigy), before + 1, "its own prowess isn't doubled");
}

// ---- Core: triggers, batch 5 (sacrifice, enter and life-lost emits) ----

#[test]
fn mazirek_counters_your_creatures_when_a_player_sacrifices() {
    let mut g = table();
    let m = put(&mut g, 0, "Mazirek, Kraul Death Priest");
    let lions = put(&mut g, 0, "Savannah Lions");
    put(&mut g, 1, "Savannah Lions");
    let e = hand(&mut g, 0, "Diabolic Edict");
    play(&mut g, e, false, &[Target::Player(1)], &[]);
    assert_eq!(g.power(m), 3);
    assert_eq!(g.power(lions), 3);
}

#[test]
fn mirkwood_bats_drains_when_you_create_or_sacrifice_a_token() {
    let mut g = table();
    put(&mut g, 0, "Mirkwood Bats");
    put(&mut g, 0, "Pitiless Plunderer");
    let lions = put(&mut g, 0, "Savannah Lions");
    kill(&mut g, lions); // a Treasure is created
    assert_eq!(g.players[1].life, 39);
    let t = tokens(&g, 0, "Treasure")[0];
    g.manual_mana[0] = true;
    act(&mut g, Action::TapForMana(t, 0));
    g.manual_mana[0] = false;
    cast_settle(&mut g, "Sol Ring"); // triggers go on the stack as priority returns
    assert!(g.obj(t).zone != Zone::Battlefield);
    assert_eq!(g.players[1].life, 38, "sacrificed");
}

#[test]
fn al_bhed_salvagers_drains_when_it_or_your_creature_or_artifact_dies() {
    let mut g = table();
    let a = put(&mut g, 0, "Al Bhed Salvagers");
    let ring = put(&mut g, 0, "Sol Ring");
    let decay = hand(&mut g, 0, "Abrupt Decay");
    act(&mut g, Action::Cast(decay));
    act(&mut g, Action::Target(obj(ring)));
    settle(&mut g, Target::Player(1));
    assert_eq!(g.players[1].life, 39);
    kill(&mut g, a);
    assert_eq!(g.players[1].life, 38, "itself");
    assert_eq!(g.players[0].life, 42);
}

#[test]
fn altar_of_the_brood_mills_when_another_permanent_of_yours_enters() {
    let mut g = table();
    put(&mut g, 0, "Altar of the Brood");
    let n = g.players[1].graveyard.len();
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    settle(&mut g, Target::Player(1));
    assert_eq!(g.players[1].graveyard.len(), n + 1);
}

#[test]
fn ivy_lane_denizen_counters_when_another_green_creature_enters() {
    let mut g = table();
    put(&mut g, 0, "Ivy Lane Denizen");
    let lions = put(&mut g, 0, "Savannah Lions");
    cast_settle(&mut g, "Savannah Lions"); // white: nothing
    assert_eq!(g.power(lions), 2);
    let oak = hand(&mut g, 0, "Scurry Oak");
    act(&mut g, Action::Cast(oak));
    settle(&mut g, obj(lions));
    assert_eq!(g.power(lions), 3);
}

#[test]
fn wound_reflection_doubles_the_opponents_loss_at_end_step() {
    let mut g = table();
    put(&mut g, 0, "Wound Reflection");
    let angel = put(&mut g, 0, "Serra Angel");
    let f = hand(&mut g, 0, "Soul's Fire");
    play(&mut g, f, false, &[obj(angel), Target::Player(1)], &[]);
    assert_eq!(g.players[1].life, 36);
    while g.step != Step::Cleanup && g.turn == 3 {
        let a = if g.pending == Pending::DeclareAttackers(0) {
            Action::FinishAttacks
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[1].life, 32);
}

// ---- Core: triggers, batch 6 (cast, copy, damage, graveyard and batched emits) ----

/// Passes to player 0's second main phase, answering blocks with none and may-triggers with yes.
fn through_combat(g: &mut Game) {
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::Decide(_) => Action::Yes,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn witherbloom_apprentice_drains_on_an_instant() {
    let mut g = table();
    put(&mut g, 0, "Witherbloom Apprentice");
    let lions = put(&mut g, 1, "Savannah Lions");
    kill(&mut g, lions);
    assert_eq!(g.players[1].life, 39);
    assert_eq!(g.players[0].life, 41);
    cast_settle(&mut g, "Sol Ring");
    assert_eq!(g.players[1].life, 39, "an artifact spell isn't magecraft");
}

#[test]
fn glimpse_of_nature_draws_for_each_creature_spell_this_turn() {
    let mut g = table();
    cast_settle(&mut g, "Glimpse of Nature");
    let n = g.players[0].library.len();
    cast_settle(&mut g, "Savannah Lions");
    cast_settle(&mut g, "Savannah Lions");
    cast_settle(&mut g, "Sol Ring");
    assert_eq!(g.players[0].library.len(), n - 2);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let n = g.players[0].library.len();
    cast_settle(&mut g, "Savannah Lions");
    assert_eq!(g.players[0].library.len(), n, "only that turn");
}

#[test]
fn zenith_chronicler_draws_for_the_other_player_on_a_first_multicolored_spell() {
    let mut g = table();
    put(&mut g, 0, "Zenith Chronicler");
    let n = g.players[1].library.len();
    for _ in 0..2 {
        let ring = put(&mut g, 1, "Sol Ring");
        let decay = hand(&mut g, 0, "Abrupt Decay");
        play(&mut g, decay, false, &[obj(ring)], &[]);
    }
    assert_eq!(
        g.players[1].library.len(),
        n - 1,
        "the first multicolored spell only"
    );
}

#[test]
fn no_mercy_destroys_a_creature_that_damages_you() {
    let mut g = table();
    put(&mut g, 1, "No Mercy");
    let lions = put(&mut g, 0, "Savannah Lions");
    attack(&mut g, &[lions]);
    through_combat(&mut g);
    assert_eq!(g.players[1].life, 38);
    assert!(in_graveyard(&g, 0, lions));
}

#[test]
fn francisco_explores_when_a_pirate_deals_damage_to_a_player() {
    let mut g = table();
    let f = put(&mut g, 0, "Francisco, Fowl Marauder");
    assert!(g.def_of(f).cant_block);
    g.objects[f.0 as usize].counters = 1; // a 1/2 Pirate
    let hand_before = g.players[0].hand.len();
    attack(&mut g, &[f]);
    through_combat(&mut g);
    assert_eq!(g.players[1].life, 39);
    // The draw step drew one; the explore revealed a Plains (a land) and took it.
    assert_eq!(g.players[0].hand.len(), hand_before + 2);
}

#[test]
fn syr_konrad_pings_for_deaths_discards_and_cards_leaving_your_graveyard() {
    let mut g = table();
    put(&mut g, 0, "Syr Konrad, the Grim");
    let lions = put(&mut g, 1, "Savannah Lions");
    kill(&mut g, lions);
    assert_eq!(g.players[1].life, 39, "another creature died");
    let mine = hand(&mut g, 0, "Savannah Lions");
    loot_away(&mut g, mine);
    assert_eq!(g.players[1].life, 38, "a creature card from anywhere else");
    let ring = hand(&mut g, 0, "Sol Ring");
    loot_away(&mut g, ring);
    assert_eq!(g.players[1].life, 38, "not a creature card");
    let lib = g.players[1].library.len();
    let k = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Syr Konrad, the Grim")
        .unwrap();
    act(&mut g, Action::Activate(k, 0));
    settle(&mut g, Target::Player(1));
    assert_eq!(g.players[1].library.len(), lib - 1, "each player mills");
}

#[test]
fn species_specialist_draws_when_the_chosen_type_dies() {
    let mut g = table();
    let s = put(&mut g, 0, "Species Specialist");
    g.objects[s.0 as usize].chosen_type = Some("Cat".to_string());
    let n = g.players[0].library.len();
    let lions = put(&mut g, 1, "Savannah Lions");
    kill(&mut g, lions);
    assert_eq!(g.players[0].library.len(), n - 1);
    let angel = put(&mut g, 1, "Serra Angel");
    kill(&mut g, angel);
    assert_eq!(g.players[0].library.len(), n - 1, "not a Cat");
}

#[test]
fn bloodchief_ascension_quests_on_life_lost_then_drains_on_cards_to_their_graveyard() {
    let mut g = table();
    let b = put(&mut g, 0, "Bloodchief Ascension");
    let lions = put(&mut g, 1, "Savannah Lions");
    kill(&mut g, lions);
    assert_eq!(g.players[1].life, 40, "no quest counters yet");
    let angel = put(&mut g, 0, "Serra Angel");
    let f = hand(&mut g, 0, "Soul's Fire");
    play(&mut g, f, false, &[obj(angel), Target::Player(1)], &[]);
    while g.turn == 3 {
        let legal = g.legal_actions();
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            _ if legal.contains(&Action::Yes) => Action::Yes,
            _ if matches!(legal.first(), Some(Action::Discard(_))) => legal[0], // to hand size
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(b).quest, 1);
    g.objects[b.0 as usize].quest = 3;
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let lions = put(&mut g, 1, "Savannah Lions");
    let life = g.players[1].life;
    kill(&mut g, lions);
    assert_eq!(g.players[1].life, life - 2);
    assert_eq!(g.players[0].life, 42);
}

#[test]
fn cryptcaller_chariot_makes_tapped_zombies_for_discards() {
    let mut g = table();
    put(&mut g, 0, "Cryptcaller Chariot");
    let c = hand(&mut g, 0, "Sol Ring");
    loot_away(&mut g, c);
    let z = tokens(&g, 0, "Zombie");
    assert_eq!(z.len(), 1);
    assert!(g.obj(z[0]).tapped);
    assert_eq!((g.power(z[0]), g.toughness(z[0])), (2, 2));
}

// ---- Core: triggers, batch 7 ----

/// Settles the stack, answering each target with the first legal one and each question with yes.
fn settle_first(g: &mut Game) {
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let legal = g.legal_actions();
        let a = match g.pending {
            Pending::ChooseTarget(_) => legal[0],
            Pending::Decide(_) if legal.contains(&Action::Yes) => Action::Yes,
            Pending::Decide(_) => legal[0],
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn ghostly_pilferer_pays_to_draw_on_untap_and_draws_on_a_spell_not_from_hand() {
    let mut g = table();
    let gp = put(&mut g, 0, "Ghostly Pilferer");
    g.objects[gp.0 as usize].tapped = true;
    let lib = g.players[0].library.len();
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        let a = if matches!(g.pending, Pending::Decide(_)) {
            Action::Yes
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
    }
    assert_eq!(
        g.players[0].library.len(),
        lib - 2,
        "the draw step and the paid trigger"
    );
    // The opponent casts their commander from the command zone.
    g.start_turn(6, 1);
    while !(g.step == Step::Main1 && g.active == 1) {
        act(&mut g, Action::PassPriority);
    }
    let lib = g.players[0].library.len();
    let hand_spell = hand(&mut g, 1, "Sol Ring");
    act(&mut g, Action::Cast(hand_spell));
    settle_first(&mut g);
    assert_eq!(g.players[0].library.len(), lib, "a spell from hand doesn't draw");
    // (Cast before Ms. Bumbleflower is out: her cast trigger makes the opponent draw.)
    let c = g.commanders[1].expect("a commander");
    act(&mut g, Action::Cast(c));
    settle_first(&mut g);
    assert_eq!(g.players[0].library.len(), lib - 1, "cast from the command zone");
}

#[test]
fn ghostly_pilferer_discards_to_become_unblockable() {
    let mut g = table();
    let gp = put(&mut g, 0, "Ghostly Pilferer");
    let c = hand(&mut g, 0, "Sol Ring");
    act(&mut g, Action::ActivateWith(gp, 0, c)); // the discarded card
    while !g.items.is_empty() || matches!(g.pending, Pending::Decide(_)) {
        let a = if g.legal_actions().contains(&Action::Discard(c)) {
            Action::Discard(c)
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
    }
    assert!(in_graveyard(&g, 0, c));
    assert!(g.keywords(gp).has(Keywords::UNBLOCKABLE));
}

// ---- Core: triggers, batch 9 ----

/// Player 0's main phase on turn 5, so creatures put in on turn 3 can tap.
fn main5(g: &mut Game) {
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(g, Action::PassPriority);
    }
}

#[test]
fn archmage_emeritus_draws_on_an_instant() {
    let mut g = table();
    put(&mut g, 0, "Archmage Emeritus");
    let n = g.players[0].library.len();
    let lions = put(&mut g, 1, "Savannah Lions");
    kill(&mut g, lions);
    assert_eq!(g.players[0].library.len(), n - 1);
    cast_settle(&mut g, "Sol Ring");
    assert_eq!(g.players[0].library.len(), n - 1, "not an instant or sorcery");
}

#[test]
fn lobber_crew_pings_and_untaps_on_a_multicolored_spell() {
    let mut g = table();
    let lc = put(&mut g, 0, "Lobber Crew");
    let spitfire = put(&mut g, 0, "Chandra's Spitfire");
    main5(&mut g);
    act(&mut g, Action::Activate(lc, 0));
    settle(&mut g, Target::Player(1));
    assert_eq!(g.players[1].life, 39);
    assert!(g.obj(lc).tapped);
    assert_eq!(
        g.power(spitfire),
        4,
        "Chandra's Spitfire: the opponent was dealt noncombat damage"
    );
    let ring = put(&mut g, 1, "Sol Ring");
    let decay = hand(&mut g, 0, "Abrupt Decay");
    play(&mut g, decay, false, &[obj(ring)], &[]);
    assert!(!g.obj(lc).tapped, "a multicolored spell untaps it");
}

#[test]
fn chandras_spitfire_ignores_combat_damage() {
    let mut g = table();
    let spitfire = put(&mut g, 0, "Chandra's Spitfire");
    let lions = put(&mut g, 0, "Savannah Lions");
    attack(&mut g, &[lions]);
    through_combat(&mut g);
    assert_eq!(g.players[1].life, 38);
    assert_eq!(g.power(spitfire), 1);
}

#[test]
fn curiosity_and_ophidian_eye_draw_when_the_creature_damages_an_opponent() {
    for aura in ["Curiosity", "Ophidian Eye"] {
        let mut g = table();
        let lions = put(&mut g, 0, "Savannah Lions");
        let a = hand(&mut g, 0, aura);
        play(&mut g, a, false, &[obj(lions)], &[]);
        assert_eq!(g.obj(a).zone, Zone::Battlefield, "{aura}");
        let n = g.players[0].library.len();
        attack(&mut g, &[lions]);
        through_combat(&mut g);
        assert_eq!(g.players[1].life, 38, "{aura}");
        assert_eq!(
            g.players[0].library.len(),
            n - 2,
            "{aura}: the draw step and the trigger"
        );
    }
}

// ---- Core: triggers, batch 10 ----

/// Passes (answering with `target` when it's legal, else the first legal choice) until the turn
/// that's on now is over.
fn finish_turn(g: &mut Game, target: Target) {
    let turn = g.turn;
    while g.turn == turn && g.result.is_none() {
        let legal = g.legal_actions();
        let a = if legal.contains(&Action::Target(target)) {
            Action::Target(target)
        } else if legal.contains(&Action::PassPriority) {
            Action::PassPriority
        } else if legal.contains(&Action::FinishAttacks) {
            Action::FinishAttacks
        } else if legal.contains(&Action::FinishBlocks) {
            Action::FinishBlocks
        } else {
            legal[0]
        };
        act(g, a);
    }
}

#[test]
fn witch_of_the_moors_needs_life_gained_this_turn() {
    for gain in [false, true] {
        let mut g = table();
        put(&mut g, 0, "Witch of the Moors");
        let theirs = put(&mut g, 1, "Savannah Lions");
        let card = g.add_card(0, cid("Savannah Lions"), Zone::Graveyard);
        if gain {
            let mine = put(&mut g, 0, "Savannah Lions");
            let s = hand(&mut g, 0, "Swords to Plowshares");
            play(&mut g, s, false, &[obj(mine)], &[]);
            assert_eq!(g.players[0].life, 42);
        }
        finish_turn(&mut g, obj(card));
        assert_eq!(in_graveyard(&g, 1, theirs), gain, "the opponent sacrifices");
        assert_eq!(g.obj(card).zone == Zone::Hand, gain, "the card comes back");
    }
}

#[test]
fn felidar_sovereign_and_test_of_endurance_win_at_their_life_totals() {
    for (name, n) in [("Felidar Sovereign", 40), ("Test of Endurance", 50)] {
        let mut g = table();
        put(&mut g, 0, name);
        g.players[0].life = n - 1;
        g.start_turn(5, 0);
        to_main1(&mut g);
        assert!(g.result.is_none(), "{name}: {} life", n - 1);
        g.players[0].life = n;
        g.start_turn(7, 0);
        to_main1(&mut g);
        assert_eq!(g.result, Some(bolt_core::Outcome::Winner(0)), "{name}");
    }
}

fn to_main1(g: &mut Game) {
    while g.result.is_none() && g.step != Step::Main1 {
        act(g, Action::PassPriority);
    }
}

#[test]
fn pyrohemia_pings_everything_and_goes_with_no_creatures() {
    let mut g = table();
    let p = put(&mut g, 0, "Pyrohemia");
    let lions = put(&mut g, 1, "Savannah Lions");
    let bears = put(&mut g, 1, "Vampire Nighthawk");
    act(&mut g, Action::Activate(p, 0));
    resolve_all(&mut g, &[]);
    assert!(in_graveyard(&g, 1, lions));
    assert_eq!(g.obj(bears).zone, Zone::Battlefield, "2 toughness");
    assert_eq!((g.players[0].life, g.players[1].life), (39, 39));
    finish_turn(&mut g, Target::Player(1));
    assert_eq!(g.obj(p).zone, Zone::Battlefield, "a creature is out");
    let mut g = table();
    let p = put(&mut g, 0, "Pyrohemia");
    finish_turn(&mut g, Target::Player(1));
    assert!(in_graveyard(&g, 0, p), "no creatures at the end step");
}

#[test]
fn vizkopa_guildmage_grants_lifelink_and_drains_what_you_gain() {
    let mut g = table();
    let vg = put(&mut g, 0, "Vizkopa Guildmage");
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::Activate(vg, 0));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(lions).has(Keywords::LIFELINK));
    act(&mut g, Action::Activate(vg, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 40, "nothing gained yet");
    let s = hand(&mut g, 0, "Swords to Plowshares");
    play(&mut g, s, false, &[obj(lions)], &[]);
    assert_eq!(g.players[0].life, 42);
    assert_eq!(g.players[1].life, 38, "that much life");
    g.start_turn(5, 0);
    to_main1(&mut g);
    let other = put(&mut g, 0, "Savannah Lions");
    let s = hand(&mut g, 0, "Swords to Plowshares");
    play(&mut g, s, false, &[obj(other)], &[]);
    assert_eq!(g.players[1].life, 38, "only that turn");
}

#[test]
fn amulet_of_vigor_untaps_what_enters_tapped() {
    let mut g = table();
    put(&mut g, 0, "Amulet of Vigor");
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    act(&mut g, Action::Activate(elder, 0));
    resolve_all(&mut g, &[Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(!g.obj(forest).tapped, "it entered tapped, then untapped");
}

#[test]
fn skittering_precursor_makes_a_spawn_for_a_nontoken_sacrifice() {
    let mut g = table();
    let sp = put(&mut g, 0, "Skittering Precursor");
    assert_eq!(g.colors_of(sp), [false; 5], "devoid");
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    act(&mut g, Action::Activate(elder, 0));
    resolve_all(&mut g, &[Action::Pick(forest)]);
    let spawn = tokens(&g, 0, "Eldrazi Spawn");
    assert_eq!(spawn.len(), 1);
    g.manual_mana[0] = true;
    let n = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a, Action::Activate(x, _) | Action::TapForMana(x, _) if *x == spawn[0]))
        .expect("the spawn's mana ability");
    act(&mut g, n);
    resolve_all(&mut g, &[]);
    assert!(tokens(&g, 0, "Eldrazi Spawn").is_empty(), "a token: no new spawn");
}

#[test]
fn it_that_betrays_takes_what_the_opponent_sacrifices() {
    let mut g = table();
    put(&mut g, 0, "It That Betrays");
    let lions = put(&mut g, 1, "Savannah Lions");
    let e = hand(&mut g, 0, "Diabolic Edict");
    play(&mut g, e, false, &[Target::Player(1)], &[]);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield);
    assert_eq!(g.obj(lions).controller, 0);
}

#[test]
fn glaring_fleshraker_makes_spawn_that_ping() {
    let mut g = table();
    put(&mut g, 0, "Glaring Fleshraker");
    cast_settle(&mut g, "Sol Ring");
    assert_eq!(tokens(&g, 0, "Eldrazi Spawn").len(), 1);
    assert_eq!(g.players[1].life, 39, "the spawn is a colorless creature");
    cast_settle(&mut g, "Savannah Lions");
    assert_eq!(tokens(&g, 0, "Eldrazi Spawn").len(), 1, "a white spell");
    assert_eq!(g.players[1].life, 39, "a white creature");
}

#[test]
fn aurelia_untaps_and_adds_a_combat_the_first_time_only() {
    let mut g = table();
    let a = put(&mut g, 0, "Aurelia, the Warleader");
    let lions = put(&mut g, 0, "Savannah Lions");
    attack(&mut g, &[a, lions]);
    while !g.items.is_empty()
        || g.legal_actions().contains(&Action::PassPriority) && g.step == Step::DeclareAttackers
    {
        act(&mut g, Action::PassPriority);
    }
    assert!(!g.obj(lions).tapped, "untapped");
    let mut attacks = 1;
    while g.step != Step::Main2 {
        let legal = g.legal_actions();
        let next = if g.pending == Pending::DeclareAttackers(0) {
            attacks += 1;
            act(&mut g, Action::Attack(a));
            Action::FinishAttacks
        } else if legal.contains(&Action::FinishBlocks) {
            Action::FinishBlocks
        } else {
            Action::PassPriority
        };
        act(&mut g, next);
    }
    assert_eq!(attacks, 2, "one additional combat");
    assert_eq!(
        g.players[1].life,
        40 - 3 - 2 - 3,
        "Aurelia (3/4) twice, the Lions once"
    );
}

#[test]
fn ghyrson_starn_adds_two_to_exactly_one_damage() {
    let mut g = table();
    put(&mut g, 0, "Ghyrson Starn, Kelermorph");
    let lc = put(&mut g, 0, "Lobber Crew");
    main5(&mut g);
    act(&mut g, Action::Activate(lc, 0));
    settle(&mut g, Target::Player(1));
    assert_eq!(g.players[1].life, 37, "1, then Ghyrson's 2");
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    assert_eq!(g.players[1].life, 34, "3 damage isn't exactly 1");
}

#[test]
fn graf_mole_gains_three_when_you_sacrifice_a_clue() {
    let mut g = table();
    put(&mut g, 0, "Graf Mole");
    let clue_def = set("CMD")
        .unwrap()
        .defs
        .iter()
        .position(|d| d.token && d.name == "Clue")
        .expect("the Clue token");
    let clue = g.add_card(0, clue_def, Zone::Battlefield);
    act(&mut g, Action::Activate(clue, 0));
    resolve_all(&mut g, &[]);
    assert_ne!(g.obj(clue).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, 43);
}

/// Passes until turn `turn` is over (attacks and blocks declared empty).
fn end_turn(g: &mut Game, turn: u32) {
    while g.turn == turn && g.result.is_none() {
        let legal = g.legal_actions();
        let a = [Action::PassPriority, Action::FinishAttacks, Action::FinishBlocks]
            .into_iter()
            .find(|a| legal.contains(a))
            .unwrap_or(legal[0]);
        act(g, a);
    }
}

#[test]
fn lunar_convocation_drains_after_a_gain_and_makes_a_bat_after_both() {
    let mut g = table();
    let lc = put(&mut g, 0, "Lunar Convocation");
    put(&mut g, 0, "Aetherflux Reservoir");
    cast_settle(&mut g, "Sol Ring"); // Aetherflux: gain 1
    end_turn(&mut g, 3);
    assert_eq!(g.players[1].life, 39, "gained life: each opponent loses 1");
    assert!(tokens(&g, 0, "Bat").is_empty(), "no life lost");
    main5(&mut g);
    act(&mut g, Action::Activate(lc, 0)); // {1}{B}, pay 2 life: draw
    resolve_all(&mut g, &[]);
    cast_settle(&mut g, "Sol Ring");
    end_turn(&mut g, 5);
    assert_eq!(g.players[1].life, 38);
    let bat = tokens(&g, 0, "Bat");
    assert_eq!(bat.len(), 1, "gained and lost life");
    assert!(g.keywords(bat[0]).has(Keywords::FLYING));
    assert_eq!(g.colors_of(bat[0]), [false, false, true, false, false]);
    assert_eq!(g.power(bat[0]), 1);
}

#[test]
fn saffi_returns_the_target_creature_when_it_dies_this_turn() {
    let mut g = table();
    let saffi = put(&mut g, 0, "Saffi Eriksdotter");
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::Activate(saffi, 0));
    act(&mut g, Action::Target(Target::Object(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(saffi).zone, Zone::Graveyard, "sacrificed as the cost");
    for _ in 0..3 {
        put(&mut g, 0, "Swamp");
    }
    let d = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, d, false, &[Target::Object(lions)], &[]);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield, "back");
}

#[test]
fn sefris_ventures_on_a_creature_card_and_reanimates_on_completing_a_dungeon() {
    use bolt_core::{rooms, Dungeon};
    let mut g = table();
    put(&mut g, 0, "Sefris of the Hidden Ways");
    let lions = put(&mut g, 0, "Savannah Lions");
    let bottom = rooms(Dungeon::LostMine)
        .iter()
        .position(|r| r.next.is_empty())
        .expect("a bottommost room") as u8;
    g.venture[0] = Some((Dungeon::LostMine, bottom));
    for _ in 0..3 {
        put(&mut g, 0, "Swamp");
    }
    let d = hand(&mut g, 0, "Hero's Downfall");
    act(&mut g, Action::Cast(d));
    act(&mut g, Action::Target(obj(lions)));
    // The venture completes Lost Mine (then a new dungeon: the first choice); Create Undead
    // returns the Lions, the only creature card in the graveyard.
    settle_first(&mut g);
    assert_eq!(g.completed_dungeons[0], 1);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield, "returned");
    assert!(g.venture[0].is_some(), "and into a new dungeon");
}
