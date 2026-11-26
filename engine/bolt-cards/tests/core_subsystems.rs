//! Core theme "subsystems" (lane d4): dungeons, venture and the initiative, completed dungeons,
//! emblems with triggered abilities, and dice, each tested directly on a card definition edited to
//! carry it, in a 1v1 Commander game on the CMD pool.
#![allow(dead_code)]

mod common;

use std::sync::Arc;

use bolt_cards::set;
use bolt_core::effect::{Ability, Effect, Filter, Scope, Static, StaticCondition, Trigger, When, Who};
use bolt_core::{
    rooms, Action, Activated, CardDef, Controller, Dungeon, EmblemKind, Game, Keywords, ObjId, Pending,
    StackKind, Step, SubsystemEffect, Target, Zone, VENTURE_CHOICES,
};
use common::act;

fn cid(name: &str) -> usize {
    set("CMD")
        .unwrap()
        .index_of(name)
        .unwrap_or_else(|| panic!("{name} is not in the CMD pool"))
}

/// The test card: Savannah Lions, given "{0}: <effects>" (its activated ability 0).
const CARD: &str = "Savannah Lions";

fn sub(e: SubsystemEffect) -> Effect {
    Effect::Subsystem(e)
}

/// A Commander game on the CMD pool with the test card's "{0}: `effects`" (targets `targets`), at
/// player 0's first main phase on turn 3, with 5 of each basic for both players.
fn game_with(targets: Vec<Filter>, effects: Vec<Effect>, edit: impl Fn(&mut Vec<CardDef>)) -> Game {
    let mut defs: Vec<CardDef> = (*set("CMD").unwrap().defs).clone();
    defs[cid(CARD)].activated.push(Activated {
        ability: Ability::new(targets, effects),
        ..Activated::default()
    });
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
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g
}

fn venture_game() -> Game {
    game_with(vec![], vec![sub(SubsystemEffect::Venture)], |_| {})
}

/// Activates the test card's ability and resolves everything: decisions take `answers` in order,
/// then the first legal answer.
fn activate(g: &mut Game, card: ObjId, answers: &[Action]) {
    act(g, Action::Activate(card, 0));
    settle(g, answers);
}

fn settle(g: &mut Game, answers: &[Action]) {
    let mut answers = answers.iter();
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) | Pending::Decide(_) => match answers.next() {
                Some(&a) => a,
                None => g.legal_actions()[0],
            },
            _ => Action::PassPriority,
        };
        act(g, a);
    }
    assert!(answers.next().is_none(), "unused answers");
}

fn tokens(g: &Game, p: usize, name: &str) -> usize {
    g.permanents(p)
        .filter(|&x| g.is_token(x) && g.def_of(x).name == name)
        .count()
}

#[test]
fn every_dungeon_room_leads_down_to_one_bottommost_room() {
    for d in [
        Dungeon::LostMine,
        Dungeon::MadMage,
        Dungeon::Tomb,
        Dungeon::Undercity,
    ] {
        let r = rooms(d);
        for (i, room) in r.iter().enumerate() {
            assert!(
                room.next
                    .iter()
                    .all(|&n| (n as usize) > i && (n as usize) < r.len()),
                "{}",
                room.name
            );
        }
        assert_eq!(r.iter().filter(|x| x.next.is_empty()).count(), 1, "{d:?}");
        assert!(r.last().unwrap().next.is_empty());
    }
    assert!(
        !VENTURE_CHOICES.contains(&Dungeon::Undercity),
        "CR 701.49a: not Undercity"
    );
}

#[test]
fn rooms_still_missing_a_piece_are_listed() {
    let missing: Vec<&str> = [
        Dungeon::LostMine,
        Dungeon::MadMage,
        Dungeon::Tomb,
        Dungeon::Undercity,
    ]
    .iter()
    .flat_map(|&d| rooms(d).iter().filter(|r| !r.ready).map(|r| r.name))
    .collect();
    assert_eq!(
        missing,
        [
            "Twisted Caverns",
            "Runestone Caverns",
            "Mad Wizard's Lair",
            "Arena"
        ]
    );
}

