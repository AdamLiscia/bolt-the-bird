//! Lane d3's core theme, effects and statics: each new piece tested directly, on a test card built
//! here (appended to the CMD pool's definitions), not only through a printed card.
#![allow(dead_code)]

mod common;

use bolt_cards::set;
use bolt_core::effect::{
    Ability, Effect, Filter, Scale, Scope, SearchFor, SearchTo, Static, Trigger, When, Who,
};
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

// ---- Batch 2 ----

/// Casts test card `def` and passes until player `p` has a decision.
fn cast_until_decision(g: &mut Game, def: usize, x: Option<u8>) {
    let c = g.add_card(0, def, Zone::Hand);
    act(g, x.map_or(Action::Cast(c), |x| Action::CastX(c, x)));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(g, Action::PassPriority);
    }
}

#[test]
fn draw_per_colored_creature_counts_only_that_color() {
    let (mut g, t) = game_with(vec![sorcery(
        vec![],
        vec![Effect::DrawPerColoredCreatureYouControl(4)],
    )]);
    g.add_card(0, cid("Llanowar Elves"), Zone::Battlefield);
    g.add_card(0, cid("Birds of Paradise"), Zone::Battlefield);
    g.add_card(0, cid("Merfolk Looter"), Zone::Battlefield);
    g.add_card(1, cid("Elvish Mystic"), Zone::Battlefield);
    let h = g.players[0].hand.len();
    cast_test(&mut g, t, &[], &[]);
    assert_eq!(g.players[0].hand.len(), h + 2, "two green creatures of yours");
}

#[test]
fn regenerate_each_creature_you_control_saves_only_yours() {
    let (mut g, t) = game_with(vec![
        sorcery(vec![], vec![Effect::RegenerateEachCreatureYouControl]),
        sorcery(vec![], vec![Effect::DestroyAllCreatures]),
        sorcery(vec![], vec![Effect::DestroyAllCreaturesNoRegen]),
    ]);
    let mine = g.add_card(0, cid("Merfolk Looter"), Zone::Battlefield);
    let theirs = g.add_card(1, cid("Merfolk Looter"), Zone::Battlefield);
    cast_test(&mut g, t, &[], &[]);
    cast_test(&mut g, t + 1, &[], &[]);
    assert_eq!(g.obj(mine).zone, Zone::Battlefield);
    assert!(g.obj(mine).tapped, "regenerated: tapped");
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    cast_test(&mut g, t, &[], &[]);
    cast_test(&mut g, t + 2, &[], &[]);
    assert_eq!(g.obj(mine).zone, Zone::Graveyard, "can't be regenerated");
}

#[test]
fn regenerate_attached_shields_the_enchanted_creature() {
    let (mut g, t) = game_with(vec![sorcery(vec![], vec![Effect::DestroyAllCreatures])]);
    let looter = g.add_card(0, cid("Merfolk Looter"), Zone::Battlefield);
    let b = g.add_card(0, cid("Blessing of Leeches"), Zone::Hand);
    play(&mut g, b, false, &[obj(looter)], &[]);
    act(&mut g, Action::Activate(b, 0));
    resolve_all(&mut g, &[]);
    cast_test(&mut g, t, &[], &[]);
    assert_eq!(g.obj(looter).zone, Zone::Battlefield);
}

#[test]
fn lose_life_per_artifact_they_control() {
    let (mut g, t) = game_with(vec![sorcery(
        vec![],
        vec![Effect::LoseLifePerArtifactTheyControl(Who::EachOpponent)],
    )]);
    g.add_card(1, cid("Sol Ring"), Zone::Battlefield);
    g.add_card(1, cid("Solemn Simulacrum"), Zone::Battlefield);
    g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    cast_test(&mut g, t, &[], &[]);
    assert_eq!((g.players[0].life, g.players[1].life), (40, 38));
}

#[test]
fn non_human_draw_and_pump() {
    let (mut g, t) = game_with(vec![
        sorcery(vec![], vec![Effect::DrawGreatestPowerNonHuman]),
        sorcery(
            vec![],
            vec![Effect::PumpNonHumanYouControl {
                power: 3,
                toughness: 3,
            }],
        ),
        sorcery(vec![], vec![Effect::DrawGreatestToughnessYouControl]),
    ]);
    let human = g.add_card(0, cid("Eternal Witness"), Zone::Battlefield); // 2/1 Human
    let h = g.players[0].hand.len();
    cast_test(&mut g, t, &[], &[]);
    assert_eq!(g.players[0].hand.len(), h, "only a Human: draw 0");
    let looter = g.add_card(0, cid("Merfolk Looter"), Zone::Battlefield); // 1/1 Merfolk
    cast_test(&mut g, t, &[], &[]);
    assert_eq!(
        g.players[0].hand.len(),
        h + 1,
        "the Merfolk's power, not the Human's 2"
    );
    cast_test(&mut g, t + 1, &[], &[]);
    assert_eq!((g.power(looter), g.toughness(looter)), (4, 4));
    assert_eq!((g.power(human), g.toughness(human)), (2, 1));
    let h = g.players[0].hand.len();
    cast_test(&mut g, t + 2, &[], &[]);
    assert_eq!(
        g.players[0].hand.len(),
        h + 4,
        "the greatest toughness: the pumped 4/4"
    );
}

