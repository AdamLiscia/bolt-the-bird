# Bolt the Bird

An AI that drafts Magic: The Gathering, builds its deck, and plays it, trained from scratch on
real Arena data from [17lands](https://www.17lands.com/public_datasets).

**Status:** just started. See [PLAN.md](PLAN.md).

1. **Draft:** a transformer that picks cards the way strong human drafters do
2. **Build:** turns a drafted pool into a 40-card deck, scored by a win-rate model trained on real games
3. **Play:** a Rust rules engine and a reinforcement-learning agent that plays the deck

Set: Foundations (FDN). Data: 17lands public datasets (CC BY 4.0).

*Bolt the Bird is unofficial Fan Content permitted under the Fan Content Policy. Not
approved/endorsed by Wizards. Portions of the materials used are property of Wizards of the
Coast. ©Wizards of the Coast LLC.*

## Results so far — draft bot (FDN, held-out test drafts)

Top-1 agreement with the human's actual pick on 546,149 picks from 13k drafts the models never saw.

| Bot | All drafters | Strong drafters (≥58% game win rate) |
|---|---|---|
| Random | 23.2% | 23.3% |
| Ratings bot (games-in-hand win rate) | 36.5% | 38.8% |
| Pick-order bot (take rate when seen) | 46.9% | 48.9% |
| PoolMLP (145k params) | 68.8% | 70.1% |
| **DraftTransformer** (1.8M params, 3 epochs, ~4 h on an M4) | **70.5%** | **71.8%** |

Splits are by draft (hash of `draft_id`), so no draft appears in both training and test.
