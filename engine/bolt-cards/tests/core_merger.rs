//! The merger's pieces done by lane B: type-changing land statics (Blood Moon, Urborg), haste for
//! creatures' abilities (Thousand-Year Elixir), and ward—sacrifice nonland permanents (Valgavoth);
//! then the delegated pieces: once-each-turn modes, the exiled-until-end-step hook, threshold
//! activations, and the rest below.
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

#[test]
fn threshold_activation_and_once_each_turn_modes() {
    let mut g = table();
    put(&mut g, 0, "Monument to Endurance");
    let col = put(&mut g, 0, "Cephalid Coliseum");
    for _ in 0..6 {
        g.add_card(0, cid("Plains"), Zone::Graveyard);
    }
    assert!(
        !g.legal_actions().contains(&Action::Activate(col, 0)),
        "six cards: no threshold"
    );
    g.add_card(0, cid("Plains"), Zone::Graveyard);
    assert!(g.legal_actions().contains(&Action::Activate(col, 0)));
    for _ in 0..3 {
        hand(&mut g, 0, "Plains");
    }
    act(&mut g, Action::Activate(col, 0));
    act(&mut g, Action::Target(Target::Player(0)));
    let mut offered = Vec::new();
    while !g.items.is_empty() || matches!(g.pending, Pending::ChooseTarget(_) | Pending::Decide(_)) {
        let acts = g.legal_actions();
        let a = if let Some(&d) = acts.iter().find(|a| matches!(a, Action::Discard(_))) {
            d
        } else if acts.iter().any(|a| matches!(a, Action::Mode(_))) {
            let modes: Vec<u8> = acts
                .iter()
                .filter_map(|a| match a {
                    Action::Mode(m) => Some(*m),
                    _ => None,
                })
                .collect();
            offered.push(modes.clone());
            Action::Mode(modes[0])
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
    }
    assert_eq!(
        offered,
        vec![vec![0, 1, 2], vec![1, 2], vec![2]],
        "each mode once this turn"
    );
    assert_eq!(g.players[1].life, 37);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 1);
    assert_eq!(g.obj(col).zone, Zone::Graveyard, "sacrificed");
}

#[test]
fn threshold_lands_barbarian_ring_and_cabal_pit() {
    let mut g = table();
    let ring = put(&mut g, 0, "Barbarian Ring");
    let pit = put(&mut g, 0, "Cabal Pit");
    for _ in 0..7 {
        g.add_card(0, cid("Plains"), Zone::Graveyard);
    }
    act(&mut g, Action::Activate(ring, 0));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 38);
    let bear = put(&mut g, 1, "Savannah Lions"); // 2/1
    act(&mut g, Action::Activate(pit, 0));
    act(&mut g, Action::Target(Target::Object(bear)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(bear).zone, Zone::Graveyard);
}

#[test]
fn glimpse_the_impossible_bins_what_is_left_at_the_end_step() {
    let mut g = table();
    let top: Vec<ObjId> = (0..3)
        .map(|_| g.add_card(0, cid("Plains"), Zone::Library))
        .collect();
    let glimpse = hand(&mut g, 0, "Glimpse the Impossible");
    play(&mut g, glimpse, false, &[], &[]);
    for &c in &top {
        assert_eq!(g.obj(c).zone, Zone::Exile);
    }
    act(&mut g, Action::PlayLand(top[0]));
    assert_eq!(g.obj(top[0]).zone, Zone::Battlefield, "played from exile");
    while g.step != Step::End {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.obj(top[1]).zone, Zone::Graveyard);
    assert_eq!(g.obj(top[2]).zone, Zone::Graveyard);
    assert_eq!(tokens(&g, 0, "Eldrazi Spawn").len(), 2);
}

