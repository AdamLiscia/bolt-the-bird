//! Tarkir: Dragonstorm (TDM) cards: each script's behavior, on a Tarkir table. Progress and the order
//! cards are done in: docs/tdm-progress.md.

mod common;

use bolt_cards::set;
use bolt_core::{Action, Game, Keywords, ObjId, Pending, Target, Zone};
use common::{act, begin, obj, play, resolve_all};

fn tid(name: &str) -> usize {
    set("TDM")
        .unwrap()
        .index_of(name)
        .unwrap_or_else(|| panic!("{name} is not in TDM"))
}

/// Player 0's first main phase on turn 3 with Tarkir cards: 5 of each basic land each, libraries of
/// Plains. Every priority stop is visible.
fn table() -> Game {
    let mut g = Game::empty(set("TDM").unwrap().defs.clone(), 1, 0);
    g.auto_pass = false;
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, tid("Plains"), Zone::Library);
        }
        for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            for _ in 0..5 {
                g.add_card(p, tid(land), Zone::Battlefield);
            }
        }
    }
    g
}

fn put(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, tid(name), Zone::Battlefield)
}

fn hand(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, tid(name), Zone::Hand)
}

fn tokens(g: &Game, p: usize, name: &str) -> Vec<ObjId> {
    g.permanents(p)
        .filter(|&x| g.def_of(x).token && g.def_of(x).name == name)
        .collect()
}

#[test]
fn sibsig_appraiser_keeps_one_of_the_top_two_and_bins_the_other() {
    let mut g = table();
    let c = hand(&mut g, 0, "Sibsig Appraiser");
    begin(&mut g);
    let a = g.add_card(0, tid("Iceridge Serpent"), Zone::Library);
    let b = g.add_card(0, tid("Humbling Elder"), Zone::Library); // on top, after the draw step
    play(&mut g, c, false, &[], &[Action::Pick(a)]);
    assert_eq!(g.obj(a).zone, Zone::Hand);
    assert_eq!(g.obj(b).zone, Zone::Graveyard);
}

#[test]
fn iceridge_serpent_bounces_an_opponents_creature() {
    let mut g = table();
    let c = hand(&mut g, 0, "Iceridge Serpent");
    let theirs = put(&mut g, 1, "Humbling Elder");
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Target(obj(theirs))]);
    assert_eq!(g.obj(theirs).zone, Zone::Hand);
    assert!(g.players[1].hand.contains(&theirs));
}

#[test]
fn boulderborn_dragon_surveils_when_it_attacks() {
    let mut g = table();
    let d = put(&mut g, 0, "Boulderborn Dragon");
    assert!(g.keywords(d).has(Keywords::FLYING) && g.keywords(d).has(Keywords::VIGILANCE));
    begin(&mut g);
    let top = *g.players[0].library.last().unwrap();
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(d));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[Action::Yes]); // surveil: into the graveyard
    assert_eq!(g.obj(top).zone, Zone::Graveyard);
    assert!(!g.obj(d).tapped, "vigilance");
}

#[test]
fn humbling_elder_shrinks_an_opponents_creature() {
    let mut g = table();
    let c = hand(&mut g, 0, "Humbling Elder");
    assert!(g.def_of(c).keywords.has(Keywords::FLASH));
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    let before = g.power(theirs);
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Target(obj(theirs))]);
    assert_eq!(g.power(theirs), before - 2);
}

#[test]
fn rebellious_strike_pumps_and_draws() {
    let mut g = table();
    let c = hand(&mut g, 0, "Rebellious Strike");
    let mine = put(&mut g, 0, "Humbling Elder");
    begin(&mut g);
    let (p, n) = (g.power(mine), g.players[0].hand.len());
    play(&mut g, c, false, &[obj(mine)], &[]);
    assert_eq!(g.power(mine), p + 3);
    assert_eq!(g.players[0].hand.len(), n, "cast one, drew one");
}

#[test]
fn sarkhans_resolve_pumps_or_kills_a_flier() {
    let mut g = table();
    let c = hand(&mut g, 0, "Sarkhan's Resolve");
    let mine = put(&mut g, 0, "Humbling Elder");
    let dragon = put(&mut g, 1, "Boulderborn Dragon");
    let ground = put(&mut g, 1, "Iceridge Serpent");
    begin(&mut g);
    let (p, t) = (g.power(mine), g.toughness(mine));
    act(&mut g, Action::CastModal(c, 0));
    act(&mut g, Action::Target(obj(mine)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(mine), g.toughness(mine)), (p + 3, t + 3));

    let c2 = hand(&mut g, 0, "Sarkhan's Resolve");
    act(&mut g, Action::CastModal(c2, 1));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(ground))),
        "only creatures with flying"
    );
    act(&mut g, Action::Target(obj(dragon)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(dragon).zone, Zone::Graveyard);
}

#[test]
fn underfoot_underdogs_makes_a_goblin_and_sneaks_small_creatures_through() {
    let mut g = table();
    let c = hand(&mut g, 0, "Underfoot Underdogs");
    let big = put(&mut g, 0, "Iceridge Serpent"); // 3 power
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    let goblin = tokens(&g, 0, "Goblin");
    assert_eq!(goblin.len(), 1);
    g.start_turn(5, 0); // a later turn: no longer summoning sick, for the {T} ability
    while g.step != bolt_core::Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Activate(c, 0));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(big))),
        "power 2 or less only"
    );
    act(&mut g, Action::Target(obj(goblin[0])));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(goblin[0]).has(Keywords::UNBLOCKABLE));
}

