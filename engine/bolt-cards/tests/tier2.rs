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

// ---------------------------------------------------------------- batch 2: decisions

/// Casts `card` (kicked or not) with `targets`, then passes until the stack is empty, answering
/// trigger targets and resolution decisions from `answers` in order.
fn play(g: &mut Game, card: ObjId, kicked: bool, targets: &[Target], answers: &[Action]) {
    act(
        g,
        if kicked {
            Action::CastKicked(card)
        } else {
            Action::Cast(card)
        },
    );
    for &t in targets {
        act(g, Action::Target(t));
    }
    resolve_all(g, answers);
}

fn resolve_all(g: &mut Game, answers: &[Action]) {
    let mut answers = answers.iter();
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) | Pending::Decide(_) => *answers
                .next()
                .unwrap_or_else(|| panic!("no answer for {:?}", g.legal_actions())),
            _ => Action::PassPriority,
        };
        act(g, a);
    }
    assert!(answers.next().is_none(), "unused answers");
}

/// Adds a card to the bottom of a library, so the draw step doesn't draw it.
fn library_card(g: &mut Game, p: usize, name: &str) -> ObjId {
    let c = g.add_card(p, id(name), Zone::Library);
    g.players[p].library.rotate_right(1);
    c
}

#[test]
fn burglar_rat_makes_the_opponent_discard() {
    let mut g = table();
    let rat = hand(&mut g, 0, "Burglar Rat");
    let a = hand(&mut g, 1, "Stab");
    hand(&mut g, 1, "Forest");
    begin(&mut g);
    play(&mut g, rat, false, &[], &[Action::Discard(a)]);
    assert_eq!(g.obj(a).zone, Zone::Graveyard);
    assert_eq!(g.players[1].hand.len(), 1, "the opponent chose which");
}

#[test]
fn icewind_elemental_draws_then_discards() {
    let mut g = table();
    let c = hand(&mut g, 0, "Icewind Elemental");
    let junk = hand(&mut g, 0, "Forest");
    begin(&mut g);
    let before = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[Action::Discard(junk)]);
    assert_eq!(
        g.players[0].hand.len(),
        before - 1,
        "cast it, drew one, discarded the Forest"
    );
    assert_eq!(g.obj(junk).zone, Zone::Graveyard);
}

#[test]
fn incinerating_blast_may_loot() {
    let mut g = table();
    let c = hand(&mut g, 0, "Incinerating Blast");
    let junk = hand(&mut g, 0, "Forest");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    let before = g.players[0].hand.len();
    play(&mut g, c, false, &[obj(angel)], &[Action::Discard(junk)]);
    assert_eq!(g.obj(angel).zone, Zone::Graveyard);
    assert_eq!(g.obj(junk).zone, Zone::Graveyard);
    assert_eq!(
        g.players[0].hand.len(),
        before - 1,
        "cast it, discarded one, drew one"
    );

    let mut g = table();
    let c = hand(&mut g, 0, "Incinerating Blast");
    hand(&mut g, 0, "Forest");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    let before = g.players[0].hand.len();
    play(&mut g, c, false, &[obj(angel)], &[Action::No]);
    assert_eq!(
        g.players[0].hand.len(),
        before - 1,
        "kept the Forest, drew nothing"
    );
}

#[test]
fn wary_thespian_surveils_on_entering_and_dying() {
    let mut g = table();
    let c = hand(&mut g, 0, "Wary Thespian");
    let stab = hand(&mut g, 1, "Stab");
    begin(&mut g);
    let top = *g.players[0].library.last().unwrap();
    play(&mut g, c, false, &[], &[Action::Yes]);
    assert_eq!(g.obj(top).zone, Zone::Graveyard, "surveiled into the graveyard");
    let top = *g.players[0].library.last().unwrap();
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(c)));
    resolve_all(&mut g, &[Action::No]);
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
    assert_eq!(*g.players[0].library.last().unwrap(), top, "kept on top");
}