#[test]
fn unspent_mana_becomes_colorless_kruphix_horizon_stone() {
    use bolt_core::mana::ManaSource;
    let mut g = table();
    let k = put(&mut g, 0, "Kruphix, God of Horizons");
    assert!(!g.types_of(k).creature, "no devotion");
    put(&mut g, 0, "Horizon Stone");
    g.players[0].pool.push(ManaSource::of(&[bolt_core::Color::Green]));
    g.players[1].pool.push(ManaSource::of(&[bolt_core::Color::Red]));
    let step = g.step;
    while g.step == step {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[0].pool.len(), 1, "kept");
    assert!(g.players[0].pool[0].colorless && g.players[0].pool[0].colors == [false; 5]);
    assert!(g.players[1].pool.is_empty(), "no Kruphix: it empties");
}

#[test]
fn cast_only_during_your_end_step_necrologia() {
    let mut g = table();
    let n = hand(&mut g, 0, "Necrologia");
    assert!(!g.legal_actions().contains(&Action::CastX(n, 3)), "main phase");
    while g.step != Step::End {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert!(g.legal_actions().contains(&Action::CastX(n, 3)));
    let before = g.players[0].hand.len();
    act(&mut g, Action::CastX(n, 3));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 37);
    assert_eq!(g.players[0].hand.len(), before - 1 + 3);
}

#[test]
fn activate_only_if_you_attacked_with_two_or_more_minas_tirith_style() {
    // Minas Tirith's draw ability on a test definition (Plains: "{1}{W}, {T}: Draw a card.").
    let plains = cid("Plains");
    let mut defs = (*set("CMD").unwrap().defs).clone();
    let draw = bolt_core::effect::Ability::new(
        vec![],
        vec![bolt_core::effect::Effect::Draw {
            who: bolt_core::effect::Who::You,
            n: 1,
        }],
    );
    defs[plains].activated.push(bolt_core::card::Activated {
        mana: bolt_core::ManaCost::parse("{1}{W}").unwrap(),
        tap: true,
        ability: draw,
        min_attackers_this_turn: 2,
        ..Default::default()
    });
    let lib = vec![plains; 99];
    let mut g = Game::new_commander(
        std::sync::Arc::new(defs),
        [&lib, &lib],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    for _ in 0..4 {
        put(&mut g, 0, "Mountain");
    }
    let tirith = put(&mut g, 0, "Plains");
    put(&mut g, 0, "Plains"); // its {W}
    let a = put(&mut g, 0, "Savannah Lions");
    let b = put(&mut g, 0, "Savannah Lions");
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let idx = 0;
    assert!(
        !g.legal_actions().contains(&Action::Activate(tirith, idx)),
        "no attack yet"
    );
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(a));
    act(&mut g, Action::Attack(b));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        let x = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, x);
    }
    assert!(
        g.legal_actions().contains(&Action::Activate(tirith, idx)),
        "two attacked"
    );
}

/// A game on the CMD pool plus test sorceries (indices from the pool's length up), as `game`.
fn game_plus(extra: Vec<bolt_core::CardDef>) -> (Game, usize) {
    let mut defs = (*set("CMD").unwrap().defs).clone();
    let base = defs.len();
    defs.extend(extra);
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
    (g, base)
}

fn sorcery(effect: bolt_core::Effect, targets: Vec<bolt_core::Filter>) -> bolt_core::CardDef {
    bolt_core::CardDef::spell(
        "Room",
        "{1}",
        false,
        bolt_core::Ability::new(targets, vec![effect]),
    )
}

