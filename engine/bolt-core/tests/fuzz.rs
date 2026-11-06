//! Random legal actions across many seeded games. Every game must end, the engine must never reject
//! a legal action, and the invariants must hold after every action. `FUZZ_GAMES` sets the count.

mod common;

use bolt_core::fuzz::{random_deck, random_game};
use bolt_core::rng::Rng;
use common::*;

#[test]
fn random_games_end_and_keep_invariants() {
    let n: u64 = std::env::var("FUZZ_GAMES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(300);
    let defs = defs();
    let spells: Vec<usize> = (BEAR..defs.len()).collect();
    for seed in 0..n {
        let mut rng = Rng::new(seed);
        let a = random_deck(&[PLAINS, MOUNTAIN], &spells, &mut rng);
        let b = random_deck(&[PLAINS, MOUNTAIN], &spells, &mut rng);
        random_game(defs.clone(), [&a, &b], seed, true).unwrap_or_else(|e| panic!("{e}"));
    }
}