#[test]
fn fungi_cavern_gives_target_creature_minus_four_until_your_next_turn() {
    let mut g = venture_game();
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    let y = g.add_card(1, cid(CARD), Zone::Battlefield);
    g.venture[0] = Some((Dungeon::LostMine, 2)); // Mine Tunnels
    activate(&mut g, x, &[Action::Mode(1), Action::Target(Target::Object(y))]);
    assert_eq!(g.venture[0], Some((Dungeon::LostMine, 5)));
    assert_eq!((g.power(y), g.toughness(y)), (-2, 1));
}

#[test]
fn veils_of_fear_and_sandfall_cell_make_each_player_lose_two_unless() {
    let mut g = venture_game();
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    g.venture[0] = Some((Dungeon::Tomb, 0)); // Trapped Entry
    let (me, them) = (g.players[0].life, g.players[1].life);
    activate(&mut g, x, &[Action::Mode(0), Action::No, Action::No]); // Veils of Fear
    assert_eq!(g.venture[0], Some((Dungeon::Tomb, 1)));
    assert_eq!((g.players[0].life, g.players[1].life), (me - 2, them - 2));
    let lands = g.permanents(1).count();
    activate(&mut g, x, &[Action::No]); // Sandfall Cell: 0 loses, 1 sacrifices (first Pick)
    assert_eq!(g.venture[0], Some((Dungeon::Tomb, 3)));
    assert_eq!((g.players[0].life, g.players[1].life), (me - 4, them - 2));
    assert_eq!(g.permanents(1).count(), lands - 1);
}

#[test]
fn oubliette_discards_and_sacrifices_an_artifact_a_creature_and_a_land() {
    let mut g = venture_game();
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    g.add_card(0, cid("Plains"), Zone::Hand);
    let hand = g.players[0].hand.len();
    let lands = g.permanents(0).filter(|&p| g.types_of(p).land).count();
    g.venture[0] = Some((Dungeon::Tomb, 0));
    activate(&mut g, x, &[Action::Mode(1)]);
    assert_eq!(g.venture[0], Some((Dungeon::Tomb, 2)));
    assert_eq!(g.players[0].hand.len(), hand - 1);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    assert_eq!(g.obj(x).zone, Zone::Graveyard);
    assert_eq!(g.permanents(0).filter(|&p| g.types_of(p).land).count(), lands - 1);
}

#[test]
fn throne_of_the_dead_three_puts_a_creature_on_with_three_counters_and_completes_undercity() {
    let mut g = game_with(vec![], vec![sub(SubsystemEffect::VentureUndercity)], |_| {});
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    let elf = g.add_card(0, cid("Llanowar Elves"), Zone::Library);
    g.venture[0] = Some((Dungeon::Undercity, 7)); // Catacombs
    activate(&mut g, x, &[]);
    assert_eq!(g.obj(elf).zone, Zone::Battlefield);
    assert_eq!(g.power(elf), 4);
    assert!(g.keywords(elf).has(Keywords::HEXPROOF));
    assert!(g.completed_a_dungeon(0));
    assert_eq!(g.venture[0], None);
}

#[test]
fn venturing_runs_lost_mine_of_phandelver_top_to_bottom_and_completes_it() {
    let mut g = venture_game();
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    activate(&mut g, x, &[Action::Mode(0)]); // Lost Mine of Phandelver; Cave Entrance: scry 1
    assert_eq!(g.venture[0], Some((Dungeon::LostMine, 0)));
    activate(&mut g, x, &[Action::Mode(0)]); // Goblin Lair
    assert_eq!(g.venture[0], Some((Dungeon::LostMine, 1)));
    assert_eq!(tokens(&g, 0, "Goblin"), 1);
    let (me, them) = (g.players[0].life, g.players[1].life);
    activate(&mut g, x, &[Action::Mode(1)]); // Dark Pool
    assert_eq!((g.players[0].life, g.players[1].life), (me + 1, them - 1));
    let hand = g.players[0].hand.len();
    assert!(!g.completed_a_dungeon(0));
    activate(&mut g, x, &[]); // Temple of Dumathoin, the only way on
    assert_eq!(g.players[0].hand.len(), hand + 1);
    assert!(g.completed_a_dungeon(0));
    assert_eq!(g.venture[0], None, "the dungeon leaves the game");
    activate(&mut g, x, &[Action::Mode(1)]); // a new dungeon: Dungeon of the Mad Mage
    assert_eq!(g.venture[0], Some((Dungeon::MadMage, 0)));
    assert_eq!(g.completed_dungeons[0], 1);
}

