//! Core theme "objects and zones" (lane C): each piece tested directly, then the cards that waited on it.
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

fn top(g: &mut Game, name: &str) -> ObjId {
    g.add_card(0, cid(name), Zone::Library)
}

fn to_main1(g: &mut Game, turn: u32, active: usize) {
    g.start_turn(turn, active);
    while g.step != Step::Main1 {
        act(g, Action::PassPriority);
    }
}

// ---- Clue tokens and investigate ----

#[test]
fn investigate_makes_a_clue_that_draws_for_two() {
    let mut g = table();
    let fh = put(&mut g, 0, "Floodhound");
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Activate(fh, 0));
    resolve_all(&mut g, &[]);
    let clues = tokens(&g, 0, "Clue");
    assert_eq!(clues.len(), 1);
    assert!(g.types_of(clues[0]).artifact);
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(clues[0], 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n + 1);
    assert!(tokens(&g, 0, "Clue").is_empty(), "sacrificed");
}

#[test]
fn investigating_the_first_time_each_turn_triggers_erdwal_illuminator() {
    let mut g = table();
    put(&mut g, 0, "Erdwal Illuminator");
    let cs = hand(&mut g, 0, "Confront the Unknown");
    let target = put(&mut g, 0, "Hungry Ghoul");
    play(&mut g, cs, false, &[obj(target)], &[]);
    assert_eq!(
        tokens(&g, 0, "Clue").len(),
        2,
        "one more for the first investigate"
    );
    assert!(g.power(target) >= 3, "+1/+1 for each Clue as it resolved");
    let cs2 = hand(&mut g, 0, "Confront the Unknown");
    play(&mut g, cs2, false, &[obj(target)], &[]);
    assert_eq!(tokens(&g, 0, "Clue").len(), 3, "not the first time this turn");
}

#[test]
fn confirm_suspicions_counters_and_investigates_three_times() {
    let mut g = table();
    let think = hand(&mut g, 0, "Think Twice");
    let cs = hand(&mut g, 0, "Confirm Suspicions");
    act(&mut g, Action::Cast(think));
    act(&mut g, Action::Cast(cs));
    resolve_all(&mut g, &[Action::Target(obj(think))]);
    assert_eq!(g.obj(think).zone, Zone::Graveyard);
    assert_eq!(tokens(&g, 0, "Clue").len(), 3);
}

#[test]
fn ethereal_investigator_investigates_and_makes_a_spirit_on_the_second_draw() {
    let mut g = table();
    let ei = hand(&mut g, 0, "Ethereal Investigator");
    play(&mut g, ei, false, &[], &[]);
    assert_eq!(tokens(&g, 0, "Clue").len(), 1);
    let clue = tokens(&g, 0, "Clue")[0];
    act(&mut g, Action::Activate(clue, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(
        tokens(&g, 0, "Spirit").len(),
        1,
        "the Clue's draw was the second this turn"
    );
}

#[test]
fn hold_for_questioning_freezes_and_investigates() {
    let mut g = table();
    let top_ = put(&mut g, 1, "Sensei's Divining Top");
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let hq = hand(&mut g, 0, "Hold for Questioning");
    play(&mut g, hq, false, &[obj(ghoul)], &[]);
    let _ = top_; // its controller's other permanents still work
    assert!(g.obj(ghoul).tapped);
    assert_eq!(tokens(&g, 0, "Clue").len(), 1);
    to_main1(&mut g, 4, 1);
    assert!(g.obj(ghoul).tapped, "doesn't untap");
    assert!(!g
        .legal_actions()
        .iter()
        .any(|a| matches!(a, Action::ActivateWith(x, _, _) if *x == ghoul)));
}

#[test]
fn ongoing_investigation_investigates_for_graveyard_creatures() {
    let mut g = table();
    let oi = put(&mut g, 0, "Ongoing Investigation");
    let dead = g.add_card(0, cid("Hungry Ghoul"), Zone::Graveyard);
    let life = g.players[0].life;
    act(&mut g, Action::ActivateWith(oi, 0, dead));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Clue").len(), 1);
    assert_eq!(g.players[0].life, life + 2);
}

#[test]
fn sacrificing_several_of_a_subtype_as_a_cost() {
    let mut g = table();
    let tj = put(&mut g, 0, "Tamiyo's Journal");
    let fh = hand(&mut g, 0, "Confirm Suspicions");
    let think = hand(&mut g, 0, "Think Twice");
    act(&mut g, Action::Cast(think));
    act(&mut g, Action::Cast(fh));
    resolve_all(&mut g, &[Action::Target(obj(think))]);
    let clues = tokens(&g, 0, "Clue");
    assert_eq!(clues.len(), 3);
    let want = g.add_card(0, cid("Palinchron"), Zone::Library);
    act(&mut g, Action::ActivateWith(tj, 0, clues[0]));
    resolve_all(&mut g, &[Action::Pick(want)]);
    assert!(tokens(&g, 0, "Clue").is_empty(), "all three sacrificed");
    assert_eq!(g.obj(want).zone, Zone::Hand);
}

// ---- Food costs ----

#[test]
fn savvy_hunter_sacrifices_two_foods_to_draw() {
    let mut lands = vec!["Pippin, Warden of Isengard", "Savvy Hunter"];
    lands.extend(["Forest"; 6]);
    let mut g = game(&lands);
    let pip = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Pippin, Warden of Isengard")
        .unwrap();
    let sh = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Savvy Hunter")
        .unwrap();
    act(&mut g, Action::Activate(pip, 0));
    resolve_all(&mut g, &[]);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Activate(pip, 0));
    resolve_all(&mut g, &[]);
    let foods = tokens(&g, 0, "Food");
    assert_eq!(foods.len(), 2);
    assert!(g.legal_actions().contains(&Action::ActivateWith(sh, 0, foods[0])));
    let n = g.players[0].hand.len();
    act(&mut g, Action::ActivateWith(sh, 0, foods[0]));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n + 1);
    assert!(tokens(&g, 0, "Food").is_empty());
}

