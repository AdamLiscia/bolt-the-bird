//! The rollout pilot: guessing hidden cards (determinizing) and choosing moves by playing them out.

use std::collections::BTreeMap;

use bolt_cards::{fdn, supported_mask};
use bolt_core::fuzz::random_deck;
use bolt_core::rng::Rng;
use bolt_core::{baseline, rollout, Game, Pending, StackItem, StackKind, Zone};

fn decks(seed: u64) -> (Vec<usize>, Vec<usize>) {
    let defs = fdn();
    let mask = supported_mask();
    let lands: Vec<usize> = (0..mask.len())
        .filter(|&i| mask[i] && defs[i].types.land)
        .collect();
    let spells: Vec<usize> = (0..mask.len())
        .filter(|&i| mask[i] && !defs[i].types.land)
        .collect();
    let mut rng = Rng::new(seed);
    (
        random_deck(&lands, &spells, &mut rng),
        random_deck(&lands, &spells, &mut rng),
    )
}

/// A game a few turns in, both sides played by the baseline.
fn midgame(seed: u64) -> Game {
    let (a, b) = decks(seed);
    let mut g = Game::new(fdn(), [&a, &b], seed, 0);
    while g.result.is_none() && g.turn < 5 {
        let legal = g.legal_actions();
        g.apply(baseline::choose(&g, &legal)).expect("legal");
    }
    g
}

fn defs_in(g: &Game, ids: &[bolt_core::ObjId]) -> Vec<usize> {
    ids.iter().map(|&c| g.obj(c).def).collect()
}

fn multiset(v: impl IntoIterator<Item = usize>) -> BTreeMap<usize, usize> {
    let mut m = BTreeMap::new();
    for x in v {
        *m.entry(x).or_default() += 1;
    }
    m
}

#[test]
fn pending_names_the_player_to_act() {
    assert_eq!(Pending::Priority(1).player(), Some(1));
    assert_eq!(Pending::Bottom(0, 2).player(), Some(0));
    assert_eq!(Pending::GameOver.player(), None);
}

#[test]
fn determinizing_keeps_what_the_viewer_can_see() {
    let g = midgame(3);
    let (me, opp) = (0, 1);
    let mut h = g.clone();
    h.determinize(me, 42);
    // My hand, every battlefield and graveyard, and the zone sizes stay the same.
    assert_eq!(h.players[me].hand, g.players[me].hand);
    assert_eq!(defs_in(&h, &h.players[me].hand), defs_in(&g, &g.players[me].hand));
    for p in [me, opp] {
        assert_eq!(h.players[p].hand.len(), g.players[p].hand.len());
        assert_eq!(h.players[p].library.len(), g.players[p].library.len());
        assert_eq!(
            defs_in(&h, &h.players[p].graveyard),
            defs_in(&g, &g.players[p].graveyard)
        );
    }
    let battlefield = |g: &Game| -> Vec<(u32, usize)> {
        g.objects
            .iter()
            .filter(|o| o.zone == Zone::Battlefield)
            .map(|o| (o.id.0, o.def))
            .collect()
    };
    assert_eq!(battlefield(&h), battlefield(&g));
    // The opponent's hidden cards are the same cards, only redistributed.
    let hidden = |g: &Game, p: usize| {
        multiset(
            defs_in(g, &g.players[p].hand)
                .into_iter()
                .chain(defs_in(g, &g.players[p].library)),
        )
    };
    assert_eq!(hidden(&h, opp), hidden(&g, opp));
    assert_eq!(
        multiset(defs_in(&h, &h.players[me].library)),
        multiset(defs_in(&g, &g.players[me].library))
    );
    // Some seed actually changes the opponent's hand.
    let changed = (0..20).any(|s| {
        let mut h = g.clone();
        h.determinize(me, s);
        defs_in(&h, &h.players[opp].hand) != defs_in(&g, &g.players[opp].hand)
    });
    assert!(changed, "determinizing never changed the opponent's hand");
}

#[test]
fn determinizing_is_reproducible_from_its_seed() {
    let g = midgame(5);
    let (mut a, mut b) = (g.clone(), g.clone());
    a.determinize(0, 7);
    b.determinize(0, 7);
    assert_eq!(defs_in(&a, &a.players[1].hand), defs_in(&b, &b.players[1].hand));
    assert_eq!(
        defs_in(&a, &a.players[0].library),
        defs_in(&b, &b.players[0].library)
    );
}

#[test]
fn the_rollout_pilot_plays_legal_moves_to_the_end() {
    let cfg = rollout::Config {
        rollouts: 2,
        max_candidates: 4,
        max_actions: 3_000,
        max_objects: 2_000,
    };
    for seed in 0..3 {
        let (a, b) = decks(100 + seed);
        let mut g = Game::new(fdn(), [&a, &b], seed, 0);
        let mut n = 0u64;
        while g.result.is_none() {
            let legal = g.legal_actions();
            let action = if g.pending.player() == Some(0) {
                rollout::choose(&g, &legal, &cfg, seed * 100_000 + n)
            } else {
                baseline::choose(&g, &legal)
            };
            assert!(legal.contains(&action), "{action:?} is not legal");
            g.apply(action).expect("legal");
            n += 1;
            assert!(n < 50_000, "seed {seed}: no result");
        }
    }
}

#[test]
fn the_rollout_pilot_is_deterministic() {
    let g = midgame(9);
    let legal = g.legal_actions();
    let cfg = rollout::Config {
        rollouts: 3,
        max_candidates: 6,
        max_actions: 3_000,
        max_objects: 2_000,
    };
    assert_eq!(
        rollout::choose(&g, &legal, &cfg, 1),
        rollout::choose(&g, &legal, &cfg, 1)
    );
}

/// A playout whose board grows past `max_objects` stops and counts as a draw, so a board that
/// doubles every turn (Homunculus Horde with an extra draw) can't stall one decision for minutes.
/// With a limit below the starting board, every playout is a draw and the baseline's pick stands.
#[test]
fn a_playout_past_the_object_limit_counts_as_a_draw() {
    let cfg = rollout::Config {
        rollouts: 3,
        max_candidates: 6,
        max_actions: 3_000,
        max_objects: 1,
    };
    let mut searched = 0;
    for seed in 0..12 {
        let g = midgame(200 + seed);
        let legal = g.legal_actions();
        if legal.len() > 1 && matches!(g.pending, Pending::Priority(_)) {
            searched += 1;
        }
        assert_eq!(rollout::choose(&g, &legal, &cfg, seed), baseline::choose(&g, &legal), "seed {seed}");
    }
    assert!(searched > 0, "no position had a real choice");
}

/// Regression: a trigger whose source card is in a hidden zone (shuffled or drawn since) kept
/// pointing at that card after determinizing, which then had no such trigger and panicked.
#[test]
fn determinizing_keeps_cards_the_stack_refers_to() {
    let mut g = midgame(3);
    let defs = fdn();
    let card = g.players[1]
        .library
        .iter()
        .copied()
        .find(|&c| !defs[g.obj(c).def].triggers.is_empty())
        .expect("a card with a trigger in the library");
    let def = g.obj(card).def;
    g.triggered.push(StackItem {
        source: card,
        controller: 1,
        kind: StackKind::Trigger(0),
        targets: Vec::new(),
        kicked: false,
        flashback: false,
        extra: None,
        mode: None,
        amount: 0,
        alt: false,
        x: 0,
        division: Vec::new(),
    });
    for seed in 0..20 {
        let mut h = g.clone();
        h.determinize(0, seed);
        assert_eq!(h.obj(card).def, def, "seed {seed}");
    }
}