#[test]
fn a_room_with_two_ways_on_asks_which() {
    let mut g = venture_game();
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    activate(&mut g, x, &[Action::Mode(0)]);
    act(&mut g, Action::Activate(x, 0));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert_eq!(g.legal_actions(), [Action::Mode(0), Action::Mode(1)]);
    act(&mut g, Action::Mode(1)); // Mine Tunnels: a Treasure
    settle(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Treasure"), 1);
}

#[test]
fn hama_pashar_makes_room_abilities_trigger_twice() {
    let mut g = venture_game();
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    assert_eq!(g.room_trigger_count(0), 1);
    g.add_card(0, cid("Hama Pashar, Ruin Seeker"), Zone::Battlefield);
    assert_eq!(g.room_trigger_count(0), 2);
    assert_eq!(g.room_trigger_count(1), 1, "only its controller's dungeons");
    activate(&mut g, x, &[Action::Mode(0)]);
    activate(&mut g, x, &[Action::Mode(0)]); // Goblin Lair, twice
    assert_eq!(tokens(&g, 0, "Goblin"), 2);
}

#[test]
fn dungeon_delver_gives_your_commander_creature_the_doubling() {
    let mut g = venture_game();
    g.add_card(0, cid("Dungeon Delver"), Zone::Battlefield);
    assert_eq!(
        g.room_trigger_count(0),
        1,
        "no commander creature on the battlefield"
    );
    let c = g.add_card(0, cid("Llanowar Elves"), Zone::Battlefield);
    g.commanders[0] = Some(c);
    assert_eq!(g.room_trigger_count(0), 2);
    assert_eq!(g.room_trigger_count(1), 1);
}

#[test]
fn the_bottommost_room_completes_only_after_its_last_copy_resolves() {
    let mut g = venture_game();
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    g.add_card(0, cid("Hama Pashar, Ruin Seeker"), Zone::Battlefield);
    for a in [Action::Mode(0), Action::Mode(0), Action::Mode(1)] {
        activate(&mut g, x, &[a]);
    }
    let hand = g.players[0].hand.len();
    activate(&mut g, x, &[]); // Temple of Dumathoin, twice
    assert_eq!(g.players[0].hand.len(), hand + 2);
    assert_eq!(g.completed_dungeons[0], 1, "completed once");
}

#[test]
fn taking_the_initiative_ventures_into_undercity() {
    let mut g = game_with(vec![], vec![sub(SubsystemEffect::TakeInitiative)], |_| {});
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    activate(&mut g, x, &[]); // Secret Entrance: a basic land card into the hand
    assert_eq!(g.initiative, Some((0, x)));
    assert_eq!(g.venture[0], Some((Dungeon::Undercity, 0)));
}

#[test]
fn the_initiative_ventures_at_its_holders_upkeep_and_moves_with_combat_damage() {
    let mut g = venture_game();
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    g.take_initiative(0, x);
    g.triggered.clear();
    g.fire_subsystems(1, When::Upkeep { your_turn_only: true });
    assert!(g.triggered.is_empty(), "not the holder's upkeep");
    g.fire_subsystems(0, When::Upkeep { your_turn_only: true });
    assert_eq!(g.triggered.len(), 1);
    assert_eq!(
        (g.triggered[0].kind, g.triggered[0].controller),
        (StackKind::InitiativeVenture, 0)
    );
    g.triggered.clear();
    g.fire_subsystems(0, When::YourCreaturesDealCombatDamageToPlayer);
    assert_eq!(
        g.initiative,
        Some((0, x)),
        "damage to the opponent, who doesn't have it"
    );
    g.fire_subsystems(1, When::YourCreaturesDealCombatDamageToPlayer);
    assert_eq!(
        g.initiative.map(|i| i.0),
        Some(1),
        "damage to the holder: its controller takes it"
    );
    assert_eq!(
        g.triggered.last().map(|t| (t.kind, t.controller)),
        Some((StackKind::InitiativeVenture, 1))
    );
}

#[test]
fn a_completed_dungeon_turns_on_its_static_condition() {
    let mut g = venture_game_with_static();
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    assert_eq!(g.power(x), 2);
    g.completed_dungeons[0] = 1;
    assert_eq!(g.power(x), 3);
    g.completed_dungeons = [0, 1];
    assert_eq!(g.power(x), 2, "its controller's dungeons");
}

fn venture_game_with_static() -> Game {
    game_with(vec![], vec![], |defs| {
        defs[cid(CARD)].statics.push(Static {
            subtype: None,
            lose_abilities: false,
            base_pt: None,
            protection: [false; 5],
            scale: None,
            scope: Scope::This,
            your_turn_only: false,
            condition: Some(StaticCondition::CompletedDungeon),
            power: 1,
            toughness: 1,
            keywords: Keywords::NONE,
            set_colors: None,
            becomes_land: false,
        });
    })
}

#[test]
fn an_emblem_with_a_triggered_ability_triggers_for_its_owner() {
    let mut g = game_with(vec![], vec![], |defs| {
        defs[cid(CARD)].triggers.push(Trigger {
            when: When::Never,
            ability: Ability::new(vec![], vec![Effect::GainLife { who: Who::You, n: 1 }]),
        });
    });
    let x = g.add_card(0, cid(CARD), Zone::Graveyard);
    let n = g.def_of(x).triggers.len() - 1;
    g.emblems.push((
        0,
        EmblemKind::Triggered {
            when: When::YouDraw,
            trigger: n,
        },
        x,
    ));
    g.fire_subsystems(1, When::YouDraw);
    g.fire_subsystems(0, When::YouGainLife);
    assert!(g.triggered.is_empty());
    g.fire_subsystems(0, When::YouDraw);
    assert_eq!(g.triggered.len(), 1);
    assert_eq!(
        (g.triggered[0].kind, g.triggered[0].source),
        (StackKind::Trigger(n), x)
    );
}

#[test]
fn an_emblem_for_target_opponent_is_theirs() {
    let kind = EmblemKind::NoMaxHandSize;
    let mut g = game_with(
        vec![Filter::PLAYER.controlled_by(Controller::Opponent)],
        vec![sub(SubsystemEffect::EmblemForTarget { slot: 0, kind })],
        |_| {},
    );
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    act(&mut g, Action::Activate(x, 0));
    act(&mut g, Action::Target(Target::Player(1)));
    settle(&mut g, &[]);
    assert_eq!(g.emblems, [(1, kind, x)]);
}

#[test]
fn a_room_token_is_found_by_name_and_size() {
    let effects = vec![
        sub(SubsystemEffect::RoomToken {
            name: "Skeleton",
            power: 4,
            toughness: 1,
            n: 1,
        }),
        sub(SubsystemEffect::RoomToken {
            name: "Skeleton",
            power: 1,
            toughness: 1,
            n: 2,
        }),
        sub(SubsystemEffect::RoomToken {
            name: "God Horror",
            power: 4,
            toughness: 4,
            n: 1,
        }),
    ];
    let mut g = game_with(vec![], effects, |_| {});
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    activate(&mut g, x, &[]);
    let mine: Vec<ObjId> = g.permanents(0).filter(|&t| g.is_token(t)).collect();
    let sizes: Vec<(i32, i32)> = mine.iter().map(|&t| (g.power(t), g.toughness(t))).collect();
    assert_eq!(sizes, [(4, 1), (1, 1), (1, 1), (4, 4)]);
    assert!(g.keywords(mine[0]).has(Keywords::MENACE));
    assert!(g.keywords(mine[3]).has(Keywords::DEATHTOUCH));
    assert_eq!(g.colors_of(mine[3]), [false, false, true, false, false], "black");
}

#[test]
fn a_chapter_grant_is_had_once_it_resolves_and_until_the_saga_leaves() {
    let mut g = game_with(
        vec![],
        vec![sub(SubsystemEffect::GainChapterAbilities(1))],
        |defs| {
            let ext = &mut defs[cid(CARD)].subsystems;
            ext.chapter_mana = Some(1);
            ext.chapter_activated = vec![(2, 5)];
        },
    );
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    assert!(g.lacks_chapter_mana(x) && g.lacks_chapter_activated(x, 5));
    assert!(!g.lacks_chapter_activated(x, 0), "not a grant");
    activate(&mut g, x, &[]);
    assert!(!g.lacks_chapter_mana(x), "chapter I resolved");
    assert!(g.lacks_chapter_activated(x, 5), "chapter II hasn't");
    g.objects[x.0 as usize].zcc += 1; // it left and came back: a new object
    assert!(g.lacks_chapter_mana(x), "a new object");
}

#[test]
fn a_completed_dungeon_draws_more_and_makes_more_tokens() {
    let treasure = set("CMD")
        .unwrap()
        .defs
        .iter()
        .position(|d| d.token && d.name == "Treasure")
        .unwrap();
    let effects = vec![
        sub(SubsystemEffect::DrawIfCompletedDungeon(1)),
        sub(SubsystemEffect::TokensByDungeon {
            def: treasure,
            n: 1,
            completed: 3,
        }),
    ];
    let mut g = game_with(vec![], effects, |_| {});
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    let hand = g.players[0].hand.len();
    activate(&mut g, x, &[]);
    assert_eq!((g.players[0].hand.len(), tokens(&g, 0, "Treasure")), (hand, 1));
    g.completed_dungeons[0] = 1;
    activate(&mut g, x, &[]);
    assert_eq!(
        (g.players[0].hand.len(), tokens(&g, 0, "Treasure")),
        (hand + 1, 4)
    );
}

#[test]
fn untap_one_creature_or_all_of_them_with_a_completed_dungeon() {
    let mut g = game_with(vec![], vec![sub(SubsystemEffect::UntapOneOrAllByDungeon)], |_| {});
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    let a = g.add_card(0, cid("Llanowar Elves"), Zone::Battlefield);
    let b = g.add_card(0, cid("Llanowar Elves"), Zone::Battlefield);
    g.objects[a.0 as usize].tapped = true;
    g.objects[b.0 as usize].tapped = true;
    act(&mut g, Action::Activate(x, 0));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert_eq!(
        g.legal_actions(),
        [Action::Pick(a), Action::Pick(b)],
        "only tapped ones"
    );
    act(&mut g, Action::Pick(b));
    settle(&mut g, &[]);
    assert!(g.obj(a).tapped && !g.obj(b).tapped);
    g.objects[b.0 as usize].tapped = true;
    g.completed_dungeons[0] = 1;
    activate(&mut g, x, &[]);
    assert!(!g.obj(a).tapped && !g.obj(b).tapped, "all of them");
}

#[test]
fn the_opponent_loses_the_difference_from_life_lost_this_turn() {
    let mut g = game_with(
        vec![],
        vec![sub(SubsystemEffect::OpponentLosesDifferenceFrom(8))],
        |defs| {
            defs[cid(CARD)].activated[0].ability.effects.insert(
                0,
                Effect::LoseLife {
                    who: Who::EachOpponent,
                    n: 3,
                },
            )
        },
    );
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    let them = g.players[1].life;
    activate(&mut g, x, &[]); // loses 3, then 8 - 3
    assert_eq!(g.players[1].life, them - 8);
    activate(&mut g, x, &[]); // loses 3 more: 11 this turn, so nothing
    assert_eq!(g.players[1].life, them - 11);
}

#[test]
fn a_lost_coin_flip_deals_the_damage_to_you() {
    let mut g = game_with(
        vec![],
        vec![sub(SubsystemEffect::FlipCoinLoseDamageYou(3))],
        |_| {},
    );
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    let wins = (0..200).filter(|_| g.flip_coin()).count();
    assert!((60..=140).contains(&wins), "{wins}");
    let mut seen = [false; 2];
    for _ in 0..12 {
        let life = g.players[0].life;
        activate(&mut g, x, &[]);
        let lost = life - g.players[0].life;
        assert!(lost == 0 || lost == 3);
        seen[(lost == 3) as usize] = true;
    }
    assert_eq!(seen, [true, true]);
    assert!(g.players[1].life == 40, "only you");
}

#[test]
fn a_d20_rolls_one_through_twenty() {
    let mut g = venture_game();
    let mut seen = [false; 21];
    for _ in 0..2000 {
        let r = g.roll_die(20) as usize;
        assert!((1..=20).contains(&r));
        seen[r] = true;
    }
    assert!(seen[1..].iter().all(|&s| s));
}

#[test]
fn rolling_a_die_creates_that_many_tokens() {
    let faerie = set("CMD")
        .unwrap()
        .defs
        .iter()
        .position(|d| d.token && d.name == "Faerie")
        .unwrap();
    let mut g = game_with(
        vec![],
        vec![sub(SubsystemEffect::RollDieCreateTokens {
            sides: 20,
            def: faerie,
        })],
        |_| {},
    );
    let x = g.add_card(0, cid(CARD), Zone::Battlefield);
    activate(&mut g, x, &[]);
    assert!((1..=20).contains(&tokens(&g, 0, "Faerie")));
}
