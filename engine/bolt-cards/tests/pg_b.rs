//! Playgroup decks, lane B: each script's behavior in a 1v1 Commander game on the CMD pool.
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

#[test]
fn the_cmd_pool_loads() {
    let g = table();
    assert_eq!(g.players[0].life, 40);
}

// ---- Xenagos, batch 1 ----

/// Attack with `attackers` on turn 5; stops after attackers are declared.
fn attack(g: &mut Game, attackers: &[ObjId]) {
    g.start_turn(5, 0);
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    for &a in attackers {
        act(g, Action::Attack(a));
    }
    act(g, Action::FinishAttacks);
}

fn colors_of(g: &Game, id: ObjId) -> Vec<[bool; 5]> {
    g.mana_sources(0)
        .into_iter()
        .filter(|&(x, _)| x == id)
        .map(|(_, m)| m.colors)
        .collect()
}

#[test]
fn mana_creatures_and_lotus_petal() {
    let mut g = game(&[]);
    let birds = put(&mut g, 0, "Birds of Paradise");
    let mystic = put(&mut g, 0, "Elvish Mystic");
    let petal = put(&mut g, 0, "Lotus Petal");
    assert!(g.keywords(birds).has(Keywords::FLYING));
    assert_eq!(colors_of(&g, birds), [[true; 5]]);
    assert_eq!(colors_of(&g, mystic), [[false, false, false, false, true]]);
    assert_eq!(colors_of(&g, petal), [[true; 5]]);
    let ring = hand(&mut g, 0, "Sol Ring"); // {1}
    let mut g2 = game(&[]);
    let petal2 = put(&mut g2, 0, "Lotus Petal");
    let ring2 = hand(&mut g2, 0, "Sol Ring");
    play(&mut g2, ring2, false, &[], &[]);
    assert_eq!(g2.obj(ring2).zone, Zone::Battlefield);
    assert_eq!(g2.obj(petal2).zone, Zone::Graveyard, "sacrificed for the mana");
    let _ = ring;
}

#[test]
fn forest_searches_put_forests_onto_the_battlefield_untapped() {
    for name in ["Nature's Lore", "Three Visits"] {
        let mut g = table();
        let f = g.add_card(0, cid("Forest"), Zone::Library);
        let c = hand(&mut g, 0, name);
        play(&mut g, c, false, &[], &[Action::Pick(f)]);
        assert_eq!(g.obj(f).zone, Zone::Battlefield, "{name}");
        assert!(!g.obj(f).tapped, "{name}");
    }
    let mut g = table();
    let a = g.add_card(0, cid("Forest"), Zone::Library);
    let b = g.add_card(0, cid("Forest"), Zone::Library);
    let c = hand(&mut g, 0, "Skyshroud Claim");
    play(&mut g, c, false, &[], &[Action::Pick(a), Action::Pick(b)]);
    assert!(g.obj(a).zone == Zone::Battlefield && g.obj(b).zone == Zone::Battlefield);
    assert!(!g.obj(a).tapped && !g.obj(b).tapped);
}

#[test]
fn kodamas_reach_and_wooded_foothills_fetch() {
    let mut g = table();
    let a = *g.players[0].library.last().unwrap();
    let b = g.players[0].library[g.players[0].library.len() - 2];
    let r = hand(&mut g, 0, "Kodama's Reach");
    play(&mut g, r, false, &[], &[Action::Pick(a), Action::Pick(b)]);
    assert_eq!(g.obj(a).zone, Zone::Battlefield);
    assert!(g.obj(a).tapped);
    assert_eq!(g.obj(b).zone, Zone::Hand);

    let mut g = table();
    let w = put(&mut g, 0, "Wooded Foothills");
    let m = g.add_card(0, cid("Mountain"), Zone::Library);
    act(&mut g, Action::Activate(w, 0));
    resolve_all(&mut g, &[Action::Pick(m)]);
    assert_eq!(g.obj(m).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, 39);
}

