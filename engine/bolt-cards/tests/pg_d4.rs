//! Lane D4 (the desktop): each script's behavior in a 1v1 Commander game on the CMD pool.
#![allow(dead_code, unused_imports)]

mod common;

use bolt_cards::set;
use bolt_core::{Action, Game, Keywords, ObjId, Pending, Step, Target, Zone};
use common::{act, obj, play, resolve_all};

fn cid(name: &str) -> usize {
    set("CMD")
        .unwrap()
        .index_of(name)
        .unwrap_or_else(|| panic!("{name} is not in the CMD pool"))
}

/// A Commander game on the CMD pool (99 Plains each), Bello vs. Ms. Bumbleflower, at player 0's first
/// main phase on turn 3, with 5 of each basic for both players.
fn table() -> Game {
    let plains = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        set("CMD").unwrap().defs.clone(),
        [&plains, &plains],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    for p in 0..2 {
        for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            for _ in 0..5 {
                put(&mut g, p, land);
            }
        }
    }
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g
}

fn put(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, cid(name), Zone::Battlefield)
}

fn hand(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, cid(name), Zone::Hand)
}

fn lands(g: &Game, p: usize) -> usize {
    g.permanents(p).filter(|&x| g.def_of(x).types.land).count()
}

// ---- Split cards ----

#[test]
fn split_cards_are_supported() {
    let s = set("CMD").unwrap();
    for name in ["Boom // Bust", "Fire // Ice"] {
        let i = cid(name);
        assert!(s.supported[i], "{name}");
        assert!(s.defs[i].subsystems.split_card, "{name}");
    }
}

#[test]
fn fire_divides_two_damage_between_two_creatures() {
    let mut g = table();
    let a = put(&mut g, 1, "Llanowar Elves");
    let b = put(&mut g, 1, "Llanowar Elves");
    let c = hand(&mut g, 0, "Fire // Ice");
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(a)));
    act(&mut g, Action::Target(obj(b)));
    assert!(matches!(g.pending, Pending::Divide(0)));
    act(&mut g, Action::Amount(1));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.obj(a).zone, g.obj(b).zone),
        (Zone::Graveyard, Zone::Graveyard),
        "1 damage to each 1/1"
    );
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
}

#[test]
fn ice_taps_a_permanent_and_draws_then_goes_to_the_graveyard() {
    let mut g = table();
    let theirs = g.permanents(1).next().unwrap();
    let c = hand(&mut g, 0, "Fire // Ice");
    assert!(g.legal_actions().contains(&Action::CastOmen(c)));
    let n = g.players[0].hand.len();
    act(&mut g, Action::CastOmen(c));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert!(g.obj(theirs).tapped);
    assert_eq!(g.players[0].hand.len(), n, "cast it, drew one");
    assert_eq!(
        g.obj(c).zone,
        Zone::Graveyard,
        "a split card goes to the graveyard, not the library"
    );
}

#[test]
fn boom_destroys_a_land_of_each_player() {
    let mut g = table();
    let mine = g.permanents(0).next().unwrap();
    let theirs = g.permanents(1).next().unwrap();
    let c = hand(&mut g, 0, "Boom // Bust");
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(mine)));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(mine).zone, Zone::Graveyard);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!((lands(&g, 0), lands(&g, 1)), (24, 24));
}

#[test]
fn bust_destroys_all_lands_for_its_own_cost() {
    let mut g = table();
    let c = hand(&mut g, 0, "Boom // Bust");
    act(&mut g, Action::CastOmen(c));
    resolve_all(&mut g, &[]);
    assert_eq!((lands(&g, 0), lands(&g, 1)), (0, 0));
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
    assert_eq!(
        g.def_of(c).omen.as_ref().unwrap().cost.mana_value(),
        6,
        "{{5}}{{R}}"
    );
}

