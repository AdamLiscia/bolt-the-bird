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

// ---- round 3: core mechanics ----

/// Pass priority until a target choice is pending.
fn to_target_choice(g: &mut Game) {
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        act(g, Action::PassPriority);
    }
}

#[test]
fn twin_bolt_divides_two_damage_between_two_targets() {
    let mut g = table();
    let a = put(&mut g, 1, "Unburied Earthcarver");
    let b = put(&mut g, 1, "Loxodon Battle Priest");
    let c = hand(&mut g, 0, "Twin Bolt");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(a)));
    act(&mut g, Action::Target(obj(b)));
    assert!(matches!(g.pending, Pending::Divide(0)));
    assert_eq!(g.legal_actions(), [Action::Amount(1)], "at least 1 to each");
    act(&mut g, Action::Amount(1));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(a).damage, g.obj(b).damage), (1, 1));
}

#[test]
fn twin_bolt_cant_target_the_same_player_twice() {
    let mut g = table();
    let c = hand(&mut g, 0, "Twin Bolt");
    begin(&mut g);
    let life = g.players[1].life;
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(Target::Player(1)));
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Target(Target::Player(1))), "CR 115.3");
    assert!(legal.contains(&Action::Target(Target::Player(0))));
    act(&mut g, Action::No); // one target: it gets both
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 2);
}

#[test]
fn unsparing_boltcaster_finishes_off_a_damaged_creature() {
    let mut g = table();
    let hurt = put(&mut g, 1, "Loxodon Battle Priest"); // 3/5
    let fresh = put(&mut g, 1, "Arashin Sunshield");
    let bolt = hand(&mut g, 0, "Channeled Dragonfire");
    let c = hand(&mut g, 0, "Unsparing Boltcaster");
    begin(&mut g);
    play(&mut g, bolt, false, &[obj(hurt)], &[]);
    assert_eq!(g.obj(hurt).damage, 2);
    act(&mut g, Action::Cast(c));
    to_target_choice(&mut g);
    let legal = g.legal_actions();
    assert!(
        !legal.contains(&Action::Target(obj(fresh))),
        "dealt damage this turn only"
    );
    act(&mut g, Action::Target(obj(hurt)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(hurt).zone, Zone::Graveyard);
    assert_eq!(g.obj(fresh).zone, Zone::Battlefield);
}

#[test]
fn smile_at_death_returns_two_small_creatures_with_counters() {
    let mut g = table();
    put(&mut g, 0, "Smile at Death");
    let a = g.add_card(0, tid("Unburied Earthcarver"), Zone::Graveyard); // 2 power
    let b = g.add_card(0, tid("Delta Bloodflies"), Zone::Graveyard); // 1 power
    let big = g.add_card(0, tid("Summit Intimidator"), Zone::Graveyard); // 4 power
    let theirs = g.add_card(1, tid("Unburied Earthcarver"), Zone::Graveyard);
    g.start_turn(5, 0);
    to_target_choice(&mut g);
    assert_eq!(g.step, Step::Upkeep);
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Target(obj(big))), "power 2 or less");
    assert!(!legal.contains(&Action::Target(obj(theirs))), "your graveyard");
    act(&mut g, Action::Target(obj(a)));
    act(&mut g, Action::Target(obj(b)));
    resolve_all(&mut g, &[]);
    for x in [a, b] {
        assert_eq!(g.obj(x).zone, Zone::Battlefield);
        assert_eq!(g.obj(x).controller, 0);
        assert_eq!(g.obj(x).counters, 1);
    }
    assert_eq!(g.obj(big).zone, Zone::Graveyard);
}

#[test]
fn hardened_tactician_sacrifices_a_token_to_draw() {
    let mut g = tdm_lands(&[]);
    let t = put(&mut g, 0, "Hardened Tactician");
    let creature = put(&mut g, 0, "Unburied Earthcarver");
    let treasure = g.add_card(0, set_token(&g, "Treasure"), Zone::Battlefield);
    begin(&mut g);
    assert!(
        !g.legal_actions().contains(&Action::ActivateWith(t, 0, treasure)),
        "the Treasure can't both pay {{1}} and be sacrificed"
    );
    let plains = put(&mut g, 0, "Plains");
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::ActivateWith(t, 0, treasure)));
    assert!(!legal.contains(&Action::ActivateWith(t, 0, creature)), "a token");
    let cards = g.players[0].hand.len();
    act(&mut g, Action::ActivateWith(t, 0, treasure));
    resolve_all(&mut g, &[]);
    assert!(tokens(&g, 0, "Treasure").is_empty());
    assert!(g.obj(plains).tapped);
    assert_eq!(g.players[0].hand.len(), cards + 1);
}

#[test]
fn fire_rim_form_gives_two_power_and_first_strike_this_turn() {
    let mut g = table();
    let mine = put(&mut g, 0, "Unburied Earthcarver");
    let c = hand(&mut g, 0, "Fire-Rim Form");
    assert!(g.def_of(c).keywords.has(Keywords::FLASH));
    begin(&mut g);
    play(&mut g, c, false, &[obj(mine)], &[]);
    assert_eq!(g.obj(c).zone, Zone::Battlefield);
    assert_eq!(g.power(mine), 4);
    assert!(g.keywords(mine).has(Keywords::FIRST_STRIKE));
    to_next_turn(&mut g);
    assert_eq!(g.power(mine), 4, "the +2/+0 stays");
    assert!(
        !g.keywords(mine).has(Keywords::FIRST_STRIKE),
        "first strike until end of turn"
    );
}

/// Whether `c` may be declared as an attacker this turn (the declaration is skipped with none).
fn can_attack(g: &Game, c: bolt_core::ObjId) -> bool {
    let mut g = g.clone();
    while !matches!(g.step, Step::Main2 | Step::End) {
        if g.pending == Pending::DeclareAttackers(0) {
            return g.legal_actions().contains(&Action::Attack(c));
        }
        act(&mut g, Action::PassPriority);
    }
    false
}

#[test]
fn krotiq_nestguard_attacks_only_after_paying() {
    let mut g = table();
    let k = put(&mut g, 0, "Krotiq Nestguard");
    assert!(g.keywords(k).has(Keywords::DEFENDER));
    main_phase_turn_5(&mut g);
    assert!(!can_attack(&g, k), "defender");
    let lands = paid(&mut g, |g| {
        act(g, Action::Activate(k, 0));
        resolve_all(g, &[]);
    });
    assert_eq!(lands, 3);
    assert!(can_attack(&g, k));
    g.start_turn(7, 0); // a later turn of yours
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert!(!can_attack(&g, k), "this turn only");
}

#[test]
fn ruinous_ultimatum_destroys_their_nonland_permanents() {
    let mut g = table();
    let ench = put(&mut g, 1, "War Effort");
    let theirs = put(&mut g, 1, "Unburied Earthcarver");
    let mine = put(&mut g, 0, "Unburied Earthcarver");
    let lands = g.permanents(1).filter(|&x| g.types_of(x).land).count();
    let c = hand(&mut g, 0, "Ruinous Ultimatum");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(ench).zone, Zone::Graveyard);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.obj(mine).zone, Zone::Battlefield, "yours stay");
    assert_eq!(
        g.permanents(1).filter(|&x| g.types_of(x).land).count(),
        lands,
        "lands stay"
    );
}

#[test]
fn death_begets_life_draws_one_per_permanent_destroyed() {
    let mut g = table();
    let a = put(&mut g, 0, "Unburied Earthcarver");
    let e = put(&mut g, 0, "War Effort");
    let b = put(&mut g, 1, "Loxodon Battle Priest");
    let relic = put(&mut g, 1, "Jeskai Monument"); // an artifact: not destroyed
    let c = hand(&mut g, 0, "Death Begets Life");
    begin(&mut g);
    let cards = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[]);
    for x in [a, e, b] {
        assert_eq!(g.obj(x).zone, Zone::Graveyard);
    }
    assert_eq!(g.obj(relic).zone, Zone::Battlefield);
    assert_eq!(g.players[0].hand.len(), cards - 1 + 3, "cast one, drew three");
}

#[test]
fn death_begets_life_doesnt_count_an_indestructible_survivor() {
    let mut g = table();
    let a = put(&mut g, 0, "Unburied Earthcarver");
    let tough = put(&mut g, 0, "Loxodon Battle Priest");
    let shield = hand(&mut g, 0, "Lightfoot Technique");
    let c = hand(&mut g, 0, "Death Begets Life");
    begin(&mut g);
    play(&mut g, shield, false, &[obj(tough)], &[]);
    let cards = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(a).zone, Zone::Graveyard);
    assert_eq!(g.obj(tough).zone, Zone::Battlefield, "indestructible");
    assert_eq!(g.players[0].hand.len(), cards - 1 + 1);
}

#[test]
fn dragonback_assault_sweeps_creatures_and_planeswalkers_then_makes_dragons() {
    let mut g = table();
    let small = put(&mut g, 1, "Unburied Earthcarver"); // 2/2
    let big = put(&mut g, 1, "Loxodon Battle Priest"); // 3/5
    let mine = put(&mut g, 0, "Krotiq Nestguard"); // 4/4
    let ugin = hand(&mut g, 1, "Ugin, Eye of the Storms");
    g.move_card(ugin, Zone::Battlefield);
    assert_eq!(g.obj(ugin).loyalty, 7);
    let c = hand(&mut g, 0, "Dragonback Assault");
    let land = hand(&mut g, 0, "Forest");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(small).zone, Zone::Graveyard);
    assert_eq!(g.obj(big).damage, 3);
    assert_eq!(g.obj(mine).damage, 3, "yours too");
    assert_eq!(g.obj(ugin).loyalty, 4, "each planeswalker");
    assert_eq!(g.players[1].life, 20, "not players");
    act(&mut g, Action::PlayLand(land));
    resolve_all(&mut g, &[]);
    let d = tokens(&g, 0, "Dragon");
    assert_eq!(d.len(), 1, "landfall");
    assert_eq!((g.power(d[0]), g.toughness(d[0])), (4, 4));
    assert!(g.keywords(d[0]).has(Keywords::FLYING));
}

