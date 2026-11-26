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
const ECHOER: usize = 18; // 2/2 {B}, echo {1}
const MASTERY: usize = 19; // instant {3}{B}, or {1}{B} and the opponent draws a card
const LANDLOCK: usize = 20; // sorcery {B}: tap all lands the opponent controls
const HANDMANA: usize = 21; // sorcery {B}: {R} per card in target player's hand
const IMPULSE: usize = 22; // sorcery {B}: exile the top two, playable this turn
const SONIC: usize = 23; // 1/1 {B}, enters: damage equal to spells cast this turn to any target

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
    let echo = ManaCost::parse("{1}").expect("cost");
    let echoer = creature(
        "Echoer",
        "{B}",
        2,
        2,
        When::Upkeep { your_turn_only: true },
        Effect::Echo(echo),
    );
    let sorcery = |name: &str, targets: Vec<Filter>, e: Effect| {
        CardDef::spell(name, "{B}", false, Ability::new(targets, vec![e]))
    };
    let mut mastery = CardDef::spell(
        "Mastery",
        "{3}{B}",
        true,
        Ability::new(vec![], vec![Effect::OpponentDrawsIfAltCost(1)]),
    );
    mastery.alt_cost = Some((ManaCost::parse("{1}{B}").expect("cost"), 0));
    let mut sonic = CardDef::creature("Sonic", "{B}", 1, 1, Keywords::NONE);
    sonic.triggers.push(Trigger {
        when: When::Enters,
        ability: Ability::new(vec![Filter::ANY], vec![Effect::DamagePerSpellsCastThisTurn(0)]),
    });
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
        echoer,
        mastery,
        sorcery("Landlock", vec![], Effect::TapOpponentsLands),
        sorcery(
            "Handmana",
            vec![Filter::PLAYER],
            Effect::AddManaPerHandSize {
                color: Color::Red,
                slot: 0,
            },
        ),
        sorcery("Impulse", vec![], Effect::ExileTopToPlayThisTurn(2)),
        sonic,
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

/// Plays on to player 0's next turn (turn 5), stopping at the first decision or the main phase.
fn next_turn(g: &mut Game) {
    g.start_turn(5, 0);
    while g.step != Step::Main1 && !matches!(g.pending, Pending::Decide(_)) {
        act(g, Action::PassPriority);
    }
}

#[test]
fn echo_is_paid_or_sacrificed_once() {
    // Paid: it stays, and echo isn't asked again.
    let mut g = board(&[SWAMP, SWAMP]);
    let e = g.add_card(0, ECHOER, Zone::Hand);
    act(&mut g, Action::Cast(e));
    settle(&mut g);
    next_turn(&mut g);
    assert!(matches!(g.pending, Pending::Decide(_)), "echo is due");
    assert!(g.legal_actions().contains(&Action::Yes));
    act(&mut g, Action::Yes);
    settle(&mut g);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    assert_eq!(g.obj(e).zone, Zone::Battlefield, "paid");
    g.start_turn(7, 0);
    while g.step != Step::Main1 {
        assert!(
            !matches!(g.pending, Pending::Decide(_)),
            "no echo after the first upkeep"
        );
        act(&mut g, Action::PassPriority);
    }
    // Not paid: sacrificed.
    let mut g = board(&[SWAMP, SWAMP]);
    let e = g.add_card(0, ECHOER, Zone::Hand);
    act(&mut g, Action::Cast(e));
    settle(&mut g);
    next_turn(&mut g);
    act(&mut g, Action::No);
    settle(&mut g);
    assert_eq!(g.obj(e).zone, Zone::Graveyard, "not paid: sacrificed");
}

#[test]
fn opponent_draws_only_if_the_alternative_cost_was_paid() {
    let mut g = board(&[SWAMP, SWAMP, SWAMP, SWAMP]);
    let m = g.add_card(0, MASTERY, Zone::Hand);
    let n = g.players[1].hand.len();
    act(&mut g, Action::CastAlternative(m));
    settle(&mut g);
    assert_eq!(g.players[1].hand.len(), n + 1, "{{1}}{{B}} paid");
    assert_eq!(g.permanents(0).filter(|&x| g.obj(x).tapped).count(), 2);
    let mut g2 = board(&[SWAMP, SWAMP, SWAMP, SWAMP]);
    let m2 = g2.add_card(0, MASTERY, Zone::Hand);
    let n = g2.players[1].hand.len();
    act(&mut g2, Action::Cast(m2));
    settle(&mut g2);
    assert_eq!(g2.players[1].hand.len(), n, "the mana cost: no card");
}

