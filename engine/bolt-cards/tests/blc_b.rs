//! Bloomburrow Commander (BLC), lane B: each script's behavior in a 1v1 Commander game.

mod common;

use bolt_cards::set;
use bolt_core::{Action, Game, Keywords, ObjId, Pending, Step, Target, Zone};
use common::{act, obj, play, resolve_all};

fn bid(name: &str) -> usize {
    set("BLC")
        .unwrap()
        .index_of(name)
        .unwrap_or_else(|| panic!("{name} is not in BLC"))
}

/// A Commander game (99 Plains each) with these commanders, at player 0's first main phase with
/// `lands` for player 0 and 5 of each basic for player 1.
fn game(commanders: [&str; 2], lands: &[&str]) -> Game {
    let plains = vec![bid("Plains"); 99];
    let mut g = Game::new_commander(
        set("BLC").unwrap().defs.clone(),
        [&plains, &plains],
        [bid(commanders[0]), bid(commanders[1])],
        7,
        0,
    );
    g.auto_pass = false;
    for &l in lands {
        put(&mut g, 0, l);
    }
    for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        for _ in 0..5 {
            put(&mut g, 1, land);
        }
    }
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g
}

/// Every basic five times for player 0, Bello vs. Ms. Bumbleflower.
fn table() -> Game {
    let mut lands = Vec::new();
    for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        lands.extend([land; 5]);
    }
    game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &lands)
}

fn put(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, bid(name), Zone::Battlefield)
}

fn hand(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, bid(name), Zone::Hand)
}

fn colors_of(g: &Game, id: ObjId) -> Vec<[bool; 5]> {
    g.mana_sources(0)
        .into_iter()
        .filter(|&(x, _)| x == id)
        .map(|(_, m)| m.colors)
        .collect()
}

#[test]
fn arcane_signet_and_command_tower_make_your_commanders_colors() {
    let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[]);
    let signet = put(&mut g, 0, "Arcane Signet");
    let tower = put(&mut g, 0, "Command Tower");
    let rg = [false, false, false, true, true];
    assert_eq!(colors_of(&g, signet), [rg]);
    assert_eq!(colors_of(&g, tower), [rg]);

    // No commander: no mana.
    let mut g = Game::empty(set("BLC").unwrap().defs.clone(), 1, 0);
    let tower = g.add_card(0, bid("Command Tower"), Zone::Battlefield);
    assert!(colors_of(&g, tower).is_empty());
}

#[test]
fn exotic_orchard_and_fellwar_stone_copy_opponents_land_colors() {
    let plains = vec![bid("Plains"); 99];
    let mut g = Game::new_commander(
        set("BLC").unwrap().defs.clone(),
        [&plains, &plains],
        [bid("Bello, Bard of the Brambles"), bid("Ms. Bumbleflower")],
        7,
        0,
    );
    let orchard = put(&mut g, 0, "Exotic Orchard");
    let stone = put(&mut g, 0, "Fellwar Stone");
    assert!(colors_of(&g, orchard).is_empty(), "the opponent has no lands");
    put(&mut g, 1, "Island");
    put(&mut g, 1, "Swamp");
    put(&mut g, 0, "Forest"); // yours don't count
    let ub = [false, true, true, false, false];
    assert_eq!(colors_of(&g, orchard), [ub]);
    assert_eq!(colors_of(&g, stone), [ub]);
}

#[test]
fn sol_ring_makes_two_colorless() {
    let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[]);
    let ring = put(&mut g, 0, "Sol Ring");
    let sources: Vec<_> = g
        .mana_sources(0)
        .into_iter()
        .filter(|&(x, _)| x == ring)
        .collect();
    assert_eq!(sources.len(), 2);
    assert!(sources.iter().all(|(_, m)| m.colorless));
    let signet = hand(&mut g, 0, "Arcane Signet"); // {2}
    act(&mut g, Action::Cast(signet));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.obj(signet).zone,
        Zone::Battlefield,
        "paid with one Sol Ring tap"
    );
    assert!(g.obj(ring).tapped);
}

#[test]
fn terramorphic_expanse_fetches_a_basic_tapped() {
    let mut g = table();
    let t = put(&mut g, 0, "Terramorphic Expanse");
    let basic = *g.players[0].library.last().unwrap();
    act(&mut g, Action::Activate(t, 0));
    resolve_all(&mut g, &[Action::Pick(basic)]);
    assert_eq!(g.obj(t).zone, Zone::Graveyard);
    assert_eq!(g.obj(basic).zone, Zone::Battlefield);
    assert!(g.obj(basic).tapped);
}

#[test]
fn mind_stone_taps_for_colorless_and_cashes_in_for_a_card() {
    let mut g = table();
    let stone = put(&mut g, 0, "Mind Stone");
    assert!(g.mana_sources(0).iter().any(|&(x, m)| x == stone && m.colorless));
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(stone, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(stone).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n + 1);
}

#[test]
fn glacial_fortress_enters_tapped_unless_you_have_a_plains_or_island() {
    for (have, tapped) in [(&[][..], true), (&["Island"][..], false)] {
        let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], have);
        let f = hand(&mut g, 0, "Glacial Fortress");
        act(&mut g, Action::PlayLand(f));
        assert_eq!(g.obj(f).tapped, tapped);
        assert_eq!(
            g.def_of(f).mana.unwrap().colors,
            [true, true, false, false, false]
        );
    }
}

#[test]
fn reliquary_tower_and_thought_vessel_lift_the_hand_size_limit() {
    for name in ["Reliquary Tower", "Thought Vessel"] {
        let mut g = table();
        let x = put(&mut g, 0, name);
        assert!(g.def_of(x).no_max_hand_size);
        assert!(g.mana_sources(0).iter().any(|&(y, m)| y == x && m.colorless));
        for _ in 0..10 {
            hand(&mut g, 0, "Plains");
        }
        // To the cleanup step: no discarding down to seven.
        while g.step != Step::End {
            let a = match g.pending {
                Pending::DeclareAttackers(_) => Action::FinishAttacks,
                _ => Action::PassPriority,
            };
            act(&mut g, a);
        }
        let n = g.players[0].hand.len();
        while g.turn == 3 {
            assert!(!matches!(g.pending, Pending::Discard(_)), "{name}");
            act(&mut g, Action::PassPriority);
        }
        assert_eq!(g.players[0].hand.len(), n);
    }
}

#[test]
fn temple_of_enlightenment_enters_tapped_and_scries() {
    let mut g = table();
    let t = hand(&mut g, 0, "Temple of Enlightenment");
    let top = *g.players[0].library.last().unwrap();
    act(&mut g, Action::PlayLand(t));
    resolve_all(&mut g, &[Action::Yes]);
    assert!(g.obj(t).tapped);
    assert_eq!(g.players[0].library[0], top, "scried to the bottom");
}

#[test]
fn cultivate_puts_one_basic_onto_the_battlefield_and_one_in_hand() {
    let mut g = table();
    let c = hand(&mut g, 0, "Cultivate");
    let lib = g.players[0].library.clone();
    let (a, b) = (lib[lib.len() - 1], lib[lib.len() - 2]);
    play(&mut g, c, false, &[], &[Action::Pick(a), Action::Pick(b)]);
    assert_eq!(g.obj(a).zone, Zone::Battlefield);
    assert!(g.obj(a).tapped);
    assert_eq!(g.obj(b).zone, Zone::Hand);
}

#[test]
fn farseek_finds_a_plains_island_swamp_or_mountain() {
    let mut g = table();
    let c = hand(&mut g, 0, "Farseek");
    let forest = g.add_card(0, bid("Forest"), Zone::Library);
    let island = g.add_card(0, bid("Island"), Zone::Library);
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(!g.legal_actions().contains(&Action::Pick(forest)), "not a Forest");
    resolve_all(&mut g, &[Action::Pick(island)]);
    assert_eq!(g.obj(island).zone, Zone::Battlefield);
    assert!(g.obj(island).tapped);
}

#[test]
fn ms_bumbleflower_gives_cards_and_counters_and_draws_two_on_the_second() {
    let mut g = table();
    let flower = put(&mut g, 0, "Ms. Bumbleflower");
    assert!(g.keywords(flower).has(Keywords::VIGILANCE));
    let spells: Vec<ObjId> = (0..3).map(|_| hand(&mut g, 0, "Arcane Signet")).collect();
    let mut mine = g.players[0].hand.len();
    for (i, &s) in spells.iter().enumerate() {
        let theirs = g.players[1].hand.len();
        act(&mut g, Action::Cast(s));
        resolve_all(
            &mut g,
            &[Action::Target(Target::Player(1)), Action::Target(obj(flower))],
        );
        mine -= 1;
        if i == 1 {
            mine += 2; // the second resolution this turn
        }
        assert_eq!(g.players[1].hand.len(), theirs + 1, "target opponent draws");
        assert_eq!(g.players[0].hand.len(), mine, "spell {i}");
        assert_eq!(g.obj(flower).counters, i as i32 + 1);
        assert!(g.keywords(flower).has(Keywords::FLYING));
    }
}

#[test]
fn bello_animates_your_big_artifacts_and_enchantments_on_your_turn() {
    let mut g = table();
    put(&mut g, 0, "Bello, Bard of the Brambles");
    let stash = put(&mut g, 0, "Bootleggers' Stash"); // an artifact, mana value 6
    let onslaught = put(&mut g, 0, "Berserkers' Onslaught"); // an enchantment, 5
    let signet = put(&mut g, 0, "Arcane Signet"); // mana value 2
    let boots = put(&mut g, 0, "Swiftfoot Boots"); // Equipment
    for x in [stash, onslaught] {
        assert!(g.types_of(x).creature && g.types_of(x).artifact == g.def_of(x).types.artifact);
        assert_eq!((g.power(x), g.toughness(x)), (4, 4));
        let k = g.keywords(x);
        assert!(k.has(Keywords::INDESTRUCTIBLE) && k.has(Keywords::HASTE));
        assert!(g.has_subtype(x, "Elemental"));
    }
    assert!(!g.types_of(signet).creature, "mana value 2");
    assert!(!g.types_of(boots).creature, "Equipment");
    // It attacks (haste) and draws on combat damage.
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(stash));
    act(&mut g, Action::FinishAttacks);
    let n = g.players[0].hand.len();
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    // Berserkers' Onslaught (on the battlefield above) gives the attacking Stash double strike: two
    // 4-damage hits, and Bello's draw on each.
    assert_eq!(g.players[1].life, 32);
    assert_eq!(g.players[0].hand.len(), n + 2);
    // On the opponent's turn, just an artifact.
    g.start_turn(4, 1);
    assert!(!g.types_of(stash).creature);
}

// ---- Batch 2 ----

fn named(g: &Game, p: usize, name: &str) -> Vec<ObjId> {
    g.permanents(p).filter(|&x| g.def_of(x).name == name).collect()
}

#[test]
fn beast_within_destroys_and_gives_its_controller_a_beast() {
    let mut g = table();
    let theirs = put(&mut g, 1, "Sol Ring");
    let c = hand(&mut g, 0, "Beast Within");
    play(&mut g, c, false, &[obj(theirs)], &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    let beast = named(&g, 1, "Beast");
    assert_eq!(beast.len(), 1, "its controller's");
    assert_eq!((g.power(beast[0]), g.toughness(beast[0])), (3, 3));
}

#[test]
fn big_score_discards_draws_two_and_makes_treasures() {
    let mut g = table();
    let junk = hand(&mut g, 0, "Sol Ring");
    let c = hand(&mut g, 0, "Big Score");
    let n = g.players[0].hand.len();
    act(&mut g, Action::CastWith(c, junk));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(junk).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n - 2 + 2);
    assert_eq!(named(&g, 0, "Treasure").len(), 2);
}

#[test]
fn bastion_of_remembrance_makes_a_soldier_and_drains_on_deaths() {
    let mut g = table();
    let c = hand(&mut g, 0, "Bastion of Remembrance");
    play(&mut g, c, false, &[], &[]);
    let soldier = named(&g, 0, "Human Soldier");
    assert_eq!(soldier.len(), 1);
    assert!(g.has_subtype(soldier[0], "Human") && g.has_subtype(soldier[0], "Soldier"));
    g.move_card(soldier[0], Zone::Graveyard);
    act(&mut g, Action::PassPriority); // triggers go on the stack
    resolve_all(&mut g, &[]);
    assert_eq!((g.players[0].life, g.players[1].life), (41, 39));
}

#[test]
fn bojuka_bog_exiles_a_graveyard() {
    let mut g = table();
    let dead = g.add_card(1, bid("Sol Ring"), Zone::Graveyard);
    let bog = hand(&mut g, 0, "Bojuka Bog");
    act(&mut g, Action::PlayLand(bog));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert!(g.obj(bog).tapped);
    assert_eq!(g.obj(dead).zone, Zone::Exile);
}

