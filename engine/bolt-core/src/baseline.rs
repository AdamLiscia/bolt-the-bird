//! A rules-based player (milestone M5): curve out, attack when it's safe, block to survive.
//!
//! It is deliberately simple and deterministic, a floor that a learned player has to beat. It
//! looks only at the board, its own hand and the legal actions; it never peeks at hidden cards.

use crate::effect::Effect;
use crate::game::{Action, Game, ObjId, Pending, PlayerId, Step, Target};
use crate::{Keywords, Zone};

/// The baseline's choice from `legal`, which must be `g.legal_actions()` and not empty.
pub fn choose(g: &Game, legal: &[Action]) -> Action {
    assert!(!legal.is_empty(), "no legal actions");
    let pick = match g.pending {
        Pending::Mulligan(p) => Some(mulligan(g, p, legal)),
        Pending::Bottom(p, _) => bottom(g, p, legal),
        Pending::Priority(p) => priority(g, p, legal),
        Pending::FreeCast(_) => legal.iter().copied().find(|a| *a != Action::PassPriority),
        Pending::DeclareAttackers(p) => Some(attack(g, p, legal)),
        Pending::DeclareBlockers(p) => Some(block(g, p, legal)),
        Pending::Discard(p) => discard(g, p, legal),
        Pending::ChooseTarget(p) => target(g, p, legal),
        Pending::Decide(_) => [Action::Yes]
            .into_iter()
            .find(|a| legal.contains(a))
            .or_else(|| legal.iter().copied().find(|a| *a != Action::No)),
        Pending::Divide(_) | Pending::RemoveCounters(_) | Pending::GameOver => None,
    };
    pick.filter(|a| legal.contains(a)).unwrap_or(legal[0])
}

fn mana_value(g: &Game, c: ObjId) -> u32 {
    g.def_of(c).cost.mana_value()
}

fn lands_in_hand(g: &Game, p: PlayerId) -> usize {
    g.players[p]
        .hand
        .iter()
        .filter(|&&c| g.def_of(c).types.land)
        .count()
}

/// Keep two to five lands; never go below six cards.
fn mulligan(g: &Game, p: PlayerId, legal: &[Action]) -> Action {
    let lands = lands_in_hand(g, p);
    if (2..=5).contains(&lands) || g.players[p].mulligans >= 1 || !legal.contains(&Action::Mulligan) {
        Action::Keep
    } else {
        Action::Mulligan
    }
}

/// Bottom a land when flooded, else the most expensive spell.
fn bottom(g: &Game, p: PlayerId, legal: &[Action]) -> Option<Action> {
    let flooded = lands_in_hand(g, p) > 4;
    legal
        .iter()
        .copied()
        .filter_map(|a| match a {
            Action::Bottom(c) => Some((c, a)),
            _ => None,
        })
        .max_by_key(|&(c, _)| {
            let land = g.def_of(c).types.land;
            (land == flooded, mana_value(g, c))
        })
        .map(|(_, a)| a)
}

/// The card a casting action casts, and how much mana it puts to work (bigger is better).
fn cast_value(g: &Game, a: Action) -> Option<u32> {
    let (c, extra) = match a {
        Action::Cast(c) | Action::CastFlashback(c) | Action::CastAlternative(c) => (c, 0),
        Action::CastFromGraveyard(c) | Action::CastFromGraveyardAs(c, _) => (c, 0),
        Action::CastModal(c, _) | Action::CastWith(c, _) => (c, 0),
        Action::CastKicked(c) => (c, g.def_of(c).kicker.map_or(0, |k| k.mana_value())),
        Action::CastX(c, x) => (c, x as u32),
        _ => return None,
    };
    Some(mana_value(g, c) + extra)
}

/// In your own main phase: play a land, then the biggest spell, then a loyalty ability. Pass
/// everywhere else.
fn priority(g: &Game, p: PlayerId, legal: &[Action]) -> Option<Action> {
    let main = g.active == p && matches!(g.step, Step::Main1 | Step::Main2) && g.items.is_empty();
    if !main {
        return Some(Action::PassPriority);
    }
    if let Some(&land) = legal.iter().find(|a| matches!(a, Action::PlayLand(_))) {
        return Some(land);
    }
    let best = legal
        .iter()
        .copied()
        .filter_map(|a| cast_value(g, a).map(|v| (v, a)))
        .max_by_key(|&(v, _)| v);
    if let Some((_, a)) = best {
        return Some(a);
    }
    if let Some(&l) = legal.iter().find(|a| matches!(a, Action::Loyalty(_, 0))) {
        return Some(l);
    }
    Some(Action::PassPriority)
}

/// Whether `blocker` could block `attacker` (flying needs flying or reach).
fn can_block(g: &Game, blocker: ObjId, attacker: ObjId) -> bool {
    let (ka, kb) = (g.keywords(attacker), g.keywords(blocker));
    let flying_blocks = kb.has(Keywords::FLYING) || kb.has(Keywords::REACH);
    !g.obj(blocker).tapped && (!ka.has(Keywords::FLYING) || flying_blocks)
}

