//! The rules-based baseline player (M5): it finishes games and beats a random player.

use bolt_cards::{fdn, supported_mask};
use bolt_core::fuzz::random_deck;
use bolt_core::rng::Rng;
use bolt_core::{baseline, Game, Outcome};

/// Plays one game; `smart[p]` says whether player p is the baseline (else random).
fn play(seed: u64, smart: [bool; 2]) -> Outcome {
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
    // Same decks both ways round, so deck quality cancels out across seeds.
    let mut g = Game::new(defs, [&a, &b], seed, (seed % 2) as usize);
    for _ in 0..20_000 {
        if let Some(o) = g.result {
            return o;
        }
        let legal = g.legal_actions();
        let p = match g.pending {
            bolt_core::Pending::Mulligan(p)
            | bolt_core::Pending::Bottom(p, _)
            | bolt_core::Pending::Priority(p)
            | bolt_core::Pending::DeclareAttackers(p)
            | bolt_core::Pending::DeclareBlockers(p)
            | bolt_core::Pending::Discard(p)
            | bolt_core::Pending::ChooseTarget(p)
            | bolt_core::Pending::Decide(p)
            | bolt_core::Pending::Divide(p)
            | bolt_core::Pending::RemoveCounters(p)
            | bolt_core::Pending::FreeCast(p) => p,
            bolt_core::Pending::GameOver => unreachable!(),
        };
        let action = if smart[p] {
            baseline::choose(&g, &legal)
        } else {
            legal[rng.below(legal.len())]
        };
        g.apply(action).expect("the baseline picks legal actions");
    }
    let tail: Vec<String> = (0..12)
        .map(|_| {
            let legal = g.legal_actions();
            let a = baseline::choose(&g, &legal);
            let s = format!("{:?} {:?} -> {a:?} of {}", g.step, g.pending, legal.len());
            g.apply(a).expect("legal");
            s
        })
        .collect();
    panic!(
        "seed {seed}: no result in 20,000 actions; then:\n{}",
        tail.join("\n")
    );
}

#[test]
fn baseline_games_against_itself_finish() {
    for seed in 0..100 {
        play(seed, [true, true]);
    }
}

#[test]
fn baseline_beats_a_random_player() {
    let mut wins = 0;
    let n = 200;
    for seed in 0..n {
        // Alternate seats so neither side always has the same deck.
        let smart_seat = (seed % 2) as usize;
        let mut smart = [false, false];
        smart[smart_seat] = true;
        if play(seed, smart) == Outcome::Winner(smart_seat) {
            wins += 1;
        }
    }
    assert!(wins * 100 >= n * 80, "baseline won only {wins} of {n}");
}
