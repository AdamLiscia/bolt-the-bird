//! Playgroup decks (Nerd Leagues), lane B: the cards for Xenagos, God of Revels (Adam) first, then
//! ~/bolt-handoff/playgroup_all.json's card_order. Card text: assets/PG_oracle.json (in the CMD pool).
//! Tests: tests/pg_b.rs.
#![allow(unused_imports)]

use super::*;

/// "a Forest card" (any land with the Forest type).
fn forest() -> SearchFor {
    SearchFor::LandTypes {
        types: land_types(&["Forest"]),
        basic: false,
    }
}

/// "Creatures you control get +P/+T."
fn anthem(d: &mut CardDef, power: i32, toughness: i32) {
    d.statics.push(Static {
        subtype: None,
        lose_abilities: false,
        base_pt: None,
        protection: [false; 5],
        scale: None,
        scope: Scope::YourCreatures,
        your_turn_only: false,
        condition: None,
        power,
        toughness,
        keywords: Keywords::NONE,
        set_colors: None,
        becomes_land: false,
    });
}

/// "This creature gets +P/+T [and has keywords] as long as <condition>."
fn this_if(d: &mut CardDef, condition: StaticCondition, power: i32, toughness: i32, keywords: Keywords) {
    d.statics.push(Static {
        subtype: None,
        lose_abilities: false,
        base_pt: None,
        protection: [false; 5],
        scale: None,
        scope: Scope::This,
        your_turn_only: false,
        condition: Some(condition),
        power,
        toughness,
        keywords,
        set_colors: None,
        becomes_land: false,
    });
}

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        "Birds of Paradise" => s(&["{T}: Add one mana of any color."], |d, _| {
            d.mana = Some(ManaSource::of(&Color::ALL));
        }),
        "Elvish Mystic" => s(&["{T}: Add {G}."], |d, _| d.mana = Some(ManaSource::of(&[Color::Green]))),
        "Lotus Petal" => s(&["{T}, Sacrifice this artifact: Add one mana of any color."], |d, _| {
            d.mana = Some(ManaSource::of(&Color::ALL));
            d.sac_for_mana = true;
        }),
        "Nature's Lore" => s(&["Search your library for a Forest card, put that card onto the battlefield, then shuffle."], |d, _| {
            spell(d, vec![], vec![search(forest(), SearchTo::Battlefield { tapped: false })])
        }),
        "Three Visits" => s(&["Search your library for a Forest card, put it onto the battlefield, then shuffle."], |d, _| {
            spell(d, vec![], vec![search(forest(), SearchTo::Battlefield { tapped: false })])
        }),
        "Skyshroud Claim" => s(&["Search your library for up to two Forest cards, put them onto the battlefield, then shuffle."], |d, _| {
            let two = Effect::Search { what: forest(), to: SearchTo::Battlefield { tapped: false }, n: 2, kicked_n: None };
            spell(d, vec![], vec![two])
        }),
        "Kodama's Reach" => s(
            &["Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle."],
            |d, _| {
                spell(
                    d,
                    vec![],
                    vec![search(SearchFor::BasicLand, SearchTo::Battlefield { tapped: true }), search(SearchFor::BasicLand, SearchTo::Hand)],
                )
            },
        ),
        "Wooded Foothills" => s(
            &["{T}, Pay 1 life, Sacrifice this land: Search your library for a Mountain or Forest card, put it onto the battlefield, then shuffle."],
            |d, _| fetch_land(d, &["Mountain", "Forest"]),
        ),
        "Aggravated Assault" => s(
            &["{3}{R}{R}: Untap all creatures you control. After this main phase, there is an additional combat phase followed by an additional main phase. Activate only as a sorcery."],
            |d, _| {
                let n = activated(d, "{3}{R}{R}", false, vec![], vec![Effect::UntapCreaturesYouControl, Effect::AdditionalCombatAndMain]);
                d.activated[n].sorcery = true;
            },
        ),
        "Seize the Day" => s(
            &["Untap target creature. After this main phase, there is an additional combat phase followed by an additional main phase.", "Flashback {2}{R}"],
            |d, _| {
                spell(d, vec![CREATURE], vec![Effect::UntapTarget(0), Effect::AdditionalCombatAndMain]);
                d.flashback = ManaCost::parse("{2}{R}");
            },
        ),
        "Grim Monolith" => s(
            &["This artifact doesn't untap during your untap step.", "{T}: Add {C}{C}{C}.", "{4}: Untap this artifact."],
            |d, _| {
                d.doesnt_untap = true;
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                d.mana_amount = 3;
                activated(d, "{4}", false, vec![], vec![Effect::UntapSelf]);
            },
        ),
        "Hull Breach" => s(
            &["Choose one —", "• Destroy target artifact.", "• Destroy target enchantment.", "• Destroy target artifact and target enchantment."],
            |d, _| {
                let enchantment = Filter { players: false, creatures: false, planeswalkers: false, enchantments: true, ..Filter::ANY };
                modes(
                    d,
                    vec![
                        Ability::new(vec![ARTIFACT], vec![Effect::Destroy(0)]),
                        Ability::new(vec![enchantment], vec![Effect::Destroy(0)]),
                        Ability::new(vec![ARTIFACT, enchantment], vec![Effect::Destroy(0), Effect::Destroy(1)]),
                    ],
                );
            },
        ),
        "Heroic Intervention" => s(&["Permanents you control gain hexproof and indestructible until end of turn."], |d, _| {
            spell(d, vec![], vec![Effect::GrantAllPermanentsYouControl(Keywords::HEXPROOF.with(Keywords::INDESTRUCTIBLE))])
        }),
        "Fire-Lit Thicket" => s(&["{T}: Add {C}.", "{R/G}, {T}: Add {R}{R}, {R}{G}, or {G}{G}."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            let rg = [Color::Red, Color::Green];
            d.paid_mana = Some((ManaCost::parse("{R/G}").expect("a mana cost"), [ManaSource::of(&rg), ManaSource::of(&rg)]));
        }),
        "Ancient Tomb" => s(&["{T}: Add {C}{C}. This land deals 2 damage to you."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            d.mana_amount = 2;
            d.mana_tap_damage = 2;
        }),
        "Fiery Emancipation" => s(
            &["If a source you control would deal damage to a permanent or player, it deals triple that damage to that permanent or player instead."],
            |d, _| d.triples_your_damage = true,
        ),
        "Goblin Anarchomancer" => s(&["Each spell you cast that's red or green costs {1} less to cast."], |d, _| {
            d.red_green_spell_discount = 1;
        }),
        "Elder Gargaroth" => s(
            &[
                "Whenever this creature attacks or blocks, choose one —",
                "• Create a 3/3 green Beast creature token.",
                "• You gain 3 life.",
                "• Draw a card.",
            ],
            |d, t| {
                for when in [When::Attacks, When::Blocks] {
                    on_modal(
                        d,
                        when,
                        None,
                        vec![
                            Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.beast_3, n: 1 }]),
                            Ability::new(vec![], vec![Effect::GainLife { who: Who::You, n: 3 }]),
                            Ability::new(vec![], vec![draw(1)]),
                        ],
                    );
                }
            },
        ),
        "Fierce Empath" => s(
            &["When this creature enters, you may search your library for a creature card with mana value 6 or greater, reveal it, put it into your hand, then shuffle."],
            |d, _| may(d, When::Enters, vec![], vec![search(SearchFor::CreatureMvAtLeast(6), SearchTo::Hand)]),
        ),
        "Thornspire Verge" => s(&["{T}: Add {R}.", "{T}: Add {G}. Activate only if you control a Mountain or a Forest."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Red]));
            d.extra_color_if_you_control = Some((Color::Green, ["Mountain", "Forest"]));
        }),
        "Reflecting Pool" => s(&["{T}: Add one mana of any type that a land you control could produce."], |d, _| {
            d.mana_own_land_colors = true;
        }),
        "Stomping Ground" => s(&["As this land enters, you may pay 2 life. If you don't, it enters tapped."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Red, Color::Green]));
            d.shock_land = true; // lane C's shock lands: asked before anyone next gets priority
        }),
        "Kessig Wolf Run" => s(&["{T}: Add {C}.", "{X}{R}{G}, {T}: Target creature gets +X/+0 and gains trample until end of turn."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            activated(d, "{X}{R}{G}", true, vec![CREATURE], vec![Effect::PumpXAndGrant { slot: 0, keywords: Keywords::TRAMPLE }]);
        }),
        "Xenagos, God of Revels" => s(
            &[
                "As long as your devotion to red and green is less than seven, Xenagos isn't a creature.",
                "At the beginning of combat on your turn, another target creature you control gains haste and gets +X/+X until end of turn, where X is that creature's power.",
            ],
            |d, _| {
                d.creature_unless_devotion = Some(([false, false, false, true, true], 7));
                let another = Filter { other: true, ..YOUR_CREATURE };
                on(d, When::BeginCombatYourTurn, vec![another], vec![Effect::PumpByOwnPowerAndGrant { slot: 0, keywords: Keywords::HASTE }]);
            },
        ),
        "Nylea, God of the Hunt" => s(
            &[
                "As long as your devotion to green is less than five, Nylea isn't a creature.",
                "Other creatures you control have trample.",
                "{3}{G}: Target creature gets +2/+2 until end of turn.",
            ],
            |d, _| {
                d.creature_unless_devotion = Some(([false, false, false, false, true], 5));
                static_on(d, Scope::OtherYourCreatures, false, Keywords::TRAMPLE);
                activated(d, "{3}{G}", false, vec![CREATURE], vec![Effect::Pump { slot: 0, power: 2, toughness: 2 }]);
            },
        ),
        "Nylea, Keen-Eyed" => s(
            &[
                "As long as your devotion to green is less than five, Nylea isn't a creature.",
                "Creature spells you cast cost {1} less to cast.",
                "{2}{G}: Reveal the top card of your library. If it's a creature card, put it into your hand. Otherwise, you may put it into your graveyard.",
            ],
            |d, _| {
                d.creature_unless_devotion = Some(([false, false, false, false, true], 5));
                d.creature_spell_discount = 1;
                activated(d, "{2}{G}", false, vec![], vec![Effect::RevealTopCreatureToHandElseMay, Effect::Mill { who: Who::You, n: 1 }]);
            },
        ),
        "Old Gnawbone" => s(&["Whenever a creature you control deals combat damage to a player, create that many Treasure tokens."], |d, t| {
            on(d, When::YourCreatureDealsCombatDamageToPlayerAmount, vec![], vec![Effect::CreateTokensAmount { def: t.treasure }])
        }),
        "Balefire Dragon" => s(
            &["Whenever this creature deals combat damage to a player, it deals that much damage to each creature that player controls."],
            |d, _| on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::DamageAmountToEachOpponentCreature]),
        ),
        "Dragonborn Champion" => s(&["Whenever a source you control deals 5 or more damage to a player, draw a card."], |d, _| {
            on(d, When::YourSourceDealsFiveOrMoreToPlayer, vec![], vec![draw(1)])
        }),
        "Deus of Calamity" => s(
            &["Whenever this creature deals 6 or more damage to an opponent, destroy target land that player controls."],
            |d, _| {
                let land = Filter { players: false, creatures: false, planeswalkers: false, lands: true, controller: Controller::Opponent, ..Filter::ANY };
                on(d, When::DealsSixOrMoreToOpponent, vec![land], vec![Effect::Destroy(0)]);
            },
        ),
        "Kogla, the Titan Ape" => s(
            &[
                "When Kogla enters, it fights up to one target creature you don't control.",
                "Whenever Kogla attacks, destroy target artifact or enchantment defending player controls.",
                "{1}{G}: Return target Human you control to its owner's hand. Kogla gains indestructible until end of turn.",
            ],
            |d, _| {
                d.triggers.push(Trigger { when: When::Enters, ability: up_to(vec![OPP_CREATURE], vec![Effect::Fight(0)]) });
                on(d, When::Attacks, vec![ARTIFACT_OR_ENCHANTMENT.controlled_by(Controller::Opponent)], vec![Effect::Destroy(0)]);
                let human = Filter { subtype: Some("Human"), ..YOUR_CREATURE };
                activated(d, "{1}{G}", false, vec![human], vec![Effect::Bounce(0), Effect::GrantSelf { keywords: Keywords::INDESTRUCTIBLE }]);
            },
        ),
        "Kogla and Yidaro" => s(
            &[
                "When Kogla and Yidaro enters, choose one —",
                "• It gains trample and haste until end of turn.",
                "• It fights target creature you don't control.",
                "{2}{R}{G}, Discard this card: Destroy up to one target artifact or enchantment. Shuffle this card into your library from your graveyard, then draw a card.",
            ],
            |d, _| {
                on_modal(
                    d,
                    When::Enters,
                    None,
                    vec![
                        Ability::new(vec![], vec![Effect::GrantSelf { keywords: Keywords::TRAMPLE.with(Keywords::HASTE) }]),
                        Ability::new(vec![OPP_CREATURE], vec![Effect::Fight(0)]),
                    ],
                );
                let n = activated(
                    d,
                    "{2}{R}{G}",
                    false,
                    vec![ARTIFACT_OR_ENCHANTMENT],
                    vec![Effect::Destroy(0), Effect::ShuffleSourceFromGraveyardIntoLibrary, draw(1)],
                );
                d.activated[n].from_hand_discard = true;
                d.activated[n].ability.min_targets = Some(0);
            },
        ),
        "Malignus" => s(
            &[
                "Malignus's power and toughness are each equal to half the highest life total among your opponents, rounded up.",
                "Damage that would be dealt by this creature can't be prevented.",
            ],
            |d, _| {
                d.pt_half_highest_opponent_life = true;
                d.damage_cant_be_prevented = true;
            },
        ),
        "Questing Beast" => s(
            &[
                "Questing Beast can't be blocked by creatures with power 2 or less.",
                "Combat damage that would be dealt by creatures you control can't be prevented.",
                "Whenever Questing Beast deals combat damage to an opponent, it deals that much damage to target planeswalker that player controls.",
            ],
            |d, _| {
                d.not_blocked_by_power_at_most = Some(2);
                d.your_combat_damage_cant_be_prevented = true;
                let walker = Filter { players: false, creatures: false, planeswalkers: true, controller: Controller::Opponent, ..Filter::ANY };
                on(d, When::DealsCombatDamageToPlayer, vec![walker], vec![Effect::DamageAmountTo(0)]);
            },
        ),
        "The Great Henge" => s(
            &[
                "This spell costs {X} less to cast, where X is the greatest power among creatures you control.",
                "{T}: Add {G}{G}. You gain 2 life.",
                "Whenever a nontoken creature you control enters, put a +1/+1 counter on it and draw a card.",
            ],
            |d, _| {
                d.reduction = Some(Reduction::GreatestPowerAmongYourCreatures);
                d.mana = Some(ManaSource::of(&[Color::Green]));
                d.mana_amount = 2;
                d.mana_tap_life = 2;
                let that = Filter { triggering: true, ..CREATURE };
                on(d, When::AnotherNontokenCreatureYouControlEnters, vec![that], vec![Effect::Counters { slot: 0, n: 1 }, draw(1)]);
            },
        ),
        "Zilortha, Strength Incarnate" => s(
            &["Lethal damage dealt to creatures you control is determined by their power rather than their toughness."],
            |d, _| d.lethal_damage_by_power = true,
        ),
        "Vandalblast" => s(&["Destroy target artifact you don't control.", "Overload {4}{R}"], |d, _| {
            // Overload as an optional {4} on top of {R} (cast with `Action::CastKicked`): the same
            // total, and "target" becomes "each".
            spell(d, vec![ARTIFACT.controlled_by(Controller::Opponent)], vec![Effect::Destroy(0)]);
            d.kicker = ManaCost::parse("{4}");
            d.kicked_spell = Some(Ability::new(vec![], vec![Effect::DestroyOpponentsArtifacts]));
        }),
        "Bloodthirster" => s(
            &[
                "Whenever this creature deals combat damage to a player, untap it. After this phase, there is an additional combat phase.",
                "This creature can't attack a player it has already attacked this turn.",
            ],
            |d, _| {
                on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::UntapSelf, Effect::AdditionalCombatAfterThisPhase]);
                d.cant_attack_twice_per_turn = true;
            },
        ),
        "Karlach, Fury of Avernus" => s(
            &[
                "Whenever you attack, if it's the first combat phase of the turn, untap all attacking creatures. They gain first strike until end of turn. After this phase, there is an additional combat phase.",
                "Choose a Background",
            ],
            |d, _| {
                // "Choose a Background" is deck construction (a second commander); the engine's
                // Commander seats take two commanders as partners.
                conditional(
                    d,
                    When::YouAttack,
                    Condition::FirstCombatOfTurn,
                    vec![],
                    vec![Effect::UntapAttackersAndGrant(Keywords::FIRST_STRIKE), Effect::AdditionalCombatAfterThisPhase],
                );
            },
        ),
        "Combat Celebrant" => s(
            &["If this creature hasn't been exerted this turn, you may exert it as it attacks. When you do, untap all other creatures you control and after this phase, there is an additional combat phase."],
            |d, _| {
                // The exert choice is made as the attack trigger resolves.
                let mut a = Ability::new(vec![], vec![Effect::ExertSelfUntapOthers, Effect::AdditionalCombatAfterThisPhase]).optional();
                a.condition = Some(Condition::NotExertedThisTurn);
                d.triggers.push(Trigger { when: When::Attacks, ability: a });
            },
        ),
        "Hellkite Tyrant" => s(
            &[
                "Whenever this creature deals combat damage to a player, gain control of all artifacts that player controls.",
                "At the beginning of your upkeep, if you control twenty or more artifacts, you win the game.",
            ],
            |d, _| {
                on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::GainControlOfOpponentsArtifacts]);
                conditional(d, When::Upkeep { your_turn_only: true }, Condition::ControlArtifactsAtLeast(20), vec![], vec![Effect::WinTheGame]);
            },
        ),
        "Yavimaya, Cradle of Growth" => s(&["Each land is a Forest in addition to its other land types."], |d, _| {
            static_on(d, Scope::EachLand, false, Keywords::NONE);
            d.statics.last_mut().expect("just added").subtype = Some("Forest");
            d.lands_are_forests = true;
        }),
        "Quartzwood Crasher" => s(
            &["Whenever one or more creatures you control with trample deal combat damage to a player, create an X/X green Dinosaur Beast creature token with trample, where X is the amount of damage those creatures dealt to that player."],
            |d, t| on(d, When::YourTramplersDealCombatDamageToPlayer, vec![], vec![Effect::CreateTokenWithCountersAmount { def: t.dinosaur_beast }]),
        ),
        "Ilharg, the Raze-Boar" => s(
            &[
                "Whenever Ilharg attacks, you may put a creature card from your hand onto the battlefield tapped and attacking. Return that creature to your hand at the beginning of the next end step.",
                "When Ilharg dies or is put into exile from the battlefield, you may put it into its owner's library third from the top.",
            ],
            |d, _| {
                on(d, When::Attacks, vec![], vec![Effect::PutCreatureFromHandAttackingUntilEnd]);
                let mut a = Ability::new(vec![], vec![Effect::SourceThirdFromTop]).optional();
                a.condition = Some(Condition::SourceInGraveyardOrExile);
                d.triggers.push(Trigger { when: When::LeavesBattlefield, ability: a });
            },
        ),
        "Chaos Imps" => s(&["Unleash", "This creature has trample as long as it has a +1/+1 counter on it."], |d, _| {
            d.unleash = true;
            // Unleash: "It can't block as long as it has a +1/+1 counter on it."
            static_on(d, Scope::This, false, Keywords::CANT_BLOCK);
            d.statics.last_mut().expect("just added").condition = Some(StaticCondition::CountersAtLeast(1));
            static_on(d, Scope::This, false, Keywords::TRAMPLE);
            d.statics.last_mut().expect("just added").condition = Some(StaticCondition::CountersAtLeast(1));
        }),
        "Silvos, Rogue Elemental" => s(&["{G}: Regenerate Silvos."], |d, _| {
            activated(d, "{G}", false, vec![], vec![Effect::RegenerateSelf]);
        }),
        "Yavimaya Hollow" => s(&["{T}: Add {C}.", "{G}, {T}: Regenerate target creature."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            activated(d, "{G}", true, vec![CREATURE], vec![Effect::Regenerate(0)]);
        }),
        "Triumph of the Hordes" => s(&["Until end of turn, creatures you control get +1/+1 and gain trample and infect."], |d, _| {
            spell(d, vec![], vec![Effect::PumpAllGrantInfect { power: 1, toughness: 1, keywords: Keywords::TRAMPLE }])
        }),
        "Savage Beating" => s(
            &[
                "Cast this spell only during combat on your turn.",
                "Choose one —",
                "• Creatures you control gain double strike until end of turn.",
                "• Untap all creatures you control. After this phase, there is an additional combat phase.",
                "Entwine {1}{R}",
            ],
            |d, _| {
                d.cast_only_during_your_combat = true;
                let double = Effect::GrantAll { who: Who::You, keywords: Keywords::DOUBLE_STRIKE };
                let again = [Effect::UntapCreaturesYouControl, Effect::AdditionalCombatAfterThisPhase];
                modes(
                    d,
                    vec![
                        Ability::new(vec![], vec![double]),
                        Ability::new(vec![], again.to_vec()),
                        Ability::new(vec![], vec![double, again[0], again[1]]), // entwined
                    ],
                );
                d.entwine = ManaCost::parse("{1}{R}");
            },
        ),
        "Anzrag, the Quake-Mole" => s(
            &[
                "Whenever Anzrag becomes blocked, untap each creature you control. After this phase, there is an additional combat phase.",
                "{3}{R}{R}{G}{G}: Anzrag must be blocked each combat this turn if able.",
            ],
            |d, _| {
                on(d, When::BecomesBlocked, vec![], vec![Effect::UntapCreaturesYouControl, Effect::AdditionalCombatAfterThisPhase]);
                activated(d, "{3}{R}{R}{G}{G}", false, vec![], vec![Effect::MustBeBlockedThisTurnSelf]);
            },
        ),
        "Fanatic of Rhonas" => s(
            &[
                "{T}: Add {G}.",
                "Ferocious — {T}: Add {G}{G}{G}{G}. Activate only if you control a creature with power 4 or greater.",
                "Eternalize {2}{G}{G}",
            ],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::Green]));
                d.ferocious_mana_amount = 4;
                renew(d, "{2}{G}{G}", vec![], vec![Effect::EternalizeSelf]);
            },
        ),
        "Moraug, Fury of Akoum" => s(
            &[
                "Each creature you control gets +1/+0 for each time it has attacked this turn.",
                "Landfall — Whenever a land you control enters, if it's your main phase, there's an additional combat phase after this phase. At the beginning of that combat, untap all creatures you control.",
            ],
            |d, _| {
                d.power_per_attack_this_turn = true;
                // The untap is done as that combat begins (not put on the stack as a trigger).
                conditional(d, When::LandYouControlEnters, Condition::YourMainPhase, vec![], vec![Effect::AdditionalCombatAfterMainUntap]);
            },
        ),
        "Defiler of Vigor" => s(
            &[
                "As an additional cost to cast green permanent spells, you may pay 2 life. Those spells cost {G} less to cast if you paid life this way. This effect reduces only the amount of green mana you pay.",
                "Whenever you cast a green permanent spell, put a +1/+1 counter on each creature you control.",
            ],
            |d, _| {
                d.green_permanents_pay_life_for_g = true;
                on(d, When::YouCastGreenPermanent, vec![], vec![Effect::CountersAll { who: Who::You, n: 1 }]);
            },
        ),
        "World at War" => s(
            &[
                "After the second main phase this turn, there's an additional combat phase followed by an additional main phase. At the beginning of that combat, untap all creatures that attacked this turn.",
                "Rebound",
            ],
            |d, _| spell(d, vec![], vec![Effect::AdditionalCombatAfterSecondMainUntapAttackers, Effect::Rebound]),
        ),
        "Roaming Throne" => s(
            &[
                "Ward {2}",
                "As this creature enters, choose a creature type.",
                "This creature is the chosen type in addition to its other types.",
                "If a triggered ability of another creature you control of the chosen type triggers, it triggers an additional time.",
            ],
            |d, _| {
                d.ward = ManaCost::parse("{2}");
                d.choose_creature_type = true;
                d.doubles_chosen_type_triggers = true;
            },
        ),
        // ---- Xenagos finish: changing targets and transforming double-faced cards ----
        "Deflecting Swat" => s(
            &[
                "If you control a commander, you may cast this spell without paying its mana cost.",
                "You may choose new targets for target spell or ability.",
            ],
            |d, _| {
                d.alt_cost = Some((ManaCost::default(), 0));
                d.alt_cost_needs_commander = true;
                let spell_or_ability = Filter { abilities: true, ..SPELL };
                spell(d, vec![spell_or_ability], vec![Effect::ChangeTargets(0)]);
            },
        ),
        "Ojer Kaslem, Deepest Growth // Temple of Cultivation" => s(
            &[
                "Whenever Ojer Kaslem deals combat damage to a player, reveal that many cards from the top of your library. You may put a creature card and/or a land card from among them onto the battlefield. Put the rest on the bottom in a random order.",
                "When Ojer Kaslem dies, return it to the battlefield tapped and transformed under its owner's control.",
                "Back: {T}: Add {G}.",
                "Back: {2}{G}, {T}: Transform this land. Activate only if you control ten or more permanents and only as a sorcery.",
            ],
            |d, _| {
                on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::RevealAmountPutCreatureAndLand]);
                on(d, When::Dies, vec![], vec![Effect::ReturnSelfTappedTransformed]);
                let b = d.back.as_mut().expect("Temple of Cultivation");
                b.mana = Some(ManaSource::of(&[Color::Green]));
                let n = activated(b, "{2}{G}", true, vec![], vec![Effect::TransformSelf]);
                b.activated[n].sorcery = true;
                b.activated[n].min_permanents = 10;
            },
        ),
        "Vorinclex // The Grand Evolution" => s(
            &[
                "When Vorinclex enters, search your library for up to two Forest cards, reveal them, put them into your hand, then shuffle.",
                "{6}{G}{G}: Exile Vorinclex, then return it to the battlefield transformed under its owner's control. Activate only as a sorcery.",
                "Back: I — Mill ten cards. Put up to two creature cards from among the milled cards onto the battlefield.",
                "Back: II — Distribute seven +1/+1 counters among any number of target creatures you control.",
                "Back: III — Until end of turn, creatures you control gain \"{1}: This creature fights target creature you don't control.\" Exile this Saga, then return it to the battlefield .",
            ],
            |d, _| {
                let two = Effect::Search { what: forest(), to: SearchTo::Hand, n: 2, kicked_n: None };
                on(d, When::Enters, vec![], vec![two]);
                let n = activated(d, "{6}{G}{G}", false, vec![], vec![Effect::ExileSelfReturn { transformed: true }]);
                d.activated[n].sorcery = true;
                let b = d.back.as_mut().expect("The Grand Evolution");
                chapter(b, 1, Ability::new(vec![], vec![Effect::MillPutCreatures { n: 10, up_to: 2 }]));
                let mut seven = up_to(vec![YOUR_CREATURE; 7], vec![Effect::DividedCounters]);
                seven.divide = 7;
                chapter(b, 2, seven);
                chapter(
                    b,
                    3,
                    Ability::new(vec![], vec![Effect::GrantFightAbilityEot, Effect::ExileSelfReturn { transformed: false }]),
                );
            },
        ),
        // ---- Aesi (Time), lane B ----
        "Eternal Witness" => s(&["When this creature enters, you may return target card from your graveyard to your hand."], |d, _| {
            may(d, When::Enters, vec![graveyard_cards(false, Controller::You)], vec![Effect::Bounce(0)])
        }),
        "Negate" => s(&["Counter target noncreature spell."], |d, _| {
            spell(d, vec![Filter { spell_creature: Some(false), ..SPELL }], vec![Effect::Counter(0)])
        }),
        "Mana Leak" => s(&["Counter target spell unless its controller pays {3}."], |d, _| {
            spell(d, vec![SPELL], vec![Effect::CounterUnlessPays(0, 3, 3)])
        }),
        "Scatter Ray" => s(&["Counter target artifact or creature spell unless its controller pays {4}."], |d, _| {
            spell(d, vec![Filter { spell_artifact_or_creature: true, ..SPELL }], vec![Effect::CounterUnlessPays(0, 4, 4)])
        }),
        "Frantic Search" => s(&["Draw two cards, then discard two cards. Untap up to three lands."], |d, _| {
            spell(d, vec![], vec![draw(2), discard(2), Effect::UntapLandsUpTo(3)])
        }),
        "Courser of Kruphix" => s(
            &[
                "Play with the top card of your library revealed.",
                "You may play lands from the top of your library.",
                "Landfall — Whenever a land you control enters, you gain 1 life.",
            ],
            |d, _| {
                d.lands_from_top = true; // revealing it is information only
                on(d, When::LandYouControlEnters, vec![], vec![Effect::GainLife { who: Who::You, n: 1 }]);
            },
        ),
        "Basalt Monolith" => s(
            &["This artifact doesn't untap during your untap step.", "{T}: Add {C}{C}{C}.", "{3}: Untap this artifact."],
            |d, _| {
                d.doesnt_untap = true;
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                d.mana_amount = 3;
                activated(d, "{3}", false, vec![], vec![Effect::UntapSelf]);
            },
        ),
        "Myriad Landscape" => s(
            &[
                "This land enters tapped.",
                "{T}: Add {C}.",
                "{2}, {T}, Sacrifice this land: Search your library for up to two basic land cards that share a land type, put them onto the battlefield tapped, then shuffle.",
            ],
            |d, _| {
                d.enters_tapped = true;
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                let two = Effect::Search { what: SearchFor::BasicLandSharingType, to: SearchTo::Battlefield { tapped: true }, n: 2, kicked_n: None };
                let n = activated(d, "{2}", true, vec![], vec![two]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Silverback Elder" => s(
            &[
                "Whenever you cast a creature spell, choose one —",
                "• Destroy target artifact or enchantment.",
                "• Look at the top five cards of your library. You may put a land card from among them onto the battlefield tapped. Put the rest on the bottom of your library in a random order.",
                "• You gain 4 life.",
            ],
            |d, _| {
                let look = Effect::LookAtTop { n: 5, pick: Pick::Land, chosen_to: LookTo::BattlefieldTapped, rest_to: LookRest::BottomRandom };
                on_modal(
                    d,
                    When::YouCastCreature,
                    None,
                    vec![
                        Ability::new(vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::Destroy(0)]),
                        Ability::new(vec![], vec![look]),
                        Ability::new(vec![], vec![Effect::GainLife { who: Who::You, n: 4 }]),
                    ],
                )
            },
        ),
        "Horn of Greed" => s(&["Whenever a player plays a land, that player draws a card."], |d, _| {
            // Only the active player plays lands.
            on(d, When::PlayerPlaysLand, vec![], vec![Effect::ActivePlayerDraws(1)])
        }),
        "Simic Growth Chamber" => s(
            &["This land enters tapped.", "When this land enters, return a land you control to its owner's hand.", "{T}: Add {G}{U}."],
            |d, _| {
                d.enters_tapped = true;
                on(d, When::Enters, vec![], vec![Effect::ReturnLandYouControl]);
                d.paid_mana = Some((ManaCost::default(), [ManaSource::of(&[Color::Green]), ManaSource::of(&[Color::Blue])]));
            },
        ),
        "Emerald Medallion" => s(&["Green spells you cast cost {1} less to cast."], |d, _| d.green_spell_discount = 1),
        "Jaheira's Respite" => s(
            &[
                "Search your library for up to X basic land cards, where X is the number of creatures attacking you, put those cards onto the battlefield tapped, then shuffle.",
                "Prevent all combat damage that would be dealt this turn.",
            ],
            |d, _| spell(d, vec![], vec![Effect::SearchBasicsPerAttackersTapped, Effect::PreventAllCombatDamageThisTurn]),
        ),
        "Sprouting Vines" => s(
            &["Search your library for a basic land card, reveal that card, put it into your hand, then shuffle.", "Storm"],
            |d, _| {
                spell(d, vec![], vec![search(SearchFor::BasicLand, SearchTo::Hand)]);
                on(d, When::YouCastThis, vec![], vec![Effect::CopySourceSpell { storm: true }]);
            },
        ),
        "Sword of the Animist" => s(
            &[
                "Equipped creature gets +1/+1.",
                "Whenever equipped creature attacks, you may search your library for a basic land card, put it onto the battlefield tapped, then shuffle.",
                "Equip {2}",
            ],
            |d, _| {
                attached(d, false, 1, 1, Keywords::NONE);
                may(d, When::EquippedAttacks, vec![], vec![search(SearchFor::BasicLand, SearchTo::Battlefield { tapped: true })]);
                equip(d, "{2}");
            },
        ),
        "Tectonic Edge" => s(
            &["{T}: Add {C}.", "{1}, {T}, Sacrifice this land: Destroy target nonbasic land. Activate only if an opponent controls four or more lands."],
            |d, _| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                let nonbasic = Filter { players: false, creatures: false, planeswalkers: false, lands: true, nonbasic: true, ..Filter::ANY };
                let n = activated(d, "{1}", true, vec![nonbasic], vec![Effect::Destroy(0)]);
                d.activated[n].sacrifice_self = true;
                d.activated[n].opponent_min_lands = 4;
            },
        ),
        "Tolaria West" => s(&["This land enters tapped.", "{T}: Add {U}.", "Transmute {1}{U}{U}"], |d, _| {
            d.enters_tapped = true;
            d.mana = Some(ManaSource::of(&[Color::Blue]));
            // Transmute (CR 702.53): discard it from your hand, as a sorcery.
            let n = activated(d, "{1}{U}{U}", false, vec![], vec![search(SearchFor::ManaValue(0), SearchTo::Hand)]);
            d.activated[n].from_hand_discard = true;
            d.activated[n].sorcery = true;
        }),
        "Zimone, Quandrix Prodigy" => s(
            &[
                "{1}, {T}: You may put a land card from your hand onto the battlefield tapped.",
                "{4}, {T}: Draw a card. If you control eight or more lands, draw two cards instead.",
            ],
            |d, _| {
                activated(d, "{1}", true, vec![], vec![Effect::MayPutLandFromHandTapped]);
                activated(d, "{4}", true, vec![], vec![Effect::DrawMoreIfLands { n: 1, more: 2, lands: 8 }]);
            },
        ),
        "Rhystic Study" => s(&["Whenever an opponent casts a spell, you may draw a card unless that player pays {1}."], |d, _| {
            may(d, When::OpponentCasts, vec![], vec![Effect::UnlessOpponentPays(1), draw(1)])
        }),
        "Mystic Remora" => s(
            &["Cumulative upkeep {1}", "Whenever an opponent casts a noncreature spell, you may draw a card unless that player pays {4}."],
            |d, _| {
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::CumulativeUpkeep(1)]);
                may(d, When::OpponentCastsNoncreature, vec![], vec![Effect::UnlessOpponentPays(4), draw(1)]);
            },
        ),
        "Arcane Denial" => s(
            &[
                "Counter target spell. Its controller may draw up to two cards at the beginning of the next turn's upkeep.",
                "You draw a card at the beginning of the next turn's upkeep.",
            ],
            |d, _| {
                // "Up to two": they always draw both (docs/tdm-lane-b.md).
                spell(d, vec![SPELL], vec![Effect::NextUpkeepDraws { slot: 0, theirs: 2, yours: 1 }, Effect::Counter(0)]);
            },
        ),
        "Perennial Behemoth" => s(&["You may play lands from your graveyard.", "Unearth {G}{G}"], |d, _| {
            d.lands_from_graveyard = true;
            let n = activated(d, "{G}{G}", false, vec![], vec![Effect::Unearth]);
            d.activated[n].from_graveyard = true;
            d.activated[n].sorcery = true;
        }),
        "Walk the Aeons" => s(&["Buyback—Sacrifice three Islands.", "Target player takes an extra turn after this one."], |d, _| {
            d.kicker = Some(ManaCost::default());
            d.buyback_sacrifice = Some(("Island", 3));
            spell(d, vec![Filter::PLAYER], vec![Effect::ExtraTurnFor(0)]);
        }),
        "Nissa, Who Shakes the World" => s(
            &[
                "Whenever you tap a Forest for mana, add an additional {G}.",
                "+1: Put three +1/+1 counters on up to one target noncreature land you control. Untap it. It becomes a 0/0 Elemental creature with vigilance and haste that's still a land.",
                "−8: You get an emblem with \"Lands you control have indestructible.\" Search your library for any number of Forest cards, put them onto the battlefield tapped, then shuffle.",
            ],
            |d, _| {
                d.forests_add_g = true;
                let land = Filter { players: false, creatures: false, planeswalkers: false, lands: true, noncreature: true, controller: Controller::You, ..Filter::ANY };
                loyalty(d, 1, up_to(vec![land], vec![Effect::AnimateLandElemental(0)]));
                let forests = Effect::Search { what: forest(), to: SearchTo::Battlefield { tapped: true }, n: u8::MAX, kicked_n: None };
                loyalty(d, -8, Ability::new(vec![], vec![Effect::Emblem(EmblemKind::LandsIndestructible), forests]));
            },
        ),
        "Blessing of Frost" => s(
            &["Distribute X +1/+1 counters among any number of creatures you control, where X is the amount of {S} spent to cast this spell. Then draw a card for each creature you control with power 4 or greater."],
            |d, _| {
                d.x_is_snow_spent = true;
                spell(d, vec![], vec![Effect::DistributeCountersX, Effect::DrawPerCreaturePowerAtLeast(4)]);
            },
        ),
        // ---- Myrkul, Lord of Bones (DonSpider), lane B ----
        "Temple Garden" => s(&["As this land enters, you may pay 2 life. If you don't, it enters tapped."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Green, Color::White]));
            d.shock_land = true;
        }),
        "Godless Shrine" => s(&["As this land enters, you may pay 2 life. If you don't, it enters tapped."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::White, Color::Black]));
            d.shock_land = true;
        }),
        "Vault of Champions" => s(&["This land enters tapped unless you have two or more opponents.", "{T}: Add {W} or {B}."], |d, _| {
            d.enters_tapped = true; // a two-player game: one opponent
            d.mana = Some(ManaSource::of(&[Color::White, Color::Black]));
        }),
        "Bountiful Promenade" => s(&["This land enters tapped unless you have two or more opponents.", "{T}: Add {G} or {W}."], |d, _| {
            d.enters_tapped = true;
            d.mana = Some(ManaSource::of(&[Color::Green, Color::White]));
        }),
        "Caves of Koilos" => s(&["{T}: Add {C}.", "{T}: Add {W} or {B}. This land deals 1 damage to you."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::of(&[Color::White, Color::Black]) });
            d.pain_colored = true;
        }),
        "Wooded Bastion" => s(&["{T}: Add {C}.", "{G/W}, {T}: Add {G}{G}, {G}{W}, or {W}{W}."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            let gw = ManaSource::of(&[Color::Green, Color::White]);
            d.paid_mana = Some((ManaCost::parse("{G/W}").expect("cost"), [gw, gw]));
        }),
        "City of Brass" => s(&["Whenever this land becomes tapped, it deals 1 damage to you.", "{T}: Add one mana of any color."], |d, _| {
            d.mana = Some(ManaSource::of(&Color::ALL));
            on(d, When::BecomesTapped, vec![], vec![damage(1, Recipient::You)]);
        }),
        "Despark" => s(&["Exile target permanent with mana value 4 or greater."], |d, _| {
            let big = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, min_mana_value: Some(4), ..Filter::ANY };
            spell(d, vec![big], vec![Effect::Exile(0)]);
        }),
        "Anguished Unmaking" => s(&["Exile target nonland permanent. You lose 3 life."], |d, _| {
            let nonland = Filter { players: false, creatures: false, planeswalkers: false, nonland_permanents: true, ..Filter::ANY };
            spell(d, vec![nonland], vec![Effect::Exile(0), Effect::LoseLife { who: Who::You, n: 3 }]);
        }),
        "Anointed Procession" => s(
            &["If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead."],
            |d, _| d.doubles_tokens = true,
        ),
        "Vanquish the Horde" => s(&["This spell costs {1} less to cast for each creature on the battlefield.", "Destroy all creatures."], |d, _| {
            d.reduction = Some(Reduction::PerCreatureOnBattlefield);
            spell(d, vec![], vec![Effect::DestroyAllCreatures]);
        }),
        "Smothering Tithe" => s(&["Whenever an opponent draws a card, that player may pay {2}. If the player doesn't, you create a Treasure token."], |d, t| {
            on(d, When::OpponentDraws, vec![], vec![Effect::UnlessOpponentPays(2), Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }]);
        }),
        "Setessan Champion" => s(
            &["Constellation — Whenever an enchantment you control enters, put a +1/+1 counter on this creature and draw a card."],
            |d, _| on(d, When::EnchantmentYouControlEnters, vec![], vec![Effect::CountersSelf { n: 1 }, draw(1)]),
        ),
        "Eidolon of Blossoms" => s(
            &["Constellation — Whenever this creature or another enchantment you control enters, draw a card."],
            |d, _| on(d, When::EnchantmentYouControlEnters, vec![], vec![draw(1)]),
        ),
        "Grim Guardian" => s(
            &["Constellation — Whenever this creature or another enchantment you control enters, each opponent loses 1 life."],
            |d, _| on(d, When::EnchantmentYouControlEnters, vec![], vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }]),
        ),
        "Archon of Sun's Grace" => s(
            &[
                "Pegasus creatures you control have lifelink.",
                "Constellation — Whenever an enchantment you control enters, create a 2/2 white Pegasus creature token with flying.",
            ],
            |d, t| {
                static_on(d, Scope::OtherYourSubtype("Pegasus"), false, Keywords::LIFELINK);
                on(d, When::EnchantmentYouControlEnters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.pegasus, n: 1 }]);
            },
        ),
        "Ghostly Dancers" => s(
            &[
                "When this creature enters, return an enchantment card from your graveyard to your hand or unlock a locked door of a Room you control.",
                "Eerie — Whenever an enchantment you control enters and whenever you fully unlock a Room, create a 3/1 white Spirit creature token with flying.",
            ],
            |d, t| {
                // The engine has no Rooms: the return is the only choice, and no Room unlocks.
                on(d, When::Enters, vec![], vec![Effect::ReturnFromGraveyard { pick: Pick::Enchantment }]);
                on(d, When::EnchantmentYouControlEnters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.spirit_3_1, n: 1 }]);
            },
        ),
        "Sanctum Weaver" => s(&["{T}: Add X mana of any one color, where X is the number of enchantments you control."], |d, _| {
            d.mana = Some(ManaSource::of(&Color::ALL));
            d.mana_per_enchantment = true;
        }),
        "Serra's Sanctum" => s(&["{T}: Add {W} for each enchantment you control."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::White]));
            d.mana_per_enchantment = true;
        }),
        "Grand Abolisher" => s(
            &["During your turn, your opponents can't cast spells or activate abilities of artifacts, creatures, or enchantments."],
            |d, _| {
                d.opponents_cant_cast_your_turn = true;
                d.opponents_cant_activate_your_turn = true;
            },
        ),
        "Devoted Druid" => s(&["{T}: Add {G}.", "Put a -1/-1 counter on this creature: Untap this creature."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Green]));
            let n = activated(d, "", false, vec![], vec![Effect::UntapSelf]);
            d.activated[n].minus_counter_cost = true;
        }),
        "Yawgmoth, Thran Physician" => s(
            &[
                "Protection from Humans",
                "Pay 1 life, Sacrifice another creature: Put a -1/-1 counter on up to one target creature and draw a card.",
                "{B}{B}, Discard a card: Proliferate.",
            ],
            |d, _| {
                d.protection_from_subtype = Some("Human");
                let n = activated(d, "", false, vec![], vec![Effect::MinusCounter(0), draw(1)]);
                d.activated[n].life = 1;
                d.activated[n].sacrifice_other_creature = true;
                d.activated[n].ability = up_to(vec![CREATURE], vec![Effect::MinusCounter(0), draw(1)]);
                let m = activated(d, "{B}{B}", false, vec![], vec![Effect::Proliferate]);
                d.activated[m].discard_cost = true;
            },
        ),
        "Myrkul, Lord of Bones" => s(
            &[
                "As long as your life total is less than or equal to half your starting life total, Myrkul has indestructible.",
                "Whenever another nontoken creature you control dies, you may exile it. If you do, create a token that's a copy of that card, except it's an enchantment and loses all other card types.",
            ],
            |d, _| {
                d.indestructible_at_half_life = true;
                let it = Filter { triggering: true, ..graveyard_cards(true, Controller::You) };
                may(d, When::AnotherNontokenCreatureYouControlDies, vec![it], vec![Effect::ExileMakeEnchantmentCopy(0)]);
            },
        ),
        "Maha, Its Feathers Night" => s(&["Ward—Discard a card.", "Creatures your opponents control have base toughness 1."], |d, _| {
            d.ward_discard = true;
            d.opponents_creatures_base_toughness_one = true;
        }),
        "Esper Sentinel" => s(
            &["Whenever an opponent casts their first noncreature spell each turn, draw a card unless that player pays {X}, where X is this creature's power."],
            |d, _| on(d, When::OpponentCastsNoncreature, vec![], vec![Effect::UnlessOpponentPaysPower, draw(1)]),
        ),
        "Austere Command" => s(
            &[
                "Choose two —",
                "• Destroy all artifacts.",
                "• Destroy all enchantments.",
                "• Destroy all creatures with mana value 3 or less.",
                "• Destroy all creatures with mana value 4 or greater.",
            ],
            |d, _| {
                // The two modes chosen, as one mode for each pair (CR 700.2).
                let parts = [(true, false, (0, 0)), (false, true, (0, 0)), (false, false, (0, 3)), (false, false, (4, u8::MAX))];
                let mut pairs = Vec::new();
                for i in 0..4 {
                    for j in i + 1..4 {
                        let (a, b) = (parts[i], parts[j]);
                        pairs.push(Ability::new(
                            vec![],
                            vec![
                                Effect::DestroyAllMatching { artifacts: a.0, enchantments: a.1, creature_mv: a.2 },
                                Effect::DestroyAllMatching { artifacts: b.0, enchantments: b.1, creature_mv: b.2 },
                            ],
                        ));
                    }
                }
                modes(d, pairs);
            },
        ),
        "Dawn's Truce" => s(
            &[
                "Gift a card",
                "You and permanents you control gain hexproof until end of turn. If the gift was promised, permanents you control also gain indestructible until end of turn.",
            ],
            |d, _| {
                // Gift: an optional free additional cost, cast with `Action::CastKicked`.
                d.kicker = ManaCost::parse("{0}");
                spell(d, vec![], vec![Effect::YouGainHexproofEot, Effect::GrantAllPermanentsYouControl(Keywords::HEXPROOF)]);
                d.kicked_spell = Some(Ability::new(
                    vec![],
                    vec![
                        Effect::Draw { who: Who::EachOpponent, n: 1 },
                        Effect::YouGainHexproofEot,
                        Effect::GrantAllPermanentsYouControl(Keywords::HEXPROOF.with(Keywords::INDESTRUCTIBLE)),
                    ],
                ));
            },
        ),
        "Eldritch Evolution" => s(
            &[
                "As an additional cost to cast this spell, sacrifice a creature.",
                "Search your library for a creature card with mana value X or less, where X is 2 plus the sacrificed creature's mana value. Put that card onto the battlefield, then shuffle. Exile Eldritch Evolution.",
            ],
            |d, _| {
                d.extra_cost = Some(ExtraCost::SacrificeCreature);
                spell(d, vec![], vec![Effect::SearchCreatureBySacrificedMv { plus: 2, exact: false }, Effect::ExileSourceSpell]);
            },
        ),
        "Birthing Pod" => s(
            &["{1}{G/P}, {T}, Sacrifice a creature: Search your library for a creature card with mana value equal to 1 plus the sacrificed creature's mana value, put that card onto the battlefield, then shuffle. Activate only as a sorcery."],
            |d, _| {
                let n = activated(d, "{1}{G}", true, vec![], vec![Effect::SearchCreatureBySacrificedMv { plus: 1, exact: true }]);
                d.activated[n].phyrexian = [0, 0, 0, 0, 1];
                d.activated[n].sacrifice_creature = true;
                d.activated[n].victim_in_extra = true;
                d.activated[n].sorcery = true;
            },
        ),
        "Enduring Vitality" => s(
            &[
                "Creatures you control have \"{T}: Add one mana of any color.\"",
                "When Enduring Vitality dies, if it was a creature, return it to the battlefield under its owner's control. It's an enchantment.",
            ],
            |d, _| {
                d.creatures_tap_any_color = true;
                on(d, When::Dies, vec![], vec![Effect::ReturnSelfAsEnchantment]);
            },
        ),
        "Jaheira, Friend of the Forest" => s(&["Tokens you control have \"{T}: Add {G}.\"", "Choose a Background"], |d, _| {
            d.tokens_tap_for_green = true; // "Choose a Background" is deck building only
        }),
        "Arachnogenesis" => s(
            &["Create X 1/2 green Spider creature tokens with reach, where X is the number of creatures attacking you. Prevent all combat damage that would be dealt this turn by non-Spider creatures."],
            |d, t| spell(d, vec![], vec![Effect::CreateTokensPerAttackers { def: t.spider }, Effect::PreventCombatDamageExceptSubtype("Spider")]),
        ),
        "For the Common Good" => s(
            &["Create X tokens that are copies of target token you control. Then tokens you control gain indestructible until your next turn. You gain 1 life for each token you control."],
            |d, _| {
                let token = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, tokens_only: true, controller: Controller::You, ..Filter::ANY };
                spell(
                    d,
                    vec![token],
                    vec![Effect::CopyTokenX(0), Effect::GrantTokensUntilYourNextTurn(Keywords::INDESTRUCTIBLE), Effect::GainLifePerToken],
                );
            },
        ),
        "Mirari's Wake" => s(
            &["Creatures you control get +1/+1.", "Whenever you tap a land for mana, add one mana of any type that land produced."],
            |d, _| {
                anthem(d, 1, 1);
                d.lands_add_same = true;
            },
        ),
        "Kaya, Geist Hunter" => s(
            &[
                "+1: Creatures you control gain deathtouch until end of turn. Put a +1/+1 counter on up to one target creature token you control.",
                "−2: Until end of turn, if one or more tokens would be created under your control, twice that many of those tokens are created instead.",
                "−6: Exile all cards from all graveyards, then create a 1/1 white Spirit creature token with flying for each card exiled this way.",
            ],
            |d, t| {
                let token = Filter { tokens_only: true, ..YOUR_CREATURE };
                loyalty(d, 1, up_to(vec![token], vec![Effect::GrantAll { who: Who::You, keywords: Keywords::DEATHTOUCH }, Effect::Counters { slot: 0, n: 1 }]));
                loyalty(d, -2, Ability::new(vec![], vec![Effect::DoubleTokensThisTurn]));
                loyalty(d, -6, Ability::new(vec![], vec![Effect::ExileGraveyardsTokenPerCard { def: t.spirit }]));
            },
        ),
        "The Golden Throne" => s(
            &[
                "Arcane Life-support — If you would lose the game, instead exile The Golden Throne and your life total becomes 1.",
                "A Thousand Souls Die Every Day — {T}, Sacrifice a creature: Add three mana in any combination of colors.",
            ],
            |d, _| {
                d.lose_replacement = true;
                // The mana ability uses the stack here (docs/tdm-lane-b.md).
                let n = activated(d, "", true, vec![], vec![Effect::AddManaAnyColor, Effect::AddManaAnyColor, Effect::AddManaAnyColor]);
                d.activated[n].sacrifice_creature = true;
            },
        ),
        "Mondrak, Glory Dominus" => s(
            &[
                "If one or more tokens would be created under your control, twice that many of those tokens are created instead.",
                "{1}{W/P}{W/P}, Sacrifice two other artifacts and/or creatures: Put an indestructible counter on Mondrak.",
            ],
            |d, _| {
                d.doubles_tokens = true;
                let n = activated(d, "{1}{W}{W}", false, vec![], vec![Effect::KeywordCounterSelf(Keywords::INDESTRUCTIBLE)]);
                d.activated[n].phyrexian = [2, 0, 0, 0, 0];
                d.activated[n].sacrifice_two_artifacts_or_creatures = true;
            },
        ),
        "Exalted Sunborn" => s(
            &[
                "If one or more tokens would be created under your control, twice that many of those tokens are created instead.",
                "Warp {1}{W}",
            ],
            |d, _| {
                d.doubles_tokens = true;
                d.warp = ManaCost::parse("{1}{W}");
            },
        ),
        "Grim Haruspex" => s(&["Morph {B}", "Whenever another nontoken creature you control dies, draw a card."], |d, _| {
            d.morph = ManaCost::parse("{B}");
            on(d, When::AnotherNontokenCreatureYouControlDies, vec![], vec![draw(1)]);
        }),
        "Black Market Connections" => s(
            &[
                "At the beginning of your first main phase, choose one or more —",
                "• Sell Contraband — Create a Treasure token. You lose 1 life.",
                "• Buy Information — Draw a card. You lose 2 life.",
                "• Hire a Mercenary — Create a 3/2 colorless Shapeshifter creature token with changeling. You lose 3 life.",
            ],
            |d, t| {
                // "One or more": a mode for each nonempty set of the three (CR 700.2).
                let parts: [Vec<Effect>; 3] = [
                    vec![Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }, Effect::LoseLife { who: Who::You, n: 1 }],
                    vec![draw(1), Effect::LoseLife { who: Who::You, n: 2 }],
                    vec![Effect::CreateToken { who: Who::You, def: t.shapeshifter, n: 1 }, Effect::LoseLife { who: Who::You, n: 3 }],
                ];
                let mut combos = Vec::new();
                for mask in 1..8u8 {
                    let mut effects = Vec::new();
                    for (i, p) in parts.iter().enumerate() {
                        if mask & (1 << i) != 0 {
                            effects.extend(p.iter().copied());
                        }
                    }
                    combos.push(Ability::new(vec![], effects));
                }
                on_modal(d, When::FirstMainPhaseYourTurn, None, combos);
            },
        ),
        // ---- Replacement effects (Mac B core theme) ----
        "Hardened Scales" => s(
            &["If one or more +1/+1 counters would be put on a creature you control, that many plus one +1/+1 counters are put on it instead."],
            |d, _| d.counters_plus_one = true,
        ),
        "Bloodletter of Aclazotz" => s(
            &["If an opponent would lose life during your turn, they lose twice that much life instead."],
            |d, _| d.opponents_lose_double_your_turn = true,
        ),
        "Alhammarret's Archive" => s(
            &[
                "If you would gain life, you gain twice that much life instead.",
                "If you would draw a card except the first one you draw in each of your draw steps, draw two cards instead.",
            ],
            |d, _| {
                d.life_gain_doubled = true;
                d.extra_draws_doubled = true;
            },
        ),
        "Teferi's Ageless Insight" => s(
            &["If you would draw a card except the first one you draw in each of your draw steps, draw two cards instead."],
            |d, _| d.extra_draws_doubled = true,
        ),
        "Quina, Qu Gourmet" => s(
            &[
                "If one or more tokens would be created under your control, those tokens plus a 1/1 green Frog creature token are created instead.",
                "{2}, Sacrifice a Frog: Put a +1/+1 counter on Quina.",
            ],
            |d, t| {
                d.tokens_plus_one = Some(t.frog);
                let n = activated(d, "{2}", false, vec![], vec![Effect::CountersSelf { n: 1 }]);
                d.activated[n].sacrifice_subtype = Some("Frog");
            },
        ),
        "Force of Negation" => s(
            &[
                "If it's not your turn, you may exile a blue card from your hand rather than pay this spell's mana cost.",
                "Counter target noncreature spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard.",
            ],
            |d, _| {
                d.pitch_not_your_turn = Some(1); // blue
                spell(d, vec![Filter { spell_creature: Some(false), ..SPELL }], vec![Effect::CounterExile(0)]);
            },
        ),
        "Deepglow Skate" => s(
            &["When this creature enters, double the number of each kind of counter on any number of target permanents."],
            |d, _| {
                let any = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, ..Filter::ANY };
                let effects = (0..10).map(Effect::DoubleEachCounter).collect();
                d.triggers.push(Trigger { when: When::Enters, ability: up_to(vec![any; 10], effects) });
            },
        ),
        "Bogardan Dragonheart" => s(
            &["Sacrifice another creature: Until end of turn, this creature becomes a Dragon with base power and toughness 4/4, flying, and haste."],
            |d, _| {
                let dragon = Effect::BecomeSubtypeUntilEot { subtype: "Dragon", power: 4, toughness: 4, keywords: Keywords::FLYING.with(Keywords::HASTE) };
                let n = activated(d, "", false, vec![], vec![dragon]);
                d.activated[n].sacrifice_other_creature = true;
            },
        ),
        "Whip of Erebos" => s(
            &[
                "Creatures you control have lifelink.",
                "{2}{B}{B}, {T}: Return target creature card from your graveyard to the battlefield. It gains haste. Exile it at the beginning of the next end step. If it would leave the battlefield, exile it instead of putting it anywhere else. Activate only as a sorcery.",
            ],
            |d, _| {
                static_on(d, Scope::YourCreatures, false, Keywords::LIFELINK);
                let n = activated(d, "{2}{B}{B}", true, vec![graveyard_cards(true, Controller::You)], vec![Effect::ReanimateUnearthed(0)]);
                d.activated[n].sorcery = true;
            },
        ),
        "Words of War" => s(
            &["{1}: The next time you would draw a card this turn, this enchantment deals 2 damage to any target instead."],
            |d, _| {
                // The target is chosen as it's activated (docs/tdm-lane-b.md).
                activated(d, "{1}", false, vec![Filter::ANY], vec![Effect::NextDrawDealsDamage { slot: 0, n: 2 }]);
            },
        ),
        // ---- The merger's three: type-changing statics, haste for abilities ----
        "Blood Moon" => s(&["Nonbasic lands are Mountains."], |d, _| d.nonbasic_lands_are_mountains = true),
        "Urborg, Tomb of Yawgmoth" => s(&["Each land is a Swamp in addition to its other land types."], |d, _| d.lands_are_swamps = true),
        "Thousand-Year Elixir" => s(
            &["You may activate abilities of creatures you control as though those creatures had haste.", "{1}, {T}: Untap target creature."],
            |d, _| {
                d.creatures_abilities_haste = true;
                activated(d, "{1}", true, vec![CREATURE], vec![Effect::UntapTarget(0)]);
            },
        ),
        // ---- Merger-delegated: draw step, untap step, cast legality, hand size ----
        "Damia, Sage of Stone" => s(
            &[
                "Skip your draw step.",
                "At the beginning of your upkeep, if you have fewer than seven cards in hand, draw cards equal to the difference.",
            ],
            |d, _| {
                d.skip_draw_step = true;
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::DrawUpToHandSize(7)]);
            },
        ),
        "Drumbellower" => s(&["Untap all creatures you control during each other player's untap step."], |d, _| {
            d.creatures_untap_on_others_untap = true;
        }),
        "Savor the Moment" => s(&["Take an extra turn after this one. Skip the untap step of that turn."], |d, _| {
            spell(d, vec![], vec![Effect::ExtraTurnSkipUntap]);
        }),
        "Gaddock Teeg" => s(
            &[
                "Noncreature spells with mana value 4 or greater can't be cast.",
                "Noncreature spells with {X} in their mana costs can't be cast.",
            ],
            |d, _| d.teeg_lock = true,
        ),
        "Trusted Advisor" => s(
            &[
                "Your maximum hand size is increased by two.",
                "At the beginning of your upkeep, return a blue creature you control to its owner's hand.",
            ],
            |d, _| {
                d.max_hand_plus = 2;
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::ReturnCreatureOfColorYouControl(1)]);
            },
        ),
        "Jin-Gitaxias, Core Augur" => s(
            &["At the beginning of your end step, draw seven cards.", "Each opponent's maximum hand size is reduced by seven."],
            |d, _| {
                on(d, When::EndStep { your_turn_only: true }, vec![], vec![draw(7)]);
                d.opponents_max_hand_minus = 7;
            },
        ),
        // ---- Tatyova, Benthic Druid (Grande), lane B ----
        "Migration Path" => s(
            &["Search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle.", "Cycling {2}"],
            |d, _| {
                let two = Effect::Search { what: SearchFor::BasicLand, to: SearchTo::Battlefield { tapped: true }, n: 2, kicked_n: None };
                spell(d, vec![], vec![two]);
                let n = activated(d, "{2}", false, vec![], vec![draw(1)]);
                d.activated[n].from_hand_discard = true;
            },
        ),
        "Joint Exploration" => s(
            &["Kicker {G}", "Scry 2, then draw a card. If this spell was kicked, you may put a land card from your hand onto the battlefield."],
            |d, _| {
                d.kicker = ManaCost::parse("{G}");
                spell(d, vec![], vec![Effect::Scry { n: 2 }, draw(1)]);
                d.kicked_spell = Some(Ability::new(vec![], vec![Effect::Scry { n: 2 }, draw(1), Effect::MayPutLandFromHand]));
            },
        ),
        "Khalni Heart Expedition" => s(
            &[
                "Landfall — Whenever a land you control enters, you may put a quest counter on this enchantment.",
                "Remove three quest counters from this enchantment and sacrifice it: Search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle.",
            ],
            |d, _| {
                // Quest counters are this card's own kind of counter (`Object::kind_counters`).
                may(d, When::LandYouControlEnters, vec![], vec![Effect::KindCounterSelf(1)]);
                let two = Effect::Search { what: SearchFor::BasicLand, to: SearchTo::Battlefield { tapped: true }, n: 2, kicked_n: None };
                let n = activated(d, "", false, vec![], vec![two]);
                d.activated[n].remove_kind_counters = 3;
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Noxious Revival" => s(&["Put target card from a graveyard on top of its owner's library."], |d, _| {
            spell(d, vec![graveyard_cards(false, Controller::Any)], vec![Effect::ToLibrary(0, true)]);
        }),
        "On the Trail" => s(
            &["Whenever you draw your second card each turn, you may put a land card from your hand onto the battlefield tapped."],
            |d, _| on(d, When::YouDrawSecond, vec![], vec![Effect::MayPutLandFromHandTapped]),
        ),
        "Rude Awakening" => s(
            &[
                "Choose one —",
                "• Untap all lands you control.",
                "• Until end of turn, lands you control become 2/2 creatures that are still lands.",
                "Entwine {2}{G}",
            ],
            |d, _| {
                let animate = Effect::LandsBecomeCreaturesEot { power: 2, toughness: 2 };
                modes(
                    d,
                    vec![
                        Ability::new(vec![], vec![Effect::UntapLandsYouControl]),
                        Ability::new(vec![], vec![animate]),
                        Ability::new(vec![], vec![Effect::UntapLandsYouControl, animate]), // entwined
                    ],
                );
                d.entwine = ManaCost::parse("{2}{G}");
            },
        ),
        "Sakura-Tribe Scout" => s(&["{T}: You may put a land card from your hand onto the battlefield."], |d, _| {
            activated(d, "", true, vec![], vec![Effect::MayPutLandFromHand]);
        }),
        "Skyshroud Ranger" => s(&["{T}: You may put a land card from your hand onto the battlefield. Activate only as a sorcery."], |d, _| {
            let n = activated(d, "", true, vec![], vec![Effect::MayPutLandFromHand]);
            d.activated[n].sorcery = true;
        }),
        "Splendid Reclamation" => s(&["Return all land cards from your graveyard to the battlefield tapped."], |d, _| {
            spell(d, vec![], vec![Effect::ReturnLandsFromGraveyardTapped]);
        }),
        "Squall Line" => s(&["Squall Line deals X damage to each creature with flying and each player."], |d, _| {
            spell(d, vec![], vec![Effect::DamageX { to: Recipient::FliersAndEachPlayer }]);
        }),
        "Stroke of Genius" => s(&["Target player draws X cards."], |d, _| spell(d, vec![Filter::PLAYER], vec![Effect::DrawXTarget(0)])),
        "Wipe Away" => s(&["Split second", "Return target permanent to its owner's hand."], |d, _| {
            d.split_second = true;
            let any = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, ..Filter::ANY };
            spell(d, vec![any], vec![Effect::Bounce(0)]);
        }),
        "Laboratory Maniac" => s(
            &["If you would draw a card while your library has no cards in it, you win the game instead."],
            |d, _| d.win_on_empty_draw = true,
        ),
        // ---- Zhulodok, Void Gorger (Adam), lane B ----
        "Expedition Map" => s(
            &["{2}, {T}, Sacrifice this artifact: Search your library for a land card, reveal it, put it into your hand, then shuffle."],
            |d, _| {
                let n = activated(d, "{2}", true, vec![], vec![search(SearchFor::Land, SearchTo::Hand)]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Strip Mine" => s(&["{T}: Add {C}.", "{T}, Sacrifice this land: Destroy target land."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            let land = Filter { players: false, creatures: false, planeswalkers: false, lands: true, ..Filter::ANY };
            let n = activated(d, "", true, vec![land], vec![Effect::Destroy(0)]);
            d.activated[n].sacrifice_self = true;
        }),
        "Wasteland" => s(&["{T}: Add {C}.", "{T}, Sacrifice this land: Destroy target nonbasic land."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            let land = Filter { players: false, creatures: false, planeswalkers: false, lands: true, nonbasic: true, ..Filter::ANY };
            let n = activated(d, "", true, vec![land], vec![Effect::Destroy(0)]);
            d.activated[n].sacrifice_self = true;
        }),
        "Palladium Myr" => s(&["{T}: Add {C}{C}."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            d.mana_amount = 2;
        }),
        "Worn Powerstone" => s(&["This artifact enters tapped.", "{T}: Add {C}{C}."], |d, _| {
            d.enters_tapped = true;
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            d.mana_amount = 2;
        }),
        "Voltaic Key" => s(&["{1}, {T}: Untap target artifact."], |d, _| {
            let artifact = Filter { players: false, creatures: false, planeswalkers: false, artifacts: true, ..Filter::ANY };
            activated(d, "{1}", true, vec![artifact], vec![Effect::UntapTarget(0)]);
        }),
        "Manifold Key" => s(&["{1}, {T}: Untap another target artifact.", "{3}, {T}: Target creature can't be blocked this turn."], |d, _| {
            let artifact = Filter { players: false, creatures: false, planeswalkers: false, artifacts: true, other: true, ..Filter::ANY };
            activated(d, "{1}", true, vec![artifact], vec![Effect::UntapTarget(0)]);
            activated(d, "{3}", true, vec![CREATURE], vec![Effect::Grant { slot: 0, keywords: Keywords::UNBLOCKABLE }]);
        }),
        "Urza's Cave" => s(
            &["{T}: Add {C}.", "{3}, {T}, Sacrifice this land: Search your library for a land card, put it onto the battlefield tapped, then shuffle."],
            |d, _| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                let n = activated(d, "{3}", true, vec![], vec![search(SearchFor::Land, SearchTo::Battlefield { tapped: true })]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Mithril Coat" => s(
            &["When Mithril Coat enters, attach it to target legendary creature you control.", "Equipped creature has indestructible.", "Equip {3}"],
            |d, _| {
                let legend = Filter { legendary_only: true, ..YOUR_CREATURE };
                on(d, When::Enters, vec![legend], vec![Effect::Attach { slot: 0 }]);
                attached(d, false, 0, 0, Keywords::INDESTRUCTIBLE);
                equip(d, "{3}");
            },
        ),
        "Artisan of Kozilek" => s(
            &["When you cast this spell, you may return target creature card from your graveyard to the battlefield.", "Annihilator 2"],
            |d, _| {
                may(d, When::CastThisSpell, vec![graveyard_cards(true, Controller::You)], vec![Effect::Reanimate(0)]);
                on(d, When::Attacks, vec![], vec![Effect::SacrificePermanents { who: Who::EachOpponent, n: 2 }]);
            },
        ),
        "Fountainport" => s(
            &[
                "{T}: Add {C}.",
                "{2}, {T}, Sacrifice a token: Draw a card.",
                "{3}, {T}, Pay 1 life: Create a 1/1 blue Fish creature token.",
                "{4}, {T}: Create a Treasure token.",
            ],
            |d, t| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                let n = activated(d, "{2}", true, vec![], vec![draw(1)]);
                d.activated[n].sacrifice_token = true;
                let n = activated(d, "{3}", true, vec![], vec![Effect::CreateToken { who: Who::You, def: t.fish, n: 1 }]);
                d.activated[n].life = 1;
                activated(d, "{4}", true, vec![], vec![Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }]);
            },
        ),
        // ---- Oloro, Ageless Ascetic (Adam), lane B ----
        "Sea of Clouds" => s(&["This land enters tapped unless you have two or more opponents.", "{T}: Add {W} or {U}."], |d, _| {
            d.enters_tapped = true; // a two-player game: one opponent
            d.mana = Some(ManaSource::of(&[Color::White, Color::Blue]));
        }),
        "Archangel of Thune" => s(&["Whenever you gain life, put a +1/+1 counter on each creature you control."], |d, _| {
            on(d, When::YouGainLife, vec![], vec![Effect::CountersAll { who: Who::You, n: 1 }]);
        }),
        "Cliffhaven Vampire" => s(&["Whenever you gain life, each opponent loses 1 life."], |d, _| {
            on(d, When::YouGainLife, vec![], vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }]);
        }),
        "Drogskol Reaver" => s(&["Whenever you gain life, draw a card."], |d, _| on(d, When::YouGainLife, vec![], vec![draw(1)])),
        "Rhox Faithmender" => s(&["If you would gain life, you gain twice that much life instead."], |d, _| d.life_gain_doubled = true),
        "Serra Ascendant" => s(&["As long as you have 30 or more life, this creature gets +5/+5 and has flying."], |d, _| {
            this_if(d, StaticCondition::LifeAtLeast(30), 5, 5, Keywords::FLYING);
        }),
        "Divinity of Pride" => s(&["This creature gets +4/+4 as long as you have 25 or more life."], |d, _| {
            this_if(d, StaticCondition::LifeAtLeast(25), 4, 4, Keywords::NONE);
        }),
        "Voice of the Blessed" => s(
            &[
                "Whenever you gain life, put a +1/+1 counter on this creature.",
                "As long as this creature has four or more +1/+1 counters on it, it has flying and vigilance.",
                "As long as this creature has ten or more +1/+1 counters on it, it has indestructible.",
            ],
            |d, _| {
                on(d, When::YouGainLife, vec![], vec![Effect::CountersSelf { n: 1 }]);
                this_if(d, StaticCondition::CountersAtLeast(4), 0, 0, Keywords::FLYING.with(Keywords::VIGILANCE));
                this_if(d, StaticCondition::CountersAtLeast(10), 0, 0, Keywords::INDESTRUCTIBLE);
            },
        ),
        "Heliod, Sun-Crowned" => s(
            &[
                "As long as your devotion to white is less than five, Heliod isn't a creature.",
                "Whenever you gain life, put a +1/+1 counter on target creature or enchantment you control.",
                "{1}{W}: Another target creature gains lifelink until end of turn.",
            ],
            |d, _| {
                d.creature_unless_devotion = Some(([true, false, false, false, false], 5));
                let yours = Filter { players: false, planeswalkers: false, enchantments: true, controller: Controller::You, ..Filter::CREATURE };
                on(d, When::YouGainLife, vec![yours], vec![Effect::Counters { slot: 0, n: 1 }]);
                let other = Filter { other: true, ..CREATURE };
                activated(d, "{1}{W}", false, vec![other], vec![Effect::Grant { slot: 0, keywords: Keywords::LIFELINK }]);
            },
        ),
        // ---- Ghyrson Starn, Kelermorph (Spr3adsh33t), lane B ----
        "Basilisk Collar" => s(&["Equipped creature has deathtouch and lifelink.", "Equip {2}"], |d, _| {
            attached(d, false, 0, 0, Keywords::DEATHTOUCH.with(Keywords::LIFELINK));
            equip(d, "{2}");
        }),
        "Gorgon's Head" => s(&["Equipped creature has deathtouch.", "Equip {2}"], |d, _| {
            attached(d, false, 0, 0, Keywords::DEATHTOUCH);
            equip(d, "{2}");
        }),
        "Electrostatic Field" => s(&["Whenever you cast an instant or sorcery spell, this creature deals 1 damage to each opponent."], |d, _| {
            on(d, When::YouCastInstantOrSorcery, vec![], vec![damage(1, Recipient::EachOpponent)]);
        }),
        "Erebor Flamesmith" => s(&["Whenever you cast an instant or sorcery spell, this creature deals 1 damage to each opponent."], |d, _| {
            on(d, When::YouCastInstantOrSorcery, vec![], vec![damage(1, Recipient::EachOpponent)]);
        }),
        "Kessig Flamebreather" => s(&["Whenever you cast a noncreature spell, this creature deals 1 damage to each opponent."], |d, _| {
            on(d, When::YouCastNoncreature, vec![], vec![damage(1, Recipient::EachOpponent)]);
        }),
        "Grapeshot" => s(&["Grapeshot deals 1 damage to any target.", "Storm"], |d, _| {
            // The storm copies keep the original's target (choosing new ones isn't offered).
            spell(d, vec![Filter::ANY], vec![damage(1, Recipient::Target(0))]);
            on(d, When::YouCastThis, vec![], vec![Effect::CopySourceSpell { storm: true }]);
        }),
        "The Surgical Bay" => s(&["This land enters tapped.", "{T}: Add {U}.", "{1}{U}, {T}, Sacrifice this land: Draw a card."], |d, _| {
            d.enters_tapped = true;
            d.mana = Some(ManaSource::of(&[Color::Blue]));
            let n = activated(d, "{1}{U}", true, vec![], vec![draw(1)]);
            d.activated[n].sacrifice_self = true;
        }),
        "The Autonomous Furnace" => s(&["This land enters tapped.", "{T}: Add {R}.", "{1}{R}, {T}, Sacrifice this land: Draw a card."], |d, _| {
            d.enters_tapped = true;
            d.mana = Some(ManaSource::of(&[Color::Red]));
            let n = activated(d, "{1}{R}", true, vec![], vec![draw(1)]);
            d.activated[n].sacrifice_self = true;
        }),
        "Training Center" => s(&["This land enters tapped unless you have two or more opponents.", "{T}: Add {U} or {R}."], |d, _| {
            d.enters_tapped = true; // a two-player game: one opponent
            d.mana = Some(ManaSource::of(&[Color::Blue, Color::Red]));
        }),
        "Blisterspit Gremlin" => s(
            &["{1}, {T}: This creature deals 1 damage to each opponent.", "Whenever you cast a noncreature spell, untap this creature."],
            |d, _| {
                activated(d, "{1}", true, vec![], vec![damage(1, Recipient::EachOpponent)]);
                on(d, When::YouCastNoncreature, vec![], vec![Effect::UntapSelf]);
            },
        ),
        "Thermo-Alchemist" => s(
            &["{T}: This creature deals 1 damage to each opponent.", "Whenever you cast an instant or sorcery spell, untap this creature."],
            |d, _| {
                activated(d, "", true, vec![], vec![damage(1, Recipient::EachOpponent)]);
                on(d, When::YouCastInstantOrSorcery, vec![], vec![Effect::UntapSelf]);
            },
        ),
        "Unruly Catapult" => s(
            &["{T}: This creature deals 1 damage to each opponent.", "Whenever you cast an instant or sorcery spell, untap this creature."],
            |d, _| {
                activated(d, "", true, vec![], vec![damage(1, Recipient::EachOpponent)]);
                on(d, When::YouCastInstantOrSorcery, vec![], vec![Effect::UntapSelf]);
            },
        ),
        "Gelectrode" => s(
            &["{T}: This creature deals 1 damage to any target.", "Whenever you cast an instant or sorcery spell, you may untap this creature."],
            |d, _| {
                activated(d, "", true, vec![Filter::ANY], vec![damage(1, Recipient::Target(0))]);
                may(d, When::YouCastInstantOrSorcery, vec![], vec![Effect::UntapSelf]);
            },
        ),
        "Goblin Sharpshooter" => s(
            &[
                "This creature doesn't untap during your untap step.",
                "Whenever a creature dies, untap this creature.",
                "{T}: This creature deals 1 damage to any target.",
            ],
            |d, _| {
                d.doesnt_untap = true;
                on(d, When::AnotherCreatureDies, vec![], vec![Effect::UntapSelf]);
                activated(d, "", true, vec![Filter::ANY], vec![damage(1, Recipient::Target(0))]);
            },
        ),
        "Cast into the Fire" => s(
            &[
                "Choose one —",
                "• Cast into the Fire deals 1 damage to each of up to two target creatures.",
                "• Exile target artifact.",
            ],
            |d, _| {
                let artifact = Filter { players: false, creatures: false, planeswalkers: false, artifacts: true, ..Filter::ANY };
                modes(
                    d,
                    vec![
                        up_to(vec![CREATURE, CREATURE], vec![damage(1, Recipient::Target(0)), damage(1, Recipient::Target(1))]),
                        Ability::new(vec![artifact], vec![Effect::Exile(0)]),
                    ],
                );
            },
        ),
        "Consider" => s(&["Surveil 1.", "Draw a card."], |d, _| spell(d, vec![], vec![Effect::Surveil { n: 1 }, draw(1)])),
        "Serum Visions" => s(&["Draw a card. Scry 2."], |d, _| spell(d, vec![], vec![draw(1), Effect::Scry { n: 2 }])),
        "Electrolyze" => s(&["Electrolyze deals 2 damage divided as you choose among one or two targets.", "Draw a card."], |d, _| {
            let mut a = Ability::new(vec![Filter::ANY, Filter::ANY], vec![Effect::DividedDamage, draw(1)]);
            a.min_targets = Some(1);
            a.divide = 2;
            d.spell = Some(a);
        }),
        "Niv-Mizzet, Parun" => s(
            &[
                "This spell can't be countered.",
                "Whenever you draw a card, Niv-Mizzet deals 1 damage to any target.",
                "Whenever a player casts an instant or sorcery spell, you draw a card.",
            ],
            |d, _| {
                d.uncounterable = true;
                on(d, When::YouDraw, vec![Filter::ANY], vec![damage(1, Recipient::Target(0))]);
                on(d, When::YouCastInstantOrSorcery, vec![], vec![draw(1)]);
                on(d, When::OpponentCastsInstantOrSorcery, vec![], vec![draw(1)]);
            },
        ),
        "Paradox Haze" => s(
            &[
                "Enchant player",
                "At the beginning of enchanted player's first upkeep each turn, that player gets an additional upkeep step after this step.",
            ],
            |d, _| {
                d.aura = true;
                d.extra_upkeep_for_enchanted = true;
                spell(d, vec![Filter::PLAYER], vec![]);
            },
        ),
        "Dawn of Hope" => s(
            &["Whenever you gain life, you may pay {2}. If you do, draw a card.", "{3}{W}: Create a 1/1 white Soldier creature token with lifelink."],
            |d, t| {
                on(d, When::YouGainLife, vec![], vec![Effect::MayPay(ManaCost::parse("{2}").expect("cost")), draw(1)]);
                activated(d, "{3}{W}", false, vec![], vec![Effect::CreateToken { who: Who::You, def: t.soldier_lifelink, n: 1 }]);
            },
        ),
        "Murmuring Mystic" => s(
            &["Whenever you cast an instant or sorcery spell, create a 1/1 blue Bird Illusion creature token with flying."],
            |d, t| on(d, When::YouCastInstantOrSorcery, vec![], vec![Effect::CreateToken { who: Who::You, def: t.bird_illusion, n: 1 }]),
        ),
        "Third Path Iconoclast" => s(
            &["Whenever you cast a noncreature spell, create a 1/1 colorless Soldier artifact creature token."],
            |d, t| on(d, When::YouCastNoncreature, vec![], vec![Effect::CreateToken { who: Who::You, def: t.soldier_artifact, n: 1 }]),
        ),
        "Storm-Kiln Artist" => s(
            &[
                "This creature gets +1/+0 for each artifact you control.",
                "Magecraft — Whenever you cast or copy an instant or sorcery spell, create a Treasure token.",
            ],
            |d, t| {
                d.statics.push(Static {
                    subtype: None,
                    lose_abilities: false,
                    base_pt: None,
                    protection: [false; 5],
                    scale: Some(Scale::PerArtifactYouControl),
                    scope: Scope::This,
                    your_turn_only: false,
                    condition: None,
                    power: 1,
                    toughness: 0,
                    keywords: Keywords::NONE,
                    set_colors: None,
                    becomes_land: false,
                });
                on(d, When::YouCastOrCopyInstantOrSorcery, vec![], vec![Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }]);
            },
        ),
        "Misdirection" => s(
            &["You may exile a blue card from your hand rather than pay this spell's mana cost.", "Change the target of target spell with a single target."],
            |d, _| {
                d.mana_ext.pitch_with_life = Some((1, 0)); // blue, no life
                // "Change the target": offered as ChooseNewTarget, where keeping it is also allowed.
                spell(d, vec![Filter { single_target: true, ..SPELL }], vec![Effect::ChangeTargets(0)]);
            },
        ),
        "Young Pyromancer" => s(&["Whenever you cast an instant or sorcery spell, create a 1/1 red Elemental creature token."], |d, t| {
            on(d, When::YouCastInstantOrSorcery, vec![], vec![Effect::CreateToken { who: Who::You, def: t.elemental_1, n: 1 }]);
        }),
        "Monument to Endurance" => s(
            &[
                "Whenever you discard a card, choose one that hasn't been chosen this turn —",
                "• Draw a card.",
                "• Create a Treasure token.",
                "• Each opponent loses 3 life.",
            ],
            |d, t| {
                let modes = vec![
                    Ability::new(vec![], vec![draw(1)]),
                    Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }]),
                    Ability::new(vec![], vec![Effect::LoseLife { who: Who::EachOpponent, n: 3 }]),
                ];
                on_modal(d, When::YouDiscard, None, modes);
                d.modes_once_each_turn = true;
            },
        ),
        "Glimpse the Impossible" => s(
            &["Exile the top three cards of your library. You may play those cards this turn. At the beginning of the next end step, if any of those cards remain exiled, put them into your graveyard, then create a 0/1 colorless Eldrazi Spawn creature token for each card put into your graveyard this way. Those tokens have \"Sacrifice this token: Add {C}.\""],
            |d, t| spell(d, vec![], vec![Effect::ExileTopToPlayThenGraveyardTokens { n: 3, def: t.eldrazi_spawn }]),
        ),
        "Barbarian Ring" => s(
            &[
                "{T}: Add {R}. This land deals 1 damage to you.",
                "Threshold — {R}, {T}, Sacrifice this land: It deals 2 damage to any target. Activate only if there are seven or more cards in your graveyard.",
            ],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::Red]));
                d.pain_colored = true;
                let n = activated(d, "{R}", true, vec![Filter::ANY], vec![damage(2, Recipient::Target(0))]);
                d.activated[n].sacrifice_self = true;
                d.activated[n].needs_threshold = true;
            },
        ),
        "Cephalid Coliseum" => s(
            &[
                "{T}: Add {U}. This land deals 1 damage to you.",
                "Threshold — {U}, {T}, Sacrifice this land: Target player draws three cards, then discards three cards. Activate only if there are seven or more cards in your graveyard.",
            ],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::Blue]));
                d.pain_colored = true;
                let effects = vec![Effect::Draw { who: Who::TargetPlayer(0), n: 3 }, Effect::Discard { who: Who::TargetPlayer(0), n: 3 }];
                let n = activated(d, "{U}", true, vec![Filter::PLAYER], effects);
                d.activated[n].sacrifice_self = true;
                d.activated[n].needs_threshold = true;
            },
        ),
        "Cabal Pit" => s(
            &[
                "{T}: Add {B}. This land deals 1 damage to you.",
                "Threshold — {B}, {T}, Sacrifice this land: Target creature gets -2/-2 until end of turn. Activate only if there are seven or more cards in your graveyard.",
            ],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::Black]));
                d.pain_colored = true;
                let n = activated(d, "{B}", true, vec![CREATURE], vec![Effect::Pump { slot: 0, power: -2, toughness: -2 }]);
                d.activated[n].sacrifice_self = true;
                d.activated[n].needs_threshold = true;
            },
        ),
        "Kruphix, God of Horizons" => s(
            &[
                "As long as your devotion to green and blue is less than seven, Kruphix isn't a creature.",
                "You have no maximum hand size.",
                "If you would lose unspent mana, that mana becomes colorless instead.",
            ],
            |d, _| {
                d.creature_unless_devotion = Some(([false, true, false, false, true], 7));
                d.no_max_hand_size = true;
                d.unspent_mana_becomes_colorless = true;
            },
        ),
        "Horizon Stone" => s(&["If you would lose unspent mana, that mana becomes colorless instead."], |d, _| {
            d.unspent_mana_becomes_colorless = true;
        }),
        "Necrologia" => s(
            &["Cast this spell only during your end step.", "As an additional cost to cast this spell, pay X life.", "Draw X cards."],
            |d, _| {
                d.cast_only_during_your_end_step = true;
                d.mana_ext.pay_x_life = true;
                spell(d, vec![], vec![Effect::DrawX]);
            },
        ),
        "Invert Polarity" => s(
            &["Choose target spell, then flip a coin. If you win the flip, gain control of that spell and you may choose new targets for it. If you lose the flip, counter that spell."],
            |d, _| spell(d, vec![SPELL], vec![Effect::FlipGainControlOfSpellOrCounter(0)]),
        ),
        "Ulamog, the Ceaseless Hunger" => s(
            &["When you cast this spell, exile two target permanents.", "Whenever Ulamog attacks, defending player exiles the top twenty cards of their library."],
            |d, _| {
                let perm = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, ..Filter::ANY };
                on(d, When::CastThisSpell, vec![perm, perm], vec![Effect::Exile(0), Effect::Exile(1)]);
                on(d, When::Attacks, vec![], vec![Effect::DefendingPlayerExilesTop(20)]);
            },
        ),
        "Void Winnower" => s(
            &["Your opponents can't cast spells with even mana values.", "Your opponents can't block with creatures with even mana values."],
            |d, _| d.opponents_even_locked = true,
        ),
        "Epic Experiment" => s(
            &["Exile the top X cards of your library. You may cast instant and sorcery spells with mana value X or less from among them without paying their mana costs. Then put all cards exiled this way that weren't cast into your graveyard."],
            |d, _| spell(d, vec![], vec![Effect::EpicExperiment]),
        ),
        "Metallurgic Summonings" => s(
            &[
                "Whenever you cast an instant or sorcery spell, create an X/X colorless Construct artifact creature token, where X is that spell's mana value.",
                "{3}{U}{U}, Exile this enchantment: Return all instant and sorcery cards from your graveyard to your hand. Activate only if you control six or more artifacts.",
            ],
            |d, t| {
                on(d, When::YouCastInstantOrSorcery, vec![], vec![Effect::CreateTokenXxTriggeringSpellManaValue { def: t.construct_x }]);
                let n = activated(d, "{3}{U}{U}", false, vec![], vec![Effect::ReturnAllInstantsAndSorceriesToHand]);
                d.activated[n].exile_self = true;
                d.activated[n].min_artifacts = 6;
            },
        ),
        "Ugin, the Spirit Dragon" => s(
            &[
                "+2: Ugin deals 3 damage to any target.",
                "\u{2212}X: Exile each permanent with mana value X or less that's one or more colors.",
                "\u{2212}10: You gain 7 life, draw seven cards, then put up to seven permanent cards from your hand onto the battlefield.",
            ],
            |d, _| {
                loyalty(d, 2, Ability::new(vec![Filter::ANY], vec![damage(3, Recipient::Target(0))]));
                loyalty(d, bolt_core::card::LOYALTY_MINUS_X, Ability::new(vec![], vec![Effect::ExileEachColoredPermanentMvAtMostX]));
                let ten = vec![Effect::GainLife { who: Who::You, n: 7 }, draw(7), Effect::PermanentsFromHandUpTo(7)];
                loyalty(d, -10, Ability::new(vec![], ten));
            },
        ),
        "Bolas's Citadel" => s(
            &[
                "You may look at the top card of your library any time.",
                "You may play lands and cast spells from the top of your library. If you cast a spell this way, pay life equal to its mana value rather than pay its mana cost.",
                "{T}, Sacrifice ten nonland permanents: Each opponent loses 10 life.",
            ],
            |d, _| {
                // Looking at the top card any time is information only.
                d.citadel = true;
                let n = activated(d, "", true, vec![], vec![Effect::LoseLife { who: Who::EachOpponent, n: 10 }]);
                d.activated[n].sacrifice_nonland_n = 10;
            },
        ),
        "Gadrak, the Crown-Scourge" => s(
            &[
                "Gadrak can't attack unless you control four or more artifacts.",
                "At the beginning of your end step, create a Treasure token for each nontoken creature that died this turn.",
            ],
            |d, t| {
                d.attacks_only_with_artifacts = 4;
                on(d, When::EndStep { your_turn_only: true }, vec![], vec![Effect::CreateTokenPerNontokenCreatureDiedThisTurn { def: t.treasure }]);
            },
        ),
        "Xorn" => s(&["If you would create one or more Treasure tokens, instead create those tokens plus an additional Treasure token."], |d, t| {
            d.treasures_plus_one = Some(t.treasure);
        }),
        "Silent Arbiter" => s(&["No more than one creature can attack each combat.", "No more than one creature can block each combat."], |d, _| {
            d.one_attacker_one_blocker = true;
        }),
        "Expropriate" => s(
            &["Council's dilemma — Starting with you, each player votes for time or money. For each time vote, take an extra turn after this one. For each money vote, choose a permanent owned by the voter and gain control of it. Exile Expropriate."],
            |d, _| spell(d, vec![], vec![Effect::Expropriate, Effect::ExileSourceSpell]),
        ),
        "Kozilek's Command" => s(
            &[
                "Choose two —",
                "• Target player creates X 0/1 colorless Eldrazi Spawn creature tokens with \"Sacrifice this token: Add {C}.\"",
                "• Target player scries X, then draws a card.",
                "• Exile target creature with mana value X or less.",
                "• Exile up to X target cards from graveyards.",
            ],
            |d, t| {
                // Each pair of modes is one mode; the "up to X" cards come last (they're optional).
                let creature = Filter { mana_value_at_most_x: true, ..CREATURE };
                let card = Filter { up_to_x: true, ..graveyard_cards(false, Controller::Any) };
                const UP_TO: u8 = 12;
                let part = |i: usize, slot: u8| -> (Vec<Filter>, Vec<Effect>) {
                    match i {
                        0 => (vec![Filter::PLAYER], vec![Effect::TargetPlayerCreatesXTokens { slot, def: t.eldrazi_spawn }]),
                        1 => (vec![Filter::PLAYER], vec![Effect::ScryXTargetPlayer(slot), Effect::Draw { who: Who::TargetPlayer(slot), n: 1 }]),
                        2 => (vec![creature], vec![Effect::Exile(slot)]),
                        _ => (vec![card; UP_TO as usize], (slot..slot + UP_TO).map(Effect::Exile).collect()),
                    }
                };
                let mut pairs = Vec::new();
                for a in 0..4 {
                    for b in a + 1..4 {
                        let (mut targets, mut effects) = part(a, 0);
                        let (t2, e2) = part(b, targets.len() as u8);
                        targets.extend(t2);
                        effects.extend(e2);
                        let required = if b == 3 { targets.len() - UP_TO as usize } else { targets.len() };
                        let mut ability = Ability::new(targets, effects);
                        ability.min_targets = Some(required);
                        pairs.push(ability);
                    }
                }
                modes(d, pairs);
            },
        ),
        "Ugin, the Ineffable" => s(
            &[
                "Colorless spells you cast cost {2} less to cast.",
                "+1: Exile the top card of your library face down and look at it. Create a 2/2 colorless Spirit creature token. When that token leaves the battlefield, put the exiled card into your hand.",
                "\u{2212}3: Destroy target permanent that's one or more colors.",
            ],
            |d, t| {
                d.colorless_spell_discount = 2;
                // The delayed "when that token leaves the battlefield" (see `Game::token_linked_exile`).
                on(d, When::Never, vec![], vec![Effect::LinkedExiledCardToHand]);
                let trigger = d.triggers.len() - 1;
                let plus = Effect::ExileTopFaceDownCreateLinkedToken { def: t.spirit_2, trigger };
                loyalty(d, 1, Ability::new(vec![], vec![plus]));
                let colored = Filter {
                    players: false,
                    creatures: false,
                    planeswalkers: false,
                    any_permanent: true,
                    colored: true,
                    ..Filter::ANY
                };
                loyalty(d, -3, Ability::new(vec![colored], vec![Effect::Destroy(0)]));
            },
        ),
        "Crested Sunmare" => s(
            &[
                "Other Horses you control have indestructible.",
                "At the beginning of each end step, if you gained life this turn, create a 5/5 white Horse creature token.",
            ],
            |d, t| {
                lord(d, "Horse");
                let horses = d.statics.last_mut().expect("the Horse static");
                (horses.power, horses.toughness, horses.keywords) = (0, 0, Keywords::INDESTRUCTIBLE);
                let horse = Effect::CreateToken { who: Who::You, def: t.horse_5, n: 1 };
                let each = When::EndStep { your_turn_only: false };
                conditional(d, each, Condition::YouGainedLifeThisTurn, vec![], vec![horse]);
            },
        ),
        "Ajani, Strength of the Pride" => s(
            &[
                "+1: You gain life equal to the number of creatures you control plus the number of planeswalkers you control.",
                "\u{2212}2: Create a 2/2 white Cat Soldier creature token named Ajani's Pridemate with \"Whenever you gain life, put a +1/+1 counter on this token.\"",
                "0: If you have at least 15 life more than your starting life total, exile Ajani and each artifact and creature your opponents control.",
            ],
            |d, t| {
                let plus = Effect::GainLifePerCreatureAndPlaneswalkerYouControl;
                loyalty(d, 1, Ability::new(vec![], vec![plus]));
                let cat = Effect::CreateToken { who: Who::You, def: t.ajanis_pridemate, n: 1 };
                loyalty(d, -2, Ability::new(vec![], vec![cat]));
                let zero = Effect::ExileSelfAndOpponentsArtifactsCreaturesIfLifeAbove(15);
                loyalty(d, 0, Ability::new(vec![], vec![zero]));
            },
        ),
        "Sheltered by Ghosts" => s(
            &[
                "Enchant creature you control",
                "When this Aura enters, exile target nonland permanent an opponent controls until this Aura leaves the battlefield.",
                "Enchanted creature gets +1/+0 and has lifelink and ward {2}.",
            ],
            |d, _| {
                d.aura = true;
                d.spell = Some(Ability::new(vec![YOUR_CREATURE], vec![]));
                on(d, When::Enters, vec![OPP_NONLAND], vec![Effect::ExileUntilSourceLeaves(0)]);
                attached(d, false, 1, 0, Keywords::LIFELINK);
                d.enchanted_ward = Some(ManaCost::parse("{2}").expect("cost"));
            },
        ),
        "Rankle, Master of Pranks" => s(
            &[
                "Whenever Rankle deals combat damage to a player, choose any number \u{2014}",
                "\u{2022} Each player discards a card.",
                "\u{2022} Each player loses 1 life and draws a card.",
                "\u{2022} Each player sacrifices a creature of their choice.",
            ],
            |d, _| {
                // "Choose any number": every set of the three modes is one mode (bit i: mode i), the
                // empty set last, as Casualties of War.
                let part = |i: usize| -> Vec<Effect> {
                    let both = [Who::You, Who::EachOpponent];
                    match i {
                        0 => both.iter().map(|&who| Effect::Discard { who, n: 1 }).collect(),
                        1 => both
                            .iter()
                            .flat_map(|&who| [Effect::LoseLife { who, n: 1 }, Effect::Draw { who, n: 1 }])
                            .collect(),
                        _ => both.iter().map(|&who| Effect::SacrificeCreatures { who, n: 1 }).collect(),
                    }
                };
                let mut sets: Vec<Ability> = (1u32..8)
                    .map(|m| {
                        let effects = (0..3).filter(|i| m & (1 << i) != 0).flat_map(part).collect();
                        Ability::new(vec![], effects)
                    })
                    .collect();
                sets.push(Ability::new(vec![], vec![]));
                on_modal(d, When::DealsCombatDamageToPlayer, None, sets);
            },
        ),
        "Raphael, Tag Team Tough" => s(
            &["Whenever Raphael deals combat damage to a player for the first time each turn, untap all attacking creatures. After this combat phase, there is an additional combat phase."],
            |d, _| {
                let effects = vec![Effect::UntapAttackersAndGrant(Keywords::NONE), Effect::AdditionalCombatAfterThisPhase];
                on(d, When::DealsCombatDamageToPlayerFirstTimeEachTurn, vec![], effects);
            },
        ),
        "The Immortal Sun" => s(
            &[
                "Players can't activate planeswalkers' loyalty abilities.",
                "At the beginning of your draw step, draw an additional card.",
                "Spells you cast cost {1} less to cast.",
                "Creatures you control get +1/+1.",
            ],
            |d, _| {
                d.players_cant_activate_loyalty = true;
                conditional(d, When::EachDrawStep, Condition::YourTurn, vec![], vec![draw(1)]);
                d.mana_ext.spell_discount_all = 1;
                anthem(d, 1, 1);
            },
        ),
        "Archfiend of Despair" => s(
            &[
                "Your opponents can't gain life.",
                "At the beginning of each end step, each opponent loses life equal to the life that player lost this turn.",
            ],
            |d, _| {
                d.opponents_cant_gain_life = true;
                let lose = Effect::LoseLifeLostThisTurn(Who::EachOpponent);
                on(d, When::EndStep { your_turn_only: false }, vec![], vec![lose]);
            },
        ),
        "Brightclimb Pathway // Grimclimb Pathway" => s(&["{T}: Add {W}.", "Back: {T}: Add {B}."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::White]));
            d.modal_back = true;
            d.back.as_mut().expect("Grimclimb Pathway").mana = Some(ManaSource::of(&[Color::Black]));
        }),
        "Gisela, Blade of Goldnight" => s(
            &[
                "If a source would deal damage to an opponent or a permanent an opponent controls, that source deals double that damage to that player or permanent instead.",
                "If a source would deal damage to you or a permanent you control, prevent half that damage, rounded up.",
            ],
            |d, _| d.prevents_half_damage_to_you_and_yours = true,
        ),
        "Sower of Discord" => s(
            &[
                "As this creature enters, choose two players.",
                "Whenever damage is dealt to one of the chosen players, the other chosen player also loses that much life.",
            ],
            |d, _| {
                // Two players: both are chosen, so damage to either makes the other lose that much.
                let other = Filter { triggering: true, ..Filter::PLAYER };
                on(d, When::Never, vec![other], vec![Effect::LoseLifeAmount(Who::TargetPlayer(0))]);
                d.damage_to_a_player_other_loses = Some(d.triggers.len() - 1);
            },
        ),
        "Eiganjo Castle" => s(
            &["{T}: Add {W}.", "{W}, {T}: Prevent the next 2 damage that would be dealt to target legendary creature this turn."],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::White]));
                let legendary = Filter { legendary_only: true, ..CREATURE };
                let shield = Effect::PreventNextDamageToTargetThisTurn { slot: 0, n: 2 };
                activated(d, "{W}", true, vec![legendary], vec![shield]);
            },
        ),
        "Bennie Bracks, Zoologist" => s(
            &["Convoke", "At the beginning of each end step, if you created a token this turn, draw a card."],
            |d, _| {
                d.mana_ext.convoke = true;
                let each = When::EndStep { your_turn_only: false };
                conditional(d, each, Condition::YouCreatedTokenThisTurn, vec![], vec![draw(1)]);
            },
        ),
        "Inventors' Fair" => s(
            &[
                "At the beginning of your upkeep, if you control three or more artifacts, you gain 1 life.",
                "{T}: Add {C}.",
                "{4}, {T}, Sacrifice Inventors' Fair: Search your library for an artifact card, reveal it, put it into your hand, then shuffle. Activate only if you control three or more artifacts.",
            ],
            |d, _| {
                let gain = Effect::GainLife { who: Who::You, n: 1 };
                let upkeep = When::Upkeep { your_turn_only: true };
                conditional(d, upkeep, Condition::ControlArtifactsAtLeast(3), vec![], vec![gain]);
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                let n = activated(d, "{4}", true, vec![], vec![search(SearchFor::Artifact, SearchTo::Hand)]);
                d.activated[n].sacrifice_self = true;
                d.activated[n].min_artifacts = 3;
            },
        ),
        "Smaug the Magnificent" => s(
            &[
                "Whenever Smaug attacks, he deals damage equal to the number of Treasures you control to any target.",
                "At the beginning of your upkeep, create a Treasure token.",
            ],
            |d, t| {
                let burn = Effect::SourceDamagePerSubtypeYouControl { to: 0, sub: "Treasure" };
                on(d, When::Attacks, vec![Filter::ANY], vec![burn]);
                let treasure = Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 };
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![treasure]);
            },
        ),
        "Mahadi, Emporium Master" => s(
            &["At the beginning of your end step, create a Treasure token for each creature that died this turn."],
            |d, t| {
                let treasures = Effect::CreateTokenPerCreatureDiedThisTurn { def: t.treasure };
                on(d, When::EndStep { your_turn_only: true }, vec![], vec![treasures]);
            },
        ),
        "Goblin Engineer" => s(
            &[
                "When this creature enters, you may search your library for an artifact card, put it into your graveyard, then shuffle.",
                "{R}, {T}, Sacrifice an artifact: Return target artifact card with mana value 3 or less from your graveyard to the battlefield.",
            ],
            |d, _| {
                // Searching may find nothing, which covers "you may".
                on(d, When::Enters, vec![], vec![search(SearchFor::Artifact, SearchTo::Graveyard)]);
                let card = Filter { artifact_cards: true, max_mana_value: Some(3), ..graveyard_cards(false, Controller::You) };
                let n = activated(d, "{R}", true, vec![card], vec![Effect::Reanimate(0)]);
                d.activated[n].sacrifice_artifact = true;
            },
        ),
        "Cavern of Souls" => s(
            &[
                "As this land enters, choose a creature type.",
                "{T}: Add {C}.",
                "{T}: Add one mana of any color. Spend this mana only to cast a creature spell of the chosen type, and that spell can't be countered.",
            ],
            |d, _| {
                d.choose_creature_type = true;
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                d.any_color_for_chosen_type = true;
                d.chosen_type_mana_uncounterable = true;
            },
        ),
        "Peregrin Took" => s(
            &[
                "If one or more tokens would be created under your control, those tokens plus an additional Food token are created instead.",
                "Sacrifice three Foods: Draw a card.",
            ],
            |d, t| {
                d.tokens_plus_one = Some(t.food);
                let n = activated(d, "", false, vec![], vec![draw(1)]);
                d.activated[n].sacrifice_subtype = Some("Food");
                d.activated[n].sacrifice_subtype_count = 3;
            },
        ),
        "Erebos, God of the Dead" => s(
            &[
                "As long as your devotion to black is less than five, Erebos isn't a creature.",
                "Your opponents can't gain life.",
                "{1}{B}, Pay 2 life: Draw a card.",
            ],
            |d, _| {
                d.creature_unless_devotion = Some(([false, false, true, false, false], 5));
                d.opponents_cant_gain_life = true;
                let n = activated(d, "{1}{B}", false, vec![], vec![draw(1)]);
                d.activated[n].life = 2;
            },
        ),
        "Clearwater Pathway // Murkwater Pathway" => s(&["{T}: Add {U}.", "Back: {T}: Add {B}."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Blue]));
            d.modal_back = true;
            d.back.as_mut().expect("Murkwater Pathway").mana = Some(ManaSource::of(&[Color::Black]));
        }),
        "Hengegate Pathway // Mistgate Pathway" => s(&["{T}: Add {W}.", "Back: {T}: Add {U}."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::White]));
            d.modal_back = true;
            d.back.as_mut().expect("Mistgate Pathway").mana = Some(ManaSource::of(&[Color::Blue]));
        }),
        "Barkchannel Pathway // Tidechannel Pathway" => s(&["{T}: Add {G}.", "Back: {T}: Add {U}."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Green]));
            d.modal_back = true;
            d.back.as_mut().expect("Tidechannel Pathway").mana = Some(ManaSource::of(&[Color::Blue]));
        }),
        "Nyxbloom Ancient" => s(
            &["If you tap a permanent for mana, it produces three times as much of that mana instead."],
            |d, _| d.triples_mana_tapped = true,
        ),
        "Sylvan Library" => s(
            &["At the beginning of your draw step, you may draw two additional cards. If you do, choose two cards in your hand drawn this turn. For each of those cards, pay 4 life or put the card on top of your library."],
            |d, _| conditional(d, When::EachDrawStep, Condition::YourTurn, vec![], vec![Effect::SylvanLibrary]),
        ),
        "Deflecting Palm" => s(
            &["The next time a source of your choice would deal damage to you this turn, prevent that damage. If damage is prevented this way, Deflecting Palm deals that much damage to that source's controller."],
            |d, _| {
                spell(d, vec![], vec![Effect::PreventNextFromChosenSource { trigger: 0 }]);
                on(d, When::Never, vec![], vec![Effect::DamageExtrasControllerAmount]);
            },
        ),
        "Sigarda, Host of Herons" => s(&["Spells and abilities your opponents control can't cause you to sacrifice permanents."], |d, _| {
            d.opponents_cant_make_you_sacrifice = true;
        }),
        "Valgavoth, Terror Eater" => s(
            &[
                "Ward\u{2014}Sacrifice three nonland permanents.",
                "If a card you didn't control would be put into an opponent's graveyard from anywhere, exile it instead.",
                "During your turn, you may play cards exiled with Valgavoth. If you cast a spell this way, pay life equal to its mana value rather than pay its mana cost.",
            ],
            |d, _| {
                d.ward_sacrifice = 3;
                d.ward_sacrifice_nonland = true;
                d.exiles_opponents_cards_to_play = true;
            },
        ),
        _ => None,
    }
}