#[test]
fn creatures_from_hand_up_to_stops_at_its_number() {
    let (mut g, t) = game_with(vec![sorcery(vec![], vec![Effect::CreaturesFromHandUpTo(2)])]);
    let a = g.add_card(0, cid("Merfolk Looter"), Zone::Hand);
    let b = g.add_card(0, cid("Consecrated Sphinx"), Zone::Hand);
    let c = g.add_card(0, cid("Llanowar Elves"), Zone::Hand);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Hand);
    cast_until_decision(&mut g, t, None);
    assert!(
        !g.legal_actions().contains(&Action::Pick(ring)),
        "creature cards only"
    );
    resolve_all(&mut g, &[Action::Pick(a), Action::Pick(b)]);
    assert_eq!(
        (g.obj(a).zone, g.obj(b).zone, g.obj(c).zone),
        (Zone::Battlefield, Zone::Battlefield, Zone::Hand)
    );
    cast_test(&mut g, t, &[], &[Action::No]);
    assert_eq!(g.obj(c).zone, Zone::Hand, "up to: none is fine");
}

#[test]
fn may_shuffle_shuffles_only_on_yes() {
    let (mut g, t) = game_with(vec![sorcery(vec![], vec![Effect::MayShuffle])]);
    for name in ["Sol Ring", "Counterspell", "Merfolk Looter", "Llanowar Elves"] {
        g.add_card(0, cid(name), Zone::Library);
    }
    let before = g.players[0].library.clone();
    cast_test(&mut g, t, &[], &[Action::No]);
    assert_eq!(g.players[0].library, before);
    cast_test(&mut g, t, &[], &[Action::Yes]);
    assert_ne!(g.players[0].library, before, "103 cards: a shuffle moves them");
    let mut a = g.players[0].library.clone();
    let mut b = before;
    a.sort();
    b.sort();
    assert_eq!(a, b, "the same cards");
}

#[test]
fn return_permanent_you_control_with_and_without_lands() {
    let (mut g, t) = game_with(vec![
        sorcery(vec![], vec![Effect::ReturnPermanentYouControl { nonland: true }]),
        sorcery(vec![], vec![Effect::ReturnPermanentYouControl { nonland: false }]),
    ]);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    let theirs = g.add_card(1, cid("Sol Ring"), Zone::Battlefield);
    let land = g.permanents(0).find(|&x| g.types_of(x).land).unwrap();
    cast_until_decision(&mut g, t, None);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(ring)));
    assert!(!legal.contains(&Action::Pick(land)), "nonland");
    assert!(!legal.contains(&Action::Pick(theirs)), "yours");
    resolve_all(&mut g, &[Action::Pick(ring)]);
    assert_eq!(g.obj(ring).zone, Zone::Hand);
    cast_until_decision(&mut g, t + 1, None);
    assert!(g.legal_actions().contains(&Action::Pick(land)));
    resolve_all(&mut g, &[Action::Pick(land)]);
    assert_eq!(g.obj(land).zone, Zone::Hand);
}

#[test]
fn search_creatures_up_to_x_for_the_opponent() {
    let mut d = sorcery(vec![], vec![Effect::SearchCreaturesUpToX(Who::EachOpponent)]);
    d.cost = ManaCost::parse("{X}").unwrap();
    let (mut g, t) = game_with(vec![d]);
    let a = g.add_card(1, cid("Merfolk Looter"), Zone::Library);
    let b = g.add_card(1, cid("Llanowar Elves"), Zone::Library);
    let ring = g.add_card(1, cid("Sol Ring"), Zone::Library);
    cast_until_decision(&mut g, t, Some(2));
    assert_eq!(g.pending, Pending::Decide(1), "the opponent searches");
    assert!(!g.legal_actions().contains(&Action::Pick(ring)), "creatures only");
    resolve_all(&mut g, &[Action::Pick(a), Action::Pick(b)]);
    assert_eq!((g.obj(a).zone, g.obj(b).zone), (Zone::Hand, Zone::Hand));
    assert_eq!(g.obj(a).owner, 1);
}

#[test]
fn untap_creatures_attacked_this_turn_only_those() {
    let (mut g, t) = game_with(vec![sorcery(
        vec![],
        vec![Effect::UntapCreaturesAttackedThisTurn],
    )]);
    let attacked = g.add_card(0, cid("Merfolk Looter"), Zone::Battlefield);
    let other = g.add_card(0, cid("Merfolk Looter"), Zone::Battlefield);
    g.objects[attacked.0 as usize].tapped = true;
    g.objects[other.0 as usize].tapped = true;
    g.attacked_creatures_this_turn.push(attacked);
    cast_test(&mut g, t, &[], &[]);
    assert!(!g.obj(attacked).tapped);
    assert!(g.obj(other).tapped);
}

