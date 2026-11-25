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
