//! Playgroup decks (Nerd Leagues), lane D3 (the desktop): the cards for the deck in handoff/lanes/d3.json. Conventions:
//! docs/lane-brief.md. Card text: assets/PG_oracle.json (in the CMD pool). Tests: tests/pg_d3.rs.
#![allow(unused_imports)]

use super::*;

/// "target permanent": any permanent, lands included.
const PERMANENT: Filter = Filter {
    players: false,
    creatures: false,
    planeswalkers: false,
    any_permanent: true,
    ..Filter::ANY
};

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
        // ---- Unblocked by lane d3's core theme, batch 3 (target filters, scaled statics) ----
        "Go for the Throat" => s(&["Destroy target nonartifact creature."], |d, _| {
            spell(d, vec![Filter { nonartifact: true, ..CREATURE }], vec![Effect::Destroy(0)])
        }),
        "Doom Blade" => s(&["Destroy target nonblack creature."], |d, _| {
            spell(d, vec![Filter { not_color: Some(2), ..CREATURE }], vec![Effect::Destroy(0)])
        }),
        "Red Elemental Blast" => s(
            &["Choose one —", "• Counter target blue spell.", "• Destroy target blue permanent."],
            |d, _| {
                let blue_spell = Filter { color: Some(1), ..SPELL };
                let blue_permanent = Filter { color: Some(1), ..PERMANENT };
                modes(
                    d,
                    vec![
                        Ability::new(vec![blue_spell], vec![Effect::Counter(0)]),
                        Ability::new(vec![blue_permanent], vec![Effect::Destroy(0)]),
                    ],
                );
            },
        ),
        "Pyroblast" => s(
            &["Choose one —", "• Counter target spell if it's blue.", "• Destroy target permanent if it's blue."],
            |d, _| {
                modes(
                    d,
                    vec![
                        Ability::new(vec![SPELL], vec![Effect::CounterIfColor { slot: 0, color: 1 }]),
                        Ability::new(vec![PERMANENT], vec![Effect::DestroyIfColor { slot: 0, color: 1 }]),
                    ],
                );
            },
        ),
        "Sultai Charm" => s(
            &[
                "Choose one —",
                "• Destroy target monocolored creature.",
                "• Destroy target artifact or enchantment.",
                "• Draw two cards, then discard a card.",
            ],
            |d, _| {
                modes(
                    d,
                    vec![
                        Ability::new(vec![Filter { monocolored: true, ..CREATURE }], vec![Effect::Destroy(0)]),
                        Ability::new(vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::Destroy(0)]),
                        Ability::new(vec![], vec![draw(2), discard(1)]),
                    ],
                );
            },
        ),
        "Disdainful Stroke" => s(&["Counter target spell with mana value 4 or greater."], |d, _| {
            spell(d, vec![Filter { spell_min_mana_value: Some(4), ..SPELL }], vec![Effect::Counter(0)])
        }),
        "Strix Serenade" => s(
            &["Counter target artifact, creature, or planeswalker spell. Its controller creates a 2/2 blue Bird creature token with flying."],
            |d, t| {
                let target = Filter { spell_artifact_creature_or_planeswalker: true, ..SPELL };
                let bird = Effect::CreateToken { who: Who::ControllerOf(0), def: t.bird_2, n: 1 };
                spell(d, vec![target], vec![bird, Effect::Counter(0)]);
            },
        ),
        "Siren Stormtamer" => s(
            &["{U}, Sacrifice this creature: Counter target spell or ability that targets you or a creature you control."],
            |d, _| {
                let target = Filter { abilities: true, targets_you_or_your_creature: true, ..SPELL };
                let n = activated(d, "{U}", false, vec![target], vec![Effect::CounterSpellOrAbility(0)]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Cranial Plating" => s(
            &[
                "Equipped creature gets +1/+0 for each artifact you control.",
                "{B}{B}: Attach this Equipment to target creature you control.",
                "Equip {1}",
            ],
            |d, _| {
                attached(d, false, 1, 0, Keywords::NONE);
                d.statics.last_mut().expect("just added").scale = Some(Scale::PerArtifactYouControl);
                activated(d, "{B}{B}", false, vec![YOUR_CREATURE], vec![Effect::Attach { slot: 0 }]);
                equip(d, "{1}");
            },
        ),
        "Ancestral Mask" => s(
            &["Enchant creature", "Enchanted creature gets +2/+2 for each other enchantment on the battlefield."],
            |d, _| {
                d.aura = true;
                d.spell = Some(Ability::new(vec![CREATURE], vec![]));
                attached(d, false, 2, 2, Keywords::NONE);
                d.statics.last_mut().expect("just added").scale = Some(Scale::PerOtherEnchantmentOnBattlefield);
            },
        ),
        "Knowledge Is Power" => s(
            &["Creatures you control get +X/+X, where X is the number of cards you've drawn this turn."],
            |d, _| {
                static_on(d, Scope::YourCreatures, false, Keywords::NONE);
                let s = d.statics.last_mut().expect("just added");
                (s.power, s.toughness, s.scale) = (1, 1, Some(Scale::PerCardDrawnThisTurn));
            },
        ),
        "Multani, Maro-Sorcerer" => s(
            &["Multani's power and toughness are each equal to the total number of cards in all players' hands."],
            |d, _| {
                static_on(d, Scope::This, false, Keywords::NONE);
                let s = d.statics.last_mut().expect("just added");
                (s.power, s.toughness, s.scale) = (1, 1, Some(Scale::PerCardInAllHands));
            },
        ),
        "Pathbreaker Ibex" => s(
            &["Whenever this creature attacks, creatures you control gain trample and get +X/+X until end of turn, where X is the greatest power among creatures you control."],
            |d, _| on(d, When::Attacks, vec![], vec![Effect::PumpAllGreatestPowerGrant(Keywords::TRAMPLE)]),
        ),
        // ---- Unblocked by lane d3's core theme, batch 4 ----
        "Mutilate" => s(&["All creatures get -1/-1 until end of turn for each Swamp you control."], |d, _| {
            spell(d, vec![], vec![Effect::AllCreaturesMinusPerSwampYouControl])
        }),
        "Defile" => s(&["Target creature gets -1/-1 until end of turn for each Swamp you control."], |d, _| {
            spell(d, vec![CREATURE], vec![Effect::MinusPerSwampYouControl(0)])
        }),
        "Seeds of Innocence" => s(
            &["Destroy all artifacts. They can't be regenerated. The controller of each of those artifacts gains life equal to its mana value."],
            |d, _| spell(d, vec![], vec![Effect::DestroyAllArtifactsNoRegenGainManaValue]),
        ),
        "Trophy Mage" => s(
            &["When this creature enters, you may search your library for an artifact card with mana value 3, reveal it, put it into your hand, then shuffle."],
            |d, _| may(d, When::Enters, vec![], vec![search(SearchFor::ArtifactManaValue(3), SearchTo::Hand)]),
        ),
        "Jirina, Dauntless General" => s(
            &["When Jirina enters, exile target player's graveyard.", "Sacrifice Jirina: Humans you control gain hexproof and indestructible until end of turn."],
            |d, _| {
                on(d, When::Enters, vec![Filter::PLAYER], vec![Effect::ExileGraveyard { who: Who::TargetPlayer(0) }]);
                let grant = Effect::GrantSubtypeYouControlEot { sub: "Human", keywords: Keywords::HEXPROOF.with(Keywords::INDESTRUCTIBLE) };
                let n = activated(d, "", false, vec![], vec![grant]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Privileged Position" => s(&["Other permanents you control have hexproof."], |d, _| {
            static_on(d, Scope::OtherYourPermanents, false, Keywords::HEXPROOF)
        }),
        "Sterling Grove" => s(
            &[
                "Other enchantments you control have shroud.",
                "{1}, Sacrifice this enchantment: Search your library for an enchantment card, reveal it, then shuffle and put that card on top.",
            ],
            |d, _| {
                static_on(d, Scope::OtherYourEnchantments, false, Keywords::SHROUD);
                let n = activated(d, "{1}", false, vec![], vec![search(SearchFor::Enchantment, SearchTo::Top)]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Farewell" => s(
            &[
                "Choose one or more —",
                "• Exile all artifacts.",
                "• Exile all creatures.",
                "• Exile all enchantments.",
                "• Exile all graveyards.",
            ],
            |d, _| {
                // Every nonempty set of the four modes, as one mode each (bit i: mode i).
                let one = |i: u32| match i {
                    0 => vec![Effect::ExileAllOfType(0)],
                    1 => vec![Effect::ExileAllOfType(1)],
                    2 => vec![Effect::ExileAllOfType(2)],
                    _ => vec![Effect::ExileGraveyard { who: Who::You }, Effect::ExileGraveyard { who: Who::EachOpponent }],
                };
                let sets = (1u32..16).map(|m| Ability::new(vec![], (0..4).filter(|i| m & (1 << i) != 0).flat_map(one).collect()));
                modes(d, sets.collect());
            },
        ),
        "Gravitational Shift" => s(&["Creatures with flying get +2/+0.", "Creatures without flying get -2/-0."], |d, _| {
            static_on(d, Scope::CreaturesWithFlying, false, Keywords::NONE);
            d.statics.last_mut().expect("just added").power = 2;
            static_on(d, Scope::CreaturesWithoutFlying, false, Keywords::NONE);
            d.statics.last_mut().expect("just added").power = -2;
        }),
        "Shield of the Oversoul" => s(
            &[
                "Enchant creature",
                "As long as enchanted creature is green, it gets +1/+1 and has indestructible.",
                "As long as enchanted creature is white, it gets +1/+1 and has flying.",
            ],
            |d, _| {
                d.aura = true;
                d.spell = Some(Ability::new(vec![CREATURE], vec![]));
                attached(d, false, 1, 1, Keywords::INDESTRUCTIBLE);
                d.statics.last_mut().expect("just added").condition = Some(StaticCondition::AttachedIsColor(4));
                attached(d, false, 1, 1, Keywords::FLYING);
                d.statics.last_mut().expect("just added").condition = Some(StaticCondition::AttachedIsColor(0));
            },
        ),
        "Sword of the Paruns" => s(
            &[
                "As long as equipped creature is tapped, tapped creatures you control get +2/+0.",
                "As long as equipped creature is untapped, untapped creatures you control get +0/+2.",
                "{3}: You may tap or untap equipped creature.",
                "Equip {3}",
            ],
            |d, _| {
                static_on(d, Scope::YourTappedCreatures, false, Keywords::NONE);
                let s = d.statics.last_mut().expect("just added");
                (s.power, s.condition) = (2, Some(StaticCondition::AttachedTapped(true)));
                static_on(d, Scope::YourUntappedCreatures, false, Keywords::NONE);
                let s = d.statics.last_mut().expect("just added");
                (s.toughness, s.condition) = (2, Some(StaticCondition::AttachedTapped(false)));
                activated(d, "{3}", false, vec![], vec![Effect::TapOrUntapAttached]);
                equip(d, "{3}");
            },
        ),
        "Inexorable Tide" => s(&["Whenever you cast a spell, proliferate."], |d, _| {
            on(d, When::YouCast, vec![], vec![Effect::Proliferate])
        }),
        "Volt Charge" => s(&["Volt Charge deals 3 damage to any target. Proliferate."], |d, _| {
            spell(d, vec![Filter::ANY], vec![damage(3, Recipient::Target(0)), Effect::Proliferate])
        }),
        // ---- Unblocked by lane d3's core theme, batch 5 ----
        "Cataclysmic Gearhulk" => s(
            &["When this creature enters, each player chooses an artifact, a creature, an enchantment, and a planeswalker from among the nonland permanents they control, then sacrifices the rest."],
            |d, _| on(d, When::Enters, vec![], vec![Effect::EachKeepsOneNonlandOfEachType]),
        ),
        "Teferi's Puzzle Box" => s(
            &["At the beginning of each player's draw step, that player puts the cards in their hand on the bottom of their library in any order, then draws that many cards."],
            |d, _| on(d, When::EachDrawStep, vec![], vec![Effect::HandToBottomDrawThatMany(Who::ActivePlayer)]),
        ),
        "Deadly Brew" => s(
            &["Each player sacrifices a creature or planeswalker of their choice. If you sacrificed a permanent this way, you may return another permanent card from your graveyard to your hand."],
            |d, _| spell(d, vec![], vec![Effect::EachSacrificeCreatureOrPlaneswalkerMayReturnOther]),
        ),
        "Wave of Vitriol" => s(
            &["Each player sacrifices all artifacts, enchantments, and nonbasic lands they control. For each land sacrificed this way, its controller may search their library for a basic land card and put it onto the battlefield tapped. Then each player who searched their library this way shuffles."],
            |d, _| spell(d, vec![], vec![Effect::WaveOfVitriol]),
        ),
        "Signal the Clans" => s(
            &["Search your library for three creature cards and reveal them. If you reveal three cards with different names, choose one of them at random and put that card into your hand. Shuffle the rest into your library."],
            |d, _| spell(d, vec![], vec![Effect::SignalTheClans]),
        ),
        "Moonsilver Key" => s(
            &["{1}, {T}, Sacrifice this artifact: Search your library for an artifact card with a mana ability or a basic land card, reveal it, put it into your hand, then shuffle."],
            |d, _| {
                let n = activated(d, "{1}", true, vec![], vec![search(SearchFor::ArtifactWithManaAbilityOrBasicLand, SearchTo::Hand)]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "God-Eternal Bontu" => s(
            &[
                "When God-Eternal Bontu enters, sacrifice any number of other permanents, then draw that many cards.",
                "When God-Eternal Bontu dies or is put into exile from the battlefield, you may put her into her owner's library third from the top.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::SacrificeOthersDrawThatMany]);
                let mut a = Ability::new(vec![], vec![Effect::SourceThirdFromTop]).optional();
                a.condition = Some(Condition::SourceInGraveyardOrExile);
                d.triggers.push(Trigger { when: When::LeavesBattlefield, ability: a });
            },
        ),
        "Imp's Mischief" => s(
            &["Change the target of target spell with a single target. You lose life equal to that spell's mana value."],
            |d, _| {
                let one = Filter { single_target: true, ..SPELL };
                spell(d, vec![one], vec![Effect::ChangeTheTarget(0), Effect::LoseLifeSlotSpellManaValue(0)]);
            },
        ),
        "Boseiju, Who Endures" => s(
            &[
                "{T}: Add {G}.",
                "Channel — {1}{G}, Discard this card: Destroy target artifact, enchantment, or nonbasic land an opponent controls. That player may search their library for a land card with a basic land type, put it onto the battlefield, then shuffle. This ability costs {1} less to activate for each legendary creature you control.",
            ],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::Green]));
                let f = Filter { lands: true, nonbasic: true, controller: Controller::Opponent, ..ARTIFACT_OR_ENCHANTMENT };
                let n = activated(d, "{1}{G}", false, vec![f], vec![Effect::DestroyControllerMayFetchLandWithBasicType(0)]);
                d.activated[n].from_hand_discard = true;
                d.activated[n].less_per_legendary_creature = true;
            },
        ),
        "Cindervines" => s(
            &[
                "Whenever an opponent casts a noncreature spell, this enchantment deals 1 damage to that player.",
                "{1}, Sacrifice this enchantment: Destroy target artifact or enchantment. This enchantment deals 2 damage to that permanent's controller.",
            ],
            |d, _| {
                on(d, When::OpponentCastsNoncreature, vec![], vec![damage(1, Recipient::EachOpponent)]);
                let n = activated(d, "{1}", false, vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::DestroyDamageController { slot: 0, n: 2 }]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Braids, Arisen Nightmare" => s(
            &["At the beginning of your end step, you may sacrifice an artifact, creature, enchantment, land, or planeswalker. If you do, each opponent may sacrifice a permanent of their choice that shares a card type with it. For each opponent who doesn't, that player loses 2 life and you draw a card."],
            |d, _| on(d, When::EndStep { your_turn_only: true }, vec![], vec![Effect::BraidsEndStep]),
        ),
        "Spellskite" => s(&["{U/P}: Change a target of target spell or ability to this creature."], |d, _| {
            let target = Filter { abilities: true, ..SPELL };
            let n = activated(d, "{U}", false, vec![target], vec![Effect::ChangeATargetToSource(0)]);
            d.activated[n].phyrexian = [0, 1, 0, 0, 0];
        }),
        "Defense of the Heart" => s(
            &["At the beginning of your upkeep, if an opponent controls three or more creatures, sacrifice this enchantment, search your library for up to two creature cards, put those cards onto the battlefield, then shuffle."],
            |d, _| {
                let fetch = Effect::Search { what: SearchFor::CreatureMvAtLeast(0), to: SearchTo::Battlefield { tapped: false }, n: 2, kicked_n: None };
                let mut a = Ability::new(vec![], vec![Effect::SacrificeSource, fetch]);
                a.condition = Some(Condition::OpponentControlsCreaturesAtLeast(3));
                d.triggers.push(Trigger { when: When::Upkeep { your_turn_only: true }, ability: a });
            },
        ),
        "Gift of Estates" => s(
            &["If an opponent controls more lands than you, search your library for up to three Plains cards, reveal them, put them into your hand, then shuffle."],
            |d, _| {
                let plains = SearchFor::LandTypes { types: [true, false, false, false, false], basic: false };
                let fetch = Effect::Search { what: plains, to: SearchTo::Hand, n: 3, kicked_n: None };
                let mut a = Ability::new(vec![], vec![Effect::StopUnlessCondition, fetch]);
                a.condition = Some(Condition::OpponentControlsMoreLands);
                d.spell = Some(a);
            },
        ),
        "Masked Vandal" => s(
            &[
                "Changeling",
                "When this creature enters, you may exile a creature card from your graveyard. If you do, exile target artifact or enchantment an opponent controls.",
            ],
            |d, _| {
                d.changeling = true;
                let theirs = Filter { controller: Controller::Opponent, ..ARTIFACT_OR_ENCHANTMENT };
                on(d, When::Enters, vec![theirs], vec![Effect::MayExileFromGraveyard { n: 1, creature_only: true }, Effect::Exile(0)]);
            },
        ),
        // ---- Unblocked by lane d3's core theme, batch 6 ----
        "Genji Glove" => s(
            &[
                "Equipped creature has double strike.",
                "Whenever equipped creature attacks, if it's the first combat phase of the turn, untap it. After this phase, there is an additional combat phase.",
                "Equip {3}",
            ],
            |d, _| {
                attached(d, false, 0, 0, Keywords::DOUBLE_STRIKE);
                let mut a = Ability::new(vec![], vec![Effect::UntapAttached, Effect::AdditionalCombatAfterThisPhase]);
                a.condition = Some(Condition::FirstCombatOfTurn);
                d.triggers.push(Trigger { when: When::EquippedAttacks, ability: a });
                equip(d, "{3}");
            },
        ),
        "Rise and Shine" => s(
            &[
                "Target noncreature artifact you control becomes a 0/0 artifact creature. Put four +1/+1 counters on each artifact that became a creature this way.",
                "Overload {4}{U}{U}",
            ],
            |d, _| {
                let mine = Filter { noncreature: true, controller: Controller::You, ..ARTIFACT };
                let animate = Effect::SlotBecomesCreatureForGood { slot: 0, power: 0, toughness: 0, subtype: "" };
                spell(d, vec![mine], vec![animate, Effect::Counters { slot: 0, n: 4 }]);
                // Overload {4}{U}{U} is cast as a {3}{U} kicker on {1}{U}: the same total, and "kicked"
                // swaps in the overloaded text ("each" instead of "target").
                d.kicker = ManaCost::parse("{3}{U}");
                d.kicked_spell = Some(Ability::new(vec![], vec![Effect::NoncreatureArtifactsBecomeZeroZeroWithCounters(4)]));
            },
        ),
        // ---- Unblocked by lane d3's core theme, batch 7 ----
        "Winds of Abandon" => s(
            &[
                "Exile target creature you don't control. For each creature exiled this way, its controller searches their library for a basic land card. Those players put those cards onto the battlefield tapped, then shuffle.",
                "Overload {4}{W}{W}",
            ],
            |d, _| {
                spell(d, vec![OPP_CREATURE], vec![Effect::ExileControllerMayFetchBasic(0)]);
                // Overload {4}{W}{W} is cast as a {4}{W} kicker on {1}{W}: the same total, and
                // "kicked" swaps in the overloaded text ("each" instead of "target").
                d.kicker = ManaCost::parse("{4}{W}");
                d.kicked_spell = Some(Ability::new(vec![], vec![Effect::ExileOpponentsCreaturesTheyFetchBasicsTapped]));
            },
        ),
        "Tainted Pact" => s(
            &["Exile the top card of your library. You may put that card into your hand unless it has the same name as another card exiled this way. Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first."],
            |d, _| spell(d, vec![], vec![Effect::TaintedPact]),
        ),
        "Demonlord Belzenlok" => s(
            &["When Demonlord Belzenlok enters, exile cards from the top of your library until you exile a nonland card, then put that card into your hand. If the card's mana value is 4 or greater, repeat this process. Demonlord Belzenlok deals 1 damage to you for each card put into your hand this way."],
            |d, _| on(d, When::Enters, vec![], vec![Effect::BelzenlokDig]),
        ),
        // ---- Unblocked by lane d3's core theme, batch 8 (protection from a chosen quality) ----
        "Benevolent Bodyguard" => s(
            &["Sacrifice this creature: Target creature you control gains protection from the color of your choice until end of turn."],
            |d, _| {
                let n = activated(d, "", false, vec![YOUR_CREATURE], vec![Effect::ProtectionChosenColorEot(Some(0))]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Cartel Aristocrat" => s(
            &["Sacrifice another creature: This creature gains protection from the color of your choice until end of turn."],
            |d, _| {
                let n = activated(d, "", false, vec![], vec![Effect::ProtectionChosenColorEot(None)]);
                d.activated[n].sacrifice_other_creature = true;
            },
        ),
        "Akroma's Will" => s(
            &[
                "Choose one. If you control a commander as you cast this spell, you may choose both instead.",
                "• Creatures you control gain flying, vigilance, and double strike until end of turn.",
                "• Creatures you control gain lifelink, indestructible, and protection from each color until end of turn.",
            ],
            |d, _| {
                let air = Effect::GrantAll { who: Who::You, keywords: Keywords::FLYING.with(Keywords::VIGILANCE).with(Keywords::DOUBLE_STRIKE) };
                let guard = Effect::GrantAll { who: Who::You, keywords: Keywords::LIFELINK.with(Keywords::INDESTRUCTIBLE) };
                let pro = Effect::YourCreaturesProtectionEachColorEot;
                d.modes = vec![
                    Ability::new(vec![], vec![air]),
                    Ability::new(vec![], vec![guard, pro]),
                    Ability::new(vec![], vec![air, guard, pro]),
                ];
                d.last_mode_needs_commander = true;
            },
        ),
        "Serra's Emissary" => s(
            &["As this creature enters, choose a card type.", "You and creatures you control have protection from the chosen card type."],
            |d, _| d.protection_chosen_card_type = true,
        ),
        // ---- Unblocked by lane d3's core theme, batch 9 ----
        "Return to Dust" => s(
            &["Exile target artifact or enchantment. If you cast this spell during your main phase, you may exile up to one other target artifact or enchantment."],
            |d, _| {
                let second = Filter { only_in_your_main_phase: true, ..ARTIFACT_OR_ENCHANTMENT };
                let mut a = Ability::new(vec![ARTIFACT_OR_ENCHANTMENT, second], vec![Effect::Exile(0), Effect::Exile(1)]);
                a.min_targets = Some(1);
                d.spell = Some(a);
            },
        ),
        "Reconnaissance" => s(&["{0}: Remove target attacking creature you control from combat and untap it."], |d, _| {
            let mine = Filter { attacking: true, ..YOUR_CREATURE };
            activated(d, "{0}", false, vec![mine], vec![Effect::RemoveFromCombatAndUntap(0)]);
        }),
        "Raffine, Scheming Seer" => s(
            &[
                "Flying, ward {1}",
                "Whenever you attack, target attacking creature connives X, where X is the number of attacking creatures.",
            ],
            |d, _| {
                d.keywords = d.keywords.with(Keywords::FLYING);
                d.ward = ManaCost::parse("{1}");
                let attacker = Filter { attacking: true, ..YOUR_CREATURE };
                on(d, When::YouAttack, vec![attacker], vec![Effect::ConniveAttackers(0)]);
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
    // Core theme d3, batch 3 (target filters, scaled statics)
    "Go for the Throat",
    "Doom Blade",
    "Red Elemental Blast",
    "Pyroblast",
    "Sultai Charm",
    "Disdainful Stroke",
    "Strix Serenade",
    "Siren Stormtamer",
    "Cranial Plating",
    "Ancestral Mask",
    "Knowledge Is Power",
    "Multani, Maro-Sorcerer",
    "Pathbreaker Ibex",
    // Core theme d3, batch 4
    "Mutilate",
    "Defile",
    "Seeds of Innocence",
    "Trophy Mage",
    "Jirina, Dauntless General",
    "Privileged Position",
    "Sterling Grove",
    "Farewell",
    "Gravitational Shift",
    "Shield of the Oversoul",
    "Sword of the Paruns",
    "Inexorable Tide",
    "Volt Charge",
    // Core theme d3, batch 5
    "Cataclysmic Gearhulk",
    "Teferi's Puzzle Box",
    "Deadly Brew",
    "Wave of Vitriol",
    "Signal the Clans",
    "Moonsilver Key",
    "God-Eternal Bontu",
    "Imp's Mischief",
    "Boseiju, Who Endures",
    "Cindervines",
    "Braids, Arisen Nightmare",
    "Spellskite",
    "Defense of the Heart",
    "Gift of Estates",
    "Masked Vandal",
    // Core theme d3, batch 6
    "Genji Glove",
    "Rise and Shine",
    // Core theme d3, batch 7
    "Winds of Abandon",
    "Tainted Pact",
    "Demonlord Belzenlok",
    // Core theme d3, batch 8
    "Benevolent Bodyguard",
    "Cartel Aristocrat",
    "Akroma's Will",
    "Serra's Emissary",
    // Core theme d3, batch 9
    "Return to Dust",
    "Reconnaissance",
    "Raffine, Scheming Seer",
];
