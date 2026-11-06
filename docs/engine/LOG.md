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
- 2026-11-05: **M3 batch 6.** Optional "sacrifice another creature" inside triggers, abilities
  activated from the graveyard, "look at the top N" with a pick and the rest to the graveyard or the
  bottom in random order, choosing a card from your graveyard, target limits by the source's power
  (last known, for dies triggers), subtype filters, "an opponent lost life this turn", and
  another-nontoken-creature-dies triggers. 11 cards (Vampire Gourmand, High-Society Hunter,
  Reassembling Skeleton, Gutless Plunderer, Squad Rallier, Inspiration from Beyond, Alesha, Fiendish
  Panda, Kiora, Cephalid Inkmage, Stromkirk Bloodthief): **190 supported, 233 tests.** Test totals
  are now counted with stdout and stderr captured separately (a combined capture was cutting the
  core crate's results off locally; CI's own log shows every test file running). 10,000 release fuzz
  games pass. **Next: the remaining ~50 tier-3 cards.**
- 2026-11-05: **M3 batch 7.** Stun counters (CR 122.1d), "you may pay {R}" inside triggers, ward paid
  in life, "can't be countered", affinity, "loses N life unless they discard", reveal until a creature,
  doubling counters on targets, a power gate inside an effect, any-permanent targets, "other [type]
  you control" statics, and triggers for nontoken creatures of a type entering (itself included),
  big creatures entering, attacking with N or more, opponents casting, and counters being put on a
  creature (once each turn). 16 cards (Koma, Drakuseth, Arahbo, Scavenging Ooze, Grappling Kraken,
  Sire of Seven Deaths, Exemplar of Light, Zimone, ...): **206 supported, 249 tests.** 10,000 release
  fuzz games pass. **Next: the last ~35 tier-3 cards.**
- 2026-11-05: **M3 batch 8.** Triggers that carry an amount ("that much", "that many"), all life loss
  through one function, "whenever an opponent loses life", noncombat damage to an opponent, "whenever
  you cast a spell", incubation counters, no maximum hand size; creature-type changes (gained types,
  "becomes a Werewolf", types from statics, last known types for "if it wasn't a Demon"), permanent
  keyword grants, "activate only once", kicked spells with different targets, extra land drops,
  creature cards put onto the battlefield from a library look, "sacrifice, discard or lose life",
  power and toughness from opponents' graveyards, mill until a land, life-total statics and hexproof
  from instants. 12 cards (Bloodthirsty Conqueror, Niv-Mizzet, Drake Hatcher, Infernal Vessel,
  Valkyrie's Call, Skyknight Squire, Mild-Mannered Librarian, Divine Resilience, Loot, Perforating
  Artist, Consuming Aberration, Elenda): **218 supported, 261 tests.** 10,000 release fuzz games pass.
  **Next: the last ~23 tier-3 cards (Witness Protection, Curator of Destinies, Kykar, Muldrotha, Time
  Stop, Heraldic Banner, ...).**
- 2026-11-05: **M3 batch 9a.** Alternative costs (CR 118.9), each player sacrificing creatures of
  their choice, leylines (CR 103.6a), "cast spells as though they had flash", triggers from the
  graveyard, flashback granted until end of turn for the card's mana cost, soul counters with last known
  count, "tap N untapped [type] you control" costs, instant/sorcery graveyard targets. 7 cards
  (Blasphemous Edict, Leyline Axe, High Fae Trickster, Flamewake Phoenix, Sphinx of Forgotten Lore,
  Ravenous Amulet, Lathril): **225 supported, 269 tests.** 10,000 release fuzz games pass.
  **Next: batch 9b (Kykar, Nine-Lives Familiar, Time Stop, Witness Protection, Heraldic Banner,
  Strongbox Raider, Curator of Destinies).**
- 2026-11-05: **M3 batch 9b.** Delayed "at the beginning of the next end step" returns (CR 603.7),
  revival counters, ending the turn (CR 724), ability loss with a base 1/1 (layers 6 and 7b; a running
  count of ability-loss Auras keeps the check free when there are none, and an invariant checks that
  count), card colors and "choose a color" with mana of the chosen color, playing a card from exile
  until the end of your next turn, and two piles chosen by the opponent. 7 cards (Kykar, Nine-Lives
  Familiar, Time Stop, Witness Protection, Heraldic Banner, Strongbox Raider, Curator of Destinies):
  **232 supported, 276 tests.** 10,000 release fuzz games pass, 920 games/s with checks. **Next: the
  last 9 tier-3 cards (Imprisoned in the Moon, Soulstone Sanctuary, Kellan, Tinybones, Muldrotha,
  Fishing Pole, Secluded Courtyard, Akroma's Memorial, Sphinx's Tutelage).**
