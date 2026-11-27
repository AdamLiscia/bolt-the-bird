//! Mac B core theme, replacement effects: each piece tested on a card that uses it, in a 1v1
//! Commander game on the CMD pool.
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
fn counters_plus_one_hardened_scales() {
    let mut g = table();
    put(&mut g, 0, "Hardened Scales");
    let champ = put(&mut g, 0, "Setessan Champion");
    let theirs = put(&mut g, 1, "Setessan Champion");
    let guardian = hand(&mut g, 0, "Grim Guardian");
    play(&mut g, guardian, false, &[], &[]);
    assert_eq!(g.obj(champ).counters, 2, "one plus one");
    assert_eq!(g.obj(theirs).counters, 0);
}

#[test]
fn opponents_lose_double_your_turn_bloodletter() {
    let mut g = table();
    put(&mut g, 0, "Bloodletter of Aclazotz");
    let bolt = hand(&mut g, 0, "Burst Lightning");
    play(&mut g, bolt, false, &[Target::Player(1)], &[]);
    assert_eq!(g.players[1].life, 36, "2 damage, 4 life lost");
    // Not on their turn.
    let mut g = table();
    put(&mut g, 0, "Bloodletter of Aclazotz");
    let bolt = hand(&mut g, 1, "Burst Lightning");
    g.start_turn(4, 1);
    while !matches!(g.pending, Pending::Priority(1)) || g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[1].life, 38);
}

#[test]
fn life_gain_and_extra_draws_doubled_alhammarrets_archive() {
    let mut g = table();
    put(&mut g, 0, "Alhammarret's Archive");
    put(&mut g, 0, "Courser of Kruphix");
    let land = hand(&mut g, 0, "Forest");
    act(&mut g, Action::PlayLand(land));
    resolve_all(&mut g, &[]);
    assert_eq!(g.players[0].life, 42, "1 life doubled");
    let n = g.players[0].hand.len();
    put(&mut g, 0, "Horn of Greed");
    // Turn 5's draw step: one card (the first isn't replaced).
    g.start_turn(5, 0);
    for _ in 0..10 {
        if g.step == Step::Main1 {
            break;
        }
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.players[0].hand.len(), n + 1);
}

#[test]
fn extra_draws_doubled_teferis_ageless_insight() {
    let mut g = table();
    put(&mut g, 0, "Teferi's Ageless Insight");
    let search = hand(&mut g, 0, "Frantic Search");
    let n = g.players[0].hand.len();
    act(&mut g, Action::Cast(search));
    pass_until_asked(&mut g);
    // Draw two became draw four; then discard two.
    assert_eq!(g.players[0].hand.len(), n - 1 + 4);
}

#[test]
fn tokens_plus_one_quina_and_sacrifice_a_frog() {
    let mut g = table();
    let quina = put(&mut g, 0, "Quina, Qu Gourmet");
    put(&mut g, 0, "Archon of Sun's Grace");
    let guardian = hand(&mut g, 0, "Grim Guardian");
    play(&mut g, guardian, false, &[], &[]);
    assert_eq!(tokens(&g, 0, "Pegasus").len(), 1);
    let frogs = tokens(&g, 0, "Frog");
    assert_eq!(frogs.len(), 1);
    act(&mut g, Action::ActivateWith(quina, 0, frogs[0]));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(quina).counters, 1);
}

#[test]
fn counter_exile_force_of_negation() {
    let mut g = table();
    let force = hand(&mut g, 0, "Force of Negation");
    let blue = hand(&mut g, 0, "Mana Leak");
    let ring = hand(&mut g, 1, "Sol Ring");
    g.start_turn(4, 1);
    while !matches!(g.pending, Pending::Priority(1)) || g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Cast(ring));
    act(&mut g, Action::PassPriority); // to player 0
    assert!(
        has(&g, Action::CastWith(force, blue)),
        "pitch a blue card on their turn"
    );
    act(&mut g, Action::CastWith(force, blue));
    act(&mut g, Action::Target(Target::Object(ring)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(ring).zone, Zone::Exile);
    assert_eq!(g.obj(blue).zone, Zone::Exile);
}

fn has(g: &Game, a: Action) -> bool {
    g.legal_actions().contains(&a)
}

