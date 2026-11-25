//! Playgroup decks (Nerd Leagues), lane C: the cards for Glarb, Calamity's Augur (DonSpider) first, then
//! ~/bolt-handoff/playgroup_all.json's card_order. Card text: assets/PG_oracle.json (in the CMD pool).
//! Tests: tests/pg_c.rs.
#![allow(unused_imports)]

use super::*;
use bolt_core::EntryDoubler;

fn dual(d: &mut CardDef, a: Color, b: Color) {
    d.mana = Some(ManaSource::of(&[a, b]));
}

/// "Cycling {cost}": "{cost}, Discard this card: Draw a card." from your hand.
fn cycling(d: &mut CardDef, cost: &str) {
    let n = activated(d, cost, false, vec![], vec![draw(1)]);
    d.activated[n].from_hand_discard = true;
}

/// A surveil land: enters tapped, surveils 1, taps for either color.
fn surveil_land(d: &mut CardDef, a: Color, b: Color) {
    d.enters_tapped = true;
    on(d, When::Enters, vec![], vec![Effect::Surveil { n: 1 }]);
    dual(d, a, b);
}

const SURVEIL_LAND: &[&str] = &["This land enters tapped.", "When this land enters, surveil 1."];
const SHOCK_LAND: &[&str] = &["As this land enters, you may pay 2 life. If you don't, it enters tapped."];

