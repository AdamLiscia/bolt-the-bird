//! Tier 4, batch 11: Etali, Primal Storm (casting spells without paying their mana costs, CR
//! 608.2g, 118.9).

mod common;

use bolt_core::{Action, Game, Pending, Step, Target, Zone};
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

/// Etali attacks with `ours` and `theirs` on top of the libraries; returns once the window to
/// cast them is open.
fn attack(g: &mut Game, ours: &str, theirs: &str) -> (bolt_core::ObjId, bolt_core::ObjId) {
    let etali = put(g, 0, "Etali, Primal Storm");
    pass_to(g, Step::DeclareAttackers);
    let a = g.add_card(0, id(ours), Zone::Library);
    let b = g.add_card(1, id(theirs), Zone::Library);
    act(g, Action::Attack(etali));
    act(g, Action::FinishAttacks);
    act(g, Action::PassPriority);
    act(g, Action::PassPriority);
    (a, b)
}

#[test]
fn etali_casts_spells_from_both_libraries_for_free() {
    let mut g = table();
    begin(&mut g);
    for c in g.permanents(0).collect::<Vec<_>>() {
        g.objects[c.0 as usize].tapped = true; // no mana: the casts must be free
    }
    let (bolt, lions) = attack(&mut g, "Burst Lightning", "Savannah Lions");
    assert!(matches!(g.pending, Pending::FreeCast(0)), "{:?}", g.pending);
    assert_eq!((g.obj(bolt).zone, g.obj(lions).zone), (Zone::Exile, Zone::Exile));
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Cast(bolt)) && legal.contains(&Action::Cast(lions)));
    act(&mut g, Action::Cast(lions));
    assert!(matches!(g.pending, Pending::FreeCast(0)), "the window stays open");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::PassPriority); // done casting
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 18);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield);
    assert_eq!((g.obj(lions).controller, g.obj(lions).owner), (0, 1));
}

#[test]
fn etali_cant_play_lands_and_uncast_cards_stay_exiled() {
    let mut g = table();
    begin(&mut g);
    let (bolt, land) = attack(&mut g, "Burst Lightning", "Forest");
    assert!(!g.legal_actions().contains(&Action::PlayLand(land)));
    assert!(!g.legal_actions().contains(&Action::Cast(land)));
    act(&mut g, Action::PassPriority); // cast nothing
    assert!(matches!(g.pending, Pending::Priority(_)));
    assert_eq!(g.obj(bolt).zone, Zone::Exile);
    assert!(
        !g.legal_actions().contains(&Action::Cast(bolt)),
        "only during the ability"
    );
}

#[test]
fn no_window_when_nothing_can_be_cast() {
    let mut g = table();
    begin(&mut g);
    attack(&mut g, "Forest", "Island");
    assert!(matches!(g.pending, Pending::Priority(_)));
}
