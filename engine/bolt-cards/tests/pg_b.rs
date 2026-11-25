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
