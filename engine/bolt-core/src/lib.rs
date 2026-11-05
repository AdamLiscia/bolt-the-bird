//! Rules core for the Bolt the Bird Magic engine.
//!
//! Game state is plain, cloneable data, and [`Game::apply`] is the only thing that changes it.
//! Rules cite the Comprehensive Rules (CR) section they follow.

pub mod card;
pub mod effect;
pub mod fuzz;
pub mod game;
pub mod mana;
pub mod rng;

pub use card::{CardDef, Keywords, Types};
pub use effect::{
    Ability, Condition, Controller, Effect, Filter, Recipient, SearchFor, SearchTo, Trigger, When, Who,
};
pub use game::{
    Action, Chosen, Decision, Event, Game, Modifier, ObjId, Outcome, Pending, PlayerId, Resolving,
    RulesError, StackItem, StackKind, Step, Target, Zone,
};
pub use mana::{ManaCost, ManaSource};

/// The five colors of Magic, in WUBRG order (CR 105.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Color {
    White,
    Blue,
    Black,
    Red,
    Green,
}

impl Color {
    /// All five colors in WUBRG order.
    pub const ALL: [Color; 5] = [Color::White, Color::Blue, Color::Black, Color::Red, Color::Green];

    /// The one-letter symbol used in mana costs and by 17lands (`W`, `U`, `B`, `R`, `G`).
    pub fn symbol(self) -> char {
        match self {
            Color::White => 'W',
            Color::Blue => 'U',
            Color::Black => 'B',
            Color::Red => 'R',
            Color::Green => 'G',
        }
    }

    /// Parses a one-letter color symbol.
    pub fn from_symbol(c: char) -> Option<Color> {
        Color::ALL.into_iter().find(|col| col.symbol() == c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_symbols_round_trip() {
        for c in Color::ALL {
            assert_eq!(Color::from_symbol(c.symbol()), Some(c));
        }
        assert_eq!(Color::from_symbol('X'), None);
    }

    #[test]
    fn wubrg_order() {
        let s: String = Color::ALL.iter().map(|c| c.symbol()).collect();
        assert_eq!(s, "WUBRG");
    }
}
