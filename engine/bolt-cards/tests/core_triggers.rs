//! Core theme "triggers" (lane d2): each new piece tested directly, on a card definition edited to
//! carry it, in a 1v1 Commander game on the CMD pool.
#![allow(dead_code)]

mod common;

use std::sync::Arc;

use bolt_cards::set;
use bolt_core::effect::{Ability, Condition, Effect, Filter, Trigger, When, Who};
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

// ---- batch 2: life lost and cards drawn by an amount ----

/// Hero's Downfall, its spell replaced by `targets` and `effects`.
fn downfall_as(targets: Vec<Filter>, effects: Vec<Effect>) -> impl Fn(&mut Vec<CardDef>) {
    move |defs: &mut Vec<CardDef>| {
        defs[cid("Hero's Downfall")].spell = Some(Ability::new(targets.clone(), effects.clone()));
    }
}

#[test]
fn lose_life_slot_mana_value_reads_the_target_before_it_moves() {
    let creature = Filter {
        players: false,
        planeswalkers: false,
        ..Filter::ANY
    };
    let mut g = game_with(downfall_as(
        vec![creature],
        vec![Effect::LoseLifeSlotManaValue(0), Effect::Destroy(0)],
    ));
    let angel = g.add_card(1, cid("Serra Angel"), Zone::Battlefield); // mana value 5
    let d = g.add_card(0, cid("Hero's Downfall"), Zone::Hand);
    play(&mut g, d, false, &[obj(angel)], &[]);
    assert_eq!(g.obj(angel).zone, Zone::Graveyard);
    assert_eq!(g.players[0].life, 35);
}

#[test]
fn reveal_top_to_hand_lose_mana_value_takes_the_top_card() {
    let mut g = game_with(ring_with(trigger(
        When::Upkeep { your_turn_only: true },
        None,
        vec![Effect::RevealTopToHandLoseManaValue],
    )));
    g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    let top = g.add_card(0, cid("Serra Angel"), Zone::Library);
    g.start_turn(5, 0);
    to_main(&mut g);
    assert_eq!(g.obj(top).zone, Zone::Hand);
    assert_eq!(g.players[0].life, 35);
}

#[test]
fn lose_life_per_card_in_hand_counts_on_resolution() {
    let mut g = game_with(downfall_as(vec![], vec![Effect::LoseLifePerCardInHand]));
    let d = g.add_card(0, cid("Hero's Downfall"), Zone::Hand);
    let n = g.players[0].hand.len() as i32;
    play(&mut g, d, false, &[], &[]);
    assert_eq!(g.players[0].life, 40 - (n - 1), "the spell has left the hand");
}

#[test]
fn draw_and_lose_per_graveyard_card_for_the_target_player() {
    let mut g = game_with(downfall_as(
        vec![Filter::PLAYER],
        vec![Effect::DrawAndLosePerGraveyardCard(Who::TargetPlayer(0))],
    ));
    for _ in 0..3 {
        g.add_card(1, cid("Plains"), Zone::Graveyard);
    }
    let n = g.players[1].hand.len();
    let d = g.add_card(0, cid("Hero's Downfall"), Zone::Hand);
    play(&mut g, d, false, &[bolt_core::Target::Player(1)], &[]);
    assert_eq!(g.players[1].hand.len(), n + 3);
    assert_eq!(g.players[1].life, 37);
    assert_eq!(g.players[0].life, 40);
}

#[test]
fn discard_hands_draw_that_many_minus_one_for_each_player() {
    let mut g = game_with(downfall_as(
        vec![],
        vec![Effect::DiscardHandsDrawThatManyMinusOne],
    ));
    let d = g.add_card(0, cid("Hero's Downfall"), Zone::Hand);
    let (a, b) = (g.players[0].hand.len() - 1, g.players[1].hand.len());
    let (ga, gb) = (g.players[0].graveyard.len(), g.players[1].graveyard.len());
    play(&mut g, d, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), a - 1);
    assert_eq!(g.players[1].hand.len(), b - 1);
    assert_eq!(g.players[0].graveyard.len(), ga + a + 1, "its hand and the spell");
    assert_eq!(g.players[1].graveyard.len(), gb + b);
}

#[test]
fn draw_and_lose_per_subtype_you_control_counts_the_subtype() {
    let mut g = game_with(ring_with(trigger(
        When::Upkeep { your_turn_only: true },
        None,
        vec![Effect::DrawAndLosePerSubtypeYouControl("Cat")],
    )));
    g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    for _ in 0..2 {
        g.add_card(0, cid("Savannah Lions"), Zone::Battlefield); // Cat Soldier
    }
    g.add_card(1, cid("Savannah Lions"), Zone::Battlefield); // not yours
    let n = g.players[0].hand.len();
    g.start_turn(5, 0);
    to_main(&mut g);
    assert_eq!(g.players[0].hand.len(), n + 2 + 1, "two Cats, then the draw step");
    assert_eq!(g.players[0].life, 38);
}

// ---- batch 2: decisions that pick what's sacrificed or returned ----

fn named(g: &Game, p: usize, name: &str) -> ObjId {
    g.permanents(p)
        .find(|&x| g.def_of(x).name == name)
        .expect("on the battlefield")
}

#[test]
fn sacrifice_another_gain_and_draw_power_picks_another_creature() {
    let enters = |defs: &mut Vec<CardDef>| {
        let t = trigger(When::Enters, None, vec![Effect::SacrificeAnotherGainAndDrawPower]);
        defs[cid("Savannah Lions")].triggers.push(t);
    };
    let mut g = game_with(enters);
    let angel = g.add_card(0, cid("Serra Angel"), Zone::Battlefield); // 4/4
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Hand);
    let n = g.players[0].hand.len();
    act(&mut g, Action::Cast(lions));
    while !matches!(g.pending, bolt_core::Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Pick(lions)),
        "another creature"
    );
    resolve_all(&mut g, &[Action::Pick(angel)]);
    assert_eq!(g.obj(angel).zone, Zone::Graveyard);
    assert_eq!(g.players[0].life, 44);
    assert_eq!(g.players[0].hand.len(), n - 1 + 4);

    // Nothing else to sacrifice: nothing happens.
    let mut g = game_with(enters);
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Hand);
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, 40);
}

#[test]
fn return_non_subtype_land_or_sacrifice_self_returns_a_land_or_goes() {
    let enters = |defs: &mut Vec<CardDef>| {
        let t = trigger(
            When::Enters,
            None,
            vec![Effect::ReturnNonSubtypeLandOrSacrificeSelf("Forest")],
        );
        defs[cid("Savage Lands")].triggers.push(t);
    };
    for keep in [false, true] {
        let mut g = game_with(enters);
        let plains = named(&g, 0, "Plains");
        let forest = named(&g, 0, "Forest");
        let s = g.add_card(0, cid("Savage Lands"), Zone::Hand);
        act(&mut g, Action::PlayLand(s));
        while !matches!(g.pending, bolt_core::Pending::Decide(_)) {
            act(&mut g, Action::PassPriority);
        }
        let legal = g.legal_actions();
        assert!(legal.contains(&Action::Pick(plains)));
        assert!(!legal.contains(&Action::Pick(forest)), "it has the subtype");
        assert!(!legal.contains(&Action::Pick(s)), "not itself");
        if keep {
            resolve_all(&mut g, &[Action::Pick(plains)]);
            assert_eq!(g.obj(plains).zone, Zone::Hand);
            assert_eq!(g.obj(s).zone, Zone::Battlefield);
        } else {
            resolve_all(&mut g, &[Action::No]);
            assert_eq!(g.obj(s).zone, Zone::Graveyard, "sacrificed");
        }
    }
}

// ---- batch 3: dies, discard, life and cast emits ----

fn life(g: &Game, p: usize) -> i32 {
    g.players[p].life
}

/// Exquisite Blood, carrying `triggers` too (an enchantment, so it isn't a creature or artifact).
fn blood_with(triggers: Vec<Trigger>) -> impl Fn(&mut Vec<CardDef>) {
    move |defs: &mut Vec<CardDef>| {
        defs[cid("Exquisite Blood")]
            .triggers
            .extend(triggers.iter().cloned());
    }
}

fn gain(n: i32) -> Vec<Effect> {
    vec![Effect::GainLife { who: Who::You, n }]
}

/// Hero's Downfall on `target`, with three more Swamps to pay for it.
fn kill(g: &mut Game, target: ObjId) {
    for _ in 0..3 {
        g.add_card(0, cid("Swamp"), Zone::Battlefield);
    }
    let d = g.add_card(0, cid("Hero's Downfall"), Zone::Hand);
    play(g, d, false, &[obj(target)], &[]);
    assert_eq!(g.obj(target).zone, Zone::Graveyard);
}

