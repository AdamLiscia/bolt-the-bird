# Open questions for Adam

Judgment calls made during autonomous sessions. Each one has the choice taken and why, so it can be
reversed later.

## 2026-11-05: `bolt-py` waits until M5

**Choice:** the M0 workspace has `bolt-core` and `bolt-cards` only. The PyO3 crate is added at M5,
when there is an API to bind.
**Why:** a PyO3 extension crate needs a Python toolchain to build and link its tests, which slows CI
and adds a failure mode before there's anything to expose. Nothing else depends on it until M5.
**To reverse:** add `engine/bolt-py` with `pyo3 = { features = ["extension-module", "abi3-py311"] }`.

## 2026-11-05: Rust pinned to 1.92 at the repo root

**Choice:** `rust-toolchain.toml` pins 1.92 with clippy and rustfmt. CI uses the same version.
**Why:** a new clippy release can add lints, and with `-D warnings` that breaks CI with no code change.
Upgrades become one deliberate commit.

## 2026-11-05: Mana is paid automatically

**Choice:** `Cast` picks which lands to tap. Colored pips are matched first by backtracking, narrowest
sources first, so dual lands stay untapped as long as possible. Tapping lands is not an action.
**Why:** tapping for mana one land at a time multiplies the action space for no strategic gain in
almost every Limited game. Forge and most game-AI engines pay automatically.
**To reverse:** add `Action::TapForMana(ObjId)` and a mana pool. Revisit for cards whose mana choice
matters (mana abilities with costs, "spend only on" restrictions).

## 2026-11-05: Combat damage is assigned automatically

**Choice:** an attacker blocked by several creatures assigns lethal damage to each blocker in
declaration order, and the rest goes to the last blocker, or with trample to the player. Deathtouch
makes 1 damage lethal.
**Why:** Foundations removed damage assignment order, so the attacking player divides damage freely.
That's a large action space that rarely matters. The automatic split is what a player does almost
every time.
**To reverse:** add a `Pending::AssignDamage` decision with one action per split.

## 2026-11-05: Legend rule keeps the newest

**Choice:** with two legendary permanents of the same name, the engine keeps the one that entered
last. The player would choose (CR 704.5j).
**Why:** it's the usual choice, and it removes a rare decision. It's a one-line change to a
`Pending::ChooseLegend`.

## 2026-11-05: Priority passes itself when passing is the only legal action

**Choice:** `Game::auto_pass` is on by default. The game moves on when the deciding player has
nothing to do but pass.
**Why:** most priority stops have only one option, and agents would spend most of their actions
passing. No information is hidden, because there was no choice. Tests and viewers turn it off.

## 2026-11-05: Mulligans are decided one player at a time

**Choice:** the player who goes first finishes their mulligans, then the other player decides.
CR 103.5 has both players decide each round together.
**Why:** it's simpler and only reveals hand size, which is public anyway. Easy to change if a viewer
needs the exact order.

## 2026-11-05: Objects keep their ID across zone changes

**Choice:** a card has one `ObjId` from library to graveyard. CR 400.7 says it becomes a new object
each time it changes zones.
**Why:** nothing in M1 can tell the difference. M2 brings targets and "that creature" references, and
there zone changes must break them. Planned for M2: a per-object zone-change counter that targets
check.

## 2026-11-05: ~~Activated-ability costs are paid before targets are chosen~~ (reversed)

**Was:** costs first, then targets. **Now:** targets first, then costs, as CR 602.2b says. The fuzzer
found a real difference: Cathar Commando's only legal target was the Treasure paying for it, and
paying first left no target to choose. Covered by a regression test.

## 2026-11-05: "Any number of targets" is capped at eight

**Choice:** Divine Resilience's kicked mode ("any number of target creatures you control") offers up
to eight targets.
**Why:** the action space needs a fixed number of target slots. A Limited board almost never has more
than eight creatures on one side. Raise the cap if a game ever needs it.

## 2026-11-05: Types granted by static abilities don't feed other static abilities

