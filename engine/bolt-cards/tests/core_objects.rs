//! Core theme "objects and zones" (lane C): each piece tested directly, then the cards that waited on it.
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

fn top(g: &mut Game, name: &str) -> ObjId {
    g.add_card(0, cid(name), Zone::Library)
}

fn to_main1(g: &mut Game, turn: u32, active: usize) {
    g.start_turn(turn, active);
    while g.step != Step::Main1 {
        act(g, Action::PassPriority);
    }
}

// ---- Clue tokens and investigate ----

#[test]
fn investigate_makes_a_clue_that_draws_for_two() {
    let mut g = table();
    let fh = put(&mut g, 0, "Floodhound");
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Activate(fh, 0));
    resolve_all(&mut g, &[]);
    let clues = tokens(&g, 0, "Clue");
    assert_eq!(clues.len(), 1);
    assert!(g.types_of(clues[0]).artifact);
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(clues[0], 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n + 1);
    assert!(tokens(&g, 0, "Clue").is_empty(), "sacrificed");
}

#[test]
fn investigating_the_first_time_each_turn_triggers_erdwal_illuminator() {
    let mut g = table();
    put(&mut g, 0, "Erdwal Illuminator");
    let cs = hand(&mut g, 0, "Confront the Unknown");
    let target = put(&mut g, 0, "Hungry Ghoul");
    play(&mut g, cs, false, &[obj(target)], &[]);
    assert_eq!(
        tokens(&g, 0, "Clue").len(),
        2,
        "one more for the first investigate"
    );
    assert!(g.power(target) >= 3, "+1/+1 for each Clue as it resolved");
    let cs2 = hand(&mut g, 0, "Confront the Unknown");
    play(&mut g, cs2, false, &[obj(target)], &[]);
    assert_eq!(tokens(&g, 0, "Clue").len(), 3, "not the first time this turn");
}

#[test]
fn confirm_suspicions_counters_and_investigates_three_times() {
    let mut g = table();
    let think = hand(&mut g, 0, "Think Twice");
    let cs = hand(&mut g, 0, "Confirm Suspicions");
    act(&mut g, Action::Cast(think));
    act(&mut g, Action::Cast(cs));
    resolve_all(&mut g, &[Action::Target(obj(think))]);
    assert_eq!(g.obj(think).zone, Zone::Graveyard);
    assert_eq!(tokens(&g, 0, "Clue").len(), 3);
}

#[test]
fn ethereal_investigator_investigates_and_makes_a_spirit_on_the_second_draw() {
    let mut g = table();
    let ei = hand(&mut g, 0, "Ethereal Investigator");
    play(&mut g, ei, false, &[], &[]);
    assert_eq!(tokens(&g, 0, "Clue").len(), 1);
    let clue = tokens(&g, 0, "Clue")[0];
    act(&mut g, Action::Activate(clue, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(
        tokens(&g, 0, "Spirit").len(),
        1,
        "the Clue's draw was the second this turn"
    );
}

#[test]
fn hold_for_questioning_freezes_and_investigates() {
    let mut g = table();
    let top_ = put(&mut g, 1, "Sensei's Divining Top");
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let hq = hand(&mut g, 0, "Hold for Questioning");
    play(&mut g, hq, false, &[obj(ghoul)], &[]);
    let _ = top_; // its controller's other permanents still work
    assert!(g.obj(ghoul).tapped);
    assert_eq!(tokens(&g, 0, "Clue").len(), 1);
    to_main1(&mut g, 4, 1);
    assert!(g.obj(ghoul).tapped, "doesn't untap");
    assert!(!g
        .legal_actions()
        .iter()
        .any(|a| matches!(a, Action::ActivateWith(x, _, _) if *x == ghoul)));
}

#[test]
fn ongoing_investigation_investigates_for_graveyard_creatures() {
    let mut g = table();
    let oi = put(&mut g, 0, "Ongoing Investigation");
    let dead = g.add_card(0, cid("Hungry Ghoul"), Zone::Graveyard);
    let life = g.players[0].life;
    act(&mut g, Action::ActivateWith(oi, 0, dead));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Clue").len(), 1);
    assert_eq!(g.players[0].life, life + 2);
}

#[test]
fn sacrificing_several_of_a_subtype_as_a_cost() {
    let mut g = table();
    let tj = put(&mut g, 0, "Tamiyo's Journal");
    let fh = hand(&mut g, 0, "Confirm Suspicions");
    let think = hand(&mut g, 0, "Think Twice");
    act(&mut g, Action::Cast(think));
    act(&mut g, Action::Cast(fh));
    resolve_all(&mut g, &[Action::Target(obj(think))]);
    let clues = tokens(&g, 0, "Clue");
    assert_eq!(clues.len(), 3);
    let want = g.add_card(0, cid("Palinchron"), Zone::Library);
    act(&mut g, Action::ActivateWith(tj, 0, clues[0]));
    resolve_all(&mut g, &[Action::Pick(want)]);
    assert!(tokens(&g, 0, "Clue").is_empty(), "all three sacrificed");
    assert_eq!(g.obj(want).zone, Zone::Hand);
}

// ---- Food costs ----

#[test]
fn savvy_hunter_sacrifices_two_foods_to_draw() {
    let mut lands = vec!["Pippin, Warden of Isengard", "Savvy Hunter"];
    lands.extend(["Forest"; 6]);
    let mut g = game(&lands);
    let pip = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Pippin, Warden of Isengard")
        .unwrap();
    let sh = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Savvy Hunter")
        .unwrap();
    act(&mut g, Action::Activate(pip, 0));
    resolve_all(&mut g, &[]);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Activate(pip, 0));
    resolve_all(&mut g, &[]);
    let foods = tokens(&g, 0, "Food");
    assert_eq!(foods.len(), 2);
    assert!(g.legal_actions().contains(&Action::ActivateWith(sh, 0, foods[0])));
    let n = g.players[0].hand.len();
    act(&mut g, Action::ActivateWith(sh, 0, foods[0]));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n + 1);
    assert!(tokens(&g, 0, "Food").is_empty());
}

#[test]
fn pippin_sacrifices_four_foods_for_a_team_pump() {
    let mut lands = vec!["Pippin, Warden of Isengard", "Hungry Ghoul"];
    lands.extend(["Forest"; 4]);
    let mut g = game(&lands);
    let pip = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Pippin, Warden of Isengard")
        .unwrap();
    let ghoul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    let food_def = g.defs.iter().position(|d| d.token && d.name == "Food").unwrap();
    for _ in 0..4 {
        g.add_card(0, food_def, Zone::Battlefield);
    }
    let f0 = tokens(&g, 0, "Food")[0];
    act(&mut g, Action::ActivateWith(pip, 1, f0));
    resolve_all(&mut g, &[]);
    assert!(tokens(&g, 0, "Food").is_empty());
    assert_eq!(g.power(ghoul), 5);
    assert!(g.keywords(ghoul).has(Keywords::HASTE));
}

// ---- Adventures ----

#[test]
fn an_adventure_is_exiled_and_the_creature_cast_later() {
    let mut g = table();
    let ghoul = top(&mut g, "Hungry Ghoul");
    for _ in 0..6 {
        top(&mut g, "Plains");
    }
    let bf = hand(&mut g, 0, "Bramble Familiar // Fetch Quest");
    act(&mut g, Action::CastOmen(bf));
    resolve_all(&mut g, &[Action::Pick(ghoul)]);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    assert_eq!(g.obj(bf).zone, Zone::Exile, "on an adventure");
    assert!(g.legal_actions().contains(&Action::Cast(bf)));
    play(&mut g, bf, false, &[], &[]);
    assert_eq!(g.obj(bf).zone, Zone::Battlefield);
}

#[test]
fn monster_manual_studies_then_puts_creatures_in() {
    let mut g = table();
    let pal = top(&mut g, "Palinchron");
    for _ in 0..4 {
        top(&mut g, "Plains");
    }
    let mm = hand(&mut g, 0, "Monster Manual // Zoological Study");
    act(&mut g, Action::CastOmen(mm));
    resolve_all(&mut g, &[Action::Pick(pal)]);
    assert_eq!(g.obj(pal).zone, Zone::Hand);
    assert_eq!(g.obj(mm).zone, Zone::Exile);
    play(&mut g, mm, false, &[], &[]);
    to_main1(&mut g, 5, 0);
    act(&mut g, Action::Activate(mm, 0));
    resolve_all(&mut g, &[Action::Pick(pal)]);
    assert_eq!(g.obj(pal).zone, Zone::Battlefield);
}

// ---- Reanimating Auras ----

#[test]
fn animate_dead_returns_a_creature_and_takes_it_back_when_it_leaves() {
    let mut g = table();
    let theirs = g.add_card(1, cid("Palinchron"), Zone::Graveyard);
    let ad = hand(&mut g, 0, "Animate Dead");
    play(&mut g, ad, false, &[obj(theirs)], &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Battlefield);
    assert_eq!(g.obj(theirs).controller, 0);
    assert_eq!(
        g.obj(ad).zone,
        Zone::Battlefield,
        "attached, not a state-based death"
    );
    assert_eq!(g.power(theirs), 3, "-1/-0");
    let br = hand(&mut g, 0, "Broken Bond");
    play(&mut g, br, false, &[obj(ad)], &[Action::No]); // no land from hand
    assert_eq!(g.obj(ad).zone, Zone::Graveyard);
    assert_eq!(
        g.obj(theirs).zone,
        Zone::Graveyard,
        "sacrificed when the Aura left"
    );
}

#[test]
fn dance_of_the_dead_returns_it_tapped_and_can_untap_it() {
    let mut g = table();
    let mine = g.add_card(0, cid("Hungry Ghoul"), Zone::Graveyard);
    let dd = hand(&mut g, 0, "Dance of the Dead");
    play(&mut g, dd, false, &[obj(mine)], &[]);
    assert!(g.obj(mine).tapped);
    assert_eq!(g.power(mine), 3);
    g.start_turn(5, 0);
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    resolve_all(&mut g, &[Action::Yes]);
    assert!(!g.obj(mine).tapped, "paid {{1}}{{B}}");
}

