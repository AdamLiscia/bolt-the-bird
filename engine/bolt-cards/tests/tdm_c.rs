//! Tarkir: Dragonstorm, lane C: each script's behavior on a Tarkir table. Log: docs/tdm-lane-c.md.

mod common;
mod tdm_common;

use bolt_core::{Action, Game, Keywords, Pending, Step, Target, Zone};
use common::{act, begin, obj, play, resolve_all};
use tdm_common::*;

/// Pass priority until player 0's main phase on a later turn (turn 5), so nothing is summoning sick.
fn main_phase_turn_5(g: &mut Game) {
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(g, Action::PassPriority);
    }
}

/// Pass priority until combat damage has been dealt this turn (blocking nothing).
fn through_combat_damage(g: &mut Game) {
    while g.step != Step::End {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

/// Pass (attacking and blocking with nothing) until the next turn has begun.
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
fn summit_intimidator_stops_a_creature_from_blocking() {
    let mut g = table();
    let attacker = put(&mut g, 0, "Unburied Earthcarver");
    let theirs = put(&mut g, 1, "Loxodon Battle Priest");
    let other = put(&mut g, 1, "Arashin Sunshield");
    let c = hand(&mut g, 0, "Summit Intimidator");
    assert!(g.def_of(c).keywords.has(Keywords::REACH));
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Target(obj(theirs))]);
    assert!(g.keywords(theirs).has(Keywords::CANT_BLOCK));
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(attacker));
    act(&mut g, Action::FinishAttacks);
    while g.pending != Pending::DeclareBlockers(1) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(
        !legal.contains(&Action::Block {
            blocker: theirs,
            attacker
        }),
        "it can't block this turn"
    );
    assert!(
        legal.contains(&Action::Block {
            blocker: other,
            attacker
        }),
        "the others still can"
    );
    act(&mut g, Action::FinishBlocks);
    to_next_turn(&mut g);
    assert!(!g.keywords(theirs).has(Keywords::CANT_BLOCK), "only this turn");
}

#[test]
fn arashin_sunshield_exiles_two_from_one_graveyard_and_taps() {
    let mut g = table();
    let a = g.add_card(1, tid("Lightfoot Technique"), Zone::Graveyard);
    let b = g.add_card(1, tid("Summit Intimidator"), Zone::Graveyard);
    let mine = g.add_card(0, tid("Wail of War"), Zone::Graveyard);
    let theirs = put(&mut g, 1, "Unburied Earthcarver");
    let c = hand(&mut g, 0, "Arashin Sunshield");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Target(obj(a)));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(mine))),
        "a single graveyard"
    );
    act(&mut g, Action::Target(obj(b)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(a).zone, Zone::Exile);
    assert_eq!(g.obj(b).zone, Zone::Exile);
    assert_eq!(g.obj(mine).zone, Zone::Graveyard);

    main_phase_turn_5(&mut g);
    act(&mut g, Action::Activate(c, 0));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert!(g.obj(theirs).tapped);
    assert!(g.obj(c).tapped, "{{T}} is part of the cost");
}

#[test]
fn lightfoot_technique_adds_a_counter_flying_and_indestructible() {
    let mut g = table();
    let mine = put(&mut g, 0, "Unburied Earthcarver");
    let c = hand(&mut g, 0, "Lightfoot Technique");
    begin(&mut g);
    play(&mut g, c, false, &[obj(mine)], &[]);
    assert_eq!(g.obj(mine).counters, 1);
    assert_eq!((g.power(mine), g.toughness(mine)), (3, 3));
    let k = g.keywords(mine);
    assert!(k.has(Keywords::FLYING) && k.has(Keywords::INDESTRUCTIBLE));
    to_next_turn(&mut g);
    assert!(!g.keywords(mine).has(Keywords::FLYING), "until end of turn");
    assert_eq!(g.obj(mine).counters, 1, "the counter stays");
}

