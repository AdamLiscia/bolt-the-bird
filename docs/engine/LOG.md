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
- 2026-11-05: **M1 done.** `bolt-cards` loads all 286 FDN cards from `assets/FDN_oracle.json` in model
  order (a card's index is its draft-model index). `supported()` derives support from rules text and is
  pinned by a test: the 5 basics and the 8 keyword-only creatures (tier 1). Per-card tests, real-card
  combat tests (Nighthawk vs. Serra Angel, Swiftblade Vindicator trampling over on the second strike,
  Brazen Scourge's haste), and 300 fuzz games with real supported cards. 56 tests in the workspace.
  **Next: M2, starting with targets (with a zone-change counter, see QUESTIONS) and the effect DSL.**