#[test]
fn tap_opponents_lands_taps_only_theirs() {
    let mut g = board(&[SWAMP]);
    let theirs: Vec<ObjId> = (0..3).map(|_| put(&mut g, 1, FOREST)).collect();
    let bear = put(&mut g, 1, BEAR);
    let mine = put(&mut g, 0, FOREST);
    let c = g.add_card(0, LANDLOCK, Zone::Hand);
    act(&mut g, Action::Cast(c));
    settle(&mut g);
    assert!(theirs.iter().all(|&l| g.obj(l).tapped));
    assert!(
        !g.obj(bear).tapped && !g.obj(mine).tapped,
        "lands, the opponent's"
    );
}

#[test]
fn add_mana_per_hand_size_counts_the_target_players_hand() {
    let mut g = board(&[SWAMP]);
    for _ in 0..3 {
        g.add_card(1, BEAR, Zone::Hand);
    }
    let c = g.add_card(0, HANDMANA, Zone::Hand);
    let n = g.players[1].hand.len();
    act(&mut g, Action::Cast(c));
    act(&mut g, Action::Target(Target::Player(1)));
    settle(&mut g);
    assert_eq!(pool_of(&g, 3), n);
}

#[test]
fn exile_top_to_play_this_turn_lasts_only_this_turn() {
    let mut g = board(&[SWAMP, FOREST, FOREST]);
    let top = g.add_card(0, ELF, Zone::Library);
    let second = g.add_card(0, FOREST, Zone::Library);
    let c = g.add_card(0, IMPULSE, Zone::Hand);
    act(&mut g, Action::Cast(c));
    settle(&mut g);
    assert_eq!((g.obj(top).zone, g.obj(second).zone), (Zone::Exile, Zone::Exile));
    assert!(
        g.legal_actions().contains(&Action::Cast(second))
            || g.legal_actions().contains(&Action::PlayLand(second))
    );
    assert!(
        g.legal_actions().contains(&Action::Cast(top)) || g.legal_actions().contains(&Action::PlayLand(top))
    );
    g.start_turn(5, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let legal = g.legal_actions();
    assert!(
        !legal.contains(&Action::PlayLand(second)) && !legal.contains(&Action::PlayLand(top)),
        "next turn: gone"
    );
}

#[test]
fn damage_per_spells_cast_this_turn() {
    let mut g = board(&[SWAMP, SWAMP, SWAMP]);
    let c = g.add_card(0, LANDLOCK, Zone::Hand);
    act(&mut g, Action::Cast(c));
    settle(&mut g);
    let s = g.add_card(0, SONIC, Zone::Hand);
    let life = g.players[1].life;
    act(&mut g, Action::Cast(s));
    while !matches!(g.pending, Pending::ChooseTarget(_)) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Target(Target::Player(1)));
    settle(&mut g);
    assert_eq!(g.players[1].life, life - 2, "Landlock and Sonic itself");
}

// ---- batch 3: sacrifice-only mana sources, {C}, static cost reductions, fear and intimidate,
// lands that enter tapped unless ----

mod batch3 {
    use super::*;
    use bolt_core::card::ManaExt;
    use bolt_core::{ManaSource, Types};