#[test]
fn castle_ardenvale_checks_for_a_plains_and_makes_humans() {
    let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &["Island"]);
    let castle = hand(&mut g, 0, "Castle Ardenvale");
    act(&mut g, Action::PlayLand(castle));
    assert!(g.obj(castle).tapped, "no Plains");
    let mut g = table();
    let castle = put(&mut g, 0, "Castle Ardenvale");
    act(&mut g, Action::Activate(castle, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(named(&g, 0, "Human").len(), 1);
}

#[test]
fn battle_lands_and_fast_lands_count_your_lands() {
    // Canopy Vista: two basics or it enters tapped. It's a Forest Plains.
    let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &["Forest"]);
    let v = hand(&mut g, 0, "Canopy Vista");
    act(&mut g, Action::PlayLand(v));
    assert!(g.obj(v).tapped);
    assert!(g.has_subtype(v, "Forest") && g.has_subtype(v, "Plains"));
    let mut g = game(
        ["Bello, Bard of the Brambles", "Ms. Bumbleflower"],
        &["Forest", "Mountain"],
    );
    let c = hand(&mut g, 0, "Cinder Glade");
    act(&mut g, Action::PlayLand(c));
    assert!(!g.obj(c).tapped);
    assert_eq!(
        g.def_of(c).mana.unwrap().colors,
        [false, false, false, true, true]
    );
    // Seachrome Coast: two or fewer other lands, or it enters tapped.
    let mut g = game(
        ["Bello, Bard of the Brambles", "Ms. Bumbleflower"],
        &["Plains", "Island"],
    );
    let s = hand(&mut g, 0, "Seachrome Coast");
    act(&mut g, Action::PlayLand(s));
    assert!(!g.obj(s).tapped);
    let mut g = game(
        ["Bello, Bard of the Brambles", "Ms. Bumbleflower"],
        &["Plains", "Island", "Plains"],
    );
    let s = hand(&mut g, 0, "Seachrome Coast");
    act(&mut g, Action::PlayLand(s));
    assert!(g.obj(s).tapped);
}

#[test]
fn circuit_mender_gains_two_and_draws_when_it_leaves() {
    let mut g = table();
    let c = hand(&mut g, 0, "Circuit Mender");
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.players[0].life, 42);
    let n = g.players[0].hand.len();
    g.move_card(c, Zone::Exile);
    act(&mut g, Action::PassPriority); // triggers go on the stack
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n + 1);
}

#[test]
fn aether_channeler_picks_a_bird_a_bounce_or_a_card() {
    let mut g = table();
    let theirs = put(&mut g, 1, "Sol Ring");
    let c = hand(&mut g, 0, "Aether Channeler");
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    resolve_all(&mut g, &[Action::Mode(1), Action::Target(obj(theirs))]);
    assert_eq!(g.obj(theirs).zone, Zone::Hand);
    let c = hand(&mut g, 0, "Aether Channeler");
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    resolve_all(&mut g, &[Action::Mode(0)]);
    assert_eq!(named(&g, 0, "Bird").len(), 1);
}

#[test]
fn blade_splicer_makes_a_golem_with_first_strike() {
    let mut g = table();
    let c = hand(&mut g, 0, "Blade Splicer");
    play(&mut g, c, false, &[], &[]);
    let golem = named(&g, 0, "Phyrexian Golem");
    assert_eq!(golem.len(), 1);
    assert!(g.types_of(golem[0]).artifact && g.types_of(golem[0]).creature);
    assert_eq!(g.power(golem[0]), 3);
    assert!(g.keywords(golem[0]).has(Keywords::FIRST_STRIKE));
    assert!(
        !g.keywords(c).has(Keywords::FIRST_STRIKE),
        "the Splicer isn't a Golem"
    );
}

#[test]
fn chart_a_course_discards_unless_you_attacked() {
    let mut g = table();
    let c = hand(&mut g, 0, "Chart a Course");
    let junk = hand(&mut g, 0, "Sol Ring");
    let n = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[Action::Discard(junk)]);
    assert_eq!(g.players[0].hand.len(), n - 1 + 2 - 1);
    assert_eq!(g.obj(junk).zone, Zone::Graveyard);
}

#[test]
fn blasphemous_act_costs_less_per_creature_and_kills_everything() {
    let mut g = table();
    for _ in 0..3 {
        put(&mut g, 0, "Circuit Mender");
        put(&mut g, 1, "Circuit Mender");
    }
    let c = hand(&mut g, 0, "Blasphemous Act");
    let before = g
        .permanents(0)
        .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
        .count();
    act(&mut g, Action::Cast(c));
    resolve_all(&mut g, &[]);
    let after = g
        .permanents(0)
        .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
        .count();
    assert_eq!(before - after, 3, "{{8}}{{R}} minus six creatures");
    assert!(g
        .permanents(0)
        .chain(g.permanents(1))
        .all(|x| !g.types_of(x).creature));
}

// ---- Batch 3: Zinnia, Hazel, offspring ----

fn tokens_named(g: &Game, p: usize, name: &str) -> Vec<ObjId> {
    g.permanents(p)
        .filter(|&x| g.is_token(x) && g.def_of(x).name == name)
        .collect()
}

#[test]
fn zinnia_grants_offspring_to_your_creature_spells() {
    let mut g = table();
    let zinnia = put(&mut g, 0, "Zinnia, Valley's Voice");
    assert!(g.keywords(zinnia).has(Keywords::FLYING));
    assert_eq!(g.power(zinnia), 1);
    let mender = hand(&mut g, 0, "Circuit Mender"); // {3}
    assert!(
        g.legal_actions().contains(&Action::CastKicked(mender)),
        "offspring {{2}}"
    );
    let before = g
        .permanents(0)
        .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
        .count();
    act(&mut g, Action::CastKicked(mender));
    resolve_all(&mut g, &[]);
    let after = g
        .permanents(0)
        .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
        .count();
    assert_eq!(before - after, 5);
    let copy = tokens_named(&g, 0, "Circuit Mender");
    assert_eq!(copy.len(), 1, "a token copy");
    assert_eq!((g.power(copy[0]), g.toughness(copy[0])), (1, 1));
    assert_eq!(g.players[0].life, 44, "both entered: gain 2 twice");
    assert_eq!(g.power(zinnia), 2, "+1/+0 for the 1/1 copy");
    assert_eq!(g.power(mender), 2);

    // Without Zinnia: no offspring.
    let mut g = table();
    let mender = hand(&mut g, 0, "Circuit Mender");
    assert!(!g.legal_actions().contains(&Action::CastKicked(mender)));
}

#[test]
fn agate_instigator_has_offspring_and_pings_for_each_creature() {
    let mut g = table();
    let a = hand(&mut g, 0, "Agate Instigator");
    act(&mut g, Action::CastKicked(a));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens_named(&g, 0, "Agate Instigator").len(), 1);
    assert_eq!(g.players[1].life, 39, "the copy entering");
    let a2 = hand(&mut g, 0, "Agate Instigator");
    play(&mut g, a2, false, &[], &[]);
    assert_eq!(g.players[1].life, 37, "two Instigators see it enter");
}

#[test]
fn hazel_taps_tokens_for_mana_and_copies_tokens_at_end_of_turn() {
    let mut g = table();
    let hazel = put(&mut g, 0, "Hazel of the Rootbloom");
    let score = hand(&mut g, 0, "Big Score");
    let junk = hand(&mut g, 0, "Sol Ring");
    act(&mut g, Action::CastWith(score, junk));
    resolve_all(&mut g, &[]);
    let treasures = tokens_named(&g, 0, "Treasure");
    assert_eq!(treasures.len(), 2);
    assert!(g.legal_actions().contains(&Action::ActivateX {
        id: hazel,
        n: 0,
        victim: None,
        x: 2
    }));
    assert!(!g.legal_actions().contains(&Action::ActivateX {
        id: hazel,
        n: 0,
        victim: None,
        x: 3
    }));
    act(
        &mut g,
        Action::ActivateX {
            id: hazel,
            n: 0,
            victim: None,
            x: 2,
        },
    );
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 38, "pay 2 life");
    assert!(g.obj(hazel).tapped && treasures.iter().all(|&t| g.obj(t).tapped));
    assert_eq!(g.players[0].pool.len(), 2);
    assert!(g.players[0].pool.iter().all(|m| m.colors == [true; 5]));
    // End step: a copy of a token.
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    act(&mut g, Action::Target(obj(treasures[0])));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens_named(&g, 0, "Treasure").len(), 3);
}

#[test]
fn hazel_copies_a_squirrel_token_twice() {
    let mut g = table();
    put(&mut g, 0, "Zinnia, Valley's Voice");
    put(&mut g, 0, "Hazel of the Rootbloom");
    // A Squirrel token: Zinnia's offspring copy of a Squirrel creature card (its own rules text isn't
    // scripted, but it casts and enters).
    let squirrel_card = hand(&mut g, 0, "Bloodroot Apothecary");
    act(&mut g, Action::CastKicked(squirrel_card));
    let mut guard = 0;
    while (!g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)))
        && guard < 20
    {
        let a = g
            .legal_actions()
            .iter()
            .copied()
            .find(|a| matches!(a, Action::PassPriority))
            .unwrap_or(g.legal_actions()[0]);
        act(&mut g, a);
        guard += 1;
    }
    let squirrel = tokens_named(&g, 0, "Bloodroot Apothecary");
    assert_eq!(squirrel.len(), 1);
    assert!(g.has_subtype(squirrel[0], "Squirrel"));
    let mut asked = 0;
    while g.turn == 3 {
        let a = match g.pending {
            Pending::ChooseTarget(_) => {
                asked += 1;
                Action::Target(obj(squirrel[0]))
            }
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::Discard(_) => g.legal_actions()[0], // cleanup: down to seven
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(asked, 1);
    assert_eq!(
        tokens_named(&g, 0, "Bloodroot Apothecary").len(),
        3,
        "two more copies"
    );
}

// ---- Batch 4 ----

#[test]
fn checklands_fastlands_and_typed_lands() {
    for (name, needs, colors) in [
        ("Clifftop Retreat", "Mountain", [true, false, false, true, false]),
        ("Hinterland Harbor", "Island", [false, true, false, false, true]),
    ] {
        let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[]);
        let l = hand(&mut g, 0, name);
        act(&mut g, Action::PlayLand(l));
        assert!(g.obj(l).tapped, "{name}");
        assert_eq!(g.def_of(l).mana.unwrap().colors, colors);
        let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[needs]);
        let l = hand(&mut g, 0, name);
        act(&mut g, Action::PlayLand(l));
        assert!(!g.obj(l).tapped, "{name} with a {needs}");
    }
    let mut g = game(
        ["Bello, Bard of the Brambles", "Ms. Bumbleflower"],
        &["Forest", "Forest", "Forest"],
    );
    let l = hand(&mut g, 0, "Copperline Gorge");
    act(&mut g, Action::PlayLand(l));
    assert!(g.obj(l).tapped, "three other lands");
    let mut g = game(
        ["Bello, Bard of the Brambles", "Ms. Bumbleflower"],
        &["Plains", "Island"],
    );
    let l = hand(&mut g, 0, "Prairie Stream");
    act(&mut g, Action::PlayLand(l));
    assert!(!g.obj(l).tapped && g.has_subtype(l, "Island"));
    let mut g = table();
    let l = hand(&mut g, 0, "Haunted Mire");
    act(&mut g, Action::PlayLand(l));
    assert!(g.obj(l).tapped && g.has_subtype(l, "Swamp") && g.has_subtype(l, "Forest"));
    assert_eq!(
        g.def_of(l).mana.unwrap().colors,
        [false, false, true, false, true]
    );
}

#[test]
fn cloudblazer_inspiring_overseer_and_harmonize_draw() {
    let mut g = table();
    let c = hand(&mut g, 0, "Cloudblazer");
    let n = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[]);
    assert_eq!((g.players[0].life, g.players[0].hand.len()), (42, n - 1 + 2));
    assert!(g.keywords(c).has(Keywords::FLYING));
    let o = hand(&mut g, 0, "Inspiring Overseer");
    let n = g.players[0].hand.len();
    play(&mut g, o, false, &[], &[]);
    assert_eq!((g.players[0].life, g.players[0].hand.len()), (43, n));
    let h = hand(&mut g, 0, "Harmonize");
    let n = g.players[0].hand.len();
    play(&mut g, h, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), n - 1 + 3);
}

#[test]
fn generous_gift_putrefy_and_path_to_exile_remove_things() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let c = hand(&mut g, 0, "Generous Gift");
    play(&mut g, c, false, &[obj(ring)], &[]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    let e = named(&g, 1, "Elephant");
    assert_eq!((e.len(), g.power(e[0])), (1, 3));

    let mut g = table();
    let mender = put(&mut g, 1, "Circuit Mender");
    let c = hand(&mut g, 0, "Putrefy");
    play(&mut g, c, false, &[obj(mender)], &[]);
    assert_eq!(g.obj(mender).zone, Zone::Graveyard);

    let mut g = table();
    let mender = put(&mut g, 1, "Circuit Mender");
    let c = hand(&mut g, 0, "Path to Exile");
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(mender)));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.pending, Pending::Decide(1), "its controller searches");
    let basic = *g.players[1].library.last().unwrap();
    resolve_all(&mut g, &[Action::Pick(basic)]);
    assert_eq!(g.obj(mender).zone, Zone::Exile);
    assert_eq!(g.obj(basic).zone, Zone::Battlefield);
    assert!(g.obj(basic).tapped);
}

#[test]
fn hedron_archive_makes_two_and_cashes_in_for_two_cards() {
    let mut g = table();
    let a = put(&mut g, 0, "Hedron Archive");
    assert_eq!(g.mana_sources(0).iter().filter(|&&(x, _)| x == a).count(), 2);
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(a, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n + 2);
}

