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

#[test]
fn necropotence_skips_the_draw_exiles_discards_and_pays_life_for_cards() {
    let mut g = table();
    let necro = put(&mut g, 0, "Necropotence");
    let hand_before = g.players[0].hand.len();
    main5(&mut g);
    assert_eq!(g.players[0].hand.len(), hand_before, "no draw step");
    let top = *g.players[0].library.last().unwrap();
    act(&mut g, Action::Activate(necro, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 39);
    assert_eq!(g.obj(top).zone, Zone::Exile);
    assert!(g.obj(top).face_down, "face down");
    while g.players[0].hand.len() < 9 {
        hand(&mut g, 0, "Plains");
    }
    end_turn(&mut g, 5);
    assert_eq!(g.obj(top).zone, Zone::Hand, "at your end step");
    assert!(!g.obj(top).face_down);
    // The cleanup discards' exile triggers resolve (CR 514.3a) by the opponent's first main phase.
    while g.step != Step::Main1 {
        let legal = g.legal_actions();
        let a = if legal.contains(&Action::PassPriority) {
            Action::PassPriority
        } else {
            legal[0]
        };
        act(&mut g, a);
    }
    assert!(
        !g.players[0]
            .graveyard
            .iter()
            .any(|&c| g.def_of(c).name == "Plains"),
        "the cleanup discards were exiled"
    );
    assert!(g.exile.iter().any(|&c| g.def_of(c).name == "Plains"));
}

#[test]
fn sheoldred_whispering_one_reanimates_at_your_upkeep_and_edicts_at_theirs() {
    let mut g = table();
    put(&mut g, 0, "Sheoldred, Whispering One");
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Graveyard);
    let theirs = put(&mut g, 1, "Savannah Lions");
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        let a = match g.pending {
            Pending::ChooseTarget(_) => Action::Target(obj(lions)),
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(lions).zone, Zone::Battlefield, "returned at your upkeep");
    assert_eq!(
        g.obj(theirs).zone,
        Zone::Battlefield,
        "not your opponent's upkeep"
    );
    g.start_turn(6, 1);
    while g.step != Step::Main1 {
        let legal = g.legal_actions();
        let a = if legal.contains(&Action::PassPriority) {
            Action::PassPriority
        } else {
            legal[0]
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "they sacrificed it");
    assert_eq!(g.obj(lions).zone, Zone::Battlefield);
}

#[test]
fn pact_of_negation_counters_then_demands_payment_at_your_upkeep() {
    let mut g = table();
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    let pact = hand(&mut g, 0, "Pact of Negation");
    act(&mut g, Action::Cast(pact));
    act(&mut g, Action::Target(Target::Object(bolt)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 40, "countered");
    let mut skip = g.clone();
    g.start_turn(5, 0);
    let mut asked = 0;
    while g.step != Step::Main1 {
        let a = if matches!(g.pending, Pending::Decide(_)) {
            asked += 1;
            Action::Yes
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
    }
    assert_eq!(asked, 1, "asked once, at the upkeep");
    assert!(!g.players[0].lost, "paid {{3}}{{U}}{{U}}");
    // Not paying: lose.
    skip.start_turn(5, 0);
    while !matches!(skip.pending, Pending::Decide(_)) {
        act(&mut skip, Action::PassPriority);
    }
    act(&mut skip, Action::No);
    assert!(skip.players[0].lost || skip.result.is_some());
}

#[test]
fn sigil_of_the_empty_throne_makes_an_angel_for_an_enchantment_spell() {
    let mut g = table();
    put(&mut g, 0, "Sigil of the Empty Throne");
    cast_settle(&mut g, "Sol Ring");
    assert!(tokens(&g, 0, "Angel").is_empty(), "not an enchantment");
    cast_settle(&mut g, "Exquisite Blood");
    let angel = tokens(&g, 0, "Angel");
    assert_eq!(angel.len(), 1);
    assert_eq!((g.power(angel[0]), g.toughness(angel[0])), (4, 4));
    assert!(g.keywords(angel[0]).has(Keywords::FLYING));
    assert_eq!(g.colors_of(angel[0]), [true, false, false, false, false]);
}

#[test]
fn summoning_station_makes_pinchers_and_untaps_when_an_artifact_dies() {
    let mut g = table();
    let station = put(&mut g, 0, "Summoning Station");
    act(&mut g, Action::Activate(station, 0));
    resolve_all(&mut g, &[]);
    let pincher = tokens(&g, 0, "Pincher");
    assert_eq!(pincher.len(), 1);
    assert_eq!((g.power(pincher[0]), g.toughness(pincher[0])), (2, 2));
    assert!(g.obj(station).tapped);
    let clue_def = set("CMD")
        .unwrap()
        .defs
        .iter()
        .position(|d| d.token && d.name == "Clue")
        .expect("the Clue token");
    let clue = g.add_card(0, clue_def, Zone::Battlefield);
    act(&mut g, Action::Activate(clue, 0));
    resolve_all(&mut g, &[Action::Yes]); // "you may untap"
    assert!(!g.obj(station).tapped, "untapped");
}

#[test]
fn ulvenwald_mysteries_investigates_on_a_death_and_makes_soldiers_from_clues() {
    let mut g = table();
    put(&mut g, 0, "Ulvenwald Mysteries");
    let lions = put(&mut g, 0, "Savannah Lions");
    kill(&mut g, lions);
    let clues = tokens(&g, 0, "Clue");
    assert_eq!(clues.len(), 1, "investigated");
    act(&mut g, Action::Activate(clues[0], 0));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Human Soldier").len(), 1);
}

#[test]
fn fleeting_memories_investigates_and_mills_on_a_clue_sacrifice() {
    let mut g = table();
    cast_settle(&mut g, "Fleeting Memories");
    let clues = tokens(&g, 0, "Clue");
    assert_eq!(clues.len(), 1, "investigated as it entered");
    let library = g.players[1].library.len();
    act(&mut g, Action::Activate(clues[0], 0));
    settle(&mut g, Target::Player(1));
    assert_eq!(
        g.players[1].library.len(),
        library - 3,
        "target player mills three"
    );
}

// ---- Core: triggers, batch 16 ----

#[test]
fn utvara_hellkite_makes_a_dragon_for_each_dragon_attacking() {
    let mut g = table();
    let utvara = put(&mut g, 0, "Utvara Hellkite");
    let lions = put(&mut g, 0, "Savannah Lions");
    attack(&mut g, &[utvara, lions]);
    resolve_all(&mut g, &[]);
    let dragons = tokens(&g, 0, "Dragon");
    assert_eq!(dragons.len(), 1, "for Utvara, not the Lions");
    let d = dragons[0];
    assert_eq!((g.power(d), g.toughness(d)), (6, 6));
    assert!(g.keywords(d).has(Keywords::FLYING));
    assert_eq!(g.def_of(d).colors, [false, false, false, true, false]);
}

#[test]
fn well_of_lost_dreams_pays_x_up_to_the_life_gained_to_draw_x() {
    let mut g = table();
    put(&mut g, 0, "Well of Lost Dreams");
    let lions = put(&mut g, 0, "Savannah Lions");
    let swords = hand(&mut g, 0, "Swords to Plowshares");
    let before = g.players[0].hand.len();
    act(&mut g, Action::Cast(swords));
    act(&mut g, Action::Target(obj(lions)));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[0].life, 42, "gained the Lions' power");
    assert!(
        has(&g, Action::Amount(2)) && !has(&g, Action::Amount(3)),
        "X at most 2"
    );
    resolve_all(&mut g, &[Action::Amount(2)]);
    assert_eq!(g.players[0].hand.len(), before - 1 + 2);
}

#[test]
fn hullbreaker_horror_bounces_on_each_spell_you_cast() {
    let mut g = table();
    let horror = put(&mut g, 0, "Hullbreaker Horror");
    assert!(g.def_of(horror).uncounterable);
    let theirs = put(&mut g, 1, "Savannah Lions");
    let ring = hand(&mut g, 0, "Sol Ring");
    act(&mut g, Action::Cast(ring));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    // Only your own spell is on the stack: no spell you don't control to return.
    assert!(!has(&g, Action::Mode(0)));
    assert!(
        has(&g, Action::Mode(1)) && has(&g, Action::Mode(2)),
        "a permanent, or none"
    );
    resolve_all(&mut g, &[Action::Mode(1), Action::Target(obj(theirs))]);
    assert_eq!(g.obj(theirs).zone, Zone::Hand);
    assert_eq!(g.obj(ring).zone, Zone::Battlefield);
    let ring2 = hand(&mut g, 0, "Sol Ring");
    act(&mut g, Action::Cast(ring2));
    resolve_all(&mut g, &[Action::Mode(2)]);
    assert_eq!(g.obj(ring2).zone, Zone::Battlefield, "choosing none");
}

#[test]
fn call_of_the_ring_tempts_at_upkeep_and_draws_for_two_life_on_choosing() {
    let mut g = table();
    put(&mut g, 0, "Call of the Ring");
    let lions = put(&mut g, 0, "Savannah Lions");
    g.start_turn(5, 0);
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.ring_bearer(0), Some(lions), "the only creature, chosen");
    let before = g.players[0].hand.len();
    act(&mut g, Action::Yes); // pay 2 life
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 38);
    assert_eq!(g.players[0].hand.len(), before + 1);
}

#[test]
fn composer_of_spring_puts_a_land_or_with_six_enchantments_a_creature_tapped() {
    let mut g = table();
    put(&mut g, 0, "Composer of Spring");
    let forest = hand(&mut g, 0, "Forest");
    let lions = hand(&mut g, 0, "Savannah Lions");
    let blood = hand(&mut g, 0, "Exquisite Blood");
    act(&mut g, Action::Cast(blood));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(has(&g, Action::Pick(forest)) && has(&g, Action::No));
    assert!(!has(&g, Action::Pick(lions)), "one enchantment: a land only");
    act(&mut g, Action::Pick(forest));
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(g.obj(forest).tapped);
    for _ in 0..4 {
        put(&mut g, 0, "Exquisite Blood"); // five enchantments; the next one cast makes six
    }
    let blood2 = hand(&mut g, 0, "Exquisite Blood");
    act(&mut g, Action::Cast(blood2));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(has(&g, Action::Pick(lions)), "six enchantments: a creature too");
    act(&mut g, Action::Pick(lions));
    assert_eq!(g.obj(lions).zone, Zone::Battlefield);
    assert!(g.obj(lions).tapped);
}