    pub const SWAMP: usize = 0;
    pub const SPAWN: usize = 1; // 0/1 token: "Sacrifice this creature: Add {C}."
    pub const ONE: usize = 2; // instant {1}: draw a card
    pub const CEE: usize = 3; // instant {C}: draw a card
    pub const WASTES: usize = 4; // land: {T}: Add {C}
    pub const NATURALIST: usize = 5; // enchantment spells cost {1} less
    pub const ENCH: usize = 6; // enchantment {2}
    pub const MONUMENT: usize = 7; // black creature spells cost {1} less
    pub const BLACK_BEAR: usize = 8; // black 2/2 {1}{B}
    pub const ANIMAR: usize = 9; // creature spells cost {1} less per +1/+1 counter on it
    pub const MAGNUS: usize = 10; // instants and sorceries cost {1} less per creature token
    pub const FEARFUL: usize = 11; // green 2/2 with fear
    pub const MENACING: usize = 12; // green 2/2 with intimidate
    pub const GREEN_BEAR: usize = 13;
    pub const BLACK_KNIGHT: usize = 14;
    pub const GOLEM: usize = 15; // colorless artifact creature
    pub const RED_BEAR: usize = 16;
    pub const RIDGE: usize = 17; // land: enters tapped unless you control two or more other lands
    pub const BIG_ONE: usize = 18; // sorcery {3}: draw a card
    pub const FREYA: usize = 19; // sorcery {0}: draw a card for each green creature you control
    pub const AURA: usize = 20; // Aura: enchanted land adds one more mana of any color
    pub const GHAST: usize = 21; // creature: your Swamps add an additional {B}
    pub const LANTERN: usize = 22; // artifact: your lands tap for any color
    pub const GROVE: usize = 23; // land: {C}, or {R}/{G} and each opponent gains 1 life
    pub const EVOKER: usize = 24; // 2/2 {3}, evoke {1}

    fn ext(f: impl FnOnce(&mut ManaExt)) -> ManaExt {
        let mut e = ManaExt::default();
        f(&mut e);
        e
    }

    fn colored(name: &str, cost: &str, color: usize, k: Keywords) -> CardDef {
        let mut d = CardDef::creature(name, cost, 2, 2, k);
        d.colors[color] = true;
        d
    }

    pub fn defs() -> Arc<Vec<CardDef>> {
        let draw = || Ability::new(vec![], vec![Effect::Draw { who: Who::You, n: 1 }]);
        let mut spawn = CardDef::creature("Spawn", "", 0, 1, Keywords::NONE);
        spawn.token = true;
        spawn.mana = Some(ManaSource {
            colorless: true,
            ..ManaSource::default()
        });
        spawn.sac_for_mana = true;
        spawn.mana_ext.sac_mana_no_tap = true;
        let mut wastes = super::land("Wastes");
        wastes.mana = Some(ManaSource {
            colorless: true,
            ..ManaSource::default()
        });
        let mut naturalist = CardDef::creature("Naturalist", "{G}", 1, 1, Keywords::NONE);
        naturalist.mana_ext = ext(|e| e.enchantment_spell_discount = 1);
        let ench = CardDef {
            name: "Ench".into(),
            cost: ManaCost::parse("{2}").expect("cost"),
            types: Types {
                enchantment: true,
                ..Types::default()
            },
            ..CardDef::default()
        };
        let monument = CardDef {
            name: "Monument".into(),
            types: Types {
                artifact: true,
                ..Types::default()
            },
            mana_ext: ext(|e| e.black_creature_spell_discount = 1),
            ..CardDef::default()
        };
        let mut animar = CardDef::creature("Animar", "{G}", 1, 1, Keywords::NONE);
        animar.mana_ext = ext(|e| e.creature_discount_per_counter = true);
        let mut magnus = CardDef::creature("Magnus", "{R}", 4, 5, Keywords::NONE);
        magnus.mana_ext = ext(|e| e.instant_sorcery_discount_per_creature_token = true);
        let mut golem = CardDef::creature("Golem", "{3}", 2, 2, Keywords::NONE);
        golem.types.artifact = true;
        let mut ridge = CardDef::basic_land("Ridge", Color::Red);
        ridge.types.basic = false;
        ridge.subtypes.clear();
        ridge.mana_ext = ext(|e| e.enters_tapped_unless_other_lands = 2);
        Arc::new(vec![
            CardDef::basic_land("Swamp", Color::Black),
            spawn,
            CardDef::spell("One", "{1}", true, draw()),
            CardDef::spell("Cee", "{C}", true, draw()),
            wastes,
            naturalist,
            ench,
            monument,
            colored("Black Bear", "{1}{B}", 2, Keywords::NONE),
            animar,
            magnus,
            colored("Fearful", "{1}{G}", 4, Keywords::FEAR),
            colored("Menacing", "{1}{G}", 4, Keywords::INTIMIDATE),
            colored("Green Bear", "{1}{G}", 4, Keywords::NONE),
            colored("Black Knight", "{1}{B}", 2, Keywords::NONE),
            golem,
            colored("Red Bear", "{1}{R}", 3, Keywords::NONE),
            ridge,
            CardDef::spell("Big One", "{3}", false, draw()),
            CardDef::spell(
                "Freya",
                "",
                false,
                Ability::new(
                    vec![],
                    vec![Effect::DrawPerCreatureYouControlOfColor(Color::Green)],
                ),
            ),
            CardDef {
                name: "Aura".into(),
                types: Types {
                    enchantment: true,
                    ..Types::default()
                },
                aura: true,
                mana_ext: ext(|e| e.enchanted_land_extra_any = true),
                ..CardDef::default()
            },
            {
                let mut d = CardDef::creature("Ghast", "{B}", 2, 2, Keywords::NONE);
                d.mana_ext = ext(|e| e.your_subtype_lands_add = Some(("Swamp", Color::Black)));
                d
            },
            CardDef {
                name: "Lantern".into(),
                types: Types {
                    artifact: true,
                    ..Types::default()
                },
                mana_ext: ext(|e| e.lands_tap_any_color = true),
                ..CardDef::default()
            },
            {
                let mut d = super::land("Grove");
                let mut m = ManaSource::of(&[Color::Red, Color::Green]);
                m.colorless = true;
                d.mana = Some(m);
                d.mana_ext = ext(|e| e.colored_gives_opponent_life = true);
                d
            },
            {
                let mut d = CardDef::creature("Evoker", "{3}", 2, 2, Keywords::NONE);
                d.alt_cost = Some((ManaCost::parse("{1}").expect("cost"), 0));
                d.mana_ext.evoke = true;
                d.triggers.push(Trigger {
                    when: When::Enters,
                    ability: Ability::new(vec![], vec![Effect::SacrificeIfEvoked]),
                });
                d
            },
        ])
    }

