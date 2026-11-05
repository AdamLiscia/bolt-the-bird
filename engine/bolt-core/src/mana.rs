//! Mana costs and paying them from lands (CR 106, 202, 601.2g-h).

use crate::Color;

/// A mana cost: generic mana plus colored pips. Hybrid and X costs come in later milestones.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ManaCost {
    pub generic: u8,
    /// Colored pips, indexed in WUBRG order.
    pub colored: [u8; 5],
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

    /// Mana value (CR 202.3).
    pub fn mana_value(&self) -> u32 {
        self.generic as u32 + self.colored.iter().map(|&n| n as u32).sum::<u32>()
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
    let pips: Vec<usize> = (0..5)
        .flat_map(|c| std::iter::repeat_n(c, cost.colored[c] as usize))
        .collect();
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
    pips: &[usize],
    sources: &[ManaSource],
    order: &[usize],
    used: &mut [bool],
    chosen: &mut Vec<usize>,
) -> bool {
    let Some((&color, rest)) = pips.split_first() else {
        return true;
    };
    for &i in order {
        if !used[i] && sources[i].colors[color] {
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
        assert_eq!(ManaCost::parse("{X}{R}"), None, "X costs are not supported yet");
        assert_eq!(ManaCost::parse("{W/U}"), None, "hybrid is not supported yet");
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
