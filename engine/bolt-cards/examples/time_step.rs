//! Replays a fuzz seed for N actions, then times the next action. For profiling.
use std::time::Instant;

use bolt_cards::{fdn, supported_mask};
use bolt_core::fuzz::random_deck;
use bolt_core::rng::Rng;
use bolt_core::Game;

fn main() {
    let args: Vec<u64> = std::env::args().skip(1).filter_map(|s| s.parse().ok()).collect();
    let (seed, steps) = (args[0], args[1] as usize);
    let defs = fdn();
    let mask = supported_mask();
    let lands: Vec<usize> = (0..mask.len())
        .filter(|&i| mask[i] && defs[i].types.land)
        .collect();
    let spells: Vec<usize> = (0..mask.len())
        .filter(|&i| mask[i] && !defs[i].types.land)
        .collect();
    let mut rng = Rng::new(seed);
    let a = random_deck(&lands, &spells, &mut rng);
    let b = random_deck(&lands, &spells, &mut rng);
    let mut g = Game::new(defs.clone(), [&a, &b], seed, (seed % 2) as usize);
    let mut agent = Rng::new(seed ^ 0xA5A5_A5A5);
    for _ in 0..steps {
        let legal = g.legal_actions();
        let act = legal[agent.below(legal.len())];
        g.apply(act).expect("legal");
    }
    let t = Instant::now();
    let legal = g.legal_actions();
    eprintln!("legal_actions: {} in {:?}", legal.len(), t.elapsed());
    let t = Instant::now();
    let _ = g.check_invariants();
    eprintln!("check_invariants in {:?}", t.elapsed());
    let one = g.battlefield[g.battlefield.len() - 1];
    let t = Instant::now();
    for _ in 0..100 {
        std::hint::black_box(g.power(one));
    }
    eprintln!("power x100 in {:?}", t.elapsed());
    let act = legal[agent.below(legal.len())];
    let t = Instant::now();
    g.apply(act).expect("legal");
    eprintln!("apply {act:?} in {:?}", t.elapsed());
    for _ in 0..3 {
        let legal = g.legal_actions();
        let act = legal[agent.below(legal.len())];
        let t = Instant::now();
        g.apply(act).expect("legal");
        eprintln!(
            "apply {act:?} in {:?}, {} permanents",
            t.elapsed(),
            g.battlefield.len()
        );
    }
}
