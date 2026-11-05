//! Times each fuzz game and reports the slowest, to find hangs. For debugging.
//!
//!     cargo run --release -p bolt-cards --example slow_seeds -- 2000 5

use std::time::{Duration, Instant};

use bolt_cards::{fdn, supported_mask};
use bolt_core::fuzz::{random_deck, random_game};
use bolt_core::rng::Rng;

fn main() {
    let n: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(2000);
    let limit = Duration::from_secs(std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(5));
    let defs = fdn();
    let mask = supported_mask();
    let lands: Vec<usize> = (0..mask.len())
        .filter(|&i| mask[i] && defs[i].types.land)
        .collect();
    let spells: Vec<usize> = (0..mask.len())
        .filter(|&i| mask[i] && !defs[i].types.land)
        .collect();
    let mut worst = (Duration::ZERO, 0);
    let from: u64 = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(0);
    for seed in from..n {
        eprintln!("start {seed}");
        let mut rng = Rng::new(seed);
        let a = random_deck(&lands, &spells, &mut rng);
        let b = random_deck(&lands, &spells, &mut rng);
        let start = Instant::now();
        let r = random_game(defs.clone(), [&a, &b], seed, false);
        let t = start.elapsed();
        if t > worst.0 {
            worst = (t, seed);
            println!(
                "seed {seed}: {t:?} {:?}",
                r.as_ref().map(|r| (r.turns, r.actions))
            );
        }
        if t > limit {
            println!("seed {seed} took {t:?}: stopping");
            return;
        }
    }
    println!("slowest: seed {} in {:?}", worst.1, worst.0);
}