#[test]
fn unburied_earthcarver_eats_another_creature_for_a_counter() {
    let mut g = table();
    let e = put(&mut g, 0, "Unburied Earthcarver");
    let food = put(&mut g, 0, "Summit Intimidator");
    begin(&mut g);
    assert!(
        !g.legal_actions().contains(&Action::ActivateWith(e, 0, e)),
        "another creature"
    );
    let lands = paid(&mut g, |g| {
        act(g, Action::ActivateWith(e, 0, food));
        resolve_all(g, &[]);
    });
    assert_eq!(lands, 2);
    assert_eq!(g.obj(food).zone, Zone::Graveyard);
    assert_eq!(g.obj(e).counters, 1);
    assert_eq!(g.power(e), 3);
}

#[test]
fn sunset_strikemaster_taps_for_red() {
    let mut g = tdm_lands(&["Plains"]);
    let s = put(&mut g, 0, "Sunset Strikemaster");
    let bolt = hand(&mut g, 0, "Channeled Dragonfire"); // {R}
    main_phase_turn_5(&mut g);
    let life = g.players[1].life;
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    assert_eq!(g.players[1].life, life - 2);
    assert!(g.obj(s).tapped, "the only red source");
}

#[test]
fn sunset_strikemaster_sacrifices_to_shoot_down_a_flier() {
    let mut g = table();
    let s = put(&mut g, 0, "Sunset Strikemaster");
    let flier = put(&mut g, 1, "Jeskai Shrinekeeper");
    let ground = put(&mut g, 1, "Unburied Earthcarver");
    main_phase_turn_5(&mut g);
    act(&mut g, Action::Activate(s, 0));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(ground))),
        "only creatures with flying"
    );
    act(&mut g, Action::Target(obj(flier)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(s).zone, Zone::Graveyard, "sacrificed as the cost");
    assert_eq!(g.obj(flier).zone, Zone::Graveyard, "6 damage to a 3/3");
}

#[test]
fn opulent_palace_enters_tapped_and_makes_black_green_or_blue() {
    let mut g = tdm_lands(&[]);
    let land = hand(&mut g, 0, "Opulent Palace");
    begin(&mut g);
    act(&mut g, Action::PlayLand(land));
    assert!(g.obj(land).tapped, "enters tapped");
    g.manual_mana[0] = true;
    main_phase_turn_5(&mut g);
    let mut colors: Vec<u8> = g
        .legal_actions()
        .into_iter()
        .filter_map(|a| match a {
            Action::TapForMana(x, c) if x == land => Some(c),
            _ => None,
        })
        .collect();
    colors.sort_unstable();
    assert_eq!(colors, [1, 2, 4], "U, B or G (WUBRG order)");
}

#[test]
fn jeskai_shrinekeeper_gains_and_draws_on_combat_damage() {
    let mut g = table();
    let k = put(&mut g, 0, "Jeskai Shrinekeeper");
    let ks = g.keywords(k);
    assert!(ks.has(Keywords::FLYING) && ks.has(Keywords::HASTE));
    attack_with(&mut g, k);
    let (life, opp, cards) = (g.players[0].life, g.players[1].life, g.players[0].hand.len());
    through_combat_damage(&mut g);
    assert_eq!(g.players[1].life, opp - 3);
    assert_eq!(g.players[0].life, life + 1);
    assert_eq!(g.players[0].hand.len(), cards + 1);
}

#[test]
fn war_effort_pumps_and_adds_an_attacking_warrior_each_combat() {
    let mut g = table();
    put(&mut g, 0, "War Effort");
    let e = put(&mut g, 0, "Unburied Earthcarver");
    let theirs = put(&mut g, 1, "Summit Intimidator");
    assert_eq!((g.power(e), g.toughness(e)), (3, 2), "+1/+0");
    assert_eq!(g.power(theirs), 4, "yours only");
    attack_with(&mut g, e);
    let w = tokens(&g, 0, "Warrior");
    assert_eq!(w.len(), 1, "one per combat, not per attacker");
    let combat = g.combat.as_ref().expect("in combat");
    assert!(g.obj(w[0]).tapped && combat.attackers.contains(&w[0]));
    assert_eq!(g.power(w[0]), 2, "the Warrior gets +1/+0 too");
    while g.turn == 5 && !tokens(&g, 0, "Warrior").is_empty() {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.step, Step::End, "sacrificed at the beginning of the end step");
}

