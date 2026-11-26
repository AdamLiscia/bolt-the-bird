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
            spell(d, vec![], vec![Effect::DestroyAllCreaturesNoRegen])
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
        "Greensleeves, Maro-Sorcerer" => s(
            &[
                "Protection from planeswalkers and from Wizards",
                "Greensleeves's power and toughness are each equal to the number of lands you control.",
                "Landfall — Whenever a land you control enters, create a 3/3 green Badger creature token.",
            ],
            |d, t| {
                d.protection_from_planeswalkers_and = Some("Wizard");
                d.pt_lands = true;
                on(d, When::LandYouControlEnters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.badger, n: 1 }]);
            },
        ),
        "The Gitrog Monster" => s(
            &[
                "At the beginning of your upkeep, sacrifice The Gitrog Monster unless you sacrifice a land.",
                "You may play an additional land on each of your turns.",
                "Whenever one or more land cards are put into your graveyard from anywhere, draw a card.",
            ],
            |d, _| {
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::SacrificeLandOrSource]);
                d.extra_land_drops = 1;
                on(d, When::LandCardsToYourGraveyard, vec![], vec![draw(1)]);
            },
        ),
        "Colossal Grave-Reaver" => s(
            &[
                "Whenever this creature enters or attacks, mill three cards.",
                "Whenever one or more creature cards are put into your graveyard from your library, put one of them onto the battlefield.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::Mill { who: Who::You, n: 3 }]);
                on(d, When::Attacks, vec![], vec![Effect::Mill { who: Who::You, n: 3 }]);
                on(d, When::CreatureCardsMilledToYourGraveyard, vec![], vec![Effect::MilledCreatureToBattlefield]);
            },
        ),
        "Wrenn and Seven" => s(
            &[
                "+1: Reveal the top four cards of your library. Put all land cards revealed this way into your hand and the rest into your graveyard.",
                "0: Put any number of land cards from your hand onto the battlefield tapped.",
                "−3: Create a green Treefolk creature token with reach and \"This token's power and toughness are each equal to the number of lands you control.\"",
                "−8: Return all permanent cards from your graveyard to your hand. You get an emblem with \"You have no maximum hand size.\"",
            ],
            |d, t| {
                loyalty(d, 1, Ability::new(vec![], vec![Effect::RevealTopLandsToHandRestGraveyard(4)]));
                loyalty(d, 0, Ability::new(vec![], vec![Effect::LandsFromHandTapped]));
                loyalty(d, -3, Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.treefolk, n: 1 }]));
                loyalty(d, -8, Ability::new(vec![], vec![Effect::ReturnPermanentCardsToHand, Effect::Emblem(EmblemKind::NoMaxHandSize)]));
            },
        ),
        "Nissa of Shadowed Boughs" => s(
            &[
                "Landfall — Whenever a land you control enters, put a loyalty counter on Nissa.",
                "+1: Untap target land you control. You may have it become a 3/3 Elemental creature with haste and menace until end of turn. It's still a land.",
                "−5: You may put a creature card with mana value less than or equal to the number of lands you control onto the battlefield from your hand or graveyard with two +1/+1 counters on it.",
            ],
            |d, _| {
                on(d, When::LandYouControlEnters, vec![], vec![Effect::LoyaltySelf(1)]);
                let land = Filter { players: false, creatures: false, planeswalkers: false, lands: true, controller: Controller::You, ..Filter::ANY };
                let become_ = Effect::SlotBecomesCreatureUntilEot {
                    slot: 0,
                    power: 3,
                    toughness: 3,
                    subtype: "Elemental",
                    keywords: Keywords::HASTE.with(Keywords::MENACE),
                };
                loyalty(d, 1, Ability::new(vec![land], vec![Effect::UntapTarget(0), Effect::MayContinue, become_]));
                loyalty(d, -5, Ability::new(vec![], vec![Effect::CreatureFromHandOrGraveyardMvAtMostLands]));
            },
        ),
        "Delighted Halfling" => s(
            &[
                "{T}: Add {C}.",
                "{T}: Add one mana of any color. Spend this mana only to cast a legendary spell, and that spell can't be countered.",
            ],
            |d, _| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                d.any_color_for_legendary = true;
            },
        ),
        "Force of Vigor" => s(
            &[
                "If it's not your turn, you may exile a green card from your hand rather than pay this spell's mana cost.",
                "Destroy up to two target artifacts and/or enchantments.",
            ],
            |d, _| {
                d.pitch_not_your_turn = Some(4);
                let mut a = up_to(vec![ARTIFACT_OR_ENCHANTMENT, ARTIFACT_OR_ENCHANTMENT], vec![Effect::Destroy(0), Effect::Destroy(1)]);
                a.min_targets = Some(0);
                d.spell = Some(a);
            },
        ),
        "Dread Return" => s(
            &["Return target creature card from your graveyard to the battlefield.", "Flashback—Sacrifice three creatures."],
            |d, _| {
                spell(d, vec![graveyard_cards(true, Controller::You)], vec![Effect::Reanimate(0)]);
                d.flashback = Some(ManaCost::default());
                d.flashback_sacrifice = 3;
            },
        ),
        "Scapeshift" => s(
            &["Sacrifice any number of lands. Search your library for up to that many land cards, put them onto the battlefield tapped, then shuffle."],
            |d, _| spell(d, vec![], vec![Effect::Scapeshift]),
        ),
        "Rise of the Witch-king" => s(
            &["Each player sacrifices a creature of their choice. If you sacrificed a creature this way, you may return another permanent card from your graveyard to the battlefield."],
            |d, _| {
                spell(d, vec![], vec![Effect::SacrificeCreatures { who: Who::EachOpponent, n: 1 }, Effect::SacrificeThenReturnOther])
            },
        ),
        "Will of the Abzan" => s(
            &[
                "Choose one. If you control a commander as you cast this spell, you may choose both instead.",
                "• Any number of target opponents each sacrifice a creature with the greatest power among creatures that player controls and lose 3 life.",
                "• Return target creature card from your graveyard to the battlefield.",
            ],
            |d, _| {
                // One opponent in a two-player game.
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                let card = graveyard_cards(true, Controller::You);
                let punish = [Effect::SacrificeGreatestPower(Who::TargetPlayer(0)), Effect::LoseLife { who: Who::TargetPlayer(0), n: 3 }];
                d.modes = vec![
                    Ability::new(vec![opp], punish.to_vec()),
                    Ability::new(vec![card], vec![Effect::Reanimate(0)]),
                    Ability::new(vec![opp, card], [punish[0], punish[1], Effect::Reanimate(1)].to_vec()),
                ];
                d.last_mode_needs_commander = true;
            },
        ),
        "Koma, Cosmos Serpent" => s(
            &[
                "This spell can't be countered.",
                "At the beginning of each upkeep, create a 3/3 blue Serpent creature token named Koma's Coil.",
                "Sacrifice another Serpent: Choose one —",
                "• Tap target permanent. Its activated abilities can't be activated this turn.",
                "• Koma gains indestructible until end of turn.",
            ],
            |d, t| {
                d.uncounterable = true;
                on(d, When::Upkeep { your_turn_only: false }, vec![], vec![Effect::CreateToken { who: Who::You, def: t.komas_coil, n: 1 }]);
                // The mode is chosen as it's activated (CR 700.2): one ability per mode.
                let any = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, controller: Controller::Any, ..Filter::ANY };
                for (targets, effect) in [(vec![any], Effect::TapStopActivations(0)), (vec![], Effect::GrantSelf { keywords: Keywords::INDESTRUCTIBLE })] {
                    let n = activated(d, "", false, targets, vec![effect]);
                    d.activated[n].sacrifice_subtype = Some("Serpent");
                    d.activated[n].sacrifice_another = true;
                }
            },
        ),
        "Sensei's Divining Top" => s(
            &[
                "{1}: Look at the top three cards of your library, then put them back in any order.",
                "{T}: Draw a card, then put this artifact on top of its owner's library.",
            ],
            |d, _| {
                activated(d, "{1}", false, vec![], vec![Effect::LookTopReorder(3)]);
                activated(d, "", true, vec![], vec![Effect::DrawThenSourceOnTop]);
            },
        ),
        "Hedge Shredder" => s(
            &[
                "Whenever this Vehicle attacks, you may mill two cards.",
                "Whenever one or more land cards are put into your graveyard from your library, put them onto the battlefield tapped.",
                "Crew 1",
            ],
            |d, _| {
                may(d, When::Attacks, vec![], vec![Effect::Mill { who: Who::You, n: 2 }]);
                on(d, When::LandCardsMilledToYourGraveyard, vec![], vec![Effect::MilledLandsToBattlefieldTapped]);
                let n = activated(d, "", false, vec![], vec![Effect::CrewedSelf]);
                d.activated[n].crew = 1;
            },
        ),
        "Nissa, Worldsoul Speaker" => s(
            &[
                "Landfall — Whenever a land you control enters, you get {E}{E} .",
                "You may pay eight {E} rather than pay the mana cost for permanent spells you cast.",
            ],
            |d, _| {
                on(d, When::LandYouControlEnters, vec![], vec![Effect::Energy(2)]);
                d.permanents_for_energy = 8;
            },
        ),
        // ---- lane B: Glarb's clones and Zimone (copy exceptions, manifest dread) ----
        "Phyrexian Metamorph" => s(
            &["You may have this creature enter as a copy of any artifact or creature on the battlefield, except it's an artifact in addition to its other types."],
            |d, _| {
                d.enter_as_copy = true;
                d.copy_artifact = true;
            },
        ),
        "Sakashima of a Thousand Faces" => s(
            &[
                "You may have Sakashima enter as a copy of another creature you control, except it has Sakashima's other abilities.",
                "The \"legend rule\" doesn't apply to permanents you control.",
                "Partner",
            ],
            |d, _| {
                d.enter_as_copy = true;
                d.copy_only_yours = true;
                d.copy_keeps_abilities = true;
                d.no_legend_rule = true;
            },
        ),
        "Sakashima the Impostor" => s(
            &["You may have Sakashima the Impostor enter as a copy of any creature on the battlefield, except its name is Sakashima the Impostor, it's legendary in addition to its other types, and it has \"{2}{U}{U}: Return Sakashima the Impostor to its owner's hand at the beginning of the next end step.\""],
            |d, _| {
                d.enter_as_copy = true;
                d.copy_keeps_name = true;
                let n = activated(d, "{2}{U}{U}", false, vec![], vec![Effect::ReturnSelfToHandAtNextEnd]);
                d.activated[n].copy_only = true;
            },
        ),
        "Zimone, Mystery Unraveler" => s(
            &["Landfall — Whenever a land you control enters, manifest dread if this is the first time this ability has resolved this turn. Otherwise, you may turn a permanent you control face up."],
            |d, _| on(d, When::LandYouControlEnters, vec![], vec![Effect::ManifestDreadFirstElseTurnFaceUp]),
        ),
        "Jace, the Mind Sculptor" => s(
            &[
                "+2: Look at the top card of target player's library. You may put that card on the bottom of that player's library.",
                "0: Draw three cards, then put two cards from your hand on top of your library in any order.",
                "−1: Return target creature to its owner's hand.",
                "−12: Exile all cards from target player's library, then that player shuffles their hand into their library.",
            ],
            |d, _| {
                loyalty(d, 2, Ability::new(vec![Filter::PLAYER], vec![Effect::LookTopMayBottom(0)]));
                loyalty(d, 0, Ability::new(vec![], vec![Effect::DrawThenPutBack { draw: 3, back: 2 }]));
                loyalty(d, -1, Ability::new(vec![CREATURE], vec![Effect::Bounce(0)]));
                loyalty(d, -12, Ability::new(vec![Filter::PLAYER], vec![Effect::ExileLibraryShuffleHandIn(0)]));
            },
        ),
        "Scroll Rack" => s(
            &["{1}, {T}: Exile any number of cards from your hand face down. Put that many cards from the top of your library into your hand. Then look at the exiled cards and put them on top of your library in any order."],
            |d, _| {
                activated(d, "{1}", true, vec![], vec![Effect::ScrollRack]);
            },
        ),
        "Conduit of Worlds" => s(
            &[
                "You may play lands from your graveyard.",
                "{T}: Choose target nonland permanent card in your graveyard. If you haven't cast a spell this turn, you may cast that card. If you do, you can't cast additional spells this turn. Activate only as a sorcery.",
            ],
            |d, _| {
                d.lands_from_graveyard = true;
                // The card stays castable for the rest of the turn while no spell has been cast.
                let card = Filter { nonland_permanents: true, ..graveyard_cards(false, Controller::You) };
                let n = activated(d, "", true, vec![card], vec![Effect::ConduitCast(0)]);
                d.activated[n].sorcery = true;
            },
        ),
        "Legolas's Quick Reflexes" => s(
            &[
                "Split second",
                "Untap target creature. Until end of turn, it gains reach, hexproof, and \"Whenever this creature becomes tapped, it deals damage equal to its power to up to one target creature.\"",
            ],
            |d, _| {
                d.split_second = true;
                d.triggers.push(Trigger { when: When::Never, ability: up_to(vec![CREATURE], vec![Effect::ExtraDealsDamageEqualPower(0)]) });
                spell(
                    d,
                    vec![CREATURE],
                    vec![
                        Effect::UntapTarget(0),
                        Effect::Grant { slot: 0, keywords: Keywords::REACH.with(Keywords::HEXPROOF) },
                        Effect::GrantTappedTrigger { slot: 0, trigger: 0 },
                    ],
                );
            },
        ),
        // ---- Aesi, Tyrant of Gyre Strait (Time) ----
        "Swan Song" => s(
            &["Counter target enchantment, instant, or sorcery spell. Its controller creates a 2/2 blue Bird creature token with flying."],
            |d, t| {
                let f = Filter { spells_enchantment_instant_sorcery: true, ..SPELL };
                spell(d, vec![f], vec![Effect::Counter(0), Effect::CreateToken { who: Who::ControllerOf(0), def: t.bird_2, n: 1 }]);
            },
        ),
        "Biorhythm" => s(&["Each player's life total becomes the number of creatures they control."], |d, _| {
            spell(d, vec![], vec![Effect::LifeBecomesCreatureCount])
        }),
        "Sunder" => s(&["Return all lands to their owners' hands."], |d, _| spell(d, vec![], vec![Effect::BounceAllLands])),
        "Ghost Town" => s(&["{T}: Add {C}.", "{0}: Return this land to its owner's hand. Activate only if it's not your turn."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            let n = activated(d, "", false, vec![], vec![Effect::BounceSelf]);
            d.activated[n].not_your_turn_only = true;
        }),
        "Palinchron" => s(
            &["When this creature enters, untap up to seven lands.", "{2}{U}{U}: Return this creature to its owner's hand."],
            |d, _| {
                // "Up to seven lands": your own tapped ones (the engine's choice; untapping an opponent's
                // land never helps you).
                on(d, When::Enters, vec![], vec![Effect::UntapLands(7)]);
                activated(d, "{2}{U}{U}", false, vec![], vec![Effect::BounceSelf]);
            },
        ),
        "Broken Bond" => s(
            &["Destroy target artifact or enchantment. You may put a land card from your hand onto the battlefield."],
            |d, _| spell(d, vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::Destroy(0), Effect::MayLandFromHand]),
        ),
        "Collector Ouphe" => s(&["Activated abilities of artifacts can't be activated."], |d, _| d.stops_artifact_abilities = true),
        "Mox Amber" => s(&["{T}: Add one mana of any color among legendary creatures and planeswalkers you control."], |d, _| {
            d.mana_legendary_colors = true
        }),
        "Voidslime" => s(&["Counter target spell, activated ability, or triggered ability."], |d, _| {
            spell(d, vec![Filter { abilities: true, ..SPELL }], vec![Effect::CounterSpellOrAbility(0)])
        }),
        "Awaken the Woods" => s(&["Create X 1/1 green Forest Dryad land creature tokens."], |d, t| {
            spell(d, vec![], vec![Effect::CreateTokensX { def: t.forest_dryad }])
        }),
        "Sylvan Advocate" => s(
            &["As long as you control six or more lands, this creature and land creatures you control get +2/+2."],
            |d, _| {
                for scope in [Scope::This, Scope::YourLandCreatures] {
                    static_on(d, scope, false, Keywords::NONE);
                    let st = d.statics.last_mut().expect("just pushed");
                    st.power = 2;
                    st.toughness = 2;
                    st.condition = Some(StaticCondition::ControlLandsAtLeast(6));
                }
            },
        ),
        "Omnath, Locus of Mana" => s(
            &["You don't lose unspent green mana as steps and phases end.", "Omnath gets +1/+1 for each unspent green mana you have."],
            |d, _| d.keeps_green_mana = true,
        ),
        "Cyclone Summoner" => s(
            &["When this creature enters, if you cast it from your hand, return all permanents to their owners' hands except for Giants, Wizards, and lands."],
            |d, _| on_if(d, Condition::CastFromHand, vec![], vec![Effect::BounceAllExceptLandsAndSubtypes(&["Giant", "Wizard"])]),
        ),
        "Roil Elemental" => s(
            &["Landfall — Whenever a land you control enters, you may gain control of target creature for as long as you control this creature."],
            |d, _| may(d, When::LandYouControlEnters, vec![CREATURE], vec![Effect::GainControlWhileSource(0)]),
        ),
        "Yavimaya Dryad" => s(
            &["When this creature enters, you may search your library for a Forest card, put it onto the battlefield tapped under target player's control, then shuffle."],
            |d, _| may(d, When::Enters, vec![Filter::PLAYER], vec![Effect::SearchForestForPlayer(0)]),
        ),
        "Otawara, Soaring City" => s(
            &[
                "{T}: Add {U}.",
                "Channel — {3}{U}, Discard this card: Return target artifact, creature, enchantment, or planeswalker to its owner's hand. This ability costs {1} less to activate for each legendary creature you control.",
            ],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::Blue]));
                let f = Filter { players: false, artifacts: true, enchantments: true, ..Filter::ANY };
                let n = activated(d, "{3}{U}", false, vec![f], vec![Effect::Bounce(0)]);
                d.activated[n].from_hand_discard = true;
                d.activated[n].less_per_legendary_creature = true;
            },
        ),
        "Scatter to the Winds" => s(&["Counter target spell.", "Awaken 3—{4}{U}{U}"], |d, _| {
            spell(d, vec![SPELL], vec![Effect::Counter(0)]);
            // Awaken is cast as a {3} kicker on {1}{U}{U}: the same total, {4}{U}{U}.
            d.kicker = ManaCost::parse("{3}");
            let land = Filter { players: false, creatures: false, planeswalkers: false, lands: true, controller: Controller::You, ..Filter::ANY };
            d.kicked_spell = Some(Ability::new(vec![SPELL, land], vec![Effect::Counter(0), Effect::Awaken { slot: 1, n: 3 }]));
        }),
        "Three Steps Ahead" => s(
            &[
                "Spree",
                "+ {1}{U} — Counter target spell.",
                "+ {3} — Create a token that's a copy of target artifact or creature you control.",
                "+ {2} — Draw two cards, then discard a card.",
            ],
            |d, _| {
                // Spree: one mode for each combination of the + modes, each with their total cost.
                let yours = Filter { players: false, planeswalkers: false, artifacts: true, controller: Controller::You, ..Filter::ANY };
                let copy = |slot| Effect::CopyToken { slot: Some(slot), haste_and_sacrifice: false, extra_subtype: None };
                let loot = [draw(2), discard(1)];
                let cost = |c: &str| ManaCost::parse(c).expect("cost");
                d.modes = vec![
                    Ability::new(vec![SPELL], vec![Effect::Counter(0)]),
                    Ability::new(vec![yours], vec![copy(0)]),
                    Ability::new(vec![], loot.to_vec()),
                    Ability::new(vec![SPELL, yours], vec![Effect::Counter(0), copy(1)]),
                    Ability::new(vec![SPELL], vec![Effect::Counter(0), loot[0], loot[1]]),
                    Ability::new(vec![yours], vec![copy(0), loot[0], loot[1]]),
                    Ability::new(vec![SPELL, yours], vec![Effect::Counter(0), copy(1), loot[0], loot[1]]),
                ];
                d.mode_costs = ["{1}{U}", "{3}", "{2}", "{4}{U}", "{3}{U}", "{5}", "{6}{U}"].map(cost).to_vec();
            },
        ),
        "Plaza of Heroes" => s(
            &[
                "{T}: Add {C}.",
                "{T}: Add one mana of any color. Spend this mana only to cast a legendary spell.",
                "{T}: Add one mana of any color among legendary permanents you control.",
                "{3}, {T}, Exile this land: Target legendary creature gains hexproof and indestructible until end of turn.",
            ],
            |d, _| {
                d.plaza_mana = true;
                let legend = Filter { legendary_only: true, ..CREATURE };
                let n = activated(d, "{3}", true, vec![legend], vec![Effect::Grant { slot: 0, keywords: Keywords::HEXPROOF.with(Keywords::INDESTRUCTIBLE) }]);
                d.activated[n].exile_self = true;
            },
        ),
        "Bear Umbra" => s(
            &[
                "Enchant creature",
                "Enchanted creature gets +2/+2 and has \"Whenever this creature attacks, untap all lands you control.\"",
                "Umbra armor",
            ],
            |d, _| {
                d.aura = true;
                d.spell = Some(Ability::new(vec![CREATURE], vec![]));
                attached(d, false, 2, 2, Keywords::NONE);
                on(d, When::EquippedAttacks, vec![], vec![Effect::UntapLandsYouControl]);
                d.umbra_armor = true;
            },
        ),
        "Shifting Woodland" => s(
            &[
                "This land enters tapped unless you control a Forest.",
                "{T}: Add {G}.",
                "Delirium — {2}{G}{G}: This land becomes a copy of target permanent card in your graveyard until end of turn. Activate only if there are four or more card types among cards in your graveyard.",
            ],
            |d, _| {
                d.enters_tapped_unless = Some(["Forest", "Forest"]);
                d.mana = Some(ManaSource::of(&[Color::Green]));
                let card = Filter { permanent_cards: true, ..graveyard_cards(false, Controller::You) };
                let n = activated(d, "{2}{G}{G}", false, vec![card], vec![Effect::BecomeCopyOfSlotUntilEot(0)]);
                d.activated[n].needs_delirium = true;
            },
        ),
        "Khalni Ambush // Khalni Territory" => s(
            &[
                "Target creature you control fights target creature you don't control.",
                "Back: This land enters tapped.",
                "Back: {T}: Add {G}.",
            ],
            |d, _| {
                spell(d, vec![YOUR_CREATURE, OPP_CREATURE], vec![Effect::FightBetween { a: 0, b: 1 }]);
                d.modal_back = true;
                let b = d.back.as_mut().expect("Khalni Territory");
                b.enters_tapped = true;
                b.mana = Some(ManaSource::of(&[Color::Green]));
            },
        ),
        "Nissa, Vastwood Seer // Nissa, Sage Animist" => s(
            &[
                "When Nissa enters, you may search your library for a basic Forest card, reveal it, put it into your hand, then shuffle.",
                "Whenever a land you control enters, if you control seven or more lands, exile Nissa, then return her to the battlefield transformed under her owner's control.",
                "Back: +1: Reveal the top card of your library. If it's a land card, put it onto the battlefield. Otherwise, put it into your hand.",
                "Back: −2: Create Ashaya, the Awoken World, a legendary 4/4 green Elemental creature token.",
                "Back: −7: Untap up to six target lands. They become 6/6 Elemental creatures. They're still lands.",
            ],
            |d, t| {
                let forest = SearchFor::LandTypes { types: [false, false, false, false, true], basic: true };
                may(d, When::Enters, vec![], vec![search(forest, SearchTo::Hand)]);
                let mut flip = Ability::new(vec![], vec![Effect::ExileSelfReturn { transformed: true }]);
                flip.condition = Some(Condition::ControlLandsAtLeast(7));
                d.triggers.push(Trigger { when: When::LandYouControlEnters, ability: flip });
                let b = d.back.as_mut().expect("Nissa, Sage Animist");
                loyalty(b, 1, Ability::new(vec![], vec![Effect::RevealTopLandToBattlefieldElseHand]));
                loyalty(b, -2, Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.ashaya, n: 1 }]));
                let land = Filter { players: false, creatures: false, planeswalkers: false, lands: true, ..Filter::ANY };
                let mut effects = Vec::new();
                for slot in 0..6 {
                    effects.push(Effect::UntapTarget(slot));
                    effects.push(Effect::SlotBecomesCreatureForGood { slot, power: 6, toughness: 6, subtype: "Elemental" });
                }
                loyalty(b, -7, up_to(vec![land; 6], effects));
            },
        ),
        "Deadeye Navigator" => s(
            &[
                "Soulbond",
                "As long as Deadeye Navigator is paired with another creature, each of those creatures has \"{1}{U}: Exile this creature, then return it to the battlefield under your control.\"",
            ],
            |d, _| {
                // Soulbond: pair as it enters, or as another creature enters while it's unpaired.
                d.soulbond_flicker = true;
                may(d, When::Enters, vec![], vec![Effect::SoulbondPairOnEnter]);
                let it = Filter { triggering: true, ..CREATURE };
                may(d, When::AnotherCreatureYouControlEnters, vec![it], vec![Effect::SoulbondPairWith(0)]);
            },
        ),
        // ---- Etali, Primal Conqueror (RuricMo) ----
        "Green Sun's Zenith" => s(
            &["Search your library for a green creature card with mana value X or less, put it onto the battlefield, then shuffle. Shuffle Green Sun's Zenith into its owner's library."],
            |d, _| {
                spell(d, vec![], vec![search(SearchFor::GreenCreatureMvAtMostX, SearchTo::Battlefield { tapped: false })]);
                d.shuffle_on_resolve = true;
            },
        ),
        "Bane of Progress" => s(
            &["When this creature enters, destroy all artifacts and enchantments. Put a +1/+1 counter on this creature for each permanent destroyed this way."],
            |d, _| on(d, When::Enters, vec![], vec![Effect::DestroyArtifactsEnchantmentsCountersSelf]),
        ),
        "Temple of the False God" => s(&["{T}: Add {C}{C}. Activate only if you control five or more lands."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            d.mana_amount = 2;
            d.mana_only_with_lands = 5;
        }),
        "Circle of Dreams Druid" => s(&["{T}: Add {G} for each creature you control."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Green]));
            d.mana_per_creature = true;
        }),
        "Tribute to the World Tree" => s(
            &["Whenever a creature you control enters, draw a card if its power is 3 or greater. Otherwise, put two +1/+1 counters on it."],
            |d, _| {
                let it = Filter { triggering: true, ..CREATURE };
                on(d, When::AnotherCreatureYouControlEnters, vec![it], vec![Effect::DrawIfPowerThreeElseTwoCounters(0)]);
            },
        ),
        "Parallel Lives" => s(
            &["If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead."],
            |d, _| d.doubles_tokens = true,
        ),
        "Spire Garden" => s(&["This land enters tapped unless you have two or more opponents.", "{T}: Add {R} or {G}."], |d, _| {
            d.enters_tapped = true; // a two-player game: one opponent
            dual(d, Color::Red, Color::Green);
        }),
        "Worldspine Wurm" => s(
            &[
                "When this creature dies, create three 5/5 green Wurm creature tokens with trample.",
                "When Worldspine Wurm is put into a graveyard from anywhere, shuffle it into its owner's library.",
            ],
            |d, t| {
                on(d, When::Dies, vec![], vec![Effect::CreateToken { who: Who::You, def: t.wurm_5, n: 3 }]);
                on(d, When::PutIntoGraveyardFromAnywhere, vec![], vec![Effect::ShuffleSourceFromGraveyardIntoLibrary]);
            },
        ),
        "Ulamog, the Infinite Gyre" => s(
            &[
                "When you cast this spell, destroy target permanent.",
                "Annihilator 4",
                "When Ulamog is put into a graveyard from anywhere, its owner shuffles their graveyard into their library.",
            ],
            |d, _| {
                let any = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, ..Filter::ANY };
                on(d, When::CastThisSpell, vec![any], vec![Effect::Destroy(0)]);
                on(d, When::Attacks, vec![], vec![Effect::SacrificePermanents { who: Who::EachOpponent, n: 4 }]);
                on(d, When::PutIntoGraveyardFromAnywhere, vec![], vec![Effect::OwnerShufflesGraveyardIntoLibrary]);
            },
        ),
        "Lurking Predators" => s(
            &["Whenever an opponent casts a spell, reveal the top card of your library. If it's a creature card, put it onto the battlefield. Otherwise, you may put that card on the bottom of your library."],
            |d, _| on(d, When::OpponentCasts, vec![], vec![Effect::RevealTopCreatureToBattlefieldElseMayBottom]),
        ),
        "Ulamog, the Defiler" => s(
            &[
                "When you cast this spell, target opponent exiles the top half of their library, rounded up.",
                "Ward—Sacrifice two permanents.",
                "Ulamog enters with a number of +1/+1 counters on it equal to the greatest mana value among cards in exile.",
                "Ulamog has annihilator X, where X is the number of +1/+1 counters on it.",
            ],
            |d, _| {
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                on(d, When::CastThisSpell, vec![opp], vec![Effect::ExileTopHalfOfLibrary(0)]);
                d.ward_sacrifice = 2;
                d.enters_counters_greatest_exiled_mv = true;
                on(d, When::Attacks, vec![], vec![Effect::AnnihilatorPerCounters]);
            },
        ),
        "Homeward Path" => s(&["{T}: Add {C}.", "{T}: Each player gains control of all creatures they own."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            activated(d, "", true, vec![], vec![Effect::CreaturesToOwners]);
        }),
        "Purphoros, God of the Forge" => s(
            &[
                "As long as your devotion to red is less than five, Purphoros isn't a creature.",
                "Whenever another creature you control enters, Purphoros deals 2 damage to each opponent.",
                "{2}{R}: Creatures you control get +1/+0 until end of turn.",
            ],
            |d, _| {
                d.creature_unless_devotion = Some(([false, false, false, true, false], 5));
                on(d, When::AnotherCreatureYouControlEnters, vec![], vec![damage(2, Recipient::EachOpponent)]);
                activated(d, "{2}{R}", false, vec![], vec![Effect::PumpAll { who: Who::You, power: 1, toughness: 0 }]);
            },
        ),
        "Conjurer's Closet" => s(
            &["At the beginning of your end step, you may exile target creature you control, then return that card to the battlefield under your control."],
            |d, _| may(d, When::EndStep { your_turn_only: true }, vec![YOUR_CREATURE], vec![Effect::BlinkSlot(0)]),
        ),
        "Cavern-Hoard Dragon" => s(
            &[
                "This spell costs {X} less to cast, where X is the greatest number of artifacts an opponent controls.",
                "Whenever this creature deals combat damage to a player, you create a Treasure token for each artifact that player controls.",
            ],
            |d, t| {
                d.reduction = Some(Reduction::GreatestArtifactsAnOpponentControls);
                on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::TreasuresPerOpponentArtifact { def: t.treasure }]);
            },
        ),
        "Displaced Dinosaurs" => s(
            &["As a historic permanent you control enters, it becomes a 7/7 Dinosaur creature in addition to its other types."],
            |d, _| d.historic_become_dinosaurs = true,
        ),
        "Vorinclex, Voice of Hunger" => s(
            &[
                "Whenever you tap a land for mana, add one mana of any type that land produced.",
                "Whenever an opponent taps a land for mana, that land doesn't untap during its controller's next untap step.",
            ],
            // Mana abilities' triggers resolve at once (CR 605.4a): the land's mana is simply doubled.
            |d, _| d.vorinclex = true,
        ),
        "Blightsteel Colossus" => s(
            &[
                "Trample, infect, indestructible",
                "If Blightsteel Colossus would be put into a graveyard from anywhere, reveal Blightsteel Colossus and shuffle it into its owner's library instead.",
            ],
            |d, _| {
                d.keywords = d.keywords.with(Keywords::TRAMPLE).with(Keywords::INDESTRUCTIBLE);
                d.infect = true;
                d.shuffle_instead_of_graveyard = true;
            },
        ),
        "Etali, Primal Conqueror // Etali, Primal Sickness" => s(
            &[
                "When Etali enters, each player exiles cards from the top of their library until they exile a nonland card. You may cast any number of spells from among the nonland cards exiled this way without paying their mana costs.",
                "{9}{G/P}: Transform Etali. Activate only as a sorcery.",
                "Back: Whenever Etali deals combat damage to a player, they get that many poison counters.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::EachExileUntilNonlandCastFree]);
                // {G/P}: {G}, or 2 life instead (one ability for each way to pay).
                for (cost, life) in [("{9}{G}", 0), ("{9}", 2)] {
                    let n = activated(d, cost, false, vec![], vec![Effect::TransformSelf]);
                    d.activated[n].sorcery = true;
                    d.activated[n].life = life;
                }
                let b = d.back.as_mut().expect("Etali, Primal Sickness");
                on(b, When::DealsCombatDamageToPlayer, vec![], vec![Effect::PoisonAmountToOpponent]);
            },
        ),
        "Sneak Attack" => s(
            &["{R}: You may put a creature card from your hand onto the battlefield. That creature gains haste. Sacrifice the creature at the beginning of the next end step."],
            |d, _| {
                activated(d, "{R}", false, vec![], vec![Effect::SneakAttack]);
            },
        ),
        "Terror of the Peaks" => s(
            &[
                "Spells your opponents cast that target this creature cost an additional 3 life to cast.",
                "Whenever another creature you control enters, this creature deals damage equal to that creature's power to any target.",
            ],
            |d, _| {
                d.targeted_life_tax = 3;
                let it = Filter { triggering: true, ..CREATURE };
                on(d, When::AnotherCreatureYouControlEnters, vec![it, Filter::ANY], vec![Effect::DamageEqualSlotPower { from: 0, to: 1 }]);
            },
        ),
        "Xenagos, the Reveler" => s(
            &[
                "+1: Add X mana in any combination of {R} and/or {G}, where X is the number of creatures you control.",
                "0: Create a 2/2 red and green Satyr creature token with haste.",
                "−6: Exile the top seven cards of your library. You may put any number of creature and/or land cards from among them onto the battlefield.",
            ],
            |d, t| {
                loyalty(d, 1, Ability::new(vec![], vec![Effect::AddRedOrGreenPerCreature]));
                loyalty(d, 0, Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.satyr, n: 1 }]));
                loyalty(d, -6, Ability::new(vec![], vec![Effect::ExileTopPutCreaturesAndLands(7)]));
            },
        ),
        "Fauna Shaman" => s(
            &["{G}, {T}, Discard a creature card: Search your library for a creature card, reveal it, put it into your hand, then shuffle."],
            |d, _| {
                let n = activated(d, "{G}", true, vec![], vec![search(SearchFor::CreatureMvAtLeast(0), SearchTo::Hand)]);
                d.activated[n].discard_creature_card = true;
            },
        ),
        "Ghalta, Stampede Tyrant" => s(
            &["When Ghalta enters, put any number of creature cards from your hand onto the battlefield."],
            |d, _| on(d, When::Enters, vec![], vec![Effect::CreaturesFromHand]),
        ),
        "Maze of Ith" => s(
            &["{T}: Untap target attacking creature. Prevent all combat damage that would be dealt to and dealt by that creature this turn."],
            |d, _| {
                let attacking = Filter { in_combat: true, controller: Controller::Any, ..CREATURE };
                activated(d, "", true, vec![attacking], vec![Effect::UntapTarget(0), Effect::PreventCombatDamageToAndBy(0)]);
            },
        ),
        "Ohran Frostfang" => s(
            &["Attacking creatures you control have deathtouch.", "Whenever a creature you control deals combat damage to a player, draw a card."],
            |d, _| {
                static_on(d, Scope::YourAttackingCreatures, false, Keywords::DEATHTOUCH);
                on(d, When::YourCreatureDealsCombatDamageToPlayerAmount, vec![], vec![draw(1)]);
            },
        ),
        "Temur Sabertooth" => s(
            &["{1}{G}: You may return another creature you control to its owner's hand. If you do, this creature gains indestructible until end of turn."],
            |d, _| {
                activated(d, "{1}{G}", false, vec![], vec![Effect::MayReturnAnotherThenIndestructible]);
            },
        ),
        "The Fire Crystal" => s(
            &[
                "Red spells you cast cost {1} less to cast.",
                "Creatures you control have haste.",
                "{4}{R}{R}, {T}: Create a token that's a copy of target creature you control. Sacrifice it at the beginning of the next end step.",
            ],
            |d, _| {
                d.red_spell_discount = 1;
                static_on(d, Scope::YourCreatures, false, Keywords::HASTE);
                let copy = Effect::CopyToken { slot: Some(0), haste_and_sacrifice: true, extra_subtype: None };
                activated(d, "{4}{R}{R}", true, vec![YOUR_CREATURE], vec![copy]);
            },
        ),
        "Cultivator Colossus" => s(
            &[
                "Cultivator Colossus's power and toughness are each equal to the number of lands you control.",
                "When this creature enters, you may put a land card from your hand onto the battlefield tapped. If you do, draw a card and repeat this process.",
            ],
            |d, _| {
                d.pt_lands = true;
                on(d, When::Enters, vec![], vec![Effect::LandFromHandTappedDrawRepeat]);
            },
        ),
        "Terrain Generator" => s(&["{T}: Add {C}.", "{2}, {T}: You may put a basic land card from your hand onto the battlefield tapped."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            activated(d, "{2}", true, vec![], vec![Effect::BasicLandFromHandTapped]);
        }),
        "The Reaver Cleaver" => s(
            &[
                "Equipped creature gets +1/+1 and has trample and \"Whenever this creature deals combat damage to a player or planeswalker, create that many Treasure tokens.\"",
                "Equip {3}",
            ],
            |d, t| {
                attached(d, false, 1, 1, Keywords::TRAMPLE);
                // Its combat damage to a player (planeswalkers: not tracked as a trigger here).
                on(d, When::EquippedDealsCombatDamageToPlayer, vec![], vec![Effect::CreateTokensAmount { def: t.treasure }]);
                equip(d, "{3}");
            },
        ),
        "Apex Devastator" => s(&["Cascade, cascade, cascade, cascade"], |d, _| {
            for _ in 0..4 {
                on(d, When::CastThisSpell, vec![], vec![Effect::Cascade]);
            }
        }),
        "Orthion, Hero of Lavabrink" => s(
            &[
                "{1}{R}, {T}: Create a token that's a copy of another target creature you control. It gains haste. Sacrifice it at the beginning of the next end step. Activate only as a sorcery.",
                "{6}{R}{R}{R}, {T}: Create five tokens that are copies of another target creature you control. They gain haste. Sacrifice them at the beginning of the next end step. Activate only as a sorcery.",
            ],
            |d, _| {
                let other = Filter { other: true, ..YOUR_CREATURE };
                for (cost, n) in [("{1}{R}", 1), ("{6}{R}{R}{R}", 5)] {
                    let i = activated(d, cost, true, vec![other], vec![Effect::CopiesHasteSacrifice { slot: 0, n }]);
                    d.activated[i].sorcery = true;
                }
            },
        ),
        "Sarkhan's Unsealing" => s(
            &[
                "Whenever you cast a creature spell with power 4, 5, or 6, this enchantment deals 4 damage to any target.",
                "Whenever you cast a creature spell with power 7 or greater, this enchantment deals 4 damage to each opponent and each creature and planeswalker they control.",
            ],
            |d, _| {
                conditional(d, When::YouCastCreature, Condition::CastPowerBetween(4, 6), vec![Filter::ANY], vec![Effect::UnsealingDamage(0)]);
                conditional(d, When::YouCastCreature, Condition::CastPowerBetween(7, i32::MAX), vec![], vec![Effect::UnsealingDamage(0)]);
            },
        ),
        "Hellkite Courser" => s(
            &["When this creature enters, you may put a commander you own from the command zone onto the battlefield. It gains haste. Return it to the command zone at the beginning of the next end step."],
            |d, _| on(d, When::Enters, vec![], vec![Effect::CommanderFromCommandZoneUntilEnd]),
        ),
        "Protean Hulk" => s(
            &["When this creature dies, search your library for any number of creature cards with total mana value 6 or less, put them onto the battlefield, then shuffle."],
            |d, _| on(d, When::Dies, vec![], vec![Effect::SearchCreaturesTotalManaValue(6)]),
        ),
        "Selvala, Heart of the Wilds" => s(
            &[
                "Whenever another creature enters, its controller may draw a card if its power is greater than each other creature's power.",
                "{G}, {T}: Add X mana in any combination of colors, where X is the greatest power among creatures you control.",
            ],
            |d, _| {
                // "May draw": the card is drawn (an optional draw the controller takes).
                let it = Filter { triggering: true, controller: Controller::Any, ..CREATURE };
                for when in [When::AnotherCreatureYouControlEnters, When::CreatureOpponentControlsEnters] {
                    on(d, when, vec![it], vec![Effect::DrawIfGreatestPower(0)]);
                }
                activated(d, "{G}", true, vec![], vec![Effect::AddAnyColorsGreatestPower]);
            },
        ),
        "Vigor" => s(
            &[
                "If damage would be dealt to another creature you control, prevent that damage. Put a +1/+1 counter on that creature for each 1 damage prevented this way.",
                "When Vigor is put into a graveyard from anywhere, shuffle it into its owner's library.",
            ],
            |d, _| {
                d.damage_to_yours_becomes_counters = true;
                on(d, When::PutIntoGraveyardFromAnywhere, vec![], vec![Effect::ShuffleSourceFromGraveyardIntoLibrary]);
            },
        ),
        "Molten Echoes" => s(
            &[
                "As this enchantment enters, choose a creature type.",
                "Whenever a nontoken creature you control of the chosen type enters, create a token that's a copy of that creature. That token gains haste. Exile it at the beginning of the next end step.",
            ],
            |d, _| {
                d.choose_creature_type = true;
                let it = Filter { triggering: true, ..CREATURE };
                on(d, When::AnotherNontokenCreatureYouControlEnters, vec![it], vec![Effect::CopyIfChosenTypeExileAtEnd(0)]);
            },
        ),
        "Primal Surge" => s(
            &["Exile the top card of your library. If it's a permanent card, you may put it onto the battlefield. If you do, repeat this process."],
            |d, _| spell(d, vec![], vec![Effect::PrimalSurge]),
        ),
        "Flameshadow Conjuring" => s(
            &["Whenever a nontoken creature you control enters, you may pay {R}. If you do, create a token that's a copy of that creature. That token gains haste. Exile it at the beginning of the next end step."],
            |d, _| {
                let it = Filter { triggering: true, ..CREATURE };
                let pay = Effect::MayPay(ManaCost::parse("{R}").expect("cost"));
                on(d, When::AnotherNontokenCreatureYouControlEnters, vec![it], vec![pay, Effect::CopiesHasteExileAtEnd { slot: 0, n: 1 }]);
            },
        ),
        "Mirror March" => s(
            &["Whenever a nontoken creature you control enters, flip a coin until you lose a flip. For each flip you won, create a token that's a copy of that creature. Those tokens gain haste. Exile them at the beginning of the next end step."],
            |d, _| {
                let it = Filter { triggering: true, ..CREATURE };
                on(d, When::AnotherNontokenCreatureYouControlEnters, vec![it], vec![Effect::CoinFlipCopiesExileAtEnd(0)]);
            },
        ),
        "Rionya, Fire Dancer" => s(
            &["At the beginning of combat on your turn, create X tokens that are copies of another target creature you control, where X is one plus the number of instant and sorcery spells you've cast this turn. They gain haste. Exile them at the beginning of the next end step."],
            |d, _| {
                let other = Filter { other: true, ..YOUR_CREATURE };
                on(d, When::BeginCombatYourTurn, vec![other], vec![Effect::CopiesHasteExileAtEnd { slot: 0, n: 0 }]);
            },
        ),
        "Twinflame" => s(
            &[
                "Strive — This spell costs {2}{R} more to cast for each target beyond the first.",
                "Choose any number of target creatures you control. For each of them, create a token that's a copy of that creature, except it has haste. Exile those tokens at the beginning of the next end step.",
            ],
            |d, _| {
                // Strive: one mode per number of targets (1 to 5), each {2}{R} more per extra target.
                d.modes = (1..=5u8)
                    .map(|k| {
                        let effects = (0..k).map(|slot| Effect::CopiesHasteExileAtEnd { slot, n: 1 }).collect();
                        Ability::new(vec![YOUR_CREATURE; k as usize], effects)
                    })
                    .collect();
                d.mode_costs = ["{0}", "{2}{R}", "{4}{R}{R}", "{6}{R}{R}{R}", "{8}{R}{R}{R}{R}"]
                    .map(|c| ManaCost::parse(c).expect("cost"))
                    .to_vec();
            },
        ),
        "Mosswort Bridge" => s(
            &[
                "Hideaway 4",
                "This land enters tapped.",
                "{T}: Add {G}.",
                "{G}, {T}: You may play the exiled card without paying its mana cost if creatures you control have total power 10 or greater.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::Hideaway(4)]);
                d.enters_tapped = true;
                d.mana = Some(ManaSource::of(&[Color::Green]));
                activated(d, "{G}", true, vec![], vec![Effect::PlayHideaway(0)]);
            },
        ),
        "Spinerock Knoll" => s(
            &[
                "Hideaway 4",
                "This land enters tapped.",
                "{T}: Add {R}.",
                "{R}, {T}: You may play the exiled card without paying its mana cost if an opponent was dealt 7 or more damage this turn.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::Hideaway(4)]);
                d.enters_tapped = true;
                d.mana = Some(ManaSource::of(&[Color::Red]));
                activated(d, "{R}", true, vec![], vec![Effect::PlayHideaway(1)]);
            },
        ),
        "Cursed Mirror" => s(
            &["{T}: Add {R}.", "As this artifact enters, you may have it become a copy of any creature on the battlefield until end of turn, except it has haste."],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::Red]));
                d.enter_as_copy = true;
                d.copy_until_eot_haste = true;
            },
        ),
        "Wand of Wonder" => s(
            &[
                "{4}, {T}: Roll a d20. Each opponent exiles cards from the top of their library until they exile an instant or sorcery card, then shuffles the rest into their library. You may cast up to X instant and/or sorcery spells from among cards exiled this way without paying their mana costs.",
                "1—9 | X is one.",
                "10—19 | X is two.",
                "20 | X is three.",
            ],
            // One opponent exiles one instant or sorcery, so up to one is cast whatever X is.
            |d, _| {
                activated(d, "{4}", true, vec![], vec![Effect::WandOfWonder]);
            },
        ),
        "Vivien, Monsters' Advocate" => s(
            &[
                "You may look at the top card of your library any time.",
                "You may cast creature spells from the top of your library.",
                "+1: Create a 3/3 green Beast creature token. Put your choice of a reach counter, a vigilance counter, or a trample counter on it.",
                "−2: When you next cast a creature spell this turn, search your library for a creature card with lesser mana value, put it onto the battlefield, then shuffle.",
            ],
            |d, t| {
                d.top_spells_creatures = true; // looking at the top card is information only
                on(d, When::Never, vec![], vec![Effect::SearchCreatureLesserManaValue]);
                loyalty(d, 1, Ability::new(vec![], vec![Effect::CreateTokenWithKeywordCounterChoice { def: t.beast_3 }]));
                loyalty(d, -2, Ability::new(vec![], vec![Effect::NextCreatureSpellThisTurn(0)]));
            },
        ),
        "Delina, Wild Mage" => s(
            &[
                "Whenever Delina attacks, choose target creature you control, then roll a d20.",
                "1—14 | Create a tapped and attacking token that's a copy of that creature, except it's not legendary and it has \"At end of combat, exile this token.\"",
                "15—20 | Create one of those tokens. You may roll again.",
            ],
            |d, _| on(d, When::Attacks, vec![YOUR_CREATURE], vec![Effect::DelinaRoll(0)]),
        ),
        "Jaxis, the Troublemaker" => s(
            &[
                "{R}, {T}, Discard a card: Create a token that's a copy of another target creature you control. It gains haste and \"When this token dies, draw a card.\" Sacrifice it at the beginning of the next end step. Activate only as a sorcery.",
                "Blitz {1}{R}",
            ],
            |d, _| {
                on(d, When::Never, vec![], vec![draw(1)]); // the granted "when this dies, draw a card"
                let other = Filter { other: true, ..YOUR_CREATURE };
                let n = activated(d, "{R}", true, vec![other], vec![Effect::CopyHasteDiesDrawSacrifice { slot: 0, trigger: 0 }]);
                d.activated[n].discard_any_card = true;
                d.activated[n].sorcery = true;
                d.alt_cost = Some((ManaCost::parse("{1}{R}").expect("cost"), 0));
                d.blitz = Some(0);
            },
        ),
        "Zopandrel, Hunger Dominus" => s(
            &[
                "At the beginning of each combat, double the power and toughness of each creature you control until end of turn.",
                "{G/P}{G/P}, Sacrifice two other creatures: Put an indestructible counter on Zopandrel.",
            ],
            |d, _| {
                on(d, When::BeginCombatEachTurn, vec![], vec![Effect::DoublePowerToughnessAll]);
                // {G/P}{G/P}: each pip {G} or 2 life (one ability per way to pay).
                for (cost, life) in [("{G}{G}", 0), ("{G}", 2), ("", 4)] {
                    let n = activated(d, cost, false, vec![], vec![Effect::KeywordCounterSelf(Keywords::INDESTRUCTIBLE)]);
                    d.activated[n].sacrifice_two_others = true;
                    d.activated[n].life = life;
                }
            },
        ),
        "Mirage Phalanx" => s(
            &[
                "Soulbond",
                "As long as Mirage Phalanx is paired with another creature, each of those creatures has \"At the beginning of combat on your turn, create a token that's a copy of this creature, except it has haste and loses soulbond. Exile it at end of combat.\"",
            ],
            |d, _| {
                d.soulbond_combat_copies = true;
                may(d, When::Enters, vec![], vec![Effect::SoulbondPairOnEnter]);
                let it = Filter { triggering: true, ..CREATURE };
                may(d, When::AnotherCreatureYouControlEnters, vec![it], vec![Effect::SoulbondPairWith(0)]);
                on(d, When::BeginCombatYourTurn, vec![], vec![Effect::PhalanxCopies]);
            },
        ),
        "Ezuri's Predation" => s(
            &["For each creature your opponents control, create a 4/4 green Phyrexian Beast creature token. Each of those tokens fights a different one of those creatures."],
            |d, t| spell(d, vec![], vec![Effect::TokenFightsEachOpponentCreature { def: t.phyrexian_beast }]),
        ),
        // ---- Objects and zones (core theme, mac-c): Clues, Food costs, Adventures ----
        "Confirm Suspicions" => s(&["Counter target spell.", "Investigate three times."], |d, t| {
            spell(d, vec![SPELL], vec![Effect::Counter(0), Effect::Investigate { def: t.clue, n: 3 }]);
        }),
        "Confront the Unknown" => s(&["Investigate, then target creature gets +1/+1 until end of turn for each Clue you control."], |d, t| {
            spell(d, vec![CREATURE], vec![Effect::Investigate { def: t.clue, n: 1 }, Effect::PumpPerYourSubtype { slot: 0, sub: "Clue" }]);
        }),
        "Erdwal Illuminator" => s(&["Whenever you investigate for the first time each turn, investigate an additional time."], |d, t| {
            on(d, When::YouInvestigateFirstTimeEachTurn, vec![], vec![Effect::Investigate { def: t.clue, n: 1 }]);
        }),
        "Ethereal Investigator" => s(
            &[
                "When this creature enters, investigate X times, where X is the number of opponents you have.",
                "Whenever you draw your second card each turn, create a 1/1 white Spirit creature token with flying.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::Investigate { def: t.clue, n: 1 }]); // one opponent
                on(d, When::YouDrawSecond, vec![], vec![Effect::CreateToken { who: Who::You, def: t.spirit, n: 1 }]);
            },
        ),
        "Floodhound" => s(&["{3}, {T}: Investigate."], |d, t| {
            activated(d, "{3}", true, vec![], vec![Effect::Investigate { def: t.clue, n: 1 }]);
        }),
        "Hold for Questioning" => s(
            &[
                "Enchant creature or planeswalker",
                "When this Aura enters, tap enchanted permanent and investigate.",
                "Enchanted permanent doesn't untap during its controller's untap step and its activated abilities can't be activated.",
            ],
            |d, t| {
                d.aura = true;
                d.spell = Some(Ability::new(vec![CREATURE_OR_PLANESWALKER], vec![]));
                on(d, When::Enters, vec![], vec![Effect::TapAttached, Effect::Investigate { def: t.clue, n: 1 }]);
                d.enchanted_doesnt_untap = true;
                d.enchanted_stops_activations = true;
            },
        ),
        "Ongoing Investigation" => s(
            &[
                "Whenever one or more creatures you control deal combat damage to a player, investigate.",
                "{1}{G}, Exile a creature card from your graveyard: Investigate. You gain 2 life.",
            ],
            |d, t| {
                on(d, When::YourCreaturesDealCombatDamageToPlayer, vec![], vec![Effect::Investigate { def: t.clue, n: 1 }]);
                let n = activated(d, "{1}{G}", false, vec![], vec![Effect::Investigate { def: t.clue, n: 1 }, Effect::GainLife { who: Who::You, n: 2 }]);
                d.activated[n].exile_creature_card = true;
            },
        ),
        "Tamiyo's Journal" => s(
            &[
                "At the beginning of your upkeep, investigate.",
                "{T}, Sacrifice three Clues: Search your library for a card, put that card into your hand, then shuffle.",
            ],
            |d, t| {
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::Investigate { def: t.clue, n: 1 }]);
                let n = activated(d, "", true, vec![], vec![search(SearchFor::Any, SearchTo::Hand)]);
                d.activated[n].sacrifice_subtype = Some("Clue");
                d.activated[n].sacrifice_subtype_count = 3;
            },
        ),
        "Pippin, Warden of Isengard" => s(
            &[
                "Partner with Merry, Warden of Isengard",
                "{1}, {T}: Create a Food token.",
                "{T}, Sacrifice four Foods: Other creatures you control get +3/+3 and gain haste until end of turn. Activate only as a sorcery.",
            ],
            |d, t| {
                // Partner with: "When this creature enters, target player may put Merry into their hand
                // from their library" (you, in a two-player game).
                may(d, When::Enters, vec![], vec![search(SearchFor::Named(&["Merry, Warden of Isengard"]), SearchTo::Hand)]);
                activated(d, "{1}", true, vec![], vec![Effect::CreateToken { who: Who::You, def: t.food, n: 1 }]);
                let pump = Effect::PumpOthersAndGrant { power: 3, toughness: 3, keywords: Keywords::HASTE };
                let n = activated(d, "", true, vec![], vec![pump]);
                d.activated[n].sacrifice_subtype = Some("Food");
                d.activated[n].sacrifice_subtype_count = 4;
                d.activated[n].sorcery = true;
            },
        ),
        "Savvy Hunter" => s(
            &["Whenever this creature attacks or blocks, create a Food token.", "Sacrifice two Foods: Draw a card."],
            |d, t| {
                for when in [When::Attacks, When::Blocks] {
                    on(d, when, vec![], vec![Effect::CreateToken { who: Who::You, def: t.food, n: 1 }]);
                }
                let n = activated(d, "", false, vec![], vec![draw(1)]);
                d.activated[n].sacrifice_subtype = Some("Food");
                d.activated[n].sacrifice_subtype_count = 2;
            },
        ),
        "Bramble Familiar // Fetch Quest" => s(
            &[
                "{T}: Add {G}.",
                "{1}{G}, {T}, Discard a card: Return this creature to its owner's hand.",
                "Omen: Mill seven cards. Then put a creature, enchantment, or land card from among the milled cards onto the battlefield.",
            ],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::Green]));
                let n = activated(d, "{1}{G}", true, vec![], vec![Effect::BounceSelf]);
                d.activated[n].discard_any_card = true;
                omen(d, Ability::new(vec![], vec![Effect::MillPutCreatureEnchantmentOrLand(7)]));
                d.adventure = true;
            },
        ),
        "Monster Manual // Zoological Study" => s(
            &[
                "{1}{G}, {T}: You may put a creature card from your hand onto the battlefield.",
                "Omen: Mill five cards, then return a creature card milled this way to your hand.",
            ],
            |d, _| {
                activated(d, "{1}{G}", true, vec![], vec![Effect::MayPutCreatureFromHand]);
                omen(d, Ability::new(vec![], vec![Effect::MillReturnCreatureToHand(5)]));
                d.adventure = true;
            },
        ),
        "Animate Dead" => s(
            &[
                "Enchant creature card in a graveyard",
                "When this Aura enters, if it's on the battlefield, it loses \"enchant creature card in a graveyard\" and gains \"enchant creature put onto the battlefield with this Aura.\" Return enchanted creature card to the battlefield under your control and attach this Aura to it. When this Aura leaves the battlefield, that creature's controller sacrifices it.",
                "Enchanted creature gets -1/-0.",
            ],
            |d, _| {
                // The return happens as the Aura enters (its enters trigger, done at once).
                d.aura = true;
                d.spell = Some(Ability::new(vec![graveyard_cards(true, Controller::Any)], vec![]));
                d.reanimating_aura = true;
                attached(d, false, -1, 0, Keywords::NONE);
            },
        ),
        "Dance of the Dead" => s(
            &[
                "Enchant creature card in a graveyard",
                "When this Aura enters, if it's on the battlefield, it loses \"enchant creature card in a graveyard\" and gains \"enchant creature put onto the battlefield with this Aura.\" Put enchanted creature card onto the battlefield tapped under your control and attach this Aura to it. When this Aura leaves the battlefield, that creature's controller sacrifices it.",
                "Enchanted creature gets +1/+1 and doesn't untap during its controller's untap step.",
                "At the beginning of the upkeep of enchanted creature's controller, that player may pay {1}{B}. If the player does, untap that creature.",
            ],
            |d, _| {
                d.aura = true;
                d.spell = Some(Ability::new(vec![graveyard_cards(true, Controller::Any)], vec![]));
                d.reanimating_aura = true;
                d.reanimates_tapped = true;
                attached(d, false, 1, 1, Keywords::NONE);
                d.enchanted_doesnt_untap = true;
                let pay = Effect::MayPay(ManaCost::parse("{1}{B}").expect("cost"));
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![pay, Effect::UntapAttached]);
            },
        ),
        "Necromancy" => s(
            &[
                "You may cast this spell as though it had flash. If you cast it any time a sorcery couldn't have been cast, the controller of the permanent it becomes sacrifices it at the beginning of the next cleanup step.",
                "When this enchantment enters, if it's on the battlefield, it becomes an Aura with \"enchant creature put onto the battlefield with Necromancy.\" Put target creature card from a graveyard onto the battlefield under your control and attach this enchantment to it. When this enchantment leaves the battlefield, that creature's controller sacrifices it.",
            ],
            |d, _| {
                d.keywords = d.keywords.with(Keywords::FLASH);
                d.flash_cast_sacrificed = true; // at the next end step here
                d.reanimating_aura = true;
                on(d, When::Enters, vec![graveyard_cards(true, Controller::Any)], vec![Effect::ReanimateAttachSource(0)]);
            },
        ),
        "Teferi's Protection" => s(
            &[
                "Until your next turn, your life total can't change and you gain protection from everything. All permanents you control phase out.",
                "Exile Teferi's Protection.",
            ],
            |d, _| {
                // Protection from everything for a player: damage to you can't change your life either.
                spell(d, vec![], vec![Effect::LifeLockedUntilYourNextTurn, Effect::PhaseOut { all: true }, Effect::ExileSourceSpell]);
            },
        ),
        "Clever Concealment" => s(&["Convoke", "Any number of target nonland permanents you control phase out."], |d, _| {
            d.mana_ext.convoke = true;
            let yours = Filter { players: false, creatures: false, planeswalkers: false, nonland_permanents: true, controller: Controller::You, ..Filter::ANY };
            d.spell = Some(up_to(vec![yours; 8], vec![Effect::PhaseOut { all: false }]));
        }),
        "Tireless Tracker" => s(
            &[
                "Landfall — Whenever a land you control enters, investigate.",
                "Whenever you sacrifice a Clue, put a +1/+1 counter on this creature.",
            ],
            |d, t| {
                on(d, When::LandYouControlEnters, vec![], vec![Effect::Investigate { def: t.clue, n: 1 }]);
                on(d, When::YouSacrificeSubtype("Clue"), vec![], vec![Effect::CountersSelf { n: 1 }]);
            },
        ),
        "Academy Manufactor" => s(&["If you would create a Clue, Food, or Treasure token, instead create one of each."], |d, t| {
            d.clue_food_treasure = Some([t.clue, t.food, t.treasure]);
        }),
        "Wavesifter" => s(&["When this creature enters, investigate twice.", "Evoke {G}{U}"], |d, t| {
            on(d, When::Enters, vec![], vec![Effect::Investigate { def: t.clue, n: 2 }]);
            d.alt_cost = Some((ManaCost::parse("{G}{U}").expect("cost"), 0));
            d.mana_ext.evoke = true;
            on(d, When::Enters, vec![], vec![Effect::SacrificeIfEvoked]);
        }),
        // ---- Damia, Sage of Stone (Grande): its last three ----
        "All Suns' Dawn" => s(
            &["For each color, return up to one target card of that color from your graveyard to your hand. Exile All Suns' Dawn."],
            // The cards are chosen as it resolves (a graveyard has no hexproof to target past).
            |d, _| spell(d, vec![], vec![Effect::OneCardOfEachColorToHand, Effect::ExileSourceSpell]),
        ),
        "Chain of Smog" => s(
            &["Target player discards two cards. That player may copy this spell and may choose a new target for that copy."],
            |d, _| spell(d, vec![Filter::PLAYER], vec![Effect::Discard { who: Who::TargetPlayer(0), n: 2 }, Effect::ChainCopy(0)]),
        ),
        "Kira, Great Glass-Spinner" => s(
            &["Creatures you control have \"Whenever this creature becomes the target of a spell or ability for the first time each turn, counter that spell or ability.\""],
            |d, _| {
                on(d, When::Never, vec![], vec![Effect::CounterExtra]);
                d.kira_trigger = Some(0);
            },
        ),
        "Trostani, Selesnya's Voice" => s(
            &[
                "Whenever another creature you control enters, you gain life equal to that creature's toughness.",
                "{1}{G}{W}, {T}: Populate.",
            ],
            |d, _| {
                let it = Filter { triggering: true, ..CREATURE };
                on(d, When::AnotherCreatureYouControlEnters, vec![it], vec![Effect::GainLifeEqualSlotToughness(0)]);
                activated(d, "{1}{G}{W}", true, vec![], vec![Effect::Populate]);
            },
        ),
        "Isochron Scepter" => s(
            &[
                "Imprint — When this artifact enters, you may exile an instant card with mana value 2 or less from your hand.",
                "{2}, {T}: You may copy the exiled card. If you do, you may cast the copy without paying its mana cost.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::ImprintInstantFromHand]);
                activated(d, "{2}", true, vec![], vec![Effect::CastImprintedCopy]);
            },
        ),
        // ---- Objects and zones batch 6: sacrifice X Clues, and copies ----
        "Lonis, Cryptozoologist" => s(
            &[
                "Whenever another nontoken creature you control enters, investigate.",
                "{T}, Sacrifice X Clues: Target opponent reveals the top X cards of their library. You may put a nonland permanent card with mana value X or less from among them onto the battlefield under your control. That player puts the rest on the bottom of their library in a random order.",
            ],
            |d, t| {
                on(d, When::AnotherNontokenCreatureYouControlEnters, vec![], vec![Effect::Investigate { def: t.clue, n: 1 }]);
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                let n = activated(d, "", true, vec![opp], vec![Effect::OpponentRevealsTopXPutNonlandPermanent(0)]);
                d.activated[n].sacrifice_x_of_subtype = Some("Clue");
            },
        ),
        "Vaultborn Tyrant" => s(
            &[
                "Whenever this creature or another creature you control with power 4 or greater enters, you gain 3 life and draw a card.",
                "When this creature dies, if it's not a token, create a token that's a copy of it, except it's an artifact in addition to its other types.",
            ],
            |d, _| {
                on(d, When::YourCreatureEntersWithPowerAtLeast(4), vec![], vec![Effect::GainLife { who: Who::You, n: 3 }, draw(1)]);
                on(d, When::Dies, vec![], vec![Effect::TokenCopyOfSourceArtifactIfNontoken]);
            },
        ),
        "Phantasmal Image" => s(
            &["You may have this creature enter as a copy of any creature on the battlefield, except it's an Illusion in addition to its other types and it has \"When this creature becomes the target of a spell or ability, sacrifice it.\""],
            |d, _| {
                d.enter_as_copy = true;
                d.zones.copy_adds_subtypes = &["Illusion"];
                d.zones.copy_sacrifice_when_targeted = true;
            },
        ),
        "Mockingbird" => s(
            &["You may have this creature enter as a copy of any creature on the battlefield with mana value less than or equal to the amount of mana spent to cast this creature, except it's a Bird in addition to its other types and it has flying."],
            |d, _| {
                d.enter_as_copy = true;
                d.zones.copy_mv_at_most_spent = true;
                d.zones.copy_adds_subtypes = &["Bird"];
                d.zones.copy_adds_keywords = Keywords::FLYING;
            },
        ),
        "Glasspool Mimic // Glasspool Shore" => s(
            &[
                "You may have this creature enter as a copy of a creature you control, except it's a Shapeshifter Rogue in addition to its other types.",
                "Back: This land enters tapped.",
                "Back: {T}: Add {U}.",
            ],
            |d, _| {
                d.enter_as_copy = true;
                d.copy_only_yours = true;
                d.zones.copy_adds_subtypes = &["Shapeshifter", "Rogue"];
                d.modal_back = true;
                let b = d.back.as_mut().expect("Glasspool Shore");
                b.enters_tapped = true;
                b.mana = Some(ManaSource::of(&[Color::Blue]));
            },
        ),
        // ---- Objects and zones batch 7: morph, back faces, persist and plot ----
        "Aphetto Alchemist" => s(&["{T}: Untap target artifact or creature.", "Morph {U}"], |d, _| {
            let it = Filter { players: false, planeswalkers: false, artifacts: true, ..Filter::ANY };
            activated(d, "", true, vec![it], vec![Effect::UntapTarget(0)]);
            d.morph = ManaCost::parse("{U}");
        }),
        "Rattleclaw Mystic" => s(
            &["{T}: Add {G}, {U}, or {R}.", "Morph {2}", "When this creature is turned face up, add {G}{U}{R}."],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::Green, Color::Blue, Color::Red]));
                d.morph = ManaCost::parse("{2}");
                let add = |color| Effect::AddMana { color, n: 1 };
                on(d, When::TurnedFaceUp, vec![], vec![add(Color::Green), add(Color::Blue), add(Color::Red)]);
            },
        ),
        "Sundering Eruption // Volcanic Fissure" => s(
            &[
                "Destroy target land. Its controller may search their library for a basic land card, put it onto the battlefield tapped, then shuffle. Creatures without flying can't block this turn.",
                "Back: As this land enters, you may pay 3 life. If you don't, it enters tapped.",
                "Back: {T}: Add {R}.",
            ],
            |d, _| {
                let land = Filter { players: false, creatures: false, planeswalkers: false, lands: true, ..Filter::ANY };
                spell(d, vec![land], vec![Effect::DestroyControllerMayFetchBasicTapped(0), Effect::CreaturesWithoutFlyingCantBlockThisTurn]);
                d.modal_back = true;
                let b = d.back.as_mut().expect("Volcanic Fissure");
                b.shock_land = true;
                b.zones.shock_life = 3;
                b.mana = Some(ManaSource::of(&[Color::Red]));
            },
        ),
        "Puppeteer Clique" => s(
            &[
                "When this creature enters, put target creature card from an opponent's graveyard onto the battlefield under your control. It gains haste. At the beginning of your next end step, exile it.",
                "Persist",
            ],
            |d, _| {
                on(d, When::Enters, vec![graveyard_cards(true, Controller::Opponent)], vec![Effect::ReanimateHasteExileAtEnd(0)]);
                on(d, When::Dies, vec![], vec![Effect::Persist]);
            },
        ),
        "Railway Brawler" => s(
            &["Whenever another creature you control enters, put X +1/+1 counters on it, where X is its power.", "Plot {3}{G}"],
            |d, _| {
                let it = Filter { triggering: true, ..CREATURE };
                on(d, When::AnotherCreatureYouControlEnters, vec![it], vec![Effect::CountersEqualPower(0)]);
                let n = activated(d, "{3}{G}", false, vec![], vec![]);
                d.activated[n].plot = true;
                d.activated[n].sorcery = true;
            },
        ),
        // ---- Malik, Grim Manipulator (queue.json, lane C), batch 1: no new core ----
        "Diabolic Tutor" => s(&["Search your library for a card, put that card into your hand, then shuffle."], |d, _| {
            spell(d, vec![], vec![search(SearchFor::Any, SearchTo::Hand)])
        }),
        "Sundown Pass" => s(
            &["This land enters tapped unless you control two or more other lands.", "{T}: Add {R} or {W}."],
            |d, _| {
                d.mana_ext.enters_tapped_unless_other_lands = 2;
                dual(d, Color::Red, Color::White);
            },
        ),
        "Shattered Sanctum" => s(
            &["This land enters tapped unless you control two or more other lands.", "{T}: Add {W} or {B}."],
            |d, _| {
                d.mana_ext.enters_tapped_unless_other_lands = 2;
                dual(d, Color::White, Color::Black);
            },
        ),
        "Inspiring Vantage" => s(
            &["This land enters tapped unless you control two or fewer other lands.", "{T}: Add {R} or {W}."],
            |d, _| {
                d.enters_tapped_unless_at_most_lands = Some(2);
                dual(d, Color::Red, Color::White);
            },
        ),
        "Smoldering Marsh" => s(&["This land enters tapped unless you control two or more basic lands."], |d, _| {
            d.enters_tapped_unless_basics = 2;
            dual(d, Color::Black, Color::Red);
        }),
        "Blightstep Pathway // Searstep Pathway" => s(&["{T}: Add {B}.", "Back: {T}: Add {R}."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Black]));
            d.modal_back = true;
            d.back.as_mut().expect("Searstep Pathway").mana = Some(ManaSource::of(&[Color::Red]));
        }),
        "Needleverge Pathway // Pillarverge Pathway" => s(&["{T}: Add {R}.", "Back: {T}: Add {W}."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Red]));
            d.modal_back = true;
            d.back.as_mut().expect("Pillarverge Pathway").mana = Some(ManaSource::of(&[Color::White]));
        }),
        "Merciless Executioner" => s(&["When this creature enters, each player sacrifices a creature of their choice."], |d, _| {
            on(
                d,
                When::Enters,
                vec![],
                vec![Effect::SacrificeCreatures { who: Who::You, n: 1 }, Effect::SacrificeCreatures { who: Who::EachOpponent, n: 1 }],
            )
        }),
        "Massacre Wurm" => s(
            &[
                "When this creature enters, creatures your opponents control get -2/-2 until end of turn.",
                "Whenever a creature an opponent controls dies, that player loses 2 life.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::PumpAll { who: Who::EachOpponent, power: -2, toughness: -2 }]);
                // "That player": the one opponent.
                on(d, When::OpponentCreatureDies, vec![], vec![Effect::LoseLife { who: Who::EachOpponent, n: 2 }]);
            },
        ),
        "Butcher of Malakir" => s(
            &["Whenever this creature or another creature you control dies, each opponent sacrifices a creature of their choice."],
            |d, _| on(d, When::CreatureYouControlDies, vec![], vec![Effect::SacrificeCreatures { who: Who::EachOpponent, n: 1 }]),
        ),
        "Reckless Fireweaver" => s(&["Whenever an artifact you control enters, this creature deals 1 damage to each opponent."], |d, _| {
            on(d, When::ArtifactYouControlEnters, vec![], vec![damage(1, Recipient::EachOpponent)])
        }),
        "Mayhem Devil" => s(&["Whenever a player sacrifices a permanent, this creature deals 1 damage to any target."], |d, _| {
            on(d, When::PlayerSacrificesAnother, vec![Filter::ANY], vec![damage(1, Recipient::Target(0))])
        }),
        "Teleportation Circle" => s(
            &["At the beginning of your end step, exile up to one target artifact or creature you control, then return that card to the battlefield under its owner's control."],
            |d, _| {
                let yours = Filter { players: false, planeswalkers: false, artifacts: true, controller: Controller::You, ..Filter::ANY };
                on(d, When::EndStep { your_turn_only: true }, vec![yours], vec![Effect::BlinkSlot(0)]);
                d.triggers.last_mut().expect("the trigger").ability.min_targets = Some(0);
            },
        ),
        "Crackling Doom" => s(
            &["Crackling Doom deals 2 damage to each opponent. Each opponent sacrifices a creature with the greatest power among creatures that player controls."],
            |d, _| spell(d, vec![], vec![damage(2, Recipient::EachOpponent), Effect::SacrificeGreatestPower(Who::EachOpponent)]),
        ),
        "Charming Prince" => s(
            &[
                "When this creature enters, choose one —",
                "• Scry 2.",
                "• You gain 3 life.",
                "• Exile another target creature you own. Return it to the battlefield under your control at the beginning of the next end step.",
            ],
            |d, _| {
                // "You own": your creatures (the ones you control and own in a two-player game).
                let another = Filter { other: true, ..YOUR_CREATURE };
                on_modal(
                    d,
                    When::Enters,
                    None,
                    vec![
                        Ability::new(vec![], vec![Effect::Scry { n: 2 }]),
                        Ability::new(vec![], vec![Effect::GainLife { who: Who::You, n: 3 }]),
                        Ability::new(vec![another], vec![Effect::Flicker(0)]),
                    ],
                )
            },
        ),
        "Ephemerate" => s(
            &["Exile target creature you control, then return it to the battlefield under its owner's control.", "Rebound"],
            |d, _| spell(d, vec![YOUR_CREATURE], vec![Effect::BlinkSlot(0), Effect::Rebound]),
        ),
        "Treasure Vault" => s(&["{T}: Add {C}.", "{X}{X}, {T}, Sacrifice this land: Create X Treasure tokens."], |d, t| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            let n = activated(d, "{X}{X}", true, vec![], vec![Effect::CreateTokensX { def: t.treasure }]);
            d.activated[n].sacrifice_self = true;
        }),
        // ---- Malik, batch 2: objects and zones ----
        "Nahiri's Resolve" => s(
            &[
                "Creatures you control get +1/+0 and have haste.",
                "At the beginning of your end step, exile any number of nontoken artifacts and/or creatures you control. Return those cards to the battlefield under their owner's control at the beginning of your next upkeep.",
            ],
            |d, _| {
                static_on(d, Scope::YourCreatures, false, Keywords::HASTE);
                d.statics.last_mut().expect("the static").power = 1;
                on(d, When::EndStep { your_turn_only: true }, vec![], vec![Effect::ExileYoursUntilYourNextUpkeep]);
            },
        ),
        "Treasure Map // Treasure Cove" => s(
            &[
                "{1}, {T}: Scry 1. Put a landmark counter on this artifact. Then if there are three or more landmark counters on it, remove those counters, transform this artifact, and create three Treasure tokens.",
                "Back: {T}: Add {C}.",
                "Back: {T}, Sacrifice a Treasure: Draw a card.",
            ],
            |d, t| {
                activated(d, "{1}", true, vec![], vec![Effect::Scry { n: 1 }, Effect::LandmarkTransformTreasures { def: t.treasure }]);
                let b = d.back.as_mut().expect("Treasure Cove");
                b.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                let n = activated(b, "", true, vec![], vec![draw(1)]);
                b.activated[n].sacrifice_subtype = Some("Treasure");
            },
        ),
        // ---- Bruna, Light of Alabaster (queue.json, lane C), batch 1 ----
        "Supreme Verdict" => s(&["This spell can't be countered.", "Destroy all creatures."], |d, _| {
            d.uncounterable = true;
            spell(d, vec![], vec![Effect::DestroyAllCreatures]);
        }),
        "Idyllic Tutor" => s(&["Search your library for an enchantment card, reveal it, put it into your hand, then shuffle."], |d, _| {
            spell(d, vec![], vec![search(SearchFor::Enchantment, SearchTo::Hand)])
        }),
        "Talisman of Progress" => s(&["{T}: Add {C}.", "{T}: Add {W} or {U}. This artifact deals 1 damage to you."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::of(&[Color::White, Color::Blue]) });
            d.pain_colored = true;
        }),
        "Deserted Beach" => s(
            &["This land enters tapped unless you control two or more other lands.", "{T}: Add {W} or {U}."],
            |d, _| {
                d.mana_ext.enters_tapped_unless_other_lands = 2;
                dual(d, Color::White, Color::Blue);
            },
        ),
        "Prismatic Vista" => s(
            &["{T}, Pay 1 life, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield, then shuffle."],
            |d, _| {
                let n = activated(d, "", true, vec![], vec![search(SearchFor::BasicLand, SearchTo::Battlefield { tapped: false })]);
                d.activated[n].sacrifice_self = true;
                d.activated[n].life = 1;
            },
        ),
        "Dovin's Veto" => s(&["This spell can't be countered.", "Counter target noncreature spell."], |d, _| {
            d.uncounterable = true;
            spell(d, vec![Filter { spell_creature: Some(false), ..SPELL }], vec![Effect::Counter(0)]);
        }),
        "Eel Umbra" => s(&["Enchant creature", "Enchanted creature gets +1/+1.", "Umbra armor"], |d, _| {
            d.aura = true;
            d.spell = Some(Ability::new(vec![CREATURE], vec![]));
            attached(d, false, 1, 1, Keywords::NONE);
            d.umbra_armor = true;
        }),
        "Crashing Drawbridge" => s(&["{T}: Creatures you control gain haste until end of turn."], |d, _| {
            activated(d, "", true, vec![], vec![Effect::GrantAll { who: Who::You, keywords: Keywords::HASTE }]);
        }),
        "Heliod's Pilgrim" => s(
            &["When this creature enters, you may search your library for an Aura card, reveal it, put it into your hand, then shuffle."],
            |d, _| may(d, When::Enters, vec![], vec![search(SearchFor::Subtype("Aura"), SearchTo::Hand)]),
        ),
        "Galadriel's Dismissal" => s(
            &[
                "Kicker {2}{W}",
                "Target creature phases out. If this spell was kicked, each creature target player controls phases out instead.",
            ],
            |d, _| {
                d.kicker = ManaCost::parse("{2}{W}");
                spell(d, vec![CREATURE], vec![Effect::PhaseOutSlot(0)]);
                d.kicked_spell = Some(Ability::new(vec![Filter::PLAYER], vec![Effect::PhaseOutCreaturesOfPlayer(0)]));
            },
        ),
        "Mirrormade" => s(&["You may have this enchantment enter as a copy of any artifact or enchantment on the battlefield."], |d, _| {
            d.enter_as_copy = true;
            d.zones.copy_artifact_or_enchantment = true;
        }),
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
    "Greensleeves, Maro-Sorcerer",
    "The Gitrog Monster",
    "Colossal Grave-Reaver",
    "Wrenn and Seven",
    "Nissa of Shadowed Boughs",
    "Delighted Halfling",
    "Force of Vigor",
    "Dread Return",
    "Scapeshift",
    "Rise of the Witch-king",
    "Will of the Abzan",
    "Koma, Cosmos Serpent",
    "Sensei's Divining Top",
    "Hedge Shredder",
    "Nissa, Worldsoul Speaker",
    // lane B
    "Phyrexian Metamorph",
    "Sakashima of a Thousand Faces",
    "Sakashima the Impostor",
    "Zimone, Mystery Unraveler",
    "Jace, the Mind Sculptor",
    "Scroll Rack",
    "Conduit of Worlds",
    "Legolas's Quick Reflexes",
    "Swan Song",
    "Biorhythm",
    "Sunder",
    "Ghost Town",
    "Palinchron",
    "Broken Bond",
    "Collector Ouphe",
    "Mox Amber",
    "Voidslime",
    "Awaken the Woods",
    "Sylvan Advocate",
    "Omnath, Locus of Mana",
    "Cyclone Summoner",
    "Roil Elemental",
    "Yavimaya Dryad",
    "Otawara, Soaring City",
    "Scatter to the Winds",
    "Three Steps Ahead",
    "Plaza of Heroes",
    "Bear Umbra",
    "Shifting Woodland",
    "Khalni Ambush // Khalni Territory",
    "Nissa, Vastwood Seer // Nissa, Sage Animist",
    "Deadeye Navigator",
    "Green Sun's Zenith",
    "Bane of Progress",
    "Temple of the False God",
    "Circle of Dreams Druid",
    "Tribute to the World Tree",
    "Parallel Lives",
    "Spire Garden",
    "Worldspine Wurm",
    "Ulamog, the Infinite Gyre",
    "Lurking Predators",
    "Ulamog, the Defiler",
    "Homeward Path",
    "Purphoros, God of the Forge",
    "Conjurer's Closet",
    "Cavern-Hoard Dragon",
    "Displaced Dinosaurs",
    "Vorinclex, Voice of Hunger",
    "Blightsteel Colossus",
    "Etali, Primal Conqueror // Etali, Primal Sickness",
    "Sneak Attack",
    "Terror of the Peaks",
    "Xenagos, the Reveler",
    "Fauna Shaman",
    "Ghalta, Stampede Tyrant",
    "Maze of Ith",
    "Ohran Frostfang",
    "Temur Sabertooth",
    "The Fire Crystal",
    "Cultivator Colossus",
    "Terrain Generator",
    "The Reaver Cleaver",
    "Apex Devastator",
    "Orthion, Hero of Lavabrink",
    "Sarkhan's Unsealing",
    "Hellkite Courser",
    "Protean Hulk",
    "Selvala, Heart of the Wilds",
    "Vigor",
    "Molten Echoes",
    "Primal Surge",
    "Flameshadow Conjuring",
    "Mirror March",
    "Rionya, Fire Dancer",
    "Twinflame",
    "Mosswort Bridge",
    "Spinerock Knoll",
    "Cursed Mirror",
    "Wand of Wonder",
    "Vivien, Monsters' Advocate",
    "Delina, Wild Mage",
    "Jaxis, the Troublemaker",
    "Zopandrel, Hunger Dominus",
    "Mirage Phalanx",
    "Ezuri's Predation",
    "Confirm Suspicions",
    "Confront the Unknown",
    "Erdwal Illuminator",
    "Ethereal Investigator",
    "Floodhound",
    "Hold for Questioning",
    "Ongoing Investigation",
    "Tamiyo's Journal",
    "Pippin, Warden of Isengard",
    "Savvy Hunter",
    "Bramble Familiar // Fetch Quest",
    "Monster Manual // Zoological Study",
    "Animate Dead",
    "Dance of the Dead",
    "Necromancy",
    "Teferi's Protection",
    "Clever Concealment",
    "Tireless Tracker",
    "Academy Manufactor",
    "Wavesifter",
    "All Suns' Dawn",
    "Chain of Smog",
    "Kira, Great Glass-Spinner",
    "Trostani, Selesnya's Voice",
    "Isochron Scepter",
    "Lonis, Cryptozoologist",
    "Vaultborn Tyrant",
    "Phantasmal Image",
    "Mockingbird",
    "Glasspool Mimic // Glasspool Shore",
    "Aphetto Alchemist",
    "Rattleclaw Mystic",
    "Sundering Eruption // Volcanic Fissure",
    "Puppeteer Clique",
    "Railway Brawler",
    "Diabolic Tutor",
    "Sundown Pass",
    "Shattered Sanctum",
    "Inspiring Vantage",
    "Smoldering Marsh",
    "Blightstep Pathway // Searstep Pathway",
    "Needleverge Pathway // Pillarverge Pathway",
    "Merciless Executioner",
    "Massacre Wurm",
    "Butcher of Malakir",
    "Reckless Fireweaver",
    "Mayhem Devil",
    "Teleportation Circle",
    "Crackling Doom",
    "Charming Prince",
    "Ephemerate",
    "Treasure Vault",
    "Nahiri's Resolve",
    "Treasure Map // Treasure Cove",
    "Supreme Verdict",
    "Idyllic Tutor",
    "Talisman of Progress",
    "Deserted Beach",
    "Prismatic Vista",
    "Dovin's Veto",
    "Eel Umbra",
    "Crashing Drawbridge",
    "Heliod's Pilgrim",
    "Galadriel's Dismissal",
    "Mirrormade",
];
