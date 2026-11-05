//! One test per scripted card, written from its rules text.

use bolt_cards::{fdn, index_of};
use bolt_core::{Action, Event, Game, ObjId, Pending, Step, Target, Zone};

fn id(name: &str) -> usize {
    index_of(name).unwrap_or_else(|| panic!("{name}"))
}

/// Player 0's first main phase on turn 3. Each player has 5 of each basic land, and libraries of
/// Aegis Turtles. Every priority stop is visible (no auto-pass).
fn table() -> Game {
    let mut g = Game::empty(fdn(), 1, 0);
    g.auto_pass = false;
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, id("Aegis Turtle"), Zone::Library);
        }
        for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            for _ in 0..5 {
                g.add_card(p, id(land), Zone::Battlefield);
            }
        }
    }
    g
}

fn begin(g: &mut Game) {
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(g, Action::PassPriority);
    }
}

fn act(g: &mut Game, a: Action) -> Vec<Event> {
    g.apply(a)
        .unwrap_or_else(|e| panic!("{e}; legal: {:?}", g.legal_actions()))
}

fn put(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, id(name), Zone::Battlefield)
}

fn hand(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, id(name), Zone::Hand)
}

/// Casts a card with the given targets, then passes until the stack is empty. Targets for
/// triggers that come up are taken from `then`.
fn cast(g: &mut Game, card: ObjId, targets: &[Target], then: &[Target]) -> Vec<Event> {
    let mut ev = act(g, Action::Cast(card));
    for &t in targets {
        ev.extend(act(g, Action::Target(t)));
    }
    let mut then = then.iter();
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_)) {
        if let Pending::ChooseTarget(_) = g.pending {
            ev.extend(act(
                g,
                Action::Target(*then.next().expect("a target for the trigger")),
            ));
        } else {
            ev.extend(act(g, Action::PassPriority));
        }
    }
    ev
}

fn obj(id: ObjId) -> Target {
    Target::Object(id)
}

fn tokens_named(g: &Game, p: usize, name: &str) -> usize {
    g.creatures(p)
        .filter(|&c| g.def_of(c).token && g.def_of(c).name == name)
        .count()
}

#[test]
fn stab() {
    let mut g = table();
    let s = hand(&mut g, 0, "Stab");
    let bear = put(&mut g, 1, "Savannah Lions");
    let big = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    cast(&mut g, s, &[obj(big)], &[]);
    assert_eq!((g.power(big), g.toughness(big)), (2, 2));
    let s2 = hand(&mut g, 0, "Stab");
    cast(&mut g, s2, &[obj(bear)], &[]);
    assert_eq!(g.obj(bear).zone, Zone::Graveyard, "toughness 0 dies (CR 704.5f)");
}

#[test]
fn giant_growth() {
    let mut g = table();
    let c = hand(&mut g, 0, "Giant Growth");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    cast(&mut g, c, &[obj(lions)], &[]);
    assert_eq!((g.power(lions), g.toughness(lions)), (5, 4));
}

#[test]
fn heros_downfall() {
    let mut g = table();
    let c = hand(&mut g, 0, "Hero's Downfall");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    assert!(!g.legal_actions().is_empty());
    act(&mut g, Action::Cast(c));
    assert!(
        !g.legal_actions().contains(&Action::Target(Target::Player(1))),
        "not a player"
    );
    act(&mut g, Action::Target(obj(angel)));
    while !g.items.is_empty() {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(angel).zone, Zone::Graveyard);
}

#[test]
fn boltwave() {
    let mut g = table();
    let c = hand(&mut g, 0, "Boltwave");
    begin(&mut g);
    cast(&mut g, c, &[], &[]);
    assert_eq!((g.players[0].life, g.players[1].life), (20, 17));
}