#[test]
fn duty_beyond_death_sacrifices_then_protects_and_grows_the_rest() {
    let mut g = table();
    let fodder = put(&mut g, 0, "Summit Intimidator");
    let a = put(&mut g, 0, "Unburied Earthcarver");
    let b = put(&mut g, 0, "Loxodon Battle Priest");
    let theirs = put(&mut g, 1, "Arashin Sunshield");
    let c = hand(&mut g, 0, "Duty Beyond Death");
    begin(&mut g);
    act(&mut g, Action::CastWith(c, fodder));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(fodder).zone, Zone::Graveyard, "sacrificed as the cost");
    for x in [a, b] {
        assert_eq!(g.obj(x).counters, 1);
        assert!(g.keywords(x).has(Keywords::INDESTRUCTIBLE));
    }
    assert_eq!(g.obj(theirs).counters, 0, "yours only");
    assert!(!g.keywords(theirs).has(Keywords::INDESTRUCTIBLE));
}

// ---- round 3, batch 2: the Sieges ----

/// Cast a Siege from hand on turn 3 and choose mode `m`.
fn siege_in_mode(g: &mut Game, name: &str, m: u8) -> bolt_core::ObjId {
    let c = hand(g, 0, name);
    begin(g);
    act(g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(g, Action::PassPriority);
    }
    assert_eq!(g.legal_actions(), [Action::Mode(0), Action::Mode(1)], "two modes");
    act(g, Action::Mode(m));
    resolve_all(g, &[]);
    assert_eq!(g.obj(c).zone, Zone::Battlefield);
    assert_eq!(g.obj(c).chosen_mode, Some(m));
    c
}

#[test]
fn windcrag_siege_mardu_doubles_attack_triggers() {
    let mut g = table();
    let p = put(&mut g, 0, "Dalkovan Packbeasts"); // Mobilize 3
    siege_in_mode(&mut g, "Windcrag Siege", 0);
    attack_with(&mut g, p);
    assert_eq!(tokens(&g, 0, "Warrior").len(), 6, "mobilize triggered twice");
}

#[test]
fn windcrag_siege_jeskai_makes_a_lifelink_haste_goblin_each_upkeep() {
    let mut g = table();
    let p = put(&mut g, 0, "Dalkovan Packbeasts");
    siege_in_mode(&mut g, "Windcrag Siege", 1);
    assert!(tokens(&g, 0, "Goblin").is_empty());
    g.start_turn(5, 0);
    while g.step == Step::Upkeep || !g.items.is_empty() {
        act(&mut g, Action::PassPriority);
    }
    let goblin = tokens(&g, 0, "Goblin");
    assert_eq!(goblin.len(), 1);
    let k = g.keywords(goblin[0]);
    assert!(k.has(Keywords::LIFELINK) && k.has(Keywords::HASTE));
    let mut g2 = g.clone();
    while g2.pending != Pending::DeclareAttackers(0) {
        act(&mut g2, Action::PassPriority);
    }
    act(&mut g2, Action::Attack(p));
    act(&mut g2, Action::FinishAttacks);
    resolve_all(&mut g2, &[]);
    assert_eq!(
        tokens(&g2, 0, "Warrior").len(),
        3,
        "Jeskai doesn't double attack triggers"
    );
}

#[test]
fn frostcliff_siege_jeskai_draws_once_per_combat_damage_step() {
    let mut g = table();
    let a = put(&mut g, 0, "Unburied Earthcarver");
    let b = put(&mut g, 0, "Summit Intimidator");
    siege_in_mode(&mut g, "Frostcliff Siege", 0);
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(a));
    act(&mut g, Action::Attack(b));
    act(&mut g, Action::FinishAttacks);
    let cards = g.players[0].hand.len();
    through_combat_damage(&mut g);
    assert_eq!(g.players[0].hand.len(), cards + 1, "one card for two creatures");
}

#[test]
fn frostcliff_siege_temur_pumps_your_creatures_with_trample_and_haste() {
    let mut g = table();
    let mine = put(&mut g, 0, "Unburied Earthcarver");
    let theirs = put(&mut g, 1, "Unburied Earthcarver");
    siege_in_mode(&mut g, "Frostcliff Siege", 1);
    assert_eq!((g.power(mine), g.toughness(mine)), (3, 2));
    let k = g.keywords(mine);
    assert!(k.has(Keywords::TRAMPLE) && k.has(Keywords::HASTE));
    assert_eq!(g.power(theirs), 2, "yours only");
    assert!(!g.keywords(theirs).has(Keywords::TRAMPLE));
}

#[test]
fn hollowmurk_siege_sultai_draws_on_the_first_counter_each_turn() {
    let mut g = table();
    let mine = put(&mut g, 0, "Unburied Earthcarver");
    let theirs = put(&mut g, 1, "Unburied Earthcarver");
    siege_in_mode(&mut g, "Hollowmurk Siege", 0);
    let first = hand(&mut g, 0, "Lightfoot Technique");
    let second = hand(&mut g, 0, "Lightfoot Technique");
    let n = g.players[0].hand.len();
    play(&mut g, first, false, &[obj(theirs)], &[]);
    assert_eq!(g.players[0].hand.len(), n - 1, "not on an opponent's creature");
    play(&mut g, second, false, &[obj(mine)], &[]);
    assert_eq!(g.players[0].hand.len(), n - 2 + 1, "drew one");
    let third = hand(&mut g, 0, "Lightfoot Technique");
    play(&mut g, third, false, &[obj(mine)], &[]);
    assert_eq!(
        g.players[0].hand.len(),
        n - 1,
        "cast one, drew none: only once each turn"
    );
    // A later turn: an endure Spirit's counters as it enters count too (CR 122.6).
    main_phase_turn_5(&mut g);
    let kin = hand(&mut g, 0, "Fortress Kin-Guard"); // enters, endures 1
    let n = g.players[0].hand.len();
    play(&mut g, kin, false, &[], &[Action::No]); // endure: make a Spirit, not a counter
    assert_eq!(tokens(&g, 0, "Spirit").len(), 1);
    assert_eq!(g.players[0].hand.len(), n - 1 + 1, "the Spirit's counter drew");
}

#[test]
fn hollowmurk_siege_abzan_grows_an_attacker_and_gives_it_menace() {
    let mut g = table();
    let a = put(&mut g, 0, "Unburied Earthcarver");
    siege_in_mode(&mut g, "Hollowmurk Siege", 1);
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(a));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[Action::Target(obj(a))]);
    assert_eq!(g.obj(a).counters, 1);
    assert!(g.keywords(a).has(Keywords::MENACE));
}

#[test]
fn glacierwood_siege_temur_mills_four_on_each_instant_or_sorcery() {
    let mut g = table();
    siege_in_mode(&mut g, "Glacierwood Siege", 0);
    let spell = hand(&mut g, 0, "Unending Whisper");
    let lib = g.players[1].library.len();
    play(&mut g, spell, false, &[], &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].library.len(), lib - 4);
    let creature = hand(&mut g, 0, "Unburied Earthcarver");
    play(&mut g, creature, false, &[], &[]);
    assert_eq!(g.players[1].library.len(), lib - 4, "creature spells don't");
}

#[test]
fn glacierwood_siege_sultai_plays_lands_from_the_graveyard_with_your_land_drop() {
    let mut g = tdm_lands(&["Forest", "Forest", "Island"]);
    let a = g.add_card(0, tid("Forest"), Zone::Graveyard);
    let b = g.add_card(0, tid("Island"), Zone::Graveyard);
    let c = hand(&mut g, 0, "Glacierwood Siege");
    begin(&mut g);
    assert!(
        !g.legal_actions().contains(&Action::PlayLand(a)),
        "not without the Siege"
    );
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Mode(1));
    resolve_all(&mut g, &[]);
    assert!(g.legal_actions().contains(&Action::PlayLand(a)));
    act(&mut g, Action::PlayLand(a));
    assert_eq!(g.obj(a).zone, Zone::Battlefield);
    assert!(!g.legal_actions().contains(&Action::PlayLand(b)), "one land drop");
}

#[test]
fn barrensteppe_siege_abzan_puts_a_counter_on_each_creature_at_your_end_step() {
    let mut g = table();
    let a = put(&mut g, 0, "Unburied Earthcarver");
    let b = put(&mut g, 0, "Arashin Sunshield");
    let theirs = put(&mut g, 1, "Unburied Earthcarver");
    siege_in_mode(&mut g, "Barrensteppe Siege", 0);
    to_next_turn(&mut g);
    assert_eq!((g.obj(a).counters, g.obj(b).counters), (1, 1));
    assert_eq!(g.obj(theirs).counters, 0);
    to_next_turn(&mut g); // the opponent's end step: nothing
    assert_eq!(g.obj(a).counters, 1, "your end step only");
}

