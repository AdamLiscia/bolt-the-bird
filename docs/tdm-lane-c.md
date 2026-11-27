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

## Round 3, batch 3: hands, graveyards, power-scaled effects, Dragon-only rules (11 cards)

Mechanics added (bolt-core, all at the end of their enum or struct):
- `Effect::ChooseExileFromHand { slot }` with `Decision::PickExile { victim }`: you choose a nonland card
  from the target player's hand and exile it.
- `Effect::BounceDrawIfToken(slot)`: return it to hand, and draw if it was a token.
- `Effect::MillSourcePower(slot)`: mill equal to the source's power (last known if it left).
- `Effect::MayExileFromGraveyard { n, creature_only }` with `Decision::ExileFromGraveyard`: "you may exile N
  cards from your graveyard. If you do, ..." The rest of the effects happen only if all N are exiled
  (nothing is offered when there are fewer than N). With `Effect::Reflexive` it covers "When you do".
- `Effect::GateAttackingAtLeast(n)`: "if you control N or more attacking creatures", checked on resolution.
- `When::EquippedAttacks` (fired when the equipped creature is declared as an attacker; Windcrag's Mardu
  mode applies to it too) and `When::CreatureYouControlDiesFromGraveyard` (cards already in the dying
  creature's controller's graveyard; not the dying card itself).
- `Pick::InstantSorceryOrSubtype(&str)`; `Scope::YourUntappedSubtype(&str)`.
- `CardDef::mana_only_if_you_control` (Mox Jasper), `CardDef::mana_equal_power` (Rainveil Rejuvenator: one
  activation makes that much mana; unused mana from it is lost, the same as `mana_per_subtype`),
  `CardDef::free_subtype_spells` (Dracogenesis: Dragon spells from any zone cost nothing; this doesn't
  combine with alternative costs, flashback/harmonize or X, and never applies to an Omen half).

Cards: Aggressive Negotiations, Sunpearl Kirin, Veteran Ice Climber, Kishla Trawlers, Rainveil Rejuvenator,
Aegis Sculptor, Stormbeacon Blade, Dragonologist, Furious Forebear, Mox Jasper, Dracogenesis.

Known edges:
- Furious Forebear: when it dies in the same event as another creature of yours, the engine moves them one at
  a time, so it can see the other's death if that one is moved after it. It doesn't see its own death.
- Waiting for the tarkir merge (`Object::keyword_counters`): Yathan Tombguard, Delta Bloodflies and Formation
  Breaker ("a creature with a counter on it"), and Armament Dragon (`Effect::DividedCounters`).

## Round 3, batch 4 (9 cards), on tarkir 20425f5

Mechanics added (bolt-core, all at the end of their enum or struct):
- `When::YourCreatureWithCounterDealsCombatDamageToPlayer` (uses `Game::has_counter`, which counts keyword counters).
- `When::YourCreatureTargetedByOpponent`: fired from `ward_check` (every spell, ability and copy as it gets its
  targets) for each target that is a creature on the battlefield, or a creature spell, controlled by an opponent
  of the targeting item's controller.
- `When::LeavesBattlefield`: fired from `move_to` for any move off the battlefield.
- `Effect::TopCardsOntoBattlefield { n, up_to, noncreature_nonland, max_mv, rest_to_hand }` with
  `Decision::PutFromAmong`: put up to N (or any number) permanent cards from the top N onto the battlefield, the
  rest to hand or to the bottom in a random order. Cards go onto the battlefield one at a time.
- `Effect::ExileSourceSpell` ("Exile Genesis Ultimatum").
- `Effect::ReturnPermanentsDifferentNames` with `Decision::ReturnDifferentNames` (one at a time, by name).
- `Effect::DiscardUpToThenDrawThatMany(n)` with `Decision::DiscardUpToDraw`.
- `Effect::ExileRandomFromGraveyardPlayable` (`Object::playable` for this turn).
- `Effect::ExileTopOfDamagedCastFree`: the top X of the opponent's library (X the combat damage, the trigger's
  amount), with nonland cards of mana value X or less castable free in the existing free-cast window (Etali).
- `Effect::ChooseExileFromHandLinked { slot }` with `Decision::PickExileLinked`, `Object::linked_card`, and
  `Effect::LinkedOwnerCreatesSpirit { spirit }` (the endure Spirit, given X counters).

Cards: Yathan Tombguard, Armament Dragon (on `Effect::DividedCounters`), Genesis Ultimatum, United Battlefront,
Eerie Ultimatum, Tersa Lightshatter, Surrak, Elusive Hunter, Kotis, the Fangkeeper, Severance Priest.

Still needing core (assessed, not done):
- Dragonbroods' Relic: a mana ability with "tap an untapped creature you control" in its cost, and the
  Reliquary Dragon token (all colors, flying, lifelink, its own enters trigger).
- Flamehold Grappler: copying the next spell you cast this turn, with new targets allowed (the existing copy
  keeps the same targets).
- Sidisi, Regent of the Mire: a "sacrifice a creature with mana value X" cost tied to a target's mana value.
- Ureni, the Song Unending: divided damage whose total is X (lands you control) chosen on resolution of the
  count, among any number of targets; `Ability::divide` is a fixed number.
- Ugin, Eye of the Storms: "when you cast this spell" and "whenever you cast a colorless spell" triggers, a
  colored-permanent filter, and the -11.
- All-Out Assault: an extra combat and main phase, and a delayed untap trigger.
- Maelstrom of the Spirit Dragon: mana spendable on Dragon spells or Omen spells. Payment only knows the card
  (`PayFor::Spell(card)`), not whether it is being cast as its Omen half.

## Round 3, batch 5 (5 cards)

Mechanics added (bolt-core, all at the end of their enum or struct):
- `Ability::divide_lands`: the divided total is the number of lands its controller controls as it goes on the
  stack. No more targets than the total are offered (each needs at least 1), and one player can't fill two
  slots of a divided ability.
- `Activated::sacrifice_mv_x` and `Filter::mana_value_is_x`: "Sacrifice a creature you control with mana value
  X other than this", offered as `ActivateX { victim, x }` with X the victim's mana value plus 1, and only when
  a target of that mana value exists.
- `CardDef::mana_taps_creature`: the mana ability also taps an untapped creature you control. The payer picks
  one that isn't a mana source itself (lowest power), so it can't be one the same payment needs. A creature
  that is also a mana source is never tapped for it, which only ever makes the Relic unavailable, never
  illegal.
- The Reliquary Dragon token (`Tokens::reliquary_dragon`, the last token): 4/4, all colors, flying, lifelink,
  "When this token enters, it deals 3 damage to any target."
- Additional combat and main phases: `Effect::AdditionalCombatAndMain` with `Game::extra_combats`,
  `extra_combat_resume` and `extra_main_resume` (CR 500.8). An extra combat comes right after the current main
  phase, then an extra main phase (a `Main2`), then whatever came next (the regular combat, or the end step).
- `Condition::YourMainPhase`; `Effect::DelayedOnNextAttackThisTurn(n)` (`Game::delayed_on_next_attack`,
  removed when it triggers); `Effect::UntapCreaturesYouControl`.
- `PayFor::OmenSpell` (via `pay_for_spell(card, mode)`): mana payment knows a card is being cast as its Omen
  half. `CardDef::any_color_for_dragon_or_omen`: any color only for a Dragon spell or an Omen spell; plain {C}
  otherwise. Kept out of manual mana, like other restricted sources. `SearchFor::Subtype`.

Cards: Ureni, the Song Unending, Sidisi, Regent of the Mire, Dragonbroods' Relic, All-Out Assault, Maelstrom of
the Spirit Dragon.

## Round 3, batch 6 (2 cards)

Mechanics added (bolt-core, all at the end of their enum or struct):
- Spell copies with new targets: `Effect::CopyNextSpellThisTurn(n)` (`Game::copy_next_spell`; the delayed
  trigger fires from `finish_cast` with the spell in `extra`) and `Effect::CopySpellNewTargets`, which queues
  the copy in `Game::pending_copies`. When the ability finishes resolving, a copy with targets goes through the
  normal target choice (any legal targets, so "may choose new targets" includes the old ones); if no full set
  of targets is possible it keeps the original ones. A copy of a permanent spell becomes a token as it resolves
  (CR 707.10f), attached to its target if it's an Aura.
- `When::CastThisSpell` (triggers from the stack as the spell is cast) and `When::YouCastColorless`.
- `Filter::colored` ("a permanent that's one or more colors"), `Effect::AddColorless(n)`,
  `SearchFor::ColorlessNonland`, `SearchTo::ExileCastFreeThisTurn`.

Cards: Flamehold Grappler, Ugin, Eye of the Storms.

Known edge: with two copy triggers from one cast (two Grapplers), the second copy waits for the next ability to
finish resolving if the first one needed a target choice.

## Round 3, batch 7: high-pick uncommons, then rares on small mechanics (13 cards)

Mechanics added (bolt-core, all at the end of their enum or struct):
- `Reduction::PerCreatureYouControlPowerAtLeast(n)`; `Effect::CounterUnlessPaysX(slot)` (the spell's X).
- `Effect::MayDiscardReflexiveIfNonland(n)` with `Decision::DiscardReflexive`: "you may discard a card. When
  you discard a nonland card this way, ..." (the reflexive trigger's source is the spell).
- `Effect::GrantYourCreaturesWithCounters(Keywords)`: the creatures are fixed as it resolves (CR 611.2c).
- `Effect::DiscardTwoUnlessCreature` with `Decision::DiscardTwoUnlessCreature`.
- `Activated::life_x` ("Pay X life", only X up to your life is offered) and `Effect::EndureX`.
- `Effect::DamageEqualCreaturesYouControl(slot)` (counted on resolution, after the sacrifice),
  `Effect::MillLandsYouControl`, `Effect::CountersPerLandCardInGraveyard(slot)`.
- `When::AnotherNontokenCreatureYouControlEnters` (the entering creature fills a triggering slot) and
  `Effect::SlotEnduresSourceCounters` with `Decision::EndureOther`: another creature endures X, X the number of
  counters on the source (+1/+1, stun and keyword counters).
- `Effect::MayContinue` ("You may ..." for the rest of the effects) and `Effect::AttachToCreated`.
- `Effect::GrantHarmonize(slot)` and `Object::harmonize_eot`: harmonize until end of turn for its mana cost,
  including the tap-a-creature reduction.

Cards: Spectral Denial, Rally the Monastery, Glacial Dragonhunt, Synchronized Charge, Winternight Stories, Krumar
Initiate, Sinkhole Surveyor, Qarsi Revenant, Stadium Headliner, Lasyd Prowler, Warden of the Grove, Cori-Steel
Cutter, Songcrafter Mage.

Assessed and left (needs core):
- Rite of Renewal: "up to two" targets of yours, then a required target player and "up to four" cards from that
  player's graveyard. Target slots are filled in order with one minimum count, so the four can't be chosen
  after skipping the two, and no filter ties cards to the player in another slot. It also needs "shuffle
  target cards into their library".

## Round 3, batch 8a: the rest of Tarkir (5 cards)

Mechanics added (bolt-core, all at the end of their enum or struct):
- `CardDef::opponents_cant_cast_your_turn` (no casting from any zone, free casts included; lands can still be
  played and abilities activated), `CardDef::warrior_tokens_unsacrificeable_end_step` (`cant_be_sacrificed`:
  checked for end-step sacrifices, "sacrifices a creature" effects and every sacrifice cost),
  `CardDef::creature_spell_discount`.
- `When::CastCreatureYouControlEnters`: fired as a cast creature spell resolves onto the battlefield (the
  "if you cast it" creatures), with the creature in a triggering slot.
- `Effect::ReturnUpToPermanentCardsToHand(n)` and `Effect::ShuffleUpToFromGraveyard { slot, n }` with their
  decisions. Rite of Renewal's cards are chosen as it resolves rather than as targets (the coordinator's call):
  it targets only the player.

Cards: Anafenza, Unyielding Lineage, Voice of Victory, Zurgo, Thunder's Decree, The Sibsig Ceremony, Rite of
Renewal.

## Round 3, batch 8b (5 cards)

Mechanics added (bolt-core, all at the end of their enum or struct):
- `Effect::MayBeholdThen` (no Dragon to behold: no question, nothing happens) and `Effect::SubtypeUntilEotSelf`
  (`Object::eot_subtype`).
- `Effect::CopyLinkedExileAttacking`: a tapped, attacking token copy of the card the source exiled until it
  leaves, sacrificed at the next end step.
