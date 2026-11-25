//! Bloomburrow Commander (BLC), lane C: each script's behavior in a 1v1 Commander game.

mod common;

use bolt_cards::set;
use bolt_core::{Action, Game, Keywords, ObjId, Pending, Step, Target, Zone};
use common::{act, obj, play, resolve_all};

fn bid(name: &str) -> usize {
    set("BLC")
        .unwrap()
        .index_of(name)
        .unwrap_or_else(|| panic!("{name} is not in BLC"))
}

/// A Commander game (99 Plains each), Bello vs. Ms. Bumbleflower, at player 0's first main phase on
/// turn 3, with 5 of each basic for each player.
fn table() -> Game {
    let plains = vec![bid("Plains"); 99];
    let mut g = Game::new_commander(
        set("BLC").unwrap().defs.clone(),
        [&plains, &plains],
        [bid("Bello, Bard of the Brambles"), bid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    for p in 0..2 {
        for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
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

fn put(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, bid(name), Zone::Battlefield)
}

fn hand(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, bid(name), Zone::Hand)
}

fn tokens(g: &Game, p: usize, name: &str) -> Vec<ObjId> {
    g.permanents(p)
        .filter(|&x| g.is_token(x) && g.def_of(x).name == name)
        .collect()
}

fn token_def(g: &Game, name: &str) -> usize {
    (0..g.defs.len())
        .find(|&i| g.defs[i].token && g.defs[i].name == name)
        .expect("a token")
}

/// Pass through to the next turn (attacking and blocking with nothing).
fn to_next_turn(g: &mut Game) {
    let turn = g.turn;
    while g.turn == turn {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn toski_cant_be_countered_must_attack_and_draws_on_combat_damage() {
    let mut g = table();
    let t = put(&mut g, 0, "Toski, Bearer of Secrets");
    let d = g.def_of(t);
    assert!(d.uncounterable && d.must_attack && g.keywords(t).has(Keywords::INDESTRUCTIBLE));
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(t));
    act(&mut g, Action::FinishAttacks);
    let cards = g.players[0].hand.len();
    while g.step != Step::End {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[0].hand.len(), cards + 1);
}

#[test]
fn tear_asunder_exiles_an_artifact_or_with_kicker_any_nonland_permanent() {
    let mut g = table();
    let signet = put(&mut g, 1, "Arcane Signet");
    let creature = put(&mut g, 1, "Jolly Gerbils");
    let c = hand(&mut g, 0, "Tear Asunder");
    act(&mut g, Action::Cast(c));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(creature))),
        "unkicked: artifact or enchantment"
    );
    act(&mut g, Action::Target(obj(signet)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(signet).zone, Zone::Exile);
    let c2 = hand(&mut g, 0, "Tear Asunder");
    play(&mut g, c2, true, &[obj(creature)], &[]);
    assert_eq!(g.obj(creature).zone, Zone::Exile, "kicked: a creature too");
}

#[test]
fn ogre_slumlord_makes_deathtouch_rats_when_nontoken_creatures_die() {
    let mut g = table();
    let o = put(&mut g, 0, "Ogre Slumlord");
    let theirs = put(&mut g, 1, "Combat Celebrant"); // 4/1
    let c = hand(&mut g, 0, "Starstorm");
    act(&mut g, Action::CastX(c, 1)); // the 3/3 Ogre survives
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    let rats = tokens(&g, 0, "Rat");
    assert_eq!(rats.len(), 1);
    assert!(g.keywords(rats[0]).has(Keywords::DEATHTOUCH), "Rats you control");
    assert!(!g.keywords(o).has(Keywords::DEATHTOUCH));
}

#[test]
fn tireless_provisioner_makes_food_or_treasure_on_landfall() {
    let mut g = table();
    put(&mut g, 0, "Tireless Provisioner");
    let land = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(land));
    resolve_all(&mut g, &[Action::Mode(1)]);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1);
    assert!(tokens(&g, 0, "Food").is_empty());
}

