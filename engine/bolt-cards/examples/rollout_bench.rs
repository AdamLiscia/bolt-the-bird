//! The rollout pilot against the baseline on random decks: win rate and time per game.
//!
//!     cargo run --release -p bolt-cards --example rollout_bench -- GAMES ROLLOUTS CANDIDATES THREADS
//!
//! Each seed is played twice with the decks swapped, so deck strength cancels out.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use bolt_cards::{fdn, supported_mask};
use bolt_core::fuzz::random_deck;
use bolt_core::rng::Rng;
use bolt_core::{baseline, rollout, Game, Outcome};

/// Score for the rollout pilot (1 win, 0.5 draw) and its decisions searched.
fn play(
    seed: u64,
    pilot_seat: usize,
    swap: bool,
    cfg: &rollout::Config,
    pool: &(Vec<usize>, Vec<usize>),
) -> f64 {
    let mut rng = Rng::new(seed);
    let a = random_deck(&pool.0, &pool.1, &mut rng);
    let b = random_deck(&pool.0, &pool.1, &mut rng);
    let decks = if swap {
        [b.as_slice(), a.as_slice()]
    } else {
        [a.as_slice(), b.as_slice()]
    };
    let mut g = Game::new(fdn(), decks, seed, (seed % 2) as usize);
    let mut n = 0u64;
    while g.result.is_none() {
        let legal = g.legal_actions();
        let action = if g.pending.player() == Some(pilot_seat) {
            rollout::choose(&g, &legal, cfg, seed.wrapping_mul(1_000_003) + n)
        } else {
            baseline::choose(&g, &legal)
        };
        g.apply(action).expect("legal");
        n += 1;
        if n > 100_000 || g.turn > 200 {
            return 0.5;
        }
    }
    match g.result.expect("over") {
        Outcome::Winner(w) => (w == pilot_seat) as u8 as f64,
        Outcome::Draw => 0.5,
    }
}

fn main() {
    let arg = |i: usize, d: u64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (seeds, rollouts, cands, threads) = (arg(1, 40), arg(2, 16), arg(3, 8), arg(4, 8));
    let cfg = rollout::Config {
        rollouts: rollouts as u32,
        max_candidates: cands as usize,
        max_actions: 5_000,
        max_objects: 2_000,
    };
    let defs = fdn();
    let mask = supported_mask();
    let lands: Vec<usize> = (0..mask.len())
        .filter(|&i| mask[i] && defs[i].types.land)
        .collect();
    let spells: Vec<usize> = (0..mask.len())
        .filter(|&i| mask[i] && !defs[i].types.land)
        .collect();
    let pool = Arc::new((lands, spells));
    let next = Arc::new(AtomicU64::new(0));
    let start = Instant::now();
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let (pool, next) = (pool.clone(), next.clone());
            std::thread::spawn(move || {
                let mut scores = Vec::new();
                loop {
                    let s = next.fetch_add(1, Ordering::Relaxed);
                    if s >= seeds {
                        return scores;
                    }
                    // The pilot plays each seed's deck A and then deck B, from alternating seats.
                    let seat = (s % 2) as usize;
                    let x = play(s, seat, false, &cfg, &pool);
                    let y = play(s, seat, true, &cfg, &pool);
                    scores.push((x + y) / 2.0);
                }
            })
        })
        .collect();
    let scores: Vec<f64> = handles
        .into_iter()
        .flat_map(|h| h.join().expect("thread"))
        .collect();
    let secs = start.elapsed().as_secs_f64();
    let n = scores.len() as f64;
    let mean = scores.iter().sum::<f64>() / n;
    let var = scores.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    println!(
        "rollouts={rollouts} candidates={cands}: pilot vs baseline {:.1}% ± {:.1} over {} games, {:.1}s ({:.2} s/game/thread)",
        100.0 * mean,
        100.0 * (var / n).sqrt(),
        2 * seeds,
        secs,
        secs * threads as f64 / (2 * seeds) as f64
    );
}