- Dragonfire Blade: `CardDef::equipped_hexproof_from_monocolored` (checked in `slot_ok` against the targeting
  item's source), and `Activated::less_per_target_color`. That equip is offered when the most colorful creature
  you control makes it affordable; only targets you can then afford are offered; the discount is passed to the
  payment as the ability's X.
- Card copies (CR 707.12): `Effect::ExileCastCopyFree` makes an `Object::card_copy` in exile, castable free in
  the free-cast window. It exists only there, on the stack, or on the battlefield (where it becomes a token); a
  copy not cast, or put anywhere else, ceases to exist. `Filter::nonland_cards`.
- `Effect::SearchMonocoloredOpponentPicks` with `Decision::SearchMonoNames` and `Decision::OpponentShufflesOne`
  (asked of the opponent); the other cards are cast free in the free-cast window.

Cards: Sarkhan, Dragon Ascendant, Mardu Siegebreaker, Dragonfire Blade, Shiko, Paragon of the Way, Emergent
Ultimatum.

## Round 3, batch 8c (1 card)

- Decayed (CR 702.147): `CardDef::decayed` and `Object::decayed` (a decayed counter, which counts for "a counter
  on it"). It can't block; when it's declared as an attacker it's marked and sacrificed as combat ends (the
  "sacrifice at end of combat" is set on declaration rather than by a trigger on the stack).
- `Ability::targets_exactly_x`: "each of X target creatures": stopping is allowed only at exactly X targets, it
  finishes at X, and X is offered only up to the number of legal targets. `Effect::DecayedCounter(slot)`.

Card: Rot-Curse Rakshasa.

Tarkir cards still not done (needs core):
- Naga Fleshcrafter: "enter as a copy of any creature on the battlefield" (clones), and "each other creature
  you control becomes a copy of that creature until end of turn".
- Nature's Rhythm: harmonize with {X} needs choosing X when casting from the graveyard; there's no action for
  that (`CastFlashback`/`CastHarmonize` carry no X), so it needs a new Action.
- Teval, Arbiter of Virtue: delve for every spell you cast (choosing graveyard cards to exile as part of the
  cost while casting).
- Taigam, Master Opportunist: suspend (time counters, upkeep removal, casting free when the last is removed),
  plus copying the second spell and exiling the original.
- Scavenger Regent: on hold (needs a new Action, per the coordinator).

## Bloomburrow Commander (BLC), lane C, batch 1 (11 cards)

Module `engine/bolt-cards/src/scripts/blc_c.rs` (after `blc_b` in `script()`), tests in
`engine/bolt-cards/tests/blc_c.rs`, checked by `every_bloomburrow_commander_script_matches_its_card_text`.
Range: `shared_order` 150-299 in ~/bolt-handoff/blc.json, skipping lane B's names.

Mechanics added (bolt-core, all at the end of their enum or struct): `Effect::BlinkSlot`,
`Effect::DrawPerCreatureYouControl`, `Effect::GainLifePerCreaturePowerAtLeast`, `Effect::CreateTokensX`,
`Effect::ReturnCreatureYouControl` (with `Decision::ReturnOwnCreature`), `Effect::BounceAllCreatures`,
`When::AnotherCreatureDies` (any player's, tokens included). Casting: an X spell's targets are checked for
each X (a "mana value X" target), and a spell with a kicked version is castable when either version has legal
targets, each way only with its own targets.

Cards: Toski, Bearer of Secrets, Tear Asunder, Ogre Slumlord, Tireless Provisioner, Starstorm, Restoration
Angel, Poison-Tip Archer, Shamanic Revelation, Stolen by the Fae, Time Wipe, Perplexing Test.

Not attempted (Tarkir, judged too large for the time left): Naga Fleshcrafter (clones) and Taigam (suspend).

## BLC 2026-11-25

Lane C's list in ~/bolt-handoff/blc_lanes_2026-11-25.json ("C"), worked top-down.

Batch 1 (11): Garruk's Packleader, Grim Backwoods, Talisman of Impulse, Thriving Bluff, Spine of Ish Sah,
Windgrace's Judgment, Psychosis Crawler, Deadly Dispute, Martial Coup, Intellectual Offering, Morbid Opportunist.
Core: `When::AnotherCreatureYouControlEntersWithPowerAtLeast`, `When::OtherCreaturesDieOnceEachTurn`,
`Effect::DestroyAllCreaturesIfXAtLeast` (Martial Coup destroys before making its tokens, so they survive),
`Effect::UntapAllNonland`, `ExtraCost::SacrificeArtifactOrCreature`, `CardDef::pt_hand_size`.

Skipped so far (reason):
- Academy Manufactor, Arthur, Cache Grab, Combat Celebrant (exert), Evercoat Ursine (hideaway), Fisher's and
  Fortune Teller's Talent (Class levels), Hazel's Brewmaster, Echoing Assault, Coveted Jewel (control change on
  unblocked attack): too large for this pass.
- Casualties of War: "choose one or more" with one target each; the engine fills target slots in order with no
  way to skip a slot, so choosing e.g. only the land mode isn't possible without a new Action.

Batch 2 (9): Illusory Ambusher, Body of Knowledge, Tetsuko Umezawa, Deep Forest Hermit, Nadier's Nightblade,
Junk Winder, Goreclaw, Grumgully, Chittering Witch.
Core: `When::DealtDamage` (amount on the trigger), `When::TokenYouControlLeaves`, `When::TokenYouControlEnters`,
`Effect::VanishingTick` (with `CardDef::vanishing`, using the object's `time_counters`, now cleared when it leaves
the battlefield), `Effect::TapFreeze` (with `Object::skip_untap`: "doesn't untap during its controller's next
untap step"), `Effect::PumpYoursPowerAtLeastAndGrant`, `Reduction::PerTokenYouControl`,
`CardDef::{small_creatures_unblockable, big_creature_spell_discount, other_non_subtype_enters_with_counter}`,
`Activated::sacrifice_creature` ("Sacrifice a creature", this one included). Chittering Witch's "number of
opponents" is 1 in the 1v1 game.

Batch 3 (9): Long River's Pull, Wear Down, Jolly Gerbils, Oran-Rief, Rishkar, Thickest in the Thicket,
Twenty-Toed Toad, Domri Anarch of Bolas, Luminous Broodmoth.
Gift: "Gift a card" is promised by casting the spell kicked for {0} (`kicker = Some(ManaCost::default())`); the
kicked version starts with `Effect::GiftCard` (the opponent draws, and `When::YouGiveGift` triggers).
Core: `When::{YouGiveGift, CreatureYouControlWithoutFlyingDies}`, `Condition::ControlGreatestPower`,
`Effect::{GiftCard, CountersOnGreenEnteredThisTurn, CountersEqualPower, WinIfCountersOrHandAtLeast, AddManaEither
(one pool mana that pays either color), CreatureSpellsUncounterableThisTurn, ReturnDiedWithFlyingCounter}`,
`CardDef::{counter_creatures_tap_for_green, max_hand_size}`, `Game::creatures_uncounterable_turn`.
Not done: Raging Ravine (becoming a creature has no way to set its colors to red and green).

Batch 4 (9): Beastmaster Ascension, Bident of Thassa, Boss's Chauffeur, Honored Dreyleader, Idol of Oblivion,
Second Harvest, Skyclave Apparition, Squirrel Nest, Chasm Skulker.
Core: `Keywords::ISLANDWALK` (bit 16; can't be blocked while the defender controls an Island),
`When::{CreatureYouControlAttacks, AnotherYouControlEntersWithAnySubtype}`, `StaticCondition::QuestCountersAtLeast`,
`Effect::{QuestCounterSelf, OpponentsAttackThisTurn, CreateTokensPerLastCounters, CountersSelfPerOtherWithSubtypes,
CopyEachTokenYouControl, ExileLinked, LinkedOwnerCreatesXx}`, `Filter::nontoken` (and `max_mana_value` now also
limits battlefield targets), `Activated::if_created_token_this_turn`, `CardDef::enters_counters_one_plus_others`,
`Object::quest`, `Game::{must_attack_turn, token_created_turn}`. A granted Aura ability with `tap_attached` is
offered only while the enchanted permanent can tap. Tokens (after lane B's Thopter and Storm Crow at 40-41):
Squid 42, Citizen 43, Eldrazi 44, Illusion 45 (Skyclave's X/X uses `copy_base_pt`).

Batch 5 (7): Tempt with Bunnies, Path of Discovery, Tendershoot Dryad, Wandertale Mentor, Trailtracker Scout,
Sword of the Squeak, Rapid Augmenter.
Core: `When::{AnotherCreatureYouControlEntersWithBasePower, AnotherCreatureYouControlEntersNotCast, Expend}` (the
entering creature fills a `triggering` first target, also for `AnotherYouControlEntersWithAnySubtype`),
`StaticCondition::CitysBlessing`, `Scale::PerYourCreatureBasePowerOrToughnessOne`, `Effect::{OpponentMay, Explore}`
(Yes to the May mills the revealed nonland), `CardDef::ascend`, `Game::{city_blessing, mana_spent_on_spells}`.
Expend counts the mana value of each spell's paid cost. Saproling token at 46.

Paused here (BLC) for the playgroup lists. BLC lane C not done: Academy Manufactor, Arthur, Cache Grab, Casualties
of War, Combat Celebrant, Coveted Jewel, Echoing Assault, Evercoat Ursine, Fisher's Talent, Fortune Teller's
Talent, Gilded Lotus ("three mana of any one color": the payer has no one-color constraint), Hazel's Brewmaster,
Kwain (both players choose), Maskwood Nexus, Mr. Foxglove, Perch Protection, Plumb the Forbidden, Promise of
Loyalty, Raging Ravine, Realm-Cloaked Giant (adventure), Rolling Hamsphere (crew), Rose Room Treasurer, Saw in
Half, Shield Broker, Simic Ascendancy, Storm of Souls, Swarmyard (regeneration), Tamiyo, Woe Strider (escape).

## Playgroup 2026-11-25: Glarb, Calamity's Augur (DonSpider), lane C

List: "C_glarb" in ~/bolt-handoff/pg_lanes_round0.json. Module `scripts/pg_c.rs`, tests `tests/pg_c.rs`.

PG batch 1 (17): Bloodstained Mire, Polluted Delta, Windswept Heath, Drowned Catacomb, Sunken Hollow, Undergrowth
Stadium (one opponent: always tapped), Undercity Sewers, Underground Mortuary, Hedge Maze, Zagoth Triome, Breeding
Pool, Overgrown Tomb, Watery Grave, Damnation, Exploration, Azusa, Aesi.
Core: shock lands, `CardDef::shock_land` with `Decision::ShockLand` (Yes: pay 2 life; No: tapped), asked from
`give_priority` via `Object::shock_pending`, so a fetched shock land asks too, before anyone can act.

PG batch 2 (17): Glarb, Oracle of Mul Daya, Traveling Chocobo, Crucible of Worlds, Icetill Explorer, Ancient
Greenwarden, Yarok, Seedborn Muse, Lumra, Ob Nixilis the Fallen, Doom Whisperer, Matoya, Lightning Greaves,
Assassin's Trophy, Cyclonic Rift, Drag to the Roots, Rite of Replication.
Core: `CardDef::{lands_from_graveyard, lands_from_top, top_spells_min_mana_value, top_spells_subtype,
entering_triggers_again (new enum EntryDoubler), untaps_on_others_untap, pt_lands}`, `Keywords::SHROUD` (bit 17),
`Reduction::Delirium`, `When::YouScryOrSurveil`, `Effect::{DestroyControllerMayFetchBasic, BounceOpponentsNonland,
CountersOnYourSubtype, ReturnLandsFromGraveyardTapped, InsectOrCopyWithSixLands}`. `zone_triggers` now wraps
`zone_triggers_once` to repeat an entering's triggers for each doubler. "Look at the top card any time" and "play
with the top card revealed" are information only. Overload is cast as a {5} kicker on Cyclonic Rift (the same
total cost); kicked plays the overloaded text.

PG batch 3 (8): Avenger of Zendikar, Scute Swarm, Wayward Swordtooth, Retreat to Coralhelm, Guardian Project,
Dryad of the Ilysian Grove, Clever Impersonator, Teval the Balanced Scale.
Core: `CardDef::{fights_only_with_blessing, lands_every_basic_type, enter_as_copy_nonland}`,
`LookTo::BattlefieldTapped`, `When::CardsLeaveYourGraveyard` (any turn), `Effect::{TapOrUntap, DrawIfUniqueName,
ReturnLandFromGraveyardTapped}`, `Game::graveyard_leave_any_pending`. A clone that copies a planeswalker takes its
loyalty. Tokens: Plant 51, green Insect 52.

PG batch 4 (4): Greensleeves, The Gitrog Monster, Colossal Grave-Reaver, Wrenn and Seven.
Core: `CardDef::protection_from_planeswalkers_and` (in `protected_from`), `When::{LandCardsToYourGraveyard,
CreatureCardsMilledToYourGraveyard}` (batched like the graveyard-leave triggers), `EmblemKind::NoMaxHandSize`,
`Decision::{SacrificeLandOrThis, LandsFromHand}`, `Effect::{SacrificeLandOrSource, MilledCreatureToBattlefield,
RevealTopLandsToHandRestGraveyard, LandsFromHandTapped, ReturnPermanentCardsToHand}`, `Object::milled_turn`,
`Game::{land_to_graveyard_pending, creatures_milled_pending}`. Grave-Reaver offers the creature cards milled this
turn, and taking one is optional (the engine's pick can't be forced without re-moving the rest). Tokens: Badger
54, Treefolk 55 (P/T = lands you control).

PG batch 5 (1): Nissa of Shadowed Boughs. Core: `LookTo::BattlefieldTwoCounters`,
`Effect::CreatureFromHandOrGraveyardMvAtMostLands`.

Glarb list: 47 of 65 scripted. Not done (each needs more than this session had): Jace, the Mind Sculptor;
Sensei's Divining Top and Scroll Rack (library ordering); Koma (a modal activated ability); Sakashima of a
Thousand Faces and Sakashima the Impostor (copies with exceptions); Phyrexian Metamorph ({U/P} mana); Scapeshift,
Rise of the Witch-king, Will of the Abzan (multi-player choices); Dread Return (flashback by sacrificing creatures);
Force of Vigor (alternative cost); Delighted Halfling (mana for legendary spells only); Conduit of Worlds; Hedge
Shredder (crew); Legolas's Quick Reflexes (split second); Nissa, Worldsoul Speaker (energy); Zimone (manifest
dread).

PG batch 6 (8): Delighted Halfling, Force of Vigor, Dread Return, Scapeshift, Rise of the Witch-king, Will of the
Abzan, Koma, Cosmos Serpent, Sensei's Divining Top.
Core: `CardDef::{any_color_for_legendary, flashback_sacrifice, pitch_not_your_turn, last_mode_needs_commander}`,
`Activated::sacrifice_another`, `Object::activations_stopped_turn` (in `activation_stopped`), `Decision::{FlashbackSacrifice,
SacrificeLandsThenSearch, SacrificeOneOf, SacrificeThenReturnOther, ReturnOtherPermanent}`, `Effect::{Scapeshift,
SacrificeGreatestPower, SacrificeThenReturnOther, TapStopActivations, LookTopReorder, DrawThenSourceOnTop}`.
Shortcuts: the Halfling's mana paying for any legendary spell counts as its restricted ability (always the better
choice), so the spell can't be countered. Dread Return's three sacrifices are picked right after the cast, before
anyone gets priority. Force of Vigor's pitch is `CastWith(card, green card)`, offered only off your turn. Will of the
Abzan's "both" is a third mode, legal only while you control a commander; "any number of target opponents" is the
one opponent. Koma's modal ability is two activated abilities (the mode is chosen on activation, CR 700.2).

PG batch 7 (2): Hedge Shredder, Nissa, Worldsoul Speaker.
Core: `Activated::crew` (offered as `ActivateWith(vehicle, n, creature)` for each untapped creature with enough power;
paying taps it), `Effect::{CrewedSelf, MilledLandsToBattlefieldTapped, Energy}`, `When::LandCardsMilledToYourGraveyard`,
`Game::lands_milled_pending`, `Player::energy` (at the end of `Player`), `CardDef::permanents_for_energy`. Crew taps one
creature (crew 1 never needs two). A crewed Vehicle is an artifact creature until end of turn (`eot_creature`).
Hedge Shredder returns the land cards milled this turn that are still in your graveyard. Nissa's energy cast is
`CastWith(permanent card, Nissa)`, mapped to an alternative cost of eight {E} (not for spells with a sacrifice or
discard cost of their own).

PG batch 8 (4): Jace, the Mind Sculptor, Scroll Rack, Conduit of Worlds, Legolas's Quick Reflexes. (Lane B takes
Phyrexian Metamorph, both Sakashimas and Zimone, which finishes Glarb.)
Core: `CardDef::split_second` (priority offers only `PassPriority` while one is on the stack; mana moves are added by
the caller), `Object::conduit_turn`, `Game::{granted_tapped, no_more_spells_turn}` (the latter in `cant_cast_now`),
`Decision::{HandToTop, ScrollRackExile}`, `Effect::{LookTopMayBottom, DrawThenPutBack, ExileLibraryShuffleHandIn,
ScrollRack, ConduitCast, GrantTappedTrigger, ExtraDealsDamageEqualPower}`.
Shortcuts: Jace's and Scroll Rack's "in any order" is the order picked (Jace: each card goes over the last;
Scroll Rack: `ScryOrder`). Conduit's "you may cast that card" is deferred: the card stays castable from the
graveyard for the rest of the turn while you haven't cast a spell, and casting it locks out further spells. The
granted "becomes tapped" trigger is Legolas's own `When::Never` trigger, with the tapped creature as the item's
`extra`, like the granted combat-damage triggers.

## Playgroup 2026-11-25: Aesi, Tyrant of Gyre Strait (Time), lane C

List: "C" in ~/bolt-handoff/pg_lanes_aesi.json (24 cards). Same module and tests as Glarb.

Aesi batch 1 (9): Swan Song, Biorhythm, Sunder, Ghost Town, Palinchron, Broken Bond, Collector Ouphe, Mox Amber,
Voidslime.
Core: `CardDef::{stops_artifact_abilities (in activation_stopped, so artifact mana too), mana_legendary_colors}`,
`Activated::not_your_turn_only`, `Filter::spells_enchantment_instant_sorcery`, `Decision::LandFromHand`,
`Effect::{CounterSpellOrAbility, LifeBecomesCreatureCount, BounceAllLands, UntapLands, MayLandFromHand}`. Token: Bird
2/2 flying at base+56. Palinchron's "untap up to seven lands" untaps up to seven of your own tapped lands. Biorhythm
sets each life total by gaining or losing the difference. Voidslime removes a countered ability from the stack; a
countered spell copy just ceases to exist.

Aesi batch 2 (9): Awaken the Woods, Sylvan Advocate, Omnath, Locus of Mana, Cyclone Summoner, Roil Elemental, Yavimaya
Dryad, Otawara, Soaring City, Scatter to the Winds, Three Steps Ahead.
Core: `Keywords::FORESTWALK` (bit 18), `CardDef::{keeps_green_mana, mode_costs}`, `Activated::less_per_legendary_creature`,
`Scope::YourLandCreatures`, `StaticCondition::ControlLandsAtLeast`, `Condition::CastFromHand`,
`SearchTo::BattlefieldTappedUnder`, `Game::control_while` (checked in state-based actions),
`Effect::{BounceAllExceptLandsAndSubtypes, GainControlWhileSource, SearchForestForPlayer, Awaken}`. Token: Forest Dryad
(1/1 land creature, a Forest) at base+57.
Shortcuts: awaken is cast as a {3} kicker (the same total as the awaken cost), like overload; the awakened land is a
creature for good (`eot_creature` with turn `u32::MAX`, cleared when it leaves). Spree is one mode per combination of
+ modes (seven for Three Steps Ahead), each with its summed additional cost in `mode_costs`. Omnath keeps only
single-green pool entries; its bonus counts them.

Aesi batch 3 (5): Plaza of Heroes, Bear Umbra, Shifting Woodland, Khalni Ambush // Khalni Territory, Nissa, Vastwood
Seer // Nissa, Sage Animist.
Core: `CardDef::{plaza_mana, umbra_armor, modal_back}`, `Activated::needs_delirium` (`Game::graveyard_card_types`),
`Filter::legendary_only`, `Condition::ControlLandsAtLeast`, `Effect::{BecomeCopyOfSlotUntilEot (the existing
copy_until_eot, undone in cleanup), SlotBecomesCreatureForGood}`, `Game::use_umbra_armor` (in `destroy`,
`destroy_no_regen`, and state-based lethal damage). Token: Ashaya, the Awoken World (legendary 4/4 Elemental) at
base+58.
Modal double-faced cards: `modal_back` with a land back offers `PlayLand` from hand and sets `transformed` as it
enters (the front is still castable). A loyalty ability on the stack whose planeswalker left (−7 at 7 loyalty) and
turned front face up resolves from the back face's abilities.
Assets: Khalni Ambush and Nissa, Vastwood Seer refreshed with `fetch_oracle.py --set PG --refresh` (run with
PYTHONPATH=src:scripts so it writes this worktree's assets; the venv's `bolt` points at ~/bolt-the-bird), then
`make_cmd_set.py`. Only their `back` keys changed.
Bear Umbra's attack trigger is `When::EquippedAttacks`, which fires for anything attached (Auras too).

Merged tarkir 00f0db4 (lane B's Glarb clones and Zimone) after Aesi batch 3: end-of-block conflicts, tarkir's side
first; tests/pg_c.rs is tarkir's file with lane C's tests appended (lane B's `lb_` block comes first).

Aesi batch 4 (1): Deadeye Navigator (verified after the WIP commit c672eb1; it passed unchanged). Soulbond as `CardDef::soulbond_flicker`, `Object::paired_with`
(`Game::paired_partner` checks the pairing still holds), `Decision::SoulbondPair`, `Effect::{SoulbondPairOnEnter,
SoulbondPairWith}`; the granted "{1}{U}: Exile this creature, then return it" comes after kept abilities and any
granted fight ability in `activated_of` and the activation list. Aesi lane C: 24 of 24.

## Playgroup 2026-11-25: Etali, Primal Conqueror (RuricMo), lane C (round 2)

List: the Etali deck's cards in ~/bolt-handoff/playgroup_all.json card_order that set_support for CMD shows as "no"
(57). Etali's back face refreshed first (`fetch_oracle.py --set PG --refresh`, then `make_cmd_set.py`).

Etali batch 1 (9): Green Sun's Zenith, Bane of Progress, Silverback Elder, Temple of the False God, Circle of Dreams
Druid, Tribute to the World Tree, Parallel Lives, Spire Garden, Worldspine Wurm.
Core: `CardDef::{mana_per_creature, mana_only_with_lands, shuffle_on_resolve}`, `SearchFor::{GreenCreatureMvAtMostX,
ColoredCreatureMvAtMost}`, `When::PutIntoGraveyardFromAnywhere` (fired from `move_to` for the card itself),
`Effect::{DestroyArtifactsEnchantmentsCountersSelf, DrawIfPowerThreeElseTwoCounters}`. Token: Wurm 5/5 trample at
base+59.

Merged tarkir 8e8df2a (desktop lanes setup) after Etali batch 1: no conflicts.

Etali batch 2 (7): Ulamog, the Infinite Gyre, Lurking Predators, Ulamog, the Defiler, Homeward Path, Purphoros,
Conjurer's Closet, Cavern-Hoard Dragon.
Core: `CardDef::{ward_sacrifice, enters_counters_greatest_exiled_mv}`, `Reduction::GreatestArtifactsAnOpponentControls`,
`StackKind::WardSacrifice` (with `Decision::PayWardSacrifice`), `Decision::SacrificePermanents`,
`Effect::{SacrificePermanents (annihilator N), AnnihilatorPerCounters, OwnerShufflesGraveyardIntoLibrary,
RevealTopCreatureToBattlefieldElseMayBottom, CreaturesToOwners, ExileTopHalfOfLibrary, TreasuresPerOpponentArtifact}`,
`Game::sacrifice_permanents`. Annihilator is the attack trigger "defending player sacrifices N" (one opponent).

Merged origin/tarkir a48163c (queue files, bolt.jobs) as 17f7972. From here on Etali's list is its "C" entry in
handoff/lanes/queue.json (cards listed under an earlier deck belong to that deck's lane).

Etali batch 3 (7): Displaced Dinosaurs, Vorinclex, Voice of Hunger, Blightsteel Colossus, Etali, Primal Conqueror //
Etali, Primal Sickness, Sneak Attack, Terror of the Peaks, Xenagos, the Reveler.
Core: `CardDef::{infect (in has_infect), historic_become_dinosaurs (applied as a permanent enters), vorinclex,
targeted_life_tax}`, `Game::land_tapped_for_mana` (Vorinclex's freeze, in both mana payment paths but not the
two-mana Signet plan), `Decision::{SneakAttack, ExiledToBattlefield}`, `Effect::{EachExileUntilNonlandCastFree,
PoisonAmountToOpponent, SneakAttack, AddRedOrGreenPerCreature, ExileTopPutCreaturesAndLands}`. Token: Satyr 2/2 haste
at base+60.
Shortcuts: Vorinclex's mana trigger doubles the land's mana (a mana ability's trigger resolves at once). Etali's
{9}{G/P} is two abilities ({9}{G}, or {9} and 2 life). Terror of the Peaks' 3 life is paid as the spell is cast; it
doesn't stop a player with less life from casting it. Xenagos's {R}/{G} mana is pool entries that can be either.

Etali batch 4 (12): Fauna Shaman, Ghalta, Maze of Ith, Ohran Frostfang, Temur Sabertooth, The Fire Crystal,
Cultivator Colossus, Terrain Generator, The Reaver Cleaver, Apex Devastator, Orthion, Sarkhan's Unsealing.
Core: `CardDef::red_spell_discount`, `Activated::discard_creature_card`, `Condition::CastPowerBetween` (checked as a
"you cast a creature spell" trigger triggers; those triggers now carry the spell as `extra`), `Game::combat_damage_by_prevented`,
`Decision::{CreaturesFromHand, ReturnAnotherForIndestructible, LandTappedDrawRepeat, BasicFromHandTapped}`,
`Effect::{CreaturesFromHand, PreventCombatDamageToAndBy, MayReturnAnotherThenIndestructible,
LandFromHandTappedDrawRepeat, BasicLandFromHandTapped, CopiesHasteSacrifice, UnsealingDamage}`. "When you cast this
spell" triggers carry the spell as `extra` (so cascade works from them); equipment's combat-damage triggers get the
damage as their amount.
Shortcuts: Maze of Ith targets a creature in combat (attacking or blocking). The Reaver Cleaver's trigger covers
combat damage to a player (planeswalkers aren't attacked for it here).

Merged origin/tarkir 4b9b8ed (lane B's Aesi; the play network's new action kinds) after Etali batch 4: end-of-block
conflicts in card.rs, effect.rs, game.rs and lib.rs, tarkir's side first.

Etali batch 5 (10): Hellkite Courser, Protean Hulk, Selvala, Vigor, Molten Echoes, Primal Surge, Flameshadow
Conjuring, Mirror March, Rionya, Twinflame.
Core: `CardDef::damage_to_yours_becomes_counters` (in `deal_damage`), `Object::{exile_at_end, command_at_end}` (handled
with the end-step sacrifices), `Decision::{HulkSearch, PrimalSurge, CommanderCourser}`, `Effect::{CopiesHasteExileAtEnd,
CopyIfChosenTypeExileAtEnd, CoinFlipCopiesExileAtEnd, CommanderFromCommandZoneUntilEnd, SearchCreaturesTotalManaValue,
DrawIfGreatestPower, AddAnyColorsGreatestPower, PrimalSurge}`, `Game::{primal_surge, copies_exile_at_end}`.
Shortcuts: Selvala's "may draw" always draws. Twinflame's strive is one mode per number of targets (1 to 5), each
with its added cost in `mode_costs`. Ezuri's Predation waits for token slot 63 (after lane B's 61 and 62).

Etali batch 6 (5): Mosswort Bridge, Spinerock Knoll, Cursed Mirror, Wand of Wonder, Vivien, Monsters' Advocate.
Core: `CardDef::{copy_until_eot_haste (with enter_as_copy), top_spells_creatures}`, `Object::hideaway`,
`Game::{damage_to_player_turn, next_creature_spell}`, `Decision::{HideawayPick, KeywordCounterChoice}`,
`Effect::{Hideaway, PlayHideaway, WandOfWonder, CreateTokenWithKeywordCounterChoice, NextCreatureSpellThisTurn,
SearchCreatureLesserManaValue}`. A hideaway card is played through `playable` and `cast_free` for the turn (a land
uses a land drop). Wand of Wonder: one opponent exiles one instant or sorcery, so X only caps it at one.

Etali batch 7 (4): Delina, Wild Mage, Jaxis, the Troublemaker, Zopandrel, Hunger Dominus, Mirage Phalanx.
Core: `CardDef::{blitz (with alt_cost), soulbond_combat_copies}`, `Activated::{discard_any_card, sacrifice_two_others}`,
`Object::{exile_end_combat, lost_soulbond}`, `Decision::DelinaAgain`, `Effect::{DelinaRoll, CopyHasteDiesDrawSacrifice,
KeywordCounterSelf, PhalanxCopies}`, `Game::{token_copy_of, delina_roll}`.
Shortcuts: Zopandrel's second sacrifice is the engine's pick (your other creature with the least power); its
{G/P}{G/P} is three abilities ({G}{G}; {G} and 2 life; 4 life). Mirage Phalanx's copy of each paired creature comes
from Phalanx's own trigger (one trigger for both).

Merged tarkir 62051fe (lane B's Myrkul batch 1, desktop d3, core themes): end-of-block conflicts, tarkir's side first.
With it: Ezuri's Predation (`Effect::TokenFightsEachOpponentCreature`; token Phyrexian Beast 4/4 at base+63, after lane
B's 61 and 62). Etali (queue list) complete.

## Core theme: objects and zones (mac-c), batch 1: Clues, Food costs, Adventures

Fast-forwarded to tarkir-b 4c78d77 (lane B's Myrkul 40/45; its Shapeshifter is token 64). Tests: tests/core_objects.rs.
Core: token Clue at base+65 (`scripts::clue`), `Effect::{Investigate, PumpPerYourSubtype, MillPutCreatureEnchantmentOrLand,
MillReturnCreatureToHand, MayPutCreatureFromHand}`, `When::YouInvestigateFirstTimeEachTurn`, `Game::investigated_turn`,
`Activated::sacrifice_subtype_count` ("Sacrifice four Foods": the victim, then the engine picks the rest),
`CardDef::{adventure, enchanted_stops_activations}`, `Decision::{PickOneOf, MayCreatureFromHand}`.
Adventures reuse the Omen path: the spell half is cast with `CastOmen` from the "Omen: " lines and the oracle's
`omen`; `adventure` makes it go to exile, castable by its owner from there, instead of shuffling in.
Cards (12): Confirm Suspicions, Confront the Unknown, Erdwal Illuminator, Ethereal Investigator, Floodhound, Hold for
Questioning, Ongoing Investigation, Tamiyo's Journal, Pippin, Savvy Hunter, Bramble Familiar, Monster Manual.

Objects and zones batch 2: reanimating Auras (Animate Dead, Dance of the Dead, Necromancy). Fast-forwarded to tarkir
e245c6b first. Core: `CardDef::{reanimating_aura, reanimates_tapped, flash_cast_sacrificed}`, `Effect::{ReanimateAttachSource,
UntapAttached}`, `Game::{reanimate_attach, flash_casts}`. The Aura's enters trigger is done as it enters: an Aura spell
with `reanimating_aura` brings its graveyard target back under your control and attaches to it (the state-based
check accepts a creature for it); when it leaves the battlefield, `move_to` sacrifices that creature (computed
before lane B's guarded replacement block, applied after the zone change). Necromancy cast when a sorcery couldn't
be is sacrificed at the next end step (the cleanup step here).

Objects and zones batch 3: phasing (Teferi's Protection, Clever Concealment). Fast-forwarded to tarkir 476ec4c.
Core: `Effect::{PhaseOut, LifeLockedUntilYourNextTurn}`, `Game::{phased_out, life_locked_until, phase_out, life_locked}`.
A phased-out permanent leaves the `battlefield` list but keeps Zone::Battlefield and its zcc (no zone change, no
triggers); what's attached phases out with it; it phases in at the start of its controller's untap step (before
Savor the Moment's skip). Teferi's Protection: life can't change until your next turn (lose_life and gain_life),
which also covers damage; d3's protection_grants hold only colors for an object this turn, so "protection from
everything" against targeting waits for a player-level grant there.

Objects and zones batch 4: Tireless Tracker, Academy Manufactor, Wavesifter (on lane B's 4c65873). Core:
`CardDef::clue_food_treasure` (Academy Manufactor, in `create_token` before the "those tokens plus" extras).
Tireless Tracker uses d2's `When::YouSacrificeSubtype("Clue")`; the "sacrifice N of a subtype" cost now reports each
sacrifice through `Game::sacrificed` (a destroyed Clue doesn't count). Wavesifter uses d1's evoke.

Damia, Sage of Stone (Grande): its last three, All Suns' Dawn, Chain of Smog, Kira. Core: `CardDef::kira_trigger`,
`Object::kira_targeted_turn` (checked in `ward_check`: the first time each turn a creature of a Kira controller is
targeted, Kira's trigger counters that spell or ability via `Effect::CounterExtra`), `Effect::{OneCardOfEachColorToHand,
ChainCopy}`, `Decision::{CardOfColorToHand, ChainCopy}`. All Suns' Dawn picks its cards as it resolves (one per color);
Chain of Smog's copy is a `SpellCopy` controlled by the target player with the player they choose as its target.

Objects and zones batch 5: populate (Trostani, Selesnya's Voice) and imprint/copy-cast (Isochron Scepter). Core:
`Effect::{Populate, GainLifeEqualSlotToughness, ImprintInstantFromHand, CastImprintedCopy}`,
`Decision::{Populate, ImprintInstant}`, `Game::populate_copy`. The imprinted card is linked through
`Object::hideaway`; a copy is cast through the free-cast window (`card_copy`, `cast_free`).

Objects and zones batch 6: sacrifice X Clues (Lonis, Cryptozoologist) and copy exceptions (Vaultborn Tyrant,
Phantasmal Image, Mockingbird, Glasspool Mimic // Glasspool Shore). Lonis's "Sacrifice X Clues" uses lane B's `Activated::sacrifice_x_of_subtype` (Chatterfang; it arrived in
the same merge, so lane C's own copy of the piece was dropped). Core: `CardDef::zones` (`ZonesExt`, the Mac C group: `copy_adds_subtypes`,
`copy_adds_keywords`, `copy_sacrifice_when_targeted`, `copy_mv_at_most_spent`), `Object::{mana_spent,
sacrifice_when_targeted}`, `StackKind::SacrificeWhenTargeted { zcc }` (pushed from `ward_check`),
`Decision::PutFromOpponentsTop`, `Effect::{OpponentRevealsTopXPutNonlandPermanent, SacrificeSourceWhenTargeted,
TokenCopyOfSourceArtifactIfNontoken}`. The copy exceptions are applied as the clone becomes a copy
(`Decision::EnterAsCopy`): extra creature types in `extra_subtypes`, keywords in `granted`.
Shortcuts: Vaultborn's token copy becomes an artifact just after it enters (a `TypeChange`), so "artifact
enters" triggers don't see it; its "this creature or another creature with power 4 or greater" trigger is
`YourCreatureEntersWithPowerAtLeast(4)` (Vaultborn itself is 6/6).
Glasspool Shore was already in the oracle data (no refetch needed).

Objects and zones batch 7: morph (Aphetto Alchemist, Rattleclaw Mystic), back faces and their pieces (Sundering
Eruption // Volcanic Fissure, Puppeteer Clique, Railway Brawler). Lane B's face-down work already covered casting
face down for {3} (`CastAlternative`) and turning face up for the morph cost (`Activate(id, 0)`, a special
action); morph needed only scripts, plus the trigger. Core: `When::TurnedFaceUp` (pushed by `Game::turned_face_up`
from the special action and from `Decision::TurnFaceUp`), `Activated::plot` and `Object::plotted` (plot: activated
from hand at sorcery speed, the card exiled as the cost and marked `cast_free`; castable from exile by its owner
at sorcery timing on a later turn), `ZonesExt::shock_life` (Volcanic Fissure's 3 life, in the `ShockLand`
answer), `Effect::{DestroyControllerMayFetchBasicTapped, CreaturesWithoutFlyingCantBlockThisTurn,
ReanimateHasteExileAtEnd}`. Puppeteer Clique's persist is d1's `Effect::Persist`.
Shortcuts: plot uses the stack (an ability with no effect) instead of being a special action. "Creatures without
flying can't block this turn" applies to the creatures on the battlefield as it resolves. Puppeteer Clique's
creature is exiled at the next end step (the same as "your next end step" unless it entered on an opponent's
turn).
Touched owned code (flag for review): the `ShockLand` life amount (d1's land entry), the plot exile-cast check
in the castable-from-exile filter (merger's timing), and the hand condition of the activation listing.

## Malik, Grim Manipulator (queue.json, lane C)

Unsupported list from set_support (run from the batch 7 build's binary): 52 of 54.
Batch 1 (17, no new core): Diabolic Tutor, Sundown Pass, Shattered Sanctum, Inspiring Vantage, Smoldering Marsh,
Blightstep Pathway // Searstep Pathway, Needleverge Pathway // Pillarverge Pathway, Merciless Executioner, Massacre
Wurm, Butcher of Malakir, Reckless Fireweaver, Mayhem Devil, Teleportation Circle, Crackling Doom, Charming Prince,
Ephemerate, Treasure Vault.
Shortcuts: Massacre Wurm's "that player" is the one opponent. Mayhem Devil uses `When::PlayerSacrificesAnother`,
so sacrificing Mayhem Devil itself doesn't trigger it. Charming Prince's "another target creature you own" is one
you control.
Malik batch 2 (objects and zones): Nahiri's Resolve, Treasure Map // Treasure Cove. Core: `Effect::{ExileYoursUntilYourNextUpkeep,
LandmarkTransformTreasures}`, `Decision::ExileUntilYourNextUpkeep` (Pick one at a time, No to stop; labelled),
`Object::return_at_upkeep` (returned at the start of that player's upkeep step, before upkeep triggers). Landmark
counters are the source's `kind_counters`; transforming is in place (`transformed`, same object).
Still Malik, not done: Prosper, Tome-Bound (needs d2's "play a card from exile" trigger), Glimpse of Tomorrow
(suspend, a later lane C batch), Gisa, Glorious Resurrector (Mac B's die-to-exile replacement first), Inventors'
Fair (d3: a search for any artifact card).

## Bruna, Light of Alabaster (queue.json, lane C)

Unsupported: 46 of 48. Batch 1 (11): Supreme Verdict, Idyllic Tutor, Talisman of Progress, Deserted Beach, Prismatic
Vista, Dovin's Veto, Eel Umbra, Crashing Drawbridge, Heliod's Pilgrim, Galadriel's Dismissal, Mirrormade.
Core: `Effect::{PhaseOutSlot, PhaseOutCreaturesOfPlayer}` (they phase in on their controller's untap step, unlike
`PhaseOut`, which uses the caster's), `ZonesExt::copy_artifact_or_enchantment` (in `copy_option`; Auras excluded).

## Olivia, Crimson Bride (queue.json, lane C)

Unsupported: 41 of 41. Batch 1 (9, no new core): Rakdos Signet, Wayfarer's Bauble, Faithless Looting, Tormenting
Voice, Electric Revelation, Seize the Spoils, Unexpected Windfall, Cathartic Pyre, Foreboding Ruins.

## Vaevictis Asmadi, the Dire (queue.json, lane C)

Unsupported: 22 of 22. Batch 1 (2): Tranquility, Vesuva. Core: `ZonesExt::enters_as_land_copy_tapped` and
`Object::land_copy_pending`: set as the land enters (however it enters), asked in `give_priority` right after shock
lands as the existing `Decision::EnterAsCopy` (any land; a copy enters tapped, `No` leaves it untapped as itself).
Shortcut: a Vesuva copying a shock land isn't asked for the 2 life (it's already tapped).

Olivia batch 2: Charming Scoundrel, Witch's Mark. Token 97 is the Wicked Role (`Tokens::wicked_role`, an Aura token with
the Role subtype), with placeholders at 91-96 for d2's and d4's tokens. Core: `Effect::{CreateRoleAttached,
MayDiscardToDrawN}` (one Role of yours per creature: the older goes to the graveyard, CR 303.7) and
`When::ThisPutIntoGraveyardFromBattlefield` (one `fire_source` line in `move_to`, next to
`PutIntoGraveyardFromAnywhere`; tokens included).

## Objects and zones batch 8: Auras put onto the battlefield attached (Bruna, Light-Paws, Sovereigns)

Core in a new file, `engine/bolt-core/src/game/zones.rs` (`mod zones;` next to `mod d4;`); game.rs gets only
dispatch arms at the ends of `run_effect`, `decision_actions` and `decide`.
- `Game::put_aura_attached(aura, to, zcc, p)` (pub): an Aura card enters attached without being cast (CR 303.4f);
  if it couldn't enchant `to`, it stays where it was. `Game::could_enchant(aura, x, p)`: the Aura's enchant
  filter (its spell's first target) through `target_ok` with `triggering` set (no shroud or hexproof check, as it
  isn't targeted) and `protected_from`.
- `Decision::AttachAura { to, zcc, library, max_mv, new_name }` (Pick or No; labelled): from the library one card,
  then shuffle (Light-Paws, Sovereigns); otherwise asked again after each, over Auras on the battlefield (moved
  onto it) and Aura cards in your graveyard and hand (Bruna).
- `Effect::{AttachAurasToSource, SearchAuraForSourceIfCast(slot), SearchAuraForLoneAttacker}`.
- "If you cast it" is the existing `Object::was_cast` (set as a spell resolves onto the battlefield, cleared by
  `move_to` as anything enters), read on the triggering Aura.
- `When::CastAuraYouControlEnters`: `When::SubtypeYouControlEnters` fires only for entering creatures, so Light-Paws
  triggers from one call (`Game::cast_aura_entered`) in the Aura-spell block of the resolution path, after it's
  attached; the trigger itself is therefore "if you cast it".
Shortcuts: Light-Paws fizzles if the Aura has left before its trigger resolves. Sovereigns is a
`When::YouAttack` trigger with a lone-attacker check on resolution (as d1's exalted), so it doesn't wait on d2.

## Objects and zones batch 9: Olivia, Dragonhawk, Glimpse of Tomorrow (suspend); Moria Scavenger, Professional Face-Breaker

- `Effect::ReturnAttackingUntilNoLegendaryVampire(slot)` with `Object::exile_without_legendary_vampire` (a zcc):
  checked at the top of `give_priority` (`Game::exile_without_legendary_vampire`, right after state-based
  actions). Shortcut: the state trigger exiles at once instead of using the stack.
- `Effect::{ExileTopPowerFourPlayableUntilNextEndStep { trigger }, DamageEachOpponentAmount}` and
  `Game::delayed_end_step_exiled`: at the beginning of that end step (one call in `Step::End`, before the end-step
  triggers fire) the source's `When::Never` trigger goes on the stack with 2 per card still exiled as its amount.
  The cards are playable through that turn (`playable` until its number), so also during its end step.
- Suspend: an `Activated::plot` ability (exiled from hand as the cost, sorcery speed) whose effect
  `Effect::SuspendSource(n)` swaps plotted for n time counters; the existing upkeep code removes them and casts it
  free. Uses the stack (like plot), rather than being a special action. `ZonesExt::no_mana_cost`: `cast_modes`
  offers nothing unless `cast_free` (CR 202.1b), one guard line at its top.
- `Effect::GlimpseOfTomorrow` with `Decision::AuraEntersAttachedTo { aura, auras, rest }` (Pick the host; labelled).
  Tokens and card copies shuffled in cease to exist; "that many" counts them. Enchant-player Auras and Animate
  Dead-style Auras find no host and go to the bottom.
- `Effect::AmassIfExtraCreatureCard` (Moria Scavenger; the discarded card is the item's `extra`). Professional
  Face-Breaker needed nothing new.
Touched code owned elsewhere (flag for review): the `give_priority` line, the `Step::End` line, the `cast_modes` guard.

## Objects and zones batch 10 (Bruna's Auras), unbuilt

Steel of the Godhead (d3's `StaticCondition::AttachedIsColor`, no new core), Face of Divinity
(`StaticCondition::AttachedHasAnotherAura`, an arm appended to the static-condition match), Shielded by Faith (the
existing `Effect::Attach` from the two "a creature enters" triggers, as optional triggers), Eldrazi Conscription
(+10/+10 trample and annihilator 2 as a `When::EquippedAttacks` trigger on the Aura, which fires for Auras too).
Still needing core in Bruna: Daybreak Coronet (a target filter "creature with another Aura attached": d3's
`Filter`), Super State (base P/T 9/9 from an Aura without losing abilities: `attached_static` treats any
`base_pt` as Witness Protection's Citizen; d3), Celestial Mantle (an effect doubling a life total: d3), All That
Glitters, Ethereal Armor, Auramancer's Guise, Sage's Reverie (counts in the scaled-statics block: d3), Unquestioned
Authority (protection from creatures: d3), Open the Armory (a search for an Aura or Equipment card: d3).

## Objects and zones batch 11 (unbuilt): Gisa, then Tergrid and Vohar

- Gisa, Glorious Resurrector (built, verified at 9dab6d8): lane B's `opponents_creatures_die_exiled` and
  `Game::exiled_with`; `Effect::ReturnExiledWithSourceDecayed` (`Game::return_exiled_with_decayed`).
- Casting a nonland modal back face: `Action::CastModal(c, 1)` for a modal double-faced card in hand whose front
  has no modes (`Game::back_face_castable` lists it: a nonland permanent back, not an Aura, no {X}, at its
  timing, affordable). The apply arm sets `transformed` and casts it normally, so the spell is the back face;
  `move_to` clears `transformed` when it leaves the stack for anywhere but the battlefield. The service view
  labels it "Cast <back name>". Not covered: instant or sorcery backs, Aura backs, X backs, backs with targets
  beyond what `cast` checks.
- Tergrid, God of Fright // Tergrid's Lantern: d2's `OpponentSacrificesNontoken` and `OpponentDiscards` as optional
  triggers with `Reanimate`; the Lantern's `Effect::TargetPlayerLosesUnlessSacrificesOrDiscards` (the existing
  Torment decision, once) and an untap ability.
- Vohar, Vodalian Desecrator: `Effect::DiscardThenIfInstantOrSorcery` with `Decision::DiscardThenIfInstantOrSorcery`
  (labelled), and the sacrifice ability as `GrantFlashback` (Snapcaster's grant: its mana cost, exiled instead).

## Script-only pass: Olivia and Bruna (2026-11-26), batch 1 (unbuilt)

Lavaspur Boots (lane B's `enchanted_ward`), Mother of Runes (d3's `ProtectionChosenColorEot`), Port Razer (the
existing `AdditionalCombatAfterThisPhase` and `cant_attack_twice_per_turn`), Breath of Fury (new
`Effect::SacrificeEnchantedMoveAura` with `Decision::MoveAuraTo`, labelled: the Aura moves before state-based
actions can bin it; with nothing to move to, "If you do" fails).
Still waiting, by owner:
- d1: Mystic Gate (filter mana), Bitter Triumph, Cathartic Reunion, Demand Answers, Sazacap's Brew (additional
  costs), Hoarding Broodlord, Neheb, the Eternal, Ozai, Heliod's back (cost reduction per card drawn).
- d2: Mesmeric Orb (any permanent untaps), Deep Gnome Terramancer (lands enter for an opponent without being
  played), Archaeomancer's Map (an opponent's land enters), Lavinia (an opponent casts a spell with no mana
  spent), Necropolis Regent (a creature-and-amount combat damage trigger), Azra Oddsmaker, Twilight Diviner,
  Olivia's Attendants (deals-damage amount, and a Blood token).
- d3: Fabled Passage, All That Glitters, Ethereal Armor, Auramancer's Guise, Sage's Reverie, Celestial Mantle,
  Daybreak Coronet, Unquestioned Authority, Giver of Runes, Pippin, Open the Armory, Consuming Tide, Single
  Combat, Gifts Ungiven, Super State, Heliod's front; Exhume, Living Death, Burning Inquiry, Change of Fortune,
  Dragon Mage, Burning-Rune Demon, Rakdos Charm, Vile Mutilator, Twilight Prophet.
- d4: Invasion of Theros (battles), Summon: Bahamut (a Saga creature).
- lane B: Rankle and Torbran ("choose any number" modes, the +2 damage replacement).

## Lane B's decks, Mac C pieces, batch 1 (unbuilt; lane C scripts these, lane B skips them)

Kaalia of the Vast (`Effect::CreatureFromHandAttacking(subtypes)` with `Decision::CreatureFromHandAttacking`,
labelled; not when Kaalia attacks a planeswalker), Command Beacon (`Effect::CommanderFromCommandZoneToHand`; `move_to`
no longer sends a commander moving from the command zone to its hand back to the command zone), Mystic Forge
(`ZonesExt::top_spells_artifact_or_colorless`, one clause in the top-of-library cast check; `Effect::ExileTopCards`).
Next in this list: Lithoform Engine, Thespian's Stage and The Mycosynth Gardens (copies), then Wandering Archaic.
Waiting on others: Tendo Ice Bridge and Cavern of Souls (d1 mana), Kozilek, the Great Distortion (d1 cost, d3 draw),
Flayer of Loyalties (d3: threaten with base P/T and granted annihilator), Rakdos, Patron of Chaos (d3), The One Ring
(d3: player protection from everything), Prosper (d2: "whenever you play a card from exile").

## Held back (lane C), with what each needs

Done since: Light-Paws, Sovereigns, Bruna, Olivia, Dragonhawk, Glimpse of Tomorrow (batches 8-9). Routed
(2026-11-26): Sheltered by Ghosts (Aura-granted ward: lane B, merger work), Smaug the Magnificent (d3), Rankle and
Torbran and Raphael (lane B), Pippin, Guard of the Citadel (d3: a per-object chosen-card-type protection grant, and a
card-type choice askable mid-resolution). Waiting on others: Gisa (Mac B), Prosper (d2), Inventors' Fair (d3).

## Lane B's decks, Mac C pieces, batch 2: copies (unbuilt)

Built first: the batch 1 WIP plus the tarkir 0834aa0 merge, verified at d72d923 (pg_c 208, workspace 2118 passed).
- Lithoform Engine (listed): `Effect::CopyTargetAbilityYouControl(slot)` (targets with `Filter { abilities, spells:
  false }`; the topmost ability of that source is copied if you control it) and `Effect::CopyTargetSpellYouControl
  { slot, permanent }` (instant or sorcery by `spell_instant_or_sorcery`; a permanent spell's copy resolves into a
  token by the existing CR 707.10f code). Both go through `pending_copies`; its loop now also lets an ability's
  copy choose new targets (unless a slot is `triggering`). Every existing pending copy is a spell copy, so their
  behaviour is unchanged.
  Shortcuts, both waiting on d3's `Filter`: "an ability you control" and "a permanent spell" are checked as the
  ability resolves, not as it targets, so an opponent's ability or an instant can be targeted and nothing happens.
  Needs d3: a `Filter` for "permanent spell" and a controller check for `abilities`.
- Thespian's Stage (listed): `Effect::SourceBecomesCopyOfSlot { slot, keep_own }`: `become_copy` for good, with
  `copy_exceptions = keep_own`. "Except it has this ability" is the copy ability twice: once printed, once
  `copy_only` (kept by `copy_keeps_abilities`, as Sakashima's).
- The Mycosynth Gardens: scripted with the same effect (`keep_own: false`); listed by lane B at 496558d with its paid mana. Before that it was not listed: "{1}, {T}: Add one mana
  of any color." needs d1 (a mana ability with a mana cost). It also needs the merger: the activation listing's
  `trigger_can_target` pre-check uses X = 0, so a `mana_value_is_x` target rejects every X and the ability is never
  offered. The merger fixed that on tarkir (X-dependent targets are checked for each X), so its test is back.
- Wandering Archaic // Explore the Vastlands: not scripted. The copy part needs no new Mac C core:
  `may(d, When::OpponentCastsInstantOrSorcery, vec![], vec![Effect::UnlessOpponentPays(2), Effect::CopySpellNewTargets])`,
  as Rhystic Study does. But it needs d2 to set the cast spell as `extra` on `OpponentCastsInstantOrSorcery`
  triggers (that `fire` passes None, and `CopySpellNewTargets` reads `extra`). The back face needs a sorcery
  modal back cast (`back_face_castable` covers permanent backs only; Mac C) and an "Each player looks at the top
  five..." effect (d3).
Tests: pg_c 212 (4 new), core_objects 39 at 49d2821 (Rogue's Passage; the Mycosynth test is back in batch 3).

## Borrowed d2 pieces (the desktop is offline), batch 3 (unbuilt)

Fast-forwarded to tarkir 53fd106 (the merger's X-dependent activation targets; needs_core borrow notes).
- `When::OpponentCastsInstantOrSorcery` now carries the cast spell as `extra`: its emit in `finish_cast` is one
  line, `Game::fire_with_extra` (game/zones.rs: `fire`, then `extra` on the triggers it added). Arasta and
  Niv-Mizzet, Parun ignore `extra`.
- `When::YouPlayCardFromExile`, emitted by `Game::played_from_exile(p, card)`: one line in `cast_now` (before the
  card moves to the stack) and one in the `PlayLand` arm (before it moves to the battlefield); it fires only if the
  card is in exile then. A spell's trigger goes on the stack above it.
- Prosper, Tome-Bound (listed): the existing `ExileTopToPlayAll(1)` at your end step ("until the end of your next
  turn") and Pact Boon's Treasure.
- Wandering Archaic // Explore the Vastlands: still not scripted. Its front is ready to write
  (`may(d, When::OpponentCastsInstantOrSorcery, vec![], vec![Effect::UnlessOpponentPays(2), Effect::CopySpellNewTargets])`),
  but the back needs an effect that doesn't exist yet: each player looks at the top five, may reveal a land
  and/or an instant or sorcery, takes them, the rest go to the bottom in random order, and everyone gains 3
  (d3's look-at-five). It also needs `back_face_castable` extended to instant and sorcery backs (Mac C). I'll do
  that piece when the effect lands, or both if it's handed to me.
Tests: core_objects 42 (the Mycosynth copy test back, Arasta's trigger carrying the spell, Prosper playing a land
from exile and not from hand); pg_c 212.

## needs_core (mac-c), batch 12: Sheoldred and Animar (unbuilt)

tarkir was already at a6fb683 (nothing to merge). Tokens: 100 is a placeholder for d1 (`Tokens::placeholder_100`),
101 the 1/1 green Insect with flying and deathtouch (`Tokens::insect_deathtouch`); 102-104 still free for lane C.
Core (game/zones.rs, dispatch arms after `SourceBecomesCopyOfSlot`):
`Effect::{DestroyAllCreaturesTappedTokenPerNontoken, TargetPlayerDrawsHalfLibraryLosesHalfLife,
EachPlayerDrawsPerKindCounter, SearchCreatureBySacrificedMvPlusCounter, ExchangeControlWithSlotOrSacrifice}` and
`SearchTo::BattlefieldPlusCounter` (one arm in the `Decision::Search` answer).
Cards (6, listed): Blood Money, Peer into the Abyss, Otherworld Atlas (charge counters are `kind_counters`), Neoform,
Gilded Drake, Hornet Queen. Tests in core_objects (47).
Shortcut: Neoform's counter is put on just after the card enters. (Gilded Drake with its one target illegal
doesn't resolve, CR 608.2b, so it isn't sacrificed: correct, per the merger.)
needs_core.json: the 7 blocked entries now name the blocker's owner (d1: Elder Brain, Walking Ballista, Somberwald
Sage, Vizier, Karador; d2: The Ozolith; mac-b: Rhythm of the Wild), each with lane C's part noted.
Waiting on others:
- Elder Brain: d1 ("spend mana as though it were mana of any color"). The exile-hand, draw-that-many and
  play-while-exiled part is lane C's and needs no new piece beyond `playable`.
- Walking Ballista: d1 (a "remove a +1/+1 counter from this" activation cost); `enters_with_x_counters` and `{4}`
  exist.
- Rhythm of the Wild: riot (enters with your choice of a +1/+1 counter or haste: Mac B replacement, or d1
  keywords) and "creature spells you control can't be countered" (a static: merger).
- Somberwald Sage: d1 (mana spendable only on creature spells).
- Vizier of the Menagerie: `top_spells_creatures` exists; "spend mana of any type to cast creature spells" is d1.
- The Ozolith: d2 (a "creature you control leaves the battlefield, if it had counters" trigger with its last-known
  counters); moving the counters at the beginning of combat is lane C's once that exists.
- Karador, Ghost Chieftain: d1 (costs {1} less per creature card in your graveyard); "once during each of your
  turns, cast a creature spell from your graveyard" is lane C's (cast-from-zone) and next.

## Handoff (2026-11-26, power outage): for the next lane C

State: tarkir-c at the commit after 6265d77 (notes only). Built and verified: everything through a6fb683
(on tarkir; 2,133 passing there). **Unbuilt WIP on top: 75620ee (batch 12 cards) and 6265d77 (needs_core owners).**

When the merger sends the go:
1. Merge tarkir (it should include lane B's 25c40ea). Take tarkir's `token_defs` and `Tokens` exactly; lane C's
   slots are after 100: `placeholder_100` (d1's, it renames it) and 101 `insect_deathtouch` (Hornet Queen).
   102-104 are lane C's and unused. Check the ends of CardDef, Game, Object, Effect, When, Decision, SearchTo,
   ManaExt, ManaCount, SubsystemExt, ZonesExt: each the union of both sides, no duplicates. `cargo build` before
   committing the merge.
2. Full check: `PATH=/opt/homebrew/opt/rustup/bin:$PATH CARGO_INCREMENTAL=0
   CARGO_TARGET_DIR=/Users/adamliscia/bolt-target-lanes`, touch bolt-core/src/*.rs, bolt-core/src/game/*.rs and
   bolt-cards sources first; `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
   `cargo test --no-fail-fast` (per-binary counts equal the `#[test]` counts: core_objects 47, pg_c 212 from lane
   C's side; play.rs is ignored), then `.venv/bin/ruff check .`, `ruff format --check .`,
   `.venv/bin/python -m pytest -q tests`.
3. Batch 12 untested so far, so the likely spots if something fails: `Game::exchange_control_or_sacrifice`
   (Gilded Drake, an "up to one" trigger pushed as a raw `Trigger` with `up_to`), Neoform's `CastWith` sacrifice
   plus the `Decision::Search` answer `SearchTo::BattlefieldPlusCounter`, and Peer into the Abyss's test (it sets
   player 1's life to 39 and expects 19).
4. Commit, send the merger the hash and totals. Never push; never commit to tarkir.

Open lane C work, waiting on other owners (needs_core.json names each blocker and lane C's part):
- Elder Brain (d1: spend mana as any color), then lane C's exile-hand, draw-that-many, play-while-exiled.
- Karador (d1: cost per graveyard creature), then lane C's once-each-turn creature cast from the graveyard.
- The Ozolith (d2: leaves-with-counters trigger), then lane C's counter keeping and moving.
- Wandering Archaic // Explore the Vastlands: d3's look-at-five effect, then lane C extends `back_face_castable`
  (game/zones.rs) to instant and sorcery backs. The front is one line (see "Borrowed d2 pieces, batch 3").
- Lithoform Engine's targeting shortcuts wait on d3's `Filter` (permanent spells; a controller check for
  `abilities`).
- Earlier lists in this file: "Held back (lane C)", the Olivia/Bruna script-only pass ("Still waiting, by owner"),
  and lane B's decks' Mac C pieces.
Rules that bit this lane: Effect is `Copy` (no `.clone()`, clippy fails); test bindings must be used or `_`-named;
`obj_mut` isn't public to tests (drive state through actions, `start_turn`, `to_main1`).

## Merge of tarkir 25c40ea, and needs_core (mac-c) batch 13 (unbuilt)

Merge 09782f0: tarkir 25c40ea (lane B) into tarkir-c. One conflict, the end of `Effect` (lane B's
`DamageExtrasControllerAmount`, `SylvanLibrary`, then lane C's batch 12 variants). Tokens: tarkir's, then 100, 101.
needs_core.json: union (828 entries both sides). Every variant/field set checked as the union, no duplicates.
Unbuilt: the merger held the build slot (2026-11-26).

Batch 13, unbuilt. Core (appended; arms after `ExchangeControlWithSlotOrSacrifice`, functions in game/zones.rs):
- `Effect::CountersOnAttached(n)` (Bone Sabres), `Effect::CopyAttachedThenEightSameNameWins` (Mechanized
  Production: token copy of the enchanted artifact via `token_copy_of`, doubled, then the eight-same-name win).
- `ZonesExt::cast_from_graveyard_or_exile` (Squee): two lines in `priority_actions`' `from_exile` (owner only) and
  `from_graveyard` filters.
- Tokens 102 `wurm_deathtouch`, 103 `wurm_lifelink` (Wurmcoil Engine). 104 still free.
Cards (4): Wurmcoil Engine, Bone Sabres, Squee, the Immortal, Mechanized Production. Tests in core_objects (51).
Likely failure spots: the Squee test (cast from exile/graveyard through `play`), Mechanized Production's
`upkeep_through` (start_turn(5, 0) and (7, 0) on player 0 only).

Looked at and left (mac-c owner in needs_core.json, blocker elsewhere):
- Urza, Lord High Artificer: "Tap an untapped artifact you control: Add {U}" (d1, mana production). The Construct is
  `saga_construct`; the {5} shuffle-exile-play-free is lane C's once the mana piece lands.
- The Necrobloom: dredge 2 is a draw replacement (Mac B); the landfall choice is simple.
- Final Act: d3-shaped (destroy all planeswalkers, mass effects) plus "each opponent loses all counters"; no battles in
  the engine. Doable Farewell-style (31 mode sets) if the merger hands it over.
- Briarbridge Patrol: "deals damage to one or more creatures" trigger and a clues-sacrificed-this-turn count (d2).
- Toxrill: slime-counter scaled static (d3) and a dies-with-slime trigger (d2), plus a Slug token.
- Teferi, Master of Time: instant-speed loyalty (merger timing); phasing and extra turns exist.
- Rona: "whenever you cast a legendary spell" (d2), then the back face (lane C).
- Karador: lane C's once-each-turn graveyard creature cast still waits on d1's cost reduction.

Built on 2a71b71 (tarkir 47148af merged): one fix, the Aura state-based check in game.rs now keeps an Aura whose
enchant filter allows artifacts on an artifact (`|| (ty.artifact && f.artifacts)`; Mechanized Production fell off
its Sol Ring). Squee's two `priority_actions` lines approved by the merger. 2,145 Rust tests pass (core_objects 51,
pg_c 212, play.rs 3 ignored), clippy, fmt, ruff and pytest clean.

Final Act (unbuilt, approved by the merger): Farewell's way, 31 mode sets (bit i: mode i). New
`Effect::{DestroyAllPlaneswalkers, EachOpponentLosesAllCounters}` (zones.rs; poison and energy are the player
counters the engine has). "Destroy all battles" is an empty mode: the engine has no battles, so there is nothing to
destroy; revisit when battles (Invasion of Ikoria) land. Test in core_objects (52).
Built on 6343801 (tarkir 56ccd07 merged): Final Act passes. 2,173 Rust tests (core_objects 52, pg_c 212, play.rs 3
ignored), pytest 166, clippy, fmt and ruff clean.

## 2026-11-27: Agent of the Iron Throne, Summon: Bahamut, and the Bruna/Olivia routing

- Agent of the Iron Throne and Summon: Bahamut: skipped. d4 batch 8 (`SubsystemExt::commanders_have_trigger`,
  `SubsystemEffect::DamageEachOpponentTotalManaValueOfOthers`) isn't on origin/lane-d4 (still 855074f) or tarkir.
- Bruna, Light-Paws, Sovereigns of Lost Alara, Olivia, Dragonhawk, Charming Scoundrel, Witch's Mark: all already
  scripted in pg_c (batches 9-11) and built on tarkir; their needs_core entries are now marked done. The rest of
  those two decks is in "Still waiting, by owner" under the script-only pass above.

## Live-validator queue, batch 14 (unbuilt)

Core (appended): `Effect::TargetOpponentMaySacrificeTwoOrYouDrawTwo(slot)` with `Decision::PatronOfChaosSacrifice
{ controller, first }` (Pick, Pick; `No` before the first: you draw two; offered only with two or more nonland
nontoken permanents that can be sacrificed, else you just draw), labelled in service_view and
test_decision_labels; `ZonesExt::top_spells_flash_or_flying` (one more clause in `priority_actions`' `top`).
Cards (4): Rakdos, Patron of Chaos; Hanweir Battlements (Hanweir Garrison isn't in the pool, so the meld ability
resolves and does nothing: noted in the script; meld itself isn't built); Errant and Giada. Tests in core_objects
(55). Tokens 104-106 unused.
Likely failure spots: the Rakdos test's `to_end_step_target` (expects the end-step trigger to ask for its target),
and Hanweir's ability indices (mana ability not in `activated`: haste 0, meld 1).
Left, by blocker:
- Tendo Ice Bridge, Flitterwing Nuisance, Glen Elendra Guardian: d1 ("remove a counter" as a mana or activation
  cost, like Walking Ballista). The enters-with-counter part is easy once that lands.
- Inkshield: Mac B (prevent combat damage, a token per point prevented); needs a 2/1 W/B Inkling flier (104).
- Skycat Sovereign: d3 (+1/+1 per other flier you control, scaled statics); Cat Bird token would be 105.
- Mausoleum Wanderer: "counter unless they pay X, X its power" (d3 effect).
- Deep-Cavern Bat: lane C, next (look at the hand, exile a nonland card until it leaves).
- The One Ring: burden counters, the upkeep loss and the draw are lane C's; "you gain protection from everything"
  for a player needs damage prevention (Mac B) and untargetability (d3's target_ok). Next after the Bat, or split.

## Merge of tarkir e369439, and batch 15: Elder Brain, Inkshield, The One Ring, Deep-Cavern Bat (built)

Merge a1a959e: tarkir e369439 into tarkir-c. Conflicts only at the ends of `Effect` and `Decision` (tarkir's
variants first, then lane C's). Tokens are tarkir's exactly: 100 `bird_flying_wu` (d1), 101-103 lane C, 104
`placeholder_104` (lane B's, still free for lane C), 105 `inkling`, 106 `warrior_white`. needs_core.json: union by
card (835).

Core (appended; arms after `TargetOpponentMaySacrificeTwoOrYouDrawTwo`, functions in game/zones.rs):
- `Effect::ExileDefendingHandDrawThatManyPlayable` (Elder Brain, on `When::Attacks`; nothing if it's attacking a
  planeswalker): exiles the defending player's hand, sets `playable = Some((you, u32::MAX))` and d1's
  `any_color_mana` on each, then they draw that many.
- `Effect::{YouLoseLifePerKindCounter, YouDrawPerKindCounter}` (The One Ring; burden counters are `kind_counters`).
- `Effect::ExileFromHandUntilSourceLeaves(slot)` with `Decision::ExileFromHandUntilSourceLeaves { victim, source,
  zcc }` (Pick a nonland card, or No; labelled, tested) and `Game::hand_links`: one call in `move_to`'s
  from-battlefield block (`return_hand_links`) sends the card back to its owner's hand (CR 610.3), unlike
  `links`, which return to the battlefield.
Cards (4): Elder Brain, Inkshield (lane B's `PreventCombatDamageToYouMakingTokens`, token 105), The One Ring (lane
B's `YouGainProtectionUntilYourNextTurn` behind `Condition::CastIt`; "can't be enchanted" by a Curse isn't
modelled), Deep-Cavern Bat. Tests in core_objects (59). Fix: `to_end_step_target` (Rakdos) now finishes attacks
and blocks on the way.
Built: 2,220 Rust tests pass (core_objects 59, pg_c 212, play.rs 3 ignored), every binary's count equals its
`#[test]` count; clippy, fmt, ruff (check and format) clean; pytest 172.

## Batch 16 (unbuilt, waiting on the merger's go): Rona, and the Urabrask and Sheoldred transform conditions

tarkir-c fast-forwarded to tarkir 2a99d72 (it already held 0f8556c; no merge commit needed).
Core (appended):
- `Activated::{min_instants_sorceries_this_turn, opponent_min_graveyard}` with two clauses in the activation
  legality check (next to `needs_threshold`). **An edit inside an existing function: needs the merger's OK.**
- `Effect::DamageSourceControllerExilesRandomFromHand` (zones.rs `exile_random_from_damager_hand`): a land asks
  the new `Decision::MayPutExiledLandOntoBattlefield { card }` (Yes/No; labelled, tested); a nonland card goes to
  `free_casts` with `free_caster` = you.
- **Recorded borrow from d2:** one line in the `When::DealtDamage` emit (deal_damage, `Target::Object`) sets each
  trigger's `extra` to the damage source if it was empty. Only blc_c's `DrawAmount` cards use that trigger, and
  they ignore `extra`.
- `When::YouCastLegendary` was already on tarkir (d2), so no borrow there.
Cards (1): Rona, Herald of Invasion // Rona, Tolarian Obliterator. Tests in core_objects (61: Rona, and the two
activation conditions on an edited Otherworld Atlas).
Not scripted, by blocker:
- Urabrask // The Great Work: the transform is `ExileSelfReturn { transformed: true }` with `sorcery` and
  `min_instants_sorceries_this_turn = 3`. Waits on d4's chapter III (cast instants/sorceries from any graveyard
  this turn, exile them instead). Chapters I and II look doable with existing effects.
- Sheoldred // The True Scriptures: transform is `ExileSelfReturn` + `sorcery` + `opponent_min_graveyard = 8`.
  Waits on d4's Saga. Also the front's "each opponent sacrifices a nontoken creature or planeswalker" has no
  effect yet (`EachPlayerSacrificesNontokenCreature` is creatures only, every player). Chapter III could be
  `ReanimateAll { all_graveyards: true, .. }` then `ExileSelfReturn { transformed: false }`.
- Toxrill: dropped per the merger (d3 slime store, d2 Slug 109 and dies trigger). 104 still free.
Likely failure spots when built: the Rona test (the {B/P} transform paid from table(), `play` answering `Yes`
for the land decision), `g.objects[..].tapped` set directly.
Merger approved (2026-11-27): the two activation-legality clauses and the `DealtDamage` `extra` borrow.
Added, unbuilt: `Effect::OpponentSacrificesNontokenCreatureOrPlaneswalker` with
`Decision::SacrificeNontokenCreatureOrPlaneswalker` (Pick; asked only when there is one; labelled, tested), a
sibling of Accursed Marauder's edict, for Sheoldred's front. Test in core_objects (62), on an edited Inkshield.
Sheoldred itself still waits on d4's Saga. Build after lane B's dredge batch, on the merger's go.
Built on the merge of tarkir 8a4b5cf (lane B's dredge and The Necrobloom): conflicts only at the ends of
`Effect`, `Decision` and test_decision_labels.py (tarkir's first, then lane C's); tokens are tarkir's exactly;
needs_core.json union (835). Built cold: 2,245 Rust tests pass (core_objects 62, play.rs 3 ignored), every
binary's count equals its `#[test]` count; clippy, fmt, ruff clean; pytest 177. No fixes were needed.

## Batch 17 (unbuilt): live-missing and idea quick wins in objects and zones

tarkir-c fast-forwarded to local tarkir bffcfdd (the 100 new cards' oracle). Token 104 now goes to lane B.
Core (appended): `Effect::{GainLifePerCreatureYouControl, DestroyAllOtherPermanentsExceptLandsAndTokens}` (Elspeth
Tirel; zones.rs `destroy_all_other_nonland_nontoken`).
Cards (6), all on existing pieces except Elspeth: Silundi Vision // Silundi Isle (Ghyrson; `LookAtTop` +
`modal_back`), Slip Out the Back (Najeela; `Counters` + `PhaseOutSlot`), City of Shadows (Atraxa; storage counters are
`kind_counters`, `exile_creature_you_control`, `mana_per_kind_counter`), Elspeth Tirel (Atraxa), Ramunap Excavator
(Hearthhull, idea; `lands_from_graveyard`), Aquitect's Defenses (Niv-Mizzet, idea). Tests in core_objects (68).
needs_core.json: the four entries marked, and Ramunap and Aquitect's Defenses added as lane C's claims.
Left in the named decks, by blocker (lane C's theme, not done):
- Tandem Lookout (Ghyrson): soulbond's granted "deals damage to an opponent, draw" needs a d2 emit for granted
  triggers (the pairing exists).
- Invasion of Arcavios (Ghyrson): battles aren't in the engine.
- Urza (Lonis): d1 (tap an artifact for mana). Briarbridge Patrol (Lonis): d2. The Earth Crystal (Korvold): Mac B
  doubling and d1 discount. Flitterwing Nuisance and Glen Elendra Guardian (Raffine): d1 (remove a counter as a
  cost). Mausoleum Wanderer, Skycat Sovereign (Raffine): d3. Veil of Summer, Derevi, Druids' Repository (Najeela):
  mixed / merger / d1. Vraska, Tekuthal, Ichormoon Gauntlet, Nissa (Atraxa): proliferate and compleated, not started.
- Idea decks, possible next in this theme: Arthur, Marigold Knight (Zinnia: top six onto the battlefield attacking,
  returned at end of combat), Morningtide's Light (exile and return at the next end step), Mirrorform (needs a
  non-Aura target filter, d3), Goliath Daydreamer (cast from exile with dream counters), Ashling's transform.
Likely failure spots when built: Silundi's `Pick` answer, Elspeth's loyalty indexes (+2 is 0, −2 is 1, −5 is 2) and
setting `loyalty` directly, City of Shadows paying Sol Ring with only its stored {C}.

## Batch 18 (unbuilt): Arthur, Goliath Daydreamer; Ashling held

Core (appended):
- `LookTo::BattlefieldAttackingReturnAtEndOfCombat` (one arm in the `Decision::LookPick` answer) and
  `Game::hand_at_end_combat`, emptied by `return_hand_at_end_combat` from one call at the top of the
  `Step::EndCombat` block (next to decayed and Delina's exile). **Merger: that call is in turn structure.**
- `Effect::{DreamExileExtraIfCastFromHand, MayCastDreamCardFree}` with `Game::{dream_marked, dream_counters}`.
  The resolution site's `leave_stack` branch is now `else if !self.exile_with_dream_counter(source)`: a marked
  spell is exiled with a dream counter instead (zone destination, lane C's). The attack trigger opens a one-spell
  free-cast window (`free_casts_one`) over the dream cards the controller owns.
Cards (2): Arthur, Marigold Knight (`GateAttackingAtLeast(2)` then `LookAtTop`), Goliath Daydreamer. Tests in
core_objects (71). Shortcut: the dream cards not cast keep `cast_free` while they stay in exile; nothing else
reads it outside a free-cast window.
Held: Ashling, Rekindled // Ashling, Rimebound needs "whenever this transforms into [face]" (d2) and mana spendable
only on spells with mana value 4 or greater (d1). Its transform (`MayPay` then `TransformSelf` on
`When::FirstMainPhaseYourTurn`) exists. Recorded in needs_core.json.
Likely failure spots when built: Arthur's `resolve_all` answer (the LookPick `Pick`), the `combat` check, and
Goliath's free-cast window after the attack trigger (`Pending::FreeCast`).
Merger approved (2026-11-27): the `return_hand_at_end_combat()` call at the top of `Step::EndCombat`, and the `else if !self.exile_with_dream_counter(source)` branch at resolution. Ashling held as recorded. Build after d3 (Mikaeus) and lane B, on the go.

## Handoff (2026-11-27, context full): for the next lane C

State: tarkir-c at the commit after 584431f (notes only). Built and on tarkir: everything through 096072f.
**Unbuilt WIP on top: 662042a (batch 17) and 389f90e (batch 18), both accepted by the merger; build on its go**
(order: merger verification, d3 Mikaeus, lane B, then lane C). Full check as in the batch 12 handoff above;
core_objects should count 71. Likely failure spots are listed under batches 17 and 18.

Queued by the merger, **once d3 b30 is on tarkir** (not there at b7d3a48). Write unbuilt, merge tarkir first:
- Pippin, Guard of the Citadel (Bruna's deck, lane C's): d3's `Effect::ProtectionFromCardTypeEot { slot, kind }`,
  kind in ChooseCardType order (0 artifact, 1 creature, 2 enchantment, 3 instant, 4 land, 5 planeswalker,
  6 sorcery).
- Toxrill, the Corrosive (Sefris): `Effect::SlimeCounterOnEachCreatureYouDontControl` at each end step
  (`When::EndStep { your_turn_only: false }`), the static `Static { scope: Scope::OpponentsCreatures, power: -1,
  toughness: -1, scale: Some(Scale::PerSlimeCounterOnIt) }`, d2's dies trigger (reads `last_slime > 0`, makes the
  Slug, token 109), and "{U}{B}, Sacrifice a Slug: Draw a card" from existing pieces (`sacrifice_subtype`).
Token 104 is lane B's now; ask the merger for any token. Report to the merger with hashes and totals; never push.

## Merge of tarkir 1eaaf4e; batches 17 and 18 built; Pippin and Toxrill (2026-11-27)

Merge c4735d0: tarkir 1eaaf4e (219ba6e plus d1 b23-b27, d2 b25-b29, d3 b30-b31, lane T) into tarkir-c.
Conflicts only at the end of `Effect` (tarkir's first, then lane C's) and needs_core.json (three-way union by
card; Goliath Daydreamer keeps lane C's claim, since d2 didn't script it). Tokens are tarkir's exactly (109
`slug`, 110 `phyrexian_horror_x`). No duplicate members in any enum or struct.
Cards (2), on d3 b30's pieces: Pippin, Guard of the Citadel (`ProtectionChosenCardTypeEot(Some(0))`, the type
asked with `Decision::ProtectionCardType` as it resolves), Toxrill, the Corrosive (end-step slime, the scaled
static, the slimed-creature-dies Slug, and `sacrifice_subtype = "Slug"` for the draw). Sefris is finished.
Built cold, two test fixes: Aquitect's Defenses now passes through cleanup (start_turn skips it, so
`until_eot` was never cleared), and Elspeth Tirel's test sets her loyalty to 4 first. 2,335 Rust tests pass
(core_objects 73, 4 ignored: 3 play.rs, 1 rollout); every binary's count equals its `#[test]` count; clippy,
fmt, ruff (check and format) clean; pytest 194 passed, 1 skipped.

## Batch 19 (unbuilt): Rev, Tithe Extractor; Ashiok, Nightmare Muse's pieces

Core (appended; arms after `MayCastDreamCardFree`, functions in game/zones.rs):
- `Effect::ExileTopOfOpponentFaceDownCastable` (Rev): the opponent's top card to exile, recorded in the new
  `Game::face_down_exiled` (not `Object::face_down`, which would make `def_of` a 2/2 with no cost); `playable =
  Some((you, u32::MAX))` unless it's a land (a land can't be cast). `Game::exiled_face_down(c)` reads both.
  Limit: the observation still shows the card's face; only the rules treat it as face down.
- `Effect::CastFaceUpOpponentExiledFreeUpTo(n)` (Ashiok's −7) with `Game::free_casts_max`: **approved edits in
  existing functions**, a guard in `free_cast_actions` (closes once `n` of the window's cards have left exile,
  `free_casts_cast`) and the reset in `close_free_casts`.
- `Effect::EachOpponentExilesTop(n)` (the Nightmare token's attack/block trigger, n = 2).
Cards (1): Rev, Tithe Extractor ("that player" is the one opponent). Ashiok waits on d3's −3 and on token 120
(appended after d2's 111-118 and d3's 119 once they're on tarkir; no placeholders). Tests in core_objects (77:
two for Rev, the −7 and the exile-two on an edited Inkshield).
Likely failure spots when built: Rev's attack trigger target answer in `resolve_all`, the −7 window reopening
after each cast (three `Cast` + `Target` in a row), Bolt from exile paid with the table's Mountains.

## Batch 20 (unbuilt): Alchemist's Talent, Fortune Teller's Talent, Evercoat Ursine

The Talents were handed back from d4 (retired); d4's Class pieces and d1's `treasures_add_two_at_level` and
`not_from_hand_discount` were already on tarkir.
Core (appended; arms before `EachOpponentExilesTop`, functions in game/zones.rs):
- `Effect::CreateTokensTapped { def, n }` (two tapped Treasures), `Effect::DamageEachOpponentCastSpellManaValue`
  (level 3, after `StopUnlessTreasureManaSpentOnExtra`, on `When::YouCast` behind `ClassLevelAtLeast(3)`).
- `ZonesExt::top_cards_if_cast_spell_at_level` (Fortune Teller's level 2): one clause in `priority_actions`' `top`
  filter (`top_cards_after_a_spell`), before the land/spell split, so lands and spells both. Lane C's
  top-of-library play, as the merger routed it.
- `Effect::{HideawayAnother(n), MayCastOneHideawayFree}` with `Game::hideaway_more` (Evercoat Ursine: the first
  hideaway card is kept there, then the existing `Decision::HideawayPick` is asked again). **Ursine is unclaimed**
  (script kept with only its hideaway line in `lines`, so it is not supported or built, and off `PG_C_SCRIPTED`;
  merger 2026-11-27; unlisting alone would still claim it, since support comes from the lines): "play one of them" includes a hidden land played as a
  land play (CR 305.2), and the one-spell free-cast window is spells only. Needs: `free_cast_actions` offering
  `PlayLand` while a land play remains, the `Pending::FreeCast` mapping letting `PlayLand` through to its own arm,
  and that arm closing the window (`close_free_casts`) after the land enters.
Tests in core_objects (81: Alchemist's two, Fortune Teller's, and Ursine's pieces on an edited Hungry Ghoul).
Likely failure spots when built: the Treasure-only Bolt (`game(&[])`, paid by the Treasure), the Ursine
hideaway picks in `play`'s answers (second Bolt from the first three, then the first from the next three), the
Class level set directly on `class_level`.

## Batch 21 (unbuilt): Blade of the Bloodchief (Korvold, from live_missing.json)

Core (appended): `Effect::CountersOnAttachedBySubtype { n, subtype, n_if }` (zones.rs
`counters_on_attached_by_subtype`), on `When::AnotherCreatureDies` (the Blade isn't a creature, so that's every
creature). Card (1): Blade of the Bloodchief. Test in core_objects (82).
Nesting Dragon (Korvold) waits on two token numbers: a 0/2 red Dragon Egg with defender and a dies trigger, and a
2/2 red flying Dragon with "{R}: +1/+0".

### Nesting Dragon (written, unbuilt; needs its two tokens)

Scripted against `t.dragon_egg` (123) and `t.dragon_2_firebreathing` (124), test in core_objects (83). To append
once d2's 111-118, d3's 119 and 121 and d1's 122 are on tarkir, after 120 (Ashiok's Nightmare):
- 123: `red(token("Dragon Egg", "Dragon Egg", 0, 2, Keywords::DEFENDER))`.
- 124: a 2/2 red Dragon, `red(token("Dragon", "Dragon", 2, 2, Keywords::FLYING))` with `activated(&mut t, "{R}",
  false, vec![], vec![Effect::PumpSelf { power: 1, toughness: 0 }])`.
- The Egg's "When this token dies, create [124]": `token_defs()` doesn't know the token base, so no token row can
  name another token. Proposed (merger's file, lib.rs `CardSet::load`, after `defs.extend(scripts::token_defs())`):
  one line pushing `Trigger { when: When::Dies, ability: CreateToken { who: You, def: tokens.dragon_2_firebreathing,
  n: 1 } }` onto `defs[tokens.dragon_egg]`. Or `token_defs(&Tokens)` if the merger prefers.
Merger approved option (a) (2026-11-27): the wiring is in lib.rs `CardSet::load` (commit after 2d87a69). Both
commits need the token rows; if the build comes first, revert both and restore them once the tokens land.

## Merge of tarkir 51b84e4; batches 19-21, Nesting Dragon and Ashiok built (2026-11-27)

Merge f087200: one conflict, the end of `Effect` (tarkir's first, then lane C's). Tokens: tarkir's, with
`placeholder_120` replaced by 120 `nightmare` (2/3 blue and black; attacks or blocks: `EachOpponentExilesTop(2)`),
122 `placeholder_122` (d1's Human Cleric, not on tarkir yet), 123 `dragon_egg`, 124 `dragon_2_firebreathing`.
Ashiok, Nightmare Muse scripted (+1 the Nightmare, −3 d3's `BounceThenOwnerExilesFromHand`, −7 lane C's window);
test in core_objects (84). Built cold, no fixes needed: 2,412 Rust tests pass (core_objects 84, 4 ignored: 3
play.rs, 1 rollout), every binary's count equals its `#[test]` count; clippy, fmt, ruff (check and format) clean;
pytest 201 passed, 2 skipped.

## Batch 22 (unbuilt): Wishclaw Talisman (Kenrith), Midgar, City of Mako (d2's handback)

Core (appended; functions in game/zones.rs):
- `Effect::OpponentGainsControlOfSource` (`give_source_to_opponent`: controller and `since_turn`, for good).
- `ZonesExt::enters_with_kind_counters` with one call (`enter_with_kind_counters`) after the vanishing block in the
  enter path. **An edit in the enter path (merger's): one line.** d1's `remove_counters_self` isn't on tarkir;
  the wish counter is paid with the existing `Activated::remove_kind_counters`.
- Midgar: in `priority_actions`' cast list, a land with an Omen/Adventure half no longer `continue`s before the
  Omen block (`def.types.land && def.omen.is_none()`), and a `continue` after it keeps the land itself from being
  offered as a `Cast`. After the Adventure the card is `playable` from exile, played as a land.
- Reactor Raid uses **d2's** `Effect::MaySacrificeArtifactOrCreature` and `Decision::MaySacrificeArtifactOrCreature
  { source }` (lane-d2 b35, 592dc7b, with its labels): lane C's same-named pair was dropped (merger, 2026-11-27).
  **Doesn't compile until d2's b35 is on tarkir.**
Cards (2): Wishclaw Talisman, Midgar, City of Mako // Reactor Raid. Tests in core_objects (87).
Likely failure spots when built: the Talisman's activation legality for player 1 after `to_main1(6, 1)` (it must
untap in their untap step), the Midgar hand-count arithmetic.

## Batch 23 (unbuilt): the monarch; Court of Grace, Forth Eorlingas!

Core (appended; functions in game/zones.rs), modelled on d4's initiative:
- `Game::monarch: Option<(PlayerId, ObjId)>` (the seat, and the card that made them monarch: the source of the
  draw trigger) and `Game::monarch_on_combat_damage: [Option<(turn, card)>; 2]` (Forth Eorlingas!).
- `StackKind::MonarchDraw` with `monarch_draw_ability()` (draw a card): **one arm in `ability_of`'s
  exhaustive `StackKind` match.**
- **One call at the top of `Step::End`** (`monarch_end_step`: the active monarch's draw trigger goes on
  `triggered`). Turn structure: the merger's.
- **One call at the end of `fire()`** (`fire_monarch`, after `fire_subsystems`): on
  `When::YourCreaturesDealCombatDamageToPlayer` for `p`, the opponent's monarchy passes to `p`, or `p` becomes
  the monarch if Forth Eorlingas! resolved for them this turn (1v1: the player dealt damage is the opponent).
  `fire()` is d2's.
- `Effect::{BecomeMonarch, BecomeMonarchOnCombatDamageThisTurn, CreateTokenIfMonarchElse { monarch, other }}`.
- Observation: `monarch` (a seat or None) after `starting_life` (bolt-py); the view's state has a top-level
  `monarch` (service_view, docs/api.md row updated), with a test in tests/test_service_view.py.
Cards (2, drafted against idea_next11.json; they compile only once the 172 new cards and token 133 are in):
Court of Grace (`t.spirit`, `t.angel_4`), Forth Eorlingas! (`t.human_knight_2_trample_haste`, **token 133 asked
for**: a 2/2 red Human Knight with trample and haste). Tests in core_objects (90).
Likely failure spots when built: the end-step draw count in the Court test (the trigger resolves at End), the
Ghoul's attack on turn 6 for player 1, `CastX(forth, 2)` paying {2}{R}{W}.

## Batch 24 (unbuilt): Mirrorform; Walk-In Closet held

- `Effect::YourNonlandPermanentsBecomeCopyOf(slot)` (zones.rs `nonland_permanents_become_copy`, through the
  existing `become_copy`, for good). Mirrorform's target is d3's `Filter::not_subtype = Some("Aura")` on any
  permanent (no Filter change). Test in core_objects (91).
- Walk-In Closet // Forgotten Cellar: **held.** The oracle data has only the first door ("You may play lands
  from your graveyard.") and the Room reminder; Forgotten Cellar's text and cost aren't in it, and Rooms (two
  doors, unlocking) aren't in the engine. Needs `fetch_oracle.py` to keep both doors (as d4's split fields do)
  and a Room/unlock subsystem; claiming the card on its first door alone would leave out half of it.
- Sanar, Innovative First-Year: `Effect::VividRevealExileByColor` on `When::FirstMainPhaseYourTurn`, with
  `Decision::VividExile { cards, colors }` (Pick a revealed card with a color still open, or No: stop and
  shuffle) and `Decision::VividColor { cards, colors, card }` (Mode: which of its open colors a multicolored
  card uses up; asked only when there's more than one). Exiled cards get `playable` this turn. Labelled in
  service_view, tested in test_decision_labels; test in core_objects (92). The revealed cards stay in the
  library while choosing, then it's shuffled.
Merger approved (2026-11-27): token 133 `human_knight_2_trample_haste` (append its row once 125-132 are on
tarkir, at the build), the three monarch hooks, and holding Walk-In Closet (logged in needs_core.json, owner
mac-c: "Rooms subsystem plus a second-door oracle fetch").

## Batch 25 (unbuilt): Druids' Repository (Najeela)

On d1's `ManaExt::counter_mana_no_tap` (lane-d1 e4ce66a, **doesn't compile until it's on tarkir**): any-color
`ManaSource`, `mana_per_kind_counter`, `counter_mana_no_tap`; charge counters are `kind_counters`, one per
attacking creature you control (`When::CreatureYouControlAttacks`, `KindCounterSelf(1)`). Test in core_objects
(93): two attackers, two counters; a landless Bolt paid by removing one.

### Kambal, Profiteering Mayor: held, needs d2

Both of its triggers are batched ("one or more tokens ... enter"). `When::TokenYouControlEnters` fires once per
token (move_to's enter path), and there's no event for tokens entering under an opponent's control. Needs d2:
the two batched events, with the batch's token list (and "only once each turn" on the first). Lane C's part is
then small: for each listed token, `token_copy_of(you, it)` and tap it. Logged in needs_core.json (owner d2).