#[test]
fn aggravated_assault_and_seize_the_day_give_extra_combats() {
    for name in ["Aggravated Assault", "Seize the Day"] {
        let mut g = table();
        let valet = put(&mut g, 0, "Devilish Valet"); // 1/3 haste
        let life = g.players[1].life;
        attack(&mut g, &[valet]);
        while g.step != Step::Main2 {
            act(&mut g, Action::PassPriority);
        }
        assert!(g.obj(valet).tapped);
        if name == "Aggravated Assault" {
            let a = put(&mut g, 0, name);
            act(&mut g, Action::Activate(a, 0));
            resolve_all(&mut g, &[]);
        } else {
            let s = hand(&mut g, 0, name);
            play(&mut g, s, false, &[obj(valet)], &[]);
        }
        assert!(!g.obj(valet).tapped, "{name}: untapped");
        while g.pending != Pending::DeclareAttackers(0) {
            act(&mut g, Action::PassPriority);
        }
        act(&mut g, Action::Attack(valet));
        act(&mut g, Action::FinishAttacks);
        while g.step != Step::Main2 && g.step != Step::Main1 {
            act(&mut g, Action::PassPriority);
        }
        assert_eq!(g.players[1].life, life - 2, "{name}: a second combat");
    }
}

#[test]
fn grim_monolith_makes_three_and_untaps_for_four() {
    let mut g = game(&[]);
    let m = put(&mut g, 0, "Grim Monolith");
    assert_eq!(g.mana_sources(0).iter().filter(|&&(x, _)| x == m).count(), 3);
    let ring = hand(&mut g, 0, "Sol Ring");
    play(&mut g, ring, false, &[], &[]);
    assert!(g.obj(m).tapped);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert!(g.obj(m).tapped, "doesn't untap");
    for _ in 0..4 {
        put(&mut g, 0, "Mountain");
    }
    act(&mut g, Action::Activate(m, 0));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(m).tapped);
}

#[test]
fn hull_breach_destroys_an_artifact_and_or_an_enchantment() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let ench = put(&mut g, 1, "Fiery Emancipation");
    let h = hand(&mut g, 0, "Hull Breach");
    act(&mut g, Action::CastModal(h, 2));
    act(&mut g, Action::Target(obj(ring)));
    act(&mut g, Action::Target(obj(ench)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    assert_eq!(g.obj(ench).zone, Zone::Graveyard);
}

#[test]
fn heroic_intervention_protects_all_your_permanents() {
    let mut g = table();
    let ring = put(&mut g, 0, "Sol Ring");
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    let h = hand(&mut g, 0, "Heroic Intervention");
    play(&mut g, h, false, &[], &[]);
    for x in [ring, elder] {
        let k = g.keywords(x);
        assert!(k.has(Keywords::HEXPROOF) && k.has(Keywords::INDESTRUCTIBLE));
    }
    let act_ = hand(&mut g, 0, "Blasphemous Act");
    play(&mut g, act_, false, &[], &[]);
    assert_eq!(g.obj(elder).zone, Zone::Battlefield, "indestructible");
}

#[test]
fn fire_lit_thicket_filters_and_ancient_tomb_hurts() {
    let mut g = game(&[]);
    let t = put(&mut g, 0, "Fire-Lit Thicket");
    put(&mut g, 0, "Mountain");
    assert!(g.mana_sources(0).iter().any(|&(x, m)| x == t && m.colorless));
    let s = hand(&mut g, 0, "Siege-Gang Commander"); // {3}{R}{R}: 2 from the Thicket... not enough
    assert!(!g.legal_actions().contains(&Action::Cast(s)));
    let cobra = hand(&mut g, 0, "Lotus Cobra"); // {1}{G}: Mountain pays {R/G}, Thicket makes {G}{G}
    play(&mut g, cobra, false, &[], &[]);
    assert_eq!(g.obj(cobra).zone, Zone::Battlefield);

    let mut g = game(&[]);
    let tomb = put(&mut g, 0, "Ancient Tomb");
    assert_eq!(g.mana_sources(0).iter().filter(|&&(x, _)| x == tomb).count(), 2);
    let ring = hand(&mut g, 0, "Sol Ring");
    play(&mut g, ring, false, &[], &[]);
    assert_eq!(g.players[0].life, 38);
}

#[test]
fn fiery_emancipation_triples_and_goblin_anarchomancer_discounts() {
    let mut g = table();
    put(&mut g, 0, "Fiery Emancipation");
    let sg = put(&mut g, 0, "Siege-Gang Commander");
    act(&mut g, Action::ActivateWith(sg, 0, sg));
    resolve_all(&mut g, &[Action::Target(Target::Player(1))]);
    assert_eq!(g.players[1].life, 34, "2 tripled");

    let mut g = game(&["Forest", "Mountain"]);
    put(&mut g, 0, "Goblin Anarchomancer");
    let cobra = hand(&mut g, 0, "Lotus Cobra"); // {1}{G} green: {G}
    let ring = hand(&mut g, 0, "Sol Ring"); // colorless: no discount
    play(&mut g, cobra, false, &[], &[]);
    assert_eq!(g.obj(cobra).zone, Zone::Battlefield);
    let untapped = g
        .permanents(0)
        .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
        .count();
    assert_eq!(untapped, 1, "paid one land");
    let _ = ring;
}

#[test]
fn elder_gargaroth_chooses_on_attack_and_block() {
    let mut g = table();
    let e = put(&mut g, 0, "Elder Gargaroth");
    let k = g.keywords(e);
    assert!(k.has(Keywords::REACH) && k.has(Keywords::VIGILANCE) && k.has(Keywords::TRAMPLE));
    attack(&mut g, &[e]);
    resolve_all(&mut g, &[Action::Mode(0)]);
    assert_eq!(tokens(&g, 0, "Beast").len(), 1);
    // Blocking: gain 3.
    let attacker = put(&mut g, 1, "Devilish Valet");
    g.start_turn(6, 1);
    while g.pending != Pending::DeclareAttackers(1) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(attacker));
    act(&mut g, Action::FinishAttacks);
    while g.pending != Pending::DeclareBlockers(0) {
        act(&mut g, Action::PassPriority);
    }
    let life = g.players[0].life;
    act(&mut g, Action::Block { blocker: e, attacker });
    act(&mut g, Action::FinishBlocks);
    resolve_all(&mut g, &[Action::Mode(1)]);
    assert_eq!(g.players[0].life, life + 3);
}