#[test]
fn starstorm_deals_x_to_each_creature_and_cycles() {
    let mut g = table();
    let a = put(&mut g, 0, "Jolly Gerbils"); // 2/3
    let b = put(&mut g, 1, "Combat Celebrant"); // 4/1
    let c = hand(&mut g, 0, "Starstorm");
    act(&mut g, Action::CastX(c, 2));
    resolve_all(&mut g, &[]);
    assert_eq!((g.obj(a).zone, g.obj(a).damage), (Zone::Battlefield, 2));
    assert_eq!(g.obj(b).zone, Zone::Graveyard);
    let c2 = hand(&mut g, 0, "Starstorm");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(c2, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c2).zone, Zone::Graveyard, "cycled");
    assert_eq!(g.players[0].hand.len(), n);
}

#[test]
fn restoration_angel_blinks_a_non_angel_creature() {
    let mut g = table();
    let mine = put(&mut g, 0, "Jolly Gerbils");
    let theirs = put(&mut g, 1, "Combat Celebrant");
    let shield = hand(&mut g, 0, "Starstorm");
    act(&mut g, Action::CastX(shield, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(mine).damage, 1);
    let r = hand(&mut g, 0, "Restoration Angel");
    let k = g.def_of(r).keywords;
    assert!(k.has(Keywords::FLASH) && k.has(Keywords::FLYING));
    act(&mut g, Action::Cast(r));
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Target(obj(r))), "non-Angel");
    assert!(!legal.contains(&Action::Target(obj(theirs))), "you control");
    act(&mut g, Action::Target(obj(mine)));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.obj(mine).zone, Zone::Battlefield);
    assert_eq!(g.obj(mine).damage, 0, "a new object");
    assert_eq!(g.obj(mine).controller, 0);
}

#[test]
fn poison_tip_archer_drains_when_any_other_creature_dies() {
    let mut g = table();
    let a = put(&mut g, 0, "Poison-Tip Archer");
    let ks = g.keywords(a);
    assert!(ks.has(Keywords::REACH) && ks.has(Keywords::DEATHTOUCH));
    put(&mut g, 0, "Combat Celebrant"); // 4/1, yours
    put(&mut g, 1, "Combat Celebrant");
    let life = g.players[1].life;
    let c = hand(&mut g, 0, "Starstorm");
    act(&mut g, Action::CastX(c, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 2, "one each for yours and theirs");
}

#[test]
fn shamanic_revelation_draws_per_creature_and_gains_for_big_ones() {
    let mut g = table();
    put(&mut g, 0, "Jolly Gerbils");
    put(&mut g, 0, "Combat Celebrant"); // power 4
    let c = hand(&mut g, 0, "Shamanic Revelation");
    let (cards, life) = (g.players[0].hand.len(), g.players[0].life);
    play(&mut g, c, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), cards - 1 + 2);
    assert_eq!(g.players[0].life, life + 4);
}

#[test]
fn stolen_by_the_fae_bounces_mana_value_x_and_makes_x_faeries() {
    let mut g = table();
    let two = put(&mut g, 1, "Jolly Gerbils"); // mana value 2
    let three = put(&mut g, 1, "Combat Celebrant"); // mana value 3
    let c = hand(&mut g, 0, "Stolen by the Fae");
    act(&mut g, Action::CastX(c, 2));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(three))),
        "mana value X"
    );
    act(&mut g, Action::Target(obj(two)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(two).zone, Zone::Hand);
    let f = tokens(&g, 0, "Faerie");
    assert_eq!(f.len(), 2);
    assert!(g.keywords(f[0]).has(Keywords::FLYING));
}

#[test]
fn time_wipe_saves_one_of_yours_and_destroys_the_rest() {
    let mut g = table();
    let keep = put(&mut g, 0, "Jolly Gerbils");
    let other = put(&mut g, 0, "Combat Celebrant");
    let theirs = put(&mut g, 1, "Combat Celebrant");
    let c = hand(&mut g, 0, "Time Wipe");
    act(&mut g, Action::Cast(c));
    while !matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Pick(theirs)),
        "a creature you control"
    );
    act(&mut g, Action::Pick(keep));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(keep).zone, Zone::Hand);
    assert_eq!(
        (g.obj(other).zone, g.obj(theirs).zone),
        (Zone::Graveyard, Zone::Graveyard)
    );
}