#[test]
fn a_split_card_has_both_halves_mana_value_off_the_stack_and_pays_only_its_half() {
    let mut g = table();
    let c = hand(&mut g, 0, "Fire // Ice");
    assert_eq!(
        g.def_of(c).cost.mana_value(),
        4,
        "{{1}}{{R}} + {{1}}{{U}} (CR 709.4)"
    );
    assert_eq!(
        g.colors_of(c),
        [false, true, false, true, false],
        "both halves' colors in hand"
    );
    let untapped = |g: &Game| g.permanents(0).filter(|&x| !g.obj(x).tapped).count();
    let before = untapped(&g);
    act(&mut g, Action::CastOmen(c));
    let theirs = g.permanents(1).next().unwrap();
    act(&mut g, Action::Target(obj(theirs)));
    assert_eq!(
        g.colors_of(c),
        [false, true, false, false, false],
        "Ice on the stack is blue"
    );
    assert_eq!(before - untapped(&g), 2, "Ice's {{1}}{{U}}");
    resolve_all(&mut g, &[]);
    let c2 = hand(&mut g, 0, "Fire // Ice");
    let before = untapped(&g);
    act(&mut g, Action::Cast(c2));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::No); // one target
    assert_eq!(
        g.colors_of(c2),
        [false, false, false, true, false],
        "Fire on the stack is red"
    );
    assert_eq!(before - untapped(&g), 2, "Fire's {{1}}{{R}}, not both halves");
    resolve_all(&mut g, &[]);
}

#[test]
fn a_split_cards_right_half_is_cast_only_from_hand() {
    let mut g = table();
    let c = g.add_card(0, cid("Fire // Ice"), Zone::Graveyard);
    assert!(!g.legal_actions().contains(&Action::CastOmen(c)));
}

// ---- Dice ----

#[test]
fn ancient_gold_dragon_rolls_a_d20_for_faerie_dragons() {
    let mut g = table();
    assert!(set("CMD").unwrap().supported[cid("Ancient Gold Dragon")]);
    let d = put(&mut g, 0, "Ancient Gold Dragon");
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(d));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    let faeries: Vec<ObjId> = g
        .permanents(0)
        .filter(|&x| g.is_token(x) && g.def_of(x).name == "Faerie Dragon")
        .collect();
    assert!((1..=20).contains(&faeries.len()), "{}", faeries.len());
    let f = faeries[0];
    assert_eq!((g.power(f), g.toughness(f)), (1, 1));
    assert!(g.keywords(f).has(Keywords::FLYING));
    assert!(g.has_subtype(f, "Dragon"), "a Faerie Dragon is a Dragon");
    assert_eq!(g.colors_of(f), [false, true, false, false, false], "blue");
}

#[test]
fn ancient_copper_dragon_rolls_a_d20_for_treasures() {
    supported("Ancient Copper Dragon");
    let mut g = table();
    let d = put(&mut g, 0, "Ancient Copper Dragon");
    advance(&mut g, |g| g.pending == Pending::DeclareAttackers(0));
    act(&mut g, Action::Attack(d));
    act(&mut g, Action::FinishAttacks);
    advance(&mut g, |g| g.step == Step::Main2);
    let n = g
        .permanents(0)
        .filter(|&x| g.is_token(x) && g.def_of(x).name == "Treasure")
        .count();
    assert!((1..=20).contains(&n), "{n}");
}

// ---- Coin flips ----

#[test]
fn mana_crypt_taps_for_two_and_sometimes_deals_three_at_your_upkeep() {
    supported("Mana Crypt");
    let mut g = table();
    let c = put(&mut g, 0, "Mana Crypt");
    assert_eq!(
        g.mana_sources(0).iter().filter(|s| s.0 == c).count(),
        2,
        "{{C}}{{C}}"
    );
    let mut hurt = [false; 2];
    for turn in 0..20 {
        let life = g.players[0].life;
        g.start_turn(5 + 2 * turn, 0);
        advance(&mut g, |g| g.step == Step::Main1);
        let lost = life - g.players[0].life;
        assert!(lost == 0 || lost == 3, "{lost}");
        hurt[(lost == 3) as usize] = true;
    }
    assert_eq!(hurt, [true, true], "won some flips and lost some");
}

// ---- Dungeons ----