#[test]
fn oloro_gains_two_from_the_command_zone_at_your_upkeep() {
    let mut g = table();
    let oloro = g.add_card(0, cid("Oloro, Ageless Ascetic"), Zone::Command);
    assert!(g.command.contains(&oloro));
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[0].life, 42, "the command-zone trigger only");
    assert_eq!(
        g.players[1].life, 40,
        "its gain-life trigger works only on the battlefield"
    );
}

#[test]
fn oloro_on_the_battlefield_gains_two_and_pays_one_to_drain_and_draw() {
    let mut g = table();
    put(&mut g, 0, "Oloro, Ageless Ascetic");
    g.start_turn(5, 0);
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[0].life, 42);
    let before = g.players[0].hand.len();
    act(&mut g, Action::Yes); // pay {1}
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 39);
    assert_eq!(g.players[0].hand.len(), before + 1);
}

#[test]
fn angel_of_destiny_shares_the_life_and_wins_at_fifteen_over() {
    let mut g = table();
    let angel = put(&mut g, 0, "Angel of Destiny");
    attack(&mut g, &[angel]);
    through_combat(&mut g);
    // Double strike: 2 and 2; each time both players gain that much.
    assert_eq!(g.players[0].life, 44);
    assert_eq!(g.players[1].life, 40);
    assert!(!g.players[1].lost);
    g.players[0].life = 55; // 15 over the starting 40
    while g.result.is_none() && g.step != Step::Cleanup {
        act(&mut g, Action::PassPriority);
    }
    assert!(g.players[1].lost, "it attacked this turn");
}

#[test]
fn bonus_round_copies_each_instant_or_sorcery_cast_this_turn() {
    let mut g = table();
    let round = hand(&mut g, 0, "Bonus Round");
    play(&mut g, round, false, &[], &[]);
    assert_eq!(g.obj(round).zone, Zone::Graveyard);
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    // Keep the copy's target (the opponent).
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let legal = g.legal_actions();
        let a = match g.pending {
            Pending::ChooseTarget(_) if legal.contains(&Action::Target(Target::Player(1))) => {
                Action::Target(Target::Player(1))
            }
            Pending::ChooseTarget(_) | Pending::Decide(_) if legal.contains(&Action::No) => Action::No,
            Pending::ChooseTarget(_) | Pending::Decide(_) => legal[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[1].life, 34, "the Bolt and its copy");
    // Next turn it's over.
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let bolt2 = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, bolt2, false, &[Target::Player(1)], &[]);
    assert_eq!(g.players[1].life, 31, "until end of turn only");
}

// ---- Ygra, Eater of All ----

fn token_def(g: &Game, name: &str) -> usize {
    g.defs
        .iter()
        .position(|d| d.token && d.name == name)
        .unwrap_or_else(|| panic!("the {name} token"))
}

/// Lightning Bolt from player 0 at `target`, passing until the ward's question or until it leaves
/// the stack.
fn bolt_at(g: &mut Game, target: ObjId) -> ObjId {
    let bolt = hand(g, 0, "Lightning Bolt");
    act(g, Action::Cast(bolt));
    act(g, Action::Target(Target::Object(target)));
    for _ in 0..20 {
        if matches!(g.pending, Pending::Decide(0)) || g.obj(bolt).zone != Zone::Stack {
            break;
        }
        act(g, Action::PassPriority);
    }
    bolt
}

#[test]
fn ygra_ward_takes_a_food_and_the_food_feeds_ygra() {
    let mut g = table();
    let ygra = put(&mut g, 1, "Ygra, Eater of All");
    // Player 0 controls no creature (so no Food): the ward can't be paid.
    let bolt = bolt_at(&mut g, ygra);
    assert_eq!(g.obj(bolt).zone, Zone::Graveyard, "countered");
    assert_eq!(g.obj(ygra).damage, 0);
    // A creature of theirs is a Food: the ward offers it.
    let lions = put(&mut g, 0, "Savannah Lions");
    let bolt = bolt_at(&mut g, ygra);
    assert!(matches!(g.pending, Pending::Decide(0)));
    assert!(has(&g, Action::Pick(lions)));
    act(&mut g, Action::Pick(lions));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard, "sacrificed for the ward");
    assert_eq!(g.obj(bolt).zone, Zone::Graveyard);
    assert_eq!(g.obj(ygra).damage, 3, "the Bolt resolved");
    assert_eq!(g.obj(ygra).counters, 2, "a Food went to the graveyard");
}

#[test]
fn ygra_makes_every_other_creature_a_food_artifact_with_the_food_ability() {
    let mut g = table();
    let ygra = put(&mut g, 0, "Ygra, Eater of All");
    let mine = put(&mut g, 0, "Savannah Lions");
    let theirs = put(&mut g, 1, "Serra Angel");
    for c in [mine, theirs] {
        assert!(g.types_of(c).artifact && g.types_of(c).creature && g.has_subtype(c, "Food"));
    }
    assert!(
        !g.types_of(ygra).artifact && !g.has_subtype(ygra, "Food"),
        "other creatures only"
    );
    assert!(
        has(&g, Action::Activate(mine, 0)),
        "{{2}}, {{T}}, sacrifice: gain 3"
    );
    act(&mut g, Action::Activate(mine, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(mine).zone, Zone::Graveyard);
    assert_eq!(g.players[0].life, 43);
    assert_eq!(g.obj(ygra).counters, 2);
    // Without Ygra they're plain creatures again.
    let d = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, d, false, &[obj(ygra)], &[]);
    assert!(!g.types_of(theirs).artifact && !g.has_subtype(theirs, "Food"));
}

#[test]
fn ygra_grows_on_any_food_to_a_graveyard_but_not_other_artifacts() {
    let mut g = table();
    let ygra = put(&mut g, 0, "Ygra, Eater of All");
    let food = g.add_card(0, token_def(&g, "Food"), Zone::Battlefield);
    act(&mut g, Action::Activate(food, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ygra).counters, 2, "a Food token");
    let clue = g.add_card(0, token_def(&g, "Clue"), Zone::Battlefield);
    act(&mut g, Action::Activate(clue, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ygra).counters, 2, "a Clue isn't a Food");
    let theirs = put(&mut g, 1, "Serra Angel");
    let d = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, d, false, &[obj(theirs)], &[]);
    assert_eq!(g.obj(ygra).counters, 4, "the opponent's creature was a Food");
}

#[test]
fn mesmeric_orb_mills_the_controller_of_each_permanent_that_untaps() {
    let mut g = table();
    put(&mut g, 0, "Mesmeric Orb");
    let mine: Vec<ObjId> = (0..2).map(|_| put(&mut g, 0, "Forest")).collect();
    let theirs = put(&mut g, 1, "Forest");
    for &x in mine.iter().chain([&theirs]) {
        g.objects[x.0 as usize].tapped = true;
    }
    let (gy0, gy1) = (g.players[0].graveyard.len(), g.players[1].graveyard.len());
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[0].graveyard.len(), gy0 + 2, "two of yours untapped");
    assert_eq!(
        g.players[1].graveyard.len(),
        gy1,
        "theirs stays tapped on your turn"
    );
    g.start_turn(6, 1);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[1].graveyard.len(), gy1 + 1, "its controller mills");
}

#[test]
fn hallowed_haunting_makes_spirit_clerics_that_count_spirits_and_flies_at_seven() {
    let mut g = table();
    put(&mut g, 0, "Hallowed Haunting");
    for n in 1..=2 {
        let blood = hand(&mut g, 0, "Exquisite Blood");
        play(&mut g, blood, false, &[], &[]);
        let spirits = tokens(&g, 0, "Spirit Cleric");
        assert_eq!(spirits.len(), n);
        for &s in &spirits {
            assert_eq!(
                (g.power(s), g.toughness(s)),
                (n as i32, n as i32),
                "the Spirits you control"
            );
            assert!(!g.keywords(s).has(Keywords::FLYING), "{n}: three enchantments");
        }
    }
    for _ in 0..4 {
        put(&mut g, 0, "Exquisite Blood"); // seven enchantments
    }
    let lions = put(&mut g, 0, "Savannah Lions");
    for c in [lions, tokens(&g, 0, "Spirit Cleric")[0]] {
        assert!(g.keywords(c).has(Keywords::FLYING) && g.keywords(c).has(Keywords::VIGILANCE));
    }
    let theirs = put(&mut g, 1, "Savannah Lions");
    assert!(!g.keywords(theirs).has(Keywords::FLYING), "creatures you control");
}

/// From player 0's main phase to the end step's Palantír question, answering the target and the
/// scry along the way; returns whether the opponent was asked.
fn palantir_to_the_question(g: &mut Game) -> bool {
    for _ in 0..200 {
        match g.pending {
            Pending::Decide(1) => return true,
            Pending::ChooseTarget(_) => {
                act(g, Action::Target(Target::Player(1)));
            }
            Pending::Decide(_) => {
                let a = g.legal_actions()[0];
                act(g, a);
            }
            _ if g.step == Step::Cleanup || g.turn != 3 => return false,
            _ => {
                act(g, Action::PassPriority);
            }
        }
    }
    false
}