#[test]
fn barrensteppe_siege_mardu_makes_them_sacrifice_if_yours_died() {
    let mut g = table();
    let e = put(&mut g, 0, "Unburied Earthcarver");
    let fodder = put(&mut g, 0, "Summit Intimidator");
    let theirs = put(&mut g, 1, "Loxodon Battle Priest");
    let mut quiet = table();
    let q_theirs = put(&mut quiet, 1, "Loxodon Battle Priest");
    siege_in_mode(&mut quiet, "Barrensteppe Siege", 1);
    to_next_turn(&mut quiet);
    assert_eq!(
        quiet.obj(q_theirs).zone,
        Zone::Battlefield,
        "no creature of yours died"
    );

    siege_in_mode(&mut g, "Barrensteppe Siege", 1);
    act(&mut g, Action::ActivateWith(e, 0, fodder));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(fodder).zone, Zone::Graveyard);
    to_next_turn(&mut g);
    assert_eq!(
        g.obj(theirs).zone,
        Zone::Graveyard,
        "each opponent sacrificed one"
    );
}

// ---- round 3, batch 3 ----

#[test]
fn aggressive_negotiations_exiles_a_nonland_card_and_grows_a_creature() {
    let mut g = table();
    let land = hand(&mut g, 1, "Forest");
    let card = hand(&mut g, 1, "Twin Bolt");
    let mine = put(&mut g, 0, "Unburied Earthcarver");
    let c = hand(&mut g, 0, "Aggressive Negotiations");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::Target(obj(mine)));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.legal_actions(), [Action::Pick(card)], "nonland cards only");
    act(&mut g, Action::Pick(card));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(card).zone, Zone::Exile);
    assert_eq!(g.obj(land).zone, Zone::Hand);
    assert_eq!(g.obj(mine).counters, 1);

    // The creature is "up to one".
    let other = hand(&mut g, 1, "Twin Bolt");
    let c2 = hand(&mut g, 0, "Aggressive Negotiations");
    act(&mut g, Action::Cast(c2));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::No);
    resolve_all(&mut g, &[Action::Pick(other)]);
    assert_eq!(g.obj(other).zone, Zone::Exile);
    assert_eq!(g.obj(mine).counters, 1);
}

#[test]
fn sunpearl_kirin_bounces_and_draws_only_for_a_token() {
    let mut g = table();
    let goblin = g.add_card(0, set_token(&g, "Goblin"), Zone::Battlefield);
    let creature = put(&mut g, 0, "Unburied Earthcarver");
    let land = g.permanents(0).find(|&x| g.types_of(x).land).unwrap();
    let k = hand(&mut g, 0, "Sunpearl Kirin");
    let ks = g.def_of(k).keywords;
    assert!(ks.has(Keywords::FLASH) && ks.has(Keywords::FLYING));
    begin(&mut g);
    act(&mut g, Action::Cast(k));
    to_target_choice(&mut g);
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Target(obj(k))), "other");
    assert!(!legal.contains(&Action::Target(obj(land))), "nonland");
    let cards = g.players[0].hand.len();
    act(&mut g, Action::Target(obj(goblin)));
    resolve_all(&mut g, &[]);
    assert!(tokens(&g, 0, "Goblin").is_empty());
    assert_eq!(g.players[0].hand.len(), cards + 1, "drew for the token");

    let k2 = hand(&mut g, 0, "Sunpearl Kirin");
    let cards = g.players[0].hand.len();
    play(&mut g, k2, false, &[], &[Action::Target(obj(creature))]);
    assert_eq!(g.obj(creature).zone, Zone::Hand);
    assert_eq!(
        g.players[0].hand.len(),
        cards - 1 + 1,
        "the creature came back, no draw"
    );
}

#[test]
fn veteran_ice_climber_is_unblockable_and_mills_by_its_power() {
    let mut g = table();
    let v = put(&mut g, 0, "Veteran Ice Climber");
    assert!(g.keywords(v).has(Keywords::UNBLOCKABLE) && g.keywords(v).has(Keywords::VIGILANCE));
    let lib = g.players[1].library.len();
    attack_with(&mut g, v);
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].library.len(), lib - 1, "power 1");
}

#[test]
fn kishla_trawlers_trades_a_creature_card_for_a_spell_card() {
    let mut g = table();
    let creature = g.add_card(0, tid("Unburied Earthcarver"), Zone::Graveyard);
    let spell = g.add_card(0, tid("Twin Bolt"), Zone::Graveyard);
    let c = hand(&mut g, 0, "Kishla Trawlers");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(
        g.legal_actions(),
        [Action::Pick(creature), Action::No],
        "creature cards, or decline"
    );
    act(&mut g, Action::Pick(creature));
    resolve_all(&mut g, &[Action::Target(obj(spell))]);
    assert_eq!(g.obj(creature).zone, Zone::Exile);
    assert_eq!(g.obj(spell).zone, Zone::Hand);

    let spell2 = g.add_card(0, tid("Twin Bolt"), Zone::Graveyard);
    let _creature2 = g.add_card(0, tid("Summit Intimidator"), Zone::Graveyard);
    let c2 = hand(&mut g, 0, "Kishla Trawlers");
    play(&mut g, c2, false, &[], &[Action::No]);
    assert_eq!(g.obj(spell2).zone, Zone::Graveyard, "declined: no return");
}

#[test]
fn rainveil_rejuvenator_mills_and_taps_for_green_equal_to_its_power() {
    let mut g = tdm_lands(&["Forest", "Forest", "Forest", "Forest"]);
    let r = hand(&mut g, 0, "Rainveil Rejuvenator");
    begin(&mut g);
    let lib = g.players[0].library.len();
    play(&mut g, r, false, &[], &[Action::Yes]);
    assert_eq!(g.players[0].library.len(), lib - 3);
    main_phase_turn_5(&mut g);
    let from_it = g.mana_sources(0).iter().filter(|(x, _)| *x == r).count();
    assert_eq!(from_it, 2, "power 2");
    let breaker = hand(&mut g, 0, "Formation Breaker"); // {1}{G}
    for f in g
        .permanents(0)
        .filter(|&x| g.types_of(x).land)
        .collect::<Vec<_>>()
    {
        g.move_card(f, Zone::Graveyard); // only the Rejuvenator left to pay
    }
    act(&mut g, Action::Cast(breaker));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(breaker).zone, Zone::Battlefield);
    assert!(g.obj(r).tapped);
}

#[test]
fn aegis_sculptor_grows_by_exiling_two_graveyard_cards() {
    let mut g = table();
    let a = put(&mut g, 0, "Aegis Sculptor");
    assert!(g.def_of(a).ward.is_some());
    let one = g.add_card(0, tid("Twin Bolt"), Zone::Graveyard);
    g.start_turn(5, 0);
    assert_eq!(g.obj(a).counters, 0, "one card isn't enough");
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let two = g.add_card(0, tid("Summit Intimidator"), Zone::Graveyard);
    g.start_turn(7, 0);
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Pick(one));
    act(&mut g, Action::Pick(two));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(one).zone, g.obj(two).zone), (Zone::Exile, Zone::Exile));
    assert_eq!(g.obj(a).counters, 1);
}

#[test]
fn stormbeacon_blade_pumps_and_draws_with_three_attackers() {
    let mut g = table();
    let blade = put(&mut g, 0, "Stormbeacon Blade");
    let a = put(&mut g, 0, "Unburied Earthcarver");
    let b = put(&mut g, 0, "Summit Intimidator");
    let c = put(&mut g, 0, "Arashin Sunshield");
    begin(&mut g);
    act(&mut g, Action::Activate(blade, 0));
    act(&mut g, Action::Target(obj(a)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(a), 5);

    let mut alone = g.clone();
    attack_with(&mut alone, a);
    let cards = alone.players[0].hand.len();
    resolve_all(&mut alone, &[]);
    assert_eq!(alone.players[0].hand.len(), cards, "one attacker: no card");

    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    let cards = g.players[0].hand.len();
    for x in [a, b, c] {
        act(&mut g, Action::Attack(x));
    }
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), cards + 1, "three attackers: draw");
}

#[test]
fn dragonologist_finds_a_dragon_and_shields_untapped_dragons() {
    let mut g = table();
    let mine = put(&mut g, 0, "Boulderborn Dragon");
    let theirs = put(&mut g, 1, "Boulderborn Dragon");
    let tapper = put(&mut g, 0, "Arashin Sunshield"); // {W}, {T}: tap target creature
    let c = hand(&mut g, 0, "Dragonologist");
    begin(&mut g);
    let dragon = g.add_card(0, tid("Boulderborn Dragon"), Zone::Library);
    let creature = g.add_card(0, tid("Unburied Earthcarver"), Zone::Library);
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(dragon)));
    assert!(
        !legal.contains(&Action::Pick(creature)),
        "instants, sorceries, Dragons"
    );
    act(&mut g, Action::Pick(dragon));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(dragon).zone, Zone::Hand);
    assert!(
        g.players[0].library[..5].contains(&creature),
        "the rest on the bottom, in a random order"
    );
    assert!(g.keywords(mine).has(Keywords::HEXPROOF));
    assert!(!g.keywords(theirs).has(Keywords::HEXPROOF), "yours only");
    assert!(!g.keywords(c).has(Keywords::HEXPROOF), "Dragons only");
    main_phase_turn_5(&mut g);
    act(&mut g, Action::Activate(tapper, 0)); // your own ability can target it
    act(&mut g, Action::Target(obj(mine)));
    resolve_all(&mut g, &[]);
    assert!(g.obj(mine).tapped);
    assert!(!g.keywords(mine).has(Keywords::HEXPROOF), "untapped only");
}

