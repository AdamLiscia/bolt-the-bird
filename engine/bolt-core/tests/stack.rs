//! The stack, targets, triggers and until-end-of-turn effects (CR 115, 405, 601, 603, 608, 611).

use std::sync::Arc;

use bolt_core::{
    Ability, Action, CardDef, Color, Controller, Effect, Event, Filter, Game, Keywords, ObjId, Pending,
    Recipient, Step, Target, Trigger, When, Who, Zone,
};

const MOUNTAIN: usize = 0;
const BEAR: usize = 1;
const SHOCK: usize = 2; // instant, 2 damage to any target
const BOOST: usize = 3; // instant, +3/+3 until end of turn
const HUNTER: usize = 4; // 1/1, enters: draw a card
const SAGE: usize = 5; // 1/1, dies: you gain 3 life
const SPITE: usize = 6; // sorcery, 3 damage to each opponent
const BLAST: usize = 7; // instant, kicker {4}: 2 damage, 4 if kicked
const WARD: usize = 8; // 2/2 hexproof
const PINGER: usize = 9; // 2/2, enters: 1 damage to target creature an opponent controls
const GOLEM: usize = 10; // uncastable filler for libraries

fn defs() -> Arc<Vec<CardDef>> {
    let any = Filter::ANY;
    let mut hunter = CardDef::creature("Hunter", "{1}{R}", 1, 1, Keywords::NONE);
    hunter.triggers.push(Trigger {
        when: When::Enters,
        ability: Ability {
            targets: vec![],
            effects: vec![Effect::Draw { who: Who::You, n: 1 }],
        },
    });
    let mut sage = CardDef::creature("Sage", "{R}", 1, 1, Keywords::NONE);
    sage.triggers.push(Trigger {
        when: When::Dies,
        ability: Ability {
            targets: vec![],
            effects: vec![Effect::GainLife { who: Who::You, n: 3 }],
        },
    });
    let mut blast = CardDef::spell(
        "Blast",
        "{R}",
        true,
        Ability {
            targets: vec![any],
            effects: vec![Effect::Damage {
                amount: 2,
                kicked: Some(4),
                to: Recipient::Target(0),
            }],
        },
    );
    blast.kicker = Some(bolt_core::ManaCost::parse("{4}").unwrap());
    let mut pinger = CardDef::creature("Pinger", "{1}{R}", 2, 2, Keywords::NONE);
    pinger.triggers.push(Trigger {
        when: When::Enters,
        ability: Ability {
            targets: vec![Filter::CREATURE.controlled_by(Controller::Opponent)],
            effects: vec![Effect::Damage {
                amount: 1,
                kicked: None,
                to: Recipient::Target(0),
            }],
        },
    });
    Arc::new(vec![
        CardDef::basic_land("Mountain", Color::Red),
        CardDef::creature("Bear", "{1}{R}", 2, 2, Keywords::NONE),
        CardDef::spell(
            "Shock",
            "{R}",
            true,
            Ability {
                targets: vec![any],
                effects: vec![Effect::Damage {
                    amount: 2,
                    kicked: None,
                    to: Recipient::Target(0),
                }],
            },
        ),
        CardDef::spell(
            "Boost",
            "{R}",
            true,
            Ability {
                targets: vec![Filter::CREATURE],
                effects: vec![Effect::Pump {
                    slot: 0,
                    power: 3,
                    toughness: 3,
                }],
            },
        ),
        hunter,
        sage,
        CardDef::spell(
            "Spite",
            "{R}",
            false,
            Ability {
                targets: vec![],
                effects: vec![Effect::Damage {
                    amount: 3,
                    kicked: None,
                    to: Recipient::EachOpponent,
                }],
            },
        ),
        blast,
        CardDef::creature("Warded", "{1}{R}", 2, 2, Keywords::HEXPROOF),
        pinger,
        CardDef::creature("Golem", "{9}", 9, 9, Keywords::NONE),
    ])
}

/// Player 0's main phase on turn 3, stopping at every priority, with `lands` Mountains each.
fn main_phase(lands: usize) -> Game {
    let mut g = Game::empty(defs(), 1, 0);
    g.auto_pass = false;
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, GOLEM, Zone::Library);
        }
        for _ in 0..lands {
            g.add_card(p, MOUNTAIN, Zone::Battlefield);
        }
    }
    g
}

fn start(g: &mut Game) {
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(g, Action::PassPriority);
    }
}

