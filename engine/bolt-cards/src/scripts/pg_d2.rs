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
        _ => None,
    }
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
];
