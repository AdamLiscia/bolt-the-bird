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

#[test]
fn moraug_adds_a_combat_on_landfall_and_grows_repeat_attackers() {
    let mut g = table();
    put(&mut g, 0, "Moraug, Fury of Akoum");
    let valet = put(&mut g, 0, "Devilish Valet"); // 1/3
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let forest = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(forest));
    resolve_all(&mut g, &[]);
    let life = g.players[1].life;
    // The additional combat (right after this main phase).
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(valet));
    act(&mut g, Action::FinishAttacks);
    assert_eq!(g.power(valet), 2, "attacked once: +1/+0");
    // Then the turn's own combat, with no main phase in between (Moraug can still attack).
    while g.pending != Pending::DeclareAttackers(0) {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
        assert!(g.step != Step::Main2, "no main phase between the combats");
    }
    assert_eq!(g.players[1].life, life - 2);
    assert_eq!(g.combats_this_turn, 2);

    // Landfall outside your main phase does nothing.
    let mut g = table();
    put(&mut g, 0, "Moraug, Fury of Akoum");
    let w = put(&mut g, 0, "Wooded Foothills");
    let theirs = put(&mut g, 1, "Siege-Gang Commander");
    let _ = theirs;
    g.start_turn(4, 1);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::PassPriority); // player 0 gets priority on the opponent's turn
    if g.legal_actions().contains(&Action::Activate(w, 0)) {
        let m = g.add_card(0, cid("Mountain"), Zone::Library);
        act(&mut g, Action::Activate(w, 0));
        resolve_all(&mut g, &[Action::Pick(m)]);
        assert!(g.extra_combats.is_empty());
    }
}

#[test]
fn defiler_of_vigor_trades_life_for_green_and_grows_the_team() {
    let mut g = game(&["Forest"]);
    let d = put(&mut g, 0, "Defiler of Vigor");
    assert!(g.keywords(d).has(Keywords::TRAMPLE));
    let cobra = hand(&mut g, 0, "Lotus Cobra"); // {1}{G}: one Forest isn't enough without the life
    assert!(!g.legal_actions().contains(&Action::Cast(cobra)));
    act(&mut g, Action::CastAlternative(cobra));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(cobra).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, 38);
    assert_eq!(g.obj(d).counters, 1, "cast a green permanent spell");
    let ring = hand(&mut g, 0, "Sol Ring"); // colorless: no option
    assert!(!g.legal_actions().contains(&Action::CastAlternative(ring)));
}

#[test]
fn world_at_war_adds_a_late_combat_and_rebounds() {
    let mut g = table();
    let valet = put(&mut g, 0, "Devilish Valet");
    let w = hand(&mut g, 0, "World at War");
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    play(&mut g, w, false, &[], &[]);
    assert_eq!(g.obj(w).zone, Zone::Exile, "rebound: exiled from hand");
    let life = g.players[1].life;
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(valet));
    act(&mut g, Action::FinishAttacks);
    // Through main phase 2, then the additional combat: the Valet untaps as it begins.
    let mut seen_main2 = false;
    while g.pending != Pending::DeclareAttackers(0) {
        if g.step == Step::Main2 {
            seen_main2 = true;
        }
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(seen_main2, "after the second main phase");
    assert!(!g.obj(valet).tapped, "untapped: it attacked this turn");
    act(&mut g, Action::Attack(valet));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[1].life, life - 2);
    // Next upkeep: cast it free from exile.
    g.start_turn(7, 0);
    while !matches!(g.pending, Pending::FreeCast(0)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(g.legal_actions().contains(&Action::Cast(w)));
    act(&mut g, Action::Cast(w));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(w).zone, Zone::Graveyard, "cast from exile: no rebound");
}

#[test]
fn roaming_throne_doubles_triggers_of_the_chosen_type() {
    let mut g = table();
    let e = put(&mut g, 0, "Elder Gargaroth");
    let t = hand(&mut g, 0, "Roaming Throne");
    act(&mut g, Action::Cast(t));
    while !matches!(g.pending, Pending::Decide(0)) {
        act(&mut g, Action::PassPriority);
    }
    let options = g.creature_types_known_to(0);
    let beast = options.iter().position(|o| o == "Beast").expect("Beast is known") as u8;
    act(&mut g, Action::Mode(beast));
    resolve_all(&mut g, &[]);
    assert!(g.has_subtype(t, "Beast"), "the chosen type too");
    assert!(g.def_of(t).ward.is_some());
    attack(&mut g, &[e]);
    // Gargaroth's attack trigger, twice: two Beasts.
    resolve_all(&mut g, &[Action::Mode(0), Action::Mode(0)]);
    assert_eq!(tokens(&g, 0, "Beast").len(), 2);
}

// ---- Xenagos finish: Deflecting Swat, Ojer Kaslem, Vorinclex ----

/// Makes `name` on player 0's battlefield their commander.
fn commander_on_field(g: &mut Game, name: &str) -> ObjId {
    let c = put(g, 0, name);
    g.commanders[0] = Some(c);
    c
}

fn has(g: &Game, a: Action) -> bool {
    g.legal_actions().contains(&a)
}

/// Passes priority until a decision or target choice is pending.
fn pass_until_asked(g: &mut Game) {
    for _ in 0..50 {
        if matches!(g.pending, Pending::Decide(_) | Pending::ChooseTarget(_)) {
            return;
        }
        act(g, Action::PassPriority);
    }
    panic!("never asked: {:?}", g.pending);
}

#[test]
fn deflecting_swat_is_free_with_a_commander_and_redirects_a_spell() {
    let mut g = game(&["Mountain"]);
    let swat = hand(&mut g, 0, "Deflecting Swat");
    let bolt = hand(&mut g, 0, "Burst Lightning");
    let mine = put(&mut g, 0, "Savannah Lions");
    let theirs = put(&mut g, 1, "Llanowar Elves");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Object(mine)));
    // No commander: only the {2}{R} cost, which one Mountain can't pay.
    assert!(!has(&g, Action::CastAlternative(swat)));
    assert!(!has(&g, Action::Cast(swat)));
    commander_on_field(&mut g, "Xenagos, God of Revels");
    assert!(has(&g, Action::CastAlternative(swat)));
    act(&mut g, Action::CastAlternative(swat));
    act(&mut g, Action::Target(Target::Object(bolt)));
    pass_until_asked(&mut g);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Target(Target::Object(theirs))));
    assert!(legal.contains(&Action::Target(Target::Player(1))));
    assert!(
        !legal.contains(&Action::Target(Target::Object(mine))),
        "not its current target"
    );
    assert!(legal.contains(&Action::No));
    act(&mut g, Action::Target(Target::Object(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.obj(mine).zone, Zone::Battlefield);
    assert_eq!(g.obj(swat).zone, Zone::Graveyard);
}

#[test]
fn deflecting_swat_can_keep_the_targets_and_redirect_an_ability() {
    let mut g = game(&["Mountain", "Mountain", "Mountain"]);
    let mine = put(&mut g, 0, "Savannah Lions");
    let pinger = put(&mut g, 0, "Fanatical Firebrand");
    let theirs = put(&mut g, 1, "Llanowar Elves");
    // An ability on the stack: Fanatical Firebrand (sacrificed as it's activated) at my own Lions.
    act(&mut g, Action::Activate(pinger, 0));
    act(&mut g, Action::Target(Target::Object(mine)));
    let swat = hand(&mut g, 0, "Deflecting Swat");
    act(&mut g, Action::Cast(swat)); // {2}{R}, paid
    act(&mut g, Action::Target(Target::Object(pinger)));
    pass_until_asked(&mut g);
    assert!(g
        .legal_actions()
        .contains(&Action::Target(Target::Object(theirs))));
    act(&mut g, Action::Target(Target::Object(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard, "the ping went to the Elves");
    assert_eq!(g.obj(mine).zone, Zone::Battlefield);

    // "Keep the targets": No leaves a spell's target as it was.
    let mut g = game(&["Mountain"]);
    commander_on_field(&mut g, "Xenagos, God of Revels");
    let swat = hand(&mut g, 0, "Deflecting Swat");
    let bolt = hand(&mut g, 0, "Burst Lightning");
    let theirs = put(&mut g, 1, "Llanowar Elves");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Object(theirs)));
    act(&mut g, Action::CastAlternative(swat));
    act(&mut g, Action::Target(Target::Object(bolt)));
    pass_until_asked(&mut g);
    act(&mut g, Action::No);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
}

#[test]
fn ojer_kaslem_returns_as_its_temple_and_transforms_back() {
    let mut g = table();
    let ojer = put(&mut g, 0, "Ojer Kaslem, Deepest Growth // Temple of Cultivation");
    assert!(g.types_of(ojer).creature);
    assert!(g.keywords(ojer).has(Keywords::TRAMPLE));
    assert_eq!((g.power(ojer), g.toughness(ojer)), (6, 5));
    // It dies: back tapped and transformed, a land that taps for {G}.
    g.objects[ojer.0 as usize].damage = 5;
    act(&mut g, Action::PassPriority);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ojer).zone, Zone::Battlefield);
    assert!(g.obj(ojer).transformed && g.obj(ojer).tapped);
    assert_eq!(g.def_of(ojer).name, "Temple of Cultivation");
    assert!(g.types_of(ojer).land && !g.types_of(ojer).creature);
    assert!(g.def_of(ojer).mana.is_some());
    // {2}{G}, {T}: transform, with ten or more permanents, as a sorcery.
    g.objects[ojer.0 as usize].tapped = false;
    assert!(g.permanents(0).count() >= 10);
    assert!(has(&g, Action::Activate(ojer, 0)));
    act(&mut g, Action::Activate(ojer, 0));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(ojer).transformed);
    assert!(g.types_of(ojer).creature);
    assert_eq!(
        g.def_of(ojer).name,
        "Ojer Kaslem, Deepest Growth // Temple of Cultivation"
    );
    // Fewer than ten permanents: no transforming.
    let mut g = game(&["Forest", "Forest", "Forest"]);
    let ojer = put(&mut g, 0, "Ojer Kaslem, Deepest Growth // Temple of Cultivation");
    g.objects[ojer.0 as usize].transformed = true;
    assert!(!has(&g, Action::Activate(ojer, 0)));
}