#[test]
fn watcher_of_the_wayside_mills_a_player_and_gains_two() {
    let mut g = table();
    let c = hand(&mut g, 0, "Watcher of the Wayside");
    begin(&mut g);
    let (lib, life) = (g.players[1].library.len(), g.players[0].life);
    play(&mut g, c, false, &[], &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].library.len(), lib - 2);
    assert_eq!(g.players[0].life, life + 2);
}

#[test]
fn meticulous_artisan_makes_a_treasure_and_has_prowess() {
    let mut g = table();
    let c = hand(&mut g, 0, "Meticulous Artisan");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1);
    let p = g.power(c);
    let strike = hand(&mut g, 0, "Rebellious Strike");
    let other = put(&mut g, 0, "Humbling Elder");
    play(&mut g, strike, false, &[obj(other)], &[]);
    assert_eq!(g.power(c), p + 1, "prowess");
}

#[test]
fn stormplain_detainment_exiles_until_it_leaves() {
    let mut g = table();
    let c = hand(&mut g, 0, "Stormplain Detainment");
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Target(obj(theirs))]);
    assert_eq!(g.obj(theirs).zone, Zone::Exile);
    g.move_card(c, Zone::Graveyard);
    resolve_all(&mut g, &[]);
    assert!(
        g.permanents(1).any(|x| g.def_of(x).name == "Iceridge Serpent"),
        "it comes back when the enchantment leaves"
    );
}

// ---- batch 2: behold (CR 701.4x-style: choose a Dragon you control or reveal one from hand) ----

fn untapped_lands(g: &Game, p: usize) -> usize {
    g.permanents(p)
        .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
        .count()
}

#[test]
fn caustic_exhale_costs_one_more_unless_a_dragon_is_beheld() {
    let mut g = table();
    let c = hand(&mut g, 0, "Caustic Exhale");
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    begin(&mut g);
    assert!(
        !g.legal_actions().contains(&Action::CastKicked(c)),
        "no Dragon to behold"
    );
    let before = untapped_lands(&g, 0);
    play(&mut g, c, false, &[obj(theirs)], &[]);
    assert_eq!(before - untapped_lands(&g, 0), 2, "{{B}} plus {{1}}");
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "3/... gets -3/-3"); // Iceridge Serpent is 3/3

    let c2 = hand(&mut g, 0, "Caustic Exhale");
    hand(&mut g, 0, "Boulderborn Dragon"); // a Dragon card to reveal
    let other = put(&mut g, 1, "Humbling Elder");
    let before = untapped_lands(&g, 0);
    play(&mut g, c2, true, &[obj(other)], &[]);
    assert_eq!(before - untapped_lands(&g, 0), 1, "beheld: just {{B}}");
}

#[test]
fn dispelling_exhale_counters_unless_paid_more_with_a_dragon() {
    let mut g = table();
    let target = hand(&mut g, 0, "Iceridge Serpent");
    let ex = hand(&mut g, 0, "Dispelling Exhale");
    begin(&mut g);
    act(&mut g, Action::Cast(target));
    act(&mut g, Action::Cast(ex));
    act(&mut g, Action::Target(obj(target)));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority); // Dispelling resolves: the spell's controller decides
    assert!(matches!(g.pending, Pending::Decide(0)));
    act(&mut g, Action::No);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(target).zone, Zone::Graveyard, "not paid: countered");

    // Beheld: the price is {4}.
    let t2 = hand(&mut g, 0, "Humbling Elder");
    let ex2 = hand(&mut g, 0, "Dispelling Exhale");
    put(&mut g, 0, "Boulderborn Dragon");
    act(&mut g, Action::Cast(t2));
    act(&mut g, Action::CastKicked(ex2));
    act(&mut g, Action::Target(obj(t2)));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    let before = untapped_lands(&g, 0);
    act(&mut g, Action::Yes);
    assert_eq!(before - untapped_lands(&g, 0), 4);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(t2).zone, Zone::Battlefield, "paid: it resolves");
}

#[test]
fn molten_exhale_has_flash_only_when_a_dragon_is_beheld() {
    let mut g = table();
    let c = hand(&mut g, 0, "Molten Exhale");
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    begin(&mut g);
    assert!(
        g.legal_actions().contains(&Action::Cast(c)),
        "a sorcery, in a main phase"
    );
    act(&mut g, Action::PassPriority); // to beginning of combat
    act(&mut g, Action::PassPriority);
    assert!(g.step != bolt_core::Step::Main1);
    let at_speed = |g: &Game| {
        (
            g.legal_actions().contains(&Action::Cast(c)),
            g.legal_actions().contains(&Action::CastKicked(c)),
        )
    };
    while !matches!(g.pending, Pending::Priority(0)) || g.step == bolt_core::Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    {
        assert_eq!(at_speed(&g), (false, false), "no Dragon: sorcery speed only");
        hand(&mut g, 0, "Boulderborn Dragon");
        assert_eq!(at_speed(&g), (false, true), "beheld: as though it had flash");
        act(&mut g, Action::CastKicked(c));
        act(&mut g, Action::Target(obj(theirs)));
        resolve_all(&mut g, &[]);
        assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "4 damage to a 3/3");
    }
}

