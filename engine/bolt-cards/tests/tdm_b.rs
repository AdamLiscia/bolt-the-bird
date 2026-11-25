//! Tarkir: Dragonstorm lane B cards: each script's behavior, on a Tarkir table. Log: docs/tdm-lane-b.md.

mod common;
mod tdm_common;

use bolt_core::{Action, Color, Filter, Game, Keywords, ObjId, Pending, Step, Target, Zone};
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

// ---- Batch 2: slow lands, becomes tapped, a creature with a counter, X = creatures ----

/// Player 0's turn 5 (nothing summoning sick), at the declare-attackers step.
fn to_attacks(g: &mut Game) {
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
}

/// Declares these attackers, then resolves attack triggers with `answers`.
fn attack(g: &mut Game, attackers: &[ObjId], answers: &[Action]) {
    for &a in attackers {
        act(g, Action::Attack(a));
    }
    act(g, Action::FinishAttacks);
    resolve_all(g, answers);
}

#[test]
fn slow_lands_enter_tapped_unless_you_control_one_of_two_types() {
    for (name, needs, color) in [
        ("Cori Mountain Monastery", "Island", Color::Red),
        ("Kishla Village", "Swamp", Color::Green),
    ] {
        let mut g = tdm_lands(&["Forest"]);
        let land = hand(&mut g, 0, name);
        begin(&mut g);
        act(&mut g, Action::PlayLand(land));
        assert!(g.obj(land).tapped, "{name}: no {needs}");
        let mut want = [false; 5];
        want[color as usize] = true;
        assert_eq!(g.def_of(land).mana.unwrap().colors, want);

        let mut g = tdm_lands(&[needs]);
        let land = hand(&mut g, 0, name);
        begin(&mut g);
        act(&mut g, Action::PlayLand(land));
        assert!(!g.obj(land).tapped, "{name}: with a {needs}");
    }
    // The other type works too.
    let mut g = tdm_lands(&["Plains"]);
    let land = hand(&mut g, 0, "Cori Mountain Monastery");
    begin(&mut g);
    act(&mut g, Action::PlayLand(land));
    assert!(!g.obj(land).tapped);
}

#[test]
fn cori_mountain_monastery_exiles_the_top_card_to_play() {
    let mut g = table();
    let land = put(&mut g, 0, "Cori Mountain Monastery");
    begin(&mut g);
    let top = *g.players[0].library.last().unwrap();
    act(&mut g, Action::Activate(land, 0));
    resolve_all(&mut g, &[Action::Pick(top)]);
    assert!(g.obj(land).tapped);
    assert_eq!(g.obj(top).zone, Zone::Exile);
    assert!(g.obj(top).playable.is_some());
}

#[test]
fn kishla_village_surveils_two() {
    let mut g = table();
    let land = put(&mut g, 0, "Kishla Village");
    begin(&mut g);
    let lib = &g.players[0].library;
    let (top, second) = (lib[lib.len() - 1], lib[lib.len() - 2]);
    act(&mut g, Action::Activate(land, 0));
    resolve_all(&mut g, &[Action::Yes, Action::No]);
    assert_eq!(g.obj(top).zone, Zone::Graveyard);
    assert_eq!(*g.players[0].library.last().unwrap(), second);
}

#[test]
fn rescue_leopard_loots_when_it_becomes_tapped() {
    let mut g = table();
    let leopard = put(&mut g, 0, "Rescue Leopard");
    to_attacks(&mut g);
    let junk = hand(&mut g, 0, "Cruel Truths");
    let n = g.players[0].hand.len();
    attack(&mut g, &[leopard], &[Action::Discard(junk)]);
    assert_eq!(g.obj(junk).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n, "discarded one, drew one");

    // Tapped by an opponent's effect: it still triggers, and the loot is optional.
    let mut g = table();
    let leopard = put(&mut g, 0, "Rescue Leopard");
    let sage = hand(&mut g, 1, "Constrictor Sage");
    g.start_turn(4, 1);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Cast(sage));
    let n = g.players[0].hand.len();
    hand(&mut g, 0, "Cruel Truths");
    resolve_all(&mut g, &[Action::Target(obj(leopard)), Action::No]);
    assert!(g.obj(leopard).tapped);
    assert_eq!(g.players[0].hand.len(), n + 1, "declined");
}

#[test]
fn rescue_leopard_doesnt_trigger_entering_tapped_or_already_tapped() {
    let mut g = table();
    let leopard = hand(&mut g, 0, "Rescue Leopard");
    begin(&mut g);
    play(&mut g, leopard, false, &[], &[]);
    assert!(
        g.items.is_empty() && !matches!(g.pending, Pending::Decide(_)),
        "casting it taps lands, not it"
    );
}