#[test]
fn ojer_kaslem_puts_a_creature_and_a_land_from_the_revealed_cards() {
    let mut g = table();
    let ojer = put(&mut g, 0, "Ojer Kaslem, Deepest Growth // Temple of Cultivation");
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Library);
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    let elves = g.add_card(0, cid("Llanowar Elves"), Zone::Library);
    g.add_card(0, cid("Plains"), Zone::Library); // drawn on turn 5
    attack(&mut g, &[ojer]);
    pass_until_asked(&mut g);
    // The top six: picks are a creature and a land.
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(lions)) && legal.contains(&Action::Pick(forest)));
    act(&mut g, Action::Pick(elves));
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Pick(lions)), "one creature only");
    assert!(legal.contains(&Action::Pick(forest)));
    act(&mut g, Action::Pick(forest));
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
    assert_eq!(g.obj(forest).zone, Zone::Battlefield);
    assert_eq!(g.obj(lions).zone, Zone::Library);
    let lib = &g.players[0].library;
    assert!(lib[..4].contains(&lions), "the rest went to the bottom");
    assert_eq!(g.players[1].life, 34);
}

#[test]
fn vorinclex_fetches_forests_and_becomes_the_grand_evolution() {
    let mut g = table();
    let f1 = g.add_card(0, cid("Forest"), Zone::Library);
    let f2 = g.add_card(0, cid("Forest"), Zone::Library);
    let vor = hand(&mut g, 0, "Vorinclex // The Grand Evolution");
    play(&mut g, vor, false, &[], &[Action::Pick(f1), Action::Pick(f2)]);
    assert_eq!(g.obj(vor).zone, Zone::Battlefield);
    assert!(g.keywords(vor).has(Keywords::REACH) && g.keywords(vor).has(Keywords::TRAMPLE));
    assert_eq!(g.obj(f1).zone, Zone::Hand);
    assert_eq!(g.obj(f2).zone, Zone::Hand);

    // {6}{G}{G}: exile it, back as The Grand Evolution; chapter I mills ten and puts two creatures in.
    let mut g = table();
    let vor = put(&mut g, 0, "Vorinclex // The Grand Evolution");
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Library);
    let elves = g.add_card(0, cid("Llanowar Elves"), Zone::Library);
    for _ in 0..8 {
        g.add_card(0, cid("Plains"), Zone::Library);
    }
    assert!(has(&g, Action::Activate(vor, 0)));
    act(&mut g, Action::Activate(vor, 0));
    pass_until_asked(&mut g);
    assert!(g.obj(vor).transformed);
    assert_eq!(g.def_of(vor).name, "The Grand Evolution");
    assert_eq!(g.obj(vor).lore, 1);
    assert!(g.legal_actions().contains(&Action::Pick(lions)));
    act(&mut g, Action::Pick(lions));
    act(&mut g, Action::Pick(elves));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
    assert_eq!(g.players[0].graveyard.len(), 8);

    // Chapter II: seven +1/+1 counters, divided.
    g.start_turn(5, 0);
    pass_until_asked(&mut g);
    assert_eq!(g.obj(vor).lore, 2);
    act(&mut g, Action::Target(Target::Object(lions)));
    act(&mut g, Action::No);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).counters, 7);

    // Chapter III: the team can fight for {1}; the Saga comes back as Vorinclex (its search
    // triggers: find nothing).
    let theirs = put(&mut g, 1, "Llanowar Elves");
    g.start_turn(7, 0);
    pass_until_asked(&mut g);
    while g.obj(vor).zone == Zone::Battlefield && g.obj(vor).transformed {
        match g.pending {
            Pending::Decide(_) | Pending::ChooseTarget(_) => {
                let a = *g.legal_actions().last().unwrap();
                act(&mut g, a)
            }
            _ => act(&mut g, Action::PassPriority),
        };
    }
    resolve_all(&mut g, &[Action::No]);
    assert!(!g.obj(vor).transformed);
    assert_eq!(g.obj(vor).zone, Zone::Battlefield);
    assert!(g.types_of(vor).creature);
    let n = g.def_of(lions).activated.len();
    assert!(has(&g, Action::Activate(lions, n)), "the granted fight ability");
    act(&mut g, Action::Activate(lions, n));
    act(&mut g, Action::Target(Target::Object(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    // Not next turn.
    g.start_turn(9, 0);
    assert!(!has(&g, Action::Activate(lions, n)));
}

// ---- Aesi (Time), lane B ----

/// Library card on top (drawn next).
fn top(g: &mut Game, p: usize, name: &str) -> ObjId {
    g.add_card(p, cid(name), Zone::Library)
}

fn hand_size(g: &Game, p: usize) -> usize {
    g.players[p].hand.len()
}

#[test]
fn negate_mana_leak_and_scatter_ray_counter_their_kinds_of_spell() {
    let mut g = table();
    let ring = hand(&mut g, 0, "Sol Ring");
    let negate = hand(&mut g, 0, "Negate");
    act(&mut g, Action::Cast(ring));
    // Sol Ring on the stack: an artifact, noncreature spell.
    act(&mut g, Action::Cast(negate));
    assert!(has(&g, Action::Target(Target::Object(ring))));
    act(&mut g, Action::Target(Target::Object(ring)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard, "Negate countered Sol Ring");
    // Scatter Ray: artifact or creature spells, unless {4}.
    let mut g = table();
    let ring = hand(&mut g, 0, "Sol Ring");
    let bolt = hand(&mut g, 0, "Burst Lightning");
    let ray = hand(&mut g, 0, "Scatter Ray");
    act(&mut g, Action::Cast(ring));
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::Cast(ray));
    let legal = g.legal_actions();
    assert!(
        !legal.contains(&Action::Target(Target::Object(bolt))),
        "not an artifact or creature"
    );
    assert!(legal.contains(&Action::Target(Target::Object(ring))));
    act(&mut g, Action::Target(Target::Object(ring)));
    resolve_all(&mut g, &[Action::No]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    // Mana Leak: countered unless {3} is paid; its controller declines.
    let mut g = table();
    let bolt = hand(&mut g, 0, "Burst Lightning");
    let leak = hand(&mut g, 0, "Mana Leak");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::Cast(leak));
    act(&mut g, Action::Target(Target::Object(bolt)));
    resolve_all(&mut g, &[Action::No]);
    assert_eq!(g.players[1].life, 40, "countered");
}

#[test]
fn eternal_witness_returns_a_card_and_frantic_search_untaps_three_lands() {
    let mut g = table();
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Graveyard);
    let wit = hand(&mut g, 0, "Eternal Witness");
    play(
        &mut g,
        wit,
        false,
        &[],
        &[Action::Target(Target::Object(lions)), Action::Yes],
    );
    assert_eq!(g.obj(lions).zone, Zone::Hand);

    let mut g = game(&["Island", "Island", "Island"]);
    top(&mut g, 0, "Plains");
    top(&mut g, 0, "Plains");
    let search = hand(&mut g, 0, "Frantic Search");
    let a = hand(&mut g, 0, "Savannah Lions");
    let b = hand(&mut g, 0, "Savannah Lions");
    play(
        &mut g,
        search,
        false,
        &[],
        &[Action::Discard(a), Action::Discard(b)],
    );
    assert_eq!(g.obj(a).zone, Zone::Graveyard);
    let tapped = g
        .permanents(0)
        .filter(|&x| g.types_of(x).land && g.obj(x).tapped)
        .count();
    assert_eq!(tapped, 0, "all three lands untapped");
}

#[test]
fn courser_plays_lands_from_the_top_and_gains_life() {
    let mut g = table();
    put(&mut g, 0, "Courser of Kruphix");
    let forest = top(&mut g, 0, "Forest");
    assert!(has(&g, Action::PlayLand(forest)));
    act(&mut g, Action::PlayLand(forest));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 41);
}

