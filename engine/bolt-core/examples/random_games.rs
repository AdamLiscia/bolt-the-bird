//! Plays random-agent games with invariant checks and prints throughput.
//!
//!     cargo run --release --example random_games -- 10000

use std::sync::Arc;
use std::time::Instant;

use bolt_core::fuzz::{random_deck, random_game};
use bolt_core::rng::Rng;
use bolt_core::{CardDef, Color, Keywords, Outcome};

fn main() {
    let n: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_000);
    let k = Keywords::NONE;
    let defs = Arc::new(vec![
        CardDef::basic_land("Plains", Color::White),
        CardDef::basic_land("Mountain", Color::Red),
        CardDef::creature("Bear", "{1}{W}", 2, 2, k),
        CardDef::creature("Ogre", "{4}{R}", 5, 5, k),
        CardDef::creature("Bird", "{W}", 1, 1, Keywords::FLYING),
        CardDef::creature("Spider", "{1}{W}", 1, 3, Keywords::REACH),
        CardDef::creature("Knight", "{1}{W}", 2, 2, Keywords::FIRST_STRIKE),
        CardDef::creature("Doubler", "{2}{W}", 2, 2, Keywords::DOUBLE_STRIKE),
        CardDef::creature("Asp", "{R}", 1, 1, Keywords::DEATHTOUCH),
        CardDef::creature("Trampler", "{3}{R}{R}", 5, 5, Keywords::TRAMPLE),
        CardDef::creature("Linker", "{1}{W}", 2, 2, Keywords::LIFELINK),
        CardDef::creature("Brute", "{2}{R}", 3, 3, Keywords::MENACE),
        CardDef::creature("Golem", "{3}", 3, 3, Keywords::INDESTRUCTIBLE),
        CardDef::creature("Ambusher", "{1}{W}", 2, 1, Keywords::FLASH),
    ]);
    let spells: Vec<usize> = (2..defs.len()).collect();
    let start = Instant::now();
    let (mut turns, mut actions, mut wins, mut draws) = (0u64, 0u64, [0u64; 2], 0u64);
    for seed in 0..n {
        let mut rng = Rng::new(seed);
        let a = random_deck(&[0, 1], &spells, &mut rng);
        let b = random_deck(&[0, 1], &spells, &mut rng);
        let r = random_game(defs.clone(), [&a, &b], seed, true).unwrap_or_else(|e| panic!("{e}"));
        turns += r.turns as u64;
        actions += r.actions as u64;
        match r.outcome {
            Outcome::Winner(p) => wins[p] += 1,
            Outcome::Draw => draws += 1,
        }
    }
    let secs = start.elapsed().as_secs_f64();
    println!(
        "{n} games, no invariant broken. {:.0} games/s with checks. Avg {:.1} turns, {:.0} actions. \
         Wins {}/{}, draws {draws}",
        n as f64 / secs,
        turns as f64 / n as f64,
        actions as f64 / n as f64,
        wins[0],
        wins[1],
    );
}
