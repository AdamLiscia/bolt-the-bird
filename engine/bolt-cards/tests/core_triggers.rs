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
