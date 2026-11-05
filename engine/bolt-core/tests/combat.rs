//! Combat rules (CR 506-511) and the evergreen keywords (CR 702).

mod common;

use bolt_core::Action;
use common::*;

#[test]
fn unblocked_attacker_damages_the_defending_player() {
    // CR 510.1a: an unblocked creature assigns its combat damage to the player it attacks.
    let mut g = board();
    let bear = put(&mut g, 0, BEAR);
    fight(&mut g, &[bear], &[]);
    assert_eq!(g.players[1].life, 18);
}

#[test]
fn attacking_taps_the_creature() {
    // CR 508.1f.
    let mut g = board();
    let bear = put(&mut g, 0, BEAR);
    to_attacks(&mut g);
    act(&mut g, Action::Attack(bear));
    assert!(g.obj(bear).tapped);
}

#[test]
fn blocked_creatures_trade() {
    // CR 510.1c-d, 704.5g: blocked and blocking creatures deal damage to each other.
    let mut g = board();
    let a = put(&mut g, 0, BEAR);
    let b = put(&mut g, 1, BEAR);
    fight(&mut g, &[a], &[(b, a)]);
    assert!(!alive(&g, a) && !alive(&g, b));
    assert_eq!(g.players[1].life, 20);
}

#[test]
fn summoning_sick_creatures_cannot_attack_but_haste_can() {
    // CR 302.6, 702.10b.
    let mut g = board();
    let sick = put(&mut g, 0, BEAR);
    let runner = put(&mut g, 0, RUNNER);
    g.objects[sick.0 as usize].since_turn = 3;
    g.objects[runner.0 as usize].since_turn = 3;
    to_attacks(&mut g);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Attack(runner)));
    assert!(!legal.contains(&Action::Attack(sick)));
}

#[test]
fn defender_cannot_attack() {
    // CR 702.3b.
    let mut g = board();
    let wall = put(&mut g, 0, WALL);
    let bear = put(&mut g, 0, BEAR);
    to_attacks(&mut g);
    assert!(!g.legal_actions().contains(&Action::Attack(wall)));
    assert!(g.legal_actions().contains(&Action::Attack(bear)));
}

#[test]
fn flying_is_blocked_only_by_flying_or_reach() {
    // CR 702.9b, 702.17b.
    let mut g = board();
    let bird = put(&mut g, 0, BIRD);
    let bear = put(&mut g, 1, BEAR);
    let spider = put(&mut g, 1, SPIDER);
    to_attacks(&mut g);
    act(&mut g, Action::Attack(bird));
    act(&mut g, Action::FinishAttacks);
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Block {
        blocker: bear,
        attacker: bird
    }));
    assert!(legal.contains(&Action::Block {
        blocker: spider,
        attacker: bird
    }));
}

#[test]
fn first_strike_kills_before_regular_damage() {
    // CR 702.7b, 510.4: a first-strike damage step happens before the regular one.
    let mut g = board();
    let knight = put(&mut g, 0, KNIGHT);
    let bear = put(&mut g, 1, BEAR);
    fight(&mut g, &[knight], &[(bear, knight)]);
    assert!(alive(&g, knight), "the bear died before it could deal damage");
    assert!(!alive(&g, bear));
}

#[test]
fn double_strike_deals_damage_twice() {
    // CR 702.4b.
    let mut g = board();
    let d = put(&mut g, 0, DOUBLER);
    fight(&mut g, &[d], &[]);
    assert_eq!(g.players[1].life, 16);
}

#[test]
fn deathtouch_kills_with_any_damage() {
    // CR 702.2b, 704.5h.
    let mut g = board();
    let asp = put(&mut g, 0, ASP);
    let ogre = put(&mut g, 1, OGRE);
    fight(&mut g, &[asp], &[(ogre, asp)]);
    assert!(!alive(&g, ogre) && !alive(&g, asp));
}

#[test]
fn trample_assigns_the_excess_to_the_player() {
    // CR 702.19b-c: lethal damage to blockers, the rest to the player.
    let mut g = board();
    let t = put(&mut g, 0, TRAMPLER);
    let bear = put(&mut g, 1, BEAR);
    fight(&mut g, &[t], &[(bear, t)]);
    assert!(!alive(&g, bear));
    assert_eq!(g.players[1].life, 17);
}

