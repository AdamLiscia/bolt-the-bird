//! Random-agent games with random decks of supported FDN cards, invariants checked after every
//! action. Prints throughput and coverage.
//!
//!     cargo run --release -p bolt-cards --example random_fdn_games -- 10000

use std::time::Instant;

use bolt_cards::{fdn, supported_mask};
use bolt_core::fuzz::{random_deck, random_game};
use bolt_core::rng::Rng;
use bolt_core::Outcome;

fn main() {
    let n: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000);
    let defs = fdn();
    let mask = supported_mask();
    let lands: Vec<usize> = (0..mask.len())
        .filter(|&i| mask[i] && defs[i].types.land)
        .collect();
    let spells: Vec<usize> = (0..mask.len())
        .filter(|&i| mask[i] && !defs[i].types.land)
        .collect();
    let start = Instant::now();
    let (mut turns, mut actions, mut draws) = (0u64, 0u64, 0u64);
    for seed in 0..n {
        let mut rng = Rng::new(seed);
        let a = random_deck(&lands, &spells, &mut rng);
        let b = random_deck(&lands, &spells, &mut rng);
        let r = random_game(defs.clone(), [&a, &b], seed, true).unwrap_or_else(|e| panic!("{e}"));
        turns += r.turns as u64;
        actions += r.actions as u64;
        draws += (r.outcome == Outcome::Draw) as u64;
    }
    let secs = start.elapsed().as_secs_f64();
    println!(
        "{n} games with {} supported cards, no invariant broken. {:.0} games/s with checks. \
         Avg {:.1} turns, {:.0} actions, {draws} draws",
        spells.len() + lands.len(),
        n as f64 / secs,
        turns as f64 / n as f64,
        actions as f64 / n as f64,
    );
}
