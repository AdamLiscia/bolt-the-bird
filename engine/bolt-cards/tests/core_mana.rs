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

#[test]
fn escape_casts_from_the_graveyard_exiling_others_and_adds_counters() {
    use batch3::*;
    let mut d = (*defs()).clone();
    let mut esc = CardDef::creature("Escaper", "{5}", 1, 1, Keywords::NONE);
    esc.mana_ext.escape = Some((ManaCost::parse("{1}").expect("cost"), 2, 3));
    d.push(esc); // 25
    let mut g = Game::empty(Arc::new(d), 1, 0);
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, GREEN_BEAR, Zone::Library);
        }
    }
    g.add_card(0, SWAMP, Zone::Battlefield);
    g.auto_pass = false;
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    let e = g.add_card(0, 25, Zone::Graveyard);
    g.add_card(0, ENCH, Zone::Graveyard);
    assert!(
        !g.legal_actions().contains(&Action::CastAlternative(e)),
        "one other card"
    );
    g.add_card(0, ENCH, Zone::Graveyard);
    act(&mut g, Action::CastAlternative(e));
    settle(&mut g);
    assert_eq!(g.obj(e).zone, Zone::Battlefield);
    assert_eq!(g.obj(e).counters, 3);
    assert!(g.players[0].graveyard.is_empty());
    assert_eq!(g.exile.len(), 2);
}

// ---- batch 6: attacking-alone and legendary-only block restrictions, tapping a legendary
// creature as a cost, a discount on every spell, free casts from hand ----

/// Player 0's main phase on turn 3 with `extra` appended to batch 3's cards (from index 25),
/// libraries of Green Bears and `swamps` Swamps for player 0.
fn batch6_board(extra: Vec<CardDef>, swamps: usize) -> Game {
    use batch3::*;
    let mut d = (*defs()).clone();
    d.extend(extra);
    let mut g = Game::empty(Arc::new(d), 1, 0);
    for p in 0..2 {
        for _ in 0..10 {
            g.add_card(p, GREEN_BEAR, Zone::Library);
        }
    }
    for _ in 0..swamps {
        g.add_card(0, SWAMP, Zone::Battlefield);
    }
    g.auto_pass = false;
    g.start_turn(3, 0);
    while g.step != Step::Main1 {
        act(&mut g, Action::PassPriority);
    }
    g
}

#[test]
fn unblockable_while_attacking_alone() {
    use batch3::*;
    let mut snake = CardDef::creature("Snake", "{1}{U}", 1, 1, Keywords::NONE);
    snake.mana_ext.unblockable_attacking_alone = true;
    let mut g = batch6_board(vec![snake], 0);
    let s = put(&mut g, 0, 25);
    put(&mut g, 1, GREEN_BEAR);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(s));
    act(&mut g, Action::FinishAttacks);
    while g.step != Step::Main2 {
        let block = g
            .legal_actions()
            .iter()
            .any(|a| matches!(a, Action::Block { .. }));
        assert!(!block, "attacking alone: unblockable");
        let a = if matches!(g.pending, Pending::DeclareBlockers(_)) {
            Action::FinishBlocks
        } else {
            Action::PassPriority
        };
        act(&mut g, a);
    }

    let mut snake = CardDef::creature("Snake", "{1}{U}", 1, 1, Keywords::NONE);
    snake.mana_ext.unblockable_attacking_alone = true;
    let mut g = batch6_board(vec![snake], 0);
    let s = put(&mut g, 0, 25);
    let other = put(&mut g, 0, GREEN_BEAR);
    let b2 = put(&mut g, 1, GREEN_BEAR);
    while g.pending != Pending::DeclareAttackers(0) {
        act(&mut g, Action::PassPriority);
    }
    act(&mut g, Action::Attack(s));
    act(&mut g, Action::Attack(other));
    act(&mut g, Action::FinishAttacks);
    while !matches!(g.pending, Pending::DeclareBlockers(_)) {
        act(&mut g, Action::PassPriority);
    }
    assert!(g.legal_actions().contains(&Action::Block {
        blocker: b2,
        attacker: s
    }));
}

