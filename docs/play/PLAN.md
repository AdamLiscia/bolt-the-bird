# Phase 4: a player that plays Magic well

Every result so far runs through the engine's own pilots: the rules-based baseline and the smart
(rollout) player, which looks one move ahead and assumes the opponent never responds. Phase 4 builds
a learned player, first from how people play (17lands replays), then by self-play with PPO.

Why it matters: every drafting grade runs through a pilot, so a stronger pilot makes them more
trustworthy; it is what "play against the bot" needs; and it is the hard part of game AI.

## What the replay data can teach

`replay_data_public.FDN.PremierDraft.csv.gz` (17lands): one row per game, from the 17lands user's
side. Per turn, for both players: cards drawn, lands played, creatures and other spells cast,
abilities used, creatures that attacked / were blocked / blocked, creatures killed, mana spent,
and an end-of-turn snapshot (the user's hand as cards, the opponent's hand as a count, both
boards, both life totals). Not recorded: targets, the order of actions within a turn, which
blocker blocked which attacker, and the opponent's hand.

So imitation works at **turn level**: given the position at the start of a turn (hand, lands,
boards, life, turn number, on the play or not), which cards did the person cast, and which
creatures attacked or blocked? That covers the decisions that matter most in Limited (curve,
tempo, when to attack), and leaves targets and instant-speed play to search and self-play.

## Milestones

| | What | Done when |
|---|---|---|
| P0 | **Replay data.** Parse rows into per-turn decision records from the user's side; map 17lands card ids to engine cards; coverage and sanity statistics | Records for every game; card coverage measured; tests on the parser |
| P1 | **Turn-level imitation models** (PyTorch): what to cast from hand, which creatures attack, which block; held-out games | Beats simple baselines (cast the most expensive castable card; attack with everything that survives) on held-out turns |
| P2 | **The imitation player in the engine**: engine positions mapped to the same features; the models choose casts, attacks and blocks, the existing pilots handle targets and the rest | Wins more than the rules-based pilot, measured on fixed deck pairs with sides swapped |
| P3 | **Self-play with PPO**: a policy and value network over engine positions and legal actions, started from P1/P2, reward only for winning, KL leash to the imitation player, every checkpoint kept | Beats the smart 4 × 6 player; a held-out deck set confirms it |
| P4 | **Close the loop**: re-grade the drafting bots with the strong player | The tournament rerun with the new pilot |

## Lessons carried over from the drafting work

- **Reward only what you want.** Win or loss, nothing in between (the judge, and a Slay the Spire
  agent that learned to heal for the reward, both show what shaping does).
- **Measure against a player it never trained against**, on fields it never saw.
- **PPO over-optimizes**: keep every checkpoint, evaluate the best, use a leash.
- **Engine crashes are rare and survivable**: engine work runs in the restartable worker.

## Compute

P0–P2 run on the laptop. P3 is the heavy part: every decision in every self-play game calls the
network, so the games are batched across many environments and the big runs move to the cloud
once the pipeline works at small scale (estimated $50–300 per serious run).

## Protocol

As for the engine (docs/engine/ENGINE_PLAN.md): tests first, small commits, a line in
docs/play/LOG.md per step, every long run through `scripts/guard.sh`.
