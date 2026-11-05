//! Tier-3 cards: one test per card, written from its rules text.

mod common;

use bolt_core::{Action, Keywords, Pending, Step, Zone};
use common::*;

#[test]
fn gain_lands_enter_tapped_gain_one_and_make_two_colors() {
    let gain_lands = [
        ("Bloodfell Caves", 2, 3),
        ("Blossoming Sands", 4, 0),
        ("Dismal Backwater", 1, 2),
        ("Jungle Hollow", 2, 4),
        ("Rugged Highlands", 3, 4),
        ("Scoured Barrens", 0, 2),
        ("Swiftwater Cliffs", 1, 3),
        ("Thornwood Falls", 4, 1),
        ("Tranquil Cove", 0, 1),
        ("Wind-Scarred Crag", 3, 0),
    ];
    for (name, a, b) in gain_lands {
        let mut g = lands(&[]);
        let land = hand(&mut g, 0, name);
        begin(&mut g);
        act(&mut g, Action::PlayLand(land));
        resolve_all(&mut g, &[]);
        assert!(g.obj(land).tapped, "{name} enters tapped");
        assert_eq!(g.players[0].life, 21, "{name} gains 1");
        let m = g.def_of(land).mana.expect("a mana source");
        assert!(
            m.colors[a] && m.colors[b] && m.colors.iter().filter(|&&c| c).count() == 2,
            "{name}"
        );
    }
}

#[test]
fn evolving_wilds_fetches_a_tapped_basic() {
    let mut g = lands(&["Evolving Wilds"]);
    let wilds = g.permanents(0).next().unwrap();
    let forest = library_card(&mut g, 0, "Forest");
    begin(&mut g);
    act(&mut g, Action::Activate(wilds, 0));
    assert_eq!(g.obj(wilds).zone, Zone::Graveyard, "sacrificed as a cost");
    resolve_all(&mut g, &[Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(g.obj(forest).tapped);
}

#[test]
fn rogues_passage_makes_a_creature_unblockable() {
    let mut g = lands(&["Rogue's Passage", "Mountain", "Mountain", "Mountain", "Mountain"]);
    let passage = g.permanents(0).next().unwrap();
    let lions = put(&mut g, 0, "Savannah Lions");
    put(&mut g, 1, "Aegis Turtle");
    begin(&mut g);
    act(&mut g, Action::Activate(passage, 0));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(lions));
    act(&mut g, Action::FinishAttacks);
    assert_ne!(g.pending, Pending::DeclareBlockers(1), "no legal blocks");
}

#[test]
fn llanowar_elves_taps_for_green_but_not_while_summoning_sick() {
    // CR 302.6.
    let mut g = lands(&["Forest"]);
    let elves = hand(&mut g, 0, "Llanowar Elves");
    let pair = hand(&mut g, 0, "Bite Down"); // {1}{G}
    begin(&mut g);
    play(&mut g, elves, false, &[], &[]);
    assert!(
        !g.legal_actions().contains(&Action::Cast(pair)),
        "the elves are summoning sick"
    );
    let mut g = lands(&["Forest"]);
    put(&mut g, 0, "Llanowar Elves");
    let growth = hand(&mut g, 0, "Giant Growth");
    let wolf = hand(&mut g, 0, "Ambush Wolf"); // {2}{G}: three mana, and there are two
    begin(&mut g);
    assert!(g.legal_actions().contains(&Action::Cast(growth)));
    assert!(!g.legal_actions().contains(&Action::Cast(wolf)));
}

#[test]
fn dazzling_angel_gains_life_when_another_creature_enters() {
    let mut g = table();
    put(&mut g, 0, "Dazzling Angel");
    let lions = hand(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(g.players[0].life, 21);
}

#[test]
fn beast_kin_ranger_gets_bigger_when_another_creature_enters() {
    let mut g = table();
    let ranger = put(&mut g, 0, "Beast-Kin Ranger");
    let lions = hand(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(g.power(ranger), 4);
}

#[test]
fn hungry_ghoul_eats_another_creature() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    assert!(
        !g.legal_actions().contains(&Action::ActivateWith(ghoul, 0, ghoul)),
        "another creature"
    );
    act(&mut g, Action::ActivateWith(ghoul, 0, lions));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
    assert_eq!(g.power(ghoul), 3);
}

#[test]
fn cathar_commando_sacrifices_to_destroy_an_enchantment() {
    let mut g = table();
    let cathar = put(&mut g, 0, "Cathar Commando");
    let light = put(&mut g, 1, "Banishing Light");
    begin(&mut g);
    act(&mut g, Action::Activate(cathar, 0));
    act(&mut g, Action::Target(obj(light)));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.obj(cathar).zone, g.obj(light).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
}

#[test]
fn strix_lookout_loots_with_a_tap() {
    let mut g = table();
    let strix = put(&mut g, 0, "Strix Lookout");
    let junk = hand(&mut g, 0, "Forest");
    begin(&mut g);
    act(&mut g, Action::Activate(strix, 0));
    assert!(g.obj(strix).tapped);
    resolve_all(&mut g, &[Action::Discard(junk)]);
    assert_eq!(g.obj(junk).zone, Zone::Graveyard);
}

#[test]
fn spectral_sailor_draws_for_four() {
    let mut g = table();
    let sailor = put(&mut g, 0, "Spectral Sailor");
    begin(&mut g);
    let before = g.players[0].hand.len();
    act(&mut g, Action::Activate(sailor, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), before + 1);
}

#[test]
fn treetop_snarespinner_only_at_sorcery_speed() {
    let mut g = table();
    let spider = put(&mut g, 0, "Treetop Snarespinner");
    let lions = put(&mut g, 0, "Savannah Lions");
    g.start_turn(3, 0);
    assert_eq!(g.step, Step::Upkeep);
    assert!(
        !g.legal_actions().contains(&Action::Activate(spider, 0)),
        "not in upkeep"
    );
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Activate(spider, 0));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(lions), 3);
}

