//! Property test: every spell `legal_actions` offers as castable can be paid when it's cast.
//!
//! Random Commander positions from the playable decks (handoff/cmd/playable_decks.txt), stepped by
//! the baseline pilot, seeded. At each stopping point, every cast action on offer is tried on a copy
//! of the game, its targets and decisions answered as a player could (random legal answers), until
//! the spell is on the stack. `finish_cast` panicking with "payable" means the castability check and
//! the payment disagreed (the 2d177cf `pay_mana_for` "payable" panic lane S is hunting).
//!
//! Ignored until lane S's fix lands (target choosing offers only targets that keep the spell
//! payable, and the cast is offered only if some affordable target exists, CR 601.2g-h): the
//! castability check prices the spell with no targets while finish_cast prices it with the chosen
//! ones. Bolt's gate runs it with `--include-ignored`. `BOLT_CAST_POSITIONS` sets how many
//! positions (default 300).

use std::panic::{catch_unwind, AssertUnwindSafe};

use bolt_cards::set;
use bolt_core::rng::Rng;
use bolt_core::{baseline, Action, Game, ObjId, Pending, Step, Zone};

/// The playable decks: (commanders, the other cards, the line's comment).
fn decks() -> Vec<(Vec<usize>, Vec<usize>, String)> {
    include_str!("../../../handoff/cmd/playable_decks.txt")
        .lines()
        .filter(|l| l.contains(" | "))
        .map(|l| {
            let (body, note) = l.split_once('#').unwrap_or((l, ""));
            let (cmd, cards) = body.split_once(" | ").expect("commanders | cards");
            let ids = |s: &str| {
                s.split_whitespace()
                    .map(|x| x.parse().expect("a card number"))
                    .collect()
            };
            (ids(cmd), ids(cards), note.trim().to_string())
        })
        .collect()
}

/// The card a cast action casts, if it is one.
fn cast_card(a: &Action) -> Option<ObjId> {
    let s = format!("{a:?}");
    if !s.starts_with("Cast") {
        return None;
    }
    // Every cast action carries the card first: "CastX(ObjId(12), 3)".
    let n: String = s
        .split("ObjId(")
        .nth(1)?
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    n.parse().ok().map(ObjId)
}

fn panic_text(e: &(dyn std::any::Any + Send)) -> String {
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default()
}

