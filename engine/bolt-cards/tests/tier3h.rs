//! Tier-3 cards, batch 9a: alternative costs, leylines, flash for everything, graveyard triggers,
//! granted flashback, soul counters, and tapping creatures as a cost.

mod common;

use std::sync::Arc;

use bolt_cards::fdn;
use bolt_core::{Action, Game, Keywords, ObjId, Pending, Step, Zone};
use common::*;

fn attack_with(g: &mut Game, attackers: &[ObjId]) {
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    for &a in attackers {
        act(g, Action::Attack(a));
    }
    act(g, Action::FinishAttacks);
}

#[test]
fn blasphemous_edict_costs_b_with_thirteen_creatures() {
    let mut g = lands(&["Swamp"]);
    let edict = hand(&mut g, 0, "Blasphemous Edict");
    for _ in 0..7 {
        put(&mut g, 0, "Savannah Lions");
        put(&mut g, 1, "Savannah Lions");
    }
    begin(&mut g);
    act(&mut g, Action::CastAlternative(edict));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.creatures(0).count() + g.creatures(1).count(),
        0,
        "seven each: all of them"
    );

    let mut g = lands(&["Swamp"]);
    let edict = hand(&mut g, 0, "Blasphemous Edict");
    put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    assert!(
        !g.legal_actions().contains(&Action::CastAlternative(edict)),
        "only one creature"
    );
}

#[test]
fn leyline_axe_starts_on_the_battlefield() {
    // CR 103.6a. A 40-card deck of Leyline Axes: every one in the opening hand begins in play.
    let defs = fdn();
    let axe = id("Leyline Axe");
    let deck = vec![axe; 40];
    let mut g = Game::new(Arc::clone(&defs), [&deck, &deck], 1, 0);
    g.auto_pass = false;
    act(&mut g, Action::Keep);
    act(&mut g, Action::Keep);
    assert_eq!(
        g.permanents(0)
            .filter(|&c| g.def_of(c).name == "Leyline Axe")
            .count(),
        7
    );
}

#[test]
fn leyline_axe_equipped() {
    let mut g = table();
    let axe = put(&mut g, 0, "Leyline Axe");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    act(&mut g, Action::Activate(axe, 0));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(lions), 3);
    let k = g.keywords(lions);
    assert!(k.has(Keywords::DOUBLE_STRIKE) && k.has(Keywords::TRAMPLE));
}

#[test]
fn high_fae_trickster_gives_everything_flash() {
    let mut g = table();
    put(&mut g, 1, "High Fae Trickster");
    let lions = hand(&mut g, 1, "Savannah Lions");
    begin(&mut g);
    act(&mut g, Action::PassPriority);
    assert!(
        g.legal_actions().contains(&Action::Cast(lions)),
        "on the opponent's turn"
    );
}

#[test]
fn flamewake_phoenix_returns_with_ferocious() {
    let mut g = lands(&["Mountain"]);
    let phoenix = g.add_card(0, id("Flamewake Phoenix"), Zone::Graveyard);
    put(&mut g, 0, "Serra Angel");
    begin(&mut g);
    while g.step != Step::BeginCombat || g.pending != Pending::Decide(0) {
        act(&mut g, Action::PassPriority);
    }
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.obj(phoenix).zone, Zone::Battlefield);
    assert!(g.keywords(phoenix).has(Keywords::HASTE));
    assert!(
        !g.legal_actions().contains(&Action::FinishAttacks),
        "it attacks each combat if able"
    );
}

#[test]
fn sphinx_of_forgotten_lore_grants_flashback() {
    let mut g = table();
    let sphinx = put(&mut g, 0, "Sphinx of Forgotten Lore");
    let stab = g.add_card(0, id("Stab"), Zone::Graveyard);
    let theirs = put(&mut g, 1, "Savannah Lions");
    begin(&mut g);
    attack_with(&mut g, &[sphinx]);
    resolve_all(&mut g, &[Action::Target(obj(stab))]);
    act(&mut g, Action::CastFlashback(stab));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.obj(stab).zone, Zone::Exile, "flashback exiles it");
}

#[test]
fn ravenous_amulet_collects_souls() {
    let mut g = table();
    let amulet = put(&mut g, 0, "Ravenous Amulet");
    let a = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    act(&mut g, Action::ActivateWith(amulet, 0, a));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(amulet).soul, 1);
    g.objects[amulet.0 as usize].tapped = false;
    act(&mut g, Action::Activate(amulet, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.players[1].life, 19,
        "one soul counter, counted after it was sacrificed"
    );
}

#[test]
fn lathril_makes_elves_and_taps_ten_to_drain() {
    let mut g = table();
    let lathril = put(&mut g, 0, "Lathril, Blade of the Elves");
    begin(&mut g);
    attack_with(&mut g, &[lathril]);
    while g.step != Step::CombatDamage {
        act(&mut g, Action::PassPriority);
    }
    resolve_all(&mut g, &[]);
    assert_eq!(tokens_named(&g, 0, "Elf Warrior"), 2);

    let mut g = table();
    let lathril = put(&mut g, 0, "Lathril, Blade of the Elves");
    let elves: Vec<ObjId> = (0..10).map(|_| put(&mut g, 0, "Llanowar Elves")).collect();
    begin(&mut g);
    act(&mut g, Action::Activate(lathril, 0));
    resolve_all(&mut g, &[]);
    assert_eq!((g.players[0].life, g.players[1].life), (30, 10));
    assert!(elves.iter().all(|&e| g.obj(e).tapped));
}
