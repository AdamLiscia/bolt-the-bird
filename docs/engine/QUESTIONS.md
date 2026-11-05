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
