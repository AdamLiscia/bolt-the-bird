//! Lane d3's core theme, effects and statics: each new piece tested directly, on a test card built
//! here (appended to the CMD pool's definitions), not only through a printed card.
#![allow(dead_code)]

mod common;

use bolt_cards::set;
use bolt_core::effect::{Ability, Effect, Filter, Scope, SearchFor, SearchTo, Static, Trigger, When, Who};
use bolt_core::{Action, CardDef, Game, Keywords, ManaCost, ObjId, Pending, Step, Target, Types, Zone};
use common::{act, obj, play, resolve_all};

fn cid(name: &str) -> usize {
    set("CMD")
        .unwrap()
        .index_of(name)
        .unwrap_or_else(|| panic!("{name} is not in the CMD pool"))
}

/// A free sorcery doing `effects` with `targets`.
fn sorcery(targets: Vec<Filter>, effects: Vec<Effect>) -> CardDef {
    CardDef {
        name: "Test Sorcery".into(),
        cost: ManaCost::parse("{0}").unwrap(),
        types: Types {
            sorcery: true,
            ..Types::default()
        },
        spell: Some(Ability::new(targets, effects)),
        ..CardDef::default()
    }
}

/// A Commander game on the CMD pool plus `extra` test definitions (their indices follow the pool's),
/// at player 0's first main phase on turn 3, 5 of each basic for both players.
fn game_with(extra: Vec<CardDef>) -> (Game, usize) {
    let mut defs = (*set("CMD").unwrap().defs).clone();
    let first = defs.len();
    defs.extend(extra);
    let plains = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        std::sync::Arc::new(defs),
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
    (g, first)
}

fn cast_test(g: &mut Game, def: usize, targets: &[Target], answers: &[Action]) {
    let c = g.add_card(0, def, Zone::Hand);
    play(g, c, false, targets, answers);
}

fn count(g: &Game, p: usize, f: impl Fn(&Game, ObjId) -> bool) -> usize {
    g.permanents(p).filter(|&x| f(g, x)).count()
}

#[test]
fn destroy_all_artifacts_and_destroy_all_lands() {
    let (mut g, t) = game_with(vec![sorcery(vec![], vec![Effect::DestroyAllArtifacts])]);
    let mine = g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    let theirs = g.add_card(1, cid("Sol Ring"), Zone::Battlefield);
    cast_test(&mut g, t, &[], &[]);
    assert_eq!(
        (g.obj(mine).zone, g.obj(theirs).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
    assert_eq!(count(&g, 0, |g, x| g.types_of(x).land), 25, "lands stay");

    let (mut g, t) = game_with(vec![sorcery(vec![], vec![Effect::DestroyAllLands])]);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    cast_test(&mut g, t, &[], &[]);
    assert_eq!(
        count(&g, 0, |g, x| g.types_of(x).land) + count(&g, 1, |g, x| g.types_of(x).land),
        0
    );
    assert_eq!(g.obj(ring).zone, Zone::Battlefield);
}

#[test]
fn mill_amount_mills_the_items_amount() {
    // Through Altar of Dementia's sacrifice: the amount is the victim's power.
    let (mut g, _) = game_with(vec![]);
    let altar = g.add_card(0, cid("Altar of Dementia"), Zone::Battlefield);
    let sphinx = g.add_card(0, cid("Consecrated Sphinx"), Zone::Battlefield); // power 4
    let lib = g.players[1].library.len();
    act(&mut g, Action::ActivateWith(altar, 0, sphinx));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(sphinx).zone, Zone::Graveyard);
    assert_eq!(g.players[1].library.len(), lib - 4);
}

#[test]
fn target_draws_x_loses_x() {
    let (mut g, t) = game_with(vec![{
        let mut d = sorcery(vec![Filter::PLAYER], vec![Effect::TargetDrawsXLosesX(0)]);
        d.cost = ManaCost::parse("{X}").unwrap();
        d
    }]);
    let c = g.add_card(0, t, Zone::Hand);
    let h = g.players[1].hand.len();
    act(&mut g, Action::CastX(c, 3));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].hand.len(), h + 3);
    assert_eq!(g.players[1].life, 37);
}

