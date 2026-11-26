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
