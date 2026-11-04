# Bolt the Bird — plan

An AI that drafts Magic: The Gathering, builds a deck from what it drafted, and then plays it.
Trained from scratch on real Arena data. Started 2026-11-04.

## Decisions (and why)

| Decision | Choice | Why |
|---|---|---|
| Format | Arena Premier Draft | 17lands publishes millions of real picks and games per set |
| Set | Foundations (FDN) | Built as the simple, evergreen core set, so the play engine stays tractable |
| Draft model | Transformer over pack + pool | Attends across the cards you hold and the cards offered; the best fit and the best story |
| Training hardware | Mac (Apple M4, PyTorch on MPS) | Plenty for models this size; the desktop's AMD GPU has no usable PyTorch backend |
| Engine language | Rust, exposed to Python via PyO3 | RL needs millions of fast simulated games; training stays in PyTorch |
| Repo | Public GitHub, with a write-up and web demo | It's a portfolio project; the life dashboard gets a tab that reads its outputs |

## Data — 17lands public datasets, FDN Premier Draft

Checked 2026-11-04 (published 2024-12-19):

| File | Compressed | What it holds |
|---|---|---|
| `draft_data_public.FDN.PremierDraft.csv.gz` | 190 MB | One row per pick: pack contents, pool so far, the pick, player rank and win rate |
| `game_data_public.FDN.PremierDraft.csv.gz` | 60 MB | One row per game: the deck played and whether it won |
| `replay_data_public.FDN.PremierDraft.csv.gz` | 438 MB | Turn-by-turn game records, for the play engine later |

The pick file is about 7 GB unpacked (about 5M picks, extrapolated from a sample). **Never unpack it.**
`scripts/prepare.py` streams the gzip and writes compact integer arrays to `data/` (gitignored).

## Phases

### 1. Draft bot
- Each card is a token with a learned embedding. The input is the pool so far, the cards in the pack,
  and the pack/pick position. A transformer encoder scores each card in the pack, and the loss is
  cross-entropy against the human's pick.
- Train on picks from high-ranked, high-win-rate players, since those are the picks worth copying.
- **Baselines:** (a) take the card with the highest 17lands rating, (b) a card-embedding + MLP model.
- **Metrics:** top-1 pick accuracy on held-out drafts (split by draft, never by pick), broken
  down by pick number. Early picks and late picks behave very differently.

### 2. Deck builder + win-rate predictor
- A model trained on `game_data` predicts game win rate from a 40-card deck, using real human results.
- The deck builder chooses 23 spells + 17 lands from the drafted pool to maximize the predicted win rate.
- That closes the loop: the draft bot can be judged on how good its *finished deck* is, not just
  on whether it matched the human's pick.

### 3. Rust engine + RL player
- A rules engine for the FDN card pool. Start with creatures, combat and simple spells, then add
  the remaining cards. Each unsupported card is listed, never silently skipped.
- Imitation learning from `replay_data` gives a starting policy; self-play RL improves it.
- **Benchmark:** the RL player vs. a rules-based player, over thousands of games. Check that
  simulated deck win rates track the phase 2 predictor.

## Ships as
- A GitHub README with the eval write-up: charts, baselines, and what worked and what didn't
- A web demo: paste or click a pack and see the bot's pick with its reasoning (scores per card)
- A Bolt the Bird tab in the life dashboard
