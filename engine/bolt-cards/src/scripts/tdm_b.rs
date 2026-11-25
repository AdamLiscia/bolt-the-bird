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
];