#[test]
fn perplexing_test_bounces_tokens_or_nontoken_creatures() {
    let mut g = table();
    let real = put(&mut g, 1, "Jolly Gerbils");
    g.add_card(1, token_def(&g, "Rat"), Zone::Battlefield);
    let c = hand(&mut g, 0, "Perplexing Test");
    act(&mut g, Action::CastModal(c, 0));
    resolve_all(&mut g, &[]);
    assert!(tokens(&g, 1, "Rat").is_empty(), "tokens bounced (and gone)");
    assert_eq!(g.obj(real).zone, Zone::Battlefield);
    let c2 = hand(&mut g, 0, "Perplexing Test");
    act(&mut g, Action::CastModal(c2, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(real).zone, Zone::Hand);
    let _ = (Target::Player(0), to_next_turn as fn(&mut Game));
}

// ---- lane A, 2026-11-25 ----

#[test]
fn end_raze_forerunners_pumps_the_others() {
    let mut g = table();
    let bear = put(&mut g, 0, "Sakura-Tribe Elder");
    let p = g.power(bear);
    let boar = hand(&mut g, 0, "End-Raze Forerunners");
    play(&mut g, boar, false, &[], &[]);
    assert_eq!(g.power(bear), p + 2);
    assert!(g.keywords(bear).has(Keywords::VIGILANCE) && g.keywords(bear).has(Keywords::TRAMPLE));
    assert_eq!(g.power(boar), 7, "other creatures only");
}

#[test]
fn elspeth_suns_champion_makes_soldiers_and_wipes_big_creatures() {
    let mut g = table();
    let elspeth = hand(&mut g, 0, "Elspeth, Sun's Champion");
    play(&mut g, elspeth, false, &[], &[]);
    assert_eq!(g.obj(elspeth).loyalty, 4);
    let big = put(&mut g, 1, "End-Raze Forerunners"); // 7/7
    let small = put(&mut g, 1, "Sakura-Tribe Elder");
    act(&mut g, Action::Loyalty(elspeth, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Soldier").len(), 3);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Loyalty(elspeth, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(big).zone, Zone::Graveyard, "power 4 or greater");
    assert_eq!(g.obj(small).zone, Zone::Battlefield);
    assert_eq!(tokens(&g, 0, "Soldier").len(), 3, "1/1s survive");
}

#[test]
fn elspeths_emblem_gives_plus_two_and_flying() {
    let mut g = table();
    let elspeth = hand(&mut g, 0, "Elspeth, Sun's Champion");
    play(&mut g, elspeth, false, &[], &[]);
    assert_eq!(g.obj(elspeth).loyalty, 4);
    let bear = put(&mut g, 0, "Sakura-Tribe Elder");
    let p = g.power(bear);
    for turn in [3u32, 5, 7] {
        if turn > 3 {
            g.start_turn(turn, 0);
            while g.step != Step::Main1 {
                act(&mut g, Action::PassPriority);
            }
        }
        act(&mut g, Action::Loyalty(elspeth, 0)); // +1, from 4 to 7
        resolve_all(&mut g, &[]);
    }
    assert_eq!(g.obj(elspeth).loyalty, 7);
    g.start_turn(9, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Loyalty(elspeth, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(g.power(bear), p + 2);
    assert!(g.keywords(bear).has(Keywords::FLYING));
}

#[test]
fn chatterstorm_copies_for_each_earlier_spell() {
    let mut g = table();
    let a = hand(&mut g, 0, "Chatterstorm");
    let b = hand(&mut g, 0, "Chatterstorm");
    play(&mut g, a, false, &[], &[]);
    assert_eq!(tokens(&g, 0, "Squirrel").len(), 1, "first spell: no copies");
    play(&mut g, b, false, &[], &[]);
    assert_eq!(
        tokens(&g, 0, "Squirrel").len(),
        3,
        "second spell: itself plus one copy"
    );
}

#[test]
fn arasta_makes_a_spider_when_an_opponent_casts_an_instant_or_sorcery() {
    let mut g = table();
    put(&mut g, 1, "Arasta of the Endless Web");
    let storm = hand(&mut g, 0, "Chatterstorm");
    play(&mut g, storm, false, &[], &[]);
    let spiders = tokens(&g, 1, "Spider");
    assert_eq!(spiders.len(), 1);
    assert!(g.keywords(spiders[0]).has(Keywords::REACH));
    let elder = hand(&mut g, 0, "Sakura-Tribe Elder");
    play(&mut g, elder, false, &[], &[]);
    assert_eq!(tokens(&g, 1, "Spider").len(), 1, "a creature spell doesn't count");
}

/// A Commander game where player 0 has exactly these lands.
fn with_lands(names: &[&str]) -> Game {
    let plains = vec![bid("Plains"); 99];
    let mut g = Game::new_commander(
        set("BLC").unwrap().defs.clone(),
        [&plains, &plains],
        [bid("Bello, Bard of the Brambles"), bid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    for n in names {
        put(&mut g, 0, n);
    }
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g
}

#[test]
fn a_pain_land_deals_one_damage_only_when_its_colored_mana_is_needed() {
    // {1}{U} with Adarkar Wastes and a Plains: the Wastes has to make {U}.
    let mut g = with_lands(&["Adarkar Wastes", "Plains"]);
    let life = g.players[0].life;
    let escort = hand(&mut g, 0, "Plumecreed Escort");
    play(&mut g, escort, false, &[], &[Action::Target(obj(escort))]); // its enter trigger
    assert_eq!(g.obj(escort).zone, Zone::Battlefield);
    assert_eq!(
        g.players[0].life,
        life - 1,
        "colored mana from a pain land: 1 damage"
    );

    // {2} with the same lands: the Wastes can make {C}, so no damage.
    let mut g = with_lands(&["Adarkar Wastes", "Plains"]);
    let life = g.players[0].life;
    let thopter = hand(&mut g, 0, "Ornithopter of Paradise");
    play(&mut g, thopter, false, &[], &[]);
    assert_eq!(g.obj(thopter).zone, Zone::Battlefield);
    assert_eq!(
        g.players[0].life, life,
        "generic mana: tapped for {{C}}, no damage"
    );
}

#[test]
fn a_signet_turns_one_mana_into_two_colors() {
    // {1}{U} from a Plains and an Azorius Signet: the Plains pays the Signet's {1}; {W}{U} pays the spell.
    let mut g = with_lands(&["Plains", "Azorius Signet"]);
    let escort = hand(&mut g, 0, "Plumecreed Escort");
    assert!(g.legal_actions().contains(&Action::Cast(escort)));
    play(&mut g, escort, false, &[], &[Action::Target(obj(escort))]);
    assert_eq!(g.obj(escort).zone, Zone::Battlefield);
    assert!(g
        .permanents(0)
        .filter(|&x| g.def_of(x).name != "Plumecreed Escort")
        .all(|x| g.obj(x).tapped));

    // A Signet alone makes nothing: its {1} needs another source.
    let mut g = with_lands(&["Azorius Signet"]);
    let thopter = hand(&mut g, 0, "Ornithopter of Paradise");
    assert!(!g.legal_actions().contains(&Action::Cast(thopter)));
}

#[test]
fn a_filter_land_turns_hybrid_mana_into_two_of_its_colors() {
    // {1}{U} from a Mountain and Cascade Bluffs: only the filter ability ({U/R}: add two of U/R) gets blue.
    let mut g = with_lands(&["Mountain", "Cascade Bluffs"]);
    let escort = hand(&mut g, 0, "Plumecreed Escort");
    assert!(
        g.legal_actions().contains(&Action::Cast(escort)),
        "Mountain into the Bluffs makes {{U}}"
    );
    play(&mut g, escort, false, &[], &[Action::Target(obj(escort))]);
    assert_eq!(g.obj(escort).zone, Zone::Battlefield);
    // The Bluffs' own {T}: Add {C} still works on its own.
    let mut g = with_lands(&["Cascade Bluffs"]);
    let thopter = hand(&mut g, 0, "Ornithopter of Paradise"); // {2}
    assert!(
        !g.legal_actions().contains(&Action::Cast(thopter)),
        "one land, one mana"
    );
}

#[test]
fn necroblossom_snarl_enters_untapped_only_with_a_swamp_or_forest_in_hand() {
    let mut g = with_lands(&[]);
    let snarl = hand(&mut g, 0, "Necroblossom Snarl");
    act(&mut g, Action::PlayLand(snarl));
    assert!(g.obj(snarl).tapped, "nothing to reveal");
    let mut g = with_lands(&[]);
    hand(&mut g, 0, "Forest");
    let snarl = hand(&mut g, 0, "Necroblossom Snarl");
    act(&mut g, Action::PlayLand(snarl));
    assert!(!g.obj(snarl).tapped, "revealed a Forest");
}

#[test]
fn primeval_bounty_rewards_creatures_noncreatures_and_lands() {
    let mut g = table();
    put(&mut g, 0, "Primeval Bounty");
    let elder = hand(&mut g, 0, "Sakura-Tribe Elder");
    play(&mut g, elder, false, &[], &[]);
    assert_eq!(tokens(&g, 0, "Beast").len(), 1, "a creature spell: a 3/3 Beast");
    let storm = hand(&mut g, 0, "Chatterstorm");
    play(&mut g, storm, false, &[], &[Action::Target(obj(elder))]);
    assert_eq!(g.obj(elder).counters, 3, "a noncreature spell: three counters");
    let life = g.players[0].life;
    let forest = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(forest));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, life + 3, "landfall: 3 life");
}

#[test]
fn nested_shambler_leaves_tapped_squirrels_equal_to_its_power() {
    let mut g = table();
    let shambler = put(&mut g, 0, "Nested Shambler");
    let p = g.power(shambler);
    let putrefy = hand(&mut g, 0, "Putrefy"); // destroy target artifact or creature
    play(&mut g, putrefy, false, &[obj(shambler)], &[]);
    assert_eq!(g.obj(shambler).zone, Zone::Graveyard);
    let squirrels = tokens(&g, 0, "Squirrel");
    assert_eq!(squirrels.len(), p.max(0) as usize);
    assert!(squirrels.iter().all(|&s| g.obj(s).tapped));
}

#[test]
fn greater_good_draws_the_sacrificed_power_then_discards_three() {
    let mut g = table();
    put(&mut g, 0, "Greater Good");
    let boar = put(&mut g, 0, "End-Raze Forerunners"); // power 7
    for _ in 0..10 {
        g.add_card(0, bid("Plains"), Zone::Library);
    }
    let hand_before = g.players[0].hand.len();
    let act_sac = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a, Action::ActivateWith(_, _, v) if *v == boar))
        .expect("sacrifice the Boar");
    act(&mut g, act_sac);
    assert_eq!(g.obj(boar).zone, Zone::Graveyard);
    let mut discards = 0;
    for _ in 0..30 {
        if g.items.is_empty() && !matches!(g.pending, Pending::Decide(_) | Pending::Discard(_)) {
            break;
        }
        let legal = g.legal_actions();
        if let Some(d) = legal.iter().find(|a| matches!(a, Action::Discard(_))) {
            act(&mut g, *d);
            discards += 1;
        } else {
            act(&mut g, Action::PassPriority);
        }
    }
    assert_eq!(discards, 3);
    assert_eq!(g.players[0].hand.len(), hand_before + 7 - 3);
}

// ---- lane C, 2026-11-25 ----

#[test]
fn garruks_packleader_draws_for_another_power_3_creature() {
    let mut g = table();
    put(&mut g, 0, "Garruk's Packleader");
    let n = g.players[0].hand.len();
    let small = hand(&mut g, 0, "Morbid Opportunist"); // 1/3
    play(&mut g, small, false, &[], &[]);
    assert_eq!(g.players[0].hand.len(), n, "power 1: no trigger");
    let big = hand(&mut g, 0, "Garruk's Packleader"); // 4/4: only the first one triggers
    play(&mut g, big, false, &[], &[Action::Yes]);
    assert_eq!(g.players[0].hand.len(), n + 1);
}

#[test]
fn grim_backwoods_taps_for_colorless_and_sacrifices_a_creature_to_draw() {
    let mut g = table();
    let land = put(&mut g, 0, "Grim Backwoods");
    assert_eq!(g.mana_sources(0).iter().filter(|&&(x, _)| x == land).count(), 1);
    let victim = put(&mut g, 0, "Morbid Opportunist");
    let n = g.players[0].hand.len();
    act(&mut g, Action::ActivateWith(land, 0, victim));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(victim).zone, Zone::Graveyard);
    assert!(g.obj(land).tapped);
    assert_eq!(g.players[0].hand.len(), n + 1);
}

#[test]
fn talisman_of_impulse_pings_only_for_red_or_green() {
    // {1}{G} from a Plains and the Talisman: the Talisman makes {G}, 1 damage.
    let mut g = with_lands(&["Plains", "Talisman of Impulse"]);
    let life = g.players[0].life;
    let elder = hand(&mut g, 0, "Sakura-Tribe Elder");
    play(&mut g, elder, false, &[], &[]);
    assert_eq!(g.obj(elder).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, life - 1);
    // {2}: the Talisman makes {C}, no damage.
    let mut g = with_lands(&["Plains", "Talisman of Impulse"]);
    let life = g.players[0].life;
    let signet = hand(&mut g, 0, "Arcane Signet");
    play(&mut g, signet, false, &[], &[]);
    assert_eq!(g.obj(signet).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, life);
}

#[test]
fn thriving_bluff_enters_tapped_and_makes_red_or_the_chosen_color() {
    let mut g = table();
    let bluff = hand(&mut g, 0, "Thriving Bluff");
    act(&mut g, Action::PlayLand(bluff));
    assert!(matches!(g.pending, Pending::Decide(0)));
    assert!(
        !g.legal_actions().contains(&Action::Mode(3)),
        "a color other than red"
    );
    act(&mut g, Action::Mode(0)); // white
    assert!(g.obj(bluff).tapped);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let any = g
        .mana_sources(0)
        .into_iter()
        .filter(|&(x, _)| x == bluff)
        .fold([false; 5], |acc, (_, m)| {
            std::array::from_fn(|i| acc[i] || m.colors[i])
        });
    assert_eq!(any, [true, false, false, true, false], "red or white");
}

#[test]
fn spine_of_ish_sah_destroys_a_permanent_and_returns_to_hand_from_the_graveyard() {
    let mut g = table();
    let land = g.permanents(1).find(|&x| g.types_of(x).land).unwrap();
    let spine = hand(&mut g, 0, "Spine of Ish Sah");
    play(&mut g, spine, false, &[], &[Action::Target(obj(land))]);
    assert_eq!(g.obj(land).zone, Zone::Graveyard, "any permanent, lands too");
    let dispute = hand(&mut g, 0, "Deadly Dispute");
    act(&mut g, Action::CastWith(dispute, spine));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(spine).zone, Zone::Hand, "back to its owner's hand");
}

#[test]
fn windgraces_judgment_destroys_a_nonland_permanent_an_opponent_controls() {
    let mut g = table();
    let theirs = put(&mut g, 1, "Arcane Signet");
    let mine = put(&mut g, 0, "Arcane Signet");
    let c = hand(&mut g, 0, "Windgrace's Judgment");
    act(&mut g, Action::Cast(c));
    assert!(!g.legal_actions().contains(&Action::Target(obj(mine))));
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.obj(mine).zone, Zone::Battlefield);
}

