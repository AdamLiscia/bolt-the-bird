//! Bloomburrow Commander (BLC), lane B: the four precons' cards, most-shared first. Tests:
//! tests/blc_b.rs.
#![allow(unused_imports)]

use super::*;

/// "{T}: Add {A} or {B}." with an enters-tapped rule.
fn dual(d: &mut CardDef, a: Color, b: Color) {
    d.mana = Some(ManaSource::of(&[a, b]));
}

fn colorless(d: &mut CardDef) {
    d.mana = Some(ManaSource {
        colorless: true,
        ..ManaSource::default()
    });
}

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        // ---- Commanders ----
        "Ms. Bumbleflower" => s(
            &["Whenever you cast a spell, target opponent draws a card. Put a +1/+1 counter on target creature. It gains flying until end of turn. If this is the second time this ability has resolved this turn, you draw two cards."],
            |d, _| {
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                on(
                    d,
                    When::YouCast,
                    vec![opp, CREATURE],
                    vec![
                        Effect::Draw { who: Who::TargetPlayer(0), n: 1 },
                        Effect::Counters { slot: 1, n: 1 },
                        Effect::Grant { slot: 1, keywords: Keywords::FLYING },
                        Effect::DrawIfSecondResolutionThisTurn(2),
                    ],
                );
            },
        ),
        "Bello, Bard of the Brambles" => s(
            &["During your turn, each non-Equipment artifact and non-Aura enchantment you control with mana value 4 or greater is a 4/4 Elemental creature in addition to its other types and has indestructible, haste, and \"Whenever this creature deals combat damage to a player, draw a card.\""],
            |d, _| {
                d.animates_big_noncreatures_your_turn = true;
                on(d, When::Never, vec![], vec![draw(1)]); // trigger 0: the granted draw
            },
        ),
        // ---- Mana: the staples every deck shares ----
        "Arcane Signet" => s(&["{T}: Add one mana of any color in your commander's color identity."], |d, _| {
            d.mana_commander_identity = true;
        }),
        "Command Tower" => s(&["{T}: Add one mana of any color in your commander's color identity."], |d, _| {
            d.mana_commander_identity = true;
        }),
        "Exotic Orchard" => s(&["{T}: Add one mana of any color that a land an opponent controls could produce."], |d, _| {
            d.mana_opponent_land_colors = true;
        }),
        "Fellwar Stone" => s(&["{T}: Add one mana of any color that a land an opponent controls could produce."], |d, _| {
            d.mana_opponent_land_colors = true;
        }),
        "Sol Ring" => s(&["{T}: Add {C}{C}."], |d, _| {
            colorless(d);
            d.mana_amount = 2;
        }),
        "Terramorphic Expanse" => s(
            &["{T}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle."],
            |d, _| {
                let n = activated(d, "", true, vec![], vec![search(SearchFor::BasicLand, SearchTo::Battlefield { tapped: true })]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Mind Stone" => s(&["{T}: Add {C}.", "{1}, {T}, Sacrifice this artifact: Draw a card."], |d, _| {
            colorless(d);
            let n = activated(d, "{1}", true, vec![], vec![draw(1)]);
            d.activated[n].sacrifice_self = true;
        }),
        "Glacial Fortress" => s(
            &["This land enters tapped unless you control a Plains or an Island.", "{T}: Add {W} or {U}."],
            |d, _| {
                d.enters_tapped_unless = Some(["Plains", "Island"]);
                dual(d, Color::White, Color::Blue);
            },
        ),
        "Reliquary Tower" => s(&["You have no maximum hand size.", "{T}: Add {C}."], |d, _| {
            d.no_max_hand_size = true;
            colorless(d);
        }),
        "Thought Vessel" => s(&["You have no maximum hand size.", "{T}: Add {C}."], |d, _| {
            d.no_max_hand_size = true;
            colorless(d);
        }),
        "Temple of Enlightenment" => s(
            &["This land enters tapped.", "When this land enters, scry 1.", "{T}: Add {W} or {U}."],
            |d, _| {
                d.enters_tapped = true;
                on(d, When::Enters, vec![], vec![Effect::Scry { n: 1 }]);
                dual(d, Color::White, Color::Blue);
            },
        ),
        "Cultivate" => s(
            &["Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle."],
            |d, _| {
                spell(
                    d,
                    vec![],
                    vec![
                        search(SearchFor::BasicLand, SearchTo::Battlefield { tapped: true }),
                        search(SearchFor::BasicLand, SearchTo::Hand),
                    ],
                )
            },
        ),
        "Farseek" => s(
            &["Search your library for a Plains, Island, Swamp, or Mountain card, put it onto the battlefield tapped, then shuffle."],
            |d, _| {
                let types = land_types(&["Plains", "Island", "Swamp", "Mountain"]);
                spell(d, vec![], vec![search(SearchFor::LandTypes { types, basic: false }, SearchTo::Battlefield { tapped: true })]);
            },
        ),
        // ---- Batch 2: most-shared removal, card draw, lands ----
        "Beast Within" => s(
            &["Destroy target permanent. Its controller creates a 3/3 green Beast creature token."],
            |d, t| {
                let any = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, ..Filter::ANY };
                spell(d, vec![any], vec![Effect::Destroy(0), Effect::CreateToken { who: Who::ControllerOf(0), def: t.beast_3, n: 1 }]);
            },
        ),
        "Big Score" => s(
            &["As an additional cost to cast this spell, discard a card.", "Draw two cards and create two Treasure tokens."],
            |d, t| {
                d.extra_cost = Some(ExtraCost::Discard);
                spell(d, vec![], vec![draw(2), Effect::CreateToken { who: Who::You, def: t.treasure, n: 2 }]);
            },
        ),
        "Bastion of Remembrance" => s(
            &[
                "When this enchantment enters, create a 1/1 white Human Soldier creature token.",
                "Whenever a creature you control dies, each opponent loses 1 life and you gain 1 life.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.human_soldier, n: 1 }]);
                on(
                    d,
                    When::CreatureYouControlDies,
                    vec![],
                    vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }, Effect::GainLife { who: Who::You, n: 1 }],
                );
            },
        ),
        "Bojuka Bog" => s(
            &["This land enters tapped.", "When this land enters, exile target player's graveyard.", "{T}: Add {B}."],
            |d, _| {
                d.enters_tapped = true;
                on(d, When::Enters, vec![Filter::PLAYER], vec![Effect::ExileGraveyard { who: Who::TargetPlayer(0) }]);
                d.mana = Some(ManaSource::of(&[Color::Black]));
            },
        ),
        "Castle Ardenvale" => s(
            &[
                "This land enters tapped unless you control a Plains.",
                "{T}: Add {W}.",
                "{2}{W}{W}, {T}: Create a 1/1 white Human creature token.",
            ],
            |d, t| {
                d.enters_tapped_unless = Some(["Plains", "Plains"]);
                d.mana = Some(ManaSource::of(&[Color::White]));
                activated(d, "{2}{W}{W}", true, vec![], vec![Effect::CreateToken { who: Who::You, def: t.human, n: 1 }]);
            },
        ),
        "Canopy Vista" => s(&["This land enters tapped unless you control two or more basic lands."], |d, _| {
            d.enters_tapped_unless_basics = 2;
            dual(d, Color::Green, Color::White);
        }),
        "Cinder Glade" => s(&["This land enters tapped unless you control two or more basic lands."], |d, _| {
            d.enters_tapped_unless_basics = 2;
            dual(d, Color::Red, Color::Green);
        }),
        "Seachrome Coast" => s(
            &["This land enters tapped unless you control two or fewer other lands.", "{T}: Add {W} or {U}."],
            |d, _| {
                d.enters_tapped_unless_at_most_lands = Some(2);
                dual(d, Color::White, Color::Blue);
            },
        ),
        "Circuit Mender" => s(
            &["When this creature enters, you gain 2 life.", "When this creature leaves the battlefield, draw a card."],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::GainLife { who: Who::You, n: 2 }]);
                on(d, When::LeavesBattlefield, vec![], vec![draw(1)]);
            },
        ),
        "Aether Channeler" => s(
            &[
                "When this creature enters, choose one —",
                "• Create a 1/1 white Bird creature token with flying.",
                "• Return another target nonland permanent to its owner's hand.",
                "• Draw a card.",
            ],
            |d, t| {
                let other = Filter { other: true, players: false, creatures: false, planeswalkers: false, nonland_permanents: true, ..Filter::ANY };
                on_modal(
                    d,
                    When::Enters,
                    None,
                    vec![
                        Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.bird, n: 1 }]),
                        Ability::new(vec![other], vec![Effect::Bounce(0)]),
                        Ability::new(vec![], vec![draw(1)]),
                    ],
                );
            },
        ),
        "Blade Splicer" => s(
            &[
                "When this creature enters, create a 3/3 colorless Phyrexian Golem artifact creature token.",
                "Golems you control have first strike.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.golem, n: 1 }]);
                static_on(d, Scope::OtherYourSubtype("Golem"), false, Keywords::FIRST_STRIKE);
            },
        ),
        "Chart a Course" => s(&["Draw two cards. Then discard a card unless you attacked this turn."], |d, _| {
            spell(d, vec![], vec![draw(2), Effect::DiscardUnlessAttacked])
        }),
        "Blasphemous Act" => s(
            &["This spell costs {1} less to cast for each creature on the battlefield.", "Blasphemous Act deals 13 damage to each creature."],
            |d, _| {
                d.reduction = Some(Reduction::PerCreatureOnBattlefield);
                spell(d, vec![], vec![damage(13, Recipient::EachCreature { without_flying: false })]);
            },
        ),
        // ---- Batch 3: the other two commanders, offspring ----
        "Zinnia, Valley's Voice" => s(
            &[
                "Zinnia gets +X/+0, where X is the number of other creatures you control with base power 1.",
                "Creature spells you cast gain offspring {2} as you cast them.",
            ],
            |d, _| {
                static_on(d, Scope::This, false, Keywords::NONE);
                let st = d.statics.last_mut().expect("just added");
                st.power = 1;
                st.scale = Some(Scale::PerOtherYourCreatureBasePowerOne);
                d.grants_offspring = ManaCost::parse("{2}");
            },
        ),
        "Hazel of the Rootbloom" => s(
            &[
                "{T}, Pay 2 life, Tap X untapped tokens you control: Add X mana in any combination of colors.",
                "At the beginning of your end step, create a token that's a copy of target token you control. If that token is a Squirrel, instead create two tokens that are copies of it.",
            ],
            |d, _| {
                // Played as an activated ability that uses the stack (the mana payer can't tap tokens
                // and pay life while paying a cost), with the mana going to the pool as any color.
                let n = activated(d, "", true, vec![], vec![Effect::AddManaAnyColorsX]);
                d.activated[n].life = 2;
                d.activated[n].tap_x_untapped_tokens = true;
                let token = Filter {
                    players: false,
                    creatures: false,
                    planeswalkers: false,
                    any_permanent: true,
                    tokens_only: true,
                    controller: Controller::You,
                    ..Filter::ANY
                };
                on(
                    d,
                    When::EndStep { your_turn_only: true },
                    vec![token],
                    vec![Effect::CopyTokenTwiceIfSubtype { slot: 0, sub: "Squirrel" }],
                );
            },
        ),
        "Agate Instigator" => s(
            &["Offspring {1}{R}", "Whenever another creature you control enters, this creature deals 1 damage to each opponent."],
            |d, _| {
                d.offspring = ManaCost::parse("{1}{R}");
                on(d, When::AnotherCreatureYouControlEnters, vec![], vec![damage(1, Recipient::EachOpponent)]);
            },
        ),
        // ---- Batch 4 ----
        "Clifftop Retreat" => s(&["This land enters tapped unless you control a Mountain or a Plains.", "{T}: Add {R} or {W}."], |d, _| {
            d.enters_tapped_unless = Some(["Mountain", "Plains"]);
            dual(d, Color::Red, Color::White);
        }),
        "Hinterland Harbor" => s(&["This land enters tapped unless you control a Forest or an Island.", "{T}: Add {G} or {U}."], |d, _| {
            d.enters_tapped_unless = Some(["Forest", "Island"]);
            dual(d, Color::Green, Color::Blue);
        }),
        "Copperline Gorge" => s(&["This land enters tapped unless you control two or fewer other lands.", "{T}: Add {R} or {G}."], |d, _| {
            d.enters_tapped_unless_at_most_lands = Some(2);
            dual(d, Color::Red, Color::Green);
        }),
        "Prairie Stream" => s(&["This land enters tapped unless you control two or more basic lands."], |d, _| {
            d.enters_tapped_unless_basics = 2;
            dual(d, Color::White, Color::Blue);
        }),
        "Haunted Mire" => s(&["This land enters tapped."], |d, _| {
            d.enters_tapped = true;
            dual(d, Color::Black, Color::Green);
        }),
        "Cloudblazer" => s(&["When this creature enters, you gain 2 life and draw two cards."], |d, _| {
            on(d, When::Enters, vec![], vec![Effect::GainLife { who: Who::You, n: 2 }, draw(2)])
        }),
        "Inspiring Overseer" => s(&["When this creature enters, you gain 1 life and draw a card."], |d, _| {
            on(d, When::Enters, vec![], vec![Effect::GainLife { who: Who::You, n: 1 }, draw(1)])
        }),
        "Harmonize" => s(&["Draw three cards."], |d, _| spell(d, vec![], vec![draw(3)])),
        "Generous Gift" => s(
            &["Destroy target permanent. Its controller creates a 3/3 green Elephant creature token."],
            |d, t| {
                let any = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, ..Filter::ANY };
                spell(d, vec![any], vec![Effect::Destroy(0), Effect::CreateToken { who: Who::ControllerOf(0), def: t.elephant_3, n: 1 }]);
            },
        ),
        "Putrefy" => s(&["Destroy target artifact or creature. It can't be regenerated."], |d, _| {
            // The engine has no regeneration, so nothing can regenerate it.
            let f = Filter { artifacts: true, ..CREATURE };
            spell(d, vec![f], vec![Effect::Destroy(0)]);
        }),
        "Path to Exile" => s(
            &["Exile target creature. Its controller may search their library for a basic land card, put that card onto the battlefield tapped, then shuffle."],
            |d, _| spell(d, vec![CREATURE], vec![Effect::ExileControllerMayFetchBasic(0)]),
        ),
        "Hedron Archive" => s(&["{T}: Add {C}{C}.", "{2}, {T}, Sacrifice this artifact: Draw two cards."], |d, _| {
            colorless(d);
            d.mana_amount = 2;
            let n = activated(d, "{2}", true, vec![], vec![draw(2)]);
            d.activated[n].sacrifice_self = true;
        }),
        "Kalonian Hydra" => s(
            &[
                "This creature enters with four +1/+1 counters on it.",
                "Whenever this creature attacks, double the number of +1/+1 counters on each creature you control.",
            ],
            |d, _| {
                d.enters_counters = Some((4, Condition::Always));
                on(d, When::Attacks, vec![], vec![Effect::DoubleCountersEachYours]);
            },
        ),
        "Managorger Hydra" => s(&["Whenever a player casts a spell, put a +1/+1 counter on this creature."], |d, _| {
            on(d, When::YouCast, vec![], vec![Effect::CountersSelf { n: 1 }]);
            on(d, When::OpponentCasts, vec![], vec![Effect::CountersSelf { n: 1 }]);
        }),
        "Moldervine Reclamation" => s(&["Whenever a creature you control dies, you gain 1 life and draw a card."], |d, _| {
            on(d, When::CreatureYouControlDies, vec![], vec![Effect::GainLife { who: Who::You, n: 1 }, draw(1)])
        }),
        "Prosperous Innkeeper" => s(
            &["When this creature enters, create a Treasure token.", "Whenever another creature you control enters, you gain 1 life."],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }]);
                on(d, When::AnotherCreatureYouControlEnters, vec![], vec![Effect::GainLife { who: Who::You, n: 1 }]);
            },
        ),
        "Rampaging Baloths" => s(&["Landfall — Whenever a land you control enters, create a 4/4 green Beast creature token."], |d, t| {
            on(d, When::LandYouControlEnters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.beast_4, n: 1 }])
        }),
        "Loran of the Third Path" => s(
            &["When Loran enters, destroy up to one target artifact or enchantment.", "{T}: You and target opponent each draw a card."],
            |d, _| {
                d.triggers.push(Trigger { when: When::Enters, ability: up_to(vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::Destroy(0)]) });
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                activated(d, "", true, vec![opp], vec![draw(1), Effect::Draw { who: Who::TargetPlayer(0), n: 1 }]);
            },
        ),
        _ => None,
    }
}

