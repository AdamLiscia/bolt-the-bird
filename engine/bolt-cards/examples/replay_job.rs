//! Replays one pilot_games job and reports each decision: turn, objects, time. For hunting hangs.
//!
//!     replay_job DECKS "id a b seed first" ROLLOUTS CANDIDATES [PLAYOUT_FROM]
//!
//! With PLAYOUT_FROM = n, it stops before decision n and instead plays one baseline playout from
//! that position (as the rollout pilot would), printing objects every 100 actions.

use std::time::Instant;

use bolt_cards::fdn;
use bolt_core::{baseline, rollout, Game};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let decks: Vec<Vec<usize>> = std::fs::read_to_string(&a[1])
        .expect("decks")
        .lines()
        .map(|l| l.split_whitespace().map(|x| x.parse().expect("n")).collect())
        .collect();
    let j: Vec<u64> = a[2].split_whitespace().map(|x| x.parse().expect("n")).collect();
    let cfg = rollout::Config {
        rollouts: a[3].parse().expect("r"),
        max_candidates: a[4].parse().expect("c"),
        max_actions: 5_000,
        max_objects: 2_000,
    };
    let stop: Option<u64> = a.get(5).and_then(|s| s.parse().ok());
    let mut g = Game::new(
        fdn(),
        [&decks[j[1] as usize], &decks[j[2] as usize]],
        j[3],
        j[4] as usize,
    );
    for n in 0..100_000u64 {
        if g.result.is_some() {
            println!("over: {:?} turn {}", g.result, g.turn);
            return;
        }
        if Some(n) == stop && std::env::var("WORLDS").is_ok() {
            // The rollout pilot's own guessed worlds for this decision, each legal move played out.
            let legal = g.legal_actions();
            let who = g.pending.player().expect("a player acts");
            let mut rng = bolt_core::rng::Rng::new(j[3].wrapping_mul(1_000_003).wrapping_add(n));
            let worlds: Vec<u64> = (0..cfg.rollouts).map(|_| rng.next_u64()).collect();
            for &w in &worlds {
                for &first in &legal {
                    let mut h = g.clone();
                    h.determinize(who, w);
                    let t = Instant::now();
                    if h.apply(first).is_err() {
                        continue;
                    }
                    let mut k = 0;
                    let mut worst = (0u128, 0usize, 0u32);
                    while h.result.is_none() && k < 5_000 {
                        let ta = Instant::now();
                        let l = h.legal_actions();
                        if l.is_empty() {
                            println!(
                                "NO LEGAL ACTIONS: world {w} first {first:?} action {k}, turn {}, step {:?}, pending {:?}\n  resolving {:?}\n  choosing {:?}",
                                h.turn, h.step, h.pending, h.resolving, h.choosing
                            );
                            if let Some(c) = &h.choosing {
                                let d = h.def_of(c.item.source);
                                println!("  casting {:?}", d.name);
                            }
                            break;
                        }
                        h.apply(baseline::choose(&h, &l)).expect("legal");
                        let ms = ta.elapsed().as_millis();
                        if ms > worst.0 {
                            worst = (ms, h.objects.len(), h.turn);
                        }
                        if ms > 2_000 {
                            println!(
                                "    slow action {k}: {ms} ms, turn {} objects {} legal {}",
                                h.turn,
                                h.objects.len(),
                                l.len()
                            );
                        }
                        k += 1;
                    }
                    println!(
                        "world {w} first {first:?}: {k} actions, {} ms, {:?} turn {}, objects {}, worst action {} ms (objects {}, turn {})",
                        t.elapsed().as_millis(), h.result, h.turn, h.objects.len(), worst.0, worst.1, worst.2
                    );
                }
            }
            return;
        }
        if Some(n) == stop {
            let mut h = g.clone();
            for k in 0..5_000 {
                if h.result.is_some() {
                    println!("playout over at action {k}: {:?} turn {}", h.result, h.turn);
                    let ev = h.take_events();
                    let n = ev.len();
                    println!("  {n} events since the last take; the last 12:");
                    for e in &ev[n.saturating_sub(12)..] {
                        println!("  {e:?}");
                    }
                    break;
                }
                let t = Instant::now();
                let legal = h.legal_actions();
                let tl = t.elapsed().as_millis();
                let act = baseline::choose(&h, &legal);
                if k >= 170 {
                    println!(
                        "about to: action {k} turn {} {act:?} pending {:?} legal {:?}",
                        h.turn, h.pending, legal
                    );
                }
                h.apply(act).expect("legal");
                let ms = t.elapsed().as_millis();
                if k % 100 == 0 || ms > 100 {
                    println!(
                        "playout action {k}: turn {} objects {} legal {} ({tl} ms) {act:?} took {ms} ms, pending {:?}",
                        h.turn, h.objects.len(), legal.len(), h.pending
                    );
                }
            }
            return;
        }
        let legal = g.legal_actions();
        if std::env::var("EVERY").is_ok() {
            eprintln!(
                "start decision {n}: turn {} objects {} legal {}",
                g.turn,
                g.objects.len(),
                legal.len()
            );
        }
        let t = Instant::now();
        let act = rollout::choose(&g, &legal, &cfg, j[3].wrapping_mul(1_000_003).wrapping_add(n));
        let ms = t.elapsed().as_millis();
        if ms > 200 {
            println!(
                "decision {n}: turn {} objects {} legal {} took {ms} ms",
                g.turn,
                g.objects.len(),
                legal.len()
            );
        }
        g.apply(act).expect("legal");
    }
}