#[test]
fn burst_lightning() {
    let mut g = table();
    let c = hand(&mut g, 0, "Burst Lightning");
    begin(&mut g);
    cast(&mut g, c, &[Target::Player(1)], &[]);
    assert_eq!(g.players[1].life, 18);
    let k = hand(&mut g, 0, "Burst Lightning");
    act(&mut g, Action::CastKicked(k));
    act(&mut g, Action::Target(Target::Player(1)));
    while !g.items.is_empty() {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[1].life, 14, "kicked: 4 damage");
}

#[test]
fn joust_through_needs_a_creature_in_combat() {
    let mut g = table();
    let c = hand(&mut g, 1, "Joust Through");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    // Before combat there's no legal target, so it can't be cast.
    act(&mut g, Action::PassPriority);
    assert!(!g.legal_actions().contains(&Action::Cast(c)));
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(lions));
    act(&mut g, Action::FinishAttacks);
    act(&mut g, Action::PassPriority);
    assert_eq!(g.pending, Pending::Priority(1));
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(obj(lions)));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
    assert_eq!(g.players[1].life, 21);
}

#[test]
fn seismic_rupture_spares_fliers() {
    let mut g = table();
    let c = hand(&mut g, 0, "Seismic Rupture");
    let lions = put(&mut g, 1, "Savannah Lions");
    let mine = put(&mut g, 0, "Savannah Lions");
    let hawk = put(&mut g, 1, "Healer's Hawk");
    begin(&mut g);
    cast(&mut g, c, &[], &[]);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
    assert_eq!(
        g.obj(mine).zone,
        Zone::Graveyard,
        "each creature, including your own"
    );
    assert_eq!(g.obj(hawk).zone, Zone::Battlefield);
}

#[test]
fn hidetsugus_second_rite() {
    let mut g = table();
    let c = hand(&mut g, 0, "Hidetsugu's Second Rite");
    begin(&mut g);
    cast(&mut g, c, &[Target::Player(1)], &[]);
    assert_eq!(g.players[1].life, 20, "not at exactly 10: nothing happens");
    g.players[1].life = 10;
    let c2 = hand(&mut g, 0, "Hidetsugu's Second Rite");
    cast(&mut g, c2, &[Target::Player(1)], &[]);
    assert!(g.is_over());
}

#[test]
fn make_your_move_needs_power_four() {
    let mut g = table();
    let c = hand(&mut g, 0, "Make Your Move");
    let lions = put(&mut g, 1, "Savannah Lions");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Target(obj(lions))));
    assert!(legal.contains(&Action::Target(obj(angel))));
}

#[test]
fn broken_wings_needs_flying() {
    let mut g = table();
    let c = hand(&mut g, 0, "Broken Wings");
    let lions = put(&mut g, 1, "Savannah Lions");
    let hawk = put(&mut g, 1, "Healer's Hawk");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    assert_eq!(g.legal_actions(), vec![Action::Target(obj(hawk))]);
    let _ = lions;
}

#[test]
fn helpful_hunter_draws() {
    let mut g = table();
    let c = hand(&mut g, 0, "Helpful Hunter");
    begin(&mut g);
    let before = g.players[0].hand.len();
    cast(&mut g, c, &[], &[]);
    assert_eq!(g.players[0].hand.len(), before, "cast one, drew one");
}

#[test]
fn fleeting_distraction() {
    let mut g = table();
    let c = hand(&mut g, 0, "Fleeting Distraction");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    let before = g.players[0].hand.len();
    cast(&mut g, c, &[obj(angel)], &[]);
    assert_eq!(g.power(angel), 3);
    assert_eq!(g.players[0].hand.len(), before);
}

#[test]
fn felling_blow() {
    let mut g = table();
    let c = hand(&mut g, 0, "Felling Blow");
    let mine = put(&mut g, 0, "Brazen Scourge"); // 3/3, becomes 4/4
    let theirs = put(&mut g, 1, "Serra Angel"); // 4/4
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(theirs))),
        "first target is yours"
    );
    act(&mut g, Action::Target(obj(mine)));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(mine))),
        "second is theirs"
    );
    act(&mut g, Action::Target(obj(theirs)));
    while !g.items.is_empty() {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.power(mine), 4);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.obj(mine).damage, 0, "one-sided: not a fight");
}