#[test]
fn kalonian_hydra_enters_with_four_and_doubles_on_attack() {
    let mut g = table();
    let h = hand(&mut g, 0, "Kalonian Hydra");
    let mender = put(&mut g, 0, "Circuit Mender");
    play(&mut g, h, false, &[], &[]);
    assert_eq!(g.obj(h).counters, 4);
    assert!(g.keywords(h).has(Keywords::TRAMPLE));
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(h));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(h).counters, 8);
    assert_eq!(g.obj(mender).counters, 0, "none to double");
}

#[test]
fn managorger_hydra_grows_whenever_anyone_casts() {
    let mut g = table();
    let h = put(&mut g, 0, "Managorger Hydra");
    let mine = hand(&mut g, 0, "Sol Ring");
    play(&mut g, mine, false, &[], &[]);
    assert_eq!(g.obj(h).counters, 1);
    // The opponent casts one on their turn.
    let theirs = hand(&mut g, 1, "Sol Ring");
    g.start_turn(4, 1);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Cast(theirs));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(h).counters, 2);
}

#[test]
fn moldervine_innkeeper_and_baloths() {
    let mut g = table();
    put(&mut g, 0, "Moldervine Reclamation");
    let mender = put(&mut g, 0, "Circuit Mender");
    let n = g.players[0].hand.len();
    g.move_card(mender, Zone::Graveyard);
    act(&mut g, Action::PassPriority);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 41);
    assert_eq!(
        g.players[0].hand.len(),
        n + 2,
        "Moldervine's card and the Mender's own"
    );

    let mut g = table();
    let inn = hand(&mut g, 0, "Prosperous Innkeeper");
    play(&mut g, inn, false, &[], &[]);
    assert_eq!(named(&g, 0, "Treasure").len(), 1);
    let m = hand(&mut g, 0, "Circuit Mender");
    play(&mut g, m, false, &[], &[]);
    assert_eq!(g.players[0].life, 40 + 1 + 2);

    let mut g = table();
    put(&mut g, 0, "Rampaging Baloths");
    let l = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(l));
    resolve_all(&mut g, &[]);
    let b = named(&g, 0, "Beast");
    assert_eq!((b.len(), g.power(b[0])), (1, 4));
}

#[test]
fn loran_destroys_an_artifact_and_draws_for_both() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let loran = hand(&mut g, 0, "Loran of the Third Path");
    play(&mut g, loran, false, &[], &[Action::Target(obj(ring))]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let (a, b) = (g.players[0].hand.len(), g.players[1].hand.len());
    act(&mut g, Action::Activate(loran, 0));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert_eq!((g.players[0].hand.len(), g.players[1].hand.len()), (a + 1, b + 1));
}

// ---- Batch 5 ----

#[test]
fn cycling_lands_enter_tapped_and_cycle_from_hand() {
    for (name, color) in [("Tranquil Thicket", 4), ("Barren Moor", 2), ("Forgotten Cave", 3)] {
        let mut g = table();
        let l = hand(&mut g, 0, name);
        let other = hand(&mut g, 0, name);
        act(&mut g, Action::PlayLand(l));
        assert!(g.obj(l).tapped);
        assert!(g.def_of(l).mana.unwrap().colors[color]);
        assert!(
            !g.legal_actions().contains(&Action::Activate(l, 0)),
            "no cycling on the battlefield"
        );
        let n = g.players[0].hand.len();
        act(&mut g, Action::Activate(other, 0));
        resolve_all(&mut g, &[]);
        assert_eq!(g.obj(other).zone, Zone::Graveyard, "{name} discarded");
        assert_eq!(g.players[0].hand.len(), n, "and a card drawn");
    }
}

#[test]
fn explore_gives_an_extra_land_drop_and_a_card() {
    let mut g = table();
    let a = hand(&mut g, 0, "Forest");
    let b = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(a));
    assert!(!g.legal_actions().contains(&Action::PlayLand(b)));
    let e = hand(&mut g, 0, "Explore");
    play(&mut g, e, false, &[], &[]);
    assert!(g.legal_actions().contains(&Action::PlayLand(b)));
}

#[test]
fn coiling_oracle_puts_a_land_down_or_takes_the_card() {
    let mut g = table();
    let c = hand(&mut g, 0, "Coiling Oracle");
    let top = *g.players[0].library.last().unwrap(); // a Plains
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(top).zone, Zone::Battlefield);
    let c = hand(&mut g, 0, "Coiling Oracle");
    let spell = g.add_card(0, bid("Harmonize"), Zone::Library);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(spell).zone, Zone::Hand);
}

#[test]
fn maelstrom_pulse_takes_every_copy() {
    let mut g = table();
    let a = put(&mut g, 1, "Sol Ring");
    let b = put(&mut g, 1, "Sol Ring");
    let mine = put(&mut g, 0, "Sol Ring");
    let other = put(&mut g, 1, "Mind Stone");
    let c = hand(&mut g, 0, "Maelstrom Pulse");
    play(&mut g, c, false, &[obj(a)], &[]);
    for x in [a, b, mine] {
        assert_eq!(g.obj(x).zone, Zone::Graveyard);
    }
    assert_eq!(g.obj(other).zone, Zone::Battlefield);
}

#[test]
fn plumecreed_escort_and_ornithopter() {
    let mut g = table();
    let mender = put(&mut g, 0, "Circuit Mender");
    let p = hand(&mut g, 0, "Plumecreed Escort");
    assert!(g.def_of(p).keywords.has(Keywords::FLASH));
    play(&mut g, p, false, &[], &[Action::Target(obj(mender))]);
    assert!(g.keywords(mender).has(Keywords::HEXPROOF));
    let o = put(&mut g, 0, "Ornithopter of Paradise");
    assert!(g.keywords(o).has(Keywords::FLYING));
    assert!(g
        .mana_sources(0)
        .iter()
        .any(|&(x, m)| x == o && m.colors == [true; 5]));
}

#[test]
fn haywire_mite_gains_on_death_and_exiles_noncreature_artifacts() {
    let mut g = table();
    let mite = put(&mut g, 0, "Haywire Mite");
    let ring = put(&mut g, 1, "Sol Ring");
    let thopter = put(&mut g, 1, "Ornithopter of Paradise");
    act(&mut g, Action::Activate(mite, 0));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(thopter))),
        "noncreature only"
    );
    act(&mut g, Action::Target(obj(ring)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ring).zone, Zone::Exile);
    assert_eq!(g.obj(mite).zone, Zone::Graveyard);
    assert_eq!(g.players[0].life, 42, "it died");
}

#[test]
fn cut_a_deal_and_pull_from_tomorrow_draw() {
    let mut g = table();
    let c = hand(&mut g, 0, "Cut a Deal");
    let (a, b) = (g.players[0].hand.len(), g.players[1].hand.len());
    play(&mut g, c, false, &[], &[]);
    assert_eq!((g.players[0].hand.len(), g.players[1].hand.len()), (a, b + 1));
    let p = hand(&mut g, 0, "Pull from Tomorrow");
    let junk = hand(&mut g, 0, "Sol Ring");
    let n = g.players[0].hand.len();
    act(&mut g, Action::CastX(p, 3));
    resolve_all(&mut g, &[Action::Discard(junk)]);
    assert_eq!(g.players[0].hand.len(), n - 1 + 3 - 1);
}

#[test]
fn hanged_executioner_makes_a_spirit_and_exiles_itself_to_exile_a_creature() {
    let mut g = table();
    let h = hand(&mut g, 0, "Hanged Executioner");
    play(&mut g, h, false, &[], &[]);
    let spirit = named(&g, 0, "Spirit");
    assert_eq!(spirit.len(), 1);
    assert!(g.keywords(spirit[0]).has(Keywords::FLYING));
    let theirs = put(&mut g, 1, "Circuit Mender");
    act(&mut g, Action::Activate(h, 0));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(h).zone, Zone::Exile);
    assert_eq!(g.obj(theirs).zone, Zone::Exile);
}

#[test]
fn inferno_titan_divides_three_damage_and_firebreathes() {
    let mut g = table();
    let a = put(&mut g, 1, "Agate Instigator"); // 1/3
    let t = hand(&mut g, 0, "Inferno Titan");
    act(&mut g, Action::Cast(t));
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Target(obj(a)));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::No);
    act(&mut g, Action::Amount(2));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(a).damage, 2);
    assert_eq!(g.players[1].life, 39);
    let p = g.power(t);
    act(&mut g, Action::Activate(t, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(t), p + 1);
}

#[test]
fn lotus_cobra_adds_mana_on_landfall() {
    let mut g = table();
    put(&mut g, 0, "Lotus Cobra");
    let l = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(l));
    assert!(!g.items.is_empty(), "a trigger, on the stack");
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].pool.len(), 1);
    assert_eq!(g.players[0].pool[0].colors, [true; 5]);
}

// ---- Batch 6 ----

fn play_land(g: &mut Game, name: &str) -> ObjId {
    let l = hand(g, 0, name);
    act(g, Action::PlayLand(l));
    l
}

#[test]
fn checklands_temples_and_other_taplands() {
    for (name, needs, color) in [
        ("Rootbound Crag", "Forest", 3),
        ("Sulfur Falls", "Island", 1),
        ("Sunpetal Grove", "Plains", 0),
        ("Woodland Cemetery", "Swamp", 2),
    ] {
        let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[]);
        let l = play_land(&mut g, name);
        assert!(g.obj(l).tapped, "{name}");
        assert!(g.def_of(l).mana.unwrap().colors[color]);
        let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[needs]);
        let l = play_land(&mut g, name);
        assert!(!g.obj(l).tapped, "{name} with a {needs}");
    }
    let mut g = game(
        ["Bello, Bard of the Brambles", "Ms. Bumbleflower"],
        &["Forest", "Forest"],
    );
    let l = play_land(&mut g, "Razorverge Thicket");
    assert!(!g.obj(l).tapped, "two other lands");
    for name in [
        "Temple of Abandon",
        "Temple of Epiphany",
        "Temple of Malady",
        "Temple of Mystery",
        "Temple of Plenty",
        "Temple of Triumph",
    ] {
        let mut g = table();
        let l = hand(&mut g, 0, name);
        act(&mut g, Action::PlayLand(l));
        resolve_all(&mut g, &[Action::No]);
        assert!(g.obj(l).tapped, "{name}");
    }
    let mut g = table();
    let l = play_land(&mut g, "Seaside Citadel");
    assert!(g.obj(l).tapped);
    assert_eq!(g.def_of(l).mana.unwrap().colors, [true, true, false, false, true]);
    let mut g = table();
    let l = play_land(&mut g, "Wooded Ridgeline");
    assert!(g.obj(l).tapped && g.has_subtype(l, "Mountain"));
    let mut g = table();
    let l = hand(&mut g, 0, "Sheltered Thicket");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(l, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.obj(l).zone, g.players[0].hand.len()),
        (Zone::Graveyard, n),
        "cycled"
    );
}

#[test]
fn ramp_rampant_growth_sakura_tribe_elder_thran_dynamo() {
    let mut g = table();
    let c = hand(&mut g, 0, "Rampant Growth");
    let top = *g.players[0].library.last().unwrap();
    play(&mut g, c, false, &[], &[Action::Pick(top)]);
    assert_eq!(g.obj(top).zone, Zone::Battlefield);
    assert!(g.obj(top).tapped);
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    let top = *g.players[0].library.last().unwrap();
    act(&mut g, Action::Activate(elder, 0));
    resolve_all(&mut g, &[Action::Pick(top)]);
    assert_eq!(g.obj(elder).zone, Zone::Graveyard);
    assert_eq!(g.obj(top).zone, Zone::Battlefield);
    let dynamo = put(&mut g, 0, "Thran Dynamo");
    assert_eq!(g.mana_sources(0).iter().filter(|&&(x, _)| x == dynamo).count(), 3);
}

#[test]
fn card_draw_secret_rendezvous_spirited_companion_sphinx() {
    let mut g = table();
    let c = hand(&mut g, 0, "Secret Rendezvous");
    let (a, b) = (g.players[0].hand.len(), g.players[1].hand.len());
    play(&mut g, c, false, &[Target::Player(1)], &[]);
    assert_eq!(
        (g.players[0].hand.len(), g.players[1].hand.len()),
        (a - 1 + 3, b + 3)
    );
    let c = hand(&mut g, 0, "Spirited Companion");
    let n = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), n);
    let c = hand(&mut g, 0, "Sphinx of Enlightenment");
    let (a, b) = (g.players[0].hand.len(), g.players[1].hand.len());
    play(&mut g, c, false, &[], &[Action::Target(Target::Player(1))]);
    assert_eq!(
        (g.players[0].hand.len(), g.players[1].hand.len()),
        (a - 1 + 3, b + 1)
    );
}

#[test]
fn swords_to_plowshares_and_rapid_hybridization() {
    let mut g = table();
    let titan = put(&mut g, 1, "Inferno Titan"); // power 6
    let c = hand(&mut g, 0, "Swords to Plowshares");
    play(&mut g, c, false, &[obj(titan)], &[]);
    assert_eq!(g.obj(titan).zone, Zone::Exile);
    assert_eq!(g.players[1].life, 46);
    let mender = put(&mut g, 1, "Circuit Mender");
    let c = hand(&mut g, 0, "Rapid Hybridization");
    play(&mut g, c, false, &[obj(mender)], &[]);
    assert_eq!(g.obj(mender).zone, Zone::Graveyard);
    assert_eq!(named(&g, 1, "Frog Lizard").len(), 1);
}