#[test]
fn your_creatures_blocked_only_by_legendary_creatures() {
    use batch3::*;
    let mut lighter = CardDef::creature("Lighter", "{2}{W}{U}", 2, 2, Keywords::NONE);
    lighter.mana_ext.your_creatures_blocked_only_by_legendary = true;
    let mut legend = CardDef::creature("Legend", "{1}{G}", 2, 2, Keywords::NONE);
    legend.types.legendary = true;
    let mut g = batch6_board(vec![lighter, legend], 0);
    put(&mut g, 0, 25);
    let a = put(&mut g, 0, GREEN_BEAR);
    let blockers = [GREEN_BEAR, GOLEM, 26].map(|b| put(&mut g, 1, b));
    let able = blockers_for(&mut g, a);
    assert_eq!(able, vec![blockers[2]], "only the legendary creature");
}

#[test]
fn tap_an_untapped_legendary_creature_as_a_cost() {
    use batch3::*;
    let mut descent = land("Descent");
    descent.activated.push(Activated {
        mana: ManaCost::default(),
        tap: true,
        tap_untapped_legendary: true,
        ability: Ability::new(vec![], vec![Effect::Draw { who: Who::You, n: 1 }]),
        ..Activated::default()
    });
    let mut legend = CardDef::creature("Legend", "{1}{G}", 2, 2, Keywords::NONE);
    legend.types.legendary = true;
    let mut g = batch6_board(vec![descent, legend], 0);
    let d = put(&mut g, 0, 25);
    put(&mut g, 0, GREEN_BEAR);
    assert!(
        !g.legal_actions().contains(&Action::Activate(d, 0)),
        "no legendary creature"
    );
    let l = put(&mut g, 0, 26);
    g.objects[l.0 as usize].since_turn = g.turn; // summoning sick still pays a tap cost that isn't {T}
    let n = g.players[0].hand.len();
    act(&mut g, Action::Activate(d, 0));
    settle(&mut g);
    assert!(g.obj(d).tapped && g.obj(l).tapped);
    assert_eq!(g.players[0].hand.len(), n + 1);
    g.objects[d.0 as usize].tapped = false;
    assert!(
        !g.legal_actions().contains(&Action::Activate(d, 0)),
        "the legendary creature is tapped"
    );
}

#[test]
fn every_spell_costs_one_less() {
    use batch3::*;
    let sun = CardDef {
        name: "Sun".into(),
        types: bolt_core::Types {
            artifact: true,
            ..bolt_core::Types::default()
        },
        mana_ext: bolt_core::card::ManaExt {
            spell_discount_all: 1,
            ..bolt_core::card::ManaExt::default()
        },
        ..CardDef::default()
    };
    let mut g = batch6_board(vec![sun], 5);
    put(&mut g, 0, 25);
    cast(&mut g, BIG_ONE);
    assert_eq!(tapped_swamps(&g), 2, "{{3}} less {{1}}");
    cast(&mut g, BLACK_BEAR);
    assert_eq!(tapped_swamps(&g), 3, "{{1}}{{B}} less {{1}}");
    put(&mut g, 1, 25);
    cast(&mut g, BIG_ONE);
    assert_eq!(
        tapped_swamps(&g),
        5,
        "an opponent's Sun doesn't discount your spells"
    );
}