#[test]
fn traveling_botanist_takes_a_land_or_may_bin_a_nonland() {
    let mut g = table();
    let botanist = put(&mut g, 0, "Traveling Botanist");
    to_attacks(&mut g);
    let forest = g.add_card(0, tid("Forest"), Zone::Library);
    attack(&mut g, &[botanist], &[Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Hand);

    let mut g = table();
    let botanist = put(&mut g, 0, "Traveling Botanist");
    to_attacks(&mut g);
    let spell = g.add_card(0, tid("Cruel Truths"), Zone::Library);
    act(&mut g, Action::Attack(botanist));
    act(&mut g, Action::FinishAttacks);
    while g.decision.is_none() {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Pick(spell)),
        "only a land goes to hand"
    );
    resolve_all(&mut g, &[Action::No, Action::Yes]);
    assert_eq!(g.obj(spell).zone, Zone::Graveyard);

    // A land not taken may stay on top.
    let mut g = table();
    let botanist = put(&mut g, 0, "Traveling Botanist");
    to_attacks(&mut g);
    let forest = g.add_card(0, tid("Forest"), Zone::Library);
    attack(&mut g, &[botanist], &[Action::No, Action::No]);
    assert_eq!(*g.players[0].library.last().unwrap(), forest);
}

#[test]
fn trade_route_envoy_draws_with_a_countered_creature_or_grows() {
    let mut g = table();
    let first = hand(&mut g, 0, "Trade Route Envoy");
    let second = hand(&mut g, 0, "Trade Route Envoy");
    begin(&mut g);
    let n = g.players[0].hand.len();
    play(&mut g, first, false, &[], &[]);
    assert_eq!(g.obj(first).counters, 1, "no creature with a counter: a counter");
    assert_eq!(g.players[0].hand.len(), n - 1, "no draw");
    play(&mut g, second, false, &[], &[]);
    assert_eq!(
        g.obj(second).counters,
        0,
        "the first one has a counter: draw instead"
    );
    assert_eq!(g.players[0].hand.len(), n - 1, "cast one, drew one");
}

#[test]
fn trade_route_envoy_counts_any_kind_of_counter_but_only_yours() {
    // A stun counter on an opponent's creature doesn't count.
    let mut g = table();
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    let sage = hand(&mut g, 0, "Constrictor Sage");
    let envoy = hand(&mut g, 0, "Trade Route Envoy");
    begin(&mut g);
    play(&mut g, sage, false, &[], &[Action::Target(obj(theirs))]);
    play(&mut g, envoy, false, &[], &[]);
    assert_eq!(g.obj(envoy).counters, 1);

    // A keyword counter on yours does.
    let mut g = table();
    let mine = put(&mut g, 0, "Iceridge Serpent");
    let assistant = g.add_card(0, tid("Alchemist's Assistant"), Zone::Graveyard);
    let envoy = hand(&mut g, 0, "Trade Route Envoy");
    begin(&mut g);
    act(&mut g, Action::Activate(assistant, 0));
    resolve_all(&mut g, &[Action::Target(obj(mine))]);
    assert!(g.keywords(mine).has(Keywords::LIFELINK));
    let n = g.players[0].hand.len();
    play(&mut g, envoy, false, &[], &[]);
    assert_eq!(g.obj(envoy).counters, 0);
    assert_eq!(g.players[0].hand.len(), n, "cast one, drew one");
}

#[test]
fn delta_bloodflies_drains_on_attack_only_with_a_countered_creature() {
    let mut g = table();
    let flies = put(&mut g, 0, "Delta Bloodflies");
    assert!(g.keywords(flies).has(Keywords::FLYING));
    to_attacks(&mut g);
    attack(&mut g, &[flies], &[]);
    assert_eq!(g.players[1].life, 20);

    let mut g = table();
    let flies = put(&mut g, 0, "Delta Bloodflies");
    let envoy = hand(&mut g, 0, "Trade Route Envoy");
    begin(&mut g);
    play(&mut g, envoy, false, &[], &[]);
    to_attacks(&mut g);
    attack(&mut g, &[flies], &[]);
    assert_eq!(g.players[1].life, 19);
}

#[test]
fn formation_breaker_cant_be_blocked_by_smaller_creatures_and_grows_with_counters() {
    let mut g = table();
    let breaker = put(&mut g, 0, "Formation Breaker");
    let small = put(&mut g, 1, "Humbling Elder"); // power 1
    let big = put(&mut g, 1, "Iceridge Serpent"); // power 3
    assert_eq!((g.power(breaker), g.toughness(breaker)), (2, 1));
    to_attacks(&mut g);
    attack(&mut g, &[breaker], &[]);
    while g.pending != Pending::DeclareBlockers(1) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Block {
        blocker: small,
        attacker: breaker
    }));
    assert!(legal.contains(&Action::Block {
        blocker: big,
        attacker: breaker
    }));

    let mut g = table();
    let breaker = put(&mut g, 0, "Formation Breaker");
    let envoy = hand(&mut g, 0, "Trade Route Envoy");
    begin(&mut g);
    play(&mut g, envoy, false, &[], &[]);
    assert_eq!((g.power(breaker), g.toughness(breaker)), (3, 3));
}