#[test]
fn lightshell_duo_has_prowess() {
    let mut g = table();
    let duo = put(&mut g, 0, "Lightshell Duo");
    let growth = hand(&mut g, 0, "Giant Growth");
    let lions = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, growth, false, &[obj(lions)], &[]);
    assert_eq!((g.power(duo), g.toughness(duo)), (4, 5), "+1/+1 from prowess");

    let mut g = table();
    let duo = hand(&mut g, 0, "Lightshell Duo");
    begin(&mut g);
    play(&mut g, duo, false, &[], &[Action::Yes, Action::No]);
    assert_eq!(g.players[0].graveyard.len(), 1, "surveil 2: one to the graveyard");
}

#[test]
fn uncharted_voyage_lets_the_owner_choose_top_or_bottom() {
    let mut g = table();
    let c = hand(&mut g, 0, "Uncharted Voyage");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    play(&mut g, c, false, &[obj(angel)], &[Action::No, Action::No]);
    assert_eq!(g.players[1].library[0], angel, "the owner chose the bottom");
}

#[test]
fn refute_counters_and_loots() {
    let mut g = table();
    let stab = hand(&mut g, 0, "Stab");
    let refute = hand(&mut g, 1, "Refute");
    let junk = hand(&mut g, 1, "Forest");
    let lions = put(&mut g, 1, "Savannah Lions");
    begin(&mut g);
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(lions)));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(refute));
    act(&mut g, Action::Target(obj(stab)));
    resolve_all(&mut g, &[Action::Discard(junk)]);
    assert_eq!(g.obj(stab).zone, Zone::Graveyard);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield, "Stab never resolved");
}

#[test]
fn essence_scatter_only_counters_creatures() {
    let mut g = table();
    let stab = hand(&mut g, 0, "Stab");
    let lions = hand(&mut g, 0, "Savannah Lions");
    let scatter = hand(&mut g, 1, "Essence Scatter");
    let target = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(target)));
    act(&mut g, Action::PassPriority);
    assert!(
        !g.legal_actions().contains(&Action::Cast(scatter)),
        "no creature spell to target"
    );
    resolve_all(&mut g, &[]);
    act(&mut g, Action::Cast(lions));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(scatter));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
}

#[test]
fn reclamation_sage_may_destroy_an_enchantment() {
    let mut g = table();
    let sage = hand(&mut g, 0, "Reclamation Sage");
    let light = put(&mut g, 1, "Banishing Light");
    begin(&mut g);
    play(
        &mut g,
        sage,
        false,
        &[],
        &[Action::Target(obj(light)), Action::Yes],
    );
    assert_eq!(g.obj(light).zone, Zone::Graveyard);

    let mut g = table();
    let sage = hand(&mut g, 0, "Reclamation Sage");
    begin(&mut g);
    play(&mut g, sage, false, &[], &[]); // no target: the trigger is removed
    assert_eq!(g.obj(sage).zone, Zone::Battlefield);
}

#[test]
fn affectionate_indrik_fights() {
    let mut g = table();
    let indrik = hand(&mut g, 0, "Affectionate Indrik");
    let lions = put(&mut g, 1, "Savannah Lions");
    begin(&mut g);
    play(
        &mut g,
        indrik,
        false,
        &[],
        &[Action::Target(obj(lions)), Action::Yes],
    );
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
    assert_eq!(g.obj(indrik).damage, 2, "fighting is both ways");
}

#[test]
fn meteor_golem_cannot_hit_lands() {
    let mut g = table();
    let golem = hand(&mut g, 0, "Meteor Golem");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    act(&mut g, Action::Cast(golem));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert_eq!(g.legal_actions(), vec![Action::Target(obj(angel))]);
}

#[test]
fn bigfin_bouncer_returns_to_hand() {
    let mut g = table();
    let c = hand(&mut g, 0, "Bigfin Bouncer");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Target(obj(angel))]);
    assert_eq!(g.obj(angel).zone, Zone::Hand);
    assert!(g.players[1].hand.contains(&angel));
}

#[test]
fn campus_guide_puts_a_basic_on_top() {
    let mut g = table();
    let c = hand(&mut g, 0, "Campus Guide");
    let forest = library_card(&mut g, 0, "Forest");
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Yes, Action::Pick(forest)]);
    assert_eq!(*g.players[0].library.last().unwrap(), forest);
}

