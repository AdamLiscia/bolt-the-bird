# Lane brief: scripting cards in Bolt's rules engine

For a coding lane that starts cold: the desktop's lanes (`lane-d1`, `lane-d2`, ...), or any helper. One
person merges every lane, the Mac session, on branch `tarkir`. **A lane never pushes to `tarkir`**: it
pushes only its own branch, and reports.

## Your assignment

A handoff file in `handoff/lanes/<lane>.json` on `tarkir`:

```json
{"lane": "d1", "branch": "lane-d1", "module": "pg_d1", "deck": "Ygra, Eater of All (Grande)",
 "cards": ["Abrupt Decay", "..."], "notes": "..."}
```

Script the `cards` in order, in your module (`engine/bolt-cards/src/scripts/<module>.rs`), with tests in
`engine/bolt-cards/tests/<module>.rs`. Both files already exist and are wired in; you only add to them.

## The standing queue: keep going without waiting

`handoff/lanes/queue.json` lists every remaining playgroup deck through round 5, **each owned by one
lane** (`"lane": "d1"`, ...). Ownership is fixed in the file, so two lanes can never pick the same deck:

1. Finish your first assignment (`handoff/lanes/<lane>.json`).
2. Then take **your own** decks from `queue.json`, top to bottom. Don't take another lane's deck.
3. Skip any card `set_support` already shows as `yes`. Another lane may have done it, since decks share
   cards; each card is listed under only its first deck.
4. When a deck's cards all play (or are only "needs core" skips), report "<Deck> (<owner>): scripted,
   <n> needs core" and start your next deck.
5. When your decks run out, report "lane done". The lead may move decks between lanes in `queue.json`
   (pull `origin/tarkir` before starting each deck).

**Cards that need engine core** go to the lead in your report, never into `queue.json`. The lead adds them
to `handoff/lanes/needs_core.json`, which the Mac lanes work through.

**Stop:** at the end of round 5 (all of your decks done), and report. **Pause:** 05:00–10:00 ET (the daily
jobs), during Adam's streams, and on the desktop while Adam is gaming, when he or the orchestrator says so.
Finish or commit the current card first.

## Setting up

```bash
git fetch origin && git worktree add ../bolt-<lane> -b <branch> origin/tarkir
cd ../bolt-<lane>
python3 -m venv .venv && .venv/bin/pip install -e ".[dev,practice]"
source ~/.cargo/env
export CARGO_TARGET_DIR=$HOME/bolt-target-<lane>   # one per lane: never share a target dir
```

Each lane gets its **own** target directory. Two worktrees sharing one hand each other stale builds (both
produce the same artifact names), and a run can then test the other lane's code.

## What a card script is

Scripts are keyed by **exact card name**. A card is "supported" (playable) only when every rules line of its
text is claimed by its script and its cost parses. The card text lives in `assets/CMD_oracle.json` (the
Commander pool, which is every card the engine knows), built from `assets/PG_oracle.json` for the playgroup decks.

```rust
"Sylvan Library" => s(
    &[
        "At the beginning of your draw step, you may draw two additional cards. If you do, choose two cards in your hand drawn this turn. For each of those cards, pay 4 life or put the card on top of your library.",
    ],
    |d, _| { /* build the CardDef: triggers, effects, statics */ },
),
```

- **The lines** are the card's rules text, one entry per line, **exactly** as in the oracle text, with
  reminder text (anything in parentheses) removed. Lines that are only keywords the engine implements
  (`Flying, trample`) are claimed automatically and don't go in the list.
- An Omen's spell half is written `"Omen: <line>"`; a double-faced card's back face is `"Back: <line>"`.
- **The build closure** fills in the `CardDef` with the helpers in `scripts.rs`: `spell`, `on` (a trigger),
  `may`, `on_if` and `conditional`, `activated`, `loyalty`, `static_on`, `lord`, `equip`, `attached`,
  `chapter`, `modes`, `search`, `draw`, `damage`, `up_to`, and more. Read the existing modules first
  (`scripts.rs`, `blc_b.rs`, `blc_c.rs`, `pg_b.rs`, `pg_c.rs`): most effects you need already exist, and reusing
  them is the whole job.
- Add every card you finish to your module's `PG_<LANE>_SCRIPTED` list. The lib test
  `every_playgroup_script_matches_its_card_text` fails if a listed card's lines don't match its text.

Check one card's support:

```bash
cd engine && cargo run -q --release -p bolt-cards --example set_support -- CMD | grep "Card Name"
```

## The engine core: don't change it

**Desktop lanes don't edit `bolt-core`** (`card.rs`, `effect.rs`, `game.rs`) or other lanes' modules. Core
changes go through the Mac's lanes, because two branches editing the same enums conflict at every merge.
If a card needs something the engine doesn't have (a new effect, trigger, decision or keyword), **skip it**
and list it in your report as "needs core: <what it needs>". The Mac adds the core piece, and a later
assignment gives you the card.

Never add an `Action` variant anywhere: the play model's action vocabulary depends on it.

**Tokens are shared too:** the token list in `bolt-cards/src/scripts.rs` is numbered, and two lanes appending at
once collide. Don't add tokens. Use an existing one (`t.<name>` in the `Tokens` struct), or report the card as
"needs core: a <P/T> <color> <type> token".

## Tests

One behavior test per card in your test file. It checks what the text does, not just that the card casts.
The test files have helpers for a Commander game on the CMD pool: `cid(name)`, `game(lands)`, `table()`
(5 of each basic land for each player), `put`, `hand`, `tokens`, plus `act`, `play`, `resolve_all` and
`obj` from `tests/common`.

```rust
#[test]
fn sylvan_library_draws_two_extra_and_pays_or_puts_back() {
    let mut g = table();
    put(&mut g, 0, "Sylvan Library");
    // ... set up, act, then assert the board, life, hand, zones ...
}
```

Pitfall: the draw step draws the top card of the library, so add library cards after the game reaches
the main phase, or put a spare card on top.

## Before every commit

From `engine/`, all with your own target dir and `nice -n 15 -j 4` (the desktop has 16 threads; with three
lanes, keep it to `-j 4` each).

**Each batch** (a desktop lane changes only its own module and test file, so this covers what it can break):

```bash
cargo test -q -p bolt-cards --lib                 # includes every_playgroup_script_matches_its_card_text
cargo test -q -p bolt-cards --test <module>       # your tests: the count must equal your #[test] count
cargo clippy -q -p bolt-cards --all-targets -- -D warnings
cargo fmt --check
```

**Once per finished deck**, before reporting it: the whole workspace.

```bash
cargo test -q
cargo clippy -q --all-targets -- -D warnings
cd .. && .venv/bin/ruff check -q . && .venv/bin/ruff format -q --check .
```

The lead's merge always runs the full suite and lint again, in its own target dir.

All clean, or it doesn't get committed. Then commit about every 8–10 cards:

```
<Lane>: <deck>, <n> cards (<names>)

<anything a reviewer should know: shortcuts, cards skipped and why>

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: <your session link>
```

Push your branch (`git push origin <branch>`), and report: commit hash, cards done, cards skipped with
"needs core: ...", test counts. Before each commit, `git merge origin/tarkir` so your branch stays
mergeable; if that conflicts in a file you don't own, stop and report instead of resolving it.

## Simplifications

Simplify only when the result plays the same 1v1 (for example, "each opponent" means the one opponent)
and say so in the commit message. Never claim a line the script doesn't actually do.