#[test]
fn palantir_of_orthanc_draws_or_mills_by_influence_and_drains() {
    let mut g = table();
    let pal = put(&mut g, 0, "Palant\u{ed}r of Orthanc");
    assert!(palantir_to_the_question(&mut g));
    assert_eq!(g.obj(pal).kind_counters, 1, "an influence counter");
    let hand_before = g.players[0].hand.len();
    act(&mut g, Action::Yes); // they let you draw
    assert_eq!(g.players[0].hand.len(), hand_before + 1);

    let mut g = table();
    let pal = put(&mut g, 0, "Palant\u{ed}r of Orthanc");
    g.objects[pal.0 as usize].kind_counters = 1; // the second end step: two counters
    assert!(palantir_to_the_question(&mut g));
    assert_eq!(g.obj(pal).kind_counters, 2);
    let angels = [
        g.add_card(0, cid("Serra Angel"), Zone::Library),
        g.add_card(0, cid("Serra Angel"), Zone::Library),
    ];
    act(&mut g, Action::No); // they refuse
    for a in angels {
        assert_eq!(g.obj(a).zone, Zone::Graveyard, "milled two");
    }
    assert_eq!(g.players[1].life, 30, "two mana value 5 cards");
}

#[test]
fn panharmonicon_doubles_triggers_from_artifacts_and_creatures_entering() {
    let mut g = table();
    put(&mut g, 0, "Panharmonicon");
    put(&mut g, 0, "Impact Tremors");
    let elves = hand(&mut g, 0, "Llanowar Elves");
    play(&mut g, elves, false, &[], &[]);
    assert_eq!(g.players[1].life, 38, "Impact Tremors twice");
    let hand_before = g.players[0].hand.len();
    let visionary = hand(&mut g, 0, "Elvish Visionary");
    play(&mut g, visionary, false, &[], &[]);
    assert_eq!(
        g.players[0].hand.len(),
        hand_before + 2,
        "its own enter trigger twice"
    );
    assert_eq!(g.players[1].life, 36);
}

#[test]
fn daring_sleuth_transforms_on_a_clue_and_investigates_on_combat_damage() {
    let mut g = table();
    let sleuth = put(&mut g, 0, "Daring Sleuth // Bearer of Overwhelming Truths");
    let clue = g.add_card(0, token_def(&g, "Clue"), Zone::Battlefield);
    act(&mut g, Action::Activate(clue, 0)); // {2}, sacrifice: draw
    resolve_all(&mut g, &[]);
    assert!(g.obj(sleuth).transformed);
    assert_eq!(g.def_of(sleuth).name, "Bearer of Overwhelming Truths");
    attack(&mut g, &[sleuth]);
    through_combat(&mut g);
    assert_eq!(tokens(&g, 0, "Clue").len(), 1, "investigated");
    let power = g.power(sleuth);
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    assert_eq!(g.power(sleuth), power + 1, "prowess");
}

#[test]
fn currency_converter_exiles_discards_and_turns_them_into_treasure_or_rogues() {
    let mut g = table();
    let cc = put(&mut g, 0, "Currency Converter");
    let angel = hand(&mut g, 0, "Serra Angel");
    let forest = hand(&mut g, 0, "Forest");
    // {2}, {T}: draw, then discard; discard the Angel and exile it.
    act(&mut g, Action::Activate(cc, 0));
    let mut answered = false;
    while !g.items.is_empty() || matches!(g.pending, Pending::Decide(_)) {
        let a = match g.pending {
            _ if has(&g, Action::Discard(angel)) => Action::Discard(angel),
            Pending::Decide(_) => {
                answered = true;
                Action::Yes // may exile it
            }
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(answered);
    assert_eq!(g.obj(angel).zone, Zone::Exile, "exiled with it");
    // Discarding the Forest some other way: Exile it too.
    g.objects[cc.0 as usize].tapped = false;
    act(&mut g, Action::Activate(cc, 0));
    while !g.items.is_empty() || matches!(g.pending, Pending::Decide(_)) {
        let a = match g.pending {
            _ if has(&g, Action::Discard(forest)) => Action::Discard(forest),
            Pending::Decide(_) => Action::Yes,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(forest).zone, Zone::Exile);
    // {T}: the Angel back to the graveyard makes a Rogue; the Forest a Treasure.
    g.objects[cc.0 as usize].tapped = false;
    act(&mut g, Action::Activate(cc, 1));
    resolve_all(&mut g, &[Action::Pick(angel)]);
    assert_eq!(g.obj(angel).zone, Zone::Graveyard);
    assert_eq!(tokens(&g, 0, "Rogue").len(), 1);
    let rogue = tokens(&g, 0, "Rogue")[0];
    assert_eq!((g.power(rogue), g.toughness(rogue)), (2, 2));
    g.objects[cc.0 as usize].tapped = false;
    act(&mut g, Action::Activate(cc, 1));
    resolve_all(&mut g, &[Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Graveyard);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1);
}

/// Resolves everything, targeting the opponent and answering yes.
fn settle_yes_at_opponent(g: &mut Game) {
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) => Action::Target(Target::Player(1)),
            Pending::Decide(_) => Action::Yes,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn basking_broodscale_adapts_once_and_all_will_be_one_burns_for_the_counters() {
    let mut g = table();
    put(&mut g, 0, "All Will Be One");
    let lizard = put(&mut g, 0, "Basking Broodscale");
    act(&mut g, Action::Activate(lizard, 0)); // adapt 1
    settle_yes_at_opponent(&mut g);
    assert_eq!(g.obj(lizard).counters, 1);
    assert_eq!(tokens(&g, 0, "Eldrazi Spawn").len(), 1, "a counter was put on it");
    assert_eq!(g.players[1].life, 39, "one counter: 1 damage");
    act(&mut g, Action::Activate(lizard, 0)); // it has a counter: adapt does nothing
    settle_yes_at_opponent(&mut g);
    assert_eq!(g.obj(lizard).counters, 1);
    assert_eq!(tokens(&g, 0, "Eldrazi Spawn").len(), 1);
    assert_eq!(g.players[1].life, 39);
}

#[test]
fn moonstone_exiles_a_discarded_card_to_play_until_your_next_turn() {
    let mut g = table();
    put(&mut g, 0, "Moonstone, Harsh Mistress");
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    loot_away(&mut g, bolt); // Moonstone's may is taken
    assert_eq!(g.obj(bolt).zone, Zone::Exile);
    assert!(has(&g, Action::Cast(bolt)), "playable from exile");
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    assert_eq!(g.players[1].life, 37);
}

#[test]
fn the_ozolith_keeps_a_leaving_creatures_counters_and_moves_them_at_combat() {
    let mut g = table();
    let oz = put(&mut g, 0, "The Ozolith");
    let lizard = put(&mut g, 0, "Basking Broodscale");
    act(&mut g, Action::Activate(lizard, 0)); // adapt 1
    settle_yes_at_opponent(&mut g);
    assert_eq!(g.obj(lizard).counters, 1);
    let d = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, d, false, &[obj(lizard)], &[]);
    assert_eq!(g.obj(oz).counters, 1, "those counters");
    let lions = put(&mut g, 0, "Savannah Lions");
    g.start_turn(5, 0);
    for _ in 0..100 {
        match g.pending {
            Pending::ChooseTarget(_) => {
                act(&mut g, Action::Target(Target::Object(lions)));
            }
            Pending::Decide(_) => {
                act(&mut g, Action::Yes);
            }
            _ if g.obj(oz).counters == 0 && g.items.is_empty() => break,
            _ => {
                act(&mut g, Action::PassPriority);
            }
        }
    }
    assert_eq!(g.obj(lions).counters, 1, "moved onto the Lions");
    assert_eq!(g.obj(oz).counters, 0);
}

#[test]
fn enduring_tenacity_drains_on_life_gain_and_returns_as_an_enchantment() {
    let mut g = table();
    let snake = put(&mut g, 0, "Enduring Tenacity");
    let food = g.add_card(0, token_def(&g, "Food"), Zone::Battlefield);
    act(&mut g, Action::Activate(food, 0));
    settle_yes_at_opponent(&mut g);
    assert_eq!(g.players[1].life, 37, "that much life");
    let d = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, d, false, &[obj(snake)], &[]);
    assert_eq!(g.obj(snake).zone, Zone::Battlefield, "back");
    assert!(!g.types_of(snake).creature && g.types_of(snake).enchantment);
}

#[test]
fn enduring_courage_pumps_and_hastes_each_other_creature_entering() {
    let mut g = table();
    put(&mut g, 0, "Enduring Courage");
    let elves = hand(&mut g, 0, "Llanowar Elves");
    play(&mut g, elves, false, &[], &[]);
    assert_eq!(g.power(elves), 3);
    assert!(g.keywords(elves).has(Keywords::HASTE));
}

#[test]
fn field_of_the_dead_makes_zombies_at_seven_land_names() {
    let mut g = table(); // five basic names
    let field = put(&mut g, 0, "Field of the Dead"); // six
    assert!(tokens(&g, 0, "Zombie").is_empty());
    let tower = hand(&mut g, 0, "Command Tower");
    act(&mut g, Action::PlayLand(tower)); // seven names
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Zombie").len(), 1);
    let _ = field;
}

#[test]
fn mystic_sanctuary_untapped_puts_an_instant_on_top() {
    let mut g = table(); // five Islands
    let bolt = g.add_card(0, cid("Lightning Bolt"), Zone::Graveyard);
    let sanctuary = hand(&mut g, 0, "Mystic Sanctuary");
    act(&mut g, Action::PlayLand(sanctuary));
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) => Action::Target(Target::Object(bolt)),
            Pending::Decide(_) => Action::Yes,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(!g.obj(sanctuary).tapped);
    assert_eq!(g.players[0].library.last(), Some(&bolt), "on top");
}

