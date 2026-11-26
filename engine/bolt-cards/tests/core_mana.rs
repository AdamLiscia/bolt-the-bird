//! Core theme d1 (mana, costs and keywords): its engine pieces tested directly, on hand-made cards.
//! The playgroup cards they unblock are tested in tests/pg_d1.rs.

use std::sync::Arc;

use bolt_core::effect::ManaCount;
use bolt_core::{
    Ability, Action, Activated, CardDef, Color, Effect, Event, Filter, Game, Keywords, ManaCost, ObjId,
    Pending, Reduction, Step, Target, Trigger, When, Who, Zone,
};

const SWAMP: usize = 0;
const FOREST: usize = 1;
const BEAR: usize = 2; // 2/2 {1}{G}
const EXALTER: usize = 3; // 0/1, exalted
const UNDYING: usize = 4; // 2/2, undying
const PERSIST: usize = 5; // 2/2, persist
const DOOM: usize = 6; // instant {B}: destroy target creature
const KNIGHT: usize = 7; // 2/2 {B}{B}
const MERCHANT: usize = 8; // 2/4 {3}{B}{B}, enters: drain devotion to black
const SHRINE: usize = 9; // land: {T}: devotion to black in {B}
const COFFERS: usize = 10; // land: {T}: {B} per Swamp you control
const STRONGHOLD: usize = 11; // land: {T}: {B} per basic Swamp you control
const PRIEST: usize = 12; // 1/1 Elf: {T}: {G} per Elf on the battlefield
const ELITE: usize = 13; // 3/3: {T}: {G} equal to its power
const STORY: usize = 14; // instant {5}{U}{U}: costs {3} less with 7+ cards in an opponent's graveyard
const NONBASIC_SWAMP: usize = 15; // a nonbasic land with the Swamp type
const ELF: usize = 16; // 1/1 Elf {G}
const ISLAND: usize = 17;

fn creature(name: &str, cost: &str, p: i32, t: i32, when: When, effect: Effect) -> CardDef {
    let mut d = CardDef::creature(name, cost, p, t, Keywords::NONE);
    d.triggers.push(Trigger {
        when,
        ability: Ability::new(vec![], vec![effect]),
    });
    d
}

fn tap_ability(d: &mut CardDef, effect: Effect) {
    d.activated.push(Activated {
        mana: ManaCost::default(),
        tap: true,
        ability: Ability::new(vec![], vec![effect]),
        ..Activated::default()
    });
}

fn land(name: &str) -> CardDef {
    let mut d = CardDef::basic_land(name, Color::Black);
    d.types.basic = false;
    d.mana = None;
    d.subtypes.clear();
    d
}

fn defs() -> Arc<Vec<CardDef>> {
    let mut doom = CardDef::spell(
        "Doom",
        "{B}",
        true,
        Ability::new(vec![Filter::CREATURE], vec![Effect::Destroy(0)]),
    );
    doom.colors[2] = true;
    let mut shrine = land("Shrine");
    tap_ability(&mut shrine, Effect::AddManaDevotion(Color::Black));
    let mut coffers = land("Coffers");
    tap_ability(
        &mut coffers,
        Effect::AddManaPerCount {
            color: Color::Black,
            count: ManaCount::YouControlSubtype("Swamp"),
        },
    );
    let mut stronghold = land("Stronghold");
    tap_ability(
        &mut stronghold,
        Effect::AddManaPerCount {
            color: Color::Black,
            count: ManaCount::YouControlBasicSubtype("Swamp"),
        },
    );
    let mut priest = CardDef::creature("Priest", "{1}{G}", 1, 1, Keywords::NONE);
    priest.subtypes = vec!["Elf".into()];
    tap_ability(
        &mut priest,
        Effect::AddManaPerCount {
            color: Color::Green,
            count: ManaCount::OnBattlefieldSubtype("Elf"),
        },
    );
    let mut elite = CardDef::creature("Elite", "{2}{G}", 3, 3, Keywords::NONE);
    tap_ability(&mut elite, Effect::AddManaSourcePower(Color::Green));
    let mut story = CardDef::spell(
        "Story",
        "{5}{U}{U}",
        true,
        Ability::new(vec![], vec![Effect::Draw { who: Who::You, n: 4 }]),
    );
    story.reduction = Some(Reduction::IfOpponentGraveyardAtLeast(7, 3));
    let mut nonbasic_swamp = CardDef::basic_land("Swampy", Color::Black);
    nonbasic_swamp.types.basic = false;
    nonbasic_swamp.subtypes = vec!["Swamp".into()];
    let mut elf = CardDef::creature("Elf", "{G}", 1, 1, Keywords::NONE);
    elf.subtypes = vec!["Elf".into()];
    Arc::new(vec![
        CardDef::basic_land("Swamp", Color::Black),
        CardDef::basic_land("Forest", Color::Green),
        CardDef::creature("Bear", "{1}{G}", 2, 2, Keywords::NONE),
        creature("Exalter", "{G}", 0, 1, When::YouAttack, Effect::Exalted),
        creature("Undying", "{1}{B}", 2, 2, When::Dies, Effect::Undying),
        creature("Persist", "{1}{B}", 2, 2, When::Dies, Effect::Persist),
        doom,
        CardDef::creature("Knight", "{B}{B}", 2, 2, Keywords::NONE),
        creature(
            "Merchant",
            "{3}{B}{B}",
            2,
            4,
            When::Enters,
            Effect::DrainDevotion([false, false, true, false, false]),
        ),
        shrine,
        coffers,
        stronghold,
        priest,
        elite,
        story,
        nonbasic_swamp,
        elf,
        CardDef::basic_land("Island", Color::Blue),
    ])
}

