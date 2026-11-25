//! Bloomburrow Commander (BLC), lane B: the four precons' cards, most-shared first. Tests:
//! tests/blc_b.rs.
#![allow(unused_imports)]

use super::*;

/// "{T}: Add {A} or {B}." with an enters-tapped rule.
fn dual(d: &mut CardDef, a: Color, b: Color) {
    d.mana = Some(ManaSource::of(&[a, b]));
}

/// "Cycling {cost}": "{cost}, Discard this card: Draw a card." from your hand.
fn cycling(d: &mut CardDef, cost: &str) {
    let n = activated(d, cost, false, vec![], vec![draw(1)]);
    d.activated[n].from_hand_discard = true;
}

/// "This land enters tapped." / "{T}: Add {C}." / "Cycling {C}" (the cycling lands).
fn cycling_land(d: &mut CardDef, color: Color, cost: &str) {
    d.enters_tapped = true;
    d.mana = Some(ManaSource::of(&[color]));
    cycling(d, cost);
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
        // ---- Batch 5: cycling, an extra land, and more ----
        "Tranquil Thicket" => s(&["This land enters tapped.", "{T}: Add {G}.", "Cycling {G}"], |d, _| cycling_land(d, Color::Green, "{G}")),
        "Barren Moor" => s(&["This land enters tapped.", "{T}: Add {B}.", "Cycling {B}"], |d, _| cycling_land(d, Color::Black, "{B}")),
        "Forgotten Cave" => s(&["This land enters tapped.", "{T}: Add {R}.", "Cycling {R}"], |d, _| cycling_land(d, Color::Red, "{R}")),
        "Explore" => s(&["You may play an additional land this turn.", "Draw a card."], |d, _| {
            spell(d, vec![], vec![Effect::ExtraLandThisTurn, draw(1)])
        }),
        "Coiling Oracle" => s(
            &["When this creature enters, reveal the top card of your library. If it's a land card, put it onto the battlefield. Otherwise, put that card into your hand."],
            |d, _| on(d, When::Enters, vec![], vec![Effect::RevealTopLandToBattlefieldElseHand]),
        ),
        "Maelstrom Pulse" => s(
            &["Destroy target nonland permanent and all other permanents with the same name as that permanent."],
            |d, _| {
                let nonland = Filter { controller: Controller::Any, ..OPP_NONLAND };
                spell(d, vec![nonland], vec![Effect::DestroyWithSameName(0)]);
            },
        ),
        "Plumecreed Escort" => s(&["When this creature enters, target creature you control gains hexproof until end of turn."], |d, _| {
            on(d, When::Enters, vec![YOUR_CREATURE], vec![Effect::Grant { slot: 0, keywords: Keywords::HEXPROOF }])
        }),
        "Haywire Mite" => s(
            &[
                "When this creature dies, you gain 2 life.",
                "{G}, Sacrifice this creature: Exile target noncreature artifact or noncreature enchantment.",
            ],
            |d, _| {
                on(d, When::Dies, vec![], vec![Effect::GainLife { who: Who::You, n: 2 }]);
                let f = Filter { noncreature: true, ..ARTIFACT_OR_ENCHANTMENT };
                let n = activated(d, "{G}", false, vec![f], vec![Effect::Exile(0)]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Ornithopter of Paradise" => s(&["{T}: Add one mana of any color."], |d, _| {
            d.mana = Some(ManaSource::of(&Color::ALL));
        }),
        "Cut a Deal" => s(
            &["Each opponent draws a card, then you draw a card for each opponent who drew a card this way."],
            |d, _| spell(d, vec![], vec![Effect::Draw { who: Who::EachOpponent, n: 1 }, draw(1)]),
        ),
        "Pull from Tomorrow" => s(&["Draw X cards, then discard a card."], |d, _| {
            spell(d, vec![], vec![Effect::DrawX, discard(1)])
        }),
        "Hanged Executioner" => s(
            &[
                "When this creature enters, create a 1/1 white Spirit creature token with flying.",
                "{3}{W}, Exile this creature: Exile target creature.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.spirit, n: 1 }]);
                let n = activated(d, "{3}{W}", false, vec![CREATURE], vec![Effect::Exile(0)]);
                d.activated[n].exile_self = true;
            },
        ),
        "Inferno Titan" => s(
            &[
                "{R}: This creature gets +1/+0 until end of turn.",
                "Whenever this creature enters or attacks, it deals 3 damage divided as you choose among one, two, or three targets.",
            ],
            |d, _| {
                activated(d, "{R}", false, vec![], vec![Effect::PumpSelf { power: 1, toughness: 0 }]);
                for when in [When::Enters, When::Attacks] {
                    let mut a = up_to(vec![Filter::ANY; 3], vec![Effect::DividedDamage]);
                    a.min_targets = Some(1);
                    a.divide = 3;
                    d.triggers.push(Trigger { when, ability: a });
                }
            },
        ),
        "Lotus Cobra" => s(&["Landfall — Whenever a land you control enters, add one mana of any color."], |d, _| {
            on(d, When::LandYouControlEnters, vec![], vec![Effect::AddManaAnyColor])
        }),
        // ---- Batch 6: lands, ramp and staples ----
        "Rampant Growth" => s(
            &["Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle."],
            |d, _| spell(d, vec![], vec![search(SearchFor::BasicLand, SearchTo::Battlefield { tapped: true })]),
        ),
        "Razorverge Thicket" => s(&["This land enters tapped unless you control two or fewer other lands.", "{T}: Add {G} or {W}."], |d, _| {
            d.enters_tapped_unless_at_most_lands = Some(2);
            dual(d, Color::Green, Color::White);
        }),
        "Rootbound Crag" => s(&["This land enters tapped unless you control a Mountain or a Forest.", "{T}: Add {R} or {G}."], |d, _| {
            d.enters_tapped_unless = Some(["Mountain", "Forest"]);
            dual(d, Color::Red, Color::Green);
        }),
        "Sulfur Falls" => s(&["This land enters tapped unless you control an Island or a Mountain.", "{T}: Add {U} or {R}."], |d, _| {
            d.enters_tapped_unless = Some(["Island", "Mountain"]);
            dual(d, Color::Blue, Color::Red);
        }),
        "Sunpetal Grove" => s(&["This land enters tapped unless you control a Forest or a Plains.", "{T}: Add {G} or {W}."], |d, _| {
            d.enters_tapped_unless = Some(["Forest", "Plains"]);
            dual(d, Color::Green, Color::White);
        }),
        "Woodland Cemetery" => s(&["This land enters tapped unless you control a Swamp or a Forest.", "{T}: Add {B} or {G}."], |d, _| {
            d.enters_tapped_unless = Some(["Swamp", "Forest"]);
            dual(d, Color::Black, Color::Green);
        }),
        "Temple of Abandon" => s(&["This land enters tapped.", "When this land enters, scry 1.", "{T}: Add {R} or {G}."], |d, _| {
            d.enters_tapped = true;
            on(d, When::Enters, vec![], vec![Effect::Scry { n: 1 }]);
            dual(d, Color::Red, Color::Green);
        }),
        "Temple of Epiphany" => s(&["This land enters tapped.", "When this land enters, scry 1.", "{T}: Add {U} or {R}."], |d, _| {
            d.enters_tapped = true;
            on(d, When::Enters, vec![], vec![Effect::Scry { n: 1 }]);
            dual(d, Color::Blue, Color::Red);
        }),
        "Temple of Malady" => s(&["This land enters tapped.", "When this land enters, scry 1.", "{T}: Add {B} or {G}."], |d, _| {
            d.enters_tapped = true;
            on(d, When::Enters, vec![], vec![Effect::Scry { n: 1 }]);
            dual(d, Color::Black, Color::Green);
        }),
        "Temple of Mystery" => s(&["This land enters tapped.", "When this land enters, scry 1.", "{T}: Add {G} or {U}."], |d, _| {
            d.enters_tapped = true;
            on(d, When::Enters, vec![], vec![Effect::Scry { n: 1 }]);
            dual(d, Color::Green, Color::Blue);
        }),
        "Temple of Plenty" => s(&["This land enters tapped.", "When this land enters, scry 1.", "{T}: Add {G} or {W}."], |d, _| {
            d.enters_tapped = true;
            on(d, When::Enters, vec![], vec![Effect::Scry { n: 1 }]);
            dual(d, Color::Green, Color::White);
        }),
        "Temple of Triumph" => s(&["This land enters tapped.", "When this land enters, scry 1.", "{T}: Add {R} or {W}."], |d, _| {
            d.enters_tapped = true;
            on(d, When::Enters, vec![], vec![Effect::Scry { n: 1 }]);
            dual(d, Color::Red, Color::White);
        }),
        "Seaside Citadel" => s(&["This land enters tapped.", "{T}: Add {G}, {W}, or {U}."], |d, _| {
            d.enters_tapped = true;
            d.mana = Some(ManaSource::of(&[Color::Green, Color::White, Color::Blue]));
        }),
        "Wooded Ridgeline" => s(&["This land enters tapped."], |d, _| {
            d.enters_tapped = true;
            dual(d, Color::Red, Color::Green);
        }),
        "Sheltered Thicket" => s(&["This land enters tapped.", "Cycling {2}"], |d, _| {
            d.enters_tapped = true;
            dual(d, Color::Red, Color::Green);
            cycling(d, "{2}");
        }),
        "Thran Dynamo" => s(&["{T}: Add {C}{C}{C}."], |d, _| {
            colorless(d);
            d.mana_amount = 3;
        }),
        "Sakura-Tribe Elder" => s(
            &["Sacrifice this creature: Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle."],
            |d, _| {
                let n = activated(d, "", false, vec![], vec![search(SearchFor::BasicLand, SearchTo::Battlefield { tapped: true })]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Secret Rendezvous" => s(&["You and target opponent each draw three cards."], |d, _| {
            let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
            spell(d, vec![opp], vec![draw(3), Effect::Draw { who: Who::TargetPlayer(0), n: 3 }]);
        }),
        "Spirited Companion" => s(&["When this creature enters, draw a card."], |d, _| on(d, When::Enters, vec![], vec![draw(1)])),
        "Sphinx of Enlightenment" => s(&["When this creature enters, target opponent draws a card and you draw three cards."], |d, _| {
            let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
            on(d, When::Enters, vec![opp], vec![Effect::Draw { who: Who::TargetPlayer(0), n: 1 }, draw(3)]);
        }),
        "Swords to Plowshares" => s(&["Exile target creature. Its controller gains life equal to its power."], |d, _| {
            spell(d, vec![CREATURE], vec![Effect::ExileControllerGainsPower(0)])
        }),
        "Selfless Spirit" => s(&["Sacrifice this creature: Creatures you control gain indestructible until end of turn."], |d, _| {
            let n = activated(d, "", false, vec![], vec![Effect::GrantAll { who: Who::You, keywords: Keywords::INDESTRUCTIBLE }]);
            d.activated[n].sacrifice_self = true;
        }),
        "Squirrel Sovereign" => s(&["Other Squirrels you control get +1/+1."], |d, _| lord(d, "Squirrel")),
        "Zulaport Cutthroat" => s(
            &["Whenever this creature or another creature you control dies, each opponent loses 1 life and you gain 1 life."],
            |d, _| {
                on(
                    d,
                    When::CreatureYouControlDies,
                    vec![],
                    vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }, Effect::GainLife { who: Who::You, n: 1 }],
                )
            },
        ),
        "Sun Titan" => s(
            &["Whenever this creature enters or attacks, you may return target permanent card with mana value 3 or less from your graveyard to the battlefield."],
            |d, _| {
                let card = Filter { permanent_cards: true, max_mana_value: Some(3), ..graveyard_cards(false, Controller::You) };
                may(d, When::Enters, vec![card], vec![Effect::Reanimate(0)]);
                may(d, When::Attacks, vec![card], vec![Effect::Reanimate(0)]);
            },
        ),
        "Sunscorch Regent" => s(&["Whenever an opponent casts a spell, put a +1/+1 counter on this creature and you gain 1 life."], |d, _| {
            on(d, When::OpponentCasts, vec![], vec![Effect::CountersSelf { n: 1 }, Effect::GainLife { who: Who::You, n: 1 }])
        }),
        "Rapid Hybridization" => s(
            &["Destroy target creature. It can't be regenerated. That creature's controller creates a 3/3 green Frog Lizard creature token."],
            |d, t| {
                // The engine has no regeneration.
                spell(d, vec![CREATURE], vec![Effect::Destroy(0), Effect::CreateToken { who: Who::ControllerOf(0), def: t.frog_lizard, n: 1 }]);
            },
        ),
        // ---- Batch 7 ----
        "Thriving Heath" => s(
            &["This land enters tapped. As it enters, choose a color other than white.", "{T}: Add {W} or one mana of the chosen color."],
            |d, _| {
                d.enters_tapped = true;
                d.choose_color = true;
                d.thriving_color = Some(Color::White);
            },
        ),
        "Thriving Isle" => s(
            &["This land enters tapped. As it enters, choose a color other than blue.", "{T}: Add {U} or one mana of the chosen color."],
            |d, _| {
                d.enters_tapped = true;
                d.choose_color = true;
                d.thriving_color = Some(Color::Blue);
            },
        ),
        "Angel of the Ruins" => s(
            &["When this creature enters, exile up to two target artifacts and/or enchantments.", "Plainscycling {2}"],
            |d, _| {
                d.triggers.push(Trigger {
                    when: When::Enters,
                    ability: up_to(vec![ARTIFACT_OR_ENCHANTMENT; 2], vec![Effect::Exile(0), Effect::Exile(1)]),
                });
                let plains = SearchFor::LandTypes { types: land_types(&["Plains"]), basic: false };
                let n = activated(d, "{2}", false, vec![], vec![search(plains, SearchTo::Hand)]);
                d.activated[n].from_hand_discard = true;
            },
        ),
        "Berserkers' Onslaught" => s(&["Attacking creatures you control have double strike."], |d, _| {
            static_on(d, Scope::YourAttackingCreatures, false, Keywords::DOUBLE_STRIKE)
        }),
        "Binding the Old Gods" => s(
            &[
                "I — Destroy target nonland permanent an opponent controls.",
                "II — Search your library for a Forest card, put it onto the battlefield tapped, then shuffle.",
                "III — Creatures you control gain deathtouch until end of turn.",
            ],
            |d, _| {
                chapter(d, 1, Ability::new(vec![OPP_NONLAND], vec![Effect::Destroy(0)]));
                let forest = SearchFor::LandTypes { types: land_types(&["Forest"]), basic: false };
                chapter(d, 2, Ability::new(vec![], vec![search(forest, SearchTo::Battlefield { tapped: true })]));
                chapter(d, 3, Ability::new(vec![], vec![Effect::GrantAll { who: Who::You, keywords: Keywords::DEATHTOUCH }]));
            },
        ),
        "Decimate" => s(&["Destroy target artifact, target creature, target enchantment, and target land."], |d, _| {
            let enchantment = Filter { players: false, creatures: false, planeswalkers: false, enchantments: true, ..Filter::ANY };
            let land = Filter { players: false, creatures: false, planeswalkers: false, lands: true, ..Filter::ANY };
            spell(
                d,
                vec![ARTIFACT, CREATURE, enchantment, land],
                vec![Effect::Destroy(0), Effect::Destroy(1), Effect::Destroy(2), Effect::Destroy(3)],
            );
        }),
        "Gratuitous Violence" => s(
            &["If a creature you control would deal damage to a permanent or player, it deals double that damage instead."],
            |d, _| d.doubles_your_creatures_damage = true,
        ),
        "Jazal Goldmane" => s(
            &["{3}{W}{W}: Attacking creatures you control get +X/+X until end of turn, where X is the number of attacking creatures."],
            |d, _| {
                activated(d, "{3}{W}{W}", false, vec![], vec![Effect::PumpAttackersPerAttacker]);
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
    "Tranquil Thicket",
    "Barren Moor",
    "Forgotten Cave",
    "Explore",
    "Coiling Oracle",
    "Maelstrom Pulse",
    "Plumecreed Escort",
    "Haywire Mite",
    "Ornithopter of Paradise",
    "Cut a Deal",
    "Pull from Tomorrow",
    "Hanged Executioner",
    "Inferno Titan",
    "Lotus Cobra",
    "Rampant Growth",
    "Razorverge Thicket",
    "Rootbound Crag",
    "Sulfur Falls",
    "Sunpetal Grove",
    "Woodland Cemetery",
    "Temple of Abandon",
    "Temple of Epiphany",
    "Temple of Malady",
    "Temple of Mystery",
    "Temple of Plenty",
    "Temple of Triumph",
    "Seaside Citadel",
    "Wooded Ridgeline",
    "Sheltered Thicket",
    "Thran Dynamo",
    "Sakura-Tribe Elder",
    "Secret Rendezvous",
    "Spirited Companion",
    "Sphinx of Enlightenment",
    "Swords to Plowshares",
    "Selfless Spirit",
    "Squirrel Sovereign",
    "Zulaport Cutthroat",
    "Sun Titan",
    "Sunscorch Regent",
    "Rapid Hybridization",
    "Thriving Heath",
    "Thriving Isle",
    "Angel of the Ruins",
    "Berserkers' Onslaught",
    "Binding the Old Gods",
    "Decimate",
    "Gratuitous Violence",
    "Jazal Goldmane",
];
