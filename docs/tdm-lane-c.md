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
