# Plan

What's done, what's next, and why. Results live in the [README](README.md). The decision log
records choices with their reasons, so they don't get relitigated later.

## Status

| Phase | What | Status |
|---|---|---|
| 1 | Imitation: DraftTransformer on human picks | Done (2026-11-04): 70.5% top-1 |
| 2 | Judge, deck builder, draft simulator | Done (2026-11-04) |
| 2b | RL against judge A, evaluated by judge B | Done (2026-11-05): +3.85 pts over 550 iterations, stopped at diminishing returns |
| 3 | Rust rules engine with PyO3 bindings | Next |
| 4 | Gameplay agent: rules-based, then imitation, then self-play | After 3 |

## Phase 3: the rules engine

**Goal:** play decks out, so "judged win rate" becomes "played win rate." The working plan,
milestones and session protocol are in [docs/engine/ENGINE_PLAN.md](docs/engine/ENGINE_PLAN.md). The
card audit ([docs/engine/CARD_AUDIT.md](docs/engine/CARD_AUDIT.md)) found that Limited decks are
diverse: 219 of 281 cards are needed before half of real decks are playable. So the engine aims for
the whole set, and is useful early through drafts restricted to supported cards.

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
| 2026-11-04 | Whole set, plus restricted drafts early | The card audit: partial card coverage covers few real decks |
| 2026-11-05 | Stop RL at 550 iterations | Gain flat (+0.02 over 25 iterations) and agreement with the human model at the 80% drift limit |
| 2026-11-05 | Keep the White-Red lean | Real games back it (+2.1 vs. pilot skill, under 8% of decks). Not a judge exploit |
