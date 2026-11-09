# M6: played games against judge B

Judge B says the RL bot's decks are better than the human model's. This is the first test of that
claim in **played games**, using the rules engine.

**Setup** (`scripts/engine_validate.py`, `results/engine_validation.json`, 326 s on CPU):

- **Decks:** the 512 held-out tables the RL evaluation uses (test packs, seed 12345, seat 0
  greedy), drafted once by the RL bot and once by the human model, then built by
  `bolt.deckbuild` and scored by judge B. Judge B's numbers reproduce the published ones exactly.
- **Pilot:** the rules-based baseline (`bolt_core::baseline`) plays both sides of every game.
- **Head-to-head:** each table's bot deck against the human model's deck from the same packs,
  40 games per table (20,480 games), alternating who plays first.
- **Gauntlet:** all 1,024 decks against a fixed field of 64 human-model decks from other tables,
  4 games per opponent (258,048 games).
- 11 games hit the 200-turn limit and count as draws (see `QUESTIONS.md`).

## Results

| | Judge B (predicted) | Played, gauntlet | Played, head-to-head |
|---|---|---|---|
| Bot decks | 63.87% | 51.10% | 51.36% ± 0.80 |
| Human-model decks | 60.02% | 49.51% | |
| Gap | **+3.85 ± 0.23** | **+1.59 ± 0.64** | +1.36 (vs. 50%) |

Played win rate by judge B quintile (gauntlet, all 1,024 decks):

| Judge B | 55.2% | 59.1% | 61.6% | 64.4% | 69.4% |
|---|---|---|---|---|---|
| Played | 48.5% | 48.3% | 49.0% | 52.4% | 53.3% |

Correlation between judge B's score and the played win rate across decks: Pearson 0.14,
Spearman 0.13.

## What this says

- **The direction holds.** The bot's decks win more played games than the human model's: +1.6
  points in the gauntlet, 2.5 standard errors from zero. Head-to-head they win 51.4%, 1.7
  standard errors above even, so that comparison is suggestive rather than conclusive.
- **The size doesn't.** Played games show about 40% of the gap judge B predicts.
- **Judge B ranks decks weakly under this pilot.** The top two quintiles win about 4 points more
  than the bottom three. Within each group the ordering is mostly noise.

## Caveats

- **The pilot is simple.** It curves out and makes safe attacks; it doesn't hold up removal, read
  combat tricks or plan turns ahead. Judge B learned from humans playing, so decks that reward
  skill can look worse here than they are. A stronger pilot is the obvious next step.
- **One field.** The gauntlet field is human-model decks only, not the real 17lands field.
- **Same packs.** The head-to-head decks come from identical packs, which keeps the pairing fair
  but means both decks often share colors and cards.

# M7: the same question with a stronger pilot

The obvious objection to M6 is the pilot. M7 replaces it with a search pilot
(`bolt_core::rollout`): at each priority, attack and block decision it tries up to 6 moves, plays
each out 4 times with the baseline in worlds where the hidden cards are guessed again
(`Game::determinize`, no peeking), and keeps the move that wins most. It beats the baseline
**72.5% ± 3.1** on random decks (`rollout_bench`, 200 games).

**Setup** (`scripts/engine_pilot_validate.py`, `engine/bolt-cards/examples/pilot_games.rs`,
`scripts/engine_pilot_overnight.sh`): the same 1,024 M6 decks. Each table's bot deck and
human-model deck play every deck of a 64-deck human-model field once, with the same seeds, so the
gap between them is paired: 65,408 games per pilot per field, about 4½ hours on 8 CPU threads for
the rollout pilot. Field A is M6's field (`results/engine_pilot_validation.json`). Fields B and C
are replications: 64 other human-model decks each, no deck shared between fields, new seeds
(`results/engine_pilot_validation_b.json`, `_c`). The baseline plays the identical games as a
control. Rollout games stopped at the 180-second cap (4 in A, 5 in B, 5 in C) count as draws.

Gap: the bot decks' played win rate minus the human-model decks', in points, ± one standard error
over the 512 tables.