#[test]
fn bontu_attacks_only_after_a_creature_of_yours_died_and_drains_on_a_sacrifice() {
    let mut g = table();
    let bontu = put(&mut g, 0, "Bontu the Glorified");
    put(&mut g, 0, "Savannah Lions");
    let elves = put(&mut g, 0, "Llanowar Elves");
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    assert!(!has(&g, Action::Attack(bontu)), "nothing died");
    let mut g = table();
    let bontu = put(&mut g, 0, "Bontu the Glorified");
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::ActivateWith(bontu, 0, lions));
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = if matches!(g.pending, Pending::Decide(_) | Pending::ChooseTarget(_)) {
            g.legal_actions()[0]
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
    assert_eq!((g.players[0].life, g.players[1].life), (41, 39));
    while g.pending != Pending::DeclareAttackers(0) {
        let a = match g.pending {
            Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(
        has(&g, Action::Attack(bontu)),
        "a creature died under your control this turn"
    );
    let _ = elves;
}

#[test]
fn mana_drain_counters_and_adds_colorless_at_your_next_main_phase() {
    let mut g = table();
    let angel = hand(&mut g, 0, "Serra Angel");
    act(&mut g, Action::Cast(angel));
    let drain = hand(&mut g, 0, "Mana Drain");
    act(&mut g, Action::Cast(drain));
    act(&mut g, Action::Target(Target::Object(angel)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(angel).zone, Zone::Graveyard, "countered");
    while g.step != Step::Main2 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(
        g.players[0].pool.iter().filter(|m| m.colorless).count(),
        5,
        "its mana value in {{C}}"
    );
}

#[test]
fn beza_catches_up_on_each_count_an_opponent_leads() {
    let mut g = table();
    put(&mut g, 1, "Forest"); // more lands
    g.players[1].life = 50; // more life
    put(&mut g, 1, "Savannah Lions");
    put(&mut g, 1, "Llanowar Elves"); // two creatures to Beza's one
    for _ in 0..4 {
        hand(&mut g, 1, "Plains"); // more cards in hand
    }
    let beza = hand(&mut g, 0, "Beza, the Bounding Spring");
    let before = g.players[0].hand.len();
    play(&mut g, beza, false, &[], &[]);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1);
    assert_eq!(g.players[0].life, 44);
    assert_eq!(tokens(&g, 0, "Fish").len(), 2);
    assert_eq!(g.players[0].hand.len(), before - 1 + 1, "cast Beza, drew one");

    let mut g = table(); // level on everything
    let beza = hand(&mut g, 0, "Beza, the Bounding Spring");
    for _ in 0..8 {
        hand(&mut g, 0, "Plains");
    }
    play(&mut g, beza, false, &[], &[]);
    assert!(tokens(&g, 0, "Treasure").is_empty() && tokens(&g, 0, "Fish").is_empty());
    assert_eq!(g.players[0].life, 40);
}

#[test]
fn tandem_lookout_pairs_and_each_draws_on_damage_to_an_opponent() {
    let mut g = table();
    let lions = put(&mut g, 0, "Savannah Lions");
    let lookout = hand(&mut g, 0, "Tandem Lookout");
    act(&mut g, Action::Cast(lookout));
    while !g.items.is_empty() || matches!(g.pending, Pending::Decide(_)) {
        let a = if has(&g, Action::Pick(lions)) {
            Action::Pick(lions)
        } else if matches!(g.pending, Pending::Decide(_)) {
            Action::Yes
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
    }
    assert_eq!(g.paired_partner(lookout), Some(lions));
    attack(&mut g, &[lions]);
    let before = g.players[0].hand.len();
    through_combat(&mut g);
    assert_eq!(g.players[1].life, 38);
    assert_eq!(
        g.players[0].hand.len(),
        before + 1,
        "the paired Lions drew a card"
    );
}

#[test]
fn rose_room_treasurer_makes_two_treasures_then_pays_x_for_damage() {
    let mut g = table();
    put(&mut g, 0, "Rose Room Treasurer");
    for n in 1..=3 {
        let elves = hand(&mut g, 0, "Llanowar Elves");
        act(&mut g, Action::Cast(elves));
        while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
            let a = match g.pending {
                Pending::ChooseTarget(_) => Action::Target(Target::Player(1)),
                Pending::Decide(_) => Action::Amount(2),
                _ => Action::PassPriority,
            };
            act(&mut g, a);
        }
        assert_eq!(
            tokens(&g, 0, "Treasure").len(),
            n.min(2),
            "the first and second time"
        );
    }
    assert_eq!(g.players[1].life, 38, "the third time: pay {{2}}, 2 damage");
}

#[test]
fn sabotender_and_tannuk_burn_on_landfall_and_tannuk_draws_the_second_time() {
    let mut g = table();
    put(&mut g, 0, "Sabotender");
    put(&mut g, 0, "Tannuk, Memorial Ensign");
    put(&mut g, 0, "Composer of Spring");
    let forest = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(forest));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 38, "1 each");
    let island = hand(&mut g, 0, "Island");
    let blood = hand(&mut g, 0, "Exquisite Blood");
    let before = g.players[0].hand.len();
    play(&mut g, blood, false, &[], &[Action::Pick(island)]); // Composer puts the Island in
    assert_eq!(g.obj(island).zone, Zone::Battlefield);
    assert_eq!(g.players[1].life, 36);
    assert_eq!(
        g.players[0].hand.len(),
        before - 2 + 1,
        "Tannuk's second resolution draws"
    );
}

#[test]
fn seasoned_dungeoneer_takes_the_initiative_and_protects_an_attacking_warrior() {
    let mut g = table();
    let dungeoneer = hand(&mut g, 0, "Seasoned Dungeoneer");
    act(&mut g, Action::Cast(dungeoneer));
    // The initiative ventures into Undercity: answer its room choices.
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) | Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.initiative.map(|(p, _)| p), Some(0), "you take the initiative");
    let angel = put(&mut g, 1, "Serra Angel");
    let lions = put(&mut g, 0, "Savannah Lions"); // a Cat Soldier: not a party member
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        match g.pending {
            Pending::Decide(_) | Pending::ChooseTarget(_) => {
                let a = g.legal_actions()[0];
                act(&mut g, a);
            }
            _ => {
                act(&mut g, Action::PassPriority);
            }
        }
    }
    act(&mut g, Action::Attack(dungeoneer));
    act(&mut g, Action::Attack(lions));
    act(&mut g, Action::FinishAttacks);
    assert!(
        !has(&g, Action::Target(Target::Object(lions))),
        "a Cleric, Rogue, Warrior or Wizard only"
    );
    while !matches!(g.pending, Pending::DeclareBlockers(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) => Action::Target(Target::Object(dungeoneer)),
            Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(
        !has(
            &g,
            Action::Block {
                blocker: angel,
                attacker: dungeoneer
            }
        ),
        "protection from creatures"
    );
    assert!(has(
        &g,
        Action::Block {
            blocker: angel,
            attacker: lions
        }
    ));
}

// ---- Core: triggers, batch 30 (idea decks: Sephiroth, Tifa) ----

/// Passes (answering targets and decisions with the first legal choice) until `p`'s turn `turn`
/// reaches `step` with an empty stack.
fn to_step(g: &mut Game, turn: u32, p: usize, step: Step) {
    g.start_turn(turn, p);
    loop {
        if g.step == step
            && g.items.is_empty()
            && !matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_))
        {
            return;
        }
        let a = match g.pending {
            Pending::ChooseTarget(_) | Pending::Decide(_) => g.legal_actions()[0],
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

/// Passes until the stack is empty, answering every question with its first legal choice.
fn drain(g: &mut Game) {
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) | Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn greedy_freebooter_dies_into_a_scry_and_a_treasure() {
    let mut g = table();
    let f = put(&mut g, 0, "Greedy Freebooter");
    let kill = hand(&mut g, 0, "Hero's Downfall");
    act(&mut g, Action::Cast(kill));
    act(&mut g, Action::Target(Target::Object(f)));
    drain(&mut g);
    assert!(in_graveyard(&g, 0, f));
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1);
}

#[test]
fn bitterblossom_costs_a_life_for_a_flying_faerie_rogue_each_upkeep() {
    let mut g = table();
    put(&mut g, 0, "Bitterblossom");
    to_step(&mut g, 5, 0, Step::Main1);
    assert_eq!(g.players[0].life, 39);
    let rogue = tokens(&g, 0, "Faerie Rogue");
    assert_eq!(rogue.len(), 1);
    assert!(g.keywords(rogue[0]).has(Keywords::FLYING));
    assert_eq!((g.power(rogue[0]), g.toughness(rogue[0])), (1, 1));
    assert!(g.has_subtype(rogue[0], "Faerie") && g.has_subtype(rogue[0], "Rogue"));
    to_step(&mut g, 6, 1, Step::Main1);
    assert_eq!(g.players[0].life, 39, "your upkeep only");
}

#[test]
fn jadar_makes_a_decayed_zombie_only_while_you_control_none() {
    let mut g = table();
    put(&mut g, 0, "Jadar, Ghoulcaller of Nephalia");
    to_step(&mut g, 5, 0, Step::End);
    let z = tokens(&g, 0, "Zombie");
    assert_eq!(z.len(), 1);
    assert!(g.def_of(z[0]).decayed);
    assert_eq!((g.power(z[0]), g.toughness(z[0])), (2, 2));
    to_step(&mut g, 7, 0, Step::End);
    assert_eq!(
        tokens(&g, 0, "Zombie").len(),
        1,
        "you already control a creature with decayed"
    );
}

#[test]
fn ophiomancer_makes_a_deathtouch_snake_each_upkeep_if_you_have_none() {
    let mut g = table();
    put(&mut g, 0, "Ophiomancer");
    to_step(&mut g, 4, 1, Step::Main1); // the opponent's upkeep counts
    let s = tokens(&g, 0, "Snake");
    assert_eq!(s.len(), 1);
    assert!(g.keywords(s[0]).has(Keywords::DEATHTOUCH));
    to_step(&mut g, 5, 0, Step::Main1);
    assert_eq!(tokens(&g, 0, "Snake").len(), 1, "you control a Snake");
}