    pub fn board(lands: &[usize]) -> Game {
        let mut g = Game::empty(defs(), 1, 0);
        for p in 0..2 {
            for _ in 0..10 {
                g.add_card(p, GREEN_BEAR, Zone::Library);
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

    pub fn tapped_swamps(g: &Game) -> usize {
        g.permanents(0)
            .filter(|&x| g.obj(x).tapped && g.def_of(x).name == "Swamp")
            .count()
    }

    /// Casts `def` from player 0's hand and resolves it.
    pub fn cast(g: &mut Game, def: usize) -> ObjId {
        let c = g.add_card(0, def, Zone::Hand);
        act(g, Action::Cast(c));
        settle(g);
        c
    }
}

#[test]
fn a_sacrifice_only_mana_source_works_summoning_sick_or_tapped() {
    use batch3::*;
    let mut g = board(&[]);
    // Entered this turn: no {T} in its cost, so it can still pay.
    let spawn = put(&mut g, 0, SPAWN);
    g.objects[spawn.0 as usize].since_turn = g.turn;
    let c = g.add_card(0, ONE, Zone::Hand);
    assert!(g.legal_actions().contains(&Action::Cast(c)));
    act(&mut g, Action::Cast(c));
    settle(&mut g);
    assert_ne!(g.obj(spawn).zone, Zone::Battlefield, "sacrificed for the mana");
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
    // A tapped one is still a source.
    let spawn = put(&mut g, 0, SPAWN);
    g.objects[spawn.0 as usize].tapped = true;
    let made: Vec<_> = g.mana_sources(0).into_iter().filter(|s| s.0 == spawn).collect();
    assert_eq!(made.len(), 1);
    assert!(made[0].1.colorless);
}

#[test]
fn colorless_pips_need_colorless_mana() {
    use batch3::*;
    let mut g = board(&[SWAMP, SWAMP]);
    let c = g.add_card(0, CEE, Zone::Hand);
    assert!(
        !g.legal_actions().contains(&Action::Cast(c)),
        "a Swamp can't pay {{C}}"
    );
    let w = put(&mut g, 0, WASTES);
    act(&mut g, Action::Cast(c));
    settle(&mut g);
    assert!(g.obj(w).tapped);
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
}

#[test]
fn static_spell_discounts_by_type_color_counters_and_tokens() {
    use batch3::*;
    let five = [SWAMP; 5];
    let mut g = board(&five);
    put(&mut g, 0, NATURALIST);
    cast(&mut g, ENCH);
    assert_eq!(tapped_swamps(&g), 1, "enchantment {{2}} less {{1}}");

    let mut g = board(&five);
    put(&mut g, 0, MONUMENT);
    cast(&mut g, BLACK_BEAR);
    assert_eq!(tapped_swamps(&g), 1, "black creature {{1}}{{B}} less {{1}}");
    cast(&mut g, ENCH);
    assert_eq!(tapped_swamps(&g), 3, "not a black creature: full price");

    let mut g = board(&five);
    let a = put(&mut g, 0, ANIMAR);
    g.objects[a.0 as usize].counters = 1;
    cast(&mut g, BLACK_BEAR);
    assert_eq!(tapped_swamps(&g), 1, "creature {{1}}{{B}} less {{1}} per counter");

    let mut g = board(&five);
    put(&mut g, 0, MAGNUS);
    put(&mut g, 0, GOLEM); // not a token: no discount
    let spawns = [put(&mut g, 0, SPAWN), put(&mut g, 0, SPAWN)];
    cast(&mut g, BIG_ONE);
    let sacrificed = spawns
        .iter()
        .filter(|&&s| g.obj(s).zone != Zone::Battlefield)
        .count();
    assert_eq!(
        tapped_swamps(&g) + sacrificed,
        1,
        "sorcery {{3}} less {{2}} for two creature tokens"
    );
}

/// Attacks with `attacker` and returns the creatures player 1 may block it with.
fn blockers_for(g: &mut Game, attacker: ObjId) -> Vec<ObjId> {
    while g.pending != Pending::DeclareAttackers(0) {
        act(g, Action::PassPriority);
    }
    act(g, Action::Attack(attacker));
    act(g, Action::FinishAttacks);
    while !matches!(g.pending, Pending::DeclareBlockers(_)) {
        act(g, Action::PassPriority);
    }
    g.legal_actions()
        .into_iter()
        .filter_map(|a| match a {
            Action::Block { blocker, attacker: x } if x == attacker => Some(blocker),
            _ => None,
        })
        .collect()
}

#[test]
fn fear_and_intimidate_limit_blockers() {
    use batch3::*;
    for (kind, ok) in [
        (FEARFUL, [false, true, true, false]),
        (MENACING, [true, false, true, false]),
    ] {
        let mut g = board(&[]);
        let a = put(&mut g, 0, kind);
        let blockers = [GREEN_BEAR, BLACK_KNIGHT, GOLEM, RED_BEAR].map(|b| put(&mut g, 1, b));
        let able = blockers_for(&mut g, a);
        for (b, want) in blockers.iter().zip(ok) {
            assert_eq!(
                able.contains(b),
                want,
                "{} blocked by {}",
                g.def_of(a).name,
                g.def_of(*b).name
            );
        }
    }
}

#[test]
fn enters_tapped_unless_you_control_n_other_lands() {
    use batch3::*;
    for (lands, tapped) in [(&[SWAMP][..], true), (&[SWAMP, SWAMP][..], false)] {
        let mut g = board(lands);
        let r = g.add_card(0, RIDGE, Zone::Hand);
        act(&mut g, Action::PlayLand(r));
        assert_eq!(g.obj(r).tapped, tapped, "{} other lands", lands.len());
    }
}

#[test]
fn pay_x_life_casts_with_x_up_to_your_life() {
    use bolt_core::card::ManaExt;
    let mut deluge = CardDef::spell(
        "Deluge",
        "{B}",
        false,
        Ability::new(vec![], vec![Effect::AllCreaturesGetMinusX]),
    );
    deluge.mana_ext = ManaExt {
        pay_x_life: true,
        ..ManaExt::default()
    };
    let mut g = Game::empty(
        Arc::new(vec![
            CardDef::basic_land("Swamp", Color::Black),
            deluge,
            CardDef::creature("Bear", "{1}{G}", 2, 2, Keywords::NONE),
        ]),
        1,
        0,
    );
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, 2, Zone::Library);
        }
    }
    g.add_card(0, 0, Zone::Battlefield);
    g.auto_pass = false;
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g.players[0].life = 5;
    let bear = g.add_card(1, 2, Zone::Battlefield);
    let c = g.add_card(0, 1, Zone::Hand);
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::CastX(c, 5)) && !legal.contains(&Action::CastX(c, 6)));
    act(&mut g, Action::CastX(c, 2));
    settle(&mut g);
    assert_eq!(g.players[0].life, 3);
    assert_eq!(g.obj(bear).zone, Zone::Graveyard, "-2/-2");
}