#[test]
fn osseous_exhale_hits_an_attacker_and_gains_life_if_beheld() {
    let mut g = table();
    let attacker = put(&mut g, 0, "Iceridge Serpent");
    let c = hand(&mut g, 0, "Osseous Exhale");
    hand(&mut g, 0, "Boulderborn Dragon");
    begin(&mut g);
    assert!(
        !g.legal_actions()
            .iter()
            .any(|a| matches!(a, Action::Cast(x) | Action::CastKicked(x) if *x == c)),
        "no attacker yet"
    );
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(attacker));
    act(&mut g, Action::FinishAttacks);
    let life = g.players[0].life;
    play(&mut g, c, true, &[obj(attacker)], &[]);
    assert_eq!(g.obj(attacker).zone, Zone::Graveyard);
    assert_eq!(g.players[0].life, life + 2);
}

#[test]
fn piercing_exhale_bites_and_surveils_if_beheld() {
    let mut g = table();
    let mine = put(&mut g, 0, "Iceridge Serpent"); // 3 power
    let theirs = put(&mut g, 1, "Humbling Elder");
    put(&mut g, 0, "Boulderborn Dragon");
    let c = hand(&mut g, 0, "Piercing Exhale");
    begin(&mut g);
    let top = *g.players[0].library.last().unwrap();
    play(
        &mut g,
        c,
        true,
        &[obj(mine), obj(theirs)],
        &[Action::Yes, Action::Yes],
    );
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.obj(top).zone, Zone::Graveyard, "surveil 2");
}

#[test]
fn riverwalk_technique_tucks_a_permanent_or_counters_a_noncreature_spell() {
    let mut g = table();
    let c = hand(&mut g, 0, "Riverwalk Technique");
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    begin(&mut g);
    act(&mut g, Action::CastModal(c, 0));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[Action::No]); // its owner puts it on the bottom
    assert_eq!(g.obj(theirs).zone, Zone::Library);
    assert_eq!(g.players[1].library[0], theirs, "bottom");

    let strike = hand(&mut g, 0, "Rebellious Strike");
    let creature = hand(&mut g, 0, "Humbling Elder");
    let c2 = hand(&mut g, 0, "Riverwalk Technique");
    let mine = put(&mut g, 0, "Iceridge Serpent");
    act(&mut g, Action::Cast(creature));
    act(&mut g, Action::Cast(strike));
    act(&mut g, Action::Target(obj(mine)));
    act(&mut g, Action::CastModal(c2, 1));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(creature))),
        "noncreature spells only"
    );
    act(&mut g, Action::Target(obj(strike)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(strike).zone, Zone::Graveyard);
    assert_eq!(g.obj(creature).zone, Zone::Battlefield);
}

#[test]
fn ainok_wayfarer_takes_a_milled_land_or_grows() {
    let mut g = table();
    let c = hand(&mut g, 0, "Ainok Wayfarer");
    begin(&mut g);
    let top: Vec<ObjId> = g.players[0].library.iter().rev().take(3).copied().collect(); // Plains
    play(&mut g, c, false, &[], &[Action::Pick(top[0])]);
    assert_eq!(g.obj(top[0]).zone, Zone::Hand);
    assert!(g.obj(top[1]).zone == Zone::Graveyard && g.obj(top[2]).zone == Zone::Graveyard);
    assert_eq!(g.obj(c).counters, 0);

    let c2 = hand(&mut g, 0, "Ainok Wayfarer");
    play(&mut g, c2, false, &[], &[Action::No]);
    assert_eq!(g.obj(c2).counters, 1, "no land taken: a +1/+1 counter");
}

#[test]
fn sibsig_appraiser_must_take_one() {
    let mut g = table();
    let c = hand(&mut g, 0, "Sibsig Appraiser");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    for _ in 0..6 {
        if matches!(g.pending, Pending::Decide(_)) {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::No),
        "\"Put one of them into your hand\""
    );
}

#[test]
fn dragonstorm_globe_grows_dragons_and_makes_any_color() {
    let mut g = table();
    put(&mut g, 0, "Dragonstorm Globe");
    let d = hand(&mut g, 0, "Boulderborn Dragon");
    begin(&mut g);
    assert!(g
        .mana_sources(0)
        .iter()
        .any(|(id, m)| g.def_of(*id).name == "Dragonstorm Globe" && m.colors == [true; 5]));
    play(&mut g, d, false, &[], &[]);
    assert_eq!(g.obj(d).counters, 1);
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    let other = hand(&mut g, 0, "Humbling Elder");
    play(&mut g, other, false, &[], &[Action::Target(obj(theirs))]);
    assert_eq!(g.obj(other).counters, 0, "only Dragons");
}