#[test]
fn forsaken_miner_returns_when_you_commit_a_crime() {
    let mut g = table();
    let miner = g.add_card(0, cid("Forsaken Miner"), Zone::Graveyard);
    // Targeting your own creature is no crime.
    let mine = put(&mut g, 0, "Savannah Lions");
    let kill = hand(&mut g, 0, "Hero's Downfall");
    act(&mut g, Action::Cast(kill));
    act(&mut g, Action::Target(Target::Object(mine)));
    drain(&mut g);
    assert_eq!(g.obj(miner).zone, Zone::Graveyard);
    // Targeting the opponent's creature is: pay {B} to return it.
    let theirs = put(&mut g, 1, "Savannah Lions");
    let kill = hand(&mut g, 0, "Hero's Downfall");
    act(&mut g, Action::Cast(kill));
    act(&mut g, Action::Target(Target::Object(theirs)));
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::Decide(_) => Action::Yes,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(miner).zone, Zone::Battlefield);
    assert!(!g.obj(miner).tapped);
    assert!(in_graveyard(&g, 1, theirs));
}

#[test]
fn tifa_lockhart_doubles_her_power_on_landfall() {
    let mut g = table();
    let tifa = put(&mut g, 0, "Tifa Lockhart");
    let forest = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(forest));
    drain(&mut g);
    assert_eq!((g.power(tifa), g.toughness(tifa)), (2, 2));
}

#[test]
fn bristly_bill_counts_on_landfall_and_doubles_counters() {
    let mut g = table();
    let bill = put(&mut g, 0, "Bristly Bill, Spine Sower");
    let lions = put(&mut g, 0, "Savannah Lions");
    let forest = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(forest));
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) => Action::Target(Target::Object(lions)),
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(lions).counters, 1);
    act(&mut g, Action::Activate(bill, 0));
    drain(&mut g);
    assert_eq!(g.obj(lions).counters, 2);
    assert_eq!(g.power(lions), 4);
}

#[test]
fn mightform_harmonizer_doubles_a_creatures_power_on_landfall() {
    let mut g = table();
    put(&mut g, 0, "Mightform Harmonizer");
    let lions = put(&mut g, 0, "Savannah Lions");
    let forest = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(forest));
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) => Action::Target(Target::Object(lions)),
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!((g.power(lions), g.toughness(lions)), (4, 1));
    assert!(g.def_of(cid_obj(&g, "Mightform Harmonizer")).warp.is_some());
}

fn cid_obj(g: &Game, name: &str) -> ObjId {
    g.permanents(0)
        .find(|&x| g.def_of(x).name == name)
        .expect("on the battlefield")
}

#[test]
fn skyclave_pick_axe_attaches_as_it_enters_and_pumps_on_landfall() {
    let mut g = table();
    let lions = put(&mut g, 0, "Savannah Lions");
    let axe = hand(&mut g, 0, "Skyclave Pick-Axe");
    act(&mut g, Action::Cast(axe));
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) => Action::Target(Target::Object(lions)),
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(axe).attached_to.map(|a| a.0), Some(lions));
    let forest = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(forest));
    drain(&mut g);
    assert_eq!((g.power(lions), g.toughness(lions)), (4, 3));
}

#[test]
fn staff_of_titania_pumps_per_forest_and_makes_a_dryad_on_attack() {
    let mut g = table();
    let staff = put(&mut g, 0, "Staff of Titania");
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::Activate(staff, 0));
    act(&mut g, Action::Target(Target::Object(lions)));
    drain(&mut g);
    assert_eq!((g.power(lions), g.toughness(lions)), (7, 6), "five Forests");
    attack(&mut g, &[lions]);
    drain(&mut g);
    let dryads = tokens(&g, 0, "Forest Dryad");
    assert_eq!(dryads.len(), 1);
    assert_eq!(g.power(lions), 8, "the Dryad is a Forest too");
}

// ---- Core: triggers, batch 31 (Bello, Ghyrson, Tifa, Sephiroth) ----

/// Passes until the stack is empty, answering targets with the opponent (else the first legal
/// choice) and decisions with `Yes` if offered (else the first legal choice).
fn drain_yes(g: &mut Game) {
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let legal = g.legal_actions();
        let a = match g.pending {
            Pending::ChooseTarget(_) if legal.contains(&Action::Target(Target::Player(1))) => {
                Action::Target(Target::Player(1))
            }
            Pending::Decide(_) if legal.contains(&Action::Yes) => Action::Yes,
            Pending::ChooseTarget(_) | Pending::Decide(_) => legal[0],
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn grothama_lets_attackers_fight_it_and_draws_for_the_damage_as_it_leaves() {
    let mut g = table();
    let grothama = put(&mut g, 0, "Grothama, All-Devouring");
    let lions = put(&mut g, 0, "Savannah Lions");
    attack(&mut g, &[lions]);
    drain_yes(&mut g); // you may have it fight Grothama: yes
    assert!(in_graveyard(&g, 0, lions), "10 damage back");
    assert_eq!(g.obj(grothama).damage, 2);
    for _ in 0..3 {
        put(&mut g, 0, "Swamp");
    }
    let kill = hand(&mut g, 0, "Hero's Downfall");
    let (mine, theirs) = (g.players[0].hand.len(), g.players[1].hand.len());
    act(&mut g, Action::Cast(kill));
    act(&mut g, Action::Target(Target::Object(grothama)));
    drain_yes(&mut g);
    assert!(in_graveyard(&g, 0, grothama));
    assert_eq!(g.players[0].hand.len(), mine - 1 + 2, "your sources dealt it 2");
    assert_eq!(g.players[1].hand.len(), theirs, "theirs dealt it none");
}

#[test]
fn ral_storm_conduit_pings_on_casts_and_copies_and_copies_the_next_instant() {
    let mut g = table();
    let ral = put(&mut g, 0, "Ral, Storm Conduit");
    g.objects[ral.0 as usize].loyalty = 4; // put onto the battlefield without its counters
    let loyalty = g.obj(ral).loyalty;
    act(&mut g, Action::Loyalty(ral, 1)); // -2
    drain_yes(&mut g);
    assert_eq!(g.obj(ral).loyalty, loyalty - 2);
    let elves = hand(&mut g, 0, "Llanowar Elves");
    play(&mut g, elves, false, &[], &[]); // a creature spell doesn't use up the copy
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    drain_yes(&mut g);
    assert_eq!(
        g.players[1].life,
        40 - 3 - 3 - 1 - 1,
        "the bolt, its copy and two pings"
    );
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    drain_yes(&mut g);
    assert_eq!(g.players[1].life, 32 - 3 - 1, "only the next one is copied");
}

#[test]
fn scythecat_cub_counts_then_doubles_on_the_second_landfall() {
    let mut g = table();
    put(&mut g, 0, "Scythecat Cub");
    put(&mut g, 0, "Composer of Spring");
    let lions = put(&mut g, 0, "Savannah Lions");
    g.objects[lions.0 as usize].counters = 2;
    let forest = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(forest));
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) => Action::Target(Target::Object(lions)),
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(lions).counters, 3, "the first time: a counter");
    let island = hand(&mut g, 0, "Island");
    let blood = hand(&mut g, 0, "Exquisite Blood");
    act(&mut g, Action::Cast(blood));
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) => Action::Target(Target::Object(lions)),
            Pending::Decide(_) => Action::Pick(island), // Composer puts the Island in
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(island).zone, Zone::Battlefield);
    assert_eq!(g.obj(lions).counters, 6, "the second time: doubled instead");
}

#[test]
fn claim_the_kingdom_sacrifices_on_the_fourth_plan_counter_for_an_indestructible_counter() {
    let mut g = table();
    let claim = put(&mut g, 0, "Claim the Kingdom");
    let lions = put(&mut g, 0, "Savannah Lions");
    g.objects[claim.0 as usize].kind_counters = 3;
    let forest = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(forest));
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) => Action::Target(Target::Object(lions)),
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(lions).counters, 1);
    assert!(in_graveyard(&g, 0, claim), "the fourth plan counter");
    assert!(g.keywords(lions).has(Keywords::INDESTRUCTIBLE));
}

#[test]
fn sephiroth_drains_on_deaths_and_transforms_the_fourth_time_with_an_emblem() {
    let mut g = table();
    let seph = put(
        &mut g,
        0,
        "Sephiroth, Fabled SOLDIER // Sephiroth, One-Winged Angel",
    );
    for _ in 0..12 {
        put(&mut g, 0, "Swamp");
    }
    for n in 1..=5 {
        let victim = put(&mut g, 1, "Savannah Lions");
        let kill = hand(&mut g, 0, "Hero's Downfall");
        act(&mut g, Action::Cast(kill));
        act(&mut g, Action::Target(Target::Object(victim)));
        drain_yes(&mut g);
        assert_eq!(g.players[1].life, 40 - n, "death {n}");
        assert_eq!(g.players[0].life, 40 + n);
        assert_eq!(g.obj(seph).transformed, n >= 4, "transformed on the fourth");
    }
    assert_eq!(g.emblems.len(), 1, "Super Nova's emblem drains from the fifth on");
    assert_eq!(g.def_of(seph).name, "Sephiroth, One-Winged Angel");
}

#[test]
fn sephiroth_one_winged_angel_sacrifices_any_number_on_attack_to_draw() {
    let mut g = table();
    let seph = put(
        &mut g,
        0,
        "Sephiroth, Fabled SOLDIER // Sephiroth, One-Winged Angel",
    );
    g.objects[seph.0 as usize].transformed = true;
    let a = put(&mut g, 0, "Savannah Lions");
    let b = put(&mut g, 0, "Llanowar Elves");
    attack(&mut g, &[seph]);
    let before = g.players[0].hand.len();
    let mut sacrificed = 0;
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let legal = g.legal_actions();
        let act_now = match g.pending {
            Pending::Decide(_) if sacrificed < 2 && legal.contains(&Action::Pick(a)) => {
                sacrificed += 1;
                Action::Pick(a)
            }
            Pending::Decide(_) if sacrificed < 2 && legal.contains(&Action::Pick(b)) => {
                sacrificed += 1;
                Action::Pick(b)
            }
            Pending::Decide(_) if legal.contains(&Action::Yes) => Action::Yes,
            Pending::ChooseTarget(_) | Pending::Decide(_) => legal[0],
            _ => Action::PassPriority,
        };
        act(&mut g, act_now);
    }
    assert!(in_graveyard(&g, 0, a) && in_graveyard(&g, 0, b));
    assert_eq!(g.players[0].hand.len(), before + 2, "two sacrificed, two cards");
    // Their deaths drain only with the emblem (not made here: it was transformed by hand).
    assert_eq!(g.players[1].life, 40);
}

