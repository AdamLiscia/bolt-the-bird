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
