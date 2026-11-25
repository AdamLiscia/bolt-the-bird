//! Playgroup decks (Nerd Leagues), lane B: the cards for Xenagos, God of Revels (Adam) first, then
//! ~/bolt-handoff/playgroup_all.json's card_order. Card text: assets/PG_oracle.json (in the CMD pool).
//! Tests: tests/pg_b.rs.
#![allow(unused_imports)]

use super::*;

#[allow(clippy::match_single_binding)] // empty until the lane adds its first card
pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    let _ = s;
    match name {
        _ => None,
    }
}

/// Lane B's scripted playgroup cards (checked against their text in `lib.rs`).
pub const PG_B_SCRIPTED: &[&str] = &[];
