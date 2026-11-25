//! Playgroup decks (Nerd Leagues), lane D3 (the desktop): the cards for the deck in handoff/lanes/d3.json. Conventions:
//! docs/lane-brief.md. Card text: assets/PG_oracle.json (in the CMD pool). Tests: tests/pg_d3.rs.
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

/// This lane's scripted playgroup cards (checked against their text in `lib.rs`).
pub const PG_D3_SCRIPTED: &[&str] = &[];
