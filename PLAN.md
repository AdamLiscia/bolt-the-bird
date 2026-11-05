# Plan

What's done, what's next, and why. Results live in the [README](README.md). The decision log
records choices with their reasons, so they don't get relitigated later.

## Status

| Phase | What | Status |
|---|---|---|
| 1 | Imitation: DraftTransformer on human picks | Done (2026-11-04): 70.5% top-1 |
| 2 | Judge, deck builder, draft simulator | Done (2026-11-04) |
| 2b | RL against judge A, evaluated by judge B | v0.1 done: +1.99 pts over 150 iterations. A longer run is in progress |
| 3 | Rust rules engine with PyO3 bindings | Next |
| 4 | Gameplay agent: rules-based, then imitation, then self-play | After 3 |

## Phase 3: the rules engine

**Goal:** play decks out, so "judged win rate" becomes "played win rate."

1. **Card audit.** Sort all 281 Foundations cards into tiers by rules complexity: vanilla and
   keywords, simple spells, triggers, statics, and one-offs. Measure the share of real 17lands decks
   each tier covers.
2. **Rules core.** Zones, turn structure, priority, mana, casting and the stack, combat with
   evergreen keywords, state-based actions.
3. **Cards in tiers.** Data-driven definitions for the common patterns, and hand-written code for the
   rest. Unsupported cards are listed, never silently skipped.
4. **Testing.** Rules tests per card from the card's text. Random-agent fuzzing across thousands of
   games to find crashes and rules violations.
5. **Python bindings.** Expose state, legal actions and step, so gameplay agents train in Python.
6. **Validation.** Play bot decks against human-model decks. Do played win rates rank decks the same
   way the judges do?

## Decision log

| Date | Decision | Why |
|---|---|---|
| 2026-11-04 | Foundations, Premier Draft | Simplest recent set with deep 17lands data, which keeps the engine tractable |
| 2026-11-04 | Train on the Mac (M4, MPS), not the desktop | The desktop's RX 5700 XT has no reliable PyTorch backend |
| 2026-11-04 | Split by draft | Splitting by pick leaks the pool between train and test |
| 2026-11-04 | Skill as a judge input | Good players build better decks *and* pilot better |
| 2026-11-04 | Two judges on disjoint halves | Grade the RL bot with a judge it never trained against |
| 2026-11-04 | RL on CPU | A NaN-gradient bug hung the Apple GPU. Fixed and covered by a test, but CPU is fast enough |
| 2026-11-04 | Rust for the engine | Self-play needs millions of fast games. PyO3 keeps training in Python |