#[test]
fn noncreature_artifacts_become_creatures_and_artifact_creature_scope() {
    let (mut g, t) = game_with(vec![
        sorcery(
            vec![],
            vec![Effect::NoncreatureArtifactsBecomeCreatures {
                power: 4,
                toughness: 4,
            }],
        ),
        {
            let mut d = CardDef::creature("Test Awakener", "{0}", 1, 1, Keywords::NONE);
            d.statics.push(static_of(
                Scope::OtherYourArtifactCreatures,
                0,
                0,
                Keywords::FLYING,
            ));
            d
        },
    ]);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    let theirs = g.add_card(1, cid("Sol Ring"), Zone::Battlefield);
    let golem = g.add_card(0, cid("Solemn Simulacrum"), Zone::Battlefield);
    cast_test(&mut g, t, &[], &[]);
    assert!(g.types_of(ring).creature && g.types_of(ring).artifact);
    assert_eq!((g.power(ring), g.toughness(ring)), (4, 4));
    assert!(!g.types_of(theirs).creature, "yours only");
    assert_eq!(g.power(golem), 2, "already a creature");

    let aw = g.add_card(0, t + 1, Zone::Battlefield);
    let looter = g.add_card(0, cid("Merfolk Looter"), Zone::Battlefield);
    let their_golem = g.add_card(1, cid("Solemn Simulacrum"), Zone::Battlefield);
    assert!(g.keywords(golem).has(Keywords::FLYING));
    assert!(g.keywords(ring).has(Keywords::FLYING), "animated this turn");
    assert!(!g.keywords(looter).has(Keywords::FLYING), "not an artifact");
    assert!(!g.keywords(their_golem).has(Keywords::FLYING));
    assert!(!g.keywords(aw).has(Keywords::FLYING), "other");
}

#[test]
fn search_to_graveyard_counter_only_for_instants_and_sorceries() {
    // From a sorcery (not a permanent: no counter goes anywhere); Oriq Loremage's test has the counter.
    let (mut g, t) = game_with(vec![sorcery(
        vec![],
        vec![Effect::SearchToGraveyardCounterIfInstantOrSorcery],
    )]);
    let spell = g.add_card(0, cid("Counterspell"), Zone::Library);
    cast_until_decision(&mut g, t, None);
    assert!(g.legal_actions().contains(&Action::No));
    resolve_all(&mut g, &[Action::Pick(spell)]);
    assert_eq!(g.obj(spell).zone, Zone::Graveyard);
}

// ---- Batch 3: target filters and scaled statics ----

/// A free instant doing `effects` with `targets` (castable with a spell on the stack).
fn instant(targets: Vec<Filter>, effects: Vec<Effect>) -> CardDef {
    let mut d = sorcery(targets, effects);
    d.types = Types {
        instant: true,
        ..Types::default()
    };
    d
}

const SPELLS: Filter = Filter {
    players: false,
    creatures: false,
    planeswalkers: false,
    spells: true,
    ..Filter::ANY
};

#[test]
fn color_monocolored_and_nonartifact_filters() {
    let (mut g, _) = game_with(vec![]);
    let blue = g.add_card(0, cid("Merfolk Looter"), Zone::Battlefield);
    let green = g.add_card(1, cid("Llanowar Elves"), Zone::Battlefield);
    let multi = g.add_card(1, cid("Atraxa, Praetors' Voice"), Zone::Battlefield);
    let golem = g.add_card(1, cid("Solemn Simulacrum"), Zone::Battlefield);
    let ok = |g: &Game, f: Filter, x: ObjId| g.target_ok(&f, 0, Target::Object(x));
    let blue_f = Filter {
        color: Some(1),
        ..Filter::CREATURE
    };
    assert!(ok(&g, blue_f, blue) && !ok(&g, blue_f, green) && ok(&g, blue_f, multi));
    let nonblack = Filter {
        not_color: Some(2),
        ..Filter::CREATURE
    };
    assert!(ok(&g, nonblack, blue) && !ok(&g, nonblack, multi) && ok(&g, nonblack, golem));
    let mono = Filter {
        monocolored: true,
        ..Filter::CREATURE
    };
    assert!(ok(&g, mono, blue) && ok(&g, mono, green));
    assert!(!ok(&g, mono, multi), "four colors");
    assert!(!ok(&g, mono, golem), "colorless");
    let nonartifact = Filter {
        nonartifact: true,
        ..Filter::CREATURE
    };
    assert!(ok(&g, nonartifact, blue) && !ok(&g, nonartifact, golem));
    // Unset, the new fields change nothing.
    assert!(ok(&g, Filter::CREATURE, golem) && ok(&g, Filter::CREATURE, multi));
}