#[test]
fn coordinated_maneuver_deals_damage_per_creature_or_destroys_an_enchantment() {
    let mut g = table();
    put(&mut g, 0, "Humbling Elder");
    put(&mut g, 0, "Humbling Elder");
    put(&mut g, 0, "Humbling Elder");
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    let snare = put(&mut g, 1, "Stormplain Detainment");
    begin(&mut g);
    let c = hand(&mut g, 0, "Coordinated Maneuver");
    act(&mut g, Action::CastModal(c, 0));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "3 damage");

    let c = hand(&mut g, 0, "Coordinated Maneuver");
    act(&mut g, Action::CastModal(c, 1));
    act(&mut g, Action::Target(obj(snare)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(snare).zone, Zone::Graveyard);

    let mut g = table();
    put(&mut g, 0, "Humbling Elder");
    put(&mut g, 0, "Humbling Elder");
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    begin(&mut g);
    let c = hand(&mut g, 0, "Coordinated Maneuver");
    act(&mut g, Action::CastModal(c, 0));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).damage, 2, "two creatures: 2 damage");
}

#[test]
fn frontline_rush_makes_goblins_or_pumps_per_creature() {
    let mut g = table();
    let mine = put(&mut g, 0, "Humbling Elder");
    begin(&mut g);
    let c = hand(&mut g, 0, "Frontline Rush");
    act(&mut g, Action::CastModal(c, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Goblin").len(), 2);

    let c = hand(&mut g, 0, "Frontline Rush");
    act(&mut g, Action::CastModal(c, 1));
    act(&mut g, Action::Target(obj(mine)));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.power(mine), g.toughness(mine)),
        (1 + 3, 2 + 3),
        "three creatures"
    );
}

#[test]
fn marshal_of_the_lost_pumps_by_the_number_of_attackers() {
    let mut g = table();
    let marshal = put(&mut g, 0, "Marshal of the Lost");
    let elder = put(&mut g, 0, "Humbling Elder");
    put(&mut g, 0, "Iceridge Serpent"); // stays home
    assert!(g.keywords(marshal).has(Keywords::DEATHTOUCH));
    to_attacks(&mut g);
    attack(&mut g, &[marshal, elder], &[Action::Target(obj(elder))]);
    assert_eq!((g.power(elder), g.toughness(elder)), (3, 4));
}

#[test]
fn craterhoof_behemoth_pumps_and_tramples_your_team() {
    let mut g = table();
    let elder = put(&mut g, 0, "Humbling Elder");
    put(&mut g, 0, "Iceridge Serpent");
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    let hoof = hand(&mut g, 0, "Craterhoof Behemoth");
    begin(&mut g);
    play(&mut g, hoof, false, &[], &[]);
    assert!(g.keywords(hoof).has(Keywords::HASTE));
    assert_eq!((g.power(elder), g.toughness(elder)), (4, 5), "three creatures");
    assert_eq!(g.power(hoof), 8);
    assert!(g.keywords(elder).has(Keywords::TRAMPLE));
    assert_eq!(g.power(theirs), 3, "yours only");
    assert!(!g.keywords(theirs).has(Keywords::TRAMPLE));
}

#[test]
fn duty_beyond_death_protects_and_grows_your_creatures() {
    let mut g = table();
    let fodder = put(&mut g, 0, "Humbling Elder");
    let a = put(&mut g, 0, "Iceridge Serpent");
    let b = put(&mut g, 0, "Dusyut Earthcarver");
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    let c = hand(&mut g, 0, "Duty Beyond Death");
    begin(&mut g);
    act(&mut g, Action::CastWith(c, fodder));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(fodder).zone, Zone::Graveyard);
    for x in [a, b] {
        assert_eq!(g.obj(x).counters, 1);
        assert!(g.keywords(x).has(Keywords::INDESTRUCTIBLE));
    }
    assert_eq!(g.obj(theirs).counters, 0);
    let prey = hand(&mut g, 0, "Dragon's Prey");
    play(&mut g, prey, false, &[obj(a)], &[]);
    assert_eq!(g.obj(a).zone, Zone::Battlefield);
}

// ---- Batch 3: if you cast it, by name, can't be countered, hexproof until damage ----

