//! Playgroup decks (Nerd Leagues), lane D3 (the desktop): the cards for the deck in handoff/lanes/d3.json. Conventions:
//! docs/lane-brief.md. Card text: assets/PG_oracle.json (in the CMD pool). Tests: tests/pg_d3.rs.
#![allow(unused_imports)]

use super::*;

/// "{T}: Add {A} or {B}." (or more colors).
fn lands_mana(d: &mut CardDef, colors: &[Color]) {
    d.mana = Some(ManaSource::of(colors));
}

/// "{T}: Add {C}."
fn colorless(d: &mut CardDef) {
    d.mana = Some(ManaSource {
        colorless: true,
        ..ManaSource::default()
    });
}

/// "Cycling {cost}": "{cost}, Discard this card: Draw a card." from your hand.
fn cycling(d: &mut CardDef, cost: &str) {
    let n = activated(d, cost, false, vec![], vec![draw(1)]);
    d.activated[n].from_hand_discard = true;
}

/// "{cost}, {T}: Add" two mana: the first `a`, the second `b` (Signets, Darkwater Catacombs).
fn two_mana(d: &mut CardDef, cost: &str, a: Color, b: Color) {
    d.paid_mana = Some((
        ManaCost::parse(cost).expect("a mana cost"),
        [ManaSource::of(&[a]), ManaSource::of(&[b])],
    ));
}

/// "{T}: Draw a card, then discard a card." (the loot creatures)
fn looter(d: &mut CardDef) {
    activated(d, "", true, vec![], vec![draw(1), discard(1)]);
}