#[test]
fn spell_mana_value_and_type_filters() {
    let mut x_spell = instant(vec![], vec![]);
    x_spell.cost = ManaCost::parse("{X}").unwrap();
    let (mut g, t) = game_with(vec![x_spell]);
    let sphinx = g.add_card(0, cid("Consecrated Sphinx"), Zone::Hand);
    act(&mut g, Action::Cast(sphinx));
    let ok = |g: &Game, f: Filter, x: ObjId| g.target_ok(&f, 1, Target::Object(x));
    let four = Filter {
        spell_min_mana_value: Some(4),
        ..SPELLS
    };
    let seven = Filter {
        spell_min_mana_value: Some(7),
        ..SPELLS
    };
    let acp = Filter {
        spell_artifact_creature_or_planeswalker: true,
        ..SPELLS
    };
    assert!(ok(&g, four, sphinx) && !ok(&g, seven, sphinx) && ok(&g, acp, sphinx));
    let x = g.add_card(0, t, Zone::Hand);
    act(&mut g, Action::CastX(x, 5));
    assert!(ok(&g, four, x), "X counts on the stack: mana value 5");
    assert!(!ok(&g, seven, x));
    assert!(!ok(&g, acp, x), "an instant");
}

#[test]
fn targets_you_or_your_creature_filter() {
    let (mut g, t) = game_with(vec![
        instant(vec![Filter::CREATURE], vec![Effect::Destroy(0)]),
        instant(
            vec![Filter::PLAYER],
            vec![Effect::LoseLife {
                who: Who::TargetPlayer(0),
                n: 1,
            }],
        ),
    ]);
    let looter = g.add_card(0, cid("Merfolk Looter"), Zone::Battlefield);
    let f = Filter {
        abilities: true,
        targets_you_or_your_creature: true,
        ..SPELLS
    };
    let kill = g.add_card(0, t, Zone::Hand);
    act(&mut g, Action::Cast(kill));
    act(&mut g, Action::Target(obj(looter)));
    assert!(g.target_ok(&f, 0, obj(kill)), "it targets my creature");
    assert!(!g.target_ok(&f, 1, obj(kill)), "not the opponent's");
    let drain = g.add_card(0, t + 1, Zone::Hand);
    act(&mut g, Action::Cast(drain));
    act(&mut g, Action::Target(Target::Player(1)));
    assert!(g.target_ok(&f, 1, obj(drain)), "it targets that player");
    assert!(!g.target_ok(&f, 0, obj(drain)));
}

#[test]
fn destroy_and_counter_if_color() {
    let permanent = Filter {
        players: false,
        creatures: false,
        planeswalkers: false,
        any_permanent: true,
        ..Filter::ANY
    };
    let (mut g, t) = game_with(vec![sorcery(
        vec![permanent],
        vec![Effect::DestroyIfColor { slot: 0, color: 1 }],
    )]);
    let blue = g.add_card(1, cid("Merfolk Looter"), Zone::Battlefield);
    let green = g.add_card(1, cid("Llanowar Elves"), Zone::Battlefield);
    cast_test(&mut g, t, &[obj(green)], &[]);
    assert_eq!(g.obj(green).zone, Zone::Battlefield, "not blue: nothing");
    cast_test(&mut g, t, &[obj(blue)], &[]);
    assert_eq!(g.obj(blue).zone, Zone::Graveyard);
}

#[test]
fn pump_all_greatest_power_grant() {
    let (mut g, t) = game_with(vec![sorcery(
        vec![],
        vec![Effect::PumpAllGreatestPowerGrant(Keywords::TRAMPLE)],
    )]);
    let sphinx = g.add_card(0, cid("Consecrated Sphinx"), Zone::Battlefield);
    let looter = g.add_card(0, cid("Merfolk Looter"), Zone::Battlefield);
    let theirs = g.add_card(1, cid("Merfolk Looter"), Zone::Battlefield);
    cast_test(&mut g, t, &[], &[]);
    assert_eq!((g.power(looter), g.toughness(looter)), (5, 5));
    assert_eq!(g.power(sphinx), 8);
    assert!(g.keywords(looter).has(Keywords::TRAMPLE));
    assert_eq!(g.power(theirs), 1);
}

fn scaled_creature(name: &str, scale: Scale) -> CardDef {
    let mut d = CardDef::creature(name, "{0}", 0, 0, Keywords::NONE);
    let mut s = static_of(Scope::This, 1, 1, Keywords::NONE);
    s.scale = Some(scale);
    d.statics.push(s);
    d
}

#[test]
fn new_scales_count_their_things() {
    let (mut g, t) = game_with(vec![
        scaled_creature("Test Artifacts", Scale::PerArtifactYouControl),
        scaled_creature("Test Enchantments", Scale::PerOtherEnchantmentOnBattlefield),
        scaled_creature("Test Draws", Scale::PerCardDrawnThisTurn),
        scaled_creature("Test Fliers", Scale::PerOtherCreatureWithFlying),
        scaled_creature("Test Hands", Scale::PerCardInAllHands),
    ]);
    let a = g.add_card(0, t, Zone::Battlefield);
    g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    g.add_card(0, cid("Solemn Simulacrum"), Zone::Battlefield);
    g.add_card(1, cid("Sol Ring"), Zone::Battlefield);
    assert_eq!(g.power(a), 2, "your two artifacts");

    let e = g.add_card(0, t + 1, Zone::Battlefield);
    g.add_card(0, cid("Rhystic Study"), Zone::Battlefield);
    g.add_card(1, cid("Rhystic Study"), Zone::Battlefield);
    assert_eq!(g.power(e), 2, "anyone's enchantments");

    let d = g.add_card(0, t + 2, Zone::Battlefield);
    g.draws_this_turn[0] = 3;
    assert_eq!(g.toughness(d), 3);

    let f = g.add_card(0, t + 3, Zone::Battlefield);
    g.add_card(1, cid("Birds of Paradise"), Zone::Battlefield);
    g.add_card(0, cid("Consecrated Sphinx"), Zone::Battlefield);
    assert_eq!(g.power(f), 2, "both players' fliers");

    let h = g.add_card(0, t + 4, Zone::Battlefield);
    let n = g.players[0].hand.len() + g.players[1].hand.len();
    assert_eq!(g.power(h), n as i32);
}