/// After attacks are declared: no blocks, then pass until a decision comes up or the turn's combat
/// is over.
fn combat_until_decision(g: &mut Game) {
    while g.step != Step::End && !matches!(g.pending, Pending::Decide(_)) {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn wingspan_stride_pumps_gives_flying_and_returns_to_hand() {
    let mut g = table();
    let elder = put(&mut g, 0, "Humbling Elder");
    let aura = hand(&mut g, 0, "Wingspan Stride");
    begin(&mut g);
    play(&mut g, aura, false, &[obj(elder)], &[]);
    assert_eq!((g.power(elder), g.toughness(elder)), (2, 3));
    assert!(g.keywords(elder).has(Keywords::FLYING));
    act(&mut g, Action::Activate(aura, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(aura).zone, Zone::Hand);
    assert_eq!((g.power(elder), g.toughness(elder)), (1, 2));
    assert!(!g.keywords(elder).has(Keywords::FLYING));
}

#[test]
fn mistrise_village_makes_the_next_spell_uncounterable() {
    let mut g = tdm_lands(&["Forest"]);
    let land = hand(&mut g, 0, "Mistrise Village");
    begin(&mut g);
    act(&mut g, Action::PlayLand(land));
    assert!(!g.obj(land).tapped, "a Forest");

    let mut g = table();
    let village = put(&mut g, 0, "Mistrise Village");
    let first = hand(&mut g, 0, "Cruel Truths");
    let second = hand(&mut g, 0, "Cruel Truths");
    let a = hand(&mut g, 1, "Riverwalk Technique");
    let b = hand(&mut g, 1, "Riverwalk Technique");
    begin(&mut g);
    act(&mut g, Action::Activate(village, 0));
    resolve_all(&mut g, &[]);
    for (spell, counter, survives) in [(first, a, true), (second, b, false)] {
        act(&mut g, Action::Cast(spell));
        act(&mut g, Action::PassPriority);
        act(&mut g, Action::CastModal(counter, 1));
        act(&mut g, Action::Target(obj(spell)));
        let answers: &[Action] = if survives { &[Action::No, Action::No] } else { &[] };
        resolve_all(&mut g, answers);
        assert_eq!(g.obj(counter).zone, Zone::Graveyard);
        assert_eq!(g.obj(spell).zone, Zone::Graveyard);
        if survives {
            assert_eq!(g.players[0].life, 18, "it resolved");
        } else {
            assert_eq!(g.players[0].life, 18, "countered: no more life lost");
        }
    }
}

#[test]
fn tempest_hawk_fetches_another_tempest_hawk() {
    let mut g = table();
    let hawk = put(&mut g, 0, "Tempest Hawk");
    to_attacks(&mut g);
    let other = g.add_card(0, tid("Tempest Hawk"), Zone::Library);
    act(&mut g, Action::Attack(hawk));
    act(&mut g, Action::FinishAttacks);
    combat_until_decision(&mut g);
    assert_eq!(g.players[1].life, 18);
    let legal = g.legal_actions();
    assert!(
        legal.contains(&Action::Yes) && legal.contains(&Action::No),
        "you may"
    );
    act(&mut g, Action::Yes);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(other)));
    assert!(
        legal.iter().filter(|a| matches!(a, Action::Pick(_))).count() == 1,
        "only a Tempest Hawk"
    );
    resolve_all(&mut g, &[Action::Pick(other)]);
    assert_eq!(g.obj(other).zone, Zone::Hand);
}

#[test]
fn dragonstorm_forecaster_finds_a_globe_or_a_boulderborn_dragon() {
    let mut g = table();
    let f = put(&mut g, 0, "Dragonstorm Forecaster");
    let globe = g.add_card(0, tid("Dragonstorm Globe"), Zone::Library);
    let dragon = g.add_card(0, tid("Boulderborn Dragon"), Zone::Library);
    let serpent = g.add_card(0, tid("Iceridge Serpent"), Zone::Library);
    main_turn_5(&mut g);
    act(&mut g, Action::Activate(f, 0));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(globe)) && legal.contains(&Action::Pick(dragon)));
    assert!(!legal.contains(&Action::Pick(serpent)), "named cards only");
    resolve_all(&mut g, &[Action::Pick(dragon)]);
    assert_eq!(g.obj(dragon).zone, Zone::Hand);
    assert!(g.obj(f).tapped);
}

fn main_turn_5(g: &mut Game) {
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(g, Action::PassPriority);
    }
}

#[test]
fn iridescent_tiger_adds_five_colors_only_if_cast() {
    let mut g = table();
    let tiger = hand(&mut g, 0, "Iridescent Tiger");
    begin(&mut g);
    play(&mut g, tiger, false, &[], &[]);
    let pool = &g.players[0].pool;
    assert_eq!(pool.len(), 5);
    for c in 0..5 {
        assert!(pool.iter().any(|m| m.colors[c]), "color {c}");
    }

    let mut g = table();
    let tiger = hand(&mut g, 0, "Iridescent Tiger");
    begin(&mut g);
    g.move_card(tiger, Zone::Battlefield);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(tiger).zone, Zone::Battlefield);
    assert!(g.players[0].pool.is_empty(), "put onto the battlefield, not cast");
}