#[test]
fn basalt_monolith_myriad_landscape_and_tectonic_edge() {
    let mut g = table();
    let basalt = put(&mut g, 0, "Basalt Monolith");
    assert!(g.def_of(basalt).doesnt_untap && g.def_of(basalt).mana_amount == 3);
    assert_eq!(g.def_of(basalt).activated.len(), 1);

    // Two basics sharing a land type.
    let mut g = table();
    let myriad = put(&mut g, 0, "Myriad Landscape");
    let f1 = top(&mut g, 0, "Forest");
    let f2 = top(&mut g, 0, "Forest");
    let island = top(&mut g, 0, "Island");
    act(&mut g, Action::Activate(myriad, 0));
    pass_until_asked(&mut g);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(island)) && legal.contains(&Action::Pick(f1)));
    act(&mut g, Action::Pick(f1));
    let legal = g.legal_actions();
    assert!(
        legal.contains(&Action::Pick(f2)) && !legal.contains(&Action::Pick(island)),
        "same type only"
    );
    act(&mut g, Action::Pick(f2));
    resolve_all(&mut g, &[]);
    assert!(g.obj(f1).zone == Zone::Battlefield && g.obj(f1).tapped && g.obj(f2).tapped);
    assert_eq!(g.obj(myriad).zone, Zone::Graveyard);

    // Tectonic Edge: a nonbasic land, while an opponent has four or more lands.
    let mut g = table();
    let edge = put(&mut g, 0, "Tectonic Edge");
    let theirs = put(&mut g, 1, "Myriad Landscape");
    act(&mut g, Action::Activate(edge, 0));
    let basic = g.permanents(1).find(|&x| g.types_of(x).basic).unwrap();
    assert!(!has(&g, Action::Target(Target::Object(basic))), "nonbasic only");
    act(&mut g, Action::Target(Target::Object(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.obj(edge).zone, Zone::Graveyard);
}

#[test]
fn silverback_elder_horn_of_greed_and_simic_growth_chamber() {
    let mut g = table();
    put(&mut g, 0, "Silverback Elder");
    let lions = hand(&mut g, 0, "Savannah Lions");
    play(&mut g, lions, false, &[], &[Action::Mode(2)]);
    assert_eq!(g.players[0].life, 44);

    let mut g = table();
    put(&mut g, 0, "Horn of Greed");
    let f = hand(&mut g, 0, "Forest");
    let before = hand_size(&g, 0);
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[]);
    assert_eq!(hand_size(&g, 0), before, "played one, drew one");

    let mut g = game(&["Forest"]);
    let forest = g.permanents(0).next().unwrap();
    let chamber = hand(&mut g, 0, "Simic Growth Chamber");
    act(&mut g, Action::PlayLand(chamber));
    resolve_all(&mut g, &[Action::Pick(forest)]);
    assert_eq!(g.obj(forest).zone, Zone::Hand);
    assert!(g.obj(chamber).tapped);
}

#[test]
fn emerald_medallion_makes_green_spells_cheaper() {
    let mut g = game(&["Forest", "Forest"]);
    let courser = hand(&mut g, 0, "Courser of Kruphix"); // {1}{G}{G}
    assert!(!has(&g, Action::Cast(courser)));
    put(&mut g, 0, "Emerald Medallion");
    assert!(has(&g, Action::Cast(courser)));
}

#[test]
fn jaheiras_respite_fetches_per_attacker_and_fogs() {
    let mut g = table();
    let a = put(&mut g, 1, "Savannah Lions");
    let b = put(&mut g, 1, "Savannah Lions");
    let f1 = top(&mut g, 0, "Forest");
    let f2 = top(&mut g, 0, "Island");
    let respite = hand(&mut g, 0, "Jaheira's Respite");
    g.start_turn(4, 1);
    while g.pending != Pending::DeclareAttackers(1) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(a));
    act(&mut g, Action::Attack(b));
    act(&mut g, Action::FinishAttacks);
    while g.pending != Pending::Priority(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Cast(respite));
    resolve_all(&mut g, &[Action::Pick(f1), Action::Pick(f2)]);
    assert!(g.obj(f1).zone == Zone::Battlefield && g.obj(f1).tapped);
    assert!(g.obj(f2).zone == Zone::Battlefield);
    for _ in 0..20 {
        if g.step == Step::End {
            break;
        }
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[0].life, 40, "combat damage prevented");
}

#[test]
fn sprouting_vines_storms_and_sword_of_the_animist_fetches_on_attack() {
    let mut g = table();
    let f1 = top(&mut g, 0, "Forest");
    let f2 = top(&mut g, 0, "Forest");
    let bolt = hand(&mut g, 0, "Burst Lightning");
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    let vines = hand(&mut g, 0, "Sprouting Vines");
    play(&mut g, vines, false, &[], &[Action::Pick(f2), Action::Pick(f1)]);
    assert_eq!(g.obj(f1).zone, Zone::Hand);
    assert_eq!(g.obj(f2).zone, Zone::Hand, "the storm copy found one too");

    let mut g = table();
    let lions = put(&mut g, 0, "Savannah Lions");
    let sword = put(&mut g, 0, "Sword of the Animist");
    act(&mut g, Action::Activate(sword, 0));
    resolve_all(&mut g, &[Action::Target(Target::Object(lions))]);
    assert_eq!(g.power(lions), 3);
    let forest = g.add_card(0, cid("Forest"), Zone::Library);
    g.add_card(0, cid("Plains"), Zone::Library); // drawn on turn 5
    attack(&mut g, &[lions]);
    resolve_all(&mut g, &[Action::Yes, Action::Pick(forest)]);
    assert!(g.obj(forest).zone == Zone::Battlefield && g.obj(forest).tapped);
}

#[test]
fn tolaria_west_transmutes_and_zimone_puts_lands_and_draws() {
    let mut g = table();
    let petal = g.add_card(0, cid("Lotus Petal"), Zone::Library);
    top(&mut g, 0, "Plains");
    let west = hand(&mut g, 0, "Tolaria West");
    assert!(has(&g, Action::Activate(west, 0)));
    act(&mut g, Action::Activate(west, 0));
    resolve_all(&mut g, &[Action::Pick(petal)]);
    assert_eq!(g.obj(west).zone, Zone::Graveyard);
    assert_eq!(g.obj(petal).zone, Zone::Hand);

    let mut g = table();
    let zimone = put(&mut g, 0, "Zimone, Quandrix Prodigy");
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::Activate(zimone, 0));
    resolve_all(&mut g, &[Action::Pick(f)]);
    assert!(g.obj(f).zone == Zone::Battlefield && g.obj(f).tapped);
    assert_eq!(g.players[0].lands_played, 0, "not a land drop");
    let mut g = table();
    let zimone = put(&mut g, 0, "Zimone, Quandrix Prodigy");
    let before = hand_size(&g, 0);
    act(&mut g, Action::Activate(zimone, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(hand_size(&g, 0), before + 2, "eight or more lands: two cards");
}

/// Passes until player `p`'s first main phase of the current turn.
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
fn rhystic_study_draws_unless_the_caster_pays() {
    let mut g = table();
    put(&mut g, 0, "Rhystic Study");
    let bolt = hand(&mut g, 1, "Burst Lightning");
    let before = hand_size(&g, 0);
    act(&mut g, Action::PassPriority); // player 1 gets priority
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(0)));
    resolve_all(&mut g, &[Action::Yes, Action::No]);
    assert_eq!(hand_size(&g, 0), before + 1, "they didn't pay");
    // They pay {1}: no card.
    let mut g = table();
    put(&mut g, 0, "Rhystic Study");
    let bolt = hand(&mut g, 1, "Burst Lightning");
    let before = hand_size(&g, 0);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(0)));
    resolve_all(&mut g, &[Action::Yes, Action::Yes]);
    assert_eq!(hand_size(&g, 0), before);
}

#[test]
fn mystic_remora_has_cumulative_upkeep() {
    let mut g = table();
    let remora = put(&mut g, 0, "Mystic Remora");
    g.start_turn(5, 0);
    pass_until_asked(&mut g);
    assert!(g.legal_actions().contains(&Action::Yes));
    act(&mut g, Action::Yes); // {1}
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(remora).zone, Zone::Battlefield);
    g.start_turn(7, 0);
    pass_until_asked(&mut g);
    act(&mut g, Action::No); // won't pay {2}
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(remora).zone, Zone::Graveyard);
}

#[test]
fn arcane_denial_counters_and_both_draw_next_upkeep() {
    let mut g = table();
    let bolt = hand(&mut g, 1, "Burst Lightning");
    let denial = hand(&mut g, 0, "Arcane Denial");
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(0)));
    act(&mut g, Action::PassPriority); // back to player 0
    act(&mut g, Action::Cast(denial));
    act(&mut g, Action::Target(Target::Object(bolt)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 40);
    assert_eq!(g.obj(bolt).zone, Zone::Graveyard);
    let (mine, theirs) = (hand_size(&g, 0), hand_size(&g, 1));
    g.start_turn(4, 1);
    to_main(&mut g);
    assert_eq!(hand_size(&g, 0), mine + 1);
    assert_eq!(
        hand_size(&g, 1),
        theirs + 3,
        "two from Arcane Denial and the draw step"
    );
}