// ---- Batch 4 ----

#[test]
fn minus_per_swamp_you_control_all_or_one() {
    let (mut g, t) = game_with(vec![
        sorcery(vec![], vec![Effect::AllCreaturesMinusPerSwampYouControl]),
        sorcery(vec![Filter::CREATURE], vec![Effect::MinusPerSwampYouControl(0)]),
    ]);
    let sphinx = g.add_card(0, cid("Consecrated Sphinx"), Zone::Battlefield); // 4/6
    let theirs = g.add_card(1, cid("Consecrated Sphinx"), Zone::Battlefield);
    cast_test(&mut g, t + 1, &[obj(theirs)], &[]);
    assert_eq!((g.power(theirs), g.toughness(theirs)), (-1, 1), "five Swamps");
    assert_eq!(g.power(sphinx), 4);
    let looter = g.add_card(1, cid("Merfolk Looter"), Zone::Battlefield);
    cast_test(&mut g, t, &[], &[]);
    assert_eq!(g.obj(looter).zone, Zone::Graveyard);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "-10/-10 in all");
    assert_eq!(g.toughness(sphinx), 1);
}

#[test]
fn destroy_all_artifacts_no_regen_gain_mana_value() {
    let (mut g, t) = game_with(vec![sorcery(
        vec![],
        vec![Effect::DestroyAllArtifactsNoRegenGainManaValue],
    )]);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Battlefield); // mv 1
    let golem = g.add_card(1, cid("Solemn Simulacrum"), Zone::Battlefield); // mv 4
    g.objects[golem.0 as usize].regen_shields = (g.turn, 1);
    cast_test(&mut g, t, &[], &[Action::No]); // Solemn's may-draw on death
    assert_eq!(
        (g.obj(ring).zone, g.obj(golem).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
    assert_eq!((g.players[0].life, g.players[1].life), (41, 44));
}

#[test]
fn grant_subtype_you_control_until_end_of_turn() {
    let (mut g, t) = game_with(vec![sorcery(
        vec![],
        vec![Effect::GrantSubtypeYouControlEot {
            sub: "Human",
            keywords: Keywords::HEXPROOF,
        }],
    )]);
    let human = g.add_card(0, cid("Eternal Witness"), Zone::Battlefield);
    let merfolk = g.add_card(0, cid("Merfolk Looter"), Zone::Battlefield);
    let theirs = g.add_card(1, cid("Thought Courier"), Zone::Battlefield);
    cast_test(&mut g, t, &[], &[]);
    assert!(g.keywords(human).has(Keywords::HEXPROOF));
    assert!(!g.keywords(merfolk).has(Keywords::HEXPROOF));
    assert!(!g.keywords(theirs).has(Keywords::HEXPROOF), "yours only");
}

#[test]
fn artifact_mana_value_search() {
    let search = Effect::Search {
        what: SearchFor::ArtifactManaValue(1),
        to: SearchTo::Hand,
        n: 1,
        kicked_n: None,
    };
    let (mut g, t) = game_with(vec![sorcery(vec![], vec![search])]);
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    let golem = g.add_card(0, cid("Solemn Simulacrum"), Zone::Library);
    let bolt = g.add_card(0, cid("Lightning Bolt"), Zone::Library); // mana value 1, not an artifact
    let c = g.add_card(0, t, Zone::Hand);
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(ring)));
    assert!(!legal.contains(&Action::Pick(golem)) && !legal.contains(&Action::Pick(bolt)));
    resolve_all(&mut g, &[Action::Pick(ring)]);
}