/// Player 0's main phase on turn 3, with `lands` for player 0 and libraries of Bears.
fn board(lands: &[usize]) -> Game {
    let mut g = Game::empty(defs(), 1, 0);
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, BEAR, Zone::Library);
        }
    }
    for &l in lands {
        g.add_card(0, l, Zone::Battlefield);
    }
    g.auto_pass = false;
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g
}

fn act(g: &mut Game, a: Action) -> Vec<Event> {
    g.apply(a)
        .unwrap_or_else(|e| panic!("{e}; legal: {:?}", g.legal_actions()))
}

fn put(g: &mut Game, p: usize, def: usize) -> ObjId {
    g.add_card(p, def, Zone::Battlefield)
}

/// Passes priority until the stack is empty and no decision is pending.
fn settle(g: &mut Game) {
    while !g.items.is_empty() {
        act(g, Action::PassPriority);
    }
}

/// Casts Doom (from player 0's hand) on `target`, paying with a Swamp.
fn doom(g: &mut Game, target: ObjId) {
    let c = g.add_card(0, DOOM, Zone::Hand);
    act(g, Action::Cast(c));
    act(g, Action::Target(Target::Object(target)));
    settle(g);
}

/// Attacks with `attackers` from player 0's main phase, and returns at the combat damage step's end.
fn attack(g: &mut Game, attackers: &[ObjId]) {
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    for &a in attackers {
        act(g, Action::Attack(a));
    }
    act(g, Action::FinishAttacks);
    while g.step != Step::EndCombat && g.step != Step::Main2 {
        match g.pending {
            Pending::DeclareBlockers(_) => {
                act(g, Action::FinishBlocks);
            }
            _ => {
                act(g, Action::PassPriority);
            }
        }
    }
}

#[test]
fn exalted_pumps_a_creature_attacking_alone() {
    let mut g = board(&[]);
    let bear = put(&mut g, 0, BEAR);
    put(&mut g, 0, EXALTER);
    put(&mut g, 0, EXALTER);
    let life = g.players[1].life;
    attack(&mut g, &[bear]);
    assert_eq!(g.players[1].life, life - 4, "two exalted instances: 2 + 1 + 1");
}

#[test]
fn exalted_does_nothing_for_two_attackers() {
    let mut g = board(&[]);
    let a = put(&mut g, 0, BEAR);
    let b = put(&mut g, 0, BEAR);
    put(&mut g, 0, EXALTER);
    let life = g.players[1].life;
    attack(&mut g, &[a, b]);
    assert_eq!(g.players[1].life, life - 4, "no bonus");
}

#[test]
fn undying_returns_once_with_a_plus_counter() {
    let mut g = board(&[SWAMP, SWAMP]);
    let u = put(&mut g, 0, UNDYING);
    doom(&mut g, u);
    assert_eq!(g.obj(u).zone, Zone::Battlefield, "returned");
    assert_eq!(g.obj(u).counters, 1);
    assert_eq!(g.power(u), 3);
    doom(&mut g, u);
    assert_eq!(g.obj(u).zone, Zone::Graveyard, "it had a +1/+1 counter");
}