#[test]
fn necromancy_at_instant_speed_is_sacrificed_at_end_of_turn() {
    let mut g = table();
    let ghoul = g.add_card(1, cid("Hungry Ghoul"), Zone::Graveyard);
    let nec = hand(&mut g, 0, "Necromancy");
    to_main1(&mut g, 4, 1);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(nec));
    resolve_all(&mut g, &[Action::Target(obj(ghoul))]);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    assert_eq!(g.obj(ghoul).controller, 0);
    assert!(
        g.obj(nec).sacrifice_at_end,
        "cast when a sorcery couldn't have been"
    );
}

// ---- Phasing ----

#[test]
fn teferis_protection_phases_everything_out_and_locks_life_until_your_next_turn() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let tp = hand(&mut g, 0, "Teferi's Protection");
    play(&mut g, tp, false, &[], &[]);
    assert_eq!(g.obj(tp).zone, Zone::Exile);
    assert!(g.permanents(0).next().is_none(), "all phased out");
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield, "no zone change");
    to_main1(&mut g, 4, 1);
    for _ in 0..3 {
        put(&mut g, 1, "Hungry Ghoul");
    }
    let life = g.players[0].life;
    let bio = hand(&mut g, 1, "Biorhythm"); // each life total becomes its creature count
    play(&mut g, bio, false, &[], &[]);
    assert_eq!(g.players[1].life, 3);
    assert_eq!(g.players[0].life, life, "life can't change");
    g.start_turn(5, 0);
    assert!(g.permanents(0).any(|x| x == ghoul), "phased in on your untap");
}

#[test]
fn clever_concealment_phases_out_chosen_permanents() {
    let mut g = table();
    let a = put(&mut g, 0, "Hungry Ghoul");
    let b = put(&mut g, 0, "Aegis Turtle");
    let cc = hand(&mut g, 0, "Clever Concealment");
    act(&mut g, Action::Cast(cc));
    act(&mut g, Action::Target(obj(a)));
    act(&mut g, Action::No); // no more targets
    resolve_all(&mut g, &[]);
    assert!(!g.permanents(0).any(|x| x == a), "phased out");
    assert!(g.permanents(0).any(|x| x == b));
    g.start_turn(5, 0);
    assert!(g.permanents(0).any(|x| x == a));
}

// ---- More Clues: Tireless Tracker, Academy Manufactor, Wavesifter ----

#[test]
fn tireless_tracker_investigates_on_landfall_and_grows_on_clue_sacrifice() {
    let mut g = table();
    let tt = put(&mut g, 0, "Tireless Tracker");
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[]);
    let clues = tokens(&g, 0, "Clue");
    assert_eq!(clues.len(), 1);
    act(&mut g, Action::Activate(clues[0], 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(tt), 4, "a +1/+1 counter for the sacrificed Clue");
}

#[test]
fn academy_manufactor_makes_one_of_each_instead() {
    let mut g = table();
    put(&mut g, 0, "Academy Manufactor");
    let fh = put(&mut g, 0, "Floodhound");
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Activate(fh, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Clue").len(), 1);
    assert_eq!(tokens(&g, 0, "Food").len(), 1);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1);
}

#[test]
fn wavesifter_evoked_still_investigates_twice() {
    let mut g = table();
    let w = hand(&mut g, 0, "Wavesifter");
    act(&mut g, Action::CastAlternative(w));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Clue").len(), 2);
    assert_eq!(g.obj(w).zone, Zone::Graveyard, "evoked: sacrificed");
}

#[test]
fn a_destroyed_clue_is_not_sacrificed_for_tireless_tracker() {
    let mut g = table();
    let tt = put(&mut g, 0, "Tireless Tracker");
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Clue").len(), 1);
    let bane = hand(&mut g, 0, "Bane of Progress"); // destroys all artifacts and enchantments
    play(&mut g, bane, false, &[], &[]);
    assert!(tokens(&g, 0, "Clue").is_empty(), "destroyed");
    assert_eq!(g.power(tt), 3, "destroyed, not sacrificed: no counter");
}

// ---- Damia's last three: All Suns' Dawn, Chain of Smog, Kira ----

#[test]
fn all_suns_dawn_returns_one_card_of_each_color() {
    let mut g = table();
    let green = g.add_card(0, cid("Avenger of Zendikar"), Zone::Graveyard);
    let green2 = g.add_card(0, cid("Fauna Shaman"), Zone::Graveyard);
    let black = g.add_card(0, cid("Hungry Ghoul"), Zone::Graveyard);
    let asd = hand(&mut g, 0, "All Suns' Dawn");
    play(
        &mut g,
        asd,
        false,
        &[],
        &[Action::Pick(black), Action::Pick(green)],
    );
    assert_eq!(g.obj(black).zone, Zone::Hand);
    assert_eq!(g.obj(green).zone, Zone::Hand);
    assert_eq!(g.obj(green2).zone, Zone::Graveyard, "up to one of each color");
    assert_eq!(g.obj(asd).zone, Zone::Exile);
}

#[test]
fn chain_of_smog_can_be_copied_back_by_its_target() {
    let mut g = table();
    let cs = hand(&mut g, 0, "Chain of Smog");
    let n0 = g.players[0].hand.len() - 1;
    let n1 = g.players[1].hand.len();
    act(&mut g, Action::Cast(cs));
    act(&mut g, Action::Target(Target::Player(1)));
    let mut copied = false;
    for _ in 0..60 {
        if g.items.is_empty() && !matches!(g.pending, Pending::Decide(_) | Pending::ChooseTarget(_)) {
            break;
        }
        let a = match &g.decision {
            Some(bolt_core::Decision::ChainCopy { .. }) if !copied => {
                copied = true;
                Action::Mode(0) // copy it back at the caster
            }
            Some(bolt_core::Decision::ChainCopy { .. }) => Action::No,
            Some(_) if matches!(g.pending, Pending::Decide(_)) => g.legal_actions()[0],
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[1].hand.len(), n1 - 2);
    assert_eq!(
        g.players[0].hand.len(),
        n0 - 2,
        "the copy made the caster discard two"
    );
}

#[test]
fn kira_counters_the_first_spell_targeting_each_creature_each_turn() {
    let mut g = table();
    put(&mut g, 1, "Kira, Great Glass-Spinner");
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let k1 = hand(&mut g, 0, "Khalni Ambush // Khalni Territory");
    let mine = put(&mut g, 0, "Palinchron");
    act(&mut g, Action::Cast(k1));
    act(&mut g, Action::Target(obj(mine)));
    act(&mut g, Action::Target(obj(ghoul)));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.obj(ghoul).zone,
        Zone::Battlefield,
        "the first targeting is countered"
    );
    assert_eq!(g.obj(k1).zone, Zone::Graveyard);
    let k2 = hand(&mut g, 0, "Khalni Ambush // Khalni Territory");
    act(&mut g, Action::Cast(k2));
    act(&mut g, Action::Target(obj(mine)));
    act(&mut g, Action::Target(obj(ghoul)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ghoul).zone, Zone::Graveyard, "not the first time this turn");
}

// ---- Populate (Trostani, Selesnya's Voice) ----

#[test]
fn populate_copies_a_creature_token_and_trostani_gains_toughness() {
    let mut lands = vec!["Trostani, Selesnya's Voice"];
    lands.extend(["Forest", "Plains", "Plains"]);
    let mut g = game(&lands);
    let tr = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Trostani, Selesnya's Voice")
        .unwrap();
    let beast = g
        .defs
        .iter()
        .position(|d| d.token && d.name == "Phyrexian Beast")
        .unwrap();
    g.add_card(0, beast, Zone::Battlefield);
    let life = g.players[0].life;
    act(&mut g, Action::Activate(tr, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Phyrexian Beast").len(), 2, "populated");
    assert_eq!(g.players[0].life, life + 4, "the copy's toughness");
}

// ---- Imprint and copy-cast (Isochron Scepter) ----

#[test]
fn isochron_scepter_imprints_an_instant_and_casts_copies_free() {
    let mut g = table();
    let think = hand(&mut g, 0, "Think Twice"); // mana value 2
    let sc = hand(&mut g, 0, "Isochron Scepter");
    play(&mut g, sc, false, &[], &[Action::Pick(think)]);
    assert_eq!(g.obj(think).zone, Zone::Exile);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(sc, 0));
    let mut cast = false;
    for _ in 0..20 {
        let free = g
            .legal_actions()
            .into_iter()
            .find(|a| matches!(a, Action::Cast(_)));
        let a = match free {
            Some(c) if !cast => {
                cast = true;
                c // the free copy
            }
            _ if g.items.is_empty() && cast => break,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(
        g.players[0].hand.len(),
        n + 1,
        "the copy of Think Twice drew a card"
    );
    assert_eq!(g.obj(think).zone, Zone::Exile, "the imprinted card stays");
}

// ---- Sacrifice X Clues (Lonis, Cryptozoologist) ----

fn clue_def(g: &Game) -> usize {
    g.defs.iter().position(|d| d.token && d.name == "Clue").unwrap()
}

/// Passes priority until a decision is asked (at most 20 times).
fn until_decide(g: &mut Game) {
    for _ in 0..20 {
        if matches!(g.pending, Pending::Decide(_)) {
            return;
        }
        act(g, Action::PassPriority);
    }
    panic!("no decision: {:?}", g.legal_actions());
}

#[test]
fn sacrifice_x_clues_reports_each_sacrifice_and_puts_a_revealed_permanent_under_your_control() {
    let mut g = table();
    let lonis = put(&mut g, 0, "Lonis, Cryptozoologist");
    let tt = put(&mut g, 0, "Tireless Tracker");
    let clue = clue_def(&g);
    for _ in 0..3 {
        g.add_card(0, clue, Zone::Battlefield);
    }
    to_main1(&mut g, 5, 0);
    let forest = g.add_card(1, cid("Forest"), Zone::Library);
    let sol = g.add_card(1, cid("Sol Ring"), Zone::Library); // the top card
    let legal = g.legal_actions();
    let x = |x| Action::ActivateX {
        id: lonis,
        n: 0,
        victim: None,
        x,
    };
    assert!(legal.contains(&x(1)) && legal.contains(&x(3)));
    assert!(!legal.contains(&x(4)), "only three Clues");
    act(&mut g, x(2));
    act(&mut g, Action::Target(Target::Player(1)));
    assert_eq!(tokens(&g, 0, "Clue").len(), 1, "two sacrificed as the cost");
    until_decide(&mut g);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(sol)));
    assert!(!legal.contains(&Action::Pick(forest)), "a land can't be put");
    act(&mut g, Action::Pick(sol));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(sol).zone, Zone::Battlefield);
    assert_eq!(g.obj(sol).controller, 0, "under your control");
    assert_eq!(g.players[1].library[0], forest, "the rest on the bottom");
    assert_eq!(g.power(tt), 5, "Tireless Tracker saw both sacrifices");
}

