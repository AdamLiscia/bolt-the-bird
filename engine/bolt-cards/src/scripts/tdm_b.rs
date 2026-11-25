//! Tarkir: Dragonstorm, lane B: plain cards on existing engine rules (card scripts only; any
//! card needing engine-core work goes back to the main lane). Tests: tests/tdm_b.rs.
#![allow(unused_imports)]

use super::*;

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    #[allow(clippy::match_single_binding)]
    match name {
        _ => {
            let _ = s;
            None
        }
    }
}

/// Lane B's scripted cards (checked against their text in `lib.rs`).
pub const TDM_B_SCRIPTED: &[&str] = &[];