#[test]
fn enchantment_flying_and_tapped_scopes() {
    let mk = |name: &str, scope: Scope, p: i32, t: i32, k: Keywords| {
        let mut d = CardDef::creature(name, "{0}", 1, 1, Keywords::NONE);
        d.statics.push(static_of(scope, p, t, k));
        d
    };
    let (mut g, t) = game_with(vec![
        mk("Test Grove", Scope::OtherYourEnchantments, 0, 0, Keywords::SHROUD),
        mk("Test Fliers", Scope::CreaturesWithFlying, 2, 0, Keywords::NONE),
        mk(
            "Test Walkers",
            Scope::CreaturesWithoutFlying,
            -2,
            0,
            Keywords::NONE,
        ),
        mk("Test Tapped", Scope::YourTappedCreatures, 0, 3, Keywords::NONE),
        mk(
            "Test Untapped",
            Scope::YourUntappedCreatures,
            0,
            1,
            Keywords::NONE,
        ),
    ]);
    let study = g.add_card(0, cid("Rhystic Study"), Zone::Battlefield);
    let theirs = g.add_card(1, cid("Rhystic Study"), Zone::Battlefield);
    g.add_card(0, t, Zone::Battlefield);
    assert!(g.keywords(study).has(Keywords::SHROUD));
    assert!(!g.keywords(theirs).has(Keywords::SHROUD));

    let birds = g.add_card(1, cid("Birds of Paradise"), Zone::Battlefield); // 0/1 flier
    let looter = g.add_card(1, cid("Merfolk Looter"), Zone::Battlefield); // 1/1
    g.add_card(0, t + 1, Zone::Battlefield);
    g.add_card(0, t + 2, Zone::Battlefield);
    assert_eq!(g.power(birds), 2, "anyone's flier");
    assert_eq!(g.power(looter), -1);

    let mine = g.add_card(0, cid("Merfolk Looter"), Zone::Battlefield);
    g.add_card(0, t + 3, Zone::Battlefield);
    g.add_card(0, t + 4, Zone::Battlefield);
    assert_eq!(g.toughness(mine), 2, "untapped");
    g.objects[mine.0 as usize].tapped = true;
    assert_eq!(g.toughness(mine), 4, "tapped");
    assert_eq!(g.toughness(looter), 1, "theirs");
}

#[test]
fn attached_tapped_and_color_conditions() {
    use bolt_core::effect::StaticCondition;
    let mut aura = CardDef {
        name: "Test Aura".into(),
        types: Types {
            enchantment: true,
            ..Types::default()
        },
        ..CardDef::default()
    };
    let mut green = static_of(Scope::Attached, 1, 0, Keywords::NONE);
    green.condition = Some(StaticCondition::AttachedIsColor(4));
    let mut tapped = static_of(Scope::Attached, 0, 5, Keywords::NONE);
    tapped.condition = Some(StaticCondition::AttachedTapped(true));
    aura.statics.extend([green, tapped]);
    let (mut g, t) = game_with(vec![aura]);
    let elves = g.add_card(0, cid("Llanowar Elves"), Zone::Battlefield);
    let looter = g.add_card(0, cid("Merfolk Looter"), Zone::Battlefield);
    let a = g.add_card(0, t, Zone::Battlefield);
    let b = g.add_card(0, t, Zone::Battlefield);
    g.objects[a.0 as usize].attached_to = Some((elves, g.obj(elves).zcc));
    g.objects[b.0 as usize].attached_to = Some((looter, g.obj(looter).zcc));
    assert_eq!((g.power(elves), g.power(looter)), (2, 1), "green only");
    assert_eq!(g.toughness(elves), 1);
    g.objects[elves.0 as usize].tapped = true;
    assert_eq!(g.toughness(elves), 6, "tapped");
}

// ---- batch 5 ----

#[test]
fn stop_unless_condition_checks_a_spells_condition_as_it_resolves() {
    use bolt_core::effect::Condition;
    let mut s = sorcery(
        vec![],
        vec![Effect::StopUnlessCondition, Effect::Draw { who: Who::You, n: 1 }],
    );
    s.spell.as_mut().unwrap().condition = Some(Condition::OpponentControlsCreaturesAtLeast(2));
    let (mut g, t) = game_with(vec![s]);
    let before = g.players[0].hand.len();
    g.add_card(1, cid("Merfolk Looter"), Zone::Battlefield);
    cast_test(&mut g, t, &[], &[]);
    assert_eq!(g.players[0].hand.len(), before, "one creature: stops");
    g.add_card(1, cid("Llanowar Elves"), Zone::Battlefield);
    cast_test(&mut g, t, &[], &[]);
    assert_eq!(g.players[0].hand.len(), before + 1, "two: draws");
}

#[test]
fn sacrifice_source_sacrifices_the_permanent() {
    let mut def = sorcery(vec![], vec![]);
    def.types = Types {
        artifact: true,
        ..Types::default()
    };
    def.spell = None;
    def.activated.push(bolt_core::Activated {
        ability: Ability::new(vec![], vec![Effect::SacrificeSource]),
        ..bolt_core::Activated::default()
    });
    let (mut g, t) = game_with(vec![def]);
    let x = g.add_card(0, t, Zone::Battlefield);
    act(&mut g, Action::Activate(x, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(x).zone, Zone::Graveyard);
}

#[test]
fn single_target_filter_takes_only_spells_with_one_target() {
    let two = sorcery(vec![Filter::CREATURE, Filter::CREATURE], vec![]);
    let (mut g, t) = game_with(vec![two]);
    let a = g.add_card(1, cid("Merfolk Looter"), Zone::Battlefield);
    let b = g.add_card(1, cid("Llanowar Elves"), Zone::Battlefield);
    let bolt = g.add_card(0, cid("Lightning Bolt"), Zone::Hand);
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(obj(a)));
    let s = g.add_card(0, t, Zone::Hand);
    let f = Filter {
        players: false,
        creatures: false,
        planeswalkers: false,
        spells: true,
        single_target: true,
        ..Filter::ANY
    };
    assert_eq!(g.candidates(&f, 0), vec![obj(bolt)]);
    // A sorcery with two targets isn't one.
    resolve_all(&mut g, &[]);
    act(&mut g, Action::Cast(s));
    act(&mut g, Action::Target(obj(b)));
    let c = g.add_card(1, cid("Elvish Mystic"), Zone::Battlefield);
    act(&mut g, Action::Target(obj(c)));
    assert!(g.candidates(&f, 0).is_empty());
}