#[test]
fn lonis_investigates_for_another_nontoken_creature() {
    let mut g = table();
    put(&mut g, 0, "Lonis, Cryptozoologist");
    let elf = hand(&mut g, 0, "Llanowar Elves");
    play(&mut g, elf, false, &[], &[]);
    assert_eq!(tokens(&g, 0, "Clue").len(), 1);
}

// ---- Copies: copy exceptions (Phantasmal Image, Mockingbird, Glasspool Mimic) ----

#[test]
fn phantasmal_image_is_an_illusion_and_is_sacrificed_when_targeted() {
    let mut g = table();
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let img = hand(&mut g, 0, "Phantasmal Image");
    play(&mut g, img, false, &[], &[Action::Pick(ghoul)]);
    assert_eq!(g.def_of(img).name, "Hungry Ghoul");
    assert!(g.has_subtype(img, "Illusion") && g.has_subtype(img, "Zombie"));
    let growth = hand(&mut g, 0, "Giant Growth");
    play(&mut g, growth, false, &[obj(img)], &[]);
    assert_eq!(g.obj(img).zone, Zone::Graveyard, "sacrificed as it was targeted");
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
}

#[test]
fn mockingbird_copies_only_up_to_the_mana_spent_and_is_a_flying_bird() {
    let mut g = table();
    let ghoul = put(&mut g, 1, "Hungry Ghoul"); // mana value 2
    let mb = hand(&mut g, 0, "Mockingbird");
    act(&mut g, Action::CastX(mb, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.def_of(mb).name,
        "Mockingbird",
        "one mana spent: nothing to copy"
    );
    let mb2 = hand(&mut g, 0, "Mockingbird");
    act(&mut g, Action::CastX(mb2, 1));
    resolve_all(&mut g, &[Action::Pick(ghoul)]);
    assert_eq!(g.def_of(mb2).name, "Hungry Ghoul");
    assert!(g.has_subtype(mb2, "Bird") && g.has_subtype(mb2, "Zombie"));
    assert!(g.keywords(mb2).has(Keywords::FLYING));
}

#[test]
fn glasspool_mimic_copies_your_creature_with_extra_types_or_is_played_as_its_land() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let palin = put(&mut g, 1, "Palinchron");
    let gm = hand(&mut g, 0, "Glasspool Mimic // Glasspool Shore");
    act(&mut g, Action::Cast(gm));
    until_decide(&mut g);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(ghoul)));
    assert!(
        !legal.contains(&Action::Pick(palin)),
        "only a creature you control"
    );
    act(&mut g, Action::Pick(ghoul));
    resolve_all(&mut g, &[]);
    assert_eq!(g.def_of(gm).name, "Hungry Ghoul");
    for t in ["Shapeshifter", "Rogue", "Zombie"] {
        assert!(g.has_subtype(gm, t), "{t}");
    }
    let mut g = table();
    let gs = hand(&mut g, 0, "Glasspool Mimic // Glasspool Shore");
    act(&mut g, Action::PlayLand(gs));
    resolve_all(&mut g, &[]);
    assert_eq!(g.def_of(gs).name, "Glasspool Shore");
    assert!(g.types_of(gs).land && g.obj(gs).tapped);
}

// ---- A token copy that's also an artifact (Vaultborn Tyrant) ----

#[test]
fn vaultborn_tyrant_returns_as_an_artifact_token_copy_once() {
    let mut g = table();
    let vt = put(&mut g, 0, "Vaultborn Tyrant");
    let kill = hand(&mut g, 0, "Hero's Downfall");
    let (life, n) = (g.players[0].life, g.players[0].hand.len());
    play(&mut g, kill, false, &[obj(vt)], &[]);
    let copies = tokens(&g, 0, "Vaultborn Tyrant");
    assert_eq!(copies.len(), 1);
    let t = copies[0];
    assert!(g.types_of(t).artifact && g.types_of(t).creature);
    assert_eq!(g.players[0].life, life + 3, "the copy entering: gain 3");
    assert_eq!(g.players[0].hand.len(), n, "Downfall cast, one card drawn");
    let kill2 = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, kill2, false, &[obj(t)], &[]);
    assert!(
        tokens(&g, 0, "Vaultborn Tyrant").is_empty(),
        "a token makes no copy"
    );
}

// ---- Morph: turned face up (Rattleclaw Mystic, Aphetto Alchemist) ----

#[test]
fn rattleclaw_mystic_cast_face_down_adds_three_mana_when_turned_face_up() {
    let mut g = table();
    let rm = hand(&mut g, 0, "Rattleclaw Mystic");
    act(&mut g, Action::CastAlternative(rm)); // face down for {3}
    resolve_all(&mut g, &[]);
    assert!(g.obj(rm).face_down);
    assert_eq!((g.power(rm), g.toughness(rm)), (2, 2));
    act(&mut g, Action::Activate(rm, 0)); // turn it face up for {2}
    assert_eq!(g.def_of(rm).name, "Rattleclaw Mystic");
    let pool = g.players[0].pool.len();
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].pool.len(), pool + 3, "{{G}}{{U}}{{R}}");
}

#[test]
fn aphetto_alchemist_untaps_a_creature_and_has_morph() {
    let mut g = table();
    let al = put(&mut g, 0, "Aphetto Alchemist");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    to_main1(&mut g, 5, 0);
    g.objects[ghoul.0 as usize].tapped = true;
    act(&mut g, Action::Activate(al, 0));
    act(&mut g, Action::Target(obj(ghoul)));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(ghoul).tapped);
    let al2 = hand(&mut g, 0, "Aphetto Alchemist");
    assert!(g.legal_actions().contains(&Action::CastAlternative(al2)), "morph");
}

// ---- Back faces: Sundering Eruption // Volcanic Fissure ----

#[test]
fn sundering_eruption_destroys_a_land_fetches_tapped_and_stops_ground_blockers() {
    let mut g = table();
    let land = g.permanents(1).find(|&x| g.def_of(x).name == "Forest").unwrap();
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let angel = put(&mut g, 1, "Serra Angel");
    let se = hand(&mut g, 0, "Sundering Eruption // Volcanic Fissure");
    act(&mut g, Action::Cast(se));
    act(&mut g, Action::Target(obj(land)));
    until_decide(&mut g);
    let pick = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a, Action::Pick(_)))
        .expect("a basic to find");
    let Action::Pick(basic) = pick else { unreachable!() };
    act(&mut g, pick);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(land).zone, Zone::Graveyard);
    assert_eq!(g.obj(basic).zone, Zone::Battlefield);
    assert!(g.obj(basic).tapped, "onto the battlefield tapped");
    assert!(g.keywords(ghoul).has(Keywords::CANT_BLOCK));
    assert!(!g.keywords(angel).has(Keywords::CANT_BLOCK), "it has flying");
}

#[test]
fn volcanic_fissure_asks_three_life_or_enters_tapped() {
    let mut g = table();
    let vf = hand(&mut g, 0, "Sundering Eruption // Volcanic Fissure");
    let life = g.players[0].life;
    act(&mut g, Action::PlayLand(vf));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.def_of(vf).name, "Volcanic Fissure");
    assert_eq!(g.players[0].life, life - 3);
    assert!(!g.obj(vf).tapped);
}

// ---- Puppeteer Clique: an opponent's creature for a turn, and persist ----