fn act(g: &mut Game, a: Action) -> Vec<Event> {
    g.apply(a)
        .unwrap_or_else(|e| panic!("{e}; legal: {:?}", g.legal_actions()))
}

fn both_pass(g: &mut Game) -> Vec<Event> {
    let mut ev = act(g, Action::PassPriority);
    ev.extend(act(g, Action::PassPriority));
    ev
}

fn hand(g: &mut Game, p: usize, def: usize) -> ObjId {
    g.add_card(p, def, Zone::Hand)
}

#[test]
fn a_targeted_spell_asks_for_targets_then_resolves() {
    // CR 601.2c, 608.2.
    let mut g = main_phase(1);
    let shock = hand(&mut g, 0, SHOCK);
    let bear = g.add_card(1, BEAR, Zone::Battlefield);
    start(&mut g);
    act(&mut g, Action::Cast(shock));
    assert_eq!(g.pending, Pending::ChooseTarget(0));
    let options = g.legal_actions();
    assert!(options.contains(&Action::Target(Target::Player(1))));
    assert!(options.contains(&Action::Target(Target::Object(bear))));
    act(&mut g, Action::Target(Target::Object(bear)));
    assert_eq!(g.items.len(), 1);
    both_pass(&mut g);
    assert_eq!(g.obj(bear).zone, Zone::Graveyard);
    assert_eq!(g.obj(shock).zone, Zone::Graveyard);
}

#[test]
fn last_in_first_out() {
    // CR 405.5: the top of the stack resolves first. Shock at a bear, answered by Boost: the bear
    // survives.
    let mut g = main_phase(1);
    let shock = hand(&mut g, 0, SHOCK);
    let boost = hand(&mut g, 1, BOOST);
    let bear = g.add_card(1, BEAR, Zone::Battlefield);
    start(&mut g);
    act(&mut g, Action::Cast(shock));
    act(&mut g, Action::Target(Target::Object(bear)));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(boost));
    act(&mut g, Action::Target(Target::Object(bear)));
    both_pass(&mut g); // Boost resolves
    assert_eq!((g.power(bear), g.toughness(bear)), (5, 5));
    both_pass(&mut g); // Shock resolves
    assert_eq!(g.obj(bear).zone, Zone::Battlefield);
    assert_eq!(g.obj(bear).damage, 2);
}

#[test]
fn a_spell_whose_only_target_is_gone_fizzles() {
    // CR 608.2b. Two Shocks at one bear: the second has no legal target left.
    let mut g = main_phase(2);
    let s1 = hand(&mut g, 0, SHOCK);
    let s2 = hand(&mut g, 0, SHOCK);
    let bear = g.add_card(1, BEAR, Zone::Battlefield);
    start(&mut g);
    act(&mut g, Action::Cast(s1));
    act(&mut g, Action::Target(Target::Object(bear)));
    act(&mut g, Action::Cast(s2));
    act(&mut g, Action::Target(Target::Object(bear)));
    both_pass(&mut g); // s2 kills the bear
    let ev = both_pass(&mut g);
    assert!(ev.contains(&Event::Fizzled { source: s1 }));
    assert_eq!(g.obj(s1).zone, Zone::Graveyard);
}

#[test]
fn pump_wears_off_at_end_of_turn() {
    // CR 514.2, 611.2a.
    let mut g = main_phase(1);
    let boost = hand(&mut g, 0, BOOST);
    let bear = g.add_card(0, BEAR, Zone::Battlefield);
    start(&mut g);
    act(&mut g, Action::Cast(boost));
    act(&mut g, Action::Target(Target::Object(bear)));
    both_pass(&mut g);
    assert_eq!(g.power(bear), 5);
    while g.turn == 3 {
        let a = if g.legal_actions().contains(&Action::PassPriority) {
            Action::PassPriority
        } else {
            Action::FinishAttacks
        };
        act(&mut g, a);
    }
    assert_eq!(g.power(bear), 2);
}

#[test]
fn sorceries_wait_for_an_empty_stack_in_your_main_phase() {
    // CR 307.1.
    let mut g = main_phase(2);
    let spite = hand(&mut g, 0, SPITE);
    let shock = hand(&mut g, 0, SHOCK);
    start(&mut g);
    act(&mut g, Action::Cast(shock));
    act(&mut g, Action::Target(Target::Player(1)));
    assert!(
        !g.legal_actions().contains(&Action::Cast(spite)),
        "stack is not empty"
    );
    both_pass(&mut g);
    act(&mut g, Action::Cast(spite));
    both_pass(&mut g);
    assert_eq!(g.players[1].life, 15);
}

