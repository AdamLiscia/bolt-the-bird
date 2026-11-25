//! Tarkir: Dragonstorm lane B cards: each script's behavior, on a Tarkir table. Log: docs/tdm-lane-b.md.

mod common;
mod tdm_common;

use bolt_core::{Action, Color, Game, Keywords, ObjId, Pending, Step, Target, Zone};
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