#[test]
fn puppeteer_clique_borrows_a_creature_until_the_end_step_and_persists() {
    let mut g = table();
    let ghoul = g.add_card(1, cid("Hungry Ghoul"), Zone::Graveyard);
    let pc = hand(&mut g, 0, "Puppeteer Clique");
    play(&mut g, pc, false, &[], &[Action::Target(obj(ghoul))]);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    assert_eq!(g.obj(ghoul).controller, 0);
    assert!(g.keywords(ghoul).has(Keywords::HASTE));
    let kill = hand(&mut g, 0, "Hero's Downfall");
    play(&mut g, kill, false, &[obj(pc)], &[]);
    assert_eq!(g.obj(pc).zone, Zone::Battlefield, "persist");
    assert_eq!(g.power(pc), 2, "with a -1/-1 counter");
    let turn = g.turn;
    for _ in 0..40 {
        if g.turn != turn {
            break;
        }
        let legal = g.legal_actions();
        let a = if legal.contains(&Action::PassPriority) {
            Action::PassPriority
        } else {
            legal[0] // a cleanup discard
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(ghoul).zone, Zone::Exile, "exiled at the end step");
}

// ---- Plot (Railway Brawler) ----

#[test]
fn railway_brawler_is_plotted_then_cast_free_on_a_later_turn() {
    let mut g = table();
    let rb = hand(&mut g, 0, "Railway Brawler");
    assert!(g.legal_actions().contains(&Action::Activate(rb, 0)));
    act(&mut g, Action::Activate(rb, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(rb).zone, Zone::Exile);
    assert!(
        !g.legal_actions().contains(&Action::Cast(rb)),
        "not the turn it was plotted"
    );
    to_main1(&mut g, 5, 0);
    assert!(g.legal_actions().contains(&Action::Cast(rb)));
    let untapped = g.permanents(0).filter(|&x| !g.obj(x).tapped).count();
    play(&mut g, rb, false, &[], &[]);
    assert_eq!(g.obj(rb).zone, Zone::Battlefield);
    assert_eq!(
        g.permanents(0).filter(|&x| !g.obj(x).tapped).count(),
        untapped + 1,
        "free: no land tapped (and the Brawler itself)"
    );
    let elf = hand(&mut g, 0, "Llanowar Elves");
    play(&mut g, elf, false, &[], &[]);
    assert_eq!(g.power(elf), 2, "X +1/+1 counters, X its power");
}

// ---- Auras put onto the battlefield attached, not cast (game/zones.rs) ----

#[test]
fn an_aura_put_onto_the_battlefield_enters_attached_only_to_what_it_could_enchant() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let land = g.permanents(0).find(|&x| g.types_of(x).land).unwrap();
    let before = g.power(ghoul);
    let ws = hand(&mut g, 0, "Wingspan Stride");
    let land_zcc = g.obj(land).zcc;
    assert!(
        !g.put_aura_attached(ws, land, land_zcc, 0),
        "Enchant creature: not a land"
    );
    assert_eq!(g.obj(ws).zone, Zone::Hand, "it stays where it was (CR 303.4g)");
    let zcc = g.obj(ghoul).zcc;
    assert!(
        !g.put_aura_attached(ws, ghoul, zcc + 1, 0),
        "not once the creature has left"
    );
    assert!(g.put_aura_attached(ws, ghoul, zcc, 0));
    assert_eq!(g.obj(ws).zone, Zone::Battlefield);
    assert_eq!(g.obj(ws).attached_to, Some((ghoul, zcc)));
    assert_eq!(g.power(ghoul), before + 1);
    assert!(g.keywords(ghoul).has(Keywords::FLYING));
    let dead = g.add_card(0, cid("Blanchwood Armor"), Zone::Graveyard);
    assert!(g.put_aura_attached(dead, ghoul, zcc, 0), "from a graveyard too");
    assert_eq!(g.obj(dead).attached_to, Some((ghoul, zcc)));
}

// ---- Copies (Lithoform Engine, Thespian's Stage, The Mycosynth Gardens) ----

#[test]
fn a_permanent_becomes_a_copy_for_good_without_its_own_abilities() {
    // The Mycosynth Gardens (not listed yet: its {1} filter mana waits on d1). Mind Stone has
    // mana value 2, so X = 2.
    let mut g = table();
    let gardens = put(&mut g, 0, "The Mycosynth Gardens");
    let stone = put(&mut g, 0, "Mind Stone");
    act(
        &mut g,
        Action::ActivateX {
            id: gardens,
            n: 0,
            victim: None,
            x: 2,
        },
    );
    act(&mut g, Action::Target(obj(stone)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.name_of(gardens), "Mind Stone");
    assert!(g.types_of(gardens).artifact && !g.types_of(gardens).land);
    assert_eq!(g.def_of(gardens).activated.len(), 1, "Mind Stone's ability only");
    to_main1(&mut g, 5, 0);
    assert_eq!(g.name_of(gardens), "Mind Stone", "not just until end of turn");
}

#[test]
fn a_copied_ability_that_targets_chooses_new_targets() {
    // Lithoform Engine copying a targeted activated ability: Rogue's Passage's.
    let mut g = table();
    let engine = put(&mut g, 0, "Lithoform Engine");
    let passage = put(&mut g, 0, "Rogue's Passage");
    let a = put(&mut g, 0, "Hungry Ghoul");
    let b = put(&mut g, 0, "Hungry Ghoul");
    act(&mut g, Action::Activate(passage, 0));
    act(&mut g, Action::Target(obj(a)));
    act(&mut g, Action::Activate(engine, 0));
    act(&mut g, Action::Target(obj(passage)));
    resolve_all(&mut g, &[Action::Target(obj(b))]);
    assert!(g.keywords(a).has(Keywords::UNBLOCKABLE));
    assert!(g.keywords(b).has(Keywords::UNBLOCKABLE), "the copy's new target");
}

// ---- Borrowed d2 pieces: "that spell" on an opponent's instant or sorcery, playing from exile ----

#[test]
fn an_opponents_instant_or_sorcery_trigger_carries_that_spell() {
    let mut g = table();
    let arasta = put(&mut g, 0, "Arasta of the Endless Web");
    to_main1(&mut g, 4, 1);
    let bolt = hand(&mut g, 1, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(0)));
    assert!(
        g.items
            .iter()
            .any(|i| i.source == arasta && i.extra == Some(bolt)),
        "the trigger knows the spell"
    );
}

#[test]
fn playing_a_card_from_exile_triggers_and_from_hand_does_not() {
    // Prosper, Tome-Bound: its end-step exile, then Pact Boon.
    let mut g = table();
    let prosper = put(&mut g, 0, "Prosper, Tome-Bound");
    let from_hand = hand(&mut g, 0, "Mountain");
    act(&mut g, Action::PlayLand(from_hand));
    resolve_all(&mut g, &[]);
    assert!(tokens(&g, 0, "Treasure").is_empty(), "not from exile");
    let exiled = top(&mut g, "Mountain");
    for _ in 0..200 {
        if g.obj(exiled).zone == Zone::Exile {
            break;
        }
        let legal = g.legal_actions();
        let a = [Action::PassPriority, Action::FinishAttacks, Action::FinishBlocks]
            .into_iter()
            .find(|a| legal.contains(a))
            .expect("a pass");
        act(&mut g, a);
    }
    assert_eq!(g.obj(exiled).zone, Zone::Exile, "Mystic Arcanum at the end step");
    assert!(g.obj(prosper).zone == Zone::Battlefield);
    to_main1(&mut g, 5, 0);
    assert!(
        g.legal_actions().contains(&Action::PlayLand(exiled)),
        "playable until the end of your next turn"
    );
    act(&mut g, Action::PlayLand(exiled));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1, "played from exile");
}

// ---- needs_core (mac-c), batch 12: Sheoldred and Animar ----

#[test]
fn hornet_queen_makes_deathtouch_insects_and_blood_money_pays_only_for_nontokens() {
    let mut g = table();
    let queen = hand(&mut g, 0, "Hornet Queen");
    act(&mut g, Action::Cast(queen));
    resolve_all(&mut g, &[]);
    let insects = tokens(&g, 0, "Insect");
    assert_eq!(insects.len(), 4);
    assert!(g.keywords(insects[0]).has(Keywords::FLYING) && g.keywords(insects[0]).has(Keywords::DEATHTOUCH));
    assert_eq!(
        g.colors_of(insects[0]),
        [false, false, false, false, true],
        "green"
    );
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let money = hand(&mut g, 0, "Blood Money");
    act(&mut g, Action::Cast(money));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(queen).zone, Zone::Graveyard);
    assert_eq!(g.obj(ghoul).zone, Zone::Graveyard);
    assert!(tokens(&g, 0, "Insect").is_empty());
    let treasures = tokens(&g, 0, "Treasure");
    assert_eq!(treasures.len(), 2, "Hornet Queen and the Ghoul, not the Insects");
    assert!(treasures.iter().all(|&t| g.obj(t).tapped));
}

#[test]
fn peer_into_the_abyss_draws_half_the_library_and_halves_life_rounding_up() {
    let mut g = table();
    let lib = g.players[1].library.len();
    let hand_before = g.players[1].hand.len();
    g.players[1].life = 39;
    let peer = hand(&mut g, 0, "Peer into the Abyss");
    act(&mut g, Action::Cast(peer));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].hand.len(), hand_before + lib.div_ceil(2));
    assert_eq!(g.players[1].life, 19, "loses 20 of 39");
}

#[test]
fn otherworld_atlas_charges_then_each_player_draws_per_counter() {
    let mut g = table();
    let atlas = put(&mut g, 0, "Otherworld Atlas");
    act(&mut g, Action::Activate(atlas, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(atlas).kind_counters, 1);
    to_main1(&mut g, 5, 0);
    let (mine, theirs) = (g.players[0].hand.len(), g.players[1].hand.len());
    act(&mut g, Action::Activate(atlas, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), mine + 1);
    assert_eq!(g.players[1].hand.len(), theirs + 1);
}

#[test]
fn neoform_finds_one_more_mana_value_with_an_extra_counter() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let sage = top(&mut g, "Somberwald Sage");
    let neoform = hand(&mut g, 0, "Neoform");
    act(&mut g, Action::CastWith(neoform, ghoul));
    resolve_all(&mut g, &[Action::Pick(sage)]);
    assert_eq!(g.obj(ghoul).zone, Zone::Graveyard, "sacrificed");
    assert_eq!(g.obj(sage).zone, Zone::Battlefield, "mana value 3");
    assert_eq!(g.obj(sage).counters, 1);
}

#[test]
fn gilded_drake_exchanges_control_with_an_opponents_creature() {
    let mut g = table();
    let theirs = put(&mut g, 1, "Hungry Ghoul");
    let drake = hand(&mut g, 0, "Gilded Drake");
    act(&mut g, Action::Cast(drake));
    resolve_all(&mut g, &[Action::Target(obj(theirs))]);
    assert_eq!(g.obj(drake).controller, 1);
    assert_eq!(g.obj(theirs).controller, 0);
}

// ---- Batch 13: counters on the attached creature, cast from graveyard or exile, copy the enchanted artifact ----

#[test]
fn wurmcoil_engine_dies_into_a_deathtouch_wurm_and_a_lifelink_wurm() {
    let mut g = table();
    let engine = put(&mut g, 0, "Wurmcoil Engine");
    let dam = hand(&mut g, 0, "Damnation");
    play(&mut g, dam, false, &[], &[]);
    assert_eq!(g.obj(engine).zone, Zone::Graveyard);
    let wurms = tokens(&g, 0, "Phyrexian Wurm");
    assert_eq!(wurms.len(), 2);
    assert!(wurms
        .iter()
        .all(|&w| g.types_of(w).artifact && g.power(w) == 3 && g.toughness(w) == 3));
    assert!(wurms.iter().all(|&w| g.colors_of(w) == [false; 5]), "colorless");
    let dt = wurms
        .iter()
        .filter(|&&w| g.keywords(w).has(Keywords::DEATHTOUCH))
        .count();
    let ll = wurms
        .iter()
        .filter(|&&w| g.keywords(w).has(Keywords::LIFELINK))
        .count();
    assert_eq!((dt, ll), (1, 1), "one with deathtouch, the other with lifelink");
}