#[test]
fn double_each_counter_deepglow_skate() {
    let mut g = table();
    let champ = put(&mut g, 0, "Setessan Champion");
    g.objects[champ.0 as usize].counters = 3;
    let kaya = put(&mut g, 0, "Kaya, Geist Hunter");
    g.objects[kaya.0 as usize].loyalty = 3;
    let skate = hand(&mut g, 0, "Deepglow Skate");
    play(
        &mut g,
        skate,
        false,
        &[],
        &[
            Action::Target(Target::Object(champ)),
            Action::Target(Target::Object(kaya)),
            Action::No,
        ],
    );
    assert_eq!(g.obj(champ).counters, 6);
    assert_eq!(g.obj(kaya).loyalty, 6);
}

#[test]
fn become_subtype_until_eot_bogardan_dragonheart() {
    let mut g = table();
    let heart = put(&mut g, 0, "Bogardan Dragonheart");
    let lions = put(&mut g, 0, "Savannah Lions");
    act(&mut g, Action::ActivateWith(heart, 0, lions));
    resolve_all(&mut g, &[]);
    assert_eq!((g.power(heart), g.toughness(heart)), (4, 4));
    assert!(g.keywords(heart).has(Keywords::FLYING) && g.keywords(heart).has(Keywords::HASTE));
    assert!(g.has_subtype(heart, "Dragon"));
    g.start_turn(5, 0);
    assert_eq!(g.power(heart), 2);
}

