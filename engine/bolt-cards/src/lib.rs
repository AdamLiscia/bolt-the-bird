//! Foundations (FDN) card definitions.
//!
//! [`fdn`] returns all 286 cards in the same order as `assets/FDN_cards.json`, so a card's index here
//! is its index in the draft models, and a drafted deck plays without translation.
//!
//! [`supported`] is the source of truth for restricted drafts: a card is playable only if every
//! line of its rules text is implemented. Unsupported cards are listed, never skipped.

pub mod scripts;

use std::sync::{Arc, OnceLock};

use bolt_core::{CardDef, Color, Keywords, ManaCost, ManaSource, Types};
use serde::Deserialize;

const ORACLE: &str = include_str!("../../../assets/FDN_oracle.json");

/// One card as Scryfall describes it (`assets/FDN_oracle.json`, written by `scripts/card_audit.py`).
#[derive(Clone, Debug, Deserialize)]
pub struct Oracle {
    pub name: String,
    pub mana_cost: Option<String>,
    pub type_line: String,
    pub oracle_text: Option<String>,
    pub power: Option<String>,
    pub toughness: Option<String>,
    pub rarity: String,
}

/// The Scryfall data for every FDN card, in model order.
pub fn oracle() -> &'static [Oracle] {
    static CELL: OnceLock<Vec<Oracle>> = OnceLock::new();
    CELL.get_or_init(|| serde_json::from_str(ORACLE).expect("assets/FDN_oracle.json parses"))
}

/// Card definitions for all of FDN in model order, then the token definitions. Unsupported cards
/// still get their printed characteristics, so decks load whole; [`supported`] says which ones the
/// engine can play.
pub fn fdn() -> Arc<Vec<CardDef>> {
    static CELL: OnceLock<Arc<Vec<CardDef>>> = OnceLock::new();
    CELL.get_or_init(|| {
        let mut defs: Vec<CardDef> = oracle().iter().map(|o| definition(o).0).collect();
        defs.extend(scripts::token_defs());
        Arc::new(defs)
    })
    .clone()
}

/// Number of real cards (tokens follow them in [`fdn`]).
pub fn card_count() -> usize {
    oracle().len()
}

fn tokens() -> scripts::Tokens {
    scripts::Tokens::starting_at(card_count())
}

/// Whether the engine implements every rule on each card, by model index.
pub fn supported_mask() -> &'static [bool] {
    static CELL: OnceLock<Vec<bool>> = OnceLock::new();
    CELL.get_or_init(|| oracle().iter().map(|o| definition(o).1).collect())
}

/// Names of the cards the engine fully implements, in model order.
pub fn supported() -> Vec<&'static str> {
    oracle()
        .iter()
        .zip(supported_mask())
        .filter(|(_, &ok)| ok)
        .map(|(o, _)| o.name.as_str())
        .collect()
}

/// Model index of a card by exact name.
pub fn index_of(name: &str) -> Option<usize> {
    oracle().iter().position(|o| o.name == name)
}

/// Rules text with reminder text removed, one ability per line.
fn abilities(text: &str) -> Vec<String> {
    let mut out = String::new();
    let mut depth = 0;
    for ch in text.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

/// Whether an ability line is only implemented keywords, e.g. `Flying, vigilance`.
fn keyword_line(line: &str) -> Option<Keywords> {
    line.split(',').try_fold(Keywords::NONE, |k, w| {
        Keywords::named(w.trim()).map(|x| k.with(x))
    })
}

/// Builds the definition and decides support. A card is supported when its cost parses and every
/// ability line is either implemented keywords or claimed by its script.
fn definition(o: &Oracle) -> (CardDef, bool) {
    let types = Types::parse(&o.type_line);
    let subtypes = o
        .type_line
        .split('—')
        .nth(1)
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_default();
    let cost = o.mana_cost.as_deref().map(ManaCost::parse);
    let mut keywords = Keywords::NONE;
    let mut rest = Vec::new();
    for line in abilities(o.oracle_text.as_deref().unwrap_or("")) {
        match keyword_line(&line) {
            Some(k) => keywords = keywords.with(k),
            None => rest.push(line),
        }
    }
    let basic_color = types
        .basic
        .then(|| match o.type_line.rsplit(' ').next().unwrap_or("") {
            "Plains" => Color::White,
            "Island" => Color::Blue,
            "Swamp" => Color::Black,
            "Mountain" => Color::Red,
            _ => Color::Green,
        });
    let stat = |s: &Option<String>| s.as_deref().and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut def = CardDef {
        name: o.name.clone(),
        cost: cost.flatten().unwrap_or_default(),
        types,
        subtypes,
        power: stat(&o.power),
        toughness: stat(&o.toughness),
        keywords,
        mana: basic_color.map(|c| ManaSource::of(&[c])),
        spell: None,
        triggers: Vec::new(),
        kicker: None,
        token: false,
        activated: Vec::new(),
        sac_for_mana: false,
        extra_cost: None,
        reduction: None,
        flashback: None,
    };
    let mut claimed = rest.is_empty();
    if let Some(s) = scripts::script(&o.name) {
        if s.lines == rest.as_slice() {
            (s.build)(&mut def, &tokens());
            claimed = true;
        }
    }
    let castable = (types.is_permanent() && !types.land) || def.spell.is_some();
    let supported = basic_color.is_some() || (castable && claimed && matches!(cost, Some(Some(_))));
    (def, supported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_cards_load_in_model_order() {
        let defs = fdn();
        assert_eq!(card_count(), 286);
        assert_eq!(defs[0].name, "Abrade", "same order as assets/FDN_cards.json");
    }

    #[test]
    fn support_is_basics_keyword_creatures_and_scripts() {
        // Changing this is a deliberate act: each new card needs its own rules test.
        let keyword_only = [
            "Aegis Turtle",
            "Brazen Scourge",
            "Healer's Hawk",
            "Quakestrider Ceratops",
            "Savannah Lions",
            "Serra Angel",
            "Swiftblade Vindicator",
            "Vampire Nighthawk",
        ];
        let basics = ["Forest", "Island", "Mountain", "Plains", "Swamp"];
        let mut expected: Vec<&str> = keyword_only
            .iter()
            .chain(&basics)
            .chain(scripts::SCRIPTED)
            .copied()
            .collect();
        expected.sort_unstable();
        let mut got = supported();
        got.sort_unstable();
        assert_eq!(got, expected);
    }

    #[test]
    fn every_script_matches_its_card_text() {
        for &name in scripts::SCRIPTED {
            let i = index_of(name).unwrap_or_else(|| panic!("{name} is not in FDN"));
            assert!(
                supported_mask()[i],
                "{name}: script lines don't match the card text"
            );
        }
    }

    #[test]
    fn tokens_follow_the_cards() {
        let defs = fdn();
        assert_eq!(defs.len(), card_count() + scripts::token_defs().len());
        assert!(defs[card_count()..].iter().all(|d| d.token));
    }

    #[test]
    fn supported_has_no_duplicates() {
        let mut names = supported();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), supported().len());
    }

    #[test]
    fn reminder_text_is_ignored() {
        assert_eq!(
            abilities("Flying (This can't be blocked.)\nLifelink"),
            ["Flying", "Lifelink"]
        );
    }
}
