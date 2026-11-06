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
}

impl Default for Config {
    fn default() -> Self {
        Config {
            rollouts: 16,
            max_candidates: 8,
            max_actions: 5_000,
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

/// Plays `g` to the end with the baseline on both sides: 1 if `p` wins, 0.5 for a draw.
fn playout(mut g: Game, p: PlayerId, max_actions: u32) -> f64 {
    for _ in 0..max_actions {
        if let Some(o) = g.result {
            return match o {
                Outcome::Winner(w) => (w == p) as u8 as f64,
                Outcome::Draw => 0.5,
            };
        }
        let legal = g.legal_actions();
        if g.apply(baseline::choose(&g, &legal)).is_err() {
            return 0.5;
        }
    }
    0.5
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
    let mut rng = Rng::new(seed);
    let worlds: Vec<u64> = (0..cfg.rollouts).map(|_| rng.next_u64()).collect();
    let mut best = (f64::MIN, cands[0]);
    for &a in &cands {
        let mut total = 0.0;
        for &w in &worlds {
            let mut h = g.clone();
            h.determinize(p, w);
            total += match h.apply(a) {
                Ok(_) => playout(h, p, cfg.max_actions),
                Err(_) => 0.0,
            };
        }
        // Ties go to the earlier candidate, so the baseline's pick wins a tie.
        if total > best.0 {
            best = (total, a);
        }
    }
    best.1
}
