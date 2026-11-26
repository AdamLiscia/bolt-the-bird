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

## Batch 6 (engine core): copies of permanent spells, storm, finality, and more rares (7)

Mechanics added (appended at the end of their enums and structs):
- `When::YouCastThis` ("When you cast this spell"), fired as the spell is put on the stack, with a cast snapshot kept for it; the amount is the number of spells cast before it this turn by any player (storm, CR 702.40a), fixed as it's cast.
- `Effect::CopySourceSpell { storm }`; a resolving copy of a permanent spell (`StackKind::SpellCopy`) becomes a token copy (CR 707.10f), not doubled by token doublers since it isn't created.
- `Filter::nonbasic`; `SearchTo::BattlefieldTappedStunned`; `Effect::DestroyLandControllerFetchesStunned(slot)` (that land's controller searches).
- `Effect::StillnessCheck` with `Decision::PutOnTopFromGraveyard` (picked one at a time; the last picked is on top).
- `Object::finality` (exiled instead of dying; counts as a counter) and `CardDef::cast_from_graveyard_finality` (offered as `CastFromGraveyard`, sorcery timing). `StaticCondition::CounterKindsAmongYoursAtLeast(n)` with `Game::counter_kinds` (+1/+1, stun, finality, each keyword counter, and the other counter kinds the engine tracks).
- `CardDef::doubles_tokens` (`Game::token_doubling`, used by every token maker, including copy tokens) and `Effect::GrantAllUntilYourNextTurn` with `Game::until_your_next_turn`.
- `Effect::SpiritDragonsUpkeep` with `Decision::DragonForColor`: a Dragon of each color in WUBRG order (skipped if you have none of it), then a win with five different Dragons (unless the opponent can't lose).

| Card | Test |
|---|---|
| Sage of the Skies | `sage_of_the_skies_copies_itself_after_another_spell` |
| Stormscale Scion | `stormscale_scion_storms_and_pumps_other_dragons` |
| Magmatic Hellkite | `magmatic_hellkite_wrecks_a_nonbasic_and_gives_them_a_stunned_basic` |
| Stillness in Motion | `stillness_in_motion_mills_and_refills_an_empty_library` |
| Hundred-Battle Veteran | `hundred_battle_veteran_returns_with_finality_and_counts_counter_kinds` |
| Elspeth, Storm Slayer | `elspeth_doubles_tokens_grows_and_lifts_the_team_and_kills_big_creatures` |
| Call the Spirit Dragons | `call_the_spirit_dragons_protects_dragons_and_wins_with_five` |

Known limit: Host of the Hereafter's last-known counters (+1/+1, stun, keyword) don't include a finality counter.

## Batch 7 (engine core): delayed "this turn" triggers, Saga chapters, free casts, prevention shields (7)

Mechanics added (appended at the end of their enums and structs):
- `Effect::DelayedThisTurn { when, trigger }` with `Game::delayed_this_turn` (cleared each turn): `When::CreatureYouControlEntersThisTurn` (a delayed-only event) and `When::YouCastNoncreature`.
- `LookRest::BottomAnyOrderTakeOne` with `Decision::BottomOrder` (each card picked goes under the one before; one must be taken).
- `Effect::MayDiscardThen` with `Decision::MayDiscardOrStop`, then a reflexive trigger; `Filter::creature_or_land` for graveyard cards.
- `Effect::ExileUntilNonlandCastFree { max_mv }` through the free-cast window, with `Game::free_casts_to_hand` (not cast: into your hand).
- `Effect::BecomeCreatureUntilEot` with `Object::eot_creature` (a creature, base P/T, a creature type and keywords this turn only, in addition to its other types); `When::BeginCombatEachTurn`; `Activated::discard_hand`.
- `Effect::PreventNextFromChosenSource` with `Decision::ChooseDamageSource` and `Game::damage_shields` (checked in `deal_damage` for damage to a player), then a reflexive trigger with the amount and the source; `Effect::DamageExtrasControllerAndDrawAmount`.
- Scavenger Regent is left for the lead: casting an X Omen needs a new `Action` (a model contract), so no ward—discard or -X/-X code was kept.

| Card | Test |
|---|---|
| Thunder of Unity | `thunder_of_unity_draws_then_drains_for_creatures_entering_that_turn` |
| Rediscover the Way | `rediscover_the_way_digs_and_then_gives_double_strike_on_noncreature_spells` |
| Awaken the Honored Dead | `awaken_the_honored_dead_destroys_mills_and_trades_a_discard_for_a_card` |
| Bloomvine Regent | `bloomvine_regent_gains_life_for_dragons_and_its_omen_fetches_forests` |
| Breaching Dragonstorm | `breaching_dragonstorm_casts_the_first_nonland_free_or_hands_it_over` |
| Reverberating Summons | `reverberating_summons_becomes_a_hasty_monk_after_two_spells`, `reverberating_summons_cashes_in_your_hand_for_two_cards` |
| New Way Forward | `new_way_forward_turns_prevented_damage_back_on_its_source` |

## Batch 8 (engine core): during your turn, cards leaving your graveyard (7)

Armament Dragon, Yathan Tombguard and Dragonbroods' Relic were done by lane C (its versions kept at the merge).

Mechanics added (appended at the end of their enums and structs):
- A `Scope::This` static with `base_pt` (and no ability loss) sets the creature's base power and toughness; with `your_turn_only`, only during its controller's turn (Snowmelt Stag). "During your turn, this creature has first strike" was already a `your_turn_only` static.
- Cards leaving a graveyard: `Game::graveyard_left_this_turn` and `Game::graveyard_leave_pending` (set in `move_to` for a card, not a token, leaving its owner's graveyard during its owner's turn). `Game::flush_graveyard_leaves`, run as triggers are put on the stack, fires `When::CardsLeaveYourGraveyardDuringYourTurn` once for everything that left since the last time, and `When::CardLeavesYourGraveyardOnceEachTurn` at most once each turn per source (`Object::once_each_turn`). Limit: two separate instructions in one resolution that each move cards out count as one event.
- `Activated::your_turn_only` and `Activated::if_card_left_graveyard`.
- `CardDef::your_turn_discount_per_big_creature` (Temur Battlecrier), in `spell_cost`.
- Removed batch 7's unused ward—discard and -X/-X code.

| Card | Test |
|---|---|
| Snowmelt Stag | `snowmelt_stag_is_5_2_on_your_turn_and_can_slip_past_blockers` |
| Bearer of Glory | `bearer_of_glory_has_first_strike_on_your_turn_and_pumps_the_team` |
| Kishla Skimmer | `kishla_skimmer_draws_once_each_turn_when_a_card_leaves_your_graveyard` |
| Essence Anchor | `essence_anchor_surveils_and_makes_zombies_after_a_card_leaves_your_graveyard` |
| Kheru Goldkeeper | `kheru_goldkeeper_and_attuned_hunter_reward_cards_leaving_your_graveyard` |
| Attuned Hunter | `kheru_goldkeeper_and_attuned_hunter_reward_cards_leaving_your_graveyard`, `attuned_hunter_counts_cards_leaving_at_once_as_one_event` |
| Temur Battlecrier | `temur_battlecrier_discounts_your_spells_per_big_creature_on_your_turn` |

## Needs core

Cards from the lane's list, in list order, skipped because the engine can't do them exactly yet. (Sultai Monument, Fleeting Effigy, Encroaching Dragonstorm, Misty Rainforest and Arid Mesa were done by the lead in Tarkir batch 8.)

- Rite of Renewal: no "shuffle target cards from a graveyard into their library", and no "exile this spell" on resolution.
- Teval, Arbiter of Virtue: no delve granted to your spells; no lose-life-equal-to-mana-value cast trigger.
- Shiko, Paragon of the Way: no exile-then-cast-a-copy-free from the graveyard.

- Emergent Ultimatum: no search for up to three monocolored cards with different names into exile, no opponent's choice to shuffle one back, no cast-the-rest-free, and no self-exile.

Round 2 (after the chunk 1 merge, e5c03c9): the cards left on the list after Inspired Ultimatum
(Misty Rainforest, Arid Mesa, Emergent Ultimatum, Ruinous Ultimatum) all need core, and every
earlier card on the list was already assessed. No new cards were scripted; the list is used up.
- Scavenger Regent: an X cost on an Omen half needs a new `Action` and a change to the Python action encoding; it also needs ward—discard.

## BLC 2026-11-25

Lane B's half of `~/bolt-handoff/blc_lanes_2026-11-25.json` ("B", 76 names), worked top-down.

### Batch 8: 12 cards

Thriving Grove, Game Trail (the lead's `enters_tapped_unless_reveal`), Talisman of Resilience
(`pain_colored` on an artifact), Unnatural Growth, Siege-Gang Commander, Thopter Engineer, Devilish
Valet, Steelburr Champion, Prosperous Bandit, Curiosity Crafter, Triskaidekaphile, Murmuration.

Core:
- `Activated::sacrifice_subtype` ("Sacrifice a Goblin"; the source counts if it has the type),
  activated with `Action::ActivateWith` like `sacrifice_token`.
- `Scope::YourArtifactCreatures`; `Condition::HandSizeExactly`.
- `When::OpponentCastsNoncreature`, `When::YourCreatureTokenDealsCombatDamageToPlayer`.
- `Effect::DoublePowerSelf`, `CreateTokensAmountTapped`, `WinTheGame`, `CreateTokensPerSpellCastThisTurn`.
- Tokens: Thopter (1/1 flying artifact), Storm Crow (1/2 flying Bird).

### Batch 9: 11 cards

Golgari Rot Farm, Gruul Turf, Loyal Warhound, Warstorm Surge, Rites of Flourishing, Garruk, Cursed
Huntsman, Plaguecrafter, Chaos Warp, Decree of Pain, Spore Frog, Riot Control.

Core:
- Karoos: `paid_mana` with an empty cost; `Effect::ReturnLandYouControl` (`Decision::ReturnOwnLand`).
- `Effect::DamageEqualSlotPower` (a triggering creature hits any target, last-known power),
  `ActivePlayerDraws` with `When::EachDrawStep` (fired for both players' permanents),
  `CardDef::extra_land_drop_each_player`.
- Garruk: Wolf token, `Effect::LoyaltyEachYouControlSubtype`, `EmblemKind::GarrukAnthem`.
- `Effect::SacrificeCreatureOrPlaneswalkerElseDiscard` (`Decision::SacrificeCreatureOrPlaneswalker`):
  Plaguecrafter asks you, then your opponent (not simultaneous; nothing is hidden, so the order
  only matters for who sees whose choice).
- `Effect::ChaosWarp`, `DestroyAllCreaturesDrawPerDestroyed`; `When::YouCycleThis` fired from the
  graveyard as a cycling cost is paid.
- `Game::combat_damage_prevented_turn` (Spore Frog) and `damage_to_player_prevented_turn` (Riot
  Control), checked at the top of `deal_damage`; `Effect::GainLifePerOpponentCreature`.
- `Condition::OpponentControlsMoreLands`.

### Batch 10: 11 cards

Scurry of Squirrels, Faeburrow Elder, Beledros Witherbloom, Gilded Goose, Jacked Rabbit, Selvala,
Explorer Returned, Swarmyard Massacre, Skullclamp, Outpost Siege, Tainted Wood, Llanowar Loamspeaker.

Core:
- `Scale::PerColorAmongYourPermanents` (Faeburrow's mana is the existing `vivid_mana`).
- `Activated::once_each_turn` (`Object::activated_turn`); `Effect::UntapLandsYouControl`; Pest token.
- `Condition::XAtLeast` (ravenous's draw), `Effect::CreateTokensPower`.
- `Effect::SelvalaParley`, `ShrinkAllWithoutSubtypesPerYours`, `SlotBecomesCreatureUntilEot`.
- `When::EquippedDies` (Equipment still attached when the creature hits the graveyard) and
  `When::CreatureYouControlLeavesBattlefield`.
- `CardDef::colored_mana_only_if_you_control` (Tainted Wood keeps its {C}).

Notes: Scurry's myriad does nothing in a two-player game (no opponent other than the defending
player). Gilded Goose's Food ability and Selvala's parley are mana abilities played on the stack, as
Hazel's is. Outpost Siege's Khans card is offered as a pick (the existing `ExileTopChooseOneToPlay`
with one card): declining is never better than taking it.

### Batch 11: 9 cards

Peerless Recycling, Octomancer, Rowdy Research, Jolrael, Mwonvuli Recluse, Skyfisher Spider,
Pollywog Prodigy, Mangara, the Diplomat, Hoofprints of the Stag, Chitterspitter.

Core:
- Gift as a free kicker (`kicker = {0}`, cast with `Action::CastKicked`; `kicked_spell` or
  `Condition::Kicked`). Octopus token; `Filter::entered_this_turn`.
- `Reduction::PerCreatureAttackedThisTurn` (`Game::attacked_creatures_this_turn`).
- `Effect::YourCreaturesBasePtHandSize` (via `eot_creature`), `GainLifePerCreatureCardInGraveyard`,
  `ExileSourceFromGraveyard`, `Evolve`, `KindCounterSelf`, `MaySacrificeToken` (`Decision::SacrificeToken`).
- `Object::kind_counters` (hoofprint, acorn), `Activated::remove_kind_counters`, `Scale::PerKindCounter`.
- `When::OpponentCastsNoncreatureMvBelowPower` (filtered as it triggers), `OpponentAttacksWithAtLeastTwo`
  (two players: every attacker attacks you or your planeswalkers), `OpponentCastsSecondSpell`.
- Elemental 4/4 flying token.

Notes: evolve compares only as it resolves (not also as it triggers).

### Batch 12: 4 cards

Teapot Slinger, Helm of the Host, Tempt with Discovery, Wildsear, Scouring Maw.

Core:
- Expend: `Game::mana_spent_on_spells` (reset each turn), `When::YouExpend(n)` fired for each total
  passed as a spell's cost is paid.
- `Object::not_legendary` (read by `types_of`; the legend rule now uses `types_of` for both
  permanents) and `Effect::CopyAttachedNonlegendaryHaste`.
- Tempting offer: `SearchFor::Land`, `Effect::TemptOpponentsLand` (`Decision::TemptOffer`, the
  opponent's land search on Yes) and `SearchLandIfTempted` (`Game::tempt_accepted`).
- Cascade: `When::YouCastEnchantmentFromHand` and `Effect::Cascade`; the card hit is offered in the
  free-cast window, and `Game::free_casts_to_bottom` puts it on the bottom if not cast.

### Paused

Paused on 2026-11-25 for the playgroup (Xenagos) list. Left on the BLC list:
- Not attempted, need core: Path of Ancestry (spent-mana tracking), Alchemist's Talent and
  Gourmand's Talent (Class levels plus granted abilities / Treasure-mana tracking), Baird (attack
  tax), Bloodroot Apothecary (toxic, poison), Bootleggers' Stash (granted land ability), Calamity of
  Cinders (convoke), Chatterfang (forestwalk, token replacement), Communal Brewing (counters on a cast
  creature), Esika's Chariot (crew), Forgotten Ancient (moving counters), Grothama, Illusionist's
  Gambit, Insatiable Frugivore, Martial Impetus (goad), Moonstone Eulogist and Ravenous Squirrel
  ("whenever you sacrifice"), Mosswort Bridge (hideaway), Pyreswipe Hawk (control for as long as),
  Rain of Riches, Rootcast Apprenticeship (repeated modes), Sunbird's Invocation, Tenuous Truce
  (enchant opponent), The Odd Acorn Gang (granted Squirrel ability), Wolfwillow Haven.
- Not claimable: Brightcap Badger // Fungus Frolic and Dusk // Dawn (the oracle data has only the
  front face).
- Staged but not applied: Wizard Class and Kodama of the East Tree (`/tmp/laneb_batch13.py`).

## Playgroup: Xenagos (Adam), 2026-11-25

The "B_xenagos" list in `~/bolt-handoff/pg_lanes_round0.json` (58 cards), in `scripts/pg_b.rs` and
`tests/pg_b.rs`.

### PG batch 1: 19 cards

Birds of Paradise, Elvish Mystic, Lotus Petal, Nature's Lore, Three Visits, Skyshroud Claim,
Kodama's Reach, Wooded Foothills, Aggravated Assault, Seize the Day, Grim Monolith, Hull Breach,
Heroic Intervention, Fire-Lit Thicket, Ancient Tomb, Fiery Emancipation, Goblin Anarchomancer, Elder
Gargaroth, Fierce Empath.

Core: `CardDef::mana_tap_damage` (Ancient Tomb, both payment paths), `triples_your_damage`,
`red_green_spell_discount`; `Effect::GrantAllPermanentsYouControl`; `SearchFor::CreatureMvAtLeast`;
`When::Blocks` (fired once per blocker as blocks are finished).

### PG batches 2 and 3: 25 cards

Thornspire Verge, Reflecting Pool, Stomping Ground, Kessig Wolf Run, Xenagos, God of Revels, Nylea, God
of the Hunt, Nylea, Keen-Eyed, Old Gnawbone, Balefire Dragon, Dragonborn Champion, Deus of Calamity,
Kogla, the Titan Ape, Kogla and Yidaro, Malignus, Questing Beast, The Great Henge, Zilortha, Strength
Incarnate, Vandalblast; Bloodthirster, Karlach, Fury of Avernus, Combat Celebrant, Hellkite Tyrant,
Yavimaya, Cradle of Growth, Quartzwood Crasher, Ilharg, the Raze-Boar.

Core:
- Devotion (`Game::devotion`, CR 700.5) and `CardDef::creature_unless_devotion` (read by `types_of`).
- `Scope::OtherYourCreatures`, `Scope::EachLand` (with a static subtype: Yavimaya's Forests;
  `CardDef::lands_are_forests` adds {G} to lands with plain mana).
- Mana: `extra_color_if_you_control` (Verge), `mana_own_land_colors` (Reflecting Pool),
  `mana_tap_life` (Great Henge). Stomping Ground uses lane C's `CardDef::shock_land` (my own shock
  field was dropped in the merge with tarkir d506a28).
- Damage: `damage_cant_be_prevented`, `your_combat_damage_cant_be_prevented` (both skip the
  turn-scoped prevention and protection in `deal_damage`), `When::YourSourceDealsFiveOrMoreToPlayer`,
  `When::DealsSixOrMoreToOpponent`, `When::YourCreatureDealsCombatDamageToPlayerAmount`,
  `When::YourTramplersDealCombatDamageToPlayer`, `lethal_damage_by_power` (SBA and combat assignment).
- Combat: `Effect::AdditionalCombatAfterThisPhase` (`Game::combats_after_this_phase`: EndCombat goes
  straight to BeginCombat), `Game::combats_this_turn` and `Condition::FirstCombatOfTurn`,
  `CardDef::cant_attack_twice_per_turn` (two players), exert (`Object::exerted_turn`, using
  lane C's `skip_untap`), `not_blocked_by_power_at_most`.
- `Effect::GainControlOfOpponentsArtifacts` (for good), `Condition::ControlArtifactsAtLeast`,
  `PutCreatureFromHandAttackingUntilEnd` (`Game::return_to_hand_at_end`), `SourceThirdFromTop`.
- Vandalblast's overload is an optional {4} on top of {R} (`Action::CastKicked`), the same total.

Notes: Combat Celebrant's exert is chosen as its attack trigger resolves rather than as it attacks.

### PG batch 4: 4 cards

Chaos Imps, Silvos, Rogue Elemental, Yavimaya Hollow, Triumph of the Hordes.

Core:
- Unleash (`CardDef::unleash`, `Decision::Unleash`), asked as the spell resolves like the Sieges'
  modes; "can't block while it has a +1/+1 counter" is a `CountersAtLeast(1)` static in its script.
- Regeneration (CR 701.15): `Object::regen_shields`, `Effect::RegenerateSelf` / `Regenerate`; used by
  `destroy` and by the lethal-damage state-based action. `Effect::DestroyNoRegen` for "It can't be
  regenerated" (Putrefy, Rapid Hybridization and Decree of Pain now use it; their old "the engine
  has no regeneration" notes are gone).
- Infect until end of turn (`Game::infect_until_eot`, `Effect::PumpAllGrantInfect`): damage to
  players is poison (`Player::poison`, ten loses, CR 704.5c), to creatures -1/-1 counters (taken
  off the +1/+1 count). Infect isn't a printed keyword the engine knows.

### PG batch 5: 3 cards

Savage Beating, Anzrag, the Quake-Mole, Fanatic of Rhonas.

Core: `CardDef::entwine` (the last mode is all of them, for the entwine cost more) and
`cast_only_during_your_combat`; `When::BecomesBlocked`, `Effect::MustBeBlockedThisTurnSelf`
(`Game::must_be_blocked`, checked in `blocks_are_legal`: an untapped creature able to block it must);
`CardDef::ferocious_mana_amount`; eternalize (`Effect::EternalizeSelf`, `Object::eternalized`: black,
no mana cost for devotion).

### PG batch 6: 2 cards

Moraug, Fury of Akoum, Defiler of Vigor.

Core:
- `Object::attacks_turn` (times declared as an attacker this turn) and
  `CardDef::power_per_attack_this_turn` (+1/+0 each, in `power`).
- `Effect::AdditionalCombatAfterMainUntap`: an `extra_combats` entry marked in
  `Game::extra_combats_untap_only`; that combat untaps the active player's creatures as it begins and
  is followed by what came next, with no additional main phase (`Game::combat_untap_only`). The
  untap is done as the combat begins rather than as a delayed trigger on the stack.
- Defiler of Vigor: `CardDef::green_permanents_pay_life_for_g` offers `Action::CastAlternative` for a
  green permanent spell with a {G} in its cost (cards without their own alternative cost): {G} less
  and 2 life as it's cast. `When::YouCastGreenPermanent`.

### PG batch 7: World at War

`Effect::AdditionalCombatAfterSecondMainUntapAttackers` (`Game::combats_after_main2`: the turn's
second main phase goes on to a combat, which untaps the creatures that attacked this turn as it
begins, then a main phase) and rebound (`Effect::Rebound`, `Game::rebound`: cast from hand, exiled
as it resolves; offered in the free-cast window at its owner's next upkeep, as suspend is).

### PG batch 8: Roaming Throne

`CardDef::doubles_chosen_type_triggers` (with the existing `choose_creature_type`): the Throne has its
chosen type (`has_base_subtype`), and `Game::double_for_thrones`, run as waiting triggers are put on
the stack, adds one more copy of each trigger of another creature its controller controls of that
type. `Game::triggered_scanned` keeps each waiting trigger from being doubled twice.

### Xenagos: not scripted

- Deflecting Swat: no way to choose new targets for a spell or ability already on the stack.
- Ojer Kaslem, Deepest Growth // Temple of Cultivation and Vorinclex // The Grand Evolution:
  transforming double-faced cards; the back faces aren't in the card data, and the engine has no
  transform.

### Xenagos finish 2026-11-25

Deflecting Swat, Ojer Kaslem, Deepest Growth // Temple of Cultivation, Vorinclex // The Grand
Evolution: Xenagos's deck is complete.

Data: `scripts/fetch_oracle.py` keeps a `back` face (name, mana_cost, type_line, oracle_text, power,
toughness, colors, loyalty) for `transform` and `modal_dfc` layouts, and `--refresh NAME...` re-fetches
only those cards in place. The two DFCs were refreshed in `assets/PG_oracle.json`, then CMD rebuilt
(`make_cmd_set.py`). Other DFCs in PG have no `back` yet: refresh them as they're scripted.

Transform (CR 701.28, 712):
- `Oracle::back` / `BackFace`; `CardDef::back: Option<Box<CardDef>>` is built from the back face
  (keyword lines become its keywords); its other lines must be claimed by the front's script as
  `"Back: <line>"`, and the script builds the back through `d.back`. Color identity counts the back.
- `Object::transformed`; `Game::def_of` returns the back face while it's set, so types, P/T,
  abilities, mana and triggers all follow (`types_of` and `has_base_subtype` now go through
  `def_of`). Cleared when it leaves the battlefield, dropping leave triggers collected from the back
  face (as for copies).
- `Effect::TransformSelf`, `Effect::ReturnSelfTappedTransformed`, `Effect::ExileSelfReturn {
  transformed }`. `Activated::min_permanents` ("Activate only if you control ten or more permanents").
- Not done: copying a transformed permanent copies its front; transforming doesn't re-register
  static abilities (neither card has one).

Changing targets (CR 115.7): `Filter::abilities` (an ability on the stack is targeted as its source:
the topmost one), `Effect::ChangeTargets(slot)`, `Decision::ChooseNewTarget { item, slot }`
answered with `Target` or `No`. New targets must pass the same checks as when chosen (`candidates`,
`slot_ok`), can't be the current one, one another slot holds, or the spell itself. Ward isn't
re-checked for a new target. `CardDef::alt_cost_needs_commander` (Swat's free cast).

Also: `Effect::RevealAmountPutCreatureAndLand`, `Effect::MillPutCreatures`,
`Decision::PutPicksOntoBattlefield`; `Effect::GrantFightAbilityEot` with `Game::granted_fight`: the
granted "{1}: fights target creature you don't control" is activated as ability number
`activated.len()` of the creature (`Game::activated_of`), so no new action kinds.

bolt-py: `transformed` on battlefield objects; `ChooseNewTarget` (`card`, `slot`, `current` /
`current_player`) and `PutPicksOntoBattlefield` (`cards`, `left`) decision details. Service: `face`,
`back` on view objects, `back_name` / `back_image` in `/v1/cards` (docs/api.md).

### Glarb: clones and Zimone (2026-11-25)

Phyrexian Metamorph, Sakashima of a Thousand Faces, Sakashima the Impostor, Zimone, Mystery
Unraveler (scripts in pg_c.rs, tests in tests/pg_c.rs, in one lane B block).

- Phyrexian mana: `CardDef::phyrexian` (pips by color; `definition()` parses `{U/P}` as `{U}` and
  counts it). `Action::CastAlternative` pays every Phyrexian pip with 2 life.
- Copy exceptions on `EnterAsCopy` (`Object::copy_exceptions`, read from the clone's own card in
  `printed_def` via `Game::copy_printed`): `copy_artifact` (artifact-or-creature options, artifact
  in addition), `copy_only_yours`, `copy_keeps_name` (`Game::name_of`, legendary, used by the legend
  rule), `copy_keeps_abilities` (`no_legend_rule`, and `Activated::copy_only` abilities, activated
  past the copied card's own as `Game::activated_of` numbers them). `CardDef::no_legend_rule`.
  `Effect::ReturnSelfToHandAtNextEnd`.
- Face down (CR 708): `Object::face_down`; `def_of` returns `face_down_def()` (a 2/2 with no name,
  text or color); cleared when it leaves the battlefield. `Effect::ManifestDreadFirstElseTurnFaceUp`
  (Zimone, counting `resolved_this_turn`), `Decision::ManifestDread`, `Decision::TurnFaceUp`.
  Turning a manifested creature card face up for its mana cost is offered as `Action::Activate(id, 0)`
  and is a special action (no stack). bolt-py hides the card (-2) from the other player in the
  observation and the event log; the service shows "Face-down creature".
- Not done: the play model's own features (bolt-py encoders other than the observation) aren't
  masked; nothing triggers on "turned face up"; Phyrexian pips can't be split between life and mana.

### Aesi (Time), lane B, batch 1 (2026-11-25)

18 of the 25: Eternal Witness, Negate, Mana Leak, Scatter Ray, Frantic Search, Courser of Kruphix,
Basalt Monolith, Myriad Landscape, Silverback Elder, Horn of Greed, Simic Growth Chamber, Emerald
Medallion, Jaheira's Respite, Sprouting Vines, Sword of the Animist, Tectonic Edge, Tolaria West,
Zimone, Quandrix Prodigy. None of the list is a double-faced card in the pool (all `normal` layout), so
no back faces were fetched.

Core: `CardDef::green_spell_discount`; `Activated::opponent_min_lands`;
`Filter::spell_artifact_or_creature`; `SearchFor::BasicLandSharingType` (after the first find, the rest
must share one of its basic land types) and `SearchFor::ManaValue` (transmute, as a from-hand discard
activation at sorcery speed); `When::PlayerPlaysLand` (fired for both players' permanents as a land is
played; Horn of Greed draws for the active player, the only one who plays lands);
`Effect::UntapLandsUpTo` (Frantic Search untaps your tapped lands, most colors first, rather than
asking), `MayPutLandFromHandTapped` with `Decision::LandFromHandTapped`, `DrawMoreIfLands`,
`SearchBasicsPerAttackersTapped`.

### Aesi (Time), lane B, batch 2 (2026-11-25)

The other 7: Rhystic Study, Mystic Remora, Arcane Denial, Perennial Behemoth, Walk the Aeons, Nissa,
Who Shakes the World, Blessing of Frost. Lane B's 25 are done.

- "Unless that player pays": `Effect::UnlessOpponentPays(n)` asks the opponent
  `Decision::PayToStop`; paying stops the rest of the effects. The "you may" is the trigger's
  optional, asked first.
- Cumulative upkeep: `Effect::CumulativeUpkeep(n)` adds an age counter (`kind_counters`) and asks
  `Decision::PayUpkeep` (pay, or sacrifice it).
- Arcane Denial: `Effect::NextUpkeepDraws` (read before the counter) fills
  `Game::next_upkeep_draws`, drawn as the next upkeep begins. "Up to two" is always two.
- Unearth: `Effect::Unearth`, `Object::unearthed` (exiled at the next end step, and instead of
  leaving the battlefield), haste through `suspend_haste`.
- Buyback by sacrifice: `CardDef::buyback_sacrifice` (cast with `CastKicked`, `kicker` set to {0};
  the tapped permanents of the subtype are sacrificed first); it returns to hand as it resolves.
  `Effect::ExtraTurnFor(slot)`.
- Nissa: `CardDef::forests_add_g` (each Forest offers an extra {G} in `untapped_sources`; the manual
  `TapForMana` path doesn't add it), `Effect::AnimateLandElemental`, `EmblemKind::LandsIndestructible`.
- Snow: `Types::snow`; `Game::snow_paid` counts mana from snow sources as a spell is paid for;
  `CardDef::x_is_snow_spent` stores it as the entry's X. `Effect::DistributeCountersX` with
  `Decision::DistributeCounters` (one counter at a time, not targeted),
  `Effect::DrawPerCreaturePowerAtLeast`.

### Myrkul, Lord of Bones (DonSpider), batch 1 (2026-11-25)

25 of the deck's 45: Temple Garden, Godless Shrine, Vault of Champions, Bountiful Promenade, Caves of
Koilos, Wooded Bastion, City of Brass, Despark, Anguished Unmaking, Anointed Procession, Vanquish the
Horde, Smothering Tithe, Setessan Champion, Eidolon of Blossoms, Grim Guardian, Archon of Sun's
Grace, Ghostly Dancers, Sanctum Weaver, Serra's Sanctum, Grand Abolisher, Devoted Druid, Yawgmoth,
Thran Physician, Myrkul, Lord of Bones, Maha, Its Feathers Night, Esper Sentinel.

Core: `Recipient::You`; `When::EnchantmentYouControlEnters` (constellation, the entering enchantment
included); `Pick::Enchantment`; `CardDef::mana_per_enchantment` (Sanctum Weaver's X may split
across colors rather than one color), `opponents_cant_activate_your_turn` (Grand Abolisher: no
abilities, mana ones included, of opponents' artifacts, creatures and enchantments during your
turn), `protection_from_subtype`, `indestructible_at_half_life` (`Game::starting_life`),
`opponents_creatures_base_toughness_one` (applied after other base P/T); `Activated::minus_counter_cost`,
`discard_cost` (`ActivateWith` the discarded card). -1/-1 counters are negative `Object::counters`
(`Effect::MinusCounter`), so they cancel +1/+1 counters. `Effect::Proliferate` chooses for you: your
permanents with +1/+1 counters, your planeswalkers, opponents' permanents with -1/-1 counters.
Myrkul: `Effect::ExileMakeEnchantmentCopy`, `Object::copy_enchantment_only`; dying nontoken creatures
fill a `triggering` target for "another nontoken creature you control dies" abilities. Esper
Sentinel: `Effect::UnlessOpponentPaysPower`, checked against `Game::noncreature_casts` as it
resolves. Ghostly Dancers: the engine has no Rooms, so only the return half. Tokens: Pegasus
(base+61) and Spirit 3/1 (base+62); base+59 and +60 are placeholders for lane C's Wurm and Satyr.

### Myrkul batch 2 (2026-11-25)

11 more (36 of 45): Austere Command, Dawn's Truce, Eldritch Evolution, Birthing Pod, Enduring
Vitality, Jaheira, Friend of the Forest, Arachnogenesis, For the Common Good, Mirari's Wake, Kaya,
Geist Hunter, The Golden Throne.

- Austere Command's "choose two" is one mode for each pair (6 modes, `CastModal`), each two
  `Effect::DestroyAllMatching`.
- Dawn's Truce: gift as `CastKicked`; `Effect::YouGainHexproofEot` (`Game::player_hexproof_turn`).
- `Effect::SearchCreatureBySacrificedMv` (the sacrificed creature in the entry's `extra`: a spell's
  additional cost, or an ability's with `Activated::victim_in_extra`); `SearchFor::CreatureMvExactly`.
- Phyrexian mana in activated costs: `Activated::phyrexian`; the engine pays the mana when it can and
  2 life a pip otherwise (Birthing Pod).
- `CardDef::creatures_tap_any_color`, `tokens_tap_for_green`, `lands_add_same` (Mirari's Wake: each
  land tapped for mana offers one more of what it makes), `lose_replacement` (The Golden Throne, in
  the state-based loss check; its sacrifice-a-creature mana ability uses the stack here).
- Enduring Vitality: `Effect::ReturnSelfAsEnchantment` (`Object::last_enchantment_only` for "if it
  was a creature").
- Arachnogenesis: `Effect::CreateTokensPerAttackers`, `PreventCombatDamageExceptSubtype`
  (`Game::combat_damage_prevented_except`).
- For the Common Good: `Effect::CopyTokenX`, `GrantTokensUntilYourNextTurn`, `GainLifePerToken`.
- Kaya: `Effect::DoubleTokensThisTurn` (`Game::tokens_doubled_turn`), `ExileGraveyardsTokenPerCard`.

### Myrkul batch 3 (2026-11-25)

Mondrak, Glory Dominus, Exalted Sunborn, Grim Haruspex, Black Market Connections (40 of 45).

- `Activated::sacrifice_two_others` (the second is the engine's choice: a token, else the lowest
  mana value), `Effect::KeywordCounterSelf`; Mondrak's {W/P} pips use `Activated::phyrexian`.
- Warp: `CardDef::warp`, cast with `CastAlternative`; `Object::warped` exiles it at the next end
  step with `playable` for its owner.
- Morph: `CardDef::morph`, cast face down for {3} with `CastAlternative` (`Object::face_down` from the
  cast on; revealed if it leaves the stack any other way); turned face up for the morph cost with
  the `Activate(id, 0)` special action. bolt-py hides a face-down spell on the stack too.
- `When::FirstMainPhaseYourTurn`; "choose one or more" as a mode for each nonempty set.
- `CardDef::changeling` (every creature type, in `has_subtype`). Tokens: base+63 is a placeholder for
  lane C's Phyrexian Beast, base+64 Shapeshifter 3/2 changeling.

Needs core, not done: Teferi's Protection and Clever Concealment (phasing), Promise of Loyalty (vow
counters), Sphere of Safety (attack tax), Mirkwood Bats ("whenever you sacrifice a token").

### Core theme: replacement effects, batch 1 (2026-11-25)

Cards: Hardened Scales, Bloodletter of Aclazotz, Alhammarret's Archive, Teferi's Ageless Insight,
Quina, Qu Gourmet, Force of Negation, Deepglow Skate, Bogardan Dragonheart. Tests:
tests/core_replace.rs.

- `CardDef::counters_plus_one` (in `add_counters`, creatures you control), `opponents_lose_double_your_turn`
  (in `lose_life`, so damage counts), `life_gain_doubled` (in `gain_life`, doubled for each),
  `extra_draws_doubled` (in `draw`; the draw step's first draw is marked by `Game::in_draw_step_draw`;
  `Game::draw_replacing` keeps it from replacing its own draws; two of them still draw two),
  `tokens_plus_one` / `tokens_plus_each` (in `create_token`, not applied to their own extras:
  `Game::token_replacing`; tokens made by copy effects don't get them yet).
- `Effect::CounterExile`, `DoubleEachCounter` (+1/+1 and -1/-1, loyalty, stun, kind, quest, time
  counters), `BecomeSubtypeUntilEot` (through `Object::eot_creature`).
- Tokens: base+65 to +68 are placeholders for lane C's Clue and d1's Spawns and Elf Druid; Frog is
  base+69.

Not done here: Nyxbloom Ancient (mana production), Damn (overload cost) are d1's; Peregrin Took and
Chatterfang have their replacement in place (`tokens_plus_one`, `tokens_plus_each`) but need "Sacrifice
three Foods" / "Sacrifice X Squirrels" costs (d1); Academy Manufactor waits for lane C's Clue.

### Core theme: replacement effects, batch 2 (2026-11-25)

Whip of Erebos and Words of War scripted; Dauthi Voidwalker's pieces in.

- `Effect::ReanimateUnearthed` (Whip: haste, exiled at end step or if it would leave, as unearth).
- `Effect::NextDrawDealsDamage`, `Game::draw_damage_shields` (in `draw`, before the Archive
  replacement). Words of War's target is chosen as the ability is activated, not as the draw is
  replaced.
- `CardDef::opponents_cards_exiled_with_void`, `Object::void_counter` (in `move_to`: a nontoken card
  headed for its owner's graveyard while that owner's opponent controls one is exiled with a void
  counter), `Effect::PlayVoidCardFree`, `Decision::PickVoidCard` (`Pick` or `No`). Dauthi Voidwalker
  itself waits for shadow (block legality, d1).
- Labels in service_view.py for `LandFromHandTapped`, `DistributeCounters`, `ManifestDread`,
  `TurnFaceUp`, `PickVoidCard`, `PayToStop`, `PayUpkeep`; bolt-py reports `cost` (and `card`) for
  the last two.

Not done: Valgavoth (ward—sacrifice three is a new ward kind; playing its exiled cards for life is
a cost, d1), From the Catacombs (the initiative, mac-c; escape, d1).

### The merger's three (2026-11-25)

Blood Moon, Urborg, Tomb of Yawgmoth, Thousand-Year Elixir scripted; Valgavoth's ward piece. Tests:
tests/core_merger.rs.

- `CardDef::nonbasic_lands_are_mountains`: `Game::blood_mooned` (a nonbasic land on the battlefield
  while one is out) counts as `lost_abilities`, is a Mountain and no other land type in
  `has_subtype`, and makes only {R} in `single_sources`. Lands still enter tapped by their own
  text under Blood Moon (CR 614.12 would have them enter untapped).
- `CardDef::lands_are_swamps`: `Game::urborged` (every land, while an Urborg with its ability is out):
  a Swamp in `has_subtype`, and {B} added to its mana. A Blood Mooned Urborg has no effect.
- `CardDef::creatures_abilities_haste`: in `can_tap`, so tap abilities and mana of your creatures.
- `CardDef::ward_sacrifice_nonland` with lane C's `ward_sacrifice`: lands neither count toward paying
  nor are offered (Valgavoth itself still needs its other abilities).

### Merger-delegated six (2026-11-25)

Damia, Sage of Stone, Drumbellower, Savor the Moment, Gaddock Teeg, Trusted Advisor, Jin-Gitaxias,
Core Augur. Tests: tests/core_merger.rs.

- `CardDef::skip_draw_step` (the Upkeep step goes straight to Main1), `Effect::DrawUpToHandSize`.
- `CardDef::creatures_untap_on_others_untap` (next to Seedborn Muse in the untap step).
- `Effect::ExtraTurnSkipUntap`, `Game::skip_next_untap` (the skipped step leaves the player's permanents
  tapped).
- `CardDef::teeg_lock` (at the top of `cast_modes`: noncreature with mana value 4+ or {X}).
- `CardDef::max_hand_plus`, `opponents_max_hand_minus` (in `max_hand`).
- `Effect::ReturnCreatureOfColorYouControl`, `Decision::ReturnOwnCreatureOfColor`.

### Tatyova, Benthic Druid (Grande), batch 1 (2026-11-25)

13 of 23: Migration Path, Joint Exploration, Khalni Heart Expedition, Noxious Revival, On the Trail,
Rude Awakening, Sakura-Tribe Scout, Skyshroud Ranger, Splendid Reclamation, Squall Line, Stroke of
Genius, Wipe Away, Laboratory Maniac.

Core: `Effect::MayPutLandFromHand` with `Decision::LandFromHand` (untapped), `LandsBecomeCreaturesEot`
(through `eot_creature`), `DrawXTarget`; `Recipient::FliersAndEachPlayer`; `CardDef::win_on_empty_draw`
(in `draw`, where an empty library would otherwise mark the loss). Khalni Heart Expedition's quest
counters are its own kind of counter.

Needs core (routed by the lead): Ensnare, Gush, Thwart, Harrow, Meloku, Misdirection, Trade Routes,
Kruphix (d1); Amulet of Vigor, Cloudstone Curio (d2); Misdirection's single-target filter (d3).

### Zhulodok, Void Gorger (Adam), batch 1 (2026-11-26)

11 of 70, on existing core only: Expedition Map, Strip Mine, Wasteland, Palladium Myr, Worn
Powerstone, Voltaic Key, Manifold Key, Urza's Cave, Mithril Coat, Artisan of Kozilek (annihilator as
an attack trigger, as d1 does), Fountainport. The other 59 need core, routed to the lead by theme.

### Oloro, Ageless Ascetic (Adam), batch 1 (2026-11-26)

9 of 26, on existing core: Sea of Clouds, Archangel of Thune, Cliffhaven Vampire, Drogskol Reaver,
Rhox Faithmender, Serra Ascendant, Divinity of Pride, Voice of the Blessed, Heliod, Sun-Crowned. The
rest need core (routed): Ghostly Prison, Propaganda (attack tax, d3); Paradox Haze (an extra upkeep,
merger); Mana Confluence (pay-life mana, d1); Felidar Sovereign, Test of Endurance (upkeep life
checks, d2); Oloro (a command-zone trigger, d2); Crested Sunmare, Dawn of Hope (tokens: Horse 5/5,
Soldier with lifelink); Karlov (remove counters as a cost, d1); Ajani, Angel of Destiny, Debt to the
Deathless, Lunar Convocation, Vizkopa Guildmage, Wall of Reverence, Will (d1/d2/d3).

### Ghyrson Starn, Kelermorph (Spr3adsh33t), batch 1 (2026-11-26)

19 of 50, on existing core: Basilisk Collar, Gorgon's Head, Electrostatic Field, Erebor Flamesmith,
Kessig Flamebreather, Grapeshot (its storm copies keep the original's target), The Surgical Bay, The
Autonomous Furnace, Training Center, Blisterspit Gremlin, Thermo-Alchemist, Unruly Catapult,
Gelectrode, Goblin Sharpshooter, Cast into the Fire, Consider, Serum Visions, Electrolyze, Niv-Mizzet,
Parun. The rest need core (routed in the report).

Also: Paradox Haze (merger: `CardDef::extra_upkeep_for_enchanted`, `Object::enchanted_player` for
"Enchant player" Auras, `Game::upkeeps_this_turn` and `extra_upkeeps` in the step machine), and with
tokens 70 to 75 (Horse 5/5, Soldier 1/1 lifelink, Ajani's Pridemate, Bird Illusion, Soldier 1/1
artifact, Construct 0/0 artifact): Dawn of Hope, Murmuring Mystic, Third Path Iconoclast.

Ghyrson, with d2's new triggers: Archmage Emeritus, Storm-Kiln Artist, Lobber Crew, Chandra's
Spitfire, Curiosity, Ophidian Eye.

After tarkir 973b06c: Misdirection (d1's pitch_with_life with no life, d3's single-target filter,
my ChangeTargets; keeping the target is also offered) and Mana Confluence (its mana on the stack,
as d1's Sunbaked Canyon). Five of my Ghyrson scripts were dropped because d2 had scripted the same
cards (Archmage Emeritus, Lobber Crew, Chandra's Spitfire, Curiosity, Ophidian Eye); Storm-Kiln
Artist stays.

Token 84: Young Pyromancer's 1/1 Elemental (83 is a placeholder for d2's Bat). Unified the draw
effects: d1's DrawPerCreatureYouControlOfColor stays, d3's DrawPerColoredCreatureYouControl is gone
(Regal Force and its core_effects test point at d1's). Split halves refetched (Dusk // Dawn,
Commit // Memory, Walk-In Closet // Forgotten Cellar had no `split`; every PG DFC already had its
back). Run the asset scripts with PYTHONPATH=src: the .venv's editable install otherwise resolves
ROOT to ~/bolt-the-bird.
BLC: Casualties of War (31 mode sets, as Farewell), Esika's Chariot.

Merger pieces: `CardDef::modes_once_each_turn` + `Game::modes_chosen` (Monument to Endurance),
`Effect::ExileTopToPlayThenGraveyardTokens` + `Game::exiled_until_end`, handled at the beginning of
the next end step (Glimpse the Impossible, with d1's Eldrazi Spawn), `Activated::needs_threshold`
(Barbarian Ring, Cephalid Coliseum, Cabal Pit). Tests in core_merger.rs.
More merger pieces: `CardDef::unspent_mana_becomes_colorless` in the pool emptying (Kruphix,
Horizon Stone; Omnath's green stays green) and `cast_only_during_your_end_step` (Necrologia, with
d1's pay_x_life). Mana Confluence dropped: d1 has it.
`Activated::min_attackers_this_turn` (Minas Tirith's draw: 2; only the active player has attacked),
tested on a test definition since d1 scripts the card.
BLC: Chatterfang, with `Activated::sacrifice_x_of_subtype` ("Sacrifice X Squirrels", an
ActivateX) and `Effect::PumpXMinusX`; its replacement is the existing tokens_plus_each.
Attack tax by target: `CardDef::attack_tax_player_only` (Ghostly Prison, Propaganda),
`attack_tax(defender, at_planeswalker)`, and `Game::attacking_planeswalker` set while
AttackPlaneswalker declares through Attack. Attack and AttackPlaneswalker are each offered only
when their own tax is payable. Baird (BLC) taxes both.
Merger BLC: Perch Protection (`Effect::GiftExtraTurn`; LifeLockedUntilYourNextTurn now counts
queued extra turns via `Game::next_turn_of`), Coveted Jewel (`When::OpponentCreaturesAttackYouUnblocked`,
`Effect::AttackingPlayerDrawsAndTakesSource`). Tokens 89 Blood and 90 red Hamster (85-88 placeholders);
Young Pyromancer's Elemental is now red.
Illusionist's Gambit: `CardDef::cast_only_during_opponents_declare_blockers`,
`Effect::RemoveAttackersFromCombatUntapNoAttack` and `Game::cant_attack_this_turn`, then the
existing additional combat.
Hazel's Brewmaster: `CardDef::foods_have_exiled_abilities`, `Effect::ExileCardWithSource`,
`Game::exiled_with`, and `Game::borrowed_activated` (a Food's borrowed abilities, after kept ones,
in `activated_of` and the activation list). Mana abilities aren't borrowed.
d4's rooms: Twisted Caverns (`Effect::CantAttackUntilYourNextTurn`, `Game::cant_attack_until`,
which Illusionist's Gambit now uses too), Arena (`Effect::Goad`, `Game::goaded`, `is_goaded` in
the must-attack check), Mad Wizard's Lair (`Effect::DrawThreeMayCastOneFree`, Rishkar's free-cast
window). Urza's Saga and Invert Polarity wait for d4's lacks_chapter_activated and flip_coin.
After d4 batch 4: the activation list skips `lacks_chapter_activated` abilities (Urza's Saga's
chapter grants), and Invert Polarity (`Effect::FlipGainControlOfSpellOrCounter`, d4's flip_coin;
won: the spell's controller changes and ChooseNewTarget is offered).
Turn order after extra turns (CR 500.7): `Game::normal_next` keeps the player the normal order
gives next, so a gifted extra turn for the opponent doesn't replace their own; `next_turn_of`
follows the same rule. Coveted Jewel's trigger also fires when no blocks can be declared.

d3's effects for lane B's decks (moved 2026-11-26):
- BLC: Calamity of Cinders, Realm-Cloaked Giant, Swarmyard (one ability per type), Rolling
  Hamsphere, Pyreswipe Hawk, Martial Impetus (`goads_enchanted`, When::EnchantedAttacks),
  Bootleggers' Stash and The Odd Acorn Gang (`lands_you_control_have`, `subtype_you_control_has`,
  in borrowed_activated; When::YourSubtypeDealCombatDamageToPlayer).
- Zhulodok: Ulamog, Void Winnower (`opponents_even_locked`: the printed mana value; an X spell's X
  isn't counted).
- Ghyrson: Epic Experiment (free casts; `free_casts_rest_to_graveyard`).
- BLC: Kwain (Decision::MayDrawGainOne, you then the opponent), Mr. Foxglove (FoxgloveDraw).
- Merger: Aftermath (`CardDef::aftermath`: the right half cast from the graveyard only, exiled as
  it resolves; Dusk // Dawn), Classes (`Object::class_level`, `Game::class_level`,
  `Activated::level_up_to` / `min_class_level`, `Effect::ClassLevelUp`, When::ClassBecomesLevel,
  Condition::ClassLevelAtLeast; Wizard Class). Level-gated statics still need d3's
  StaticCondition arm. Exalted Flamer dropped (d3 has it).
- Ghyrson: Metallurgic Summonings (YouCastInstantOrSorcery triggers now carry the spell as `extra`; Activated::min_artifacts).
- BLC: Cache Grab (Decision::CacheGrabPick).
- BLC: Forgotten Ancient (Decision::MoveCounterFrom).
- Merger: priority in cleanup (CR 514.3a): finish_cleanup gives priority when something triggered, and `Game::cleanup_again` makes another cleanup step follow.
- PPO blockers: ChooseTarget excludes an object only within one "target" word (a run of
  identical filters, `Game::same_word_slots`), and trigger_can_target needs as many objects as a
  word has slots (Piercing Exhale); harmonize's affordability taps its creature first, so the X
  offered is payable.
- Merger: −X loyalty (`card::LOYALTY_MINUS_X`, Decision::ChooseLoyaltyX answered with Amount);
  Ugin, the Spirit Dragon (ExileEachColoredPermanentMvAtMostX, PermanentsFromHandUpTo).
- Shield counters (`Object::shield_counters`, replacing damage and destruction) and Shield Broker (`Game::control_while_shielded`; a commander target is skipped at resolution, since the filter can't exclude it: d3).
- Merger: Bolas's Citadel (`CardDef::citadel`, `Object::citadel_cast`: free from the top, its mana
  value in life; `Activated::sacrifice_nonland_n`), Gadrak (`attacks_only_with_artifacts`,
  `Game::nontoken_creatures_died`), Xorn (`treasures_plus_one`), Silent Arbiter
  (`one_attacker_one_blocker`), Expropriate (Decision::ExpropriateVote / ExpropriatePick).
- Kozilek's Command: modal X casting (CastModal asks X first; ChooseCastX offers only X with legal targets for the mode), with d3's mana_value_at_most_x / up_to_x (up_to_x now counts only its own word's targets).
- PPO blockers, round 2: Dragon's Prey's target-dependent {2} (castability needs an affordable target; ChooseTarget offers only affordable ones), and harmonize affordability excludes its creature as payment does (a Devotee's untapped mana ability).

### Merger queue, 2026-11-26: Ugin, the Ineffable and four BLC cards

Ugin, the Ineffable (pg_b), Communal Brewing, Rootcast Apprenticeship, Insatiable Frugivore,
Bloodroot Apothecary (blc_b). Tests: tests/pg_b.rs (`ugin_the_ineffable_*`), tests/blc_b.rs.

- `CardDef::colorless_spell_discount` (in `spell_cost`, after the d1 discounts) and `CardDef::toxic`
  (in `deal_damage`: combat damage to a player adds N poison counters, CR 702.164c).
- Ugin's +1: `Effect::ExileTopFaceDownCreateLinkedToken` with `Game::token_linked_exile`; the token
  leaving the battlefield (in `zone_triggers_once`) puts the source's `When::Never` trigger
  (`Effect::LinkedExiledCardToHand`, the card in `extra` and its zcc in `amount`) on the stack, so it
  works with Ugin gone. Token 98: a colorless 2/2 Spirit (`Tokens::spirit_2`; 91-97 placeholders).
  Its −3 uses d3's existing `Filter::colored`.
- Communal Brewing: `Effect::KindCounterSelfThenPerTargetDraw` (ingredient counters are
  `kind_counters`; "any number of target opponents" is up to one) and
  `Effect::CastSpellEntersWithCountersPerKindCounter` with `Game::spell_enters_with_counters`, applied
  in the enter path next to `escaped`.
- Rootcast Apprenticeship: 20 modes (every multiset of three). `Effect::SacrificeNontokenArtifact`
  with `Decision::SacrificeNontokenArtifact` (automatic with one). **Known hack:** a mode chosen more
  than once has separate "target" words, but `Game::same_word_slots` treats a run of identical
  filters as one word (different objects). Neighbouring slots therefore alternate `Filter::other`,
  a no-op for a sorcery's targets, so three "target creature" slots may name one creature. If
  `same_word_slots` or `other` changes, this breaks (the Rootcast test covers it).
- Insatiable Frugivore: `Effect::MayExileThreeRepeatFood` with `Decision::ExileThreeForFood` (Pick
  three, then a Food and asked again; No only before the first pick), `Effect::PumpYoursXPowerGrant`
  with Chatterfang's `sacrifice_x_of_subtype("Food")`.
- Bloodroot Apothecary: `When::OpponentSacrificesNoncreatureToken` (in `Game::sacrificed`),
  `Effect::PoisonCounters`; "that player" is the opponent (1v1).

After tarkir 782b6e5: `Effect::LoseLifeCastSpellManaValue` (Teval) reads d4's `stack_mana_cost`.
Oloro, newly unblocked: Crested Sunmare (d2's `Condition::YouGainedLifeThisTurn`, no new core) and
Ajani, Strength of the Pride (`Effect::GainLifePerCreatureAndPlaneswalkerYouControl`,
`Effect::ExileSelfAndOpponentsArtifactsCreaturesIfLifeAbove(15)`, against `Game::starting_life`).
Their tokens 70 (Horse 5/5) and 72 (Ajani's Pridemate) are colorless in `token_defs`, though the
cards say white: the merger's to fix in the token block.
Built 2026-11-26: Insatiable Frugivore's `No` gets its own answer arm (it was offered but unhandled).

### Merger pieces, 2026-11-26 (second list)

- Granted ward: `CardDef::enchanted_ward` on an Aura or Equipment, read in `Game::ward_check` next to the
  creature's own ward (Sheltered by Ghosts, scripted in pg_b).
- Rankle, Master of Pranks: "choose any number" as one mode per set of the three modes (7, then the
  empty set last) through `on_modal`; no new core.
- Raphael, Tag Team Tough: `When::DealsCombatDamageToPlayerFirstTimeEachTurn` with
  `Game::combat_damaged_player`, then the existing untap-attackers and additional-combat effects.
- Fisher's Talent's token replacements: `CardDef::class_token_replacements` (level, from, to),
  applied once each in order at the top of `create_token` (`Game::class_token_replacement`). Token
  99: a blue 3/3 Shark (`Tokens::shark`); Fish (18) and Octopus (49) are now blue, as every card
  that makes them says. The card itself (its reveal-a-land Fish) waits on d3.
- There and Back Again, chapter I: `Effect::TargetCantBlockWhileYouControlSource` with
  `Game::cant_block_while_controlled`, checked first in `can_block`. The Saga itself is d4's.
- Gisa, Glorious Resurrector: `CardDef::opponents_creatures_die_exiled`, a replacement in `move_to`
  (an opponent's creature going from the battlefield to a graveyard is exiled), recorded in
  `Game::exiled_with` as (Gisa, its zcc, the card, its zcc in exile) for lane C's upkeep return.
- The Immortal Sun: `CardDef::players_cant_activate_loyalty`, checked at the top of `loyalty_actions`;
  its draw, discount and anthem were already in.
- Archfiend of Despair: `CardDef::opponents_cant_gain_life`, checked in `gain_life`; its end step is
  the existing `LoseLifeLostThisTurn`.

### Script-only decks: Kaalia, Zhulodok, Malik (2026-11-26)

Scripted: Brightclimb Pathway, Gisela, Blade of Goldnight, Sower of Discord, Eiganjo Castle, Bennie
Bracks (pg_b). Pieces (lane B's replacement theme, plus one condition):
- Gisela: `CardDef::prevents_half_damage_to_you_and_yours`; any source's damage to an opponent of
  hers or their permanents is doubled after the other doublers, and damage to you or yours has half
  prevented, rounded up (`Game::prevent_damage_replacements`).
- `Effect::PreventNextDamageToTargetThisTurn` with `Game::damage_shields_on` (Eiganjo Castle).
- Sower of Discord: `CardDef::damage_to_a_player_other_loses` (two players: both chosen), a
  `When::Never` trigger put on the stack from `deal_damage` with the other player in a triggering
  slot and the damage as the amount.
- `Condition::YouCreatedTokenThisTurn` (Bennie Bracks), from `Game::token_created_turn`.
- Malik, more: Inventors' Fair (`SearchFor::Artifact`), Smaug the Magnificent
  (`Effect::SourceDamagePerSubtypeYouControl`), Mahadi (`Effect::CreateTokenPerCreatureDiedThisTurn`
  with `Game::creatures_died`, tokens included). Small d3-style pieces nobody else was on.
