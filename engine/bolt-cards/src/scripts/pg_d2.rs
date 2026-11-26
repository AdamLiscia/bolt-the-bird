//! Playgroup decks (Nerd Leagues), lane D2 (the desktop): the cards for the deck in handoff/lanes/d2.json. Conventions:
//! docs/lane-brief.md. Card text: assets/PG_oracle.json (in the CMD pool). Tests: tests/pg_d2.rs.
#![allow(unused_imports)]

use super::*;
use bolt_core::{EntryDoubler, Suppression, TriggerDoubler};

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        // ---- Core: triggers, batch 1 (trigger conditions and amounts) ----
        "Lifeblood Hydra" => s(
            &[
                "This creature enters with X +1/+1 counters on it.",
                "When this creature dies, you gain life and draw cards equal to its power.",
            ],
            |d, _| {
                d.enters_with_x_counters = true;
                on(d, When::Dies, vec![], vec![Effect::GainLifeAndDrawSourcePower]);
            },
        ),
        "Mana Vault" => s(
            &[
                "This artifact doesn't untap during your untap step.",
                "At the beginning of your upkeep, you may pay {4}. If you do, untap this artifact.",
                "At the beginning of your draw step, if this artifact is tapped, it deals 1 damage to you.",
                "{T}: Add {C}{C}{C}.",
            ],
            |d, _| {
                d.doesnt_untap = true;
                let four = ManaCost::parse("{4}").expect("a mana cost");
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::MayPay(four), Effect::UntapSelf]);
                let yours_and_tapped = Condition::Both(Box::new(Condition::YourTurn), Box::new(Condition::SourceTapped));
                conditional(d, When::EachDrawStep, yours_and_tapped, vec![], vec![damage(1, Recipient::You)]);
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                d.mana_amount = 3;
            },
        ),
        "Howling Mine" => s(
            &["At the beginning of each player's draw step, if this artifact is untapped, that player draws an additional card."],
            |d, _| conditional(d, When::EachDrawStep, Condition::SourceUntapped, vec![], vec![Effect::ActivePlayerDraws(1)]),
        ),
        "Loyal Drake" => s(
            &["Lieutenant — At the beginning of combat on your turn, if you control your commander, draw a card."],
            |d, _| conditional(d, When::BeginCombatYourTurn, Condition::ControlCommander, vec![], vec![draw(1)]),
        ),
        "Aetherflux Reservoir" => s(
            &[
                "Whenever you cast a spell, you gain 1 life for each spell you've cast this turn.",
                "Pay 50 life: This artifact deals 50 damage to any target.",
            ],
            |d, _| {
                on(d, When::YouCast, vec![], vec![Effect::GainLifePerSpellCastThisTurn]);
                let n = activated(d, "", false, vec![Filter::ANY], vec![damage(50, Recipient::Target(0))]);
                d.activated[n].life = 50;
            },
        ),
        "Hydroid Krasis" => s(
            &[
                "When you cast this spell, you gain half X life and draw half X cards. Round down each time.",
                "This creature enters with X +1/+1 counters on it.",
            ],
            |d, _| {
                on(d, When::CastThisSpell, vec![], vec![Effect::GainAndDrawHalfX]);
                d.enters_with_x_counters = true;
            },
        ),
        "Malakir Rebirth // Malakir Mire" => s(
            &[
                "Choose target creature. You lose 2 life. Until end of turn, that creature gains \"When this creature dies, return it to the battlefield tapped under its owner's control.\"",
                "Back: This land enters tapped.",
                "Back: {T}: Add {B}.",
            ],
            |d, _| {
                // The granted trigger is Fake Your Own Death's (GrantDiesTrigger). Its back face,
                // Malakir Mire, is played as a land instead (a modal double-faced card).
                d.modal_back = true;
                let mire = d.back.as_mut().expect("Malakir Mire");
                mire.enters_tapped = true;
                mire.mana = Some(ManaSource::of(&[Color::Black]));
                spell(
                    d,
                    vec![CREATURE],
                    vec![Effect::LoseLife { who: Who::You, n: 2 }, Effect::GrantDiesTrigger { slot: 0, trigger: 0 }],
                );
                on(d, When::Never, vec![graveyard_cards(true, Controller::Any)], vec![Effect::ReturnTapped(0)]);
            },
        ),
        "Burgeoning" => s(
            &["Whenever an opponent plays a land, you may put a land card from your hand onto the battlefield."],
            |d, _| {
                // A land is played only on its player's own turn: "an opponent plays" is
                // PlayerPlaysLand on a turn that isn't yours.
                conditional(d, When::PlayerPlaysLand, Condition::NotYourTurn, vec![], vec![Effect::MayLandFromHand]);
            },
        ),
        // ---- Core: triggers, batch 2 (life lost and cards drawn by an amount) ----
        "Reanimate" => s(
            &["Put target creature card from a graveyard onto the battlefield under your control. You lose life equal to that card's mana value."],
            |d, _| {
                // The life loss is counted first, while the card is still the target in its graveyard.
                let card = graveyard_cards(true, Controller::Any);
                spell(d, vec![card], vec![Effect::LoseLifeSlotManaValue(0), Effect::Reanimate(0)]);
            },
        ),
        "Feed the Swarm" => s(
            &["Destroy target creature or enchantment an opponent controls. You lose life equal to that permanent's mana value."],
            |d, _| {
                let f = Filter { players: false, planeswalkers: false, enchantments: true, controller: Controller::Opponent, ..Filter::ANY };
                spell(d, vec![f], vec![Effect::LoseLifeSlotManaValue(0), Effect::Destroy(0)]);
            },
        ),
        "Darkstar Augur" => s(
            &[
                "Offspring {B}",
                "At the beginning of your upkeep, reveal the top card of your library and put that card into your hand. You lose life equal to its mana value.",
            ],
            |d, _| {
                d.offspring = ManaCost::parse("{B}");
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::RevealTopToHandLoseManaValue]);
            },
        ),
        "Castle Locthwain" => s(
            &[
                "This land enters tapped unless you control a Swamp.",
                "{T}: Add {B}.",
                "{1}{B}{B}, {T}: Draw a card, then you lose life equal to the number of cards in your hand.",
            ],
            |d, _| {
                d.enters_tapped_unless = Some(["Swamp", "Swamp"]);
                d.mana = Some(ManaSource::of(&[Color::Black]));
                activated(d, "{1}{B}{B}", true, vec![], vec![draw(1), Effect::LoseLifePerCardInHand]);
            },
        ),
        "Eldritch Pact" => s(
            &["Target player draws X cards and loses X life, where X is the number of cards in their graveyard."],
            |d, _| spell(d, vec![Filter::PLAYER], vec![Effect::DrawAndLosePerGraveyardCard(Who::TargetPlayer(0))]),
        ),
        "Dark Deal" => s(
            &["Each player discards all the cards in their hand, then draws that many cards minus one."],
            |d, _| spell(d, vec![], vec![Effect::DiscardHandsDrawThatManyMinusOne]),
        ),
        "Graveborn Muse" => s(
            &["At the beginning of your upkeep, you draw X cards and lose X life, where X is the number of Zombies you control."],
            |d, _| on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::DrawAndLosePerSubtypeYouControl("Zombie")]),
        ),
        "Disciple of Bolas" => s(
            &["When this creature enters, sacrifice another creature. You gain X life and draw X cards, where X is that creature's power."],
            |d, _| on(d, When::Enters, vec![], vec![Effect::SacrificeAnotherGainAndDrawPower]),
        ),
        "Darigaaz's Caldera" => s(
            &[
                "When this land enters, sacrifice it unless you return a non-Lair land you control to its owner's hand.",
                "{T}: Add {B}, {R}, or {G}.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::ReturnNonSubtypeLandOrSacrificeSelf("Lair")]);
                d.mana = Some(ManaSource::of(&[Color::Black, Color::Red, Color::Green]));
            },
        ),
        "Breena, the Demagogue" => s(
            &["Whenever a player attacks one of your opponents, if that opponent has more life than another of your opponents, that attacking player draws a card and you put two +1/+1 counters on a creature you control."],
            |_, _| {
                // 1v1: there's never "another of your opponents", so the trigger never does anything.
            },
        ),
        // ---- Core: triggers, batch 3 (dies, discard, life and cast emits) ----
        "Sanguine Bond" => s(&["Whenever you gain life, target opponent loses that much life."], |d, _| {
            on(d, When::YouGainLife, vec![opponent_player()], vec![Effect::LoseLifeAmount(Who::TargetPlayer(0))])
        }),
        "Vito, Thorn of the Dusk Rose" => s(
            &[
                "Whenever you gain life, target opponent loses that much life.",
                "{3}{B}{B}: Creatures you control gain lifelink until end of turn.",
            ],
            |d, _| {
                on(d, When::YouGainLife, vec![opponent_player()], vec![Effect::LoseLifeAmount(Who::TargetPlayer(0))]);
                activated(d, "{3}{B}{B}", false, vec![], vec![Effect::GrantAll { who: Who::You, keywords: Keywords::LIFELINK }]);
            },
        ),
        "Defiant Bloodlord" => s(&["Whenever you gain life, target opponent loses that much life."], |d, _| {
            on(d, When::YouGainLife, vec![opponent_player()], vec![Effect::LoseLifeAmount(Who::TargetPlayer(0))])
        }),
        "Pitiless Plunderer" => s(&["Whenever another creature you control dies, create a Treasure token."], |d, t| {
            on(d, When::AnotherCreatureYouControlDies, vec![], vec![Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }])
        }),
        "Elas il-Kor, Sadistic Pilgrim" => s(
            &[
                "Whenever another creature you control enters, you gain 1 life.",
                "Whenever another creature you control dies, each opponent loses 1 life.",
            ],
            |d, _| {
                on(d, When::AnotherCreatureYouControlEnters, vec![], vec![Effect::GainLife { who: Who::You, n: 1 }]);
                on(d, When::AnotherCreatureYouControlDies, vec![], vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }]);
            },
        ),
        "Undead Augur" => s(
            &["Whenever this creature or another Zombie you control dies, you draw a card and lose 1 life."],
            |d, _| {
                let effects = vec![draw(1), Effect::LoseLife { who: Who::You, n: 1 }];
                on(d, When::ThisOrAnotherYouControlDiesWithSubtype("Zombie"), vec![], effects);
            },
        ),
        "Disciple of the Vault" => s(
            &["Whenever an artifact is put into a graveyard from the battlefield, you may have target opponent lose 1 life."],
            |d, _| {
                let lose = vec![Effect::LoseLife { who: Who::TargetPlayer(0), n: 1 }];
                may(d, When::ArtifactToGraveyardFromBattlefield, vec![opponent_player()], lose);
            },
        ),
        "Viridian Revel" => s(
            &["Whenever an artifact is put into an opponent's graveyard from the battlefield, you may draw a card."],
            |d, _| may(d, When::ArtifactToOpponentsGraveyardFromBattlefield, vec![], vec![draw(1)]),
        ),
        "Feast of Sanity" => s(
            &["Whenever you discard a card, this enchantment deals 1 damage to any target and you gain 1 life."],
            |d, _| {
                let effects = vec![damage(1, Recipient::Target(0)), Effect::GainLife { who: Who::You, n: 1 }];
                on(d, When::YouDiscard, vec![Filter::ANY], effects);
            },
        ),
        "Shadowstorm Vizier" => s(
            &["Whenever you cycle or discard a card, this creature gets +1/+1 until end of turn."],
            |d, _| {
                // Cycling discards the card (CR 702.29a): "cycle or discard" is one discard trigger.
                on(d, When::YouDiscard, vec![], vec![Effect::PumpSelf { power: 1, toughness: 1 }]);
            },
        ),
        "Drake Haven" => s(
            &["Whenever you cycle or discard a card, you may pay {1}. If you do, create a 2/2 blue Drake creature token with flying."],
            |d, t| {
                // Cycling discards the card: one discard trigger. The engine's 2/2 flying Drake token
                // (token colors aren't modeled).
                let one = ManaCost::parse("{1}").expect("a mana cost");
                let effects = vec![Effect::MayPay(one), Effect::CreateToken { who: Who::You, def: t.drake, n: 1 }];
                on(d, When::YouDiscard, vec![], effects);
            },
        ),
        "Enchantress's Presence" => s(&["Whenever you cast an enchantment spell, draw a card."], |d, _| {
            on(d, When::YouCastEnchantment, vec![], vec![draw(1)])
        }),
        "Argothian Enchantress" => s(&["Whenever you cast an enchantment spell, draw a card."], |d, _| {
            on(d, When::YouCastEnchantment, vec![], vec![draw(1)])
        }),
        "Satyr Enchanter" => s(&["Whenever you cast an enchantment spell, draw a card."], |d, _| {
            on(d, When::YouCastEnchantment, vec![], vec![draw(1)])
        }),
        "Mesa Enchantress" => s(&["Whenever you cast an enchantment spell, you may draw a card."], |d, _| {
            may(d, When::YouCastEnchantment, vec![], vec![draw(1)])
        }),
        "Verduran Enchantress" => s(&["Whenever you cast an enchantment spell, you may draw a card."], |d, _| {
            may(d, When::YouCastEnchantment, vec![], vec![draw(1)])
        }),
        "Sythis, Harvest's Hand" => s(&["Whenever you cast an enchantment spell, you gain 1 life and draw a card."], |d, _| {
            on(d, When::YouCastEnchantment, vec![], vec![Effect::GainLife { who: Who::You, n: 1 }, draw(1)])
        }),
        "Mirran Spy" => s(&["Whenever you cast an artifact spell, you may untap target creature."], |d, _| {
            may(d, When::YouCastArtifact, vec![CREATURE], vec![Effect::UntapTarget(0)])
        }),
        "Revel in Riches" => s(
            &[
                "Whenever a creature an opponent controls dies, create a Treasure token.",
                "At the beginning of your upkeep, if you control ten or more Treasures, you win the game.",
            ],
            |d, t| {
                on(d, When::OpponentCreatureDies, vec![], vec![Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }]);
                let ten = Condition::ControlSubtypeAtLeast("Treasure", 10);
                conditional(d, When::Upkeep { your_turn_only: true }, ten, vec![], vec![Effect::WinTheGame]);
            },
        ),
        "Yahenni, Undying Partisan" => s(
            &[
                "Whenever a creature an opponent controls dies, put a +1/+1 counter on Yahenni.",
                "Sacrifice another creature: Yahenni gains indestructible until end of turn.",
            ],
            |d, _| {
                on(d, When::OpponentCreatureDies, vec![], vec![Effect::CountersSelf { n: 1 }]);
                let n = activated(d, "", false, vec![], vec![Effect::GrantSelf { keywords: Keywords::INDESTRUCTIBLE }]);
                d.activated[n].sacrifice_other_creature = true;
            },
        ),
        "Herd Baloth" => s(
            &["Whenever one or more +1/+1 counters are put on this creature, you may create a 4/4 green Beast creature token."],
            |d, t| may(d, When::PlusCountersPutOnThis, vec![], vec![Effect::CreateToken { who: Who::You, def: t.beast_4, n: 1 }]),
        ),
        "Scurry Oak" => s(
            &[
                "Evolve",
                "Whenever one or more +1/+1 counters are put on this creature, you may create a 1/1 green Squirrel creature token.",
            ],
            |d, t| {
                let that = Filter { triggering: true, ..CREATURE };
                on(d, When::AnotherCreatureYouControlEnters, vec![that], vec![Effect::Evolve(0)]);
                may(d, When::PlusCountersPutOnThis, vec![], vec![Effect::CreateToken { who: Who::You, def: t.squirrel, n: 1 }]);
            },
        ),
        "Vilis, Broker of Blood" => s(
            &[
                "{B}, Pay 2 life: Target creature gets -1/-1 until end of turn.",
                "Whenever you lose life, draw that many cards.",
            ],
            |d, _| {
                let n = activated(d, "{B}", false, vec![CREATURE], vec![Effect::Pump { slot: 0, power: -1, toughness: -1 }]);
                d.activated[n].life = 2;
                on(d, When::YouLoseLife, vec![], vec![Effect::DrawAmount]);
            },
        ),
        // ---- Core: triggers, batch 4 (suppressed and doubled triggers) ----
        "Hushbringer" => s(&["Creatures entering or dying don't cause abilities to trigger."], |d, _| {
            d.suppresses_triggers = Some(Suppression::CreaturesEnteringOrDying);
        }),
        "Doorkeeper Thrull" => s(&["Artifacts and creatures entering don't cause abilities to trigger."], |d, _| {
            d.suppresses_triggers = Some(Suppression::ArtifactsOrCreaturesEntering);
        }),
        "Elesh Norn, Mother of Machines" => s(
            &[
                "If a permanent entering causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.",
                "Permanents entering don't cause abilities of permanents your opponents control to trigger.",
            ],
            |d, _| {
                d.entering_triggers_again = Some(EntryDoubler::Permanent);
                d.suppresses_triggers = Some(Suppression::PermanentsEnteringForOpponents);
            },
        ),
        "Harmonic Prodigy" => s(
            &[
                "Prowess",
                "If a triggered ability of a Shaman or another Wizard you control triggers, that ability triggers an additional time.",
            ],
            |d, _| {
                on(d, When::YouCastNoncreature, vec![], vec![Effect::PumpSelf { power: 1, toughness: 1 }]);
                d.doubles_triggers_of = Some(TriggerDoubler::ShamanOrOtherWizard);
            },
        ),
        // ---- Core: triggers, batch 5 (sacrifice, enter and life-lost emits) ----
        "Mazirek, Kraul Death Priest" => s(
            &["Whenever a player sacrifices another permanent, put a +1/+1 counter on each creature you control."],
            |d, _| on(d, When::PlayerSacrificesAnother, vec![], vec![Effect::CountersAll { who: Who::You, n: 1 }]),
        ),
        "Mirkwood Bats" => s(&["Whenever you create or sacrifice a token, each opponent loses 1 life."], |d, _| {
            // A token entering the battlefield is a token being created.
            for when in [When::TokenYouControlEnters, When::YouSacrificeToken] {
                on(d, when, vec![], vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }]);
            }
        }),
        "Al Bhed Salvagers" => s(
            &["Whenever this creature or another creature or artifact you control dies, target opponent loses 1 life and you gain 1 life."],
            |d, _| {
                let drain = vec![Effect::LoseLife { who: Who::TargetPlayer(0), n: 1 }, Effect::GainLife { who: Who::You, n: 1 }];
                on(d, When::ThisOrAnotherCreatureOrArtifactYouControlDies, vec![opponent_player()], drain);
            },
        ),
        "Altar of the Brood" => s(&["Whenever another permanent you control enters, each opponent mills a card."], |d, _| {
            on(d, When::AnotherPermanentYouControlEnters, vec![], vec![Effect::Mill { who: Who::EachOpponent, n: 1 }])
        }),
        "Ivy Lane Denizen" => s(
            &["Whenever another green creature you control enters, put a +1/+1 counter on target creature."],
            |d, _| on(d, When::AnotherCreatureYouControlEntersWithColor(4), vec![CREATURE], vec![Effect::Counters { slot: 0, n: 1 }]),
        ),
        "Wound Reflection" => s(
            &["At the beginning of each end step, each opponent loses life equal to the life they lost this turn."],
            |d, _| {
                // 1v1: "each opponent" is the one opponent.
                on(d, When::EndStep { your_turn_only: false }, vec![], vec![Effect::LoseLifeLostThisTurn(Who::EachOpponent)]);
            },
        ),
        // ---- Core: triggers, batch 6 (cast, copy, damage, graveyard and batched emits) ----
        "Witherbloom Apprentice" => s(
            &["Magecraft — Whenever you cast or copy an instant or sorcery spell, each opponent loses 1 life and you gain 1 life."],
            |d, _| {
                let drain = vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }, Effect::GainLife { who: Who::You, n: 1 }];
                on(d, When::YouCastOrCopyInstantOrSorcery, vec![], drain);
            },
        ),
        "Glimpse of Nature" => s(&["Whenever you cast a creature spell this turn, draw a card."], |d, _| {
            spell(d, vec![], vec![Effect::DelayedThisTurn { when: When::YouCastCreature, trigger: 0 }]);
            // Trigger 0 goes on the stack only from the delayed entry (a sorcery never fires it).
            on(d, When::YouCastCreature, vec![], vec![draw(1)]);
        }),
        "Zenith Chronicler" => s(
            &["Whenever a player casts their first multicolored spell each turn, each other player draws a card."],
            |d, _| {
                // 1v1: "each other player" is the caster's opponent.
                on(d, When::YouCastFirstMulticolored, vec![], vec![Effect::Draw { who: Who::EachOpponent, n: 1 }]);
                on(d, When::OpponentCastsFirstMulticolored, vec![], vec![draw(1)]);
            },
        ),
        "No Mercy" => s(&["Whenever a creature deals damage to you, destroy it."], |d, _| {
            let it = Filter { triggering: true, ..CREATURE };
            on(d, When::CreatureDealsDamageToYou, vec![it], vec![Effect::Destroy(0)]);
        }),
        "Francisco, Fowl Marauder" => s(
            &[
                "Francisco can't block.",
                "Whenever one or more Pirates you control deal damage to a player, Francisco explores.",
                "Partner",
            ],
            |d, _| {
                d.cant_block = true;
                let this = Filter { triggering: true, ..CREATURE };
                on(d, When::YourSubtypeDealDamageToPlayer("Pirate"), vec![this], vec![Effect::Explore(0)]);
            },
        ),
        "Syr Konrad, the Grim" => s(
            &[
                "Whenever another creature dies, or a creature card is put into a graveyard from anywhere other than the battlefield, or a creature card leaves your graveyard, Syr Konrad deals 1 damage to each opponent.",
                "{1}{B}: Each player mills a card.",
            ],
            |d, _| {
                for when in [
                    When::AnotherCreatureDies,
                    When::CreatureCardToGraveyardNotFromBattlefield,
                    When::CreatureCardLeavesYourGraveyard,
                ] {
                    on(d, when, vec![], vec![damage(1, Recipient::EachOpponent)]);
                }
                let mill = vec![Effect::Mill { who: Who::You, n: 1 }, Effect::Mill { who: Who::EachOpponent, n: 1 }];
                activated(d, "{1}{B}", false, vec![], mill);
            },
        ),
        "Species Specialist" => s(
            &["As this creature enters, choose a creature type.", "Whenever a creature of the chosen type dies, you may draw a card."],
            |d, _| {
                d.choose_creature_type = true;
                may(d, When::CreatureOfChosenTypeDies, vec![], vec![draw(1)]);
            },
        ),
        "Bloodchief Ascension" => s(
            &[
                "At the beginning of each end step, if an opponent lost 2 or more life this turn, you may put a quest counter on this enchantment.",
                "Whenever a card is put into an opponent's graveyard from anywhere, if this enchantment has three or more quest counters on it, you may have that player lose 2 life. If you do, you gain 2 life.",
            ],
            |d, _| {
                let mut quest = Ability::new(vec![], vec![Effect::QuestCounterSelf]).optional();
                quest.condition = Some(Condition::OpponentLostLifeAtLeast(2));
                d.triggers.push(Trigger { when: When::EndStep { your_turn_only: false }, ability: quest });
                // 1v1: "that player" is the one opponent.
                let drain = vec![Effect::LoseLife { who: Who::EachOpponent, n: 2 }, Effect::GainLife { who: Who::You, n: 2 }];
                let mut drain = Ability::new(vec![], drain).optional();
                drain.condition = Some(Condition::SourceQuestCountersAtLeast(3));
                d.triggers.push(Trigger { when: When::CardToOpponentsGraveyard, ability: drain });
            },
        ),
        "Cryptcaller Chariot" => s(
            &["Whenever you discard one or more cards, create that many tapped 2/2 black Zombie creature tokens.", "Crew 2"],
            |d, t| {
                on(d, When::YouDiscardOneOrMore, vec![], vec![Effect::CreateTokensAmountTapped { def: t.zombie }]);
                let n = activated(d, "", false, vec![], vec![Effect::CrewedSelf]);
                d.activated[n].crew = 2;
            },
        ),
        // ---- Core: triggers, batch 7 ----
        "Ghostly Pilferer" => s(
            &[
                "Whenever this creature becomes untapped, you may pay {2}. If you do, draw a card.",
                "Whenever an opponent casts a spell from anywhere other than their hand, draw a card.",
                "Discard a card: This creature can't be blocked this turn.",
            ],
            |d, _| {
                let two = ManaCost::parse("{2}").expect("a mana cost");
                on(d, When::BecomesUntapped, vec![], vec![Effect::MayPay(two), draw(1)]);
                on(d, When::OpponentCastsNotFromHand, vec![], vec![draw(1)]);
                let n = activated(d, "", false, vec![], vec![Effect::GrantSelf { keywords: Keywords::UNBLOCKABLE }]);
                d.activated[n].discard_cost = true;
            },
        ),
        // ---- Core: triggers, batch 9 ----
        "Archmage Emeritus" => s(&["Magecraft — Whenever you cast or copy an instant or sorcery spell, draw a card."], |d, _| {
            on(d, When::YouCastOrCopyInstantOrSorcery, vec![], vec![draw(1)])
        }),
        "Lobber Crew" => s(
            &["{T}: This creature deals 1 damage to each opponent.", "Whenever you cast a multicolored spell, untap this creature."],
            |d, _| {
                activated(d, "", true, vec![], vec![damage(1, Recipient::EachOpponent)]);
                on(d, When::YouCastMulticolored, vec![], vec![Effect::UntapSelf]);
            },
        ),
        "Chandra's Spitfire" => s(
            &["Whenever an opponent is dealt noncombat damage, this creature gets +3/+0 until end of turn."],
            |d, _| on(d, When::OpponentDealtNoncombatDamage, vec![], vec![Effect::PumpSelf { power: 3, toughness: 0 }]),
        ),
        "Curiosity" => s(
            &["Enchant creature", "Whenever enchanted creature deals damage to an opponent, you may draw a card."],
            |d, _| {
                d.aura = true;
                d.spell = Some(Ability::new(vec![CREATURE], vec![]));
                may(d, When::EnchantedDealsDamageToOpponent, vec![], vec![draw(1)]);
            },
        ),
        "Ophidian Eye" => s(
            &["Enchant creature", "Whenever enchanted creature deals damage to an opponent, you may draw a card."],
            |d, _| {
                d.aura = true;
                d.spell = Some(Ability::new(vec![CREATURE], vec![]));
                may(d, When::EnchantedDealsDamageToOpponent, vec![], vec![draw(1)]);
            },
        ),
        // ---- Core: triggers, batch 10 (life and board conditions, enters tapped, nontoken sacrifice) ----
        "Witch of the Moors" => s(
            &["At the beginning of your end step, if you gained life this turn, each opponent sacrifices a creature of their choice and you return up to one target creature card from your graveyard to your hand."],
            |d, _| {
                let mut ability = up_to(
                    vec![graveyard_cards(true, Controller::You)],
                    vec![Effect::SacrificeCreatures { who: Who::EachOpponent, n: 1 }, Effect::Bounce(0)],
                );
                ability.condition = Some(Condition::YouGainedLifeThisTurn);
                d.triggers.push(Trigger { when: When::EndStep { your_turn_only: true }, ability });
            },
        ),
        "Felidar Sovereign" => s(&["At the beginning of your upkeep, if you have 40 or more life, you win the game."], |d, _| {
            conditional(d, When::Upkeep { your_turn_only: true }, Condition::LifeAtLeast(40), vec![], vec![Effect::WinTheGame])
        }),
        "Test of Endurance" => s(&["At the beginning of your upkeep, if you have 50 or more life, you win the game."], |d, _| {
            conditional(d, When::Upkeep { your_turn_only: true }, Condition::LifeAtLeast(50), vec![], vec![Effect::WinTheGame])
        }),
        "Pyrohemia" => s(
            &[
                "At the beginning of the end step, if no creatures are on the battlefield, sacrifice this enchantment.",
                "{R}: This enchantment deals 1 damage to each creature and each player.",
            ],
            |d, _| {
                let end = When::EndStep { your_turn_only: false };
                conditional(d, end, Condition::NoCreaturesOnBattlefield, vec![], vec![Effect::SacrificeSource]);
                let each = vec![damage(1, Recipient::EachCreature { without_flying: false }), damage(1, Recipient::EachPlayer)];
                activated(d, "{R}", false, vec![], each);
            },
        ),
        "Vizkopa Guildmage" => s(
            &[
                "{1}{W}{B}: Target creature gains lifelink until end of turn.",
                "{1}{W}{B}: Whenever you gain life this turn, each opponent loses that much life.",
            ],
            |d, _| {
                // Trigger 0: the delayed "whenever you gain life this turn" (the amount is the life gained).
                on(d, When::Never, vec![], vec![Effect::LoseLifeAmount(Who::EachOpponent)]);
                activated(d, "{1}{W}{B}", false, vec![CREATURE], vec![Effect::Grant { slot: 0, keywords: Keywords::LIFELINK }]);
                let delayed = Effect::DelayedThisTurn { when: When::YouGainLife, trigger: 0 };
                activated(d, "{1}{W}{B}", false, vec![], vec![delayed]);
            },
        ),
        "Amulet of Vigor" => s(&["Whenever a permanent you control enters tapped, untap it."], |d, _| {
            let it = Filter {
                triggering: true,
                any_permanent: true,
                players: false,
                creatures: false,
                planeswalkers: false,
                controller: Controller::You,
                ..Filter::ANY
            };
            on(d, When::PermanentYouControlEntersTapped, vec![it], vec![Effect::UntapTarget(0)]);
        }),
        "Skittering Precursor" => s(
            &[
                "Devoid",
                "Whenever you sacrifice a nontoken permanent, create a 0/1 colorless Eldrazi Spawn creature token with \"Sacrifice this token: Add {C}.\"",
            ],
            |d, t| {
                // Devoid: the card's colors are already none (CR 702.114a).
                d.colors = [false; 5];
                let spawn = Effect::CreateToken { who: Who::You, def: t.eldrazi_spawn, n: 1 };
                on(d, When::YouSacrificeNontoken, vec![], vec![spawn]);
            },
        ),
        "It That Betrays" => s(
            &["Annihilator 2", "Whenever an opponent sacrifices a nontoken permanent, put that card onto the battlefield under your control."],
            |d, _| {
                on(d, When::Attacks, vec![], vec![Effect::SacrificePermanents { who: Who::EachOpponent, n: 2 }]);
                let that = Filter { triggering: true, ..graveyard_cards(false, Controller::Any) };
                on(d, When::OpponentSacrificesNontoken, vec![that], vec![Effect::Reanimate(0)]);
            },
        ),
        "Glaring Fleshraker" => s(
            &[
                "Whenever you cast a colorless spell, create a 0/1 colorless Eldrazi Spawn creature token with \"Sacrifice this token: Add {C}.\"",
                "Whenever another colorless creature you control enters, this creature deals 1 damage to each opponent.",
            ],
            |d, t| {
                let spawn = Effect::CreateToken { who: Who::You, def: t.eldrazi_spawn, n: 1 };
                on(d, When::YouCastColorless, vec![], vec![spawn]);
                on(d, When::AnotherColorlessCreatureYouControlEnters, vec![], vec![damage(1, Recipient::EachOpponent)]);
            },
        ),
        "Aurelia, the Warleader" => s(
            &["Whenever Aurelia attacks for the first time each turn, untap all creatures you control. After this phase, there is an additional combat phase."],
            |d, _| {
                let effects = vec![Effect::UntapCreaturesYouControl, Effect::AdditionalCombatAfterThisPhase];
                on(d, When::AttacksFirstTimeEachTurn, vec![], effects);
            },
        ),
        "Ghyrson Starn, Kelermorph" => s(
            &[
                "Ward {2}",
                "Three Autostubs — Whenever another source you control deals exactly 1 damage to a permanent or player, Ghyrson Starn deals 2 damage to that permanent or player.",
            ],
            |d, _| {
                d.ward = ManaCost::parse("{2}");
                let that = Filter { triggering: true, any_permanent: true, ..Filter::ANY };
                on(d, When::AnotherSourceYouControlDealsExactlyOne, vec![that], vec![damage(2, Recipient::Target(0))]);
            },
        ),
        "Graf Mole" => s(&["Whenever you sacrifice a Clue, you gain 3 life."], |d, _| {
            on(d, When::YouSacrificeSubtype("Clue"), vec![], vec![Effect::GainLife { who: Who::You, n: 3 }])
        }),
        // ---- The Gitrog, Ravenous Ride ----
        "Demonic Tutor" => s(&["Search your library for a card, put that card into your hand, then shuffle."], |d, _| {
            spell(d, vec![], vec![search(SearchFor::Any, SearchTo::Hand)])
        }),
        "Exquisite Blood" => s(&["Whenever an opponent loses life, you gain that much life."], |d, _| {
            on(d, When::OpponentLosesLife, vec![], vec![Effect::GainLifeAmount])
        }),
        "Goldvein Hydra" => s(
            &[
                "This creature enters with X +1/+1 counters on it.",
                "When this creature dies, create a number of tapped Treasure tokens equal to its power.",
            ],
            |d, t| {
                d.enters_with_x_counters = true;
                on(d, When::Dies, vec![], vec![Effect::CreateTokensPowerTapped { def: t.treasure }]);
            },
        ),
        "Grave Pact" => s(
            &["Whenever a creature you control dies, each other player sacrifices a creature of their choice."],
            |d, _| {
                // 1v1: "each other player" is the one opponent.
                on(d, When::CreatureYouControlDies, vec![], vec![Effect::SacrificeCreatures { who: Who::EachOpponent, n: 1 }])
            },
        ),
        "Rotting Regisaur" => s(&["At the beginning of your upkeep, discard a card."], |d, _| {
            on(d, When::Upkeep { your_turn_only: true }, vec![], vec![discard(1)])
        }),
        "Voracious Hydra" => s(
            &[
                "This creature enters with X +1/+1 counters on it.",
                "When this creature enters, choose one —",
                "• Double the number of +1/+1 counters on this creature.",
                "• This creature fights target creature you don't control.",
            ],
            |d, _| {
                d.enters_with_x_counters = true;
                on_modal(
                    d,
                    When::Enters,
                    None,
                    vec![
                        Ability::new(vec![], vec![Effect::DoubleCountersSelf]),
                        Ability::new(vec![OPP_CREATURE], vec![Effect::Fight(0)]),
                    ],
                );
            },
        ),
        "Daemogoth Titan" => s(&["Whenever this creature attacks or blocks, sacrifice a creature."], |d, _| {
            for when in [When::Attacks, When::Blocks] {
                on(d, when, vec![], vec![Effect::SacrificeCreatures { who: Who::You, n: 1 }]);
            }
        }),
        "Fleetfeather Sandals" => s(&["Equipped creature has flying and haste.", "Equip {2}"], |d, _| {
            attached(d, false, 0, 0, Keywords::FLYING.with(Keywords::HASTE));
            equip(d, "{2}");
        }),
        "Lantern of the Lost" => s(
            &[
                "When this artifact enters, exile target card from a graveyard.",
                "{1}, {T}, Exile this artifact: Exile all cards from all graveyards, then draw a card.",
            ],
            |d, _| {
                on(d, When::Enters, vec![graveyard_cards(false, Controller::Any)], vec![Effect::Exile(0)]);
                let all = vec![
                    Effect::ExileGraveyard { who: Who::You },
                    Effect::ExileGraveyard { who: Who::EachOpponent },
                    draw(1),
                ];
                let n = activated(d, "{1}", true, vec![], all);
                d.activated[n].exile_self = true;
            },
        ),
        // ---- Kaalia of the Vast ----
        "Blood Crypt" => s(SHOCK_LAND, |d, _| shock_land(d, Color::Black, Color::Red)),
        "Sacred Foundry" => s(SHOCK_LAND, |d, _| shock_land(d, Color::Red, Color::White)),
        "Deadly Rollick" => s(&[FREE_WITH_COMMANDER, "Exile target creature."], |d, _| {
            free_with_commander(d);
            spell(d, vec![CREATURE], vec![Effect::Exile(0)]);
        }),
        "Flawless Maneuver" => s(
            &[FREE_WITH_COMMANDER, "Creatures you control gain indestructible until end of turn."],
            |d, _| {
                free_with_commander(d);
                spell(d, vec![], vec![Effect::GrantAll { who: Who::You, keywords: Keywords::INDESTRUCTIBLE }]);
            },
        ),
        "Fetid Heath" => s(&["{T}: Add {C}.", "{W/B}, {T}: Add {W}{W}, {W}{B}, or {B}{B}."], |d, _| {
            filter_land(d, "{W/B}", Color::White, Color::Black)
        }),
        "Graven Cairns" => s(&["{T}: Add {C}.", "{B/R}, {T}: Add {B}{B}, {B}{R}, or {R}{R}."], |d, _| {
            filter_land(d, "{B/R}", Color::Black, Color::Red)
        }),
        "Dragonskull Summit" => s(&["This land enters tapped unless you control a Swamp or a Mountain.", "{T}: Add {B} or {R}."], |d, _| {
            d.enters_tapped_unless = Some(["Swamp", "Mountain"]);
            d.mana = Some(ManaSource::of(&[Color::Black, Color::Red]));
        }),
        "Vampiric Tutor" => s(&["Search your library for a card, then shuffle and put that card on top. You lose 2 life."], |d, _| {
            spell(d, vec![], vec![search(SearchFor::Any, SearchTo::Top), Effect::LoseLife { who: Who::You, n: 2 }])
        }),
        "Luxury Suite" => s(&[BATTLEBOND_LAND, "{T}: Add {B} or {R}."], |d, _| {
            d.enters_tapped = true; // a two-player game: one opponent
            d.mana = Some(ManaSource::of(&[Color::Black, Color::Red]));
        }),
        "Spectator Seating" => s(&[BATTLEBOND_LAND, "{T}: Add {R} or {W}."], |d, _| {
            d.enters_tapped = true; // a two-player game: one opponent
            d.mana = Some(ManaSource::of(&[Color::Red, Color::White]));
        }),
        "Talisman of Indulgence" => s(&["{T}: Add {C}.", "{T}: Add {B} or {R}. This artifact deals 1 damage to you."], |d, _| {
            pain_mana(d, Color::Black, Color::Red)
        }),
        "Talisman of Hierarchy" => s(&["{T}: Add {C}.", "{T}: Add {W} or {B}. This artifact deals 1 damage to you."], |d, _| {
            pain_mana(d, Color::White, Color::Black)
        }),
        "Sulfurous Springs" => s(&["{T}: Add {C}.", "{T}: Add {B} or {R}. This land deals 1 damage to you."], |d, _| {
            pain_mana(d, Color::Black, Color::Red)
        }),
        "Mortify" => s(&["Destroy target creature or enchantment."], |d, _| {
            let f = Filter { players: false, planeswalkers: false, enchantments: true, ..Filter::ANY };
            spell(d, vec![f], vec![Effect::Destroy(0)])
        }),
        "Pestilence Demon" => s(&["{B}: This creature deals 1 damage to each creature and each player."], |d, _| {
            let each = vec![damage(1, Recipient::EachCreature { without_flying: false }), damage(1, Recipient::EachPlayer)];
            activated(d, "{B}", false, vec![], each);
        }),
        "Akroma, Angel of Wrath" => s(
            &["Flying, first strike, vigilance, trample, haste, protection from black and from red"],
            |d, _| {
                d.keywords = Keywords::FLYING
                    .with(Keywords::FIRST_STRIKE)
                    .with(Keywords::VIGILANCE)
                    .with(Keywords::TRAMPLE)
                    .with(Keywords::HASTE);
                static_on(d, Scope::This, false, Keywords::NONE);
                d.statics.last_mut().expect("just added").protection = [false, false, true, true, false];
            },
        ),
        "Flagstones of Trokair" => s(
            &[
                "{T}: Add {W}.",
                "When Flagstones of Trokair is put into a graveyard from the battlefield, you may search your library for a Plains card, put it onto the battlefield tapped, then shuffle.",
            ],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::White]));
                let plains = SearchFor::LandTypes { types: land_types(&["Plains"]), basic: false };
                // `When::Dies` is "put into a graveyard from the battlefield" for any permanent.
                may(d, When::Dies, vec![], vec![search(plains, SearchTo::Battlefield { tapped: true })]);
            },
        ),
        "Shinka, the Bloodsoaked Keep" => s(
            &["{T}: Add {R}.", "{R}, {T}: Target legendary creature gains first strike until end of turn."],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::Red]));
                let legend = Filter { legendary_only: true, ..CREATURE };
                activated(d, "{R}", true, vec![legend], vec![Effect::Grant { slot: 0, keywords: Keywords::FIRST_STRIKE }]);
            },
        ),
        // ---- Henrika Domnathi ----
        "Diabolic Intent" => s(
            &[
                "As an additional cost to cast this spell, sacrifice a creature.",
                "Search your library for a card, put that card into your hand, then shuffle.",
            ],
            |d, _| {
                d.extra_cost = Some(ExtraCost::SacrificeCreature);
                spell(d, vec![], vec![search(SearchFor::Any, SearchTo::Hand)]);
            },
        ),
        "Cordial Vampire" => s(
            &["Whenever this creature or another creature dies, put a +1/+1 counter on each Vampire you control."],
            |d, _| {
                for when in [When::Dies, When::AnotherCreatureDies] {
                    on(d, when, vec![], vec![Effect::CountersOnYourSubtype("Vampire")]);
                }
            },
        ),
        "Sheoldred, the Apocalypse" => s(
            &["Whenever you draw a card, you gain 2 life.", "Whenever an opponent draws a card, they lose 2 life."],
            |d, _| {
                on(d, When::YouDraw, vec![], vec![Effect::GainLife { who: Who::You, n: 2 }]);
                // 1v1: "they" is the one opponent.
                on(d, When::OpponentDraws, vec![], vec![Effect::LoseLife { who: Who::EachOpponent, n: 2 }]);
            },
        ),
        "Diabolic Edict" => s(&["Target player sacrifices a creature of their choice."], |d, _| {
            spell(d, vec![Filter::PLAYER], vec![Effect::SacrificeCreatures { who: Who::TargetPlayer(0), n: 1 }])
        }),
        "Diresight" => s(&["Surveil 2, then draw two cards. You lose 2 life."], |d, _| {
            spell(d, vec![], vec![Effect::Surveil { n: 2 }, draw(2), Effect::LoseLife { who: Who::You, n: 2 }])
        }),
        "Kokusho, the Evening Star" => s(
            &["When Kokusho dies, each opponent loses 5 life. You gain life equal to the life lost this way."],
            |d, _| {
                // 1v1, and the engine has no life-loss replacement effects: the life lost is 5.
                let drain = vec![Effect::LoseLife { who: Who::EachOpponent, n: 5 }, Effect::GainLife { who: Who::You, n: 5 }];
                on(d, When::Dies, vec![], drain);
            },
        ),
        // ---- Korvold, Fae-Cursed King ----
        "Deserted Temple" => s(&["{T}: Add {C}.", "{1}, {T}: Untap target land."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            let land = Filter { players: false, creatures: false, planeswalkers: false, lands: true, ..Filter::ANY };
            activated(d, "{1}", true, vec![land], vec![Effect::UntapTarget(0)]);
        }),
        "Rakdos Carnarium" => s(
            &[
                "This land enters tapped.",
                "When this land enters, return a land you control to its owner's hand.",
                "{T}: Add {B}{R}.",
            ],
            |d, _| {
                d.enters_tapped = true;
                on(d, When::Enters, vec![], vec![Effect::ReturnLandYouControl]);
                d.paid_mana = Some((ManaCost::default(), [ManaSource::of(&[Color::Black]), ManaSource::of(&[Color::Red])]));
            },
        ),
        "Savage Lands" => s(&["This land enters tapped.", "{T}: Add {B}, {R}, or {G}."], |d, _| {
            d.enters_tapped = true;
            d.mana = Some(ManaSource::of(&[Color::Black, Color::Red, Color::Green]));
        }),
        "Shadowblood Ridge" => s(&["{1}, {T}: Add {B}{R}."], |d, _| {
            let br = [ManaSource::of(&[Color::Black]), ManaSource::of(&[Color::Red])];
            d.paid_mana = Some((ManaCost::parse("{1}").expect("a mana cost"), br));
        }),
        "Soul's Fire" => s(&["Target creature you control deals damage equal to its power to any target."], |d, _| {
            spell(d, vec![YOUR_CREATURE, Filter::ANY], vec![Effect::DamageEqualSlotPower { from: 0, to: 1 }])
        }),
        "Thrór's Map" => s(
            &[
                "When Thrór's Map enters, search your library for a basic land card, reveal it, put it into your hand, then shuffle.",
                "{2}, {T}: Draw a card, then discard a card.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![search(SearchFor::BasicLand, SearchTo::Hand)]);
                activated(d, "{2}", true, vec![], vec![draw(1), discard(1)]);
            },
        ),
        // ---- Damia, Sage of Stone ----
        "Rejuvenating Springs" => s(&[BATTLEBOND_LAND, "{T}: Add {G} or {U}."], |d, _| {
            d.enters_tapped = true; // a two-player game: one opponent
            d.mana = Some(ManaSource::of(&[Color::Green, Color::Blue]));
        }),
        "Morphic Pool" => s(&[BATTLEBOND_LAND, "{T}: Add {U} or {B}."], |d, _| {
            d.enters_tapped = true; // a two-player game: one opponent
            d.mana = Some(ManaSource::of(&[Color::Blue, Color::Black]));
        }),
        "Simic Signet" => s(&["{1}, {T}: Add {G}{U}."], |d, _| {
            let gu = [ManaSource::of(&[Color::Green]), ManaSource::of(&[Color::Blue])];
            d.paid_mana = Some((ManaCost::parse("{1}").expect("a mana cost"), gu));
        }),
        "Underground River" => s(&["{T}: Add {C}.", "{T}: Add {U} or {B}. This land deals 1 damage to you."], |d, _| {
            pain_mana(d, Color::Blue, Color::Black)
        }),
        "Sunken Ruins" => s(&["{T}: Add {C}.", "{U/B}, {T}: Add {U}{U}, {U}{B}, or {B}{B}."], |d, _| {
            filter_land(d, "{U/B}", Color::Blue, Color::Black)
        }),
        "Leyline of Anticipation" => s(
            &[
                "If this card is in your opening hand, you may begin the game with it on the battlefield.",
                "You may cast spells as though they had flash.",
            ],
            |d, _| {
                d.leyline = true;
                d.grants_flash = true;
            },
        ),
        "Okina, Temple to the Grandfathers" => s(
            &["{T}: Add {G}.", "{G}, {T}: Target legendary creature gets +1/+1 until end of turn."],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::Green]));
                let legend = Filter { legendary_only: true, ..CREATURE };
                activated(d, "{G}", true, vec![legend], vec![Effect::Pump { slot: 0, power: 1, toughness: 1 }]);
            },
        ),
        // ---- Raffine, Scheming Seer ----
        "Bitterbloom Bearer" => s(
            &["At the beginning of your upkeep, you lose 1 life and create a 1/1 blue and black Faerie creature token with flying."],
            |d, t| {
                let effects =
                    vec![Effect::LoseLife { who: Who::You, n: 1 }, Effect::CreateToken { who: Who::You, def: t.faerie, n: 1 }];
                // The engine's 1/1 flying Faerie token (token colors aren't modeled).
                on(d, When::Upkeep { your_turn_only: true }, vec![], effects);
            },
        ),
        "Queza, Augur of Agonies" => s(
            &["Whenever you draw a card, target opponent loses 1 life and you gain 1 life."],
            |d, _| {
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                let drain = vec![Effect::LoseLife { who: Who::TargetPlayer(0), n: 1 }, Effect::GainLife { who: Who::You, n: 1 }];
                on(d, When::YouDraw, vec![opp], drain);
            },
        ),
        "Vault of the Archangel" => s(
            &["{T}: Add {C}.", "{2}{W}{B}, {T}: Creatures you control gain deathtouch and lifelink until end of turn."],
            |d, _| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                let k = Keywords::DEATHTOUCH.with(Keywords::LIFELINK);
                activated(d, "{2}{W}{B}", true, vec![], vec![Effect::GrantAll { who: Who::You, keywords: k }]);
            },
        ),
        // ---- Core: triggers, batch 12 ----
        "Sefris of the Hidden Ways" => s(
            &[
                "Whenever one or more creature cards are put into your graveyard from anywhere, venture into the dungeon. This ability triggers only once each turn.",
                "Create Undead — Whenever you complete a dungeon, return target creature card from your graveyard to the battlefield.",
            ],
            |d, _| {
                let venture = Effect::Subsystem(bolt_core::SubsystemEffect::Venture);
                on(d, When::CreatureCardsToYourGraveyardOnceEachTurn, vec![], vec![venture]);
                on(d, When::YouCompleteDungeon, vec![graveyard_cards(true, Controller::You)], vec![Effect::Reanimate(0)]);
            },
        ),
        "Lunar Convocation" => s(
            &[
                "At the beginning of your end step, if you gained life this turn, each opponent loses 1 life.",
                "At the beginning of your end step, if you gained and lost life this turn, create a 1/1 black Bat creature token with flying.",
                "{1}{B}, Pay 2 life: Draw a card.",
            ],
            |d, t| {
                let end = When::EndStep { your_turn_only: true };
                let drain = vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }];
                conditional(d, end, Condition::YouGainedLifeThisTurn, vec![], drain);
                let both = Condition::Both(Box::new(Condition::YouGainedLifeThisTurn), Box::new(Condition::YouLostLifeThisTurn));
                conditional(d, end, both, vec![], vec![Effect::CreateToken { who: Who::You, def: t.bat, n: 1 }]);
                let n = activated(d, "{1}{B}", false, vec![], vec![draw(1)]);
                d.activated[n].life = 2;
            },
        ),
        "Saffi Eriksdotter" => s(
            &["Sacrifice Saffi Eriksdotter: When target creature is put into your graveyard this turn, return that card to the battlefield."],
            |d, _| {
                let trigger = d.triggers.len();
                let delayed = Effect::DelayedWhenSlotToYourGraveyardThisTurn { slot: 0, trigger };
                let n = activated(d, "", false, vec![Filter::CREATURE], vec![delayed]);
                d.activated[n].sacrifice_self = true;
                // Only from the delayed entry: "that card", in your graveyard, back under your control.
                let that = Filter { triggering: true, ..graveyard_cards(false, Controller::Any) };
                on(d, When::SlotCreatureToYourGraveyardThisTurn, vec![that], vec![Effect::Reanimate(0)]);
            },
        ),
        // ---- Core: triggers, batch 13 ----
        "Sheoldred, Whispering One" => s(
            &[
                "At the beginning of your upkeep, return target creature card from your graveyard to the battlefield.",
                "At the beginning of each opponent's upkeep, that player sacrifices a creature of their choice.",
            ],
            |d, _| {
                let card = graveyard_cards(true, Controller::You);
                on(d, When::Upkeep { your_turn_only: true }, vec![card], vec![Effect::Reanimate(0)]);
                // An opponent's upkeep is one that isn't yours; "that player" is the active player.
                let sac = vec![Effect::SacrificeCreatures { who: Who::ActivePlayer, n: 1 }];
                conditional(d, When::Upkeep { your_turn_only: false }, Condition::NotYourTurn, vec![], sac);
            },
        ),
        "Necropotence" => s(
            &[
                "Skip your draw step.",
                "Whenever you discard a card, exile that card from your graveyard.",
                "Pay 1 life: Exile the top card of your library face down. Put that card into your hand at the beginning of your next end step.",
            ],
            |d, _| {
                d.skip_draw_step = true;
                let that = Filter { triggering: true, ..graveyard_cards(false, Controller::You) };
                on(d, When::YouDiscard, vec![that], vec![Effect::Exile(0)]);
                let n = activated(d, "", false, vec![], vec![Effect::ExileTopFaceDownToHandAtYourNextEndStep]);
                d.activated[n].life = 1;
            },
        ),
        // ---- Core: triggers, batch 14 ----
        "Pact of Negation" => s(
            &[
                "Counter target spell.",
                "At the beginning of your next upkeep, pay {3}{U}{U}. If you don't, you lose the game.",
            ],
            |d, _| {
                let trigger = d.triggers.len();
                spell(d, vec![SPELL], vec![Effect::Counter(0), Effect::DelayedAtYourNextUpkeep { trigger }]);
                // Only from the delayed entry.
                let cost = ManaCost::parse("{3}{U}{U}").expect("a mana cost");
                on(d, When::Never, vec![], vec![Effect::PayOrLoseGame(cost)]);
            },
        ),
        "Sigil of the Empty Throne" => s(
            &["Whenever you cast an enchantment spell, create a 4/4 white Angel creature token with flying."],
            |d, t| on(d, When::YouCastEnchantment, vec![], vec![Effect::CreateToken { who: Who::You, def: t.angel_4, n: 1 }]),
        ),
        "Summoning Station" => s(
            &[
                "{T}: Create a 2/2 colorless Pincher creature token.",
                "Whenever an artifact is put into a graveyard from the battlefield, you may untap this artifact.",
            ],
            |d, t| {
                activated(d, "", true, vec![], vec![Effect::CreateToken { who: Who::You, def: t.pincher, n: 1 }]);
                on(d, When::ArtifactToGraveyardFromBattlefield, vec![], vec![Effect::UntapSelf]);
                let n = d.triggers.len() - 1;
                d.triggers[n].ability.optional = true;
            },
        ),
        _ => None,
    }
}

