//! Tarkir: Dragonstorm lane B cards: each script's behavior, on a Tarkir table. Log: docs/tdm-lane-b.md.

mod common;
mod tdm_common;

use bolt_core::{Action, Color, Keywords, Target, Zone};
use common::{act, begin, obj, play, resolve_all};
use tdm_common::*;

#[test]
fn heritage_reclamation_destroys_an_artifact_or_enchantment_or_exiles_a_card_and_draws() {
    let mut g = table();
    let globe = put(&mut g, 1, "Dragonstorm Globe");
    let snare = put(&mut g, 1, "Stormplain Detainment");
    let elder = put(&mut g, 1, "Humbling Elder");
    let dead = g.add_card(1, tid("Iceridge Serpent"), Zone::Graveyard);
    begin(&mut g);

    let c = hand(&mut g, 0, "Heritage Reclamation");
    act(&mut g, Action::CastModal(c, 0));
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Target(obj(elder))), "artifacts only");
    assert!(!legal.contains(&Action::Target(obj(snare))), "artifacts only");
    act(&mut g, Action::Target(obj(globe)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(globe).zone, Zone::Graveyard);

    let c = hand(&mut g, 0, "Heritage Reclamation");
    act(&mut g, Action::CastModal(c, 1));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(elder))),
        "enchantments only"
    );
    act(&mut g, Action::Target(obj(snare)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(snare).zone, Zone::Graveyard);

    let c = hand(&mut g, 0, "Heritage Reclamation");
    let n = g.players[0].hand.len();
    act(&mut g, Action::CastModal(c, 2));
    act(&mut g, Action::Target(obj(dead)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(dead).zone, Zone::Exile);
    assert_eq!(g.players[0].hand.len(), n, "cast one, drew one");

    // "Up to one": no target, still draws.
    let c = hand(&mut g, 0, "Heritage Reclamation");
    let n = g.players[0].hand.len();
    act(&mut g, Action::CastModal(c, 2));
    act(&mut g, Action::No);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n);
}

#[test]
fn cruel_truths_surveils_two_draws_two_and_costs_two_life() {
    let mut g = table();
    let c = hand(&mut g, 0, "Cruel Truths");
    begin(&mut g);
    let lib = &g.players[0].library;
    let (top, second) = (lib[lib.len() - 1], lib[lib.len() - 2]);
    let (n, life) = (g.players[0].hand.len(), g.players[0].life);
    play(&mut g, c, false, &[], &[Action::Yes, Action::No]);
    assert_eq!(g.obj(top).zone, Zone::Graveyard, "surveilled away");
    assert_eq!(g.obj(second).zone, Zone::Hand, "kept, then drawn");
    assert_eq!(g.players[0].hand.len(), n - 1 + 2);
    assert_eq!(g.players[0].life, life - 2);
}

#[test]
fn aleshas_legacy_gives_deathtouch_and_indestructible() {
    let mut g = table();
    let mine = put(&mut g, 0, "Humbling Elder");
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    begin(&mut g);
    let c = hand(&mut g, 0, "Alesha's Legacy");
    act(&mut g, Action::Cast(c));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(theirs))),
        "a creature you control"
    );
    act(&mut g, Action::Target(obj(mine)));
    resolve_all(&mut g, &[]);
    let k = g.keywords(mine);
    assert!(k.has(Keywords::DEATHTOUCH) && k.has(Keywords::INDESTRUCTIBLE));
    let prey = hand(&mut g, 0, "Dragon's Prey");
    play(&mut g, prey, false, &[obj(mine)], &[]);
    assert_eq!(g.obj(mine).zone, Zone::Battlefield, "indestructible");
}

#[test]
fn worthy_cost_sacrifices_a_creature_to_exile_one() {
    let mut g = table();
    let fodder = put(&mut g, 0, "Humbling Elder");
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    let c = hand(&mut g, 0, "Worthy Cost");
    begin(&mut g);
    assert!(
        !g.legal_actions().contains(&Action::Cast(c)),
        "the sacrifice isn't optional"
    );
    act(&mut g, Action::CastWith(c, fodder));
    act(&mut g, Action::Target(obj(theirs)));
    assert_eq!(g.obj(fodder).zone, Zone::Graveyard, "sacrificed as it's cast");
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Exile);
}

#[test]
fn bewildering_blizzard_draws_three_and_weakens_opposing_creatures() {
    let mut g = table();
    let mine = put(&mut g, 0, "Iceridge Serpent");
    let theirs = put(&mut g, 1, "Dusyut Earthcarver");
    let c = hand(&mut g, 0, "Bewildering Blizzard");
    begin(&mut g);
    let n = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), n - 1 + 3);
    assert_eq!((g.power(theirs), g.toughness(theirs)), (1, 4));
    assert_eq!(g.power(mine), 3, "only opponents' creatures");
}