#[test]
fn furious_forebear_returns_when_another_creature_of_yours_dies() {
    let mut g = table();
    let f = g.add_card(0, tid("Furious Forebear"), Zone::Graveyard);
    let e = put(&mut g, 0, "Unburied Earthcarver");
    let fodder = put(&mut g, 0, "Summit Intimidator");
    let theirs = put(&mut g, 1, "Unburied Earthcarver");
    let bolt = hand(&mut g, 0, "Channeled Dragonfire");
    begin(&mut g);
    play(&mut g, bolt, false, &[obj(theirs)], &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.obj(f).zone, Zone::Graveyard, "not for an opponent's creature");
    act(&mut g, Action::ActivateWith(e, 0, fodder));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.obj(f).zone, Zone::Hand, "paid {{1}}{{W}}");

    // It doesn't see its own death.
    let mut g2 = table();
    let f2 = put(&mut g2, 0, "Furious Forebear");
    let e2 = put(&mut g2, 0, "Unburied Earthcarver");
    begin(&mut g2);
    act(&mut g2, Action::ActivateWith(e2, 0, f2));
    resolve_all(&mut g2, &[]);
    assert_eq!(g2.obj(f2).zone, Zone::Graveyard);
}

#[test]
fn mox_jasper_makes_mana_only_with_a_dragon() {
    let mut g = tdm_lands(&[]);
    let mox = put(&mut g, 0, "Mox Jasper");
    begin(&mut g);
    assert!(g.mana_sources(0).iter().all(|(x, _)| *x != mox), "no Dragon");
    put(&mut g, 0, "Boulderborn Dragon");
    let from_mox: Vec<_> = g.mana_sources(0).into_iter().filter(|(x, _)| *x == mox).collect();
    assert_eq!(from_mox.len(), 1);
    assert_eq!(from_mox[0].1.colors, [true; 5], "any color");
}

#[test]
fn dracogenesis_casts_dragons_for_free() {
    let mut g = tdm_lands(&[]);
    put(&mut g, 0, "Dracogenesis");
    let d = hand(&mut g, 0, "Boulderborn Dragon"); // {5}
    let other = hand(&mut g, 0, "Unburied Earthcarver");
    begin(&mut g);
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Cast(other)), "only Dragon spells");
    act(&mut g, Action::Cast(d));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(d).zone, Zone::Battlefield);
}

// ---- round 3, batch 4 ----

#[test]
fn yathan_tombguard_draws_for_creatures_with_counters_that_connect() {
    let mut g = table();
    put(&mut g, 0, "Yathan Tombguard");
    let a = put(&mut g, 0, "Unburied Earthcarver");
    let b = put(&mut g, 0, "Summit Intimidator");
    let c = hand(&mut g, 0, "Lightfoot Technique");
    begin(&mut g);
    play(&mut g, c, false, &[obj(a)], &[]);
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(a));
    act(&mut g, Action::Attack(b));
    act(&mut g, Action::FinishAttacks);
    let (cards, life) = (g.players[0].hand.len(), g.players[0].life);
    through_combat_damage(&mut g);
    assert_eq!(g.players[0].hand.len(), cards + 1, "one creature had a counter");
    assert_eq!(g.players[0].life, life - 1);
}

#[test]
fn armament_dragon_distributes_three_counters() {
    let mut g = table();
    let a = put(&mut g, 0, "Unburied Earthcarver");
    let b = put(&mut g, 0, "Summit Intimidator");
    let theirs = put(&mut g, 1, "Arashin Sunshield");
    let d = hand(&mut g, 0, "Armament Dragon");
    begin(&mut g);
    act(&mut g, Action::Cast(d));
    to_target_choice(&mut g);
    assert!(!g.legal_actions().contains(&Action::Target(obj(theirs))), "yours");
    act(&mut g, Action::Target(obj(a)));
    act(&mut g, Action::Target(obj(b)));
    act(&mut g, Action::No);
    assert!(matches!(g.pending, Pending::Divide(0)));
    act(&mut g, Action::Amount(2));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(a).counters, g.obj(b).counters), (2, 1));
}

#[test]
fn genesis_ultimatum_puts_permanents_down_and_the_rest_in_hand() {
    let mut g = table();
    let c = hand(&mut g, 0, "Genesis Ultimatum");
    begin(&mut g);
    let five: Vec<_> = [
        "Plains",
        "Twin Bolt",
        "War Effort",
        "Forest",
        "Unburied Earthcarver",
    ]
    .iter()
    .map(|n| g.add_card(0, tid(n), Zone::Library))
    .collect();
    let (plains, bolt, effort, forest, creature) = (five[0], five[1], five[2], five[3], five[4]);
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Pick(bolt)),
        "permanent cards"
    );
    act(&mut g, Action::Pick(creature));
    act(&mut g, Action::Pick(forest));
    act(&mut g, Action::No);
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.obj(creature).zone, g.obj(forest).zone),
        (Zone::Battlefield, Zone::Battlefield)
    );
    for x in [plains, bolt, effort] {
        assert_eq!(g.obj(x).zone, Zone::Hand);
    }
    assert_eq!(g.obj(c).zone, Zone::Exile, "Exile Genesis Ultimatum");
}

#[test]
fn united_battlefront_puts_two_cheap_noncreature_permanents_down() {
    let mut g = table();
    let c = hand(&mut g, 0, "United Battlefront");
    begin(&mut g);
    let names = [
        "War Effort",
        "Stormbeacon Blade",
        "Mox Jasper",
        "Dragonologist",
        "Jeskai Monument",
        "Forest",
        "Twin Bolt",
    ];
    let seven: Vec<_> = names
        .iter()
        .map(|n| g.add_card(0, tid(n), Zone::Library))
        .collect();
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    let mut legal = g.legal_actions();
    legal.sort_by_key(|a| format!("{a:?}"));
    let mut want = vec![
        Action::Pick(seven[1]),
        Action::Pick(seven[2]),
        Action::Pick(seven[4]),
        Action::No,
    ];
    want.sort_by_key(|a| format!("{a:?}"));
    assert_eq!(legal, want, "noncreature, nonland, mana value 3 or less");
    act(&mut g, Action::Pick(seven[1]));
    act(&mut g, Action::Pick(seven[2]));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.obj(seven[1]).zone, g.obj(seven[2]).zone),
        (Zone::Battlefield, Zone::Battlefield)
    );
    let bottom = &g.players[0].library[..5];
    for x in [seven[0], seven[3], seven[4], seven[5], seven[6]] {
        assert!(bottom.contains(&x), "the rest on the bottom");
    }
}

#[test]
fn eerie_ultimatum_returns_permanents_with_different_names() {
    let mut g = table();
    let a = g.add_card(0, tid("Unburied Earthcarver"), Zone::Graveyard);
    let a2 = g.add_card(0, tid("Unburied Earthcarver"), Zone::Graveyard);
    let e = g.add_card(0, tid("War Effort"), Zone::Graveyard);
    let bolt = g.add_card(0, tid("Twin Bolt"), Zone::Graveyard);
    let c = hand(&mut g, 0, "Eerie Ultimatum");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Pick(bolt)),
        "permanent cards"
    );
    act(&mut g, Action::Pick(a));
    assert!(!g.legal_actions().contains(&Action::Pick(a2)), "different names");
    act(&mut g, Action::Pick(e));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.obj(a).zone, g.obj(e).zone),
        (Zone::Battlefield, Zone::Battlefield)
    );
    assert_eq!(
        (g.obj(a2).zone, g.obj(bolt).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
}

#[test]
fn tersa_lightshatter_rummages_up_to_two_and_digs_at_threshold() {
    let mut g = table();
    let t = hand(&mut g, 0, "Tersa Lightshatter");
    assert!(g.def_of(t).keywords.has(Keywords::HASTE));
    let x = hand(&mut g, 0, "Twin Bolt");
    let _y = hand(&mut g, 0, "Lightfoot Technique");
    begin(&mut g);
    let cards = g.players[0].hand.len(); // includes Tersa
    act(&mut g, Action::Cast(t));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Discard(x));
    act(&mut g, Action::No); // only one
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(x).zone, Zone::Graveyard);
    assert_eq!(
        g.players[0].hand.len(),
        cards - 1 - 1 + 1,
        "cast Tersa, discarded one, drew one"
    );

    for _ in 0..6 {
        g.add_card(0, tid("Twin Bolt"), Zone::Graveyard);
    }
    assert_eq!(g.players[0].graveyard.len(), 7);
    attack_with(&mut g, t);
    assert_eq!(g.players[0].graveyard.len(), 6, "one exiled at random");
    let exiled: Vec<_> = g.exile.iter().copied().filter(|&c| g.obj(c).owner == 0).collect();
    assert_eq!(exiled.len(), 1);
    assert_eq!(g.obj(exiled[0]).playable, Some((0, 5)), "playable this turn");
}

#[test]
fn tersa_without_seven_cards_exiles_nothing() {
    let mut g = table();
    let t = put(&mut g, 0, "Tersa Lightshatter");
    g.add_card(0, tid("Twin Bolt"), Zone::Graveyard);
    attack_with(&mut g, t);
    assert_eq!(g.players[0].graveyard.len(), 1);
}