/// Lane B's scripted BLC cards (checked against their text in `lib.rs`).
pub const BLC_B_SCRIPTED: &[&str] = &[
    "Ms. Bumbleflower",
    "Bello, Bard of the Brambles",
    "Arcane Signet",
    "Command Tower",
    "Exotic Orchard",
    "Fellwar Stone",
    "Sol Ring",
    "Terramorphic Expanse",
    "Mind Stone",
    "Glacial Fortress",
    "Reliquary Tower",
    "Thought Vessel",
    "Temple of Enlightenment",
    "Cultivate",
    "Farseek",
    "Beast Within",
    "Big Score",
    "Bastion of Remembrance",
    "Bojuka Bog",
    "Castle Ardenvale",
    "Canopy Vista",
    "Cinder Glade",
    "Seachrome Coast",
    "Circuit Mender",
    "Aether Channeler",
    "Blade Splicer",
    "Chart a Course",
    "Blasphemous Act",
    "Zinnia, Valley's Voice",
    "Hazel of the Rootbloom",
    "Agate Instigator",
    "Clifftop Retreat",
    "Hinterland Harbor",
    "Copperline Gorge",
    "Prairie Stream",
    "Haunted Mire",
    "Cloudblazer",
    "Inspiring Overseer",
    "Harmonize",
    "Generous Gift",
    "Putrefy",
    "Path to Exile",
    "Hedron Archive",
    "Kalonian Hydra",
    "Managorger Hydra",
    "Moldervine Reclamation",
    "Prosperous Innkeeper",
    "Rampaging Baloths",
    "Loran of the Third Path",
];