#[test]
fn yathan_roadwatcher_mills_four_and_reanimates_a_cheap_creature_if_cast() {
    let mut g = table();
    let elder = g.add_card(0, tid("Humbling Elder"), Zone::Graveyard);
    let big = g.add_card(0, tid("Dusyut Earthcarver"), Zone::Graveyard); // mana value 6
    let yathan = hand(&mut g, 0, "Yathan Roadwatcher");
    begin(&mut g);
    let lib = g.players[0].library.len();
    act(&mut g, Action::Cast(yathan));
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[0].library.len(), lib - 4, "milled four first");
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(big))),
        "mana value 3 or less"
    );
    act(&mut g, Action::Target(obj(elder)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elder).zone, Zone::Battlefield);

    let mut g = table();
    let yathan = hand(&mut g, 0, "Yathan Roadwatcher");
    begin(&mut g);
    let lib = g.players[0].library.len();
    g.move_card(yathan, Zone::Battlefield);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].library.len(), lib, "not cast: no mill");
}

#[test]
fn unrooted_ancestor_sacrifices_for_indestructible_and_taps() {
    let mut g = table();
    let anc = put(&mut g, 0, "Unrooted Ancestor");
    let fodder = put(&mut g, 0, "Humbling Elder");
    assert!(g.keywords(anc).has(Keywords::FLASH));
    begin(&mut g);
    act(&mut g, Action::ActivateWith(anc, 0, fodder));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(fodder).zone, Zone::Graveyard);
    assert!(g.keywords(anc).has(Keywords::INDESTRUCTIBLE));
    assert!(g.obj(anc).tapped);
}

#[test]
fn strategic_betrayal_exiles_their_chosen_creature_and_graveyard() {
    let mut g = table();
    let keep = put(&mut g, 1, "Iceridge Serpent");
    let lose = put(&mut g, 1, "Humbling Elder");
    let dead = g.add_card(1, tid("Cruel Truths"), Zone::Graveyard);
    let mine = g.add_card(0, tid("Cruel Truths"), Zone::Graveyard);
    let c = hand(&mut g, 0, "Strategic Betrayal");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    assert!(
        !g.legal_actions().contains(&Action::Target(Target::Player(0))),
        "an opponent"
    );
    act(&mut g, Action::Target(Target::Player(1)));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.pending, Pending::Decide(1), "they choose");
    resolve_all(&mut g, &[Action::Pick(lose)]);
    assert_eq!(g.obj(lose).zone, Zone::Exile);
    assert_eq!(g.obj(keep).zone, Zone::Battlefield);
    assert_eq!(g.obj(dead).zone, Zone::Exile);
    assert!(g.players[1].graveyard.is_empty());
    assert_eq!(g.obj(mine).zone, Zone::Graveyard);
}

#[test]
fn lie_in_wait_returns_a_creature_card_and_deals_its_power() {
    let mut g = table();
    let card = g.add_card(0, tid("Dusyut Earthcarver"), Zone::Graveyard); // 4 power
    let theirs = put(&mut g, 1, "Dusyut Earthcarver"); // 4/4
    let c = hand(&mut g, 0, "Lie in Wait");
    begin(&mut g);
    play(&mut g, c, false, &[obj(card), obj(theirs)], &[]);
    assert_eq!(g.obj(card).zone, Zone::Hand);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
}

#[test]
fn effortless_master_enters_with_counters_after_two_spells() {
    let mut g = table();
    let m = hand(&mut g, 0, "Effortless Master");
    begin(&mut g);
    play(&mut g, m, false, &[], &[]);
    assert_eq!(g.obj(m).counters, 0, "the first spell this turn");
    assert!(g.keywords(m).has(Keywords::MENACE) && g.keywords(m).has(Keywords::VIGILANCE));

    let mut g = table();
    let m = hand(&mut g, 0, "Effortless Master");
    let first = hand(&mut g, 0, "Cruel Truths");
    begin(&mut g);
    play(&mut g, first, false, &[], &[Action::No, Action::No]);
    play(&mut g, m, false, &[], &[]);
    assert_eq!(g.obj(m).counters, 2);
    assert_eq!(g.power(m), 6);
}

#[test]
fn karakyk_guardian_has_hexproof_until_it_deals_damage() {
    let mut g = table();
    let k = put(&mut g, 0, "Karakyk Guardian");
    let theirs = Filter::CREATURE;
    assert!(g.keywords(k).has(Keywords::HEXPROOF));
    assert!(!g.target_ok(&theirs, 1, obj(k)), "opponents can't target it");
    assert!(g.target_ok(&theirs, 0, obj(k)), "you can");
    to_attacks(&mut g);
    act(&mut g, Action::Attack(k));
    act(&mut g, Action::FinishAttacks);
    combat_until_decision(&mut g);
    assert_eq!(g.players[1].life, 14);
    assert!(!g.keywords(k).has(Keywords::HEXPROOF), "it has dealt damage");
    assert!(g.target_ok(&theirs, 1, obj(k)));
}

