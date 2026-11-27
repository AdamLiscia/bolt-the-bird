//! Regressions for the smart pilot's Commander panics before the playgroup demo: about one call in
//! twenty panicked inside a playout. The playouts weren't at fault: they played the games out far
//! enough to reach two engine bugs that baseline-only games hit too.
//!
//! - Misdirection ("target spell with a single target") in hand while a permanent spell (Sol Ring)
//!   is on the stack: checking whether it could be cast looked up the permanent spell's targets in a
//!   spell ability it doesn't have.
//! - Nissa, Vastwood Seer with two land triggers on the stack: the first transformed her, and the
//!   second resolved against her back face, which has no triggers.

mod common;

use bolt_cards::set;
use bolt_core::rng::Rng;
use bolt_core::{baseline, rollout, Action, Game, Pending, StackItem, StackKind, Step, Zone};
use common::{act, resolve_all};

fn cid(name: &str) -> usize {
    set("CMD")
        .unwrap()
        .index_of(name)
        .unwrap_or_else(|| panic!("{name} is not in the CMD pool"))
}

/// Player 0's first main phase on turn 3 of a Commander game (99 Plains each), each player with 5 of
/// each basic land.
fn table() -> Game {
    let plains = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        set("CMD").unwrap().defs.clone(),
        [&plains, &plains],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    for p in 0..2 {
        for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            for _ in 0..5 {
                g.add_card(p, cid(land), Zone::Battlefield);
            }
        }
    }
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g
}

#[test]
fn misdirection_in_hand_while_a_permanent_spell_is_on_the_stack() {
    let mut g = table();
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Hand);
    let mis = g.add_card(1, cid("Misdirection"), Zone::Hand);
    act(&mut g, Action::Cast(ring));
    act(&mut g, Action::PassPriority); // player 1's priority: their legal moves are worked out
    assert_eq!(g.pending, Pending::Priority(1));
    let legal = g.legal_actions();
    assert!(
        !legal
            .iter()
            .any(|&a| matches!(a, Action::Cast(c) | Action::CastWith(c, _) if c == mis)),
        "Sol Ring has no target to change: {legal:?}"
    );
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ring).zone, Zone::Battlefield);
}

#[test]
fn nissa_with_a_second_land_trigger_after_she_transforms() {
    let mut g = table();
    let nissa = g.add_card(
        0,
        cid("Nissa, Vastwood Seer // Nissa, Sage Animist"),
        Zone::Battlefield,
    );
    let woods = g.add_card(0, cid("Awaken the Woods"), Zone::Hand);
    // Two Forest Dryads: two lands enter at once, two of Nissa's land triggers.
    act(&mut g, Action::CastX(woods, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(nissa).zone, Zone::Battlefield);
    assert_eq!(g.def_of(nissa).name, "Nissa, Sage Animist");
}

/// A playgroup deck from tests/data/cmd_repro_decks.txt: (commanders, the other cards).
fn deck(n: usize) -> (Vec<usize>, Vec<usize>) {
    let line = include_str!("data/cmd_repro_decks.txt")
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .nth(n)
        .expect("a deck");
    let (cmd, cards) = line.split_once(" | ").expect("commanders | cards");
    let ids = |s: &str| s.split("; ").map(cid).collect::<Vec<_>>();
    (ids(cmd), ids(cards))
}

/// Lane T's repro, without the search's safety net: the game `bolt_engine.Game.commander` builds
/// (decks `a` and `b` of the data file, `seed`, `first = seed % 2`), stepped by the baseline to the
/// first priority; then every legal move there is played out in the guessed world the smart pilot's
/// first rollout uses (`smart(1, 2, seed)`), baseline on both sides, to the end.
fn first_priority_playouts(a: usize, b: usize, seed: u64) {
    let ((ca, da), (cb, db)) = (deck(a), deck(b));
    let mut g = Game::new_commander_with(
        set("CMD").unwrap().defs.clone(),
        [&da, &db],
        [&ca, &cb],
        bolt_core::CommanderOptions::default(),
        seed,
        (seed % 2) as usize,
    );
    g.commander_choice = true;
    while !matches!(g.pending, Pending::Priority(_)) {
        let legal = g.legal_actions();
        g.apply(baseline::choose(&g, &legal)).expect("legal");
    }
    let p = g.pending.player().expect("a player");
    let world = Rng::new(seed).next_u64();
    for a in g.legal_actions() {
        let mut h = g.clone();
        h.manual_mana = [false; 2];
        h.determinize(p, world);
        if h.apply(a).is_err() {
            continue;
        }
        for _ in 0..5_000 {
            if h.result.is_some() || h.objects.len() > 2_000 {
                break;
            }
            let legal = h.legal_actions();
            h.apply(baseline::choose(&h, &legal)).expect("legal");
        }
    }
    // And the pilot itself answers.
    let legal = g.legal_actions();
    let cfg = rollout::Config {
        rollouts: 1,
        max_candidates: 2,
        ..Default::default()
    };
    assert!(legal.contains(&rollout::choose(&g, &legal, &cfg, seed)));
}

/// Xenagos (deck 0) vs. Aesi (deck 3), seed 0: "index out of bounds" in `ability_of` (Nissa).
#[test]
fn xenagos_vs_aesi_seed_0_plays_out() {
    first_priority_playouts(0, 1, 0);
}

/// Tatyova (deck 6) vs. Myrkul (deck 4), seed 0: "a spell with an ability" (Misdirection).
#[test]
fn tatyova_vs_myrkul_seed_0_plays_out() {
    first_priority_playouts(3, 2, 0);
}

/// The search's safety net: a stack entry that panics when it resolves (a trigger its source doesn't
/// have) makes every playout panic. The pilot scores those as losses and plays the baseline's move,
/// and never panics out to its caller.
#[test]
fn a_panicking_playout_scores_as_a_loss() {
    let mut g = table();
    let plains = g.add_card(0, cid("Plains"), Zone::Battlefield);
    g.add_card(0, cid("Opt"), Zone::Hand); // an instant: a move besides passing
    g.items.push(StackItem {
        source: plains,
        controller: 0,
        kind: StackKind::Trigger(99),
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
    let legal = g.legal_actions();
    assert!(legal.len() > 1, "a real choice: {legal:?}");
    let cfg = rollout::Config {
        rollouts: 2,
        max_candidates: 4,
        ..Default::default()
    };
    assert_eq!(rollout::choose(&g, &legal, &cfg, 1), baseline::choose(&g, &legal));
}

/// With no time to search, the pilot plays the baseline's move; with a little, it stops on time.
#[test]
fn the_time_limit_stops_the_search() {
    let mut g = table();
    g.add_card(0, cid("Sol Ring"), Zone::Hand);
    g.add_card(0, cid("Forest"), Zone::Hand);
    let legal = g.legal_actions();
    assert!(legal.len() > 1, "a real choice: {legal:?}");
    let none = rollout::Config {
        rollouts: 1_000,
        max_candidates: 8,
        time_limit: Some(std::time::Duration::ZERO),
        ..Default::default()
    };
    assert_eq!(
        rollout::choose(&g, &legal, &none, 1),
        baseline::choose(&g, &legal)
    );
    let some = rollout::Config {
        time_limit: Some(std::time::Duration::from_millis(200)),
        ..none
    };
    let t = std::time::Instant::now();
    let a = rollout::choose(&g, &legal, &some, 1);
    assert!(legal.contains(&a));
    assert!(t.elapsed().as_secs() < 10, "{:?} for a 200 ms limit", t.elapsed());
}