#[test]
fn artifact_with_a_mana_ability_or_basic_land_search() {
    let (mut g, t) = game_with(vec![sorcery(
        vec![],
        vec![Effect::Search {
            what: SearchFor::ArtifactWithManaAbilityOrBasicLand,
            to: SearchTo::Hand,
            n: 1,
            kicked_n: None,
        }],
    )]);
    let signet = g.add_card(0, cid("Selesnya Signet"), Zone::Library);
    let stone = g.add_card(0, cid("Fellwar Stone"), Zone::Library);
    let thopter = g.add_card(0, cid("Ornithopter"), Zone::Library);
    let tomb = g.add_card(0, cid("Overgrown Tomb"), Zone::Library);
    let c = g.add_card(0, t, Zone::Hand);
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(signet)) && legal.contains(&Action::Pick(stone)));
    assert!(!legal.contains(&Action::Pick(thopter)) && !legal.contains(&Action::Pick(tomb)));
    resolve_all(&mut g, &[Action::No]);
}

#[test]
fn destroy_then_fetch_a_land_with_a_basic_type_or_damage_the_controller() {
    let (mut g, t) = game_with(vec![
        sorcery(
            vec![Filter::CREATURE],
            vec![Effect::DestroyControllerMayFetchLandWithBasicType(0)],
        ),
        sorcery(
            vec![Filter::CREATURE],
            vec![Effect::DestroyDamageController { slot: 0, n: 2 }],
        ),
    ]);
    let a = g.add_card(1, cid("Merfolk Looter"), Zone::Battlefield);
    let grave = g.add_card(1, cid("Watery Grave"), Zone::Library);
    cast_test(&mut g, t, &[obj(a)], &[Action::Pick(grave), Action::No]);
    assert_eq!(g.obj(a).zone, Zone::Graveyard);
    assert_eq!(
        g.obj(grave).zone,
        Zone::Battlefield,
        "a nonbasic land with a basic type"
    );
    assert!(g.obj(grave).tapped, "declined the shock land's 2 life");
    let b = g.add_card(1, cid("Llanowar Elves"), Zone::Battlefield);
    cast_test(&mut g, t + 1, &[obj(b)], &[]);
    assert_eq!(g.obj(b).zone, Zone::Graveyard);
    assert_eq!(g.players[1].life, 38);
}

#[test]
fn change_the_target_must_change_and_change_a_target_to_the_source() {
    let mut instant = sorcery(
        vec![Filter {
            spells: true,
            ..Filter::ANY
        }],
        vec![Effect::ChangeTheTarget(0)],
    );
    instant.types = Types {
        instant: true,
        ..Types::default()
    };
    let skite = CardDef {
        name: "Test Skite".into(),
        types: Types {
            artifact: true,
            creature: true,
            ..Types::default()
        },
        power: 0,
        toughness: 4,
        activated: vec![bolt_core::Activated {
            ability: Ability::new(
                vec![Filter {
                    spells: true,
                    abilities: true,
                    ..Filter::ANY
                }],
                vec![Effect::ChangeATargetToSource(0)],
            ),
            ..bolt_core::Activated::default()
        }],
        ..CardDef::default()
    };
    let (mut g, t) = game_with(vec![instant, skite]);
    let looter = g.add_card(1, cid("Merfolk Looter"), Zone::Battlefield);
    let bolt = g.add_card(0, cid("Lightning Bolt"), Zone::Hand);
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    let c = g.add_card(0, t, Zone::Hand);
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(bolt)));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::No) && !legal.contains(&Action::Target(Target::Player(1))));
    assert!(legal.contains(&Action::Target(obj(looter))));
    resolve_all(&mut g, &[Action::Target(obj(looter))]);
    assert_eq!((g.obj(looter).zone, g.players[1].life), (Zone::Graveyard, 40));

    let s = g.add_card(0, t + 1, Zone::Battlefield);
    let bolt = g.add_card(0, cid("Lightning Bolt"), Zone::Hand);
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::Activate(s, 0));
    act(&mut g, Action::Target(obj(bolt)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(s).damage, g.players[1].life), (3, 40));
}