#[test]
#[ignore = "fails until lane S's CR 601.2g-h target-affordability fix lands; Bolt's gate runs it"]
fn every_castable_spell_can_be_paid() {
    let positions: u64 = std::env::var("BOLT_CAST_POSITIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);
    let decks = decks();
    assert!(decks.len() >= 2, "the playable decks");
    let defs = set("CMD").unwrap().defs.clone();
    std::panic::set_hook(Box::new(|_| {})); // the panics are collected below
    let mut failures: Vec<String> = Vec::new();
    let mut tried = 0usize;
    for seed in 0..positions {
        let (a, b) = (
            (seed as usize) % decks.len(),
            (seed as usize * 7 + 3) % decks.len(),
        );
        let ((ca, da, na), (cb, db, nb)) = (&decks[a], &decks[b]);
        let mut rng = Rng::new(seed);
        // Odd seeds: manual mana (mana tapped and floated first) and some random moves.
        let wild = seed % 2 == 1;
        let mut g = Game::new_commander_with(
            defs.clone(),
            [da, db],
            [ca, cb],
            bolt_core::CommanderOptions::default(),
            seed,
            (seed % 2) as usize,
        );
        g.manual_mana = [wild; 2];
        let mut tried_here = 0;
        for step in 0..400 {
            if g.result.is_some() || g.objects.len() > 1_500 {
                break;
            }
            if matches!(g.pending, Pending::Priority(_)) && step % 3 == 0 && tried_here < 30 {
                for cast in g.legal_actions().into_iter().filter(|a| cast_card(a).is_some()) {
                    let card = cast_card(&cast).expect("a cast");
                    let name = g.def_of(card).name.clone();
                    let mut answers: Vec<Action> = Vec::new();
                    let mut pick = Rng::new(seed ^ ((card.0 as u64) << 8) ^ step as u64);
                    let mut h = g.clone();
                    tried += 1;
                    tried_here += 1;
                    let run = catch_unwind(AssertUnwindSafe(|| {
                        h.apply(cast).expect("legal");
                        for _ in 0..200 {
                            if matches!(h.pending, Pending::Priority(_)) || h.result.is_some() {
                                break;
                            }
                            let legal = h.legal_actions();
                            let a = legal[pick.below(legal.len())];
                            answers.push(a);
                            h.apply(a).expect("legal");
                        }
                    }));
                    if let Err(e) = run {
                        let t = panic_text(&*e);
                        if t.contains("payable") {
                            failures.push(format!(
                                "seed {seed} ({na} vs {nb}), step {step}, manual mana {wild}: {name} via {cast:?}, answers {answers:?}: {t}"
                            ));
                        }
                    }
                }
            }
            let legal = g.legal_actions();
            let a = if wild && rng.below(4) == 0 {
                legal[rng.below(legal.len())]
            } else {
                baseline::choose(&g, &legal)
            };
            let moved = catch_unwind(AssertUnwindSafe(|| {
                let mut h = g.clone();
                h.apply(a).expect("legal");
                h
            }));
            match moved {
                Ok(h) => g = h,
                Err(e) => {
                    let t = panic_text(&*e);
                    if t.contains("payable") {
                        failures.push(format!(
                            "seed {seed} ({na} vs {nb}), step {step}: playing {a:?}: {t}"
                        ));
                    }
                    break;
                }
            }
        }
    }
    let _ = std::panic::take_hook();
    eprintln!("{positions} positions, {tried} casts tried");
    assert!(tried > 0, "some casts were tried");
    assert!(
        failures.is_empty(),
        "{} unpayable casts:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn cid(name: &str) -> usize {
    set("CMD")
        .unwrap()
        .index_of(name)
        .unwrap_or_else(|| panic!("{name} is not in the CMD pool"))
}

/// The farm's 9166 panic, reduced: The Great Henge costs {X} less, X the greatest power among your
/// creatures, and Psychosis Crawler's power is your hand size. The card moves to the stack before the
/// cost is fixed (CR 601.2a), so the Crawler is one smaller when Henge is paid. Henge must either not
/// be offered or be payable. Ignored until lane S's fix (price a cast as if the card had left its
/// zone: hand-size and graveyard-count terms).
#[test]
#[ignore = "fails until lane S prices a cast with the card already off its zone (CR 601.2a)"]
fn the_great_henge_with_psychosis_crawler_is_payable_when_offered() {
    let plains = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        set("CMD").unwrap().defs.clone(),
        [&plains, &plains],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        g.apply(Action::PassPriority).expect("legal");
    }
    for c in g.players[0].hand.clone() {
        g.move_card(c, Zone::Library);
    }
    for land in ["Forest", "Forest", "Plains"] {
        g.add_card(0, cid(land), Zone::Battlefield);
    }
    g.add_card(0, cid("Psychosis Crawler"), Zone::Battlefield);
    let henge = g.add_card(0, cid("The Great Henge"), Zone::Hand);
    for _ in 0..5 {
        g.add_card(0, cid("Plains"), Zone::Hand);
    }
    // Six cards in hand: Crawler 6/6, so Henge looks like {1}{G}{G}; on the stack it's {2}{G}{G}.
    if g.legal_actions().contains(&Action::Cast(henge)) {
        let cast = catch_unwind(AssertUnwindSafe(|| g.apply(Action::Cast(henge)).map(|_| ())));
        assert!(
            matches!(cast, Ok(Ok(()))),
            "offered with three lands but not payable: {:?}",
            cast.map_err(|e| panic_text(&*e))
        );
    }
}