#[test]
fn a_free_cast_from_hand_casts_one_spell_up_to_the_mana_value() {
    use batch3::*;
    let expert = CardDef::spell(
        "Expert",
        "{B}",
        false,
        Ability::new(vec![], vec![Effect::MayCastFromHandFree { max_mv: 5 }]),
    );
    let six = CardDef::creature("Six", "{5}{G}", 6, 6, Keywords::NONE);
    let mut g = batch6_board(vec![expert, six], 1);
    let big = [
        g.add_card(0, BIG_ONE, Zone::Hand),
        g.add_card(0, BIG_ONE, Zone::Hand),
    ];
    let six = g.add_card(0, 26, Zone::Hand);
    let swamp = g.add_card(0, SWAMP, Zone::Hand);
    let e = g.add_card(0, 25, Zone::Hand);
    act(&mut g, Action::Cast(e));
    settle(&mut g);
    assert_eq!(g.pending, Pending::FreeCast(0));
    let legal = g.legal_actions();
    assert!(legal.contains(&Action::Cast(big[0])) && legal.contains(&Action::Cast(big[1])));
    assert!(!legal.contains(&Action::Cast(six)), "mana value 6");
    assert!(!legal.contains(&Action::PlayLand(swamp)) && !legal.contains(&Action::Cast(swamp)));
    let n = g.players[0].hand.len();
    act(&mut g, Action::Cast(big[0]));
    assert!(
        !g.legal_actions().contains(&Action::Cast(big[1])),
        "one spell only"
    );
    settle(&mut g);
    assert_eq!(g.obj(big[0]).zone, Zone::Graveyard);
    assert_eq!(g.players[0].hand.len(), n - 1 + 1, "cast one, drew one");
    assert_eq!(tapped_swamps(&g), 1, "only Expert was paid for");
    assert!(!g.obj(big[1]).cast_free);
}

#[test]
fn shadow_blocks_and_is_blocked_only_by_shadow() {
    use batch3::*;
    let shade = || CardDef::creature("Shade", "{B}", 1, 1, Keywords::SHADOW);
    let mut g = batch6_board(vec![shade(), shade()], 0);
    let a = put(&mut g, 0, 25);
    let bear = put(&mut g, 1, GREEN_BEAR);
    let other = put(&mut g, 1, 26);
    assert_eq!(blockers_for(&mut g, a), vec![other], "{:?} can't", bear);
    let mut g = batch6_board(vec![shade()], 0);
    let a = put(&mut g, 0, GREEN_BEAR);
    put(&mut g, 1, 25);
    let bear = put(&mut g, 1, GREEN_BEAR);
    assert_eq!(
        blockers_for(&mut g, a),
        vec![bear],
        "a shadow creature can't block it"
    );
}

#[test]
fn alternative_cost_of_life_sacrifices_and_taps() {
    use batch3::*;
    use bolt_core::card::AltCostExt;
    let mut demon = CardDef::creature("Demon", "{9}", 9, 9, Keywords::NONE);
    demon.mana_ext.alt_cost_ext = Some(AltCostExt {
        life: 6,
        sacrifice: 2,
        sacrifice_color: Some(2),
        ..AltCostExt::default()
    });
    let mut angel = CardDef::creature("Angel", "{9}", 7, 7, Keywords::NONE);
    angel.mana_ext.alt_cost_ext = Some(AltCostExt {
        mana: ManaCost::parse("{B}").expect("cost"),
        tap: 2,
        tap_keywords: Keywords::FEAR,
        ..AltCostExt::default()
    });
    let mut g = batch6_board(vec![demon, angel], 1);
    let blacks = [put(&mut g, 0, BLACK_KNIGHT), put(&mut g, 0, BLACK_BEAR)];
    let green = put(&mut g, 0, GREEN_BEAR);
    let d = g.add_card(0, 25, Zone::Hand);
    g.players[0].life = 5;
    assert!(!g.legal_actions().contains(&Action::CastAlternative(d)), "5 life");
    g.players[0].life = 7;
    act(&mut g, Action::CastAlternative(d));
    settle(&mut g);
    assert_eq!(g.obj(d).zone, Zone::Battlefield);
    assert_eq!(g.players[0].life, 1);
    assert!(blacks.iter().all(|&b| g.obj(b).zone == Zone::Graveyard));
    assert_eq!(g.obj(green).zone, Zone::Battlefield);

    let mut g = batch6_board(
        vec![CardDef::default(), {
            let mut a = CardDef::creature("Angel", "{9}", 7, 7, Keywords::NONE);
            a.mana_ext.alt_cost_ext = Some(AltCostExt {
                mana: ManaCost::parse("{B}").expect("cost"),
                tap: 2,
                tap_keywords: Keywords::FEAR,
                ..AltCostExt::default()
            });
            a
        }],
        1,
    );
    let f = put(&mut g, 0, FEARFUL);
    put(&mut g, 0, GREEN_BEAR);
    let c = g.add_card(0, 26, Zone::Hand);
    assert!(
        !g.legal_actions().contains(&Action::CastAlternative(c)),
        "one with fear"
    );
    let f2 = put(&mut g, 0, FEARFUL);
    act(&mut g, Action::CastAlternative(c));
    settle(&mut g);
    assert_eq!(g.obj(c).zone, Zone::Battlefield);
    assert!(g.obj(f).tapped && g.obj(f2).tapped);
    assert_eq!(tapped_swamps(&g), 1, "{{B}}");
}