#[test]
fn selfless_spirit_squirrel_sovereign_and_zulaport_cutthroat() {
    let mut g = table();
    let spirit = put(&mut g, 0, "Selfless Spirit");
    let mender = put(&mut g, 0, "Circuit Mender");
    act(&mut g, Action::Activate(spirit, 0));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(mender).has(Keywords::INDESTRUCTIBLE));
    let sov = put(&mut g, 0, "Squirrel Sovereign");
    let hazel = put(&mut g, 0, "Hazel of the Rootbloom"); // a Squirrel
    assert_eq!(g.power(hazel), g.def_of(hazel).power + 1);
    assert_eq!(g.power(sov), 2, "other Squirrels");
    let mut g = table();
    put(&mut g, 0, "Zulaport Cutthroat");
    let m = put(&mut g, 0, "Circuit Mender");
    g.move_card(m, Zone::Graveyard);
    act(&mut g, Action::PassPriority);
    resolve_all(&mut g, &[]);
    assert_eq!((g.players[0].life, g.players[1].life), (41, 39));
}

#[test]
fn sun_titan_returns_a_cheap_permanent_and_sunscorch_regent_grows() {
    let mut g = table();
    let ring = g.add_card(0, bid("Sol Ring"), Zone::Graveyard);
    let big = g.add_card(0, bid("Thran Dynamo"), Zone::Graveyard); // mana value 4
    let t = hand(&mut g, 0, "Sun Titan");
    act(&mut g, Action::Cast(t));
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(!g.legal_actions().contains(&Action::Target(obj(big))));
    resolve_all(&mut g, &[Action::Target(obj(ring)), Action::Yes]);
    assert_eq!(g.obj(ring).zone, Zone::Battlefield);
    assert!(g.keywords(t).has(Keywords::VIGILANCE));

    let mut g = table();
    let r = put(&mut g, 0, "Sunscorch Regent");
    let theirs = hand(&mut g, 1, "Sol Ring");
    g.start_turn(4, 1);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Cast(theirs));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(r).counters, 1);
    assert_eq!(g.players[0].life, 41);
}

// ---- batch 7 (finished by lane A, 2026-11-25) ----

/// Attack with `attackers` on a later turn (no summoning sickness); stops after attackers are declared.
fn attack(g: &mut Game, attackers: &[ObjId]) {
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    for &a in attackers {
        act(g, Action::Attack(a));
    }
    act(g, Action::FinishAttacks);
}

#[test]
fn thriving_isle_enters_tapped_and_makes_blue_or_the_chosen_color() {
    let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[]);
    let isle = hand(&mut g, 0, "Thriving Isle");
    act(&mut g, Action::PlayLand(isle));
    assert!(
        matches!(g.pending, Pending::Decide(0)),
        "choose a color as it enters"
    );
    assert!(
        !g.legal_actions().contains(&Action::Mode(1)),
        "a color other than blue"
    );
    act(&mut g, Action::Mode(4)); // green
    assert!(g.obj(isle).tapped);
    let made: Vec<[bool; 5]> = {
        g.start_turn(5, 0);
        while g.step != Step::Main1 {
            act(&mut g, Action::PassPriority);
        }
        colors_of(&g, isle)
    };
    let any = made
        .iter()
        .fold([false; 5], |acc, c| std::array::from_fn(|i| acc[i] || c[i]));
    assert_eq!(any, [false, true, false, false, true], "blue or green");
}

#[test]
fn angel_of_the_ruins_exiles_two_and_plainscycles() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let onslaught = put(&mut g, 1, "Berserkers' Onslaught");
    let angel = hand(&mut g, 0, "Angel of the Ruins");
    play(
        &mut g,
        angel,
        false,
        &[],
        &[Action::Target(obj(ring)), Action::Target(obj(onslaught))],
    );
    assert_eq!(g.obj(ring).zone, Zone::Exile);
    assert_eq!(g.obj(onslaught).zone, Zone::Exile);

    let mut g = table();
    let angel = hand(&mut g, 0, "Angel of the Ruins");
    let plains = g.add_card(0, bid("Plains"), Zone::Library);
    let cycle = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a, Action::Activate(id, _) if *id == angel))
        .expect("plainscycling from hand");
    act(&mut g, cycle);
    resolve_all(&mut g, &[Action::Pick(plains)]);
    assert_eq!(g.obj(angel).zone, Zone::Graveyard);
    assert_eq!(g.obj(plains).zone, Zone::Hand);
}

#[test]
fn decimate_destroys_one_of_each() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let elder = put(&mut g, 1, "Sakura-Tribe Elder");
    let onslaught = put(&mut g, 1, "Berserkers' Onslaught");
    let land = put(&mut g, 1, "Command Tower");
    let decimate = hand(&mut g, 0, "Decimate");
    play(
        &mut g,
        decimate,
        false,
        &[obj(ring), obj(elder), obj(onslaught), obj(land)],
        &[],
    );
    for x in [ring, elder, onslaught, land] {
        assert_eq!(g.obj(x).zone, Zone::Graveyard);
    }
}

#[test]
fn berserkers_onslaught_and_gratuitous_violence_hit_harder() {
    let mut g = table();
    put(&mut g, 0, "Berserkers' Onslaught");
    put(&mut g, 0, "Gratuitous Violence");
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    let p = g.power(elder);
    let life = g.players[1].life;
    attack(&mut g, &[elder]);
    assert!(
        g.keywords(elder).has(Keywords::DOUBLE_STRIKE),
        "attacking: double strike"
    );
    while g.step != Step::Main2 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[1].life, life - 4 * p, "two hits, each doubled");
}

#[test]
fn jazal_goldmane_pumps_attackers_by_their_number() {
    let mut g = table();
    let jazal = put(&mut g, 0, "Jazal Goldmane");
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    let p = g.power(elder);
    attack(&mut g, &[jazal, elder]);
    let pump = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a, Action::Activate(id, _) if *id == jazal))
        .expect("Jazal's ability during combat");
    act(&mut g, pump);
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(elder), p + 2, "two attackers: +2/+2");
}

#[test]
fn binding_the_old_gods_destroys_then_fetches_a_forest() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let saga = hand(&mut g, 0, "Binding the Old Gods");
    play(&mut g, saga, false, &[], &[Action::Target(obj(ring))]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard, "chapter I");
    let forest = g.add_card(0, bid("Forest"), Zone::Library);
    g.add_card(0, bid("Plains"), Zone::Library); // on top: the draw step takes this, not the Forest
    g.start_turn(5, 0);
    let mut picked = false;
    for _ in 0..30 {
        if g.step == Step::Main1 && g.items.is_empty() && !matches!(g.pending, Pending::Decide(_)) && picked {
            break;
        }
        if matches!(g.pending, Pending::Decide(0)) && g.legal_actions().contains(&Action::Pick(forest)) {
            act(&mut g, Action::Pick(forest));
            picked = true;
            continue;
        }
        act(&mut g, Action::PassPriority);
    }
    assert!(picked, "chapter II searches for a Forest");
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(g.obj(forest).tapped);
}

// ---- batch 8 (2026-11-25 list) ----

/// A token definition's index by name.
fn token_def(name: &str) -> usize {
    let s = set("BLC").unwrap();
    (s.card_count()..s.defs.len())
        .find(|&i| s.defs[i].name == name)
        .unwrap_or_else(|| panic!("no {name} token"))
}

/// Passes until the next main phase 2 (through combat).
fn to_main2(g: &mut Game) {
    while g.step != Step::Main2 {
        act(g, Action::PassPriority);
    }
}

/// Puts player 1 at their first main phase, turn 4.
fn their_main(g: &mut Game) {
    g.start_turn(4, 1);
    while g.step != Step::Main1 {
        act(g, Action::PassPriority);
    }
}

#[test]
fn thriving_grove_and_game_trail() {
    let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[]);
    let grove = hand(&mut g, 0, "Thriving Grove");
    act(&mut g, Action::PlayLand(grove));
    assert!(
        !g.legal_actions().contains(&Action::Mode(4)),
        "a color other than green"
    );
    act(&mut g, Action::Mode(1));
    assert!(g.obj(grove).tapped);

    // Game Trail: no Mountain or Forest to reveal (the hand is Plains), so it enters tapped.
    let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[]);
    let trail = hand(&mut g, 0, "Game Trail");
    act(&mut g, Action::PlayLand(trail));
    assert!(g.obj(trail).tapped);
    let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[]);
    hand(&mut g, 0, "Forest");
    let trail = hand(&mut g, 0, "Game Trail");
    act(&mut g, Action::PlayLand(trail));
    assert!(!g.obj(trail).tapped, "revealed a Forest");
    let rg = [false, false, false, true, true];
    let made = colors_of(&g, trail)
        .iter()
        .fold([false; 5], |acc, c| std::array::from_fn(|i| acc[i] || c[i]));
    assert_eq!(made, rg);
}

#[test]
fn talisman_of_resilience_makes_colorless_free_and_black_or_green_for_one_damage() {
    let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[]);
    let t = put(&mut g, 0, "Talisman of Resilience");
    put(&mut g, 0, "Sol Ring");
    let sources: Vec<_> = g.mana_sources(0).into_iter().filter(|&(x, _)| x == t).collect();
    assert!(sources.iter().any(|(_, m)| m.colorless));
    assert!(sources.iter().any(|(_, m)| m.colors[2]) && sources.iter().any(|(_, m)| m.colors[4]));
    let cobra = hand(&mut g, 0, "Lotus Cobra"); // {1}{G}: the green comes from the Talisman
    let life = g.players[0].life;
    play(&mut g, cobra, false, &[], &[]);
    assert_eq!(g.obj(cobra).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, life - 1);
}

#[test]
fn unnatural_growth_doubles_your_creatures_each_combat() {
    let mut g = table();
    put(&mut g, 0, "Unnatural Growth");
    let valet = put(&mut g, 0, "Devilish Valet"); // 1/3
    let theirs = put(&mut g, 1, "Sakura-Tribe Elder");
    attack(&mut g, &[]);
    assert_eq!((g.power(valet), g.toughness(valet)), (2, 6));
    assert_eq!(g.power(theirs), 1, "only yours");
}

#[test]
fn siege_gang_commander_makes_goblins_and_sacrifices_goblins_for_damage() {
    let mut g = table();
    let sg = hand(&mut g, 0, "Siege-Gang Commander");
    play(&mut g, sg, false, &[], &[]);
    let goblins = tokens_named(&g, 0, "Goblin");
    assert_eq!(goblins.len(), 3);
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    assert!(
        !g.legal_actions().contains(&Action::ActivateWith(sg, 0, elder)),
        "only a Goblin"
    );
    act(&mut g, Action::ActivateWith(sg, 0, goblins[0]));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].life, 38);
    assert_eq!(tokens_named(&g, 0, "Goblin").len(), 2);
    // It is a Goblin itself, and still deals the damage after it's gone.
    act(&mut g, Action::ActivateWith(sg, 0, sg));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert_eq!(g.obj(sg).zone, Zone::Graveyard);
    assert_eq!(g.players[1].life, 36);
}

#[test]
fn thopter_engineer_makes_a_thopter_and_gives_artifact_creatures_haste() {
    let mut g = table();
    let e = hand(&mut g, 0, "Thopter Engineer");
    play(&mut g, e, false, &[], &[]);
    let thopter = tokens_named(&g, 0, "Thopter");
    assert_eq!(thopter.len(), 1);
    let k = g.keywords(thopter[0]);
    assert!(k.has(Keywords::FLYING) && k.has(Keywords::HASTE));
    assert!(g.types_of(thopter[0]).artifact);
    assert!(!g.keywords(e).has(Keywords::HASTE), "not an artifact");
    let theirs = g.add_card(1, token_def("Thopter"), Zone::Battlefield);
    assert!(!g.keywords(theirs).has(Keywords::HASTE), "only yours");
}

#[test]
fn devilish_valet_doubles_its_power_whenever_another_creature_enters() {
    let mut g = table();
    let valet = put(&mut g, 0, "Devilish Valet");
    let k = g.keywords(valet);
    assert!(k.has(Keywords::TRAMPLE) && k.has(Keywords::HASTE));
    let a = hand(&mut g, 0, "Sakura-Tribe Elder");
    play(&mut g, a, false, &[], &[]);
    assert_eq!(g.power(valet), 2);
    let b = hand(&mut g, 0, "Sakura-Tribe Elder");
    play(&mut g, b, false, &[], &[]);
    assert_eq!((g.power(valet), g.toughness(valet)), (4, 3));
}

#[test]
fn steelburr_champion_grows_on_opponents_noncreature_spells() {
    let mut g = table();
    let c = hand(&mut g, 0, "Steelburr Champion");
    act(&mut g, Action::CastKicked(c));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens_named(&g, 0, "Steelburr Champion").len(), 1, "offspring");
    assert!(g.keywords(c).has(Keywords::VIGILANCE));
    let ring = hand(&mut g, 1, "Sol Ring");
    let elder = hand(&mut g, 1, "Sakura-Tribe Elder");
    their_main(&mut g);
    act(&mut g, Action::Cast(elder));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).counters, 0, "a creature spell");
    act(&mut g, Action::Cast(ring));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).counters, 1);
}

