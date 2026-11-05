# Engine log

One line per working step, newest last. The next session continues from the last line.

- 2026-11-04: Card audit done (`CARD_AUDIT.md`). Plan written (`ENGINE_PLAN.md`). Rust 1.92 installed
  via Homebrew rustup (binary at `/opt/homebrew/opt/rustup/bin`). **Next: M0, the Cargo workspace and CI.**
- 2026-11-05: **M0 done.** Cargo workspace `engine/` (`bolt-core`, `bolt-cards`), Rust pinned to 1.92,
  `forbid(unsafe_code)`, CI job running fmt, clippy `-D warnings` and tests, `make engine-test`. 3 tests.
  `bolt-py` deferred to M5 (QUESTIONS). **Next: M1, starting with game state, zones and the turn structure.**
- 2026-11-05: **M1 rules core.** `bolt-core`: game state, London mulligan, play/draw, the full turn
  structure, priority and the stack, one land per turn, automatic mana payment, casting creatures
  (flash), combat with flying, reach, first strike, double strike, deathtouch, trample, lifelink,
  vigilance, haste, menace, defender and indestructible, state-based actions (life, decking, lethal
  damage, deathtouch, toughness, legend rule), discard to seven. 43 tests, each rule citing the CR.
  Fuzzer: 10,000 random-agent games, no panic, every game ends, invariants checked after every action
  (6,261 games/s with checks, `cargo run --release --example random_games`). Simplifications are in
  QUESTIONS. **Next: M1 card data, loading real FDN creatures into `bolt-cards`.**