#[test]
fn solemn_simulacrum_ramps_and_replaces_itself() {
    let mut g = table();
    let c = hand(&mut g, 0, "Solemn Simulacrum");
    let stab = hand(&mut g, 1, "Stab");
    let forest = library_card(&mut g, 0, "Forest");
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Yes, Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert!(g.obj(forest).tapped);
    // Two Stabs' worth: -2/-2 kills the 2/2, and it may draw.
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(c)));
    let before = g.players[0].hand.len();
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.players[0].hand.len(), before + 1);
}

#[test]
fn micromancer_finds_a_one_mana_spell() {
    let mut g = table();
    let c = hand(&mut g, 0, "Micromancer");
    let stab = library_card(&mut g, 0, "Stab");
    let downfall = library_card(&mut g, 0, "Hero's Downfall");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    resolve_all(&mut g, &[Action::Yes, Action::Pick(stab)]);
    assert_eq!(g.obj(stab).zone, Zone::Hand);
    assert_eq!(g.obj(downfall).zone, Zone::Library, "three mana: not eligible");
}

#[test]
fn tutors() {
    let mut g = table();
    let demon = hand(&mut g, 0, "Rune-Scarred Demon");
    let x = library_card(&mut g, 0, "Serra Angel");
    begin(&mut g);
    play(&mut g, demon, false, &[], &[Action::Pick(x)]);
    assert_eq!(g.obj(x).zone, Zone::Hand);

    let mut g = table();
    let tutor = hand(&mut g, 0, "Grim Tutor");
    let x = library_card(&mut g, 0, "Serra Angel");
    begin(&mut g);
    play(&mut g, tutor, false, &[], &[Action::Pick(x)]);
    assert_eq!(g.obj(x).zone, Zone::Hand);
    assert_eq!(g.players[0].life, 17);
}

#[test]
fn grow_from_the_ashes_kicked_finds_two() {
    let mut g = table();
    let c = hand(&mut g, 0, "Grow from the Ashes");
    let f1 = library_card(&mut g, 0, "Forest");
    let f2 = library_card(&mut g, 0, "Island");
    begin(&mut g);
    play(&mut g, c, true, &[], &[Action::Pick(f1), Action::Pick(f2)]);
    assert_eq!(
        (g.obj(f1).zone, g.obj(f2).zone),
        (Zone::Battlefield, Zone::Battlefield)
    );
    assert!(!g.obj(f1).tapped, "untapped");
}

#[test]
fn pilfer_takes_a_nonland_card() {
    let mut g = table();
    let c = hand(&mut g, 0, "Pilfer");
    let stab = hand(&mut g, 1, "Stab");
    hand(&mut g, 1, "Forest");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert_eq!(
        g.legal_actions(),
        vec![Action::Pick(stab)],
        "lands can't be chosen"
    );
    act(&mut g, Action::Pick(stab));
    assert_eq!(g.obj(stab).zone, Zone::Graveyard);
}

#[test]
fn hare_apparent_counts_the_other_hares() {
    let mut g = table();
    put(&mut g, 0, "Hare Apparent");
    put(&mut g, 0, "Hare Apparent");
    let c = hand(&mut g, 0, "Hare Apparent");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(tokens_named(&g, 0, "Rabbit"), 2);
}

#[test]
fn snakeskin_veil_protects() {
    let mut g = table();
    let veil = hand(&mut g, 1, "Snakeskin Veil");
    let stab = hand(&mut g, 0, "Stab");
    let lions = put(&mut g, 1, "Savannah Lions");
    begin(&mut g);
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(lions)));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(veil));
    act(&mut g, Action::Target(obj(lions)));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority); // the Veil resolves first
    assert_eq!((g.power(lions), g.toughness(lions)), (3, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield, "hexproof made Stab fizzle");
}

#[test]
fn stroke_of_midnight_leaves_a_human() {
    let mut g = table();
    let c = hand(&mut g, 0, "Stroke of Midnight");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    play(&mut g, c, false, &[obj(angel)], &[]);
    assert_eq!(g.obj(angel).zone, Zone::Graveyard);
    assert_eq!(tokens_named(&g, 1, "Human"), 1, "its controller gets the token");
}