#[test]
fn prosperous_bandit_makes_tapped_treasures_for_its_damage() {
    let mut g = table();
    let b = put(&mut g, 0, "Prosperous Bandit");
    assert!(g.keywords(b).has(Keywords::FIRST_STRIKE));
    let life = g.players[1].life;
    attack(&mut g, &[b]);
    to_main2(&mut g);
    assert_eq!(g.players[1].life, life - 2);
    let t = tokens_named(&g, 0, "Treasure");
    assert_eq!(t.len(), 2);
    assert!(t.iter().all(|&x| g.obj(x).tapped));
}

#[test]
fn curiosity_crafter_draws_when_a_creature_token_connects() {
    let mut g = table();
    let crafter = put(&mut g, 0, "Curiosity Crafter");
    let thopter = g.add_card(0, token_def("Thopter"), Zone::Battlefield);
    attack(&mut g, &[crafter]);
    let before = g.players[0].hand.len();
    to_main2(&mut g);
    assert_eq!(g.players[0].hand.len(), before, "a nontoken creature: no card");
    attack(&mut g, &[crafter, thopter]);
    let before = g.players[0].hand.len();
    to_main2(&mut g);
    assert_eq!(g.players[0].hand.len(), before + 1, "the token");
    assert!(g.def_of(crafter).no_max_hand_size);
}

#[test]
fn triskaidekaphile_wins_with_exactly_thirteen_cards() {
    let mut g = table();
    let t = put(&mut g, 0, "Triskaidekaphile");
    let before = g.players[0].hand.len();
    act(&mut g, Action::Activate(t, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), before + 1, "{{3}}{{U}}: draw");
    for n in [12, 13] {
        while g.players[0].hand.len() < n {
            hand(&mut g, 0, "Plains");
        }
        g.start_turn(5 + n as u32, 0);
        while g.result.is_none() && g.step != Step::Main1 {
            act(&mut g, Action::PassPriority);
        }
        if n == 12 {
            assert!(g.result.is_none(), "twelve: no win");
        } else {
            assert_eq!(g.result, Some(bolt_core::Outcome::Winner(0)));
        }
    }
}

#[test]
fn murmuration_pumps_birds_and_makes_a_storm_crow_per_spell() {
    let mut g = table();
    put(&mut g, 0, "Murmuration");
    for _ in 0..2 {
        let ring = hand(&mut g, 0, "Sol Ring");
        play(&mut g, ring, false, &[], &[]);
    }
    while g.step != Step::End {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    resolve_all(&mut g, &[]);
    let crows = tokens_named(&g, 0, "Storm Crow");
    assert_eq!(crows.len(), 2);
    for c in crows {
        assert_eq!((g.power(c), g.toughness(c)), (2, 3), "1/2, +1/+1");
        let k = g.keywords(c);
        assert!(k.has(Keywords::FLYING) && k.has(Keywords::VIGILANCE));
    }
}

// ---- batch 9 ----

#[test]
fn karoos_bounce_a_land_and_tap_for_two_colors() {
    let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[]);
    let forest = put(&mut g, 0, "Forest");
    let farm = hand(&mut g, 0, "Golgari Rot Farm");
    act(&mut g, Action::PlayLand(farm));
    resolve_all(&mut g, &[Action::Pick(forest)]);
    assert!(g.obj(farm).tapped);
    assert_eq!(g.obj(forest).zone, Zone::Hand);

    // {B}{G} from one tap: Sol Ring pays the {1} of Putrefy.
    let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[]);
    let farm = put(&mut g, 0, "Golgari Rot Farm");
    put(&mut g, 0, "Sol Ring");
    let elder = put(&mut g, 1, "Sakura-Tribe Elder");
    let putrefy = hand(&mut g, 0, "Putrefy");
    play(&mut g, putrefy, false, &[obj(elder)], &[]);
    assert_eq!(g.obj(elder).zone, Zone::Graveyard);
    assert!(g.obj(farm).tapped);

    let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[]);
    let turf = put(&mut g, 0, "Gruul Turf");
    let cobra = hand(&mut g, 0, "Lotus Cobra"); // {1}{G}: {R}{G}
    play(&mut g, cobra, false, &[], &[]);
    assert_eq!(g.obj(cobra).zone, Zone::Battlefield);
    assert!(g.obj(turf).tapped);
}

#[test]
fn loyal_warhound_fetches_a_plains_only_when_behind_on_lands() {
    let mut g = game(
        ["Bello, Bard of the Brambles", "Ms. Bumbleflower"],
        &["Plains", "Plains"],
    );
    let dog = hand(&mut g, 0, "Loyal Warhound");
    let plains = *g.players[0].library.last().unwrap();
    play(&mut g, dog, false, &[], &[Action::Pick(plains)]);
    assert!(g.keywords(dog).has(Keywords::VIGILANCE));
    assert_eq!(g.obj(plains).zone, Zone::Battlefield);
    assert!(g.obj(plains).tapped);

    let mut g = table(); // 25 lands each: not behind
    let dog = hand(&mut g, 0, "Loyal Warhound");
    let lib = g.players[0].library.len();
    play(&mut g, dog, false, &[], &[]);
    assert_eq!(g.players[0].library.len(), lib);
}

#[test]
fn warstorm_surge_makes_entering_creatures_deal_their_power() {
    let mut g = table();
    put(&mut g, 0, "Warstorm Surge");
    let titan = hand(&mut g, 0, "Kalonian Hydra"); // enters with four counters: 4/4
    let theirs = put(&mut g, 1, "Sakura-Tribe Elder");
    play(&mut g, titan, false, &[], &[Action::Target(obj(theirs))]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    let elder = hand(&mut g, 0, "Sakura-Tribe Elder");
    play(&mut g, elder, false, &[], &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].life, 39, "1 power");
}

#[test]
fn rites_of_flourishing_gives_each_player_a_card_and_a_land() {
    let mut g = table();
    put(&mut g, 0, "Rites of Flourishing");
    let before = g.players[0].hand.len();
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[0].hand.len(), before + 2);
    let a = hand(&mut g, 0, "Forest");
    let b = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(a));
    assert!(g.legal_actions().contains(&Action::PlayLand(b)), "a second land");
    act(&mut g, Action::PlayLand(b));
    let c = hand(&mut g, 0, "Forest");
    assert!(!g.legal_actions().contains(&Action::PlayLand(c)), "not a third");

    let theirs = g.players[1].hand.len();
    their_main(&mut g);
    assert_eq!(g.players[1].hand.len(), theirs + 2, "each player");
}

#[test]
fn garruk_makes_wolves_that_feed_him_and_an_emblem() {
    let mut g = table();
    let garruk = hand(&mut g, 0, "Garruk, Cursed Huntsman");
    play(&mut g, garruk, false, &[], &[]);
    assert_eq!(g.obj(garruk).loyalty, 5);
    act(&mut g, Action::Loyalty(garruk, 0));
    resolve_all(&mut g, &[]);
    let wolves = tokens_named(&g, 0, "Wolf");
    assert_eq!(wolves.len(), 2);
    assert_eq!(g.obj(garruk).loyalty, 5);
    let gift = hand(&mut g, 0, "Beast Within");
    play(&mut g, gift, false, &[obj(wolves[0])], &[]);
    assert_eq!(g.obj(garruk).loyalty, 6, "a Wolf died");

    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Loyalty(garruk, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(garruk).zone, Zone::Graveyard, "0 loyalty");
    let w = wolves[1];
    assert_eq!((g.power(w), g.toughness(w)), (5, 5));
    assert!(g.keywords(w).has(Keywords::TRAMPLE));

    // −3: destroy target creature, draw a card.
    let mut g = table();
    let garruk = hand(&mut g, 0, "Garruk, Cursed Huntsman");
    play(&mut g, garruk, false, &[], &[]);
    let elder = put(&mut g, 1, "Sakura-Tribe Elder");
    let cards = g.players[0].hand.len();
    act(&mut g, Action::Loyalty(garruk, 1));
    resolve_all(&mut g, &[Action::Target(obj(elder))]);
    assert_eq!(g.obj(elder).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), cards + 1);
    assert_eq!(g.obj(garruk).loyalty, 2);
}

#[test]
fn plaguecrafter_makes_each_player_sacrifice_or_discard() {
    let mut g = table();
    let elder = put(&mut g, 1, "Sakura-Tribe Elder");
    let pc = hand(&mut g, 0, "Plaguecrafter");
    play(&mut g, pc, false, &[], &[]);
    assert_eq!(g.obj(pc).zone, Zone::Graveyard, "your only creature");
    assert_eq!(g.obj(elder).zone, Zone::Graveyard);

    let mut g = table();
    let mine = put(&mut g, 0, "Sakura-Tribe Elder");
    let pc = hand(&mut g, 0, "Plaguecrafter");
    let card = g.players[1].hand[0];
    let cards = g.players[1].hand.len();
    play(
        &mut g,
        pc,
        false,
        &[],
        &[Action::Pick(mine), Action::Discard(card)],
    );
    assert_eq!(g.obj(mine).zone, Zone::Graveyard, "your choice");
    assert_eq!(g.obj(pc).zone, Zone::Battlefield);
    assert_eq!(g.players[1].hand.len(), cards - 1, "no creature: a discard");
}

#[test]
fn chaos_warp_shuffles_a_permanent_away_and_reveals_a_replacement() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let before = g.permanents(1).count();
    let warp = hand(&mut g, 0, "Chaos Warp");
    play(&mut g, warp, false, &[obj(ring)], &[]);
    // Their library is Plains (and now the Ring): whichever is on top is a permanent card.
    assert_eq!(g.permanents(1).count(), before);
    let new = g.permanents(1).last().unwrap();
    assert!(new == ring || g.def_of(new).name == "Plains");
    assert_eq!(g.obj(new).controller, 1);
}

#[test]
fn decree_of_pain_wipes_and_draws_or_cycles_for_minus_two() {
    let mut g = table();
    put(&mut g, 0, "Sakura-Tribe Elder");
    put(&mut g, 1, "Sakura-Tribe Elder");
    let valet = put(&mut g, 1, "Devilish Valet");
    let decree = hand(&mut g, 0, "Decree of Pain");
    let cards = g.players[0].hand.len();
    play(&mut g, decree, false, &[], &[]);
    assert!(g.creatures(0).next().is_none() && g.creatures(1).next().is_none());
    assert_eq!(g.obj(valet).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), cards - 1 + 3, "three destroyed");

    let mut g = table();
    let elder = put(&mut g, 1, "Sakura-Tribe Elder");
    let valet = put(&mut g, 1, "Devilish Valet"); // 1/3
    let decree = hand(&mut g, 0, "Decree of Pain");
    let cards = g.players[0].hand.len();
    let cycle = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a, Action::Activate(id, _) if *id == decree))
        .expect("cycling from hand");
    act(&mut g, cycle);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(decree).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), cards, "discarded it, drew one");
    assert_eq!(g.obj(elder).zone, Zone::Graveyard);
    assert_eq!(g.toughness(valet), 1);
}

#[test]
fn spore_frog_fogs_combat_damage() {
    let mut g = table();
    let frog = put(&mut g, 0, "Spore Frog");
    let valet = put(&mut g, 0, "Devilish Valet");
    let life = g.players[1].life;
    attack(&mut g, &[valet]);
    act(&mut g, Action::Activate(frog, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(frog).zone, Zone::Graveyard);
    to_main2(&mut g);
    assert_eq!(g.players[1].life, life);
    // Only this turn.
    g.start_turn(7, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(valet));
    act(&mut g, Action::FinishAttacks);
    to_main2(&mut g);
    assert_eq!(g.players[1].life, life - 1);
}

#[test]
fn riot_control_gains_per_opposing_creature_and_prevents_damage_to_you() {
    let mut g = table();
    put(&mut g, 1, "Sakura-Tribe Elder");
    put(&mut g, 1, "Devilish Valet");
    put(&mut g, 0, "Spore Frog"); // yours don't count
    let sg = put(&mut g, 0, "Siege-Gang Commander");
    let riot = hand(&mut g, 0, "Riot Control");
    play(&mut g, riot, false, &[], &[]);
    assert_eq!(g.players[0].life, 42);
    act(&mut g, Action::ActivateWith(sg, 0, sg));
    resolve_all(&mut g, &[Action::Target(Target::Player(0))]);
    assert_eq!(g.players[0].life, 42, "prevented");
}

// ---- batch 10 ----

/// Passes to main phase 2, answering target choices and decisions from `answers` in order.
fn to_main2_answering(g: &mut Game, answers: &[Action]) {
    let mut answers = answers.iter();
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::ChooseTarget(_) | Pending::Decide(_) => *answers.next().expect("an answer"),
            _ => Action::PassPriority,
        };
        act(g, a);
    }
    assert!(answers.next().is_none(), "unused answers");
}

#[test]
fn scurry_of_squirrels_puts_a_counter_on_a_creature_when_it_connects() {
    let mut g = table();
    let scurry = put(&mut g, 0, "Scurry of Squirrels");
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    let life = g.players[1].life;
    attack(&mut g, &[scurry]);
    to_main2_answering(&mut g, &[Action::Target(obj(elder))]);
    assert_eq!(g.players[1].life, life - 2);
    assert_eq!(g.obj(elder).counters, 1);
    assert_eq!(
        g.permanents(0).filter(|&x| g.is_token(x)).count(),
        0,
        "no myriad copies 1v1"
    );
}

