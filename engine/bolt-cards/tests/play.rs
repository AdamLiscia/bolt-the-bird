//! Playing whole games with a chosen pilot, one at a time or many across threads.

use bolt_cards::play::{play_game, play_many, Job, Pilot};
use bolt_cards::{fdn, supported_mask};
use bolt_core::fuzz::random_deck;
use bolt_core::rng::Rng;
use bolt_core::rollout;

fn jobs(n: u64) -> Vec<Job> {
    let defs = fdn();
    let mask = supported_mask();
    let lands: Vec<usize> = (0..mask.len())
        .filter(|&i| mask[i] && defs[i].types.land)
        .collect();
    let spells: Vec<usize> = (0..mask.len())
        .filter(|&i| mask[i] && !defs[i].types.land)
        .collect();
    (0..n)
        .map(|s| {
            let mut rng = Rng::new(500 + s);
            Job {
                a: random_deck(&lands, &spells, &mut rng),
                b: random_deck(&lands, &spells, &mut rng),
                seed: s,
                first: (s % 2) as usize,
            }
        })
        .collect()
}

fn smart() -> Pilot {
    Pilot::Smart(rollout::Config {
        rollouts: 1,
        max_candidates: 2,
        max_actions: 3_000,
        max_objects: 2_000,
    })
}

#[test]
fn every_game_ends_with_a_result_and_a_turn_count() {
    for p in [Pilot::Baseline, smart()] {
        for j in jobs(4) {
            let r = play_game(&j, &p);
            assert!(
                ["p0", "p1", "draw", "timeout"].contains(&r.result),
                "{}",
                r.result
            );
            assert!(r.turns >= 1);
        }
    }
}

#[test]
fn many_games_on_threads_match_one_at_a_time_in_order() {
    let js = jobs(12);
    for p in [Pilot::Baseline, smart()] {
        let one: Vec<_> = js.iter().map(|j| play_game(j, &p)).collect();
        assert_eq!(play_many(&js, &p, 4), one);
        assert_eq!(play_many(&js, &p, 1), one);
    }
}

#[test]
fn the_same_job_plays_the_same_game() {
    let j = &jobs(1)[0];
    assert_eq!(play_game(j, &smart()), play_game(j, &smart()));
}