#[test]
fn surrak_draws_when_an_opponent_targets_your_creature() {
    let mut g = table();
    let s = put(&mut g, 0, "Surrak, Elusive Hunter");
    assert!(g.def_of(s).uncounterable && g.keywords(s).has(Keywords::TRAMPLE));
    let mine = put(&mut g, 0, "Unburied Earthcarver");
    let own = hand(&mut g, 0, "Lightfoot Technique");
    begin(&mut g);
    let cards = g.players[0].hand.len();
    play(&mut g, own, false, &[obj(mine)], &[]);
    assert_eq!(g.players[0].hand.len(), cards - 1, "your own spell: no draw");

    let bolt = hand(&mut g, 1, "Channeled Dragonfire");
    g.start_turn(6, 1);
    while !(g.step == Step::Main1 && g.pending == Pending::Priority(1)) {
        act(&mut g, Action::PassPriority);
    }
    let cards = g.players[0].hand.len();
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(obj(mine)));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.players[0].hand.len(),
        cards + 1,
        "an opponent's spell targeted it"
    );
}

#[test]
fn kotis_exiles_by_damage_and_casts_cheap_spells_free() {
    let mut g = table();
    let k = put(&mut g, 0, "Kotis, the Fangkeeper"); // 2/1
    assert!(g.keywords(k).has(Keywords::INDESTRUCTIBLE));
    let cheap = g.add_card(1, tid("Twin Bolt"), Zone::Library); // MV 2
    let pricey = g.add_card(1, tid("War Effort"), Zone::Library); // MV 4, on top
    attack_with(&mut g, k);
    for c in g.permanents(0).collect::<Vec<_>>() {
        if g.types_of(c).land {
            g.objects[c.0 as usize].tapped = true; // no mana: the cast must be free
        }
    }
    while !matches!(g.pending, Pending::FreeCast(_)) {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(
        (g.obj(cheap).zone, g.obj(pricey).zone),
        (Zone::Exile, Zone::Exile),
        "two damage: top two"
    );
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Cast(cheap)));
    assert!(!legal.contains(&Action::Cast(pricey)), "mana value 2 or less");
    let life = g.players[1].life;
    act(&mut g, Action::Cast(cheap));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::No);
    act(&mut g, Action::PassPriority); // done casting
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 2);
}

#[test]
fn severance_priest_exiles_a_card_and_pays_its_owner_back_with_a_spirit() {
    let mut g = table();
    let pricey = hand(&mut g, 1, "War Effort"); // mana value 4
    let land = hand(&mut g, 1, "Forest");
    let p = hand(&mut g, 0, "Severance Priest");
    let bolt = hand(&mut g, 0, "Channeled Dragonfire");
    begin(&mut g);
    act(&mut g, Action::Cast(p));
    to_target_choice(&mut g);
    act(&mut g, Action::Target(Target::Player(1)));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(
        g.legal_actions(),
        [Action::Pick(pricey), Action::No],
        "nonland cards, or none"
    );
    act(&mut g, Action::Pick(pricey));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(pricey).zone, Zone::Exile);
    assert_eq!(g.obj(land).zone, Zone::Hand);
    assert!(tokens(&g, 1, "Spirit").is_empty());
    let bolt2 = hand(&mut g, 0, "Channeled Dragonfire");
    play(&mut g, bolt, false, &[obj(p)], &[]);
    play(&mut g, bolt2, false, &[obj(p)], &[]); // 4 damage: it dies, so it leaves the battlefield
    assert_eq!(g.obj(p).zone, Zone::Graveyard);
    let spirit = tokens(&g, 1, "Spirit");
    assert_eq!(spirit.len(), 1, "the exiled card's owner gets the Spirit");
    assert_eq!((g.power(spirit[0]), g.toughness(spirit[0])), (4, 4));
}

// ---- round 3, batch 5 ----

#[test]
fn ureni_divides_damage_equal_to_your_lands_among_their_creatures() {
    let mut g = tdm_lands(&[
        "Forest", "Island", "Mountain", "Forest", "Island", "Mountain", "Forest", "Island",
    ]);
    let u = hand(&mut g, 0, "Ureni, the Song Unending");
    let a = put(&mut g, 1, "Loxodon Battle Priest"); // 3/5
    let b = put(&mut g, 1, "Unburied Earthcarver"); // 2/2
    let mine = put(&mut g, 0, "Summit Intimidator");
    begin(&mut g);
    let k = g.keywords(u);
    assert!(k.has(Keywords::FLYING));
    act(&mut g, Action::Cast(u));
    to_target_choice(&mut g);
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(mine))),
        "your opponents' only"
    );
    act(&mut g, Action::Target(obj(a)));
    act(&mut g, Action::Target(obj(b)));
    act(&mut g, Action::No);
    assert!(matches!(g.pending, Pending::Divide(0)));
    assert_eq!(
        g.legal_actions().len(),
        7,
        "8 lands: 1 to 7 for the first, the rest to the second"
    );
    act(&mut g, Action::Amount(5));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(a).zone, Zone::Graveyard, "5 to the 3/5");
    assert_eq!(g.obj(b).zone, Zone::Graveyard, "3 to the 2/2");
}

#[test]
fn ureni_cant_take_more_targets_than_lands_and_has_protection() {
    let mut g = table();
    let u = put(&mut g, 0, "Ureni, the Song Unending");
    let a = put(&mut g, 1, "Unburied Earthcarver");
    let b = put(&mut g, 1, "Summit Intimidator");
    // Protection from white: a white spell can't target it.
    let white = hand(&mut g, 1, "Lightfoot Technique");
    g.start_turn(6, 1);
    while !(g.step == Step::Main1 && g.pending == Pending::Priority(1)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Cast(white));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(u))),
        "protection from white"
    );
    act(&mut g, Action::Target(obj(a)));
    resolve_all(&mut g, &[]);
    // One land: one target at most.
    let mut g2 = tdm_lands(&["Forest"]);
    let u2 = hand(&mut g2, 0, "Ureni, the Song Unending");
    let x = put(&mut g2, 1, "Unburied Earthcarver");
    let _y = put(&mut g2, 1, "Summit Intimidator");
    begin(&mut g2);
    g2.move_card(u2, Zone::Battlefield);
    to_target_choice(&mut g2);
    act(&mut g2, Action::Target(obj(x)));
    let legal = g2.legal_actions();
    assert!(
        legal.iter().all(|a| !matches!(a, Action::Target(_))),
        "no second target with X = 1: {legal:?}"
    );
    act(&mut g2, Action::No);
    resolve_all(&mut g2, &[]);
    assert_eq!(g2.obj(x).damage, 1);
    let _ = b;
}

#[test]
fn sidisi_trades_a_creature_for_one_with_mana_value_one_more() {
    let mut g = table();
    let s = put(&mut g, 0, "Sidisi, Regent of the Mire");
    let two = put(&mut g, 0, "Unburied Earthcarver"); // mana value 2
    let three = g.add_card(0, tid("Krotiq Nestguard"), Zone::Graveyard); // mana value 3
    let four = g.add_card(0, tid("Summit Intimidator"), Zone::Graveyard); // mana value 4
    main_phase_turn_5(&mut g);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::ActivateX {
        id: s,
        n: 0,
        victim: Some(two),
        x: 3
    }));
    assert!(
        !legal
            .iter()
            .any(|a| matches!(a, Action::ActivateX { victim: Some(v), .. } if *v == s)),
        "other than Sidisi"
    );
    act(
        &mut g,
        Action::ActivateX {
            id: s,
            n: 0,
            victim: Some(two),
            x: 3,
        },
    );
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(four))),
        "mana value X plus 1"
    );
    act(&mut g, Action::Target(obj(three)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(two).zone, Zone::Graveyard, "sacrificed");
    assert_eq!(g.obj(three).zone, Zone::Battlefield);
    assert_eq!(g.obj(four).zone, Zone::Graveyard);
    assert!(g.obj(s).tapped);
}

#[test]
fn sidisi_needs_a_matching_card_in_the_graveyard() {
    let mut g = table();
    let s = put(&mut g, 0, "Sidisi, Regent of the Mire");
    put(&mut g, 0, "Unburied Earthcarver");
    g.add_card(0, tid("Summit Intimidator"), Zone::Graveyard); // mana value 4, not 3
    main_phase_turn_5(&mut g);
    assert!(!g
        .legal_actions()
        .iter()
        .any(|a| matches!(a, Action::ActivateX { id, .. } if *id == s)));
}

#[test]
fn dragonbroods_relic_taps_a_creature_for_any_color_and_makes_a_reliquary_dragon() {
    let mut g = tdm_lands(&[]);
    let relic = put(&mut g, 0, "Dragonbroods' Relic");
    begin(&mut g);
    assert!(
        g.mana_sources(0).iter().all(|(x, _)| *x != relic),
        "no creature to tap"
    );
    let c = put(&mut g, 0, "Unburied Earthcarver");
    let from: Vec<_> = g
        .mana_sources(0)
        .into_iter()
        .filter(|(x, _)| *x == relic)
        .collect();
    assert_eq!(from.len(), 1);
    assert_eq!(from[0].1.colors, [true; 5]);
    let spell = hand(&mut g, 0, "Channeled Dragonfire"); // {R}
    play(&mut g, spell, false, &[Target::Player(1)], &[]);
    assert!(
        g.obj(relic).tapped && g.obj(c).tapped,
        "the Relic and a creature tapped"
    );
    assert_eq!(g.players[1].life, 18);

    let mut g = table();
    let relic = put(&mut g, 0, "Dragonbroods' Relic");
    let theirs = put(&mut g, 1, "Unburied Earthcarver");
    begin(&mut g);
    let life = g.players[0].life;
    act(&mut g, Action::Activate(relic, 0)); // the mana ability is not an activated-ability slot
    resolve_all(&mut g, &[Action::Target(obj(theirs))]);
    assert_eq!(g.obj(relic).zone, Zone::Graveyard, "sacrificed");
    let d = tokens(&g, 0, "Reliquary Dragon");
    assert_eq!(d.len(), 1);
    assert_eq!((g.power(d[0]), g.toughness(d[0])), (4, 4));
    let k = g.keywords(d[0]);
    assert!(k.has(Keywords::FLYING) && k.has(Keywords::LIFELINK));
    assert_eq!(g.colors_of(d[0]), [true; 5], "all colors");
    assert_eq!(
        g.obj(theirs).zone,
        Zone::Graveyard,
        "its enters trigger: 3 damage"
    );
    assert_eq!(g.players[0].life, life + 3, "lifelink on the trigger's damage");
}

