//! Whole games between two FDN decks with a chosen pilot on both sides, one at a time or many
//! across threads. `examples/pilot_games` and the Python bindings both play through here.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use bolt_core::{baseline, rollout, Game, Outcome};

use crate::fdn;

/// A game still going past this many turns is a timeout.
pub const MAX_TURNS: u32 = 200;
/// A game still going after this long is a timeout (only ever the smart pilot, on huge boards).
pub const GAME_SECONDS: u64 = 180;
/// Stack for each game thread (reserved, not used up front).
pub const STACK_BYTES: usize = 64 << 20;

/// Who plays both sides.
#[derive(Clone, Copy, Debug)]
pub enum Pilot {
    /// The rules-based player.
    Baseline,
    /// The smart (rollout) player with this search size.
    Smart(rollout::Config),
}

/// One game: deck `a` is player 0, `first` plays first, `seed` shuffles and seeds the pilot.
#[derive(Clone, Debug)]
pub struct Job {
    pub a: Vec<usize>,
    pub b: Vec<usize>,
    pub seed: u64,
    pub first: usize,
}

/// `result` is "p0", "p1", "draw", "timeout" (past `MAX_TURNS` or `GAME_SECONDS`), or "error" if
/// the engine panicked (logged to stderr with the seed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Played {
    pub result: &'static str,
    pub turns: u32,
}

fn run(j: &Job, pilot: &Pilot) -> Played {
    let started = Instant::now();
    let mut g = Game::new(fdn(), [&j.a, &j.b], j.seed, j.first);
    for n in 0..100_000u64 {
        if let Some(o) = g.result {
            let result = match o {
                Outcome::Winner(0) => "p0",
                Outcome::Winner(_) => "p1",
                Outcome::Draw => "draw",
            };
            return Played {
                result,
                turns: g.turn,
            };
        }
        if g.turn > MAX_TURNS || started.elapsed().as_secs() >= GAME_SECONDS {
            break;
        }
        let legal = g.legal_actions();
        let action = match pilot {
            Pilot::Smart(c) => rollout::choose(&g, &legal, c, j.seed.wrapping_mul(1_000_003).wrapping_add(n)),
            Pilot::Baseline => baseline::choose(&g, &legal),
        };
        g.apply(action).expect("pilots pick legal actions");
    }
    Played {
        result: "timeout",
        turns: g.turn,
    }
}

/// Plays one game. A panic inside the engine is caught and reported as "error".
pub fn play_game(j: &Job, pilot: &Pilot) -> Played {
    catch_unwind(AssertUnwindSafe(|| run(j, pilot))).unwrap_or_else(|_| {
        eprintln!(
            "panic: seed {} (decks of {} and {} cards)",
            j.seed,
            j.a.len(),
            j.b.len()
        );
        Played {
            result: "error",
            turns: 0,
        }
    })
}

/// Plays every job on up to `threads` threads; results come back in job order.
pub fn play_many(jobs: &[Job], pilot: &Pilot, threads: usize) -> Vec<Played> {
    let next = AtomicUsize::new(0);
    let out = Mutex::new(vec![None; jobs.len()]);
    std::thread::scope(|s| {
        for _ in 0..threads.max(1).min(jobs.len().max(1)) {
            // Big stacks: twice now a long run hit memory corruption that safe code shouldn't
            // allow (QUESTIONS.md); a silent stack overflow is one way to get it.
            let builder = std::thread::Builder::new().stack_size(STACK_BYTES);
            builder
                .spawn_scoped(s, || loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(j) = jobs.get(i) else { return };
                    let r = play_game(j, pilot);
                    out.lock().expect("no thread panics while holding it")[i] = Some(r);
                })
                .expect("spawn a game thread");
        }
    });
    out.into_inner()
        .expect("unpoisoned")
        .into_iter()
        .map(|r| r.expect("every job played"))
        .collect()
}
