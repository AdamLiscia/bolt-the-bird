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

## Chunk 2: the rest of the list (no cards doable on the current engine)

Assessed after the merge at e5c03c9. Every remaining card needs core:

- Dracogenesis: free casting limited to Dragon spells (`free_from_hand` covers every spell).
- Mox Jasper: a mana ability restriction "activate only if you control a Dragon".
- Scalding Tarn, Verdant Catacombs, Marsh Flats: a "pay 1 life" activation cost, and a search for a land card with one of two basic land types (not only basics; `SearchFor` has only BasicLand/Any).
- Genesis Ultimatum: look at the top five and put any number of permanent cards onto the battlefield with the rest going to hand, then exile the spell itself.
- Eerie Ultimatum: return any number of permanent cards with different names from your graveyard.
- Cori Mountain Monastery, Kishla Village, Mistrise Village: "enters tapped unless you control a [type] or a [type]". Mistrise also needs "the next spell you cast this turn can't be countered".
- Surrak, Elusive Hunter: a trigger for "a creature you control or a creature spell you control becomes the target of a spell or ability an opponent controls".
- Tersa Lightshatter: "discard up to two cards, then draw that many", and a threshold attack trigger that exiles a random graveyard card you may play this turn.
- Windcrag Siege, Frostcliff Siege, Hollowmurk Siege: "as this enters, choose [clan] or [clan]" modes, each with its own ability.
- Flamehold Grappler: "copy the next spell you cast this turn".
- Mardu Siegebreaker: copy tokens of a linked exiled card, attacking, sacrificed at your next end step.
- Maelstrom of the Spirit Dragon: mana spendable only on Dragon or Omen spells, and a search for a Dragon card.
- Kotis, the Fangkeeper: exile the top X (combat damage dealt) of their library and cast spells with mana value X or less free.
- United Battlefront: put up to two noncreature, nonland permanent cards with mana value 3 or less from the top seven onto the battlefield.
- Severance Priest: exile a card chosen from a revealed hand, and a leaves-the-battlefield trigger giving its owner an X/X Spirit (X its mana value).
- Sidisi, Regent of the Mire: a "sacrifice a creature with mana value X" cost and a return target with mana value X plus 1.
- Ureni, the Song Unending: protection from white and from black as a keyword line, and divided damage X (lands you control) among opponents' creatures and planeswalkers.
- Dragonback Assault: 3 damage to each creature and each planeswalker (`Recipient::EachCreature` skips planeswalkers).
- Ugin, Eye of the Storms: a "when you cast this spell" trigger, a "whenever you cast a colorless spell" trigger, a "permanent that's one or more colors" filter, and the -11 search.
- All-Out Assault: an additional combat phase and main phase, and "when you next attack this turn, untap each creature you control".
- Craterhoof Behemoth: +X/+X for your creatures, where X is the number of creatures you control.

## Round 3, batch 1: core mechanics (10 cards)

Engine-core work allowed from here on. New variants and fields all go at the end of their enum or struct.

Mechanics added (bolt-core):
- `Filter::was_dealt_damage`: creatures with damage marked this turn (damage is marked until cleanup, so this is exact).
- `Filter::creature_max_power` now also applies to graveyard cards (their printed power).
- Divided damage: one player can't fill two slots of a single divided "target" word (CR 115.3). The check applies only to abilities with `divide > 0`. `Effect::DividedDamage` now damages players too ("any target").
- `Activated::sacrifice_token` ("Sacrifice a token"): `ActivateWith(source, n, token)`. The token being sacrificed is kept out of the mana payment, so a Treasure can't pay {1} and also be the sacrifice.
- `Effect::GrantAttached(Keywords)`: the enchanted or equipped creature gains keywords until end of turn.
- `Effect::AttackDespiteDefenderSelf` and `Object::defender_ignored_turn`: it can attack this turn as though it didn't have defender. This resets when it leaves the battlefield.
- `Effect::CountersAll { who, n }`: +1/+1 counters on each creature a player controls.
- `Effect::DestroyOpponentsNonland`, `Effect::DestroyCreaturesAndEnchantmentsDraw` (draws one for each permanent that actually left; indestructible survivors don't count).
- `Recipient::EachCreatureAndPlaneswalker`.

Cards: Twin Bolt, Unsparing Boltcaster, Smile at Death, Hardened Tactician, Fire-Rim Form, Krotiq Nestguard, Ruinous Ultimatum, Death Begets Life, Dragonback Assault, Duty Beyond Death.

## Round 3, batch 2: the Sieges (5 cards)

Mechanics added (bolt-core, all at the end of their enum or struct):
- "As this enters, choose [A] or [B]": `CardDef::enter_modes`, `Object::chosen_mode`, and
  `Decision::ChooseEnterMode { card }` (`Mode(0)` is the first clan named). The choice is asked as the spell
  resolves, the same way `choose_color` works. **A Siege put onto the battlefield without being cast (for
  example by Perennation) gets no choice, so it has no mode and neither ability.** The mode is cleared
  when the object enters and kept after it leaves, so a trigger already on the stack still resolves.
- `Condition::ChosenMode(u8)`, `StaticCondition::ChosenMode(u8)`: an ability that exists only in one mode.
- `Condition::Both(Box<Condition>, Box<Condition>)`, `Condition::CreatureDiedUnderYourControlThisTurn`
  (tracked in `Game::creature_died_under`, reset each turn).
- `When::YourCreaturesDealCombatDamageToPlayer`: "one or more creatures you control deal combat damage to
  a player", once per combat damage step.
- `When::CounterPutOnYourCreatureOnceEachTurn` (`Object::once_each_turn`): fired from one hook,
  `counter_put_on`, for +1/+1 counters, stun counters, keyword counters, and counters a creature enters
  with (subtype bonus, `enters_counters`, the endure Spirit; CR 122.6).
- `CardDef::attack_triggers_again_in_mode`: in that mode, the active player's "attacks" and "you attack"
  triggers (mobilize included) trigger an additional time per such Siege.
- `CardDef::graveyard_lands_in_mode`: in that mode, lands can be played from your graveyard using your
  normal land drops (this doesn't use Muldrotha's once-per-type allowance).

Cards: Windcrag Siege, Frostcliff Siege, Hollowmurk Siege, Glacierwood Siege, Barrensteppe Siege.

Merge note: the keyword-counter hook is one added line in `Effect::KeywordCounter`, which on tarkir now
writes `keyword_counters`.
