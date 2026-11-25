//! Bloomburrow Commander (BLC), lane C: `shared_order` positions 150-299 in ~/bolt-handoff/blc.json.
//! Tests: tests/blc_c.rs.
#![allow(unused_imports)]

use super::*;

/// "Cycling {cost}": "{cost}, Discard this card: Draw a card." from your hand.
fn cycling(d: &mut CardDef, cost: &str) {
    let n = activated(d, cost, false, vec![], vec![draw(1)]);
    d.activated[n].from_hand_discard = true;
}

/// A pain land: {C}, or one of two colors for 1 damage.
fn pain_land(d: &mut CardDef, a: Color, b: Color) {
    d.mana = Some(ManaSource {
        colorless: true,
        ..ManaSource::of(&[a, b])
    });
    d.pain_colored = true;
}

/// "{cost}, {T}: Add" two mana: the first from `a`'s colors, the second from `b`'s.
fn two_mana(d: &mut CardDef, cost: &str, a: &[Color], b: &[Color]) {
    d.paid_mana = Some((
        ManaCost::parse(cost).expect("a mana cost"),
        [ManaSource::of(a), ManaSource::of(b)],
    ));
}

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        "Toski, Bearer of Secrets" => s(
            &[
                "This spell can't be countered.",
                "Toski attacks each combat if able.",
                "Whenever a creature you control deals combat damage to a player, draw a card.",
            ],
            |d, _| {
                d.uncounterable = true;
                d.must_attack = true;
                on(d, When::YourCreatureDealsCombatDamageToPlayer, vec![], vec![draw(1)]);
            },
        ),
        "Tear Asunder" => s(
            &["Kicker {1}{B}", "Exile target artifact or enchantment. If this spell was kicked, exile target nonland permanent instead."],
            |d, _| {
                d.kicker = ManaCost::parse("{1}{B}");
                spell(d, vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::Exile(0)]);
                let nonland = Filter { controller: Controller::Any, ..OPP_NONLAND };
                d.kicked_spell = Some(Ability::new(vec![nonland], vec![Effect::Exile(0)]));
            },
        ),
        "Ogre Slumlord" => s(
            &["Whenever another nontoken creature dies, you may create a 1/1 black Rat creature token.", "Rats you control have deathtouch."],
            |d, t| {
                may(d, When::AnotherNontokenCreatureDies, vec![], vec![Effect::CreateToken { who: Who::You, def: t.rat, n: 1 }]);
                static_on(d, Scope::OtherYourSubtype("Rat"), false, Keywords::DEATHTOUCH);
            },
        ),
        "Tireless Provisioner" => s(
            &["Landfall — Whenever a land you control enters, create a Food token or a Treasure token."],
            |d, t| {
                on_modal(
                    d,
                    When::LandYouControlEnters,
                    None,
                    vec![
                        Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.food, n: 1 }]),
                        Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }]),
                    ],
                )
            },
        ),
        "Starstorm" => s(&["Starstorm deals X damage to each creature.", "Cycling {3}"], |d, _| {
            spell(d, vec![], vec![Effect::DamageX { to: Recipient::EachCreature { without_flying: false } }]);
            cycling(d, "{3}");
        }),
        "Restoration Angel" => s(
            &["When this creature enters, you may exile target non-Angel creature you control, then return that card to the battlefield under your control."],
            |d, _| {
                let f = Filter { not_subtype: Some("Angel"), ..YOUR_CREATURE };
                may(d, When::Enters, vec![f], vec![Effect::BlinkSlot(0)]);
            },
        ),
        "Poison-Tip Archer" => s(&["Whenever another creature dies, each opponent loses 1 life."], |d, _| {
            on(d, When::AnotherCreatureDies, vec![], vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }])
        }),
        "Shamanic Revelation" => s(
            &[
                "Draw a card for each creature you control.",
                "Ferocious — You gain 4 life for each creature you control with power 4 or greater.",
            ],
            |d, _| {
                spell(
                    d,
                    vec![],
                    vec![Effect::DrawPerCreatureYouControl, Effect::GainLifePerCreaturePowerAtLeast { per: 4, power: 4 }],
                )
            },
        ),
        "Stolen by the Fae" => s(
            &["Return target creature with mana value X to its owner's hand. You create X 1/1 blue Faerie creature tokens with flying."],
            |d, t| {
                let f = Filter { mana_value_is_x: true, ..CREATURE };
                spell(d, vec![f], vec![Effect::Bounce(0), Effect::CreateTokensX { def: t.faerie }]);
            },
        ),
        "Time Wipe" => s(&["Return a creature you control to its owner's hand, then destroy all creatures."], |d, _| {
            spell(d, vec![], vec![Effect::ReturnCreatureYouControl, Effect::DestroyAllCreatures])
        }),
        "Perplexing Test" => s(
            &[
                "Choose one —",
                "• Return all creature tokens to their owners' hands.",
                "• Return all nontoken creatures to their owners' hands.",
            ],
            |d, _| {
                modes(
                    d,
                    vec![
                        Ability::new(vec![], vec![Effect::BounceAllCreatures { tokens: true }]),
                        Ability::new(vec![], vec![Effect::BounceAllCreatures { tokens: false }]),
                    ],
                )
            },
        ),
        // ---- lane A, 2026-11-25: cards the engine's pieces cover ----
        "End-Raze Forerunners" => s(
            &["When this creature enters, other creatures you control get +2/+2 and gain vigilance and trample until end of turn."],
            |d, _| {
                let keywords = Keywords::VIGILANCE.with(Keywords::TRAMPLE);
                on(d, When::Enters, vec![], vec![Effect::PumpOthersAndGrant { power: 2, toughness: 2, keywords }]);
            },
        ),
        "Elspeth, Sun's Champion" => s(
            &[
                "+1: Create three 1/1 white Soldier creature tokens.",
                "−3: Destroy all creatures with power 4 or greater.",
                "−7: You get an emblem with \"Creatures you control get +2/+2 and have flying.\"",
            ],
            |d, t| {
                loyalty(d, 1, Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.soldier, n: 3 }]));
                loyalty(d, -3, Ability::new(vec![], vec![Effect::DestroyAllCreaturesPowerAtLeast(4)]));
                loyalty(d, -7, Ability::new(vec![], vec![Effect::Emblem(EmblemKind::ElspethAnthem)]));
            },
        ),
        "Chatterstorm" => s(&["Create a 1/1 green Squirrel creature token.", "Storm"], |d, t| {
            spell(d, vec![], vec![Effect::CreateToken { who: Who::You, def: t.squirrel, n: 1 }]);
            on(d, When::YouCastThis, vec![], vec![Effect::CopySourceSpell { storm: true }]);
        }),
        "Arasta of the Endless Web" => s(
            &["Whenever an opponent casts an instant or sorcery spell, create a 1/2 green Spider creature token with reach."],
            |d, t| on(d, When::OpponentCastsInstantOrSorcery, vec![], vec![Effect::CreateToken { who: Who::You, def: t.spider, n: 1 }]),
        ),
        // Pain lands.
        "Adarkar Wastes" => s(&["{T}: Add {C}.", "{T}: Add {W} or {U}. This land deals 1 damage to you."], |d, _| pain_land(d, Color::White, Color::Blue)),
        "Battlefield Forge" => s(&["{T}: Add {C}.", "{T}: Add {R} or {W}. This land deals 1 damage to you."], |d, _| pain_land(d, Color::Red, Color::White)),
        "Brushland" => s(&["{T}: Add {C}.", "{T}: Add {G} or {W}. This land deals 1 damage to you."], |d, _| pain_land(d, Color::Green, Color::White)),
        "Karplusan Forest" => s(&["{T}: Add {C}.", "{T}: Add {R} or {G}. This land deals 1 damage to you."], |d, _| pain_land(d, Color::Red, Color::Green)),
        "Llanowar Wastes" => s(&["{T}: Add {C}.", "{T}: Add {B} or {G}. This land deals 1 damage to you."], |d, _| pain_land(d, Color::Black, Color::Green)),
        "Shivan Reef" => s(&["{T}: Add {C}.", "{T}: Add {U} or {R}. This land deals 1 damage to you."], |d, _| pain_land(d, Color::Blue, Color::Red)),
        "Yavimaya Coast" => s(&["{T}: Add {C}.", "{T}: Add {G} or {U}. This land deals 1 damage to you."], |d, _| pain_land(d, Color::Green, Color::Blue)),
        // Signets and filter lands: "{cost}, {T}: Add" two mana (the payer uses them when it needs to).
        "Skycloud Expanse" => s(&["{1}, {T}: Add {W}{U}."], |d, _| two_mana(d, "{1}", &[Color::White], &[Color::Blue])),
        "Ferrous Lake" => s(&["{1}, {T}: Add {U}{R}."], |d, _| two_mana(d, "{1}", &[Color::Blue], &[Color::Red])),
        "Mossfire Valley" => s(&["{1}, {T}: Add {R}{G}."], |d, _| two_mana(d, "{1}", &[Color::Red], &[Color::Green])),
        "Overflowing Basin" => s(&["{1}, {T}: Add {G}{U}."], |d, _| two_mana(d, "{1}", &[Color::Green], &[Color::Blue])),
        "Sungrass Prairie" => s(&["{1}, {T}: Add {G}{W}."], |d, _| two_mana(d, "{1}", &[Color::Green], &[Color::White])),
        "Sunscorched Divide" => s(&["{1}, {T}: Add {R}{W}."], |d, _| two_mana(d, "{1}", &[Color::Red], &[Color::White])),
        "Viridescent Bog" => s(&["{1}, {T}: Add {B}{G}."], |d, _| two_mana(d, "{1}", &[Color::Black], &[Color::Green])),
        "Azorius Signet" => s(&["{1}, {T}: Add {W}{U}."], |d, _| two_mana(d, "{1}", &[Color::White], &[Color::Blue])),
        "Boros Signet" => s(&["{1}, {T}: Add {R}{W}."], |d, _| two_mana(d, "{1}", &[Color::Red], &[Color::White])),
        "Golgari Signet" => s(&["{1}, {T}: Add {B}{G}."], |d, _| two_mana(d, "{1}", &[Color::Black], &[Color::Green])),
        "Gruul Signet" => s(&["{1}, {T}: Add {R}{G}."], |d, _| two_mana(d, "{1}", &[Color::Red], &[Color::Green])),
        "Izzet Signet" => s(&["{1}, {T}: Add {U}{R}."], |d, _| two_mana(d, "{1}", &[Color::Blue], &[Color::Red])),
        "Cascade Bluffs" => s(&["{T}: Add {C}.", "{U/R}, {T}: Add {U}{U}, {U}{R}, or {R}{R}."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            two_mana(d, "{U/R}", &[Color::Blue, Color::Red], &[Color::Blue, Color::Red]);
        }),
        "Flooded Grove" => s(&["{T}: Add {C}.", "{G/U}, {T}: Add {G}{G}, {G}{U}, or {U}{U}."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            two_mana(d, "{G/U}", &[Color::Green, Color::Blue], &[Color::Green, Color::Blue]);
        }),
        "Rugged Prairie" => s(&["{T}: Add {C}.", "{R/W}, {T}: Add {R}{R}, {R}{W}, or {W}{W}."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            two_mana(d, "{R/W}", &[Color::Red, Color::White], &[Color::Red, Color::White]);
        }),
        "Twilight Mire" => s(&["{T}: Add {C}.", "{B/G}, {T}: Add {B}{B}, {B}{G}, or {G}{G}."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            two_mana(d, "{B/G}", &[Color::Black, Color::Green], &[Color::Black, Color::Green]);
        }),
        "Necroblossom Snarl" => s(
            &[
                "As this land enters, you may reveal a Swamp or Forest card from your hand. If you don't, this land enters tapped.",
                "{T}: Add {B} or {G}.",
            ],
            |d, _| {
                d.enters_tapped_unless_reveal = Some(["Swamp", "Forest"]);
                d.mana = Some(ManaSource::of(&[Color::Black, Color::Green]));
            },
        ),
        "Primeval Bounty" => s(
            &[
                "Whenever you cast a creature spell, create a 3/3 green Beast creature token.",
                "Whenever you cast a noncreature spell, put three +1/+1 counters on target creature you control.",
                "Landfall — Whenever a land you control enters, you gain 3 life.",
            ],
            |d, t| {
                on(d, When::YouCastCreature, vec![], vec![Effect::CreateToken { who: Who::You, def: t.beast_3, n: 1 }]);
                on(d, When::YouCastNoncreature, vec![YOUR_CREATURE], vec![Effect::Counters { slot: 0, n: 3 }]);
                on(d, When::LandYouControlEnters, vec![], vec![Effect::GainLife { who: Who::You, n: 3 }]);
            },
        ),
        "Nested Shambler" => s(
            &["When this creature dies, create X tapped 1/1 green Squirrel creature tokens, where X is this creature's power."],
            |d, t| on(d, When::Dies, vec![], vec![Effect::CreateTokensPowerTapped { def: t.squirrel }]),
        ),
        "Greater Good" => s(
            &["Sacrifice a creature: Draw cards equal to the sacrificed creature's power, then discard three cards."],
            |d, _| {
                let n = activated(d, "", false, vec![], vec![Effect::DrawAmount, Effect::Discard { who: Who::You, n: 3 }]);
                d.activated[n].sacrifice_other_creature = true;
                d.activated[n].amount_from_victim_power = true;
            },
        ),
        // ---- lane C, 2026-11-25 ----
        "Garruk's Packleader" => s(
            &["Whenever another creature you control with power 3 or greater enters, you may draw a card."],
            |d, _| may(d, When::AnotherCreatureYouControlEntersWithPowerAtLeast(3), vec![], vec![draw(1)]),
        ),
        "Grim Backwoods" => s(&["{T}: Add {C}.", "{2}{B}{G}, {T}, Sacrifice a creature: Draw a card."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            // A land isn't a creature, so "another creature" is any creature.
            let n = activated(d, "{2}{B}{G}", true, vec![], vec![draw(1)]);
            d.activated[n].sacrifice_other_creature = true;
        }),
        "Talisman of Impulse" => s(&["{T}: Add {C}.", "{T}: Add {R} or {G}. This artifact deals 1 damage to you."], |d, _| {
            pain_land(d, Color::Red, Color::Green)
        }),
        "Thriving Bluff" => s(
            &["This land enters tapped. As it enters, choose a color other than red.", "{T}: Add {R} or one mana of the chosen color."],
            |d, _| {
                d.enters_tapped = true;
                d.choose_color = true;
                d.thriving_color = Some(Color::Red);
            },
        ),
        "Spine of Ish Sah" => s(
            &[
                "When this artifact enters, destroy target permanent.",
                "When this artifact is put into a graveyard from the battlefield, return it to its owner's hand.",
            ],
            |d, _| {
                let any = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, ..Filter::ANY };
                on(d, When::Enters, vec![any], vec![Effect::Destroy(0)]);
                on(d, When::Dies, vec![], vec![Effect::ReturnSelfToHand]);
            },
        ),
        "Windgrace's Judgment" => s(
            &["For any number of opponents, destroy target nonland permanent that player controls."],
            // One opponent in a 1v1 game: up to one target.
            |d, _| d.spell = Some(up_to(vec![OPP_NONLAND], vec![Effect::Destroy(0)])),
        ),
        "Psychosis Crawler" => s(
            &[
                "Psychosis Crawler's power and toughness are each equal to the number of cards in your hand.",
                "Whenever you draw a card, each opponent loses 1 life.",
            ],
            |d, _| {
                d.pt_hand_size = true;
                on(d, When::YouDraw, vec![], vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }]);
            },
        ),
        "Deadly Dispute" => s(
            &["As an additional cost to cast this spell, sacrifice an artifact or creature.", "Draw two cards and create a Treasure token."],
            |d, t| {
                d.extra_cost = Some(ExtraCost::SacrificeArtifactOrCreature);
                spell(d, vec![], vec![draw(2), Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }]);
            },
        ),
        "Martial Coup" => s(&["Create X 1/1 white Soldier creature tokens. If X is 5 or more, destroy all other creatures."], |d, t| {
            spell(d, vec![], vec![Effect::DestroyAllCreaturesIfXAtLeast(5), Effect::CreateTokensX { def: t.soldier }])
        }),
        "Intellectual Offering" => s(
            &[
                "Choose an opponent. You and that player each draw three cards.",
                "Choose an opponent. Untap all nonland permanents you control and all nonland permanents that player controls.",
            ],
            |d, _| {
                spell(d, vec![], vec![draw(3), Effect::Draw { who: Who::EachOpponent, n: 3 }, Effect::UntapAllNonland])
            },
        ),
        "Morbid Opportunist" => s(
            &["Whenever one or more other creatures die, draw a card. This ability triggers only once each turn."],
            |d, _| on(d, When::OtherCreaturesDieOnceEachTurn, vec![], vec![draw(1)]),
        ),
        "Illusory Ambusher" => s(&["Whenever this creature is dealt damage, draw that many cards."], |d, _| {
            on(d, When::DealtDamage, vec![], vec![Effect::DrawAmount])
        }),
        "Body of Knowledge" => s(
            &[
                "Body of Knowledge's power and toughness are each equal to the number of cards in your hand.",
                "You have no maximum hand size.",
                "Whenever this creature is dealt damage, draw that many cards.",
            ],
            |d, _| {
                d.pt_hand_size = true;
                d.no_max_hand_size = true;
                on(d, When::DealtDamage, vec![], vec![Effect::DrawAmount]);
            },
        ),
        "Tetsuko Umezawa, Fugitive" => s(&["Creatures you control with power or toughness 1 or less can't be blocked."], |d, _| {
            d.small_creatures_unblockable = true
        }),
        "Deep Forest Hermit" => s(
            &["Vanishing 3", "When this creature enters, create four 1/1 green Squirrel creature tokens.", "Squirrels you control get +1/+1."],
            |d, t| {
                d.vanishing = 3;
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::VanishingTick]);
                on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.squirrel, n: 4 }]);
                lord(d, "Squirrel"); // the Hermit is no Squirrel, so "other" changes nothing
            },
        ),
        "Nadier's Nightblade" => s(&["Whenever a token you control leaves the battlefield, each opponent loses 1 life and you gain 1 life."], |d, _| {
            on(
                d,
                When::TokenYouControlLeaves,
                vec![],
                vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }, Effect::GainLife { who: Who::You, n: 1 }],
            )
        }),
        "Junk Winder" => s(
            &[
                "Affinity for tokens",
                "Whenever a token you control enters, tap target nonland permanent an opponent controls. It doesn't untap during its controller's next untap step.",
            ],
            |d, _| {
                d.reduction = Some(Reduction::PerTokenYouControl);
                on(d, When::TokenYouControlEnters, vec![OPP_NONLAND], vec![Effect::TapFreeze(0)]);
            },
        ),
        "Goreclaw, Terror of Qal Sisma" => s(
            &[
                "Creature spells you cast with power 4 or greater cost {2} less to cast.",
                "Whenever Goreclaw attacks, each creature you control with power 4 or greater gets +1/+1 and gains trample until end of turn.",
            ],
            |d, _| {
                d.big_creature_spell_discount = 2;
                let pump = Effect::PumpYoursPowerAtLeastAndGrant { min: 4, power: 1, toughness: 1, keywords: Keywords::TRAMPLE };
                on(d, When::Attacks, vec![], vec![pump]);
            },
        ),
        "Grumgully, the Generous" => s(&["Each other non-Human creature you control enters with an additional +1/+1 counter on it."], |d, _| {
            d.other_non_subtype_enters_with_counter = Some("Human")
        }),
        "Chittering Witch" => s(
            &[
                "When this creature enters, create a number of 1/1 black Rat creature tokens equal to the number of opponents you have.",
                "{1}{B}, Sacrifice a creature: Target creature gets -2/-2 until end of turn.",
            ],
            |d, t| {
                // A 1v1 game: one opponent.
                on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.rat, n: 1 }]);
                let n = activated(d, "{1}{B}", false, vec![CREATURE], vec![Effect::Pump { slot: 0, power: -2, toughness: -2 }]);
                d.activated[n].sacrifice_creature = true;
            },
        ),
        "Long River's Pull" => s(
            &["Gift a card", "Counter target creature spell. If the gift was promised, instead counter target spell."],
            |d, _| {
                // The gift is promised by casting it "kicked" for no extra cost.
                d.kicker = Some(ManaCost::default());
                let creature_spell = Filter { spell_creature: Some(true), ..SPELL };
                spell(d, vec![creature_spell], vec![Effect::Counter(0)]);
                d.kicked_spell = Some(Ability::new(vec![SPELL], vec![Effect::GiftCard, Effect::Counter(0)]));
            },
        ),
        "Wear Down" => s(
            &[
                "Gift a card",
                "Destroy target artifact or enchantment. If the gift was promised, instead destroy two target artifacts and/or enchantments.",
            ],
            |d, _| {
                d.kicker = Some(ManaCost::default());
                spell(d, vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::Destroy(0)]);
                let two = vec![ARTIFACT_OR_ENCHANTMENT; 2];
                d.kicked_spell = Some(Ability::new(two, vec![Effect::GiftCard, Effect::Destroy(0), Effect::Destroy(1)]));
            },
        ),
        "Jolly Gerbils" => s(&["Whenever you give a gift, draw a card."], |d, _| on(d, When::YouGiveGift, vec![], vec![draw(1)])),
        "Oran-Rief, the Vastwood" => s(
            &["This land enters tapped.", "{T}: Add {G}.", "{T}: Put a +1/+1 counter on each green creature that entered this turn."],
            |d, _| {
                d.enters_tapped = true;
                d.mana = Some(ManaSource::of(&[Color::Green]));
                activated(d, "", true, vec![], vec![Effect::CountersOnGreenEnteredThisTurn]);
            },
        ),
        "Rishkar, Peema Renegade" => s(
            &[
                "When Rishkar enters, put a +1/+1 counter on each of up to two target creatures.",
                "Each creature you control with a counter on it has \"{T}: Add {G}.\"",
            ],
            |d, _| {
                let counters = vec![Effect::Counters { slot: 0, n: 1 }, Effect::Counters { slot: 1, n: 1 }];
                d.triggers.push(Trigger { when: When::Enters, ability: up_to(vec![CREATURE; 2], counters) });
                d.counter_creatures_tap_for_green = true;
            },
        ),
        "Thickest in the Thicket" => s(
            &[
                "When this enchantment enters, put X +1/+1 counters on target creature, where X is that creature's power.",
                "At the beginning of your end step, draw two cards if you control the creature with the greatest power or tied for the greatest power.",
            ],
            |d, _| {
                on(d, When::Enters, vec![CREATURE], vec![Effect::CountersEqualPower(0)]);
                conditional(d, When::EndStep { your_turn_only: true }, Condition::ControlGreatestPower, vec![], vec![draw(2)]);
            },
        ),
        "Twenty-Toed Toad" => s(
            &[
                "Your maximum hand size is twenty.",
                "Whenever you attack with two or more creatures, put a +1/+1 counter on this creature and draw a card.",
                "Whenever this creature attacks, you win the game if there are twenty or more counters on it or you have twenty or more cards in hand.",
            ],
            |d, _| {
                d.max_hand_size = 20;
                on(d, When::YouAttackWithAtLeast(2), vec![], vec![Effect::CountersSelf { n: 1 }, draw(1)]);
                on(d, When::Attacks, vec![], vec![Effect::WinIfCountersOrHandAtLeast(20)]);
            },
        ),
        "Domri, Anarch of Bolas" => s(
            &[
                "Creatures you control get +1/+0.",
                "+1: Add {R} or {G}. Creature spells you cast this turn can't be countered.",
                "−2: Target creature you control fights target creature you don't control.",
            ],
            |d, _| {
                static_on(d, Scope::YourCreatures, false, Keywords::NONE);
                d.statics.last_mut().expect("just added").power = 1;
                let mana = vec![Effect::AddManaEither(Color::Red, Color::Green), Effect::CreatureSpellsUncounterableThisTurn];
                loyalty(d, 1, Ability::new(vec![], mana));
                loyalty(d, -2, Ability::new(vec![YOUR_CREATURE, OPP_CREATURE], vec![Effect::FightBetween { a: 0, b: 1 }]));
            },
        ),
        "Luminous Broodmoth" => s(
            &["Whenever a creature you control without flying dies, return it to the battlefield under its owner's control with a flying counter on it."],
            |d, _| on(d, When::CreatureYouControlWithoutFlyingDies, vec![], vec![Effect::ReturnDiedWithFlyingCounter]),
        ),
        _ => None,
    }
}