#[test]
fn perennial_behemoth_unearths_with_haste_and_plays_lands_from_the_graveyard() {
    let mut g = table();
    put(&mut g, 0, "Perennial Behemoth");
    let forest = g.add_card(0, cid("Forest"), Zone::Graveyard);
    assert!(has(&g, Action::PlayLand(forest)));
    let mut g = table();
    let behemoth = g.add_card(0, cid("Perennial Behemoth"), Zone::Graveyard);
    g.start_turn(5, 0);
    to_main(&mut g);
    assert!(has(&g, Action::Activate(behemoth, 0)));
    act(&mut g, Action::Activate(behemoth, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(behemoth).zone, Zone::Battlefield);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    assert!(has(&g, Action::Attack(behemoth)), "haste");
    act(&mut g, Action::FinishAttacks);
    for _ in 0..30 {
        if g.obj(behemoth).zone != Zone::Battlefield {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(behemoth).zone, Zone::Exile, "exiled at the end step");
}

#[test]
fn walk_the_aeons_buys_itself_back_with_three_islands() {
    let mut g = table();
    let walk = hand(&mut g, 0, "Walk the Aeons");
    let islands = |g: &Game| g.permanents(0).filter(|&x| g.def_of(x).name == "Island").count();
    assert!(has(&g, Action::Cast(walk)) && has(&g, Action::CastKicked(walk)));
    act(&mut g, Action::CastKicked(walk));
    act(&mut g, Action::Target(Target::Player(0)));
    resolve_all(&mut g, &[]);
    assert_eq!(islands(&g), 2);
    assert_eq!(g.obj(walk).zone, Zone::Hand, "bought back");
    assert_eq!(g.extra_turns, vec![0]);
    let mut g = game(&["Island", "Island", "Forest", "Forest", "Forest", "Forest"]);
    let walk = hand(&mut g, 0, "Walk the Aeons");
    assert!(!has(&g, Action::CastKicked(walk)), "only two Islands");
}

#[test]
fn nissa_who_shakes_the_world_doubles_forests_and_animates_lands() {
    let mut g = game(&["Forest", "Forest"]);
    let courser = hand(&mut g, 0, "Courser of Kruphix");
    assert!(!has(&g, Action::Cast(courser)));
    let nissa = put(&mut g, 0, "Nissa, Who Shakes the World");
    assert!(has(&g, Action::Cast(courser)), "each Forest makes two green");
    let forest = g.permanents(0).find(|&x| g.def_of(x).name == "Forest").unwrap();
    act(&mut g, Action::Loyalty(nissa, 0));
    act(&mut g, Action::Target(Target::Object(forest)));
    resolve_all(&mut g, &[]);
    assert!(g.types_of(forest).creature && g.types_of(forest).land);
    assert_eq!((g.power(forest), g.toughness(forest)), (3, 3));
    assert!(g.keywords(forest).has(Keywords::HASTE) && g.keywords(forest).has(Keywords::VIGILANCE));
    assert!(g.has_subtype(forest, "Elemental"));
    // −8: the emblem and every Forest.
    let mut g = table();
    let nissa = put(&mut g, 0, "Nissa, Who Shakes the World");
    g.objects[nissa.0 as usize].loyalty = 8;
    let f1 = top(&mut g, 0, "Forest");
    let f2 = top(&mut g, 0, "Forest");
    act(&mut g, Action::Loyalty(nissa, 1));
    resolve_all(&mut g, &[Action::Pick(f2), Action::Pick(f1)]);
    assert!(g.obj(f1).zone == Zone::Battlefield && g.obj(f1).tapped);
    assert!(g.keywords(f2).has(Keywords::INDESTRUCTIBLE));
}

#[test]
fn blessing_of_frost_counts_snow_mana() {
    let mut g = game(&[
        "Snow-Covered Forest",
        "Snow-Covered Forest",
        "Snow-Covered Island",
        "Forest",
    ]);
    let lions = put(&mut g, 0, "Savannah Lions");
    let blessing = hand(&mut g, 0, "Blessing of Frost");
    let before = hand_size(&g, 0);
    act(&mut g, Action::Cast(blessing));
    pass_until_asked(&mut g);
    let mut picks = 0;
    while matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::Pick(lions));
        picks += 1;
    }
    resolve_all(&mut g, &[]);
    assert_eq!(picks, 3, "three snow lands paid");
    assert_eq!(g.power(lions), 5);
    assert_eq!(
        hand_size(&g, 0),
        before,
        "cast one, drew one for the power-4 Lions"
    );
}

// ---- Myrkul, Lord of Bones (DonSpider), lane B ----

#[test]
fn myrkul_lands_tap_for_their_colors() {
    let mut g = game(&[]);
    for (name, colors) in [
        ("Temple Garden", [false, false, false, false, true]),
        ("Godless Shrine", [true, false, true, false, false]),
        ("Vault of Champions", [true, false, true, false, false]),
        ("Bountiful Promenade", [true, false, false, false, true]),
        ("Caves of Koilos", [true, false, true, false, false]),
    ] {
        let land = put(&mut g, 0, name);
        let made = colors_of(&g, land);
        assert!(!made.is_empty(), "{name} taps for mana");
        for (i, &want) in colors.iter().enumerate() {
            if want {
                assert!(made.iter().any(|m| m[i]), "{name} makes color {i}");
            }
        }
    }
    // A bounce-free dual that enters tapped in a two-player game.
    let mut g = game(&[]);
    let vault = hand(&mut g, 0, "Vault of Champions");
    act(&mut g, Action::PlayLand(vault));
    assert!(g.obj(vault).tapped);
}

#[test]
fn city_of_brass_pings_you_when_tapped() {
    let mut g = game(&["City of Brass"]);
    let elves = hand(&mut g, 0, "Llanowar Elves");
    play(&mut g, elves, false, &[], &[]);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, 39);
}

#[test]
fn despark_and_anguished_unmaking_exile() {
    let mut g = table();
    let small = put(&mut g, 1, "Savannah Lions");
    let big = put(&mut g, 1, "Silverback Elder");
    let despark = hand(&mut g, 0, "Despark");
    act(&mut g, Action::Cast(despark));
    assert!(
        !has(&g, Action::Target(Target::Object(small))),
        "mana value 4 or greater"
    );
    act(&mut g, Action::Target(Target::Object(big)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(big).zone, Zone::Exile);
    let unmaking = hand(&mut g, 0, "Anguished Unmaking");
    play(&mut g, unmaking, false, &[Target::Object(small)], &[]);
    assert_eq!(g.obj(small).zone, Zone::Exile);
    assert_eq!(g.players[0].life, 37);
}

#[test]
fn anointed_procession_vanquish_the_horde_and_smothering_tithe() {
    let mut g = table();
    put(&mut g, 0, "Anointed Procession");
    put(&mut g, 0, "Smothering Tithe");
    // Player 1 draws on their turn and doesn't pay: two Treasures (doubled).
    g.start_turn(4, 1);
    for _ in 0..20 {
        if matches!(g.pending, Pending::Decide(1)) {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::No);
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 2);

    let mut g = game(&["Plains", "Plains"]);
    for _ in 0..3 {
        put(&mut g, 0, "Savannah Lions");
        put(&mut g, 1, "Savannah Lions");
    }
    let horde = hand(&mut g, 0, "Vanquish the Horde"); // {6}{W}{W}, {6} less
    assert!(has(&g, Action::Cast(horde)));
    play(&mut g, horde, false, &[], &[]);
    assert_eq!(g.creatures(0).count() + g.creatures(1).count(), 0);
}

#[test]
fn constellation_creatures_trigger_on_enchantments() {
    let mut g = table();
    let champ = put(&mut g, 0, "Setessan Champion");
    put(&mut g, 0, "Archon of Sun's Grace");
    let guardian = hand(&mut g, 0, "Grim Guardian"); // an enchantment creature
    let before = hand_size(&g, 0);
    play(&mut g, guardian, false, &[], &[]);
    assert_eq!(g.obj(champ).counters, 1);
    assert_eq!(hand_size(&g, 0), before, "cast it, drew one");
    assert_eq!(g.players[1].life, 39, "Grim Guardian itself entering");
    let pegasus = tokens(&g, 0, "Pegasus");
    assert_eq!(pegasus.len(), 1);
    assert!(g.keywords(pegasus[0]).has(Keywords::LIFELINK));
    assert!(g.keywords(pegasus[0]).has(Keywords::FLYING));
}

#[test]
fn ghostly_dancers_returns_an_enchantment_and_makes_spirits() {
    let mut g = table();
    let study = g.add_card(0, cid("Rhystic Study"), Zone::Graveyard);
    let dancers = hand(&mut g, 0, "Ghostly Dancers");
    play(&mut g, dancers, false, &[], &[Action::Pick(study)]);
    assert_eq!(g.obj(study).zone, Zone::Hand);
    play(&mut g, study, false, &[], &[]);
    let spirits: Vec<ObjId> = tokens(&g, 0, "Spirit");
    assert_eq!(spirits.len(), 1);
    assert_eq!((g.power(spirits[0]), g.toughness(spirits[0])), (3, 1));
}

#[test]
fn serras_sanctum_and_sanctum_weaver_count_enchantments() {
    let mut g = game(&[]);
    let sanctum = put(&mut g, 0, "Serra's Sanctum");
    assert!(colors_of(&g, sanctum).is_empty(), "no enchantments: no mana");
    put(&mut g, 0, "Rhystic Study");
    let weaver = put(&mut g, 0, "Sanctum Weaver");
    assert_eq!(colors_of(&g, sanctum).len(), 2);
    assert_eq!(colors_of(&g, weaver).len(), 2);
}

#[test]
fn grand_abolisher_stops_opponents_on_your_turn() {
    let mut g = table();
    put(&mut g, 0, "Grand Abolisher");
    let bolt = hand(&mut g, 1, "Burst Lightning");
    let birds = put(&mut g, 1, "Birds of Paradise");
    act(&mut g, Action::PassPriority);
    assert!(!has(&g, Action::Cast(bolt)));
    assert!(colors_of(&g, birds).is_empty() || g.mana_sources(1).iter().all(|&(x, _)| x != birds));
}

#[test]
fn devoted_druid_untaps_for_a_minus_counter() {
    let mut g = game(&[]);
    let druid = put(&mut g, 0, "Devoted Druid");
    g.objects[druid.0 as usize].tapped = true;
    assert!(has(&g, Action::Activate(druid, 0)));
    act(&mut g, Action::Activate(druid, 0));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(druid).tapped);
    assert_eq!(g.toughness(druid), 1);
    g.objects[druid.0 as usize].tapped = true;
    act(&mut g, Action::Activate(druid, 0));
    resolve_all(&mut g, &[]);
    act(&mut g, Action::PassPriority);
    assert_eq!(g.obj(druid).zone, Zone::Graveyard, "0/0 after the second counter");
}

