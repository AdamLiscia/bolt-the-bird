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