#[test]
fn all_out_assault_adds_a_combat_and_untaps_on_the_next_attack() {
    let mut g = table();
    let a = put(&mut g, 0, "Unburied Earthcarver");
    let theirs = put(&mut g, 1, "Unburied Earthcarver");
    let c = hand(&mut g, 0, "All-Out Assault");
    main_phase_turn_5(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!((g.power(a), g.toughness(a)), (3, 3));
    assert!(g.keywords(a).has(Keywords::DEATHTOUCH));
    assert_eq!(g.power(theirs), 2, "yours only");
    let life = g.players[1].life;
    // The additional combat: attack, and the creature untaps.
    to_declare_attackers(&mut g);
    act(&mut g, Action::Attack(a));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[]);
    assert!(!g.obj(a).tapped, "untapped when you next attacked");
    through_combat_damage_or_main(&mut g);
    assert_eq!(g.step, Step::Main2, "the additional main phase");
    assert_eq!(g.players[1].life, life - 3);
    // Then the turn's regular combat: it can attack again, and doesn't untap this time.
    to_declare_attackers(&mut g);
    act(&mut g, Action::Attack(a));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[]);
    assert!(g.obj(a).tapped, "only the next attack");
    through_combat_damage_or_main(&mut g);
    assert_eq!(g.step, Step::Main2);
    assert_eq!(g.players[1].life, life - 6);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert_eq!(g.step, Step::End, "no more combats");
}

#[test]
fn all_out_assault_outside_a_main_phase_adds_no_combat() {
    let mut g = table();
    let c = hand(&mut g, 0, "All-Out Assault");
    begin(&mut g);
    g.move_card(c, Zone::Battlefield); // enters during your main phase: the trigger checks
    let mut g2 = g.clone();
    resolve_pending_triggers(&mut g2);
    assert_eq!(g2.extra_combats.len(), 1, "your main phase");
    // On the opponent's turn it does nothing.
    let mut g3 = table();
    let aoa = hand(&mut g3, 0, "All-Out Assault");
    g3.start_turn(6, 1);
    while !(g3.step == Step::Main1 && g3.pending == Pending::Priority(1)) {
        act(&mut g3, Action::PassPriority);
    }
    g3.move_card(aoa, Zone::Battlefield);
    resolve_pending_triggers(&mut g3);
    assert!(g3.extra_combats.is_empty(), "not your main phase");
}

/// Pass until the triggered abilities waiting to go on the stack have resolved.
fn resolve_pending_triggers(g: &mut Game) {
    act(g, Action::PassPriority); // the next priority puts them on the stack
    resolve_all(g, &[]);
}

fn to_declare_attackers(g: &mut Game) {
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
}

/// Pass through combat (blocking nothing) to the main phase after it.
fn through_combat_damage_or_main(g: &mut Game) {
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn maelstrom_makes_any_color_only_for_dragons_and_omens() {
    let mut g = tdm_lands(&["Maelstrom of the Spirit Dragon"]);
    let land = g.permanents(0).next().unwrap();
    let omen = hand(&mut g, 0, "Sagu Wildling"); // Omen: Roost Seek, {G}
    let red = hand(&mut g, 0, "Channeled Dragonfire"); // {R}
    begin(&mut g);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::CastOmen(omen)), "an Omen spell");
    assert!(!legal.contains(&Action::Cast(red)), "not other spells");
    let forest = g.add_card(0, tid("Forest"), Zone::Library);
    act(&mut g, Action::CastOmen(omen));
    resolve_all(&mut g, &[Action::Pick(forest)]);
    assert!(g.obj(land).tapped);

    // {C} for anything; any color for a Dragon spell.
    let mut g = tdm_lands(&[
        "Maelstrom of the Spirit Dragon",
        "Maelstrom of the Spirit Dragon",
        "Maelstrom of the Spirit Dragon",
        "Maelstrom of the Spirit Dragon",
        "Maelstrom of the Spirit Dragon",
    ]);
    let monument = hand(&mut g, 0, "Jeskai Monument"); // {2}
    let dragon = hand(&mut g, 0, "Sagu Wildling"); // {4}{G} Dragon
    begin(&mut g);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Cast(monument)) && legal.contains(&Action::Cast(dragon)));
    act(&mut g, Action::Cast(dragon));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(dragon).zone, Zone::Battlefield);
}

#[test]
fn maelstrom_sacrifices_to_find_a_dragon() {
    let mut g = table();
    let m = put(&mut g, 0, "Maelstrom of the Spirit Dragon");
    begin(&mut g);
    let d = g.add_card(0, tid("Boulderborn Dragon"), Zone::Library);
    act(&mut g, Action::Activate(m, 0));
    resolve_all(&mut g, &[Action::Pick(d)]);
    assert_eq!(g.obj(d).zone, Zone::Hand);
    assert_eq!(g.obj(m).zone, Zone::Graveyard);
}

#[test]
fn flamehold_grappler_copies_the_next_spell_with_new_targets() {
    let mut g = table();
    let fg = hand(&mut g, 0, "Flamehold Grappler");
    let theirs = put(&mut g, 1, "Unburied Earthcarver"); // 2/2
    let bolt = hand(&mut g, 0, "Channeled Dragonfire"); // 2 damage to any target
    let later = hand(&mut g, 0, "Channeled Dragonfire");
    begin(&mut g);
    assert!(g.def_of(fg).keywords.has(Keywords::FIRST_STRIKE));
    play(&mut g, fg, false, &[], &[]);
    let life = g.players[1].life;
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[Action::Target(obj(theirs))]); // the copy's new target
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "the copy hit the creature");
    assert_eq!(g.players[1].life, life - 2, "the original hit the player");
    play(&mut g, later, false, &[Target::Player(1)], &[]);
    assert_eq!(g.players[1].life, life - 4, "only the next spell is copied");
}

#[test]
fn flamehold_grappler_copies_a_creature_spell_as_a_token() {
    let mut g = table();
    let fg = hand(&mut g, 0, "Flamehold Grappler");
    let c = hand(&mut g, 0, "Unburied Earthcarver");
    begin(&mut g);
    play(&mut g, fg, false, &[], &[]);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(c).zone, Zone::Battlefield);
    let copies: Vec<_> = g
        .permanents(0)
        .filter(|&x| x != c && g.def_of(x).name == "Unburied Earthcarver")
        .collect();
    assert_eq!(copies.len(), 1);
    assert!(
        g.is_token(copies[0]),
        "a copy of a permanent spell becomes a token"
    );
}