#[test]
fn twisted_caverns_cant_attack_until_your_next_turn() {
    use bolt_core::{Effect, Filter};
    let (mut g, base) = game_plus(vec![sorcery(
        Effect::CantAttackUntilYourNextTurn(0),
        vec![Filter::CREATURE],
    )]);
    let lions = put(&mut g, 0, "Savannah Lions");
    let s = g.add_card(0, base, Zone::Hand);
    play(&mut g, s, false, &[Target::Object(lions)], &[]);
    while g.step != Step::Main2 {
        assert_ne!(
            g.pending,
            Pending::DeclareAttackers(0),
            "it can't attack this turn"
        );
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(
        g.cant_attack_until.last().map(|e| e.2),
        Some(g.turn + 2),
        "player 0's next turn"
    );
}

#[test]
fn arena_goads_a_creature_into_attacking() {
    use bolt_core::{Effect, Filter};
    let (mut g, base) = game_plus(vec![sorcery(Effect::Goad(0), vec![Filter::CREATURE])]);
    let lions = put(&mut g, 1, "Savannah Lions");
    let s = g.add_card(0, base, Zone::Hand);
    play(&mut g, s, false, &[Target::Object(lions)], &[]);
    assert!(g.is_goaded(lions));
    g.start_turn(4, 1);
    while g.pending != Pending::DeclareAttackers(1) {
        let a = match g.pending {
            Pending::Decide(_) | Pending::ChooseTarget(_) => *g.legal_actions().last().unwrap(),
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Attack(lions)));
    assert!(!legal.contains(&Action::FinishAttacks), "goaded: it must attack");
}

#[test]
fn mad_wizards_lair_draws_three_and_may_cast_one_free() {
    let (mut g, base) = game_plus(vec![sorcery(bolt_core::Effect::DrawThreeMayCastOneFree, vec![])]);
    let lions = [
        g.add_card(0, cid("Savannah Lions"), Zone::Library),
        g.add_card(0, cid("Savannah Lions"), Zone::Library),
    ];
    let plains = g.add_card(0, cid("Plains"), Zone::Library);
    let s = g.add_card(0, base, Zone::Hand);
    act(&mut g, Action::Cast(s));
    while !matches!(g.pending, Pending::FreeCast(_)) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Cast(lions[0])) && legal.contains(&Action::Cast(lions[1])));
    assert_eq!(g.obj(plains).zone, Zone::Hand, "drawn, not castable");
    act(&mut g, Action::Cast(lions[0]));
    assert!(
        !g.legal_actions().contains(&Action::Cast(lions[1])),
        "one spell only"
    );
}

