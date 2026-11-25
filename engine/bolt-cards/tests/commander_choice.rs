//! Commander (CR 903.9a), with `Game::commander_choice` on: a commander put into a graveyard or exile
//! stays there until its owner chooses, as a state-based action, whether to move it to the command
//! zone. Off (the default), it goes straight to the command zone (tests/commander.rs).

mod common;

use bolt_cards::fdn;
use bolt_core::{Action, Game, Pending, Step, Zone};
use common::*;

fn commander_game() -> Game {
    let turtles = vec![id("Aegis Turtle"); 99];
    let mut g = Game::new_commander(
        fdn(),
        [&turtles, &turtles],
        [id("Serra Angel"), id("Savannah Lions")],
        5,
        0,
    );
    g.auto_pass = false;
    g.commander_choice = true;
    for p in 0..2 {
        for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            for _ in 0..5 {
                g.add_card(p, id(land), Zone::Battlefield);
            }
        }
    }
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g
}

#[test]
fn a_commander_that_dies_waits_in_the_graveyard_for_its_owners_choice() {
    let mut g = commander_game();
    let lions = g.commanders[1].unwrap();
    g.move_card(lions, Zone::Battlefield);
    let stab = hand(&mut g, 0, "Stab"); // -2/-2: the 2/1 Lions die
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(lions)));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(lions).zone, Zone::Graveyard, "it died");
    assert_eq!(g.pending, Pending::Decide(1), "its owner chooses");
    assert_eq!(g.legal_actions(), [Action::Yes, Action::No]);
    act(&mut g, Action::Yes);
    assert_eq!(g.obj(lions).zone, Zone::Command);
    assert!(matches!(g.pending, Pending::Priority(_)));
}

#[test]
fn the_owner_may_leave_an_exiled_commander_in_exile() {
    let mut g = commander_game();
    let angel = g.commanders[0].unwrap();
    g.move_card(angel, Zone::Battlefield);
    g.move_card(angel, Zone::Exile);
    assert_eq!(g.obj(angel).zone, Zone::Exile);
    act(&mut g, Action::PassPriority); // the next priority check asks
    assert_eq!(g.pending, Pending::Decide(0));
    act(&mut g, Action::No);
    assert_eq!(g.obj(angel).zone, Zone::Exile);
    // Asked once: passing on doesn't ask again.
    for _ in 0..3 {
        assert!(!matches!(g.pending, Pending::Decide(_)));
        act(&mut g, Action::PassPriority);
    }
}

#[test]
fn hand_and_library_still_go_to_the_command_zone() {
    let mut g = commander_game();
    let lions = g.commanders[1].unwrap();
    g.move_card(lions, Zone::Battlefield);
    g.move_card(lions, Zone::Hand);
    assert_eq!(g.obj(lions).zone, Zone::Command);
}

// ---- Partners, per-commander damage, options ----

use bolt_core::CommanderOptions;

/// Player 0 has two commanders (Serra Angel and Savannah Lions), player 1 has Serra Angel.
fn partner_game(options: CommanderOptions) -> Game {
    let turtles = vec![id("Aegis Turtle"); 98];
    let mut g = Game::new_commander_with(
        fdn(),
        [&turtles, &turtles],
        [&[id("Serra Angel"), id("Savannah Lions")], &[id("Serra Angel")]],
        options,
        5,
        0,
    );
    g.auto_pass = false;
    for p in 0..2 {
        for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            for _ in 0..5 {
                g.add_card(p, id(land), Zone::Battlefield);
            }
        }
    }
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g
}

fn untapped(g: &Game) -> usize {
    g.permanents(0)
        .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
        .count()
}

#[test]
fn partners_each_pay_their_own_tax() {
    let mut g = partner_game(CommanderOptions::default());
    let (angel, lions) = (g.commanders[0].unwrap(), g.partners[0].unwrap());
    assert_eq!(
        (g.obj(angel).zone, g.obj(lions).zone),
        (Zone::Command, Zone::Command)
    );
    assert!(g.is_commander(lions));
    assert_eq!(g.players[0].library.len() + g.players[0].hand.len(), 98);
    for _ in 0..10 {
        g.add_card(0, id("Plains"), Zone::Battlefield);
    }
    let before = untapped(&g);
    act(&mut g, Action::Cast(angel));
    resolve_all(&mut g, &[]);
    assert_eq!(before - untapped(&g), 5, "the Angel's first cast: no tax");
    g.move_card(angel, Zone::Exile);
    assert_eq!(g.obj(angel).zone, Zone::Command);
    let before = untapped(&g);
    act(&mut g, Action::Cast(lions));
    resolve_all(&mut g, &[]);
    assert_eq!(
        before - untapped(&g),
        1,
        "the Lions' own first cast: no tax either"
    );
    let before = untapped(&g);
    act(&mut g, Action::Cast(angel));
    resolve_all(&mut g, &[]);
    assert_eq!(before - untapped(&g), 7, "the Angel's second: {{2}} more");
}

#[test]
fn commander_damage_is_counted_per_commander() {
    let mut g = partner_game(CommanderOptions::default());
    let (angel, lions) = (g.commanders[0].unwrap(), g.partners[0].unwrap());
    g.move_card(angel, Zone::Battlefield);
    g.move_card(lions, Zone::Battlefield);
    g.commander_damage_from.push((angel, 1, 15));
    g.commander_damage_from.push((lions, 1, 15));
    g.commander_damage[1] = 30;
    g.players[1].life = 100;
    act(&mut g, Action::PassPriority);
    assert!(!g.players[1].lost, "30 from two commanders isn't 21 from one");
    g.commander_damage_from[0].2 = 21;
    act(&mut g, Action::PassPriority);
    assert!(g.players[1].lost);
}

#[test]
fn a_mirror_match_keeps_two_commander_damage_entries() {
    let turtles = vec![id("Aegis Turtle"); 99];
    let mut g = Game::new_commander(
        fdn(),
        [&turtles, &turtles],
        [id("Serra Angel"), id("Serra Angel")],
        5,
        0,
    );
    let (a, b) = (g.commanders[0].unwrap(), g.commanders[1].unwrap());
    assert_ne!(a, b, "two objects");
    g.auto_pass = false;
    g.move_card(a, Zone::Battlefield);
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(a));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        let x = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, x);
    }
    assert_eq!(g.commander_damage_from, [(a, 1, 4)]);
    assert_eq!(g.commander_damage[1], 4);
}

#[test]
fn starting_life_and_per_seat_auto_return_are_options() {
    let mut g = partner_game(CommanderOptions {
        starting_life: 30,
        auto_command_zone: [true, false],
    });
    assert_eq!((g.players[0].life, g.players[1].life), (30, 30));
    g.commander_choice = true;
    let mine = g.commanders[0].unwrap();
    let theirs = g.commanders[1].unwrap();
    g.move_card(mine, Zone::Battlefield);
    g.move_card(theirs, Zone::Battlefield);
    g.move_card(mine, Zone::Exile);
    g.move_card(theirs, Zone::Exile);
    act(&mut g, Action::PassPriority);
    assert_eq!(g.obj(mine).zone, Zone::Command, "seat 0 always returns");
    assert_eq!(g.pending, Pending::Decide(1), "seat 1 is asked");
    act(&mut g, Action::No);
    assert_eq!(g.obj(theirs).zone, Zone::Exile);
}