#[test]
fn fierce_empath_finds_a_big_creature() {
    let mut g = table();
    let big = g.add_card(0, cid("Elder Gargaroth"), Zone::Library); // mana value 5: too small
    let bigger = g.add_card(0, cid("Inferno Titan"), Zone::Library); // 6
    let e = hand(&mut g, 0, "Fierce Empath");
    act(&mut g, Action::Cast(e));
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Yes);
    assert!(!g.legal_actions().contains(&Action::Pick(big)));
    resolve_all(&mut g, &[Action::Pick(bigger)]);
    assert_eq!(g.obj(bigger).zone, Zone::Hand);
}

// ---- Xenagos, batch 2 ----

/// Passes to main phase 2 answering targets and decisions in order.
fn to_main2(g: &mut Game, answers: &[Action]) {
    let mut answers = answers.iter();
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::ChooseTarget(_) | Pending::Decide(_) => *answers.next().expect("an answer"),
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn thornspire_verge_reflecting_pool_and_stomping_ground() {
    let mut g = game(&[]);
    let v = put(&mut g, 0, "Thornspire Verge");
    assert_eq!(colors_of(&g, v), [[false, false, false, true, false]]);
    put(&mut g, 0, "Forest");
    assert_eq!(colors_of(&g, v), [[false, false, false, true, true]]);
    let pool = put(&mut g, 0, "Reflecting Pool");
    assert_eq!(
        colors_of(&g, pool),
        [[false, false, false, true, true]],
        "what your lands make"
    );

    let mut g = game(&[]);
    let s = hand(&mut g, 0, "Stomping Ground");
    act(&mut g, Action::PlayLand(s));
    act(&mut g, Action::Yes);
    assert!(!g.obj(s).tapped);
    assert_eq!(g.players[0].life, 38);
    assert_eq!(colors_of(&g, s), [[false, false, false, true, true]]);
    let mut g = game(&[]);
    let s = hand(&mut g, 0, "Stomping Ground");
    act(&mut g, Action::PlayLand(s));
    act(&mut g, Action::No);
    assert!(g.obj(s).tapped);
    assert_eq!(g.players[0].life, 40);
}

#[test]
fn kessig_wolf_run_pumps_by_x_with_trample() {
    let mut g = table();
    let run = put(&mut g, 0, "Kessig Wolf Run");
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    act(
        &mut g,
        Action::ActivateX {
            id: run,
            n: 0,
            victim: None,
            x: 3,
        },
    );
    resolve_all(&mut g, &[Action::Target(obj(elder))]);
    assert_eq!((g.power(elder), g.toughness(elder)), (4, 1));
    assert!(g.keywords(elder).has(Keywords::TRAMPLE));
}

#[test]
fn xenagos_is_a_creature_with_devotion_and_doubles_an_attacker() {
    let mut g = table();
    let x = put(&mut g, 0, "Xenagos, God of Revels"); // {3}{R}{G}: devotion 2
    assert!(!g.types_of(x).creature);
    assert!(g.keywords(x).has(Keywords::INDESTRUCTIBLE));
    let mut g = table();
    put(&mut g, 0, "Xenagos, God of Revels");
    let valet = put(&mut g, 0, "Devilish Valet"); // 1/3
    g.start_turn(5, 0);
    let mut targeted = false;
    while g.pending != Pending::DeclareAttackers(0) {
        let a = if matches!(g.pending, Pending::ChooseTarget(0)) {
            targeted = true;
            Action::Target(obj(valet))
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
    }
    assert!(targeted);
    assert_eq!((g.power(valet), g.toughness(valet)), (2, 4), "+1/+1, X its power");
    // Devotion 7: Xenagos (RG), Questing Beast (GG), Zilortha (RG), plus Kogla (GGG).
    let mut g = table();
    let x = put(&mut g, 0, "Xenagos, God of Revels");
    put(&mut g, 0, "Questing Beast");
    put(&mut g, 0, "Zilortha, Strength Incarnate");
    assert!(!g.types_of(x).creature, "six");
    put(&mut g, 0, "Kogla, the Titan Ape");
    assert!(g.types_of(x).creature, "nine");
    assert_eq!(g.devotion(0, [false, false, false, true, true]), 9);
}

#[test]
fn the_nyleas() {
    let mut g = table();
    let n = put(&mut g, 0, "Nylea, God of the Hunt");
    assert!(!g.types_of(n).creature);
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    assert!(g.keywords(elder).has(Keywords::TRAMPLE));
    act(&mut g, Action::Activate(n, 0));
    resolve_all(&mut g, &[Action::Target(obj(elder))]);
    assert_eq!(g.power(elder), 3);

    let mut g = game(&["Forest", "Forest", "Forest"]);
    let k = put(&mut g, 0, "Nylea, Keen-Eyed");
    let top = g.add_card(0, cid("Sakura-Tribe Elder"), Zone::Library);
    act(&mut g, Action::Activate(k, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(top).zone, Zone::Hand, "a creature: to hand");
    let mut g = game(&["Forest", "Forest", "Forest"]);
    let k = put(&mut g, 0, "Nylea, Keen-Eyed");
    let top = g.add_card(0, cid("Sol Ring"), Zone::Library);
    act(&mut g, Action::Activate(k, 0));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.obj(top).zone, Zone::Graveyard, "not a creature: may mill it");
    // Creature spells cost {1} less: Lotus Cobra with one Forest.
    let mut g = game(&["Forest"]);
    put(&mut g, 0, "Nylea, Keen-Eyed");
    let cobra = hand(&mut g, 0, "Lotus Cobra");
    assert!(g.legal_actions().contains(&Action::Cast(cobra)));
}

#[test]
fn old_gnawbone_and_balefire_dragon_cash_in_combat_damage() {
    let mut g = table();
    put(&mut g, 0, "Old Gnawbone");
    let valet = put(&mut g, 0, "Devilish Valet");
    let dragon = put(&mut g, 0, "Balefire Dragon");
    let theirs = put(&mut g, 1, "Inferno Titan"); // 6/6
    attack(&mut g, &[valet, dragon]);
    to_main2(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1 + 6);
    assert_eq!(
        g.obj(theirs).zone,
        Zone::Graveyard,
        "6 damage to each of their creatures"
    );
}

#[test]
fn dragonborn_champion_and_deus_of_calamity_reward_big_hits() {
    let mut g = table();
    put(&mut g, 0, "Dragonborn Champion");
    let deus = put(&mut g, 0, "Deus of Calamity");
    let land = g.permanents(1).find(|&x| g.types_of(x).land).unwrap();
    attack(&mut g, &[deus]);
    let cards = g.players[0].hand.len();
    to_main2(&mut g, &[Action::Target(obj(land))]);
    assert_eq!(g.players[0].hand.len(), cards + 1, "6 damage: draw");
    assert_eq!(g.obj(land).zone, Zone::Graveyard);
}

#[test]
fn kogla_fights_destroys_and_bounces_humans() {
    let mut g = table();
    let elder = put(&mut g, 1, "Sakura-Tribe Elder");
    let k = hand(&mut g, 0, "Kogla, the Titan Ape");
    play(&mut g, k, false, &[], &[Action::Target(obj(elder))]);
    assert_eq!(g.obj(elder).zone, Zone::Graveyard);
    let ring = put(&mut g, 1, "Sol Ring");
    attack(&mut g, &[k]);
    resolve_all(&mut g, &[Action::Target(obj(ring))]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);

    let mut g = table();
    let k = put(&mut g, 0, "Kogla, the Titan Ape");
    let human = put(&mut g, 0, "Thopter Engineer"); // a Human
    act(&mut g, Action::Activate(k, 0));
    resolve_all(&mut g, &[Action::Target(obj(human))]);
    assert_eq!(g.obj(human).zone, Zone::Hand);
    assert!(g.keywords(k).has(Keywords::INDESTRUCTIBLE));
}

#[test]
fn kogla_and_yidaro_gets_haste_or_fights_and_cycles_into_removal() {
    let mut g = table();
    let k = hand(&mut g, 0, "Kogla and Yidaro");
    play(&mut g, k, false, &[], &[Action::Mode(0)]);
    let kw = g.keywords(k);
    assert!(kw.has(Keywords::TRAMPLE) && kw.has(Keywords::HASTE));

    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let k = hand(&mut g, 0, "Kogla and Yidaro");
    let cards = g.players[0].hand.len();
    act(&mut g, Action::Activate(k, 0));
    resolve_all(&mut g, &[Action::Target(obj(ring))]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    assert_eq!(g.obj(k).zone, Zone::Library, "shuffled in");
    assert_eq!(g.players[0].hand.len(), cards, "discarded it, drew one");
}

#[test]
fn malignus_is_half_their_life_and_ignores_fog() {
    let mut g = table();
    let m = put(&mut g, 0, "Malignus");
    assert_eq!((g.power(m), g.toughness(m)), (20, 20));
    g.players[1].life = 7;
    assert_eq!(g.power(m), 4, "rounded up");
    let frog = put(&mut g, 1, "Spore Frog");
    attack(&mut g, &[m]);
    // They fog: Malignus's damage can't be prevented.
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Activate(frog, 0));
    to_main2(&mut g, &[]);
    assert_eq!(g.obj(frog).zone, Zone::Graveyard);
    assert_eq!(g.players[1].life, 3, "7 - 4");
}

#[test]
fn questing_beast_dodges_small_blockers() {
    let mut g = table();
    let q = put(&mut g, 0, "Questing Beast");
    let k = g.keywords(q);
    assert!(k.has(Keywords::VIGILANCE) && k.has(Keywords::DEATHTOUCH) && k.has(Keywords::HASTE));
    let small = put(&mut g, 1, "Sakura-Tribe Elder");
    let big = put(&mut g, 1, "Inferno Titan");
    attack(&mut g, &[q]);
    while g.pending != Pending::DeclareBlockers(1) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Block {
        blocker: small,
        attacker: q
    }));
    assert!(legal.contains(&Action::Block {
        blocker: big,
        attacker: q
    }));
}

