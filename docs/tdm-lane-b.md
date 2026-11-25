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

## Needs core

Cards from the lane's list, in list order, skipped because the engine can't do them exactly yet.

- Rescue Leopard: no `When` for "whenever this creature becomes tapped".
- Embermouth Sentinel: the search destination depends on controlling a Dragon (top of library vs battlefield tapped); no condition-dependent `SearchTo` and no negated condition.
- Seize Opportunity: no effect for "exile the top two, you may play *those cards*" (both, not choose one); `ExileTopChooseOneToPlay` picks one.
- Tempest Hawk: no `SearchFor` by card name; no deckbuilding exemption ("any number of cards named ...").
- Trade Route Envoy: no condition "you control a creature with a counter on it", and no draw-else-counter effect.
- Wingspan Stride: no effect that returns the source permanent from the battlefield to its owner's hand (`ReturnSelfToHand` only works from the graveyard).
- Ringing Strike Mastery: no "enchanted creature doesn't untap" static, and no way for an Aura to grant an activated ability ("{5}: Untap this creature").
- Fresh Start: the lose-all-abilities static forces a base P/T (`base_pt: None` becomes 0/0); needs "loses all abilities" without setting base P/T, plus -5/-0.
- Dragonstorm Forecaster: no `SearchFor` by card name (Dragonstorm Globe or Boulderborn Dragon).
- Karakyk Guardian: no "hexproof as long as it hasn't dealt damage yet" (needs a has-dealt-damage flag and a static condition).
- Sultai Monument: `SearchFor::BasicLand` can't restrict land type (basic Swamp, Forest, or Island); the activated ability is otherwise fine.
- Sonic Shrieker: "if a player is dealt damage this way, they discard": `Who::TargetPlayer` resolves a creature target to its controller, so the discard can't be limited to a player target that was dealt damage.
- Lie in Wait: no damage "equal to that card's power" dealt by the spell itself (`Bite` makes the returned card the damage source).
- Iridescent Tiger: no "if you cast it" condition on an enters trigger.
- Fleeting Effigy: no effect that returns the source from the battlefield to its owner's hand (end-step bounce).
- Rite of Renewal: no "shuffle target cards from a graveyard into their library", and no "exile this spell" on resolution.
- Encroaching Dragonstorm: no self-bounce from the battlefield (When a Dragon you control enters); search n=2 is fine.
- Breaching Dragonstorm: no exile-until-nonland with cast-free-if-MV-8-or-less; no self-bounce.
- Reverberating Summons: no "becomes a creature if you've cast two or more spells this turn"; no "discard your hand" cost.
- Overwhelming Surge: no "choose one or both" modal spells; no "noncreature artifact" filter.
- Salt Road Skirmish: no tokens that gain haste and are sacrificed at the next end step without being tapped and attacking (`Mobilize` makes them attacking).
- Effortless Master: no "if you've cast two or more spells this turn" condition for `enters_counters`.
- Hundred-Battle Veteran: no static condition counting kinds of counters; no cast-from-graveyard with a finality counter.
- Desperate Measures: `GrantDiesTrigger` fires whoever controls the creature when it dies; needs "dies under your control".
- Host of the Hereafter: no "if it had counters, put its counters on" a target (moving counters from a dead creature).
- Unrooted Ancestor: no effect that taps the source ("Tap it.").
- Stalwart Successor: no trigger for "the first time counters are put on a creature you control this turn".
- Strategic Betrayal: no "target opponent exiles a creature they control" (sacrifice exists, exile-of-choice doesn't).
- Ambling Stormshell: no stun counters on the source; no "Whenever you cast a Turtle spell" trigger.
- Eshki Dragonclaw: no condition "cast both a creature spell and a noncreature spell this turn".
- Lotuslight Dancers: no search for one card of each of three colors into the graveyard.
- Sage of the Skies: no cast trigger that copies the spell itself.
- Magmatic Hellkite: no "nonbasic land" filter; no opponent search with a stun counter on the land.
- Stillness in Motion: no library-empty check, self-exile, or put-five-from-graveyard-on-top.
- Herd Heirloom: no creature-spell-only any-color mana; no granted "deals combat damage to a player, draw" trigger.
- Narset, Jeskai Waymaster: no "discard your hand, draw equal to spells cast this turn".
- New Way Forward: no prevention of the next damage from a chosen source with a reflexive redirect.
- Clarion Conqueror: no static that stops activated abilities of artifacts, creatures and planeswalkers.
- Dalkovan Encampment: no "enters tapped unless you control a Swamp or a Mountain"; no delayed "whenever you attack this turn" trigger.
- Great Arashin City: no "enters tapped unless ..."; no "exile a creature card from your graveyard" cost.
- Glacierwood Siege: no "as this enters, choose Temur or Sultai" modes; no play-lands-from-graveyard.
- Barrensteppe Siege: no "choose Abzan or Mardu" modes; no each-opponent-sacrifices with a died-under-your-control condition.
- Yathan Roadwatcher: no "if you cast it" condition; no reflexive reanimation with a mana-value filter after a mill.
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
- Misty Rainforest: no "pay 1 life" cost; `SearchFor` can't name land types (Forest or Island).
- Arid Mesa: no "pay 1 life" cost; `SearchFor` can't name land types (Mountain or Plains).

- Emergent Ultimatum: no search for up to three monocolored cards with different names into exile, no opponent's choice to shuffle one back, no cast-the-rest-free, and no self-exile.
- Ruinous Ultimatum: no "destroy all nonland permanents your opponents control" (only `DestroyAllCreatures` exists).

Round 2 (after the chunk 1 merge, e5c03c9): the cards left on the list after Inspired Ultimatum
(Misty Rainforest, Arid Mesa, Emergent Ultimatum, Ruinous Ultimatum) all need core, and every
earlier card on the list was already assessed. No new cards were scripted; the list is used up.