// ---------------------------------------------------------------- Food and Treasure

fn token_of(g: &Game, p: usize, name: &str) -> ObjId {
    g.permanents(p)
        .find(|&c| g.def_of(c).token && g.def_of(c).name == name)
        .expect("the token")
}

#[test]
fn bake_into_a_pie_and_its_food() {
    let mut g = table();
    let c = hand(&mut g, 0, "Bake into a Pie");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    play(&mut g, c, false, &[obj(angel)], &[]);
    assert_eq!(g.obj(angel).zone, Zone::Graveyard);
    let food = token_of(&g, 0, "Food");
    // {2}, {T}, sacrifice: gain 3 life. It uses the stack.
    act(&mut g, Action::Activate(food, 0));
    assert_eq!(
        g.obj(food).zone,
        Zone::Gone,
        "sacrificed as a cost; a token ceases to exist"
    );
    assert_eq!(g.items.len(), 1);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 23);
}

#[test]
fn treasure_pays_for_a_spell_only_when_needed() {
    // CR 111.10a. Gleaming Barrier dies and leaves a Treasure, which then pays for white mana that
    // the Mountains can't make. The spare Mountain stays untapped.
    let mut g = Game::empty(fdn(), 1, 0);
    g.auto_pass = false;
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, id("Aegis Turtle"), Zone::Library);
        }
    }
    for _ in 0..6 {
        put(&mut g, 0, "Mountain");
    }
    let barrier = put(&mut g, 0, "Gleaming Barrier");
    let blast = hand(&mut g, 0, "Incinerating Blast");
    let lions = hand(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    assert!(
        !g.legal_actions().contains(&Action::Cast(lions)),
        "no white mana yet"
    );
    play(&mut g, blast, false, &[obj(barrier)], &[Action::No]);
    assert_eq!(g.obj(barrier).zone, Zone::Graveyard);
    let treasure = token_of(&g, 0, "Treasure");
    act(&mut g, Action::Cast(lions));
    assert_eq!(g.obj(treasure).zone, Zone::Gone, "sacrificed for mana");
    let untapped = g
        .permanents(0)
        .filter(|&c| g.def_of(c).name == "Mountain" && !g.obj(c).tapped)
        .count();
    assert_eq!(untapped, 1);
}

#[test]
fn an_offer_you_cant_refuse_pays_the_victim() {
    let mut g = table();
    let stab = hand(&mut g, 0, "Stab");
    let offer = hand(&mut g, 1, "An Offer You Can't Refuse");
    let lions = put(&mut g, 1, "Savannah Lions");
    begin(&mut g);
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(lions)));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(offer));
    act(&mut g, Action::Target(obj(stab)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield);
    let treasures = g
        .permanents(0)
        .filter(|&c| g.def_of(c).name == "Treasure")
        .count();
    assert_eq!(treasures, 2, "the countered spell's controller gets them");
}

// ---------------------------------------------------------------- additional costs, reductions, flashback

/// A table where player 0 has exactly the lands named.
fn lands(names: &[&str]) -> Game {
    let mut g = Game::empty(fdn(), 1, 0);
    g.auto_pass = false;
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, id("Aegis Turtle"), Zone::Library);
        }
    }
    for n in names {
        put(&mut g, 0, n);
    }
    g
}

#[test]
fn arbiter_of_woe_needs_a_sacrifice() {
    let mut g = table();
    let arbiter = hand(&mut g, 0, "Arbiter of Woe");
    let victim = hand(&mut g, 1, "Stab");
    begin(&mut g);
    assert!(!g
        .legal_actions()
        .iter()
        .any(|a| matches!(a, Action::CastWith(c, _) if *c == arbiter)));
    let lions = put(&mut g, 0, "Savannah Lions");
    assert!(g.legal_actions().contains(&Action::CastWith(arbiter, lions)));
    assert!(
        !g.legal_actions().contains(&Action::Cast(arbiter)),
        "the sacrifice isn't optional"
    );
    act(&mut g, Action::CastWith(arbiter, lions));
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
    let before = g.players[0].hand.len();
    resolve_all(&mut g, &[Action::Discard(victim)]);
    assert_eq!((g.players[0].life, g.players[1].life), (22, 18));
    assert_eq!(g.players[0].hand.len(), before + 1);
    assert_eq!(g.obj(victim).zone, Zone::Graveyard);
}