#[test]
fn yawgmoth_sacrifices_for_counters_and_proliferates() {
    let mut g = table();
    let yawg = put(&mut g, 0, "Yawgmoth, Thran Physician");
    let fodder = put(&mut g, 0, "Savannah Lions");
    let theirs = put(&mut g, 1, "Silverback Elder");
    act(&mut g, Action::ActivateWith(yawg, 0, fodder));
    act(&mut g, Action::Target(Target::Object(theirs)));
    let before = hand_size(&g, 0);
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 39);
    assert_eq!(g.obj(fodder).zone, Zone::Graveyard);
    assert_eq!(g.toughness(theirs), 6);
    assert_eq!(hand_size(&g, 0), before + 1);
    let card = g.players[0].hand[0];
    act(&mut g, Action::ActivateWith(yawg, 1, card));
    resolve_all(&mut g, &[]);
    assert_eq!(g.toughness(theirs), 5, "proliferated their -1/-1 counter");
    assert_eq!(g.obj(card).zone, Zone::Graveyard);
}

#[test]
fn myrkul_makes_enchantment_copies_and_is_indestructible_at_half_life() {
    let mut g = table();
    let myrkul = put(&mut g, 0, "Myrkul, Lord of Bones");
    assert!(!g.keywords(myrkul).has(Keywords::INDESTRUCTIBLE));
    g.players[0].life = 20;
    assert!(g.keywords(myrkul).has(Keywords::INDESTRUCTIBLE));
    let witness = put(&mut g, 0, "Eternal Witness");
    let study = g.add_card(0, cid("Rhystic Study"), Zone::Graveyard);
    g.objects[witness.0 as usize].damage = 2;
    act(&mut g, Action::PassPriority);
    // Myrkul's may, then the token copy's "enters" trigger (a copy of Eternal Witness).
    resolve_all(
        &mut g,
        &[Action::Yes, Action::Target(Target::Object(study)), Action::Yes],
    );
    assert_eq!(g.obj(witness).zone, Zone::Exile);
    let copies = tokens(&g, 0, "Eternal Witness");
    assert_eq!(copies.len(), 1);
    let t = g.types_of(copies[0]);
    assert!(t.enchantment && !t.creature);
    assert_eq!(g.obj(study).zone, Zone::Hand);
}

#[test]
fn maha_makes_their_creatures_base_toughness_one() {
    let mut g = table();
    put(&mut g, 0, "Maha, Its Feathers Night");
    let elder = put(&mut g, 1, "Silverback Elder");
    let mine = put(&mut g, 0, "Silverback Elder");
    assert_eq!(g.toughness(elder), 1);
    assert_eq!(g.toughness(mine), 7);
}

#[test]
fn esper_sentinel_taxes_the_first_noncreature_spell() {
    let mut g = table();
    put(&mut g, 0, "Esper Sentinel");
    let a = hand(&mut g, 1, "Burst Lightning");
    let b = hand(&mut g, 1, "Burst Lightning");
    let before = hand_size(&g, 0);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(a));
    act(&mut g, Action::Target(Target::Player(0)));
    resolve_all(&mut g, &[Action::No]);
    assert_eq!(hand_size(&g, 0), before + 1);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(b));
    act(&mut g, Action::Target(Target::Player(0)));
    resolve_all(&mut g, &[]);
    assert_eq!(hand_size(&g, 0), before + 1, "only the first");
}

#[test]
fn austere_command_destroys_two_kinds() {
    let mut g = table();
    let ring = put(&mut g, 1, "Sol Ring");
    let study = put(&mut g, 1, "Anointed Procession");
    let lions = put(&mut g, 1, "Savannah Lions");
    let elder = put(&mut g, 1, "Silverback Elder");
    let cmd = hand(&mut g, 0, "Austere Command");
    // Mode 2: artifacts and creatures with mana value 3 or less (pairs in order 01 02 03 12 13 23).
    act(&mut g, Action::CastModal(cmd, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
    assert_eq!(g.obj(study).zone, Zone::Battlefield);
    assert_eq!(g.obj(elder).zone, Zone::Battlefield);
}

#[test]
fn dawns_truce_gives_you_hexproof() {
    let mut g = table();
    let truce = hand(&mut g, 0, "Dawn's Truce");
    let bolt = hand(&mut g, 1, "Burst Lightning");
    play(&mut g, truce, false, &[], &[]);
    act(&mut g, Action::PassPriority);
    act(&mut g, Action::Cast(bolt));
    assert!(!has(&g, Action::Target(Target::Player(0))), "hexproof");
}

#[test]
fn eldritch_evolution_and_birthing_pod_climb_mana_values() {
    let mut g = table();
    let lions = put(&mut g, 0, "Savannah Lions"); // mana value 1
    let elder = g.add_card(0, cid("Silverback Elder"), Zone::Library); // 5
    let courser = g.add_card(0, cid("Courser of Kruphix"), Zone::Library); // 3
    let evo = hand(&mut g, 0, "Eldritch Evolution");
    act(&mut g, Action::CastWith(evo, lions));
    pass_until_asked(&mut g);
    let legal = g.legal_actions();
    assert!(
        legal.contains(&Action::Pick(courser)) && !legal.contains(&Action::Pick(elder)),
        "3 or less"
    );
    act(&mut g, Action::Pick(courser));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(courser).zone, Zone::Battlefield);
    assert_eq!(g.obj(evo).zone, Zone::Exile);

    let mut g = table();
    let pod = put(&mut g, 0, "Birthing Pod");
    let witness = put(&mut g, 0, "Eternal Witness"); // 3
    let elder = g.add_card(0, cid("Silverback Elder"), Zone::Library); // 5
    let medallion = g.add_card(0, cid("Nissa, Who Shakes the World"), Zone::Library); // 5, not a creature
    let study = g.add_card(0, cid("Grim Guardian"), Zone::Library); // 3
    let four = g.add_card(0, cid("Eidolon of Blossoms"), Zone::Library); // 4
    act(&mut g, Action::ActivateWith(pod, 0, witness));
    pass_until_asked(&mut g);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Pick(four)));
    assert!(
        !legal.contains(&Action::Pick(elder))
            && !legal.contains(&Action::Pick(study))
            && !legal.contains(&Action::Pick(medallion))
    );
    act(&mut g, Action::Pick(four));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(four).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, 40, "paid green mana, not life");
}

#[test]
fn enduring_vitality_and_jaheira_make_mana() {
    let mut g = table();
    put(&mut g, 0, "Enduring Vitality");
    let lions = put(&mut g, 0, "Savannah Lions");
    assert_eq!(colors_of(&g, lions), [[true; 5]]);
    let mut g = table();
    put(&mut g, 0, "Jaheira, Friend of the Forest");
    let tithe = put(&mut g, 0, "Smothering Tithe");
    let _ = tithe;
    let mut h = table();
    put(&mut h, 0, "Jaheira, Friend of the Forest");
    put(&mut h, 0, "Archon of Sun's Grace");
    let guardian = hand(&mut h, 0, "Grim Guardian");
    play(&mut h, guardian, false, &[], &[]);
    let pegasus = tokens(&h, 0, "Pegasus")[0];
    h.start_turn(5, 0); // no longer summoning sick
    assert_eq!(colors_of(&h, pegasus), [[false, false, false, false, true]]);
}

#[test]
fn enduring_vitality_returns_as_an_enchantment() {
    let mut g = table();
    let vit = put(&mut g, 0, "Enduring Vitality");
    g.objects[vit.0 as usize].damage = 3;
    act(&mut g, Action::PassPriority);
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(vit).zone, Zone::Battlefield);
    let t = g.types_of(vit);
    assert!(t.enchantment && !t.creature);
}

#[test]
fn arachnogenesis_makes_spiders_and_stops_other_combat_damage() {
    let mut g = table();
    let a = put(&mut g, 1, "Savannah Lions");
    let b = put(&mut g, 1, "Savannah Lions");
    let spell = hand(&mut g, 0, "Arachnogenesis");
    g.start_turn(4, 1);
    while g.pending != Pending::DeclareAttackers(1) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(a));
    act(&mut g, Action::Attack(b));
    act(&mut g, Action::FinishAttacks);
    while g.pending != Pending::Priority(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Cast(spell));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Spider").len(), 2);
    for _ in 0..20 {
        if g.step == Step::End {
            break;
        }
        let x = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, x);
    }
    assert_eq!(g.players[0].life, 40);
}

#[test]
fn for_the_common_good_copies_tokens_and_gains_life() {
    let mut g = table();
    put(&mut g, 0, "Archon of Sun's Grace");
    let guardian = hand(&mut g, 0, "Grim Guardian");
    play(&mut g, guardian, false, &[], &[]);
    let pegasus = tokens(&g, 0, "Pegasus")[0];
    let common = hand(&mut g, 0, "For the Common Good");
    act(&mut g, Action::CastX(common, 2));
    act(&mut g, Action::Target(Target::Object(pegasus)));
    resolve_all(&mut g, &[]);
    let all = tokens(&g, 0, "Pegasus");
    assert_eq!(all.len(), 3);
    assert!(g.keywords(all[2]).has(Keywords::INDESTRUCTIBLE));
    assert_eq!(g.players[0].life, 43);
}

