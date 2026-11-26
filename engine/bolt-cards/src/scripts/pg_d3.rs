//! Playgroup decks (Nerd Leagues), lane D3 (the desktop): the cards for the deck in handoff/lanes/d3.json. Conventions:
//! docs/lane-brief.md. Card text: assets/PG_oracle.json (in the CMD pool). Tests: tests/pg_d3.rs.
#![allow(unused_imports)]

use super::*;

/// "{T}: Add {A} or {B}." (or more colors).
fn lands_mana(d: &mut CardDef, colors: &[Color]) {
    d.mana = Some(ManaSource::of(colors));
}

/// "{T}: Add {C}."
fn colorless(d: &mut CardDef) {
    d.mana = Some(ManaSource {
        colorless: true,
        ..ManaSource::default()
    });
}

/// "Cycling {cost}": "{cost}, Discard this card: Draw a card." from your hand.
fn cycling(d: &mut CardDef, cost: &str) {
    let n = activated(d, cost, false, vec![], vec![draw(1)]);
    d.activated[n].from_hand_discard = true;
}

/// "{cost}, {T}: Add" two mana: the first `a`, the second `b` (Signets, Darkwater Catacombs).
fn two_mana(d: &mut CardDef, cost: &str, a: Color, b: Color) {
    d.paid_mana = Some((
        ManaCost::parse(cost).expect("a mana cost"),
        [ManaSource::of(&[a]), ManaSource::of(&[b])],
    ));
}

/// "{T}: Draw a card, then discard a card." (the loot creatures)
fn looter(d: &mut CardDef) {
    activated(d, "", true, vec![], vec![draw(1), discard(1)]);
}

/// A Triome: "This land enters tapped." / "Cycling {3}", tapping for its three basic land types' colors.
fn triome(d: &mut CardDef, colors: &[Color]) {
    d.enters_tapped = true;
    lands_mana(d, colors);
    cycling(d, "{3}");
}

/// A Talisman: "{T}: Add {C}." and "{T}: Add {A} or {B}. This artifact deals 1 damage to you."
fn talisman(d: &mut CardDef, a: Color, b: Color) {
    d.mana = Some(ManaSource {
        colorless: true,
        ..ManaSource::of(&[a, b])
    });
    d.pain_colored = true;
}

/// "Each player draws a card." (two players: you, then the opponent)
fn each_player_draws() -> Vec<Effect> {
    vec![
        draw(1),
        Effect::Draw {
            who: Who::EachOpponent,
            n: 1,
        },
    ]
}