- 2026-11-05: **M1 done.** `bolt-cards` loads all 286 FDN cards from `assets/FDN_oracle.json` in model
  order (a card's index is its draft-model index). `supported()` derives support from rules text and is
  pinned by a test: the 5 basics and the 8 keyword-only creatures (tier 1). Per-card tests, real-card
  combat tests (Nighthawk vs. Serra Angel, Swiftblade Vindicator trampling over on the second strike,
  Brazen Scourge's haste), and 300 fuzz games with real supported cards. 56 tests in the workspace.
  **Next: M2, starting with targets (with a zone-change counter, see QUESTIONS) and the effect DSL.**
- 2026-11-05: **M2 mechanisms.** Stack entries for spells and triggered abilities, target choice as
  a decision (`Pending::ChooseTarget`), target filters (any target, creature/player, controller,
  flying, power, in combat, hexproof), zone-change counters so old targets fizzle (CR 400.7, 608.2b),
  enters/dies triggers in APNAP order (removed when they have no legal target), kicker, instant and
  sorcery timing, until-end-of-turn pumps and keyword grants, +1/+1 counters, and the effect DSL
  (damage, destroy, draw, life, pump, grant, counters). 11 new stack tests, 67 total.
  **Next: M2 cards, writing FDN tier-2 scripts in build order, each with a test from its text.**
- 2026-11-05: **M2 cards, batch 1.** Core: tokens (cease to exist off the battlefield, CR 704.5d),
  bite damage, graveyard exile, intervening-if conditions (raid, "another Elf"). `bolt-cards`: card
  scripts that name the exact rules lines they implement, checked against Scryfall by a test; token
  definitions after the 286 cards. 22 tier-2 cards scripted, each with a test from its text. 35 cards
  supported. 1,000 fuzz games with real cards. 91 tests.
  **Next: M2 cards, batch 2. Choices during resolution (discard, "you may", surveil, search), then
  counterspells, fight, additional costs and cost reductions.**
- 2026-11-05: **M2 cards, batch 2 (scripts).** Core: resolution can pause for a decision and resume
  (`Pending::Decide`): "you may", discard, choose-a-card discard, surveil, library search (to hand,
  battlefield or top), top-or-bottom. Also counterspells (spells as targets), bounce, fight, prowess
  (cast triggers), and tokens for any player. 22 more cards scripted (57 supported); the drift test
  confirms every script matches its card text, and 1,000 fuzz games pass.
  **Next: a rules test for each batch-2 card, then the rest of tier 2 (additional costs, cost
  reductions, "up to" targets, flashback, Food and Treasure).**
- 2026-11-05: **M2 batch-2 tests.** A rules test for each of the 22 batch-2 cards (decisions,
  counterspells, fight, prowess, tutors, kicker search). 112 tests in the workspace.
  **Next: the rest of tier 2: additional costs (sacrifice, discard), cost reductions, "up to" targets,
  flashback, reflexive triggers, Food and Treasure tokens, and Banishing Light's linked exile.**
- 2026-11-05: **Activated abilities.** `Action::Activate`, with mana, {T} and sacrifice-this costs (CR
  602); {T} needs no summoning sickness (CR 302.6). Treasure is a mana source that is sacrificed when
  used, and the automatic payer spends it last. Food and Treasure tokens. 3 more cards (Bake into a Pie,
  Gleaming Barrier, An Offer You Can't Refuse): 60 supported, 115 tests.
  **Next: additional costs (sacrifice, discard), cost reductions, flashback, "up to" targets.**
- 2026-11-05: **Costs.** Cast modes: additional costs as part of the cast action (`CastWith`:
  sacrifice a creature, discard a card, or "sacrifice or pay"), cost reductions (if you control a
  subtype; if it targets a tapped creature, with targets limited to affordable ones), flashback
  (`CastFlashback`, exiled when it leaves the stack), and exile. 6 cards (Arbiter of Woe, Eaten Alive,
  Thrill of Possibility, Arcane Epiphany, Luminous Rebuke, Think Twice): 66 supported, 121 tests.
  **Next: the last 6 tier-2 cards: "up to" and graveyard targets (Felidar Savior, Macabre Waltz,
  Soul-Shackled Zombie), reflexive triggers and tapping (Faebloom Trick), linked exile (Banishing
  Light), and a granted dies trigger (Fake Your Own Death).**
- 2026-11-05: **M2 done.** "Up to N" targets (optional slots, no repeats, "other", a single
  graveyard), cards in graveyards as targets, reflexive triggers ("when you do"), tapping, exile until
  the source leaves (CR 610.3), dies triggers granted until end of turn, and any nonland permanent is
  castable. The last 6 tier-2 cards (Felidar Savior, Macabre Waltz, Soul-Shackled Zombie, Faebloom
  Trick, Banishing Light, Fake Your Own Death). **All 72 tier 0-2 cards are supported**, each with a
  rules test. 127 tests. 10,000 release-mode fuzz games with all 72 cards: no invariant broken, 2,007
  games/s with checks (`cargo run --release -p bolt-cards --example random_fdn_games`).
  **Next: M3, tier 3 in build order (`CARD_AUDIT.md`), starting with the mechanisms most cards share:
  static P/T effects (anthems), auras, equipment, and attack/block triggers.**
- 2026-11-05: **M3 batch 1.** Core: lands that enter tapped, enters-with-counters replacements
  (kicked, raid), triggers for another creature entering, attacking, first life gain each turn and the
  second card drawn; static abilities (layer 6/7 keywords and P/T: this, your creatures, your creatures
  with counters, "during your turn"); sorcery-speed abilities; "sacrifice another creature" costs;
  "can't block" and "can't be blocked"; graveyard targets by mana value. 32 cards, including all ten
  gain lands, Evolving Wilds, Rogue's Passage and Llanowar Elves: **104 supported, 152 tests.**
  **The fuzzer found two real bugs**, both fixed with regression tests: sacrificing a creature that
  had paid mana for the same cost, and activated abilities paying costs before choosing targets (now
  CR 602.2b order; QUESTIONS updated). 10,000 release fuzz games with all 104 cards pass.
  `examples/replay.rs` replays any fuzz seed. **Next: M3 batch 2: modal spells (Bushwhack, Goblin
  Surprise), equipment and auras, then the next cards in build order.**
- 2026-11-05: **M3 batch 2.** Modal spells (`CastModal`, mode chosen on casting, CR 700.2),
  Equipment and Auras (attachment pinned to the creature's zone-change count; equip at sorcery speed;
  Auras cast with a target and enter attached; CR 704.5m-n unattach and Aura-to-graveyard), static
  abilities on the attached creature, landfall, "deals combat damage to a player" triggers (the creature
  or its Equipment), fights between two targets, each-player damage, pump for any player's creatures.
  12 cards (Abrade, Bushwhack, Goblin Surprise, Seeker's Folly, Slagstorm, Run Away Together, Goldvein
  Pick, Quick-Draw Katana, Swiftfoot Boots, Adventuring Gear, Celestial Armor, Twinblade Blessing):
  **116 supported, 165 tests.** 10,000 release fuzz games pass. **Next: M3 batch 3 in build order.**
- 2026-11-05: **M3 batch 3.** Triggers: every life gain, "whenever you attack", another creature of a
  subtype entering, this or another creature dying, beginning of combat, end step, casting on an
  opponent's turn. Morbid and "another creature with power 4+" conditions; threshold statics; mill;
  keyword grants to all your creatures; "doesn't untap"; instant/sorcery discounts; "other creatures
  with flying" statics. 25 cards: **141 supported, 187 tests** (counted from verbose `cargo test`; the
  totals logged for the last two batches came from a quick count that undercounts, so they were low).
  10,000 release fuzz games pass. **Next: M3 batch 4 in build order (ward, "look at the top N",
  activated abilities from the graveyard, ETB modal triggers).**
- 2026-11-05: **M3 batch 4.** Ward (a trigger on being targeted by an opponent; the spell or ability
  is countered unless its controller pays, CR 702.21), modal triggered abilities (mode chosen as the
  trigger goes on the stack, CR 700.2b), upkeep triggers, draw triggers (yours and opponents'),
  instant/sorcery and "noncreature or Dragon" cast triggers, destroy all creatures, "can't block this
  turn", last known power for sacrificed sources (CR 608.2h), and "that creature" references that
  aren't targets. 20 cards (including Day of Judgment, Phyrexian Arena, Archmage of Runes, Cackling
  Prowler, Apothecary Stomper, Wardens of the Cycle): **161 supported, 207 tests** (per-binary count).
  10,000 release fuzz games pass. **Next: M3 batch 5: "look at the top N", optional costs in triggers,
  graveyard-activated abilities, and the rest of tier 3.**
- 2026-11-05: **M3 batch 5.** Unconditional enters-with counters, threshold trigger conditions,
  "as long as it's untapped" statics, Condemn, bounce all attackers, mass reanimation, counter and
  power doubling, "attacks each combat if able" (CR 508.1d), "can't be blocked by Walls", opponents'
  creatures entering tapped, and "you can't lose the game". 18 cards (Anthem of Champions, Juggernaut,
  Paradise Druid, Mossborn Hydra, Herald of Eternal Dawn, Authority of the Consuls, ...): **179
  supported, 222 tests** (counted from saved `cargo test` output; piping it live dropped binaries).
  10,000 release fuzz games pass. **Next: the remaining ~60 tier-3 cards: library looks, optional
  trigger costs, graveyard abilities, alternate ward costs.**
