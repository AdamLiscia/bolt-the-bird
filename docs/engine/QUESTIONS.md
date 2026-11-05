# Open questions for Adam

Judgment calls made during autonomous sessions. Each one has the choice taken and why, so it can be
reversed later.

## 2026-11-05: `bolt-py` waits until M5

**Choice:** the M0 workspace has `bolt-core` and `bolt-cards` only. The PyO3 crate is added at M5,
when there is an API to bind.
**Why:** a PyO3 extension crate needs a Python toolchain to build and link its tests, which slows CI
and adds a failure mode before there's anything to expose. Nothing else depends on it until M5.
**To reverse:** add `engine/bolt-py` with `pyo3 = { features = ["extension-module", "abi3-py311"] }`.

## 2026-11-05: Rust pinned to 1.92 at the repo root

**Choice:** `rust-toolchain.toml` pins 1.92 with clippy and rustfmt. CI uses the same version.
**Why:** a new clippy release can add lints, and with `-D warnings` that breaks CI with no code change.
Upgrades become one deliberate commit.