#[test]
fn the_great_henge_costs_less_taps_for_gg_and_grows_new_creatures() {
    let mut g = game(&["Forest", "Forest"]);
    put(&mut g, 0, "Inferno Titan"); // power 6
    put(&mut g, 0, "Sol Ring");
    let h = hand(&mut g, 0, "The Great Henge");
    play(&mut g, h, false, &[], &[]);
    assert_eq!(g.obj(h).zone, Zone::Battlefield, "{{7}}{{G}}{{G}} less 6");
    // Its mana: {G}{G} and 2 life, the only source here.
    let mut g = game(&[]);
    put(&mut g, 0, "The Great Henge");
    let life = g.players[0].life;
    let elder = hand(&mut g, 0, "Sakura-Tribe Elder");
    let cards = g.players[0].hand.len();
    play(&mut g, elder, false, &[], &[]);
    assert_eq!(g.obj(elder).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, life + 2);
    assert_eq!(g.obj(elder).counters, 1);
    assert_eq!(g.players[0].hand.len(), cards - 1 + 1);
}

#[test]
fn zilortha_uses_power_for_lethal_damage() {
    let mut g = table();
    put(&mut g, 0, "Zilortha, Strength Incarnate");
    let valet = put(&mut g, 0, "Devilish Valet"); // 1/3
    let sg = put(&mut g, 1, "Siege-Gang Commander");
    their_turn(&mut g);
    act(&mut g, Action::ActivateWith(sg, 0, sg));
    resolve_all(&mut g, &[Action::Target(obj(valet))]);
    assert_eq!(g.obj(valet).zone, Zone::Graveyard, "2 damage >= power 1");
}