#[test]
fn draw_per_creature_you_control_of_a_color() {
    use batch3::*;
    let mut g = board(&[]);
    put(&mut g, 0, GREEN_BEAR);
    put(&mut g, 0, GREEN_BEAR);
    put(&mut g, 0, BLACK_KNIGHT);
    put(&mut g, 1, GREEN_BEAR);
    let n = g.players[0].hand.len();
    cast(&mut g, FREYA);
    assert_eq!(g.players[0].hand.len(), n + 2, "two green creatures of yours");
}

fn sources_of(g: &Game, id: ObjId) -> Vec<bolt_core::ManaSource> {
    g.mana_sources(0)
        .into_iter()
        .filter(|s| s.0 == id)
        .map(|s| s.1)
        .collect()
}

#[test]
fn extra_mana_from_a_land_aura_and_from_subtype_adders() {
    use batch3::*;
    let mut g = board(&[SWAMP, SWAMP]);
    let lands: Vec<ObjId> = g.permanents(0).collect();
    let aura = put(&mut g, 0, AURA);
    let zcc = g.obj(lands[0]).zcc;
    g.objects[aura.0 as usize].attached_to = Some((lands[0], zcc));
    assert_eq!(sources_of(&g, lands[0]).len(), 2);
    assert!(sources_of(&g, lands[0])[0].colors == [true; 5]);
    assert_eq!(sources_of(&g, lands[1]).len(), 1, "not enchanted");
    put(&mut g, 0, GHAST);
    assert_eq!(sources_of(&g, lands[1]).len(), 2, "a Swamp adds {{B}} more");
    assert_eq!(sources_of(&g, lands[0]).len(), 3);
    // Paying uses the extra mana: {1}{B}... two Swamps make four.
    g.manual_mana[0] = true;
    act(&mut g, Action::TapForMana(lands[1], 2));
    assert_eq!(g.players[0].pool.len(), 2);
}