#[test]
fn undergrowth_leopard_eats_an_artifact() {
    let mut g = table();
    let l = put(&mut g, 0, "Undergrowth Leopard");
    let globe = put(&mut g, 1, "Dragonstorm Globe");
    begin(&mut g);
    act(&mut g, Action::Activate(l, 0));
    act(&mut g, Action::Target(obj(globe)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(globe).zone, Zone::Graveyard);
    assert_eq!(g.obj(l).zone, Zone::Graveyard, "sacrificed");
}

#[test]
fn narsets_rebuke_exiles_what_it_kills_and_adds_three_mana() {
    let mut g = table();
    let c = hand(&mut g, 0, "Narset's Rebuke");
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    begin(&mut g);
    play(&mut g, c, false, &[obj(theirs)], &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Exile);
    assert_eq!(g.players[0].pool.len(), 3, "{{U}}{{R}}{{W}}");
}

// ---- batch 3: {2/C} hybrid costs (CR 107.4e) ----

/// Player 0's main phase with exactly these lands and a Tarkir library.
fn tdm_lands(names: &[&str]) -> Game {
    let mut g = Game::empty(set("TDM").unwrap().defs.clone(), 1, 0);
    g.auto_pass = false;
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, tid("Plains"), Zone::Library);
        }
    }
    for n in names {
        put(&mut g, 0, n);
    }
    g
}

#[test]
fn a_two_or_color_cost_pays_with_the_color_when_it_can() {
    let mut g = tdm_lands(&["Forest", "Island", "Mountain", "Plains", "Plains"]);
    let c = hand(&mut g, 0, "Temur Tawnyback"); // {2/G}{2/U}{2/R}
    let junk = hand(&mut g, 0, "Plains");
    assert_eq!(g.def_of(c).cost.mana_value(), 6, "each {{2/C}} counts as 2");
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Discard(junk)]);
    assert_eq!(g.obj(c).zone, Zone::Battlefield);
    assert_eq!(
        untapped_lands(&g, 0),
        2,
        "G, U and R paid with one land each; the Plains stay up"
    );
}

#[test]
fn a_two_or_color_cost_pays_two_generic_without_the_color() {
    let mut g = tdm_lands(&["Plains", "Plains", "Plains", "Plains", "Plains", "Plains"]);
    let c = hand(&mut g, 0, "Temur Tawnyback");
    begin(&mut g);
    assert!(
        g.legal_actions().contains(&Action::Cast(c)),
        "six Plains pay {{2}}{{2}}{{2}}"
    );
    let mut short = tdm_lands(&["Plains", "Plains", "Plains", "Plains", "Plains"]);
    let c2 = hand(&mut short, 0, "Temur Tawnyback");
    begin(&mut short);
    assert!(!short.legal_actions().contains(&Action::Cast(c2)), "five can't");
}

#[test]
fn temur_tawnyback_loots() {
    let mut g = table();
    let c = hand(&mut g, 0, "Temur Tawnyback");
    let junk = hand(&mut g, 0, "Plains");
    begin(&mut g);
    let n = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[Action::Discard(junk)]);
    assert_eq!(g.obj(junk).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n - 1, "cast it, drew one, discarded one");
}

#[test]
fn defibrillating_current_burns_and_heals() {
    let mut g = table();
    let c = hand(&mut g, 0, "Defibrillating Current");
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    begin(&mut g);
    let life = g.players[0].life;
    play(&mut g, c, false, &[obj(theirs)], &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.players[0].life, life + 2);
}

#[test]
fn dragonclaw_strike_doubles_then_fights_or_not() {
    let mut g = table();
    let mine = put(&mut g, 0, "Humbling Elder"); // 1/2
    let theirs = put(&mut g, 1, "Iceridge Serpent"); // 3/3? (doubles to 2/4 vs 3 power)
    let c = hand(&mut g, 0, "Dragonclaw Strike");
    begin(&mut g);
    let (p, t) = (g.power(mine), g.toughness(mine));
    play(&mut g, c, false, &[obj(mine), obj(theirs)], &[]);
    assert_eq!((g.power(mine), g.toughness(mine)), (2 * p, 2 * t));
    assert!(
        g.obj(theirs).damage > 0 || g.obj(theirs).zone == Zone::Graveyard,
        "they fought"
    );

    // Up to one: the fight can be skipped.
    let c2 = hand(&mut g, 0, "Dragonclaw Strike");
    act(&mut g, Action::Cast(c2));
    act(&mut g, Action::Target(obj(mine)));
    assert!(
        g.legal_actions().contains(&Action::No)
            || g.legal_actions()
                .iter()
                .any(|a| matches!(a, Action::PassPriority)),
        "no second target needed: {:?}",
        g.legal_actions()
    );
}

#[test]
fn gurmag_nightwatch_keeps_one_on_top_and_bins_the_rest() {
    let mut g = table();
    let c = hand(&mut g, 0, "Gurmag Nightwatch");
    begin(&mut g);
    let top: Vec<ObjId> = g.players[0].library.iter().rev().take(3).copied().collect();
    play(&mut g, c, false, &[], &[Action::Pick(top[1])]);
    assert_eq!(*g.players[0].library.last().unwrap(), top[1]);
    assert!(g.obj(top[0]).zone == Zone::Graveyard && g.obj(top[2]).zone == Zone::Graveyard);
}