#[test]
fn exile_all_of_type_takes_only_that_type() {
    let (mut g, t) = game_with(vec![
        sorcery(vec![], vec![Effect::ExileAllOfType(0)]),
        sorcery(vec![], vec![Effect::ExileAllOfType(1)]),
    ]);
    let ring = g.add_card(1, cid("Sol Ring"), Zone::Battlefield);
    let looter = g.add_card(1, cid("Merfolk Looter"), Zone::Battlefield);
    cast_test(&mut g, t, &[], &[]);
    assert_eq!(
        (g.obj(ring).zone, g.obj(looter).zone),
        (Zone::Exile, Zone::Battlefield)
    );
    cast_test(&mut g, t + 1, &[], &[]);
    assert_eq!(g.obj(looter).zone, Zone::Exile);
}

#[test]
fn untap_nonland_you_control_leaves_lands_and_theirs() {
    let (mut g, t) = game_with(vec![
        sorcery(vec![], vec![Effect::UntapNonlandYouControl]),
        sorcery(vec![], vec![Effect::UntapNonlandYouControl]),
    ]);
    let bell = g.add_card(0, cid("Mikokoro, Center of the Sea"), Zone::Battlefield);
    let ring = g.add_card(0, cid("Temple Bell"), Zone::Battlefield);
    act(&mut g, Action::Activate(ring, 0)); // tap the Bell
    resolve_all(&mut g, &[]);
    act(&mut g, Action::Activate(bell, 0)); // tap a land ({2} paid by two more lands)
    resolve_all(&mut g, &[]);
    assert!(g.obj(ring).tapped && g.obj(bell).tapped);
    let tapped_lands = count(&g, 0, |g, x| g.types_of(x).land && g.obj(x).tapped);
    cast_test(&mut g, t, &[], &[]);
    assert!(!g.obj(ring).tapped, "a nonland permanent untaps");
    assert!(g.obj(bell).tapped, "a land doesn't");
    assert_eq!(
        count(&g, 0, |g, x| g.types_of(x).land && g.obj(x).tapped),
        tapped_lands
    );
}

#[test]
fn gain_life_per_card_in_hand_and_per_land() {
    let (mut g, t) = game_with(vec![
        sorcery(vec![], vec![Effect::GainLifePerCardInHand]),
        sorcery(vec![], vec![Effect::GainAndDrawPerLandYouControl]),
    ]);
    let c = g.add_card(0, t, Zone::Hand);
    let n = g.players[0].hand.len() as i32 - 1; // after it leaves the hand
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.players[0].life, 40 + n);
    let h = g.players[0].hand.len();
    cast_test(&mut g, t + 1, &[], &[]);
    assert_eq!(g.players[0].life, 40 + n + 25, "25 lands");
    assert_eq!(g.players[0].hand.len(), h + 25);
}

#[test]
fn exile_source_from_graveyard_or_stop_gates_the_rest() {
    // A creature whose dies trigger exiles it from the graveyard, then draws.
    let mut d = CardDef::creature("Test Dier", "{0}", 1, 1, Keywords::NONE);
    d.triggers.push(Trigger {
        when: When::Dies,
        ability: Ability::new(
            vec![],
            vec![
                Effect::ExileSourceFromGraveyardOrStop,
                Effect::Draw { who: Who::You, n: 1 },
            ],
        ),
    });
    let kill = sorcery(vec![Filter::CREATURE], vec![Effect::Destroy(0)]);
    let (mut g, t) = game_with(vec![d, kill]);
    let c = g.add_card(0, t, Zone::Battlefield);
    let h = g.players[0].hand.len();
    cast_test(&mut g, t + 1, &[obj(c)], &[]);
    assert_eq!(g.obj(c).zone, Zone::Exile);
    assert_eq!(g.players[0].hand.len(), h + 1, "exiled: the draw happens");
}