#[test]
fn mirraris_wake_doubles_land_mana_and_pumps() {
    let mut g = game(&["Forest", "Forest"]);
    let lions = put(&mut g, 0, "Savannah Lions");
    let courser = hand(&mut g, 0, "Courser of Kruphix");
    assert!(!has(&g, Action::Cast(courser)));
    put(&mut g, 0, "Mirari's Wake");
    assert!(has(&g, Action::Cast(courser)));
    assert_eq!(g.power(lions), 3);
}

#[test]
fn kaya_doubles_tokens_and_exiles_graveyards() {
    let mut g = table();
    let kaya = put(&mut g, 0, "Kaya, Geist Hunter");
    g.objects[kaya.0 as usize].loyalty = 3;
    act(&mut g, Action::Loyalty(kaya, 1)); // −2
    resolve_all(&mut g, &[]);
    put(&mut g, 0, "Archon of Sun's Grace");
    let guardian = hand(&mut g, 0, "Grim Guardian");
    play(&mut g, guardian, false, &[], &[]);
    assert_eq!(tokens(&g, 0, "Pegasus").len(), 2);
    let mut g = table();
    let kaya = put(&mut g, 0, "Kaya, Geist Hunter");
    g.objects[kaya.0 as usize].loyalty = 6;
    g.add_card(0, cid("Savannah Lions"), Zone::Graveyard);
    g.add_card(1, cid("Savannah Lions"), Zone::Graveyard);
    let base = g.players[0].graveyard.len() + g.players[1].graveyard.len();
    act(&mut g, Action::Loyalty(kaya, 2));
    resolve_all(&mut g, &[]);
    assert!(g.players[0].graveyard.is_empty() && g.players[1].graveyard.is_empty());
    // Kaya herself went to the graveyard at 0 loyalty before the ability resolved.
    assert_eq!(tokens(&g, 0, "Spirit").len(), base + 1);
}

#[test]
fn the_golden_throne_saves_you_once_and_makes_three_mana() {
    let mut g = table();
    let throne = put(&mut g, 0, "The Golden Throne");
    g.players[0].life = 0;
    act(&mut g, Action::PassPriority);
    assert!(!g.is_over());
    assert_eq!(g.players[0].life, 1);
    assert_eq!(g.obj(throne).zone, Zone::Exile);
    let mut g = table();
    let throne = put(&mut g, 0, "The Golden Throne");
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::ActivateWith(throne, 0, lions));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].pool.len(), 3);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard);
}

#[test]
fn mondrak_doubles_tokens_and_gains_an_indestructible_counter() {
    let mut g = table();
    let mondrak = put(&mut g, 0, "Mondrak, Glory Dominus");
    put(&mut g, 0, "Archon of Sun's Grace");
    let guardian = hand(&mut g, 0, "Grim Guardian");
    play(&mut g, guardian, false, &[], &[]);
    let pegs = tokens(&g, 0, "Pegasus");
    assert_eq!(pegs.len(), 2);
    let ring = put(&mut g, 0, "Sol Ring");
    act(&mut g, Action::ActivateWith(mondrak, 0, ring));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(mondrak).has(Keywords::INDESTRUCTIBLE));
    assert_eq!(g.obj(ring).zone, Zone::Graveyard);
    assert_eq!(
        tokens(&g, 0, "Pegasus").len(),
        1,
        "the second sacrifice was a token"
    );
}

#[test]
fn exalted_sunborn_warps_in_and_comes_back_from_exile() {
    let mut g = table();
    let sun = hand(&mut g, 0, "Exalted Sunborn");
    assert!(has(&g, Action::CastAlternative(sun)));
    act(&mut g, Action::CastAlternative(sun));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(sun).zone, Zone::Battlefield);
    for _ in 0..40 {
        if g.obj(sun).zone == Zone::Exile {
            break;
        }
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(sun).zone, Zone::Exile);
    g.start_turn(5, 0);
    to_main(&mut g);
    assert!(has(&g, Action::Cast(sun)), "castable from exile");
}

#[test]
fn grim_haruspex_is_cast_face_down_and_turns_up_for_its_morph_cost() {
    let mut g = table();
    let har = hand(&mut g, 0, "Grim Haruspex");
    act(&mut g, Action::CastAlternative(har));
    resolve_all(&mut g, &[]);
    assert!(g.obj(har).face_down);
    assert_eq!((g.power(har), g.toughness(har)), (2, 2));
    assert!(has(&g, Action::Activate(har, 0)));
    act(&mut g, Action::Activate(har, 0));
    assert!(!g.obj(har).face_down);
    assert_eq!(g.def_of(har).name, "Grim Haruspex");
    let lions = put(&mut g, 0, "Savannah Lions");
    let before = hand_size(&g, 0);
    g.objects[lions.0 as usize].damage = 1;
    act(&mut g, Action::PassPriority);
    resolve_all(&mut g, &[]);
    assert_eq!(hand_size(&g, 0), before + 1);
}

#[test]
fn black_market_connections_chooses_one_or_more_each_first_main_phase() {
    let mut g = table();
    put(&mut g, 0, "Black Market Connections");
    let before = hand_size(&g, 0);
    g.start_turn(5, 0);
    for _ in 0..20 {
        if matches!(g.pending, Pending::Decide(_)) {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Mode(6)); // all three
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 34);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1);
    let shifter = tokens(&g, 0, "Shapeshifter");
    assert_eq!(shifter.len(), 1);
    assert!(g.has_subtype(shifter[0], "Elf") && g.has_subtype(shifter[0], "Dragon"));
    assert_eq!(hand_size(&g, 0), before + 2, "the draw step and Buy Information");
}

// ---- Tatyova, Benthic Druid (Grande), lane B ----

#[test]
fn tatyova_land_ramp() {
    let mut g = table();
    let f1 = top(&mut g, 0, "Forest");
    let f2 = top(&mut g, 0, "Island");
    let path = hand(&mut g, 0, "Migration Path");
    play(&mut g, path, false, &[], &[Action::Pick(f1), Action::Pick(f2)]);
    assert!(g.obj(f1).tapped && g.obj(f2).zone == Zone::Battlefield);
    // Cycling from hand.
    let path2 = hand(&mut g, 0, "Migration Path");
    assert!(has(&g, Action::Activate(path2, 0)));
    // Sakura-Tribe Scout: an untapped land from hand.
    let scout = put(&mut g, 0, "Sakura-Tribe Scout");
    let land = hand(&mut g, 0, "Forest");
    act(&mut g, Action::Activate(scout, 0));
    resolve_all(&mut g, &[Action::Pick(land)]);
    assert!(g.obj(land).zone == Zone::Battlefield && !g.obj(land).tapped);
    // Splendid Reclamation.
    let a = g.add_card(0, cid("Forest"), Zone::Graveyard);
    let rec = hand(&mut g, 0, "Splendid Reclamation");
    play(&mut g, rec, false, &[], &[]);
    assert!(g.obj(a).zone == Zone::Battlefield && g.obj(a).tapped);
}

/// Resolves the stack, answering each decision with its first move (discarding other cards) until
/// `land` can be picked, then picks it.
fn answer_until_land(g: &mut Game, land: ObjId) {
    for _ in 0..60 {
        if g.obj(land).zone == Zone::Battlefield {
            break;
        }
        let legal = g.legal_actions();
        let a = if legal.contains(&Action::Pick(land)) {
            Action::Pick(land)
        } else if matches!(g.pending, Pending::Decide(_) | Pending::ChooseTarget(_)) {
            *legal
                .iter()
                .find(|a| !matches!(a, Action::Discard(c) if *c == land))
                .unwrap()
        } else {
            Action::PassPriority
        };
        act(g, a);
    }
    resolve_all(g, &[]);
}

#[test]
fn joint_exploration_and_khalni_heart_expedition() {
    let mut g = table();
    let land = hand(&mut g, 0, "Forest");
    let je = hand(&mut g, 0, "Joint Exploration");
    act(&mut g, Action::CastKicked(je));
    answer_until_land(&mut g, land);
    assert!(g.obj(land).zone == Zone::Battlefield && !g.obj(land).tapped);
    let mut g = table();
    let khalni = put(&mut g, 0, "Khalni Heart Expedition");
    g.objects[khalni.0 as usize].kind_counters = 2;
    let f = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(f));
    resolve_all(&mut g, &[Action::Yes]);
    assert_eq!(g.obj(khalni).kind_counters, 3);
    let b1 = top(&mut g, 0, "Forest");
    let b2 = top(&mut g, 0, "Forest");
    act(&mut g, Action::Activate(khalni, 0));
    resolve_all(&mut g, &[Action::Pick(b2), Action::Pick(b1)]);
    assert_eq!(g.obj(khalni).zone, Zone::Graveyard);
    assert!(g.obj(b1).tapped);
}

