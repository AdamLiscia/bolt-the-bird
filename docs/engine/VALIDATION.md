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

## The played-reward bot (2026-11-10)

The strength ladder says the judge rewards decks that only beat a pilot who never looks ahead. So
`train_rl.py --reward played` replaces judge A with played games: each practice table's seat-0
deck plays its 7 tablemates twice each with smart 1 × 2 on both sides, and the win rate is the
reward (`bolt.played`, `bolt_engine.play_games`). Before training, `scripts/played_reward_check.py`
showed the reward is learnable: split-half correlation 0.56 over 128 tables (reliability 0.71
with both halves), and judge A tracks it at only 0.11.

Training: 128 tables, KL 0.05, lr 3e-5 as before; ~210 s per iteration. Played evals during
training (256 test tables × 16 field-A games, smart 1 × 2): +2.73 (iteration 30), +5.44 (60),
+7.01 (90), +7.34 (120); judge B meanwhile +0.2 to +0.7. The run stopped at iteration 139 (memory
corruption, QUESTIONS.md); checkpoint 135 is evaluated.

Evaluation (`engine_pilot_validate.py newbot`, `results/engine_pilot_rl_played135.json`): the
same 512 test tables drafted exactly as in M6 (the script checks the human-model side reproduces
`data/engine_decks_512.json`), the bot's decks through the same three round-robins, paired with the
human-model games already played. 98,112 bot-side games per pilot.

| Pilot | A | B | C | **All three** | Win rate (bot / human model) | Judge-trained bot |
|---|---|---|---|---|---|---|
| Rules-based | +7.89 | +7.77 | +8.50 | **+8.06 ± 0.57** | 57.4% / 49.4% | +1.54 |
| Smart 1 × 2 (trained against) | +7.62 | +7.32 | +8.47 | **+7.81 ± 0.54** | 57.2% / 49.4% | +0.30 |
| Smart 4 × 6 (never seen) | +5.89 | +5.79 | +6.04 | **+5.91 ± 0.50** | 55.5% / 49.6% | −0.21 |

**What it says.** A reward from played games fixes what the judge got wrong: the edge holds with
a pilot the bot never trained with and with a stronger one it never saw, on every field. It shrinks
from about 8 points to about 6 as play gets stronger, so part of it is tuned to weaker play. Judge B
rates the bot only +0.63 ± 0.21 better: what it learned is mostly invisible to the judge. Caveats:
one run, cut short; the pilots are still far below a strong human; opponents are human-model
decks, not people.

## The played-reward bot, finished (2026-11-12)

The run resumed from checkpoint 135 (rl_played_b, which died at 171 with the third memory
corruption) and from 165 (rl_played_c, engine games in a restartable worker process, no crash and
no restart in ~150k games) to iteration 250. Played evals during training (smart 1 × 2, 256 tables
× 16 field-A games): +8.34 (135), +9.46 (150), +9.61 (165, 180), +10.88 (210), +11.25 (240),
+11.40 (250); judge B +0.6 to +0.9 throughout. Same evaluation as for iteration 135
(`results/engine_pilot_rl_played250.json`):

| Pilot | A | B | C | **All three** | Win rate (bot / human model) | Iteration 135 |
|---|---|---|---|---|---|---|
| Rules-based | +12.17 | +12.08 | +12.53 | **+12.26 ± 0.63** | 61.6% / 49.4% | +8.06 |
| Smart 1 × 2 (trained against) | +11.20 | +10.88 | +11.61 | **+11.23 ± 0.59** | 60.7% / 49.4% | +7.81 |
| Smart 4 × 6 (never seen) | +8.88 | +8.82 | +8.94 | **+8.88 ± 0.54** | 58.5% / 49.6% | +5.91 |

From 135 to 250 the edge grew by +4.2, +3.4 and +3.0 points with the three pilots: almost as much
with the stronger pilot it never trained against as with the one it did, so the extra training
made better decks rather than decks tuned to its training pilot. The drop from the 1 × 2 to the
4 × 6 pilot is about the same at both checkpoints (−1.9 and −2.4).

## PPO (2026-11-15)

