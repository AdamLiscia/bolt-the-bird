//! Tier-3 cards, batch 8: triggers with amounts, creature-type changes, kicked targets, extra land
//! drops, dynamic power and toughness, and hexproof from instants.

mod common;

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
fn bloodthirsty_conqueror_gains_what_the_opponent_loses() {
    let mut g = table();
    put(&mut g, 0, "Bloodthirsty Conqueror");
    let c = hand(&mut g, 0, "Boltwave");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!((g.players[0].life, g.players[1].life), (23, 17));
}

#[test]
fn niv_mizzet_draws_for_noncombat_damage() {
    let mut g = table();
    put(&mut g, 0, "Niv-Mizzet, Visionary");
    let c = hand(&mut g, 0, "Boltwave");
    begin(&mut g);
    let before = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), before - 1 + 3);
}

#[test]
fn drake_hatcher_incubates_drakes() {
    let mut g = table();
    let h = put(&mut g, 0, "Drake Hatcher");
    let growth = hand(&mut g, 0, "Giant Growth");
    begin(&mut g);
    play(&mut g, growth, false, &[obj(h)], &[]); // 1/3 +3/+3 +1/+1 prowess = 5/7
    attack_with(&mut g, &[h]);
    pass_to(&mut g, Step::CombatDamage);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(h).incubation, 5);
    act(&mut g, Action::Activate(h, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens_named(&g, 0, "Drake"), 1);
    assert_eq!(g.obj(h).incubation, 2);
    assert!(
        !g.legal_actions().contains(&Action::Activate(h, 0)),
        "only two left"
    );
}

#[test]
fn infernal_vessel_returns_once_as_a_demon() {
    let mut g = table();
    let v = put(&mut g, 0, "Infernal Vessel");
    let s1 = hand(&mut g, 1, "Stab");
    begin(&mut g);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(s1));
    act(&mut g, Action::Target(obj(v)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(v).zone, Zone::Battlefield);
    assert_eq!(g.power(v), 4);
    assert!(g.has_subtype(v, "Demon"));
    // As a Demon it stays dead.
    g.objects[v.0 as usize].damage = 10;
    let s2 = hand(&mut g, 0, "Stab");
    play(&mut g, s2, false, &[obj(v)], &[]);
    assert_eq!(g.obj(v).zone, Zone::Graveyard);
}

#[test]
fn valkyries_call_returns_creatures_as_angels() {
    let mut g = table();
    put(&mut g, 0, "Valkyrie's Call");
    let lions = put(&mut g, 0, "Savannah Lions");
    let stab = hand(&mut g, 1, "Stab");
    begin(&mut g);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield);
    assert!(g.has_subtype(lions, "Angel"));
    assert!(g.keywords(lions).has(Keywords::FLYING));
    assert_eq!(g.power(lions), 3);
}

#[test]
fn skyknight_squire_flies_at_three_counters() {
    let mut g = table();
    let squire = put(&mut g, 0, "Skyknight Squire");
    begin(&mut g);
    for _ in 0..3 {
        let l = hand(&mut g, 0, "Savannah Lions");
        play(&mut g, l, false, &[], &[]);
    }
    assert_eq!(g.obj(squire).counters, 3);
    assert!(g.keywords(squire).has(Keywords::FLYING));
    assert!(g.has_subtype(squire, "Knight"));
}

#[test]
fn mild_mannered_librarian_transforms_once() {
    let mut g = table();
    let lib = put(&mut g, 0, "Mild-Mannered Librarian");
    begin(&mut g);
    act(&mut g, Action::Activate(lib, 0));
    resolve_all(&mut g, &[]);
    assert!(g.has_subtype(lib, "Werewolf") && !g.has_subtype(lib, "Human"));
    assert_eq!(g.power(lib), 3);
    assert!(
        !g.legal_actions().contains(&Action::Activate(lib, 0)),
        "only once"
    );
}

#[test]
fn divine_resilience_kicked_protects_several() {
    let mut g = table();
    let c = hand(&mut g, 0, "Divine Resilience");
    let a = put(&mut g, 0, "Savannah Lions");
    let b = put(&mut g, 0, "Serra Angel");
    begin(&mut g);
    act(&mut g, Action::CastKicked(c));
    act(&mut g, Action::Target(obj(a)));
    act(&mut g, Action::Target(obj(b)));
    act(&mut g, Action::No);
    resolve_all(&mut g, &[]);
    assert!(g.keywords(a).has(Keywords::INDESTRUCTIBLE) && g.keywords(b).has(Keywords::INDESTRUCTIBLE));
}

#[test]
fn loot_plays_two_lands_and_digs_for_creatures() {
    let mut g = lands(&[]);
    put(&mut g, 0, "Loot, Exuberant Explorer");
    let a = hand(&mut g, 0, "Forest");
    let b = hand(&mut g, 0, "Forest");
    begin(&mut g);
    act(&mut g, Action::PlayLand(a));
    assert!(
        g.legal_actions().contains(&Action::PlayLand(b)),
        "an additional land"
    );
    act(&mut g, Action::PlayLand(b));

    let mut g = lands(&["Forest", "Forest", "Forest", "Forest", "Forest", "Forest"]);
    let loot = put(&mut g, 0, "Loot, Exuberant Explorer");
    begin(&mut g);
    let top = *g.players[0].library.last().unwrap(); // an Aegis Turtle, mana value 1
    act(&mut g, Action::Activate(loot, 0));
    resolve_all(&mut g, &[Action::Pick(top)]);
    assert_eq!(g.obj(top).zone, Zone::Battlefield);
}

#[test]
fn perforating_artist_taxes_after_an_attack() {
    let mut g = table();
    let artist = put(&mut g, 0, "Perforating Artist");
    let theirs = put(&mut g, 1, "Savannah Lions");
    begin(&mut g);
    attack_with(&mut g, &[artist]);
    pass_to(&mut g, Step::End);
    resolve_all(&mut g, &[Action::Pick(theirs)]);
    assert_eq!(
        g.obj(theirs).zone,
        Zone::Graveyard,
        "they sacrificed instead of losing life"
    );
    assert_eq!(g.players[1].life, 17, "the combat damage only");
}

#[test]
fn consuming_aberration_counts_and_mills() {
    let mut g = table();
    let ab = put(&mut g, 0, "Consuming Aberration");
    for _ in 0..3 {
        g.add_card(1, id("Stab"), Zone::Graveyard);
    }
    g.add_card(1, id("Forest"), Zone::Library);
    g.players[1].library.rotate_right(1); // a land at the bottom
    let c = hand(&mut g, 0, "Boltwave");
    begin(&mut g);
    assert_eq!(g.power(ab), 3);
    play(&mut g, c, false, &[], &[]);
    assert!(
        g.players[1].library.is_empty(),
        "revealed until the land at the bottom"
    );
    assert_eq!(g.power(ab), 14);
}

#[test]
fn elenda_grows_with_life_and_ignores_instants() {
    let mut g = table();
    let e = put(&mut g, 0, "Elenda, Saint of Dusk");
    let stab = hand(&mut g, 1, "Stab");
    begin(&mut g);
    assert_eq!(g.power(e), 4);
    g.players[0].life = 25;
    assert_eq!(g.power(e), 5);
    assert!(g.keywords(e).has(Keywords::MENACE));
    g.players[0].life = 30;
    assert_eq!(g.power(e), 10);
    act(&mut g, Action::PassPriority);
    assert!(
        !g.legal_actions().contains(&Action::Cast(stab)),
        "its only target has hexproof from instants"
    );
}
