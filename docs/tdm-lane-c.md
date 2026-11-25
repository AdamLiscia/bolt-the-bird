# Tarkir: Dragonstorm, lane C

Scripts: `engine/bolt-cards/src/scripts/tdm_c.rs`. Tests: `engine/bolt-cards/tests/tdm_c.rs`.

## Done (15)

| Card | How |
|---|---|
| Summit Intimidator | ETB: target creature gains "can't block" until end of turn |
| Arashin Sunshield | ETB: exile up to two target cards from a single graveyard; `{W}, {T}`: tap target creature |
| Lightfoot Technique | +1/+1 counter, then flying and indestructible until end of turn |
| Unburied Earthcarver | `{2}`, sacrifice another creature: +1/+1 counter on itself |
| Sunset Strikemaster | Taps for `{R}`; `{2}{R}, {T}`, sacrifice: 6 damage to target creature with flying |
| Opulent Palace | Enters tapped; taps for B, G or U |
| Jeskai Shrinekeeper | Combat damage to a player: gain 1, draw 1 |
| War Effort | Your creatures +1/+0; whenever you attack, a Warrior tapped and attacking, sacrificed at the next end step (the mobilize effect) |
| Wail of War | Modal: creatures target opponent controls get -1/-1; or return up to two creature cards from your graveyard to hand |
| Jeskai Brushmaster | Prowess (double strike is a keyword line) |
| Starry-Eyed Skyrider | Attacks: another target creature you control gains flying; attacking tokens you control have flying |
| Loxodon Battle Priest | Beginning of combat on your turn: +1/+1 counter on another target creature you control |
| Fangkeeper's Familiar | Flash; ETB modal: gain 3 + surveil 3, destroy target enchantment, or counter target creature spell |
| Inevitable Defeat | Can't be countered; exile target nonland permanent, its controller loses 3, you gain 3 |
| Perennation | Return target permanent card from your graveyard with hexproof and indestructible counters (kept as keywords for as long as it's on the battlefield, the same way renew's keyword counters work) |

## Needs core

- Aggressive Negotiations: an effect that exiles the chosen nonland card from a revealed hand (`ChooseDiscard` only discards).
- Coordinated Maneuver: damage equal to the number of creatures you control (no count-scaled damage effect).
- Fire-Rim Form: an Aura ETB effect that grants a keyword to the enchanted creature until end of turn (only `PumpAttached`, which has no keywords).
- Krotiq Nestguard: "can attack this turn as though it didn't have defender" (the attack check has no override for defender).
- Twin Bolt: target choice lets the same player fill both slots of a divided "one or two targets" spell (CR 115.3); `ChooseTarget` dedupes objects only, not players.
- Delta Bloodflies: a `Condition` "you control a creature with a counter on it".
- Roiling Dragonstorm: an effect that returns the source from the battlefield to its owner's hand (`ReturnSelfToHand` works only from the graveyard).
- Dragonbroods' Relic: a "tap an untapped creature you control" mana cost, and the Reliquary Dragon token (all colors, flying, lifelink, ETB 3 damage).
- Jeskai Monument: a search for a basic land of the named types only (`SearchFor::BasicLand` takes any basic).
- Kishla Trawlers: an optional "exile a creature card from your graveyard" action with a reflexive trigger that targets.
- Temur Monument: a search for a basic land of the named types only.
- Traveling Botanist: a `When` for "becomes tapped", and a look-at-top that lets a non-land go to the graveyard.
- Mardu Monument: a search for a basic land of the named types only, and "they gain menace and haste" for the tokens just created.
- Teeming Dragonstorm: a 2/2 white Soldier token, and returning the source from the battlefield to hand.
- Frontline Rush: +X/+X where X is the number of creatures you control.
- Sunpearl Kirin: "If it was a token, draw a card" (a condition on the bounced permanent).
- Veteran Ice Climber: mill equal to the source's power.
- Furious Forebear: a "creature you control dies" trigger that works from the graveyard, with a pay-to-return.
- Duty Beyond Death: a +1/+1 counter on each creature you control.
- Formation Breaker: "creatures with power less than this creature's power can't block it", and a static condition "you control a creature with a counter on it".
- Abzan Monument: a search for a basic land of the named types only, and an X/X Spirit with X the greatest toughness among your creatures.
- Unsparing Boltcaster: a target filter "that was dealt damage this turn".
- Armament Dragon: distributing +1/+1 counters among one to three targets.
- Aegis Sculptor: an optional upkeep "exile two cards from your graveyard. If you do, ...".
- Rainveil Rejuvenator: an optional mill ("you may mill three"), and mana equal to the source's power.
- Stormbeacon Blade: an "equipped creature attacks" trigger with an "if you control three or more attacking creatures" condition.
- Hardened Tactician: a "sacrifice a token" activation cost.
- Corroding Dragonstorm: returning the source from the battlefield to its owner's hand.
- Yathan Tombguard: a "creature you control with a counter on it deals combat damage to a player" trigger.
- Marshal of the Lost: +X/+X where X is the number of attacking creatures.
- Dragonologist: a look-at-top pick for "instant, sorcery, or Dragon card", and a static "untapped Dragons you control have hexproof".

## Notes

- The lanes share `CARGO_TARGET_DIR`, and both worktrees build `bolt-cards` to the same artifact names, so a
  test binary can link the other lane's `bolt-cards` build (every script here was missing until a rebuild).
  Touching `src/scripts/tdm_c.rs` before each run forces the right build.