#[test]
fn deathtouch_with_trample_needs_only_one_damage_per_blocker() {
    // CR 702.2c: with deathtouch, 1 damage counts as lethal for trample assignment.
    let mut g = board();
    let s = put(&mut g, 0, STING);
    let ogre = put(&mut g, 1, OGRE);
    fight(&mut g, &[s], &[(ogre, s)]);
    assert!(!alive(&g, ogre));
    assert_eq!(g.players[1].life, 17);
}

#[test]
fn blocked_creature_without_trample_deals_nothing_once_its_blocker_is_gone() {
    // CR 509.1h, 510.1c: it stays blocked. A double striker blocked by a bear kills it in the
    // first-strike step, and the second hit has nowhere to go.
    let mut g = board();
    let d = put(&mut g, 0, DOUBLER);
    let bear = put(&mut g, 1, BEAR);
    fight(&mut g, &[d], &[(bear, d)]);
    assert!(!alive(&g, bear));
    assert!(alive(&g, d));
    assert_eq!(g.players[1].life, 20);
}

#[test]
fn lifelink_gains_life_equal_to_damage() {
    // CR 702.15b.
    let mut g = board();
    let l = put(&mut g, 0, LINKER);
    fight(&mut g, &[l], &[]);
    assert_eq!((g.players[0].life, g.players[1].life), (22, 18));
}

#[test]
fn menace_needs_two_blockers() {
    // CR 702.111b.
    let mut g = board();
    let brute = put(&mut g, 0, BRUTE);
    let b1 = put(&mut g, 1, BEAR);
    to_attacks(&mut g);
    act(&mut g, Action::Attack(brute));
    act(&mut g, Action::FinishAttacks);
    // One possible blocker: blocking isn't offered, so there is no declaration to make.
    assert!(!g.legal_actions().contains(&Action::Block {
        blocker: b1,
        attacker: brute
    }));

    let mut g = board();
    let brute = put(&mut g, 0, BRUTE);
    let b1 = put(&mut g, 1, BEAR);
    let b2 = put(&mut g, 1, BEAR);
    to_attacks(&mut g);
    act(&mut g, Action::Attack(brute));
    act(&mut g, Action::FinishAttacks);
    act(
        &mut g,
        Action::Block {
            blocker: b1,
            attacker: brute,
        },
    );
    assert!(
        !g.legal_actions().contains(&Action::FinishBlocks),
        "one blocker is not legal"
    );
    assert!(g.legal_actions().contains(&Action::ResetBlocks));
    act(
        &mut g,
        Action::Block {
            blocker: b2,
            attacker: brute,
        },
    );
    act(&mut g, Action::FinishBlocks);
    assert!(!alive(&g, brute));
}

#[test]
fn indestructible_survives_lethal_damage() {
    // CR 702.12b.
    let mut g = board();
    let golem = put(&mut g, 0, GOLEM);
    let ogre = put(&mut g, 1, OGRE);
    let ev = fight(&mut g, &[golem], &[(ogre, golem)]);
    assert_eq!(damage_to(&ev, golem), 5);
    assert!(alive(&g, golem));
}

#[test]
fn damage_wears_off_at_end_of_turn() {
    // CR 514.2.
    let mut g = board();
    let golem = put(&mut g, 0, GOLEM);
    let bear = put(&mut g, 1, BEAR);
    let ev = fight(&mut g, &[golem], &[(bear, golem)]);
    assert_eq!(damage_to(&ev, golem), 2);
    assert!(g.turn > 3);
    assert_eq!(g.obj(golem).damage, 0);
}

#[test]
fn vigilance_creature_can_still_block_next_turn() {
    // CR 702.20b: it never tapped, so it is untapped for the opponent's turn.
    let mut g = board();
    let w = put(&mut g, 0, WATCHER);
    to_attacks(&mut g);
    act(&mut g, Action::Attack(w));
    assert!(!g.obj(w).tapped);
}