#[test]
fn clarion_conqueror_stops_activated_abilities_of_artifacts_and_creatures() {
    let mut g = table();
    let f = put(&mut g, 0, "Dragonstorm Forecaster");
    let globe = put(&mut g, 0, "Dragonstorm Globe");
    let village = put(&mut g, 0, "Kishla Village");
    let assistant = g.add_card(0, tid("Alchemist's Assistant"), Zone::Graveyard);
    let target = put(&mut g, 0, "Humbling Elder");
    main_turn_5(&mut g);
    assert!(g.legal_actions().contains(&Action::Activate(f, 0)));
    assert!(g.mana_sources(0).iter().any(|&(id, _)| id == globe));
    put(&mut g, 1, "Clarion Conqueror"); // an opponent's works too
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Activate(f, 0)), "a creature's ability");
    assert!(
        !g.mana_sources(0).iter().any(|&(id, _)| id == globe),
        "an artifact's mana ability"
    );
    assert!(
        legal.contains(&Action::Activate(village, 0)),
        "lands aren't affected"
    );
    assert!(
        legal.contains(&Action::Activate(assistant, 0)),
        "nor cards in graveyards"
    );
    let _ = target;
}

// ---- Batch 4: first counters each turn, delayed attack trigger, one or both, more ----

/// Pass (no attacks, no blocks) until the next turn has begun.
fn to_next_turn(g: &mut Game) {
    let turn = g.turn;
    while g.turn == turn {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn stalwart_successor_adds_a_counter_the_first_time_each_turn() {
    let mut g = table();
    let s = put(&mut g, 0, "Stalwart Successor");
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    let envoy = hand(&mut g, 0, "Trade Route Envoy");
    let ko = hand(&mut g, 0, "Knockout Maneuver");
    assert!(g.keywords(s).has(Keywords::MENACE));
    begin(&mut g);
    play(&mut g, envoy, false, &[], &[]);
    assert_eq!(g.obj(envoy).counters, 2, "its own counter, then the Successor's");
    play(&mut g, ko, false, &[obj(envoy), obj(theirs)], &[]);
    assert_eq!(g.obj(envoy).counters, 3, "not the first time this turn");
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "7 damage");
    // An opponent's creature getting counters (a stun counter) doesn't count.
    let other = put(&mut g, 1, "Iceridge Serpent");
    let sage = hand(&mut g, 0, "Constrictor Sage");
    play(&mut g, sage, false, &[], &[Action::Target(obj(other))]);
    assert_eq!(g.obj(other).stun, 1);
    assert_eq!(g.obj(other).counters, 0);
}

#[test]
fn stalwart_successor_counts_entering_with_counters_and_stun_counters() {
    // A stun counter on your own creature counts: Ambling Stormshell stuns itself.
    let mut g = table();
    put(&mut g, 0, "Stalwart Successor");
    let shell = put(&mut g, 0, "Ambling Stormshell");
    to_attacks(&mut g);
    attack(&mut g, &[shell], &[]);
    assert_eq!((g.obj(shell).stun, g.obj(shell).counters), (3, 1));

    let mut g = table();
    put(&mut g, 0, "Stalwart Successor");
    let m = hand(&mut g, 0, "Effortless Master");
    let first = hand(&mut g, 0, "Cruel Truths");
    begin(&mut g);
    play(&mut g, first, false, &[], &[Action::No, Action::No]);
    play(&mut g, m, false, &[], &[]);
    assert_eq!(g.obj(m).counters, 3, "entered with two, then one more (CR 122.6)");
}

#[test]
fn embermouth_sentinel_puts_a_basic_on_top_or_onto_the_battlefield_with_a_dragon() {
    let mut g = table();
    let c = hand(&mut g, 0, "Embermouth Sentinel");
    begin(&mut g);
    let forest = g.add_card(0, tid("Forest"), Zone::Library);
    g.players[0].library.rotate_right(1);
    play(&mut g, c, false, &[], &[Action::Yes, Action::Pick(forest)]);
    assert_eq!(*g.players[0].library.last().unwrap(), forest, "on top");

    let mut g = table();
    put(&mut g, 0, "Boulderborn Dragon");
    let c = hand(&mut g, 0, "Embermouth Sentinel");
    begin(&mut g);
    let forest = g.add_card(0, tid("Forest"), Zone::Library);
    play(&mut g, c, false, &[], &[Action::Yes, Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(g.obj(forest).tapped);

    let mut g = table();
    let c = hand(&mut g, 0, "Embermouth Sentinel");
    begin(&mut g);
    let lib = g.players[0].library.clone();
    play(&mut g, c, false, &[], &[Action::No]);
    assert_eq!(g.players[0].library, lib, "you may not search");
}

#[test]
fn seize_opportunity_exiles_two_to_play_or_pumps_up_to_two() {
    let mut g = table();
    begin(&mut g);
    let lib = &g.players[0].library;
    let (a, b) = (lib[lib.len() - 1], lib[lib.len() - 2]);
    let c = hand(&mut g, 0, "Seize Opportunity");
    act(&mut g, Action::CastModal(c, 0));
    resolve_all(&mut g, &[]);
    for x in [a, b] {
        assert_eq!(g.obj(x).zone, Zone::Exile);
        assert_eq!(
            g.obj(x).playable,
            Some((0, g.turn + 2)),
            "until the end of your next turn"
        );
    }

    let mut g = table();
    let x = put(&mut g, 0, "Humbling Elder");
    let y = put(&mut g, 1, "Iceridge Serpent");
    begin(&mut g);
    let c = hand(&mut g, 0, "Seize Opportunity");
    act(&mut g, Action::CastModal(c, 1));
    act(&mut g, Action::Target(obj(x)));
    act(&mut g, Action::Target(obj(y)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(x), g.toughness(x)), (3, 3));
    assert_eq!((g.power(y), g.toughness(y)), (5, 4));
    let c = hand(&mut g, 0, "Seize Opportunity");
    act(&mut g, Action::CastModal(c, 1));
    act(&mut g, Action::Target(obj(x)));
    act(&mut g, Action::No);
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(x), 5, "just one");
}

#[test]
fn overwhelming_surge_does_one_or_both() {
    let mut g = table();
    let elder = put(&mut g, 1, "Humbling Elder");
    let serpent = put(&mut g, 1, "Iceridge Serpent");
    let globe = put(&mut g, 1, "Dragonstorm Globe");
    let dragon = put(&mut g, 1, "Boulderborn Dragon"); // an artifact creature
    begin(&mut g);
    let c = hand(&mut g, 0, "Overwhelming Surge");
    act(&mut g, Action::CastModal(c, 1));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(dragon))),
        "noncreature artifacts only"
    );
    act(&mut g, Action::Target(obj(globe)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(globe).zone, Zone::Graveyard);

    let c = hand(&mut g, 0, "Overwhelming Surge");
    act(&mut g, Action::CastModal(c, 0));
    act(&mut g, Action::Target(obj(elder)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elder).zone, Zone::Graveyard);

    let globe2 = put(&mut g, 1, "Dragonstorm Globe");
    let c = hand(&mut g, 0, "Overwhelming Surge");
    act(&mut g, Action::CastModal(c, 2));
    act(&mut g, Action::Target(obj(serpent)));
    act(&mut g, Action::Target(obj(globe2)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(serpent).zone, Zone::Graveyard);
    assert_eq!(g.obj(globe2).zone, Zone::Graveyard);
}

#[test]
fn salt_road_skirmish_kills_and_makes_hasty_warriors_that_leave_at_end_of_turn() {
    let mut g = table();
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    let c = hand(&mut g, 0, "Salt Road Skirmish");
    begin(&mut g);
    play(&mut g, c, false, &[obj(theirs)], &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    let warriors = tokens(&g, 0, "Warrior");
    assert_eq!(warriors.len(), 2);
    for &w in &warriors {
        assert!(g.keywords(w).has(Keywords::HASTE));
        assert!(!g.obj(w).tapped, "not tapped or attacking");
    }
    to_next_turn(&mut g);
    assert!(tokens(&g, 0, "Warrior").is_empty(), "sacrificed at the end step");
}

#[test]
fn sonic_shrieker_burns_heals_and_a_burned_player_discards() {
    let mut g = table();
    let c = hand(&mut g, 0, "Sonic Shrieker");
    let card = hand(&mut g, 1, "Cruel Truths");
    begin(&mut g);
    play(
        &mut g,
        c,
        false,
        &[],
        &[Action::Target(Target::Player(1)), Action::Discard(card)],
    );
    assert_eq!((g.players[0].life, g.players[1].life), (22, 18));
    assert_eq!(g.obj(card).zone, Zone::Graveyard);

    let mut g = table();
    let c = hand(&mut g, 0, "Sonic Shrieker");
    let card = hand(&mut g, 1, "Cruel Truths");
    let theirs = put(&mut g, 1, "Humbling Elder");
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Target(obj(theirs))]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.obj(card).zone, Zone::Hand, "no player was dealt damage");
    assert_eq!(g.players[0].life, 22);
}