// ---- Core: triggers, batch 32 (Atla Palani; tokens 116-118) ----

#[test]
fn atla_palani_makes_eggs_that_hatch_into_the_next_creature_in_your_library() {
    let mut g = table();
    let atla = put(&mut g, 0, "Atla Palani, Nest Tender");
    let lions = library_top(&mut g, "Savannah Lions");
    let forest = library_top(&mut g, "Forest"); // revealed first, then bottom
    act(&mut g, Action::Activate(atla, 0));
    drain(&mut g);
    let eggs = tokens(&g, 0, "Egg");
    assert_eq!(eggs.len(), 1);
    assert!(g.keywords(eggs[0]).has(Keywords::DEFENDER));
    assert_eq!((g.power(eggs[0]), g.toughness(eggs[0])), (0, 1));
    for _ in 0..3 {
        put(&mut g, 0, "Swamp");
    }
    let kill = hand(&mut g, 0, "Hero's Downfall");
    act(&mut g, Action::Cast(kill));
    act(&mut g, Action::Target(Target::Object(eggs[0])));
    drain(&mut g);
    assert_eq!(
        g.obj(lions).zone,
        Zone::Battlefield,
        "the first creature card revealed"
    );
    assert_eq!(
        g.players[0].library.first(),
        Some(&forest),
        "the rest on the bottom"
    );
}

fn library_top(g: &mut Game, name: &str) -> ObjId {
    g.add_card(0, cid(name), Zone::Library)
}