#[test]
fn bite_down() {
    let mut g = table();
    let c = hand(&mut g, 0, "Bite Down");
    let mine = put(&mut g, 0, "Serra Angel");
    let theirs = put(&mut g, 1, "Brazen Scourge");
    begin(&mut g);
    play(&mut g, c, false, &[obj(mine), obj(theirs)], &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.obj(mine).damage, 0);
}

#[test]
fn vampire_soulcaller_cant_block_and_returns_a_creature() {
    let mut g = table();
    let c = hand(&mut g, 0, "Vampire Soulcaller");
    let dead = g.add_card(0, common::id("Savannah Lions"), Zone::Graveyard);
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Target(obj(dead))]);
    assert_eq!(g.obj(dead).zone, Zone::Hand);

    let mut g = table();
    put(&mut g, 1, "Vampire Soulcaller");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(lions));
    act(&mut g, Action::FinishAttacks);
    assert_ne!(g.pending, Pending::DeclareBlockers(1), "it can't block");
}

#[test]
fn elementalist_adept_has_prowess() {
    let mut g = table();
    let adept = put(&mut g, 0, "Elementalist Adept");
    let c = hand(&mut g, 0, "Boltwave");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.power(adept), 3);
}

#[test]
fn ambush_wolf_exiles_up_to_one_graveyard_card() {
    let mut g = table();
    let wolf = hand(&mut g, 0, "Ambush Wolf");
    let dead = g.add_card(1, common::id("Serra Angel"), Zone::Graveyard);
    begin(&mut g);
    play(&mut g, wolf, false, &[], &[Action::Target(obj(dead))]);
    assert_eq!(g.obj(dead).zone, Zone::Exile);
}

#[test]
fn goblin_boarders_enters_bigger_after_an_attack() {
    let mut g = table();
    let c = hand(&mut g, 0, "Goblin Boarders");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.power(c), 3);

    let mut g = table();
    let c = hand(&mut g, 0, "Goblin Boarders");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(lions));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        act(&mut g, Action::PassPriority);
    }
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.power(c), 4);
}

#[test]
fn gnarlid_colony_kicked_enters_with_counters_and_shares_trample() {
    let mut g = table();
    let c = hand(&mut g, 0, "Gnarlid Colony");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, c, true, &[], &[]);
    assert_eq!(g.power(c), 4);
    assert!(g.keywords(c).has(Keywords::TRAMPLE));
    assert!(
        !g.keywords(lions).has(Keywords::TRAMPLE),
        "no counter, no trample"
    );
}

#[test]
fn inspiring_paladin_has_first_strike_on_your_turn_only() {
    let mut g = table();
    let paladin = put(&mut g, 0, "Inspiring Paladin");
    let lions = put(&mut g, 0, "Savannah Lions");
    g.objects[lions.0 as usize].counters = 1;
    begin(&mut g);
    assert!(g.keywords(paladin).has(Keywords::FIRST_STRIKE));
    assert!(g.keywords(lions).has(Keywords::FIRST_STRIKE));
    while g.active == 0 {
        let a = if g.legal_actions().contains(&Action::PassPriority) {
            Action::PassPriority
        } else {
            Action::FinishAttacks
        };
        act(&mut g, a);
    }
    assert!(
        !g.keywords(paladin).has(Keywords::FIRST_STRIKE),
        "not on the opponent's turn"
    );
}

#[test]
fn courageous_goblin_needs_a_big_friend() {
    let mut g = table();
    let goblin = put(&mut g, 0, "Courageous Goblin");
    put(&mut g, 0, "Serra Angel");
    begin(&mut g);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(goblin));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(goblin), 3);
    assert!(g.keywords(goblin).has(Keywords::MENACE));
}