#[test]
fn faeburrow_elder_grows_and_taps_for_each_color_you_have() {
    let mut g = table();
    let elder = put(&mut g, 0, "Faeburrow Elder");
    assert!(g.keywords(elder).has(Keywords::VIGILANCE));
    assert_eq!((g.power(elder), g.toughness(elder)), (2, 2), "green and white");
    put(&mut g, 1, "Devilish Valet"); // theirs don't count
    put(&mut g, 0, "Sol Ring"); // colorless
    assert_eq!(g.power(elder), 2);
    put(&mut g, 0, "Devilish Valet");
    assert_eq!((g.power(elder), g.toughness(elder)), (3, 3));
    let made = colors_of(&g, elder);
    assert_eq!(made.len(), 3, "one mana of each of three colors");
}

#[test]
fn beledros_makes_pests_every_upkeep_and_untaps_lands_once_a_turn() {
    let mut g = table();
    let b = put(&mut g, 0, "Beledros Witherbloom");
    their_main(&mut g);
    let pests = tokens_named(&g, 0, "Pest");
    assert_eq!(pests.len(), 1, "on the opponent's upkeep too");

    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(tokens_named(&g, 0, "Pest").len(), 2);
    let titan = hand(&mut g, 0, "Kalonian Hydra");
    play(&mut g, titan, false, &[], &[]);
    let tapped = g
        .permanents(0)
        .filter(|&x| g.types_of(x).land && g.obj(x).tapped)
        .count();
    assert_eq!(tapped, 5);
    let life = g.players[0].life;
    act(&mut g, Action::Activate(b, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, life - 10);
    assert!(g
        .permanents(0)
        .filter(|&x| g.types_of(x).land)
        .all(|x| !g.obj(x).tapped));
    assert!(
        !g.legal_actions().contains(&Action::Activate(b, 0)),
        "once each turn"
    );

    // A Pest dying gains 1.
    let gift = hand(&mut g, 0, "Beast Within");
    let life = g.players[0].life;
    play(&mut g, gift, false, &[obj(pests[0])], &[]);
    assert_eq!(g.players[0].life, life + 1);
}

#[test]
fn gilded_goose_makes_food_and_eats_it_for_mana() {
    let mut g = table();
    let goose = hand(&mut g, 0, "Gilded Goose");
    play(&mut g, goose, false, &[], &[]);
    assert!(g.keywords(goose).has(Keywords::FLYING));
    let food = tokens_named(&g, 0, "Food");
    assert_eq!(food.len(), 1);

    let mut g = table();
    let goose = put(&mut g, 0, "Gilded Goose");
    let food = g.add_card(0, token_def("Food"), Zone::Battlefield);
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    assert!(
        !g.legal_actions().contains(&Action::ActivateWith(goose, 1, elder)),
        "only a Food"
    );
    let pool = g.players[0].pool.len();
    act(&mut g, Action::ActivateWith(goose, 1, food));
    resolve_all(&mut g, &[]);
    assert!(tokens_named(&g, 0, "Food").is_empty());
    assert_eq!(g.players[0].pool.len(), pool + 1);
    assert!(g.obj(goose).tapped);

    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Activate(goose, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens_named(&g, 0, "Food").len(), 1, "{{1}}{{G}}, {{T}}: a Food");
}

#[test]
fn jacked_rabbit_is_ravenous_and_makes_rabbits_for_its_power() {
    let mut g = table();
    let r = hand(&mut g, 0, "Jacked Rabbit");
    let cards = g.players[0].hand.len();
    act(&mut g, Action::CastX(r, 5));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(r).counters, 5);
    assert_eq!(g.players[0].hand.len(), cards - 1 + 1, "X 5: a card");
    attack(&mut g, &[r]);
    while !g.items.is_empty() {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(tokens_named(&g, 0, "Rabbit").len(), 6, "power 6");

    let mut g = table();
    let r = hand(&mut g, 0, "Jacked Rabbit");
    let cards = g.players[0].hand.len();
    act(&mut g, Action::CastX(r, 4));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), cards - 1, "X 4: no card");
}

#[test]
fn selvala_parleys_for_green_mana_life_and_cards() {
    let mut g = table();
    let s = put(&mut g, 0, "Selvala, Explorer Returned");
    g.add_card(1, bid("Sol Ring"), Zone::Library); // their top: nonland; yours: a Plains
    let (mine, theirs) = (g.players[0].hand.len(), g.players[1].hand.len());
    let (pool, life) = (g.players[0].pool.len(), g.players[0].life);
    act(&mut g, Action::Activate(s, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].pool.len(), pool + 1);
    assert!(g.players[0].pool.last().unwrap().colors[4], "green");
    assert_eq!(g.players[0].life, life + 1);
    assert_eq!(
        (g.players[0].hand.len(), g.players[1].hand.len()),
        (mine + 1, theirs + 1)
    );
}

#[test]
fn swarmyard_massacre_shrinks_everything_but_the_vermin() {
    let mut g = table();
    let valet = put(&mut g, 1, "Devilish Valet"); // 1/3
    let elder = put(&mut g, 1, "Sakura-Tribe Elder");
    let sq = put(&mut g, 1, "Squirrel Sovereign"); // a Squirrel: spared
    let m = hand(&mut g, 0, "Swarmyard Massacre");
    play(&mut g, m, false, &[], &[]);
    let mine = tokens_named(&g, 0, "Squirrel");
    assert_eq!(mine.len(), 2);
    assert_eq!(g.toughness(mine[0]), 1);
    assert_eq!(g.obj(elder).zone, Zone::Graveyard);
    assert_eq!(g.toughness(valet), 1, "-2/-2 for your two Squirrels");
    assert_eq!(g.obj(sq).zone, Zone::Battlefield);
}

#[test]
fn skullclamp_draws_two_when_the_equipped_creature_dies() {
    let mut g = table();
    let clamp = put(&mut g, 0, "Skullclamp");
    let valet = put(&mut g, 0, "Devilish Valet");
    act(&mut g, Action::Activate(clamp, 0));
    resolve_all(&mut g, &[Action::Target(obj(valet))]);
    assert_eq!((g.power(valet), g.toughness(valet)), (2, 2));
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    let cards = g.players[0].hand.len();
    act(&mut g, Action::Activate(clamp, 0));
    resolve_all(&mut g, &[Action::Target(obj(elder))]);
    assert_eq!(g.obj(elder).zone, Zone::Graveyard, "1/0");
    assert_eq!(g.players[0].hand.len(), cards + 2);
}

#[test]
fn outpost_siege_pings_when_your_creatures_leave_or_exiles_your_top_card() {
    let mut g = table();
    let siege = hand(&mut g, 0, "Outpost Siege");
    play(&mut g, siege, false, &[], &[Action::Mode(1)]);
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    let basic = *g.players[0].library.last().unwrap();
    act(&mut g, Action::Activate(elder, 0));
    resolve_all(&mut g, &[Action::Target(Target::Player(1)), Action::Pick(basic)]);
    assert_eq!(g.players[1].life, 39, "Dragons: the Elder left");

    let mut g = table();
    let siege = hand(&mut g, 0, "Outpost Siege");
    play(&mut g, siege, false, &[], &[Action::Mode(0)]);
    g.start_turn(5, 0);
    let top = *g.players[0].library.last().unwrap();
    while g.step != Step::Main1 {
        let a = if matches!(g.pending, Pending::Decide(0)) {
            Action::Pick(top)
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(top).zone, Zone::Exile, "Khans: the top card");
    assert!(
        g.legal_actions().contains(&Action::PlayLand(top)),
        "playable this turn"
    );
}

#[test]
fn tainted_wood_needs_a_swamp_for_colored_mana() {
    let mut g = game(["Bello, Bard of the Brambles", "Ms. Bumbleflower"], &[]);
    let w = put(&mut g, 0, "Tainted Wood");
    let only = colors_of(&g, w);
    assert_eq!(only, [[false; 5]], "just {{C}}");
    assert!(g.mana_sources(0).iter().any(|&(x, m)| x == w && m.colorless));
    put(&mut g, 0, "Swamp");
    let bg = [false, false, true, false, true];
    assert_eq!(colors_of(&g, w), [bg]);
}

#[test]
fn llanowar_loamspeaker_taps_for_any_color_and_animates_a_land() {
    let mut g = table();
    let ls = put(&mut g, 0, "Llanowar Loamspeaker");
    assert_eq!(colors_of(&g, ls), [[true; 5]]);
    let forest = put(&mut g, 0, "Forest");
    act(&mut g, Action::Activate(ls, 0));
    resolve_all(&mut g, &[Action::Target(obj(forest))]);
    assert!(g.types_of(forest).creature && g.types_of(forest).land);
    assert_eq!((g.power(forest), g.toughness(forest)), (3, 3));
    assert!(g.keywords(forest).has(Keywords::HASTE));
    g.start_turn(5, 0);
    assert!(!g.types_of(forest).creature, "until end of turn");
}

// ---- batch 11 ----

#[test]
fn peerless_recycling_returns_one_or_two_with_the_gift() {
    let mut g = table();
    let ring = g.add_card(0, bid("Sol Ring"), Zone::Graveyard);
    let elder = g.add_card(0, bid("Sakura-Tribe Elder"), Zone::Graveyard);
    let harmonize = g.add_card(0, bid("Harmonize"), Zone::Graveyard); // not a permanent card
    let r = hand(&mut g, 0, "Peerless Recycling");
    act(&mut g, Action::Cast(r));
    assert!(!g.legal_actions().contains(&Action::Target(obj(harmonize))));
    act(&mut g, Action::Target(obj(ring)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ring).zone, Zone::Hand);
    assert_eq!(g.obj(elder).zone, Zone::Graveyard);

    let mut g = table();
    let ring = g.add_card(0, bid("Sol Ring"), Zone::Graveyard);
    let elder = g.add_card(0, bid("Sakura-Tribe Elder"), Zone::Graveyard);
    let r = hand(&mut g, 0, "Peerless Recycling");
    let theirs = g.players[1].hand.len();
    play(&mut g, r, true, &[obj(ring), obj(elder)], &[]);
    assert_eq!(g.obj(ring).zone, Zone::Hand);
    assert_eq!(g.obj(elder).zone, Zone::Hand);
    assert_eq!(g.players[1].hand.len(), theirs + 1, "the gift: a card");
}

#[test]
fn octomancer_gifts_an_octopus_and_copies_new_tokens_each_end_step() {
    let mut g = table();
    let o = hand(&mut g, 0, "Octomancer");
    act(&mut g, Action::CastKicked(o));
    resolve_all(&mut g, &[]);
    let octo = tokens_named(&g, 1, "Octopus");
    assert_eq!(octo.len(), 1);
    assert_eq!(g.power(octo[0]), 8);
    let sg = hand(&mut g, 0, "Siege-Gang Commander");
    play(&mut g, sg, false, &[], &[]);
    let old = g.add_card(0, token_def("Thopter"), Zone::Battlefield); // there since before this turn
    let goblin = tokens_named(&g, 0, "Goblin")[0];
    while g.step != Step::End {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(matches!(g.pending, Pending::ChooseTarget(0)));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(old))),
        "entered this turn"
    );
    resolve_all(&mut g, &[Action::Target(obj(goblin))]);
    assert_eq!(tokens_named(&g, 0, "Goblin").len(), 4);

    let mut g = table();
    let o = hand(&mut g, 0, "Octomancer");
    play(&mut g, o, false, &[], &[]);
    assert!(tokens_named(&g, 1, "Octopus").is_empty(), "no gift");
}

#[test]
fn rowdy_research_costs_less_per_creature_that_attacked() {
    let mut g = game(
        ["Bello, Bard of the Brambles", "Ms. Bumbleflower"],
        &["Island", "Island", "Island", "Island", "Island"],
    );
    let a = put(&mut g, 0, "Sakura-Tribe Elder");
    let b = put(&mut g, 0, "Devilish Valet");
    let r = hand(&mut g, 0, "Rowdy Research");
    assert!(
        !g.legal_actions().contains(&Action::Cast(r)),
        "{{6}}{{U}} with five lands"
    );
    attack(&mut g, &[a, b]);
    to_main2(&mut g);
    assert!(
        g.legal_actions().contains(&Action::Cast(r)),
        "two attacked: {{4}}{{U}}"
    );
    let cards = g.players[0].hand.len();
    play(&mut g, r, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), cards - 1 + 3);
}

#[test]
fn jolrael_makes_cats_on_the_second_draw_and_sets_power_by_hand_size() {
    let mut g = table();
    let j = put(&mut g, 0, "Jolrael, Mwonvuli Recluse");
    let h = hand(&mut g, 0, "Harmonize"); // draws three: the second makes a Cat
    play(&mut g, h, false, &[], &[]);
    let cats = tokens_named(&g, 0, "Cat");
    assert_eq!(cats.len(), 1);
    assert_eq!(g.power(cats[0]), 2);
    let x = g.players[0].hand.len() as i32;
    act(&mut g, Action::Activate(j, 0));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(j), g.toughness(j)), (x, x));
    assert_eq!(g.power(cats[0]), x);
    g.start_turn(5, 0);
    assert_eq!(g.power(j), 1, "until end of turn");
}