#[test]
fn pippin_sacrifices_four_foods_for_a_team_pump() {
    let mut lands = vec!["Pippin, Warden of Isengard", "Hungry Ghoul"];
    lands.extend(["Forest"; 4]);
    let mut g = game(&lands);
    let pip = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Pippin, Warden of Isengard")
        .unwrap();
    let ghoul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    let food_def = g.defs.iter().position(|d| d.token && d.name == "Food").unwrap();
    for _ in 0..4 {
        g.add_card(0, food_def, Zone::Battlefield);
    }
    let f0 = tokens(&g, 0, "Food")[0];
    act(&mut g, Action::ActivateWith(pip, 1, f0));
    resolve_all(&mut g, &[]);
    assert!(tokens(&g, 0, "Food").is_empty());
    assert_eq!(g.power(ghoul), 5);
    assert!(g.keywords(ghoul).has(Keywords::HASTE));
}

// ---- Adventures ----

#[test]
fn an_adventure_is_exiled_and_the_creature_cast_later() {
    let mut g = table();
    let ghoul = top(&mut g, "Hungry Ghoul");
    for _ in 0..6 {
        top(&mut g, "Plains");
    }
    let bf = hand(&mut g, 0, "Bramble Familiar // Fetch Quest");
    act(&mut g, Action::CastOmen(bf));
    resolve_all(&mut g, &[Action::Pick(ghoul)]);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    assert_eq!(g.obj(bf).zone, Zone::Exile, "on an adventure");
    assert!(g.legal_actions().contains(&Action::Cast(bf)));
    play(&mut g, bf, false, &[], &[]);
    assert_eq!(g.obj(bf).zone, Zone::Battlefield);
}

#[test]
fn monster_manual_studies_then_puts_creatures_in() {
    let mut g = table();
    let pal = top(&mut g, "Palinchron");
    for _ in 0..4 {
        top(&mut g, "Plains");
    }
    let mm = hand(&mut g, 0, "Monster Manual // Zoological Study");
    act(&mut g, Action::CastOmen(mm));
    resolve_all(&mut g, &[Action::Pick(pal)]);
    assert_eq!(g.obj(pal).zone, Zone::Hand);
    assert_eq!(g.obj(mm).zone, Zone::Exile);
    play(&mut g, mm, false, &[], &[]);
    to_main1(&mut g, 5, 0);
    act(&mut g, Action::Activate(mm, 0));
    resolve_all(&mut g, &[Action::Pick(pal)]);
    assert_eq!(g.obj(pal).zone, Zone::Battlefield);
}