#[test]
fn psychosis_crawler_is_hand_sized_and_drains_on_each_draw() {
    let mut g = table();
    let c = put(&mut g, 0, "Psychosis Crawler");
    let n = g.players[0].hand.len() as i32;
    assert_eq!((g.power(c), g.toughness(c)), (n, n));
    let life = g.players[1].life;
    let signet = put(&mut g, 0, "Arcane Signet");
    let dispute = hand(&mut g, 0, "Deadly Dispute");
    act(&mut g, Action::CastWith(dispute, signet));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 2, "two draws");
    assert_eq!(g.power(c), n + 2, "two cards came");
}

#[test]
fn deadly_dispute_sacrifices_an_artifact_or_creature_to_draw_two_and_make_a_treasure() {
    let mut g = table();
    let c = hand(&mut g, 0, "Deadly Dispute");
    assert!(
        !g.legal_actions().contains(&Action::Cast(c)),
        "the sacrifice is required"
    );
    let signet = put(&mut g, 0, "Arcane Signet");
    let creature = put(&mut g, 0, "Morbid Opportunist");
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::CastWith(c, signet)) && legal.contains(&Action::CastWith(c, creature)));
    let n = g.players[0].hand.len();
    act(&mut g, Action::CastWith(c, signet));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(signet).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n - 1 + 2);
    assert_eq!(
        g.permanents(0)
            .filter(|&x| g.def_of(x).name == "Treasure")
            .count(),
        1
    );
}