#[test]
fn counters_on_attached_bone_sabres_puts_four_counters_on_the_attacker() {
    let mut lands = vec!["Hungry Ghoul"];
    for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        lands.extend([land; 5]);
    }
    let mut g = game(&lands);
    let ghoul = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Hungry Ghoul")
        .unwrap();
    let sabres = put(&mut g, 0, "Bone Sabres");
    act(&mut g, Action::Activate(sabres, 0));
    resolve_all(&mut g, &[Action::Target(obj(ghoul))]);
    assert_eq!(g.obj(sabres).attached_to.map(|(c, _)| c), Some(ghoul));
    let base = g.power(ghoul);
    while g.step != Step::DeclareAttackers {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(ghoul));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ghoul).counters, 4);
    assert_eq!(g.power(ghoul), base + 4);
}

#[test]
fn cast_from_graveyard_or_exile_squee_comes_back_from_either_zone() {
    let mut g = table();
    let dead = g.add_card(0, cid("Squee, the Immortal"), Zone::Graveyard);
    let gone = g.add_card(0, cid("Squee, the Immortal"), Zone::Exile);
    let theirs = g.add_card(1, cid("Squee, the Immortal"), Zone::Exile);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Cast(dead)), "from your graveyard");
    assert!(legal.contains(&Action::Cast(gone)), "from exile");
    assert!(
        !legal.contains(&Action::Cast(theirs)),
        "only its owner's permission"
    );
    play(&mut g, dead, false, &[], &[]);
    assert_eq!(g.obj(dead).zone, Zone::Battlefield);
    play(&mut g, gone, false, &[], &[]);
    assert_eq!(g.obj(gone).zone, Zone::Battlefield);
}

/// Passes from the start of player 0's turn `turn` until main phase 1 or the game ends.
fn upkeep_through(g: &mut Game, turn: u32) {
    g.start_turn(turn, 0);
    while g.step != Step::Main1 && g.result.is_none() {
        act(g, Action::PassPriority);
    }
}

#[test]
fn copy_attached_mechanized_production_copies_each_upkeep_and_wins_at_eight() {
    let mut g = table();
    let ring = put(&mut g, 0, "Sol Ring");
    let mp = hand(&mut g, 0, "Mechanized Production");
    play(&mut g, mp, false, &[obj(ring)], &[]);
    assert_eq!(g.obj(mp).attached_to.map(|(c, _)| c), Some(ring));
    upkeep_through(&mut g, 5);
    assert_eq!(
        tokens(&g, 0, "Sol Ring").len(),
        1,
        "a token copy of the enchanted artifact"
    );
    assert!(g.result.is_none(), "two Sol Rings");
    for _ in 0..5 {
        put(&mut g, 0, "Sol Ring");
    }
    upkeep_through(&mut g, 7);
    assert_eq!(g.result, Some(bolt_core::Outcome::Winner(0)), "eight Sol Rings");
}

#[test]
fn final_act_does_each_chosen_mode_and_only_those() {
    let mut g = table();
    let w = hand(&mut g, 0, "Wrenn and Seven");
    play(&mut g, w, false, &[], &[]);
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let first = hand(&mut g, 0, "Final Act");
    // Mode 0b00010: planeswalkers only.
    act(&mut g, Action::CastModal(first, 0b00010 - 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(w).zone, Zone::Graveyard);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield, "creatures weren't chosen");
    g.players[1].poison = 3;
    g.players[1].energy = 2;
    g.players[0].poison = 1;
    let second = hand(&mut g, 0, "Final Act");
    // All five: creatures, planeswalkers, battles, graveyards, counters.
    act(&mut g, Action::CastModal(second, 0b11111 - 1));
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.obj(ghoul).zone,
        Zone::Exile,
        "destroyed, then its graveyard exiled"
    );
    assert_eq!((g.obj(w).zone, g.obj(first).zone), (Zone::Exile, Zone::Exile));
    assert_eq!(
        g.obj(second).zone,
        Zone::Graveyard,
        "on the stack as graveyards are exiled"
    );
    assert_eq!((g.players[1].poison, g.players[1].energy), (0, 0));
    assert_eq!(g.players[0].poison, 1, "only opponents lose their counters");
}

/// Passes until player 0's end-step trigger asks for its target.
fn to_end_step_target(g: &mut Game) {
    while !(g.step == Step::End && matches!(g.pending, Pending::ChooseTarget(_))) {
        let legal = g.legal_actions();
        let a = [Action::PassPriority, Action::FinishAttacks, Action::FinishBlocks]
            .into_iter()
            .find(|a| legal.contains(a))
            .expect("a pass");
        act(g, a);
    }
}

#[test]
fn rakdos_patron_of_chaos_opponent_sacrifices_two_or_you_draw_two() {
    let mut g = table();
    put(&mut g, 0, "Rakdos, Patron of Chaos");
    let a = put(&mut g, 1, "Sol Ring");
    let b = put(&mut g, 1, "Mind Stone");
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let hand0 = g.players[0].hand.len();
    to_end_step_target(&mut g);
    act(&mut g, Action::Target(Target::Player(1)));
    // Lands aren't offered; the opponent picks two of the rest.
    resolve_all(&mut g, &[Action::Pick(a), Action::Pick(b)]);
    assert_eq!((g.obj(a).zone, g.obj(b).zone), (Zone::Graveyard, Zone::Graveyard));
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
    assert_eq!(g.players[0].hand.len(), hand0, "they sacrificed, so no cards");
    to_main1(&mut g, 5, 0);
    let ring = put(&mut g, 1, "Sol Ring");
    let hand0 = g.players[0].hand.len();
    to_end_step_target(&mut g);
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[Action::No]);
    assert_eq!(g.obj(ring).zone, Zone::Battlefield);
    assert_eq!(g.players[0].hand.len(), hand0 + 2, "they didn't, so you draw two");
}

#[test]
fn hanweir_battlements_grants_haste_and_its_meld_finds_no_garrison() {
    let mut g = table();
    let b = put(&mut g, 0, "Hanweir Battlements");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    act(&mut g, Action::Activate(b, 0));
    resolve_all(&mut g, &[Action::Target(obj(ghoul))]);
    assert!(g.keywords(ghoul).has(Keywords::HASTE));
    assert!(g.obj(b).tapped);
    // No Hanweir Garrison in the pool: the meld ability resolves and nothing is exiled.
    let b2 = put(&mut g, 0, "Hanweir Battlements");
    act(&mut g, Action::Activate(b2, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(b2).zone, Zone::Battlefield);
    assert_eq!(g.obj(ghoul).zone, Zone::Battlefield);
}

#[test]
fn errant_and_giada_casts_fliers_and_flash_from_the_top_only() {
    let mut g = table();
    put(&mut g, 0, "Errant and Giada");
    let drake = top(&mut g, "Gilded Drake");
    assert!(
        g.legal_actions().contains(&Action::Cast(drake)),
        "a flier from the top"
    );
    let ghoul = top(&mut g, "Hungry Ghoul");
    assert!(
        !g.legal_actions().contains(&Action::Cast(ghoul)),
        "no flash or flying"
    );
}

// ---- Elder Brain, Inkshield, The One Ring, Deep-Cavern Bat ----

/// Passes (finishing attacks and blocks) until `step`.
fn pass_to(g: &mut Game, step: Step) {
    for _ in 0..200 {
        if g.step == step && matches!(g.pending, Pending::Priority(_)) {
            return;
        }
        let legal = g.legal_actions();
        let a = [Action::PassPriority, Action::FinishAttacks, Action::FinishBlocks]
            .into_iter()
            .find(|a| legal.contains(a))
            .expect("a pass");
        act(g, a);
    }
    panic!("never reached {step:?}");
}

#[test]
fn elder_brain_exiles_the_hand_they_draw_that_many_and_you_play_them_with_any_mana() {
    let mut g = game(&["Plains", "Plains", "Plains"]);
    let brain = put(&mut g, 0, "Elder Brain");
    let elves = hand(&mut g, 1, "Llanowar Elves");
    let mountain = hand(&mut g, 1, "Mountain");
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    let before: Vec<ObjId> = g.players[1].hand.clone();
    act(&mut g, Action::Attack(brain));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[]);
    for &c in &before {
        assert_eq!(g.obj(c).zone, Zone::Exile, "the whole hand is exiled");
    }
    assert_eq!(g.players[1].hand.len(), before.len(), "they draw that many");
    assert!(g.players[1].hand.iter().all(|c| !before.contains(c)));
    pass_to(&mut g, Step::Main2);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::PlayLand(mountain)), "lands too");
    assert!(legal.contains(&Action::Cast(elves)), "{{G}} paid with Plains");
    act(&mut g, Action::Cast(elves));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
    assert_eq!(g.obj(elves).controller, 0);
}

#[test]
fn inkshield_prevents_combat_damage_and_makes_an_inkling_per_point() {
    let mut g = table();
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let shield = hand(&mut g, 0, "Inkshield");
    g.start_turn(4, 1);
    while g.pending != Pending::DeclareAttackers(1) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(ghoul));
    act(&mut g, Action::FinishAttacks);
    while g.pending != Pending::Priority(0) {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    act(&mut g, Action::Cast(shield));
    while g.step != Step::Main2 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[0].life, 40, "prevented");
    let inklings = tokens(&g, 0, "Inkling");
    assert_eq!(inklings.len(), g.power(ghoul) as usize, "one per point prevented");
    assert!(g.keywords(inklings[0]).has(Keywords::FLYING));
}

#[test]
fn the_one_ring_protects_if_cast_then_burdens_draw_and_drain() {
    let mut g = table();
    let ring = hand(&mut g, 0, "The One Ring");
    play(&mut g, ring, false, &[], &[]);
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    assert!(
        !g.legal_actions().contains(&Action::Target(Target::Player(0))),
        "protection from everything"
    );
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    let hand0 = g.players[0].hand.len();
    act(&mut g, Action::Activate(ring, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ring).kind_counters, 1);
    assert_eq!(g.players[0].hand.len(), hand0 + 1, "one burden, one card");
    to_main1(&mut g, 5, 0);
    assert_eq!(g.players[0].life, 39, "1 life per burden counter at upkeep");
    let hand0 = g.players[0].hand.len();
    act(&mut g, Action::Activate(ring, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), hand0 + 2, "two burdens, two cards");
    // Not cast: no protection.
    let mut g = table();
    put(&mut g, 0, "The One Ring");
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    assert!(g.legal_actions().contains(&Action::Target(Target::Player(0))));
}