fn shock_land(d: &mut CardDef, a: Color, b: Color) {
    d.shock_land = true;
    dual(d, a, b);
}

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        // ---- Glarb, Calamity's Augur (DonSpider) ----
        "Bloodstained Mire" => s(
            &["{T}, Pay 1 life, Sacrifice this land: Search your library for a Swamp or Mountain card, put it onto the battlefield, then shuffle."],
            |d, _| fetch_land(d, &["Swamp", "Mountain"]),
        ),
        "Polluted Delta" => s(
            &["{T}, Pay 1 life, Sacrifice this land: Search your library for an Island or Swamp card, put it onto the battlefield, then shuffle."],
            |d, _| fetch_land(d, &["Island", "Swamp"]),
        ),
        "Windswept Heath" => s(
            &["{T}, Pay 1 life, Sacrifice this land: Search your library for a Forest or Plains card, put it onto the battlefield, then shuffle."],
            |d, _| fetch_land(d, &["Forest", "Plains"]),
        ),
        "Drowned Catacomb" => s(&["This land enters tapped unless you control an Island or a Swamp.", "{T}: Add {U} or {B}."], |d, _| {
            d.enters_tapped_unless = Some(["Island", "Swamp"]);
            dual(d, Color::Blue, Color::Black);
        }),
        "Sunken Hollow" => s(&["This land enters tapped unless you control two or more basic lands."], |d, _| {
            d.enters_tapped_unless_basics = 2;
            dual(d, Color::Blue, Color::Black);
        }),
        "Undergrowth Stadium" => s(&["This land enters tapped unless you have two or more opponents.", "{T}: Add {B} or {G}."], |d, _| {
            d.enters_tapped = true; // a two-player game: one opponent
            dual(d, Color::Black, Color::Green);
        }),
        "Undercity Sewers" => s(SURVEIL_LAND, |d, _| surveil_land(d, Color::Blue, Color::Black)),
        "Underground Mortuary" => s(SURVEIL_LAND, |d, _| surveil_land(d, Color::Black, Color::Green)),
        "Hedge Maze" => s(SURVEIL_LAND, |d, _| surveil_land(d, Color::Green, Color::Blue)),
        "Zagoth Triome" => s(&["This land enters tapped.", "Cycling {3}"], |d, _| {
            d.enters_tapped = true;
            d.mana = Some(ManaSource::of(&[Color::Black, Color::Green, Color::Blue]));
            cycling(d, "{3}");
        }),
        "Breeding Pool" => s(SHOCK_LAND, |d, _| shock_land(d, Color::Green, Color::Blue)),
        "Overgrown Tomb" => s(SHOCK_LAND, |d, _| shock_land(d, Color::Black, Color::Green)),
        "Watery Grave" => s(SHOCK_LAND, |d, _| shock_land(d, Color::Blue, Color::Black)),
        "Damnation" => s(&["Destroy all creatures. They can't be regenerated."], |d, _| {
            // The engine has no regeneration.
            spell(d, vec![], vec![Effect::DestroyAllCreatures])
        }),
        "Exploration" => s(&["You may play an additional land on each of your turns."], |d, _| d.extra_land_drops = 1),
        "Azusa, Lost but Seeking" => s(&["You may play two additional lands on each of your turns."], |d, _| d.extra_land_drops = 2),
        "Aesi, Tyrant of Gyre Strait" => s(
            &["You may play an additional land on each of your turns.", "Landfall — Whenever a land you control enters, you may draw a card."],
            |d, _| {
                d.extra_land_drops = 1;
                may(d, When::LandYouControlEnters, vec![], vec![draw(1)]);
            },
        ),
        "Glarb, Calamity's Augur" => s(
            &[
                "You may look at the top card of your library any time.",
                "You may play lands and cast spells with mana value 4 or greater from the top of your library.",
                "{T}: Surveil 2.",
            ],
            |d, _| {
                // Looking at the top card is information only: no rules effect.
                d.lands_from_top = true;
                d.top_spells_min_mana_value = Some(4);
                activated(d, "", true, vec![], vec![Effect::Surveil { n: 2 }]);
            },
        ),
        "Oracle of Mul Daya" => s(
            &[
                "You may play an additional land on each of your turns.",
                "Play with the top card of your library revealed.",
                "You may play lands from the top of your library.",
            ],
            |d, _| {
                d.extra_land_drops = 1;
                d.lands_from_top = true; // revealing it is information only
            },
        ),
        "Traveling Chocobo" => s(
            &[
                "You may look at the top card of your library any time.",
                "You may play lands and cast Bird spells from the top of your library.",
                "If a land or Bird you control entering the battlefield causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.",
            ],
            |d, _| {
                d.lands_from_top = true;
                d.top_spells_subtype = Some("Bird");
                d.entering_triggers_again = Some(EntryDoubler::LandOrSubtype("Bird"));
            },
        ),
        "Crucible of Worlds" => s(&["You may play lands from your graveyard."], |d, _| d.lands_from_graveyard = true),
        "Icetill Explorer" => s(
            &[
                "You may play an additional land on each of your turns.",
                "You may play lands from your graveyard.",
                "Landfall — Whenever a land you control enters, mill a card.",
            ],
            |d, _| {
                d.extra_land_drops = 1;
                d.lands_from_graveyard = true;
                on(d, When::LandYouControlEnters, vec![], vec![Effect::Mill { who: Who::You, n: 1 }]);
            },
        ),
        "Ancient Greenwarden" => s(
            &[
                "You may play lands from your graveyard.",
                "If a land entering causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.",
            ],
            |d, _| {
                d.lands_from_graveyard = true;
                d.entering_triggers_again = Some(EntryDoubler::Land);
            },
        ),
        "Yarok, the Desecrated" => s(
            &["If a permanent entering causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time."],
            |d, _| d.entering_triggers_again = Some(EntryDoubler::Permanent),
        ),
        "Seedborn Muse" => s(&["Untap all permanents you control during each other player's untap step."], |d, _| {
            d.untaps_on_others_untap = true
        }),
        "Lumra, Bellow of the Woods" => s(
            &[
                "Lumra's power and toughness are each equal to the number of lands you control.",
                "When Lumra enters, mill four cards. Then return all land cards from your graveyard to the battlefield tapped.",
            ],
            |d, _| {
                d.pt_lands = true;
                on(d, When::Enters, vec![], vec![Effect::Mill { who: Who::You, n: 4 }, Effect::ReturnLandsFromGraveyardTapped]);
            },
        ),
        "Ob Nixilis, the Fallen" => s(
            &["Landfall — Whenever a land you control enters, you may have target player lose 3 life. If you do, put three +1/+1 counters on Ob Nixilis."],
            |d, _| {
                may(d, When::LandYouControlEnters, vec![Filter::PLAYER], vec![Effect::LoseLife { who: Who::TargetPlayer(0), n: 3 }, Effect::CountersSelf { n: 3 }]);
            },
        ),
        "Doom Whisperer" => s(&["Pay 2 life: Surveil 2."], |d, _| {
            let n = activated(d, "", false, vec![], vec![Effect::Surveil { n: 2 }]);
            d.activated[n].life = 2;
        }),
        "Matoya, Archon Elder" => s(&["Whenever you scry or surveil, draw a card."], |d, _| on(d, When::YouScryOrSurveil, vec![], vec![draw(1)])),
        "Lightning Greaves" => s(&["Equipped creature has haste and shroud.", "Equip {0}"], |d, _| {
            attached(d, false, 0, 0, Keywords::HASTE.with(Keywords::SHROUD));
            equip(d, "{0}");
        }),
        "Assassin's Trophy" => s(
            &["Destroy target permanent an opponent controls. Its controller may search their library for a basic land card, put it onto the battlefield, then shuffle."],
            |d, _| {
                let theirs = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, controller: Controller::Opponent, ..Filter::ANY };
                spell(d, vec![theirs], vec![Effect::DestroyControllerMayFetchBasic(0)]);
            },
        ),
        "Cyclonic Rift" => s(&["Return target nonland permanent you don't control to its owner's hand.", "Overload {6}{U}"], |d, _| {
            spell(d, vec![OPP_NONLAND], vec![Effect::Bounce(0)]);
            // Overload {6}{U} is cast as a {5} kicker on {1}{U}: the same total, and "kicked" swaps in
            // the overloaded text ("each" instead of "target").
            d.kicker = ManaCost::parse("{5}");
            d.kicked_spell = Some(Ability::new(vec![], vec![Effect::BounceOpponentsNonland]));
        }),
        "Drag to the Roots" => s(
            &[
                "Delirium — This spell costs {2} less to cast as long as there are four or more card types among cards in your graveyard.",
                "Destroy target nonland permanent.",
            ],
            |d, _| {
                d.reduction = Some(Reduction::Delirium(2));
                let any = Filter { controller: Controller::Any, ..OPP_NONLAND };
                spell(d, vec![any], vec![Effect::Destroy(0)]);
            },
        ),
        "Rite of Replication" => s(
            &["Kicker {5}", "Create a token that's a copy of target creature. If this spell was kicked, create five of those tokens instead."],
            |d, _| {
                d.kicker = ManaCost::parse("{5}");
                let copy = Effect::CopyToken { slot: Some(0), haste_and_sacrifice: false, extra_subtype: None };
                spell(d, vec![CREATURE], vec![copy]);
                d.kicked_spell = Some(Ability::new(vec![CREATURE], vec![copy; 5]));
            },
        ),
        "Avenger of Zendikar" => s(
            &[
                "When this creature enters, create a 0/1 green Plant creature token for each land you control.",
                "Landfall — Whenever a land you control enters, you may put a +1/+1 counter on each Plant creature you control.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::CreateTokensPerLand { def: t.plant }]);
                may(d, When::LandYouControlEnters, vec![], vec![Effect::CountersOnYourSubtype("Plant")]);
            },
        ),
        "Scute Swarm" => s(
            &["Landfall — Whenever a land you control enters, create a 1/1 green Insect creature token. If you control six or more lands, create a token that's a copy of this creature instead."],
            |d, t| on(d, When::LandYouControlEnters, vec![], vec![Effect::InsectOrCopyWithSixLands { def: t.insect_green }]),
        ),
        "Wayward Swordtooth" => s(
            &[
                "Ascend",
                "You may play an additional land on each of your turns.",
                "This creature can't attack or block unless you have the city's blessing.",
            ],
            |d, _| {
                d.ascend = true;
                d.extra_land_drops = 1;
                d.fights_only_with_blessing = true;
            },
        ),
        "Retreat to Coralhelm" => s(
            &["Landfall — Whenever a land you control enters, choose one —", "• You may tap or untap target creature.", "• Scry 1."],
            |d, _| {
                on_modal(
                    d,
                    When::LandYouControlEnters,
                    None,
                    vec![up_to(vec![CREATURE], vec![Effect::TapOrUntap(0)]), Ability::new(vec![], vec![Effect::Scry { n: 1 }])],
                )
            },
        ),
        "Guardian Project" => s(
            &["Whenever a nontoken creature you control enters, if it doesn't have the same name as another creature you control or a creature card in your graveyard, draw a card."],
            |d, _| {
                let it = Filter { triggering: true, ..CREATURE };
                on(d, When::AnotherNontokenCreatureYouControlEnters, vec![it], vec![Effect::DrawIfUniqueName(0)]);
            },
        ),
        "Dryad of the Ilysian Grove" => s(
            &["You may play an additional land on each of your turns.", "Lands you control are every basic land type in addition to their other types."],
            |d, _| {
                d.extra_land_drops = 1;
                d.lands_every_basic_type = true;
            },
        ),
        "Clever Impersonator" => s(&["You may have this creature enter as a copy of any nonland permanent on the battlefield."], |d, _| {
            d.enter_as_copy = true;
            d.enter_as_copy_nonland = true;
        }),
        "Teval, the Balanced Scale" => s(
            &[
                "Whenever Teval attacks, mill three cards. Then you may return a land card from your graveyard to the battlefield tapped.",
                "Whenever one or more cards leave your graveyard, create a 2/2 black Zombie Druid creature token.",
            ],
            |d, t| {
                on(d, When::Attacks, vec![], vec![Effect::Mill { who: Who::You, n: 3 }, Effect::ReturnLandFromGraveyardTapped]);
                on(d, When::CardsLeaveYourGraveyard, vec![], vec![Effect::CreateToken { who: Who::You, def: t.zombie_druid, n: 1 }]);
            },
        ),
        _ => None,
    }
}

