//! Tier 4, batch 9: Quilled Greatwurm.

mod common;

use bolt_core::{Action, Game, Pending, Step, Zone};
use common::*;

fn pass_to(g: &mut Game, step: Step) {
    while g.step != step {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn creatures_that_deal_combat_damage_on_your_turn_get_that_many_counters() {
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Quilled Greatwurm");
    let lions = put(&mut g, 0, "Savannah Lions");
    pass_to(&mut g, Step::DeclareAttackers);
    act(&mut g, Action::Attack(lions));
    act(&mut g, Action::FinishAttacks);
    pass_to(&mut g, Step::EndCombat);
    assert_eq!(g.players[1].life, 18);
    assert_eq!(g.obj(lions).counters, 2);
}

#[test]
fn not_on_the_opponents_turn() {
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Quilled Greatwurm");
    let wall = put(&mut g, 0, "Aegis Turtle");
    let raider = put(&mut g, 1, "Gorehorn Raider");
    pass_to(&mut g, Step::End);
    pass_to(&mut g, Step::DeclareAttackers); // the opponent's turn
    act(&mut g, Action::Attack(raider));
    act(&mut g, Action::FinishAttacks);
    pass_to(&mut g, Step::DeclareBlockers);
    act(
        &mut g,
        Action::Block {
            blocker: wall,
            attacker: raider,
        },
    );
    act(&mut g, Action::FinishBlocks);
    pass_to(&mut g, Step::EndCombat);
    assert_eq!(g.obj(raider).counters, 0, "the opponent has no Greatwurm");
    assert_eq!(g.obj(wall).counters, 0, "not our turn");
}

#[test]
fn cast_from_the_graveyard_by_removing_six_counters() {
    let mut g = table();
    begin(&mut g);
    let wurm = g.add_card(0, id("Quilled Greatwurm"), Zone::Graveyard);
    let a = put(&mut g, 0, "Savannah Lions");
    let b = put(&mut g, 0, "Savannah Lions");
    g.objects[a.0 as usize].counters = 4;
    g.objects[b.0 as usize].counters = 1;
    assert!(
        !g.legal_actions().contains(&Action::CastFromGraveyard(wurm)),
        "only five"
    );
    g.objects[b.0 as usize].counters = 2;
    assert!(g.legal_actions().contains(&Action::CastFromGraveyard(wurm)));
    act(&mut g, Action::CastFromGraveyard(wurm));
    assert!(matches!(g.pending, Pending::RemoveCounters(0)));
    for _ in 0..4 {
        act(&mut g, Action::Pick(a));
    }
    assert!(
        !g.legal_actions().contains(&Action::Pick(a)),
        "a is out of counters"
    );
    act(&mut g, Action::Pick(b));
    act(&mut g, Action::Pick(b));
    assert_eq!((g.obj(a).counters, g.obj(b).counters), (0, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(wurm).zone, Zone::Battlefield);
}