**Choice:** a creature type given by a static ability (Skyknight Squire becoming a Knight) counts
everywhere except when working out other static abilities ("other Cats you control get +1/+1").
**Why:** it avoids static abilities depending on each other (CR 613 layer dependencies). No FDN card
grants a type that another FDN static ability then looks for.

## 2026-11-05: Leylines always begin the game on the battlefield

**Choice:** a Leyline Axe in an opening hand always starts on the battlefield ("you may" is taken).
**Why:** it's free and never worse than casting it for {4}. A pre-game decision would add a step to
every game for a choice nobody declines.

## 2026-11-05: Five tier-3 cards wait for a type layer

**Choice:** Imprisoned in the Moon, Soulstone Sanctuary, Secluded Courtyard, Kellan and Muldrotha
are deferred, and tier 4 starts first.
**Why:** the first two change card types on the battlefield (a creature becomes a land, a land
becomes a creature), which means reading types through a layer system instead of the printed card
in about 30 places. Courtyard needs mana that can only be spent on some spells. These are worth
doing together, once, rather than piecemeal for two cards.
**Resolved 2026-11-06:** all five are in, on `types_of`/`colors_of`, stages and `PayFor`.

## 2026-11-05: The fuzzer stops games at 1,000 permanents

**Choice:** a random game whose battlefield passes 1,000 permanents ends as a draw and is counted
separately. The engine itself has no limit.
**Why:** random players activate Krenko, Mob Boss every turn, and its Goblins double each time. At
5,000+ Goblins each trigger costs a pass over the board, so a turn costs O(board²), and the next
doubling never ends in practice. 4 of 10,000 fuzz games hit the cap.

## 2026-11-05: Mana in the pool is spent before lands

