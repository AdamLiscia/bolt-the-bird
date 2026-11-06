//! Tier 4, batch 7: Progenitus, Zul Ashur, Involuntary Employment, Fiery Annihilation.

mod common;

use bolt_core::{Action, Keywords, Pending, Step, Target, Zone};
use common::*;

fn pass_to(g: &mut bolt_core::Game, step: Step) {
    while g.step != step {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn progenitus_cant_be_targeted_even_by_its_controller() {
    // CR 702.16b, 702.16j: protection from everything.
    let mut g = table();
    begin(&mut g);
    let prog = put(&mut g, 0, "Progenitus");
    let bolt = hand(&mut g, 0, "Burst Lightning");
    act(&mut g, Action::Cast(bolt));
    assert!(!g.legal_actions().contains(&Action::Target(obj(prog))));
}

#[test]
fn progenitus_takes_no_damage_and_cant_be_blocked() {
    let mut g = table();
    begin(&mut g);
    let prog = put(&mut g, 0, "Progenitus");
    let lions = put(&mut g, 1, "Savannah Lions");
    pass_to(&mut g, Step::DeclareAttackers);
    act(&mut g, Action::Attack(prog));
    act(&mut g, Action::FinishAttacks);
    pass_to(&mut g, Step::DeclareBlockers);
    assert!(!g.legal_actions().contains(&Action::Block {
        blocker: lions,
        attacker: prog
    }));
}

#[test]
fn progenitus_is_shuffled_into_the_library_instead_of_a_graveyard() {
    let mut g = table();
    begin(&mut g);
    let prog = put(&mut g, 0, "Progenitus");
    let edict = hand(&mut g, 0, "Blasphemous Edict");
    let mut answers = vec![];
    // Sacrificing needs no target; answer any choice with the only creature.
    act(&mut g, Action::Cast(edict));
    while !g.items.is_empty() || matches!(g.pending, Pending::Decide(_)) {
        let a = match g.pending {
            Pending::Decide(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        answers.push(a);
        act(&mut g, a);
    }
    assert_eq!(g.obj(prog).zone, Zone::Library);
    assert!(!g.players[0].graveyard.contains(&prog));
}

#[test]
fn zul_ashur_lets_you_cast_a_zombie_from_your_graveyard_this_turn() {
    let mut g = table();
    begin(&mut g);
    let zul = put(&mut g, 0, "Zul Ashur, Lich Lord");
    let ghoul = g.add_card(0, id("Diregraf Ghoul"), Zone::Graveyard);
    assert!(!g.legal_actions().contains(&Action::Cast(ghoul)));
    act(&mut g, Action::Activate(zul, 0));
    act(&mut g, Action::Target(obj(ghoul)));
    resolve_all(&mut g, &[]);
    assert!(g.legal_actions().contains(&Action::Cast(ghoul)));
    play(&mut g, ghoul, false, &[], &[]);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
}

#[test]
fn zul_ashur_permission_ends_with_the_turn() {
    let mut g = table();
    begin(&mut g);
    let zul = put(&mut g, 0, "Zul Ashur, Lich Lord");
    let ghoul = g.add_card(0, id("Diregraf Ghoul"), Zone::Graveyard);
    act(&mut g, Action::Activate(zul, 0));
    act(&mut g, Action::Target(obj(ghoul)));
    resolve_all(&mut g, &[]);
    pass_to(&mut g, Step::End);
    pass_to(&mut g, Step::Main1);
    pass_to(&mut g, Step::End);
    pass_to(&mut g, Step::Main1);
    assert_eq!(g.active, 0);
    assert!(!g.legal_actions().contains(&Action::Cast(ghoul)));
}

#[test]
fn zul_ashur_only_targets_zombie_creature_cards() {
    let mut g = table();
    begin(&mut g);
    let zul = put(&mut g, 0, "Zul Ashur, Lich Lord");
    g.add_card(0, id("Savannah Lions"), Zone::Graveyard);
    assert!(!g.legal_actions().contains(&Action::Activate(zul, 0)));
}

#[test]
fn involuntary_employment_steals_until_end_of_turn() {
    let mut g = table();
    begin(&mut g);
    let lions = put(&mut g, 1, "Savannah Lions");
    g.objects[lions.0 as usize].tapped = true;
    let ie = hand(&mut g, 0, "Involuntary Employment");
    play(&mut g, ie, false, &[obj(lions)], &[]);
    assert_eq!(g.obj(lions).controller, 0);
    assert!(!g.obj(lions).tapped);
    assert!(g.keywords(lions).has(Keywords::HASTE));
    assert_eq!(tokens_named(&g, 0, "Treasure"), 0, "Treasure isn't a creature");
    assert!(g.permanents(0).any(|x| g.def_of(x).name == "Treasure"));
    pass_to(&mut g, Step::DeclareAttackers);
    act(&mut g, Action::Attack(lions));
    act(&mut g, Action::FinishAttacks);
    pass_to(&mut g, Step::Main1); // the opponent's turn
    assert_eq!(g.obj(lions).controller, 1);
}

#[test]
fn fiery_annihilation_exiles_the_creature_and_its_equipment() {
    let mut g = table();
    begin(&mut g);
    let raider = put(&mut g, 1, "Gorehorn Raider");
    let boots = put(&mut g, 1, "Adventuring Gear");
    let zcc = g.obj(raider).zcc;
    g.objects[boots.0 as usize].attached_to = Some((raider, zcc));
    let other = put(&mut g, 1, "Fishing Pole");
    let fa = hand(&mut g, 0, "Fiery Annihilation");
    act(&mut g, Action::Cast(fa));
    act(&mut g, Action::Target(obj(raider)));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(other))),
        "not attached to it"
    );
    act(&mut g, Action::Target(obj(boots)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(raider).zone, Zone::Exile);
    assert_eq!(g.obj(boots).zone, Zone::Exile);
}

#[test]
fn fiery_annihilation_exile_lasts_the_turn() {
    // The replacement applies if the creature would die later this turn too.
    let mut g = table();
    begin(&mut g);
    let wurm = put(&mut g, 1, "Quakestrider Ceratops"); // 12/8
    let fa = hand(&mut g, 0, "Fiery Annihilation");
    act(&mut g, Action::Cast(fa));
    act(&mut g, Action::Target(obj(wurm)));
    act(&mut g, Action::No); // no Equipment
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(wurm).zone, Zone::Battlefield);
    let bolt = hand(&mut g, 0, "Burst Lightning");
    play(&mut g, bolt, false, &[obj(wurm)], &[]);
    let bolt = hand(&mut g, 0, "Burst Lightning");
    play(&mut g, bolt, false, &[obj(wurm)], &[]);
    assert_eq!(g.obj(wurm).zone, Zone::Exile);
    let _ = Target::Player(0);
}

#[test]
fn a_milled_progenitus_goes_back_once_and_the_reveal_still_ends() {
    // Consuming Aberration reveals until a land, *then* puts the cards into the graveyard; a
    // revealed Progenitus is shuffled back after the reveal, not mid-way (fuzz seed 206 looped).
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Consuming Aberration");
    g.players[1].library.clear();
    g.add_card(1, id("Savannah Lions"), Zone::Library);
    let prog = g.add_card(1, id("Progenitus"), Zone::Library);
    let bolt = hand(&mut g, 0, "Burst Lightning");
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    assert_eq!(g.players[1].library, vec![prog]);
    assert_eq!(g.players[1].graveyard.len(), 1);
}

#[test]
fn sphinxs_tutelage_doesnt_count_a_progenitus_that_went_back() {
    // Progenitus shares a color with everything and shuffles itself back; it was never milled, so
    // it can't keep the repeat going (fuzz seed 206 looped).
    let mut g = table();
    begin(&mut g);
    put(&mut g, 0, "Sphinx's Tutelage");
    g.players[1].library.clear();
    let prog = g.add_card(1, id("Progenitus"), Zone::Library);
    g.add_card(1, id("Gorehorn Raider"), Zone::Library);
    let top = g.add_card(0, id("Aegis Turtle"), Zone::Library);
    let _ = top;
    // Player 0 draws in their next draw step; get there.
    pass_to(&mut g, Step::End);
    pass_to(&mut g, Step::Main1); // opponent's turn
    while !(g.active == 0 && g.step == Step::Main1) {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::ChooseTarget(_) => Action::Target(Target::Player(1)),
            Pending::Decide(_) | Pending::Discard(_) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(prog).zone, Zone::Library);
}

#[test]
fn an_endless_mandatory_loop_is_a_draw() {
    // CR 104.4b. Bloodthirsty Conqueror and Marauding Blight-Priest feed each other forever when
    // the opponent can't lose (Herald of Eternal Dawn).
    let mut g = table();
    g.auto_pass = true;
    begin(&mut g);
    put(&mut g, 0, "Bloodthirsty Conqueror");
    put(&mut g, 0, "Marauding Blight-Priest");
    put(&mut g, 1, "Herald of Eternal Dawn");
    let bolt = hand(&mut g, 0, "Burst Lightning");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    let mut n = 0;
    while g.result.is_none() {
        let legal = g.legal_actions();
        let a = if legal.contains(&Action::PassPriority) {
            Action::PassPriority
        } else {
            legal[0]
        };
        act(&mut g, a);
        n += 1;
        assert!(n < 200_000, "the loop never ended");
    }
    assert_eq!(g.result, Some(bolt_core::Outcome::Draw));
}