#[test]
fn wail_of_war_shrinks_their_team_or_returns_two_creature_cards() {
    let mut g = table();
    let a = put(&mut g, 1, "Unburied Earthcarver");
    let b = put(&mut g, 1, "Loxodon Battle Priest");
    let mine = put(&mut g, 0, "Summit Intimidator");
    let c = hand(&mut g, 0, "Wail of War");
    begin(&mut g);
    act(&mut g, Action::CastModal(c, 0));
    assert!(
        !g.legal_actions().contains(&Action::Target(Target::Player(0))),
        "target opponent"
    );
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(a), g.toughness(a)), (1, 1));
    assert_eq!((g.power(b), g.toughness(b)), (2, 4));
    assert_eq!(g.power(mine), 4, "not yours");

    let x = g.add_card(0, tid("Jeskai Shrinekeeper"), Zone::Graveyard);
    let y = g.add_card(0, tid("Arashin Sunshield"), Zone::Graveyard);
    let spell = g.add_card(0, tid("Lightfoot Technique"), Zone::Graveyard);
    let c2 = hand(&mut g, 0, "Wail of War");
    act(&mut g, Action::CastModal(c2, 1));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(spell))),
        "creature cards only"
    );
    act(&mut g, Action::Target(obj(x)));
    act(&mut g, Action::Target(obj(y)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(x).zone, Zone::Hand);
    assert_eq!(g.obj(y).zone, Zone::Hand);
    assert_eq!(g.obj(spell).zone, Zone::Graveyard);
}

#[test]
fn jeskai_brushmaster_has_double_strike_and_prowess() {
    let mut g = table();
    let m = put(&mut g, 0, "Jeskai Brushmaster");
    let other = put(&mut g, 0, "Unburied Earthcarver");
    assert!(g.keywords(m).has(Keywords::DOUBLE_STRIKE));
    let c = hand(&mut g, 0, "Lightfoot Technique");
    begin(&mut g);
    play(&mut g, c, false, &[obj(other)], &[]);
    assert_eq!((g.power(m), g.toughness(m)), (3, 5), "prowess");
    let creature = hand(&mut g, 0, "Summit Intimidator");
    play(&mut g, creature, false, &[], &[Action::Target(obj(other))]);
    assert_eq!(g.power(m), 3, "creature spells don't trigger prowess");
}

#[test]
fn starry_eyed_skyrider_gives_another_creature_and_attacking_tokens_flying() {
    let mut g = table();
    put(&mut g, 0, "War Effort"); // makes an attacking Warrior token
    let s = put(&mut g, 0, "Starry-Eyed Skyrider");
    let other = put(&mut g, 0, "Unburied Earthcarver");
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(s));
    act(&mut g, Action::FinishAttacks);
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(s))),
        "another creature"
    );
    act(&mut g, Action::Target(obj(other)));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(other).has(Keywords::FLYING));
    let w = tokens(&g, 0, "Warrior");
    assert_eq!(w.len(), 1);
    assert!(g.keywords(w[0]).has(Keywords::FLYING), "an attacking token");
    let goblin = g.add_card(0, set_token(&g, "Goblin"), Zone::Battlefield);
    assert!(!g.keywords(goblin).has(Keywords::FLYING), "only attacking tokens");
}

/// The definition index of a token by name.
fn set_token(g: &Game, name: &str) -> usize {
    (0..g.defs.len())
        .find(|&i| g.defs[i].token && g.defs[i].name == name)
        .expect("a token definition")
}

#[test]
fn loxodon_battle_priest_grows_another_creature_at_the_start_of_combat() {
    let mut g = table();
    let p = put(&mut g, 0, "Loxodon Battle Priest");
    let other = put(&mut g, 0, "Unburied Earthcarver");
    g.start_turn(5, 0);
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.step, Step::BeginCombat);
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(p))),
        "another creature"
    );
    act(&mut g, Action::Target(obj(other)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(other).counters, 1);
    assert_eq!(g.obj(p).counters, 0);
}