#[test]
fn tri_lands_enter_tapped_and_tap_for_three_colors() {
    use Color::*;
    for (name, colors) in [
        ("Frontier Bivouac", [Green, Blue, Red]),
        ("Mystic Monastery", [Blue, Red, White]),
        ("Nomad Outpost", [Red, White, Black]),
        ("Sandsteppe Citadel", [White, Black, Green]),
    ] {
        let mut g = tdm_lands(&[]);
        let land = hand(&mut g, 0, name);
        begin(&mut g);
        act(&mut g, Action::PlayLand(land));
        resolve_all(&mut g, &[]);
        assert_eq!(g.obj(land).zone, Zone::Battlefield);
        assert!(g.obj(land).tapped, "{name} enters tapped");
        let source = g.def_of(land).mana.expect("a mana source");
        let mut want = [false; 5];
        for c in colors {
            want[c as usize] = true;
        }
        assert_eq!(source.colors, want, "{name}");
        assert!(!source.colorless);
        assert!(g.mana_sources(0).is_empty(), "{name} is tapped");
        g.start_turn(5, 0);
        while g.step != bolt_core::Step::Main1 {
            act(&mut g, Action::PassPriority);
        }
        assert_eq!(g.mana_sources(0).len(), 1, "{name} untaps and taps for one mana");
    }
}

#[test]
fn knockout_maneuver_counters_then_bites() {
    let mut g = table();
    let mine = put(&mut g, 0, "Iceridge Serpent"); // 3/3, then 4/4
    let theirs = put(&mut g, 1, "Dusyut Earthcarver"); // 4/4
    let c = hand(&mut g, 0, "Knockout Maneuver");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(theirs))),
        "first: yours"
    );
    act(&mut g, Action::Target(obj(mine)));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(mine))),
        "second: theirs"
    );
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(mine).counters, 1);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "4 damage after the counter");
    assert_eq!(g.obj(mine).damage, 0, "a bite, not a fight");
}

#[test]
fn shocking_sharpshooter_pings_when_another_creature_enters() {
    let mut g = table();
    let shooter = hand(&mut g, 0, "Shocking Sharpshooter");
    begin(&mut g);
    play(&mut g, shooter, false, &[], &[]);
    assert!(g.keywords(shooter).has(Keywords::REACH));
    assert_eq!(g.players[1].life, 20, "not for itself");
    let elder = hand(&mut g, 0, "Humbling Elder");
    act(&mut g, Action::Cast(elder));
    let mut asked = false;
    while !g.items.is_empty() || matches!(g.pending, bolt_core::Pending::ChooseTarget(_)) {
        if matches!(g.pending, bolt_core::Pending::ChooseTarget(_)) {
            assert!(
                !g.legal_actions().contains(&Action::Target(Target::Player(0))),
                "an opponent"
            );
            act(&mut g, Action::Target(Target::Player(1)));
            asked = true;
        } else {
            act(&mut g, Action::PassPriority);
        }
    }
    assert!(asked);
    assert_eq!(g.players[1].life, 19);
    // An opponent's creature doesn't trigger it.
    g.add_card(1, tid("Humbling Elder"), Zone::Battlefield);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 19);
}

#[test]
fn auroral_procession_returns_a_card_from_your_graveyard() {
    let mut g = table();
    let mine = g.add_card(0, tid("Cruel Truths"), Zone::Graveyard);
    let theirs = g.add_card(1, tid("Iceridge Serpent"), Zone::Graveyard);
    let c = hand(&mut g, 0, "Auroral Procession");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(theirs))),
        "your graveyard"
    );
    act(&mut g, Action::Target(obj(mine)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(mine).zone, Zone::Hand);
    assert!(g.players[0].hand.contains(&mine));
}

#[test]
fn gurmag_rakshasa_shrinks_theirs_and_grows_yours() {
    let mut g = table();
    let theirs = put(&mut g, 1, "Humbling Elder"); // 1/2: dies
    let mine = put(&mut g, 0, "Iceridge Serpent");
    let c = hand(&mut g, 0, "Gurmag Rakshasa");
    begin(&mut g);
    play(
        &mut g,
        c,
        false,
        &[],
        &[Action::Target(obj(theirs)), Action::Target(obj(mine))],
    );
    assert!(g.keywords(c).has(Keywords::MENACE));
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!((g.power(mine), g.toughness(mine)), (5, 5));
}

#[test]
fn skirmish_rhino_drains_two() {
    let mut g = table();
    let c = hand(&mut g, 0, "Skirmish Rhino");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert!(g.keywords(c).has(Keywords::TRAMPLE));
    assert_eq!((g.players[0].life, g.players[1].life), (22, 18));
}

#[test]
fn inspired_ultimatum_gains_five_deals_five_draws_five() {
    let mut g = table();
    let theirs = put(&mut g, 1, "Dusyut Earthcarver");
    let c = hand(&mut g, 0, "Inspired Ultimatum");
    begin(&mut g);
    let n = g.players[0].hand.len();
    play(&mut g, c, false, &[Target::Player(0), obj(theirs)], &[]);
    assert_eq!(g.players[0].life, 25);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n - 1 + 5);

    let mut g = table();
    let c = hand(&mut g, 0, "Inspired Ultimatum");
    begin(&mut g);
    play(&mut g, c, false, &[Target::Player(1), Target::Player(1)], &[]);
    assert_eq!(g.players[1].life, 20, "gained 5, took 5");
}