#[test]
fn hama_pashar_doubles_your_room_abilities() {
    let mut g = table();
    assert!(set("CMD").unwrap().supported[cid("Hama Pashar, Ruin Seeker")]);
    assert_eq!(g.room_trigger_count(0), 1);
    put(&mut g, 0, "Hama Pashar, Ruin Seeker");
    assert_eq!((g.room_trigger_count(0), g.room_trigger_count(1)), (2, 1));
}

#[test]
fn dungeon_delver_doubles_them_through_your_commander_creature() {
    let mut g = table();
    assert!(set("CMD").unwrap().supported[cid("Dungeon Delver")]);
    put(&mut g, 0, "Dungeon Delver");
    assert_eq!(g.room_trigger_count(0), 1);
    // A commander creature you own on the battlefield.
    g.commanders[0] = Some(put(&mut g, 0, "Bello, Bard of the Brambles"));
    assert_eq!(g.room_trigger_count(0), 2);
}

/// Passes (no attacks, no blocks, first legal answer to decisions) until `done`.
fn advance(g: &mut Game, done: impl Fn(&Game) -> bool) {
    while !done(g) {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::Priority(_) => Action::PassPriority,
            // Decisions, targets, cleanup discards.
            _ => *g
                .legal_actions()
                .first()
                .unwrap_or_else(|| panic!("{:?}", g.pending)),
        };
        act(g, a);
    }
}

fn supported(name: &str) {
    assert!(set("CMD").unwrap().supported[cid(name)], "{name}");
}

#[test]
fn radiant_solar_ventures_for_each_nontoken_creature_and_from_hand() {
    supported("Radiant Solar");
    let mut g = table();
    put(&mut g, 0, "Radiant Solar");
    let elf = hand(&mut g, 0, "Llanowar Elves");
    act(&mut g, Action::Cast(elf)); // first answers: Lost Mine, Cave Entrance (scry 1)
    advance(&mut g, |g| {
        g.items.is_empty() && matches!(g.pending, Pending::Priority(_))
    });
    assert_eq!(g.venture[0], Some((bolt_core::Dungeon::LostMine, 0)));
    let s = hand(&mut g, 0, "Radiant Solar");
    let life = g.players[0].life;
    act(&mut g, Action::Activate(s, 0));
    advance(&mut g, |g| {
        g.items.is_empty() && matches!(g.pending, Pending::Priority(_))
    });
    assert_eq!(g.obj(s).zone, Zone::Graveyard, "discarded");
    assert_eq!(g.players[0].life, life + 3);
    assert_eq!(
        g.venture[0].map(|v| v.1),
        Some(1),
        "Goblin Lair (the first way on)"
    );
}

#[test]
fn nadaar_ventures_and_pumps_the_others_after_a_dungeon() {
    supported("Nadaar, Selfless Paladin");
    let mut g = table();
    let n = hand(&mut g, 0, "Nadaar, Selfless Paladin");
    play(&mut g, n, false, &[], &[Action::Mode(2)]); // Tomb of Annihilation: Trapped Entry
    assert_eq!(g.venture[0], Some((bolt_core::Dungeon::Tomb, 0)));
    let elf = put(&mut g, 0, "Llanowar Elves");
    assert_eq!(g.power(elf), 1);
    g.completed_dungeons[0] = 1;
    assert_eq!((g.power(elf), g.toughness(elf)), (2, 2));
    assert_eq!(g.power(n), 3, "other creatures");
}

/// Attacks with `attackers`; returns the defending player's legal block actions, then finishes
/// combat (no blocks) and resolves to the second main phase.
fn attack(g: &mut Game, attackers: &[ObjId]) -> Vec<Action> {
    advance(g, |g| g.pending == Pending::DeclareAttackers(0));
    for &a in attackers {
        act(g, Action::Attack(a));
    }
    act(g, Action::FinishAttacks);
    // With no legal block, the engine skips the declaration.
    advance(g, |g| {
        matches!(g.pending, Pending::DeclareBlockers(_)) || g.step == Step::Main2
    });
    let blocks = match g.pending {
        Pending::DeclareBlockers(_) => g.legal_actions(),
        _ => vec![Action::FinishBlocks],
    };
    advance(g, |g| g.step == Step::Main2);
    blocks
}

