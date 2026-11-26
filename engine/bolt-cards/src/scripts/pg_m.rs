//! Playgroup decks: the merger's own cards, scripted with the core hooks they need (attack legality, ...).
//! Conventions: docs/lane-brief.md. Card text: assets/PG_oracle.json (in the CMD pool). Tests: tests/pg_m.rs.
#![allow(unused_imports)]

use super::*;

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        // Myrkul's attack restrictions (the merger's hooks in attack_candidates and the Attack action).
        "Ghostly Prison" | "Propaganda" => s(
            &["Creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you."],
            |d, _| d.attack_tax = 2,
        ),
        "Sphere of Safety" => s(
            &["Creatures can't attack you or planeswalkers you control unless their controller pays {X} for each of those creatures, where X is the number of enchantments you control."],
            |d, _| d.attack_tax_per_enchantment = true,
        ),
        "Promise of Loyalty" => s(
            &["Each player puts a vow counter on a creature they control and sacrifices the rest. Each of those creatures can't attack you or planeswalkers you control for as long as it has a vow counter on it."],
            |d, _| spell(d, vec![], vec![Effect::EachVowsOneSacrificesRest]),
        ),
        _ => None,
    }
}

pub const PG_M_SCRIPTED: &[&str] = &["Ghostly Prison", "Propaganda", "Sphere of Safety", "Promise of Loyalty"];