- 2026-11-05: **M3 batch 10a.** Protection from colors (CR 702.16: no targeting, blocking or damage
  from sources of that color), repeated mill on shared colors, bait counters with an "equipped creature
  becomes untapped" trigger and a "tap the equipped creature" cost, opponent-discard triggers, and
  stashed cards playable from exile with any mana. 4 cards (Akroma's Memorial, Sphinx's Tutelage,
  Fishing Pole, Tinybones): **236 supported, 282 tests.**
  **Two bugs caught before commit.** (1) An editing slip glued the untap-step code onto a comment, so
  nothing untapped; the Fishing Pole test caught it, the fix is in, and a new core test
  (`permanents_untap_in_the_untap_step`) guards it. The editing tool that caused it is fixed too. (2)
  The fuzzer found activated abilities and modal triggers using a shallow target check that ignored
  protection (seed 6555); all target checks now use the full rules, with a regression test.
  10,000 release fuzz games pass. **Next: the last 5 tier-3 cards need card types that change on the
  battlefield (Imprisoned in the Moon, Soulstone Sanctuary), restricted mana (Secluded Courtyard),
  Kellan and Muldrotha. Then tier 4.**
- 2026-11-05: **Tier 4, batch 1.** Damage prevention until end of turn, noncombat-damage prevention,
  player hexproof, multi-mana sources (Elvish Archdruid), scaled statics ("for each Forest"), cost
  reductions (per instant/sorcery in graveyard, total power, attacking creatures), extra turns, and
  counting effects. 17 cards (Fleeting Flight, Tragic Banshee, Tolarian Terror, Dwynen, Krenko,
  Revenge of the Rats, Elvish Archdruid, Lunar Insight, Inspiring Call, Midnight Snack, Crystal
  Barricade, Blanchwood Armor, Brass's Bounty, Temporal Manipulation, Ghalta, Ashroot Animist,
  Embercleave): **253 supported, 295 tests.** The last 5 tier-3 cards are deferred (QUESTIONS).
  **Performance on huge boards.** The fuzzer hung twice (seeds 3660 and 5660): random players doubled
  Krenko's Goblins to 5,000+. Fixes: an index of permanents with static abilities (lookups no longer
  scan the board), a direct legality check for attack and block declarations (the full list can be
  hundreds of thousands of actions), until-end-of-turn effects stored per object, an invariant for
  the static index, and a 1,000-permanent cap in the fuzz harness only (QUESTIONS). New debugging
  examples: `slow_seeds` (time each game, find hangs) and `time_step` (time one action).
  10,000 release fuzz games pass at 1,121 games/s; 4 stopped at the cap. **Next: tier 4 batch 2: X
  costs (Wildwood Scourge, Goblin Negotiation, Exsanguinate, Genesis Wave, Fiend Artisan), then token
  copies.**
- 2026-11-05: **Tier 4, batch 2: X costs and hybrid mana.** {X} in costs (CR 107.3; X counts 0 in
  mana value off the stack), `CastX` with one action per affordable X, X remembered on the spell and
  the permanent ("enters with X counters"), activated abilities with {X} (`ActivateX`), hybrid pips
  ({B/G}) in parsing and payment, X damage, X drain, excess-damage tokens (CR 120.10), Genesis Wave,
  a creature tutor by X, "+1/+1 per creature card in your graveyard", and a trigger for counters put
  on other non-Hydra creatures. 5 cards (Wildwood Scourge, Goblin Negotiation, Exsanguinate, Genesis
  Wave, Fiend Artisan): **258 supported, 302 tests.** The fuzzer found one more bug (seed 5858):
  Luminous Rebuke's cost reduction counted a tapped creature it couldn't legally target (hexproof
  from instants); fixed with a regression test. 10,000 release fuzz games pass. **Next: token copies
  (Self-Reflection, Electroduplicate, Homunculus Horde, Extravagant Replication, Abyssal Harvester).**
- 2026-11-05: **Tier 4, batch 3: token copies.** Copy tokens take the original card's printed values
  (CR 707.2) and are tokens for every rule (`is_token`), with "has haste and is sacrificed at the
  beginning of the end step" and "also a Nightmare" exceptions, graveyard targets "put there this
  turn", and "exile all other Nightmare tokens". 5 cards (Self-Reflection, Electroduplicate,
  Homunculus Horde, Extravagant Replication, Abyssal Harvester): **263 supported, 308 tests.** 10,000
  release fuzz games pass. **Next: the remaining tier-4 cards: planeswalkers (Liliana, Chandra, Kaito,
  Ajani, Vivien), Doubling Season, Twinflame Tyrant, Thousand-Year Storm, Omniscience, Progenitus,
  Etali, Giada, Zul Ashur, Quilled Greatwurm, Banner of Kinship, Bloom Tender, Fiery Annihilation,
  Involuntary Employment.**