#[test]
fn noxious_revival_on_the_trail_and_rude_awakening() {
    let mut g = table();
    let dead = g.add_card(1, cid("Savannah Lions"), Zone::Graveyard);
    let rev = hand(&mut g, 0, "Noxious Revival");
    assert!(
        has(&g, Action::CastAlternative(rev)),
        "2 life for the Phyrexian pip"
    );
    play(&mut g, rev, false, &[Target::Object(dead)], &[]);
    assert_eq!(g.players[1].library.last(), Some(&dead));
    let mut g = table();
    put(&mut g, 0, "On the Trail");
    let land = hand(&mut g, 0, "Forest");
    let search = hand(&mut g, 0, "Frantic Search");
    let n = hand_size(&g, 0);
    act(&mut g, Action::Cast(search));
    let _ = n;
    answer_until_land(&mut g, land); // the discards, then On the Trail's land
    assert!(g.obj(land).zone == Zone::Battlefield && g.obj(land).tapped);
    let mut g = table();
    let rude = hand(&mut g, 0, "Rude Awakening");
    act(&mut g, Action::CastModal(rude, 2)); // entwined
    resolve_all(&mut g, &[]);
    let l = g.permanents(0).find(|&x| g.types_of(x).land).unwrap();
    assert!(g.types_of(l).creature && !g.obj(l).tapped);
    assert_eq!(g.power(l), 2);
}

#[test]
fn squall_line_stroke_of_genius_wipe_away_and_lab_maniac() {
    let mut g = table();
    let flier = put(&mut g, 1, "Birds of Paradise");
    let ground = put(&mut g, 1, "Savannah Lions");
    let squall = hand(&mut g, 0, "Squall Line");
    act(&mut g, Action::CastX(squall, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(flier).zone, Zone::Graveyard);
    assert_eq!(g.obj(ground).zone, Zone::Battlefield);
    assert_eq!((g.players[0].life, g.players[1].life), (39, 39));
    let stroke = hand(&mut g, 0, "Stroke of Genius");
    let n = hand_size(&g, 1);
    act(&mut g, Action::CastX(stroke, 2));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(hand_size(&g, 1), n + 2);
    let wipe = hand(&mut g, 0, "Wipe Away");
    play(&mut g, wipe, false, &[Target::Object(ground)], &[]);
    assert_eq!(g.obj(ground).zone, Zone::Hand);
    // Laboratory Maniac with an empty library.
    let mut g = table();
    put(&mut g, 0, "Laboratory Maniac");
    g.players[0].library.clear();
    let search = hand(&mut g, 0, "Frantic Search");
    act(&mut g, Action::Cast(search));
    for _ in 0..5 {
        if g.is_over() {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    assert!(g.is_over());
    assert!(g.players[1].lost);
}

// ---- Zhulodok, Void Gorger (Adam), lane B ----

#[test]
fn zhulodok_utility_lands_and_rocks() {
    let mut g = table();
    let map = put(&mut g, 0, "Expedition Map");
    let land = g.add_card(0, cid("Strip Mine"), Zone::Library);
    act(&mut g, Action::Activate(map, 0));
    resolve_all(&mut g, &[Action::Pick(land)]);
    assert_eq!(g.obj(land).zone, Zone::Hand);
    act(&mut g, Action::PlayLand(land));
    let theirs = g.permanents(1).find(|&x| g.types_of(x).land).unwrap();
    act(&mut g, Action::Activate(land, 0));
    act(&mut g, Action::Target(Target::Object(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.obj(land).zone, Zone::Graveyard);
    let mut g = table();
    let waste = put(&mut g, 0, "Wasteland");
    act(&mut g, Action::Activate(waste, 0));
    let basic = g.permanents(1).find(|&x| g.types_of(x).basic).unwrap();
    assert!(!has(&g, Action::Target(Target::Object(basic))), "nonbasic only");
    let mut g = game(&[]);
    let myr = put(&mut g, 0, "Palladium Myr");
    assert_eq!(g.mana_sources(0).iter().filter(|&&(x, _)| x == myr).count(), 2);
    let stone = hand(&mut g, 0, "Worn Powerstone");
    let _ = stone;
    let mut g = table();
    let cave = put(&mut g, 0, "Urza's Cave");
    let target = g.add_card(0, cid("Wasteland"), Zone::Library);
    act(&mut g, Action::Activate(cave, 0));
    resolve_all(&mut g, &[Action::Pick(target)]);
    assert!(g.obj(target).zone == Zone::Battlefield && g.obj(target).tapped);
}

#[test]
fn zhulodok_keys_coat_and_artisan() {
    let mut g = table();
    let vkey = put(&mut g, 0, "Voltaic Key");
    let ring = put(&mut g, 0, "Sol Ring");
    g.objects[ring.0 as usize].tapped = true;
    act(&mut g, Action::Activate(vkey, 0));
    act(&mut g, Action::Target(Target::Object(ring)));
    resolve_all(&mut g, &[]);
    assert!(!g.obj(ring).tapped);
    let mkey = put(&mut g, 0, "Manifold Key");
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::Activate(mkey, 1));
    act(&mut g, Action::Target(Target::Object(lions)));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(lions).has(Keywords::UNBLOCKABLE));
    // Mithril Coat attaches itself to a legendary creature.
    let mut g = table();
    let myrkul = put(&mut g, 0, "Maha, Its Feathers Night");
    let plain = put(&mut g, 0, "Savannah Lions");
    let coat = hand(&mut g, 0, "Mithril Coat");
    act(&mut g, Action::Cast(coat));
    pass_until_asked(&mut g);
    assert!(!has(&g, Action::Target(Target::Object(plain))), "legendary only");
    act(&mut g, Action::Target(Target::Object(myrkul)));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(myrkul).has(Keywords::INDESTRUCTIBLE));
    // Artisan of Kozilek: a cast trigger reanimates.
    let mut g = table();
    let dead = g.add_card(0, cid("Silverback Elder"), Zone::Graveyard);
    for _ in 0..5 {
        put(&mut g, 0, "Sol Ring");
    }
    let artisan = hand(&mut g, 0, "Artisan of Kozilek");
    act(&mut g, Action::Cast(artisan));
    resolve_all(&mut g, &[Action::Target(Target::Object(dead)), Action::Yes]);
    assert_eq!(g.obj(dead).zone, Zone::Battlefield);
    assert_eq!(g.obj(artisan).zone, Zone::Battlefield);
}

#[test]
fn fountainport_makes_fish_and_treasure_and_draws() {
    let mut g = table();
    let port = put(&mut g, 0, "Fountainport");
    act(&mut g, Action::Activate(port, 1));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Fish").len(), 1);
    assert_eq!(g.players[0].life, 39);
    let fish = tokens(&g, 0, "Fish")[0];
    g.objects[port.0 as usize].tapped = false;
    let n = hand_size(&g, 0);
    act(&mut g, Action::ActivateWith(port, 0, fish));
    resolve_all(&mut g, &[]);
    assert_eq!(hand_size(&g, 0), n + 1);
    g.objects[port.0 as usize].tapped = false;
    act(&mut g, Action::Activate(port, 2));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1);
}

// ---- Oloro, Ageless Ascetic (Adam), lane B ----

#[test]
fn oloro_lifegain_payoffs() {
    let mut g = table();
    let thune = put(&mut g, 0, "Archangel of Thune");
    let lions = put(&mut g, 0, "Savannah Lions");
    put(&mut g, 0, "Cliffhaven Vampire");
    put(&mut g, 0, "Drogskol Reaver");
    put(&mut g, 0, "Rhox Faithmender");
    let voice = put(&mut g, 0, "Voice of the Blessed");
    put(&mut g, 0, "Courser of Kruphix");
    let n = hand_size(&g, 0);
    let land = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(land));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 42, "1 doubled");
    assert_eq!(g.players[1].life, 39);
    assert_eq!(hand_size(&g, 0), n + 1 - 1 + 1, "played the land, drew one");
    assert_eq!(g.obj(lions).counters, 1);
    assert_eq!(g.obj(thune).counters, 1);
    assert_eq!(g.obj(voice).counters, 2, "Thune's and its own");
}

#[test]
fn oloro_life_total_statics() {
    let mut g = table();
    let serra = put(&mut g, 0, "Serra Ascendant");
    let divinity = put(&mut g, 0, "Divinity of Pride");
    assert_eq!(g.power(serra), 6, "40 life");
    assert!(g.keywords(serra).has(Keywords::FLYING));
    assert_eq!(g.power(divinity), 8);
    g.players[0].life = 20;
    assert_eq!(g.power(serra), 1);
    assert_eq!(g.power(divinity), 4);
    let voice = put(&mut g, 0, "Voice of the Blessed");
    g.objects[voice.0 as usize].counters = 4;
    assert!(g.keywords(voice).has(Keywords::FLYING) && !g.keywords(voice).has(Keywords::INDESTRUCTIBLE));
    g.objects[voice.0 as usize].counters = 10;
    assert!(g.keywords(voice).has(Keywords::INDESTRUCTIBLE));
}

#[test]
fn heliod_grows_on_lifegain_and_grants_lifelink() {
    let mut g = table();
    let heliod = put(&mut g, 0, "Heliod, Sun-Crowned");
    assert!(!g.types_of(heliod).creature, "no devotion");
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::Activate(heliod, 0));
    act(&mut g, Action::Target(Target::Object(lions)));
    resolve_all(&mut g, &[]);
    assert!(g.keywords(lions).has(Keywords::LIFELINK));
    put(&mut g, 0, "Courser of Kruphix");
    let land = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(land));
    resolve_all(&mut g, &[Action::Target(Target::Object(lions))]);
    assert_eq!(g.obj(lions).counters, 1);
}

// ---- Ghyrson Starn, Kelermorph (Spr3adsh33t), lane B ----