const LOOT: &str = "{T}: Draw a card, then discard a card.";

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        // ---- Sefris of the Hidden Ways (Spr3adsh33t) ----
        "Isolated Chapel" => s(
            &["This land enters tapped unless you control a Plains or a Swamp.", "{T}: Add {W} or {B}."],
            |d, _| {
                d.enters_tapped_unless = Some(["Plains", "Swamp"]);
                lands_mana(d, &[Color::White, Color::Black]);
            },
        ),
        "Dimir Signet" => s(&["{1}, {T}: Add {U}{B}."], |d, _| two_mana(d, "{1}", Color::Blue, Color::Black)),
        "Orzhov Signet" => s(&["{1}, {T}: Add {W}{B}."], |d, _| two_mana(d, "{1}", Color::White, Color::Black)),
        "Raffine's Tower" => s(&["This land enters tapped.", "Cycling {3}"], |d, _| {
            d.enters_tapped = true;
            lands_mana(d, &[Color::White, Color::Blue, Color::Black]);
            cycling(d, "{3}");
        }),
        "Counterspell" => s(&["Counter target spell."], |d, _| spell(d, vec![SPELL], vec![Effect::Counter(0)])),
        "Entomb" => s(
            &["Search your library for a card, put that card into your graveyard, then shuffle."],
            |d, _| spell(d, vec![], vec![search(SearchFor::Any, SearchTo::Graveyard)]),
        ),
        "Buried Alive" => s(
            &["Search your library for up to three creature cards, put them into your graveyard, then shuffle."],
            |d, _| {
                // Any creature card: mana value 0 or more. The search may stop early ("up to").
                let what = SearchFor::CreatureMvAtLeast(0);
                spell(d, vec![], vec![Effect::Search { what, to: SearchTo::Graveyard, n: 3, kicked_n: None }]);
            },
        ),
        "Darkwater Catacombs" => s(&["{1}, {T}: Add {U}{B}."], |d, _| two_mana(d, "{1}", Color::Blue, Color::Black)),
        "Geier Reach Sanitarium" => s(
            &["{T}: Add {C}.", "{2}, {T}: Each player draws a card, then discards a card."],
            |d, _| {
                colorless(d);
                activated(
                    d,
                    "{2}",
                    true,
                    vec![],
                    vec![
                        draw(1),
                        Effect::Draw { who: Who::EachOpponent, n: 1 },
                        discard(1),
                        Effect::Discard { who: Who::EachOpponent, n: 1 },
                    ],
                );
            },
        ),
        "Razaketh, the Foulblooded" => s(
            &["Pay 2 life, Sacrifice another creature: Search your library for a card, put that card into your hand, then shuffle."],
            |d, _| {
                let n = activated(d, "", false, vec![], vec![search(SearchFor::Any, SearchTo::Hand)]);
                d.activated[n].life = 2;
                d.activated[n].sacrifice_other_creature = true;
            },
        ),
        "Ashen Rider" => s(&["When this creature enters or dies, exile target permanent."], |d, _| {
            let any = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, ..Filter::ANY };
            on(d, When::Enters, vec![any], vec![Effect::Exile(0)]);
            on(d, When::Dies, vec![any], vec![Effect::Exile(0)]);
        }),
        "Arcane Sanctum" => s(&["This land enters tapped.", "{T}: Add {W}, {U}, or {B}."], |d, _| {
            d.enters_tapped = true;
            lands_mana(d, &[Color::White, Color::Blue, Color::Black]);
        }),
        "Cavalier of Night" => s(
            &[
                "When this creature enters, you may sacrifice another creature. When you do, destroy target creature an opponent controls.",
                "When this creature dies, return target creature card with mana value 3 or less from your graveyard to the battlefield.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::MaySacrificeAnother, Effect::Reflexive(1)]);
                on(d, When::Never, vec![OPP_CREATURE], vec![Effect::Destroy(0)]); // trigger 1: "when you do"
                let card = Filter { max_mana_value: Some(3), ..graveyard_cards(true, Controller::You) };
                on(d, When::Dies, vec![card], vec![Effect::Reanimate(0)]);
            },
        ),
        "Choked Estuary" => s(
            &[
                "As this land enters, you may reveal an Island or Swamp card from your hand. If you don't, this land enters tapped.",
                "{T}: Add {U} or {B}.",
            ],
            |d, _| {
                d.enters_tapped_unless_reveal = Some(["Island", "Swamp"]);
                lands_mana(d, &[Color::Blue, Color::Black]);
            },
        ),
        "Darkslick Shores" => s(
            &["This land enters tapped unless you control two or fewer other lands.", "{T}: Add {U} or {B}."],
            |d, _| {
                d.enters_tapped_unless_at_most_lands = Some(2);
                lands_mana(d, &[Color::Blue, Color::Black]);
            },
        ),
        "Merfolk Looter" => s(&[LOOT], |d, _| looter(d)),
        "Thought Courier" => s(&[LOOT], |d, _| looter(d)),
        "Port Town" => s(
            &[
                "As this land enters, you may reveal a Plains or Island card from your hand. If you don't, this land enters tapped.",
                "{T}: Add {W} or {U}.",
            ],
            |d, _| {
                d.enters_tapped_unless_reveal = Some(["Plains", "Island"]);
                lands_mana(d, &[Color::White, Color::Blue]);
            },
        ),
        "Vile Entomber" => s(
            &["When this creature enters, search your library for a card, put that card into your graveyard, then shuffle."],
            |d, _| on(d, When::Enters, vec![], vec![search(SearchFor::Any, SearchTo::Graveyard)]),
        ),
        // ---- Najeela, the Blade-Blossom (Time) ----
        "Flooded Strand" => s(
            &["{T}, Pay 1 life, Sacrifice this land: Search your library for a Plains or Island card, put it onto the battlefield, then shuffle."],
            |d, _| fetch_land(d, &["Plains", "Island"]),
        ),
        "Hallowed Fountain" => s(
            &["As this land enters, you may pay 2 life. If you don't, it enters tapped."],
            |d, _| {
                d.shock_land = true;
                lands_mana(d, &[Color::White, Color::Blue]);
            },
        ),
        "Ziatora's Proving Ground" => s(&["This land enters tapped.", "Cycling {3}"], |d, _| {
            triome(d, &[Color::Black, Color::Red, Color::Green])
        }),
        "Ketria Triome" => s(&["This land enters tapped.", "Cycling {3}"], |d, _| {
            triome(d, &[Color::Blue, Color::Red, Color::Green])
        }),
        "Savai Triome" => s(&["This land enters tapped.", "Cycling {3}"], |d, _| {
            triome(d, &[Color::White, Color::Black, Color::Red])
        }),
        "Selesnya Signet" => s(&["{1}, {T}: Add {G}{W}."], |d, _| two_mana(d, "{1}", Color::Green, Color::White)),
        "Boros Charm" => s(
            &[
                "Choose one —",
                "• Boros Charm deals 4 damage to target player or planeswalker.",
                "• Permanents you control gain indestructible until end of turn.",
                "• Target creature gains double strike until end of turn.",
            ],
            |d, _| {
                let player_or_pw = Filter { creatures: false, ..Filter::ANY };
                modes(
                    d,
                    vec![
                        Ability::new(vec![player_or_pw], vec![damage(4, Recipient::Target(0))]),
                        Ability::new(vec![], vec![Effect::GrantAllPermanentsYouControl(Keywords::INDESTRUCTIBLE)]),
                        Ability::new(vec![CREATURE], vec![Effect::Grant { slot: 0, keywords: Keywords::DOUBLE_STRIKE }]),
                    ],
                )
            },
        ),
        "Brainstorm" => s(
            &["Draw three cards, then put two cards from your hand on top of your library in any order."],
            |d, _| spell(d, vec![], vec![Effect::DrawThenPutBack { draw: 3, back: 2 }]),
        ),
        "Edric, Spymaster of Trest" => s(
            &["Whenever a creature deals combat damage to one of your opponents, its controller may draw a card."],
            |d, _| {
                // 1v1: only your creatures deal combat damage to your opponent, so "its controller" is you.
                may(d, When::YourCreatureDealsCombatDamageToPlayer, vec![], vec![draw(1)]);
            },
        ),
        "Opt" => s(&["Scry 1.", "Draw a card."], |d, _| spell(d, vec![], vec![Effect::Scry { n: 1 }, draw(1)])),
        "Talisman of Dominance" => s(
            &["{T}: Add {C}.", "{T}: Add {U} or {B}. This artifact deals 1 damage to you."],
            |d, _| talisman(d, Color::Blue, Color::Black),
        ),
        "Talisman of Curiosity" => s(
            &["{T}: Add {C}.", "{T}: Add {G} or {U}. This artifact deals 1 damage to you."],
            |d, _| talisman(d, Color::Green, Color::Blue),
        ),
        "Fleshbag Marauder" => s(
            &["When this creature enters, each player sacrifices a creature of their choice."],
            |d, _| {
                on(
                    d,
                    When::Enters,
                    vec![],
                    vec![
                        Effect::SacrificeCreatures { who: Who::You, n: 1 },
                        Effect::SacrificeCreatures { who: Who::EachOpponent, n: 1 },
                    ],
                )
            },
        ),
        "Growth Spiral" => s(
            &["Draw a card. You may put a land card from your hand onto the battlefield."],
            |d, _| spell(d, vec![], vec![draw(1), Effect::MayLandFromHand]),
        ),
        "Preordain" => s(&["Scry 2, then draw a card."], |d, _| spell(d, vec![], vec![Effect::Scry { n: 2 }, draw(1)])),
        "City of Death" => s(
            &[
                "I — Create a Treasure token.",
                "II, III, IV, V, VI — Create a token that's a copy of target non-Saga token you control.",
            ],
            |d, t| {
                chapter(d, 1, Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }]));
                let token = Filter {
                    players: false,
                    creatures: false,
                    planeswalkers: false,
                    any_permanent: true,
                    tokens_only: true,
                    not_subtype: Some("Saga"),
                    controller: Controller::You,
                    ..Filter::ANY
                };
                let copy = Effect::CopyToken { slot: Some(0), haste_and_sacrifice: false, extra_subtype: None };
                for n in 2..=6 {
                    chapter(d, n, Ability::new(vec![token], vec![copy]));
                }
            },
        ),
        "Sylvan Caryatid" => s(&["{T}: Add one mana of any color."], |d, _| lands_mana(d, &Color::ALL)),
        // ---- Sheoldred, the Apocalypse (Spr3adsh33t) ----
        "Decanter of Endless Water" => s(
            &["You have no maximum hand size.", "{T}: Add one mana of any color."],
            |d, _| {
                d.no_max_hand_size = true;
                lands_mana(d, &Color::ALL);
            },
        ),
        "Temple Bell" => s(&["{T}: Each player draws a card."], |d, _| {
            activated(d, "", true, vec![], each_player_draws());
        }),
        "Charcoal Diamond" => s(&["This artifact enters tapped.", "{T}: Add {B}."], |d, _| {
            d.enters_tapped = true;
            lands_mana(d, &[Color::Black]);
        }),
        "Fate Unraveler" => s(
            &["Whenever an opponent draws a card, this creature deals 1 damage to that player."],
            |d, _| on(d, When::OpponentDraws, vec![], vec![damage(1, Recipient::EachOpponent)]),
        ),
        "Underworld Dreams" => s(
            &["Whenever an opponent draws a card, this enchantment deals 1 damage to that player."],
            |d, _| on(d, When::OpponentDraws, vec![], vec![damage(1, Recipient::EachOpponent)]),
        ),
        "Font of Mythos" => s(
            &["At the beginning of each player's draw step, that player draws two additional cards."],
            |d, _| on(d, When::EachDrawStep, vec![], vec![Effect::ActivePlayerDraws(2)]),
        ),
        "Greed" => s(&["{B}, Pay 2 life: Draw a card."], |d, _| {
            let n = activated(d, "{B}", false, vec![], vec![draw(1)]);
            d.activated[n].life = 2;
        }),
        "Infernal Grasp" => s(&["Destroy target creature. You lose 2 life."], |d, _| {
            spell(d, vec![CREATURE], vec![Effect::Destroy(0), Effect::LoseLife { who: Who::You, n: 2 }])
        }),
        "Liliana's Caress" => s(&["Whenever an opponent discards a card, that player loses 2 life."], |d, _| {
            // Slot 0 is the discarded card (not a target); "that player" is the one opponent.
            let that = Filter { triggering: true, ..graveyard_cards(false, Controller::Any) };
            on(d, When::OpponentDiscards, vec![that], vec![Effect::LoseLife { who: Who::EachOpponent, n: 2 }])
        }),
        "Master of the Feast" => s(&["At the beginning of your upkeep, each opponent draws a card."], |d, _| {
            on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::Draw { who: Who::EachOpponent, n: 1 }])
        }),
        "Mikokoro, Center of the Sea" => s(&["{T}: Add {C}.", "{2}, {T}: Each player draws a card."], |d, _| {
            colorless(d);
            activated(d, "{2}", true, vec![], each_player_draws());
        }),
        "Bloodthirsty Aerialist" => s(&["Whenever you gain life, put a +1/+1 counter on this creature."], |d, _| {
            on(d, When::YouGainLife, vec![], vec![Effect::CountersSelf { n: 1 }])
        }),
        "Epicure of Blood" => s(&["Whenever you gain life, each opponent loses 1 life."], |d, _| {
            on(d, When::YouGainLife, vec![], vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }])
        }),
        "Gixian Puppeteer" => s(
            &[
                "Whenever you draw your second card each turn, each opponent loses 2 life and you gain 2 life.",
                "When this creature dies, return another target creature card with mana value 3 or less from your graveyard to the battlefield.",
            ],
            |d, _| {
                on(
                    d,
                    When::YouDrawSecond,
                    vec![],
                    vec![Effect::LoseLife { who: Who::EachOpponent, n: 2 }, Effect::GainLife { who: Who::You, n: 2 }],
                );
                let card = Filter { other: true, max_mana_value: Some(3), ..graveyard_cards(true, Controller::You) };
                on(d, When::Dies, vec![card], vec![Effect::Reanimate(0)]);
            },
        ),
        "Ob Nixilis, the Hate-Twisted" => s(
            &[
                "Whenever an opponent draws a card, Ob Nixilis deals 1 damage to that player.",
                "−2: Destroy target creature. Its controller draws two cards.",
            ],
            |d, _| {
                on(d, When::OpponentDraws, vec![], vec![damage(1, Recipient::EachOpponent)]);
                loyalty(
                    d,
                    -2,
                    Ability::new(vec![CREATURE], vec![Effect::Destroy(0), Effect::Draw { who: Who::ControllerOf(0), n: 2 }]),
                );
            },
        ),
        // ---- Animar, Soul of Elements (Adam) ----
        "Consecrated Sphinx" => s(&["Whenever an opponent draws a card, you may draw two cards."], |d, _| {
            may(d, When::OpponentDraws, vec![], vec![draw(2)])
        }),
        "Fierce Guardianship" => s(
            &[
                "If you control a commander, you may cast this spell without paying its mana cost.",
                "Counter target noncreature spell.",
            ],
            |d, _| {
                d.alt_cost = Some((ManaCost::default(), 0));
                d.alt_cost_needs_commander = true;
                spell(d, vec![Filter { spell_creature: Some(false), ..SPELL }], vec![Effect::Counter(0)]);
            },
        ),
        "Steam Vents" => s(
            &["As this land enters, you may pay 2 life. If you don't, it enters tapped."],
            |d, _| {
                d.shock_land = true;
                lands_mana(d, &[Color::Blue, Color::Red]);
            },
        ),
        "Arbor Elf" => s(&["{T}: Untap target Forest."], |d, _| {
            let forest = Filter {
                players: false,
                creatures: false,
                planeswalkers: false,
                any_permanent: true,
                subtype: Some("Forest"),
                ..Filter::ANY
            };
            activated(d, "", true, vec![forest], vec![Effect::UntapTarget(0)]);
        }),
        "Peregrine Drake" => s(&["When this creature enters, untap up to five lands."], |d, _| {
            on(d, When::Enters, vec![], vec![Effect::UntapLands(5)])
        }),
        "Cloud of Faeries" => s(&["When this creature enters, untap up to two lands.", "Cycling {2}"], |d, _| {
            on(d, When::Enters, vec![], vec![Effect::UntapLands(2)]);
            cycling(d, "{2}");
        }),
        "Ancient Ziggurat" => s(
            &["{T}: Add one mana of any color. Spend this mana only to cast a creature spell."],
            |d, _| {
                lands_mana(d, &Color::ALL);
                d.mana_only_for_creatures = true;
            },
        ),
        "Beastcaller Savant" => s(
            &["{T}: Add one mana of any color. Spend this mana only to cast a creature spell."],
            |d, _| {
                lands_mana(d, &Color::ALL);
                d.mana_only_for_creatures = true;
            },
        ),
        "Elvish Visionary" => s(&["When this creature enters, draw a card."], |d, _| {
            on(d, When::Enters, vec![], vec![draw(1)])
        }),
        "Pestermite" => s(&["When this creature enters, you may tap or untap target permanent."], |d, _| {
            let any = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, ..Filter::ANY };
            may(d, When::Enters, vec![any], vec![Effect::TapOrUntap(0)]);
        }),
        "Shared Summons" => s(
            &["Search your library for up to two creature cards with different names, reveal them, put them into your hand, then shuffle."],
            |d, _| {
                // A singleton deck: every creature card in it has a different name already.
                let what = SearchFor::CreatureMvAtLeast(0);
                spell(d, vec![], vec![Effect::Search { what, to: SearchTo::Hand, n: 2, kicked_n: None }]);
            },
        ),
        "Soul of the Harvest" => s(
            &["Whenever another nontoken creature you control enters, you may draw a card."],
            |d, _| {
                let that = Filter { triggering: true, ..CREATURE };
                may(d, When::AnotherNontokenCreatureYouControlEnters, vec![that], vec![draw(1)]);
            },
        ),
        // ---- Atraxa, Praetors' Voice (Adam) ----
        "Gavony Township" => s(
            &["{T}: Add {C}.", "{2}{G}{W}, {T}: Put a +1/+1 counter on each creature you control."],
            |d, _| {
                colorless(d);
                activated(d, "{2}{G}{W}", true, vec![], vec![Effect::CountersAll { who: Who::You, n: 1 }]);
            },
        ),
        // ---- Lonis, Cryptozoologist (Spr3adsh33t) ----
        "Adrix and Nev, Twincasters" => s(
            &[
                "Ward {2}",
                "If one or more tokens would be created under your control, twice that many of those tokens are created instead.",
            ],
            |d, _| {
                d.ward = ManaCost::parse("{2}");
                d.doubles_tokens = true;
            },
        ),
        "Displace" => s(
            &["Exile up to two target creatures you control, then return those cards to the battlefield under their owner's control."],
            |d, _| d.spell = Some(up_to(vec![YOUR_CREATURE, YOUR_CREATURE], vec![Effect::BlinkSlot(0), Effect::BlinkSlot(1)])),
        ),
        "Ghostly Flicker" => s(
            &["Exile two target artifacts, creatures, and/or lands you control, then return those cards to the battlefield under your control."],
            |d, _| {
                let yours = Filter {
                    players: false,
                    planeswalkers: false,
                    artifacts: true,
                    lands: true,
                    controller: Controller::You,
                    ..Filter::ANY
                };
                spell(d, vec![yours, yours], vec![Effect::BlinkSlot(0), Effect::BlinkSlot(1)]);
            },
        ),
        "Blur" => s(
            &[
                "Exile target creature you control, then return that card to the battlefield under its owner's control.",
                "Draw a card.",
            ],
            |d, _| spell(d, vec![YOUR_CREATURE], vec![Effect::BlinkSlot(0), draw(1)]),
        ),
        "Botanical Sanctum" => s(
            &["This land enters tapped unless you control two or fewer other lands.", "{T}: Add {G} or {U}."],
            |d, _| {
                d.enters_tapped_unless_at_most_lands = Some(2);
                lands_mana(d, &[Color::Green, Color::Blue]);
            },
        ),
        "Vineglimmer Snarl" => s(
            &[
                "As this land enters, you may reveal a Forest or Island card from your hand. If you don't, this land enters tapped.",
                "{T}: Add {G} or {U}.",
            ],
            |d, _| {
                d.enters_tapped_unless_reveal = Some(["Forest", "Island"]);
                lands_mana(d, &[Color::Green, Color::Blue]);
            },
        ),
        "Simic Cluestone" => s(&["{T}: Add {G} or {U}.", "{G}{U}, {T}, Sacrifice this artifact: Draw a card."], |d, _| {
            lands_mana(d, &[Color::Green, Color::Blue]);
            let n = activated(d, "{G}{U}", true, vec![], vec![draw(1)]);
            d.activated[n].sacrifice_self = true;
        }),
        "Shrieking Drake" => s(&["When this creature enters, return a creature you control to its owner's hand."], |d, _| {
            on(d, When::Enters, vec![], vec![Effect::ReturnCreatureYouControl])
        }),
        "Kiora's Follower" => s(&["{T}: Untap another target permanent."], |d, _| {
            let another = Filter {
                players: false,
                creatures: false,
                planeswalkers: false,
                any_permanent: true,
                other: true,
                ..Filter::ANY
            };
            activated(d, "", true, vec![another], vec![Effect::UntapTarget(0)]);
        }),
        "Thassa, Deep-Dwelling" => s(
            &[
                "As long as your devotion to blue is less than five, Thassa isn't a creature.",
                "At the beginning of your end step, exile up to one other target creature you control, then return that card to the battlefield under your control.",
                "{3}{U}: Tap another target creature.",
            ],
            |d, _| {
                d.creature_unless_devotion = Some(([false, true, false, false, false], 5));
                let other = Filter { other: true, ..YOUR_CREATURE };
                d.triggers.push(Trigger {
                    when: When::EndStep { your_turn_only: true },
                    ability: up_to(vec![other], vec![Effect::BlinkSlot(0)]),
                });
                activated(d, "{3}{U}", false, vec![Filter { other: true, ..CREATURE }], vec![Effect::Tap(0)]);
            },
        ),
        "Displacer Kitten" => s(
            &["Avoidance — Whenever you cast a noncreature spell, exile up to one target nonland permanent you control, then return that card to the battlefield under its owner's control."],
            |d, _| {
                let yours = Filter { controller: Controller::Any, ..OPP_NONLAND }.controlled_by(Controller::You);
                d.triggers.push(Trigger {
                    when: When::YouCastNoncreature,
                    ability: up_to(vec![yours], vec![Effect::BlinkSlot(0)]),
                });
            },
        ),
        // ---- Unblocked by lane d3's core theme (effects and statics), batch 1 ----
        "Altar of Dementia" => s(
            &["Sacrifice a creature: Target player mills cards equal to the sacrificed creature's power."],
            |d, _| {
                let n = activated(d, "", false, vec![Filter::PLAYER], vec![Effect::MillAmount(0)]);
                d.activated[n].sacrifice_creature = true;
                d.activated[n].amount_from_victim_power = true;
            },
        ),
        "Creeping Corrosion" => s(&["Destroy all artifacts."], |d, _| spell(d, vec![], vec![Effect::DestroyAllArtifacts])),
        "Armageddon" => s(&["Destroy all lands."], |d, _| spell(d, vec![], vec![Effect::DestroyAllLands])),
        "Damnable Pact" => s(&["Target player draws X cards and loses X life."], |d, _| {
            spell(d, vec![Filter::PLAYER], vec![Effect::TargetDrawsXLosesX(0)])
        }),
        "Merciless Eviction" => s(
            &[
                "Choose one —",
                "• Exile all artifacts.",
                "• Exile all creatures.",
                "• Exile all enchantments.",
                "• Exile all planeswalkers.",
            ],
            |d, _| modes(d, (0..4).map(|t| Ability::new(vec![], vec![Effect::ExileAllOfType(t)])).collect()),
        ),
        "Dramatic Reversal" => s(&["Untap all nonland permanents you control."], |d, _| {
            spell(d, vec![], vec![Effect::UntapNonlandYouControl])
        }),
        "Venser's Journal" => s(
            &["You have no maximum hand size.", "At the beginning of your upkeep, you gain 1 life for each card in your hand."],
            |d, _| {
                d.no_max_hand_size = true;
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::GainLifePerCardInHand]);
            },
        ),
        "Nissa, Voice of Zendikar" => s(
            &[
                "+1: Create a 0/1 green Plant creature token.",
                "−2: Put a +1/+1 counter on each creature you control.",
                "−7: You gain X life and draw X cards, where X is the number of lands you control.",
            ],
            |d, t| {
                loyalty(d, 1, Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.plant, n: 1 }]));
                loyalty(d, -2, Ability::new(vec![], vec![Effect::CountersAll { who: Who::You, n: 1 }]));
                loyalty(d, -7, Ability::new(vec![], vec![Effect::GainAndDrawPerLandYouControl]));
            },
        ),
        "Academy Rector" => s(
            &["When this creature dies, you may exile it. If you do, search your library for an enchantment card, put that card onto the battlefield, then shuffle."],
            |d, _| {
                let find = search(SearchFor::Enchantment, SearchTo::Battlefield { tapped: false });
                may(d, When::Dies, vec![], vec![Effect::ExileSourceFromGraveyardOrStop, find]);
            },
        ),
        "Seizan, Perverter of Truth" => s(
            &["At the beginning of each player's upkeep, that player loses 2 life and draws two cards."],
            |d, _| {
                let that = Who::ActivePlayer;
                on(
                    d,
                    When::Upkeep { your_turn_only: false },
                    vec![],
                    vec![Effect::LoseLife { who: that, n: 2 }, Effect::Draw { who: that, n: 2 }],
                );
            },
        ),
        "Mystical Tutor" => s(
            &["Search your library for an instant or sorcery card, reveal it, then shuffle and put that card on top."],
            |d, _| spell(d, vec![], vec![search(SearchFor::InstantOrSorcery, SearchTo::Top)]),
        ),
        "Enlightened Tutor" => s(
            &["Search your library for an artifact or enchantment card, reveal it, then shuffle and put that card on top."],
            |d, _| spell(d, vec![], vec![search(SearchFor::ArtifactOrEnchantment, SearchTo::Top)]),
        ),
        "Unmarked Grave" => s(
            &["Search your library for a nonlegendary card, put that card into your graveyard, then shuffle."],
            |d, _| spell(d, vec![], vec![search(SearchFor::Nonlegendary, SearchTo::Graveyard)]),
        ),
        "Imperial Recruiter" => s(
            &["When this creature enters, search your library for a creature card with power 2 or less, reveal it, put it into your hand, then shuffle."],
            |d, _| on(d, When::Enters, vec![], vec![search(SearchFor::CreaturePowerAtMost(2), SearchTo::Hand)]),
        ),
        "Avacyn, Angel of Hope" => s(&["Other permanents you control have indestructible."], |d, _| {
            static_on(d, Scope::OtherYourPermanents, false, Keywords::INDESTRUCTIBLE)
        }),
        "Elesh Norn, Grand Cenobite" => s(
            &["Other creatures you control get +2/+2.", "Creatures your opponents control get -2/-2."],
            |d, _| {
                static_on(d, Scope::OtherYourCreatures, false, Keywords::NONE);
                let s = d.statics.last_mut().expect("just added");
                (s.power, s.toughness) = (2, 2);
                static_on(d, Scope::OpponentsCreatures, false, Keywords::NONE);
                let s = d.statics.last_mut().expect("just added");
                (s.power, s.toughness) = (-2, -2);
            },
        ),
        "Atraxa, Praetors' Voice" => s(&["At the beginning of your end step, proliferate."], |d, _| {
            on(d, When::EndStep { your_turn_only: true }, vec![], vec![Effect::Proliferate])
        }),
        "Karn's Bastion" => s(&["{T}: Add {C}.", "{4}, {T}: Proliferate."], |d, _| {
            colorless(d);
            activated(d, "{4}", true, vec![], vec![Effect::Proliferate]);
        }),
        // ---- Unblocked by lane d3's core theme (effects and statics), batch 2 ----
        "Wrath of God" => s(&["Destroy all creatures. They can't be regenerated."], |d, _| {
            spell(d, vec![], vec![Effect::DestroyAllCreaturesNoRegen])
        }),
        "Sword of Feast and Famine" => s(
            &[
                "Equipped creature gets +2/+2 and has protection from black and from green.",
                "Whenever equipped creature deals combat damage to a player, that player discards a card and you untap all lands you control.",
                "Equip {2}",
            ],
            |d, _| {
                attached(d, false, 2, 2, Keywords::NONE);
                d.statics.last_mut().expect("just added").protection = [false, false, true, false, true];
                let discard = Effect::Discard { who: Who::EachOpponent, n: 1 };
                on(d, When::EquippedDealsCombatDamageToPlayer, vec![], vec![discard, Effect::UntapLandsYouControl]);
                equip(d, "{2}");
            },
        ),
        "Regal Force" => s(&["When this creature enters, draw a card for each green creature you control."], |d, _| {
            on(d, When::Enters, vec![], vec![Effect::DrawPerColoredCreatureYouControl(4)])
        }),
        "Loxodon Hierarch" => s(
            &["When this creature enters, you gain 4 life.", "{G}{W}, Sacrifice this creature: Regenerate each creature you control."],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::GainLife { who: Who::You, n: 4 }]);
                let n = activated(d, "{G}{W}", false, vec![], vec![Effect::RegenerateEachCreatureYouControl]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Blessing of Leeches" => s(
            &["Enchant creature", "At the beginning of your upkeep, you lose 1 life.", "{0}: Regenerate enchanted creature."],
            |d, _| {
                d.aura = true;
                d.spell = Some(Ability::new(vec![CREATURE], vec![]));
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::LoseLife { who: Who::You, n: 1 }]);
                activated(d, "{0}", false, vec![], vec![Effect::RegenerateAttached]);
            },
        ),
        "Emissary of Despair" => s(
            &["Whenever this creature deals combat damage to a player, that player loses 1 life for each artifact they control."],
            |d, _| {
                let lose = Effect::LoseLifePerArtifactTheyControl(Who::EachOpponent);
                on(d, When::DealsCombatDamageToPlayer, vec![], vec![lose]);
            },
        ),
        "Return of the Wildspeaker" => s(
            &[
                "Choose one —",
                "• Draw cards equal to the greatest power among non-Human creatures you control.",
                "• Non-Human creatures you control get +3/+3 until end of turn.",
            ],
            |d, _| {
                let pump = Effect::PumpNonHumanYouControl { power: 3, toughness: 3 };
                modes(
                    d,
                    vec![
                        Ability::new(vec![], vec![Effect::DrawGreatestPowerNonHuman]),
                        Ability::new(vec![], vec![pump]),
                    ],
                );
            },
        ),
        "Last March of the Ents" => s(
            &[
                "This spell can't be countered.",
                "Draw cards equal to the greatest toughness among creatures you control, then put any number of creature cards from your hand onto the battlefield.",
            ],
            |d, _| {
                d.uncounterable = true;
                let put = Effect::CreaturesFromHand;
                spell(d, vec![], vec![Effect::DrawGreatestToughnessYouControl, put]);
            },
        ),
        "Tooth and Nail" => s(
            &[
                "Choose one —",
                "• Search your library for up to two creature cards, reveal them, put them into your hand, then shuffle.",
                "• Put up to two creature cards from your hand onto the battlefield.",
                "Entwine {2}",
            ],
            |d, _| {
                let find = Effect::Search { what: SearchFor::CreatureMvAtLeast(0), to: SearchTo::Hand, n: 2, kicked_n: None };
                let put = Effect::CreaturesFromHandUpTo(2);
                modes(
                    d,
                    vec![
                        Ability::new(vec![], vec![find]),
                        Ability::new(vec![], vec![put]),
                        Ability::new(vec![], vec![find, put]), // entwined
                    ],
                );
                d.entwine = ManaCost::parse("{2}");
            },
        ),
        "Ponder" => s(
            &["Look at the top three cards of your library, then put them back in any order. You may shuffle.", "Draw a card."],
            |d, _| spell(d, vec![], vec![Effect::LookTopReorder(3), Effect::MayShuffle, draw(1)]),
        ),
        "Dream Stalker" => s(&["When this creature enters, return a permanent you control to its owner's hand."], |d, _| {
            on(d, When::Enters, vec![], vec![Effect::ReturnPermanentYouControl { nonland: false }])
        }),
        "Ancestral Statue" => s(&["When this creature enters, return a nonland permanent you control to its owner's hand."], |d, _| {
            on(d, When::Enters, vec![], vec![Effect::ReturnPermanentYouControl { nonland: true }])
        }),
        "Weird Harvest" => s(
            &["Each player may search their library for up to X creature cards, reveal those cards, put them into their hand, then shuffle."],
            |d, _| {
                let each = [Effect::SearchCreaturesUpToX(Who::You), Effect::SearchCreaturesUpToX(Who::EachOpponent)];
                spell(d, vec![], each.to_vec());
            },
        ),
        "Relentless Assault" => s(
            &["Untap all creatures that attacked this turn. After this main phase, there is an additional combat phase followed by an additional main phase."],
            |d, _| spell(d, vec![], vec![Effect::UntapCreaturesAttackedThisTurn, Effect::AdditionalCombatAndMain]),
        ),
        "Cyberdrive Awakener" => s(
            &[
                "Other artifact creatures you control have flying.",
                "When this creature enters, each noncreature artifact you control becomes a 4/4 artifact creature until end of turn.",
            ],
            |d, _| {
                static_on(d, Scope::OtherYourArtifactCreatures, false, Keywords::FLYING);
                let animate = Effect::NoncreatureArtifactsBecomeCreatures { power: 4, toughness: 4 };
                on(d, When::Enters, vec![], vec![animate]);
            },
        ),
        "Oriq Loremage" => s(
            &["{T}: Search your library for a card, put it into your graveyard, then shuffle. If it's an instant or sorcery card, put a +1/+1 counter on this creature."],
            |d, _| {
                activated(d, "", true, vec![], vec![Effect::SearchToGraveyardCounterIfInstantOrSorcery]);
            },
        ),
        _ => None,
    }
}

