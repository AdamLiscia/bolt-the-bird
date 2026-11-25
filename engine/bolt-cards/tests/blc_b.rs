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
