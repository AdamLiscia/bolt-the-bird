//! Core theme "triggers" (lane d2): each new piece tested directly, on a card definition edited to
//! carry it, in a 1v1 Commander game on the CMD pool.
#![allow(dead_code)]

mod common;

use std::sync::Arc;

use bolt_cards::set;
use bolt_core::effect::{Ability, Condition, Effect, Trigger, When, Who};
use bolt_core::{Action, CardDef, Game, ObjId, Step, Zone};
use common::{act, obj, play, resolve_all};

fn cid(name: &str) -> usize {
    set("CMD")
        .unwrap()
        .index_of(name)
        .unwrap_or_else(|| panic!("{name} is not in the CMD pool"))
}

/// A Commander game on the CMD pool with `edit` applied to the definitions, at player 0's first
/// main phase on turn 3, with 5 of each basic for both players.
fn game_with(edit: impl Fn(&mut Vec<CardDef>)) -> Game {
    let mut defs: Vec<CardDef> = (*set("CMD").unwrap().defs).clone();
    edit(&mut defs);
    let plains = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        Arc::new(defs),
        [&plains, &plains],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    for p in 0..2 {
        for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            for _ in 0..5 {
                g.add_card(p, cid(land), Zone::Battlefield);
            }
        }
    }
    g.start_turn(3, 0);
    to_main(&mut g);
    g
}

fn to_main(g: &mut Game) {
    while g.step != Step::Main1 {
        act(g, Action::PassPriority);
    }
}

fn trigger(when: When, condition: Option<Condition>, effects: Vec<Effect>) -> Trigger {
    let mut ability = Ability::new(vec![], effects);
    ability.condition = condition;
    Trigger { when, ability }
}

fn gain_one() -> Vec<Effect> {
    vec![Effect::GainLife { who: Who::You, n: 1 }]
}

/// Sol Ring, edited not to untap and to carry `t`, so it can be left tapped.
fn ring_with(t: Trigger) -> impl Fn(&mut Vec<CardDef>) {
    move |defs: &mut Vec<CardDef>| {
        let d = &mut defs[cid("Sol Ring")];
        d.doesnt_untap = true;
        d.triggers.push(t.clone());
    }
}

/// Taps a mana source for its mana (which then empties unused).
fn tap(g: &mut Game, id: ObjId) {
    g.manual_mana[0] = true;
    act(g, Action::TapForMana(id, 5)); // {C}
    g.manual_mana[0] = false;
    assert!(g.obj(id).tapped);
}

#[test]
fn source_tapped_and_your_turn_gate_a_draw_step_trigger() {
    let yours_tapped = Condition::Both(Box::new(Condition::YourTurn), Box::new(Condition::SourceTapped));
    let mut g = game_with(ring_with(trigger(
        When::EachDrawStep,
        Some(yours_tapped),
        gain_one(),
    )));
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    g.start_turn(4, 0);
    to_main(&mut g);
    assert_eq!(g.players[0].life, 40, "untapped: no trigger");
    tap(&mut g, ring);
    g.start_turn(5, 1);
    to_main(&mut g);
    assert_eq!(g.players[0].life, 40, "the opponent's draw step isn't yours");
    g.start_turn(6, 0);
    to_main(&mut g);
    assert!(g.obj(ring).tapped, "it doesn't untap");
    assert_eq!(g.players[0].life, 41, "your draw step, tapped");
}

#[test]
fn source_untapped_gates_a_trigger_on_each_players_draw_step() {
    let mut g = game_with(ring_with(trigger(
        When::EachDrawStep,
        Some(Condition::SourceUntapped),
        gain_one(),
    )));
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    g.start_turn(4, 1);
    to_main(&mut g);
    g.start_turn(5, 0);
    to_main(&mut g);
    assert_eq!(g.players[0].life, 42, "both draw steps while untapped");
    tap(&mut g, ring);
    g.start_turn(6, 1);
    to_main(&mut g);
    assert_eq!(g.players[0].life, 42, "tapped: no trigger");
}

#[test]
fn control_commander_needs_your_own_commander_on_the_battlefield() {
    let mut g = game_with(ring_with(trigger(
        When::BeginCombatYourTurn,
        Some(Condition::ControlCommander),
        gain_one(),
    )));
    g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    let to_combat = |g: &mut Game| {
        while g.step != Step::BeginCombat {
            act(g, Action::PassPriority);
        }
        resolve_all(g, &[]);
    };
    to_combat(&mut g);
    assert_eq!(g.players[0].life, 40, "no commander out");
    let bello = g.add_card(0, cid("Bello, Bard of the Brambles"), Zone::Battlefield);
    g.commanders[0] = Some(bello);
    g.start_turn(5, 0);
    to_combat(&mut g);
    assert_eq!(g.players[0].life, 41, "your commander out");
}

#[test]
fn gain_life_and_draw_source_power_uses_the_power_it_died_with() {
    let mut g = game_with(|defs| {
        let t = trigger(When::Dies, None, vec![Effect::GainLifeAndDrawSourcePower]);
        defs[cid("Serra Angel")].triggers.push(t);
    });
    let angel = g.add_card(0, cid("Serra Angel"), Zone::Battlefield); // 4/4
    let hand = g.players[0].hand.len();
    let kill = g.add_card(0, cid("Hero's Downfall"), Zone::Hand);
    play(&mut g, kill, false, &[obj(angel)], &[]);
    assert_eq!(g.players[0].life, 44);
    assert_eq!(g.players[0].hand.len(), hand + 4);
}

#[test]
fn gain_life_per_spell_cast_this_turn_counts_the_spell_itself() {
    let mut g = game_with(|defs| {
        let t = trigger(When::YouCast, None, vec![Effect::GainLifePerSpellCastThisTurn]);
        defs[cid("Exquisite Blood")].triggers.push(t);
    });
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    for _ in 0..2 {
        let ring = g.add_card(0, cid("Sol Ring"), Zone::Hand);
        play(&mut g, ring, false, &[], &[]);
    }
    assert_eq!(g.players[0].life, 43, "1, then 2");
}

#[test]
fn gain_and_draw_half_x_rounds_down() {
    let mut g = game_with(|defs| {
        let t = trigger(When::CastThisSpell, None, vec![Effect::GainAndDrawHalfX]);
        defs[cid("Goldvein Hydra")].triggers.push(t);
    });
    let h = g.add_card(0, cid("Goldvein Hydra"), Zone::Hand);
    let hand = g.players[0].hand.len();
    act(&mut g, Action::CastX(h, 5));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 42, "half of 5, rounded down");
    assert_eq!(g.players[0].hand.len(), hand - 1 + 2);
}

#[test]
fn not_your_turn_with_player_plays_land_is_an_opponents_land_play() {
    let mut g = game_with(ring_with(trigger(
        When::PlayerPlaysLand,
        Some(Condition::NotYourTurn),
        gain_one(),
    )));
    g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    let mine = g.add_card(0, cid("Forest"), Zone::Hand);
    act(&mut g, Action::PlayLand(mine));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 40, "your own land play");
    g.start_turn(4, 1);
    while !(g.step == Step::Main1 && g.pending == bolt_core::Pending::Priority(1)) {
        act(&mut g, Action::PassPriority);
    }
    let theirs = g.add_card(1, cid("Forest"), Zone::Hand);
    act(&mut g, Action::PlayLand(theirs));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 41, "the opponent's land play");
}