#[test]
fn keep_one_nonland_of_each_type_for_each_player() {
    let (mut g, t) = game_with(vec![sorcery(vec![], vec![Effect::EachKeepsOneNonlandOfEachType])]);
    let r1 = g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    let r2 = g.add_card(0, cid("Mind Stone"), Zone::Battlefield);
    let o1 = g.add_card(1, cid("Merfolk Looter"), Zone::Battlefield);
    let o2 = g.add_card(1, cid("Llanowar Elves"), Zone::Battlefield);
    let tomb = g.add_card(1, cid("Overgrown Tomb"), Zone::Battlefield);
    cast_test(&mut g, t, &[], &[Action::Pick(r2), Action::Pick(o2)]);
    assert_eq!(
        (g.obj(r1).zone, g.obj(r2).zone),
        (Zone::Graveyard, Zone::Battlefield)
    );
    assert_eq!(
        (g.obj(o1).zone, g.obj(o2).zone),
        (Zone::Graveyard, Zone::Battlefield)
    );
    assert_eq!(g.obj(tomb).zone, Zone::Battlefield, "lands stay");
}

#[test]
fn hand_to_bottom_then_draw_that_many_and_sacrifice_others_then_draw() {
    let (mut g, t) = game_with(vec![
        sorcery(vec![], vec![Effect::HandToBottomDrawThatMany(Who::You)]),
        sorcery(vec![], vec![Effect::SacrificeOthersDrawThatMany]),
    ]);
    let a = g.add_card(0, cid("Sol Ring"), Zone::Hand);
    let b = g.add_card(0, cid("Mind Stone"), Zone::Hand);
    let n = g.players[0].hand.len();
    // Order them: everything else, then Sol Ring, then Mind Stone last (the very bottom).
    let mut answers: Vec<Action> = g.players[0]
        .hand
        .iter()
        .filter(|&&c| c != a && c != b)
        .map(|&c| Action::Pick(c))
        .collect();
    answers.push(Action::Pick(a));
    cast_test(&mut g, t, &[], &answers);
    assert_eq!(g.players[0].hand.len(), n);
    assert_eq!(&g.players[0].library[..2], &[b, a]);

    let r = g.add_card(0, cid("Sol Ring"), Zone::Battlefield);
    let before = g.players[0].hand.len();
    cast_test(&mut g, t + 1, &[], &[Action::Pick(r), Action::No]);
    assert_eq!(g.obj(r).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), before + 1);
}

#[test]
fn each_sacrifices_a_creature_or_planeswalker_and_may_return_another() {
    let (mut g, t) = game_with(vec![sorcery(
        vec![],
        vec![Effect::EachSacrificeCreatureOrPlaneswalkerMayReturnOther],
    )]);
    let theirs = g.add_card(1, cid("Chandra, Flameshaper"), Zone::Battlefield);
    let card = g.add_card(0, cid("Sol Ring"), Zone::Graveyard);
    cast_test(&mut g, t, &[], &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "a planeswalker");
    assert_eq!(
        g.obj(card).zone,
        Zone::Graveyard,
        "you sacrificed nothing: no return"
    );
}

#[test]
fn wave_of_vitriol_and_signal_the_clans_pieces() {
    let (mut g, t) = game_with(vec![
        sorcery(vec![], vec![Effect::WaveOfVitriol]),
        sorcery(vec![], vec![Effect::SignalTheClans]),
    ]);
    let tomb = g.add_card(1, cid("Overgrown Tomb"), Zone::Battlefield);
    let basic = g.players[1].library[0];
    cast_test(&mut g, t, &[], &[Action::Pick(basic)]);
    assert_eq!(g.obj(tomb).zone, Zone::Graveyard);
    assert_eq!(
        g.obj(basic).zone,
        Zone::Battlefield,
        "only the opponent lost a land"
    );

    let a = g.add_card(0, cid("Merfolk Looter"), Zone::Library);
    let b = g.add_card(0, cid("Merfolk Looter"), Zone::Library);
    let c = g.add_card(0, cid("Llanowar Elves"), Zone::Library);
    cast_test(
        &mut g,
        t + 1,
        &[],
        &[Action::Pick(a), Action::Pick(b), Action::Pick(c)],
    );
    assert!(
        [a, b, c].iter().all(|&x| g.obj(x).zone == Zone::Library),
        "two share a name: none to hand"
    );
}

#[test]
fn braids_end_step_with_nothing_sharing_a_type() {
    let def = CardDef {
        name: "Test Braids".into(),
        types: Types {
            enchantment: true,
            ..Types::default()
        },
        triggers: vec![Trigger {
            when: When::EndStep { your_turn_only: true },
            ability: Ability::new(vec![], vec![Effect::BraidsEndStep]),
        }],
        ..CardDef::default()
    };
    let (mut g, t) = game_with(vec![def]);
    let x = g.add_card(0, t, Zone::Battlefield);
    let before = g.players[0].hand.len();
    for _ in 0..40 {
        if matches!(g.pending, Pending::Decide(_)) {
            break;
        }
        let legal = g.legal_actions();
        let a = if legal.contains(&Action::PassPriority) {
            Action::PassPriority
        } else {
            Action::FinishAttacks
        };
        act(&mut g, a);
    }
    act(&mut g, Action::Pick(x)); // an enchantment; they control none
    assert_eq!(g.players[1].life, 38);
    assert_eq!(g.players[0].hand.len(), before + 1);
}
