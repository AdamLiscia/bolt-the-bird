//! Tarkir: Dragonstorm, lane C: plain cards on existing engine rules (card scripts only; any
//! card needing engine-core work goes back to the main lane). Tests: tests/tdm_c.rs.
#![allow(unused_imports)]

use super::*;

const ENCHANTMENT: Filter = Filter {
    players: false,
    creatures: false,
    planeswalkers: false,
    enchantments: true,
    ..Filter::ANY
};

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        "Summit Intimidator" => s(&["When this creature enters, target creature can't block this turn."], |d, _| {
            on(d, When::Enters, vec![CREATURE], vec![Effect::Grant { slot: 0, keywords: Keywords::CANT_BLOCK }])
        }),
        "Arashin Sunshield" => s(
            &["When this creature enters, exile up to two target cards from a single graveyard.", "{W}, {T}: Tap target creature."],
            |d, _| {
                let f = graveyard_cards(false, Controller::Any);
                let mut ability = up_to(vec![f, f], vec![Effect::Exile(0), Effect::Exile(1)]);
                ability.same_graveyard = true;
                d.triggers.push(Trigger { when: When::Enters, ability });
                activated(d, "{W}", true, vec![CREATURE], vec![Effect::Tap(0)]);
            },
        ),
        "Lightfoot Technique" => s(
            &["Put a +1/+1 counter on target creature. It gains flying and indestructible until end of turn."],
            |d, _| {
                let k = Keywords::FLYING.with(Keywords::INDESTRUCTIBLE);
                spell(d, vec![CREATURE], vec![Effect::Counters { slot: 0, n: 1 }, Effect::Grant { slot: 0, keywords: k }])
            },
        ),
        "Unburied Earthcarver" => s(&["{2}, Sacrifice another creature: Put a +1/+1 counter on this creature."], |d, _| {
            let n = activated(d, "{2}", false, vec![], vec![Effect::CountersSelf { n: 1 }]);
            d.activated[n].sacrifice_other_creature = true;
        }),
        "Sunset Strikemaster" => s(
            &["{T}: Add {R}.", "{2}{R}, {T}, Sacrifice this creature: It deals 6 damage to target creature with flying."],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::Red]));
                let flier = Filter { creature_needs_flying: true, ..CREATURE };
                let n = activated(d, "{2}{R}", true, vec![flier], vec![damage(6, Recipient::Target(0))]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Opulent Palace" => s(&["This land enters tapped.", "{T}: Add {B}, {G}, or {U}."], |d, _| {
            d.enters_tapped = true;
            d.mana = Some(ManaSource::of(&[Color::Black, Color::Green, Color::Blue]));
        }),
        "Jeskai Shrinekeeper" => s(
            &["Whenever this creature deals combat damage to a player, you gain 1 life and draw a card."],
            |d, _| on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::GainLife { who: Who::You, n: 1 }, draw(1)]),
        ),
        "War Effort" => s(
            &[
                "Creatures you control get +1/+0.",
                "Whenever you attack, create a 1/1 red Warrior creature token that's tapped and attacking. Sacrifice it at the beginning of the next end step.",
            ],
            |d, t| {
                static_on(d, Scope::YourCreatures, false, Keywords::NONE);
                d.statics.last_mut().expect("just added").power = 1;
                // Mobilize's effect: one Warrior, tapped and attacking, sacrificed at the next end step.
                on(d, When::YouAttack, vec![], vec![Effect::Mobilize { def: t.warrior, n: Some(1) }]);
            },
        ),
        "Wail of War" => s(
            &[
                "Choose one —",
                "• Creatures target opponent controls get -1/-1 until end of turn.",
                "• Return up to two target creature cards from your graveyard to your hand.",
            ],
            |d, _| {
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                let f = graveyard_cards(true, Controller::You);
                modes(
                    d,
                    vec![
                        Ability::new(vec![opp], vec![Effect::PumpAll { who: Who::TargetPlayer(0), power: -1, toughness: -1 }]),
                        up_to(vec![f, f], vec![Effect::Bounce(0), Effect::Bounce(1)]),
                    ],
                )
            },
        ),
        "Jeskai Brushmaster" => s(&["Prowess"], |d, _| {
            on(d, When::YouCastNoncreature, vec![], vec![Effect::PumpSelf { power: 1, toughness: 1 }])
        }),
        "Starry-Eyed Skyrider" => s(
            &[
                "Whenever this creature attacks, another target creature you control gains flying until end of turn.",
                "Attacking tokens you control have flying.",
            ],
            |d, _| {
                let other = Filter { other: true, ..YOUR_CREATURE };
                on(d, When::Attacks, vec![other], vec![Effect::Grant { slot: 0, keywords: Keywords::FLYING }]);
                static_on(d, Scope::YourAttackingTokens, false, Keywords::FLYING);
            },
        ),
        "Loxodon Battle Priest" => s(
            &["At the beginning of combat on your turn, put a +1/+1 counter on another target creature you control."],
            |d, _| {
                let other = Filter { other: true, ..YOUR_CREATURE };
                on(d, When::BeginCombatYourTurn, vec![other], vec![Effect::Counters { slot: 0, n: 1 }]);
            },
        ),
        "Fangkeeper's Familiar" => s(
            &[
                "When this creature enters, choose one —",
                "• You gain 3 life and surveil 3.",
                "• Destroy target enchantment.",
                "• Counter target creature spell.",
            ],
            |d, _| {
                let creature_spell = Filter { spell_creature: Some(true), ..SPELL };
                on_modal(
                    d,
                    When::Enters,
                    None,
                    vec![
                        Ability::new(vec![], vec![Effect::GainLife { who: Who::You, n: 3 }, Effect::Surveil { n: 3 }]),
                        Ability::new(vec![ENCHANTMENT], vec![Effect::Destroy(0)]),
                        Ability::new(vec![creature_spell], vec![Effect::Counter(0)]),
                    ],
                );
            },
        ),
        "Inevitable Defeat" => s(
            &["This spell can't be countered.", "Exile target nonland permanent. Its controller loses 3 life and you gain 3 life."],
            |d, _| {
                d.uncounterable = true;
                let nonland = Filter { controller: Controller::Any, ..OPP_NONLAND };
                // The life loss reads the controller before the exile moves the card; same result.
                spell(
                    d,
                    vec![nonland],
                    vec![
                        Effect::LoseLife { who: Who::ControllerOf(0), n: 3 },
                        Effect::Exile(0),
                        Effect::GainLife { who: Who::You, n: 3 },
                    ],
                );
            },
        ),
        "Perennation" => s(
            &["Return target permanent card from your graveyard to the battlefield with a hexproof counter and an indestructible counter on it."],
            |d, _| {
                let card = Filter { permanent_cards: true, ..graveyard_cards(false, Controller::You) };
                // Keyword counters are kept as keywords it has for as long as it's on the battlefield.
                let k = Keywords::HEXPROOF.with(Keywords::INDESTRUCTIBLE);
                spell(d, vec![card], vec![Effect::ReturnWith { slot: Some(0), counters: 0, subtype: None, keywords: k }]);
            },
        ),
        // ---- lane C round 3: divided targets, damaged-this-turn, sacrifice a token, Aura enters, defender, mass removal ----
        "Twin Bolt" => s(&["Twin Bolt deals 2 damage divided as you choose among one or two targets."], |d, _| {
            let mut a = up_to(vec![Filter::ANY, Filter::ANY], vec![Effect::DividedDamage]);
            a.min_targets = Some(1);
            a.divide = 2;
            d.spell = Some(a);
        }),
        "Unsparing Boltcaster" => s(
            &["When this creature enters, it deals 5 damage to target creature an opponent controls that was dealt damage this turn."],
            |d, _| {
                let hurt = Filter { was_dealt_damage: true, ..OPP_CREATURE };
                on(d, When::Enters, vec![hurt], vec![damage(5, Recipient::Target(0))]);
            },
        ),
        "Smile at Death" => s(
            &["At the beginning of your upkeep, return up to two target creature cards with power 2 or less from your graveyard to the battlefield. Put a +1/+1 counter on each of those creatures."],
            |d, _| {
                let f = Filter { creature_max_power: Some(2), ..graveyard_cards(true, Controller::You) };
                let ability = up_to(
                    vec![f, f],
                    vec![
                        Effect::Reanimate(0),
                        Effect::Reanimate(1),
                        Effect::Counters { slot: 0, n: 1 },
                        Effect::Counters { slot: 1, n: 1 },
                    ],
                );
                d.triggers.push(Trigger { when: When::Upkeep { your_turn_only: true }, ability });
            },
        ),
        "Hardened Tactician" => s(&["{1}, Sacrifice a token: Draw a card."], |d, _| {
            let n = activated(d, "{1}", false, vec![], vec![draw(1)]);
            d.activated[n].sacrifice_token = true;
        }),
        "Fire-Rim Form" => s(
            &[
                "Enchant creature",
                "When this Aura enters, enchanted creature gains first strike until end of turn.",
                "Enchanted creature gets +2/+0.",
            ],
            |d, _| {
                d.aura = true;
                d.spell = Some(Ability::new(vec![CREATURE], vec![]));
                on(d, When::Enters, vec![], vec![Effect::GrantAttached(Keywords::FIRST_STRIKE)]);
                attached(d, false, 2, 0, Keywords::NONE);
            },
        ),
        "Krotiq Nestguard" => s(&["{2}{G}: This creature can attack this turn as though it didn't have defender."], |d, _| {
            activated(d, "{2}{G}", false, vec![], vec![Effect::AttackDespiteDefenderSelf]);
        }),
        "Ruinous Ultimatum" => s(&["Destroy all nonland permanents your opponents control."], |d, _| {
            spell(d, vec![], vec![Effect::DestroyOpponentsNonland])
        }),
        "Death Begets Life" => s(
            &["Destroy all creatures and enchantments. Draw a card for each permanent destroyed this way."],
            |d, _| spell(d, vec![], vec![Effect::DestroyCreaturesAndEnchantmentsDraw]),
        ),
        "Dragonback Assault" => s(
            &[
                "When this enchantment enters, it deals 3 damage to each creature and each planeswalker.",
                "Landfall — Whenever a land you control enters, create a 4/4 red Dragon creature token with flying.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![damage(3, Recipient::EachCreatureAndPlaneswalker)]);
                on(d, When::LandYouControlEnters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.dragon, n: 1 }]);
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
        _ => None,
    }
}

/// Lane C's scripted cards (checked against their text in `lib.rs`).
pub const TDM_C_SCRIPTED: &[&str] = &[
    "Summit Intimidator",
    "Arashin Sunshield",
    "Lightfoot Technique",
    "Unburied Earthcarver",
    "Sunset Strikemaster",
    "Opulent Palace",
    "Jeskai Shrinekeeper",
    "War Effort",
    "Wail of War",
    "Jeskai Brushmaster",
    "Starry-Eyed Skyrider",
    "Loxodon Battle Priest",
    "Fangkeeper's Familiar",
    "Inevitable Defeat",
    "Perennation",
    "Twin Bolt",
    "Unsparing Boltcaster",
    "Smile at Death",
    "Hardened Tactician",
    "Fire-Rim Form",
    "Krotiq Nestguard",
    "Ruinous Ultimatum",
    "Death Begets Life",
    "Dragonback Assault",
    "Duty Beyond Death",
];
