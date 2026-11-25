//! Bloomburrow Commander (BLC), lane C: `shared_order` positions 150-299 in ~/bolt-handoff/blc.json.
//! Tests: tests/blc_c.rs.
#![allow(unused_imports)]

use super::*;

/// "Cycling {cost}": "{cost}, Discard this card: Draw a card." from your hand.
fn cycling(d: &mut CardDef, cost: &str) {
    let n = activated(d, cost, false, vec![], vec![draw(1)]);
    d.activated[n].from_hand_discard = true;
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
        _ => None,
    }
}

/// Lane C's scripted BLC cards (checked against their text in `lib.rs`).
pub const BLC_C_SCRIPTED: &[&str] = &[
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