/// Lane C's scripted BLC cards (checked against their text in `lib.rs`).
pub const BLC_C_SCRIPTED: &[&str] = &[
    "Necroblossom Snarl",
    "Primeval Bounty",
    "Nested Shambler",
    "Greater Good",
    "Skycloud Expanse",
    "Ferrous Lake",
    "Mossfire Valley",
    "Overflowing Basin",
    "Sungrass Prairie",
    "Sunscorched Divide",
    "Viridescent Bog",
    "Azorius Signet",
    "Boros Signet",
    "Golgari Signet",
    "Gruul Signet",
    "Izzet Signet",
    "Cascade Bluffs",
    "Flooded Grove",
    "Rugged Prairie",
    "Twilight Mire",
    "Adarkar Wastes",
    "Battlefield Forge",
    "Brushland",
    "Karplusan Forest",
    "Llanowar Wastes",
    "Shivan Reef",
    "Yavimaya Coast",
    "End-Raze Forerunners",
    "Elspeth, Sun's Champion",
    "Chatterstorm",
    "Arasta of the Endless Web",
    "Toski, Bearer of Secrets",
    "Tear Asunder",
    "Ogre Slumlord",
    "Tireless Provisioner",
    "Starstorm",
    "Restoration Angel",
    "Poison-Tip Archer",
    "Shamanic Revelation",
    "Stolen by the Fae",
    "Time Wipe",
    "Perplexing Test",
    "Garruk's Packleader",
    "Grim Backwoods",
    "Talisman of Impulse",
    "Thriving Bluff",
    "Spine of Ish Sah",
    "Windgrace's Judgment",
    "Psychosis Crawler",
    "Deadly Dispute",
    "Martial Coup",
    "Intellectual Offering",
    "Morbid Opportunist",
    "Illusory Ambusher",
    "Body of Knowledge",
    "Tetsuko Umezawa, Fugitive",
    "Deep Forest Hermit",
    "Nadier's Nightblade",
    "Junk Winder",
    "Goreclaw, Terror of Qal Sisma",
    "Grumgully, the Generous",
    "Chittering Witch",
    "Long River's Pull",
    "Wear Down",
    "Jolly Gerbils",
    "Oran-Rief, the Vastwood",
    "Rishkar, Peema Renegade",
    "Thickest in the Thicket",
    "Twenty-Toed Toad",
    "Domri, Anarch of Bolas",
    "Luminous Broodmoth",
];