/// "target opponent"
fn opponent_player() -> Filter {
    Filter::PLAYER.controlled_by(Controller::Opponent)
}

const SHOCK_LAND: &[&str] = &["As this land enters, you may pay 2 life. If you don't, it enters tapped."];

/// A shock land: taps for either of its two basic types' colors.
fn shock_land(d: &mut CardDef, a: Color, b: Color) {
    d.mana = Some(ManaSource::of(&[a, b]));
    d.shock_land = true;
}

const FREE_WITH_COMMANDER: &str =
    "If you control a commander, you may cast this spell without paying its mana cost.";

fn free_with_commander(d: &mut CardDef) {
    d.alt_cost = Some((ManaCost::default(), 0));
    d.alt_cost_needs_commander = true;
}

/// "{T}: Add {C}." and "{A/B}, {T}: Add {A}{A}, {A}{B}, or {B}{B}." (the filter lands)
fn filter_land(d: &mut CardDef, cost: &str, a: Color, b: Color) {
    d.mana = Some(ManaSource {
        colorless: true,
        ..ManaSource::default()
    });
    let ab = [a, b];
    d.paid_mana = Some((
        ManaCost::parse(cost).expect("a mana cost"),
        [ManaSource::of(&ab), ManaSource::of(&ab)],
    ));
}