#[test]
fn you_gain_life_carries_the_amount_gained() {
    let mut drain = trigger(
        When::YouGainLife,
        None,
        vec![Effect::LoseLifeAmount(Who::TargetPlayer(0))],
    );
    drain.ability.targets = vec![Filter::PLAYER];
    let mut g = game_with(|defs| {
        ring_with(drain.clone())(defs);
        defs[cid("Hero's Downfall")].spell = Some(Ability::new(vec![], gain(3)));
    });
    g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    let d = g.add_card(0, cid("Hero's Downfall"), Zone::Hand);
    play(
        &mut g,
        d,
        false,
        &[],
        &[Action::Target(bolt_core::Target::Player(1))],
    );
    assert_eq!(life(&g, 0), 43);
    assert_eq!(life(&g, 1), 37, "that much life");
}

#[test]
fn opponent_creature_dies_and_another_creature_you_control_dies() {
    let mut g = game_with(|defs| {
        blood_with(vec![trigger(When::OpponentCreatureDies, None, gain(1))])(defs);
        let t = trigger(When::AnotherCreatureYouControlDies, None, gain(10));
        defs[cid("Savannah Lions")].triggers.push(t);
    });
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    let angel = g.add_card(0, cid("Serra Angel"), Zone::Battlefield);
    let theirs = g.add_card(1, cid("Serra Angel"), Zone::Battlefield);
    kill(&mut g, angel);
    assert_eq!(life(&g, 0), 50, "another creature you control");
    kill(&mut g, theirs);
    assert_eq!(life(&g, 0), 51, "a creature an opponent controls");
    kill(&mut g, lions);
    assert_eq!(life(&g, 0), 51, "not another: itself, and it's yours");
}

#[test]
fn another_creature_you_control_dies_counts_tokens() {
    let mut g = game_with(|defs| {
        let t = trigger(When::AnotherCreatureYouControlDies, None, gain(1));
        defs[cid("Serra Angel")].triggers.push(t);
        defs[cid("Savannah Lions")].token = true; // stands in for a creature token
    });
    g.add_card(0, cid("Serra Angel"), Zone::Battlefield);
    let token = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    assert!(g.is_token(token));
    let d = g.add_card(0, cid("Hero's Downfall"), Zone::Hand);
    play(&mut g, d, false, &[obj(token)], &[]);
    assert_eq!(life(&g, 0), 41);
}

#[test]
fn this_or_another_subtype_you_control_dies() {
    let mut g = game_with(|defs| {
        let t = trigger(When::ThisOrAnotherYouControlDiesWithSubtype("Cat"), None, gain(1));
        defs[cid("Savannah Lions")].triggers.push(t); // a Cat Soldier
    });
    let _a = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    let b = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    let angel = g.add_card(0, cid("Serra Angel"), Zone::Battlefield);
    let theirs = g.add_card(1, cid("Savannah Lions"), Zone::Battlefield);
    kill(&mut g, angel);
    assert_eq!(life(&g, 0), 40, "not a Cat");
    kill(&mut g, b);
    assert_eq!(life(&g, 0), 42, "itself, and another Cat for the other");
    kill(&mut g, theirs);
    assert_eq!(life(&g, 0), 42, "not yours");
}

#[test]
fn artifact_to_a_graveyard_and_to_an_opponents_graveyard() {
    let mut g = game_with(blood_with(vec![
        trigger(When::ArtifactToGraveyardFromBattlefield, None, gain(1)),
        trigger(When::ArtifactToOpponentsGraveyardFromBattlefield, None, gain(10)),
    ]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let mine = g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    let theirs = g.add_card(1, cid("Sol Ring"), Zone::Battlefield);
    let decay = g.add_card(0, cid("Abrupt Decay"), Zone::Hand);
    play(&mut g, decay, false, &[obj(theirs)], &[]);
    assert_eq!(life(&g, 0), 51);
    let decay = g.add_card(0, cid("Abrupt Decay"), Zone::Hand);
    play(&mut g, decay, false, &[obj(mine)], &[]);
    assert_eq!(life(&g, 0), 52, "any graveyard, but not an opponent's");
}

/// Any card in a graveyard.
fn graveyard_card() -> Filter {
    Filter {
        players: false,
        creatures: false,
        planeswalkers: false,
        graveyard: Some(false),
        ..Filter::ANY
    }
}

#[test]
fn you_discard_fills_the_discarded_card_and_skips_opponents_discards() {
    let mut exile = trigger(When::YouDiscard, None, vec![Effect::Exile(0)]);
    exile.ability.targets = vec![Filter {
        triggering: true,
        ..graveyard_card()
    }];
    let mut g = game_with(|defs| {
        blood_with(vec![exile.clone()])(defs);
        let both = vec![
            Effect::Discard { who: Who::You, n: 1 },
            Effect::Discard {
                who: Who::EachOpponent,
                n: 1,
            },
        ];
        defs[cid("Hero's Downfall")].spell = Some(Ability::new(vec![], both));
    });
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let mine = g.add_card(0, cid("Sol Ring"), Zone::Hand);
    let theirs = g.add_card(1, cid("Sol Ring"), Zone::Hand);
    let d = g.add_card(0, cid("Hero's Downfall"), Zone::Hand);
    play(
        &mut g,
        d,
        false,
        &[],
        &[Action::Discard(mine), Action::Discard(theirs)],
    );
    assert_eq!(
        g.obj(mine).zone,
        Zone::Exile,
        "your discard, exiled by the trigger"
    );
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "theirs doesn't trigger it");
}

#[test]
fn you_cast_enchantment_and_artifact_spells() {
    let mut g = game_with(blood_with(vec![
        trigger(When::YouCastEnchantment, None, gain(1)),
        trigger(When::YouCastArtifact, None, gain(10)),
    ]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Hand);
    play(&mut g, ring, false, &[], &[]);
    assert_eq!(life(&g, 0), 50);
    let blood = g.add_card(0, cid("Exquisite Blood"), Zone::Hand);
    play(&mut g, blood, false, &[], &[]);
    assert_eq!(life(&g, 0), 51, "only the one already out triggers");
    g.add_card(0, cid("Plains"), Zone::Battlefield);
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Hand);
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(life(&g, 0), 51, "a creature spell is neither");
}

#[test]
fn plus_counters_put_on_this_triggers_every_time() {
    let mut g = game_with(|defs| {
        let t = trigger(When::PlusCountersPutOnThis, None, gain(1));
        defs[cid("Serra Angel")].triggers.push(t);
        let c = vec![Effect::Counters { slot: 0, n: 1 }];
        let creature = Filter {
            players: false,
            planeswalkers: false,
            ..Filter::ANY
        };
        defs[cid("Hero's Downfall")].spell = Some(Ability::new(vec![creature], c));
    });
    let angel = g.add_card(0, cid("Serra Angel"), Zone::Battlefield);
    for _ in 0..2 {
        let d = g.add_card(0, cid("Hero's Downfall"), Zone::Hand);
        play(&mut g, d, false, &[obj(angel)], &[]);
    }
    assert_eq!(life(&g, 0), 42, "twice in one turn");
}

#[test]
fn you_lose_life_carries_the_amount_lost() {
    let mut g = game_with(|defs| {
        blood_with(vec![trigger(When::YouLoseLife, None, vec![Effect::DrawAmount])])(defs);
        let lose = vec![Effect::LoseLife { who: Who::You, n: 3 }];
        defs[cid("Hero's Downfall")].spell = Some(Ability::new(vec![], lose));
    });
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let d = g.add_card(0, cid("Hero's Downfall"), Zone::Hand);
    let n = g.players[0].hand.len();
    play(&mut g, d, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), n - 1 + 3);
}

#[test]
fn control_subtype_at_least_counts_your_permanents() {
    for (n, gained) in [(5, 1), (6, 0)] {
        let mut g = game_with(ring_with(trigger(
            When::Upkeep { your_turn_only: true },
            Some(Condition::ControlSubtypeAtLeast("Plains", n)),
            gain_one(),
        )));
        g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
        g.start_turn(5, 0);
        to_main(&mut g);
        assert_eq!(life(&g, 0), 40 + gained, "five Plains, at least {n}");
    }
}

// ---- batch 4: suppressed and doubled triggers ----