#[test]
fn infestation_sage_leaves_an_insect() {
    let mut g = table();
    let stab = hand(&mut g, 0, "Stab");
    let sage = put(&mut g, 1, "Infestation Sage");
    begin(&mut g);
    cast(&mut g, stab, &[obj(sage)], &[]);
    assert_eq!(tokens_named(&g, 1, "Insect"), 1);
    let insect = g.creatures(1).find(|&c| g.def_of(c).token).unwrap();
    assert!(g.keywords(insect).has(bolt_core::Keywords::FLYING));
}

#[test]
fn token_makers() {
    for (card, token, n) in [
        ("Guarded Heir", "Knight", 2),
        ("Prideful Parent", "Cat", 1),
        ("Dragon Trainer", "Dragon", 1),
        ("Resolute Reinforcements", "Soldier", 1),
    ] {
        let mut g = table();
        let c = hand(&mut g, 0, card);
        begin(&mut g);
        cast(&mut g, c, &[], &[]);
        assert_eq!(tokens_named(&g, 0, token), n, "{card}");
    }
}

#[test]
fn tokens_cease_to_exist_off_the_battlefield() {
    // CR 704.5d.
    let mut g = table();
    let c = hand(&mut g, 0, "Prideful Parent");
    let stab = hand(&mut g, 0, "Stab");
    begin(&mut g);
    cast(&mut g, c, &[], &[]);
    let cat = g.creatures(0).find(|&c| g.def_of(c).token).unwrap();
    cast(&mut g, stab, &[obj(cat)], &[]);
    assert_eq!(g.obj(cat).zone, Zone::Gone);
    g.check_invariants().unwrap();
}

#[test]
fn angel_of_finality_exiles_a_graveyard() {
    let mut g = table();
    let stab = hand(&mut g, 0, "Stab");
    let angel = hand(&mut g, 0, "Angel of Finality");
    let lions = put(&mut g, 1, "Savannah Lions");
    begin(&mut g);
    cast(&mut g, stab, &[obj(lions)], &[]);
    assert_eq!(g.players[1].graveyard.len(), 1);
    cast(&mut g, angel, &[], &[Target::Player(1)]);
    assert!(g.players[1].graveyard.is_empty());
    assert_eq!(g.obj(lions).zone, Zone::Exile);
}

#[test]
fn raid_needs_an_attack() {
    // CR 603.4: no attack, no trigger.
    let mut g = table();
    let raider = hand(&mut g, 0, "Gorehorn Raider");
    begin(&mut g);
    cast(&mut g, raider, &[], &[]);
    assert_eq!(g.players[1].life, 20);

    // Attack first, then cast it in the second main phase.
    let mut g = table();
    let raider = hand(&mut g, 0, "Gorehorn Raider");
    let buccaneer = hand(&mut g, 0, "Skyship Buccaneer");
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
    assert_eq!(g.players[1].life, 18);
    cast(&mut g, raider, &[], &[Target::Player(1)]);
    assert_eq!(g.players[1].life, 16);
    let before = g.players[0].hand.len();
    cast(&mut g, buccaneer, &[], &[]);
    assert_eq!(g.players[0].hand.len(), before, "cast one, drew one");
}

#[test]
fn dwynens_elite_needs_another_elf() {
    let mut g = table();
    let c = hand(&mut g, 0, "Dwynen's Elite");
    begin(&mut g);
    cast(&mut g, c, &[], &[]);
    assert_eq!(tokens_named(&g, 0, "Elf Warrior"), 0);

    let mut g = table();
    let c = hand(&mut g, 0, "Dwynen's Elite");
    put(&mut g, 0, "Infestation Sage"); // an Elf Warlock
    begin(&mut g);
    cast(&mut g, c, &[], &[]);
    assert_eq!(tokens_named(&g, 0, "Elf Warrior"), 1);
}