const LOOT: &str = "{T}: Draw a card, then discard a card.";

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        // ---- Sefris of the Hidden Ways (Spr3adsh33t) ----
        "Isolated Chapel" => s(
            &["This land enters tapped unless you control a Plains or a Swamp.", "{T}: Add {W} or {B}."],
            |d, _| {
                d.enters_tapped_unless = Some(["Plains", "Swamp"]);
                lands_mana(d, &[Color::White, Color::Black]);
            },
        ),
        "Dimir Signet" => s(&["{1}, {T}: Add {U}{B}."], |d, _| two_mana(d, "{1}", Color::Blue, Color::Black)),
        "Orzhov Signet" => s(&["{1}, {T}: Add {W}{B}."], |d, _| two_mana(d, "{1}", Color::White, Color::Black)),
        "Raffine's Tower" => s(&["This land enters tapped.", "Cycling {3}"], |d, _| {
            d.enters_tapped = true;
            lands_mana(d, &[Color::White, Color::Blue, Color::Black]);
            cycling(d, "{3}");
        }),
        "Counterspell" => s(&["Counter target spell."], |d, _| spell(d, vec![SPELL], vec![Effect::Counter(0)])),
        "Entomb" => s(
            &["Search your library for a card, put that card into your graveyard, then shuffle."],
            |d, _| spell(d, vec![], vec![search(SearchFor::Any, SearchTo::Graveyard)]),
        ),
        "Buried Alive" => s(
            &["Search your library for up to three creature cards, put them into your graveyard, then shuffle."],
            |d, _| {
                // Any creature card: mana value 0 or more. The search may stop early ("up to").
                let what = SearchFor::CreatureMvAtLeast(0);
                spell(d, vec![], vec![Effect::Search { what, to: SearchTo::Graveyard, n: 3, kicked_n: None }]);
            },
        ),
        "Darkwater Catacombs" => s(&["{1}, {T}: Add {U}{B}."], |d, _| two_mana(d, "{1}", Color::Blue, Color::Black)),
        "Geier Reach Sanitarium" => s(
            &["{T}: Add {C}.", "{2}, {T}: Each player draws a card, then discards a card."],
            |d, _| {
                colorless(d);
                activated(
                    d,
                    "{2}",
                    true,
                    vec![],
                    vec![
                        draw(1),
                        Effect::Draw { who: Who::EachOpponent, n: 1 },
                        discard(1),
                        Effect::Discard { who: Who::EachOpponent, n: 1 },
                    ],
                );
            },
        ),
        "Razaketh, the Foulblooded" => s(
            &["Pay 2 life, Sacrifice another creature: Search your library for a card, put that card into your hand, then shuffle."],
            |d, _| {
                let n = activated(d, "", false, vec![], vec![search(SearchFor::Any, SearchTo::Hand)]);
                d.activated[n].life = 2;
                d.activated[n].sacrifice_other_creature = true;
            },
        ),
        "Ashen Rider" => s(&["When this creature enters or dies, exile target permanent."], |d, _| {
            let any = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, ..Filter::ANY };
            on(d, When::Enters, vec![any], vec![Effect::Exile(0)]);
            on(d, When::Dies, vec![any], vec![Effect::Exile(0)]);
        }),
        "Arcane Sanctum" => s(&["This land enters tapped.", "{T}: Add {W}, {U}, or {B}."], |d, _| {
            d.enters_tapped = true;
            lands_mana(d, &[Color::White, Color::Blue, Color::Black]);
        }),
        "Cavalier of Night" => s(
            &[
                "When this creature enters, you may sacrifice another creature. When you do, destroy target creature an opponent controls.",
                "When this creature dies, return target creature card with mana value 3 or less from your graveyard to the battlefield.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::MaySacrificeAnother, Effect::Reflexive(1)]);
                on(d, When::Never, vec![OPP_CREATURE], vec![Effect::Destroy(0)]); // trigger 1: "when you do"
                let card = Filter { max_mana_value: Some(3), ..graveyard_cards(true, Controller::You) };
                on(d, When::Dies, vec![card], vec![Effect::Reanimate(0)]);
            },
        ),
        "Choked Estuary" => s(
            &[
                "As this land enters, you may reveal an Island or Swamp card from your hand. If you don't, this land enters tapped.",
                "{T}: Add {U} or {B}.",
            ],
            |d, _| {
                d.enters_tapped_unless_reveal = Some(["Island", "Swamp"]);
                lands_mana(d, &[Color::Blue, Color::Black]);
            },
        ),
        "Darkslick Shores" => s(
            &["This land enters tapped unless you control two or fewer other lands.", "{T}: Add {U} or {B}."],
            |d, _| {
                d.enters_tapped_unless_at_most_lands = Some(2);
                lands_mana(d, &[Color::Blue, Color::Black]);
            },
        ),
        "Merfolk Looter" => s(&[LOOT], |d, _| looter(d)),
        "Thought Courier" => s(&[LOOT], |d, _| looter(d)),
        "Port Town" => s(
            &[
                "As this land enters, you may reveal a Plains or Island card from your hand. If you don't, this land enters tapped.",
                "{T}: Add {W} or {U}.",
            ],
            |d, _| {
                d.enters_tapped_unless_reveal = Some(["Plains", "Island"]);
                lands_mana(d, &[Color::White, Color::Blue]);
            },
        ),
        "Vile Entomber" => s(
            &["When this creature enters, search your library for a card, put that card into your graveyard, then shuffle."],
            |d, _| on(d, When::Enters, vec![], vec![search(SearchFor::Any, SearchTo::Graveyard)]),
        ),
        _ => None,
    }
}

/// This lane's scripted playgroup cards (checked against their text in `lib.rs`).
pub const PG_D3_SCRIPTED: &[&str] = &[
    // Sefris of the Hidden Ways
    "Isolated Chapel",
    "Dimir Signet",
    "Orzhov Signet",
    "Raffine's Tower",
    "Counterspell",
    "Entomb",
    "Buried Alive",
    "Darkwater Catacombs",
    "Geier Reach Sanitarium",
    "Razaketh, the Foulblooded",
    "Ashen Rider",
    "Arcane Sanctum",
    "Cavalier of Night",
    "Choked Estuary",
    "Darkslick Shores",
    "Merfolk Looter",
    "Thought Courier",
    "Port Town",
    "Vile Entomber",
];
