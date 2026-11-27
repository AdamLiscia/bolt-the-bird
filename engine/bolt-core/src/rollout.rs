//! A search pilot: flat Monte Carlo over the legal moves, with the baseline playing the rollouts.
//!
//! At a priority, attack or block decision it tries each candidate move in several guessed worlds
//! ([`Game::determinize`]: the opponent's hand and both libraries redealt from what's hidden), plays
//! each game to the end with the baseline on both sides, and keeps the move that won most. It never
//! looks at hidden cards: every rollout starts from a guess. Every other decision (mulligans,
//! targets, discards, choices mid-resolution) goes to the baseline.
//!
//! Rollout `i` uses the same guessed world for every candidate (common random numbers), so the
//! candidates are compared on equal footing.
//!
//! A search never panics out to its caller: a candidate whose imagined game hits an engine panic
//! scores as a loss (so if every one fails, the baseline's move wins the tie). An optional time
//! limit stops the search early with the best move scored so far, or the baseline's.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::{Duration, Instant};

use crate::baseline;
use crate::game::{Action, Game, Outcome, Pending, PlayerId};
use crate::rng::Rng;

#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Rollouts per candidate move.
    pub rollouts: u32,
    /// The most moves to compare; the baseline's own choice is always one of them.
    pub max_candidates: usize,
    /// A rollout still going after this many actions counts as a draw.
    pub max_actions: u32,
    /// A rollout whose game has made more objects than this counts as a draw. Some boards grow
    /// exponentially (Homunculus Horde copying itself with an extra draw each turn), and each action
    /// gets slower as they do: one decision once ran for minutes.
    pub max_objects: usize,
    /// Stop searching after this long and play the best move scored so far (the baseline's, if
    /// none). Checked before every playout action. `None`: no limit (search runs to the end).
    pub time_limit: Option<Duration>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            rollouts: 16,
            max_candidates: 8,
            max_actions: 5_000,
            max_objects: 2_000,
            time_limit: None,
        }
    }
}

/// Moves that differ only in which copy of the same card they use are the same move.
#[derive(PartialEq, Eq)]
enum Key {
    Land(usize),
    Cast(usize),
    Other(Action),
}

fn key(g: &Game, a: Action) -> Key {
    match a {
        Action::PlayLand(c) => Key::Land(g.obj(c).def),
        Action::Cast(c) => Key::Cast(g.obj(c).def),
        _ => Key::Other(a),
    }
}

/// The moves worth comparing: the baseline's pick first, then the rest without duplicates.
fn candidates(g: &Game, legal: &[Action], max: usize) -> Vec<Action> {
    let mut out = vec![baseline::choose(g, legal)];
    let mut keys = vec![key(g, out[0])];
    for &a in legal {
        if out.len() >= max {
            break;
        }
        let k = key(g, a);
        if !keys.contains(&k) {
            keys.push(k);
            out.push(a);
        }
    }
    out
}

/// Plays `g` to the end with the baseline on both sides: 1 if `p` wins, 0.5 for a draw (or past
/// either limit). `None` if `deadline` passes first.
fn playout(mut g: Game, p: PlayerId, cfg: &Config, deadline: Option<Instant>) -> Option<f64> {
    for _ in 0..cfg.max_actions {
        if g.objects.len() > cfg.max_objects {
            return Some(0.5);
        }
        if let Some(o) = g.result {
            return Some(match o {
                Outcome::Winner(w) => (w == p) as u8 as f64,
                Outcome::Draw => 0.5,
            });
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            return None;
        }
        let legal = g.legal_actions();
        if g.apply(baseline::choose(&g, &legal)).is_err() {
            return Some(0.5);
        }
    }
    Some(0.5)
}

/// The pilot's choice from `legal` (which must be `g.legal_actions()`), searching with `seed`.
pub fn choose(g: &Game, legal: &[Action], cfg: &Config, seed: u64) -> Action {
    let searched = matches!(
        g.pending,
        Pending::Priority(_) | Pending::DeclareAttackers(_) | Pending::DeclareBlockers(_)
    );
    let Some(p) = g.pending.player() else {
        return legal[0];
    };
    if !searched || legal.len() < 2 || g.resolving.is_some() {
        return baseline::choose(g, legal);
    }
    let cands = candidates(g, legal, cfg.max_candidates.max(1));
    if cands.len() < 2 {
        return cands[0];
    }
    let deadline = cfg.time_limit.map(|t| Instant::now() + t);
    let mut rng = Rng::new(seed);
    let worlds: Vec<u64> = (0..cfg.rollouts).map(|_| rng.next_u64()).collect();
    // The best (score, move) among the candidates scored in full: one cut short by the time limit
    // isn't compared, since the others had more rollouts. None scored: the baseline's move.
    let mut best: Option<(f64, Action)> = None;
    'search: for &a in &cands {
        let mut total = 0.0;
        for &w in &worlds {
            // An engine panic in an imagined game scores that rollout as a loss.
            let score = catch_unwind(AssertUnwindSafe(|| {
                let mut h = g.clone();
                h.manual_mana = [false; 2]; // imagined games pay automatically
                h.cast_guard = false; // a panic here is caught; no snapshot per cast
                h.determinize(p, w);
                match h.apply(a) {
                    Ok(_) => playout(h, p, cfg, deadline),
                    Err(_) => Some(0.0),
                }
            }))
            .unwrap_or(Some(0.0));
            match score {
                Some(s) => total += s,
                None => break 'search,
            }
        }
        // Ties go to the earlier candidate, so the baseline's pick wins a tie.
        if best.is_none_or(|b| total > b.0) {
            best = Some((total, a));
        }
    }
    best.map_or(cands[0], |b| b.1)
}
