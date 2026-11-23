# Star Trek Commander precons: exploration (2026-11-23)

Goal (Adam, 2026-11-22): simulate the four Star Trek Commander precons (set code trc) in Bolt's
engine, 1v1 first, and measure how they match up. Card lists: Nerd Leagues' read-only export
(`precons.json`, key `trc`); oracle text: Scryfall's API. Nothing here is built yet.

## The four decks

Each list in the export has 99 cards; the commander isn't flagged in it, so the decks below are matched
by their mechanics (worth fixing in the export: add the commander as the 100th card).

| Deck | Commander | Colors | Style (from its cards) |
|---|---|---|---|
| Federation Fleet | Jean-Luc Picard, Luminary (2/4, {1}{U}{R}{W}) | Jeskai (U R W) | "Seek out" (your second draw each turn), Spacecraft, card draw from historic permanents |
| Klingon Fury | Worf, Defiant Commander (3/4, {2}{R}{W}{B}) | Mardu (R W B) | "Battlefront" attack triggers, goad, menace, equipment, tokens |
| Landing Party | Commander Spock (3/5, {2}{G}{W}{U}) | Bant (G W U) | Landfall, "Strange new worlds" (free casts off the top), 32 lands |
| We Are the Borg | The Borg Queen (4/4, {4}{W}{U}{B}) | Esper (W U B) | Artifact creatures, "Assimilate" (steal creatures from the library) |

293 unique non-basic cards across the four (70 of them lands, 93 creatures); 5 are already in the FDN
engine (Abrade, Evolving Wilds, Rogue's Passage, Swiftfoot Boots, Thrill of Possibility).

## Mechanics: what the engine has and what it needs

Already in the engine (from FDN): flying, vigilance, menace, trample, haste, deathtouch, lifelink,
flash, equip, flashback, scry, landfall, treasure, affinity, "+1/+1 counters", tokens, auras,
"loses all abilities", cast-without-paying effects, 2-player combat.

New rules to build:

| Mechanic | Cards | What it is | Size |
|---|---|---|---|
| Commander, 1v1 | all | command zone, commander tax, 21 commander damage, 40 life, 100-card singleton, color identity | days |
| Station / Spacecraft | 18 + 18 Spacecraft | tap another creature: put charge counters equal to its power on this; it becomes an artifact creature at a threshold (6+ etc.); sorcery speed | new counter type + threshold rules |
| Planet | 4 | a new card type with its own rules | to read |
| Cycling | 14 | pay, discard, draw | small |
| Seek out (ability word) | 11 | "whenever you draw your second card each turn" | a trigger (the engine counts draws per turn already in places) |
| Battlefront | 7 | attack triggers that count opponents attacked; some force attacks ("attack a player other than you") | small in 1v1 |
| Goad | 4 | the creature attacks each combat and not you if able | small in 1v1 |
| Strange new worlds | 5 | reveal the top card, cast free if mana value ≤ differently named lands (Spock); put a creature or Spacecraft from hand with extra counters | medium |
| Assimilate | 4 | put a card onto the battlefield under your control as a Borg artifact creature with a +1/+1 counter | medium (type-changing) |
| Historic | several | artifacts, legendaries and Sagas count as historic | small |
| Multikicker, metalcraft, investigate (Clues), parley | 1–2 each | known older mechanics | small each |
| Three-color mana | all | 70 unique lands, many three-color or fetch-style | the land long tail |

Text that's written for multiplayer ("each opponent", "the number of opponents you attacked",
politics such as goad) works in 1v1 as "the opponent" or 1.

## Size and plan

- Card text: median 138 characters (FDN-like); 124 cards under 120 characters, 9 over 350.
- FDN's 286 mostly-simple cards took about a week of focused work, one test per card. These 293 have
  more new mechanics: roughly 2–3 weeks, plus the Commander rules.

1. **1v1 Commander rules** in the 2-player core, with tests (command zone, tax, commander damage,
   40 life, deck checks). Days.
2. **The 293 cards**, by difficulty: lands and vanilla-ish creatures first (one deck at a time, so each
   finished deck becomes playable), then the new mechanics. A test per card from its oracle text, as
   for FDN (`docs/engine/CARD_AUDIT.md` style audit first).
3. **Round-robin** of the 4 decks, every pairing, both seats, with the smart player (works unchanged
   in 2-player games): a measured matchup table.
4. **The play network** on these decks (copy the smart player, then self-play), and the table again.