#[test]
fn lands_tap_for_any_color_with_a_lantern() {
    use batch3::*;
    let mut g = board(&[SWAMP, WASTES]);
    let lands: Vec<ObjId> = g.permanents(0).collect();
    assert_eq!(
        sources_of(&g, lands[0])[0].colors,
        [false, false, true, false, false]
    );
    put(&mut g, 0, LANTERN);
    assert_eq!(sources_of(&g, lands[0])[0].colors, [true; 5]);
    let wastes = sources_of(&g, lands[1])[0];
    assert!(wastes.colors == [true; 5] && wastes.colorless, "still {{C}} too");
}

#[test]
fn colored_mana_that_gives_the_opponent_life() {
    use batch3::*;
    let mut g = board(&[GROVE, GROVE]);
    let groves: Vec<ObjId> = g.permanents(0).collect();
    g.manual_mana[0] = true;
    act(&mut g, Action::TapForMana(groves[0], 5));
    assert_eq!(g.players[1].life, 20, "{{C}}: no life");
    act(&mut g, Action::TapForMana(groves[1], 4));
    assert_eq!(g.players[1].life, 21);
    // Paid automatically: colored mana only when needed.
    let mut g = board(&[GROVE]);
    let c = g.add_card(0, ONE, Zone::Hand);
    act(&mut g, Action::Cast(c));
    settle(&mut g);
    assert_eq!(g.players[1].life, 20, "{{1}} paid with {{C}}");
    let mut g = board(&[GROVE]);
    let c = g.add_card(0, GREEN_BEAR, Zone::Hand);
    g.players[0].pool.push(bolt_core::ManaSource {
        colorless: true,
        ..bolt_core::ManaSource::default()
    });
    act(&mut g, Action::Cast(c));
    settle(&mut g);
    assert_eq!(g.players[1].life, 21, "{{G}} from the Grove");
}

