# Bolt the Bird

An AI that drafts Magic: The Gathering, builds a deck from what it drafted, and (eventually)
plays it. Trained from scratch on real Arena data from
[17lands](https://www.17lands.com/public_datasets). Set: Foundations (FDN), Premier Draft.

The project has two goals. The first is to draft like a person: learn from 5.5 million real
picks. The second is to draft *better* than the people it learned from, by rewarding finished
decks that win instead of picks that match.

| Phase | What | Status |
|---|---|---|
| 1 | Draft model trained on human picks | Done: 70.5% top-1 agreement on held-out drafts |
| 2 | Deck judge, deck builder, draft simulator | Done |
| 2b | Reinforcement learning against the judge | Done: +2.0 points of judged win rate over the human model |
| 3 | Rust rules engine + a player trained by self-play | Planned ([PLAN.md](PLAN.md)) |

## Data

17lands records Arena drafts and games from players who run its tracker, and publishes them
anonymously.

- **Picks:** 5,530,605 picks from 132,431 drafts. Each row has the pack, the drafter's pool so
  far, the pick, and the player's rank and win-rate bucket.
- **Games:** 791,159 games. Each row has the 40-card deck, cards drawn, play/draw, and the result.

`scripts/prepare.py` streams the gzipped CSVs (about 7 GB of picks unpacked) into 114 MB of integer
arrays without ever writing the CSV to disk. **Every split is by draft** (a hash of `draft_id`):
85% train, 5% validation, 10% test. Splitting by pick would leak, because pick 4 of a draft
reveals most of the pool from pick 3.

## Phase 1: drafting like a person

Each model sees the drafter's pool and the current pack, and scores every card in the pack.

| Bot | Top-1 agreement, all drafters | Strong drafters (≥58% game win rate) |
|---|---|---|
| Random | 23.2% | 23.3% |
| Ratings bot: highest games-in-hand win rate | 36.5% | 38.8% |
| Pick-order bot: highest take rate when seen | 46.9% | 48.9% |
| PoolMLP: summed pool embedding + MLP (145k params) | 68.8% | 70.1% |
| **DraftTransformer** (1.8M params) | **70.5%** | **71.8%** |

Measured on 546,149 picks from about 13,000 test drafts. The baselines' card ratings come from
training data only.

- **Context is most of the win.** Both bots that ignore the pool top out at 47%. Seeing the pool
  adds about 22 points. The gap is widest mid-pack, once the drafter is committed to colors
  (`results/` has accuracy per pick number).
- **The transformer adds 1.7 points over the MLP.** It's a real gap at this sample size, but a
  modest one. The DraftTransformer gives each pool card and each pack card its own token, plus a
  draft-position token, with no sequence positions because a pool is a set. Self-attention lets
  each pack card look at the pool ("does this fit?") and at the rest of the pack ("what am I
  passing?"). 4 layers, d=192, 3 epochs, about 4 h on an Apple M4 using PyTorch MPS.
- **Copying only strong drafters didn't help.** Fine-tuning on players with a 58%+ game win rate
  left accuracy unchanged (71.9% on strong drafters vs. 71.8%). The next finding explains why.

## Finding: people rare-draft, and good players do it too

For each draft, take the drafter's two main colors from their final pool. Then, from pack 2
onward, ask how often a card outside those colors gets taken when it's in the pack
(`scripts/rare_drafting.py`).

| Off-color card in the pack, packs 2–3 | Taken |
|---|---|
| Rare or mythic | 11.2% |
| Common or uncommon | 3.5% |

Strong drafters do it at the same rate as everyone else (11.0% vs. 11.2%). On Arena you keep
what you draft, so a rare can be worth more to your collection than to your deck. Drafts with
two or more late off-color rare picks average 2.70 match wins, against 2.94 for drafts with
none. That's a correlation, not proof: a drafter who never settles into colors would show both.

The consequence is that rare-drafting can't be filtered out of the data. A bot that copies
humans copies the habit. To draft better than people, the bot has to optimize for winning.

## Phase 2: a judge, a deck builder, a simulator

**The judge** predicts a deck's game win rate. Player skill and play/draw are separate inputs:
good players build better decks *and* pilot them better, so without the skill input the judge
would credit the deck for the pilot. Decks are always scored with skill fixed at the median.

| Model (78,379 held-out games) | Log loss | AUC |
|---|---|---|
| Skill and play/draw only | 0.6772 | 0.584 |
| + one weight per card (linear) | 0.6745 | 0.596 |
| + card interactions (embedding sum + MLP) | **0.6734** | **0.598** |

AUC barely moves because one game of Magic is mostly noise, but the predictions hold up. Grouped
into fifths by predicted win rate, held-out decks predicted at 47.4 / 52.1 / 54.8 / 57.4 / 61.7%
actually won 46.3 / 52.0 / 54.8 / 57.4 / 62.1%. The cards the linear judge values most
(Embercleave; Liliana, Dreadhorde General; Curator of Destinies; and so on) are the set's known
bombs.

**The deck builder** tries each of the 10 color pairs. For each, it greedily adds whichever
castable spell raises the judged win rate most, includes on-color dual lands and fixers (land
colors from Scryfall), and tries both 23 spells/17 lands and 24/16. On 966 real players' own
final pools, its decks keep 79% of the player's spells, with matching creature and land counts
(14.8 vs. 15.3 creatures, 16.6 vs. 16.7 lands). The judge rates them 57.9% against the
humans' 54.5%. That figure comes from the same judge the builder optimizes, so read it as an
upper bound.

**The draft simulator** seats 8 drafters at a table with real opening packs from the data, 3
packs of 14, passing left-right-left. The seat under test gets one model; the other 7 seats run
the human-imitation model, sampling picks the way people make them.

## Phase 2b: learning to win

Seat 0 starts as the human-imitation model. Each iteration drafts 128 simulated tables, builds
seat 0's deck, and uses its judged win rate as the reward (REINFORCE with a batch-mean baseline).
A KL penalty keeps the bot close to the human model, so it improves on human drafting instead of
drifting into decks no one would build.

**To catch reward hacking, two judges are trained on disjoint halves of the games.** The bot
trains against judge A and is graded by judge B, building its decks with B too. A bot that only
learned to fool A won't score with B.

150 iterations × 128 tables (19,200 simulated drafts), CPU only, 2 h 6 min. Each evaluation
drafts 512 held-out tables twice, with the same packs and the same seed: once with the bot in
seat 0 and once with the human model. Judge B builds and scores both decks.

| Iteration | Bot (judge B) | Human model (judge B) | Gain, points | Rares taken after pack 1 | Picks matching the human model |
|---|---|---|---|---|---|
| 0 | 60.02% | 60.02% | 0.00 | 2.54 | 100% |
| 45 | 61.03% | 60.02% | +1.00 ± 0.13 | 2.37 | 95.9% |
| 90 | 61.62% | 60.02% | +1.60 ± 0.17 | 2.32 | 93.3% |
| 150 | **62.01%** | 60.02% | **+1.99 ± 0.18** | **2.15** | 90.3% |

- **The gain holds on the judge the bot never trained against.** That rules out the most direct
  form of reward hacking. Both judges still learned from the same kind of human-built decks, so
  they can share blind spots. Only played games will settle that.
- **It rare-drafts less without being told to:** 15% fewer rares after pack 1. Nothing in the
  reward mentions rarity. It learned that some of those picks cost wins.
- **It's still close to human drafting.** It agrees with the human model on 90% of picks, and the
  KL penalty kept the divergence small (0.07 nats per pick).
- **The gain was still rising at iteration 150.** Longer runs are the obvious next experiment.

## Limitations

- **Agreement with humans is a proxy.** Strong drafters disagree with each other, so the ceiling
  is well below 100%.
- **The judge only knows human-built decks.** Decks far from that distribution may be scored
  badly; the KL leash and the second judge are the guards.
- **Deck quality is judged, not played.** Real validation needs games, which is what phase 3's
  engine is for.
- **One set so far.** Other sets need their own 17lands data and a retrain.

## Reproduce

```bash
python -m venv .venv && .venv/bin/pip install -r requirements.txt
# download draft_data_public.FDN.PremierDraft.csv.gz and game_data_public.FDN.PremierDraft.csv.gz
# from 17lands into data/raw/
python scripts/prepare.py                 # stream to compact arrays
python scripts/card_info.py               # colors, rarity, land colors
python scripts/baselines.py
python scripts/train.py --model mlp --epochs 2
python scripts/train.py --model transformer --epochs 3
python scripts/rare_drafting.py
python scripts/train_judge.py             # full judge + baselines
python scripts/train_judge.py --half 0    # judge A
python scripts/train_judge.py --half 1    # judge B
python scripts/eval_builder.py
python scripts/train_rl.py --device cpu   # or mps
```

Data: 17lands public datasets (CC BY 4.0). Card data: Scryfall.

*Bolt the Bird is unofficial Fan Content permitted under the Fan Content Policy. Not
approved/endorsed by Wizards. Portions of the materials used are property of Wizards of the
Coast. ©Wizards of the Coast LLC.*