#[test]
fn skyfisher_spider_trades_a_creature_for_a_permanent_and_cashes_in_its_graveyard() {
    let mut g = table();
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    let ring = put(&mut g, 1, "Sol Ring");
    let s = hand(&mut g, 0, "Skyfisher Spider");
    play(
        &mut g,
        s,
        false,
        &[],
        &[Action::Pick(elder), Action::Target(obj(ring))],
    );
    assert_eq!(g.obj(elder).zone, Zone::Graveyard);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    assert!(g.keywords(s).has(Keywords::REACH));

    // Dies: gain 1 per creature card in your graveyard (the Elder and itself), then exile it.
    let gift = hand(&mut g, 0, "Beast Within");
    let life = g.players[0].life;
    play(&mut g, gift, false, &[obj(s)], &[Action::Yes]);
    assert_eq!(g.players[0].life, life + 2);
    assert_eq!(g.obj(s).zone, Zone::Exile);

    // Declining the sacrifice destroys nothing.
    let mut g = table();
    put(&mut g, 0, "Sakura-Tribe Elder");
    let ring = put(&mut g, 1, "Sol Ring");
    let s = hand(&mut g, 0, "Skyfisher Spider");
    play(&mut g, s, false, &[], &[Action::No]);
    assert_eq!(g.obj(ring).zone, Zone::Battlefield);
}

#[test]
fn pollywog_prodigy_evolves_and_draws_off_cheap_noncreature_spells() {
    let mut g = table();
    let p = put(&mut g, 0, "Pollywog Prodigy"); // 1/3
    let elder = hand(&mut g, 0, "Sakura-Tribe Elder"); // 1/1: not greater
    play(&mut g, elder, false, &[], &[]);
    assert_eq!(g.obj(p).counters, 0);
    let valet = hand(&mut g, 0, "Devilish Valet"); // 1/3: not greater
    play(&mut g, valet, false, &[], &[]);
    assert_eq!(g.obj(p).counters, 0);
    let hydra = hand(&mut g, 0, "Kalonian Hydra"); // 4/4
    play(&mut g, hydra, false, &[], &[]);
    assert_eq!(g.obj(p).counters, 1, "evolved: 2/4");

    let ring = hand(&mut g, 1, "Sol Ring"); // mana value 1 < 2
    let dynamo = hand(&mut g, 1, "Thran Dynamo"); // 4: not less
    their_main(&mut g);
    let cards = g.players[0].hand.len();
    act(&mut g, Action::Cast(dynamo));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), cards);
    act(&mut g, Action::Cast(ring));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), cards + 1);
}

#[test]
fn mangara_draws_on_big_attacks_and_second_spells() {
    let mut g = table();
    put(&mut g, 0, "Mangara, the Diplomat");
    let ring = hand(&mut g, 1, "Sol Ring");
    let ring2 = hand(&mut g, 1, "Sol Ring");
    let a = put(&mut g, 1, "Sakura-Tribe Elder");
    let b = put(&mut g, 1, "Devilish Valet");
    their_main(&mut g);
    let cards = g.players[0].hand.len();
    act(&mut g, Action::Cast(ring));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), cards, "first spell");
    act(&mut g, Action::Cast(ring2));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), cards + 1, "second spell");
    while g.pending != Pending::DeclareAttackers(1) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(a));
    act(&mut g, Action::Attack(b));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), cards + 2, "two attackers");
}

#[test]
fn hoofprints_of_the_stag_counts_draws_into_an_elemental() {
    let mut g = table();
    let h = put(&mut g, 0, "Hoofprints of the Stag");
    let harmonize = hand(&mut g, 0, "Harmonize");
    play(
        &mut g,
        harmonize,
        false,
        &[],
        &[Action::Yes, Action::Yes, Action::Yes],
    );
    assert_eq!(g.obj(h).kind_counters, 3);
    assert!(
        !g.legal_actions().contains(&Action::Activate(h, 0)),
        "three of four"
    );
    let one = hand(&mut g, 0, "Spirited Companion");
    play(&mut g, one, false, &[], &[Action::Yes]);
    assert_eq!(g.obj(h).kind_counters, 4);
    act(&mut g, Action::Activate(h, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(h).kind_counters, 0);
    let e = tokens_named(&g, 0, "Elemental");
    assert_eq!(e.len(), 1);
    assert!(g.keywords(e[0]).has(Keywords::FLYING));
    assert_eq!(g.power(e[0]), 4);
}

#[test]
fn chitterspitter_eats_tokens_for_acorns_that_pump_squirrels() {
    let mut g = table();
    let c = put(&mut g, 0, "Chitterspitter");
    act(&mut g, Action::Activate(c, 0));
    resolve_all(&mut g, &[]);
    let squirrels = tokens_named(&g, 0, "Squirrel");
    assert_eq!(squirrels.len(), 1);
    let thopter = g.add_card(0, token_def("Thopter"), Zone::Battlefield);
    g.start_turn(5, 0);
    let mut asked = false;
    while g.step != Step::Main1 {
        let a = if matches!(g.pending, Pending::Decide(0)) {
            asked = true;
            Action::Pick(thopter)
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
    }
    assert!(asked, "upkeep: may sacrifice a token");
    assert!(tokens_named(&g, 0, "Thopter").is_empty());
    assert_eq!(g.obj(c).kind_counters, 1);
    assert_eq!((g.power(squirrels[0]), g.toughness(squirrels[0])), (2, 2));
    // Declining puts no counter on.
    g.start_turn(7, 0);
    while g.step != Step::Main1 {
        let a = if matches!(g.pending, Pending::Decide(0)) {
            Action::No
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(c).kind_counters, 1);
}

// ---- batch 12 ----

#[test]
fn teapot_slinger_pings_when_you_spend_your_fourth_mana() {
    let mut g = table();
    let t = put(&mut g, 0, "Teapot Slinger");
    assert!(g.keywords(t).has(Keywords::MENACE));
    let life = g.players[1].life;
    let a = hand(&mut g, 0, "Sakura-Tribe Elder"); // 2 mana
    play(&mut g, a, false, &[], &[]);
    assert_eq!(g.players[1].life, life, "two spent");
    let b = hand(&mut g, 0, "Lotus Cobra"); // 2 more: the fourth
    play(&mut g, b, false, &[], &[]);
    assert_eq!(g.players[1].life, life - 2);
    let c = hand(&mut g, 0, "Sakura-Tribe Elder");
    play(&mut g, c, false, &[], &[]);
    assert_eq!(
        g.players[1].life,
        life - 2,
        "once a turn: the fourth is already spent"
    );
}

#[test]
fn helm_of_the_host_copies_the_equipped_creature_without_the_legend_rule() {
    let mut g = table();
    let helm = put(&mut g, 0, "Helm of the Host");
    let jazal = put(&mut g, 0, "Jazal Goldmane"); // legendary
    act(&mut g, Action::Activate(helm, 0));
    resolve_all(&mut g, &[Action::Target(obj(jazal))]);
    attack(&mut g, &[]);
    let copies = tokens_named(&g, 0, "Jazal Goldmane");
    assert_eq!(copies.len(), 1);
    assert!(!g.types_of(copies[0]).legendary);
    assert!(g.keywords(copies[0]).has(Keywords::HASTE));
    assert_eq!(g.obj(jazal).zone, Zone::Battlefield, "no legend rule");
}

#[test]
fn tempt_with_discovery_gives_you_a_land_and_another_if_they_accept() {
    let mut g = table();
    let forest = g.add_card(0, bid("Forest"), Zone::Library);
    let mountain = g.add_card(0, bid("Mountain"), Zone::Library);
    let theirs = *g.players[1].library.last().unwrap();
    let t = hand(&mut g, 0, "Tempt with Discovery");
    play(
        &mut g,
        t,
        false,
        &[],
        &[
            Action::Pick(forest),
            Action::Yes,
            Action::Pick(theirs),
            Action::Pick(mountain),
        ],
    );
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(!g.obj(forest).tapped);
    assert_eq!(g.obj(theirs).zone, Zone::Battlefield);
    assert_eq!(g.obj(mountain).zone, Zone::Battlefield);

    let mut g = table();
    let forest = g.add_card(0, bid("Forest"), Zone::Library);
    let t = hand(&mut g, 0, "Tempt with Discovery");
    let lands = g.permanents(0).count();
    play(&mut g, t, false, &[], &[Action::Pick(forest), Action::No]);
    assert_eq!(g.permanents(0).count(), lands + 1, "declined: just one");
}

#[test]
fn wildsear_gives_enchantments_from_hand_cascade() {
    let mut g = table();
    let w = put(&mut g, 0, "Wildsear, Scouring Maw");
    assert!(g.keywords(w).has(Keywords::TRAMPLE));
    let ring = g.add_card(0, bid("Sol Ring"), Zone::Library); // mana value 1
    let plains = g.add_card(0, bid("Plains"), Zone::Library); // on top: skipped
    let m = hand(&mut g, 0, "Murmuration"); // mana value 5
    act(&mut g, Action::Cast(m));
    while !matches!(g.pending, Pending::FreeCast(_)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Cast(ring));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ring).zone, Zone::Battlefield, "cast free");
    assert_eq!(g.obj(m).zone, Zone::Battlefield);
    assert_eq!(
        g.players[0].library.first(),
        Some(&plains),
        "the rest on the bottom"
    );

    // Not from hand, or not an enchantment: no cascade.
    let mut g = table();
    put(&mut g, 0, "Wildsear, Scouring Maw");
    let ring = g.add_card(0, bid("Sol Ring"), Zone::Library);
    let elder = hand(&mut g, 0, "Sakura-Tribe Elder");
    play(&mut g, elder, false, &[], &[]);
    assert_eq!(g.obj(ring).zone, Zone::Library);
}

#[test]
fn casualties_of_war_destroys_one_of_each_chosen_type() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let ghalta = put(&mut g, 1, "Ghalta, Primal Hunger");
    let forest = put(&mut g, 1, "Forest");
    let c = hand(&mut g, 0, "Casualties of War");
    let modes = g.def_of(c).modes.len();
    assert_eq!(modes, 31, "every nonempty set of the five");
    // Mode set 0b01011: artifact, creature, land.
    act(&mut g, Action::CastModal(c, 0b01011 - 1));
    for x in [ring, ghalta, forest] {
        act(&mut g, Action::Target(Target::Object(x)));
    }
    resolve_all(&mut g, &[]);
    for x in [ring, ghalta, forest] {
        assert_eq!(g.obj(x).zone, Zone::Graveyard);
    }
}

#[test]
fn esikas_chariot_makes_cats_crews_and_copies_a_token() {
    let mut g = table();
    let chariot = hand(&mut g, 0, "Esika's Chariot");
    play(&mut g, chariot, false, &[], &[]);
    let cats: Vec<ObjId> = g.permanents(0).filter(|&x| g.def_of(x).name == "Cat").collect();
    assert_eq!(cats.len(), 2);
    assert!(!g.types_of(chariot).creature);
    g.objects[chariot.0 as usize].since_turn = 0; // as if it had been here since the turn began
    let ghalta = put(&mut g, 0, "Ghalta, Primal Hunger");
    act(&mut g, Action::ActivateWith(chariot, 0, ghalta));
    resolve_all(&mut g, &[]);
    assert!(g.types_of(chariot).creature);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(chariot));
    act(&mut g, Action::FinishAttacks);
    act(&mut g, Action::Target(Target::Object(cats[0])));
    resolve_all(&mut g, &[]);
    let n = g.permanents(0).filter(|&x| g.def_of(x).name == "Cat").count();
    assert_eq!(n, 3);
}

#[test]
fn chatterfang_adds_squirrels_and_sacrifices_x_of_them() {
    let mut g = table();
    let chatter = put(&mut g, 0, "Chatterfang, Squirrel General");
    let chariot = hand(&mut g, 0, "Esika's Chariot");
    play(&mut g, chariot, false, &[], &[]);
    let count = |g: &Game, name: &str| g.permanents(0).filter(|&x| g.def_of(x).name == name).count();
    assert_eq!(count(&g, "Cat"), 2);
    assert_eq!(count(&g, "Squirrel"), 2, "that many Squirrels");
    let ghalta = put(&mut g, 1, "Ghalta, Primal Hunger");
    let (p, t) = (g.power(ghalta), g.toughness(ghalta));
    let legal = g.legal_actions();
    assert!(
        legal.contains(&Action::ActivateX {
            id: chatter,
            n: 0,
            victim: None,
            x: 3
        }),
        "two tokens and Chatterfang"
    );
    assert!(!legal.contains(&Action::ActivateX {
        id: chatter,
        n: 0,
        victim: None,
        x: 4
    }));
    act(
        &mut g,
        Action::ActivateX {
            id: chatter,
            n: 0,
            victim: None,
            x: 2,
        },
    );
    act(&mut g, Action::Target(Target::Object(ghalta)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(ghalta), g.toughness(ghalta)), (p + 2, t - 2));
    assert_eq!(count(&g, "Squirrel"), 0, "the tokens went first");
    assert_eq!(g.obj(chatter).zone, Zone::Battlefield);
}

#[test]
fn baird_taxes_each_attacker_one() {
    let mut g = table();
    put(&mut g, 1, "Baird, Steward of Argive");
    assert_eq!(g.attack_tax(1, false), 1);
}

