# Plan

What's done, what's next, and why. Results live in the [README](README.md). The decision log
records choices with their reasons, so they don't get relitigated later.

## Status

| Phase | What | Status |
|---|---|---|
| 1 | Imitation: DraftTransformer on human picks | Done (2026-11-04): 70.5% top-1 |
| 2 | Judge, deck builder, draft simulator | Done (2026-11-04) |
| 2b | RL against judge A, evaluated by judge B | Done (2026-11-05): +3.85 pts over 550 iterations, stopped at diminishing returns |
| 3 | Rust rules engine with PyO3 bindings | Done (2026-11-06): all 286 cards; played edge +1.54 ± 0.66 (rules-based player), −0.21 ± 0.58 (smart player) vs. +3.85 judged |
| 3b | RL rewarded by played games | Done (2026-11-12): 250 iterations; +8.88 ± 0.54 pts played with the 4 × 6 smart player it never trained against (58.5% vs. 49.6%), +12.26 with the rules-based pilot |
| 3c | PPO for the played-reward drafter | Done (2026-11-15): +12.67 ± 0.56 pts with the 4 × 6 smart player (62.3% vs. 49.6%); from scratch it matches the old method's best in 30 iterations instead of 250, then over-optimizes past ~120 |
| 4 | Gameplay agent: rules-based, then imitation, then self-play | Done (2026-11-24): v3 beats smart 4 × 6 57.9%, rules-based 75.9%, smart 1 × 2 68.0% (1,000 games each); ~790,000 self-play games. Known flaw: pumps aimed at the opponent's creature 47% of the time. Plan and log in [docs/play/](docs/play/) |
| 4b | Re-grade the drafters with the learned player (P4) | Done (2026-11-23): PPO drafter +12.57 ± 0.58 (unchanged); judge-trained drafter +4.7 on every field (was −0.2) |
| 5 | Play against it: FastAPI service ([docs/api.md](docs/api.md)) | In testing (2026-11-23): Nerd Leagues' admin-only "Practice vs Bolt" (games, two-person games, drafts with 7 bots, whole-table decks) |
| 6 | Second set: Tarkir: Dragonstorm | Started (2026-11-23): 17lands data downloaded, engine holds several sets (branch `tarkir`), 18 of 270 cards supported |
| — | Fix the pump-targeting habit | Next. Until then the service has smart 1 × 2 choose the learned player's targets |

## Phase 3: the rules engine

**Goal:** play decks out, so "judged win rate" becomes "played win rate." The working plan,
milestones and session protocol are in [docs/engine/ENGINE_PLAN.md](docs/engine/ENGINE_PLAN.md). The
card audit ([docs/engine/CARD_AUDIT.md](docs/engine/CARD_AUDIT.md)) found that Limited decks are
diverse: 219 of 281 cards are needed before half of real decks are playable. So the engine aims for
the whole set, and is useful early through drafts restricted to supported cards.

## Who does what (Adam, 2026-11-25)

- **The desktop (ROONS-DESKTOP, WSL2 `Ubuntu-24.04`) runs all training and simulation** from now on:
  Tarkir play training first, then the drafter experiments, the Foundations targeting sims, and
  Commander/Tarkir self-play. Its Claude session runs, monitors and recovers jobs, and writes only the
  machine-side pieces (the game-pause watcher, the startup task). Design: `docs/desktop-farm.md`.
- **This repo's code has one owner, the Mac session:** the engine, cards, the farm library and job
  definitions. It pushes to GitHub; the desktop pulls `tarkir` (or whatever branch is named next) and runs.
  Engine edits are never handed to the desktop.
- **The Mac keeps the game services** (8766 FDN, 8767 Tarkir, 8768 Commander) and may still do occasional
  GPU training (drafter transformers on MPS) when that's faster and Adam isn't streaming.
- **Job handoff:** when a job is ready, its definition (command, checkpoint and S3 paths, resources) is
  committed and pushed, and the Nerd Leagues orchestrator relays it to the desktop. Results come back
  through S3.

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
| 2026-11-06 | Report judged and played results side by side | Played games show about a third of the judged edge. Both numbers are kept, with the gap explained |
| 2026-11-07 | Report the round-robin, not the 16k sample | Full round-robins on three fields: the smart player's edge is −0.21 ± 0.58, not +0.79. The judged edge doesn't survive stronger play |
| 2026-11-09 | Reward the drafter with played games (smart 1 × 2 vs. tablemates) | The ladder showed the judge rewards decks that only beat players who never look ahead; a check found the played reward learnable (split-half 0.56) and judge A nearly blind to it (0.11) |
| 2026-11-09 | Call the rollout pilot the "smart player" in anything public | Adam: "search" means something else in Magic (search your library) |
| 2026-11-15 | PPO for the played-reward drafter; stop by checkpoint selection | Reached REINFORCE's best in 30 iterations instead of 250; past ~120 iterations it over-optimizes |
| 2026-11-18 | Play network: copy the smart player first, then self-play | Copying alone won 44.6% vs rules-based; DAgger didn't help, since the smart player's edge is lookahead. Self-play with PPO is the method |
| 2026-11-20 | GAE (λ 0.95) and 640 games per update for play PPO | End-of-game credit plateaued near 55% vs rules-based; per-move credit and bigger batches took it past 70% |
| 2026-11-18 | One heavy job at a time, memory caps on every run | Two jobs at once froze the Mac (2026-11-18) |
| 2026-11-22 | Serve Bolt over a local FastAPI service, never touching Nerd Leagues' database | Nerd Leagues builds the page; Bolt owns the rules, the bots and hidden information |
| 2026-11-23 | Show the network the spell it's choosing targets for | It aimed Giant Growth at a person's creature: the spell wasn't in its input during targeting |
| 2026-11-23 | In the service, smart 1 × 2 picks the learned player's targets | Retraining with the new input fixed strength but not the habit (pumps still 47% wrong) |
| 2026-11-23 | Next set: Tarkir: Dragonstorm before the Commander precons | Draft data exists on 17lands, so the drafter pipeline applies; precons need card scripting first |
| 2026-11-24 | Every public number lives in docs/headline.json; scripts/check_writeups.py checks each writeup | A changed result must not leave the website, README or resumes behind |
