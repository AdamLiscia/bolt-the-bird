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
