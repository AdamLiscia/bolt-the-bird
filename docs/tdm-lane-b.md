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

## Batch 4 (engine core): first counters each turn, a delayed attack trigger, one or both, and more (11)

Mechanics added (appended at the end of their enums and structs):
- `When::FirstCountersOnYourCreatureThisTurn`, fired from lane C's `counter_put_on` hook (so entering with counters, stun and keyword counters all count) using `Object::counters_put_turn`; slot 0 is that creature.
- `SearchTo::TopOrTappedIfYouControl(subtype)`: decided as the search resolves.
- `Effect::ExileTopToPlayAll(n)`: every exiled card is playable until the end of your next turn.
- "Choose one or both" is three modes: each alone and both together (CR 700.2). `Filter::noncreature` for "noncreature artifact".
- `Effect::SacrificeCreatedAtEnd` for the tokens just created.
- `Effect::DiscardIfPlayerTarget(slot)`: a player target that was just dealt the damage discards.
- Fresh Start: an Aura's lose-all-abilities static with no base P/T, colors or land type now leaves base P/T and creature types alone (`Game::loses_abilities_only`); Witness Protection and Imprisoned in the Moon are unchanged.
- `Effect::StunSelf(n)`, `When::YouCastSubtype(sub)`; `UntapSelf` and `UntapTarget` now respect stun counters (CR 122.1d: a stun counter comes off instead).
- `Condition::CastCreatureAndNoncreatureThisTurn` with `Game::cast_creature_noncreature`.
- `Effect::DelayedOnAttackThisTurn(trigger)` with `Game::delayed_on_attack`: the source's trigger fires each time you attack for the rest of the turn.
- `Activated::exile_creature_card`: "Exile a creature card from your graveyard" as a cost, chosen with `ActivateWith(source, n, card)`.

| Card | Test |
|---|---|
| Stalwart Successor | `stalwart_successor_adds_a_counter_the_first_time_each_turn`, `stalwart_successor_counts_entering_with_counters_and_stun_counters` |
| Embermouth Sentinel | `embermouth_sentinel_puts_a_basic_on_top_or_onto_the_battlefield_with_a_dragon` |
| Seize Opportunity | `seize_opportunity_exiles_two_to_play_or_pumps_up_to_two` |
| Overwhelming Surge | `overwhelming_surge_does_one_or_both` |
| Salt Road Skirmish | `salt_road_skirmish_kills_and_makes_hasty_warriors_that_leave_at_end_of_turn` |
| Sonic Shrieker | `sonic_shrieker_burns_heals_and_a_burned_player_discards` |
| Fresh Start | `fresh_start_shrinks_and_strips_abilities_but_keeps_the_creature` |
| Ambling Stormshell | `ambling_stormshell_stuns_itself_to_draw_and_turtles_untap_it` |
| Eshki Dragonclaw | `eshki_dragonclaw_rewards_a_creature_and_a_noncreature_spell` |
| Dalkovan Encampment | `dalkovan_encampment_makes_warriors_whenever_you_attack_this_turn` |
| Great Arashin City | `great_arashin_city_exiles_a_creature_card_for_a_spirit` |

## Batch 5 (engine core): granted abilities, last-known counters, dies under your control, rares (10)

Mechanics added (appended at the end of their enums and structs):
- `Activated::granted_to_attached`: an Aura's "Enchanted creature has \"{cost}: ...\"". The enchanted creature's controller activates it (`Game::granted_activations`), with the creature in the stack item's `extra`; Clarion Conqueror and ability loss on either one stop it. `Effect::UntapExtra`, `Effect::TapAttached`, `CardDef::enchanted_doesnt_untap`.
- `Effect::GrantDiesUnderYourControl { slot, trigger }` with `Game::dies_under_control`: a delayed trigger only if it dies under the controller of the spell that set it up, this turn.
- `Object::last_counters` (+1/+1, stun and keyword counters when it left the battlefield); stun counters now reset when a permanent leaves the battlefield. `When::CreatureYouControlWithCountersDies` (the dead creature in `extra`) and `Effect::PutDeadCountersOn(slot)`.
- `CardDef::mana_only_for_creatures`; `Effect::GrantCombatDamageTrigger { slot, trigger }` with `Game::granted_combat_damage` (until end of turn).
- `Effect::BounceSpellOrPermanent(slot)`; a 1/1 Monk token with prowess (`Tokens::monk`, appended last).
- `Effect::MaySacrificeNonland` with `Decision::SacrificeNonland` (any nonland permanent you control, itself included), then a reflexive trigger.
- `SearchFor::Colored(c)`, `SearchTo::Graveyard`.
- `Effect::DiscardHandDrawPerSpells` (an optional ability: "you may discard your hand").
- `Condition::TotalToughnessAtLeast(n)` and `Effect::BetorEndStep` (each later threshold rechecked after the step before).
- `CardDef::doubles_damage_of_new_creatures` (Neriv), in `damage_doubling` alongside the opponent doubler.