#[test]
fn kin_tree_severance_exiles_only_mana_value_three_or_more() {
    let mut g = table();
    let c = hand(&mut g, 0, "Kin-Tree Severance");
    let small = put(&mut g, 1, "Humbling Elder"); // mana value 1
    let big = put(&mut g, 1, "Iceridge Serpent"); // mana value 5
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    assert!(!g.legal_actions().contains(&Action::Target(obj(small))));
    act(&mut g, Action::Target(obj(big)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(big).zone, Zone::Exile);
}

#[test]
fn monastery_messenger_puts_a_spell_back_on_top() {
    let mut g = table();
    let c = hand(&mut g, 0, "Monastery Messenger");
    let spell_card = g.add_card(0, tid("Rebellious Strike"), Zone::Graveyard);
    let creature_card = g.add_card(0, tid("Humbling Elder"), Zone::Graveyard);
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    for _ in 0..4 {
        if matches!(g.pending, Pending::ChooseTarget(_)) {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(creature_card))),
        "noncreature, nonland only"
    );
    act(&mut g, Action::Target(obj(spell_card)));
    resolve_all(&mut g, &[]);
    assert_eq!(*g.players[0].library.last().unwrap(), spell_card);
}

#[test]
fn rakshasas_bargain_takes_two() {
    let mut g = table();
    let c = hand(&mut g, 0, "Rakshasa's Bargain");
    begin(&mut g);
    let top: Vec<ObjId> = g.players[0].library.iter().rev().take(4).copied().collect();
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert!(!g.legal_actions().contains(&Action::No), "must take");
    act(&mut g, Action::Pick(top[0]));
    assert!(
        !g.legal_actions().contains(&Action::Pick(top[0])),
        "a second, different pick"
    );
    act(&mut g, Action::Pick(top[2]));
    resolve_all(&mut g, &[]);
    assert!(g.obj(top[0]).zone == Zone::Hand && g.obj(top[2]).zone == Zone::Hand);
    assert!(g.obj(top[1]).zone == Zone::Graveyard && g.obj(top[3]).zone == Zone::Graveyard);
}

#[test]
fn reputable_merchant_grows_a_creature_on_entering_and_dying() {
    let mut g = table();
    let mine = put(&mut g, 0, "Iceridge Serpent");
    let c = hand(&mut g, 0, "Reputable Merchant");
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Target(obj(mine))]);
    assert_eq!(g.obj(mine).counters, 1);
    let kill = hand(&mut g, 0, "Caustic Exhale");
    play(&mut g, kill, false, &[obj(c)], &[Action::Target(obj(mine))]);
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
    assert_eq!(g.obj(mine).counters, 2);
}

#[test]
fn riverwheel_sweep_locks_down_a_creature_and_offers_a_card() {
    let mut g = table();
    let c = hand(&mut g, 0, "Riverwheel Sweep");
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    begin(&mut g);
    let top: Vec<ObjId> = g.players[0].library.iter().rev().take(2).copied().collect();
    play(&mut g, c, false, &[obj(theirs)], &[Action::Pick(top[0])]);
    assert!(g.obj(theirs).tapped);
    assert_eq!(g.obj(theirs).stun, 3);
    assert!(top.iter().all(|&x| g.obj(x).zone == Zone::Exile));
}

#[test]
fn jade_cast_sentinel_bottoms_a_graveyard_card() {
    let mut g = table();
    let s = put(&mut g, 0, "Jade-Cast Sentinel");
    let theirs = g.add_card(1, tid("Iceridge Serpent"), Zone::Graveyard);
    begin(&mut g);
    act(&mut g, Action::Activate(s, 0));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.players[1].library[0], theirs,
        "the bottom of its owner's library"
    );
}

// ---- batch 4: mobilize ----

/// Attack with `attacker` on a later turn (not summoning sick); stops after attacks are declared and
/// the attack triggers have resolved.
fn attack_with(g: &mut Game, attacker: ObjId) {
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    act(g, Action::Attack(attacker));
    act(g, Action::FinishAttacks);
    resolve_all(g, &[]);
}