#[test]
fn vandalblast_one_or_overloaded() {
    let mut g = table();
    let mine = put(&mut g, 0, "Mind Stone");
    let theirs = put(&mut g, 1, "Sol Ring");
    let v = hand(&mut g, 0, "Vandalblast");
    act(&mut g, Action::Cast(v));
    assert!(
        !g.legal_actions().contains(&Action::Target(obj(mine))),
        "you don't control"
    );
    act(&mut g, Action::Target(obj(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);

    let mut g = table();
    let a = put(&mut g, 1, "Sol Ring");
    let b = put(&mut g, 1, "Thran Dynamo");
    let mine = put(&mut g, 0, "Mind Stone");
    let v = hand(&mut g, 0, "Vandalblast");
    play(&mut g, v, true, &[], &[]);
    assert_eq!(g.obj(a).zone, Zone::Graveyard);
    assert_eq!(g.obj(b).zone, Zone::Graveyard);
    assert_eq!(g.obj(mine).zone, Zone::Battlefield, "you don't control");
}

fn their_turn(g: &mut Game) {
    g.start_turn(4, 1);
    while g.step != Step::Main1 {
        act(g, Action::PassPriority);
    }
}

// ---- Xenagos, batch 3 ----

/// Passes until the next declare-attackers step of player 0, answering decisions with `No`.
fn to_attackers(g: &mut Game) {
    while g.pending != Pending::DeclareAttackers(0) {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            Pending::Decide(_) => Action::No,
            _ => Action::PassPriority,
        };
        act(g, a);
    }
}

#[test]
fn bloodthirster_untaps_for_a_second_combat_but_attacks_once() {
    let mut g = table();
    let b = put(&mut g, 0, "Bloodthirster");
    let valet = put(&mut g, 0, "Devilish Valet");
    attack(&mut g, &[b]);
    to_attackers(&mut g);
    assert!(!g.obj(b).tapped, "untapped");
    assert_eq!(g.players[1].life, 34);
    let legal = g.legal_actions();
    assert!(
        !legal.contains(&Action::Attack(b)),
        "it already attacked that player"
    );
    assert!(legal.contains(&Action::Attack(valet)), "an additional combat");
}

#[test]
fn karlach_untaps_attackers_with_first_strike_for_another_combat() {
    let mut g = table();
    let k = put(&mut g, 0, "Karlach, Fury of Avernus");
    let valet = put(&mut g, 0, "Devilish Valet");
    attack(&mut g, &[k, valet]);
    resolve_all(&mut g, &[]);
    assert!(!g.obj(k).tapped && !g.obj(valet).tapped);
    assert!(g.keywords(valet).has(Keywords::FIRST_STRIKE));
    to_attackers(&mut g);
    act(&mut g, Action::Attack(k));
    act(&mut g, Action::FinishAttacks);
    assert!(
        g.items.is_empty(),
        "the second combat isn't the first: no trigger"
    );
}

#[test]
fn combat_celebrant_exerts_for_an_extra_combat() {
    let mut g = table();
    let c = put(&mut g, 0, "Combat Celebrant");
    let valet = put(&mut g, 0, "Devilish Valet");
    attack(&mut g, &[c, valet]);
    resolve_all(&mut g, &[Action::Yes]);
    assert!(!g.obj(valet).tapped, "the others untap");
    assert!(g.obj(c).tapped);
    to_attackers(&mut g);
    assert!(g.legal_actions().contains(&Action::Attack(valet)));
    // Exerted: it doesn't untap next turn.
    g.start_turn(7, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert!(g.obj(c).tapped);
}

#[test]
fn hellkite_tyrant_steals_artifacts() {
    let mut g = table();
    let h = put(&mut g, 0, "Hellkite Tyrant");
    let ring = put(&mut g, 1, "Sol Ring");
    attack(&mut g, &[h]);
    to_main2(&mut g, &[]);
    assert_eq!(g.obj(ring).controller, 0);
    // Twenty artifacts at upkeep: you win.
    for _ in 0..19 {
        put(&mut g, 0, "Sol Ring");
    }
    g.start_turn(7, 0);
    while g.result.is_none() && g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.result, Some(bolt_core::Outcome::Winner(0)));
}

#[test]
fn yavimaya_makes_every_land_a_forest() {
    let mut g = game(&[]);
    put(&mut g, 0, "Yavimaya, Cradle of Growth");
    let plains = put(&mut g, 0, "Plains");
    let theirs = g.permanents(1).find(|&x| g.def_of(x).name == "Island").unwrap();
    assert!(g.has_subtype(plains, "Forest") && g.has_subtype(theirs, "Forest"));
    assert_eq!(colors_of(&g, plains), [[true, false, false, false, true]]);
}

#[test]
fn quartzwood_crasher_makes_a_dinosaur_the_size_of_the_trample_damage() {
    let mut g = table();
    let q = put(&mut g, 0, "Quartzwood Crasher"); // 6/6 trample
    let valet = put(&mut g, 0, "Devilish Valet"); // trample too
    attack(&mut g, &[q, valet]);
    to_main2(&mut g, &[]);
    let d = tokens(&g, 0, "Dinosaur Beast");
    assert_eq!(d.len(), 1);
    assert_eq!(g.power(d[0]), 7);
    assert!(g.keywords(d[0]).has(Keywords::TRAMPLE));
}

#[test]
fn ilharg_drops_a_creature_into_the_attack_and_takes_it_back() {
    let mut g = table();
    let i = put(&mut g, 0, "Ilharg, the Raze-Boar");
    let big = hand(&mut g, 0, "Quartzwood Crasher"); // 6/6, no enters trigger
    attack(&mut g, &[i]);
    resolve_all(&mut g, &[Action::Pick(big)]);
    assert_eq!(g.obj(big).zone, Zone::Battlefield);
    assert!(g.obj(big).tapped);
    let life = g.players[1].life;
    to_main2(&mut g, &[]);
    assert_eq!(g.players[1].life, life - 6 - 6, "both attacked");
    while g.step != Step::End {
        act(&mut g, Action::PassPriority);
    }
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(big).zone, Zone::Hand, "back at the end step");

    // Dies: third from the top.
    let mut g = table();
    let i = put(&mut g, 0, "Ilharg, the Raze-Boar");
    let gift = hand(&mut g, 0, "Beast Within");
    play(&mut g, gift, false, &[obj(i)], &[Action::Yes]);
    let lib = &g.players[0].library;
    assert_eq!(lib[lib.len() - 3], i);
}