#[test]
fn deep_cavern_bat_exiles_a_nonland_card_until_it_leaves() {
    let mut g = table();
    let ring = hand(&mut g, 1, "Sol Ring");
    let land = hand(&mut g, 1, "Mountain");
    let bat = hand(&mut g, 0, "Deep-Cavern Bat");
    act(&mut g, Action::Cast(bat));
    let legal_first = {
        resolve_until_decision(&mut g);
        g.legal_actions()
    };
    assert!(legal_first.contains(&Action::Pick(ring)));
    assert!(!legal_first.contains(&Action::Pick(land)), "nonland only");
    resolve_all(&mut g, &[Action::Pick(ring)]);
    assert_eq!(g.obj(ring).zone, Zone::Exile);
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, bolt, false, &[obj(bat)], &[]);
    assert_eq!(g.obj(bat).zone, Zone::Graveyard);
    assert!(g.players[1].hand.contains(&ring), "back to its owner's hand");
}

/// Passes and targets player 1 until a resolution decision comes up.
fn resolve_until_decision(g: &mut Game) {
    while !matches!(g.pending, Pending::Decide(_)) {
        let a = match g.pending {
            Pending::ChooseTarget(_) => Action::Target(Target::Player(1)),
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

// ---- Sefris, Urabrask, The True Scriptures: transforms and Rona's back face ----

/// `game(&[])` with every basic five times for each player, after `edit` changes the CMD defs.
fn table_with(edit: impl Fn(&mut Vec<bolt_core::CardDef>)) -> Game {
    let mut defs = (*set("CMD").unwrap().defs).clone();
    edit(&mut defs);
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
        for p in 0..2 {
            for _ in 0..5 {
                put(&mut g, p, land);
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
fn activate_only_after_three_instants_or_sorceries_or_with_eight_in_an_opponents_graveyard() {
    let mut g = table_with(|defs| {
        let atlas = &mut defs[cid("Otherworld Atlas")];
        atlas.activated[0].min_instants_sorceries_this_turn = 1;
        atlas.activated[1].opponent_min_graveyard = 2;
    });
    let atlas = put(&mut g, 0, "Otherworld Atlas");
    let legal = g.legal_actions();
    assert!(
        !legal.contains(&Action::Activate(atlas, 0)),
        "no instant cast yet"
    );
    assert!(
        !legal.contains(&Action::Activate(atlas, 1)),
        "their graveyard is empty"
    );
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    g.add_card(1, cid("Plains"), Zone::Graveyard);
    assert!(
        g.legal_actions().contains(&Action::Activate(atlas, 0)),
        "one cast"
    );
    assert!(
        !g.legal_actions().contains(&Action::Activate(atlas, 1)),
        "one card"
    );
    g.add_card(1, cid("Plains"), Zone::Graveyard);
    assert!(
        g.legal_actions().contains(&Action::Activate(atlas, 1)),
        "two cards"
    );
}

#[test]
fn rona_untaps_on_legendary_casts_transforms_and_takes_a_land_from_the_damagers_hand() {
    let mut g = table();
    let rona = put(
        &mut g,
        0,
        "Rona, Herald of Invasion // Rona, Tolarian Obliterator",
    );
    g.objects[rona.0 as usize].tapped = true;
    let squee = hand(&mut g, 0, "Squee, the Immortal");
    play(&mut g, squee, false, &[], &[]);
    assert!(!g.obj(rona).tapped, "a legendary spell untaps Rona");
    act(&mut g, Action::Activate(rona, 1));
    resolve_all(&mut g, &[]);
    assert!(g.obj(rona).transformed);
    assert_eq!(g.def_of(rona).name, "Rona, Tolarian Obliterator");
    // Player 0's own Lightning Bolt: its controller (player 0) exiles a card at random, and the
    // rest of player 0's hand is Plains.
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    let lands = g.permanents(0).filter(|&x| g.types_of(x).land).count();
    let hand0 = g.players[0].hand.len();
    play(&mut g, bolt, false, &[obj(rona)], &[Action::Yes]);
    assert_eq!(g.obj(rona).zone, Zone::Battlefield, "5 toughness");
    assert_eq!(
        g.players[0].hand.len(),
        hand0 - 2,
        "the Bolt, and the exiled card"
    );
    assert_eq!(
        g.permanents(0).filter(|&x| g.types_of(x).land).count(),
        lands + 1,
        "the land onto the battlefield"
    );
}

#[test]
fn each_opponent_sacrifices_a_nontoken_creature_or_planeswalker() {
    let mut g = table_with(|defs| {
        defs[cid("Inkshield")].spell = Some(bolt_core::Ability::new(
            vec![],
            vec![bolt_core::Effect::OpponentSacrificesNontokenCreatureOrPlaneswalker],
        ));
    });
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let edict = hand(&mut g, 0, "Inkshield"); // standing in for Sheoldred's enters trigger
    act(&mut g, Action::Cast(edict));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(ghoul)));
    assert!(
        legal
            .iter()
            .all(|a| matches!(a, Action::Pick(x) if !g.types_of(*x).land)),
        "no lands"
    );
    resolve_all(&mut g, &[Action::Pick(ghoul)]);
    assert_eq!(g.obj(ghoul).zone, Zone::Graveyard);
    // Nothing to sacrifice: no question.
    let edict = hand(&mut g, 0, "Inkshield");
    play(&mut g, edict, false, &[], &[]);
}

// ---- Live-missing and idea quick wins (objects and zones), batch 17 ----

#[test]
fn silundi_vision_takes_an_instant_from_six_or_is_a_tapped_island() {
    let mut g = table();
    let bolt = top(&mut g, "Lightning Bolt");
    let v = hand(&mut g, 0, "Silundi Vision // Silundi Isle");
    play(&mut g, v, false, &[], &[Action::Pick(bolt)]);
    assert_eq!(g.obj(bolt).zone, Zone::Hand);
    let mut g = table();
    let v = hand(&mut g, 0, "Silundi Vision // Silundi Isle");
    act(&mut g, Action::PlayLand(v));
    resolve_all(&mut g, &[]);
    assert_eq!(g.def_of(v).name, "Silundi Isle");
    assert!(g.types_of(v).land && g.obj(v).tapped);
}

#[test]
fn slip_out_the_back_counters_and_phases_out() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let slip = hand(&mut g, 0, "Slip Out the Back");
    play(&mut g, slip, false, &[obj(ghoul)], &[]);
    assert_eq!(g.obj(ghoul).counters, 1);
    assert!(g.phased_out.iter().any(|&(x, _)| x == ghoul));
}

#[test]
fn city_of_shadows_stores_exiled_creatures_as_colorless_mana() {
    let mut g = game(&[]);
    let city = put(&mut g, 0, "City of Shadows");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    act(&mut g, Action::ActivateWith(city, 0, ghoul));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ghoul).zone, Zone::Exile);
    assert_eq!(g.obj(city).kind_counters, 1);
    to_main1(&mut g, 5, 0);
    let ring = hand(&mut g, 0, "Sol Ring");
    assert!(
        g.legal_actions().contains(&Action::Cast(ring)),
        "one storage counter: {{C}}"
    );
}

#[test]
fn elspeth_tirel_gains_per_creature_makes_soldiers_and_wipes_nontoken_nonlands() {
    let mut g = table();
    let elspeth = put(&mut g, 0, "Elspeth Tirel");
    g.objects[elspeth.0 as usize].loyalty = 4;
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let ring = put(&mut g, 1, "Sol Ring");
    let life = g.players[0].life;
    act(&mut g, Action::Loyalty(elspeth, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Soldier").len(), 3);
    to_main1(&mut g, 5, 0);
    act(&mut g, Action::Loyalty(elspeth, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, life + 3, "three Soldiers");
    g.objects[elspeth.0 as usize].loyalty = 5;
    to_main1(&mut g, 7, 0);
    act(&mut g, Action::Loyalty(elspeth, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(
        (g.obj(ghoul).zone, g.obj(ring).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
    assert_eq!(tokens(&g, 0, "Soldier").len(), 3, "tokens stay");
    assert!(g.permanents(1).any(|x| g.types_of(x).land), "lands stay");
}

#[test]
fn ramunap_excavator_plays_lands_from_the_graveyard() {
    let mut g = table();
    put(&mut g, 0, "Ramunap Excavator");
    let land = g.add_card(0, cid("Mountain"), Zone::Graveyard);
    assert!(g.legal_actions().contains(&Action::PlayLand(land)));
}

#[test]
fn aquitects_defenses_grants_hexproof_this_turn_and_plus_one_plus_two() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let aura = hand(&mut g, 0, "Aquitect's Defenses");
    play(&mut g, aura, false, &[obj(ghoul)], &[]);
    assert!(g.keywords(ghoul).has(Keywords::HEXPROOF));
    assert_eq!((g.power(ghoul), g.toughness(ghoul)), (3, 4));
    // Through this turn's cleanup (CR 514.2), not a jump to a new turn.
    let turn = g.turn;
    while g.turn == turn {
        let legal = g.legal_actions();
        let a = [Action::PassPriority, Action::FinishAttacks, Action::FinishBlocks]
            .into_iter()
            .find(|a| legal.contains(a))
            .unwrap_or(legal[0]); // the cleanup discard to hand size
        act(&mut g, a);
    }
    assert!(!g.keywords(ghoul).has(Keywords::HEXPROOF), "until end of turn");
    assert_eq!((g.power(ghoul), g.toughness(ghoul)), (3, 4), "the +1/+2 stays");
}

#[test]
fn arthur_puts_a_creature_from_the_top_six_in_attacking_and_returns_it_at_end_of_combat() {
    let mut g = table();
    let arthur = put(&mut g, 0, "Arthur, Marigold Knight");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    let elves = top(&mut g, "Llanowar Elves");
    act(&mut g, Action::Attack(arthur));
    act(&mut g, Action::Attack(ghoul));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[Action::Pick(elves)]);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
    assert!(g.obj(elves).tapped);
    assert!(
        g.combat.as_ref().is_some_and(|c| c.attackers.contains(&elves)),
        "attacking"
    );
    pass_to(&mut g, Step::Main2);
    assert_eq!(g.obj(elves).zone, Zone::Hand, "back at end of combat");
}

#[test]
fn arthur_alone_looks_at_nothing() {
    let mut g = table();
    let arthur = put(&mut g, 0, "Arthur, Marigold Knight");
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    top(&mut g, "Llanowar Elves");
    act(&mut g, Action::Attack(arthur));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[]);
}

#[test]
fn goliath_daydreamer_dream_exiles_hand_spells_and_casts_one_free_on_attack() {
    let mut g = table();
    let goliath = put(&mut g, 0, "Goliath Daydreamer");
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    assert_eq!(g.obj(bolt).zone, Zone::Exile, "exiled with a dream counter");
    assert_eq!(g.players[1].life, 37);
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(goliath));
    act(&mut g, Action::FinishAttacks);
    while !matches!(g.pending, Pending::FreeCast(_)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 34, "cast free from exile");
    assert_eq!(g.obj(bolt).zone, Zone::Graveyard, "not cast from hand this time");
}

// ---- Pippin and Toxrill (d3 batch 30's pieces) ----

#[test]
fn pippin_gives_another_creature_protection_from_the_chosen_card_type() {
    let mut g = table();
    let pippin = put(&mut g, 0, "Pippin, Guard of the Citadel");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    to_main1(&mut g, 5, 0);
    act(&mut g, Action::Activate(pippin, 0));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(pippin))),
        "another creature"
    );
    act(&mut g, Action::Target(obj(ghoul)));
    resolve_all(&mut g, &[Action::Mode(3)]);
    assert!(g.obj(pippin).tapped);
    let bolt = hand(&mut g, 1, "Lightning Bolt");
    let elves = put(&mut g, 1, "Llanowar Elves");
    assert!(g.type_protected(ghoul, bolt), "protection from instants");
    assert!(!g.type_protected(ghoul, elves), "not from creatures");
    to_main1(&mut g, 7, 0);
    assert!(!g.type_protected(ghoul, bolt), "until end of turn");
}

#[test]
fn toxrill_slimes_shrinks_makes_slugs_and_draws_off_them() {
    let mut g = table();
    let tox = put(&mut g, 0, "Toxrill, the Corrosive");
    let elves = put(&mut g, 1, "Llanowar Elves");
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let (p, t) = (g.power(ghoul), g.toughness(ghoul));
    pass_to(&mut g, Step::End);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elves).zone, Zone::Graveyard, "a 1/1 with a slime counter");
    assert_eq!((g.power(ghoul), g.toughness(ghoul)), (p - 1, t - 1));
    let slugs = tokens(&g, 0, "Slug");
    assert_eq!(slugs.len(), 1, "a Slug for the slimed creature that died");
    let cards = g.players[0].hand.len();
    act(&mut g, Action::ActivateWith(tox, 0, slugs[0]));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), cards + 1);
    assert!(tokens(&g, 0, "Slug").is_empty(), "the Slug was sacrificed");
}

