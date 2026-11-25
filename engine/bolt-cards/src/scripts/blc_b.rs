//! Bloomburrow Commander (BLC), lane B: the four precons' cards, most-shared first. Tests:
//! tests/blc_b.rs.
#![allow(unused_imports)]

use super::*;

/// "{T}: Add {A} or {B}." with an enters-tapped rule.
fn dual(d: &mut CardDef, a: Color, b: Color) {
    d.mana = Some(ManaSource::of(&[a, b]));
}

fn colorless(d: &mut CardDef) {
    d.mana = Some(ManaSource {
        colorless: true,
        ..ManaSource::default()
    });
}

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        // ---- Commanders ----
        "Ms. Bumbleflower" => s(
            &["Whenever you cast a spell, target opponent draws a card. Put a +1/+1 counter on target creature. It gains flying until end of turn. If this is the second time this ability has resolved this turn, you draw two cards."],
            |d, _| {
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                on(
                    d,
                    When::YouCast,
                    vec![opp, CREATURE],
                    vec![
                        Effect::Draw { who: Who::TargetPlayer(0), n: 1 },
                        Effect::Counters { slot: 1, n: 1 },
                        Effect::Grant { slot: 1, keywords: Keywords::FLYING },
                        Effect::DrawIfSecondResolutionThisTurn(2),
                    ],
                );
            },
        ),
        "Bello, Bard of the Brambles" => s(
            &["During your turn, each non-Equipment artifact and non-Aura enchantment you control with mana value 4 or greater is a 4/4 Elemental creature in addition to its other types and has indestructible, haste, and \"Whenever this creature deals combat damage to a player, draw a card.\""],
            |d, _| {
                d.animates_big_noncreatures_your_turn = true;
                on(d, When::Never, vec![], vec![draw(1)]); // trigger 0: the granted draw
            },
        ),
        // ---- Mana: the staples every deck shares ----
        "Arcane Signet" => s(&["{T}: Add one mana of any color in your commander's color identity."], |d, _| {
            d.mana_commander_identity = true;
        }),
        "Command Tower" => s(&["{T}: Add one mana of any color in your commander's color identity."], |d, _| {
            d.mana_commander_identity = true;
        }),
        "Exotic Orchard" => s(&["{T}: Add one mana of any color that a land an opponent controls could produce."], |d, _| {
            d.mana_opponent_land_colors = true;
        }),
        "Fellwar Stone" => s(&["{T}: Add one mana of any color that a land an opponent controls could produce."], |d, _| {
            d.mana_opponent_land_colors = true;
        }),
        "Sol Ring" => s(&["{T}: Add {C}{C}."], |d, _| {
            colorless(d);
            d.mana_amount = 2;
        }),
        "Terramorphic Expanse" => s(
            &["{T}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle."],
            |d, _| {
                let n = activated(d, "", true, vec![], vec![search(SearchFor::BasicLand, SearchTo::Battlefield { tapped: true })]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Mind Stone" => s(&["{T}: Add {C}.", "{1}, {T}, Sacrifice this artifact: Draw a card."], |d, _| {
            colorless(d);
            let n = activated(d, "{1}", true, vec![], vec![draw(1)]);
            d.activated[n].sacrifice_self = true;
        }),
        "Glacial Fortress" => s(
            &["This land enters tapped unless you control a Plains or an Island.", "{T}: Add {W} or {U}."],
            |d, _| {
                d.enters_tapped_unless = Some(["Plains", "Island"]);
                dual(d, Color::White, Color::Blue);
            },
        ),
        "Reliquary Tower" => s(&["You have no maximum hand size.", "{T}: Add {C}."], |d, _| {
            d.no_max_hand_size = true;
            colorless(d);
        }),
        "Thought Vessel" => s(&["You have no maximum hand size.", "{T}: Add {C}."], |d, _| {
            d.no_max_hand_size = true;
            colorless(d);
        }),
        "Temple of Enlightenment" => s(
            &["This land enters tapped.", "When this land enters, scry 1.", "{T}: Add {W} or {U}."],
            |d, _| {
                d.enters_tapped = true;
                on(d, When::Enters, vec![], vec![Effect::Scry { n: 1 }]);
                dual(d, Color::White, Color::Blue);
            },
        ),
        "Cultivate" => s(
            &["Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle."],
            |d, _| {
                spell(
                    d,
                    vec![],
                    vec![
                        search(SearchFor::BasicLand, SearchTo::Battlefield { tapped: true }),
                        search(SearchFor::BasicLand, SearchTo::Hand),
                    ],
                )
            },
        ),
        "Farseek" => s(
            &["Search your library for a Plains, Island, Swamp, or Mountain card, put it onto the battlefield tapped, then shuffle."],
            |d, _| {
                let types = land_types(&["Plains", "Island", "Swamp", "Mountain"]);
                spell(d, vec![], vec![search(SearchFor::LandTypes { types, basic: false }, SearchTo::Battlefield { tapped: true })]);
            },
        ),
        _ => None,
    }
}

/// Lane B's scripted BLC cards (checked against their text in `lib.rs`).
pub const BLC_B_SCRIPTED: &[&str] = &[
    "Ms. Bumbleflower",
    "Bello, Bard of the Brambles",
    "Arcane Signet",
    "Command Tower",
    "Exotic Orchard",
    "Fellwar Stone",
    "Sol Ring",
    "Terramorphic Expanse",
    "Mind Stone",
    "Glacial Fortress",
    "Reliquary Tower",
    "Thought Vessel",
    "Temple of Enlightenment",
    "Cultivate",
    "Farseek",
];
