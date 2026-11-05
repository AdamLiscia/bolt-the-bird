//! Tier-4 cards, batch 3: token copies (CR 707.2, 111.10).

mod common;

use bolt_core::{Action, Game, Keywords, Pending, Step, Zone};
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

fn copies_of(g: &Game, p: usize, name: &str) -> Vec<bolt_core::ObjId> {
    g.creatures(p)
        .filter(|&c| g.def_of(c).name == name && g.is_token(c))
        .collect()
}

#[test]
fn self_reflection_copies_the_card_not_its_counters() {
    let mut g = table();
    let c = hand(&mut g, 0, "Self-Reflection");
    let angel = put(&mut g, 0, "Serra Angel");
    g.objects[angel.0 as usize].counters = 2;
    begin(&mut g);
    play(&mut g, c, false, &[obj(angel)], &[]);
    let copy = copies_of(&g, 0, "Serra Angel");
    assert_eq!(copy.len(), 1);
    assert_eq!(g.power(copy[0]), 4, "CR 707.2: counters aren't copied");
    assert!(g.keywords(copy[0]).has(Keywords::FLYING));
}

#[test]
fn a_copy_token_ceases_to_exist_when_it_leaves() {
    // CR 704.5d: a copy token is still a token.
    let mut g = table();
    let c = hand(&mut g, 0, "Self-Reflection");
    let stab = hand(&mut g, 0, "Stab");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, c, false, &[obj(lions)], &[]);
    let copy = copies_of(&g, 0, "Savannah Lions")[0];
    play(&mut g, stab, false, &[obj(copy)], &[]);
    assert_eq!(g.obj(copy).zone, Zone::Gone);
}

#[test]
fn electroduplicate_copy_has_haste_and_dies_at_end_of_turn() {
    let mut g = table();
    let c = hand(&mut g, 0, "Electroduplicate");
    let angel = put(&mut g, 0, "Serra Angel");
    begin(&mut g);
    play(&mut g, c, false, &[obj(angel)], &[]);
    let copy = copies_of(&g, 0, "Serra Angel")[0];
    assert!(g.keywords(copy).has(Keywords::HASTE));
    pass_to(&mut g, Step::End);
    assert_eq!(
        g.obj(copy).zone,
        Zone::Gone,
        "sacrificed at the beginning of the end step"
    );
}

#[test]
fn homunculus_horde_copies_itself_on_the_second_draw() {
    let mut g = table();
    put(&mut g, 0, "Homunculus Horde");
    let sailor = put(&mut g, 0, "Spectral Sailor");
    begin(&mut g);
    act(&mut g, Action::Activate(sailor, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(copies_of(&g, 0, "Homunculus Horde").len(), 1);
}

#[test]
fn extravagant_replication_copies_another_permanent_each_upkeep() {
    let mut g = table();
    put(&mut g, 0, "Extravagant Replication");
    let angel = put(&mut g, 0, "Serra Angel");
    g.start_turn(3, 0);
    resolve_all(&mut g, &[Action::Target(obj(angel))]);
    assert_eq!(copies_of(&g, 0, "Serra Angel").len(), 1);
}

#[test]
fn abyssal_harvester_reanimates_a_fresh_corpse_as_a_nightmare() {
    let mut g = table();
    let h = put(&mut g, 0, "Abyssal Harvester");
    let stab = hand(&mut g, 0, "Stab");
    let lions = put(&mut g, 1, "Savannah Lions");
    let old = g.add_card(1, id("Serra Angel"), Zone::Graveyard); // put there on an earlier turn
    begin(&mut g);
    play(&mut g, stab, false, &[obj(lions)], &[]);
    act(&mut g, Action::Activate(h, 0));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(old))),
        "not put there this turn"
    );
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Exile);
    let copy = copies_of(&g, 0, "Savannah Lions");
    assert_eq!(copy.len(), 1);
    assert!(g.has_subtype(copy[0], "Nightmare"));
}