#[test]
fn chaos_imps_unleashed_tramples_but_cant_block() {
    let mut g = table();
    let i = hand(&mut g, 0, "Chaos Imps");
    play(&mut g, i, false, &[], &[Action::Yes]);
    assert_eq!(g.obj(i).counters, 1);
    let k = g.keywords(i);
    assert!(k.has(Keywords::FLYING) && k.has(Keywords::TRAMPLE) && k.has(Keywords::CANT_BLOCK));

    let mut g = table();
    let i = hand(&mut g, 0, "Chaos Imps");
    play(&mut g, i, false, &[], &[Action::No]);
    assert_eq!(g.obj(i).counters, 0);
    let k = g.keywords(i);
    assert!(!k.has(Keywords::TRAMPLE) && !k.has(Keywords::CANT_BLOCK));
}

#[test]
fn silvos_and_yavimaya_hollow_regenerate() {
    let mut g = table();
    let s = put(&mut g, 0, "Silvos, Rogue Elemental");
    act(&mut g, Action::Activate(s, 0));
    resolve_all(&mut g, &[]);
    let gift = hand(&mut g, 0, "Beast Within");
    play(&mut g, gift, false, &[obj(s)], &[]);
    assert_eq!(g.obj(s).zone, Zone::Battlefield, "regenerated");
    assert!(g.obj(s).tapped);
    // "It can't be regenerated."
    act(&mut g, Action::Activate(s, 0));
    resolve_all(&mut g, &[]);
    let p = hand(&mut g, 0, "Putrefy");
    play(&mut g, p, false, &[obj(s)], &[]);
    assert_eq!(g.obj(s).zone, Zone::Graveyard);

    // Lethal damage is regenerated too.
    let mut g = table();
    let h = put(&mut g, 0, "Yavimaya Hollow");
    let elder = put(&mut g, 0, "Sakura-Tribe Elder");
    act(&mut g, Action::Activate(h, 0));
    resolve_all(&mut g, &[Action::Target(obj(elder))]);
    let sg = put(&mut g, 0, "Siege-Gang Commander");
    act(&mut g, Action::ActivateWith(sg, 0, sg));
    resolve_all(&mut g, &[Action::Target(obj(elder))]);
    assert_eq!(g.obj(elder).zone, Zone::Battlefield);
    assert_eq!(g.obj(elder).damage, 0);
}

