# Engine log

One line per working step, newest last. The next session continues from the last line.

- 2026-11-04: Card audit done (`CARD_AUDIT.md`). Plan written (`ENGINE_PLAN.md`). Rust 1.92 installed
  via Homebrew rustup (binary at `/opt/homebrew/opt/rustup/bin`). **Next: M0, the Cargo workspace and CI.**
- 2026-11-05: **M0 done.** Cargo workspace `engine/` (`bolt-core`, `bolt-cards`), Rust pinned to 1.92,
  `forbid(unsafe_code)`, CI job running fmt, clippy `-D warnings` and tests, `make engine-test`. 3 tests.
  `bolt-py` deferred to M5 (QUESTIONS). **Next: M1, starting with game state, zones and the turn structure.**
