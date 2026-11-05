//! Foundations (FDN) card definitions.
//!
//! [`fdn`] returns all 286 cards in the same order as `assets/FDN_cards.json`, so a card's index here
//! is its index in the draft models, and a drafted deck plays without translation.
//!
//! [`supported`] is the source of truth for restricted drafts: a card is playable only if every
//! line of its rules text is implemented. Unsupported cards are listed, never skipped.

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

/// Card definitions for all of FDN, in model order. Unsupported cards still get their printed
/// characteristics, so decks load whole; [`supported`] says which ones the engine can play.
pub fn fdn() -> Arc<Vec<CardDef>> {
    static CELL: OnceLock<Arc<Vec<CardDef>>> = OnceLock::new();
    CELL.get_or_init(|| Arc::new(oracle().iter().map(|o| definition(o).0).collect()))
        .clone()
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

/// Builds the definition and decides support. A card is supported when its cost parses and every
/// ability line is a comma-separated list of implemented keywords, or when it is a basic land.
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
    let mut all_keywords = true;
    for line in abilities(o.oracle_text.as_deref().unwrap_or("")) {
        for word in line.split(',') {
            match Keywords::named(word.trim()) {
                Some(k) => keywords = keywords.with(k),
                None => all_keywords = false,
            }
        }
    }
    let basic_color = types.basic.then(|| {
        let sub = o.type_line.rsplit(' ').next().unwrap_or("");
        match sub {
            "Plains" => Color::White,
            "Island" => Color::Blue,
            "Swamp" => Color::Black,
            "Mountain" => Color::Red,
            _ => Color::Green,
        }
    });
    let stat = |s: &Option<String>| s.as_deref().and_then(|v| v.parse().ok()).unwrap_or(0);
    let def = CardDef {
        name: o.name.clone(),
        cost: cost.flatten().unwrap_or_default(),
        types,
        subtypes,
        power: stat(&o.power),
        toughness: stat(&o.toughness),
        keywords: if all_keywords { keywords } else { Keywords::NONE },
        mana: basic_color.map(|c| ManaSource::of(&[c])),
        spell: None,
        triggers: Vec::new(),
        kicker: None,
        token: false,
    };
    let supported =
        basic_color.is_some() || (types.creature && all_keywords && matches!(cost, Some(Some(_))));
    (def, supported)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_cards_load_in_model_order() {
        let defs = fdn();
        assert_eq!(defs.len(), 286);
        assert_eq!(defs[0].name, "Abrade", "same order as assets/FDN_cards.json");
    }

    #[test]
    fn supported_cards_are_pinned() {
        // Changing this list is a deliberate act: each new card needs its own rules test.
        assert_eq!(
            supported(),
            [
                "Aegis Turtle",
                "Brazen Scourge",
                "Forest",
                "Healer's Hawk",
                "Island",
                "Mountain",
                "Plains",
                "Quakestrider Ceratops",
                "Savannah Lions",
                "Serra Angel",
                "Swamp",
                "Swiftblade Vindicator",
                "Vampire Nighthawk",
            ]
        );
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
