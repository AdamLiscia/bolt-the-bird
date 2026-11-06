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