#[test]
fn yuan_ti_malison_is_unblockable_alone_and_ventures_on_damage() {
    supported("Yuan-Ti Malison");
    let mut g = table();
    let y = put(&mut g, 0, "Yuan-Ti Malison");
    put(&mut g, 1, "Llanowar Elves");
    assert_eq!(attack(&mut g, &[y]), [Action::FinishBlocks], "alone: no blocks");
    assert!(g.venture[0].is_some());
    g.start_turn(5, 0);
    let other = put(&mut g, 0, "Savannah Lions");
    assert!(attack(&mut g, &[y, other]).len() > 1, "not alone: blockable");
}

#[test]
fn midnight_pathlighter_lets_only_legends_block_and_ventures_on_damage() {
    supported("Midnight Pathlighter");
    let mut g = table();
    let m = put(&mut g, 0, "Midnight Pathlighter");
    put(&mut g, 1, "Llanowar Elves");
    assert_eq!(
        attack(&mut g, &[m]),
        [Action::FinishBlocks],
        "no legendary blocker"
    );
    assert!(g.venture[0].is_some());
    g.start_turn(5, 0);
    put(&mut g, 1, "Imoen, Mystic Trickster");
    assert!(attack(&mut g, &[m]).len() > 1, "a legendary creature can block");
}

#[test]
fn dungeon_descent_taps_a_legend_to_venture() {
    supported("Dungeon Descent");
    let mut g = table();
    let d = put(&mut g, 0, "Dungeon Descent");
    assert!(
        !g.legal_actions().contains(&Action::Activate(d, 0)),
        "no legendary creature"
    );
    let i = put(&mut g, 0, "Imoen, Mystic Trickster");
    act(&mut g, Action::Activate(d, 0));
    advance(&mut g, |g| {
        g.items.is_empty() && matches!(g.pending, Pending::Priority(_))
    });
    assert!(g.obj(d).tapped && g.obj(i).tapped);
    assert!(g.venture[0].is_some());
}

// ---- The initiative ----

#[test]
fn white_plume_adventurer_takes_the_initiative_and_untaps_at_their_upkeep() {
    supported("White Plume Adventurer");
    let mut g = table();
    let w = hand(&mut g, 0, "White Plume Adventurer");
    act(&mut g, Action::Cast(w));
    // Secret Entrance: the first basic land card found.
    advance(&mut g, |g| {
        g.items.is_empty() && !matches!(g.pending, Pending::Decide(_) | Pending::ChooseTarget(_))
    });
    assert_eq!(g.initiative.map(|i| i.0), Some(0));
    assert_eq!(g.venture[0], Some((bolt_core::Dungeon::Undercity, 0)));
    let elf = put(&mut g, 0, "Llanowar Elves");
    g.objects[elf.0 as usize].tapped = true;
    g.start_turn(4, 1);
    advance(&mut g, |g| g.step == Step::Main1);
    assert!(!g.obj(elf).tapped, "untapped at the opponent's upkeep");
    g.objects[elf.0 as usize].tapped = true;
    g.objects[w.0 as usize].tapped = true;
    g.completed_dungeons[0] = 1;
    g.start_turn(6, 1);
    advance(&mut g, |g| g.step == Step::Main1);
    assert!(!g.obj(elf).tapped && !g.obj(w).tapped, "all of them");
}

#[test]
fn imoen_draws_at_your_end_step_with_the_initiative_and_more_for_a_dungeon() {
    supported("Imoen, Mystic Trickster");
    let mut g = table();
    let i = put(&mut g, 0, "Imoen, Mystic Trickster");
    let lib = g.players[0].library.len();
    advance(&mut g, |g| g.turn == 4);
    assert_eq!(g.players[0].library.len(), lib, "no initiative");
    g.initiative = Some((0, i));
    g.completed_dungeons[0] = 1;
    g.start_turn(5, 0);
    advance(&mut g, |g| g.step == Step::Main1);
    let lib = g.players[0].library.len();
    advance(&mut g, |g| g.turn == 6);
    assert_eq!(g.players[0].library.len(), lib - 2);
}