#[test]
fn evoke_is_recorded_and_sacrifices_only_the_evoked_cast() {
    use batch3::*;
    let mut g = board(&[SWAMP, SWAMP, SWAMP, SWAMP]);
    let c = g.add_card(0, EVOKER, Zone::Hand);
    act(&mut g, Action::CastAlternative(c));
    settle(&mut g);
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
    assert_eq!(tapped_swamps(&g), 1, "evoke {{1}}");
    // The same card, cast again for its mana cost, stays.
    let c = g.add_card(0, EVOKER, Zone::Hand);
    act(&mut g, Action::Cast(c));
    settle(&mut g);
    assert_eq!(g.obj(c).zone, Zone::Battlefield);
}

#[test]
fn convoke_taps_creatures_for_generic_or_their_color() {
    use batch3::*;
    let mut d = (*defs()).clone();
    let mut chord = CardDef::spell(
        "Convoker",
        "{2}{B}",
        true,
        Ability::new(vec![], vec![Effect::Draw { who: Who::You, n: 1 }]),
    );
    chord.mana_ext.convoke = true;
    d.push(chord);
    let mut g = Game::empty(Arc::new(d), 1, 0);
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, GREEN_BEAR, Zone::Library);
        }
    }
    g.auto_pass = false;
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let green = put(&mut g, 0, GREEN_BEAR);
    let golem = put(&mut g, 0, GOLEM);
    let c = g.add_card(0, 25, Zone::Hand);
    assert!(
        !g.legal_actions().contains(&Action::Cast(c)),
        "no black creature or mana for {{B}}"
    );
    let knight = put(&mut g, 0, BLACK_KNIGHT);
    g.objects[knight.0 as usize].since_turn = g.turn;
    act(&mut g, Action::Cast(c));
    settle(&mut g);
    assert!(g.obj(green).tapped && g.obj(golem).tapped && g.obj(knight).tapped);
    assert_eq!(g.obj(c).zone, Zone::Graveyard);
}

#[test]
fn colored_spell_discount_all_lands_add_same_and_creature_adders() {
    use batch3::*;
    let mut d = (*defs()).clone();
    let mut jet = CardDef::creature("Jet", "{G}", 1, 1, Keywords::NONE);
    jet.mana_ext.colored_spell_discount = Some(([true, false, true, false, false], 1));
    let mut dictate = CardDef::creature("Dictate", "{G}", 1, 1, Keywords::NONE);
    dictate.mana_ext.all_lands_add_same = true;
    let mut leyline = CardDef::creature("Leyline", "{G}", 1, 1, Keywords::NONE);
    leyline.mana_ext.your_creatures_add = Some(Color::Green);
    let mut dork = CardDef::creature("Dork", "{G}", 1, 1, Keywords::NONE);
    dork.mana = Some(bolt_core::ManaSource::of(&[Color::Green]));
    d.extend([jet, dictate, leyline, dork]); // 25..=28
    let mut g = Game::empty(Arc::new(d), 1, 0);
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, GREEN_BEAR, Zone::Library);
        }
    }
    for _ in 0..3 {
        g.add_card(0, SWAMP, Zone::Battlefield);
    }
    g.auto_pass = false;
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    put(&mut g, 0, 25);
    let c = g.add_card(0, BLACK_BEAR, Zone::Hand);
    act(&mut g, Action::Cast(c));
    settle(&mut g);
    assert_eq!(tapped_swamps(&g), 1, "black {{1}}{{B}} less {{1}}");
    let swamp = g
        .permanents(0)
        .find(|&x| g.def_of(x).name == "Swamp" && !g.obj(x).tapped)
        .unwrap();
    assert_eq!(sources_of(&g, swamp).len(), 1);
    put(&mut g, 1, 26);
    assert_eq!(sources_of(&g, swamp).len(), 2, "an opponent's Dictate counts too");
    let dork = put(&mut g, 0, 28);
    assert_eq!(sources_of(&g, dork).len(), 1);
    put(&mut g, 0, 27);
    assert_eq!(sources_of(&g, dork).len(), 2);
    assert_eq!(sources_of(&g, swamp).len(), 2, "lands aren't creatures");
}