#[test]
fn reanimate_unearthed_whip_of_erebos() {
    let mut g = table();
    put(&mut g, 0, "Whip of Erebos");
    let elder = g.add_card(0, cid("Silverback Elder"), Zone::Graveyard);
    let whip = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Whip of Erebos")
        .unwrap();
    act(&mut g, Action::Activate(whip, 0));
    act(&mut g, Action::Target(Target::Object(elder)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(elder).zone, Zone::Battlefield);
    assert!(g.keywords(elder).has(Keywords::HASTE) && g.keywords(elder).has(Keywords::LIFELINK));
    g.objects[elder.0 as usize].damage = 7;
    act(&mut g, Action::PassPriority);
    assert_eq!(g.obj(elder).zone, Zone::Exile, "exiled instead of dying");
}

#[test]
fn next_draw_deals_damage_words_of_war() {
    let mut g = table();
    let words = put(&mut g, 0, "Words of War");
    act(&mut g, Action::Activate(words, 0));
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    let n = g.players[0].hand.len();
    let search = hand(&mut g, 0, "Frantic Search"); // draw two: the first is replaced
    act(&mut g, Action::Cast(search));
    pass_until_asked(&mut g);
    assert_eq!(g.players[1].life, 38);
    assert_eq!(g.players[0].hand.len(), n + 1, "one card drawn, one replaced");
}

#[test]
fn opponents_cards_exiled_with_void_and_played_free() {
    // A test definition: Savannah Lions with Dauthi Voidwalker's replacement.
    let mut defs = (*set("CMD").unwrap().defs).clone();
    let lions = cid("Savannah Lions");
    defs[lions].opponents_cards_exiled_with_void = true;
    defs[lions].activated.clear();
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
    put(&mut g, 0, "Savannah Lions");
    let theirs = put(&mut g, 1, "Silverback Elder");
    let bolt = hand(&mut g, 0, "Burst Lightning");
    g.objects[theirs.0 as usize].damage = 5;
    act(&mut g, Action::Cast(bolt));
    act(&mut g, Action::Target(Target::Object(theirs)));
    resolve_all(&mut g, &[]);
    assert_eq!(g.obj(theirs).zone, Zone::Exile);
    assert!(g.obj(theirs).void_counter);
    assert_eq!(g.obj(bolt).zone, Zone::Graveyard, "my own card isn't affected");
}

// ---- Lane B, 2026-11-26: Class token replacements (Fisher's Talent), Gisa's exile ----

/// The table, with the CMD definitions changed by `edit` first.
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
fn fishers_talent_replaces_fish_with_sharks_and_sharks_with_octopuses_by_level() {
    let t = bolt_cards::scripts::Tokens::starting_at(set("CMD").unwrap().card_count());
    for (level, made) in [(1, "Fish"), (2, "Shark"), (3, "Octopus")] {
        let mut g = table_with(|defs| {
            defs[cid("Fisher's Talent")].class_token_replacements =
                vec![(2, t.fish, t.shark), (3, t.shark, t.octopus)];
        });
        let talent = put(&mut g, 0, "Fisher's Talent");
        g.objects[talent.0 as usize].class_level = level;
        let port = put(&mut g, 0, "Fountainport");
        act(&mut g, Action::Activate(port, 1)); // {3}, {T}, Pay 1 life: a Fish
        resolve_all(&mut g, &[]);
        let made_ids: Vec<ObjId> = g
            .permanents(0)
            .filter(|&x| g.is_token(x) && g.def_of(x).name == made)
            .collect();
        assert_eq!(made_ids.len(), 1, "level {level}: a {made}");
        assert_eq!(
            g.colors_of(made_ids[0]),
            [false, true, false, false, false],
            "blue"
        );
        if made == "Shark" {
            assert_eq!((g.power(made_ids[0]), g.toughness(made_ids[0])), (3, 3));
        }
    }
}

#[test]
fn gisa_exiles_opponents_dying_creatures_and_remembers_them() {
    let mut g =
        table_with(|defs| defs[cid("Gisa, Glorious Resurrector")].opponents_creatures_die_exiled = true);
    let gisa = put(&mut g, 0, "Gisa, Glorious Resurrector");
    let theirs = put(&mut g, 1, "Savannah Lions");
    let mine = put(&mut g, 0, "Savannah Lions");
    for target in [theirs, mine] {
        let bolt = hand(&mut g, 0, "Lightning Bolt");
        play(&mut g, bolt, false, &[Target::Object(target)], &[]);
    }
    assert_eq!(
        g.obj(theirs).zone,
        Zone::Exile,
        "an opponent's creature is exiled instead"
    );
    assert_eq!(g.obj(mine).zone, Zone::Graveyard, "yours still dies");
    assert!(g
        .exiled_with
        .iter()
        .any(|&(s, _, c, z)| s == gisa && c == theirs && z == g.obj(theirs).zcc));
}

// ---- Pieces for lane C's cards: Inkshield, The One Ring (2026-11-27) ----

#[test]
fn inkshield_prevents_combat_damage_to_you_and_makes_inklings() {
    let t = bolt_cards::scripts::Tokens::starting_at(set("CMD").unwrap().card_count());
    let mut g = table_with(|defs| {
        defs[cid("Inkshield")].spell = Some(bolt_core::Ability::new(
            vec![],
            vec![bolt_core::Effect::PreventCombatDamageToYouMakingTokens { def: t.inkling }],
        ));
    });
    let lions = put(&mut g, 1, "Savannah Lions");
    let shield = hand(&mut g, 0, "Inkshield");
    g.start_turn(4, 1);
    while g.pending != Pending::DeclareAttackers(1) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(lions));
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
    let inklings: Vec<ObjId> = g
        .permanents(0)
        .filter(|&x| g.is_token(x) && g.def_of(x).name == "Inkling")
        .collect();
    assert_eq!(inklings.len(), 2, "one per point prevented");
    assert_eq!(g.colors_of(inklings[0]), [true, false, true, false, false]);
    assert!(g.keywords(inklings[0]).has(Keywords::FLYING));
}

#[test]
fn protection_from_everything_until_your_next_turn_stops_damage_and_targeting() {
    let mut g = table_with(|defs| {
        defs[cid("Inkshield")].spell = Some(bolt_core::Ability::new(
            vec![],
            vec![bolt_core::Effect::YouGainProtectionUntilYourNextTurn],
        ));
    });
    let shield = hand(&mut g, 0, "Inkshield"); // standing in for The One Ring's enters trigger
    play(&mut g, shield, false, &[], &[]);
    let bolt = hand(&mut g, 0, "Lightning Bolt");
    act(&mut g, Action::Cast(bolt));
    assert!(
        !g.legal_actions().contains(&Action::Target(Target::Player(0))),
        "can't be targeted"
    );
    act(&mut g, Action::Target(Target::Player(1)));
    resolve_all(&mut g, &[]);
    let lions = put(&mut g, 1, "Savannah Lions");
    g.start_turn(4, 1);
    while g.pending != Pending::DeclareAttackers(1) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(lions));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        let a = match g.pending {
            Pending::DeclareBlockers(_) => Action::FinishBlocks,
            _ => Action::PassPriority,
        };
        act(&mut g, a);
    }
    assert_eq!(g.players[0].life, 40, "damage prevented until your next turn");
}