#[test]
fn safana_makes_a_treasure_at_your_end_step_with_the_initiative() {
    supported("Safana, Calimport Cutthroat");
    let mut g = table();
    let s = put(&mut g, 0, "Safana, Calimport Cutthroat");
    g.initiative = Some((0, s));
    let treasures = |g: &Game| {
        g.permanents(0)
            .filter(|&x| g.is_token(x) && g.def_of(x).name == "Treasure")
            .count()
    };
    advance(&mut g, |g| g.turn == 4);
    assert_eq!(treasures(&g), 1);
    g.completed_dungeons[0] = 1;
    g.start_turn(5, 0);
    advance(&mut g, |g| g.turn == 6);
    assert_eq!(treasures(&g), 4, "three more");
}

// ---- Emblems ----

#[test]
fn lolth_gains_loyalty_makes_spiders_and_her_emblem_tops_up_to_eight() {
    supported("Lolth, Spider Queen");
    let mut g = table();
    let l = hand(&mut g, 0, "Lolth, Spider Queen");
    play(&mut g, l, false, &[], &[]);
    assert_eq!(g.obj(l).loyalty, 4);
    act(&mut g, Action::Loyalty(l, 1)); // −3
    resolve_all(&mut g, &[]);
    let spiders: Vec<ObjId> = g
        .permanents(0)
        .filter(|&x| g.is_token(x) && g.def_of(x).name == "Spider")
        .collect();
    assert_eq!(spiders.len(), 2);
    assert_eq!((g.power(spiders[0]), g.toughness(spiders[0])), (2, 1));
    assert!(g.keywords(spiders[0]).has(Keywords::REACH.with(Keywords::MENACE)));
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, bolt, false, &[obj(spiders[0])], &[]);
    assert_eq!(g.obj(l).loyalty, 2, "a creature you control died");
    g.start_turn(5, 0);
    advance(&mut g, |g| g.step == Step::Main1);
    g.objects[l.0 as usize].loyalty = 8;
    act(&mut g, Action::Loyalty(l, 2)); // −8
    resolve_all(&mut g, &[]);
    assert_eq!(g.emblems.len(), 1);
    let them = g.players[1].life;
    advance(&mut g, |g| g.pending == Pending::DeclareAttackers(0));
    act(&mut g, Action::Attack(spiders[1]));
    act(&mut g, Action::FinishAttacks);
    advance(&mut g, |g| g.step == Step::Main2);
    assert_eq!(g.players[1].life, them - 8, "2 combat damage, then 6 more");
}

#[test]
fn ob_nixilis_reignited_emblem_drains_its_owner_on_every_draw() {
    supported("Ob Nixilis Reignited");
    let mut g = table();
    let o = hand(&mut g, 0, "Ob Nixilis Reignited");
    play(&mut g, o, false, &[], &[]);
    g.objects[o.0 as usize].loyalty = 8;
    act(&mut g, Action::Loyalty(o, 2)); // −8
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    let them = g.players[1].life;
    g.start_turn(4, 1);
    advance(&mut g, |g| g.step == Step::Main1);
    assert_eq!(g.players[1].life, them - 2, "they drew");
    g.start_turn(5, 0);
    advance(&mut g, |g| g.step == Step::Main1);
    assert_eq!(g.players[1].life, them - 4, "you drew");
}

#[test]
fn professor_dellian_fel_emblem_drains_for_each_life_gain() {
    supported("Professor Dellian Fel");
    let mut g = table();
    let p = hand(&mut g, 0, "Professor Dellian Fel");
    play(&mut g, p, false, &[], &[]);
    g.objects[p.0 as usize].loyalty = 7;
    act(&mut g, Action::Loyalty(p, 3)); // −6
    resolve_all(&mut g, &[]);
    assert_eq!(g.emblems.len(), 1);
    g.start_turn(5, 0);
    advance(&mut g, |g| g.step == Step::Main1);
    let (me, them) = (g.players[0].life, g.players[1].life);
    act(&mut g, Action::Loyalty(p, 0)); // +2: gain 3
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]); // the emblem's target opponent
    assert_eq!((g.players[0].life, g.players[1].life), (me + 3, them - 3));
}

// ---- Choose a Background ----

