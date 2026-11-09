# Model card

Four trained models make up Bolt the Bird v0.2.0. All four are trained for one set: Magic: The
Gathering **Foundations (FDN)**, Arena **Premier Draft**.

## Data

[17lands public datasets](https://www.17lands.com/public_datasets) (CC BY 4.0). 17lands collects
drafts and games from Arena players who run its tracker, and publishes them without player identities.

| | Rows | Train | Val | Test |
|---|---|---|---|---|
| Picks | 5,530,605 from 132,431 drafts | 85% | 5% | 10% |
| Games | 791,159 | 674,335 | 38,445 | 78,379 |

- **Splits are by draft**, using a hash of `draft_id`, and the same hash splits picks and games.
  No draft contributes to both training and evaluation.
- **The two judges** split the training drafts again by a second, independent hash bit, so judge A
  and judge B never see the same game.
- **Known skew.** 17lands users are stronger than the average Arena player: the training games are
  won 54.6% of the time, not 50%. Results describe this population.

## Models

### `human_model.pt`: DraftTransformer

- **Task:** given the drafter's pool and the current pack, score each card in the pack.
- **Architecture:** one token per pool card and per pack card, plus a draft-position token.
  4-layer pre-norm transformer encoder, d=192, 6 heads, no sequence positions (a pool is a set).
  1.84M parameters.
- **Training:** cross-entropy against the human's pick. 3 epochs, AdamW, one-cycle LR (peak 1e-3),
  batch 1024. About 4 h on an Apple M4 using PyTorch MPS.
- **Result:** 70.5% top-1 agreement on 546,149 held-out picks (71.8% for drafters with a 58%+ game
  win rate). Baselines: pick-order bot 46.9%, pool-aware MLP 68.8%.

### `judge_a.pt` and `judge_b.pt`: DeckJudge

- **Task:** predict P(win) for one game from a 40-card deck, the pilot's skill bucket, and play/draw.
  Decks are scored with skill fixed at the median (0.54) and play/draw averaged.
- **Architecture:** a linear layer from card counts to a 96-d embedding sum, then a 2-layer MLP, plus a
  per-card linear term. 30 epochs, AdamW.
- **Result:** test AUC 0.596 (judge A) and 0.597 (judge B). The full-data judge reaches 0.598,
  against 0.584 for skill and play/draw alone. It is calibrated to within about a point across
  predicted-win-rate quintiles.

### `bot.pt`: the RL-trained drafter

- **Init:** `human_model.pt`.
- **Training:** REINFORCE with a batch-mean baseline. The reward is judge A's predicted win rate of
  the deck the builder makes from the bot's 42 picks. A KL penalty (β=0.05) pulls toward the human
  model. Each iteration is 128 simulated 8-seat tables, with the 7 other seats run by the human
  model (sampling). 550 iterations, 70,400 drafts, about 7 hours on CPU. Training stopped when the
  gain flattened (+0.02 points over the last 25 iterations) and agreement with the human model reached
  the 80% drift limit.
- **Evaluation:** 512 held-out tables (packs from test drafts), seat 0 greedy, with the same seed for
  bot and baseline. Decks are built and scored by **judge B**.
- **Result:** +3.85 ± 0.23 points of judged win rate over the human model (63.87% vs. 60.02%).
  Rares taken after pack 1 fell from 2.54 to 2.18 per draft. It agrees with the human model on 81%
  of picks. v0.1.0 shipped the iteration-150 bot (+1.99 ± 0.18).
- **Played result:** in the project's Rust rules engine, the same decks win **+1.54 ± 0.66** points
  more played games than the human model's with a rules-based pilot, and **−0.21 ± 0.58** with a
  smart player (it plays each candidate move out before choosing) that beats it 74.5% of the time (each deck against every deck of three separate
  64-deck fields, 196,224 games per pilot). The judged edge does not survive play. See
  `docs/engine/VALIDATION.md`.
- **Known bias:** it drafts White-Red at 45% of tables. In real 17lands games White-Red outperforms
  its pilots' skill by 2.1 points while making up under 8% of games, so the lane is strong and usually open
  against human-like drafters. It would be contested at a table of bots.

## Intended use

- Research and teaching: imitation learning, offline evaluation, reward models, RL fine-tuning
  with a KL leash.
- A practice drafting partner.

## Out of scope

- Other sets. Card IDs are FDN-specific, so another set needs its own data and a retrain.
- Running against live Arena drafts. That is against the game's terms of service.
- Claims about games between people. The played win rates come from engine pilots, which play
  far below a strong human, against a field of model-drafted decks.

## Limitations and risks

- **The judges learned from human-built decks only.** They can share blind spots, so a bot can find
  decks both judges overrate. The held-out judge B catches overfitting to judge A, but not a blind
  spot the two judges share. Played games in the rules engine suggest this happened: judge B
  predicts +3.85 points, play shows +1.5 with simple pilots and about zero with a stronger one, and
  judge B's per-deck scores correlate only 0.12–0.15 with played win rates.
- **Agreement with humans is a proxy, not a goal.** Humans disagree with each other, and they
  rare-draft (see the README), so 100% is neither possible nor desirable.
- **Opponents are simulated.** The 7 other seats are the imitation model, so the RL bot is
  optimized against model-like neighbors, not real people.