| Field | Judge B (predicted) | Played, baseline pilot | Played, rollout pilot | Rollout − baseline |
|---|---|---|---|---|
| A | +3.85 ± 0.23 | +1.45 ± 0.73 | −0.16 ± 0.64 | −1.61 ± 0.51 |
| B | +3.85 ± 0.23 | +1.53 ± 0.71 | −0.09 ± 0.65 | −1.63 ± 0.50 |
| C | +3.85 ± 0.23 | +1.64 ± 0.70 | −0.37 ± 0.62 | −2.02 ± 0.50 |
| **A + B + C** | **+3.85 ± 0.23** | **+1.54 ± 0.66** | **−0.21 ± 0.58** | **−1.75 ± 0.38** |

Win rates behind field A: bot decks 51.04% / 49.26% (baseline / rollout), human-model decks
49.59% / 49.43%; field B: 52.44% / 50.93% and 50.91% / 51.02%. Per-deck correlation with judge B's
predicted win rate is 0.12 (baseline) and 0.14–0.15 (rollout).

**The first sample was high.** M7 first ran 16 of the 64 field opponents per deck (16,384 games):
baseline +1.50 ± 0.90, rollout +0.79 ± 0.88, difference −0.71 ± 0.90, which read as "no change".
The full round-robin on field A put the rollout gap at −0.16, and field B repeated it.

**More fields won't shrink the error much.** A table's gap on field A and on field B correlate
0.74 (rollout) and 0.79 (baseline): the result is mostly set by the 512 deck pairs, not by which
opponents they meet. Adding field B moved the rollout error from ±0.64 to ±0.61. Tightening it
needs more drafts (new tables), not more games.

## What M7 says

- **The bot's edge depends on how well the decks are played.** With the baseline on both sides the
  bot's decks win about 1.5 points more often, on three separate fields. With the rollout pilot on
  both sides the edge is gone (−0.21 ± 0.58), on all three fields, and the drop is more than four standard errors.
- **Either way, judge B overstates it.** It predicts +3.85. A stronger pilot moves played results
  further from that, not closer. The RL bot was trained against judge A and evaluated by judge B;
  they learned from different halves of the same kind of human game data, and the bot found deck
  traits both reward more than play does (the shared blind spot the README already names).
- **A guess at why, not tested:** whatever the bot's decks do better pays off when both sides play
  by simple rules, and a pilot that looks ahead neutralizes it.
- **Caveat:** the rollout pilot is still weak. It searches only its next move, assumes the opponent
  never responds (no counterspells or tricks in its playouts), and leaves targets, discards and
  mulligans to the baseline. Strong play may change the answer again; a self-play pilot is the
  test for that.

## The strength ladder (2026-11-09)

The same three round-robins with two weaker search pilots in between (`scripts/engine_pilot_ladder.sh`;
strength from `rollout_bench`, 200 games each against the baseline). Gap in points, averaged per
table over fields A, B and C (196,224 games per pilot):

| Pilot | Beats the baseline | A | B | C | **All three** | Change from baseline |
|---|---|---|---|---|---|---|
| Baseline | 50% | +1.45 | +1.53 | +1.64 | **+1.54 ± 0.66** | |
| Rollout 1 × 2 | 65.0% ± 3.4 | +0.27 | −0.01 | +0.65 | **+0.30 ± 0.64** | −1.24 ± 0.26 |
| Rollout 2 × 4 | 70.0% ± 3.1 | +0.24 | −0.17 | −0.45 | **−0.13 ± 0.60** | −1.67 ± 0.32 |
| Rollout 4 × 6 | 74.5% ± 2.9 | −0.16 | −0.09 | −0.37 | **−0.21 ± 0.58** | −1.75 ± 0.38 |

**It's a cliff.** The weakest search, one playout of two moves, already removes about 80% of the
edge (−1.24 of −1.54, nearly five standard errors). The rest goes by 2 × 4. The bot's decks are
better only against a pilot that never looks ahead. Two fixes landed during these runs: a
panicking game is recorded and the run goes on (none recurred), and playouts past 2,000 objects
count as a draw (QUESTIONS.md, 2026-11-08). Field B's 2 × 4 run had 36,308 games before that
fix and the rest after.