#[test]
fn mobilize_makes_attacking_warriors_that_leave_at_the_end_step() {
    let mut g = table();
    let p = put(&mut g, 0, "Dalkovan Packbeasts"); // Mobilize 3
    let life = g.players[1].life;
    attack_with(&mut g, p);
    let warriors = tokens(&g, 0, "Warrior");
    assert_eq!(warriors.len(), 3);
    let combat = g.combat.as_ref().expect("in combat");
    assert!(warriors
        .iter()
        .all(|w| g.obj(*w).tapped && combat.attackers.contains(w)));
    while g.step != bolt_core::Step::End {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(
        g.players[1].life,
        life - 3,
        "three 1/1s unblocked (the Packbeasts are 0/4)"
    );
    while !matches!(g.step, bolt_core::Step::Cleanup) && g.turn == 5 {
        act(&mut g, Action::PassPriority);
    }
    assert!(tokens(&g, 0, "Warrior").is_empty(), "sacrificed at the end step");
}

#[test]
fn avenger_of_the_fallen_mobilizes_one_per_creature_card_in_the_graveyard() {
    let mut g = table();
    let a = put(&mut g, 0, "Avenger of the Fallen");
    for n in ["Humbling Elder", "Iceridge Serpent"] {
        g.add_card(0, tid(n), Zone::Graveyard);
    }
    g.add_card(0, tid("Rebellious Strike"), Zone::Graveyard); // not a creature
    attack_with(&mut g, a);
    assert_eq!(tokens(&g, 0, "Warrior").len(), 2);
}

#[test]
fn venerated_stormsinger_drains_when_its_warrior_is_sacrificed() {
    let mut g = table();
    let s = put(&mut g, 0, "Venerated Stormsinger");
    attack_with(&mut g, s);
    assert_eq!(tokens(&g, 0, "Warrior").len(), 1);
    // Block nothing; at the end step the Warrior is sacrificed: a creature you control dies.
    let before = (g.players[0].life, g.players[1].life);
    while g.turn == 5 && !tokens(&g, 0, "Warrior").is_empty() {
        act(&mut g, Action::PassPriority);
    }
    resolve_all(&mut g, &[]);
    let after = (g.players[0].life, g.players[1].life);
    assert_eq!(after.0, before.0 + 1);
    assert_eq!(
        after.1,
        before.1 - 1 - 1 - 3,
        "1 drained, plus 1 (Warrior) and 3 (Stormsinger) combat damage"
    );
}

#[test]
fn bone_cairn_butcher_gives_attacking_tokens_deathtouch() {
    let mut g = table();
    let b = put(&mut g, 0, "Bone-Cairn Butcher");
    attack_with(&mut g, b);
    let w = tokens(&g, 0, "Warrior");
    assert_eq!(w.len(), 2);
    assert!(w.iter().all(|&x| g.keywords(x).has(Keywords::DEATHTOUCH)));
    assert!(!g.keywords(b).has(Keywords::DEATHTOUCH), "tokens only");
}

#[test]
fn zurgos_vanguard_counts_your_creatures() {
    let mut g = table();
    let v = put(&mut g, 0, "Zurgo's Vanguard");
    assert_eq!(g.power(v), 1);
    put(&mut g, 0, "Humbling Elder");
    assert_eq!(g.power(v), 2);
    attack_with(&mut g, v);
    assert_eq!(g.power(v), 3, "its mobilized Warrior counts too");
}

#[test]
fn reigning_victor_and_nightblade_brigade_enter_with_their_triggers() {
    let mut g = table();
    let mine = put(&mut g, 0, "Humbling Elder");
    let rv = hand(&mut g, 0, "Reigning Victor");
    begin(&mut g);
    let p = g.power(mine);
    play(&mut g, rv, false, &[], &[Action::Target(obj(mine))]);
    assert_eq!(g.power(mine), p + 1);
    assert!(g.keywords(mine).has(Keywords::INDESTRUCTIBLE));
    let nb = hand(&mut g, 0, "Nightblade Brigade");
    let top = *g.players[0].library.last().unwrap();
    play(&mut g, nb, false, &[], &[Action::Yes]);
    assert_eq!(g.obj(top).zone, Zone::Graveyard, "surveil 1");
    assert!(g.keywords(nb).has(Keywords::DEATHTOUCH));
}

#[test]
fn shock_brigade_and_dragonback_lancer_mobilize_one() {
    for (name, kw) in [
        ("Shock Brigade", Keywords::MENACE),
        ("Dragonback Lancer", Keywords::FLYING),
    ] {
        let mut g = table();
        let c = put(&mut g, 0, name);
        assert!(g.keywords(c).has(kw), "{name}");
        attack_with(&mut g, c);
        assert_eq!(tokens(&g, 0, "Warrior").len(), 1, "{name}");
    }
}

// ---- batch 5: the Devotees' mana, costs less (and more), scry ----

fn paid(g: &mut Game, f: impl FnOnce(&mut Game)) -> usize {
    let before = untapped_lands(g, 0);
    f(g);
    before - untapped_lands(g, 0)
}

#[test]
fn a_devotee_fixes_one_colored_pip_once_each_turn() {
    let mut g = tdm_lands(&[
        "Plains", "Plains", "Plains", "Plains", "Plains", "Plains", "Plains", "Plains", "Plains", "Plains",
    ]);
    let serpent = hand(&mut g, 0, "Iceridge Serpent"); // {4}{U}
    let elder = hand(&mut g, 0, "Humbling Elder"); // {U}
    begin(&mut g);
    assert!(
        !g.legal_actions().contains(&Action::Cast(serpent)),
        "no blue source"
    );
    put(&mut g, 0, "Sultai Devotee"); // {1}: Add {B}, {G}, or {U}
    assert!(g.legal_actions().contains(&Action::Cast(serpent)));
    assert!(g.legal_actions().contains(&Action::Cast(elder)));
    let n = paid(&mut g, |g| play(g, serpent, false, &[], &[]));
    assert_eq!(n, 5, "{{4}} plus the {{1}} the Devotee turned blue");
    assert!(
        !g.legal_actions().contains(&Action::Cast(elder)),
        "only once each turn"
    );
}

#[test]
fn abzan_devotee_returns_from_the_graveyard() {
    let mut g = table();
    let d = g.add_card(0, tid("Abzan Devotee"), Zone::Graveyard);
    begin(&mut g);
    act(&mut g, Action::Activate(d, 0)); // its only activated ability (the mana is built into payment)
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(d).zone, Zone::Hand);
}

