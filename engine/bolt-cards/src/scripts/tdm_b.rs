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
];
