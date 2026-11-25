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
];