#[test]
fn ugin_exiles_a_colored_permanent_when_cast_and_on_colorless_spells() {
    let mut g = table();
    let ugin = hand(&mut g, 0, "Ugin, Eye of the Storms");
    let red = put(&mut g, 1, "War Effort");
    let artifact = put(&mut g, 1, "Jeskai Monument"); // colorless
    let creature = put(&mut g, 1, "Unburied Earthcarver");
    let land = g.permanents(1).find(|&x| g.types_of(x).land).unwrap();
    let mono = hand(&mut g, 0, "Jeskai Monument");
    let colored = hand(&mut g, 0, "Lightfoot Technique");
    begin(&mut g);
    act(&mut g, Action::Cast(ugin));
    to_target_choice(&mut g);
    let legal = g.legal_actions();
    assert!(
        !legal.contains(&Action::Target(obj(artifact))),
        "one or more colors"
    );
    assert!(!legal.contains(&Action::Target(obj(land))), "lands are colorless");
    act(&mut g, Action::Target(obj(red)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(red).zone, Zone::Exile, "the cast trigger");
    assert_eq!(g.obj(ugin).zone, Zone::Battlefield);
    assert_eq!(g.obj(ugin).loyalty, 7);

    // The Monument's own enters search finds nothing (No).
    play(
        &mut g,
        mono,
        false,
        &[],
        &[Action::Target(obj(creature)), Action::No],
    );
    assert_eq!(
        g.obj(creature).zone,
        Zone::Exile,
        "a colorless spell: exile another"
    );
    let mine = put(&mut g, 0, "Unburied Earthcarver");
    put(&mut g, 0, "Plains"); // the Plains paid for Ugin
    play(&mut g, colored, false, &[obj(mine)], &[]);
    assert!(
        g.items.is_empty() && g.obj(mine).counters == 1,
        "a colored spell: no trigger"
    );
}

#[test]
fn ugin_loyalty_abilities() {
    let mut g = table();
    let u = hand(&mut g, 0, "Ugin, Eye of the Storms");
    begin(&mut g);
    g.move_card(u, Zone::Battlefield);
    let (life, cards) = (g.players[0].life, g.players[0].hand.len());
    act(&mut g, Action::Loyalty(u, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.players[0].life, g.players[0].hand.len()),
        (life + 3, cards + 1),
        "+2"
    );
    assert_eq!(g.obj(u).loyalty, 9);

    let mut g2 = table();
    let u2 = hand(&mut g2, 0, "Ugin, Eye of the Storms");
    begin(&mut g2);
    g2.move_card(u2, Zone::Battlefield);
    act(&mut g2, Action::Loyalty(u2, 1));
    resolve_all(&mut g2, &[]);
    assert_eq!(g2.players[0].pool.len(), 3, "0: {{C}}{{C}}{{C}}");
    assert!(g2.players[0].pool.iter().all(|m| m.colorless));

    let mut g3 = tdm_lands(&[]);
    let u3 = hand(&mut g3, 0, "Ugin, Eye of the Storms");
    begin(&mut g3);
    g3.move_card(u3, Zone::Battlefield);
    g3.objects[u3.0 as usize].loyalty = 11;
    let dragon = g3.add_card(0, tid("Boulderborn Dragon"), Zone::Library); // a colorless artifact creature
    let mox = g3.add_card(0, tid("Mox Jasper"), Zone::Library);
    let colored = g3.add_card(0, tid("Unburied Earthcarver"), Zone::Library);
    act(&mut g3, Action::Loyalty(u3, 2));
    while !matches!(g3.pending, Pending::Decide(_)) {
        act(&mut g3, Action::PassPriority);
    }
    assert!(
        !g3.legal_actions().contains(&Action::Pick(colored)),
        "colorless nonland cards"
    );
    act(&mut g3, Action::Pick(dragon));
    act(&mut g3, Action::Pick(mox)); // the last match: the search ends
    resolve_all(&mut g3, &[]);
    assert_eq!(
        (g3.obj(dragon).zone, g3.obj(mox).zone),
        (Zone::Exile, Zone::Exile)
    );
    assert!(
        g3.legal_actions().contains(&Action::Cast(dragon)),
        "free this turn, with no lands"
    );
    act(&mut g3, Action::Cast(dragon));
    resolve_all(&mut g3, &[]);
    assert_eq!(g3.obj(dragon).zone, Zone::Battlefield);
}

// ---- round 3, batch 7: uncommons ----

#[test]
fn spectral_denial_costs_less_per_big_creature_and_counters_unless_x_is_paid() {
    let mut g = table();
    put(&mut g, 0, "Summit Intimidator"); // power 4
    put(&mut g, 0, "Unburied Earthcarver"); // power 2: doesn't count
    let bolt = hand(&mut g, 0, "Twin Bolt");
    let d = hand(&mut g, 0, "Spectral Denial");
    begin(&mut g);
    let life = g.players[1].life;
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::No);
    let n = paid(&mut g, |g| {
        act(g, Action::CastX(d, 2));
        act(g, Action::Target(obj(bolt)));
    });
    assert_eq!(
        n, 2,
        "{{2}}{{U}} minus {{1}} for one creature with power 4 or greater"
    );
    resolve_all(&mut g, &[Action::No]); // the bolt's controller doesn't pay {2}
    assert_eq!(g.obj(bolt).zone, Zone::Graveyard);
    assert_eq!(g.players[1].life, life, "countered");
}

#[test]
fn rally_the_monastery_modes_and_discount() {
    let mut g = table();
    let big = put(&mut g, 1, "Summit Intimidator"); // power 4
    let small = put(&mut g, 1, "Unburied Earthcarver");
    let r = hand(&mut g, 0, "Rally the Monastery");
    begin(&mut g);
    let n = paid(&mut g, |g| {
        act(g, Action::CastModal(r, 0));
        resolve_all(g, &[]);
    });
    assert_eq!(n, 4, "full price as the first spell");
    let monks = tokens(&g, 0, "Monk");
    assert_eq!(monks.len(), 2);
    assert_eq!((g.power(monks[0]), g.toughness(monks[0])), (1, 1));

    let r2 = hand(&mut g, 0, "Rally the Monastery");
    let n = paid(&mut g, |g| {
        act(g, Action::CastModal(r2, 2));
        assert!(
            !g.legal_actions().contains(&Action::Target(obj(small))),
            "power 4 or greater"
        );
        act(g, Action::Target(obj(big)));
        resolve_all(g, &[]);
    });
    assert_eq!(n, 2, "{{2}} less after another spell");
    assert_eq!(g.obj(big).zone, Zone::Graveyard);

    put(&mut g, 0, "Plains"); // the payer spent the Plains first
    let r3 = hand(&mut g, 0, "Rally the Monastery");
    act(&mut g, Action::CastModal(r3, 1));
    act(&mut g, Action::Target(obj(monks[0])));
    act(&mut g, Action::Target(obj(monks[1])));
    resolve_all(&mut g, &[]);
    assert!(g.power(monks[0]) >= 3 && g.toughness(monks[1]) >= 3, "+2/+2 each");
}

#[test]
fn glacial_dragonhunt_loots_and_shoots_for_a_nonland_discard() {
    let mut g = table();
    let theirs = put(&mut g, 1, "Summit Intimidator"); // 4/3
    let spell = hand(&mut g, 0, "Twin Bolt");
    let land = hand(&mut g, 0, "Forest");
    let c = hand(&mut g, 0, "Glacial Dragonhunt");
    let c2 = hand(&mut g, 0, "Glacial Dragonhunt");
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Discard(land)]);
    assert_eq!(g.obj(land).zone, Zone::Graveyard);
    assert_eq!(g.obj(theirs).damage, 0, "a land: no damage");
    play(
        &mut g,
        c2,
        false,
        &[],
        &[Action::Discard(spell), Action::Target(obj(theirs))],
    );
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "3 damage");
    assert!(g.def_of(c).harmonize);
}

#[test]
fn synchronized_charge_distributes_counters_and_pumps_countered_creatures() {
    let mut g = table();
    let a = put(&mut g, 0, "Unburied Earthcarver");
    let b = put(&mut g, 0, "Summit Intimidator");
    let had = put(&mut g, 0, "Arashin Sunshield");
    let none = put(&mut g, 0, "Loxodon Battle Priest");
    let shield = hand(&mut g, 0, "Lightfoot Technique");
    let c = hand(&mut g, 0, "Synchronized Charge");
    begin(&mut g);
    play(&mut g, shield, false, &[obj(had)], &[]);
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(a)));
    act(&mut g, Action::Target(obj(b)));
    act(&mut g, Action::Amount(1));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(a).counters, g.obj(b).counters), (1, 1));
    for x in [a, b, had] {
        let k = g.keywords(x);
        assert!(k.has(Keywords::VIGILANCE) && k.has(Keywords::TRAMPLE));
    }
    assert!(!g.keywords(none).has(Keywords::TRAMPLE), "no counters");
}

#[test]
fn winternight_stories_draws_three_then_discards_a_creature_or_two_cards() {
    let mut g = table();
    let creature = hand(&mut g, 0, "Unburied Earthcarver");
    let c = hand(&mut g, 0, "Winternight Stories");
    begin(&mut g);
    let n = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[Action::Discard(creature)]);
    assert_eq!(
        g.players[0].hand.len(),
        n - 1 + 3 - 1,
        "a creature card: just one"
    );

    let c2 = hand(&mut g, 0, "Winternight Stories");
    let spell = hand(&mut g, 0, "Twin Bolt");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Cast(c2));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Discard(spell));
    assert!(
        matches!(g.pending, Pending::Decide(_)),
        "a noncreature card: discard another"
    );
    let other = g.players[0].hand[0];
    act(&mut g, Action::Discard(other));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n - 1 + 3 - 2);
}

#[test]
fn krumar_initiate_pays_x_life_to_endure_x() {
    let mut g = table();
    let k = put(&mut g, 0, "Krumar Initiate");
    main_phase_turn_5(&mut g);
    g.players[0].life = 3;
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::ActivateX {
        id: k,
        n: 0,
        victim: None,
        x: 3
    }));
    assert!(
        !legal.contains(&Action::ActivateX {
            id: k,
            n: 0,
            victim: None,
            x: 4
        }),
        "X life at most your life"
    );
    let n = paid(&mut g, |g| {
        act(
            g,
            Action::ActivateX {
                id: k,
                n: 0,
                victim: None,
                x: 2,
            },
        );
        resolve_all(g, &[Action::Yes]); // counters, not a Spirit
    });
    assert_eq!(n, 3, "{{2}}{{B}}");
    assert_eq!(g.players[0].life, 1);
    assert_eq!(g.obj(k).counters, 2);
    assert!(g.obj(k).tapped);
}

#[test]
fn sinkhole_surveyor_pays_a_life_to_endure_when_attacking() {
    let mut g = table();
    let s = put(&mut g, 0, "Sinkhole Surveyor");
    assert!(g.keywords(s).has(Keywords::FLYING));
    let life = g.players[0].life;
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(s));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.players[0].life, life - 1);
    assert_eq!(g.obj(s).counters, 1);
}

#[test]
fn qarsi_revenant_renews_three_keyword_counters() {
    let mut g = table();
    let q = g.add_card(0, tid("Qarsi Revenant"), Zone::Graveyard);
    let mine = put(&mut g, 0, "Unburied Earthcarver");
    begin(&mut g);
    act(&mut g, Action::Activate(q, 0));
    act(&mut g, Action::Target(obj(mine)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(q).zone, Zone::Exile);
    let k = g.keywords(mine);
    assert!(k.has(Keywords::FLYING) && k.has(Keywords::DEATHTOUCH) && k.has(Keywords::LIFELINK));
}

#[test]
fn stadium_headliner_sacrifices_to_deal_damage_per_creature() {
    let mut g = table();
    let h = put(&mut g, 0, "Stadium Headliner");
    put(&mut g, 0, "Unburied Earthcarver");
    put(&mut g, 0, "Summit Intimidator");
    let theirs = put(&mut g, 1, "Arashin Sunshield"); // 3/4
    begin(&mut g);
    act(&mut g, Action::Activate(h, 0));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(h).zone, Zone::Graveyard);
    assert_eq!(g.obj(theirs).damage, 2, "two creatures left once it's sacrificed");
}

