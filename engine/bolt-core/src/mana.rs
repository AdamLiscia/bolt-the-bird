//! Mana costs and paying them from lands (CR 106, 202, 601.2g-h).

use crate::Color;

/// A mana cost: generic mana plus colored pips. Hybrid and X costs come in later milestones.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ManaCost {
    pub generic: u8,
    /// Colored pips, indexed in WUBRG order.
    pub colored: [u8; 5],
    /// Hybrid pips such as {B/G}, by color pair (see [`pair_index`]).
    pub hybrid: [u8; 10],
    /// How many {X} symbols (CR 107.3).
    pub x: u8,
}

/// The ten two-color pairs, in a fixed order.
pub const PAIRS: [(usize, usize); 10] = [
    (0, 1),
    (0, 2),
    (0, 3),
    (0, 4),
    (1, 2),
    (1, 3),
    (1, 4),
    (2, 3),
    (2, 4),
    (3, 4),
];

/// Index of a color pair in [`PAIRS`].
pub fn pair_index(a: usize, b: usize) -> usize {
    let (lo, hi) = if a < b { (a, b) } else { (b, a) };
    PAIRS
        .iter()
        .position(|&p| p == (lo, hi))
        .expect("two different colors")
}

impl ManaCost {
    /// Parses Scryfall notation such as `{2}{W}{W}`. Returns `None` for symbols not supported yet.
    pub fn parse(s: &str) -> Option<ManaCost> {
        let mut cost = ManaCost::default();
        let mut rest = s.trim();
        while !rest.is_empty() {
            let end = rest.find('}')?;
            let sym = rest.strip_prefix('{')?.get(..end - 1)?;
            rest = &rest[end + 1..];
            if let Ok(n) = sym.parse::<u8>() {
                cost.generic += n;
            } else if sym == "X" {
                cost.x += 1;
            } else if let Some((a, b)) = sym.split_once('/') {
                let ca = Color::from_symbol(a.chars().next()?)? as usize;
                let cb = Color::from_symbol(b.chars().next()?)? as usize;
                if a.len() != 1 || b.len() != 1 || ca == cb {
                    return None;
                }
                cost.hybrid[pair_index(ca, cb)] += 1;
            } else {
                let mut chars = sym.chars();
                let c = Color::from_symbol(chars.next()?)?;
                if chars.next().is_some() {
                    return None;
                }
                cost.colored[c as usize] += 1;
            }
        }
        Some(cost)
    }

    /// Mana value (CR 202.3). {X} counts as 0 here; on the stack it counts as the chosen X.
    pub fn mana_value(&self) -> u32 {
        self.generic as u32
            + self.colored.iter().map(|&n| n as u32).sum::<u32>()
            + self.hybrid.iter().map(|&n| n as u32).sum::<u32>()
    }

    /// The cost with X fixed: each {X} becomes `x` generic mana.
    pub fn with_x(&self, x: u8) -> ManaCost {
        let mut c = *self;
        c.generic = c.generic.saturating_add(x.saturating_mul(c.x));
        c.x = 0;
        c
    }
}

/// What a mana source can produce: a set of colors, plus colorless.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ManaSource {
    pub colors: [bool; 5],
    pub colorless: bool,
}

impl ManaSource {
    pub fn of(colors: &[Color]) -> Self {
        let mut s = ManaSource::default();
        for &c in colors {
            s.colors[c as usize] = true;
        }
        s
    }

    fn breadth(&self) -> usize {
        self.colors.iter().filter(|&&b| b).count() + self.colorless as usize
    }
}

