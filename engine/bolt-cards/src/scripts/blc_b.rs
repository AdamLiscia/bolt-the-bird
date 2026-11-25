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

/// A karoo: enters tapped, returns a land you control to its owner's hand, and taps for two mana.
fn karoo(d: &mut CardDef, a: Color, b: Color) {
    d.enters_tapped = true;
    on(d, When::Enters, vec![], vec![Effect::ReturnLandYouControl]);
    d.paid_mana = Some((ManaCost::default(), [ManaSource::of(&[a]), ManaSource::of(&[b])]));
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
        // ---- Batch 8 (2026-11-25 list) ----
        "Thriving Grove" => s(
            &["This land enters tapped. As it enters, choose a color other than green.", "{T}: Add {G} or one mana of the chosen color."],
            |d, _| {
                d.enters_tapped = true;
                d.choose_color = true;
                d.thriving_color = Some(Color::Green);
            },
        ),
        "Game Trail" => s(
            &[
                "As this land enters, you may reveal a Mountain or Forest card from your hand. If you don't, this land enters tapped.",
                "{T}: Add {R} or {G}.",
            ],
            |d, _| {
                d.enters_tapped_unless_reveal = Some(["Mountain", "Forest"]);
                dual(d, Color::Red, Color::Green);
            },
        ),
        "Talisman of Resilience" => s(&["{T}: Add {C}.", "{T}: Add {B} or {G}. This artifact deals 1 damage to you."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::of(&[Color::Black, Color::Green]) });
            d.pain_colored = true;
        }),
        "Unnatural Growth" => s(
            &["At the beginning of each combat, double the power and toughness of each creature you control until end of turn."],
            |d, _| on(d, When::BeginCombatEachTurn, vec![], vec![Effect::DoublePowerToughnessAll]),
        ),
        "Siege-Gang Commander" => s(
            &[
                "When this creature enters, create three 1/1 red Goblin creature tokens.",
                "{1}{R}, Sacrifice a Goblin: This creature deals 2 damage to any target.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.goblin, n: 3 }]);
                let n = activated(d, "{1}{R}", false, vec![Filter::ANY], vec![damage(2, Recipient::Target(0))]);
                d.activated[n].sacrifice_subtype = Some("Goblin");
            },
        ),
        "Thopter Engineer" => s(
            &[
                "When this creature enters, create a 1/1 colorless Thopter artifact creature token with flying.",
                "Artifact creatures you control have haste.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.thopter, n: 1 }]);
                static_on(d, Scope::YourArtifactCreatures, false, Keywords::HASTE);
            },
        ),
        "Devilish Valet" => s(
            &["Alliance — Whenever another creature you control enters, double this creature's power until end of turn."],
            |d, _| on(d, When::AnotherCreatureYouControlEnters, vec![], vec![Effect::DoublePowerSelf]),
        ),
        "Steelburr Champion" => s(
            &["Offspring {1}{W}", "Whenever an opponent casts a noncreature spell, put a +1/+1 counter on this creature."],
            |d, _| {
                d.offspring = ManaCost::parse("{1}{W}");
                on(d, When::OpponentCastsNoncreature, vec![], vec![Effect::CountersSelf { n: 1 }]);
            },
        ),
        "Prosperous Bandit" => s(
            &["Offspring {1}", "Whenever this creature deals combat damage to a player, create that many tapped Treasure tokens."],
            |d, t| {
                d.offspring = ManaCost::parse("{1}");
                on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::CreateTokensAmountTapped { def: t.treasure }]);
            },
        ),
        "Curiosity Crafter" => s(
            &["You have no maximum hand size.", "Whenever a creature token you control deals combat damage to a player, draw a card."],
            |d, _| {
                d.no_max_hand_size = true;
                on(d, When::YourCreatureTokenDealsCombatDamageToPlayer, vec![], vec![draw(1)]);
            },
        ),
        "Triskaidekaphile" => s(
            &[
                "You have no maximum hand size.",
                "At the beginning of your upkeep, if you have exactly thirteen cards in your hand, you win the game.",
                "{3}{U}: Draw a card.",
            ],
            |d, _| {
                d.no_max_hand_size = true;
                conditional(d, When::Upkeep { your_turn_only: true }, Condition::HandSizeExactly(13), vec![], vec![Effect::WinTheGame]);
                activated(d, "{3}{U}", false, vec![], vec![draw(1)]);
            },
        ),
        "Murmuration" => s(
            &[
                "Birds you control get +1/+1 and have vigilance.",
                "At the beginning of your end step, for each spell you've cast this turn, create a 1/2 blue Bird creature token with flying named Storm Crow.",
            ],
            |d, t| {
                lord(d, "Bird");
                d.statics.last_mut().expect("just added").keywords = Keywords::VIGILANCE;
                on(d, When::EndStep { your_turn_only: true }, vec![], vec![Effect::CreateTokensPerSpellCastThisTurn { def: t.storm_crow }]);
            },
        ),
        "Golgari Rot Farm" => s(
            &["This land enters tapped.", "When this land enters, return a land you control to its owner's hand.", "{T}: Add {B}{G}."],
            |d, _| karoo(d, Color::Black, Color::Green),
        ),
        "Gruul Turf" => s(
            &["This land enters tapped.", "When this land enters, return a land you control to its owner's hand.", "{T}: Add {R}{G}."],
            |d, _| karoo(d, Color::Red, Color::Green),
        ),
        "Loyal Warhound" => s(
            &["When this creature enters, if an opponent controls more lands than you, search your library for a basic Plains card, put it onto the battlefield tapped, then shuffle."],
            |d, _| {
                let plains = SearchFor::LandTypes { types: land_types(&["Plains"]), basic: true };
                conditional(
                    d,
                    When::Enters,
                    Condition::OpponentControlsMoreLands,
                    vec![],
                    vec![search(plains, SearchTo::Battlefield { tapped: true })],
                );
            },
        ),
        "Warstorm Surge" => s(&["Whenever a creature you control enters, it deals damage equal to its power to any target."], |d, _| {
            let that = Filter { triggering: true, ..CREATURE };
            on(d, When::AnotherCreatureYouControlEnters, vec![that, Filter::ANY], vec![Effect::DamageEqualSlotPower { from: 0, to: 1 }]);
        }),
        "Rites of Flourishing" => s(
            &[
                "At the beginning of each player's draw step, that player draws an additional card.",
                "Each player may play an additional land on each of their turns.",
            ],
            |d, _| {
                on(d, When::EachDrawStep, vec![], vec![Effect::ActivePlayerDraws(1)]);
                d.extra_land_drop_each_player = true;
            },
        ),
        "Garruk, Cursed Huntsman" => s(
            &[
                "0: Create two 2/2 black and green Wolf creature tokens with \"When this token dies, put a loyalty counter on each Garruk you control.\"",
                "\u{2212}3: Destroy target creature. Draw a card.",
                "\u{2212}6: You get an emblem with \"Creatures you control get +3/+3 and have trample.\"",
            ],
            |d, t| {
                loyalty(d, 0, Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.garruk_wolf, n: 2 }]));
                loyalty(d, -3, Ability::new(vec![CREATURE], vec![Effect::Destroy(0), draw(1)]));
                loyalty(d, -6, Ability::new(vec![], vec![Effect::Emblem(EmblemKind::GarrukAnthem)]));
            },
        ),
        "Plaguecrafter" => s(
            &["When this creature enters, each player sacrifices a creature or planeswalker of their choice. Each player who can't discards a card."],
            |d, _| {
                // Each player in turn order (you, then your opponent).
                on(
                    d,
                    When::Enters,
                    vec![],
                    vec![
                        Effect::SacrificeCreatureOrPlaneswalkerElseDiscard { who: Who::You },
                        Effect::SacrificeCreatureOrPlaneswalkerElseDiscard { who: Who::EachOpponent },
                    ],
                );
            },
        ),
        "Chaos Warp" => s(
            &["The owner of target permanent shuffles it into their library, then reveals the top card of their library. If it's a permanent card, they put it onto the battlefield."],
            |d, _| {
                let any = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, ..Filter::ANY };
                spell(d, vec![any], vec![Effect::ChaosWarp(0)]);
            },
        ),
        "Decree of Pain" => s(
            &[
                "Destroy all creatures. They can't be regenerated. Draw a card for each creature destroyed this way.",
                "Cycling {3}{B}{B}",
                "When you cycle this card, all creatures get -2/-2 until end of turn.",
            ],
            |d, _| {
                // The engine has no regeneration.
                spell(d, vec![], vec![Effect::DestroyAllCreaturesDrawPerDestroyed]);
                cycling(d, "{3}{B}{B}");
                on(
                    d,
                    When::YouCycleThis,
                    vec![],
                    vec![
                        Effect::PumpAll { who: Who::You, power: -2, toughness: -2 },
                        Effect::PumpAll { who: Who::EachOpponent, power: -2, toughness: -2 },
                    ],
                );
            },
        ),
        "Spore Frog" => s(&["Sacrifice this creature: Prevent all combat damage that would be dealt this turn."], |d, _| {
            let n = activated(d, "", false, vec![], vec![Effect::PreventAllCombatDamageThisTurn]);
            d.activated[n].sacrifice_self = true;
        }),
        "Riot Control" => s(
            &["You gain 1 life for each creature your opponents control. Prevent all damage that would be dealt to you this turn."],
            |d, _| spell(d, vec![], vec![Effect::GainLifePerOpponentCreature, Effect::PreventAllDamageToYouThisTurn]),
        ),
        "Scurry of Squirrels" => s(
            &["Myriad, myriad", "Whenever this creature deals combat damage to a player, put a +1/+1 counter on target creature you control."],
            |d, _| {
                // Myriad copies attack each opponent other than the defending player: in a two-player
                // game there is none, so myriad (twice) does nothing.
                on(d, When::DealsCombatDamageToPlayer, vec![YOUR_CREATURE], vec![Effect::Counters { slot: 0, n: 1 }]);
            },
        ),
        "Faeburrow Elder" => s(
            &[
                "This creature gets +1/+1 for each color among permanents you control.",
                "{T}: For each color among permanents you control, add one mana of that color.",
            ],
            |d, _| {
                static_on(d, Scope::This, false, Keywords::NONE);
                let st = d.statics.last_mut().expect("just added");
                st.power = 1;
                st.toughness = 1;
                st.scale = Some(Scale::PerColorAmongYourPermanents);
                d.vivid_mana = true;
            },
        ),
        "Beledros Witherbloom" => s(
            &[
                "At the beginning of each upkeep, create a 1/1 black and green Pest creature token with \"When this token dies, you gain 1 life.\"",
                "Pay 10 life: Untap all lands you control. Activate only once each turn.",
            ],
            |d, t| {
                on(d, When::Upkeep { your_turn_only: false }, vec![], vec![Effect::CreateToken { who: Who::You, def: t.pest, n: 1 }]);
                let n = activated(d, "", false, vec![], vec![Effect::UntapLandsYouControl]);
                d.activated[n].life = 10;
                d.activated[n].once_each_turn = true;
            },
        ),
        "Gilded Goose" => s(
            &[
                "When this creature enters, create a Food token.",
                "{1}{G}, {T}: Create a Food token.",
                "{T}, Sacrifice a Food: Add one mana of any color.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.food, n: 1 }]);
                activated(d, "{1}{G}", true, vec![], vec![Effect::CreateToken { who: Who::You, def: t.food, n: 1 }]);
                // Played as an activated ability that uses the stack (as Hazel's is): the mana payer
                // doesn't sacrifice other permanents.
                let n = activated(d, "", true, vec![], vec![Effect::AddManaAnyColor]);
                d.activated[n].sacrifice_subtype = Some("Food");
            },
        ),
        "Jacked Rabbit" => s(
            &["Ravenous", "Whenever this creature attacks, create a number of 1/1 white Rabbit creature tokens equal to this creature's power."],
            |d, t| {
                d.enters_with_x_counters = true;
                conditional(d, When::Enters, Condition::XAtLeast(5), vec![], vec![draw(1)]);
                on(d, When::Attacks, vec![], vec![Effect::CreateTokensPower { def: t.rabbit }]);
            },
        ),
        "Selvala, Explorer Returned" => s(
            &["Parley — {T}: Each player reveals the top card of their library. For each nonland card revealed this way, add {G} and you gain 1 life. Then each player draws a card."],
            |d, _| {
                // A mana ability played on the stack, like Hazel's.
                activated(d, "", true, vec![], vec![Effect::SelvalaParley]);
            },
        ),
        "Swarmyard Massacre" => s(
            &["Create two 1/1 green Squirrel creature tokens. Then each creature that isn't an Insect, Rat, Spider, or Squirrel gets -1/-1 until end of turn for each creature you control that's an Insect, Rat, Spider, or Squirrel."],
            |d, t| {
                spell(
                    d,
                    vec![],
                    vec![
                        Effect::CreateToken { who: Who::You, def: t.squirrel, n: 2 },
                        Effect::ShrinkAllWithoutSubtypesPerYours(&["Insect", "Rat", "Spider", "Squirrel"]),
                    ],
                );
            },
        ),
        "Skullclamp" => s(&["Equipped creature gets +1/-1.", "Whenever equipped creature dies, draw two cards.", "Equip {1}"], |d, _| {
            attached(d, false, 1, -1, Keywords::NONE);
            on(d, When::EquippedDies, vec![], vec![draw(2)]);
            equip(d, "{1}");
        }),
        "Outpost Siege" => s(
            &[
                "As this enchantment enters, choose Khans or Dragons.",
                "• Khans — At the beginning of your upkeep, exile the top card of your library. Until end of turn, you may play that card.",
                "• Dragons — Whenever a creature you control leaves the battlefield, this enchantment deals 1 damage to any target.",
            ],
            |d, _| {
                d.enter_modes = true;
                conditional(
                    d,
                    When::Upkeep { your_turn_only: true },
                    Condition::ChosenMode(0),
                    vec![],
                    vec![Effect::ExileTopChooseOneToPlay { n: 1, this_turn: true }],
                );
                conditional(
                    d,
                    When::CreatureYouControlLeavesBattlefield,
                    Condition::ChosenMode(1),
                    vec![Filter::ANY],
                    vec![damage(1, Recipient::Target(0))],
                );
            },
        ),
        "Tainted Wood" => s(&["{T}: Add {C}.", "{T}: Add {B} or {G}. Activate only if you control a Swamp."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::of(&[Color::Black, Color::Green]) });
            d.colored_mana_only_if_you_control = Some("Swamp");
        }),
        "Llanowar Loamspeaker" => s(
            &[
                "{T}: Add one mana of any color.",
                "{T}: Target land you control becomes a 3/3 Elemental creature with haste until end of turn. It's still a land. Activate only as a sorcery.",
            ],
            |d, _| {
                d.mana = Some(ManaSource::of(&Color::ALL));
                let land = Filter { players: false, creatures: false, planeswalkers: false, lands: true, controller: Controller::You, ..Filter::ANY };
                let n = activated(
                    d,
                    "",
                    true,
                    vec![land],
                    vec![Effect::SlotBecomesCreatureUntilEot { slot: 0, power: 3, toughness: 3, subtype: "Elemental", keywords: Keywords::HASTE }],
                );
                d.activated[n].sorcery = true;
            },
        ),
        "Peerless Recycling" => s(
            &[
                "Gift a card",
                "Return target permanent card from your graveyard to your hand. If the gift was promised, instead return two target permanent cards from your graveyard to your hand.",
            ],
            |d, _| {
                // Gift: an optional free additional cost, cast with `Action::CastKicked`.
                d.kicker = ManaCost::parse("{0}");
                let card = Filter { permanent_cards: true, ..graveyard_cards(false, Controller::You) };
                spell(d, vec![card], vec![Effect::Bounce(0)]);
                d.kicked_spell = Some(Ability::new(
                    vec![card, card],
                    vec![Effect::Draw { who: Who::EachOpponent, n: 1 }, Effect::Bounce(0), Effect::Bounce(1)],
                ));
            },
        ),
        "Octomancer" => s(
            &[
                "Gift an Octopus",
                "At the beginning of each end step, create a token that's a copy of target creature token that entered the battlefield this turn.",
            ],
            |d, t| {
                d.kicker = ManaCost::parse("{0}"); // the gift, cast with `Action::CastKicked`
                conditional(
                    d,
                    When::Enters,
                    Condition::Kicked,
                    vec![],
                    vec![Effect::CreateToken { who: Who::EachOpponent, def: t.octopus, n: 1 }],
                );
                let new_token = Filter { tokens_only: true, entered_this_turn: true, ..CREATURE };
                on(
                    d,
                    When::EndStep { your_turn_only: false },
                    vec![new_token],
                    vec![Effect::CopyToken { slot: Some(0), haste_and_sacrifice: false, extra_subtype: None }],
                );
            },
        ),
        "Rowdy Research" => s(
            &["This spell costs {1} less to cast for each creature that attacked this turn.", "Draw three cards."],
            |d, _| {
                d.reduction = Some(Reduction::PerCreatureAttackedThisTurn);
                spell(d, vec![], vec![draw(3)]);
            },
        ),
        "Jolrael, Mwonvuli Recluse" => s(
            &[
                "Whenever you draw your second card each turn, create a 2/2 green Cat creature token.",
                "{4}{G}{G}: Until end of turn, creatures you control have base power and toughness X/X, where X is the number of cards in your hand.",
            ],
            |d, t| {
                on(d, When::YouDrawSecond, vec![], vec![Effect::CreateToken { who: Who::You, def: t.cat_2, n: 1 }]);
                activated(d, "{4}{G}{G}", false, vec![], vec![Effect::YourCreaturesBasePtHandSize]);
            },
        ),
        "Skyfisher Spider" => s(
            &[
                "When this creature enters, you may sacrifice another creature. When you do, destroy target nonland permanent.",
                "When this creature dies, you may gain 1 life for each creature card in your graveyard. If you do, exile this card from your graveyard.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::MaySacrificeAnother, Effect::Reflexive(1)]);
                let nonland = Filter { controller: Controller::Any, ..OPP_NONLAND };
                on(d, When::Never, vec![nonland], vec![Effect::Destroy(0)]); // trigger 1: "when you do"
                may(d, When::Dies, vec![], vec![Effect::GainLifePerCreatureCardInGraveyard, Effect::ExileSourceFromGraveyard]);
            },
        ),
        "Pollywog Prodigy" => s(
            &[
                "Evolve",
                "Whenever an opponent casts a noncreature spell with mana value less than this creature's power, draw a card.",
            ],
            |d, _| {
                let that = Filter { triggering: true, ..CREATURE };
                on(d, When::AnotherCreatureYouControlEnters, vec![that], vec![Effect::Evolve(0)]);
                on(d, When::OpponentCastsNoncreatureMvBelowPower, vec![], vec![draw(1)]);
            },
        ),
        "Mangara, the Diplomat" => s(
            &[
                "Whenever an opponent attacks with creatures, if two or more of those creatures are attacking you and/or planeswalkers you control, draw a card.",
                "Whenever an opponent casts their second spell each turn, draw a card.",
            ],
            |d, _| {
                on(d, When::OpponentAttacksWithAtLeastTwo, vec![], vec![draw(1)]);
                on(d, When::OpponentCastsSecondSpell, vec![], vec![draw(1)]);
            },
        ),
        "Hoofprints of the Stag" => s(
            &[
                "Whenever you draw a card, you may put a hoofprint counter on this enchantment.",
                "{2}{W}, Remove four hoofprint counters from this enchantment: Create a 4/4 white Elemental creature token with flying. Activate only during your turn.",
            ],
            |d, t| {
                may(d, When::YouDraw, vec![], vec![Effect::KindCounterSelf(1)]);
                let n = activated(d, "{2}{W}", false, vec![], vec![Effect::CreateToken { who: Who::You, def: t.elemental_4, n: 1 }]);
                d.activated[n].remove_kind_counters = 4;
                d.activated[n].your_turn_only = true;
            },
        ),
        "Chitterspitter" => s(
            &[
                "At the beginning of your upkeep, you may sacrifice a token. If you do, put an acorn counter on this artifact.",
                "Squirrels you control get +1/+1 for each acorn counter on this artifact.",
                "{G}, {T}: Create a 1/1 green Squirrel creature token.",
            ],
            |d, t| {
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::MaySacrificeToken, Effect::KindCounterSelf(1)]);
                lord(d, "Squirrel");
                d.statics.last_mut().expect("just added").scale = Some(Scale::PerKindCounter);
                activated(d, "{G}", true, vec![], vec![Effect::CreateToken { who: Who::You, def: t.squirrel, n: 1 }]);
            },
        ),
        "Teapot Slinger" => s(&["Whenever you expend 4, this creature deals 2 damage to each opponent."], |d, _| {
            on(d, When::YouExpend(4), vec![], vec![damage(2, Recipient::EachOpponent)])
        }),
        "Helm of the Host" => s(
            &[
                "At the beginning of combat on your turn, create a token that's a copy of equipped creature, except the token isn't legendary. That token gains haste.",
                "Equip {5}",
            ],
            |d, _| {
                on(d, When::BeginCombatYourTurn, vec![], vec![Effect::CopyAttachedNonlegendaryHaste]);
                equip(d, "{5}");
            },
        ),
        "Tempt with Discovery" => s(
            &["Tempting offer — Search your library for a land card and put it onto the battlefield. Each opponent may search their library for a land card and put it onto the battlefield. For each opponent who searches a library this way, search your library for a land card and put it onto the battlefield. Then each player who searched a library this way shuffles."],
            |d, _| {
                spell(
                    d,
                    vec![],
                    vec![
                        search(SearchFor::Land, SearchTo::Battlefield { tapped: false }),
                        Effect::TemptOpponentsLand,
                        Effect::SearchLandIfTempted,
                    ],
                );
            },
        ),
        "Wildsear, Scouring Maw" => s(
            &["Enchantment spells you cast from your hand have cascade."],
            |d, _| on(d, When::YouCastEnchantmentFromHand, vec![], vec![Effect::Cascade]),
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
    "Thriving Grove",
    "Game Trail",
    "Talisman of Resilience",
    "Unnatural Growth",
    "Siege-Gang Commander",
    "Thopter Engineer",
    "Devilish Valet",
    "Steelburr Champion",
    "Prosperous Bandit",
    "Curiosity Crafter",
    "Triskaidekaphile",
    "Murmuration",
    "Golgari Rot Farm",
    "Gruul Turf",
    "Loyal Warhound",
    "Warstorm Surge",
    "Rites of Flourishing",
    "Garruk, Cursed Huntsman",
    "Plaguecrafter",
    "Chaos Warp",
    "Decree of Pain",
    "Spore Frog",
    "Riot Control",
    "Scurry of Squirrels",
    "Faeburrow Elder",
    "Beledros Witherbloom",
    "Gilded Goose",
    "Jacked Rabbit",
    "Selvala, Explorer Returned",
    "Swarmyard Massacre",
    "Skullclamp",
    "Outpost Siege",
    "Tainted Wood",
    "Llanowar Loamspeaker",
    "Peerless Recycling",
    "Octomancer",
    "Rowdy Research",
    "Jolrael, Mwonvuli Recluse",
    "Skyfisher Spider",
    "Pollywog Prodigy",
    "Mangara, the Diplomat",
    "Hoofprints of the Stag",
    "Chitterspitter",
    "Teapot Slinger",
    "Helm of the Host",
    "Tempt with Discovery",
    "Wildsear, Scouring Maw",
];