#[test]
fn fresh_start_shrinks_and_strips_abilities_but_keeps_the_creature() {
    let mut g = table();
    let dragon = put(&mut g, 1, "Boulderborn Dragon"); // 3/3 flying, vigilance
    let c = hand(&mut g, 0, "Fresh Start");
    assert!(g.def_of(c).keywords.has(Keywords::FLASH));
    begin(&mut g);
    play(&mut g, c, false, &[obj(dragon)], &[]);
    assert_eq!(
        (g.power(dragon), g.toughness(dragon)),
        (-2, 3),
        "keeps its base 3/3"
    );
    assert!(!g.keywords(dragon).has(Keywords::FLYING));
    assert!(g.has_subtype(dragon, "Dragon"), "types stay");
    assert!(g.types_of(dragon).creature && g.types_of(dragon).artifact);
}

#[test]
fn ambling_stormshell_stuns_itself_to_draw_and_turtles_untap_it() {
    let mut g = table();
    let s = put(&mut g, 0, "Ambling Stormshell");
    assert!(g.def_of(s).ward.is_some());
    to_attacks(&mut g);
    let n = g.players[0].hand.len();
    attack(&mut g, &[s], &[]);
    assert_eq!(g.obj(s).stun, 3);
    assert_eq!(g.players[0].hand.len(), n + 3);
    assert!(g.obj(s).tapped);
    // A Turtle spell "untaps" it: with stun counters, one comes off instead (CR 122.1d).
    let turtle = hand(&mut g, 0, "Ambling Stormshell");
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    play(&mut g, turtle, false, &[], &[]);
    assert_eq!(g.obj(s).stun, 2);
    assert!(g.obj(s).tapped);
}