/// This lane's scripted playgroup cards (checked against their text in `lib.rs`).
pub const PG_D3_SCRIPTED: &[&str] = &[
    // Sefris of the Hidden Ways
    "Isolated Chapel",
    "Dimir Signet",
    "Orzhov Signet",
    "Raffine's Tower",
    "Counterspell",
    "Entomb",
    "Buried Alive",
    "Darkwater Catacombs",
    "Geier Reach Sanitarium",
    "Razaketh, the Foulblooded",
    "Ashen Rider",
    "Arcane Sanctum",
    "Cavalier of Night",
    "Choked Estuary",
    "Darkslick Shores",
    "Merfolk Looter",
    "Thought Courier",
    "Port Town",
    "Vile Entomber",
    // Najeela, the Blade-Blossom
    "Flooded Strand",
    "Hallowed Fountain",
    "Ziatora's Proving Ground",
    "Ketria Triome",
    "Savai Triome",
    "Selesnya Signet",
    "Boros Charm",
    "Brainstorm",
    "Edric, Spymaster of Trest",
    "Opt",
    "Talisman of Dominance",
    "Talisman of Curiosity",
    "Fleshbag Marauder",
    "Growth Spiral",
    "Preordain",
    "City of Death",
    "Sylvan Caryatid",
    // Sheoldred, the Apocalypse
    "Decanter of Endless Water",
    "Temple Bell",
    "Charcoal Diamond",
    "Fate Unraveler",
    "Underworld Dreams",
    "Font of Mythos",
    "Greed",
    "Infernal Grasp",
    "Liliana's Caress",
    "Master of the Feast",
    "Mikokoro, Center of the Sea",
    "Bloodthirsty Aerialist",
    "Epicure of Blood",
    "Gixian Puppeteer",
    "Ob Nixilis, the Hate-Twisted",
    // Animar, Soul of Elements
    "Consecrated Sphinx",
    "Fierce Guardianship",
    "Steam Vents",
    "Arbor Elf",
    "Peregrine Drake",
    "Cloud of Faeries",
    "Ancient Ziggurat",
    "Beastcaller Savant",
    "Elvish Visionary",
    "Pestermite",
    "Shared Summons",
    "Soul of the Harvest",
    // Atraxa, Praetors' Voice (Gavony Township); Lonis, Cryptozoologist (the rest)
    "Gavony Township",
    "Adrix and Nev, Twincasters",
    "Displace",
    "Ghostly Flicker",
    "Blur",
    "Botanical Sanctum",
    "Vineglimmer Snarl",
    "Simic Cluestone",
    "Shrieking Drake",
    "Kiora's Follower",
    "Thassa, Deep-Dwelling",
    "Displacer Kitten",
    // Core theme d3 (effects and statics), batch 1
    "Altar of Dementia",
    "Creeping Corrosion",
    "Armageddon",
    "Damnable Pact",
    "Merciless Eviction",
    "Dramatic Reversal",
    "Venser's Journal",
    "Nissa, Voice of Zendikar",
    "Academy Rector",
    "Seizan, Perverter of Truth",
    "Mystical Tutor",
    "Enlightened Tutor",
    "Unmarked Grave",
    "Imperial Recruiter",
    "Avacyn, Angel of Hope",
    "Elesh Norn, Grand Cenobite",
    "Atraxa, Praetors' Voice",
    "Karn's Bastion",
    // Core theme d3 (effects and statics), batch 2
    "Wrath of God",
    "Sword of Feast and Famine",
    "Regal Force",
    "Loxodon Hierarch",
    "Blessing of Leeches",
    "Emissary of Despair",
    "Return of the Wildspeaker",
    "Last March of the Ents",
    "Tooth and Nail",
    "Ponder",
    "Dream Stalker",
    "Ancestral Statue",
    "Weird Harvest",
    "Relentless Assault",
    "Cyberdrive Awakener",
    "Oriq Loremage",
];