| Card | Test |
|---|---|
| Ringing Strike Mastery | `ringing_strike_mastery_taps_locks_and_grants_an_untap_for_five` |
| Desperate Measures | `desperate_measures_draws_two_when_it_dies_under_your_control` |
| Host of the Hereafter | `host_of_the_hereafter_moves_a_dead_creatures_counters`, `host_of_the_hereafter_moves_stun_and_keyword_counters_too` |
| Herd Heirloom | `herd_heirloom_mana_is_for_creatures_and_it_grants_trample_and_card_draw` |
| Jeskai Revelation | `jeskai_revelation_bounces_a_spell_burns_makes_monks_draws_and_gains` |
| Felothar, Dawn of the Abzan | `felothar_may_sacrifice_a_nonland_permanent_for_counters_on_your_team` |
| Lotuslight Dancers | `lotuslight_dancers_bins_a_black_a_green_and_a_blue_card` |
| Narset, Jeskai Waymaster | `narset_may_discard_your_hand_to_draw_per_spell_cast` |
| Betor, Kin to All | `betor_draws_untaps_and_halves_by_total_toughness` |
| Neriv, Heart of the Storm | `neriv_doubles_damage_from_creatures_that_entered_this_turn` |

Edge case, left as is: if Ringing Strike Mastery leaves the battlefield while its granted untap ability is on the stack, the ability still untaps the creature (it was recorded on activation).

## Needs core

Cards from the lane's list, in list order, skipped because the engine can't do them exactly yet. (Sultai Monument, Fleeting Effigy, Encroaching Dragonstorm, Misty Rainforest and Arid Mesa were done by the lead in Tarkir batch 8.)

- Rite of Renewal: no "shuffle target cards from a graveyard into their library", and no "exile this spell" on resolution.
- Breaching Dragonstorm: no exile-until-nonland with cast-free-if-MV-8-or-less; no self-bounce.
- Reverberating Summons: no "becomes a creature if you've cast two or more spells this turn"; no "discard your hand" cost.
- Hundred-Battle Veteran: no static condition counting kinds of counters; no cast-from-graveyard with a finality counter.
- Sage of the Skies: no cast trigger that copies the spell itself.
- Magmatic Hellkite: no "nonbasic land" filter; no opponent search with a stun counter on the land.
- Stillness in Motion: no library-empty check, self-exile, or put-five-from-graveyard-on-top.
- New Way Forward: no prevention of the next damage from a chosen source with a reflexive redirect.
- Teval, Arbiter of Virtue: no delve granted to your spells; no lose-life-equal-to-mana-value cast trigger.
- Shiko, Paragon of the Way: no exile-then-cast-a-copy-free from the graveyard.
- Elspeth, Storm Slayer: `doubles_tokens_and_counters` also doubles counters (Elspeth doubles tokens only); no "gain flying until your next turn".
- Stormscale Scion: no storm.
- Call the Spirit Dragons: no per-color counter placement on Dragons or win condition; no static granting indestructible to Dragons.

- Emergent Ultimatum: no search for up to three monocolored cards with different names into exile, no opponent's choice to shuffle one back, no cast-the-rest-free, and no self-exile.

Round 2 (after the chunk 1 merge, e5c03c9): the cards left on the list after Inspired Ultimatum
(Misty Rainforest, Arid Mesa, Emergent Ultimatum, Ruinous Ultimatum) all need core, and every
earlier card on the list was already assessed. No new cards were scripted; the list is used up.