#[test]
fn eaten_alive_sacrifice_or_pay() {
    // Just a Swamp: only the sacrifice mode is affordable.
    let mut g = lands(&["Swamp"]);
    let eat = hand(&mut g, 0, "Eaten Alive");
    let lions = put(&mut g, 0, "Savannah Lions");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    assert!(!g.legal_actions().contains(&Action::Cast(eat)));
    act(&mut g, Action::CastWith(eat, lions));
    act(&mut g, Action::Target(obj(angel)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(angel).zone, Zone::Exile);

    // Five Swamps: pay {3}{B} more instead, no sacrifice.
    let mut g = lands(&["Swamp", "Swamp", "Swamp", "Swamp", "Swamp"]);
    let eat = hand(&mut g, 0, "Eaten Alive");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    act(&mut g, Action::Cast(eat));
    act(&mut g, Action::Target(obj(angel)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(angel).zone, Zone::Exile);
    assert_eq!(g.permanents(0).filter(|&c| g.obj(c).tapped).count(), 5);
}

#[test]
fn thrill_of_possibility_discards_as_a_cost() {
    let mut g = table();
    let thrill = hand(&mut g, 0, "Thrill of Possibility");
    let junk = hand(&mut g, 0, "Forest");
    begin(&mut g);
    let before = g.players[0].hand.len();
    act(&mut g, Action::CastWith(thrill, junk));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(junk).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), before - 2 + 2);
}

#[test]
fn arcane_epiphany_is_cheaper_with_a_wizard() {
    let five = ["Island", "Island", "Island", "Island"];
    let mut g = lands(&five);
    let c = hand(&mut g, 0, "Arcane Epiphany");
    begin(&mut g);
    assert!(
        !g.legal_actions().contains(&Action::Cast(c)),
        "four lands, costs five"
    );
    put(&mut g, 0, "Micromancer"); // a Human Wizard
    assert!(g.legal_actions().contains(&Action::Cast(c)));
    let before = g.players[0].hand.len();
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), before - 1 + 3);
}

#[test]
fn luminous_rebuke_is_cheaper_against_a_tapped_creature() {
    let mut g = lands(&["Plains", "Plains"]);
    let c = hand(&mut g, 0, "Luminous Rebuke");
    let fresh = put(&mut g, 1, "Serra Angel");
    let tired = put(&mut g, 1, "Savannah Lions");
    g.objects[tired.0 as usize].tapped = true;
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    assert_eq!(
        g.legal_actions(),
        vec![Action::Target(obj(tired))],
        "only the affordable target"
    );
    let _ = fresh;
    act(&mut g, Action::Target(obj(tired)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(tired).zone, Zone::Graveyard);
}

#[test]
fn think_twice_flashes_back_then_is_exiled() {
    // CR 702.34a.
    let mut g = table();
    let c = hand(&mut g, 0, "Think Twice");
    begin(&mut g);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
    act(&mut g, Action::CastFlashback(c));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).zone, Zone::Exile);
}

// ---------------------------------------------------------------- up to, graveyards, links, granted triggers

#[test]
fn felidar_savior_counters_up_to_two_others() {
    let mut g = table();
    let c = hand(&mut g, 0, "Felidar Savior");
    let a = put(&mut g, 0, "Savannah Lions");
    let b = put(&mut g, 0, "Healer's Hawk");
    begin(&mut g);
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Target(obj(c))), "other creatures only");
    assert!(legal.contains(&Action::No), "up to two: zero is fine");
    act(&mut g, Action::Target(obj(a)));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(a))),
        "not the same one twice"
    );
    act(&mut g, Action::Target(obj(b)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(a), g.power(b)), (3, 2));

    // Choosing just one.
    let mut g = table();
    let c = hand(&mut g, 0, "Felidar Savior");
    let a = put(&mut g, 0, "Savannah Lions");
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Target(obj(a)), Action::No]);
    assert_eq!(g.power(a), 3);
}

