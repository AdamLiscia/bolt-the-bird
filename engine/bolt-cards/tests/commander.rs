//! 1v1 Commander (CR 903, the parts a two-player game needs): 40 life, the command zone,
//! commander tax, commanders return to the command zone, and 21 combat damage from one commander.

mod common;

use bolt_cards::fdn;
use bolt_core::{Action, Game, Pending, Step, Zone, COMMANDER_LIFE};
use common::*;

/// A 1v1 Commander game: 99-card libraries of Aegis Turtles, Serra Angel as player 0's commander and
/// Savannah Lions as player 1's.
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
    g
}

#[test]
fn a_commander_game_starts_at_40_with_the_commanders_in_the_command_zone() {
    let g = commander_game();
    assert_eq!(COMMANDER_LIFE, 40);
    for p in 0..2 {
        assert_eq!(g.players[p].life, 40);
        assert_eq!(g.players[p].library.len() + g.players[p].hand.len(), 99);
        let c = g.commanders[p].expect("a commander");
        assert_eq!(g.obj(c).zone, Zone::Command);
        assert_eq!(g.obj(c).owner, p);
    }
    assert_eq!(g.command.len(), 2);
}

/// Player 0's first main phase with 5 of each basic land, in a commander game.
fn main_phase_with_lands() -> Game {
    let mut g = commander_game();
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
fn you_cast_your_commander_from_the_command_zone_and_pay_more_each_time() {
    let mut g = main_phase_with_lands();
    let angel = g.commanders[0].unwrap();
    assert!(g.legal_actions().contains(&Action::Cast(angel)));
    assert!(
        !g.legal_actions()
            .contains(&Action::Cast(g.commanders[1].unwrap())),
        "not the opponent's"
    );
    let untapped = |g: &Game| {
        g.permanents(0)
            .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
            .count()
    };
    let before = untapped(&g);
    resolve_all_after(&mut g, Action::Cast(angel));
    assert_eq!(g.obj(angel).zone, Zone::Battlefield);
    assert_eq!(
        before - untapped(&g),
        5,
        "Serra Angel costs {{3}}{{W}}{{W}} the first time"
    );
    assert_eq!(g.commander_casts[0], 1);
    // It dies: back to the command zone, not the graveyard.
    let pie = hand(&mut g, 0, "Bake into a Pie"); // destroy target creature
    play(&mut g, pie, false, &[obj(angel)], &[]);
    assert_eq!(
        g.obj(angel).zone,
        Zone::Command,
        "a commander that would die goes to the command zone"
    );
    assert!(!g.players[0].graveyard.contains(&angel));
    assert!(
        g.command.contains(&angel),
        "listed in the command zone: {:?}",
        g.command
    );
    assert_eq!(g.step, Step::Main1);
    assert!(g.items.is_empty(), "stack empty: {:?}", g.items.len());
    for _ in 0..5 {
        put(&mut g, 0, "Plains"); // the first cast's payment used every Plains
    }
    let before = untapped(&g);
    resolve_all_after(&mut g, Action::Cast(angel));
    assert_eq!(before - untapped(&g), 7, "the second cast costs {{2}} more");
    assert_eq!(g.commander_casts[0], 2);
}

fn resolve_all_after(g: &mut Game, a: Action) {
    act(g, a);
    resolve_all(g, &[]);
}

#[test]
fn a_commander_bounced_or_exiled_also_goes_to_the_command_zone() {
    let mut g = main_phase_with_lands();
    let lions = g.commanders[1].unwrap();
    g.move_card(lions, Zone::Battlefield);
    g.move_card(lions, Zone::Hand);
    assert_eq!(g.obj(lions).zone, Zone::Command);
    g.move_card(lions, Zone::Battlefield);
    g.move_card(lions, Zone::Exile);
    assert_eq!(g.obj(lions).zone, Zone::Command);
}

#[test]
fn twenty_one_combat_damage_from_one_commander_loses_the_game() {
    let mut g = main_phase_with_lands();
    let angel = g.commanders[0].unwrap();
    resolve_all_after(&mut g, Action::Cast(angel));
    g.players[1].life = 100; // far above zero: only commander damage can end it
    g.commander_damage[1] = 18;
    g.start_turn(5, 0); // a later turn: no longer summoning sick
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(angel));
    act(&mut g, Action::FinishAttacks);
    for _ in 0..20 {
        if g.is_over() {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.commander_damage[1], 22, "18 + the Angel's 4");
    assert!(
        g.players[1].life > 0 && g.players[1].lost,
        "21+ commander damage loses at any life total"
    );
}

#[test]
fn limited_games_are_unchanged() {
    let g = table();
    assert_eq!(g.players[0].life, 20);
    assert!(g.commanders.iter().all(|c| c.is_none()) && g.command.is_empty());
}
