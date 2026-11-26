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

## Known rules gaps

- **Split card mana value on the stack (CR 709.3b):** a split spell's mana value should be only
  the cast half's, but checks that read `def.cost` on the stack (Pollywog Prodigy, spell mana-value
  filters) see both halves' sum. Fix later with a stack-side branch guarded on `split_card`. Also
  flashback on a split card pays the sum, and the AI's keep-mana estimate counts the sum.
- Venturing while still on a bottommost room completes that dungeon first, then starts a new one.
- A room ability's stack source is the card that made you venture (there's no dungeon object).
- A bottommost room ability that fizzles (illegal targets) doesn't complete the dungeon.
- Unready rooms still do nothing when entered: Twisted Caverns and Arena (lane B: "can't attack
  until your next turn", goad), Runestone Caverns (how long "you may play them" lasts is unsettled
  here), Mad Wizard's Lair (d3 handed it to the merger: a free cast of one of three drawn cards).
  Arena, Twisted Caverns and Runestone Caverns each have a ready sibling room to pick instead;
  Mad Wizard's Lair is Dungeon of the Mad Mage's last room, so that dungeon's ending is a no-op.
- Lolth's emblem and Ob Nixilis's "a player draws" act on the one opponent (1v1); Ob Nixilis's
  emblem is kept as two entries (its owner draws; the other player draws).

## Next

- **Urza's Saga** (handed back): needs the merger's `activations()` to skip
  `self.lacks_chapter_activated(id, n)` and d1's mana sources (`single_sources`/`tap_for_mana`)
  to skip `self.lacks_chapter_mana(id)`. Then script it: `mana` {C} with `chapter_mana: Some(1)`,
  `activated[0]` "{2}, {T}: create `t.saga_construct`" with `chapter_activated: [(2, 0)]`,
  chapters I/II `GainChapterAbilities`, III `Search { ArtifactManaCostZeroOrOne, Battlefield }`.
  Check whether the loader sets `mana` from `produced_mana` for it.
- Venture cards still waiting: Barrowin (d3: "return up to one creature card with mana value 3
  or less from your graveyard to the battlefield", untargeted), Sefris (d2: a "you complete a
  dungeon" event from `complete_dungeon`; its other trigger,
  `When::CreatureCardsToYourGraveyardOnceEachTurn`, is in).
- The Ring (Call of the Ring): the tempting and Ring-bearer state would be d4's, but the Ring's
  four abilities need d1 (blocked by greater power), d3 (legendary), d2 (bearer attacks / is
  blocked / deals damage; "you choose a Ring-bearer"); not started.
- Dice and coins still waiting: Invert Polarity (gain control of a spell: merger/Mac C),
  Underdark Rift (into the library beneath the top X: d3/Mac C), Ancient Brass Dragon (reflexive
  trigger, total mana value X from graveyards: d3).
- Split card mana value on the stack (above).