#[test]
fn mardu_devotee_scries_two() {
    let mut g = table();
    let c = hand(&mut g, 0, "Mardu Devotee");
    begin(&mut g);
    let top: Vec<ObjId> = g.players[0].library.iter().rev().take(2).copied().collect();
    play(&mut g, c, false, &[], &[Action::Yes, Action::No]);
    assert_eq!(g.players[0].library[0], top[0], "the first to the bottom");
    assert_eq!(
        *g.players[0].library.last().unwrap(),
        top[1],
        "the second kept on top"
    );
}

#[test]
fn focus_the_mind_is_cheaper_after_another_spell() {
    let mut g = table();
    let f1 = hand(&mut g, 0, "Focus the Mind");
    let junk = hand(&mut g, 0, "Plains");
    begin(&mut g);
    let n = paid(&mut g, |g| play(g, f1, false, &[], &[Action::Discard(junk)]));
    assert_eq!(n, 5, "first spell: full price");
    let f2 = hand(&mut g, 0, "Focus the Mind");
    let junk2 = hand(&mut g, 0, "Plains");
    let n = paid(&mut g, |g| play(g, f2, false, &[], &[Action::Discard(junk2)]));
    assert_eq!(n, 3, "{{2}} less after another spell");
}

#[test]
fn salt_road_packbeast_has_affinity_for_creatures() {
    let mut g = table();
    for _ in 0..3 {
        put(&mut g, 0, "Humbling Elder");
    }
    let c = hand(&mut g, 0, "Salt Road Packbeast"); // {5}{W}
    begin(&mut g);
    let n = paid(&mut g, |g| play(g, c, false, &[], &[]));
    assert_eq!(n, 3, "three creatures: {{2}}{{W}}");
}

#[test]
fn highspire_bell_ringer_discounts_the_second_spell() {
    let mut g = table();
    put(&mut g, 0, "Highspire Bell-Ringer");
    let a = hand(&mut g, 0, "Iceridge Serpent");
    let b = hand(&mut g, 0, "Iceridge Serpent");
    let c = hand(&mut g, 0, "Iceridge Serpent");
    begin(&mut g);
    assert_eq!(paid(&mut g, |g| play(g, a, false, &[], &[])), 5);
    assert_eq!(
        paid(&mut g, |g| play(g, b, false, &[], &[])),
        4,
        "the second spell"
    );
    assert_eq!(paid(&mut g, |g| play(g, c, false, &[], &[])), 5, "not the third");
}

#[test]
fn dragons_prey_costs_more_against_a_dragon() {
    let mut g = table();
    let dragon = put(&mut g, 1, "Boulderborn Dragon");
    let other = put(&mut g, 1, "Iceridge Serpent");
    let a = hand(&mut g, 0, "Dragon's Prey");
    let b = hand(&mut g, 0, "Dragon's Prey");
    begin(&mut g);
    assert_eq!(paid(&mut g, |g| play(g, a, false, &[obj(other)], &[])), 3);
    assert_eq!(paid(&mut g, |g| play(g, b, false, &[obj(dragon)], &[])), 5);
    assert_eq!(g.obj(dragon).zone, Zone::Graveyard);
}

#[test]
fn static_snare_is_cheaper_for_each_attacker_and_exiles() {
    let mut g = table();
    let a = put(&mut g, 0, "Iceridge Serpent");
    let b = put(&mut g, 0, "Humbling Elder");
    let theirs = put(&mut g, 1, "Dragonstorm Globe");
    let snare = hand(&mut g, 0, "Static Snare"); // {4}{W}, flash
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(a));
    act(&mut g, Action::Attack(b));
    act(&mut g, Action::FinishAttacks);
    let n = paid(&mut g, |g| {
        play(g, snare, false, &[], &[Action::Target(obj(theirs))])
    });
    assert_eq!(n, 3, "two attackers: {{2}}{{W}}");
    assert_eq!(g.obj(theirs).zone, Zone::Exile);
}

// ---- batch 6a: flurry and endure ----

#[test]
fn flurry_triggers_on_the_second_spell_only() {
    let mut g = table();
    put(&mut g, 0, "Cori Mountain Stalwart");
    let spells: Vec<ObjId> = (0..3).map(|_| hand(&mut g, 0, "Humbling Elder")).collect();
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    begin(&mut g);
    let life = (g.players[0].life, g.players[1].life);
    play(&mut g, spells[0], false, &[], &[Action::Target(obj(theirs))]);
    assert_eq!(
        (g.players[0].life, g.players[1].life),
        life,
        "first spell: nothing"
    );
    play(&mut g, spells[1], false, &[], &[Action::Target(obj(theirs))]);
    assert_eq!(
        (g.players[0].life, g.players[1].life),
        (life.0 + 2, life.1 - 2),
        "second spell"
    );
    play(&mut g, spells[2], false, &[], &[Action::Target(obj(theirs))]);
    assert_eq!(
        (g.players[0].life, g.players[1].life),
        (life.0 + 2, life.1 - 2),
        "not the third"
    );
}

