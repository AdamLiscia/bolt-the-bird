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
