//! Playgroup decks (Nerd Leagues), lane B: the cards for Xenagos, God of Revels (Adam) first, then
//! ~/bolt-handoff/playgroup_all.json's card_order. Card text: assets/PG_oracle.json (in the CMD pool).
//! Tests: tests/pg_b.rs.
#![allow(unused_imports)]

use super::*;

/// "a Forest card" (any land with the Forest type).
fn forest() -> SearchFor {
    SearchFor::LandTypes {
        types: land_types(&["Forest"]),
        basic: false,
    }
}

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        "Birds of Paradise" => s(&["{T}: Add one mana of any color."], |d, _| {
            d.mana = Some(ManaSource::of(&Color::ALL));
        }),
        "Elvish Mystic" => s(&["{T}: Add {G}."], |d, _| d.mana = Some(ManaSource::of(&[Color::Green]))),
        "Lotus Petal" => s(&["{T}, Sacrifice this artifact: Add one mana of any color."], |d, _| {
            d.mana = Some(ManaSource::of(&Color::ALL));
            d.sac_for_mana = true;
        }),
        "Nature's Lore" => s(&["Search your library for a Forest card, put that card onto the battlefield, then shuffle."], |d, _| {
            spell(d, vec![], vec![search(forest(), SearchTo::Battlefield { tapped: false })])
        }),
        "Three Visits" => s(&["Search your library for a Forest card, put it onto the battlefield, then shuffle."], |d, _| {
            spell(d, vec![], vec![search(forest(), SearchTo::Battlefield { tapped: false })])
        }),
        "Skyshroud Claim" => s(&["Search your library for up to two Forest cards, put them onto the battlefield, then shuffle."], |d, _| {
            let two = Effect::Search { what: forest(), to: SearchTo::Battlefield { tapped: false }, n: 2, kicked_n: None };
            spell(d, vec![], vec![two])
        }),
        "Kodama's Reach" => s(
            &["Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle."],
            |d, _| {
                spell(
                    d,
                    vec![],
                    vec![search(SearchFor::BasicLand, SearchTo::Battlefield { tapped: true }), search(SearchFor::BasicLand, SearchTo::Hand)],
                )
            },
        ),
        "Wooded Foothills" => s(
            &["{T}, Pay 1 life, Sacrifice this land: Search your library for a Mountain or Forest card, put it onto the battlefield, then shuffle."],
            |d, _| fetch_land(d, &["Mountain", "Forest"]),
        ),
        "Aggravated Assault" => s(
            &["{3}{R}{R}: Untap all creatures you control. After this main phase, there is an additional combat phase followed by an additional main phase. Activate only as a sorcery."],
            |d, _| {
                let n = activated(d, "{3}{R}{R}", false, vec![], vec![Effect::UntapCreaturesYouControl, Effect::AdditionalCombatAndMain]);
                d.activated[n].sorcery = true;
            },
        ),
        "Seize the Day" => s(
            &["Untap target creature. After this main phase, there is an additional combat phase followed by an additional main phase.", "Flashback {2}{R}"],
            |d, _| {
                spell(d, vec![CREATURE], vec![Effect::UntapTarget(0), Effect::AdditionalCombatAndMain]);
                d.flashback = ManaCost::parse("{2}{R}");
            },
        ),
        "Grim Monolith" => s(
            &["This artifact doesn't untap during your untap step.", "{T}: Add {C}{C}{C}.", "{4}: Untap this artifact."],
            |d, _| {
                d.doesnt_untap = true;
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                d.mana_amount = 3;
                activated(d, "{4}", false, vec![], vec![Effect::UntapSelf]);
            },
        ),
        "Hull Breach" => s(
            &["Choose one —", "• Destroy target artifact.", "• Destroy target enchantment.", "• Destroy target artifact and target enchantment."],
            |d, _| {
                let enchantment = Filter { players: false, creatures: false, planeswalkers: false, enchantments: true, ..Filter::ANY };
                modes(
                    d,
                    vec![
                        Ability::new(vec![ARTIFACT], vec![Effect::Destroy(0)]),
                        Ability::new(vec![enchantment], vec![Effect::Destroy(0)]),
                        Ability::new(vec![ARTIFACT, enchantment], vec![Effect::Destroy(0), Effect::Destroy(1)]),
                    ],
                );
            },
        ),
        "Heroic Intervention" => s(&["Permanents you control gain hexproof and indestructible until end of turn."], |d, _| {
            spell(d, vec![], vec![Effect::GrantAllPermanentsYouControl(Keywords::HEXPROOF.with(Keywords::INDESTRUCTIBLE))])
        }),
        "Fire-Lit Thicket" => s(&["{T}: Add {C}.", "{R/G}, {T}: Add {R}{R}, {R}{G}, or {G}{G}."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            let rg = [Color::Red, Color::Green];
            d.paid_mana = Some((ManaCost::parse("{R/G}").expect("a mana cost"), [ManaSource::of(&rg), ManaSource::of(&rg)]));
        }),
        "Ancient Tomb" => s(&["{T}: Add {C}{C}. This land deals 2 damage to you."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            d.mana_amount = 2;
            d.mana_tap_damage = 2;
        }),
        "Fiery Emancipation" => s(
            &["If a source you control would deal damage to a permanent or player, it deals triple that damage to that permanent or player instead."],
            |d, _| d.triples_your_damage = true,
        ),
        "Goblin Anarchomancer" => s(&["Each spell you cast that's red or green costs {1} less to cast."], |d, _| {
            d.red_green_spell_discount = 1;
        }),
        "Elder Gargaroth" => s(
            &[
                "Whenever this creature attacks or blocks, choose one —",
                "• Create a 3/3 green Beast creature token.",
                "• You gain 3 life.",
                "• Draw a card.",
            ],
            |d, t| {
                for when in [When::Attacks, When::Blocks] {
                    on_modal(
                        d,
                        when,
                        None,
                        vec![
                            Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.beast_3, n: 1 }]),
                            Ability::new(vec![], vec![Effect::GainLife { who: Who::You, n: 3 }]),
                            Ability::new(vec![], vec![draw(1)]),
                        ],
                    );
                }
            },
        ),
        "Fierce Empath" => s(
            &["When this creature enters, you may search your library for a creature card with mana value 6 or greater, reveal it, put it into your hand, then shuffle."],
            |d, _| may(d, When::Enters, vec![], vec![search(SearchFor::CreatureMvAtLeast(6), SearchTo::Hand)]),
        ),
        _ => None,
    }
}

/// Lane B's scripted playgroup cards (checked against their text in `lib.rs`).
pub const PG_B_SCRIPTED: &[&str] = &[
    "Birds of Paradise",
    "Elvish Mystic",
    "Lotus Petal",
    "Nature's Lore",
    "Three Visits",
    "Skyshroud Claim",
    "Kodama's Reach",
    "Wooded Foothills",
    "Aggravated Assault",
    "Seize the Day",
    "Grim Monolith",
    "Hull Breach",
    "Heroic Intervention",
    "Fire-Lit Thicket",
    "Ancient Tomb",
    "Fiery Emancipation",
    "Goblin Anarchomancer",
    "Elder Gargaroth",
    "Fierce Empath",
];
