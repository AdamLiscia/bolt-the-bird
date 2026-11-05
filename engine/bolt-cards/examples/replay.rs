//! Replays one fuzz seed and prints the state when it gets stuck. For debugging.
//!
//!     cargo run -p bolt-cards --example replay -- 2851

use bolt_cards::{fdn, supported_mask};
use bolt_core::fuzz::random_deck;
use bolt_core::rng::Rng;
use bolt_core::Game;

fn main() {
    let seed: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("a seed");
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
    let mut last = Vec::new();
    while g.result.is_none() {
        let legal = g.legal_actions();
        if legal.is_empty() {
            println!(
                "stuck: pending {:?}, step {:?}, turn {}",
                g.pending, g.step, g.turn
            );
            println!("choosing {:?}", g.choosing);
            println!("decision {:?}", g.decision);
            if let Some(c) = &g.choosing {
                println!("source {}", g.def_of(c.item.source).name);
            }
            println!("last actions: {last:?}");
            return;
        }
        let a = legal[agent.below(legal.len())];
        if std::env::var("TRACE").is_ok() {
            eprintln!(
                "turn {} step {:?} pending {:?} -> {a:?}",
                g.turn, g.step, g.pending
            );
        }
        last.push(format!("{a:?}"));
        if last.len() > 6 {
            last.remove(0);
        }
        g.apply(a).expect("legal");
    }
    println!("finished: {:?}", g.result);
}
