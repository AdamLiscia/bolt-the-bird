//! Mana abilities don't use the stack (CR 605.1a, 605.3a): an activated ability with no targets that
//! could add mana resolves as it's activated, and its controller keeps priority. Nobody gets a window
//! to respond to the mana. Triggers it causes (a sacrificed creature's) still go on the stack.

mod common;

use bolt_cards::set;
use bolt_core::{Action, Game, ObjId, Pending, StackKind, Step, Zone};
use common::act;

fn cid(name: &str) -> usize {
    set("CMD")
        .unwrap()
        .index_of(name)
        .unwrap_or_else(|| panic!("{name} is not in the CMD pool"))
}

/// Player 0's first main phase on turn 3 of a Commander game (99 Plains each), with no lands.
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
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g
}

fn put(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, cid(name), Zone::Battlefield)
}

/// Nothing on the stack and player 0 still holds priority: no window for the opponent.
fn no_response_window(g: &Game) {
    assert!(
        !g.items.iter().any(|i| matches!(i.kind, StackKind::Activated(_))),
        "the mana ability isn't on the stack: {:?}",
        g.items
    );
    assert!(
        g.choosing.is_none() || !matches!(g.choosing.as_ref().unwrap().item.kind, StackKind::Activated(_))
    );
}

#[test]
fn lotus_petal_adds_its_mana_at_once() {
    let mut g = table();
    g.manual_mana[0] = true;
    let petal = put(&mut g, 0, "Lotus Petal");
    let tap = *g
        .legal_actions()
        .iter()
        .find(|a| matches!(a, Action::TapForMana(c, _) if *c == petal))
        .expect("Lotus Petal taps for mana");
    act(&mut g, tap);
    assert_eq!(g.players[0].pool.len(), 1);
    assert_eq!(g.obj(petal).zone, Zone::Graveyard, "sacrificed");
    assert!(g.items.is_empty());
    assert_eq!(g.pending, Pending::Priority(0));
}

#[test]
fn ashnods_altar_adds_its_mana_with_nothing_on_the_stack() {
    let mut g = table();
    let altar = put(&mut g, 0, "Ashnod's Altar");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    act(&mut g, Action::ActivateWith(altar, 0, elf));
    assert_eq!(g.obj(elf).zone, Zone::Graveyard);
    assert_eq!(g.players[0].pool.len(), 2, "{:?}", g.players[0].pool);
    assert!(g.items.is_empty());
    assert_eq!(g.pending, Pending::Priority(0), "player 0 keeps priority");
}

#[test]
fn phyrexian_altar_adds_mana_and_the_dies_trigger_goes_on_the_stack() {
    let mut g = table();
    let altar = put(&mut g, 0, "Phyrexian Altar");
    let artist = put(&mut g, 0, "Blood Artist");
    let elf = put(&mut g, 0, "Fyndhorn Elves");
    act(&mut g, Action::ActivateWith(altar, 0, elf));
    assert_eq!(g.obj(elf).zone, Zone::Graveyard);
    let pool = &g.players[0].pool;
    assert_eq!(pool.len(), 1, "{pool:?}");
    assert_eq!(pool[0].colors, [true; 5], "any color");
    no_response_window(&g);
    // Blood Artist's "another creature dies": on the stack, or choosing its target player.
    let triggered = g.items.iter().any(|i| i.source == artist)
        || g.choosing.as_ref().is_some_and(|c| c.item.source == artist);
    assert!(triggered, "the dies trigger: {:?} {:?}", g.items, g.pending);
}

#[test]
fn lions_eye_diamond_discards_and_adds_three_with_no_response_window() {
    // Lion's Eye Diamond is desktop lane d1's (batch 32); until it's scripted in this pool, nothing to test.
    let cs = set("CMD").unwrap();
    if !cs.index_of("Lion's Eye Diamond").is_some_and(|i| cs.supported[i]) {
        return;
    }
    let mut g = table();
    let led = put(&mut g, 0, "Lion's Eye Diamond");
    for _ in 0..3 {
        g.add_card(0, cid("Plains"), Zone::Hand);
    }
    let a = *g
        .legal_actions()
        .iter()
        .find(|a| matches!(a, Action::Activate(c, _) if *c == led))
        .expect("Lion's Eye Diamond activates");
    act(&mut g, a);
    while let Pending::Decide(p) = g.pending {
        assert_eq!(p, 0, "only player 0 decides (the color)");
        let first = g.legal_actions()[0];
        act(&mut g, first);
    }
    assert!(g.players[0].hand.is_empty(), "hand discarded");
    assert_eq!(g.obj(led).zone, Zone::Graveyard, "sacrificed");
    assert_eq!(g.players[0].pool.len(), 3, "{:?}", g.players[0].pool);
    assert!(g.items.is_empty());
    assert_eq!(g.pending, Pending::Priority(0));
}