#[test]
fn eshki_dragonclaw_rewards_a_creature_and_a_noncreature_spell() {
    let mut g = table();
    let e = put(&mut g, 0, "Eshki Dragonclaw");
    let k = g.keywords(e);
    assert!(k.has(Keywords::VIGILANCE) && k.has(Keywords::TRAMPLE));
    assert!(g.def_of(e).ward.is_some());
    let elder = hand(&mut g, 0, "Humbling Elder");
    let spell = hand(&mut g, 0, "Cruel Truths");
    begin(&mut g);
    play(&mut g, elder, false, &[], &[]);
    play(&mut g, spell, false, &[], &[Action::No, Action::No]);
    let n = g.players[0].hand.len();
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(e).counters, 2);
    assert_eq!(g.players[0].hand.len(), n + 1);

    let mut g = table();
    let e = put(&mut g, 0, "Eshki Dragonclaw");
    let elder = hand(&mut g, 0, "Humbling Elder");
    begin(&mut g);
    play(&mut g, elder, false, &[], &[]);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(e).counters, 0, "only a creature spell");
}

#[test]
fn dalkovan_encampment_makes_warriors_whenever_you_attack_this_turn() {
    let mut g = table();
    let camp = put(&mut g, 0, "Dalkovan Encampment");
    let elder = put(&mut g, 0, "Humbling Elder");
    main_turn_5(&mut g);
    act(&mut g, Action::Activate(camp, 0));
    resolve_all(&mut g, &[]);
    assert!(tokens(&g, 0, "Warrior").is_empty(), "not until you attack");
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    attack(&mut g, &[elder], &[]);
    let warriors = tokens(&g, 0, "Warrior");
    assert_eq!(warriors.len(), 2);
    for &w in &warriors {
        assert!(g.obj(w).tapped);
        assert!(g.combat.as_ref().unwrap().attackers.contains(&w));
    }
    to_next_turn(&mut g);
    assert!(tokens(&g, 0, "Warrior").is_empty(), "sacrificed at the end step");
    // Only this turn.
    to_next_turn(&mut g);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    attack(&mut g, &[elder], &[]);
    assert!(tokens(&g, 0, "Warrior").is_empty());
}

#[test]
fn great_arashin_city_exiles_a_creature_card_for_a_spirit() {
    let mut g = table();
    let city = put(&mut g, 0, "Great Arashin City");
    let spell = g.add_card(0, tid("Cruel Truths"), Zone::Graveyard);
    begin(&mut g);
    assert!(!g
        .legal_actions()
        .iter()
        .any(|a| matches!(a, Action::ActivateWith(c, _, _) if *c == city)));
    let card = g.add_card(0, tid("Humbling Elder"), Zone::Graveyard);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::ActivateWith(city, 0, card)));
    assert!(
        !legal.contains(&Action::ActivateWith(city, 0, spell)),
        "a creature card"
    );
    act(&mut g, Action::ActivateWith(city, 0, card));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(card).zone, Zone::Exile);
    let spirit = tokens(&g, 0, "Spirit");
    assert_eq!(spirit.len(), 1);
    assert_eq!((g.power(spirit[0]), g.toughness(spirit[0])), (1, 1));
    assert!(!g.keywords(spirit[0]).has(Keywords::FLYING));
    assert!(g.obj(city).tapped);
}