/// Attack with each creature no untapped blocker can kill, or with everything if the attack is
/// lethal.
fn attack(g: &Game, p: PlayerId, legal: &[Action]) -> Action {
    let opp = 1 - p;
    let ready: Vec<ObjId> = legal
        .iter()
        .filter_map(|a| match a {
            Action::Attack(c) => Some(*c),
            _ => None,
        })
        .collect();
    let declared: Vec<ObjId> = g.combat.as_ref().map(|c| c.attackers.clone()).unwrap_or_default();
    let power: i32 = ready.iter().chain(&declared).map(|&c| g.power(c).max(0)).sum();
    let blockers: Vec<ObjId> = g.creatures(opp).filter(|&b| !g.obj(b).tapped).collect();
    let lethal = blockers.is_empty() && power >= g.players[opp].life;
    for c in ready {
        let safe = blockers.iter().all(|&b| {
            !can_block(g, b, c)
                || (g.power(b) < g.toughness(c)
                    && !g.keywords(b).has(Keywords::DEATHTOUCH)
                    && !g.keywords(c).has(Keywords::DEFENDER))
        });
        if (safe || lethal) && g.power(c) > 0 {
            return Action::Attack(c);
        }
    }
    Action::FinishAttacks
}

/// Block an attacker when the blocker survives or trades up; chump the biggest attackers when the
/// damage coming through would be lethal.
fn block(g: &Game, p: PlayerId, legal: &[Action]) -> Action {
    let Some(combat) = g.combat.as_ref() else {
        return Action::FinishBlocks;
    };
    let blocks: Vec<(ObjId, ObjId)> = legal
        .iter()
        .filter_map(|a| match a {
            Action::Block { blocker, attacker } => Some((*blocker, *attacker)),
            _ => None,
        })
        .collect();
    let unblocked: i32 = combat
        .attackers
        .iter()
        .filter(|&&a| !combat.is_blocked(a))
        .map(|&a| g.power(a).max(0))
        .sum();
    let in_danger = unblocked >= g.players[p].life;
    let kills = |x: ObjId, y: ObjId| g.power(x) >= g.toughness(y) || g.keywords(x).has(Keywords::DEATHTOUCH);
    // A declaration that isn't legal yet (a requirement or menace): keep adding blockers, and
    // start over only when none are left.
    if !legal.contains(&Action::FinishBlocks) {
        return blocks
            .first()
            .map_or(Action::ResetBlocks, |&(b, a)| Action::Block {
                blocker: b,
                attacker: a,
            });
    }
    // Only declare a block that can still be finished as is (a lone blocker on a menace attacker
    // can't), so the declaration never needs undoing.
    let finishable = |b: ObjId, a: ObjId| {
        let mut h = g.clone();
        h.apply(Action::Block {
            blocker: b,
            attacker: a,
        })
        .is_ok()
            && h.legal_actions().contains(&Action::FinishBlocks)
    };
    let good = blocks
        .iter()
        .copied()
        .filter(|&(_, a)| !combat.is_blocked(a))
        .filter(|&(b, a)| {
            let survives = !kills(a, b);
            let trade_up = kills(b, a) && mana_value(g, a) >= mana_value(g, b);
            survives || trade_up || in_danger
        })
        .max_by_key(|&(b, a)| {
            let survives = !kills(a, b);
            let trade_up = kills(b, a) && mana_value(g, a) >= mana_value(g, b);
            (survives && kills(b, a), survives, trade_up, g.power(a))
        });
    match good {
        Some((b, a)) if finishable(b, a) => Action::Block {
            blocker: b,
            attacker: a,
        },
        _ => Action::FinishBlocks,
    }
}

/// Discard the most expensive card, or a land when holding more than two.
fn discard(g: &Game, p: PlayerId, legal: &[Action]) -> Option<Action> {
    let flooded = lands_in_hand(g, p) > 2;
    legal
        .iter()
        .copied()
        .filter_map(|a| match a {
            Action::Discard(c) => Some((c, a)),
            _ => None,
        })
        .max_by_key(|&(c, _)| (g.def_of(c).types.land == flooded, mana_value(g, c)))
        .map(|(_, a)| a)
}

/// Whether the ability being put on the stack hurts what it targets.
fn harmful(g: &Game) -> Option<bool> {
    let c = g.choosing.as_ref()?;
    let def = g.def_of(c.item.source);
    if def.aura {
        // Auras that strip abilities or shrink are removal; the rest help.
        return Some(
            def.statics
                .iter()
                .any(|s| s.lose_abilities || s.power < 0 || s.becomes_land),
        );
    }
    let ability = g.ability_of(&c.item);
    let hurts = ability.effects.iter().any(|e| {
        matches!(
            e,
            Effect::Damage { .. }
                | Effect::Destroy(_)
                | Effect::Exile(_)
                | Effect::Counter(_)
                | Effect::TapAndStun(_)
                | Effect::GainControlUntilEot(_)
                | Effect::DividedDamage
                | Effect::Pump { power: ..=-1, .. }
        )
    });
    Some(hurts)
}

/// Hurt the opponent's biggest thing, help your own biggest creature.
fn target(g: &Game, p: PlayerId, legal: &[Action]) -> Option<Action> {
    let hurts = harmful(g).unwrap_or(false);
    let want = if hurts { 1 - p } else { p };
    let score = |t: Target| -> Option<i32> {
        match t {
            Target::Player(q) => (q == want).then_some(0),
            Target::Object(x) => {
                let o = g.obj(x);
                let mine = o.controller == want || o.zone != Zone::Battlefield;
                mine.then(|| if g.types_of(x).creature { 1 + g.power(x) } else { 1 })
            }
        }
    };
    let best = legal
        .iter()
        .copied()
        .filter_map(|a| match a {
            Action::Target(t) => score(t).map(|s| (s, a)),
            _ => None,
        })
        .max_by_key(|&(s, _)| s)
        .map(|(_, a)| a);
    // Nothing worth targeting: stop choosing if that's allowed.
    best.or_else(|| legal.contains(&Action::No).then_some(Action::No))
}
