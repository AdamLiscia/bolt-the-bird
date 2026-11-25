//! Playgroup decks (Nerd Leagues), lane D2 (the desktop): the cards for the deck in handoff/lanes/d2.json. Conventions:
//! docs/lane-brief.md. Card text: assets/PG_oracle.json (in the CMD pool). Tests: tests/pg_d2.rs.
#![allow(unused_imports)]

use super::*;

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
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
        _ => None,
    }
}

/// This lane's scripted playgroup cards (checked against their text in `lib.rs`).
pub const PG_D2_SCRIPTED: &[&str] = &[
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
];