#[test]
fn sun_blessed_healer_kicked_reanimates_a_cheap_permanent() {
    let mut g = table();
    let c = hand(&mut g, 0, "Sun-Blessed Healer");
    let lions = g.add_card(0, common::id("Savannah Lions"), Zone::Graveyard);
    let angel = g.add_card(0, common::id("Serra Angel"), Zone::Graveyard);
    begin(&mut g);
    act(&mut g, Action::CastKicked(c));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert_eq!(
        g.legal_actions(),
        vec![Action::Target(obj(lions))],
        "mana value 2 or less"
    );
    let _ = angel;
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield);

    let mut g = table();
    let c = hand(&mut g, 0, "Sun-Blessed Healer");
    g.add_card(0, common::id("Savannah Lions"), Zone::Graveyard);
    begin(&mut g);
    play(&mut g, c, false, &[], &[]); // not kicked: no trigger
    assert!(g.players[0].graveyard.len() == 1);
}

#[test]
fn vanguard_seraph_surveils_on_the_first_life_gain() {
    let mut g = table();
    put(&mut g, 0, "Vanguard Seraph");
    let hawk = hand(&mut g, 0, "Dazzling Angel");
    let lions = hand(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, hawk, false, &[], &[]); // the Angel itself doesn't trigger on entering
    play(&mut g, lions, false, &[], &[Action::Yes]); // 1 life from the Angel: surveil once
    assert_eq!(g.players[0].graveyard.len(), 1);
}

#[test]
fn cat_collector_makes_food_and_cats_on_your_turn() {
    let mut g = table();
    let c = hand(&mut g, 0, "Cat Collector");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    let food = token_of(&g, 0, "Food");
    act(&mut g, Action::Activate(food, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens_named(&g, 0, "Cat"), 1);
}

#[test]
fn mischievous_mystic_rewards_the_second_draw() {
    let mut g = table();
    put(&mut g, 0, "Mischievous Mystic");
    let sailor = put(&mut g, 0, "Spectral Sailor");
    begin(&mut g); // drew one card in the draw step
    act(&mut g, Action::Activate(sailor, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens_named(&g, 0, "Faerie"), 1);
}

#[test]
fn a_creature_can_tap_for_mana_and_then_be_sacrificed() {
    // Found by the fuzzer: Hungry Ghoul sacrificing the Llanowar Elves that pays for it.
    let mut g = lands(&[]);
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let elves = put(&mut g, 0, "Llanowar Elves");
    begin(&mut g);
    act(&mut g, Action::ActivateWith(ghoul, 0, elves));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elves).zone, Zone::Graveyard);
    assert_eq!(g.power(ghoul), 3);

    // The same for a spell's additional cost: Arbiter of Woe ({4}{B}{B}) with five Swamps and the
    // Elves, which pay one generic mana and are then sacrificed.
    let mut g = lands(&["Swamp", "Swamp", "Swamp", "Swamp", "Swamp"]);
    let elves = put(&mut g, 0, "Llanowar Elves");
    let arbiter = hand(&mut g, 0, "Arbiter of Woe");
    begin(&mut g);
    act(&mut g, Action::CastWith(arbiter, elves));
    assert_eq!(g.obj(elves).zone, Zone::Graveyard);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(arbiter).zone, Zone::Battlefield);
}

#[test]
fn activation_targets_are_chosen_before_costs_are_paid() {
    // CR 602.2b. Found by the fuzzer: Cathar Commando's only possible target was the Treasure that
    // paid for it. Targets come first, so the ability can be activated, and then fizzles.
    let mut g = lands(&[]);
    let cathar = put(&mut g, 0, "Cathar Commando");
    let barrier = put(&mut g, 0, "Gleaming Barrier");
    begin(&mut g);
    // Kill the Barrier for its Treasure: mark 3 damage, then a Stab (-2/-2) finishes it.
    g.objects[barrier.0 as usize].damage = 3;
    let stab = hand(&mut g, 0, "Stab");
    g.add_card(0, common::id("Swamp"), Zone::Battlefield);
    play(&mut g, stab, false, &[obj(barrier)], &[]);
    let treasure = token_of(&g, 0, "Treasure");
    act(&mut g, Action::Activate(cathar, 0));
    act(&mut g, Action::Target(obj(treasure)));
    assert_eq!(g.obj(treasure).zone, Zone::Gone, "paid for the ability");
    let ev = {
        let mut ev = act(&mut g, Action::PassPriority);
        ev.extend(act(&mut g, Action::PassPriority));
        ev
    };
    assert!(ev.contains(&bolt_core::Event::Fizzled { source: cathar }));
}