#[test]
fn ability_discount_skips_mana_abilities_and_leaves_one_mana() {
    use batch3::*;
    let mut zirda = CardDef::creature("Zirda", "{R}", 3, 3, Keywords::NONE);
    zirda.mana_ext.ability_discount = 2;
    let mut pinger = CardDef::creature("Pinger", "{R}", 1, 1, Keywords::NONE);
    for cost in ["{3}{B}", "{2}"] {
        pinger.activated.push(Activated {
            mana: ManaCost::parse(cost).expect("cost"),
            ability: Ability::new(vec![], vec![Effect::Draw { who: Who::You, n: 1 }]),
            ..Activated::default()
        });
    }
    pinger.activated.push(Activated {
        mana: ManaCost::parse("{1}").expect("cost"),
        ability: Ability::new(
            vec![],
            vec![Effect::AddMana {
                color: Color::Black,
                n: 2,
            }],
        ),
        ..Activated::default()
    });
    let mut g = batch6_board(vec![zirda, pinger], 5);
    put(&mut g, 0, 25);
    let p = put(&mut g, 0, 26);
    act(&mut g, Action::Activate(p, 0));
    settle(&mut g);
    assert_eq!(tapped_swamps(&g), 2, "{{3}}{{B}} less {{2}}");
    act(&mut g, Action::Activate(p, 1));
    settle(&mut g);
    assert_eq!(tapped_swamps(&g), 3, "{{2}}: one mana is left");
}

#[test]
fn untap_symbol_on_a_granted_ability_untaps_the_equipped_creature() {
    use batch3::*;
    let mut mantle = CardDef {
        name: "Mantle".into(),
        types: bolt_core::Types {
            artifact: true,
            ..bolt_core::Types::default()
        },
        ..CardDef::default()
    };
    mantle.activated.push(Activated {
        mana: ManaCost::parse("{1}").expect("cost"),
        granted_to_attached: true,
        untap_attached: true,
        ability: Ability::new(
            vec![],
            vec![Effect::PumpAttached {
                power: 2,
                toughness: 2,
            }],
        ),
        ..Activated::default()
    });
    let mut g = batch6_board(vec![mantle], 2);
    let m = put(&mut g, 0, 25);
    let bear = put(&mut g, 0, GREEN_BEAR);
    let zcc = g.obj(bear).zcc;
    g.objects[m.0 as usize].attached_to = Some((bear, zcc));
    assert!(!g.legal_actions().contains(&Action::Activate(m, 0)), "untapped");
    g.objects[bear.0 as usize].tapped = true;
    g.objects[bear.0 as usize].since_turn = g.turn;
    assert!(
        !g.legal_actions().contains(&Action::Activate(m, 0)),
        "summoning sick"
    );
    g.objects[bear.0 as usize].since_turn = 0;
    act(&mut g, Action::Activate(m, 0));
    settle(&mut g);
    assert!(!g.obj(bear).tapped);
    assert_eq!(g.power(bear), 4);
}

#[test]
fn first_creature_spell_each_turn_discount() {
    use batch3::*;
    let mut warp = CardDef::creature("Warp", "{R}", 0, 1, Keywords::NONE);
    warp.mana_ext.first_creature_spell_discount = 2;
    let mut g = batch6_board(vec![warp], 6);
    put(&mut g, 0, 25);
    cast(&mut g, BIG_ONE);
    assert_eq!(tapped_swamps(&g), 3, "not a creature spell");
    cast(&mut g, BLACK_BEAR);
    assert_eq!(tapped_swamps(&g), 4, "{{1}}{{B}} less {{2}}");
    cast(&mut g, BLACK_BEAR);
    assert_eq!(tapped_swamps(&g), 6, "the second creature spell");
}
