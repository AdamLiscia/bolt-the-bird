//! Plays a list of games between fixed decks with a chosen pilot on both sides, resumably.
//!
//!     pilot_games DECKS JOBS OUT PILOT [SECONDS] [THREADS] [ROLLOUTS] [CANDIDATES]
//!
//! - DECKS: one deck per line, card numbers separated by spaces.
//! - JOBS: one game per line: `id deck_a deck_b seed first`.
//! - OUT: appended one line per finished game: `id result turns`, with result `p0`, `p1`, `draw`
//!   or `timeout` (past 200 turns, or past 180 s of play), or `error` if the engine panicked (logged
//!   to stderr with the job id; the run goes on). Jobs already in OUT are skipped, so a run can stop
//!   and resume.
//! - PILOT: `baseline` or `rollout`.
//! - SECONDS: stop starting new games after this long (default: no limit), so a run fits inside a
//!   time limit and finishes the games it began.

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bolt_cards::fdn;
use bolt_core::{baseline, rollout, Game, Outcome};

const MAX_TURNS: u32 = 200;
/// A game still going after this long (only ever the rollout pilot) stops as a timeout, so every
/// game finishes inside a guarded run. Each one is logged as slow.
const GAME_SECONDS: u64 = 180;

struct Job {
    id: u64,
    a: usize,
    b: usize,
    seed: u64,
    first: usize,
}

fn play(decks: &[Vec<usize>], j: &Job, cfg: Option<&rollout::Config>) -> (&'static str, u32) {
    let started = Instant::now();
    let mut g = Game::new(fdn(), [&decks[j.a], &decks[j.b]], j.seed, j.first);
    for n in 0..100_000u64 {
        if let Some(o) = g.result {
            let r = match o {
                Outcome::Winner(0) => "p0",
                Outcome::Winner(_) => "p1",
                Outcome::Draw => "draw",
            };
            return (r, g.turn);
        }
        if g.turn > MAX_TURNS || started.elapsed().as_secs() >= GAME_SECONDS {
            return ("timeout", g.turn);
        }
        let legal = g.legal_actions();
        let action = match cfg {
            Some(c) => rollout::choose(&g, &legal, c, j.seed.wrapping_mul(1_000_003).wrapping_add(n)),
            None => baseline::choose(&g, &legal),
        };
        g.apply(action).expect("pilots pick legal actions");
    }
    ("timeout", g.turn)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert!(
        args.len() >= 5,
        "usage: pilot_games DECKS JOBS OUT PILOT [SECONDS] [THREADS] [ROLLOUTS] [CANDIDATES]"
    );
    let num = |i: usize, d: u64| args.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (seconds, threads) = (num(5, u64::MAX), num(6, 8) as usize);
    let cfg = match args[4].as_str() {
        "baseline" => None,
        "rollout" => Some(rollout::Config {
            rollouts: num(7, 4) as u32,
            max_candidates: num(8, 6) as usize,
            max_actions: 5_000,
        }),
        p => panic!("unknown pilot {p:?}"),
    };
    let decks: Vec<Vec<usize>> = fs::read_to_string(&args[1])
        .expect("decks file")
        .lines()
        .map(|l| {
            l.split_whitespace()
                .map(|x| x.parse().expect("card number"))
                .collect()
        })
        .collect();
    let done: HashSet<u64> = fs::read_to_string(&args[3])
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_whitespace().next()?.parse().ok())
        .collect();
    let jobs: Vec<Job> = fs::read_to_string(&args[2])
        .expect("jobs file")
        .lines()
        .map(|l| {
            let v: Vec<u64> = l.split_whitespace().map(|x| x.parse().expect("number")).collect();
            Job {
                id: v[0],
                a: v[1] as usize,
                b: v[2] as usize,
                seed: v[3],
                first: v[4] as usize,
            }
        })
        .filter(|j| !done.contains(&j.id))
        .collect();
    let total = jobs.len();
    let out = Arc::new(Mutex::new(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&args[3])
            .expect("output file"),
    ));
    let (jobs, decks) = (Arc::new(jobs), Arc::new(decks));
    let next = Arc::new(AtomicUsize::new(0));
    let played = Arc::new(AtomicUsize::new(0));
    let start = Instant::now();
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let (jobs, decks, next, out, played) = (
                jobs.clone(),
                decks.clone(),
                next.clone(),
                out.clone(),
                played.clone(),
            );
            std::thread::spawn(move || loop {
                if start.elapsed().as_secs() >= seconds {
                    return;
                }
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(j) = jobs.get(i) else { return };
                let t = Instant::now();
                // A panic in one game is recorded and logged, not allowed to end the run. On
                // 2026-11-08 one game panicked with an impossible index and replayed cleanly 400 times.
                let (r, turns) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    play(&decks, j, cfg.as_ref())
                }))
                .unwrap_or_else(|_| {
                    eprintln!("panic: job {} (seed {})", j.id, j.seed);
                    ("error", 0)
                });
                if t.elapsed().as_secs() >= 60 {
                    eprintln!(
                        "slow: job {} took {}s, {r} after {turns} turns",
                        j.id,
                        t.elapsed().as_secs()
                    );
                }
                let mut f = out.lock().expect("lock");
                writeln!(f, "{} {r} {turns}", j.id).expect("write");
                f.flush().expect("flush");
                played.fetch_add(1, Ordering::Relaxed);
            })
        })
        .collect();
    for h in handles {
        h.join().expect("thread");
    }
    let n = played.load(Ordering::Relaxed);
    let secs = start.elapsed().as_secs_f64();
    println!(
        "{n} games in {secs:.0}s ({:.2} games/s); {} of {total} left",
        n as f64 / secs,
        total - n
    );
}