#[test]
fn macabre_waltz_returns_creature_cards_from_your_graveyard() {
    let mut g = table();
    let w = hand(&mut g, 0, "Macabre Waltz");
    let lions = g.add_card(0, id("Savannah Lions"), Zone::Graveyard);
    let stab = g.add_card(0, id("Stab"), Zone::Graveyard);
    let theirs = g.add_card(1, id("Serra Angel"), Zone::Graveyard);
    begin(&mut g);
    act(&mut g, Action::Cast(w));
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Target(obj(lions))));
    assert!(!legal.contains(&Action::Target(obj(stab))), "creature cards only");
    assert!(
        !legal.contains(&Action::Target(obj(theirs))),
        "your graveyard only"
    );
    act(&mut g, Action::Target(obj(lions)));
    act(&mut g, Action::No);
    resolve_all(&mut g, &[Action::Discard(lions)]);
    assert_eq!(
        g.obj(lions).zone,
        Zone::Graveyard,
        "returned, then discarded by choice"
    );
}

#[test]
fn soul_shackled_zombie_drains_if_a_creature_is_exiled() {
    let mut g = table();
    let z = hand(&mut g, 0, "Soul-Shackled Zombie");
    let angel = g.add_card(1, id("Serra Angel"), Zone::Graveyard);
    let stab = g.add_card(1, id("Stab"), Zone::Graveyard);
    let mine = g.add_card(0, id("Savannah Lions"), Zone::Graveyard);
    begin(&mut g);
    act(&mut g, Action::Cast(z));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Target(obj(angel)));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(mine))),
        "a single graveyard"
    );
    act(&mut g, Action::Target(obj(stab)));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(angel).zone, g.obj(stab).zone), (Zone::Exile, Zone::Exile));
    assert_eq!((g.players[0].life, g.players[1].life), (22, 18));
}

#[test]
fn faebloom_trick_taps_when_you_do() {
    // CR 603.12: the reflexive trigger chooses its target after the spell resolves.
    let mut g = table();
    let c = hand(&mut g, 0, "Faebloom Trick");
    let angel = put(&mut g, 1, "Serra Angel");
    begin(&mut g);
    play(&mut g, c, false, &[], &[Action::Target(obj(angel))]);
    assert_eq!(tokens_named(&g, 0, "Faerie"), 2);
    assert!(g.obj(angel).tapped);
}

#[test]
fn banishing_light_returns_the_card_when_it_leaves() {
    // CR 610.3.
    let mut g = table();
    let light = hand(&mut g, 0, "Banishing Light");
    let angel = put(&mut g, 1, "Serra Angel");
    let wings = hand(&mut g, 1, "Broken Wings");
    begin(&mut g);
    play(&mut g, light, false, &[], &[Action::Target(obj(angel))]);
    assert_eq!(g.obj(angel).zone, Zone::Exile);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(wings));
    act(&mut g, Action::Target(obj(light)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(light).zone, Zone::Graveyard);
    assert_eq!(
        g.obj(angel).zone,
        Zone::Battlefield,
        "back under its owner's control"
    );
    assert_eq!(g.obj(angel).controller, 1);
}

#[test]
fn fake_your_own_death_brings_it_back() {
    let mut g = table();
    let fake = hand(&mut g, 0, "Fake Your Own Death");
    let lions = put(&mut g, 0, "Savannah Lions");
    let stab = hand(&mut g, 1, "Stab");
    begin(&mut g);
    play(&mut g, fake, false, &[obj(lions)], &[]);
    assert_eq!(g.power(lions), 4);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(stab));
    act(&mut g, Action::Target(obj(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield, "died and came back");
    assert!(g.obj(lions).tapped);
    assert_eq!(g.power(lions), 2, "a new object: the pump is gone");
    let treasure = g
        .permanents(0)
        .filter(|&c| g.def_of(c).name == "Treasure")
        .count();
    assert_eq!(treasure, 1);
}