- 2026-11-05: **Tier 4, batch 4: planeswalkers.** Starting loyalty (CR 306.5b; fetched from
  Scryfall by the new `scripts/fetch_loyalty.py`, which caught that Chandra starts at 6), loyalty
  abilities once per turn at sorcery speed and never below zero (CR 606), damage to planeswalkers
  removes loyalty (CR 120.3c), zero loyalty dies (CR 704.5i), attacking planeswalkers
  (`AttackPlaneswalker`; no damage if it left, CR 506.4c), emblems (CR 114: Vivien's anthem, Kaito's
  cast trigger), "keep one of each permanent type", and a trigger for any of your creatures dealing
  combat damage to a player. 4 planeswalkers (Liliana, Kaito, Ajani, Vivien): **267 supported, 316
  tests.** 10,000 release fuzz games pass. **Next: Chandra (mana pool, divided damage), then the
  last tier-4 cards.**
- 2026-11-05 · Tier 4 batch 5: Chandra, Flameshaper. Mana pools (CR 106.4): effects add mana to a
  per-player pool that auto-payment spends before tapping anything, emptied as each step ends (CR
  500.4). Divided damage (CR 601.2d): after the targets, the controller picks each target's share
  (at least 1 each, the last gets the rest) before costs are paid, and a target that's gone at
  resolution gets nothing. +2's exiled card is playable this turn only. 6 tests: **268 supported,
  322 tests.** 10,000 release fuzz games pass. Also fixed CI's ruff format check on
  scripts/fetch_loyalty.py. **Next: the last tier-4 cards (Doubling Season, Thousand-Year Storm,
  Omniscience, Progenitus, ...).**
- 2026-11-05 · Tier 4 batch 6: doubling and free spells. Doubling Season doubles tokens (copies
  included) and every counter we track on your permanents: +1/+1, loyalty (starting loyalty and
  plus costs), stun, incubation, revival. Twinflame Tyrant doubles damage your sources deal to an
  opponent or their permanents; two of either multiply. Omniscience makes spells cast from your hand
  cost nothing but additional costs; X is then 0, so choosing a bigger X pays normally (CR 107.3b).
  6 tests: **271 supported, 328 tests.** 10,000 release fuzz games pass. **Next: Progenitus, Zul
  Ashur, Involuntary Employment, Fiery Annihilation, then the rest of tier 4.**
- 2026-11-06 · **Incident and safeguards.** On 2026-11-05 seed 206 looped forever: Progenitus
  (protection from everything, shuffles itself into the library instead of a graveyard) made two
  "mill until" effects repeat endlessly. The event list grew without limit; two unattended fuzz runs
  reached ~42 GB each and the laptop restarted. Fixes: Consuming Aberration now reveals until a
  land and *then* moves the cards, as printed; Sphinx's Tutelage mills its two cards together and
  doesn't count one that went elsewhere (CR 701.13). Each has a regression test. Safeguards: the
  engine panics past `WORK_BUDGET` (1,000,000 zone changes or SBA passes in one action), the fuzz
  harness reports that panic with its seed (checked by putting the old loop back: seed 206 failed
  in seconds), and `scripts/guard.sh` caps every run's time and memory (session protocol rule 8).
- 2026-11-06 · Tier 4 batch 7: Progenitus (protection from everything: no targeting, damage or
  blocks; shuffled into the library instead of a graveyard), Zul Ashur (cast a Zombie from your
  graveyard this turn; ward 2 life), Involuntary Employment (control until end of turn, untap,
  haste, Treasure), Fiery Annihilation (Equipment target must be attached to the creature; exile
  instead of dying for the rest of the turn). 11 tests: **275 supported, 339 tests.** 10,000
  release fuzz games pass under the guard. **Next: Quilled Greatwurm, Giada, Banner of Kinship,
  Bloom Tender, then Thousand-Year Storm, Etali and the deferred type-changing cards.**
- 2026-11-06 · Tier 4 batch 8: Bloom Tender (vivid: one mana of each color among your
  permanents), Giada (Angels enter with a counter per Angel already there; her {W} pays only for
  Angel spells, the engine's first restricted mana), Banner of Kinship (choose a creature type as it
  enters; fellowship counters pump that type). 4 tests: **278 supported, 343 tests.** 10,000
  release fuzz games pass under the guard. **Next: Quilled Greatwurm (removing counters as a
  casting cost), then Thousand-Year Storm, Etali and the deferred type-changing cards.**
- 2026-11-06 · Tier 4 batch 9: Quilled Greatwurm. New trigger "whenever a creature you control
  deals combat damage during your turn", once per creature per damage step with the total it dealt
  (to players, creatures or planeswalkers); the counters go on only if it survived. Casting it from
  the graveyard costs its mana plus six counters removed from your creatures, one pick at a time
  (Pending::RemoveCounters). 3 tests: **279 supported, 346 tests.** 10,000 release fuzz games pass
  under the guard (932 games/s; Greatwurm games build bigger boards). **Next: Thousand-Year Storm
  (copying spells), then Etali, then the deferred type-changing cards.**
