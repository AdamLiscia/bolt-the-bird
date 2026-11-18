# P3 design: a player that learns every decision by self-play

P2 showed the limit of turn-level imitation: it copies people's restraint (holding instants and
tricks) without their instant-speed follow-through, and loses to the rules-based player 39–61
at best. P3 learns **every** decision the engine asks for (casting, attacking, blocking, targets,
yes/no choices, mulligans, instant-speed play) from wins and losses.

## Measured starting points (2026-11-18)

- A game is about **214 decisions**, 208 with more than one legal move: priority 165, declare
  attackers 25, choose target 7, declare blockers 6, yes/no 4, mulligan 2, others under 1.
- Rules-based self-play runs at **~290 games/s** in a single Python process; an observation from
  the bindings costs **~14 µs**. The network, not the engine, will set the speed.

## The network: score every legal move

One forward pass per decision:

1. **State tokens.** Every visible object: my hand; both battlefields (with tapped, power,
   toughness, damage, counters, summoning sickness, attached-to); both graveyards; the stack.
   Each token is a card embedding + zone/owner embedding + those numbers. One more token for the
   game: turn, step, whose turn, life totals, hand and library sizes, the pending decision's kind,
   lands played. The opponent's hand is a count only: the network sees what a player would see.
2. **Encoder.** A transformer over the tokens (like the turn model, larger).
3. **Action tokens.** Each legal move becomes a token: its kind (PlayLand, Cast, Attack, Block,
   Target, PassPriority, Yes/No, ...) + pointers to the objects it names (the card cast, the
   attacker, the blocker and its attacker, the target), read from the encoder's output for those
   objects. Moves that differ only by an amount (X, a mode) get that as a number.
4. **Policy head.** A score per action token, softmax over the legal moves. **Value head** on the
   game token: the chance of winning from here.

This one network replaces the rules-based fallbacks P2 needed.

## Training, in three stages

**A. Distillation (behavior cloning) from the smart player.** Generate a few million decisions from
the smart 1 × 2 player in self-play (it beats the rules-based player 65%) and train the network to
copy them. Cheap, supervised, and it gives PPO a competent start instead of random play, the same
"imitate first" step that made the drafter work. Done when it plays about as well as the smart 1 × 2
player.

**B. PPO self-play.** Many games at once; each decision samples from the policy; the only reward is
+1 for a win, −1 for a loss, 0 for a draw, at the end. Generalized advantage estimation with the
value head; clipped PPO updates, as for the drafter.
- **Opponents: a league, not just itself.** The current network, frozen past checkpoints, the
  rules-based player and the smart player, so it can't win by beating only its own habits.
- **Decks:** the 512 human-model decks and the bots' decks, sides swapped.
- **Leash:** a KL penalty toward the distilled policy, decaying over training (PPO from scratch
  over-optimized in the drafting work; this keeps early updates from drifting into nonsense).

**C. Imitation as a prior (optional).** The P1 models' turn-level predictions as an auxiliary signal
or a feature, if the distilled player plays unlike people.

## Evaluation

Every N updates, against fixed opponents on held-out deck pairs with sides swapped: the rules-based
player, smart 1 × 2 and smart 4 × 6. Every checkpoint is kept and the best is chosen by evaluation
(early stopping). The goal: beat smart 4 × 6. Then P4: re-grade the drafting bots with it.

## How the games run

- **Worker processes** each run a batch of games with a CPU copy of the current network, batching
  the pending decisions of all their games into one forward pass, and send back trajectories.
- **A learner** collects them, runs the PPO update, and sends new weights out.
- Workers are restartable (the engine's rare memory corruption, bolt.engine_worker): a dead worker
  loses its in-flight games, not the run.
- If Python becomes the bottleneck, the observation and action encoding moves into the Rust
  bindings (returning arrays instead of dicts).

## Milestones

| | What | Done when |
|---|---|---|
| P3a | Observation and action encoding + the network; tests (masking, pointers point at the right objects, order of objects doesn't matter) | A forward pass on real engine positions, batched |
| P3b | Batched game runner (worker processes, one network call per batch of decisions) | Thousands of decisions a second on the laptop; legal moves only |
| P3c | Distillation from the smart player | Plays at about the smart 1 × 2 level |
| P3d | PPO with a league, small scale on the laptop | Win rate against the fixed opponents rises |
| P3e | Scale-up in the cloud | Beats smart 4 × 6 on held-out decks |

## Compute and cost

P3a–P3d run on the laptop. At an estimated 25–100 games/s per machine with the network in the loop,
a laptop does about 100,000–300,000 self-play games overnight: enough to see learning, not enough
to finish. P3e moves to the cloud: many CPU cores for games plus a GPU for the learner, at an
estimated $50–300 for a serious run. That needs an AWS account (Adam creates it and adds the
payment method), a budget alarm, and a script that always shuts the machines down.

## Safeguards (from the drafting work)

Reward only wins. Evaluate against players it never trained against. Keep every checkpoint.
Watch for over-optimization and stop at the peak. Guard every run (time and memory). Restartable
engine workers.
