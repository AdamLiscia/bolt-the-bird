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