#[test]
fn palanis_hatcher_makes_eggs_and_hatches_one_into_a_hasty_dinosaur_each_combat() {
    let mut g = table();
    let hatcher = hand(&mut g, 0, "Palani's Hatcher");
    play(&mut g, hatcher, false, &[], &[]);
    assert_eq!(tokens(&g, 0, "Dinosaur Egg").len(), 2);
    let eggs = tokens(&g, 0, "Dinosaur Egg");
    g.start_turn(5, 0);
    let mut asked = false;
    while !(g.step == Step::BeginCombat && g.items.is_empty() && asked) {
        let a = match g.pending {
            Pending::Decide(_) => {
                let legal = g.legal_actions();
                assert!(legal.contains(&Action::Pick(eggs[0])) && legal.contains(&Action::Pick(eggs[1])));
                asked = true;
                Action::Pick(eggs[1]) // you choose which Egg
            }
            Pending::ChooseTarget(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(
        tokens(&g, 0, "Dinosaur Egg"),
        vec![eggs[0]],
        "the chosen Egg sacrificed"
    );
    let dino = tokens(&g, 0, "Dinosaur");
    assert_eq!(dino.len(), 1);
    assert_eq!((g.power(dino[0]), g.toughness(dino[0])), (3, 3));
    assert!(
        g.keywords(dino[0]).has(Keywords::HASTE),
        "other Dinosaurs have haste"
    );
    assert!(!g.keywords(hatcher).has(Keywords::HASTE), "other Dinosaurs only");
}

#[test]
fn moonstone_eulogist_bleeds_on_their_deaths_and_grows_on_artifact_sacrifices() {
    let mut g = table();
    let bat = put(&mut g, 0, "Moonstone Eulogist");
    let theirs = put(&mut g, 1, "Savannah Lions");
    for _ in 0..3 {
        put(&mut g, 0, "Swamp");
    }
    let kill = hand(&mut g, 0, "Hero's Downfall");
    act(&mut g, Action::Cast(kill));
    act(&mut g, Action::Target(Target::Object(theirs)));
    drain(&mut g);
    let blood = tokens(&g, 0, "Blood");
    assert_eq!(blood.len(), 1);
    let spare = hand(&mut g, 0, "Forest");
    act(&mut g, Action::ActivateWith(blood[0], 0, spare)); // discard the Forest
    drain(&mut g);
    assert_eq!(g.obj(bat).counters, 1, "the Blood was a sacrificed artifact");
    assert_eq!(g.players[0].life, 41);
}

#[test]
fn simic_ascendancy_grows_with_counters_and_wins_at_twenty() {
    let mut g = table();
    let asc = put(&mut g, 0, "Simic Ascendancy");
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::Activate(asc, 0));
    act(&mut g, Action::Target(Target::Object(lions)));
    drain(&mut g);
    assert_eq!(g.obj(lions).counters, 1);
    assert_eq!(g.obj(asc).kind_counters, 1, "that many growth counters");
    to_step(&mut g, 5, 0, Step::Main1);
    assert!(!g.is_over(), "one growth counter");
    g.objects[asc.0 as usize].kind_counters = 20;
    g.start_turn(7, 0);
    while !g.is_over() && g.step != Step::Main1 {
        let a = match g.pending {
            Pending::ChooseTarget(_) | Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(g.is_over(), "twenty growth counters at your upkeep");
}

#[test]
fn simic_ascendancy_counts_counters_a_creature_enters_with() {
    let mut g = table();
    let asc = put(&mut g, 0, "Simic Ascendancy");
    let h = hand(&mut g, 0, "Goldvein Hydra");
    act(&mut g, Action::CastX(h, 3));
    drain(&mut g);
    assert_eq!(g.obj(h).counters, 3);
    assert_eq!(g.obj(asc).kind_counters, 3, "entering with three +1/+1 counters");
}

#[test]
fn kinbinding_pumps_per_creature_entered_this_turn_and_makes_a_kithkin_each_combat() {
    let mut g = table();
    put(&mut g, 0, "Kinbinding");
    let lions = put(&mut g, 0, "Savannah Lions");
    to_step(&mut g, 5, 0, Step::BeginCombat);
    let kithkin = tokens(&g, 0, "Kithkin");
    assert_eq!(kithkin.len(), 1);
    assert_eq!(
        g.colors_of(kithkin[0]),
        [true, false, false, false, true],
        "green and white"
    );
    assert_eq!(
        (g.power(lions), g.toughness(lions)),
        (3, 2),
        "the Kithkin entered this turn"
    );
    assert_eq!((g.power(kithkin[0]), g.toughness(kithkin[0])), (2, 2));
}

// ---- Batch 34 (idea decks: Kenrith, Sephiroth, Tifa) ----

#[test]
fn kenrith_has_five_colored_abilities() {
    let mut g = table();
    let k = put(&mut g, 0, "Kenrith, the Returned King");
    let lions = put(&mut g, 0, "Savannah Lions");
    let theirs = put(&mut g, 1, "Savannah Lions");
    act(&mut g, Action::Activate(k, 0));
    drain(&mut g);
    for c in [lions, theirs] {
        assert!(
            g.keywords(c).has(Keywords::TRAMPLE) && g.keywords(c).has(Keywords::HASTE),
            "all creatures"
        );
    }
    act(&mut g, Action::Activate(k, 1));
    act(&mut g, Action::Target(Target::Object(lions)));
    drain(&mut g);
    assert_eq!(g.obj(lions).counters, 1);
    act(&mut g, Action::Activate(k, 2));
    act(&mut g, Action::Target(Target::Player(0)));
    drain(&mut g);
    assert_eq!(g.players[0].life, 45);
    let before = g.players[1].hand.len();
    act(&mut g, Action::Activate(k, 3));
    act(&mut g, Action::Target(Target::Player(1)));
    drain(&mut g);
    assert_eq!(g.players[1].hand.len(), before + 1);
    let dead = g.add_card(1, cid("Serra Angel"), Zone::Graveyard);
    act(&mut g, Action::Activate(k, 4));
    act(&mut g, Action::Target(Target::Object(dead)));
    drain(&mut g);
    assert_eq!(g.obj(dead).zone, Zone::Battlefield);
    assert_eq!(g.obj(dead).controller, 1, "under its owner's control");
}

#[test]
fn imperial_seal_puts_any_card_on_top_for_two_life() {
    let mut g = table();
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    g.players[0].library.rotate_right(1); // not on top
    let seal = hand(&mut g, 0, "Imperial Seal");
    play(&mut g, seal, false, &[], &[Action::Pick(ring)]);
    assert_eq!(g.players[0].library.last(), Some(&ring));
    assert_eq!(g.players[0].life, 38);
}

#[test]
fn gamble_tutors_then_discards_at_random() {
    let mut g = table();
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    let gamble = hand(&mut g, 0, "Gamble");
    let hand_before = g.players[0].hand.len(); // Gamble included
    play(&mut g, gamble, false, &[], &[Action::Pick(ring)]);
    assert_eq!(
        g.players[0].hand.len(),
        hand_before - 1 + 1 - 1,
        "cast, tutor, one at random"
    );
    assert_eq!(g.players[0].graveyard.len(), 2, "Gamble and the discarded card");
}

#[test]
fn wheel_of_fortune_discards_both_hands_and_draws_seven_each() {
    let mut g = table();
    let wheel = hand(&mut g, 0, "Wheel of Fortune");
    hand(&mut g, 1, "Sol Ring");
    let mine = g.players[0].hand.len() - 1;
    let theirs = g.players[1].hand.len();
    play(&mut g, wheel, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), 7);
    assert_eq!(g.players[1].hand.len(), 7);
    assert_eq!(g.players[0].graveyard.len(), mine + 1);
    assert_eq!(g.players[1].graveyard.len(), theirs);
}

#[test]
fn summoners_pact_finds_a_green_creature_and_must_be_paid_next_upkeep() {
    let mut g = table();
    let elves = g.add_card(0, cid("Llanowar Elves"), Zone::Library);
    let pact = hand(&mut g, 0, "Summoner's Pact");
    play(&mut g, pact, false, &[], &[Action::Pick(elves)]);
    assert_eq!(g.obj(elves).zone, Zone::Hand);
    g.start_turn(5, 0);
    let mut asked = false;
    while g.step != Step::Main1 {
        let legal = g.legal_actions();
        let a = match g.pending {
            Pending::Decide(_) if legal.contains(&Action::Yes) => {
                asked = true;
                Action::Yes // pay {2}{G}{G}
            }
            Pending::ChooseTarget(_) | Pending::Decide(_) => legal[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(asked, "asked to pay at the next upkeep");
    assert!(!g.is_over(), "paid");
}

#[test]
fn overprotect_tyvars_stand_and_tamiyos_safekeeping_protect() {
    let mut g = table();
    let lions = put(&mut g, 0, "Savannah Lions");
    let o = hand(&mut g, 0, "Overprotect");
    play(&mut g, o, false, &[obj(lions)], &[]);
    assert_eq!((g.power(lions), g.toughness(lions)), (5, 4));
    let k = g.keywords(lions);
    assert!(k.has(Keywords::TRAMPLE) && k.has(Keywords::HEXPROOF) && k.has(Keywords::INDESTRUCTIBLE));
    let elves = put(&mut g, 0, "Llanowar Elves");
    let stand = hand(&mut g, 0, "Tyvar's Stand");
    act(&mut g, Action::CastX(stand, 3));
    act(&mut g, Action::Target(Target::Object(elves)));
    drain(&mut g);
    assert_eq!((g.power(elves), g.toughness(elves)), (4, 4));
    assert!(g.keywords(elves).has(Keywords::INDESTRUCTIBLE));
    let ring = put(&mut g, 0, "Sol Ring");
    let safe = hand(&mut g, 0, "Tamiyo's Safekeeping");
    play(&mut g, safe, false, &[obj(ring)], &[]);
    assert!(
        g.keywords(ring).has(Keywords::HEXPROOF),
        "any permanent you control"
    );
    assert_eq!(g.players[0].life, 42);
}

// ---- Batch 35 (idea decks: Kenrith, Tifa, Sephiroth) ----

#[test]
fn thassas_oracle_keeps_one_on_top_and_wins_with_a_small_library() {
    let mut g = table();
    put(&mut g, 0, "Thassa's Oracle"); // devotion to blue 2 with the new one
    let keep = g.add_card(0, cid("Sol Ring"), Zone::Library);
    let oracle = hand(&mut g, 0, "Thassa's Oracle");
    play(&mut g, oracle, false, &[], &[Action::Pick(keep)]);
    assert_eq!(g.players[0].library.last(), Some(&keep), "up to one on top");
    assert!(!g.is_over(), "a big library");
    // Down to three cards with devotion four: you win.
    let lib = g.players[0].library.clone();
    for &c in &lib[..lib.len() - 3] {
        g.objects[c.0 as usize].zone = Zone::Exile;
        g.exile.push(c);
    }
    g.players[0].library.drain(..lib.len() - 3);
    let third = hand(&mut g, 0, "Thassa's Oracle");
    act(&mut g, Action::Cast(third));
    while !g.is_over() {
        let a = match g.pending {
            Pending::Decide(_) | Pending::ChooseTarget(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(g.players[1].lost, "X (6) >= three cards");
}

#[test]
fn springbloom_druid_may_trade_a_land_for_two_tapped_basics() {
    let mut g = table();
    let forest = put(&mut g, 0, "Forest");
    let druid = hand(&mut g, 0, "Springbloom Druid");
    let lands = g.permanents(0).filter(|&x| g.types_of(x).land).count();
    act(&mut g, Action::Cast(druid));
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let legal = g.legal_actions();
        let a = match g.pending {
            Pending::Decide(_) if legal.contains(&Action::Pick(forest)) => Action::Pick(forest),
            Pending::Decide(_) | Pending::ChooseTarget(_) => legal[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(in_graveyard(&g, 0, forest));
    assert_eq!(
        g.permanents(0).filter(|&x| g.types_of(x).land).count(),
        lands - 1 + 2
    );
}

#[test]
fn demonic_consultation_exiles_six_then_digs_to_the_named_card() {
    let mut g = table();
    let target = g.add_card(0, cid("Sol Ring"), Zone::Library);
    {
        // Tenth from the top: nine cards above it.
        let lib = &mut g.players[0].library;
        let c = lib.pop().expect("the card");
        let n = lib.len();
        lib.insert(n - 9, c);
    }
    let dc = hand(&mut g, 0, "Demonic Consultation");
    let before = g.players[0].library.len();
    play(&mut g, dc, false, &[], &[Action::Pick(target)]);
    assert_eq!(g.obj(target).zone, Zone::Hand);
    assert_eq!(
        g.players[0].library.len(),
        before - 10,
        "six, then the three revealed before it, then it"
    );
    // Naming a card that isn't there exiles the whole library.
    let dc = hand(&mut g, 0, "Demonic Consultation");
    play(&mut g, dc, false, &[], &[Action::No]);
    assert!(g.players[0].library.is_empty());
}

#[test]
fn ravenous_squirrel_grows_on_artifact_or_creature_sacrifices() {
    let mut g = table();
    let sq = put(&mut g, 0, "Ravenous Squirrel");
    let ring = put(&mut g, 0, "Sol Ring");
    let hand_before = g.players[0].hand.len();
    act(&mut g, Action::ActivateWith(sq, 0, ring));
    drain(&mut g);
    assert!(in_graveyard(&g, 0, ring));
    assert_eq!(g.obj(sq).counters, 1, "an artifact sacrificed");
    assert_eq!(g.players[0].life, 41);
    assert_eq!(g.players[0].hand.len(), hand_before + 1);
}

#[test]
fn delney_doubles_small_creatures_triggers() {
    let mut g = table();
    put(&mut g, 0, "Delney, Streetwise Lookout");
    put(&mut g, 0, "Greedy Freebooter"); // a 1/1 with a dies trigger
    let f = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Greedy Freebooter")
        .expect("it");
    let kill = hand(&mut g, 0, "Hero's Downfall");
    act(&mut g, Action::Cast(kill));
    act(&mut g, Action::Target(Target::Object(f)));
    drain(&mut g);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 2, "its dies trigger twice");
}

#[test]
fn horizon_explorer_makes_a_lander_when_you_attack_and_lands_enter_untapped() {
    let mut g = table();
    put(&mut g, 0, "Horizon Explorer");
    let tapland = hand(&mut g, 0, "Savage Lands"); // "This land enters tapped."
    act(&mut g, Action::PlayLand(tapland));
    drain(&mut g);
    assert!(!g.obj(tapland).tapped, "lands you control enter untapped");
    let lions = put(&mut g, 0, "Savannah Lions");
    attack(&mut g, &[lions]);
    drain(&mut g);
    let landers = tokens(&g, 0, "Lander");
    assert_eq!(landers.len(), 1);
    assert!(g.def_of(landers[0]).types.artifact);
}

#[test]
fn vanquishers_banner_pumps_and_draws_for_the_chosen_type() {
    let mut g = table();
    let banner = put(&mut g, 0, "Vanquisher's Banner");
    g.objects[banner.0 as usize].chosen_type = Some("Elf".to_string()); // chosen as it entered
    let elves = hand(&mut g, 0, "Llanowar Elves");
    let before = g.players[0].hand.len();
    play(&mut g, elves, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), before - 1 + 1, "an Elf spell: draw");
    assert_eq!((g.power(elves), g.toughness(elves)), (2, 2), "+1/+1");
    let lions = hand(&mut g, 0, "Savannah Lions");
    let before = g.players[0].hand.len();
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), before - 1, "a Cat: no draw");
    assert_eq!(g.power(lions), 2, "no pump");
}

// ---- Core: triggers, batch 38 (Ghyrson, Lonis) ----

/// Passes (no attackers, no blockers) until this turn's end step begins; its triggers are left
/// for the caller to resolve.
fn into_end_step(g: &mut Game) {
    while g.step != Step::End {
        let a = match g.pending {
            Pending::ChooseTarget(_) | Pending::Decide(_) => g.legal_actions()[0],
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn case_of_the_ransacked_lab_solves_after_four_instants_then_draws() {
    let mut g = table();
    let case = put(&mut g, 0, "Case of the Ransacked Lab");
    for _ in 0..4 {
        let bolt = hand(&mut g, 0, "Lightning Bolt");
        let before = g.players[0].hand.len();
        play(&mut g, bolt, false, &[Target::Player(1)], &[]);
        assert_eq!(g.players[0].hand.len(), before - 1, "unsolved: no card");
    }
    assert!(g.obj(case).class_level < 2, "solved only at the end step");
    into_end_step(&mut g);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(case).class_level, 2, "solved");
    to_step(&mut g, 5, 0, Step::Main1);
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    let before = g.players[0].hand.len();
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    assert_eq!(g.players[0].hand.len(), before - 1 + 1, "solved: draw a card");
}

#[test]
fn briarbridge_patrol_investigates_and_puts_a_creature_in_after_three_clues() {
    let mut g = table();
    let patrol = put(&mut g, 0, "Briarbridge Patrol");
    let lions = put(&mut g, 1, "Savannah Lions");
    let bite = hand(&mut g, 0, "Bite Down");
    play(&mut g, bite, false, &[obj(patrol), obj(lions)], &[]);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
    assert_eq!(tokens(&g, 0, "Clue").len(), 1, "investigated");
    let clue_def = set("CMD")
        .unwrap()
        .defs
        .iter()
        .position(|d| d.token && d.name == "Clue")
        .expect("the Clue token");
    g.add_card(0, clue_def, Zone::Battlefield);
    g.add_card(0, clue_def, Zone::Battlefield);
    for clue in tokens(&g, 0, "Clue") {
        act(&mut g, Action::Activate(clue, 0));
        resolve_all(&mut g, &[]);
    }
    assert!(tokens(&g, 0, "Clue").is_empty());
    let elves = hand(&mut g, 0, "Llanowar Elves");
    into_end_step(&mut g);
    resolve_all(&mut g, &[Action::Pick(elves)]);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield, "three Clues: put it in");
}

#[test]
fn raggadragga_pumps_and_untaps_mana_creatures_and_rewards_seven_mana_spells() {
    let mut g = table();
    put(&mut g, 0, "Raggadragga, Goreguts Boss");
    let elves = put(&mut g, 0, "Llanowar Elves");
    let lions = put(&mut g, 0, "Savannah Lions");
    assert_eq!((g.power(elves), g.toughness(elves)), (3, 3), "+2/+2");
    assert_eq!(g.power(lions), 2, "no mana ability");
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    assert!(!g.obj(lions).tapped);
    g.objects[lions.0 as usize].tapped = true;
    let akroma = hand(&mut g, 0, "Akroma, Angel of Wrath");
    play(&mut g, akroma, false, &[], &[Action::Target(obj(lions))]);
    assert!(!g.obj(lions).tapped, "untapped");
    assert_eq!((g.power(lions), g.toughness(lions)), (9, 8), "+7/+7");
    assert!(g.keywords(lions).has(Keywords::TRAMPLE));
    attack(&mut g, &[elves]);
    resolve_all(&mut g, &[]);
    assert!(!g.obj(elves).tapped, "untap it");
}

// ---- Core: triggers, batch 39 (Niv-Mizzet) ----

#[test]
fn end_blaze_epiphany_exiles_by_the_dead_creatures_power_and_plays_one() {
    let mut g = table();
    let lions = put(&mut g, 1, "Savannah Lions"); // 2/1
    let top = g.players[0].library.len();
    let bolt = g.add_card(0, cid("Lightning Bolt"), Zone::Library); // second from the top
    g.players[0].library.swap(top, top - 1);
    let epiphany = hand(&mut g, 0, "End-Blaze Epiphany");
    act(&mut g, Action::CastX(epiphany, 2));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[Action::Pick(bolt)]);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
    assert_eq!(g.players[0].library.len(), top - 1, "two exiled: its power");
    assert_eq!(g.obj(bolt).zone, Zone::Exile);
    assert!(
        g.legal_actions()
            .iter()
            .any(|a| matches!(a, Action::Cast(c) if *c == bolt)),
        "the chosen card can be played"
    );
    // A creature that survives: no trigger.
    let thopter = put(&mut g, 1, "Ornithopter"); // 0/2
    let library = g.players[0].library.len();
    let epiphany = hand(&mut g, 0, "End-Blaze Epiphany");
    act(&mut g, Action::CastX(epiphany, 1));
    act(&mut g, Action::Target(obj(thopter)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].library.len(), library);
}

// ---- Core: triggers, batch 40 (idea decks) ----

/// Passes through combat damage to the second main phase, answering every question with its first
/// legal choice and declaring no blockers.
fn fight(g: &mut Game) {
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::ChooseTarget(_) | Pending::Decide(_) => g.legal_actions()[0],
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn cathars_crusade_counters_each_of_your_creatures_as_one_enters() {
    let mut g = table();
    put(&mut g, 0, "Cathars' Crusade");
    let elves = put(&mut g, 0, "Llanowar Elves");
    let lions = hand(&mut g, 0, "Savannah Lions");
    play(&mut g, lions, false, &[], &[]);
    assert_eq!((g.obj(lions).counters, g.obj(elves).counters), (1, 1));
    let more = hand(&mut g, 0, "Llanowar Elves");
    play(&mut g, more, false, &[], &[]);
    assert_eq!(
        (g.obj(lions).counters, g.obj(elves).counters, g.obj(more).counters),
        (2, 2, 1)
    );
}

#[test]
fn juniper_order_ranger_counters_the_newcomer_and_itself() {
    let mut g = table();
    let ranger = hand(&mut g, 0, "Juniper Order Ranger");
    play(&mut g, ranger, false, &[], &[]);
    assert_eq!(g.obj(ranger).counters, 0, "only another creature");
    let elves = put(&mut g, 0, "Llanowar Elves");
    let lions = hand(&mut g, 0, "Savannah Lions");
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(
        (
            g.obj(lions).counters,
            g.obj(ranger).counters,
            g.obj(elves).counters
        ),
        (1, 1, 0)
    );
}

#[test]
fn adeline_counts_your_creatures_and_makes_an_attacking_human_when_you_attack() {
    let mut g = table();
    let adeline = put(&mut g, 0, "Adeline, Resplendent Cathar");
    let lions = put(&mut g, 0, "Savannah Lions");
    put(&mut g, 0, "Llanowar Elves");
    assert_eq!(g.power(adeline), 3);
    attack(&mut g, &[lions]);
    drain(&mut g);
    let humans = tokens(&g, 0, "Human");
    assert_eq!(humans.len(), 1);
    assert!(g.obj(humans[0]).tapped);
    assert!(g
        .combat
        .as_ref()
        .expect("in combat")
        .attackers
        .contains(&humans[0]));
    assert_eq!(g.power(adeline), 4);
    fight(&mut g);
    assert_eq!(g.players[1].life, 40 - 2 - 1);
    assert_eq!(tokens(&g, 0, "Human").len(), 1, "it stays");
}

#[test]
fn alela_makes_a_faerie_for_artifact_and_enchantment_spells_and_pumps_fliers() {
    let mut g = table();
    let alela = put(&mut g, 0, "Alela, Artful Provocateur");
    let ring = hand(&mut g, 0, "Sol Ring");
    play(&mut g, ring, false, &[], &[]);
    let faeries = tokens(&g, 0, "Faerie");
    assert_eq!(faeries.len(), 1);
    assert_eq!(g.power(faeries[0]), 2, "+1/+0 for another flier");
    assert_eq!(g.power(alela), 2, "not Alela herself");
    let blood = hand(&mut g, 0, "Exquisite Blood");
    play(&mut g, blood, false, &[], &[]);
    assert_eq!(tokens(&g, 0, "Faerie").len(), 2);
    put(&mut g, 0, "Plains");
    let lions = hand(&mut g, 0, "Savannah Lions");
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(tokens(&g, 0, "Faerie").len(), 2, "a creature spell");
    assert_eq!(g.power(lions), 2, "no flying: no pump");
}

#[test]
fn sharding_sphinx_makes_a_thopter_for_each_artifact_creature_that_connects() {
    let mut g = table();
    put(&mut g, 0, "Sharding Sphinx");
    let golem = put(&mut g, 0, "Solemn Simulacrum");
    let strix = put(&mut g, 0, "Baleful Strix");
    let lions = put(&mut g, 0, "Savannah Lions");
    attack(&mut g, &[golem, strix, lions]);
    fight(&mut g);
    assert_eq!(g.players[1].life, 40 - 5);
    assert_eq!(
        tokens(&g, 0, "Thopter").len(),
        2,
        "two artifact creatures, not the Cat"
    );
}

#[test]
fn thopter_spy_network_makes_thopters_with_an_artifact_and_draws_once_per_combat() {
    let mut g = table();
    put(&mut g, 0, "Thopter Spy Network");
    to_step(&mut g, 5, 0, Step::Main1);
    assert!(tokens(&g, 0, "Thopter").is_empty(), "no artifact");
    put(&mut g, 0, "Sol Ring");
    to_step(&mut g, 7, 0, Step::Main1);
    assert_eq!(tokens(&g, 0, "Thopter").len(), 1);
    let golem = put(&mut g, 0, "Solemn Simulacrum");
    let strix = put(&mut g, 0, "Baleful Strix");
    let n = g.players[0].hand.len();
    attack(&mut g, &[golem, strix]);
    fight(&mut g);
    assert_eq!(tokens(&g, 0, "Thopter").len(), 2, "the upkeep Thopter");
    assert_eq!(
        g.players[0].hand.len(),
        n + 2,
        "the draw step, then one card for both"
    );
}

#[test]
fn helm_of_the_ghastlord_draws_for_blue_and_discards_for_black() {
    let mut g = table();
    let strix = put(&mut g, 0, "Baleful Strix"); // blue and black
    let helm = hand(&mut g, 0, "Helm of the Ghastlord");
    play(&mut g, helm, false, &[obj(strix)], &[]);
    assert_eq!((g.power(strix), g.toughness(strix)), (3, 3), "+1/+1 twice");
    let lions = put(&mut g, 0, "Savannah Lions");
    let white = hand(&mut g, 0, "Helm of the Ghastlord");
    play(&mut g, white, false, &[obj(lions)], &[]);
    assert_eq!(g.power(lions), 2, "white: nothing");
    let (library, theirs) = (g.players[0].library.len(), g.players[1].hand.len());
    attack(&mut g, &[strix, lions]);
    fight(&mut g);
    assert_eq!(g.players[1].life, 40 - 5);
    assert_eq!(
        g.players[0].library.len(),
        library - 2,
        "the draw step and the blue trigger"
    );
    assert_eq!(g.players[1].hand.len(), theirs - 1, "the black trigger");
}

#[test]
fn lifeline_returns_a_dead_creature_at_the_end_step_if_another_creature_is_out() {
    let mut g = table();
    put(&mut g, 0, "Lifeline");
    let lions = put(&mut g, 0, "Savannah Lions");
    put(&mut g, 1, "Llanowar Elves");
    kill(&mut g, lions);
    into_end_step(&mut g);
    drain(&mut g);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield, "back at the end step");
    assert_eq!(g.obj(lions).controller, 0);
    // Alone on the battlefield: nothing comes back.
    let mut g = table();
    put(&mut g, 0, "Lifeline");
    let lions = put(&mut g, 0, "Savannah Lions");
    kill(&mut g, lions);
    into_end_step(&mut g);
    drain(&mut g);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
}