// ---- Rev, Tithe Extractor; Ashiok, Nightmare Muse's pieces (batch 19) ----

#[test]
fn rev_grants_deathtouch_and_exiles_their_top_card_face_down_castable() {
    let mut g = table();
    let rev = put(&mut g, 0, "Rev, Tithe Extractor");
    let bolt = g.add_card(1, cid("Lightning Bolt"), Zone::Library);
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(rev));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[Action::Target(obj(rev))]);
    assert!(g.keywords(rev).has(Keywords::DEATHTOUCH));
    pass_to(&mut g, Step::Main2);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1);
    assert_eq!(g.obj(bolt).zone, Zone::Exile);
    assert!(g.exiled_face_down(bolt));
    assert!(
        g.legal_actions().contains(&Action::Cast(bolt)),
        "castable while exiled"
    );
    to_main1(&mut g, 9, 0);
    assert!(
        g.legal_actions().contains(&Action::Cast(bolt)),
        "for as long as it remains exiled"
    );
}

#[test]
fn rev_leaves_an_exiled_land_uncastable() {
    let mut g = table();
    let land = g.add_card(1, cid("Mountain"), Zone::Library);
    let rev = put(&mut g, 0, "Rev, Tithe Extractor");
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(rev));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[Action::Target(obj(rev))]);
    pass_to(&mut g, Step::Main2);
    assert_eq!(g.obj(land).zone, Zone::Exile);
    assert!(!g.legal_actions().contains(&Action::PlayLand(land)));
}

#[test]
fn casting_up_to_three_face_up_opponent_exiled_spells_free() {
    let mut g = table_with(|defs| {
        defs[cid("Inkshield")].spell = Some(bolt_core::Ability::new(
            vec![],
            vec![bolt_core::Effect::CastFaceUpOpponentExiledFreeUpTo(3)],
        ));
    });
    let bolts: Vec<ObjId> = (0..4)
        .map(|_| g.add_card(1, cid("Lightning Bolt"), Zone::Exile))
        .collect();
    let hidden = g.add_card(1, cid("Lightning Bolt"), Zone::Exile);
    let zcc = g.obj(hidden).zcc;
    g.face_down_exiled.push((hidden, zcc));
    let mine = g.add_card(0, cid("Lightning Bolt"), Zone::Exile);
    let muse = hand(&mut g, 0, "Inkshield"); // standing in for Ashiok's −7
    act(&mut g, Action::Cast(muse));
    while !matches!(g.pending, Pending::FreeCast(_)) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Cast(hidden)), "face-up cards only");
    assert!(!legal.contains(&Action::Cast(mine)), "cards your opponents own");
    for &b in &bolts[..3] {
        act(&mut g, Action::Cast(b));
        act(&mut g, Action::Target(Target::Player(1)));
    }
    assert!(
        !matches!(g.pending, Pending::FreeCast(_)),
        "up to three: the window closes"
    );
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 31);
    assert_eq!(g.obj(bolts[3]).zone, Zone::Exile);
    assert!(!g.obj(bolts[3]).cast_free);
}

#[test]
fn each_opponent_exiles_the_top_two_cards() {
    let mut g = table_with(|defs| {
        defs[cid("Inkshield")].spell = Some(bolt_core::Ability::new(
            vec![],
            vec![bolt_core::Effect::EachOpponentExilesTop(2)],
        ));
    });
    let a = g.add_card(1, cid("Lightning Bolt"), Zone::Library);
    let b = g.add_card(1, cid("Hungry Ghoul"), Zone::Library);
    let c = hand(&mut g, 0, "Inkshield"); // standing in for the Nightmare token's trigger
    play(&mut g, c, false, &[], &[]);
    assert_eq!((g.obj(a).zone, g.obj(b).zone), (Zone::Exile, Zone::Exile));
}

// ---- Alchemist's Talent, Fortune Teller's Talent, Evercoat Ursine (batch 20) ----

#[test]
fn alchemists_talent_makes_tapped_treasures_then_burns_for_treasure_spells() {
    let mut g = table();
    let talent = hand(&mut g, 0, "Alchemist's Talent");
    play(&mut g, talent, false, &[], &[]);
    let treasures = tokens(&g, 0, "Treasure");
    assert_eq!(treasures.len(), 2);
    assert!(treasures.iter().all(|&t| g.obj(t).tapped), "tapped");
    assert!(
        g.legal_actions().contains(&Action::Activate(talent, 0)),
        "level 2 as a sorcery"
    );
    // Level 3 on a board with only Treasures, so the Bolt is paid with Treasure mana.
    let mut g = game(&[]);
    let talent = put(&mut g, 0, "Alchemist's Talent");
    g.objects[talent.0 as usize].class_level = 3;
    let treasure = g
        .defs
        .iter()
        .position(|d| d.token && d.name == "Treasure")
        .expect("Treasure");
    g.add_card(0, treasure, Zone::Battlefield);
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    assert_eq!(g.players[1].life, 36, "3 from the Bolt, 1 for its mana value");
}

#[test]
fn alchemists_talent_level_three_ignores_spells_without_treasure_mana() {
    let mut g = table();
    let talent = put(&mut g, 0, "Alchemist's Talent");
    g.objects[talent.0 as usize].class_level = 3;
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    assert_eq!(g.players[1].life, 37);
}

#[test]
fn fortune_tellers_talent_level_two_plays_the_top_after_a_spell() {
    let mut g = table();
    let talent = put(&mut g, 0, "Fortune Teller's Talent");
    let land = top(&mut g, "Mountain");
    assert!(!g.legal_actions().contains(&Action::PlayLand(land)), "level 1");
    g.objects[talent.0 as usize].class_level = 2;
    assert!(
        !g.legal_actions().contains(&Action::PlayLand(land)),
        "no spell cast yet this turn"
    );
    let elves = hand(&mut g, 0, "Llanowar Elves");
    play(&mut g, elves, false, &[], &[]);
    assert!(g.legal_actions().contains(&Action::PlayLand(land)));
    let bolt = top(&mut g, "Lightning Bolt");
    assert!(g.legal_actions().contains(&Action::Cast(bolt)), "spells too");
}