#[test]
fn invert_polarity_takes_the_spell_or_counters_it() {
    let (mut won, mut lost) = (false, false);
    for seed in 0..16 {
        let plains = vec![cid("Plains"); 99];
        let mut g = Game::new_commander(
            set("CMD").unwrap().defs.clone(),
            [&plains, &plains],
            [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
            seed,
            0,
        );
        g.auto_pass = false;
        for land in ["Island", "Island", "Mountain", "Mountain"] {
            put(&mut g, 0, land);
            put(&mut g, 1, land);
        }
        g.start_turn(3, 0);
        while g.step != Step::Main1 {
            act(&mut g, Action::PassPriority);
        }
        let bolt = hand(&mut g, 1, "Burst Lightning");
        let invert = hand(&mut g, 0, "Invert Polarity");
        act(&mut g, Action::PassPriority);
        act(&mut g, Action::Cast(bolt));
        act(&mut g, Action::Target(Target::Player(0)));
        if g.pending == Pending::Decide(1) {
            act(&mut g, Action::No); // not kicked
        }
        act(&mut g, Action::PassPriority);
        act(&mut g, Action::Cast(invert));
        act(&mut g, Action::Target(Target::Object(bolt)));
        while g.items.len() > 1 || matches!(g.pending, Pending::ChooseTarget(_)) {
            act(&mut g, Action::PassPriority);
        }
        if g.obj(bolt).zone == Zone::Graveyard && g.items.is_empty() && g.players[0].life == 40 {
            lost = true; // countered
            continue;
        }
        // Won: player 0 controls it and may pick a new target.
        assert_eq!(g.items[0].controller, 0, "seed {seed}");
        assert_eq!(g.pending, Pending::Decide(0));
        act(&mut g, Action::Target(Target::Player(1)));
        resolve_all(&mut g, &[]);
        assert_eq!(g.players[1].life, 38);
        won = true;
    }
    assert!(won && lost, "both outcomes over the seeds");
}

#[test]
fn a_chapter_granted_ability_waits_for_its_chapter_urzas_saga() {
    let island = cid("Island");
    let mut defs = (*set("CMD").unwrap().defs).clone();
    let draw = bolt_core::Ability::new(
        vec![],
        vec![bolt_core::Effect::Draw {
            who: bolt_core::Who::You,
            n: 1,
        }],
    );
    defs[island].activated.push(bolt_core::Activated {
        tap: true,
        ability: draw,
        ..Default::default()
    });
    let lib = vec![cid("Plains"); 99];
    let with = |chapter: Vec<(u8, usize)>| {
        let mut d = defs.clone();
        d[island].subsystems.chapter_activated = chapter;
        let mut g = Game::new_commander(
            std::sync::Arc::new(d),
            [&lib, &lib],
            [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
            7,
            0,
        );
        g.auto_pass = false;
        let i = put(&mut g, 0, "Island");
        g.start_turn(3, 0);
        while g.step != Step::Main1 {
            act(&mut g, Action::PassPriority);
        }
        g.legal_actions().contains(&Action::Activate(i, 0))
    };
    assert!(with(vec![]), "no chapter: a plain ability");
    assert!(!with(vec![(2, 0)]), "chapter II hasn't granted it");
}

#[test]
fn a_class_level_ability_waits_for_its_level() {
    // On a test definition: Island gains "Level 2" and a level-2 "{T}: Draw a card."
    let island = cid("Island");
    let mut defs = (*set("CMD").unwrap().defs).clone();
    let up = bolt_core::Ability::new(vec![], vec![bolt_core::Effect::ClassLevelUp(2)]);
    defs[island].activated.push(bolt_core::Activated {
        level_up_to: 2,
        sorcery: true,
        ability: up,
        ..Default::default()
    });
    let draw = bolt_core::Ability::new(
        vec![],
        vec![bolt_core::Effect::Draw {
            who: bolt_core::Who::You,
            n: 1,
        }],
    );
    defs[island].activated.push(bolt_core::Activated {
        tap: true,
        min_class_level: 2,
        ability: draw,
        ..Default::default()
    });
    let lib = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        std::sync::Arc::new(defs),
        [&lib, &lib],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    let i = put(&mut g, 0, "Island");
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert!(
        !g.legal_actions().contains(&Action::Activate(i, 1)),
        "level 1: not yet"
    );
    act(&mut g, Action::Activate(i, 0));
    resolve_all(&mut g, &[]);
    assert_eq!(g.class_level(i), 2);
    assert!(g.legal_actions().contains(&Action::Activate(i, 1)), "level 2");
    assert!(
        !g.legal_actions().contains(&Action::Activate(i, 0)),
        "already level 2"
    );
}

#[test]
fn a_discard_trigger_in_cleanup_resolves_that_turn() {
    // CR 514.3a: Monument to Endurance triggers on the discard to hand size, in cleanup.
    let mut g = table();
    put(&mut g, 0, "Monument to Endurance");
    while g.players[0].hand.len() < 9 {
        hand(&mut g, 0, "Plains");
    }
    while g.pending != Pending::Discard(0) {
        let a = match g.pending {
            Pending::DeclareAttackers(_) => Action::FinishAttacks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    let turn = g.turn;
    while g.pending == Pending::Discard(0) {
        let c = g.players[0].hand[0];
        act(&mut g, Action::Discard(c));
    }
    // Both triggers are put on the stack in this cleanup step, choosing their modes.
    while g.turn == turn && g.players[1].life == 40 {
        let a = match g.pending {
            Pending::Decide(_) => *g.legal_actions().last().unwrap(), // "Each opponent loses 3 life"
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.turn, turn, "resolved in the same turn's cleanup");
    assert_eq!(g.step, Step::Cleanup);
    assert_eq!(g.players[1].life, 37);
}

#[test]
fn one_object_can_fill_two_different_target_words() {
    // CR 115.3: Piercing Exhale's "target creature you control" and "target creature or
    // planeswalker" are different words, so a lone creature can be both.
    let mut g = table();
    let lions = put(&mut g, 0, "Savannah Lions");
    let exhale = hand(&mut g, 0, "Piercing Exhale");
    assert!(g.legal_actions().contains(&Action::Cast(exhale)));
    act(&mut g, Action::Cast(exhale));
    while matches!(g.pending, Pending::Decide(_)) {
        act(&mut g, Action::No); // don't behold
    }
    act(&mut g, Action::Target(Target::Object(lions)));
    assert!(
        g.legal_actions().contains(&Action::Target(Target::Object(lions))),
        "the same creature for the second word"
    );
    act(&mut g, Action::Target(Target::Object(lions)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(lions).zone, Zone::Graveyard, "2 damage to itself");
}

#[test]
fn harmonize_x_counts_the_tapped_creature_as_spent() {
    // Nature's Rhythm's harmonize {X}{G}{G}{G}{G}, tapping Llanowar Elves (power 1): the Elves can't
    // also make mana, so 4 Forests pay X = 1 ({1} less) but not X = 2.
    let mut g = game(&["Forest", "Forest", "Forest", "Forest"]);
    let elves = put(&mut g, 0, "Llanowar Elves");
    let rhythm = g.add_card(0, cid("Nature's Rhythm"), Zone::Graveyard);
    assert!(g.legal_actions().contains(&Action::CastHarmonize(rhythm, elves)));
    act(&mut g, Action::CastHarmonize(rhythm, elves));
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Amount(1)), "{legal:?}");
    assert!(
        !legal.contains(&Action::Amount(2)),
        "the Elves are tapped for harmonize"
    );
    act(&mut g, Action::Amount(1));
    assert!(g.obj(elves).tapped);
}

#[test]
fn bolas_citadel_casts_from_the_top_for_life() {
    let mut g = table();
    put(&mut g, 0, "Bolas's Citadel");
    let lions = g.add_card(0, cid("Savannah Lions"), Zone::Library); // on top, mana value 1
    assert!(g.legal_actions().contains(&Action::Cast(lions)));
    let untapped = g.permanents(0).filter(|&x| !g.obj(x).tapped).count();
    play(&mut g, lions, false, &[], &[]);
    assert_eq!(g.obj(lions).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, 39, "its mana value in life");
    assert_eq!(
        g.permanents(0).filter(|&x| !g.obj(x).tapped).count(),
        untapped + 1,
        "no mana: only the Lions are new"
    );
}

#[test]
fn xorn_adds_a_treasure_and_gadrak_needs_four_artifacts() {
    let mut g = table();
    put(&mut g, 0, "Xorn");
    let gadrak = put(&mut g, 0, "Gadrak, the Crown-Scourge");
    let score = hand(&mut g, 0, "Big Score"); // "create two Treasure tokens"
    let junk = hand(&mut g, 0, "Plains");
    act(&mut g, Action::CastWith(score, junk));
    resolve_all(&mut g, &[]);
    assert_eq!(tokens(&g, 0, "Treasure").len(), 3, "two plus one");
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(
        !legal.contains(&Action::Attack(gadrak)),
        "three artifacts: Gadrak can't attack"
    );
}

#[test]
fn silent_arbiter_allows_one_attacker() {
    let mut g = table();
    put(&mut g, 1, "Silent Arbiter");
    let a = put(&mut g, 0, "Savannah Lions");
    let b = put(&mut g, 0, "Savannah Lions");
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(a));
    assert!(
        !g.legal_actions().contains(&Action::Attack(b)),
        "one attacker each combat"
    );
}

#[test]
fn expropriate_counts_time_and_money_votes() {
    let mut g = table();
    let theirs = put(&mut g, 1, "Savannah Lions");
    let e = hand(&mut g, 0, "Expropriate");
    act(&mut g, Action::Cast(e));
    while g.pending != Pending::Decide(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Yes); // time
    act(&mut g, Action::No); // the opponent: money
    act(&mut g, Action::Pick(theirs));
    resolve_all(&mut g, &[]);
    assert_eq!(g.extra_turns, vec![0]);
    assert_eq!(g.obj(theirs).controller, 0);
    assert_eq!(g.obj(e).zone, Zone::Exile);
}

#[test]
fn a_target_that_raises_the_cost_needs_the_mana_dragons_prey() {
    // CR 601.2f: the total cost is fixed after targets; a Dragon target costs {2} more.
    let mut g = game(&["Swamp", "Swamp", "Swamp"]);
    let dragon = put(&mut g, 1, "Magmatic Hellkite");
    let prey = hand(&mut g, 0, "Dragon's Prey");
    assert!(
        !g.legal_actions().contains(&Action::Cast(prey)),
        "only a Dragon to target, and no {{2}} more"
    );
    let mut g = game(&["Swamp", "Swamp", "Swamp", "Swamp", "Swamp"]);
    let dragon2 = put(&mut g, 1, "Magmatic Hellkite");
    let prey = hand(&mut g, 0, "Dragon's Prey");
    act(&mut g, Action::Cast(prey));
    assert!(g
        .legal_actions()
        .contains(&Action::Target(Target::Object(dragon2))));
    act(&mut g, Action::Target(Target::Object(dragon2)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(dragon2).zone, Zone::Graveyard);
    let _ = dragon;
}

#[test]
fn harmonize_excludes_its_creature_from_paying_even_a_devotee() {
    // Mardu Devotee's "{1}: Add {R}" doesn't tap, but payment excludes the harmonize creature:
    // so does castability. Four Plains can't make Wild Ride's {R} without it.
    let mut g = game(&["Plains", "Plains", "Plains", "Plains"]);
    let devotee = put(&mut g, 0, "Mardu Devotee");
    let ride = g.add_card(0, cid("Wild Ride"), Zone::Graveyard);
    assert!(!g.legal_actions().contains(&Action::CastHarmonize(ride, devotee)));
}

// ---- Lane B, 2026-11-26: can't block while you control the source; first combat damage each turn ----

#[test]
fn a_creature_cant_block_while_you_control_the_source() {
    // Sol Ring, given There and Back Again's chapter I effect as an enters trigger for this test.
    let mut defs = (*set("CMD").unwrap().defs).clone();
    defs[cid("Sol Ring")].triggers.push(bolt_core::Trigger {
        when: bolt_core::When::Enters,
        ability: bolt_core::Ability::new(
            vec![bolt_core::Filter::CREATURE],
            vec![bolt_core::Effect::TargetCantBlockWhileYouControlSource(0)],
        ),
    });
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
        put(&mut g, 0, land);
    }
    let lions = put(&mut g, 0, "Savannah Lions");
    let wall = put(&mut g, 1, "Savannah Lions");
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let ring = hand(&mut g, 0, "Sol Ring");
    play(&mut g, ring, false, &[], &[Action::Target(Target::Object(wall))]);
    // Turn 5: their Lions can't block, so no blockers are declared and the attack connects.
    let attack = |g: &mut Game, turn: u32| -> bool {
        g.start_turn(turn, 0);
        while g.pending != Pending::DeclareAttackers(0) {
            act(g, Action::PassPriority);
        }
        act(g, Action::Attack(lions));
        act(g, Action::FinishAttacks);
        while g.step != Step::CombatDamage && g.pending != Pending::DeclareBlockers(1) {
            act(g, Action::PassPriority);
        }
        g.pending == Pending::DeclareBlockers(1)
            && g.legal_actions().contains(&Action::Block {
                blocker: wall,
                attacker: lions,
            })
    };
    assert!(!attack(&mut g, 5), "can't block while you control Sol Ring");
    g.objects[ring.0 as usize].controller = 1; // you no longer control it
    assert!(attack(&mut g, 7));
}

#[test]
fn an_x_ability_targeting_mana_value_x_is_offered_only_for_that_x() {
    // Sol Ring, given "{X}, {T}: Destroy target artifact an opponent controls with mana value X." for
    // this test (The Mycosynth Gardens' shape): X is checked against the targets for each X.
    let mut defs = (*set("CMD").unwrap().defs).clone();
    let ring_def = cid("Sol Ring");
    let n = defs[ring_def].activated.len();
    let target = bolt_core::Filter {
        players: false,
        creatures: false,
        planeswalkers: false,
        artifacts: true,
        mana_value_is_x: true,
        controller: bolt_core::Controller::Opponent,
        ..bolt_core::Filter::ANY
    };
    defs[ring_def].activated.push(bolt_core::Activated {
        mana: bolt_core::ManaCost::parse("{X}").expect("cost"),
        tap: true,
        ability: bolt_core::Ability::new(vec![target], vec![bolt_core::Effect::Destroy(0)]),
        ..bolt_core::Activated::default()
    });
    let plains = vec![cid("Plains"); 99];
    let mut g = Game::new_commander(
        std::sync::Arc::new(defs),
        [&plains, &plains],
        [cid("Bello, Bard of the Brambles"), cid("Ms. Bumbleflower")],
        7,
        0,
    );
    g.auto_pass = false;
    for _ in 0..4 {
        put(&mut g, 0, "Plains");
    }
    let ring = put(&mut g, 0, "Sol Ring");
    let stone = put(&mut g, 1, "Mind Stone"); // mana value 2
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let offered: Vec<u8> = g
        .legal_actions()
        .iter()
        .filter_map(|a| match *a {
            Action::ActivateX { id, n: k, x, .. } if id == ring && k == n => Some(x),
            _ => None,
        })
        .collect();
    assert_eq!(offered, vec![2]);
    act(
        &mut g,
        Action::ActivateX {
            id: ring,
            n,
            victim: None,
            x: 2,
        },
    );
    resolve_all(&mut g, &[Action::Target(Target::Object(stone))]);
    assert_eq!(g.obj(stone).zone, Zone::Graveyard);
}

#[test]
fn the_mycosynth_gardens_filters_one_mana_and_copies_at_the_mana_value_of_your_artifact() {
    // "{1}, {T}: Add one mana of any color": Plains and the Gardens make {G}.
    let mut g = game(&["Plains"]);
    put(&mut g, 0, "The Mycosynth Gardens");
    let elves = hand(&mut g, 0, "Llanowar Elves");
    assert!(g.legal_actions().contains(&Action::Cast(elves)));
    play(&mut g, elves, false, &[], &[]);
    assert_eq!(g.obj(elves).zone, Zone::Battlefield);

    let mut g = game(&["Plains", "Plains", "Plains", "Plains"]);
    let gardens = put(&mut g, 0, "The Mycosynth Gardens");
    let stone = put(&mut g, 0, "Mind Stone"); // mana value 2
    let copy = |a: &Action| match *a {
        Action::ActivateX { id, n: 0, x, .. } if id == gardens => Some(x),
        _ => None,
    };
    let offered: Vec<u8> = g.legal_actions().iter().filter_map(copy).collect();
    assert_eq!(offered, vec![2]);
    act(
        &mut g,
        Action::ActivateX {
            id: gardens,
            n: 0,
            victim: None,
            x: 2,
        },
    );
    resolve_all(&mut g, &[Action::Target(Target::Object(stone))]);
    assert_eq!(g.def_of(gardens).name, "Mind Stone", "a copy of it");
}
