# Tarkir: Dragonstorm, lane B

Scripts: `engine/bolt-cards/src/scripts/tdm_b.rs`. Tests: `engine/bolt-cards/tests/tdm_b.rs`.
Card scripts only, on existing engine rules.

## Cards done (15)

| Card | Test |
|---|---|
| Heritage Reclamation | `heritage_reclamation_destroys_an_artifact_or_enchantment_or_exiles_a_card_and_draws` |
| Cruel Truths | `cruel_truths_surveils_two_draws_two_and_costs_two_life` |
| Alesha's Legacy | `aleshas_legacy_gives_deathtouch_and_indestructible` |
| Worthy Cost | `worthy_cost_sacrifices_a_creature_to_exile_one` |
| Bewildering Blizzard | `bewildering_blizzard_draws_three_and_weakens_opposing_creatures` |
| Frontier Bivouac | `tri_lands_enter_tapped_and_tap_for_three_colors` |
| Mystic Monastery | `tri_lands_enter_tapped_and_tap_for_three_colors` |
| Nomad Outpost | `tri_lands_enter_tapped_and_tap_for_three_colors` |
| Sandsteppe Citadel | `tri_lands_enter_tapped_and_tap_for_three_colors` |
| Knockout Maneuver | `knockout_maneuver_counters_then_bites` |
| Shocking Sharpshooter | `shocking_sharpshooter_pings_when_another_creature_enters` |
| Auroral Procession | `auroral_procession_returns_a_card_from_your_graveyard` |
| Gurmag Rakshasa | `gurmag_rakshasa_shrinks_theirs_and_grows_yours` |
| Skirmish Rhino | `skirmish_rhino_drains_two` |
| Inspired Ultimatum | `inspired_ultimatum_gains_five_deals_five_draws_five` |

## Batch 2 (engine core): slow lands, becomes tapped, a creature with a counter, X = creatures (12)

Mechanics added (new variants and fields appended at the end of their enums and structs):
- `CardDef::enters_tapped_unless: Option<[&str; 2]>`: "This land enters tapped unless you control a [type] or a [type]."
- `When::BecomesTapped`: fired by `Game::become_tapped` wherever a permanent goes from untapped to tapped (attacking, {T} costs, mana, harmonize, `Tap`, `TapAndStun`). Entering tapped doesn't count.
- `Condition::ControlCreatureWithCounter` and `StaticCondition::ControlCreatureWithCounter`, using `Game::has_counter` (any kind: +1/+1, stun, keyword, incubation, soul, revival, fellowship, bait, loyalty). `Object::keyword_counter` records keyword counters, which otherwise only live in `granted`.
- `CardDef::not_blocked_by_lesser_power`: "Creatures with power less than this creature's power can't block it."
- `LookRest::MayGraveyard`: cards not taken may each go to the graveyard (a surveil).
- Effects: `PumpPerCreatures { slot, attacking }`, `DamagePerCreatures(slot)`, `PumpYoursPerCreatures`, `CountersAll { who, n }`, `DrawIfCounterElseCounterSelf`.

| Card | Test |
|---|---|
| Cori Mountain Monastery | `slow_lands_enter_tapped_unless_you_control_one_of_two_types`, `cori_mountain_monastery_exiles_the_top_card_to_play` |
| Kishla Village | `slow_lands_enter_tapped_unless_you_control_one_of_two_types`, `kishla_village_surveils_two` |
| Rescue Leopard | `rescue_leopard_loots_when_it_becomes_tapped`, `rescue_leopard_doesnt_trigger_entering_tapped_or_already_tapped` |
| Traveling Botanist | `traveling_botanist_takes_a_land_or_may_bin_a_nonland` |
| Trade Route Envoy | `trade_route_envoy_draws_with_a_countered_creature_or_grows`, `trade_route_envoy_counts_any_kind_of_counter_but_only_yours` |
| Delta Bloodflies | `delta_bloodflies_drains_on_attack_only_with_a_countered_creature` |
| Formation Breaker | `formation_breaker_cant_be_blocked_by_smaller_creatures_and_grows_with_counters` |
| Coordinated Maneuver | `coordinated_maneuver_deals_damage_per_creature_or_destroys_an_enchantment` |
| Frontline Rush | `frontline_rush_makes_goblins_or_pumps_per_creature` |
| Marshal of the Lost | `marshal_of_the_lost_pumps_by_the_number_of_attackers` |
| Craterhoof Behemoth | `craterhoof_behemoth_pumps_and_tramples_your_team` |
| Duty Beyond Death | `duty_beyond_death_protects_and_grows_your_creatures` |