#[test]
fn fangkeepers_familiar_gains_and_surveils_or_destroys_an_enchantment() {
    let mut g = table();
    let c = hand(&mut g, 0, "Fangkeeper's Familiar");
    assert!(g.def_of(c).keywords.has(Keywords::FLASH));
    begin(&mut g);
    let top = *g.players[0].library.last().unwrap();
    let life = g.players[0].life;
    play(
        &mut g,
        c,
        false,
        &[],
        &[Action::Mode(0), Action::Yes, Action::No, Action::No],
    );
    assert_eq!(g.players[0].life, life + 3);
    assert_eq!(g.obj(top).zone, Zone::Graveyard, "surveil 3: the top card binned");
    assert_eq!(g.players[0].graveyard.len(), 1);

    let ench = put(&mut g, 1, "War Effort");
    let c2 = hand(&mut g, 0, "Fangkeeper's Familiar");
    play(
        &mut g,
        c2,
        false,
        &[],
        &[Action::Mode(1), Action::Target(obj(ench))],
    );
    assert_eq!(g.obj(ench).zone, Zone::Graveyard);
}

#[test]
fn fangkeepers_familiar_counters_a_creature_spell() {
    let mut g = table();
    let creature = hand(&mut g, 1, "Unburied Earthcarver");
    let c = hand(&mut g, 0, "Fangkeeper's Familiar");
    g.start_turn(5, 1);
    while !(g.step == Step::Main1 && g.pending == Pending::Priority(1)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Cast(creature));
    while g.pending != Pending::Priority(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Cast(c)); // flash, in response
    resolve_all(&mut g, &[Action::Mode(2), Action::Target(obj(creature))]);
    assert_eq!(g.obj(creature).zone, Zone::Graveyard, "countered");
    assert_eq!(g.obj(c).zone, Zone::Battlefield);
}

#[test]
fn inevitable_defeat_exiles_a_nonland_permanent_and_drains_three() {
    let mut g = table();
    let ench = put(&mut g, 1, "War Effort");
    let land = g.permanents(1).find(|&x| g.types_of(x).land).unwrap();
    let c = hand(&mut g, 0, "Inevitable Defeat");
    assert!(g.def_of(c).uncounterable);
    begin(&mut g);
    let (mine, theirs) = (g.players[0].life, g.players[1].life);
    act(&mut g, Action::Cast(c));
    assert!(!g.legal_actions().contains(&Action::Target(obj(land))), "nonland");
    act(&mut g, Action::Target(obj(ench)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ench).zone, Zone::Exile);
    assert_eq!(g.players[1].life, theirs - 3);
    assert_eq!(g.players[0].life, mine + 3);

    // Its controller loses the life: exiling your own permanent costs you 3 (and gains you 3).
    let own = put(&mut g, 0, "Unburied Earthcarver");
    let c2 = hand(&mut g, 0, "Inevitable Defeat");
    let (mine, theirs) = (g.players[0].life, g.players[1].life);
    play(&mut g, c2, false, &[obj(own)], &[]);
    assert_eq!(g.obj(own).zone, Zone::Exile);
    assert_eq!((g.players[0].life, g.players[1].life), (mine, theirs));
}

#[test]
fn perennation_returns_a_permanent_card_with_hexproof_and_indestructible() {
    let mut g = table();
    let mine = g.add_card(0, tid("Jeskai Shrinekeeper"), Zone::Graveyard);
    let theirs = g.add_card(1, tid("Summit Intimidator"), Zone::Graveyard);
    let instant = g.add_card(0, tid("Lightfoot Technique"), Zone::Graveyard);
    let c = hand(&mut g, 0, "Perennation");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Target(obj(theirs))), "your graveyard");
    assert!(!legal.contains(&Action::Target(obj(instant))), "permanent cards");
    act(&mut g, Action::Target(obj(mine)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(mine).zone, Zone::Battlefield);
    assert_eq!(g.obj(mine).controller, 0);
    g.start_turn(5, 0);
    let k = g.keywords(mine);
    assert!(
        k.has(Keywords::HEXPROOF) && k.has(Keywords::INDESTRUCTIBLE),
        "counters last"
    );

    let ench = g.add_card(0, tid("War Effort"), Zone::Graveyard);
    let c2 = hand(&mut g, 0, "Perennation");
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    play(&mut g, c2, false, &[obj(ench)], &[]);
    assert_eq!(g.obj(ench).zone, Zone::Battlefield, "any permanent card");
}