#[test]
fn ghyrson_spell_payoffs_ping_and_untap() {
    let mut g = table();
    put(&mut g, 0, "Electrostatic Field");
    put(&mut g, 0, "Erebor Flamesmith");
    put(&mut g, 0, "Kessig Flamebreather");
    let thermo = put(&mut g, 0, "Thermo-Alchemist");
    g.objects[thermo.0 as usize].tapped = true;
    let consider = hand(&mut g, 0, "Consider");
    play(&mut g, consider, false, &[], &[Action::No]);
    assert_eq!(g.players[1].life, 37, "three pings");
    assert!(!g.obj(thermo).tapped);
    act(&mut g, Action::Activate(thermo, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 36);
}

#[test]
fn ghyrson_burn_and_cantrips() {
    let mut g = table();
    let a = put(&mut g, 1, "Llanowar Elves");
    let b = put(&mut g, 1, "Llanowar Elves");
    let fire = hand(&mut g, 0, "Cast into the Fire");
    act(&mut g, Action::CastModal(fire, 0));
    act(&mut g, Action::Target(Target::Object(a)));
    act(&mut g, Action::Target(Target::Object(b)));
    resolve_all(&mut g, &[]);
    assert!(g.obj(a).zone == Zone::Graveyard && g.obj(b).zone == Zone::Graveyard);
    let bolt = hand(&mut g, 0, "Burst Lightning");
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    let shot = hand(&mut g, 0, "Grapeshot");
    play(&mut g, shot, false, &[Target::Player(1)], &[]);
    assert_eq!(
        g.players[1].life, 35,
        "2, then 1 and two storm copies (two spells before it)"
    );
    let n = hand_size(&g, 0);
    let lyze = hand(&mut g, 0, "Electrolyze");
    let c = put(&mut g, 1, "Llanowar Elves");
    act(&mut g, Action::Cast(lyze));
    act(&mut g, Action::Target(Target::Object(c)));
    act(&mut g, Action::Target(Target::Player(1)));
    while matches!(g.pending, Pending::Divide(_)) {
        act(&mut g, Action::Amount(1)); // 1 and 1
    }
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
    assert_eq!(g.players[1].life, 34);
    assert_eq!(
        hand_size(&g, 0),
        n + 1,
        "Electrolyze was added after counting; it drew one"
    );
}

#[test]
fn niv_mizzet_draws_and_pings_and_sharpshooter_untaps() {
    let mut g = table();
    put(&mut g, 0, "Niv-Mizzet, Parun");
    let serum = hand(&mut g, 0, "Serum Visions");
    act(&mut g, Action::Cast(serum));
    // Niv's cast trigger draws; each draw pings (target player 1).
    for _ in 0..30 {
        if g.items.is_empty() && !matches!(g.pending, Pending::Decide(_) | Pending::ChooseTarget(_)) {
            break;
        }
        let a = match g.pending {
            Pending::ChooseTarget(_) => Action::Target(Target::Player(1)),
            Pending::Decide(_) => *g.legal_actions().last().unwrap(),
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[1].life, 38, "two draws: Niv's and Serum Visions'");
    let mut g = table();
    let shooter = put(&mut g, 0, "Goblin Sharpshooter");
    let elves = put(&mut g, 1, "Llanowar Elves");
    act(&mut g, Action::Activate(shooter, 0));
    act(&mut g, Action::Target(Target::Object(elves)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elves).zone, Zone::Graveyard);
    assert!(!g.obj(shooter).tapped, "untapped when the Elves died");
}

#[test]
fn ghyrson_lands_and_equipment() {
    let mut g = table();
    let bay = put(&mut g, 0, "The Surgical Bay");
    let n = hand_size(&g, 0);
    act(&mut g, Action::Activate(bay, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(hand_size(&g, 0), n + 1);
    let lions = put(&mut g, 0, "Savannah Lions");
    let collar = put(&mut g, 0, "Basilisk Collar");
    act(&mut g, Action::Activate(collar, 0));
    resolve_all(&mut g, &[Action::Target(Target::Object(lions))]);
    assert!(g.keywords(lions).has(Keywords::DEATHTOUCH) && g.keywords(lions).has(Keywords::LIFELINK));
}

#[test]
fn dawn_of_hope_mystic_and_iconoclast_make_tokens() {
    let mut g = table();
    let dawn = put(&mut g, 0, "Dawn of Hope");
    act(&mut g, Action::Activate(dawn, 0));
    resolve_all(&mut g, &[]);
    let soldiers = tokens(&g, 0, "Soldier");
    assert_eq!(soldiers.len(), 1);
    assert!(g.keywords(soldiers[0]).has(Keywords::LIFELINK));
    put(&mut g, 0, "Murmuring Mystic");
    put(&mut g, 0, "Third Path Iconoclast");
    let consider = hand(&mut g, 0, "Consider");
    play(&mut g, consider, false, &[], &[Action::No]);
    let birds = tokens(&g, 0, "Bird Illusion");
    assert_eq!(birds.len(), 1);
    assert!(g.keywords(birds[0]).has(Keywords::FLYING));
    let art = tokens(&g, 0, "Soldier")
        .into_iter()
        .find(|&x| g.types_of(x).artifact);
    assert!(art.is_some());
}

#[test]
fn storm_kiln_artist_makes_treasure_and_grows_with_artifacts() {
    let mut g = table();
    let kiln = put(&mut g, 0, "Storm-Kiln Artist");
    let consider = hand(&mut g, 0, "Consider");
    play(&mut g, consider, false, &[], &[Action::No]);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1);
    assert_eq!(g.power(kiln), 3, "2 plus one artifact (the Treasure)");
}

#[test]
fn misdirection_pitches_and_redirects() {
    let mut g = table();
    let mine = put(&mut g, 0, "Savannah Lions");
    let theirs = put(&mut g, 1, "Llanowar Elves");
    let bolt = hand(&mut g, 0, "Burst Lightning");
    let mis = hand(&mut g, 0, "Misdirection");
    let blue = hand(&mut g, 0, "Mana Leak");
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Object(mine)));
    assert!(has(&g, Action::CastWith(mis, blue)));
    act(&mut g, Action::CastWith(mis, blue));
    act(&mut g, Action::Target(Target::Object(bolt)));
    pass_until_asked(&mut g);
    act(&mut g, Action::Target(Target::Object(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Graveyard);
    assert_eq!(g.obj(mine).zone, Zone::Battlefield);
    // Mana Confluence is desktop lane d1's (a mana source paid with life; tests/pg_d1.rs).
}

#[test]
fn young_pyromancer_makes_elementals() {
    let mut g = table();
    put(&mut g, 0, "Young Pyromancer");
    let consider = hand(&mut g, 0, "Consider");
    play(&mut g, consider, false, &[], &[Action::No]);
    let e = tokens(&g, 0, "Elemental");
    assert_eq!(e.len(), 1);
    assert_eq!((g.power(e[0]), g.toughness(e[0])), (1, 1));
}

#[test]
fn void_winnower_stops_even_blockers_and_ulamog_exiles_twenty() {
    let mut g = table();
    put(&mut g, 0, "Void Winnower");
    let ulamog = put(&mut g, 0, "Ulamog, the Ceaseless Hunger");
    let titan = put(&mut g, 1, "Inferno Titan"); // mana value 6
    let elves = put(&mut g, 1, "Llanowar Elves"); // mana value 1
    let before = g.players[1].library.len();
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(ulamog));
    act(&mut g, Action::FinishAttacks);
    while !matches!(g.pending, Pending::DeclareBlockers(_)) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(!legal.contains(&Action::Block { blocker: titan, attacker: ulamog }), "even");
    assert!(legal.contains(&Action::Block { blocker: elves, attacker: ulamog }), "odd");
    assert_eq!(g.players[1].library.len(), before - 20);
}

#[test]
fn epic_experiment_casts_cheap_spells_and_bins_the_rest() {
    let mut g = table();
    let plains = g.add_card(0, cid("Plains"), Zone::Library);
    let leak = g.add_card(0, cid("Mana Leak"), Zone::Library); // mana value 2, no target: not castable
    let consider = g.add_card(0, cid("Consider"), Zone::Library);
    let e = hand(&mut g, 0, "Epic Experiment");
    act(&mut g, Action::CastX(e, 3));
    while !matches!(g.pending, Pending::FreeCast(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(has(&g, Action::Cast(consider)));
    act(&mut g, Action::Cast(consider));
    while !g.items.is_empty() || matches!(g.pending, Pending::Decide(_) | Pending::FreeCast(_) | Pending::ChooseTarget(_)) {
        let a = match g.pending {
            Pending::Decide(_) => Action::No,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(plains).zone, Zone::Graveyard);
    assert_eq!(g.obj(leak).zone, Zone::Graveyard);
    assert_eq!(g.obj(consider).zone, Zone::Graveyard, "cast, then resolved");
}

#[test]
fn metallurgic_summonings_makes_constructs_the_spells_size() {
    let mut g = table();
    let ms = put(&mut g, 0, "Metallurgic Summonings");
    let lyze = hand(&mut g, 0, "Electrolyze"); // mana value 3
    act(&mut g, Action::Cast(lyze));
    act(&mut g, Action::Target(Target::Player(1)));
    act(&mut g, Action::No);
    while matches!(g.pending, Pending::Divide(_)) {
        act(&mut g, Action::Amount(2));
    }
    resolve_all(&mut g, &[]);
    let c = tokens(&g, 0, "Construct");
    assert_eq!(c.len(), 1);
    assert_eq!((g.power(c[0]), g.toughness(c[0])), (3, 3));
    assert!(!has(&g, Action::Activate(ms, 0)), "one artifact: not six");
}