/// Chooses which sources to tap to pay `cost`, or `None` if it can't be paid.
///
/// The engine pays automatically (see docs/engine/QUESTIONS.md). Colored pips are matched first by
/// backtracking, trying the narrowest sources first so flexible lands stay untapped; generic mana
/// then takes the narrowest sources left. Returns indices into `sources`.
pub fn pay(cost: &ManaCost, sources: &[ManaSource]) -> Option<Vec<usize>> {
    let mut order: Vec<usize> = (0..sources.len()).collect();
    order.sort_by_key(|&i| (sources[i].breadth(), i));
    // Each pip is the set of colors that can pay it: one color, or two for hybrid.
    let mut pips: Vec<[bool; 5]> = Vec::new();
    for c in 0..5 {
        let mut mask = [false; 5];
        mask[c] = true;
        pips.extend(std::iter::repeat_n(mask, cost.colored[c] as usize));
    }
    for (i, &(a, b)) in PAIRS.iter().enumerate() {
        let mut mask = [false; 5];
        mask[a] = true;
        mask[b] = true;
        pips.extend(std::iter::repeat_n(mask, cost.hybrid[i] as usize));
    }
    let mut used = vec![false; sources.len()];
    let mut chosen = Vec::new();
    if !match_pips(&pips, sources, &order, &mut used, &mut chosen) {
        return None;
    }
    let mut generic = cost.generic as usize;
    for &i in &order {
        if generic == 0 {
            break;
        }
        if !used[i] {
            used[i] = true;
            chosen.push(i);
            generic -= 1;
        }
    }
    (generic == 0).then_some(chosen)
}

fn match_pips(
    pips: &[[bool; 5]],
    sources: &[ManaSource],
    order: &[usize],
    used: &mut [bool],
    chosen: &mut Vec<usize>,
) -> bool {
    let Some((&mask, rest)) = pips.split_first() else {
        return true;
    };
    for &i in order {
        if !used[i] && (0..5).any(|c| mask[c] && sources[i].colors[c]) {
            used[i] = true;
            chosen.push(i);
            if match_pips(rest, sources, order, used, chosen) {
                return true;
            }
            chosen.pop();
            used[i] = false;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use Color::*;

    #[test]
    fn parses_costs() {
        let c = ManaCost::parse("{2}{W}{W}").unwrap();
        assert_eq!(c.generic, 2);
        assert_eq!(c.colored, [2, 0, 0, 0, 0]);
        assert_eq!(c.mana_value(), 4);
        assert_eq!(ManaCost::parse("").unwrap().mana_value(), 0);
        assert_eq!(ManaCost::parse("{10}").unwrap().generic, 10);
        let x = ManaCost::parse("{X}{R}").unwrap();
        assert_eq!((x.x, x.mana_value()), (1, 1), "CR 202.3e: X is 0 off the stack");
        assert_eq!(x.with_x(3).mana_value(), 4);
        let h = ManaCost::parse("{B/G}{B/G}").unwrap();
        assert_eq!((h.mana_value(), h.hybrid[pair_index(2, 4)]), (2, 2));
        assert_eq!(ManaCost::parse("{W/W}"), None);
    }

    #[test]
    fn pays_with_basics() {
        let plains = ManaSource::of(&[White]);
        let mountain = ManaSource::of(&[Red]);
        let cost = ManaCost::parse("{1}{W}").unwrap();
        let mut got = pay(&cost, &[mountain, plains]).unwrap();
        got.sort();
        assert_eq!(got, vec![0, 1]);
        assert_eq!(pay(&cost, &[mountain, mountain]), None, "no white source");
        assert_eq!(pay(&cost, &[plains]), None, "not enough mana");
    }

    #[test]
    fn hybrid_pips_take_either_color() {
        let swamp = ManaSource::of(&[Black]);
        let forest = ManaSource::of(&[Green]);
        let mountain = ManaSource::of(&[Red]);
        let cost = ManaCost::parse("{B/G}{B/G}").unwrap();
        assert!(pay(&cost, &[swamp, forest]).is_some());
        assert!(pay(&cost, &[forest, forest]).is_some());
        assert!(pay(&cost, &[mountain, forest]).is_none());
    }

    #[test]
    fn keeps_the_dual_land_for_last() {
        // {W} with a Plains and a W/U dual: tap the Plains, keep the dual open.
        let dual = ManaSource::of(&[White, Blue]);
        let plains = ManaSource::of(&[White]);
        assert_eq!(
            pay(&ManaCost::parse("{W}").unwrap(), &[dual, plains]),
            Some(vec![1])
        );
    }

    #[test]
    fn backtracks_when_the_greedy_choice_fails() {
        // {W}{U} from [W/U dual, Plains]: the dual must pay U.
        let dual = ManaSource::of(&[White, Blue]);
        let plains = ManaSource::of(&[White]);
        let mut got = pay(&ManaCost::parse("{W}{U}").unwrap(), &[dual, plains]).unwrap();
        got.sort();
        assert_eq!(got, vec![0, 1]);
    }
}
