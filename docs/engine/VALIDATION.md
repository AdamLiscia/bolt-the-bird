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
`results/engine_pilot_validation.json`): the same 1,024 M6 decks. Each table's bot deck and
human-model deck play the same 16 opponents from M6's 64-deck field with the same seeds, one game
each (16,384 games per pilot, about 70 minutes on 8 CPU threads for the rollout pilot). The search
pilot is far too slow for M6's 258,000 games, so this is a sample of the gauntlet; the baseline
plays the identical games as a control. Three rollout games stopped at the 180-second cap and count
as draws.

| | Judge B (predicted) | Played, baseline pilot | Played, rollout pilot |
|---|---|---|---|
| Bot decks | 63.87% | 50.54% | 50.00% |
| Human-model decks | 60.02% | 49.04% | 49.21% |
| Gap | **+3.85 ± 0.23** | **+1.50 ± 0.90** | **+0.79 ± 0.88** |
| Correlation with judge B, per deck | | 0.16 | 0.13 |

The baseline column reproduces M6 (+1.59 ± 0.64) on this smaller sample. The paired difference
between the pilots is −0.71 ± 0.90: no change within the noise. Pooling both pilots, the bot's
decks win **+1.1 ± 0.8** points more played games.

## What M7 says

- **A stronger pilot doesn't move played results toward judge B.** If judge B's +3.85 were real
  and the baseline was just too weak to show it, a pilot that beats the baseline 72% of the time
  should have shown more of it. It shows the same or less.
- **So judge B probably overstates the bot's edge.** The bot's decks look slightly better in played
  games (+1 to +1.5 points, about one to two standard errors), well short of +3.85. The RL bot was
  trained against judge A and evaluated by judge B; they learned from different halves of the
  same kind of human game data, and the bot may have found deck traits both reward more than play
  does (the shared blind spot the README already names as a limit).
- **Caveat:** the rollout pilot is still the baseline underneath, plus search. It plays much better
  than the baseline but not like a strong human, and judge B learned from human games.