- 2026-11-06 · Tier 4 batch 10: Thousand-Year Storm and spell copies (CR 707.10). A copy is a
  stack entry of kind `SpellCopy`, built from a snapshot of the spell taken as it was cast (so it
  copies kicker, X, mode and targets even if the original has left the stack). It resolves like the
  spell and then ceases to exist instead of moving the card. Instants and sorceries are counted per
  player per turn, including ones cast before the Storm arrived. 5 tests: **280 supported, 351
  tests.** 10,000 release fuzz games pass under the guard. **Next: Etali (cast spells from exile
  for free), then the deferred cards: Kellan, Muldrotha, Imprisoned in the Moon, Soulstone
  Sanctuary, Secluded Courtyard.**
- 2026-11-06 · Tier 4 batch 11: Etali, Primal Storm. Its attack trigger exiles the top card of
  each library; then a free-cast window (`Pending::FreeCast`) opens before anyone gets priority:
  the controller may cast any of the exiled nonland cards (theirs or the opponent's) without paying
  mana costs and at any timing, one after another, or pass. CR 608.2g puts spells cast during
  resolution on the stack once the ability finishes, so the window is equivalent. Cards not cast
  stay exiled. 3 tests: **281 supported, 354 tests.** 10,000 release fuzz games pass under the
  guard. **Next: the five deferred cards (Kellan, Muldrotha, Imprisoned in the Moon, Soulstone
  Sanctuary, Secluded Courtyard), which need type changes and restricted mana.**
- 2026-11-06 · Type layer, batch 1. `types_of` and `colors_of` give a permanent's types and
  colors now (layers 4 and 5); every battlefield type check goes through them (45 sites), while
  cards elsewhere keep their printed types. Imprisoned in the Moon (enchanted permanent is only a
  colorless land with "{T}: Add {C}"; supertypes stay; a planeswalker or creature stops being one)
  and Soulstone Sanctuary ({4}: a 3/3 vigilance creature land with every creature type, with no
  end date). Fixes: Aura legality now uses the Aura's own enchant restriction instead of assuming
  "enchant creature"; Witness Protection now makes the creature green and white. 4 tests: **283
  supported, 358 tests.** 10,000 release fuzz games pass under the guard (~900 games/s; ~5% from
  the type lookup, the rest from the new cards). **Next: Kellan, Secluded Courtyard, Muldrotha.**
- 2026-11-06 · Type layer, batch 2: Kellan and Secluded Courtyard. Stages (`CardDef.stages`,
  `Object.stage`): Kellan's abilities move him Scout → Detective → Rogue only from the right
  stage; each stage sets his creature types and base P/T, keywords gained stay, and the gained
  combat-damage trigger exists only from stage 1 (checked as it would trigger). Copies start at
  stage 0, since "becomes" isn't copiable (CR 707.2). Mana now knows what it pays for (`PayFor`:
  spell, ability or other): Secluded Courtyard chooses a creature type as it enters (land plays
  can now ask), taps for {C} for anything and for any color only for creature spells of that type
  or abilities of creature sources of that type. 4 tests: **285 supported, 362 tests.** 10,000
  release fuzz games pass under the guard. **Next: Muldrotha, the last card, then M5 (PyO3
  bindings).**
- 2026-11-06 · Muldrotha, the last card. During your turn you may play one land and cast one
  permanent spell of each permanent type (artifact, creature, enchantment, planeswalker) from your
  graveyard; a card with two types is cast as the one you pick (`CastFromGraveyardAs`), and the
  land uses your land drop. 4 tests: **all 286 FDN cards supported, 366 tests.** 10,000 release
  fuzz games with every card pass under the guard (850 games/s). **M3 and M4 are done. Next: M5,
  the PyO3 bindings (`bolt_engine.Game` in Python) and a rules-based baseline player.**
- 2026-11-06 · M5 part 1: Python bindings. New crate `engine/bolt-py` (pyo3 0.30, abi3 for
  Python ≥ 3.11, built with maturin; `make engine-py`). `bolt_engine.Game(deck_a, deck_b, seed,
  first)` takes card numbers and rejects unsupported ones; `legal_actions()` returns flat
  `(kind, a, b, c)` tuples and `step(i)` applies one; `observation(player)` gives what that player
  can see (their hand, not the opponent's); `clone()` copies for search; `result()`, `to_act()`,
  `pending()`. 6 Python tests (random games finish, hidden hands, replay). CI's engine job now
  builds the wheel and runs them. The crate only warns on unsafe code, because pyo3's macros
  expand to some; the workspace still forbids it everywhere else. **Next: the rules-based
  baseline player and a games/second benchmark.**