#[test]
fn triumph_of_the_hordes_poisons() {
    let mut g = table();
    let a = put(&mut g, 0, "Quartzwood Crasher"); // 6/6 trample
    let b = put(&mut g, 0, "Devilish Valet"); // 1/3 trample haste
    let t = hand(&mut g, 0, "Triumph of the Hordes");
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    play(&mut g, t, false, &[], &[]);
    assert_eq!(g.power(a), 7);
    let life = g.players[1].life;
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(a));
    act(&mut g, Action::Attack(b));
    act(&mut g, Action::FinishAttacks);
    while g.result.is_none() && g.step != Step::Main2 {
        let x = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, x);
    }
    assert_eq!(g.players[1].life, life, "infect: no life loss");
    assert_eq!(g.players[1].poison, 7 + 2);
    assert!(g.result.is_none(), "nine poison");
}

#[test]
fn savage_beating_only_in_your_combat_and_entwined_does_both() {
    let mut g = table();
    let valet = put(&mut g, 0, "Devilish Valet");
    let sb = hand(&mut g, 0, "Savage Beating");
    assert!(
        !g.legal_actions()
            .iter()
            .any(|a| matches!(a, Action::CastModal(c, _) if *c == sb)),
        "main phase"
    );
    attack(&mut g, &[valet]);
    act(&mut g, Action::CastModal(sb, 2)); // entwined: {3}{R}{R} and {1}{R}
    resolve_all(&mut g, &[]);
    assert!(g.keywords(valet).has(Keywords::DOUBLE_STRIKE));
    assert!(!g.obj(valet).tapped);
    let untapped = g
        .permanents(0)
        .filter(|&x| g.types_of(x).land && !g.obj(x).tapped)
        .count();
    assert_eq!(
        untapped,
        25 - 7,
        "paid {{5}} + entwine {{2}}; untapping creatures leaves lands tapped"
    );
}

