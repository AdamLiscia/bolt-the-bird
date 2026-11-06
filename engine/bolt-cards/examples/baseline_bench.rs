//! Benchmarks the rules-based baseline: its win rate against a random player, and games per
//! second with the baseline on both sides.
//!
//!     cargo run --release -p bolt-cards --example baseline_bench -- 2000

use std::time::Instant;

use bolt_cards::{fdn, supported_mask};
use bolt_core::fuzz::random_deck;
use bolt_core::rng::Rng;
use bolt_core::{baseline, Game, Outcome, Pending};

fn to_act(p: &Pending) -> usize {
    match *p {
        Pending::Mulligan(p)
        | Pending::Bottom(p, _)
        | Pending::Priority(p)
        | Pending::DeclareAttackers(p)
        | Pending::DeclareBlockers(p)
        | Pending::Discard(p)
        | Pending::ChooseTarget(p)
        | Pending::Decide(p)
        | Pending::Divide(p)
        | Pending::RemoveCounters(p)
        | Pending::FreeCast(p) => p,
        Pending::GameOver => unreachable!(),
    }
}

/// Plays one game; returns the outcome and the number of actions.
fn play(seed: u64, smart: [bool; 2], lands: &[usize], spells: &[usize]) -> (Outcome, u64) {
    let mut rng = Rng::new(seed);
    let a = random_deck(lands, spells, &mut rng);
    let b = random_deck(lands, spells, &mut rng);
    let mut g = Game::new(fdn(), [&a, &b], seed, (seed % 2) as usize);
    let mut n = 0;
    while g.result.is_none() {
        let legal = g.legal_actions();
        let p = to_act(&g.pending);
        let action = if smart[p] {
            baseline::choose(&g, &legal)
        } else {
            legal[rng.below(legal.len())]
        };
        g.apply(action).expect("legal");
        n += 1;
        assert!(n < 100_000, "seed {seed}: no result");
    }
    (g.result.expect("over"), n)
}

fn main() {
    let n: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);
    let defs = fdn();
    let mask = supported_mask();
    let lands: Vec<usize> = (0..mask.len())
        .filter(|&i| mask[i] && defs[i].types.land)
        .collect();
    let spells: Vec<usize> = (0..mask.len())
        .filter(|&i| mask[i] && !defs[i].types.land)
        .collect();

    let mut wins = 0u64;
    for seed in 0..n {
        let seat = (seed % 2) as usize;
        let mut smart = [false; 2];
        smart[seat] = true;
        if play(seed, smart, &lands, &spells).0 == Outcome::Winner(seat) {
            wins += 1;
        }
    }
    println!(
        "baseline vs random: won {wins} of {n} ({:.1}%), seats alternated",
        100.0 * wins as f64 / n as f64
    );

    let start = Instant::now();
    let (mut actions, mut draws) = (0u64, 0u64);
    for seed in 0..n {
        let (o, k) = play(seed, [true, true], &lands, &spells);
        actions += k;
        draws += (o == Outcome::Draw) as u64;
    }
    let secs = start.elapsed().as_secs_f64();
    println!(
        "baseline vs baseline: {n} games in {secs:.1}s, {:.0} games/s, {:.0} actions/game, {draws} draws",
        n as f64 / secs,
        actions as f64 / n as f64
    );
}