#[test]
fn lasyd_prowler_mills_by_lands_and_renews_by_land_cards() {
    let mut g = tdm_lands(&["Forest", "Forest", "Forest", "Forest"]);
    let p = hand(&mut g, 0, "Lasyd Prowler");
    begin(&mut g);
    let lib = g.players[0].library.len();
    play(&mut g, p, false, &[], &[Action::Yes]);
    assert_eq!(g.players[0].library.len(), lib - 4, "four lands");
    let lands_in_yard = g.players[0]
        .graveyard
        .iter()
        .filter(|&&c| g.def_of(c).types.land)
        .count() as i32;
    assert_eq!(lands_in_yard, 4, "the library is Plains");
    let p2 = g.add_card(0, tid("Lasyd Prowler"), Zone::Graveyard);
    let mine = put(&mut g, 0, "Unburied Earthcarver");
    for f in g
        .permanents(0)
        .filter(|&x| g.types_of(x).land)
        .collect::<Vec<_>>()
    {
        g.objects[f.0 as usize].tapped = false;
    }
    act(&mut g, Action::Activate(p2, 0));
    act(&mut g, Action::Target(obj(mine)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(mine).counters, lands_in_yard);
}

#[test]
fn warden_of_the_grove_grows_and_makes_newcomers_endure() {
    let mut g = table();
    let w = put(&mut g, 0, "Warden of the Grove");
    let c = hand(&mut g, 0, "Unburied Earthcarver");
    begin(&mut g);
    to_next_turn(&mut g);
    assert_eq!(g.obj(w).counters, 1, "your end step");
    to_next_turn(&mut g);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    play(&mut g, c, false, &[], &[Action::Yes]);
    assert_eq!(g.obj(c).counters, 1, "endures X = 1");
    let goblin = g.add_card(0, set_token(&g, "Goblin"), Zone::Battlefield);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(goblin).counters, 0, "nontoken only");
}

#[test]
fn cori_steel_cutter_equips_and_makes_a_monk_on_the_second_spell() {
    let mut g = table();
    let cutter = put(&mut g, 0, "Cori-Steel Cutter");
    let a = hand(&mut g, 0, "Lightfoot Technique");
    let b = hand(&mut g, 0, "Lightfoot Technique");
    let mine = put(&mut g, 0, "Unburied Earthcarver");
    begin(&mut g);
    play(&mut g, a, false, &[obj(mine)], &[]);
    assert!(tokens(&g, 0, "Monk").is_empty(), "the first spell");
    put(&mut g, 0, "Plains");
    play(&mut g, b, false, &[obj(mine)], &[Action::Yes]);
    let monk = tokens(&g, 0, "Monk");
    assert_eq!(monk.len(), 1);
    assert_eq!(
        g.obj(cutter).attached_to.map(|(c, _)| c),
        Some(monk[0]),
        "attached to it"
    );
    let k = g.keywords(monk[0]);
    assert!(k.has(Keywords::TRAMPLE) && k.has(Keywords::HASTE));
    assert_eq!(g.toughness(monk[0]), 2, "1/1 plus +1/+1");
}

#[test]
fn songcrafter_mage_gives_a_graveyard_spell_harmonize_for_its_mana_cost() {
    let mut g = table();
    let spell = g.add_card(0, tid("Channeled Dragonfire"), Zone::Graveyard); // {R}, harmonize {5}{R}{R}
    let lf = g.add_card(0, tid("Lightfoot Technique"), Zone::Graveyard); // {1}{W}, no harmonize
    let m = hand(&mut g, 0, "Songcrafter Mage");
    begin(&mut g);
    assert!(g.def_of(m).keywords.has(Keywords::FLASH));
    assert!(!g.legal_actions().contains(&Action::CastFlashback(lf)));
    play(&mut g, m, false, &[], &[Action::Target(obj(lf))]);
    assert!(
        g.legal_actions().contains(&Action::CastFlashback(lf)),
        "harmonize until end of turn"
    );
    let mine = put(&mut g, 0, "Unburied Earthcarver");
    put(&mut g, 0, "Plains");
    let n = paid(&mut g, |g| {
        act(g, Action::CastFlashback(lf));
        act(g, Action::Target(obj(mine)));
        resolve_all(g, &[]);
    });
    assert_eq!(n, 2, "its mana cost, {{1}}{{W}}");
    assert_eq!(g.obj(lf).zone, Zone::Exile, "then exiled");
    assert_eq!(g.obj(spell).zone, Zone::Graveyard);
}

// ---- round 3, batch 8 ----

#[test]
fn anafenza_endures_when_another_nontoken_creature_dies() {
    let mut g = table();
    let a = put(&mut g, 0, "Anafenza, Unyielding Lineage");
    let ks = g.def_of(a).keywords;
    assert!(ks.has(Keywords::FLASH) && ks.has(Keywords::FIRST_STRIKE));
    let e = put(&mut g, 0, "Unburied Earthcarver");
    let fodder = put(&mut g, 0, "Summit Intimidator");
    let goblin = g.add_card(0, set_token(&g, "Goblin"), Zone::Battlefield);
    begin(&mut g);
    act(&mut g, Action::ActivateWith(e, 0, fodder));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.obj(a).counters, 2);
    act(&mut g, Action::ActivateWith(e, 0, goblin));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(a).counters, 2, "not for a token");
}

#[test]
fn voice_of_victory_stops_opponents_casting_on_your_turn() {
    let mut g = table();
    let v = put(&mut g, 0, "Voice of Victory");
    let instant = hand(&mut g, 1, "Twin Bolt");
    begin(&mut g);
    let mine = hand(&mut g, 0, "Lightfoot Technique");
    act(&mut g, Action::Cast(mine));
    act(&mut g, Action::Target(obj(v)));
    act(&mut g, Action::PassPriority); // to the opponent, with a spell on the stack
    assert_eq!(g.pending, Pending::Priority(1));
    assert!(
        !g.legal_actions().contains(&Action::Cast(instant)),
        "not during your turn"
    );
    resolve_all(&mut g, &[]);
    g.start_turn(6, 1);
    while !(g.step == Step::Main1 && g.pending == Pending::Priority(1)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        g.legal_actions().contains(&Action::Cast(instant)),
        "on their own turn"
    );
    attack_with(&mut g, v);
}

#[test]
fn zurgo_keeps_warrior_tokens_through_your_end_step() {
    let mut g = table();
    let z = put(&mut g, 0, "Zurgo, Thunder's Decree");
    attack_with(&mut g, z);
    assert_eq!(tokens(&g, 0, "Warrior").len(), 2, "mobilize 2");
    while g.step != Step::End {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(
        tokens(&g, 0, "Warrior").len(),
        2,
        "they can't be sacrificed during your end step"
    );
    to_next_turn(&mut g);
    assert_eq!(tokens(&g, 0, "Warrior").len(), 2, "and stay");
}

#[test]
fn the_sibsig_ceremony_discounts_creatures_and_trades_cast_ones_for_zombies() {
    let mut g = table();
    put(&mut g, 0, "The Sibsig Ceremony");
    let c = hand(&mut g, 0, "Summit Intimidator"); // {3}{R}
    let theirs = put(&mut g, 1, "Unburied Earthcarver");
    begin(&mut g);
    let n = paid(&mut g, |g| {
        act(g, Action::Cast(c));
        resolve_all(g, &[Action::Target(obj(theirs))]);
    });
    assert_eq!(n, 2, "{{2}} less");
    assert_eq!(g.obj(c).zone, Zone::Graveyard, "destroyed: you cast it");
    assert_eq!(tokens(&g, 0, "Zombie Druid").len(), 1);
    let put_in = g.add_card(0, tid("Unburied Earthcarver"), Zone::Hand);
    g.move_card(put_in, Zone::Battlefield);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(put_in).zone, Zone::Battlefield, "not cast: it stays");
    assert_eq!(tokens(&g, 0, "Zombie Druid").len(), 1);
}

#[test]
fn rite_of_renewal_returns_two_and_shuffles_four_back() {
    let mut g = table();
    let a = g.add_card(0, tid("Unburied Earthcarver"), Zone::Graveyard);
    let b = g.add_card(0, tid("War Effort"), Zone::Graveyard);
    let spell = g.add_card(0, tid("Twin Bolt"), Zone::Graveyard);
    let theirs: Vec<_> = (0..5)
        .map(|_| g.add_card(1, tid("Twin Bolt"), Zone::Graveyard))
        .collect();
    let c = hand(&mut g, 0, "Rite of Renewal");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(Target::Player(1)));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Pick(spell)),
        "permanent cards"
    );
    act(&mut g, Action::Pick(a));
    act(&mut g, Action::Pick(b));
    let lib = g.players[1].library.len();
    for &t in &theirs[..4] {
        act(&mut g, Action::Pick(t));
    }
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(a).zone, g.obj(b).zone), (Zone::Hand, Zone::Hand));
    assert_eq!(g.players[1].library.len(), lib + 4);
    assert_eq!(g.obj(theirs[4]).zone, Zone::Graveyard, "up to four");
    assert_eq!(g.obj(c).zone, Zone::Exile);
}
