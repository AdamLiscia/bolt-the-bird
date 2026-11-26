# Lane d4 log: subsystems (dungeons, the initiative, Backgrounds, emblems, dice, the Ring, sagas, split cards)

Read `docs/lane-brief.md` first. Worktree `/home/adam/bolt-d4`, branch `lane-d4`, target dir
`$HOME/bolt-target-d4`. Cards in `engine/bolt-cards/src/scripts/pg_d4.rs` (tests `tests/pg_d4.rs`);
core in `engine/bolt-core/src/game/d4.rs` (a child module of game.rs; tests `tests/core_subsystems.rs`).

## Batches

- **f0dc9d1** — split cards (Boom // Bust, Fire // Ice): `fetch_oracle.py` keeps `split`; the
  right half is the card's `omen` ("Split: <line>"), cast by `CastOmen`, to the graveyard after.
  `CardDef.subsystems: SubsystemExt` (d4's fields). Choose a Background in
  `commander_deck_problems_for`.
- **6a60e02** — dungeons (`rooms(d)`), venture (`SubsystemEffect::Venture`/`VentureUndercity`,
  `Decision::ChooseDungeon`/`ChooseRoom`, `StackKind::Room`), completion (CR 704.5t), room
  doubling (Hama Pashar, Dungeon Delver), the initiative (`take_initiative`,
  `StackKind::InitiativeVenture`), `Game::fire_subsystems` (emblem triggers via
  `EmblemKind::Triggered`, the initiative's upkeep venture and combat-damage steal),
  `Condition::CompletedDungeon`/`HaveInitiative`, `StaticCondition::CompletedDungeon`, dice
  (`roll_die`, `RollDieCreateTokens`), `EmblemForTarget`.
- **7721ebf** — tokens 77-82 (Skeleton 1/1, Skeleton 4/1 menace, God Horror, Faerie Dragon, Spider
  2/1 reach menace, Urza's Saga Construct); 70-75 are placeholders Bolt replaces with lane B's at
  merge, 76 stays a placeholder until d1's Goat. Three rooms made ready. Split mana value off the
  stack is both halves' (`cost` is the sum, `left_cost` is paid for the left half); on the stack
  `colors_of` gives the cast half's colors. Ancient Gold Dragon.

- **batch 4** — d3's room effects wired: Fungi Cavern, Veils of Fear, Oubliette, Sandfall Cell,
  Throne of the Dead Three (Lost Mine and Tomb of Annihilation are now complete). d4 effects
  `DrawIfCompletedDungeon`, `TokensByDungeon`, `UntapOneOrAllByDungeon` (`Decision::UntapOwnCreature`,
  Pick a tapped creature you control; labeled "Untap <name>"), `OpponentLosesDifferenceFrom`.
  Saga chapter grants, ready for Urza's Saga: `SubsystemExt::chapter_activated`/`chapter_mana`,
  `SubsystemEffect::GainChapterAbilities(n)`, `Game::chapter_grants`, and
  `Game::lacks_chapter_activated`/`lacks_chapter_mana` (not called yet: see Next). On d2's
  `fire()` hook: White Plume Adventurer, Imoen, Safana, Lolth, Ob Nixilis Reignited, Professor
  Dellian Fel; plus Radiant Solar and Nadaar; on d1's pieces Yuan-Ti Malison, Midnight
  Pathlighter, Dungeon Descent. Coin flips: `Game::flip_coin`, `FlipCoinLoseDamageYou` (Mana
  Crypt); Ancient Copper Dragon on `RollDieCreateTokens`.
- **batch 5** — the Ring (CR 701.54): `SubsystemEffect::RingTempts`, `Game::ring_tempts`
  (level up to 4, then the bearer: the only creature, or `Decision::ChooseRingBearer`, Pick a
  creature you control, labeled "Choose as your Ring-bearer: <name>"), `Game` fields
  `ring_level`/`ring_bearer` (creature, zcc)/`ring_tempted`/`ring_blocks_seen`, and
  `ring_bearer(p)`/`is_ring_bearer`/`ring_level`. The Ring's triggered abilities are rows of
  `rooms(Dungeon::TheRing)` (not a dungeon; `ring::LOOT`, `SACRIFICE_BLOCKER`, `DRAIN`) on
  `StackKind::Room`, so no new stack kind or `ability_of` arm. Level 2 (loot when the bearer
  attacks) and level 3 (a blocker of the bearer is sacrificed at end of combat,
  `RingSacrificeBlocker`) fire from `fire_subsystems` on `YouAttack` / `OpponentCreatureBlocks`;
  level 1 and 4 are hooks for other lanes (`ring_makes_legendary` d3, `ring_evasion_forbids_block`
  d1, `ring_bearer_dealt_combat_damage` d2), and `choose_ring_bearer`/`ring_tempts` mark where d2
  emits "you choose a Ring-bearer" / "the Ring tempts you". Split mana value on the stack:
  `Game::stack_mana_cost(id)` (the cast half's cost on the stack, else `cost`) and
  `cast_mana_cost(item)`, which `finish_cast` now reads for Pollywog Prodigy and
  `YouCastNoncreatureMvAtLeast`/`YouCastColorlessMvAtLeast` (two lines changed in the split-cast
  path). Card: Fiery Inscription.
- **04b3e9e** — Runestone Caverns on d3's `ExileTopToPlayThisTurn(2)`; with lane B's rooms every
  dungeon room is ready (the `missing()` helper is gone).
- **batch 6** — token 95 Smaug (legendary 6/6 red Dragon, flying, haste; dies into fourteen
  Treasures via `RoomToken` by name), with placeholders 91-94. Barrowin of Clan Undurr on d3's
  `ReturnCreatureMvAtMostToBattlefield(3)`. Sefris is d2's now (it uses d4's
  `YouCompleteDungeon` emit). There and Back Again and the four Bloomburrow Talents are blocked
  on other lanes' pieces (needs_core.json).

## Known rules gaps

- **Split card mana value on the stack (CR 709.3b):** the cast triggers in `finish_cast` read the
  cast half's; `target_ok`'s spell mana-value filter (d3) and `LoseLifeCastSpellManaValue`
  (merger) still read `def.cost` (handed back: use `stack_mana_cost`). Flashback on a split card
  pays the sum, and the AI's keep-mana estimate counts the sum. `colors_of` on a split spell during
  `finish_cast` (before its entry is on the stack) may still see both halves' colors.
- The Ring: its abilities' stack source is the Ring-bearer (there's no emblem object). Its
  level-3 trigger fires once per blocker per turn (`ring_blocks_seen`), so a second combat in the
  same turn with the same blocker doesn't trigger again. Level 1 and 4 do nothing until the hooks
  are called.
- Venturing while still on a bottommost room completes that dungeon first, then starts a new one.
- A room ability's stack source is the card that made you venture (there's no dungeon object).
- A bottommost room ability that fizzles (illegal targets) doesn't complete the dungeon.
- Every room is ready: Twisted Caverns, Arena and Mad Wizard's Lair are lane B's (tests/core_merger.rs),
  Runestone Caverns is d3's `ExileTopToPlayThisTurn(2)` ("you may play them" lasts this turn).
- Lolth's emblem and Ob Nixilis's "a player draws" act on the one opponent (1v1); Ob Nixilis's
  emblem is kept as two entries (its owner draws; the other player draws).

## Next

- All of the following are in `handoff/lanes/needs_core.json` ("d4 handback").
- **Urza's Saga** (handed back; still blocked at tarkir 29be832): needs the merger's `activations()` to skip
  `self.lacks_chapter_activated(id, n)` and d1's mana sources (`single_sources`/`tap_for_mana`)
  to skip `self.lacks_chapter_mana(id)`. Then script it: `mana` {C} with `chapter_mana: Some(1)`,
  `activated[0]` "{2}, {T}: create `t.saga_construct`" with `chapter_activated: [(2, 0)]`,
  chapters I/II `GainChapterAbilities`, III `Search { ArtifactManaCostZeroOrOne, Battlefield }`.
  Check whether the loader sets `mana` from `produced_mana` for it.
- Venture cards still waiting: Barrowin (d3: "return up to one creature card with mana value 3
  or less from your graveyard to the battlefield", untargeted), Sefris (d2: a "you complete a
  dungeon" event from `complete_dungeon`; its other trigger,
  `When::CreatureCardsToYourGraveyardOnceEachTurn`, is in).
- Call of the Ring: needs d2's "whenever you choose a creature as your Ring-bearer" event (from
  `choose_ring_bearer`); its upkeep tempting is ready. The Ring's level 1 (d3, d1) and level 4
  (d2) hooks. There and Back Again (Saga with the Ring) needs "can't block for as long as you
  control this Saga" and Smaug's token.
- Classes (Wizard Class, Sorcerer Class, Warlock Class, the Talents): levels need an activation
  gate in the merger's `activations()` (like Urza's Saga's), then d4 adds level state.
- Aftermath (Dusk // Dawn, Commit // Memory): casting the right half only from the graveyard is
  the merger's legal-action listing.
- Backgrounds granting triggers: Far Traveler (d3: a tapped-creature filter), Agent of the Iron
  Throne (d2: "an artifact or creature you control is put into a graveyard from the battlefield",
  once for an artifact creature).
- Dice and coins still waiting: Invert Polarity (gain control of a spell: merger/Mac C),
  Underdark Rift (into the library beneath the top X: d3/Mac C), Ancient Brass Dragon (reflexive
  trigger, total mana value X from graveyards: d3).
- Split card mana value on the stack: the d3 and merger reads (above).