#[test]
fn flurry_cards_do_their_things() {
    let mut g = table();
    let duelist = put(&mut g, 0, "Devoted Duelist");
    let jd = put(&mut g, 0, "Jeskai Devotee");
    let pp = put(&mut g, 0, "Poised Practitioner");
    put(&mut g, 0, "Wingblade Disciple");
    let adept = put(&mut g, 0, "Equilibrium Adept");
    let wayspeaker = put(&mut g, 0, "Wayspeaker Bodyguard");
    let theirs = put(&mut g, 1, "Iceridge Serpent");
    assert!(g.keywords(duelist).has(Keywords::HASTE));
    let a = hand(&mut g, 0, "Rebellious Strike");
    let b = hand(&mut g, 0, "Rebellious Strike");
    begin(&mut g);
    let (life, p) = (g.players[1].life, g.power(jd));
    play(&mut g, a, false, &[obj(duelist)], &[]);
    // Second spell: five flurry triggers. Answers in the order they come up: the Bodyguard's target,
    // then the Practitioner's scry.
    play(
        &mut g,
        b,
        false,
        &[obj(duelist)],
        &[Action::Target(obj(theirs)), Action::No],
    );
    assert_eq!(g.players[1].life, life - 1, "Duelist");
    assert_eq!(g.power(jd), p + 1, "Jeskai Devotee");
    assert_eq!(g.obj(pp).counters, 1, "Poised Practitioner");
    assert_eq!(tokens(&g, 0, "Bird").len(), 1, "Wingblade Disciple");
    assert!(
        g.keywords(adept).has(Keywords::DOUBLE_STRIKE),
        "Equilibrium Adept"
    );
    assert!(g.obj(theirs).tapped, "Wayspeaker Bodyguard");
    let _ = wayspeaker;
}

#[test]
fn wayspeaker_bodyguard_returns_a_cheap_permanent_card() {
    let mut g = table();
    let c = hand(&mut g, 0, "Wayspeaker Bodyguard");
    let cheap = g.add_card(0, tid("Humbling Elder"), Zone::Graveyard); // mana value 1
    let pricey = g.add_card(0, tid("Iceridge Serpent"), Zone::Graveyard); // 5
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    for _ in 0..4 {
        if matches!(g.pending, Pending::ChooseTarget(_)) {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(pricey))),
        "mana value 2 or less"
    );
    act(&mut g, Action::Target(obj(cheap)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(cheap).zone, Zone::Hand);
}

#[test]
fn equilibrium_adept_exiles_the_top_card_to_play() {
    let mut g = table();
    let c = hand(&mut g, 0, "Equilibrium Adept");
    begin(&mut g);
    let top = *g.players[0].library.last().unwrap();
    play(&mut g, c, false, &[], &[Action::Pick(top)]);
    assert_eq!(g.obj(top).zone, Zone::Exile);
    assert!(g.obj(top).playable.is_some());
}

#[test]
fn endure_puts_counters_or_makes_a_spirit() {
    let mut g = table();
    let a = hand(&mut g, 0, "Sandskitter Outrider"); // endures 2
    let b = hand(&mut g, 0, "Sandskitter Outrider");
    begin(&mut g);
    play(&mut g, a, false, &[], &[Action::Yes]);
    assert_eq!(g.obj(a).counters, 2);
    play(&mut g, b, false, &[], &[Action::No]);
    assert_eq!(g.obj(b).counters, 0);
    let spirit = tokens(&g, 0, "Spirit");
    assert_eq!(spirit.len(), 1);
    assert_eq!(
        (g.power(spirit[0]), g.toughness(spirit[0])),
        (2, 2),
        "a 2/2 Spirit"
    );
}

#[test]
fn the_other_endure_creatures() {
    for (name, n) in [
        ("Dusyut Earthcarver", 3),
        ("Fortress Kin-Guard", 1),
        ("Kin-Tree Nurturer", 1),
        ("Inspirited Vanguard", 2),
    ] {
        let mut g = table();
        let c = hand(&mut g, 0, name);
        begin(&mut g);
        play(&mut g, c, false, &[], &[Action::Yes]);
        assert_eq!(g.obj(c).counters, n, "{name}");
    }
    let mut g = table();
    let v = put(&mut g, 0, "Inspirited Vanguard");
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(v));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.obj(v).counters, 2, "endures when it attacks too");
}

#[test]
fn descendant_of_storms_endures_only_if_paid() {
    for (pay, counters) in [(true, 1), (false, 0)] {
        let mut g = table();
        let d = put(&mut g, 0, "Descendant of Storms");
        g.start_turn(5, 0);
        while g.pending != Pending::DeclareAttackers(0) {
            act(&mut g, Action::PassPriority);
        }
        act(&mut g, Action::Attack(d));
        act(&mut g, Action::FinishAttacks);
        let answers: &[Action] = if pay {
            &[Action::Yes, Action::Yes]
        } else {
            &[Action::No]
        };
        resolve_all(&mut g, answers);
        assert_eq!(g.obj(d).counters, counters, "paid: {pay}");
    }
}