`train_rl.py --algo ppo` replaces the one REINFORCE step per batch with PPO's clipped objective:
several steps on the same drafts (4 epochs, minibatch 1,024 picks, clip 0.2), with the same KL leash
to the human model (`bolt.rl.ppo_loss`, tests in `tests/test_rl.py`, including one showing the
gradient equals REINFORCE's before the policy moves). Same reward: seat 0's deck against its 7
tablemates, smart 1 × 2.

**Continued from the round-250 bot** (rl_ppo_from250): the smart 1 × 2 played eval went from
+11.40 (250), where REINFORCE had levelled off, to +14.45 (270), +16.11 (285), +18.92 (300),
+18.95 (315), +16.32 (330), +17.47 (345). Iteration 345 (`checkpoints/bot_rl_ppo_it345.pt`), same
evaluation as before (`results/engine_pilot_rl_ppo345.json`):

| Pilot | A | B | C | **All three** | Win rate (bot / human model) | REINFORCE, round 250 |
|---|---|---|---|---|---|---|
| Rules-based | +17.14 | +17.14 | +17.78 | **+17.36 ± 0.66** | 66.7% / 49.4% | +12.26 |
| Smart 1 × 2 (trained against) | +16.06 | +15.37 | +16.56 | **+16.00 ± 0.62** | 65.4% / 49.4% | +11.23 |
| Smart 4 × 6 (never seen) | +12.93 | +12.66 | +12.42 | **+12.67 ± 0.56** | 62.3% / 49.6% | +8.88 |

PPO added about 3.8 points against the stronger pilot it never trained against, nearly as much as
against its training pilot (+4.8), so the gain is real, not tuning to the 1 × 2 pilot. It drafts
further from people (KL to the human model 0.09 → 0.19, agreement 86% → 80%), and judge B moves
only +0.9 → +1.4.

**From scratch** (rl_ppo_scratch, the same start as the first REINFORCE run): +12.15 at iteration
30, where REINFORCE had +2.73 at 30 and +11.40 at 250, so about 8 times fewer iterations to the same
level. It peaked at +19.29 (90) and +19.43 (120), then over-optimized: +18.03 (150), +17.65 (180),
+16.72 (210), while KL rose to 0.25 and agreement fell to 77%. Only the last checkpoint was kept that
run; `train_rl.py` now keeps every one, and a rerun to 125 (rl_ppo_scratch2) recovers the peak.
PPO needs early stopping or a firmer leash than REINFORCE did.

The evaluation's field C finished in a second sitting (84% overnight, the rest on 2026-11-15).
Two more engine memory corruptions during these runs (2026-11-13 in pilot_games, 2026-11-15 in
the training worker) were retried with no lost games.

## Re-graded by the learned player (2026-11-23)

P4 of the play phase (`docs/play/`): the same games, with the learned player (`play_v2_ppo3_i850.pt`,
which beats smart 4 × 6 55.0% head to head) piloting both sides, greedy. `scripts/play_jobs_network.py`
plays the existing job files and writes the same result lines; with the rules-based player on both sides
it reproduces the engine's own results exactly (a test). All three fields, 65,408 games each.

| Gap to the human model's decks, fields a / b / c | Rules-based | Smart 4 × 6 | **Learned player** |
|---|---|---|---|
| Judge-trained bot | +1.45 / +1.53 / +1.64 | −0.16 / −0.09 / −0.37 | **+4.67 / +4.64 / +4.82** (each ± 0.60–0.63) |
| PPO bot (round 345) | +17.14 / +17.14 / +17.78 | +12.93 / +12.66 / +12.42 | **+12.64 / +12.56 / +12.51**; pooled **+12.57 ± 0.58** |

Tournament (102,400 games, average win rate against the other four): PPO from scratch 55.9%, PPO continued
55.5%, played games 53.0%, judge-trained 46.5%, human-style 39.1% (smart 4 × 6: 58.4, 57.3, 52.7, 40.7,
40.9). The PPO bot's edge doesn't move; the judge-trained bot's, which the smart player erased, comes back,
and its per-deck results now track judge B a little better (correlation 0.21 on field a, was 0.12–0.15).
776, 756 and 862 of the 65,408 games per field (about 1.2%) hit the 20,000-move cap and count as draws.
The learned player here is v2, before the targeting fix (README, "A blind spot a person found").