/// Lane B's scripted playgroup cards (checked against their text in `lib.rs`).
pub const PG_B_SCRIPTED: &[&str] = &[
    "Birds of Paradise",
    "Elvish Mystic",
    "Lotus Petal",
    "Nature's Lore",
    "Three Visits",
    "Skyshroud Claim",
    "Kodama's Reach",
    "Wooded Foothills",
    "Aggravated Assault",
    "Seize the Day",
    "Grim Monolith",
    "Hull Breach",
    "Heroic Intervention",
    "Fire-Lit Thicket",
    "Ancient Tomb",
    "Fiery Emancipation",
    "Goblin Anarchomancer",
    "Elder Gargaroth",
    "Fierce Empath",
    "Thornspire Verge",
    "Reflecting Pool",
    "Stomping Ground",
    "Kessig Wolf Run",
    "Xenagos, God of Revels",
    "Nylea, God of the Hunt",
    "Nylea, Keen-Eyed",
    "Old Gnawbone",
    "Balefire Dragon",
    "Dragonborn Champion",
    "Deus of Calamity",
    "Kogla, the Titan Ape",
    "Kogla and Yidaro",
    "Malignus",
    "Questing Beast",
    "The Great Henge",
    "Zilortha, Strength Incarnate",
    "Vandalblast",
    "Bloodthirster",
    "Karlach, Fury of Avernus",
    "Combat Celebrant",
    "Hellkite Tyrant",
    "Yavimaya, Cradle of Growth",
    "Quartzwood Crasher",
    "Ilharg, the Raze-Boar",
    "Chaos Imps",
    "Silvos, Rogue Elemental",
    "Yavimaya Hollow",
    "Triumph of the Hordes",
    "Savage Beating",
    "Anzrag, the Quake-Mole",
    "Fanatic of Rhonas",
    "Moraug, Fury of Akoum",
    "Defiler of Vigor",
    "World at War",
    "Roaming Throne",
    "Deflecting Swat",
    "Ojer Kaslem, Deepest Growth // Temple of Cultivation",
    "Vorinclex // The Grand Evolution",
    // Aesi (Time)
    "Eternal Witness",
    "Negate",
    "Mana Leak",
    "Scatter Ray",
    "Frantic Search",
    "Courser of Kruphix",
    "Basalt Monolith",
    "Myriad Landscape",
    "Silverback Elder",
    "Horn of Greed",
    "Simic Growth Chamber",
    "Emerald Medallion",
    "Jaheira's Respite",
    "Sprouting Vines",
    "Sword of the Animist",
    "Tectonic Edge",
    "Tolaria West",
    "Zimone, Quandrix Prodigy",
    "Rhystic Study",
    "Mystic Remora",
    "Arcane Denial",
    "Perennial Behemoth",
    "Walk the Aeons",
    "Nissa, Who Shakes the World",
    "Blessing of Frost",
    // Myrkul (DonSpider)
    "Temple Garden",
    "Godless Shrine",
    "Vault of Champions",
    "Bountiful Promenade",
    "Caves of Koilos",
    "Wooded Bastion",
    "City of Brass",
    "Despark",
    "Anguished Unmaking",
    "Anointed Procession",
    "Vanquish the Horde",
    "Smothering Tithe",
    "Setessan Champion",
    "Eidolon of Blossoms",
    "Grim Guardian",
    "Archon of Sun's Grace",
    "Ghostly Dancers",
    "Sanctum Weaver",
    "Serra's Sanctum",
    "Grand Abolisher",
    "Devoted Druid",
    "Yawgmoth, Thran Physician",
    "Myrkul, Lord of Bones",
    "Maha, Its Feathers Night",
    "Esper Sentinel",
    "Austere Command",
    "Dawn's Truce",
    "Eldritch Evolution",
    "Birthing Pod",
    "Enduring Vitality",
    "Jaheira, Friend of the Forest",
    "Arachnogenesis",
    "For the Common Good",
    "Mirari's Wake",
    "Kaya, Geist Hunter",
    "The Golden Throne",
    "Mondrak, Glory Dominus",
    "Exalted Sunborn",
    "Grim Haruspex",
    "Black Market Connections",
    // Replacement effects (Mac B core theme)
    "Hardened Scales",
    "Bloodletter of Aclazotz",
    "Alhammarret's Archive",
    "Teferi's Ageless Insight",
    "Quina, Qu Gourmet",
    "Force of Negation",
    "Deepglow Skate",
    "Bogardan Dragonheart",
    "Whip of Erebos",
    "Words of War",
    "Blood Moon",
    "Urborg, Tomb of Yawgmoth",
    "Thousand-Year Elixir",
    "Damia, Sage of Stone",
    "Drumbellower",
    "Savor the Moment",
    "Gaddock Teeg",
    "Trusted Advisor",
    "Jin-Gitaxias, Core Augur",
    // Tatyova (Grande)
    "Migration Path",
    "Joint Exploration",
    "Khalni Heart Expedition",
    "Noxious Revival",
    "On the Trail",
    "Rude Awakening",
    "Sakura-Tribe Scout",
    "Skyshroud Ranger",
    "Splendid Reclamation",
    "Squall Line",
    "Stroke of Genius",
    "Wipe Away",
    "Laboratory Maniac",
    // Zhulodok (Adam)
    "Expedition Map",
    "Strip Mine",
    "Wasteland",
    "Palladium Myr",
    "Worn Powerstone",
    "Voltaic Key",
    "Manifold Key",
    "Urza's Cave",
    "Mithril Coat",
    "Artisan of Kozilek",
    "Fountainport",
    // Oloro (Adam)
    "Sea of Clouds",
    "Archangel of Thune",
    "Cliffhaven Vampire",
    "Drogskol Reaver",
    "Rhox Faithmender",
    "Serra Ascendant",
    "Divinity of Pride",
    "Voice of the Blessed",
    "Heliod, Sun-Crowned",
    "Paradox Haze",
    "Dawn of Hope",
    "Murmuring Mystic",
    "Third Path Iconoclast",
    "Storm-Kiln Artist",
    "Misdirection",
    "Young Pyromancer",
    "Monument to Endurance",
    "Glimpse the Impossible",
    "Barbarian Ring",
    "Cephalid Coliseum",
    "Cabal Pit",
    "Kruphix, God of Horizons",
    "Horizon Stone",
    "Necrologia",
    "Invert Polarity",
    "Ulamog, the Ceaseless Hunger",
    "Void Winnower",
    "Epic Experiment",
    "Metallurgic Summonings",
    "Ugin, the Spirit Dragon",
    "Bolas's Citadel",
    "Gadrak, the Crown-Scourge",
    "Xorn",
    "Silent Arbiter",
    "Expropriate",
    "Kozilek's Command",
    "Ugin, the Ineffable",
    "Crested Sunmare",
    "Ajani, Strength of the Pride",
    "Sheltered by Ghosts",
    "Rankle, Master of Pranks",
    "Raphael, Tag Team Tough",
    "The Immortal Sun",
    "Archfiend of Despair",
    "Brightclimb Pathway // Grimclimb Pathway",
    "Gisela, Blade of Goldnight",
    "Sower of Discord",
    "Eiganjo Castle",
    "Bennie Bracks, Zoologist",
    "Inventors' Fair",
    "Smaug the Magnificent",
    "Mahadi, Emporium Master",
    "Goblin Engineer",
    "Cavern of Souls",
    "Peregrin Took",
    "Erebos, God of the Dead",
    "Clearwater Pathway // Murkwater Pathway",
    "Hengegate Pathway // Mistgate Pathway",
    "Barkchannel Pathway // Tidechannel Pathway",
    "Nyxbloom Ancient",
    "Sylvan Library",
    "Deflecting Palm",
    "Sigarda, Host of Herons",
    "Valgavoth, Terror Eater",
    // Ghyrson Starn (Spr3adsh33t)
    "Basilisk Collar",
    "Gorgon's Head",
    "Electrostatic Field",
    "Erebor Flamesmith",
    "Kessig Flamebreather",
    "Grapeshot",
    "The Surgical Bay",
    "The Autonomous Furnace",
    "Training Center",
    "Blisterspit Gremlin",
    "Thermo-Alchemist",
    "Unruly Catapult",
    "Gelectrode",
    "Goblin Sharpshooter",
    "Cast into the Fire",
    "Consider",
    "Serum Visions",
    "Electrolyze",
    "Niv-Mizzet, Parun",
];