const BATTLEBOND_LAND: &str = "This land enters tapped unless you have two or more opponents.";

/// "{T}: Add {C}." and "{T}: Add {A} or {B}. This deals 1 damage to you." (pain lands, Talismans)
fn pain_mana(d: &mut CardDef, a: Color, b: Color) {
    d.mana = Some(ManaSource {
        colorless: true,
        ..ManaSource::of(&[a, b])
    });
    d.pain_colored = true;
}

/// This lane's scripted playgroup cards (checked against their text in `lib.rs`).
pub const PG_D2_SCRIPTED: &[&str] = &[
    // Core: triggers, batch 1
    "Lifeblood Hydra",
    "Mana Vault",
    "Howling Mine",
    "Loyal Drake",
    "Aetherflux Reservoir",
    "Hydroid Krasis",
    "Malakir Rebirth // Malakir Mire",
    "Breena, the Demagogue",
    "Burgeoning",
    // Core: triggers, batch 2
    "Reanimate",
    "Feed the Swarm",
    "Darkstar Augur",
    "Castle Locthwain",
    "Eldritch Pact",
    "Dark Deal",
    "Graveborn Muse",
    "Disciple of Bolas",
    "Darigaaz's Caldera",
    // Core: triggers, batch 3
    "Sanguine Bond",
    "Vito, Thorn of the Dusk Rose",
    "Defiant Bloodlord",
    "Pitiless Plunderer",
    "Elas il-Kor, Sadistic Pilgrim",
    "Undead Augur",
    "Disciple of the Vault",
    "Viridian Revel",
    "Feast of Sanity",
    "Shadowstorm Vizier",
    "Drake Haven",
    "Enchantress's Presence",
    "Argothian Enchantress",
    "Satyr Enchanter",
    "Mesa Enchantress",
    "Verduran Enchantress",
    "Sythis, Harvest's Hand",
    "Mirran Spy",
    "Revel in Riches",
    "Yahenni, Undying Partisan",
    "Herd Baloth",
    "Scurry Oak",
    "Vilis, Broker of Blood",
    // Core: triggers, batch 4
    "Hushbringer",
    "Doorkeeper Thrull",
    "Elesh Norn, Mother of Machines",
    "Harmonic Prodigy",
    // Core: triggers, batch 5
    "Mazirek, Kraul Death Priest",
    "Mirkwood Bats",
    "Al Bhed Salvagers",
    "Altar of the Brood",
    "Ivy Lane Denizen",
    "Wound Reflection",
    // Core: triggers, batch 6
    "Witherbloom Apprentice",
    "Glimpse of Nature",
    "Zenith Chronicler",
    "No Mercy",
    "Francisco, Fowl Marauder",
    "Syr Konrad, the Grim",
    "Species Specialist",
    "Bloodchief Ascension",
    "Cryptcaller Chariot",
    // Core: triggers, batch 7
    "Ghostly Pilferer",
    // Core: triggers, batch 9
    "Archmage Emeritus",
    "Lobber Crew",
    "Chandra's Spitfire",
    "Curiosity",
    "Ophidian Eye",
    // Core: triggers, batch 10
    "Witch of the Moors",
    "Felidar Sovereign",
    "Test of Endurance",
    "Pyrohemia",
    "Vizkopa Guildmage",
    "Amulet of Vigor",
    "Skittering Precursor",
    "It That Betrays",
    "Glaring Fleshraker",
    "Aurelia, the Warleader",
    "Ghyrson Starn, Kelermorph",
    "Graf Mole",
    // Core: triggers, batch 12
    "Sefris of the Hidden Ways",
    "Lunar Convocation",
    "Saffi Eriksdotter",
    // Core: triggers, batch 13
    "Sheoldred, Whispering One",
    "Necropotence",
    // Core: triggers, batch 14
    "Pact of Negation",
    "Sigil of the Empty Throne",
    "Summoning Station",
    // The Gitrog, Ravenous Ride
    "Demonic Tutor",
    "Exquisite Blood",
    "Goldvein Hydra",
    "Grave Pact",
    "Rotting Regisaur",
    "Voracious Hydra",
    "Daemogoth Titan",
    "Fleetfeather Sandals",
    "Lantern of the Lost",
    // Kaalia of the Vast
    "Blood Crypt",
    "Sacred Foundry",
    "Deadly Rollick",
    "Flawless Maneuver",
    "Fetid Heath",
    "Graven Cairns",
    "Dragonskull Summit",
    "Vampiric Tutor",
    "Luxury Suite",
    "Spectator Seating",
    "Talisman of Indulgence",
    "Talisman of Hierarchy",
    "Sulfurous Springs",
    "Mortify",
    "Pestilence Demon",
    "Akroma, Angel of Wrath",
    "Flagstones of Trokair",
    "Shinka, the Bloodsoaked Keep",
    // Henrika Domnathi
    "Diabolic Intent",
    "Cordial Vampire",
    "Sheoldred, the Apocalypse",
    "Diabolic Edict",
    "Diresight",
    "Kokusho, the Evening Star",
    // Korvold, Fae-Cursed King
    "Deserted Temple",
    "Rakdos Carnarium",
    "Savage Lands",
    "Shadowblood Ridge",
    "Soul's Fire",
    "Thrór's Map",
    // Damia, Sage of Stone
    "Rejuvenating Springs",
    "Morphic Pool",
    "Simic Signet",
    "Underground River",
    "Sunken Ruins",
    "Leyline of Anticipation",
    "Okina, Temple to the Grandfathers",
    // Raffine, Scheming Seer
    "Bitterbloom Bearer",
    "Queza, Augur of Agonies",
    "Vault of the Archangel",
];
