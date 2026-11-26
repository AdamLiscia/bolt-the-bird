//! The merger's pieces done by lane B: type-changing land statics (Blood Moon, Urborg), haste for
//! creatures' abilities (Thousand-Year Elixir), and ward—sacrifice nonland permanents (Valgavoth).
#![allow(dead_code, unused_imports)]

mod common;

use bolt_cards::set;
use bolt_core::{Action, Game, Keywords, ObjId, Pending, Step, Target, Zone};
use common::{act, obj, play, resolve_all};

fn cid(name: &str) -> usize {
    set("CMD")
        .unwrap()
        .index_of(name)
        .unwrap_or_else(|| panic!("{name} is not in the CMD pool"))
}

/// A Commander game on the CMD pool (99 Plains each), Bello vs. Ms. Bumbleflower, at player 0's first
/// main phase on turn 3, with player 0's `lands` and 5 of each basic for player 1.
fn game(lands: &[&str]) -> Game {
    let plains = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        set("CMD").unwrap().defs.clone(),
        [&plains, &plains],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    for &l in lands {
        put(&mut g, 0, l);
    }
    for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        for _ in 0..5 {
            put(&mut g, 1, land);
        }
    }
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g
}

/// Every basic five times for player 0.
fn table() -> Game {
    let mut lands = Vec::new();
    for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        lands.extend([land; 5]);
    }
    game(&lands)
}

fn put(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, cid(name), Zone::Battlefield)
}

fn hand(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, cid(name), Zone::Hand)
}

fn tokens(g: &Game, p: usize, name: &str) -> Vec<ObjId> {
    g.permanents(p)
        .filter(|&x| g.is_token(x) && g.def_of(x).name == name)
        .collect()
}

fn colors(g: &Game, id: ObjId) -> Vec<[bool; 5]> {
    g.mana_sources(g.obj(id).controller)
        .into_iter()
        .filter(|&(x, _)| x == id)
        .map(|(_, m)| m.colors)
        .collect()
}

#[test]
fn blood_moon_makes_nonbasic_lands_mountains() {
    let mut g = game(&[]);
    let caves = put(&mut g, 0, "Caves of Koilos");
    let forest = put(&mut g, 0, "Forest");
    let temple = put(&mut g, 0, "Temple Garden");
    assert!(g.has_subtype(temple, "Forest"));
    put(&mut g, 1, "Blood Moon");
    assert_eq!(
        colors(&g, caves),
        [[false, false, false, true, false]],
        "only red"
    );
    assert!(g.has_subtype(temple, "Mountain") && !g.has_subtype(temple, "Forest"));
    assert_eq!(
        colors(&g, forest),
        [[false, false, false, false, true]],
        "basics keep theirs"
    );
}

#[test]
fn urborg_makes_every_land_a_swamp() {
    let mut g = game(&[]);
    let forest = put(&mut g, 0, "Forest");
    let theirs = put(&mut g, 1, "Plains");
    put(&mut g, 0, "Urborg, Tomb of Yawgmoth");
    assert!(g.has_subtype(forest, "Swamp") && g.has_subtype(forest, "Forest"));
    assert!(g.has_subtype(theirs, "Swamp"));
    assert_eq!(colors(&g, forest), [[false, false, true, false, true]]);
    // Blood Moon turns Urborg itself into a Mountain without its ability.
    let urborg = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Urborg, Tomb of Yawgmoth")
        .unwrap();
    put(&mut g, 1, "Blood Moon");
    assert!(!g.has_subtype(forest, "Swamp"));
    assert_eq!(colors(&g, urborg), [[false, false, false, true, false]]);
}

#[test]
fn thousand_year_elixir_lets_new_creatures_tap() {
    let mut g = game(&["Forest"]);
    let elves = put(&mut g, 0, "Llanowar Elves");
    g.objects[elves.0 as usize].since_turn = g.turn; // just arrived
    assert!(colors(&g, elves).is_empty(), "summoning sick");
    put(&mut g, 0, "Thousand-Year Elixir");
    assert_eq!(colors(&g, elves).len(), 1);
}

#[test]
fn ward_sacrifice_nonland_permanents() {
    // Valgavoth's ward on a test definition (Savannah Lions: "Ward—Sacrifice three nonland permanents.").
    let mut defs = (*set("CMD").unwrap().defs).clone();
    let lions = cid("Savannah Lions");
    defs[lions].ward_sacrifice = 3;
    defs[lions].ward_sacrifice_nonland = true;
    let plains = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        std::sync::Arc::new(defs),
        [&plains, &plains],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        for _ in 0..5 {
            put(&mut g, 0, land);
            put(&mut g, 1, land);
        }
    }
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let warded = put(&mut g, 1, "Savannah Lions");
    // Player 0 controls only two nonland permanents: the ward can't be paid, so the spell is countered.
    put(&mut g, 0, "Sol Ring");
    put(&mut g, 0, "Elvish Mystic");
    let bolt = hand(&mut g, 0, "Burst Lightning");
    play(&mut g, bolt, false, &[Target::Object(warded)], &[]);
    assert_eq!(g.obj(warded).zone, Zone::Battlefield);
    assert_eq!(g.obj(bolt).zone, Zone::Graveyard);
    // With three, the lands aren't offered.
    let third = put(&mut g, 0, "Llanowar Elves");
    let bolt = hand(&mut g, 0, "Burst Lightning");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Object(warded)));
    for _ in 0..10 {
        if matches!(g.pending, Pending::Decide(0)) {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(third)));
    assert!(!legal
        .iter()
        .any(|a| matches!(a, Action::Pick(x) if g.types_of(*x).land)));
}

