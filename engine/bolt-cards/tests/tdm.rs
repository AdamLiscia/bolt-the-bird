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