#[test]
fn martial_coup_makes_x_soldiers_and_at_five_destroys_everything_else() {
    let mut g = table();
    let theirs = put(&mut g, 1, "Morbid Opportunist");
    let c = hand(&mut g, 0, "Martial Coup");
    act(&mut g, Action::CastX(c, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Soldier").len(), 2);
    assert_eq!(g.obj(theirs).zone, Zone::Battlefield, "X below 5");
    g.start_turn(5, 0); // untap: the Plains pay {W}{W} again
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let c2 = hand(&mut g, 0, "Martial Coup");
    act(&mut g, Action::CastX(c2, 5));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(
        tokens(&g, 0, "Soldier").len(),
        5,
        "the new five survive, the old two don't"
    );
}

#[test]
fn intellectual_offering_draws_three_each_and_untaps_nonland_permanents() {
    let mut g = table();
    let me = put(&mut g, 0, "Morbid Opportunist");
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(me));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(g.obj(me).tapped);
    let (mine, theirs) = (g.players[0].hand.len(), g.players[1].hand.len());
    let c = hand(&mut g, 0, "Intellectual Offering");
    play(&mut g, c, false, &[], &[]);
    assert!(!g.obj(me).tapped, "nonland permanents untap");
    assert!(
        g.permanents(0).any(|x| g.types_of(x).land && g.obj(x).tapped),
        "lands don't"
    );
    assert_eq!(
        (g.players[0].hand.len(), g.players[1].hand.len()),
        (mine + 3, theirs + 3)
    );
}

#[test]
fn morbid_opportunist_draws_once_each_turn_when_other_creatures_die() {
    let mut g = table();
    put(&mut g, 0, "Morbid Opportunist");
    let coup = hand(&mut g, 0, "Martial Coup");
    act(&mut g, Action::CastX(coup, 4));
    resolve_all(&mut g, &[]);
    let n = g.players[0].hand.len();
    let storm = hand(&mut g, 0, "Starstorm");
    act(&mut g, Action::CastX(storm, 1)); // four 1/1 Soldiers die at once
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].hand.len(), n + 1);
    let theirs = put(&mut g, 1, "Sakura-Tribe Elder");
    let storm2 = hand(&mut g, 0, "Starstorm");
    act(&mut g, Action::CastX(storm2, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n + 1, "only once each turn");
}