#[test]
fn a_background_is_a_legal_second_commander_for_a_creature_that_chooses_one() {
    let s = set("CMD").unwrap();
    let deck = vec![cid("Swamp"); 98];
    let problems = bolt_cards::commander_deck_problems_for(
        s,
        &[cid("Safana, Calimport Cutthroat"), cid("Dungeon Delver")],
        &deck,
    );
    assert!(problems.is_empty(), "{problems:?}");
}

#[test]
fn a_background_needs_a_commander_that_chooses_one() {
    let s = set("CMD").unwrap();
    let deck = vec![cid("Island"); 98];
    let problems = bolt_cards::commander_deck_problems_for(
        s,
        &[cid("Hama Pashar, Ruin Seeker"), cid("Dungeon Delver")],
        &deck,
    );
    assert!(
        problems.iter().any(|p| p.contains("isn't a legendary creature")),
        "{problems:?}"
    );
    assert!(
        problems.iter().any(|p| p.contains("aren't partners")),
        "{problems:?}"
    );
}

// ---- The Ring ----

#[test]
fn fiery_inscription_tempts_you_and_burns_on_each_instant_or_sorcery() {
    supported("Fiery Inscription");
    let mut g = table();
    let elf = put(&mut g, 0, "Llanowar Elves");
    let f = hand(&mut g, 0, "Fiery Inscription");
    play(&mut g, f, false, &[], &[]);
    assert_eq!(g.obj(f).zone, Zone::Battlefield);
    assert_eq!(
        (g.ring_level(0), g.ring_bearer(0)),
        (1, Some(elf)),
        "the Ring tempted you"
    );
    let life = g.players[1].life;
    let fire = hand(&mut g, 0, "Fire // Ice");
    act(&mut g, Action::CastOmen(fire)); // Ice, an instant
    act(&mut g, Action::Target(obj(elf)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 2);
    let lions = hand(&mut g, 0, "Savannah Lions");
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(g.players[1].life, life - 2, "not for a creature spell");
}

#[test]
fn barrowin_returns_a_small_creature_card_when_attacking_after_a_completed_dungeon() {
    supported("Barrowin of Clan Undurr");
    let mut g = table();
    let b = put(&mut g, 0, "Barrowin of Clan Undurr");
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Graveyard);
    let dragon = g.add_card(0, cid("Ancient Gold Dragon"), Zone::Graveyard);
    attack(&mut g, &[b]);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard, "no completed dungeon yet");
    g.completed_dungeons[0] = 1;
    g.start_turn(5, 0);
    advance(&mut g, |g| g.pending == Pending::DeclareAttackers(0));
    act(&mut g, Action::Attack(b));
    act(&mut g, Action::FinishAttacks);
    let mut offered = Vec::new();
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::Decide(_) => {
                offered = g.legal_actions();
                Action::Pick(lions)
            }
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::Priority(_) => Action::PassPriority,
            _ => g.legal_actions()[0],
        };
        act(&mut g, a);
    }
    assert!(offered.contains(&Action::Pick(lions)), "{offered:?}");
    assert!(!offered.contains(&Action::Pick(dragon)), "mana value 3 or less");
    assert_eq!(g.obj(lions).zone, Zone::Battlefield);
    assert_eq!(g.obj(dragon).zone, Zone::Graveyard);
}

#[test]
fn smaug_is_token_95_a_legendary_flying_haste_dragon_that_dies_into_fourteen_treasures() {
    let s = set("CMD").unwrap();
    let token = |name: &str| s.defs.iter().position(|d| d.token && d.name == name).unwrap();
    let smaug = token("Smaug");
    assert_eq!(smaug, token("Hamster") + 5, "slot 95: Hamster is 90");
    let d = &s.defs[smaug];
    assert!(d.types.legendary && d.types.creature);
    assert_eq!((d.power, d.toughness), (6, 6));
    assert!(d.keywords.has(Keywords::FLYING) && d.keywords.has(Keywords::HASTE));
    let mut g = table();
    let x = g.add_card(0, smaug, Zone::Battlefield);
    let treasures = |g: &Game| {
        g.permanents(0)
            .filter(|&t| g.def_of(t).name == "Treasure")
            .count()
    };
    let before = treasures(&g);
    g.objects[x.0 as usize].damage = 6; // lethal damage: it dies to state-based actions
    act(&mut g, Action::PassPriority);
    resolve_all(&mut g, &[]);
    assert_ne!(g.obj(x).zone, Zone::Battlefield);
    assert_eq!(treasures(&g), before + 14);
}