#[test]
fn two_hideaways_and_one_free_spell_on_combat_damage() {
    // Evercoat Ursine's pieces on an edited Hungry Ghoul (the Ursine itself is unclaimed).
    let mut g = table_with(|defs| {
        let d = &mut defs[cid("Hungry Ghoul")];
        for (when, effects) in [
            (
                bolt_core::When::Enters,
                vec![
                    bolt_core::Effect::Hideaway(3),
                    bolt_core::Effect::HideawayAnother(3),
                ],
            ),
            (
                bolt_core::When::DealsCombatDamageToPlayer,
                vec![bolt_core::Effect::MayCastOneHideawayFree],
            ),
        ] {
            d.triggers.push(bolt_core::Trigger {
                when,
                ability: bolt_core::Ability::new(vec![], effects),
            });
        }
    });
    let first = top(&mut g, "Lightning Bolt");
    for _ in 0..2 {
        top(&mut g, "Plains");
    }
    let second = top(&mut g, "Lightning Bolt");
    for _ in 0..2 {
        top(&mut g, "Plains");
    }
    let bear = hand(&mut g, 0, "Hungry Ghoul");
    play(
        &mut g,
        bear,
        false,
        &[],
        &[Action::Pick(second), Action::Pick(first)],
    );
    assert_eq!(
        (g.obj(first).zone, g.obj(second).zone),
        (Zone::Exile, Zone::Exile)
    );
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(bear));
    act(&mut g, Action::FinishAttacks);
    while !matches!(g.pending, Pending::FreeCast(_)) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Cast(first)) && legal.contains(&Action::Cast(second)));
    act(&mut g, Action::Cast(first));
    act(&mut g, Action::Target(Target::Player(1)));
    assert!(!matches!(g.pending, Pending::FreeCast(_)), "one of them");
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(first).zone, Zone::Graveyard);
    assert_eq!(g.obj(second).zone, Zone::Exile);
}

#[test]
fn blade_of_the_bloodchief_grows_its_creature_twice_as_fast_on_a_vampire() {
    let mut g = table();
    let blade = put(&mut g, 0, "Blade of the Bloodchief");
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    act(&mut g, Action::Activate(blade, 0));
    resolve_all(&mut g, &[Action::Target(obj(ghoul))]);
    let elves = put(&mut g, 1, "Llanowar Elves");
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, bolt, false, &[obj(elves)], &[]);
    assert_eq!(g.obj(ghoul).counters, 1, "a creature died");
    let bat = put(&mut g, 0, "Vampire Nighthawk");
    act(&mut g, Action::Activate(blade, 0));
    resolve_all(&mut g, &[Action::Target(obj(bat))]);
    let elves = put(&mut g, 1, "Llanowar Elves");
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, bolt, false, &[obj(elves)], &[]);
    assert_eq!(g.obj(bat).counters, 2, "a Vampire gets two");
    assert_eq!(g.obj(ghoul).counters, 1, "no longer equipped");
}

#[test]
fn nesting_dragon_makes_eggs_that_hatch_into_firebreathing_dragons() {
    let mut g = table();
    put(&mut g, 0, "Nesting Dragon");
    let land = hand(&mut g, 0, "Mountain");
    act(&mut g, Action::PlayLand(land));
    resolve_all(&mut g, &[]);
    let eggs = tokens(&g, 0, "Dragon Egg");
    assert_eq!(eggs.len(), 1, "landfall");
    let egg = eggs[0];
    assert_eq!((g.power(egg), g.toughness(egg)), (0, 2));
    assert!(g.keywords(egg).has(Keywords::DEFENDER));
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    play(&mut g, bolt, false, &[obj(egg)], &[]);
    let dragons = tokens(&g, 0, "Dragon");
    assert_eq!(dragons.len(), 1, "the Egg died into a Dragon");
    let dragon = dragons[0];
    assert!(g.keywords(dragon).has(Keywords::FLYING));
    act(&mut g, Action::Activate(dragon, 0));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(dragon), g.toughness(dragon)), (3, 2), "{{R}}: +1/+0");
}

#[test]
fn ashiok_makes_nightmares_that_exile_two_and_bounces_then_exiles_from_hand() {
    let mut g = table();
    let ashiok = put(&mut g, 0, "Ashiok, Nightmare Muse");
    g.objects[ashiok.0 as usize].loyalty = 5;
    act(&mut g, Action::Loyalty(ashiok, 0));
    resolve_all(&mut g, &[]);
    let nightmare = tokens(&g, 0, "Nightmare")[0];
    assert_eq!((g.power(nightmare), g.toughness(nightmare)), (2, 3));
    let a = g.add_card(1, cid("Lightning Bolt"), Zone::Library);
    let b = g.add_card(1, cid("Hungry Ghoul"), Zone::Library);
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(nightmare));
    act(&mut g, Action::FinishAttacks);
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(a).zone, g.obj(b).zone), (Zone::Exile, Zone::Exile));
    pass_to(&mut g, Step::Main2);
    let elves = put(&mut g, 1, "Llanowar Elves");
    act(&mut g, Action::Loyalty(ashiok, 1));
    act(&mut g, Action::Target(obj(elves)));
    while !matches!(g.pending, Pending::Decide(1)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Pick(elves));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elves).zone, Zone::Exile, "bounced, then exiled from hand");
}

// ---- Wishclaw Talisman; Midgar, City of Mako (batch 22) ----

#[test]
fn wishclaw_talisman_tutors_then_goes_to_the_opponent() {
    let mut g = table();
    let talisman = hand(&mut g, 0, "Wishclaw Talisman");
    play(&mut g, talisman, false, &[], &[]);
    assert_eq!(g.obj(talisman).kind_counters, 3, "three wish counters");
    let ring = g.add_card(0, cid("Sol Ring"), Zone::Library);
    act(&mut g, Action::Activate(talisman, 0));
    resolve_all(&mut g, &[Action::Pick(ring)]);
    assert_eq!(g.obj(ring).zone, Zone::Hand);
    assert_eq!(g.obj(talisman).kind_counters, 2);
    assert_eq!(g.obj(talisman).controller, 1, "an opponent gains control of it");
    assert!(
        !g.legal_actions().contains(&Action::Activate(talisman, 0)),
        "only its controller, during their turn"
    );
    to_main1(&mut g, 6, 1);
    assert!(g.legal_actions().contains(&Action::Activate(talisman, 0)));
}

#[test]
fn midgar_is_a_tapped_swamp_or_an_adventure_that_sacrifices_to_draw_two() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let midgar = hand(&mut g, 0, "Midgar, City of Mako // Reactor Raid");
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::PlayLand(midgar)) && legal.contains(&Action::CastOmen(midgar)));
    assert!(!legal.contains(&Action::Cast(midgar)), "a land isn't cast");
    let cards = g.players[0].hand.len();
    act(&mut g, Action::CastOmen(midgar));
    resolve_all(&mut g, &[Action::Pick(ghoul)]);
    assert_eq!(g.obj(ghoul).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), cards - 1 + 2, "Midgar left, two drawn");
    assert_eq!(g.obj(midgar).zone, Zone::Exile, "on an adventure");
    act(&mut g, Action::PlayLand(midgar));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(midgar).zone, Zone::Battlefield, "played later from exile");
    assert!(g.obj(midgar).tapped);
}

#[test]
fn reactor_raid_declined_draws_nothing() {
    let mut g = table();
    put(&mut g, 0, "Hungry Ghoul");
    let midgar = hand(&mut g, 0, "Midgar, City of Mako // Reactor Raid");
    let cards = g.players[0].hand.len();
    act(&mut g, Action::CastOmen(midgar));
    resolve_all(&mut g, &[Action::No]);
    assert_eq!(g.players[0].hand.len(), cards - 1);
}

// ---- The monarch (batch 23): Court of Grace, Forth Eorlingas! ----

#[test]
fn court_of_grace_makes_you_the_monarch_who_draws_and_gets_angels() {
    let mut g = table();
    let court = hand(&mut g, 0, "Court of Grace");
    play(&mut g, court, false, &[], &[]);
    assert_eq!(g.monarch.map(|(p, _)| p), Some(0));
    let cards = g.players[0].hand.len();
    pass_to(&mut g, Step::End);
    resolve_all(&mut g, &[]);
    assert_eq!(
        g.players[0].hand.len(),
        cards + 1,
        "the monarch draws at their end step"
    );
    to_main1(&mut g, 5, 0);
    assert_eq!(tokens(&g, 0, "Angel").len(), 1, "an Angel instead of a Spirit");
    assert!(tokens(&g, 0, "Spirit").is_empty());
}

#[test]
fn combat_damage_to_the_monarch_takes_it_and_court_makes_spirits_without_it() {
    let mut g = table();
    put(&mut g, 0, "Court of Grace");
    let ghoul = put(&mut g, 1, "Hungry Ghoul");
    let court = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Court of Grace")
        .expect("court");
    g.monarch = Some((0, court));
    g.start_turn(6, 1);
    while g.pending != Pending::DeclareAttackers(1) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(ghoul));
    act(&mut g, Action::FinishAttacks);
    pass_to(&mut g, Step::Main2);
    assert_eq!(g.monarch.map(|(p, _)| p), Some(1), "combat damage to the monarch");
    to_main1(&mut g, 7, 0);
    assert_eq!(tokens(&g, 0, "Spirit").len(), 1, "not the monarch: a Spirit");
}

#[test]
fn forth_eorlingas_makes_knights_and_the_monarch_on_combat_damage() {
    let mut g = table();
    let forth = hand(&mut g, 0, "Forth Eorlingas!");
    act(&mut g, Action::CastX(forth, 2));
    resolve_all(&mut g, &[]);
    let knights = tokens(&g, 0, "Human Knight");
    assert_eq!(knights.len(), 2);
    assert!(g.keywords(knights[0]).has(Keywords::HASTE));
    assert_eq!(g.monarch, None);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(knights[0]));
    act(&mut g, Action::FinishAttacks);
    pass_to(&mut g, Step::Main2);
    assert_eq!(g.monarch.map(|(p, _)| p), Some(0));
}

#[test]
fn mirrorform_turns_your_nonland_permanents_into_copies() {
    let mut g = table();
    let ghoul = put(&mut g, 0, "Hungry Ghoul");
    let ring = put(&mut g, 0, "Sol Ring");
    let elves = put(&mut g, 1, "Llanowar Elves");
    let mirror = hand(&mut g, 0, "Mirrorform");
    play(&mut g, mirror, false, &[obj(elves)], &[]);
    assert_eq!(g.def_of(ghoul).name, "Llanowar Elves");
    assert_eq!(g.def_of(ring).name, "Llanowar Elves");
    assert!(
        g.permanents(0)
            .filter(|&x| g.types_of(x).land)
            .all(|x| g.def_of(x).name != "Llanowar Elves"),
        "lands stay"
    );
    assert_eq!(g.def_of(elves).name, "Llanowar Elves");
}
