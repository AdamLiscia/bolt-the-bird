//! Random-agent games for fuzzing and benchmarks: every legal action is equally likely.

use std::sync::Arc;

use crate::card::CardDef;
use crate::game::{Event, Game, Outcome, STARTING_LIFE};
use crate::rng::Rng;

/// Safety valve: no real game takes this many actions. Hitting it is reported as a bug.
pub const MAX_ACTIONS: usize = 100_000;

/// Random players can grow a board without limit (Krenko, Mob Boss doubles its Goblins every
/// turn). Past this many permanents a game is stopped and reported as capped, not as a bug: by
/// then each trigger costs O(board) state checks and a doubling board costs O(board²) per turn,
/// which no real game reaches.
pub const MAX_PERMANENTS: usize = 1_000;

#[derive(Debug)]
pub struct GameReport {
    pub outcome: Outcome,
    pub turns: u32,
    pub actions: usize,
    /// Stopped at `MAX_PERMANENTS` (reported as a draw).
    pub capped: bool,
}

/// Plays one game with uniformly random legal actions, checking invariants after every action.
/// Returns an error describing the first broken invariant, with the seed to replay it.
pub fn random_game(
    defs: Arc<Vec<CardDef>>,
    decks: [&[usize]; 2],
    seed: u64,
    check: bool,
) -> Result<GameReport, String> {
    let mut g = Game::new(defs, decks, seed, (seed % 2) as usize);
    let mut rng = Rng::new(seed ^ 0xA5A5_A5A5);
    let mut life = [STARTING_LIFE; 2];
    for n in 0..MAX_ACTIONS {
        if let Some(outcome) = g.result {
            return Ok(GameReport {
                outcome,
                turns: g.turn,
                actions: n,
                capped: false,
            });
        }
        if g.battlefield.len() > MAX_PERMANENTS {
            return Ok(GameReport {
                outcome: Outcome::Draw,
                turns: g.turn,
                actions: n,
                capped: true,
            });
        }
        let legal = g.legal_actions();
        if legal.is_empty() {
            return Err(format!("seed {seed}: no legal actions but the game is not over"));
        }
        let action = legal[rng.below(legal.len())];
        // A runaway action panics (see `WORK_BUDGET`); report it with its seed.
        let applied =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| g.apply(action))).map_err(|e| {
                let msg = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                format!("seed {seed}: {action:?} panicked: {msg}")
            })?;
        let events = applied.map_err(|e| format!("seed {seed}: legal action rejected: {e}"))?;
        if check {
            for e in &events {
                match *e {
                    Event::LifeLost { player, amount } => life[player] -= amount,
                    Event::LifeGained { player, amount } => life[player] += amount,
                    _ => {}
                }
            }
            for (p, &expected) in life.iter().enumerate() {
                if expected != g.players[p].life {
                    return Err(format!(
                        "seed {seed}: life of player {p} disagrees with life events"
                    ));
                }
            }
            g.check_invariants()
                .map_err(|e| format!("seed {seed}, action {n} ({action:?}): {e}"))?;
        }
    }
    Err(format!("seed {seed}: no result after {MAX_ACTIONS} actions"))
}

/// A random 40-card deck: 17 lands drawn from `lands` and 23 spells drawn from `spells`.
pub fn random_deck(lands: &[usize], spells: &[usize], rng: &mut Rng) -> Vec<usize> {
    let mut d: Vec<usize> = (0..17).map(|_| lands[rng.below(lands.len())]).collect();
    d.extend((0..23).map(|_| spells[rng.below(spells.len())]));
    d
}
