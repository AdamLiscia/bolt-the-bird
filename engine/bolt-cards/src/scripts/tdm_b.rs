//! Tarkir: Dragonstorm, lane B: plain cards on existing engine rules (card scripts only; any
//! card needing engine-core work goes back to the main lane). Tests: tests/tdm_b.rs.
#![allow(unused_imports)]

use super::*;

/// "This land enters tapped." / "{T}: Add {A}, {B}, or {C}." (the tri-lands).
fn tri_land(d: &mut CardDef, colors: &[Color]) {
    d.enters_tapped = true;
    d.mana = Some(ManaSource::of(colors));
}

const ENCHANTMENT: Filter = Filter {
    players: false,
    creatures: false,
    planeswalkers: false,
    enchantments: true,
    ..Filter::ANY
};

/// "This land enters tapped unless you control a [type] or a [type]." / "{T}: Add {C}."
fn slow_land(d: &mut CardDef, unless: [&'static str; 2], color: Color) {
    d.enters_tapped_unless = Some(unless);
    d.mana = Some(ManaSource::of(&[color]));
}

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        "Heritage Reclamation" => s(
            &[
                "Choose one —",
                "• Destroy target artifact.",
                "• Destroy target enchantment.",
                "• Exile up to one target card from a graveyard. Draw a card.",
            ],
            |d, _| {
                modes(
                    d,
                    vec![
                        Ability::new(vec![ARTIFACT], vec![Effect::Destroy(0)]),
                        Ability::new(vec![ENCHANTMENT], vec![Effect::Destroy(0)]),
                        up_to(vec![graveyard_cards(false, Controller::Any)], vec![Effect::Exile(0), draw(1)]),
                    ],
                )
            },
        ),
        "Cruel Truths" => s(&["Surveil 2, then draw two cards. You lose 2 life."], |d, _| {
            spell(
                d,
                vec![],
                vec![Effect::Surveil { n: 2 }, draw(2), Effect::LoseLife { who: Who::You, n: 2 }],
            )
        }),
        "Alesha's Legacy" => s(
            &["Target creature you control gains deathtouch and indestructible until end of turn."],
            |d, _| {
                let k = Keywords::DEATHTOUCH.with(Keywords::INDESTRUCTIBLE);
                spell(d, vec![YOUR_CREATURE], vec![Effect::Grant { slot: 0, keywords: k }])
            },
        ),
        "Worthy Cost" => s(
            &["As an additional cost to cast this spell, sacrifice a creature.", "Exile target creature or planeswalker."],
            |d, _| {
                d.extra_cost = Some(ExtraCost::SacrificeCreature);
                spell(d, vec![CREATURE_OR_PLANESWALKER], vec![Effect::Exile(0)]);
            },
        ),
        "Bewildering Blizzard" => s(
            &["Draw three cards. Creatures your opponents control get -3/-0 until end of turn."],
            |d, _| {
                spell(
                    d,
                    vec![],
                    vec![draw(3), Effect::PumpAll { who: Who::EachOpponent, power: -3, toughness: 0 }],
                )
            },
        ),
        "Frontier Bivouac" => s(&["This land enters tapped.", "{T}: Add {G}, {U}, or {R}."], |d, _| {
            tri_land(d, &[Color::Green, Color::Blue, Color::Red])
        }),
        "Mystic Monastery" => s(&["This land enters tapped.", "{T}: Add {U}, {R}, or {W}."], |d, _| {
            tri_land(d, &[Color::Blue, Color::Red, Color::White])
        }),
        "Nomad Outpost" => s(&["This land enters tapped.", "{T}: Add {R}, {W}, or {B}."], |d, _| {
            tri_land(d, &[Color::Red, Color::White, Color::Black])
        }),
        "Sandsteppe Citadel" => s(&["This land enters tapped.", "{T}: Add {W}, {B}, or {G}."], |d, _| {
            tri_land(d, &[Color::White, Color::Black, Color::Green])
        }),
        "Knockout Maneuver" => s(
            &["Put a +1/+1 counter on target creature you control, then it deals damage equal to its power to target creature an opponent controls."],
            |d, _| {
                spell(
                    d,
                    vec![YOUR_CREATURE, OPP_CREATURE],
                    vec![Effect::Counters { slot: 0, n: 1 }, Effect::Bite { from: 0, to: 1 }],
                )
            },
        ),
        "Shocking Sharpshooter" => s(
            &["Whenever another creature you control enters, this creature deals 1 damage to target opponent."],
            |d, _| {
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                on(d, When::AnotherCreatureYouControlEnters, vec![opp], vec![damage(1, Recipient::Target(0))])
            },
        ),
        "Auroral Procession" => s(&["Return target card from your graveyard to your hand."], |d, _| {
            spell(d, vec![graveyard_cards(false, Controller::You)], vec![Effect::Bounce(0)])
        }),
        "Gurmag Rakshasa" => s(
            &["When this creature enters, target creature an opponent controls gets -2/-2 until end of turn and target creature you control gets +2/+2 until end of turn."],
            |d, _| {
                on(
                    d,
                    When::Enters,
                    vec![OPP_CREATURE, YOUR_CREATURE],
                    vec![
                        Effect::Pump { slot: 0, power: -2, toughness: -2 },
                        Effect::Pump { slot: 1, power: 2, toughness: 2 },
                    ],
                )
            },
        ),
        "Skirmish Rhino" => s(
            &["When this creature enters, each opponent loses 2 life and you gain 2 life."],
            |d, _| {
                on(
                    d,
                    When::Enters,
                    vec![],
                    vec![Effect::LoseLife { who: Who::EachOpponent, n: 2 }, Effect::GainLife { who: Who::You, n: 2 }],
                )
            },
        ),
        "Inspired Ultimatum" => s(
            &["Target player gains 5 life, Inspired Ultimatum deals 5 damage to any target, then you draw five cards."],
            |d, _| {
                spell(
                    d,
                    vec![Filter::PLAYER, Filter::ANY],
                    vec![Effect::GainLife { who: Who::TargetPlayer(0), n: 5 }, damage(5, Recipient::Target(1)), draw(5)],
                )
            },
        ),
        // ---- Lane B batch 2: slow lands, becomes tapped, a creature with a counter, X = creatures ----
        "Cori Mountain Monastery" => s(
            &[
                "This land enters tapped unless you control a Plains or an Island.",
                "{T}: Add {R}.",
                "{3}{R}, {T}: Exile the top card of your library. Until the end of your next turn, you may play that card.",
            ],
            |d, _| {
                slow_land(d, ["Plains", "Island"], Color::Red);
                activated(d, "{3}{R}", true, vec![], vec![Effect::ExileTopChooseOneToPlay { n: 1, this_turn: false }]);
            },
        ),
        "Kishla Village" => s(
            &["This land enters tapped unless you control an Island or a Swamp.", "{T}: Add {G}.", "{3}{G}, {T}: Surveil 2."],
            |d, _| {
                slow_land(d, ["Island", "Swamp"], Color::Green);
                activated(d, "{3}{G}", true, vec![], vec![Effect::Surveil { n: 2 }]);
            },
        ),
        "Rescue Leopard" => s(
            &["Whenever this creature becomes tapped, you may discard a card. If you do, draw a card."],
            |d, _| on(d, When::BecomesTapped, vec![], vec![Effect::MayDiscardToDraw]),
        ),
        "Traveling Botanist" => s(
            &["Whenever this creature becomes tapped, look at the top card of your library. If it's a land card, you may reveal it and put it into your hand. If you don't put the card into your hand, you may put it into your graveyard."],
            |d, _| {
                let look = Effect::LookAtTop { n: 1, pick: Pick::Land, chosen_to: LookTo::Hand, rest_to: LookRest::MayGraveyard };
                on(d, When::BecomesTapped, vec![], vec![look]);
            },
        ),
        "Trade Route Envoy" => s(
            &["When this creature enters, draw a card if you control a creature with a counter on it. If you don't draw a card this way, put a +1/+1 counter on this creature."],
            |d, _| on(d, When::Enters, vec![], vec![Effect::DrawIfCounterElseCounterSelf]),
        ),
        "Delta Bloodflies" => s(
            &["Whenever this creature attacks, if you control a creature with a counter on it, each opponent loses 1 life."],
            |d, _| {
                conditional(
                    d,
                    When::Attacks,
                    Condition::ControlCreatureWithCounter,
                    vec![],
                    vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }],
                )
            },
        ),
        "Formation Breaker" => s(
            &[
                "Creatures with power less than this creature's power can't block it.",
                "As long as you control a creature with a counter on it, this creature gets +1/+2.",
            ],
            |d, _| {
                d.not_blocked_by_lesser_power = true;
                static_on(d, Scope::This, false, Keywords::NONE);
                let st = d.statics.last_mut().expect("just added");
                st.condition = Some(StaticCondition::ControlCreatureWithCounter);
                st.power = 1;
                st.toughness = 2;
            },
        ),
        "Coordinated Maneuver" => s(
            &[
                "Choose one —",
                "• Coordinated Maneuver deals damage equal to the number of creatures you control to target creature or planeswalker.",
                "• Destroy target enchantment.",
            ],
            |d, _| {
                modes(
                    d,
                    vec![
                        Ability::new(vec![CREATURE_OR_PLANESWALKER], vec![Effect::DamagePerCreatures(0)]),
                        Ability::new(vec![ENCHANTMENT], vec![Effect::Destroy(0)]),
                    ],
                )
            },
        ),
        "Frontline Rush" => s(
            &[
                "Choose one —",
                "• Create two 1/1 red Goblin creature tokens.",
                "• Target creature gets +X/+X until end of turn, where X is the number of creatures you control.",
            ],
            |d, t| {
                modes(
                    d,
                    vec![
                        Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.goblin, n: 2 }]),
                        Ability::new(vec![CREATURE], vec![Effect::PumpPerCreatures { slot: 0, attacking: false }]),
                    ],
                )
            },
        ),
        "Marshal of the Lost" => s(
            &["Whenever you attack, target creature gets +X/+X until end of turn, where X is the number of attacking creatures."],
            |d, _| on(d, When::YouAttack, vec![CREATURE], vec![Effect::PumpPerCreatures { slot: 0, attacking: true }]),
        ),
        "Craterhoof Behemoth" => s(
            &["When this creature enters, creatures you control gain trample and get +X/+X until end of turn, where X is the number of creatures you control."],
            |d, _| {
                on(
                    d,
                    When::Enters,
                    vec![],
                    vec![Effect::GrantAll { who: Who::You, keywords: Keywords::TRAMPLE }, Effect::PumpYoursPerCreatures],
                )
            },
        ),
        "Duty Beyond Death" => s(
            &[
                "As an additional cost to cast this spell, sacrifice a creature.",
                "Creatures you control gain indestructible until end of turn. Put a +1/+1 counter on each creature you control.",
            ],
            |d, _| {
                d.extra_cost = Some(ExtraCost::SacrificeCreature);
                spell(
                    d,
                    vec![],
                    vec![
                        Effect::GrantAll { who: Who::You, keywords: Keywords::INDESTRUCTIBLE },
                        Effect::CountersAll { who: Who::You, n: 1 },
                    ],
                );
            },
        ),
        // ---- Lane B batch 3: if you cast it, by name, can't be countered, hexproof until damage ----
        "Wingspan Stride" => s(
            &["Enchant creature", "Enchanted creature gets +1/+1 and has flying.", "{2}{U}: Return this Aura to its owner's hand."],
            |d, _| {
                d.aura = true;
                d.spell = Some(Ability::new(vec![CREATURE], vec![]));
                attached(d, false, 1, 1, Keywords::FLYING);
                activated(d, "{2}{U}", false, vec![], vec![Effect::BounceSelf]);
            },
        ),
        "Mistrise Village" => s(
            &[
                "This land enters tapped unless you control a Mountain or a Forest.",
                "{T}: Add {U}.",
                "{U}, {T}: The next spell you cast this turn can't be countered.",
            ],
            |d, _| {
                slow_land(d, ["Mountain", "Forest"], Color::Blue);
                activated(d, "{U}", true, vec![], vec![Effect::NextSpellUncounterable]);
            },
        ),
        "Tempest Hawk" => s(
            &[
                "Whenever this creature deals combat damage to a player, you may search your library for a card named Tempest Hawk, reveal it, put it into your hand, then shuffle.",
                "A deck can have any number of cards named Tempest Hawk.",
            ],
            |d, _| {
                let hawk = search(SearchFor::Named(&["Tempest Hawk"]), SearchTo::Hand);
                may(d, When::DealsCombatDamageToPlayer, vec![], vec![hawk]);
            },
        ),
        "Dragonstorm Forecaster" => s(
            &["{2}, {T}: Search your library for a card named Dragonstorm Globe or Boulderborn Dragon, reveal it, put it into your hand, then shuffle."],
            |d, _| {
                let find = search(SearchFor::Named(&["Dragonstorm Globe", "Boulderborn Dragon"]), SearchTo::Hand);
                activated(d, "{2}", true, vec![], vec![find]);
            },
        ),
        "Iridescent Tiger" => s(&["When this creature enters, if you cast it, add {W}{U}{B}{R}{G}."], |d, _| {
            let mana = Color::ALL.iter().map(|&color| Effect::AddMana { color, n: 1 }).collect();
            conditional(d, When::Enters, Condition::CastIt, vec![], mana);
        }),
        "Yathan Roadwatcher" => s(
            &["When this creature enters, if you cast it, mill four cards. When you do, return target creature card with mana value 3 or less from your graveyard to the battlefield."],
            |d, _| {
                conditional(
                    d,
                    When::Enters,
                    Condition::CastIt,
                    vec![],
                    vec![Effect::Mill { who: Who::You, n: 4 }, Effect::Reflexive(1)],
                );
                let card = Filter { max_mana_value: Some(3), ..graveyard_cards(true, Controller::You) };
                on(d, When::Never, vec![card], vec![Effect::Reanimate(0)]);
            },
        ),
        "Unrooted Ancestor" => s(
            &["{1}, Sacrifice another creature: This creature gains indestructible until end of turn. Tap it."],
            |d, _| {
                let n = activated(d, "{1}", false, vec![], vec![Effect::GrantSelf { keywords: Keywords::INDESTRUCTIBLE }, Effect::TapSelf]);
                d.activated[n].sacrifice_other_creature = true;
            },
        ),
        "Strategic Betrayal" => s(&["Target opponent exiles a creature they control and their graveyard."], |d, _| {
            let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
            spell(
                d,
                vec![opp],
                vec![Effect::PlayerExilesCreature(0), Effect::ExileGraveyard { who: Who::TargetPlayer(0) }],
            )
        }),
        "Lie in Wait" => s(
            &["Return target creature card from your graveyard to your hand. Lie in Wait deals damage equal to that card's power to target creature."],
            |d, _| {
                spell(
                    d,
                    vec![graveyard_cards(true, Controller::You), CREATURE],
                    vec![Effect::Bounce(0), Effect::DamageEqualCardPower { card: 0, to: 1 }],
                )
            },
        ),
        "Effortless Master" => s(
            &["This creature enters with two +1/+1 counters on it if you've cast two or more spells this turn."],
            |d, _| d.enters_counters = Some((2, Condition::CastTwoSpellsThisTurn)),
        ),
        "Karakyk Guardian" => s(&["This creature has hexproof as long as it hasn't dealt damage yet."], |d, _| {
            static_on(d, Scope::This, false, Keywords::HEXPROOF);
            d.statics.last_mut().expect("just added").condition = Some(StaticCondition::HasntDealtDamage);
        }),
        "Clarion Conqueror" => s(
            &["Activated abilities of artifacts, creatures, and planeswalkers can't be activated."],
            |d, _| d.stops_activated_abilities = true,
        ),
        // ---- Lane B batch 4: first counters each turn, delayed attack trigger, one or both, more ----
        "Stalwart Successor" => s(
            &["Whenever one or more counters are put on a creature you control, if it's the first time counters have been put on that creature this turn, put a +1/+1 counter on that creature."],
            |d, _| {
                let that = Filter { triggering: true, ..CREATURE };
                on(d, When::FirstCountersOnYourCreatureThisTurn, vec![that], vec![Effect::Counters { slot: 0, n: 1 }]);
            },
        ),
        "Embermouth Sentinel" => s(
            &["When this creature enters, you may search your library for a basic land card, reveal it, then shuffle and put that card on top. If you control a Dragon, put that card onto the battlefield tapped instead."],
            |d, _| {
                let find = search(SearchFor::BasicLand, SearchTo::TopOrTappedIfYouControl("Dragon"));
                may(d, When::Enters, vec![], vec![find]);
            },
        ),
        "Seize Opportunity" => s(
            &[
                "Choose one —",
                "• Exile the top two cards of your library. Until the end of your next turn, you may play those cards.",
                "• Up to two target creatures each get +2/+1 until end of turn.",
            ],
            |d, _| {
                modes(
                    d,
                    vec![
                        Ability::new(vec![], vec![Effect::ExileTopToPlayAll(2)]),
                        up_to(
                            vec![CREATURE, CREATURE],
                            vec![Effect::Pump { slot: 0, power: 2, toughness: 1 }, Effect::Pump { slot: 1, power: 2, toughness: 1 }],
                        ),
                    ],
                )
            },
        ),
        "Overwhelming Surge" => s(
            &[
                "Choose one or both —",
                "• Overwhelming Surge deals 3 damage to target creature.",
                "• Destroy target noncreature artifact.",
            ],
            |d, _| {
                // "One or both": each mode alone, or both together (CR 700.2).
                let artifact = Filter { noncreature: true, ..ARTIFACT };
                modes(
                    d,
                    vec![
                        Ability::new(vec![CREATURE], vec![damage(3, Recipient::Target(0))]),
                        Ability::new(vec![artifact], vec![Effect::Destroy(0)]),
                        Ability::new(vec![CREATURE, artifact], vec![damage(3, Recipient::Target(0)), Effect::Destroy(1)]),
                    ],
                )
            },
        ),
        "Salt Road Skirmish" => s(
            &["Destroy target creature. Create two 1/1 red Warrior creature tokens. They gain haste until end of turn. Sacrifice them at the beginning of the next end step."],
            |d, t| {
                spell(
                    d,
                    vec![CREATURE],
                    vec![
                        Effect::Destroy(0),
                        Effect::CreateToken { who: Who::You, def: t.warrior, n: 2 },
                        Effect::GrantCreated(Keywords::HASTE),
                        Effect::SacrificeCreatedAtEnd,
                    ],
                )
            },
        ),
        "Sonic Shrieker" => s(
            &["When this creature enters, it deals 2 damage to any target and you gain 2 life. If a player is dealt damage this way, they discard a card."],
            |d, _| {
                on(
                    d,
                    When::Enters,
                    vec![Filter::ANY],
                    vec![damage(2, Recipient::Target(0)), Effect::GainLife { who: Who::You, n: 2 }, Effect::DiscardIfPlayerTarget(0)],
                )
            },
        ),
        "Fresh Start" => s(&["Enchant creature", "Enchanted creature gets -5/-0 and loses all abilities."], |d, _| {
            d.aura = true;
            d.spell = Some(Ability::new(vec![CREATURE], vec![]));
            attached(d, false, -5, 0, Keywords::NONE);
            d.statics.last_mut().expect("just added").lose_abilities = true;
        }),
        "Ambling Stormshell" => s(
            &[
                "Ward {2}",
                "Whenever this creature attacks, put three stun counters on it and draw three cards.",
                "Whenever you cast a Turtle spell, untap this creature.",
            ],
            |d, _| {
                d.ward = ManaCost::parse("{2}");
                on(d, When::Attacks, vec![], vec![Effect::StunSelf(3), draw(3)]);
                on(d, When::YouCastSubtype("Turtle"), vec![], vec![Effect::UntapSelf]);
            },
        ),
        "Eshki Dragonclaw" => s(
            &[
                "Vigilance, trample, ward {1}",
                "At the beginning of combat on your turn, if you've cast both a creature spell and a noncreature spell this turn, draw a card and put two +1/+1 counters on Eshki Dragonclaw.",
            ],
            |d, _| {
                d.keywords = d.keywords.with(Keywords::VIGILANCE).with(Keywords::TRAMPLE);
                d.ward = ManaCost::parse("{1}");
                conditional(
                    d,
                    When::BeginCombatYourTurn,
                    Condition::CastCreatureAndNoncreatureThisTurn,
                    vec![],
                    vec![draw(1), Effect::CountersSelf { n: 2 }],
                );
            },
        ),
        "Dalkovan Encampment" => s(
            &[
                "This land enters tapped unless you control a Swamp or a Mountain.",
                "{T}: Add {W}.",
                "{2}{W}, {T}: Whenever you attack this turn, create two 1/1 red Warrior creature tokens that are tapped and attacking. Sacrifice them at the beginning of the next end step.",
            ],
            |d, t| {
                slow_land(d, ["Swamp", "Mountain"], Color::White);
                // Trigger 0, made by the delayed ability: Mobilize's tokens are exactly these.
                on(d, When::Never, vec![], vec![Effect::Mobilize { def: t.warrior, n: Some(2) }]);
                activated(d, "{2}{W}", true, vec![], vec![Effect::DelayedOnAttackThisTurn(0)]);
            },
        ),
        "Great Arashin City" => s(
            &[
                "This land enters tapped unless you control a Forest or a Plains.",
                "{T}: Add {B}.",
                "{1}{B}, {T}, Exile a creature card from your graveyard: Create a 1/1 white Spirit creature token.",
            ],
            |d, t| {
                slow_land(d, ["Forest", "Plains"], Color::Black);
                let n = activated(d, "{1}{B}", true, vec![], vec![Effect::CreateToken { who: Who::You, def: t.spirit_1, n: 1 }]);
                d.activated[n].exile_creature_card = true;
            },
        ),
        // ---- Lane B batch 5: granted abilities, last-known counters, dies under your control, rares ----
        "Ringing Strike Mastery" => s(
            &[
                "Enchant creature",
                "When this Aura enters, tap enchanted creature.",
                "Enchanted creature doesn't untap during its controller's untap step.",
                "Enchanted creature has \"{5}: Untap this creature.\"",
            ],
            |d, _| {
                d.aura = true;
                d.spell = Some(Ability::new(vec![CREATURE], vec![]));
                on(d, When::Enters, vec![], vec![Effect::TapAttached]);
                d.enchanted_doesnt_untap = true;
                let n = activated(d, "{5}", false, vec![], vec![Effect::UntapExtra]);
                d.activated[n].granted_to_attached = true;
            },
        ),
        "Desperate Measures" => s(
            &["Target creature gets +1/-1 until end of turn. When it dies under your control this turn, draw two cards."],
            |d, _| {
                spell(
                    d,
                    vec![CREATURE],
                    vec![Effect::Pump { slot: 0, power: 1, toughness: -1 }, Effect::GrantDiesUnderYourControl { slot: 0, trigger: 0 }],
                );
                on(d, When::Never, vec![], vec![draw(2)]);
            },
        ),
        "Host of the Hereafter" => s(
            &[
                "This creature enters with two +1/+1 counters on it.",
                "Whenever this creature or another creature you control dies, if it had counters on it, put its counters on up to one target creature you control.",
            ],
            |d, _| {
                d.enters_counters = Some((2, Condition::Always));
                d.triggers.push(Trigger {
                    when: When::CreatureYouControlWithCountersDies,
                    ability: up_to(vec![YOUR_CREATURE], vec![Effect::PutDeadCountersOn(0)]),
                });
            },
        ),
        "Herd Heirloom" => s(
            &[
                "{T}: Add one mana of any color. Spend this mana only to cast a creature spell.",
                "{T}: Until end of turn, target creature you control with power 4 or greater gains trample and \"Whenever this creature deals combat damage to a player, draw a card.\"",
            ],
            |d, _| {
                d.mana = Some(ManaSource::of(&Color::ALL));
                d.mana_only_for_creatures = true;
                on(d, When::Never, vec![], vec![draw(1)]);
                let big = Filter { creature_min_power: Some(4), ..YOUR_CREATURE };
                activated(
                    d,
                    "",
                    true,
                    vec![big],
                    vec![Effect::Grant { slot: 0, keywords: Keywords::TRAMPLE }, Effect::GrantCombatDamageTrigger { slot: 0, trigger: 0 }],
                );
            },
        ),
        "Jeskai Revelation" => s(
            &["Return target spell or permanent to its owner's hand. Jeskai Revelation deals 4 damage to any target. Create two 1/1 white Monk creature tokens with prowess. Draw two cards. You gain 4 life."],
            |d, t| {
                let spell_or_permanent = Filter { players: false, creatures: false, planeswalkers: false, spells: true, any_permanent: true, ..Filter::ANY };
                spell(
                    d,
                    vec![spell_or_permanent, Filter::ANY],
                    vec![
                        Effect::BounceSpellOrPermanent(0),
                        damage(4, Recipient::Target(1)),
                        Effect::CreateToken { who: Who::You, def: t.monk, n: 2 },
                        draw(2),
                        Effect::GainLife { who: Who::You, n: 4 },
                    ],
                )
            },
        ),
        "Felothar, Dawn of the Abzan" => s(
            &["Whenever Felothar enters or attacks, you may sacrifice a nonland permanent. When you do, put a +1/+1 counter on each creature you control."],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::MaySacrificeNonland, Effect::Reflexive(2)]);
                on(d, When::Attacks, vec![], vec![Effect::MaySacrificeNonland, Effect::Reflexive(2)]);
                on(d, When::Never, vec![], vec![Effect::CountersAll { who: Who::You, n: 1 }]);
            },
        ),
        "Lotuslight Dancers" => s(
            &["When this creature enters, search your library for a black card, a green card, and a blue card. Put those cards into your graveyard, then shuffle."],
            |d, _| {
                let find = |c: Color| search(SearchFor::Colored(c as u8), SearchTo::Graveyard);
                on(d, When::Enters, vec![], vec![find(Color::Black), find(Color::Green), find(Color::Blue)]);
            },
        ),
        "Narset, Jeskai Waymaster" => s(
            &["At the beginning of your end step, you may discard your hand. If you do, draw cards equal to the number of spells you've cast this turn."],
            |d, _| may(d, When::EndStep { your_turn_only: true }, vec![], vec![Effect::DiscardHandDrawPerSpells]),
        ),
        "Betor, Kin to All" => s(
            &["At the beginning of your end step, if creatures you control have total toughness 10 or greater, draw a card. Then if creatures you control have total toughness 20 or greater, untap each creature you control. Then if creatures you control have total toughness 40 or greater, each opponent loses half their life, rounded up."],
            |d, _| {
                conditional(
                    d,
                    When::EndStep { your_turn_only: true },
                    Condition::TotalToughnessAtLeast(10),
                    vec![],
                    vec![Effect::BetorEndStep],
                )
            },
        ),
        "Neriv, Heart of the Storm" => s(
            &["If a creature you control that entered this turn would deal damage, it deals twice that much damage instead."],
            |d, _| d.doubles_damage_of_new_creatures = true,
        ),
        // ---- Lane B batch 6: copies of permanent spells, storm, finality, rares ----
        "Sage of the Skies" => s(
            &["When you cast this spell, if you've cast another spell this turn, copy this spell."],
            |d, _| {
                conditional(d, When::YouCastThis, Condition::CastTwoSpellsThisTurn, vec![], vec![Effect::CopySourceSpell { storm: false }]);
            },
        ),
        "Stormscale Scion" => s(&["Other Dragons you control get +1/+1.", "Storm"], |d, _| {
            lord(d, "Dragon");
            on(d, When::YouCastThis, vec![], vec![Effect::CopySourceSpell { storm: true }]);
        }),
        "Magmatic Hellkite" => s(
            &["When this creature enters, destroy target nonbasic land an opponent controls. Its controller searches their library for a basic land card, puts it onto the battlefield tapped with a stun counter on it, then shuffles."],
            |d, _| {
                let land = Filter {
                    players: false,
                    creatures: false,
                    planeswalkers: false,
                    lands: true,
                    nonbasic: true,
                    controller: Controller::Opponent,
                    ..Filter::ANY
                };
                on(d, When::Enters, vec![land], vec![Effect::DestroyLandControllerFetchesStunned(0)]);
            },
        ),
        "Stillness in Motion" => s(
            &["At the beginning of your upkeep, mill three cards. Then if your library has no cards in it, exile this enchantment and put five cards from your graveyard on top of your library in any order."],
            |d, _| on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::Mill { who: Who::You, n: 3 }, Effect::StillnessCheck]),
        ),
        "Hundred-Battle Veteran" => s(
            &[
                "As long as there are three or more different kinds of counters among creatures you control, this creature gets +2/+4.",
                "You may cast this card from your graveyard. If you do, it enters with a finality counter on it.",
            ],
            |d, _| {
                static_on(d, Scope::This, false, Keywords::NONE);
                let st = d.statics.last_mut().expect("just added");
                st.condition = Some(StaticCondition::CounterKindsAmongYoursAtLeast(3));
                st.power = 2;
                st.toughness = 4;
                d.cast_from_graveyard_finality = true;
            },
        ),
        "Elspeth, Storm Slayer" => s(
            &[
                "If one or more tokens would be created under your control, twice that many of those tokens are created instead.",
                "+1: Create a 1/1 white Soldier creature token.",
                "0: Put a +1/+1 counter on each creature you control. Those creatures gain flying until your next turn.",
                "−3: Destroy target creature an opponent controls with mana value 3 or greater.",
            ],
            |d, t| {
                d.doubles_tokens = true;
                loyalty(d, 1, Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.soldier, n: 1 }]));
                loyalty(
                    d,
                    0,
                    Ability::new(
                        vec![],
                        vec![Effect::CountersAll { who: Who::You, n: 1 }, Effect::GrantAllUntilYourNextTurn(Keywords::FLYING)],
                    ),
                );
                let big = Filter { min_mana_value: Some(3), ..OPP_CREATURE };
                loyalty(d, -3, Ability::new(vec![big], vec![Effect::Destroy(0)]));
            },
        ),
        "Call the Spirit Dragons" => s(
            &[
                "Dragons you control have indestructible.",
                "At the beginning of your upkeep, for each color, put a +1/+1 counter on a Dragon you control of that color. If you put +1/+1 counters on five Dragons this way, you win the game.",
            ],
            |d, _| {
                static_on(d, Scope::OtherYourSubtype("Dragon"), false, Keywords::INDESTRUCTIBLE);
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::SpiritDragonsUpkeep]);
            },
        ),
        // ---- Lane B batch 7: delayed "this turn" triggers, Sagas, free casts, X Omen, ward—discard ----
        "Thunder of Unity" => s(
            &[
                "I — You draw two cards and you lose 2 life.",
                "II, III — Whenever a creature you control enters this turn, each opponent loses 1 life and you gain 1 life.",
            ],
            |d, _| {
                chapter(d, 1, Ability::new(vec![], vec![draw(2), Effect::LoseLife { who: Who::You, n: 2 }]));
                let delayed = Effect::DelayedThisTurn { when: When::CreatureYouControlEntersThisTurn, trigger: 3 };
                chapter(d, 2, Ability::new(vec![], vec![delayed]));
                chapter(d, 3, Ability::new(vec![], vec![delayed]));
                on(
                    d,
                    When::Never,
                    vec![],
                    vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }, Effect::GainLife { who: Who::You, n: 1 }],
                );
            },
        ),
        "Rediscover the Way" => s(
            &[
                "I, II — Look at the top three cards of your library. Put one of them into your hand and the rest on the bottom of your library in any order.",
                "III — Whenever you cast a noncreature spell this turn, target creature you control gains double strike until end of turn.",
            ],
            |d, _| {
                let look = Effect::LookAtTop { n: 3, pick: Pick::Any, chosen_to: LookTo::Hand, rest_to: LookRest::BottomAnyOrderTakeOne };
                chapter(d, 1, Ability::new(vec![], vec![look]));
                chapter(d, 2, Ability::new(vec![], vec![look]));
                chapter(d, 3, Ability::new(vec![], vec![Effect::DelayedThisTurn { when: When::YouCastNoncreature, trigger: 3 }]));
                on(d, When::Never, vec![YOUR_CREATURE], vec![Effect::Grant { slot: 0, keywords: Keywords::DOUBLE_STRIKE }]);
            },
        ),
        "Awaken the Honored Dead" => s(
            &[
                "I — Destroy target nonland permanent.",
                "II — Mill three cards.",
                "III — You may discard a card. When you do, return target creature or land card from your graveyard to your hand.",
            ],
            |d, _| {
                let nonland = Filter { controller: Controller::Any, ..OPP_NONLAND };
                chapter(d, 1, Ability::new(vec![nonland], vec![Effect::Destroy(0)]));
                chapter(d, 2, Ability::new(vec![], vec![Effect::Mill { who: Who::You, n: 3 }]));
                chapter(d, 3, Ability::new(vec![], vec![Effect::MayDiscardThen, Effect::Reflexive(3)]));
                let card = Filter { creature_or_land: true, ..graveyard_cards(false, Controller::You) };
                on(d, When::Never, vec![card], vec![Effect::Bounce(0)]);
            },
        ),
        "Bloomvine Regent" => s(
            &[
                "Whenever this creature or another Dragon you control enters, you gain 3 life.",
                "Omen: Search your library for up to two basic Forest cards, reveal them, put one onto the battlefield tapped and the other into your hand, then shuffle.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::GainLife { who: Who::You, n: 3 }]);
                on(d, When::AnotherYouControlEntersWithSubtype("Dragon"), vec![], vec![Effect::GainLife { who: Who::You, n: 3 }]);
                let forest = SearchFor::LandTypes { types: land_types(&["Forest"]), basic: true };
                omen(
                    d,
                    Ability::new(
                        vec![],
                        vec![search(forest, SearchTo::Battlefield { tapped: true }), search(forest, SearchTo::Hand)],
                    ),
                );
            },
        ),
        "Breaching Dragonstorm" => s(
            &[
                "When this enchantment enters, exile cards from the top of your library until you exile a nonland card. You may cast it without paying its mana cost if that spell's mana value is 8 or less. If you don't, put that card into your hand.",
                DRAGONSTORM_RETURN,
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::ExileUntilNonlandCastFree { max_mv: 8 }]);
                dragonstorm_return(d);
            },
        ),
        "Reverberating Summons" => s(
            &[
                "At the beginning of each combat, if you've cast two or more spells this turn, this enchantment becomes a 3/3 Monk creature with haste in addition to its other types until end of turn.",
                "{1}{R}, Discard your hand, Sacrifice this enchantment: Draw two cards.",
            ],
            |d, _| {
                let become_monk = Effect::BecomeCreatureUntilEot { power: 3, toughness: 3, subtype: "Monk", keywords: Keywords::HASTE };
                conditional(d, When::BeginCombatEachTurn, Condition::CastTwoSpellsThisTurn, vec![], vec![become_monk]);
                let n = activated(d, "{1}{R}", false, vec![], vec![draw(2)]);
                d.activated[n].discard_hand = true;
                d.activated[n].sacrifice_self = true;
            },
        ),
        "New Way Forward" => s(
            &["The next time a source of your choice would deal damage to you this turn, prevent that damage. When damage is prevented this way, New Way Forward deals that much damage to that source's controller and you draw that many cards."],
            |d, _| {
                spell(d, vec![], vec![Effect::PreventNextFromChosenSource { trigger: 0 }]);
                on(d, When::Never, vec![], vec![Effect::DamageExtrasControllerAndDrawAmount]);
            },
        ),
        _ => None,
    }
}

