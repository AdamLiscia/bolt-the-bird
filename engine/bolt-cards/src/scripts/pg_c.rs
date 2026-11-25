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
];