/// Lane C's scripted playgroup cards (checked against their text in `lib.rs`).
pub const PG_C_SCRIPTED: &[&str] = &[
    "Bloodstained Mire",
    "Polluted Delta",
    "Windswept Heath",
    "Drowned Catacomb",
    "Sunken Hollow",
    "Undergrowth Stadium",
    "Undercity Sewers",
    "Underground Mortuary",
    "Hedge Maze",
    "Zagoth Triome",
    "Breeding Pool",
    "Overgrown Tomb",
    "Watery Grave",
    "Damnation",
    "Exploration",
    "Azusa, Lost but Seeking",
    "Aesi, Tyrant of Gyre Strait",
    "Glarb, Calamity's Augur",
    "Oracle of Mul Daya",
    "Traveling Chocobo",
    "Crucible of Worlds",
    "Icetill Explorer",
    "Ancient Greenwarden",
    "Yarok, the Desecrated",
    "Seedborn Muse",
    "Lumra, Bellow of the Woods",
    "Ob Nixilis, the Fallen",
    "Doom Whisperer",
    "Matoya, Archon Elder",
    "Lightning Greaves",
    "Assassin's Trophy",
    "Cyclonic Rift",
    "Drag to the Roots",
    "Rite of Replication",
    "Avenger of Zendikar",
    "Scute Swarm",
    "Wayward Swordtooth",
    "Retreat to Coralhelm",
    "Guardian Project",
    "Dryad of the Ilysian Grove",
    "Clever Impersonator",
    "Teval, the Balanced Scale",
];