fn to_main(g: &mut Game) {
    for _ in 0..40 {
        if g.step == Step::Main1 && matches!(g.pending, Pending::Priority(_)) {
            return;
        }
        let a = match g.pending {
            Pending::Decide(_) | Pending::ChooseTarget(_) => *g.legal_actions().last().unwrap(),
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn skip_draw_step_damia_refills_at_upkeep() {
    let mut g = table();
    put(&mut g, 0, "Damia, Sage of Stone");
    while g.players[0].hand.len() > 3 {
        let c = g.players[0].hand[0];
        g.objects[c.0 as usize].zone = Zone::Hand; // keep it simple: move cards out below
        let c = g.players[0].hand.remove(0);
        g.players[0].library.insert(0, c);
        g.objects[c.0 as usize].zone = Zone::Library;
    }
    g.start_turn(5, 0);
    to_main(&mut g);
    assert_eq!(g.players[0].hand.len(), 7, "upkeep refill to seven, no draw step");
}

#[test]
fn creatures_untap_on_others_untap_drumbellower() {
    let mut g = table();
    let bell = put(&mut g, 0, "Drumbellower");
    let lions = put(&mut g, 0, "Savannah Lions");
    let ring = put(&mut g, 0, "Sol Ring");
    for x in [bell, lions, ring] {
        g.objects[x.0 as usize].tapped = true;
    }
    g.start_turn(4, 1);
    assert!(!g.obj(lions).tapped && !g.obj(bell).tapped);
    assert!(g.obj(ring).tapped, "creatures only");
}

#[test]
fn extra_turn_skip_untap_savor_the_moment() {
    let mut g = table();
    let lions = put(&mut g, 0, "Savannah Lions");
    g.objects[lions.0 as usize].tapped = true;
    let savor = hand(&mut g, 0, "Savor the Moment");
    play(&mut g, savor, false, &[], &[]);
    assert_eq!(g.extra_turns, vec![0]);
    assert!(g.skip_next_untap[0]);
    g.start_turn(4, 0);
    assert!(g.obj(lions).tapped, "the extra turn's untap step was skipped");
    g.start_turn(6, 0);
    assert!(!g.obj(lions).tapped);
}

#[test]
fn teeg_lock_gaddock_teeg() {
    let mut g = table();
    put(&mut g, 1, "Gaddock Teeg");
    let command = hand(&mut g, 0, "Austere Command"); // 6, noncreature
    let stroke = hand(&mut g, 0, "Blasphemous Act"); // 9 mana value, noncreature
    let elder = hand(&mut g, 0, "Silverback Elder"); // a creature: fine
    let bolt = hand(&mut g, 0, "Burst Lightning");
    let legal = g.legal_actions();
    assert!(!legal
        .iter()
        .any(|a| matches!(a, Action::CastModal(c, _) if *c == command)));
    assert!(!legal.contains(&Action::Cast(stroke)));
    assert!(legal.contains(&Action::Cast(elder)));
    assert!(legal.contains(&Action::Cast(bolt)));
}

#[test]
fn max_hand_plus_and_opponents_minus() {
    let mut g = table();
    put(&mut g, 0, "Trusted Advisor");
    for _ in 0..12 {
        hand(&mut g, 0, "Plains");
    }
    let n = g.players[0].hand.len();
    for _ in 0..40 {
        if g.turn == 4 {
            break;
        }
        let a = match g.pending {
            Pending::Discard(_) | Pending::Decide(_) => *g.legal_actions().first().unwrap(),
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[0].hand.len(), 9.min(n), "discarded down to nine");
    let mut g = table();
    put(&mut g, 1, "Jin-Gitaxias, Core Augur");
    for _ in 0..3 {
        hand(&mut g, 0, "Plains");
    }
    for _ in 0..40 {
        if g.turn == 4 {
            break;
        }
        let a = match g.pending {
            Pending::Discard(_) | Pending::Decide(_) => *g.legal_actions().first().unwrap(),
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[0].hand.len(), 0, "maximum hand size 0");
}

#[test]
fn extra_upkeep_paradox_haze() {
    let mut g = table();
    let haze = hand(&mut g, 0, "Paradox Haze");
    act(&mut g, Action::Cast(haze));
    act(&mut g, Action::Target(Target::Player(0)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(haze).zone, Zone::Battlefield);
    assert_eq!(g.obj(haze).enchanted_player, Some(0));
    put(&mut g, 0, "Mystic Remora");
    g.start_turn(5, 0);
    // Cumulative upkeep asks twice: once in each upkeep step.
    let mut asks = 0;
    for _ in 0..40 {
        if g.step == Step::Main1 {
            break;
        }
        match g.pending {
            Pending::Decide(_) => {
                asks += 1;
                act(&mut g, Action::Yes);
            }
            _ => {
                act(&mut g, Action::PassPriority);
            }
        }
    }
    assert_eq!(asks, 2);
}

#[test]
fn an_aura_on_a_creature_that_leaves_goes_to_the_graveyard() {
    // Paradox Haze's player Auras don't change CR 704.5m for Auras on objects.
    let mut g = table();
    let lions = put(&mut g, 0, "Savannah Lions");
    let blessing = hand(&mut g, 0, "Twinblade Blessing");
    play(&mut g, blessing, false, &[Target::Object(lions)], &[]);
    assert_eq!(g.obj(blessing).zone, Zone::Battlefield);
    let bolt = hand(&mut g, 0, "Burst Lightning");
    play(&mut g, bolt, false, &[Target::Object(lions)], &[]);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
    assert_eq!(g.obj(blessing).zone, Zone::Graveyard);
}