// ---- Lane B's effects entries ----

#[test]
fn echoing_assault_makes_a_one_one_attacking_copy_sacrificed_at_end_step() {
    supported("Echoing Assault");
    let mut g = table();
    put(&mut g, 0, "Echoing Assault");
    let lions = put(&mut g, 0, "Savannah Lions");
    advance(&mut g, |g| g.pending == Pending::DeclareAttackers(0));
    act(&mut g, Action::Attack(lions));
    act(&mut g, Action::FinishAttacks);
    // The one nontoken attacker is the trigger's only target.
    advance(&mut g, |g| {
        g.permanents(0).any(|c| g.is_token(c)) && matches!(g.pending, Pending::Priority(_))
    });
    let token = g.permanents(0).find(|&c| g.is_token(c)).expect("the copy");
    assert_eq!(g.def_of(token).name, "Savannah Lions");
    assert!(g.obj(token).tapped, "enters tapped");
    assert_eq!((g.power(token), g.toughness(token)), (1, 1), "except it's 1/1");
    assert!(
        g.combat.as_ref().unwrap().attackers.contains(&token),
        "and attacking"
    );
    assert!(
        g.keywords(token).has(Keywords::MENACE),
        "creature tokens have menace"
    );
    assert!(!g.keywords(lions).has(Keywords::MENACE), "not nontoken ones");
    let life = g.players[1].life;
    advance(&mut g, |g| g.step == Step::Main2);
    assert_eq!(g.players[1].life, life - 3, "2 + 1 combat damage");
    advance(&mut g, |g| g.step == Step::Cleanup || g.active == 1);
    assert_ne!(g.obj(token).zone, Zone::Battlefield, "sacrificed at the end step");
}

#[test]
fn commanders_plate_protects_from_colors_outside_the_commanders_identity() {
    supported("Commander's Plate");
    let mut g = table(); // player 0's commander: Bello (red and green)
    let plate = put(&mut g, 0, "Commander's Plate");
    let elf = put(&mut g, 0, "Llanowar Elves");
    assert!(
        !g.legal_actions().contains(&Action::Activate(plate, 0)),
        "equip commander: no commander on the battlefield"
    );
    act(&mut g, Action::Activate(plate, 1)); // Equip {5}
    advance(&mut g, |g| {
        g.items.is_empty() && matches!(g.pending, Pending::Priority(_))
    });
    assert_eq!(g.obj(plate).attached_to.map(|a| a.0), Some(elf));
    assert_eq!((g.power(elf), g.toughness(elf)), (4, 4));
    assert_eq!(g.protection(elf), [true, true, true, false, false], "W, U and B");
    // With no commander, every color.
    g.commanders[0] = None;
    assert_eq!(g.protection(elf), [true; 5]);
}

#[test]
fn commanders_plate_equips_a_commander_for_three() {
    let mut g = table();
    let plate = put(&mut g, 0, "Commander's Plate");
    put(&mut g, 0, "Llanowar Elves");
    let bello = put(&mut g, 0, "Bello, Bard of the Brambles");
    g.commanders[0] = Some(bello);
    let untapped = g.permanents(0).filter(|&x| !g.obj(x).tapped).count();
    act(&mut g, Action::Activate(plate, 0));
    advance(&mut g, |g| {
        g.items.is_empty() && matches!(g.pending, Pending::Priority(_))
    });
    assert_eq!(g.obj(plate).attached_to.map(|a| a.0), Some(bello));
    assert_eq!(
        g.permanents(0).filter(|&x| !g.obj(x).tapped).count(),
        untapped - 3,
        "three lands for {{3}}"
    );
    assert_eq!(g.protection(bello), [true, true, true, false, false]);
}