#[test]
fn persist_returns_once_with_a_minus_counter() {
    let mut g = board(&[SWAMP, SWAMP]);
    let p = put(&mut g, 0, PERSIST);
    doom(&mut g, p);
    assert_eq!(g.obj(p).zone, Zone::Battlefield, "returned");
    assert_eq!(g.obj(p).counters, -1);
    assert_eq!((g.power(p), g.toughness(p)), (1, 1));
    doom(&mut g, p);
    assert_eq!(g.obj(p).zone, Zone::Graveyard, "it had a -1/-1 counter");
}

#[test]
fn drain_devotion_counts_black_symbols_and_gains_what_was_lost() {
    let mut g = board(&[SWAMP, SWAMP, SWAMP, SWAMP, SWAMP]);
    put(&mut g, 0, KNIGHT); // {B}{B}
    put(&mut g, 0, BEAR); // no black
    put(&mut g, 1, KNIGHT); // the opponent's: not yours
    let m = g.add_card(0, MERCHANT, Zone::Hand);
    let (mine, theirs) = (g.players[0].life, g.players[1].life);
    act(&mut g, Action::Cast(m));
    settle(&mut g);
    assert_eq!(
        g.players[1].life,
        theirs - 4,
        "{{B}}{{B}} and the Merchant's own {{B}}{{B}}"
    );
    assert_eq!(g.players[0].life, mine + 4);
}

fn pool_of(g: &Game, color: usize) -> usize {
    g.players[0].pool.iter().filter(|m| m.colors[color]).count()
}

#[test]
fn add_mana_devotion_makes_one_per_symbol() {
    let mut g = board(&[]);
    let s = put(&mut g, 0, SHRINE);
    put(&mut g, 0, KNIGHT);
    put(&mut g, 0, KNIGHT);
    act(&mut g, Action::Activate(s, 0));
    settle(&mut g);
    assert_eq!(g.players[0].pool.len(), 4);
    assert_eq!(pool_of(&g, 2), 4);
}

#[test]
fn add_mana_per_count_counts_swamps_basic_swamps_and_every_elf() {
    let mut g = board(&[SWAMP, SWAMP, NONBASIC_SWAMP, FOREST]);
    let c = put(&mut g, 0, COFFERS);
    put(&mut g, 1, SWAMP); // the opponent's: not counted
    act(&mut g, Action::Activate(c, 0));
    settle(&mut g);
    assert_eq!(pool_of(&g, 2), 3, "two basic Swamps and a nonbasic one");

    let mut g = board(&[SWAMP, SWAMP, NONBASIC_SWAMP]);
    let s = put(&mut g, 0, STRONGHOLD);
    act(&mut g, Action::Activate(s, 0));
    settle(&mut g);
    assert_eq!(pool_of(&g, 2), 2, "basic Swamps only");

    let mut g = board(&[]);
    let priest = g.add_card(0, PRIEST, Zone::Battlefield);
    g.start_turn(5, 0); // the Priest is no longer summoning sick
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    put(&mut g, 0, ELF);
    put(&mut g, 1, ELF);
    put(&mut g, 1, BEAR);
    act(&mut g, Action::Activate(priest, 0));
    settle(&mut g);
    assert_eq!(pool_of(&g, 4), 3, "the Priest and both players' Elves");
}

#[test]
fn add_mana_source_power_makes_its_power() {
    let mut g = board(&[]);
    let e = g.add_card(0, ELITE, Zone::Battlefield);
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g.objects[e.0 as usize].counters = 2;
    act(&mut g, Action::Activate(e, 0));
    settle(&mut g);
    assert_eq!(pool_of(&g, 4), 5);
}

#[test]
fn reduction_if_opponent_graveyard_at_least() {
    let mut g = board(&[ISLAND, ISLAND, SWAMP, SWAMP]);
    let s = g.add_card(0, STORY, Zone::Hand);
    for _ in 0..6 {
        g.add_card(1, BEAR, Zone::Graveyard);
    }
    assert!(
        !g.legal_actions().contains(&Action::Cast(s)),
        "six cards: {{5}}{{U}}{{U}} with four lands"
    );
    g.add_card(1, BEAR, Zone::Graveyard);
    assert!(
        g.legal_actions().contains(&Action::Cast(s)),
        "seven: {{2}}{{U}}{{U}}"
    );
    for _ in 0..3 {
        g.add_card(0, BEAR, Zone::Graveyard);
    }
    let mut g2 = board(&[ISLAND, ISLAND, SWAMP, SWAMP]);
    let s2 = g2.add_card(0, STORY, Zone::Hand);
    for _ in 0..7 {
        g2.add_card(0, BEAR, Zone::Graveyard);
    }
    assert!(
        !g2.legal_actions().contains(&Action::Cast(s2)),
        "your own graveyard doesn't count"
    );
}