#[test]
fn active_player_is_whose_turn_it_is() {
    let mut d = CardDef::creature("Test Upkeep", "{0}", 1, 1, Keywords::NONE);
    d.triggers.push(Trigger {
        when: When::Upkeep {
            your_turn_only: false,
        },
        ability: Ability::new(
            vec![],
            vec![Effect::LoseLife {
                who: Who::ActivePlayer,
                n: 1,
            }],
        ),
    });
    let (mut g, t) = game_with(vec![d]);
    g.add_card(0, t, Zone::Battlefield);
    g.start_turn(4, 1);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!((g.players[0].life, g.players[1].life), (40, 39));
}

#[test]
fn new_searches_find_only_their_cards() {
    for (what, good, bad) in [
        (SearchFor::InstantOrSorcery, "Counterspell", "Sol Ring"),
        (SearchFor::Nonlegendary, "Sol Ring", "Razaketh, the Foulblooded"),
        (SearchFor::ArtifactOrEnchantment, "Rhystic Study", "Counterspell"),
        (SearchFor::Enchantment, "Rhystic Study", "Sol Ring"),
        (SearchFor::CreaturePowerAtMost(2), "Merfolk Looter", "Ashen Rider"),
    ] {
        let search = Effect::Search {
            what,
            to: SearchTo::Hand,
            n: 1,
            kicked_n: None,
        };
        let (mut g, t) = game_with(vec![sorcery(vec![], vec![search])]);
        let good = g.add_card(0, cid(good), Zone::Library);
        let bad = g.add_card(0, cid(bad), Zone::Library);
        let c = g.add_card(0, t, Zone::Hand);
        act(&mut g, Action::Cast(c));
        while !matches!(g.pending, Pending::Decide(_)) {
            act(&mut g, Action::PassPriority);
        }
        assert!(!g.legal_actions().contains(&Action::Pick(bad)), "{what:?}");
        resolve_all(&mut g, &[Action::Pick(good)]);
        assert_eq!(g.obj(good).zone, Zone::Hand, "{what:?}");
    }
}

fn static_of(scope: Scope, power: i32, toughness: i32, keywords: Keywords) -> Static {
    Static {
        scope,
        your_turn_only: false,
        condition: None,
        subtype: None,
        lose_abilities: false,
        base_pt: None,
        protection: [false; 5],
        scale: None,
        power,
        toughness,
        keywords,
        set_colors: None,
        becomes_land: false,
    }
}

#[test]
fn other_your_permanents_and_opponents_creatures_scopes() {
    let mut a = CardDef::creature("Test Avacyn", "{0}", 1, 1, Keywords::NONE);
    a.statics.push(static_of(
        Scope::OtherYourPermanents,
        0,
        0,
        Keywords::INDESTRUCTIBLE,
    ));
    let mut n = CardDef::creature("Test Norn", "{0}", 1, 1, Keywords::NONE);
    n.statics
        .push(static_of(Scope::OpponentsCreatures, -1, -1, Keywords::NONE));
    let (mut g, t) = game_with(vec![a, n]);
    let src = g.add_card(0, t, Zone::Battlefield);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    let land = g.permanents(0).find(|&x| g.types_of(x).land).unwrap();
    let theirs = g.add_card(1, cid("Sol Ring"), Zone::Battlefield);
    assert!(g.keywords(ring).has(Keywords::INDESTRUCTIBLE));
    assert!(g.keywords(land).has(Keywords::INDESTRUCTIBLE));
    assert!(!g.keywords(src).has(Keywords::INDESTRUCTIBLE), "other");
    assert!(!g.keywords(theirs).has(Keywords::INDESTRUCTIBLE));

    let norn = g.add_card(0, t + 1, Zone::Battlefield);
    let mine = g.add_card(0, cid("Thought Courier"), Zone::Battlefield);
    let opp = g.add_card(1, cid("Consecrated Sphinx"), Zone::Battlefield);
    assert_eq!((g.power(opp), g.toughness(opp)), (3, 5));
    assert_eq!((g.power(mine), g.toughness(mine)), (1, 1));
    assert_eq!(g.power(norn), 1);
}
