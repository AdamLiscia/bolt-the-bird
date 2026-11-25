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
];