#[test]
fn kicker_costs_more_and_does_more() {
    // CR 702.33.
    let mut g = main_phase(5);
    let blast = hand(&mut g, 0, BLAST);
    start(&mut g);
    act(&mut g, Action::CastKicked(blast));
    act(&mut g, Action::Target(Target::Player(1)));
    assert_eq!(g.permanents(0).filter(|&id| g.obj(id).tapped).count(), 5);
    both_pass(&mut g);
    assert_eq!(g.players[1].life, 16);

    let mut g = main_phase(4);
    let blast = hand(&mut g, 0, BLAST);
    start(&mut g);
    assert!(
        !g.legal_actions().contains(&Action::CastKicked(blast)),
        "kicker needs 5 mana"
    );
}

#[test]
fn hexproof_cannot_be_targeted_by_opponents() {
    // CR 702.11b.
    let mut g = main_phase(1);
    let shock = hand(&mut g, 0, SHOCK);
    let warded = g.add_card(1, WARD, Zone::Battlefield);
    start(&mut g);
    act(&mut g, Action::Cast(shock));
    assert!(!g
        .legal_actions()
        .contains(&Action::Target(Target::Object(warded))));
}

#[test]
fn enters_trigger_uses_the_stack() {
    // CR 603.3, 603.6a.
    let mut g = main_phase(2);
    let hunter = hand(&mut g, 0, HUNTER);
    start(&mut g);
    let before = g.players[0].hand.len();
    act(&mut g, Action::Cast(hunter));
    let ev = both_pass(&mut g); // hunter resolves, trigger goes on the stack
    assert!(ev.contains(&Event::Triggered { source: hunter }));
    assert_eq!(g.items.len(), 1);
    both_pass(&mut g); // trigger resolves
    assert_eq!(g.players[0].hand.len(), before, "cast one, drew one");
}

#[test]
fn dies_trigger_fires_from_the_graveyard() {
    // CR 603.10a: leaves-the-battlefield abilities look back in time.
    let mut g = main_phase(1);
    let shock = hand(&mut g, 0, SHOCK);
    let sage = g.add_card(1, SAGE, Zone::Battlefield);
    start(&mut g);
    act(&mut g, Action::Cast(shock));
    act(&mut g, Action::Target(Target::Object(sage)));
    both_pass(&mut g);
    assert_eq!(g.obj(sage).zone, Zone::Graveyard);
    both_pass(&mut g);
    assert_eq!(g.players[1].life, 23, "the sage's controller gains 3");
}

#[test]
fn targeted_trigger_asks_its_controller_and_skips_with_no_target() {
    // CR 603.3d.
    let mut g = main_phase(2);
    let pinger = hand(&mut g, 0, PINGER);
    let bear = g.add_card(1, BEAR, Zone::Battlefield);
    start(&mut g);
    act(&mut g, Action::Cast(pinger));
    both_pass(&mut g);
    assert_eq!(g.pending, Pending::ChooseTarget(0));
    assert_eq!(g.legal_actions(), vec![Action::Target(Target::Object(bear))]);
    act(&mut g, Action::Target(Target::Object(bear)));
    both_pass(&mut g);
    assert_eq!(g.obj(bear).damage, 1);

    let mut g = main_phase(2);
    let pinger = hand(&mut g, 0, PINGER);
    start(&mut g);
    act(&mut g, Action::Cast(pinger));
    both_pass(&mut g);
    assert!(g.items.is_empty(), "no opposing creature: the trigger is removed");
    assert_eq!(g.pending, Pending::Priority(0));
}

#[test]
fn a_creature_that_left_and_came_back_is_a_new_object() {
    // CR 400.7: the zone-change counter breaks old targets.
    let mut g = main_phase(1);
    let shock = hand(&mut g, 0, SHOCK);
    let bear = g.add_card(1, BEAR, Zone::Battlefield);
    start(&mut g);
    act(&mut g, Action::Cast(shock));
    act(&mut g, Action::Target(Target::Object(bear)));
    let zcc = g.obj(bear).zcc;
    g.objects[bear.0 as usize].zcc = zcc + 2; // as if it blinked
    let ev = both_pass(&mut g);
    assert!(ev.contains(&Event::Fizzled { source: shock }));
    assert_eq!(g.obj(bear).zone, Zone::Battlefield);
}