#[test]
fn anzrag_untaps_your_team_when_blocked_and_can_demand_a_block() {
    let mut g = table();
    let a = put(&mut g, 0, "Anzrag, the Quake-Mole");
    let valet = put(&mut g, 0, "Devilish Valet");
    let blocker = put(&mut g, 1, "Inferno Titan");
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Activate(a, 0));
    resolve_all(&mut g, &[]);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(a));
    act(&mut g, Action::Attack(valet));
    act(&mut g, Action::FinishAttacks);
    while g.pending != Pending::DeclareBlockers(1) {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::FinishBlocks),
        "Anzrag must be blocked"
    );
    act(&mut g, Action::Block { blocker, attacker: a });
    act(&mut g, Action::FinishBlocks);
    resolve_all(&mut g, &[]);
    assert!(!g.obj(a).tapped && !g.obj(valet).tapped, "untapped");
}

#[test]
fn fanatic_of_rhonas_makes_four_with_a_big_creature_and_eternalizes() {
    let mut g = game(&[]);
    let f = put(&mut g, 0, "Fanatic of Rhonas");
    assert_eq!(g.mana_sources(0).iter().filter(|&&(x, _)| x == f).count(), 1);
    put(&mut g, 0, "Questing Beast"); // power 4
    assert_eq!(g.mana_sources(0).iter().filter(|&&(x, _)| x == f).count(), 4);

    let mut g = table();
    let f = g.add_card(0, cid("Fanatic of Rhonas"), Zone::Graveyard);
    act(&mut g, Action::Activate(f, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(f).zone, Zone::Exile);
    let z = tokens(&g, 0, "Fanatic of Rhonas");
    assert_eq!(z.len(), 1);
    assert_eq!((g.power(z[0]), g.toughness(z[0])), (4, 4));
    assert!(g.has_subtype(z[0], "Zombie"));
    assert_eq!(g.colors_of(z[0]), [false, false, true, false, false]);
}