/// Lane B's scripted cards (checked against their text in `lib.rs`).
pub const TDM_B_SCRIPTED: &[&str] = &[
    "Heritage Reclamation",
    "Cruel Truths",
    "Alesha's Legacy",
    "Worthy Cost",
    "Bewildering Blizzard",
    "Frontier Bivouac",
    "Mystic Monastery",
    "Nomad Outpost",
    "Sandsteppe Citadel",
    "Knockout Maneuver",
    "Shocking Sharpshooter",
    "Auroral Procession",
    "Gurmag Rakshasa",
    "Skirmish Rhino",
    "Inspired Ultimatum",
    "Cori Mountain Monastery",
    "Kishla Village",
    "Rescue Leopard",
    "Traveling Botanist",
    "Trade Route Envoy",
    "Delta Bloodflies",
    "Formation Breaker",
    "Coordinated Maneuver",
    "Frontline Rush",
    "Marshal of the Lost",
    "Craterhoof Behemoth",
    "Duty Beyond Death",
    "Wingspan Stride",
    "Mistrise Village",
    "Tempest Hawk",
    "Dragonstorm Forecaster",
    "Iridescent Tiger",
    "Yathan Roadwatcher",
    "Unrooted Ancestor",
    "Strategic Betrayal",
    "Lie in Wait",
    "Effortless Master",
    "Karakyk Guardian",
    "Clarion Conqueror",
    "Stalwart Successor",
    "Embermouth Sentinel",
    "Seize Opportunity",
    "Overwhelming Surge",
    "Salt Road Skirmish",
    "Sonic Shrieker",
    "Fresh Start",
    "Ambling Stormshell",
    "Eshki Dragonclaw",
    "Dalkovan Encampment",
    "Great Arashin City",
    "Ringing Strike Mastery",
    "Desperate Measures",
    "Host of the Hereafter",
    "Herd Heirloom",
    "Jeskai Revelation",
    "Felothar, Dawn of the Abzan",
    "Lotuslight Dancers",
    "Narset, Jeskai Waymaster",
    "Betor, Kin to All",
    "Neriv, Heart of the Storm",
    "Sage of the Skies",
    "Stormscale Scion",
    "Magmatic Hellkite",
    "Stillness in Motion",
    "Hundred-Battle Veteran",
    "Elspeth, Storm Slayer",
    "Call the Spirit Dragons",
    "Thunder of Unity",
    "Rediscover the Way",
    "Awaken the Honored Dead",
    "Bloomvine Regent",
    "Breaching Dragonstorm",
    "Reverberating Summons",
    "New Way Forward",
];
