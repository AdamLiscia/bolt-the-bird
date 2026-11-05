# Engine log

One line per working step, newest last. The next session continues from the last line.

- 2026-11-04: Card audit done (`CARD_AUDIT.md`). Plan written (`ENGINE_PLAN.md`). Rust 1.92 installed
  via Homebrew rustup (binary at `/opt/homebrew/opt/rustup/bin`). **Next: M0, the Cargo workspace and CI.**
- 2026-11-05: **M0 done.** Cargo workspace `engine/` (`bolt-core`, `bolt-cards`), Rust pinned to 1.92,
  `forbid(unsafe_code)`, CI job running fmt, clippy `-D warnings` and tests, `make engine-test`. 3 tests.
  `bolt-py` deferred to M5 (QUESTIONS). **Next: M1, starting with game state, zones and the turn structure.**
- 2026-11-05: **M1 rules core.** `bolt-core`: game state, London mulligan, play/draw, the full turn
  structure, priority and the stack, one land per turn, automatic mana payment, casting creatures
  (flash), combat with flying, reach, first strike, double strike, deathtouch, trample, lifelink,
  vigilance, haste, menace, defender and indestructible, state-based actions (life, decking, lethal
  damage, deathtouch, toughness, legend rule), discard to seven. 43 tests, each rule citing the CR.
  Fuzzer: 10,000 random-agent games, no panic, every game ends, invariants checked after every action
  (6,261 games/s with checks, `cargo run --release --example random_games`). Simplifications are in
  QUESTIONS. **Next: M1 card data, loading real FDN creatures into `bolt-cards`.**