**Choice:** when paying a cost, auto-payment uses mana already in the pool (Chandra's {R}{R}{R})
before tapping any permanent.
**Why:** pool mana empties at the end of the step, so spending it first never costs the player
anything a land could have done. It keeps payment automatic (see the first entry).

## 2026-11-06: Which creature types Banner of Kinship offers

**Choice:** the type list is every creature type among creatures you control and creature cards you
own (hand, library, graveyard, exile), sorted, capped at 64.
**Why:** any creature type is legal (CR 205.3m), but a type that's on no card you could ever control
does nothing. You know your own deck, so offering its types hides nothing. If you own no creature
cards the choice is skipped, since every type would do nothing.

## 2026-11-06: Which kind of counter Quilled Greatwurm's cost removes

**Choice:** you pick the creature for each of the six counters. From that creature the engine
removes a stun counter first, then a +1/+1 counter, then a revival counter.
**Why:** losing a stun counter helps you and losing a revival counter hurts most, so this order is
what a player would choose. Picking the counter kind as well would add a second choice per counter
for almost no gain.

## 2026-11-06: Spell copies keep the original's targets

**Choice:** Thousand-Year Storm's copies always have the same targets as the original. The engine
doesn't offer "you may choose new targets for the copies".
**Why:** keeping the targets is always a legal choice. Offering new ones would mean choosing
targets in the middle of a resolving ability, once per copy, which the casting pipeline doesn't
support yet. Copies also can't be targeted by counterspells (spells are targeted by their card,
which a copy doesn't have). Both are worth revisiting if Storm turns out to matter in games.

## 2026-11-06: Muldrotha's graveyard casts are plain casts

**Choice:** a spell cast from the graveyard with Muldrotha is cast without kicker, and with X = 0.
**Why:** both are legal ways to cast it, and each extra option is another action per graveyard
card per type. Few FDN permanents have kicker or X. Add them if games show they matter.

## 2026-11-06: Played games stop at 200 turns

**Choice:** `bolt_engine.play_baseline` stops a game after turn 200 and reports "timeout"; the
validation counts it as a draw and reports how many there were.
**Why:** some boards can't end under the rules. With a Herald of Eternal Dawn on each side ("you
can't lose the game") a validation game reached turn 1,773 with both players at 19 life. In real
play someone concedes or the round's time runs out. Limited games end long before turn 200, so the
cap only catches games like that one. The engine itself has no cap; this is the harness.

## 2026-11-06: A turn that resolves 10,000 spells and abilities is a draw

**Choice:** if one turn resolves more than 10,000 stack entries, the game ends in a draw.
**Why:** CR 104.4b makes an endless loop of mandatory actions a draw, but proving a loop can't be
stopped is hard in general. The validation found one: Bloodthirsty Conqueror and Marauding
Blight-Priest trigger each other forever when the opponent can't lose (Herald of Eternal Dawn).
Real turns resolve a few dozen entries, so the threshold only catches loops like that one.
Without Herald the same combo is a win, not a draw: the opponent's life hits 0 and state-based
actions end the game (test `conqueror_and_blight_priest_kill_an_opponent_who_can_lose`).

## Rollout pilot: what it searches and what it guesses (2026-11-06)

- **Searches only priority, attack and block decisions.** Mulligans, targets, discards and choices in
  the middle of a resolving spell go to the baseline. Determinizing mid-resolution could redeal
  cards a decision is looking at, and those decisions are a small share of the game.
- **Revealed cards are guessed again.** The opponent's hand is redealt from their hand and library
  together, even if a card was revealed or bounced there earlier. Tracking what each player has seen
  would make it stronger; it isn't needed to beat the baseline. Cards the stack or an exile link
  still names keep their identity.

## 2026-11-08 — An index panic that doesn't reproduce

During the strength-ladder run (rollout 1 × 2, field A) one game panicked at `game.rs:863`
(`Game::obj`): "index out of bounds: the len is 80 but the index is 1457127744". It was job 63372
(decks 454 vs 657, seed 828471360064592330, player 1 first): the only job that chunk started and
never finished. Replayed 400 times, 8 at a time, with the same binary, it played the identical
game every time (p0 wins on turn 16) and never panicked. The engine has no `unsafe`, no iterated
hash maps, and no clock in its rules, and an ObjId is only ever made from `objects.len()`, so a
value like 1,457,127,744 shouldn't be reachable. Unexplained after two attempts, so per rule 6 it
is written up, not chased: `pilot_games` now records a panicking game as `error` (logged with job
id and seed) and keeps going, and `analyze` drops that pair. If it happens again, the seeds from
the log are the starting point; a second case that also replays cleanly points at the machine
(a memory error) rather than the engine.

## 2026-11-08 — A rollout decision that never finishes (reproducible)

Field B, rollout 2 × 4, job 36149 (`36149 666 862 1665606242745551370 1` in data/m7b/jobs.txt).
Alone, it runs past 300 s without finishing, every time; memory stays low. The per-game 180 s cap
never fires because the time goes into a single `rollout::choose` call. `sample` puts nearly all
of it under `choose → apply → settle → legal_actions → spell_cost` (plus Vec allocation): listing
legal moves inside a playout, over and over. Guess, not checked: a playout's board grows very large
(a token loop?) so each action gets slower, within the 5,000-action playout cap. Stopped here per
rule 6. Next: rerun only this seed; log the playout's object count and actions per second, then fix
the cause (and give `choose` a time or work budget so one decision can't stall a run).

**Resolved 2026-11-08 (same night).** Not a loop and not a rules bug. In some of the pilot's guessed
worlds deck 666's Homunculus Horde (a token copy of itself whenever you draw your second card each
turn) meets Phyrexian Arena's extra draw: every Horde copies itself every turn, so the board doubles
(2,128 objects by turn 34, 4,178 by turn 38 in one world) and each action slows with it. That is
correct Magic; the baseline pilots just don't close the game. Fix: `rollout::Config::max_objects`
(2,000): a playout whose game has made more objects than that counts as a draw, like one past
`max_actions`. Test: `a_playout_past_the_object_limit_counts_as_a_draw` (fails without the limit).
Job 36149 now finishes in 32 s. `examples/replay_job.rs` is the tool that found it.