#[test]
fn coveted_jewel_changes_hands_when_its_controller_is_hit_unblocked() {
    let mut g = table();
    let jewel = put(&mut g, 1, "Coveted Jewel");
    g.objects[jewel.0 as usize].tapped = true;
    let ghalta = put(&mut g, 0, "Ghalta, Primal Hunger");
    g.objects[ghalta.0 as usize].since_turn = 0; // here since the turn began
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(ghalta));
    act(&mut g, Action::FinishAttacks);
    let n = g.players[0].hand.len();
    while g.obj(jewel).controller != 0 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[0].hand.len(), n + 3);
    assert!(!g.obj(jewel).tapped, "untapped");
}

#[test]
fn perch_protection_with_the_gift_phases_out_and_locks_life_past_the_extra_turn() {
    let mut g = table();
    let perch = hand(&mut g, 0, "Perch Protection");
    let n = g.permanents(0).count();
    act(&mut g, Action::CastKicked(perch));
    resolve_all(&mut g, &[]);
    assert_eq!(g.extra_turns, vec![1], "the opponent's extra turn");
    assert_eq!(g.phased_out.len(), n + 4, "everything, the four Birds too");
    assert_eq!(g.permanents(0).count(), 0);
    // Turn 3 now, then the opponent's extra turn 4 and their turn 5: player 0's next is turn 6.
    assert_eq!(g.life_locked_until[0], Some(g.turn + 3));
    assert_eq!(g.obj(perch).zone, Zone::Exile);

    // Without the gift: just the Birds.
    let mut g = table();
    let perch = hand(&mut g, 0, "Perch Protection");
    play(&mut g, perch, false, &[], &[]);
    let birds = g.permanents(0).filter(|&x| g.def_of(x).name == "Bird").count();
    assert_eq!(birds, 4);
    assert!(g.extra_turns.is_empty() && g.phased_out.is_empty());
}

#[test]
fn illusionists_gambit_removes_attackers_and_they_cant_attack_again() {
    let mut g = table();
    let ghalta = put(&mut g, 0, "Ghalta, Primal Hunger");
    let gambit = hand(&mut g, 1, "Illusionist's Gambit");
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    assert!(!g.legal_actions().contains(&Action::Cast(gambit)));
    act(&mut g, Action::Attack(ghalta));
    act(&mut g, Action::FinishAttacks);
    while g.pending != Pending::Priority(1) || g.step != Step::DeclareBlockers {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    act(&mut g, Action::Cast(gambit));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(ghalta).tapped, "untapped");
    assert!(
        g.combat.as_ref().is_none_or(|c| c.attackers.is_empty()),
        "removed from combat"
    );
    let life = g.players[1].life;
    while g.step != Step::Main2 {
        assert_ne!(
            g.pending,
            Pending::DeclareAttackers(0),
            "Ghalta can't attack the second combat"
        );
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[1].life, life);
}

#[test]
fn hazels_brewmaster_gives_foods_the_exiled_creatures_abilities() {
    let mut g = table();
    let elder = g.add_card(1, bid("Sakura-Tribe Elder"), Zone::Graveyard);
    let brew = hand(&mut g, 0, "Hazel's Brewmaster");
    play(&mut g, brew, false, &[], &[Action::Target(Target::Object(elder))]);
    assert_eq!(g.obj(elder).zone, Zone::Exile);
    let food = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Food")
        .expect("a Food");
    let own = g.def_of(food).activated.len();
    assert!(
        g.legal_actions().contains(&Action::Activate(food, own)),
        "the Elder's ability"
    );
    act(&mut g, Action::Activate(food, own));
    assert_ne!(g.obj(food).zone, Zone::Battlefield, "sacrificed the Food");
}

#[test]
fn calamity_of_cinders_hits_only_untapped_creatures() {
    let mut g = table();
    let up = put(&mut g, 1, "Sakura-Tribe Elder");
    let down = put(&mut g, 1, "Sakura-Tribe Elder");
    g.objects[down.0 as usize].tapped = true;
    let c = hand(&mut g, 0, "Calamity of Cinders");
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(up).zone, Zone::Graveyard);
    assert_eq!(g.obj(down).zone, Zone::Battlefield);
}

#[test]
fn realm_cloaked_giants_cast_off_spares_giants() {
    let mut g = table();
    let elder = put(&mut g, 1, "Sakura-Tribe Elder");
    let ghalta = put(&mut g, 0, "Ghalta, Primal Hunger");
    let giant = hand(&mut g, 0, "Realm-Cloaked Giant // Cast Off");
    act(&mut g, Action::CastOmen(giant));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elder).zone, Zone::Graveyard);
    assert_eq!(g.obj(ghalta).zone, Zone::Graveyard, "a Dinosaur, not a Giant");
    assert_eq!(g.obj(giant).zone, Zone::Exile, "on an adventure");
}

#[test]
fn swarmyard_regenerates_each_of_its_types() {
    let mut g = table();
    let yard = put(&mut g, 0, "Swarmyard");
    assert_eq!(g.def_of(yard).activated.len(), 4);
    let chatter = put(&mut g, 0, "Chatterfang, Squirrel General");
    act(&mut g, Action::Activate(yard, 3));
    act(&mut g, Action::Target(Target::Object(chatter)));
    resolve_all(&mut g, &[]);
    assert!(g.obj(yard).tapped);
}

#[test]
fn rolling_hamsphere_makes_hamsters_and_hits_for_each() {
    let mut g = table();
    let ham = put(&mut g, 0, "Rolling Hamsphere");
    let ghalta = put(&mut g, 0, "Ghalta, Primal Hunger");
    act(&mut g, Action::ActivateWith(ham, 0, ghalta));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(ham), 4);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(ham));
    act(&mut g, Action::FinishAttacks);
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    let hamsters = g.permanents(0).filter(|&x| g.def_of(x).name == "Hamster").count();
    assert_eq!(hamsters, 3);
    assert_eq!(g.players[1].life, 40 - 3);
    assert_eq!(g.power(ham), 7, "+1/+1 for each Hamster");
}

#[test]
fn pyreswipe_hawk_grows_by_the_greatest_artifact() {
    let mut g = table();
    let hawk = put(&mut g, 0, "Pyreswipe Hawk");
    put(&mut g, 0, "Coveted Jewel"); // mana value 6
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(hawk));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(hawk), 4 + 6);
}

#[test]
fn martial_impetus_goads_and_pumps() {
    let mut g = table();
    let elder = put(&mut g, 1, "Sakura-Tribe Elder");
    let aura = hand(&mut g, 0, "Martial Impetus");
    play(&mut g, aura, false, &[Target::Object(elder)], &[]);
    assert_eq!((g.power(elder), g.toughness(elder)), (2, 2));
    assert!(g.is_goaded(elder));
}

#[test]
fn bootleggers_stash_gives_lands_a_treasure_ability() {
    let mut g = table();
    put(&mut g, 0, "Bootleggers' Stash");
    let forest = g.permanents(0).find(|&x| g.def_of(x).name == "Forest").unwrap();
    assert!(g.legal_actions().contains(&Action::Activate(forest, 0)));
    act(&mut g, Action::Activate(forest, 0));
    resolve_all(&mut g, &[]);
    let treasures = g
        .permanents(0)
        .filter(|&x| g.def_of(x).name == "Treasure")
        .count();
    assert_eq!(treasures, 1);
}

#[test]
fn the_odd_acorn_gang_gives_squirrels_a_pump() {
    let mut g = table();
    put(&mut g, 0, "The Odd Acorn Gang");
    let chatter = put(&mut g, 0, "Chatterfang, Squirrel General");
    let own = g.def_of(chatter).activated.len();
    act(&mut g, Action::Activate(chatter, own));
    act(&mut g, Action::Target(Target::Object(chatter)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(chatter), 5);
    assert!(g.keywords(chatter).has(Keywords::TRAMPLE));
}

#[test]
fn kwain_lets_each_player_draw_and_gain() {
    let mut g = table();
    let kwain = put(&mut g, 0, "Kwain, Itinerant Meddler");
    let (h0, h1) = (g.players[0].hand.len(), g.players[1].hand.len());
    act(&mut g, Action::Activate(kwain, 0));
    resolve_all(&mut g, &[Action::Yes, Action::No]);
    assert_eq!((g.players[0].hand.len(), g.players[0].life), (h0 + 1, 41));
    assert_eq!(
        (g.players[1].hand.len(), g.players[1].life),
        (h1, 40),
        "the opponent declined"
    );
}

#[test]
fn mr_foxglove_draws_the_difference() {
    let mut g = table();
    let fox = put(&mut g, 0, "Mr. Foxglove");
    g.objects[fox.0 as usize].since_turn = 0;
    for _ in 0..3 {
        g.add_card(1, bid("Plains"), Zone::Hand);
    }
    let diff = g.players[1].hand.len() - g.players[0].hand.len();
    let n = g.players[0].hand.len();
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(fox));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n + diff);
}

#[test]
fn dusk_then_dawn_from_the_graveyard() {
    let mut g = table();
    let ghalta = put(&mut g, 1, "Ghalta, Primal Hunger");
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    let dd = hand(&mut g, 0, "Dusk // Dawn");
    assert!(
        !g.legal_actions().contains(&Action::CastOmen(dd)),
        "Dawn: only from the graveyard"
    );
    play(&mut g, dd, false, &[], &[]);
    assert_eq!(g.obj(ghalta).zone, Zone::Graveyard);
    assert_eq!(g.obj(elder).zone, Zone::Battlefield, "power 1");
    assert_eq!(g.obj(dd).zone, Zone::Graveyard);
    assert!(
        !g.legal_actions().contains(&Action::Cast(dd)),
        "Dusk isn't cast from the graveyard"
    );
    let small = g.add_card(0, bid("Sakura-Tribe Elder"), Zone::Graveyard);
    for _ in 0..2 {
        put(&mut g, 0, "Plains"); // Dusk's {W}{W} may have used most of them
    }
    act(&mut g, Action::CastOmen(dd));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(small).zone, Zone::Hand);
    assert_eq!(g.obj(dd).zone, Zone::Exile, "aftermath: then exile it");
}

#[test]
fn wizard_class_levels_up_in_order() {
    let mut g = table();
    let class = put(&mut g, 0, "Wizard Class");
    assert_eq!(g.class_level(class), 1);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Activate(class, 0)), "level 2");
    assert!(
        !legal.contains(&Action::Activate(class, 1)),
        "not level 3 before level 2"
    );
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(class, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.class_level(class), 2);
    assert_eq!(g.players[0].hand.len(), n + 2, "When this Class becomes level 2");
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Activate(class, 0)));
    assert!(legal.contains(&Action::Activate(class, 1)));
}

#[test]
fn wizard_class_level_three_trigger_waits_for_level_three() {
    let mut g = table();
    let class = put(&mut g, 0, "Wizard Class");
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    let kwain = put(&mut g, 0, "Kwain, Itinerant Meddler");
    act(&mut g, Action::Activate(kwain, 0));
    resolve_all(&mut g, &[Action::Yes, Action::No]);
    assert_eq!(g.power(elder), 1, "level 1: no counter for the draw");
    g.objects[class.0 as usize].class_level = 3;
    g.objects[kwain.0 as usize].tapped = false;
    act(&mut g, Action::Activate(kwain, 0));
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::Decide(0) => Action::Yes,
            Pending::Decide(_) => Action::No,
            Pending::ChooseTarget(_) => Action::Target(Target::Object(elder)),
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.power(elder), 2, "level 3: a counter");
}

#[test]
fn cache_grab_takes_a_permanent_and_a_squirrel_makes_food() {
    let mut g = table();
    let elder = g.add_card(0, bid("Sakura-Tribe Elder"), Zone::Library);
    for _ in 0..3 {
        g.add_card(0, bid("Plains"), Zone::Library);
    }
    put(&mut g, 0, "Chatterfang, Squirrel General");
    let c = hand(&mut g, 0, "Cache Grab");
    play(&mut g, c, false, &[], &[Action::Pick(elder)]);
    assert_eq!(g.obj(elder).zone, Zone::Hand);
    let food = g.permanents(0).filter(|&x| g.def_of(x).name == "Food").count();
    assert_eq!(
        food, 1,
        "Chatterfang is a Squirrel (and adds a Squirrel token to the Food)"
    );
}

#[test]
fn forgotten_ancient_grows_and_moves_its_counters() {
    let mut g = table();
    let fa = put(&mut g, 0, "Forgotten Ancient");
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    let ring = hand(&mut g, 0, "Sol Ring");
    act(&mut g, Action::Cast(ring));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.obj(fa).counters, 1);
    g.objects[fa.0 as usize].counters = 2;
    g.start_turn(5, 0);
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Pick(elder));
    act(&mut g, Action::No);
    assert_eq!((g.obj(fa).counters, g.obj(elder).counters), (1, 1));
}

#[test]
fn shield_broker_steals_while_the_shield_lasts() {
    let mut g = table();
    let ghalta = put(&mut g, 1, "Ghalta, Primal Hunger");
    let broker = hand(&mut g, 0, "Shield Broker");
    play(&mut g, broker, false, &[], &[Action::Target(Target::Object(ghalta))]);
    assert_eq!(g.obj(ghalta).controller, 0);
    assert_eq!(g.obj(ghalta).shield_counters, 1);
    // Damage removes the shield instead, and control goes back.
    let c = hand(&mut g, 0, "Calamity of Cinders");
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(ghalta).damage, 0, "the shield took it");
    assert_eq!(g.obj(ghalta).shield_counters, 0);
    assert_eq!(g.obj(ghalta).controller, 1);
}