/// Savannah Lions, Sol Ring and Exquisite Blood with "When this enters, you gain 1 life" and "When
/// this dies, you gain 10 life".
#[allow(clippy::ptr_arg)] // passed where `game_with` takes `Fn(&mut Vec<CardDef>)`
fn enters_and_dies(defs: &mut Vec<CardDef>) {
    for name in ["Savannah Lions", "Sol Ring", "Exquisite Blood"] {
        defs[cid(name)]
            .triggers
            .push(trigger(When::Enters, None, gain(1)));
        defs[cid(name)].triggers.push(trigger(When::Dies, None, gain(10)));
    }
}

#[test]
fn creatures_entering_or_dying_dont_trigger_with_hushbringer() {
    let mut g = game_with(enters_and_dies);
    g.add_card(1, cid("Hushbringer"), Zone::Battlefield);
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Hand);
    play(&mut g, lions, false, &[], &[]);
    kill(&mut g, lions);
    assert_eq!(life(&g, 0), 40, "a creature entering or dying");
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Hand);
    play(&mut g, ring, false, &[], &[]);
    assert_eq!(life(&g, 0), 41, "an artifact entering still triggers");
}

#[test]
fn artifacts_and_creatures_entering_dont_trigger_with_doorkeeper_thrull() {
    let mut g = game_with(enters_and_dies);
    g.add_card(1, cid("Doorkeeper Thrull"), Zone::Battlefield);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Hand);
    play(&mut g, ring, false, &[], &[]);
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Hand);
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(life(&g, 0), 40);
    kill(&mut g, lions);
    assert_eq!(life(&g, 0), 50, "dying still triggers");
    let blood = g.add_card(0, cid("Exquisite Blood"), Zone::Hand);
    play(&mut g, blood, false, &[], &[]);
    assert_eq!(life(&g, 0), 51, "an enchantment entering still triggers");
}

#[test]
fn elesh_norn_doubles_yours_and_stops_opponents_entering_triggers() {
    let mut g = game_with(enters_and_dies);
    g.add_card(0, cid("Elesh Norn, Mother of Machines"), Zone::Battlefield);
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Hand);
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(life(&g, 0), 42, "twice");
    let theirs = g.add_card(1, cid("Savannah Lions"), Zone::Hand);
    g.start_turn(4, 1);
    while !(g.step == Step::Main1 && g.pending == bolt_core::Pending::Priority(1)) {
        act(&mut g, Action::PassPriority);
    }
    play(&mut g, theirs, false, &[], &[]);
    assert_eq!(life(&g, 1), 40, "the opponent's doesn't trigger");
}

#[test]
fn trigger_doublers_by_kind_of_source() {
    use bolt_core::TriggerDoubler;
    let mut g = game_with(|defs| {
        enters_and_dies(defs);
        defs[cid("Serra Angel")]
            .triggers
            .push(trigger(When::Enters, None, gain(100)));
        defs[cid("Sol Ring")].doubles_triggers_of = Some(TriggerDoubler::CreaturePowerAtMost(2));
    });
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Hand);
    play(&mut g, ring, false, &[], &[]);
    assert_eq!(life(&g, 0), 41, "not a creature: once");
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Hand);
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(life(&g, 0), 43, "power 2: twice");
    let angel = g.add_card(0, cid("Serra Angel"), Zone::Hand);
    play(&mut g, angel, false, &[], &[]);
    assert_eq!(life(&g, 0), 143, "power 4: once");
    kill(&mut g, lions);
    assert_eq!(
        life(&g, 0),
        163,
        "its dies trigger looks back at its power: twice"
    );
}

// ---- batch 5: sacrifice, enter and life-lost emits ----

#[test]
fn sacrifice_emits_for_you_any_player_tokens_and_subtypes() {
    let mut g = game_with(|defs| {
        blood_with(vec![
            trigger(When::YouSacrifice, None, gain(1)),
            trigger(When::PlayerSacrificesAnother, None, gain(10)),
            trigger(When::YouSacrificeToken, None, gain(100)),
            trigger(When::YouSacrificeSubtype("Cat"), None, gain(1000)),
        ])(defs);
        defs[cid("Savannah Lions")].token = true; // a Cat Soldier token
        let both = vec![
            Effect::SacrificeCreatures { who: Who::You, n: 1 },
            Effect::SacrificeCreatures {
                who: Who::EachOpponent,
                n: 1,
            },
        ];
        defs[cid("Hero's Downfall")].spell = Some(Ability::new(vec![], both));
    });
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    let angel = g.add_card(1, cid("Serra Angel"), Zone::Battlefield);
    let d = g.add_card(0, cid("Hero's Downfall"), Zone::Hand);
    play(&mut g, d, false, &[], &[]);
    assert_ne!(g.obj(lions).zone, Zone::Battlefield, "a token: gone");
    assert_eq!(g.obj(angel).zone, Zone::Graveyard);
    assert_eq!(
        life(&g, 0),
        40 + 1111 + 10,
        "yours: all four; theirs: a player sacrificed"
    );
}

