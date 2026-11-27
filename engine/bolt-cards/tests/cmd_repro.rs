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
use bolt_core::{baseline, rollout, Action, Game, Pending, StackItem, StackKind, Step, Target, Zone};
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

/// Nissa at 6 lands; Awaken the Woods makes two Forest Dryads, so two land triggers go on the stack.
/// The first exiles her and returns her transformed. By then she's a new object (CR 400.7), so the
/// second can't find her and does nothing: she isn't flickered back to her front face.
#[test]
fn nissa_with_a_second_land_trigger_after_she_transforms() {
    let plains = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        set("CMD").unwrap().defs.clone(),
        [&plains, &plains],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    for _ in 0..6 {
        g.add_card(0, cid("Forest"), Zone::Battlefield);
    }
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let nissa = g.add_card(
        0,
        cid("Nissa, Vastwood Seer // Nissa, Sage Animist"),
        Zone::Battlefield,
    );
    let woods = g.add_card(0, cid("Awaken the Woods"), Zone::Hand);
    let zcc = g.obj(nissa).zcc;
    act(&mut g, Action::CastX(woods, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(nissa).zone, Zone::Battlefield);
    assert_eq!(g.def_of(nissa).name, "Nissa, Sage Animist");
    assert_eq!(g.obj(nissa).zcc, zcc + 2, "exiled and returned once, not twice");
    assert_eq!(g.obj(nissa).loyalty, 3);
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

/// The search's safety net: a stack entry that panics when it resolves (its source doesn't exist)
/// makes every playout panic. The pilot scores those as losses and plays the baseline's move,
/// and never panics out to its caller.
#[test]
fn a_panicking_playout_scores_as_a_loss() {
    let mut g = table();
    g.add_card(0, cid("Opt"), Zone::Hand); // an instant: a move besides passing
    g.items.push(StackItem {
        source: bolt_core::ObjId(1_000_000), // no such object: resolving it panics
        controller: 0,
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
        source_zcc: None,
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

/// Desktop Commander training, 2026-11-27: "index out of bounds: the len is 0 but the index is 1" in
/// `legal_actions` (choosing targets). Jaxis, the Troublemaker cast for blitz dies: its granted "when
/// this creature dies, draw a card" came with the creature that died as a target its ability has no
/// slot for, so the engine asked for a second target that doesn't exist.
#[test]
fn jaxis_blitzed_dies_and_draws() {
    let mut g = table();
    let jaxis = g.add_card(0, cid("Jaxis, the Troublemaker"), Zone::Hand);
    act(&mut g, Action::CastAlternative(jaxis));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(jaxis).zone, Zone::Battlefield);
    let hand = g.players[0].hand.len();
    g.move_card(jaxis, Zone::Graveyard); // it dies
    act(&mut g, Action::PassPriority);
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.players[0].hand.len(),
        hand + 1,
        "blitz: when it dies, draw a card"
    );
}

/// The same training run: "the len is 0 but the index is 0" in `still_legal`. Shifting Woodland's
/// copy ability activated twice: the first to resolve made it a copy of a creature card with no such
/// ability, and the other then resolved as that card's (missing) ability instead of the one activated.
#[test]
fn shifting_woodland_activated_twice_copies_both_in_turn() {
    let mut g = table();
    let woodland = g.add_card(0, cid("Shifting Woodland"), Zone::Battlefield);
    let cobra = g.add_card(0, cid("Lotus Cobra"), Zone::Graveyard);
    let oracle = g.add_card(0, cid("Coiling Oracle"), Zone::Graveyard);
    for c in ["Sol Ring", "Opt", "Rampant Growth", "Forest"] {
        g.add_card(0, cid(c), Zone::Graveyard); // delirium: artifact, instant, sorcery, land
    }
    for t in [cobra, oracle] {
        act(&mut g, Action::Activate(woodland, 0));
        act(&mut g, Action::Target(Target::Object(t)));
    }
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.def_of(woodland).name,
        "Lotus Cobra",
        "the first activation resolves last"
    );
}

/// Self-play, 2026-11-27: "a trigger of its source". Phyrexian Metamorph copying Agate Instigator
/// left the battlefield (Reclamation Sage) with the copied "whenever another creature you control
/// enters" trigger on the stack. Back to its own card, it had no such trigger; the trigger still
/// resolves as the copied card's (CR 113.7a).
#[test]
fn a_copys_trigger_resolves_after_the_copy_leaves() {
    let mut g = table();
    let agate = g.add_card(0, cid("Agate Instigator"), Zone::Battlefield);
    let meta = g.add_card(0, cid("Phyrexian Metamorph"), Zone::Hand);
    act(&mut g, Action::Cast(meta));
    resolve_all(&mut g, &[Action::Pick(agate)]);
    assert_eq!(g.def_of(meta).name, "Agate Instigator", "Metamorph copied it");
    let life = g.players[1].life;
    let elves = g.add_card(0, cid("Llanowar Elves"), Zone::Hand);
    act(&mut g, Action::Cast(elves));
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::PassPriority);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
    assert_eq!(g.items.len(), 2, "both Instigators' triggers: {:?}", g.items);
    g.move_card(meta, Zone::Graveyard); // destroyed in response
    resolve_all(&mut g, &[]);
    assert_eq!(g.def_of(meta).name, "Phyrexian Metamorph");
    assert_eq!(g.players[1].life, life - 2, "both triggers deal their 1");
}

/// Lane T's runaway hunt, 2026-11-27: Chain of Smog's "that player may copy this spell" asked again
/// and again, the rules player always taking the first choice (a copy aimed at player 0, itself
/// included). It now copies only at an opponent with cards to discard, so the chain ends.
#[test]
fn the_rules_player_ends_a_chain_of_smog() {
    let mut g = table();
    let smog = g.add_card(0, cid("Chain of Smog"), Zone::Hand);
    g.add_card(0, cid("Plains"), Zone::Hand); // player 0 keeps one card
    for _ in 0..3 {
        g.add_card(1, cid("Plains"), Zone::Hand);
    }
    act(&mut g, Action::Cast(smog));
    act(&mut g, Action::Target(Target::Player(1)));
    let mut steps = 0;
    while !g.items.is_empty() || !matches!(g.pending, Pending::Priority(_)) {
        let legal = g.legal_actions();
        let a = baseline::choose(&g, &legal);
        act(&mut g, a);
        steps += 1;
        assert!(steps < 200, "the chain never ended: {:?}", g.pending);
    }
    // Player 1 discards 2 and copies it at player 0 (1 card), who discards it and copies it at
    // player 1's last card; then neither has a card to lose and the chain stops.
    assert!(g.players[0].hand.is_empty() && g.players[1].hand.is_empty());
}

/// Desktop training, 2026-11-27: 7 "a trigger of its source" panics. A trigger no definition has (no
/// snapshot, current definition, front or back face) now resolves as doing nothing.
#[test]
fn a_trigger_its_source_doesnt_have_does_nothing() {
    let mut g = table();
    let plains = g.add_card(0, cid("Plains"), Zone::Battlefield);
    g.items.push(StackItem {
        source: plains,
        controller: 0,
        kind: StackKind::Trigger(99),
        targets: Vec::new(),
        kicked: false,
        flashback: false,
        extra: None,
        mode: Some(3),
        amount: 0,
        alt: false,
        x: 0,
        division: Vec::new(),
        source_zcc: None,
    });
    let life = (g.players[0].life, g.players[1].life);
    resolve_all(&mut g, &[]);
    assert!(g.items.is_empty());
    assert_eq!((g.players[0].life, g.players[1].life), life);
    assert_eq!(g.obj(plains).zone, Zone::Battlefield);
}

/// The desktop's cmd_play_1, 2026-11-27: one engine call spun for 9.5+ minutes. The engine's loop
/// caps end such a game in a draw with a reason instead. Past MAX_OBJECTS, the next object made
/// ends it.
#[test]
fn a_runaway_board_ends_the_game_in_a_draw() {
    let mut g = table();
    while g.objects.len() < bolt_core::game::MAX_OBJECTS {
        g.add_card(0, cid("Plains"), Zone::Exile);
    }
    assert!(g.result.is_none());
    g.add_card(0, cid("Plains"), Zone::Exile);
    assert_eq!(g.result, Some(bolt_core::Outcome::Draw));
    assert!(g.end_reason.is_some_and(|r| r.contains("MAX_OBJECTS")));
    assert!(g.legal_actions().is_empty() || g.is_over());
}

/// Forced passes with nobody able to act or lose (empty libraries, but "you can't lose the game"):
/// `settle` used to pass turn after turn forever inside one call. A loop cap now ends it in a draw.
#[test]
fn turns_with_no_decisions_forever_end_in_a_draw() {
    let mut defs = (*set("CMD").unwrap().defs).clone();
    let plains = cid("Plains");
    defs[plains].cant_lose = true;
    let mut g = Game::empty(std::sync::Arc::new(defs), 1, 0);
    for p in 0..2 {
        g.add_card(p, plains, Zone::Battlefield);
    }
    g.auto_pass = true;
    g.start_turn(1, 0);
    let t = std::time::Instant::now();
    while !g.is_over() && t.elapsed().as_secs() < 60 {
        let legal = g.legal_actions();
        g.apply(baseline::choose(&g, &legal)).expect("legal");
    }
    assert_eq!(g.result, Some(bolt_core::Outcome::Draw));
    assert!(g.end_reason.is_some(), "turn {}: no loop cap tripped", g.turn);
}

/// Desktop self-play, 2026-11-27: Scute Swarm copies doubling on every land drop reached 549
/// permanents, every step slower. Past MAX_PERMANENTS the game is a draw, "board too large".
#[test]
fn scute_swarm_past_the_permanent_cap_ends_in_a_draw() {
    let mut g = table();
    let swarm = cid("Scute Swarm");
    while g.battlefield.len() < bolt_core::game::MAX_PERMANENTS - 1 {
        g.add_card(0, swarm, Zone::Battlefield);
    }
    // Two lands entering: every Swarm's landfall makes a copy (six or more lands), far past the cap.
    let woods = g.add_card(0, cid("Awaken the Woods"), Zone::Hand);
    act(&mut g, Action::CastX(woods, 2));
    let mut steps = 0;
    while !g.is_over() && steps < 10_000 {
        let legal = g.legal_actions();
        let a = baseline::choose(&g, &legal);
        act(&mut g, a);
        steps += 1;
    }
    assert_eq!(g.result, Some(bolt_core::Outcome::Draw));
    assert!(
        g.end_reason.is_some_and(|r| r.contains("board too large")),
        "{:?}",
        g.end_reason
    );
    assert!(
        g.battlefield.len() < 2 * bolt_core::game::MAX_PERMANENTS,
        "{}",
        g.battlefield.len()
    );
}

/// Farm, 2026-11-27: `finish_cast` panicked "payable": a spell offered as castable whose mana, as
/// cast, couldn't be paid. Forced here with a cost that depends on the target (Dragon's Prey's
/// "{2} more if it targets a Dragon", patched onto Lightning Bolt): castable with one Mountain, but
/// not at an Elf. The cast is rolled back (the card in hand, the Mountain untapped, nothing on the
/// stack), and that cast isn't offered again this step.
#[test]
fn an_unpayable_cast_rolls_back_instead_of_panicking() {
    let mut defs = (*set("CMD").unwrap().defs).clone();
    let bolt = cid("Lightning Bolt");
    defs[bolt].reduction = Some(bolt_core::Reduction::MoreIfTargetsSubtype("Elf", 5));
    let mut g = Game::empty(std::sync::Arc::new(defs), 1, 0);
    g.auto_pass = false;
    let mountain = g.add_card(0, cid("Mountain"), Zone::Battlefield);
    let elves = g.add_card(1, cid("Llanowar Elves"), Zone::Battlefield);
    let card = g.add_card(0, bolt, Zone::Hand);
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, cid("Plains"), Zone::Library); // no one decks out
        }
    }
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert!(g.legal_actions().contains(&Action::Cast(card)));
    act(&mut g, Action::Cast(card));
    act(&mut g, Action::Target(Target::Object(elves))); // {R} + {5}: can't pay
    assert_eq!(g.obj(card).zone, Zone::Hand, "rolled back");
    assert!(!g.obj(mountain).tapped && g.items.is_empty() && g.choosing.is_none());
    assert_eq!(g.pending, Pending::Priority(0));
    assert!(
        !g.legal_actions().contains(&Action::Cast(card)),
        "not offered again this step"
    );
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
}