Mistrise Village came in batch 3.

## Batch 3 (engine core): if you cast it, search by name, can't be countered, hexproof until damage (12)

Mechanics added (appended at the end of their enums and structs):
- `Condition::CastIt` ("if you cast it", from `Object::was_cast`, set when a permanent spell resolves; reset only on entering the battlefield) and `Condition::CastTwoSpellsThisTurn` (also accepted by `enters_counters`).
- `SearchFor::Named(&[&str])`.
- `Effect::NextSpellUncounterable` with `Game::next_spell_uncounterable` and `Object::cast_uncounterable`; every counter path now asks `Game::uncounterable`.
- `Effect::TapSelf` (through `become_tapped`, so "becomes tapped" fires).
- `Effect::PlayerExilesCreature(slot)` with `Decision::ExileCreature` (their choice; automatic with one creature).
- `Effect::DamageEqualCardPower { card, to }`: the spell deals damage equal to that card's power (printed power off the battlefield); none if that card's target became illegal.
- `StaticCondition::HasntDealtDamage` with `Object::dealt_damage` (set in `deal_damage`, reset on entering the battlefield).
- `CardDef::stops_activated_abilities` (Clarion Conqueror): artifacts, creatures and planeswalkers on the battlefield lose activations, loyalty abilities and mana abilities (including the Devotees' filter); lands and graveyard abilities are unaffected.

| Card | Test |
|---|---|
| Wingspan Stride | `wingspan_stride_pumps_gives_flying_and_returns_to_hand` |
| Mistrise Village | `mistrise_village_makes_the_next_spell_uncounterable` |
| Tempest Hawk | `tempest_hawk_fetches_another_tempest_hawk` |
| Dragonstorm Forecaster | `dragonstorm_forecaster_finds_a_globe_or_a_boulderborn_dragon` |
| Iridescent Tiger | `iridescent_tiger_adds_five_colors_only_if_cast` |
| Yathan Roadwatcher | `yathan_roadwatcher_mills_four_and_reanimates_a_cheap_creature_if_cast` |
| Unrooted Ancestor | `unrooted_ancestor_sacrifices_for_indestructible_and_taps` |
| Strategic Betrayal | `strategic_betrayal_exiles_their_chosen_creature_and_graveyard` |
| Lie in Wait | `lie_in_wait_returns_a_creature_card_and_deals_its_power` |
| Effortless Master | `effortless_master_enters_with_counters_after_two_spells` |
| Karakyk Guardian | `karakyk_guardian_has_hexproof_until_it_deals_damage` |
| Clarion Conqueror | `clarion_conqueror_stops_activated_abilities_of_artifacts_and_creatures` |

Tempest Hawk's "A deck can have any number of cards named Tempest Hawk." is claimed with no engine effect: the engine has no deck-size or copy limits.

## Needs core

Cards from the lane's list, in list order, skipped because the engine can't do them exactly yet. (Sultai Monument, Fleeting Effigy, Encroaching Dragonstorm, Misty Rainforest and Arid Mesa were done by the lead in Tarkir batch 8.)

- Embermouth Sentinel: the search destination depends on controlling a Dragon (top of library vs battlefield tapped); no condition-dependent `SearchTo` and no negated condition.
- Seize Opportunity: no effect for "exile the top two, you may play *those cards*" (both, not choose one); `ExileTopChooseOneToPlay` picks one.
- Ringing Strike Mastery: no "enchanted creature doesn't untap" static, and no way for an Aura to grant an activated ability ("{5}: Untap this creature").
- Fresh Start: the lose-all-abilities static forces a base P/T (`base_pt: None` becomes 0/0); needs "loses all abilities" without setting base P/T, plus -5/-0.
- Sonic Shrieker: "if a player is dealt damage this way, they discard": `Who::TargetPlayer` resolves a creature target to its controller, so the discard can't be limited to a player target that was dealt damage.
- Rite of Renewal: no "shuffle target cards from a graveyard into their library", and no "exile this spell" on resolution.
- Breaching Dragonstorm: no exile-until-nonland with cast-free-if-MV-8-or-less; no self-bounce.
- Reverberating Summons: no "becomes a creature if you've cast two or more spells this turn"; no "discard your hand" cost.
- Overwhelming Surge: no "choose one or both" modal spells; no "noncreature artifact" filter.
- Salt Road Skirmish: no tokens that gain haste and are sacrificed at the next end step without being tapped and attacking (`Mobilize` makes them attacking).
- Hundred-Battle Veteran: no static condition counting kinds of counters; no cast-from-graveyard with a finality counter.
- Desperate Measures: `GrantDiesTrigger` fires whoever controls the creature when it dies; needs "dies under your control".
- Host of the Hereafter: no "if it had counters, put its counters on" a target (moving counters from a dead creature).
- Stalwart Successor: no trigger for "the first time counters are put on a creature you control this turn".
- Ambling Stormshell: no stun counters on the source; no "Whenever you cast a Turtle spell" trigger.
- Eshki Dragonclaw: no condition "cast both a creature spell and a noncreature spell this turn".
- Lotuslight Dancers: no search for one card of each of three colors into the graveyard.
- Sage of the Skies: no cast trigger that copies the spell itself.
- Magmatic Hellkite: no "nonbasic land" filter; no opponent search with a stun counter on the land.
- Stillness in Motion: no library-empty check, self-exile, or put-five-from-graveyard-on-top.
- Herd Heirloom: no creature-spell-only any-color mana; no granted "deals combat damage to a player, draw" trigger.
- Narset, Jeskai Waymaster: no "discard your hand, draw equal to spells cast this turn".
- New Way Forward: no prevention of the next damage from a chosen source with a reflexive redirect.
- Dalkovan Encampment: the slow-land part now exists (`enters_tapped_unless`); still no delayed "whenever you attack this turn" trigger.
- Great Arashin City: the slow-land part now exists; still no "exile a creature card from your graveyard" cost.
- Glacierwood Siege: no "as this enters, choose Temur or Sultai" modes; no play-lands-from-graveyard.
- Barrensteppe Siege: no "choose Abzan or Mardu" modes; no each-opponent-sacrifices with a died-under-your-control condition.
- Jeskai Revelation: no Monk token with prowess; no "target spell or permanent" bounce.
- Felothar, Dawn of the Abzan: no "you may sacrifice a nonland permanent; when you do" reflexive trigger.
- Death Begets Life: no destroy-all-creatures-and-enchantments with draw per permanent destroyed.
- Teval, Arbiter of Virtue: no delve granted to your spells; no lose-life-equal-to-mana-value cast trigger.
- Shiko, Paragon of the Way: no exile-then-cast-a-copy-free from the graveyard.
- Betor, Kin to All: no total-toughness conditions; no "loses half their life".
- Elspeth, Storm Slayer: `doubles_tokens_and_counters` also doubles counters (Elspeth doubles tokens only); no "gain flying until your next turn".
- Stormscale Scion: no storm.
- Call the Spirit Dragons: no per-color counter placement on Dragons or win condition; no static granting indestructible to Dragons.
- Neriv, Heart of the Storm: no damage doubling for creatures that entered this turn.
- Smile at Death: graveyard targets can't filter on power ("power 2 or less").

- Emergent Ultimatum: no search for up to three monocolored cards with different names into exile, no opponent's choice to shuffle one back, no cast-the-rest-free, and no self-exile.
- Ruinous Ultimatum: no "destroy all nonland permanents your opponents control" (only `DestroyAllCreatures` exists).

Round 2 (after the chunk 1 merge, e5c03c9): the cards left on the list after Inspired Ultimatum
(Misty Rainforest, Arid Mesa, Emergent Ultimatum, Ruinous Ultimatum) all need core, and every
earlier card on the list was already assessed. No new cards were scripted; the list is used up.