#[test]
fn a_sacrifice_cost_emits_too() {
    let mut g = game_with(blood_with(vec![trigger(When::YouSacrifice, None, gain(1))]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    // Diabolic Intent: "As an additional cost to cast this spell, sacrifice a creature."
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    let intent = g.add_card(0, cid("Diabolic Intent"), Zone::Hand);
    act(&mut g, Action::CastWith(intent, lions));
    while !g.items.is_empty()
        || matches!(
            g.pending,
            bolt_core::Pending::Decide(_) | bolt_core::Pending::ChooseTarget(_)
        )
    {
        let a = match g.pending {
            bolt_core::Pending::Decide(_) | bolt_core::Pending::ChooseTarget(_) => {
                let legal = g.legal_actions();
                if legal.contains(&Action::Pick(lions)) {
                    Action::Pick(lions)
                } else {
                    legal[0]
                }
            }
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(life(&g, 0), 41);
}

#[test]
fn lose_life_lost_this_turn_doubles_the_loss() {
    let mut g = game_with(downfall_as(
        vec![],
        vec![
            Effect::LoseLife {
                who: Who::EachOpponent,
                n: 3,
            },
            Effect::LoseLifeLostThisTurn(Who::EachOpponent),
        ],
    ));
    let d = g.add_card(0, cid("Hero's Downfall"), Zone::Hand);
    play(&mut g, d, false, &[], &[]);
    assert_eq!(life(&g, 1), 34);
}

#[test]
fn creature_or_artifact_dies_and_another_permanent_enters() {
    let mut g = game_with(blood_with(vec![
        trigger(When::ThisOrAnotherCreatureOrArtifactYouControlDies, None, gain(1)),
        trigger(When::AnotherPermanentYouControlEnters, None, gain(10)),
    ]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Hand);
    play(&mut g, ring, false, &[], &[]);
    assert_eq!(life(&g, 0), 50, "a permanent entered");
    let decay = g.add_card(0, cid("Abrupt Decay"), Zone::Hand);
    play(&mut g, decay, false, &[obj(ring)], &[]);
    assert_eq!(life(&g, 0), 51, "an artifact died");
}

#[test]
fn another_creature_of_a_color_enters() {
    let mut g = game_with(blood_with(vec![trigger(
        When::AnotherCreatureYouControlEntersWithColor(0), // white
        None,
        gain(1),
    )]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Hand);
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(life(&g, 0), 41, "white");
    let oak = g.add_card(0, cid("Scurry Oak"), Zone::Hand);
    play(&mut g, oak, false, &[], &[]);
    assert_eq!(life(&g, 0), 41, "green");
}

// ---- Batch 6: cast, copy, damage, attack, untap, graveyard and batched emits ----

fn cast_now(g: &mut Game, p: usize, name: &str, targets: &[bolt_core::Target]) {
    let c = g.add_card(p, cid(name), Zone::Hand);
    play(g, c, false, targets, &[]);
}

#[test]
fn cast_path_mana_value_color_and_second_spell_emits() {
    let mut g = game_with(blood_with(vec![
        trigger(When::YouCastNoncreatureMvAtLeast(3), None, gain(1)),
        trigger(When::YouCastColor(2), None, gain(10)), // black
        trigger(When::PlayerCastsSecondSpell, None, gain(100)),
        trigger(When::YouCastLegendary, None, gain(1000)),
    ]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let lions = g.add_card(1, cid("Savannah Lions"), Zone::Battlefield);
    cast_now(&mut g, 0, "Hero's Downfall", &[obj(lions)]); // mana value 3, black
    assert_eq!(life(&g, 0), 51);
    cast_now(&mut g, 0, "Sol Ring", &[]); // the second spell; mana value 1
    assert_eq!(life(&g, 0), 151);
    cast_now(&mut g, 0, "Sol Ring", &[]);
    assert_eq!(life(&g, 0), 151, "the third isn't the second");
}

#[test]
fn opponents_first_noncreature_spell_each_turn() {
    let mut g = game_with(blood_with(vec![trigger(
        When::OpponentCastsFirstNoncreature,
        None,
        gain(1),
    )]));
    g.add_card(1, cid("Exquisite Blood"), Zone::Battlefield);
    cast_now(&mut g, 0, "Savannah Lions", &[]);
    cast_now(&mut g, 0, "Sol Ring", &[]);
    cast_now(&mut g, 0, "Sol Ring", &[]);
    assert_eq!(life(&g, 1), 41);
}

#[test]
fn first_multicolored_spell_for_you_and_for_the_opponent() {
    let mut g = game_with(blood_with(vec![
        trigger(When::YouCastFirstMulticolored, None, gain(1)),
        trigger(When::OpponentCastsFirstMulticolored, None, gain(10)),
    ]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    g.add_card(1, cid("Exquisite Blood"), Zone::Battlefield);
    for _ in 0..2 {
        let ring = g.add_card(1, cid("Sol Ring"), Zone::Battlefield);
        cast_now(&mut g, 0, "Abrupt Decay", &[obj(ring)]);
    }
    assert_eq!(life(&g, 0), 41);
    assert_eq!(life(&g, 1), 50);
}

#[test]
fn magecraft_triggers_on_a_cast_instant() {
    let mut g = game_with(blood_with(vec![trigger(
        When::YouCastOrCopyInstantOrSorcery,
        None,
        gain(1),
    )]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let lions = g.add_card(1, cid("Savannah Lions"), Zone::Battlefield);
    cast_now(&mut g, 0, "Hero's Downfall", &[obj(lions)]);
    cast_now(&mut g, 0, "Sol Ring", &[]);
    assert_eq!(life(&g, 0), 41);
}

/// Player 0 attacks with `attackers` on turn 5 and the game passes to the second main phase.
fn attack_through(g: &mut Game, attackers: &[ObjId]) {
    g.start_turn(5, 0);
    while g.pending != bolt_core::Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    for &a in attackers {
        act(g, Action::Attack(a));
    }
    act(g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        let a = match g.pending {
            bolt_core::Pending::DeclareBlockers(_) => Action::FinishBlocks,
            bolt_core::Pending::Decide(_) => Action::Yes,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn damage_emits_with_their_source() {
    let it = Filter {
        triggering: true,
        ..Filter::CREATURE
    };
    let mut counter = trigger(
        When::YourCreatureDealsCombatDamageToPlayerIt,
        None,
        vec![Effect::Counters { slot: 0, n: 1 }],
    );
    counter.ability.targets = vec![it];
    let mut g = game_with(|defs| {
        blood_with(vec![
            counter.clone(),
            trigger(When::SubtypeAttacks("Cat"), None, gain(1)),
            trigger(When::CreatureDealsDamageToYou, None, gain(10)),
            trigger(When::YourSubtypeDealDamageToPlayer("Cat"), None, gain(100)),
        ])(defs);
        defs[cid("Savannah Lions")]
            .triggers
            .push(trigger(When::DealsDamageToOpponent, None, gain(1000)));
    });
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    g.add_card(1, cid("Exquisite Blood"), Zone::Battlefield);
    let a = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    let b = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    attack_through(&mut g, &[a, b]);
    // Exquisite Blood itself: player 1 lost 4, so player 0 gained 4.
    assert_eq!(
        life(&g, 0),
        40 + 4 + 1 + 1 + 100 + 2000,
        "two Cats attacked; one batch; two sources"
    );
    assert_eq!(
        life(&g, 1),
        40 - 4 + 1 + 1 + 10 + 10,
        "its Cat triggers too; damaged twice"
    );
    assert_eq!(g.obj(a).counters, 1);
    assert_eq!(g.obj(b).counters, 1);
}

#[test]
fn becomes_untapped_in_the_untap_step_and_by_an_effect() {
    let mut g = game_with(|defs| {
        defs[cid("Savannah Lions")]
            .triggers
            .push(trigger(When::BecomesUntapped, None, gain(1)));
    });
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    g.objects[lions.0 as usize].tapped = true;
    g.start_turn(5, 0);
    to_main(&mut g);
    assert_eq!(life(&g, 0), 41, "the untap step");
    g.add_card(0, cid("Mirran Spy"), Zone::Battlefield);
    g.objects[lions.0 as usize].tapped = true;
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Hand);
    let untap = [Action::Target(obj(lions)), Action::Yes];
    play(&mut g, ring, false, &[], &untap);
    assert_eq!(life(&g, 0), 42, "Mirran Spy untapped it");
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Hand);
    play(&mut g, ring, false, &[], &untap);
    assert_eq!(
        life(&g, 0),
        42,
        "untapping an untapped creature isn't becoming untapped"
    );
}

#[test]
fn graveyard_emits_for_cards_to_an_opponents_graveyard_and_creature_cards() {
    let mut g = game_with(blood_with(vec![
        trigger(When::CardToOpponentsGraveyard, None, gain(1)),
        trigger(When::CreatureCardToGraveyardNotFromBattlefield, None, gain(10)),
        trigger(When::CreatureCardLeavesYourGraveyard, None, gain(100)),
    ]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let lions = g.add_card(1, cid("Savannah Lions"), Zone::Battlefield);
    cast_now(&mut g, 0, "Hero's Downfall", &[obj(lions)]);
    assert_eq!(life(&g, 0), 41, "their creature died: a card to their graveyard");
    let both = vec![Effect::Discard {
        who: Who::EachOpponent,
        n: 1,
    }];
    let mut g2 = game_with(|defs| {
        blood_with(vec![
            trigger(When::CardToOpponentsGraveyard, None, gain(1)),
            trigger(When::CreatureCardToGraveyardNotFromBattlefield, None, gain(10)),
        ])(defs);
        defs[cid("Hero's Downfall")].spell = Some(Ability::new(vec![], both.clone()));
    });
    g2.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let theirs = g2.add_card(1, cid("Savannah Lions"), Zone::Hand);
    let d = g2.add_card(0, cid("Hero's Downfall"), Zone::Hand);
    play(&mut g2, d, false, &[], &[Action::Discard(theirs)]);
    assert_eq!(life(&g2, 0), 51, "a creature card discarded into their graveyard");
}

#[test]
fn delayed_creature_dies_this_turn() {
    let mut g = game_with(|defs| {
        let d = &mut defs[cid("Sol Ring")];
        let n = d.triggers.len();
        let register = Effect::DelayedThisTurn {
            when: When::CreatureDiesThisTurn,
            trigger: n + 1,
        };
        d.triggers.push(trigger(When::Enters, None, vec![register]));
        d.triggers
            .push(trigger(When::CreatureDiesThisTurn, None, gain(1)));
    });
    let lions = g.add_card(1, cid("Savannah Lions"), Zone::Battlefield);
    cast_now(&mut g, 0, "Sol Ring", &[]);
    assert_eq!(life(&g, 0), 40);
    cast_now(&mut g, 0, "Hero's Downfall", &[obj(lions)]);
    assert_eq!(life(&g, 0), 41);
}

#[test]
fn creature_of_the_chosen_type_dies() {
    let mut g = game_with(blood_with(vec![trigger(
        When::CreatureOfChosenTypeDies,
        None,
        gain(1),
    )]));
    let e = g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    g.objects[e.0 as usize].chosen_type = Some("Cat".to_string());
    let lions = g.add_card(1, cid("Savannah Lions"), Zone::Battlefield);
    let angel = g.add_card(1, cid("Serra Angel"), Zone::Battlefield);
    cast_now(&mut g, 0, "Hero's Downfall", &[obj(angel)]);
    assert_eq!(life(&g, 0), 40);
    cast_now(&mut g, 0, "Hero's Downfall", &[obj(lions)]);
    assert_eq!(life(&g, 0), 41);
}

#[test]
fn sacrificing_a_creature_of_a_color_and_batched_subtype_sacrifices() {
    let mut g = game_with(|defs| {
        blood_with(vec![
            trigger(When::YouSacrificeCreatureOfColor(0), None, gain(1)), // white
            trigger(When::YouSacrificeCreatureOfColor(4), None, gain(10)), // green
            trigger(
                When::YouSacrificeOneOrMoreSubtype("Cat"),
                None,
                vec![Effect::GainLifeAmount],
            ),
        ])(defs);
        let edict = vec![Effect::SacrificeCreatures { who: Who::You, n: 2 }];
        defs[cid("Hero's Downfall")].spell = Some(Ability::new(vec![], edict));
    });
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    cast_now(&mut g, 0, "Hero's Downfall", &[]);
    assert_eq!(
        life(&g, 0),
        40 + 2 + 2,
        "two white creatures, and one trigger for two Cats"
    );
}

#[test]
fn batched_discards_make_one_trigger_with_the_count() {
    let mut g = game_with(|defs| {
        blood_with(vec![trigger(
            When::YouDiscardOneOrMore,
            None,
            vec![Effect::GainLifeAmount],
        )])(defs);
        let two = vec![Effect::Discard { who: Who::You, n: 2 }];
        defs[cid("Hero's Downfall")].spell = Some(Ability::new(vec![], two));
    });
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let a = g.add_card(0, cid("Sol Ring"), Zone::Hand);
    let b = g.add_card(0, cid("Sol Ring"), Zone::Hand);
    let d = g.add_card(0, cid("Hero's Downfall"), Zone::Hand);
    play(&mut g, d, false, &[], &[Action::Discard(a), Action::Discard(b)]);
    assert_eq!(life(&g, 0), 42);
}

#[test]
fn creature_with_flying_enters_and_opponents_creature_blocks() {
    let it = Filter {
        triggering: true,
        ..Filter::CREATURE
    };
    let mut flier = trigger(
        When::CreatureWithFlyingYouControlEnters,
        None,
        vec![Effect::Counters { slot: 0, n: 1 }],
    );
    flier.ability.targets = vec![it];
    let mut g = game_with(|defs| {
        blood_with(vec![
            flier.clone(),
            trigger(When::OpponentCreatureBlocks, None, gain(1)),
        ])(defs);
        let d = &mut defs[cid("Savannah Lions")];
        d.keywords = d.keywords.with(bolt_core::Keywords::FLYING);
    });
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let flying = g.add_card(0, cid("Savannah Lions"), Zone::Hand);
    play(&mut g, flying, false, &[], &[]);
    assert_eq!(g.obj(flying).counters, 1, "a flier entered");
    let oak = g.add_card(0, cid("Scurry Oak"), Zone::Hand);
    play(&mut g, oak, false, &[], &[]);
    assert_eq!(g.obj(oak).counters, 0);
    let attacker = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    let blocker = g.add_card(1, cid("Savannah Lions"), Zone::Battlefield);
    g.start_turn(5, 0);
    while g.pending != bolt_core::Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(attacker));
    act(&mut g, Action::FinishAttacks);
    while g.pending != bolt_core::Pending::DeclareBlockers(1) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Block { blocker, attacker });
    act(&mut g, Action::FinishBlocks);
    while g.step != Step::Main2 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(life(&g, 0), 41);
}

#[test]
fn opponent_searches_their_library_and_conditions_on_life_lost_and_quest_counters() {
    let mut g = game_with(|defs| {
        blood_with(vec![
            trigger(When::OpponentSearchesLibrary, None, gain(1)),
            trigger(
                When::YouCastArtifact,
                Some(Condition::OpponentLostLifeAtLeast(2)),
                gain(10),
            ),
            trigger(
                When::YouCastArtifact,
                Some(Condition::SourceQuestCountersAtLeast(3)),
                gain(100),
            ),
        ])(defs);
    });
    let e = g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let tutor = g.add_card(1, cid("Demonic Tutor"), Zone::Hand);
    g.start_turn(5, 1);
    to_main_of(&mut g, 1);
    let plains = g.players[1].library[0];
    play(&mut g, tutor, false, &[], &[Action::Pick(plains)]);
    assert_eq!(life(&g, 0), 41, "the opponent searched");
    g.start_turn(6, 0);
    to_main(&mut g);
    cast_now(&mut g, 0, "Sol Ring", &[]);
    assert_eq!(life(&g, 0), 41, "neither condition");
    g.objects[e.0 as usize].quest = 3;
    g.life_lost_turn[1] = (g.turn, 2);
    cast_now(&mut g, 0, "Sol Ring", &[]);
    assert_eq!(life(&g, 0), 151, "both conditions");
}

fn to_main_of(g: &mut Game, p: usize) {
    while !(g.step == Step::Main1 && g.active == p) {
        act(g, Action::PassPriority);
    }
}

// ---- Batch 8: counter placement (who puts the counters) ----

#[test]
fn you_put_counters_counts_each_placement_for_the_resolving_controller() {
    let mut g = game_with(|defs| {
        blood_with(vec![trigger(
            When::YouPutCounters,
            None,
            vec![Effect::GainLifeAmount],
        )])(defs);
        let counters = vec![
            Effect::Counters { slot: 0, n: 2 },
            Effect::MinusCounter(0),
            Effect::Energy(3),
        ];
        defs[cid("Hero's Downfall")].spell = Some(Ability::new(vec![Filter::CREATURE], counters));
    });
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    g.add_card(1, cid("Exquisite Blood"), Zone::Battlefield);
    let theirs = g.add_card(1, cid("Savannah Lions"), Zone::Battlefield);
    cast_now(&mut g, 0, "Hero's Downfall", &[obj(theirs)]);
    assert_eq!(g.obj(theirs).counters, 1);
    assert_eq!(
        life(&g, 0),
        46,
        "two +1/+1, one -1/-1 and three energy, all put by player 0"
    );
    assert_eq!(life(&g, 1), 40, "the creature's controller didn't put them");
}

#[test]
fn you_put_counters_outside_a_resolution_is_the_permanents_controller() {
    let mut g = game_with(blood_with(vec![trigger(
        When::YouPutCounters,
        None,
        vec![Effect::GainLifeAmount],
    )]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    // A loyalty ability's + cost is paid as it's activated, while nothing resolves.
    let pw = g.add_card(0, cid("Elspeth, Sun's Champion"), Zone::Battlefield);
    let before = g.obj(pw).loyalty;
    act(&mut g, Action::Loyalty(pw, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(pw).loyalty, before + 1);
    assert_eq!(life(&g, 0), 41, "one loyalty counter, as the cost");
}

// ---- Batch 9: multicolored casts, noncombat damage to an opponent, enchanted creature's damage ----

#[test]
fn every_multicolored_spell_and_noncombat_damage_to_an_opponent() {
    let mut g = game_with(|defs| {
        blood_with(vec![
            trigger(When::YouCastMulticolored, None, gain(1)),
            trigger(When::OpponentDealtNoncombatDamage, None, gain(10)),
        ])(defs);
        let burn = vec![Effect::Damage {
            amount: 1,
            kicked: None,
            to: bolt_core::effect::Recipient::EachOpponent,
        }];
        defs[cid("Hero's Downfall")].spell = Some(Ability::new(vec![], burn));
    });
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    for _ in 0..2 {
        let ring = g.add_card(1, cid("Sol Ring"), Zone::Battlefield);
        cast_now(&mut g, 0, "Abrupt Decay", &[obj(ring)]);
    }
    assert_eq!(life(&g, 0), 42, "each multicolored spell");
    cast_now(&mut g, 0, "Hero's Downfall", &[]);
    // Exquisite Blood itself gains the 1 the opponent lost.
    assert_eq!(life(&g, 0), 42 + 10 + 1);
}

// ---- Batch 10: enters (artifact, nonartifact, tapped), nontoken sacrifices, draws, colorless MV,
// life and board conditions ----

/// Passes (and answers with the first legal choice) until player `p`'s turn `turn` is over.
fn finish_turn(g: &mut Game, turn: u32) {
    while g.turn == turn && g.result.is_none() {
        let legal = g.legal_actions();
        let a = if legal.contains(&Action::PassPriority) {
            Action::PassPriority
        } else if legal.contains(&Action::FinishAttacks) {
            Action::FinishAttacks
        } else if legal.contains(&Action::FinishBlocks) {
            Action::FinishBlocks
        } else {
            legal[0]
        };
        act(g, a);
    }
}

#[test]
fn artifact_nonartifact_and_tapped_enters() {
    let mut g = game_with(blood_with(vec![
        trigger(When::ArtifactYouControlEnters, None, gain(1)),
        trigger(When::NonartifactPermanentYouControlEnters, None, gain(10)),
        trigger(When::PermanentYouControlEntersTapped, None, gain(100)),
    ]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    cast_now(&mut g, 0, "Sol Ring", &[]);
    assert_eq!(life(&g, 0), 41, "an artifact, untapped");
    cast_now(&mut g, 0, "Sakura-Tribe Elder", &[]);
    assert_eq!(life(&g, 0), 51, "a nonartifact permanent");
    let elder = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Sakura-Tribe Elder")
        .unwrap();
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    act(&mut g, Action::Activate(elder, 0));
    resolve_all(&mut g, &[Action::Pick(forest)]);
    assert!(
        g.obj(forest).tapped,
        "put onto the battlefield tapped by the effect"
    );
    assert_eq!(life(&g, 0), 161, "a nonartifact permanent that entered tapped");
}

#[test]
fn nontoken_sacrifices_yours_and_the_opponents() {
    let mut g = game_with(blood_with(vec![
        trigger(When::YouSacrificeNontoken, None, gain(1)),
        trigger(When::OpponentSacrificesNontoken, None, gain(10)),
    ]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let elder = g.add_card(0, cid("Sakura-Tribe Elder"), Zone::Battlefield);
    g.add_card(1, cid("Savannah Lions"), Zone::Battlefield);
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    act(&mut g, Action::Activate(elder, 0));
    resolve_all(&mut g, &[Action::Pick(forest)]);
    assert_eq!(life(&g, 0), 41);
    cast_now(&mut g, 0, "Diabolic Edict", &[bolt_core::Target::Player(1)]);
    assert_eq!(life(&g, 0), 51);
}

#[test]
fn opponent_draws_except_the_first_in_their_draw_step() {
    let mut g = game_with(blood_with(vec![trigger(
        When::OpponentDrawsExceptFirstInDrawStep,
        None,
        gain(1),
    )]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    finish_turn(&mut g, 3);
    to_main(&mut g);
    assert_eq!(g.active, 1);
    assert_eq!(life(&g, 0), 40, "the draw step's first draw");
    g.add_card(0, cid("Howling Mine"), Zone::Battlefield);
    g.start_turn(5, 1);
    to_main(&mut g);
    assert_eq!(life(&g, 0), 41, "Howling Mine's extra draw");
}

#[test]
fn colorless_spell_with_mana_value_seven_or_more() {
    let mut g = game_with(blood_with(vec![trigger(
        When::YouCastColorlessMvAtLeast(7),
        None,
        gain(1),
    )]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    cast_now(&mut g, 0, "Sol Ring", &[]);
    assert_eq!(life(&g, 0), 40, "mana value 1");
    cast_now(&mut g, 0, "Kozilek, Butcher of Truth", &[]);
    assert_eq!(life(&g, 0), 41);
}

#[test]
fn life_total_life_gained_and_no_creatures_conditions() {
    let mut g = game_with(blood_with(vec![
        trigger(
            When::Upkeep { your_turn_only: true },
            Some(Condition::LifeAtLeast(40)),
            gain(1),
        ),
        trigger(
            When::Upkeep { your_turn_only: true },
            Some(Condition::LifeAtLeast(42)),
            gain(1000),
        ),
        trigger(
            When::EndStep { your_turn_only: true },
            Some(Condition::YouGainedLifeThisTurn),
            gain(10),
        ),
        trigger(
            When::EndStep { your_turn_only: true },
            Some(Condition::NoCreaturesOnBattlefield),
            gain(100),
        ),
    ]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    finish_turn(&mut g, 3);
    assert_eq!(life(&g, 0), 140, "no life gained on turn 3; no creatures");
    g.players[0].life = 40;
    g.add_card(1, cid("Savannah Lions"), Zone::Battlefield);
    g.start_turn(5, 0);
    to_main(&mut g);
    assert_eq!(life(&g, 0), 41, "40 or more, not 42 or more");
    finish_turn(&mut g, 5);
    assert_eq!(life(&g, 0), 51, "gained life this turn; a creature is out");
}

#[test]
fn creature_cards_to_your_graveyard_once_each_turn() {
    let mut g = game_with(blood_with(vec![trigger(
        When::CreatureCardsToYourGraveyardOnceEachTurn,
        None,
        gain(1),
    )]));
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let a = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    let b = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    let theirs = g.add_card(1, cid("Savannah Lions"), Zone::Battlefield);
    kill(&mut g, theirs);
    assert_eq!(life(&g, 0), 40, "the opponent's graveyard");
    kill(&mut g, a);
    assert_eq!(life(&g, 0), 41);
    kill(&mut g, b);
    assert_eq!(life(&g, 0), 41, "once each turn");
    g.start_turn(5, 0);
    to_main(&mut g);
    let c = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    kill(&mut g, c);
    assert_eq!(life(&g, 0), 42, "a new turn");
}

#[test]
fn another_source_deals_exactly_one_damage() {
    let mut g = game_with(|defs| {
        blood_with(vec![trigger(
            When::AnotherSourceYouControlDealsExactlyOne,
            None,
            gain(10),
        )])(defs);
        let burn = |n| {
            vec![Effect::Damage {
                amount: n,
                kicked: None,
                to: bolt_core::effect::Recipient::EachOpponent,
            }]
        };
        defs[cid("Hero's Downfall")].spell = Some(Ability::new(vec![], burn(1)));
        defs[cid("Abrupt Decay")].spell = Some(Ability::new(vec![], burn(2)));
    });
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    cast_now(&mut g, 0, "Hero's Downfall", &[]);
    assert_eq!(life(&g, 0), 40 + 10 + 1, "exactly 1 (and Exquisite Blood's 1)");
    cast_now(&mut g, 0, "Abrupt Decay", &[]);
    assert_eq!(life(&g, 0), 51 + 2, "2 damage");
}

#[test]
fn an_emblems_triggered_ability_triggers_through_fire() {
    let mut g = game_with(|defs| {
        defs[cid("Savannah Lions")]
            .triggers
            .push(trigger(When::Never, None, gain_one()));
    });
    let x = g.add_card(0, cid("Savannah Lions"), Zone::Graveyard);
    let n = g.def_of(x).triggers.len() - 1;
    g.emblems.push((
        0,
        bolt_core::EmblemKind::Triggered {
            when: When::YouDraw,
            trigger: n,
        },
        x,
    ));
    g.start_turn(4, 1);
    to_main(&mut g);
    assert_eq!(life(&g, 0), 40, "the opponent's draw isn't yours");
    g.start_turn(5, 0);
    to_main(&mut g);
    assert_eq!(life(&g, 0), 41, "your draw-step draw triggers the emblem");
}

#[test]
fn completing_a_dungeon_emits_you_complete_dungeon() {
    use bolt_core::{rooms, Dungeon};
    let mut g = game_with(|defs| {
        defs[cid("Savannah Lions")]
            .triggers
            .push(trigger(When::YouCompleteDungeon, None, gain_one()));
    });
    let x = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    let them = g.add_card(1, cid("Savannah Lions"), Zone::Battlefield);
    let bottom = rooms(Dungeon::LostMine)
        .iter()
        .position(|r| r.next.is_empty())
        .expect("a bottommost room") as u8;
    g.venture[0] = Some((Dungeon::LostMine, bottom));
    g.venture_into(0, x, true); // on the bottommost room: completes it, then Undercity
    assert_eq!(g.completed_dungeons[0], 1);
    let mine: Vec<ObjId> = g.triggered.iter().map(|t| t.source).collect();
    assert!(mine.contains(&x), "yours triggers");
    assert!(!mine.contains(&them), "the opponent didn't complete one");
}

#[test]
fn another_flier_and_a_subtype_you_control_enter() {
    let it = Filter {
        triggering: true,
        ..Filter::CREATURE
    };
    let mut cat = trigger(
        When::SubtypeYouControlEnters("Cat"),
        None,
        vec![Effect::Counters { slot: 0, n: 1 }],
    );
    cat.ability.targets = vec![it];
    let mut g = game_with(|defs| {
        blood_with(vec![
            cat.clone(),
            trigger(When::AnotherCreatureWithFlyingYouControlEnters, None, gain(1)),
            trigger(When::SubtypeYouControlEnters("Elf"), None, gain(1000)),
        ])(defs);
        let d = &mut defs[cid("Savannah Lions")];
        d.keywords = d.keywords.with(bolt_core::Keywords::FLYING);
        d.triggers.push(trigger(
            When::AnotherCreatureWithFlyingYouControlEnters,
            None,
            gain(10),
        ));
    });
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let first = g.add_card(0, cid("Savannah Lions"), Zone::Hand);
    play(&mut g, first, false, &[], &[]);
    assert_eq!(life(&g, 0), 41, "another flier for the Blood; not itself");
    assert_eq!(g.obj(first).counters, 1, "a Cat entered: it, in slot 0");
    let second = g.add_card(0, cid("Savannah Lions"), Zone::Hand);
    play(&mut g, second, false, &[], &[]);
    assert_eq!(life(&g, 0), 41 + 1 + 10, "the Blood and the first Lions");
    assert_eq!(g.obj(second).counters, 1);
}

#[test]
fn a_subtype_deals_damage_to_you_with_the_source_in_slot_0() {
    let it = Filter {
        triggering: true,
        ..Filter::CREATURE
    };
    let mut cat = trigger(
        When::SubtypeDealsDamageToYou("Cat"),
        None,
        vec![Effect::Counters { slot: 0, n: 1 }],
    );
    cat.ability.targets = vec![it];
    let mut g = game_with(|defs| {
        blood_with(vec![
            cat.clone(),
            trigger(When::SubtypeDealsDamageToYou("Elf"), None, gain(1000)),
        ])(defs);
    });
    g.add_card(1, cid("Exquisite Blood"), Zone::Battlefield);
    let a = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    attack_through(&mut g, &[a]);
    assert_eq!(g.obj(a).counters, 1, "a Cat dealt damage to its controller");
    assert_eq!(life(&g, 1), 40 - 2, "not an Elf");
}

#[test]
fn you_lost_life_this_turn_condition() {
    let mut g = game_with(|defs| {
        blood_with(vec![trigger(
            When::YouDraw,
            Some(Condition::YouLostLifeThisTurn),
            gain(10),
        )])(defs);
        defs[cid("Hero's Downfall")].spell = Some(Ability::new(
            vec![],
            vec![Effect::LoseLife { who: Who::You, n: 1 }],
        ));
        defs[cid("Abrupt Decay")].spell =
            Some(Ability::new(vec![], vec![Effect::Draw { who: Who::You, n: 1 }]));
    });
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    cast_now(&mut g, 0, "Abrupt Decay", &[]);
    assert_eq!(life(&g, 0), 40, "no life lost yet");
    cast_now(&mut g, 0, "Hero's Downfall", &[]);
    cast_now(&mut g, 0, "Abrupt Decay", &[]);
    assert_eq!(life(&g, 0), 39 + 10);
}

#[test]
fn delayed_when_that_creature_is_put_into_your_graveyard_this_turn() {
    let mut g = game_with(|defs| {
        let d = &mut defs[cid("Abrupt Decay")];
        let n = d.triggers.len();
        d.spell = Some(Ability::new(
            vec![Filter::CREATURE],
            vec![Effect::DelayedWhenSlotToYourGraveyardThisTurn { slot: 0, trigger: n }],
        ));
        let that = Filter {
            triggering: true,
            graveyard: Some(true),
            creatures: false,
            players: false,
            planeswalkers: false,
            ..Filter::ANY
        };
        let mut t = trigger(
            When::SlotCreatureToYourGraveyardThisTurn,
            None,
            vec![Effect::Reanimate(0)],
        );
        t.ability.targets = vec![that];
        d.triggers.push(t);
    });
    let mine = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    let theirs = g.add_card(1, cid("Savannah Lions"), Zone::Battlefield);
    let other = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    cast_now(&mut g, 0, "Abrupt Decay", &[bolt_core::Target::Object(mine)]);
    cast_now(&mut g, 0, "Abrupt Decay", &[bolt_core::Target::Object(theirs)]);
    assert_eq!(g.delayed_slot_to_graveyard.len(), 2);
    kill(&mut g, other);
    assert_eq!(g.obj(other).zone, Zone::Graveyard, "not the one it watches");
    let downfall = |g: &mut Game, x: ObjId| {
        for _ in 0..3 {
            g.add_card(0, cid("Swamp"), Zone::Battlefield);
        }
        cast_now(g, 0, "Hero's Downfall", &[obj(x)]);
    };
    downfall(&mut g, mine);
    assert_eq!(g.obj(mine).zone, Zone::Battlefield, "returned");
    assert_eq!(g.obj(mine).controller, 0);
    downfall(&mut g, mine);
    assert_eq!(g.obj(mine).zone, Zone::Graveyard, "once, and only that object");
    kill(&mut g, theirs);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "not into your graveyard");
}

#[test]
fn instant_or_sorcery_cast_carries_its_mana_value() {
    let mut g = game_with(|defs| {
        ring_with(trigger(
            When::YouCastInstantOrSorceryWithMv,
            None,
            vec![Effect::LoseLifeAmount(Who::EachOpponent)],
        ))(defs);
        defs[cid("Hero's Downfall")].spell = Some(Ability::new(vec![], gain(1)));
    });
    g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    cast_now(&mut g, 0, "Hero's Downfall", &[]);
    assert_eq!(life(&g, 1), 40 - 3, "Hero's Downfall's mana value");
    cast_now(&mut g, 0, "Sol Ring", &[]);
    assert_eq!(life(&g, 1), 37, "not an instant or sorcery");
}

#[test]
fn exiled_face_down_goes_to_hand_at_your_next_end_step() {
    let mut g = game_with(|defs| {
        defs[cid("Savannah Lions")].activated.push(bolt_core::Activated {
            ability: Ability::new(vec![], vec![Effect::ExileTopFaceDownToHandAtYourNextEndStep]),
            ..bolt_core::Activated::default()
        });
    });
    let ring = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    let n = g.def_of(ring).activated.len() - 1;
    let top = *g.players[0].library.last().unwrap();
    act(&mut g, Action::Activate(ring, n));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(top).zone, Zone::Exile);
    assert!(g.obj(top).face_down);
    finish_turn(&mut g, 3);
    assert_eq!(g.obj(top).zone, Zone::Hand, "this turn's end step");
    assert!(!g.obj(top).face_down);
    // On the opponent's turn: not at their end step, at yours.
    let top = *g.players[0].library.last().unwrap();
    // Player 0 gets priority on the opponent's turn.
    while !g.legal_actions().contains(&Action::Activate(ring, n)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.active, 1);
    act(&mut g, Action::Activate(ring, n));
    resolve_all(&mut g, &[]);
    assert!(g.obj(top).face_down);
    finish_turn(&mut g, 4);
    assert_eq!(g.obj(top).zone, Zone::Exile, "not the opponent's end step");
    finish_turn(&mut g, 5);
    assert_eq!(g.obj(top).zone, Zone::Hand, "your next end step");
}

/// The shared token list lines up with its `Tokens` fields: every field a distinct slot inside the
/// list, the list exactly as long as its slots, and the late slots (83-93) the tokens their names
/// say (an old duplicate segment once shifted every token after 83).
#[test]
fn token_slots_line_up_with_their_fields() {
    let defs = bolt_cards::scripts::token_defs();
    let t = bolt_cards::scripts::Tokens::starting_at(0);
    let debug = format!("{t:?}");
    let body = debug.trim_start_matches("Tokens {").trim_end_matches('}');
    let mut slots: Vec<(String, usize)> = body
        .split(',')
        .filter(|f| !f.trim().is_empty())
        .map(|f| {
            let (name, n) = f.split_once(':').expect("field: slot");
            (name.trim().to_string(), n.trim().parse().expect("slot"))
        })
        .collect();
    slots.sort_by_key(|s| s.1);
    for w in slots.windows(2) {
        assert_ne!(w[0].1, w[1].1, "{} and {} share a slot", w[0].0, w[1].0);
    }
    assert_eq!(slots.len(), defs.len(), "one field per token");
    assert_eq!(slots.last().unwrap().1 + 1, defs.len(), "no gap or extra token");
    for (slot, name) in [
        (t.bat, "Bat"),
        (t.elemental_1, "Elemental"),
        (t.bear, "Bear"),
        (t.orc_army, "Orc Army"),
        (t.eldrazi_scion, "Eldrazi Scion"),
        (t.illusion_flying, "Illusion"),
        (t.blood, "Blood"),
        (t.hamster, "Hamster"),
        (t.angel_4, "Angel"),
        (t.pincher, "Pincher"),
        (t.servo, "Servo"),
        (t.walker, "Walker"),
        (t.dragon_6_6, "Dragon"),
    ] {
        assert_eq!(defs[slot].name, name, "slot {slot}");
    }
    assert_eq!((t.bat, t.hamster, t.servo), (83, 90, 93));
    assert!(defs[t.angel_4].keywords.has(bolt_core::Keywords::FLYING));
    assert_eq!(defs[t.angel_4].colors, [true, false, false, false, false]);
    assert!(defs[t.servo].types.artifact && defs[t.servo].types.creature);
    assert_eq!((defs[t.pincher].power, defs[t.pincher].toughness), (2, 2));
    assert_eq!((t.walker, t.dragon_6_6), (94, 96));
    assert_eq!(defs[t.walker].subtypes, vec!["Zombie".to_string()]);
    assert_eq!(defs[t.walker].colors, [false, false, true, false, false]);
    assert_eq!((defs[t.dragon_6_6].power, defs[t.dragon_6_6].toughness), (6, 6));
    assert!(defs[t.dragon_6_6].keywords.has(bolt_core::Keywords::FLYING));
    assert_eq!(defs[t.dragon_6_6].colors, [false, false, false, true, false]);
}

#[test]
fn artifact_you_control_and_another_creature_or_artifact_to_the_graveyard() {
    let mut g = game_with(|defs| {
        blood_with(vec![
            trigger(When::ArtifactYouControlToGraveyardFromBattlefield, None, gain(1)),
            trigger(
                When::AnotherCreatureOrArtifactYouControlToGraveyardFromBattlefield,
                None,
                gain(10),
            ),
        ])(defs);
        let permanent = Filter {
            players: false,
            planeswalkers: false,
            any_permanent: true,
            ..Filter::ANY
        };
        defs[cid("Abrupt Decay")].spell = Some(Ability::new(vec![permanent], vec![Effect::Destroy(0)]));
    });
    g.add_card(0, cid("Exquisite Blood"), Zone::Battlefield);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    let theirs = g.add_card(1, cid("Sol Ring"), Zone::Battlefield);
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    cast_now(&mut g, 0, "Abrupt Decay", &[obj(ring)]);
    assert_eq!(life(&g, 0), 40 + 1 + 10, "your artifact");
    kill(&mut g, lions);
    assert_eq!(life(&g, 0), 51 + 10, "another creature, not an artifact");
    cast_now(&mut g, 0, "Abrupt Decay", &[obj(theirs)]);
    assert_eq!(life(&g, 0), 61, "not yours");
}

#[test]
fn a_delayed_trigger_at_your_next_upkeep_pays_or_loses_the_game() {
    let mut g = game_with(|defs| {
        let d = &mut defs[cid("Hero's Downfall")];
        let n = d.triggers.len();
        d.spell = Some(Ability::new(
            vec![],
            vec![Effect::DelayedAtYourNextUpkeep { trigger: n }],
        ));
        let one = bolt_core::ManaCost::parse("{1}").expect("cost");
        d.triggers
            .push(trigger(When::Never, None, vec![Effect::PayOrLoseGame(one)]));
    });
    cast_now(&mut g, 0, "Hero's Downfall", &[]);
    assert_eq!(g.delayed_next_upkeep.len(), 1);
    g.start_turn(4, 1);
    to_main(&mut g); // the opponent's upkeep: nothing to pay
    let to_decision = |g: &mut Game| {
        g.start_turn(5, 0);
        while !matches!(g.pending, bolt_core::Pending::Decide(_)) {
            act(g, Action::PassPriority);
        }
        assert_eq!(g.step, Step::Upkeep);
    };
    let mut paid = g.clone();
    to_decision(&mut paid);
    assert_eq!(paid.legal_actions(), [Action::Yes, Action::No]);
    act(&mut paid, Action::Yes);
    to_main(&mut paid);
    assert!(!paid.players[0].lost);
    assert!(paid.delayed_next_upkeep.is_empty(), "once");
    to_decision(&mut g);
    act(&mut g, Action::No);
    assert!(
        g.players[0].lost || g.result.is_some(),
        "didn't pay: loses the game"
    );
}

// ---- Batch 16 ----

fn token_def(name: &str) -> usize {
    set("CMD")
        .unwrap()
        .defs
        .iter()
        .position(|d| d.token && d.name == name)
        .unwrap_or_else(|| panic!("the {name} token"))
}

fn attack_with(g: &mut Game, attackers: &[ObjId]) {
    g.start_turn(5, 0);
    while g.pending != bolt_core::Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    for &a in attackers {
        act(g, Action::Attack(a));
    }
    act(g, Action::FinishAttacks);
}

#[test]
fn a_subtype_you_control_attacks_counts_only_that_subtype() {
    let t = trigger(When::SubtypeYouControlAttacks("Cat"), None, gain_one());
    let mut g = game_with(ring_with(t));
    g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    let other = g.add_card(0, cid("Llanowar Elves"), Zone::Battlefield);
    attack_with(&mut g, &[lions, other]);
    resolve_all(&mut g, &[]);
    assert_eq!(life(&g, 0), 41, "once, for the Cat only");
}

#[test]
fn a_subtype_put_into_a_graveyard_from_the_battlefield_is_anyones() {
    let t = trigger(When::SubtypeToGraveyardFromBattlefield("Food"), None, gain_one());
    let mut g = game_with(ring_with(t));
    g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    g.add_card(1, cid("Sol Ring"), Zone::Battlefield);
    let food = g.add_card(0, token_def("Food"), Zone::Battlefield);
    act(&mut g, Action::Activate(food, 0)); // {2}, {T}, sacrifice: gain 3
    resolve_all(&mut g, &[]);
    assert_eq!(life(&g, 0), 44, "3 from the Food, 1 from its trigger");
    assert_eq!(life(&g, 1), 41, "the opponent's trigger sees your Food too");
    let clue = g.add_card(0, token_def("Clue"), Zone::Battlefield);
    act(&mut g, Action::Activate(clue, 0));
    resolve_all(&mut g, &[]);
    assert_eq!((life(&g, 0), life(&g, 1)), (44, 41), "not a Clue");
}

#[test]
fn enters_untapped_triggers_only_untapped() {
    for tapped in [false, true] {
        let mut g = game_with(|defs: &mut Vec<CardDef>| {
            let d = &mut defs[cid("Command Tower")];
            d.enters_tapped = tapped;
            d.triggers.push(trigger(When::EntersUntapped, None, gain_one()));
        });
        let tower = g.add_card(0, cid("Command Tower"), Zone::Hand);
        act(&mut g, Action::PlayLand(tower));
        resolve_all(&mut g, &[]);
        assert_eq!(g.obj(tower).tapped, tapped);
        assert_eq!(life(&g, 0), if tapped { 40 } else { 41 }, "tapped: {tapped}");
    }
}

#[test]
fn pay_x_up_to_the_life_gained_draws_x() {
    let mut g = game_with(|defs: &mut Vec<CardDef>| {
        let d = &mut defs[cid("Sol Ring")];
        d.triggers.push(trigger(
            When::YouGainLife,
            None,
            vec![Effect::MayPayXUpToAmountDrawX],
        ));
    });
    g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    let food = g.add_card(0, token_def("Food"), Zone::Battlefield);
    act(&mut g, Action::Activate(food, 0)); // gain 3
    while !matches!(g.pending, bolt_core::Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    let options = g.legal_actions();
    assert_eq!(
        options,
        vec![
            Action::Amount(0),
            Action::Amount(1),
            Action::Amount(2),
            Action::Amount(3)
        ],
        "X up to the 3 gained"
    );
    let before = g.players[0].hand.len();
    resolve_all(&mut g, &[Action::Amount(3)]);
    assert_eq!(g.players[0].hand.len(), before + 3);
    // Declining draws nothing.
    let food = g.add_card(0, token_def("Food"), Zone::Battlefield);
    act(&mut g, Action::Activate(food, 0));
    let before = g.players[0].hand.len();
    resolve_all(&mut g, &[Action::Amount(0)]);
    assert_eq!(g.players[0].hand.len(), before);
}

#[test]
fn ring_tempts_and_ring_bearer_choice_triggers() {
    let mut g = game_with(|defs: &mut Vec<CardDef>| {
        let d = &mut defs[cid("Sol Ring")];
        d.triggers.push(trigger(When::RingTemptsYou, None, gain_one()));
        d.triggers.push(trigger(When::YouChooseRingBearer, None, gain(2)));
    });
    g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    // Fiery Inscription: "When this enchantment enters, the Ring tempts you."
    let fiery = g.add_card(0, cid("Fiery Inscription"), Zone::Hand);
    play(&mut g, fiery, false, &[], &[]); // no creature: tempted, nobody chosen
    assert_eq!(life(&g, 0), 41);
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    let fiery = g.add_card(0, cid("Fiery Inscription"), Zone::Hand);
    play(&mut g, fiery, false, &[], &[]); // the only creature is chosen
    assert_eq!(g.ring_bearer(0), Some(lions));
    assert_eq!(life(&g, 0), 44, "tempted and chose");
}

#[test]
fn ring_bearer_combat_damage_drains_three_at_the_fourth_level() {
    let mut g = game_with(|_| {});
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Battlefield);
    for _ in 0..4 {
        g.ring_tempts(0);
    }
    attack_with(&mut g, &[lions]);
    // Through the loot (draw, then discard) and the rest of combat, taking the first answer.
    let mut steps = 0;
    while g.step != Step::Main2 && steps < 200 {
        let a = match g.pending {
            bolt_core::Pending::Decide(_) | bolt_core::Pending::ChooseTarget(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
        steps += 1;
    }
    assert_eq!(life(&g, 1), 40 - 2 - 3, "2 combat damage, then the Ring's 3");
}
