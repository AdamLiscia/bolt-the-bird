//! Card scripts: the rules text the engine implements, card by card, as effect-language data.
//!
//! Each script lists the exact ability lines it implements (reminder text removed). A card is
//! supported only when its scripts and keywords account for every line, and a test checks each
//! script's lines against the Scryfall text, so a script can't drift from its card.

use bolt_core::effect::{
    Ability, Condition, Controller, Effect, Filter, Recipient, Scope, SearchFor, SearchTo, Static,
    StaticCondition, Trigger, When, Who,
};
use bolt_core::effect::{EmblemKind, Scale};
use bolt_core::{
    Activated, Behold, CardDef, Color, ExtraCost, Keywords, LookRest, LookTo, ManaCost, ManaSource, Pick,
    Reduction, Stage, Types,
};

/// Token definitions, appended after the 286 cards. Indices are [`Tokens`] fields.
pub fn token_defs() -> Vec<CardDef> {
    let token = |name: &str, sub: &str, p, t, k| {
        let mut d = CardDef::creature(name, "", p, t, k);
        d.token = true;
        d.subtypes = sub.split(' ').map(String::from).collect();
        d
    };
    vec![
        token("Insect", "Insect", 1, 1, Keywords::FLYING),
        token("Knight", "Knight", 3, 3, Keywords::NONE),
        token("Cat", "Cat", 1, 1, Keywords::NONE),
        token("Dragon", "Dragon", 4, 4, Keywords::FLYING),
        token("Soldier", "Soldier", 1, 1, Keywords::NONE),
        token("Elf Warrior", "Elf Warrior", 1, 1, Keywords::NONE),
        token("Human", "Human", 1, 1, Keywords::NONE),
        token("Rabbit", "Rabbit", 1, 1, Keywords::NONE),
        food(),
        treasure(),
        token("Faerie", "Faerie", 1, 1, Keywords::FLYING),
        token("Goblin", "Goblin", 1, 1, Keywords::NONE),
        token("Dragon", "Dragon", 5, 5, Keywords::FLYING),
        {
            let mut scion = token("Scion of the Deep", "Octopus", 8, 8, Keywords::NONE);
            scion.types.legendary = true;
            scion
        },
        token("Raccoon", "Raccoon", 3, 3, Keywords::NONE),
        token("Koma's Coil", "Serpent", 3, 3, Keywords::NONE),
        token("Drake", "Drake", 2, 2, Keywords::FLYING),
        token("Spirit", "Spirit", 1, 1, Keywords::FLYING),
        token("Fish", "Fish", 1, 1, Keywords::NONE),
        token("Rat", "Rat", 1, 1, Keywords::NONE),
        token("Zombie", "Zombie", 2, 2, Keywords::NONE),
        token("Ninja", "Ninja", 2, 1, Keywords::NONE),
        token("Cat", "Cat", 2, 2, Keywords::NONE),
        token("Warrior", "Warrior", 1, 1, Keywords::NONE), // Tarkir's mobilize tokens (red)
        token("Spirit", "Spirit", 0, 0, Keywords::NONE),   // endure: made N/N with counters
        token("Bird", "Bird", 1, 1, Keywords::FLYING),
        token("Zombie Druid", "Zombie Druid", 2, 2, Keywords::NONE),
        token("Elephant", "Elephant", 5, 5, Keywords::NONE),
        token("Soldier", "Soldier", 2, 2, Keywords::NONE),
        token("Spirit", "Spirit", 1, 1, Keywords::NONE), // Revival of the Ancestors' white Spirits
        {
            // A 1/1 white Monk with prowess (Jeskai Revelation).
            let mut monk = token("Monk", "Monk", 1, 1, Keywords::NONE);
            on(
                &mut monk,
                When::YouCastNoncreature,
                vec![],
                vec![Effect::PumpSelf {
                    power: 1,
                    toughness: 1,
                }],
            );
            monk
        },
        {
            // Dragonbroods' Relic: "a 4/4 Dragon creature token named Reliquary Dragon that's all
            // colors. It has flying, lifelink, and 'When this token enters, it deals 3 damage to any
            // target.'"
            let mut d = token(
                "Reliquary Dragon",
                "Dragon",
                4,
                4,
                Keywords::FLYING.with(Keywords::LIFELINK),
            );
            d.colors = [true; 5];
            on(
                &mut d,
                When::Enters,
                vec![Filter::ANY],
                vec![damage(3, Recipient::Target(0))],
            );
            d
        },
        token("Beast", "Beast", 3, 3, Keywords::NONE), // Beast Within's green 3/3
        token("Human Soldier", "Human Soldier", 1, 1, Keywords::NONE), // Bastion of Remembrance
        {
            // Blade Splicer: "a 3/3 colorless Phyrexian Golem artifact creature token".
            let mut golem = token("Phyrexian Golem", "Phyrexian Golem", 3, 3, Keywords::NONE);
            golem.types.artifact = true;
            golem
        },
        token("Elephant", "Elephant", 3, 3, Keywords::NONE), // Generous Gift's green 3/3
        token("Beast", "Beast", 4, 4, Keywords::NONE),       // Rampaging Baloths' green 4/4
        token("Frog Lizard", "Frog Lizard", 3, 3, Keywords::NONE), // Rapid Hybridization
        token("Squirrel", "Squirrel", 1, 1, Keywords::NONE), // Chatterstorm
        token("Spider", "Spider", 1, 2, Keywords::REACH),    // Arasta of the Endless Web
        {
            // Thopter Engineer: "a 1/1 colorless Thopter artifact creature token with flying".
            let mut thopter = token("Thopter", "Thopter", 1, 1, Keywords::FLYING);
            thopter.types.artifact = true;
            thopter
        },
        token("Storm Crow", "Bird", 1, 2, Keywords::FLYING), // Murmuration's blue Birds
        {
            // Garruk, Cursed Huntsman: "2/2 black and green Wolf creature tokens with \"When this
            // token dies, put a loyalty counter on each Garruk you control.\""
            let mut wolf = token("Wolf", "Wolf", 2, 2, Keywords::NONE);
            on(
                &mut wolf,
                When::Dies,
                vec![],
                vec![Effect::LoyaltyEachYouControlSubtype("Garruk")],
            );
            wolf
        },
        {
            // Beledros Witherbloom: a 1/1 black and green Pest with "When this token dies, you gain 1 life."
            let mut pest = token("Pest", "Pest", 1, 1, Keywords::NONE);
            on(
                &mut pest,
                When::Dies,
                vec![],
                vec![Effect::GainLife { who: Who::You, n: 1 }],
            );
            pest
        },
        token("Octopus", "Octopus", 8, 8, Keywords::NONE), // Octomancer's gift: a blue 8/8
        token("Elemental", "Elemental", 4, 4, Keywords::FLYING), // Hoofprints of the Stag: white 4/4
    ]
}

fn artifact_token(name: &str) -> CardDef {
    CardDef {
        name: name.to_string(),
        types: Types {
            artifact: true,
            ..Types::default()
        },
        subtypes: vec![name.to_string()],
        token: true,
        ..CardDef::default()
    }
}

/// "{2}, {T}, Sacrifice this token: You gain 3 life." (CR 111.10b)
fn food() -> CardDef {
    let mut d = artifact_token("Food");
    d.activated.push(Activated {
        mana: ManaCost::parse("{2}").expect("cost"),
        tap: true,
        sacrifice_self: true,
        ability: Ability::new(vec![], vec![Effect::GainLife { who: Who::You, n: 3 }]),
        ..Activated::default()
    });
    d
}

/// "{T}, Sacrifice this token: Add one mana of any color." A mana ability, so it never uses the
/// stack; the automatic payer spends it only when nothing else works (CR 111.10a).
fn treasure() -> CardDef {
    let mut d = artifact_token("Treasure");
    d.mana = Some(ManaSource::of(&Color::ALL));
    d.sac_for_mana = true;
    d
}

/// Where each token definition lives in the full definition list.
#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    pub insect: usize,
    pub knight: usize,
    pub cat: usize,
    pub dragon: usize,
    pub soldier: usize,
    pub elf_warrior: usize,
    pub human: usize,
    pub rabbit: usize,
    pub food: usize,
    pub treasure: usize,
    pub faerie: usize,
    pub goblin: usize,
    pub dragon_5: usize,
    pub scion: usize,
    pub raccoon: usize,
    pub komas_coil: usize,
    pub zombie: usize,
    pub ninja: usize,
    pub cat_2: usize,
    pub drake: usize,
    pub spirit: usize,
    pub fish: usize,
    pub rat: usize,
    pub warrior: usize,
    pub spirit_endure: usize,
    pub bird: usize,
    pub zombie_druid: usize,
    pub elephant: usize,
    pub soldier_2: usize,
    pub spirit_1: usize,
    pub monk: usize,
    pub reliquary_dragon: usize,
    pub beast_3: usize,
    pub human_soldier: usize,
    pub golem: usize,
    pub elephant_3: usize,
    pub beast_4: usize,
    pub frog_lizard: usize,
    pub squirrel: usize,
    pub spider: usize,
    pub thopter: usize,
    pub storm_crow: usize,
    pub garruk_wolf: usize,
    pub pest: usize,
    pub octopus: usize,
    pub elemental_4: usize,
}

impl Tokens {
    pub fn starting_at(base: usize) -> Tokens {
        Tokens {
            insect: base,
            knight: base + 1,
            cat: base + 2,
            dragon: base + 3,
            soldier: base + 4,
            elf_warrior: base + 5,
            human: base + 6,
            rabbit: base + 7,
            food: base + 8,
            treasure: base + 9,
            faerie: base + 10,
            goblin: base + 11,
            dragon_5: base + 12,
            scion: base + 13,
            raccoon: base + 14,
            komas_coil: base + 15,
            drake: base + 16,
            spirit: base + 17,
            fish: base + 18,
            rat: base + 19,
            zombie: base + 20,
            ninja: base + 21,
            cat_2: base + 22,
            warrior: base + 23,
            spirit_endure: base + 24,
            bird: base + 25,
            zombie_druid: base + 26,
            elephant: base + 27,
            soldier_2: base + 28,
            spirit_1: base + 29,
            monk: base + 30,
            reliquary_dragon: base + 31,
            beast_3: base + 32,
            human_soldier: base + 33,
            golem: base + 34,
            elephant_3: base + 35,
            beast_4: base + 36,
            frog_lizard: base + 37,
            squirrel: base + 38,
            spider: base + 39,
            thopter: base + 40,
            storm_crow: base + 41,
            garruk_wolf: base + 42,
            pest: base + 43,
            octopus: base + 44,
            elemental_4: base + 45,
        }
    }
}

pub struct Script {
    pub lines: &'static [&'static str],
    pub build: fn(&mut CardDef, &Tokens),
}

fn spell(d: &mut CardDef, targets: Vec<Filter>, effects: Vec<Effect>) {
    d.spell = Some(Ability::new(targets, effects));
}

fn on(d: &mut CardDef, when: When, targets: Vec<Filter>, effects: Vec<Effect>) {
    d.triggers.push(Trigger {
        when,
        ability: Ability::new(targets, effects),
    });
}

fn on_if(d: &mut CardDef, condition: Condition, targets: Vec<Filter>, effects: Vec<Effect>) {
    let mut ability = Ability::new(targets, effects);
    ability.condition = Some(condition);
    d.triggers.push(Trigger {
        when: When::Enters,
        ability,
    });
}

fn damage(amount: i32, to: Recipient) -> Effect {
    Effect::Damage {
        amount,
        kicked: None,
        to,
    }
}

const CREATURE: Filter = Filter::CREATURE;
const CREATURE_OR_PLANESWALKER: Filter = Filter {
    players: false,
    spells: false,
    ..Filter::ANY
};
const OPP_CREATURE: Filter = Filter::CREATURE.controlled_by(Controller::Opponent);
const YOUR_CREATURE: Filter = Filter::CREATURE.controlled_by(Controller::You);
const ART_ENCH: Filter = Filter {
    players: false,
    creatures: true,
    planeswalkers: false,
    artifacts: true,
    enchantments: true,
    ..Filter::ANY
};

fn may(d: &mut CardDef, when: When, targets: Vec<Filter>, effects: Vec<Effect>) {
    d.triggers.push(Trigger {
        when,
        ability: Ability::new(targets, effects).optional(),
    });
}

const SPELL: Filter = Filter {
    players: false,
    creatures: false,
    planeswalkers: false,
    spells: true,
    ..Filter::ANY
};
const OPP_NONLAND: Filter = Filter {
    players: false,
    creatures: false,
    planeswalkers: false,
    nonland_permanents: true,
    controller: Controller::Opponent,
    ..Filter::ANY
};
const ARTIFACT_OR_ENCHANTMENT: Filter = Filter {
    players: false,
    creatures: false,
    planeswalkers: false,
    artifacts: true,
    enchantments: true,
    ..Filter::ANY
};

/// Cards in a graveyard: creature cards only, or any card.
const fn graveyard_cards(creatures_only: bool, controller: Controller) -> Filter {
    Filter {
        players: false,
        creatures: false,
        planeswalkers: false,
        graveyard: Some(creatures_only),
        controller,
        ..Filter::ANY
    }
}

fn up_to(targets: Vec<Filter>, effects: Vec<Effect>) -> Ability {
    let mut a = Ability::new(targets, effects);
    a.min_targets = Some(0);
    a
}

fn draw(n: u8) -> Effect {
    Effect::Draw { who: Who::You, n }
}

fn discard(n: u8) -> Effect {
    Effect::Discard { who: Who::You, n }
}

fn search(what: SearchFor, to: SearchTo) -> Effect {
    Effect::Search {
        what,
        to,
        n: 1,
        kicked_n: None,
    }
}

fn activated(d: &mut CardDef, cost: &str, tap: bool, targets: Vec<Filter>, effects: Vec<Effect>) -> usize {
    d.activated.push(Activated {
        mana: ManaCost::parse(cost).expect("cost"),
        tap,
        ability: Ability::new(targets, effects),
        ..Activated::default()
    });
    d.activated.len() - 1
}

/// A gain land: enters tapped, gains 1 life, taps for either of two colors.
fn gain_land(d: &mut CardDef, a: Color, b: Color) {
    d.enters_tapped = true;
    d.mana = Some(ManaSource::of(&[a, b]));
    on(
        d,
        When::Enters,
        vec![],
        vec![Effect::GainLife { who: Who::You, n: 1 }],
    );
}

const GAIN_LAND: [&str; 2] = [
    "This land enters tapped.",
    "When this land enters, you gain 1 life.",
];

fn static_on(d: &mut CardDef, scope: Scope, your_turn_only: bool, keywords: Keywords) {
    d.statics.push(Static {
        subtype: None,
        lose_abilities: false,
        base_pt: None,
        protection: [false; 5],
        scale: None,
        scope,
        your_turn_only,
        condition: None,
        power: 0,
        toughness: 0,
        keywords,
        set_colors: None,
        becomes_land: false,
    });
}

/// "Mobilize N": whenever this creature attacks, N 1/1 red Warrior tokens tapped and attacking, sacrificed
/// at the next end step. `None` for "Mobilize X, where X is the number of creature cards in your graveyard."
fn mobilize(d: &mut CardDef, t: &Tokens, n: Option<u8>) {
    on(
        d,
        When::Attacks,
        vec![],
        vec![Effect::Mobilize { def: t.warrior, n }],
    );
}

/// "{1}: Add {A}, {B}, or {C}. Activate only once each turn." (the Devotees), colors in WUBRG order.
fn devotee(d: &mut CardDef, colors: &[Color]) {
    let mut m = [false; 5];
    for &c in colors {
        m[c as usize] = true;
    }
    d.filter_mana = Some(m);
}

/// "Flurry — Whenever you cast your second spell each turn, ..."
fn flurry(d: &mut CardDef, targets: Vec<Filter>, effects: Vec<Effect>) {
    on(d, When::YouCastSecondSpell, targets, effects);
}

fn endure(n: u8, t: &Tokens) -> Effect {
    Effect::Endure {
        n,
        spirit: t.spirit_endure,
    }
}

/// "Renew — {cost}, Exile this card from your graveyard: <effect>. Activate only as a sorcery."
fn renew(d: &mut CardDef, cost: &str, targets: Vec<Filter>, effects: Vec<Effect>) {
    let n = activated(d, cost, false, targets, effects);
    let a = &mut d.activated[n];
    a.from_graveyard = true;
    a.exile_self = true;
    a.sorcery = true;
}

/// "Harmonize {cost}": cast from the graveyard for it, optionally tapping a creature for {X} less.
fn harmonize(d: &mut CardDef, cost: &str) {
    d.flashback = ManaCost::parse(cost);
    d.harmonize = true;
}

/// An Omen card's spell half (its name, cost and type come from the card data).
fn omen(d: &mut CardDef, spell: Ability) {
    d.omen = Some(bolt_core::Omen {
        name: String::new(),
        cost: ManaCost::default(),
        instant: false,
        spell,
    });
}

/// Basic land types by name, for `SearchFor::LandTypes`.
fn land_types(names: &[&str]) -> [bool; 5] {
    let all = ["Plains", "Island", "Swamp", "Mountain", "Forest"];
    let mut t = [false; 5];
    for n in names {
        t[all.iter().position(|a| a == n).expect("a basic land type")] = true;
    }
    t
}

/// "When this artifact enters, search your library for a basic A, B, or C card, reveal it, put it into
/// your hand, then shuffle." plus "{cost}, {T}, Sacrifice this artifact: <effects>. Activate only as a
/// sorcery." (the Monuments).
fn monument(d: &mut CardDef, types: &[&str], cost: &str, effects: Vec<Effect>) {
    let what = SearchFor::LandTypes {
        types: land_types(types),
        basic: true,
    };
    on(d, When::Enters, vec![], vec![search(what, SearchTo::Hand)]);
    let n = activated(d, cost, true, vec![], effects);
    d.activated[n].sacrifice_self = true;
    d.activated[n].sorcery = true;
}

/// "{T}, Pay 1 life, Sacrifice this land: Search your library for an A or B card, put it onto the
/// battlefield, then shuffle." (the fetch lands)
fn fetch_land(d: &mut CardDef, types: &[&str]) {
    let what = SearchFor::LandTypes {
        types: land_types(types),
        basic: false,
    };
    let n = activated(
        d,
        "",
        true,
        vec![],
        vec![search(what, SearchTo::Battlefield { tapped: false })],
    );
    d.activated[n].sacrifice_self = true;
    d.activated[n].life = 1;
}

/// "When a Dragon you control enters, return this enchantment to its owner's hand." (the Dragonstorms)
fn dragonstorm_return(d: &mut CardDef) {
    on(
        d,
        When::AnotherYouControlEntersWithSubtype("Dragon"),
        vec![],
        vec![Effect::BounceSelf],
    );
}

const DRAGONSTORM_RETURN: &str =
    "When a Dragon you control enters, return this enchantment to its owner's hand.";

/// A Saga chapter ability ("I — ...").
fn chapter(d: &mut CardDef, n: u8, ability: Ability) {
    d.triggers.push(Trigger {
        when: When::Chapter(n),
        ability,
    });
}

fn modes(d: &mut CardDef, modes: Vec<Ability>) {
    d.modes = modes;
}

/// "Equip {cost}": attach to target creature you control, as a sorcery (CR 702.6a).
fn equip(d: &mut CardDef, cost: &str) {
    let n = activated(
        d,
        cost,
        false,
        vec![YOUR_CREATURE],
        vec![Effect::Attach { slot: 0 }],
    );
    d.activated[n].sorcery = true;
}

fn attached(d: &mut CardDef, your_turn_only: bool, power: i32, toughness: i32, keywords: Keywords) {
    d.statics.push(Static {
        subtype: None,
        lose_abilities: false,
        base_pt: None,
        protection: [false; 5],
        scale: None,
        scope: Scope::Attached,
        your_turn_only,
        condition: None,
        power,
        toughness,
        keywords,
        set_colors: None,
        becomes_land: false,
    });
}

const ARTIFACT: Filter = Filter {
    players: false,
    creatures: false,
    planeswalkers: false,
    artifacts: true,
    ..Filter::ANY
};

fn threshold(d: &mut CardDef, power: i32, toughness: i32, keywords: Keywords) {
    d.statics.push(Static {
        subtype: None,
        lose_abilities: false,
        base_pt: None,
        protection: [false; 5],
        scale: None,
        scope: Scope::This,
        your_turn_only: false,
        condition: Some(StaticCondition::Threshold),
        power,
        toughness,
        keywords,
        set_colors: None,
        becomes_land: false,
    });
}

/// A modal triggered ability: "choose one —" (CR 700.2b).
fn on_modal(d: &mut CardDef, when: When, condition: Option<Condition>, modes: Vec<Ability>) {
    let mut ability = Ability::new(vec![], vec![]);
    ability.condition = condition;
    ability.modes = modes;
    d.triggers.push(Trigger { when, ability });
}

fn conditional(
    d: &mut CardDef,
    when: When,
    condition: Condition,
    targets: Vec<Filter>,
    effects: Vec<Effect>,
) {
    let mut ability = Ability::new(targets, effects);
    ability.condition = Some(condition);
    d.triggers.push(Trigger { when, ability });
}

fn lord(d: &mut CardDef, sub: &'static str) {
    d.statics.push(Static {
        subtype: None,
        lose_abilities: false,
        base_pt: None,
        protection: [false; 5],
        scale: None,
        scope: Scope::OtherYourSubtype(sub),
        your_turn_only: false,
        condition: None,
        power: 1,
        toughness: 1,
        keywords: Keywords::NONE,
        set_colors: None,
        becomes_land: false,
    });
}

fn loyalty(d: &mut CardDef, cost: i32, ability: Ability) {
    d.loyalty_abilities.push((cost, ability));
}

/// The script for a card, if one exists.
/// The script for a card, by name: the core scripts here, then the Tarkir lanes' modules.
pub fn script(name: &str) -> Option<Script> {
    script_core(name)
        .or_else(|| tdm_b::script(name))
        .or_else(|| tdm_c::script(name))
        .or_else(|| blc_b::script(name))
        .or_else(|| blc_c::script(name))
}

// Parallel lanes for plain Tarkir cards (no engine-core changes): each owns its module and test file.
mod tdm_b;
mod tdm_c;
pub use tdm_b::TDM_B_SCRIPTED;
pub use tdm_c::TDM_C_SCRIPTED;
// Bloomburrow Commander, lane B.
mod blc_b;
pub use blc_b::BLC_B_SCRIPTED;
// Bloomburrow Commander, lane C (shared order 150-299).
mod blc_c;
pub use blc_c::BLC_C_SCRIPTED;

fn script_core(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        "Stab" => s(&["Target creature gets -2/-2 until end of turn."], |d, _| {
            spell(d, vec![CREATURE], vec![Effect::Pump { slot: 0, power: -2, toughness: -2 }])
        }),
        "Giant Growth" => s(&["Target creature gets +3/+3 until end of turn."], |d, _| {
            spell(d, vec![CREATURE], vec![Effect::Pump { slot: 0, power: 3, toughness: 3 }])
        }),
        "Hero's Downfall" => s(&["Destroy target creature or planeswalker."], |d, _| {
            let f = Filter { players: false, ..Filter::ANY };
            spell(d, vec![f], vec![Effect::Destroy(0)])
        }),
        "Boltwave" => s(&["Boltwave deals 3 damage to each opponent."], |d, _| {
            spell(d, vec![], vec![damage(3, Recipient::EachOpponent)])
        }),
        "Burst Lightning" => s(
            &[
                "Kicker {4}",
                "Burst Lightning deals 2 damage to any target. If this spell was kicked, it deals 4 damage instead.",
            ],
            |d, _| {
                d.kicker = ManaCost::parse("{4}");
                spell(
                    d,
                    vec![Filter::ANY],
                    vec![Effect::Damage { amount: 2, kicked: Some(4), to: Recipient::Target(0) }],
                )
            },
        ),
        "Joust Through" => s(
            &["Joust Through deals 3 damage to target attacking or blocking creature. You gain 1 life."],
            |d, _| {
                let f = Filter { in_combat: true, ..CREATURE };
                spell(d, vec![f], vec![damage(3, Recipient::Target(0)), Effect::GainLife { who: Who::You, n: 1 }])
            },
        ),
        "Seismic Rupture" => s(&["Seismic Rupture deals 2 damage to each creature without flying."], |d, _| {
            spell(d, vec![], vec![damage(2, Recipient::EachCreature { without_flying: true })])
        }),
        "Hidetsugu's Second Rite" => s(
            &["If target player has exactly 10 life, Hidetsugu's Second Rite deals 10 damage to that player."],
            |d, _| spell(d, vec![Filter::PLAYER], vec![Effect::DamageIfLifeIs { slot: 0, life: 10, amount: 10 }]),
        ),
        "Make Your Move" => s(
            &["Destroy target artifact, enchantment, or creature with power 4 or greater."],
            |d, _| {
                let f = Filter { creature_min_power: Some(4), ..ART_ENCH };
                spell(d, vec![f], vec![Effect::Destroy(0)])
            },
        ),
        "Broken Wings" => s(&["Destroy target artifact, enchantment, or creature with flying."], |d, _| {
            let f = Filter { creature_needs_flying: true, ..ART_ENCH };
            spell(d, vec![f], vec![Effect::Destroy(0)])
        }),
        "Helpful Hunter" => s(&["When this creature enters, draw a card."], |d, _| {
            on(d, When::Enters, vec![], vec![Effect::Draw { who: Who::You, n: 1 }])
        }),
        "Fleeting Distraction" => s(&["Target creature gets -1/-0 until end of turn.", "Draw a card."], |d, _| {
            spell(
                d,
                vec![CREATURE],
                vec![Effect::Pump { slot: 0, power: -1, toughness: 0 }, Effect::Draw { who: Who::You, n: 1 }],
            )
        }),
        "Felling Blow" => s(
            &["Put a +1/+1 counter on target creature you control. Then that creature deals damage equal to its power to target creature an opponent controls."],
            |d, _| {
                spell(
                    d,
                    vec![YOUR_CREATURE, OPP_CREATURE],
                    vec![Effect::Counters { slot: 0, n: 1 }, Effect::Bite { from: 0, to: 1 }],
                )
            },
        ),
        "Infestation Sage" => s(
            &["When this creature dies, create a 1/1 black and green Insect creature token with flying."],
            |d, t| on(d, When::Dies, vec![], vec![Effect::CreateToken { who: Who::You, def: t.insect, n: 1 }]),
        ),
        "Guarded Heir" => s(&["When this creature enters, create two 3/3 white Knight creature tokens."], |d, t| {
            on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.knight, n: 2 }])
        }),
        "Prideful Parent" => s(&["When this creature enters, create a 1/1 white Cat creature token."], |d, t| {
            on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.cat, n: 1 }])
        }),
        "Dragon Trainer" => s(
            &["When this creature enters, create a 4/4 red Dragon creature token with flying."],
            |d, t| on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.dragon, n: 1 }]),
        ),
        "Resolute Reinforcements" => s(
            &["When this creature enters, create a 1/1 white Soldier creature token."],
            |d, t| on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.soldier, n: 1 }]),
        ),
        "Angel of Finality" => s(&["When this creature enters, exile target player's graveyard."], |d, _| {
            on(d, When::Enters, vec![Filter::PLAYER], vec![Effect::ExileGraveyard { who: Who::TargetPlayer(0) }])
        }),
        "Gorehorn Raider" => s(
            &["Raid — When this creature enters, if you attacked this turn, this creature deals 2 damage to any target."],
            |d, _| on_if(d, Condition::Attacked, vec![Filter::ANY], vec![damage(2, Recipient::Target(0))]),
        ),
        "Skyship Buccaneer" => s(
            &["Raid — When this creature enters, if you attacked this turn, draw a card."],
            |d, _| on_if(d, Condition::Attacked, vec![], vec![Effect::Draw { who: Who::You, n: 1 }]),
        ),
        "Dwynen's Elite" => s(
            &["When this creature enters, if you control another Elf, create a 1/1 green Elf Warrior creature token."],
            |d, t| {
                on_if(
                    d,
                    Condition::ControlAnother("Elf".into()),
                    vec![],
                    vec![Effect::CreateToken { who: Who::You, def: t.elf_warrior, n: 1 }],
                )
            },
        ),
        "Burglar Rat" => s(&["When this creature enters, each opponent discards a card."], |d, _| {
            on(d, When::Enters, vec![], vec![Effect::Discard { who: Who::EachOpponent, n: 1 }])
        }),
        "Icewind Elemental" => s(&["When this creature enters, draw a card, then discard a card."], |d, _| {
            on(d, When::Enters, vec![], vec![draw(1), discard(1)])
        }),
        "Incinerating Blast" => s(
            &["Incinerating Blast deals 6 damage to target creature.", "You may discard a card. If you do, draw a card."],
            |d, _| spell(d, vec![CREATURE], vec![damage(6, Recipient::Target(0)), Effect::MayDiscardToDraw]),
        ),
        "Wary Thespian" => s(&["When this creature enters or dies, surveil 1."], |d, _| {
            on(d, When::Enters, vec![], vec![Effect::Surveil { n: 1 }]);
            on(d, When::Dies, vec![], vec![Effect::Surveil { n: 1 }]);
        }),
        "Lightshell Duo" => s(&["Prowess", "When this creature enters, surveil 2."], |d, _| {
            on(d, When::YouCastNoncreature, vec![], vec![Effect::PumpSelf { power: 1, toughness: 1 }]);
            on(d, When::Enters, vec![], vec![Effect::Surveil { n: 2 }]);
        }),
        "Uncharted Voyage" => s(
            &["Target creature's owner puts it on their choice of the top or bottom of their library.", "Surveil 1."],
            |d, _| spell(d, vec![CREATURE], vec![Effect::Tuck(0), Effect::Surveil { n: 1 }]),
        ),
        "Refute" => s(&["Counter target spell. Draw a card, then discard a card."], |d, _| {
            spell(d, vec![SPELL], vec![Effect::Counter(0), draw(1), discard(1)])
        }),
        "Essence Scatter" => s(&["Counter target creature spell."], |d, _| {
            spell(d, vec![Filter { spell_creature: Some(true), ..SPELL }], vec![Effect::Counter(0)])
        }),
        "Reclamation Sage" => s(
            &["When this creature enters, you may destroy target artifact or enchantment."],
            |d, _| may(d, When::Enters, vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::Destroy(0)]),
        ),
        "Affectionate Indrik" => s(
            &["When this creature enters, you may have it fight target creature you don't control."],
            |d, _| may(d, When::Enters, vec![OPP_CREATURE], vec![Effect::Fight(0)]),
        ),
        "Meteor Golem" => s(
            &["When this creature enters, destroy target nonland permanent an opponent controls."],
            |d, _| on(d, When::Enters, vec![OPP_NONLAND], vec![Effect::Destroy(0)]),
        ),
        "Bigfin Bouncer" => s(
            &["When this creature enters, return target creature an opponent controls to its owner's hand."],
            |d, _| on(d, When::Enters, vec![OPP_CREATURE], vec![Effect::Bounce(0)]),
        ),
        "Campus Guide" => s(
            &["When this creature enters, you may search your library for a basic land card, reveal it, then shuffle and put that card on top."],
            |d, _| may(d, When::Enters, vec![], vec![search(SearchFor::BasicLand, SearchTo::Top)]),
        ),
        "Solemn Simulacrum" => s(
            &[
                "When this creature enters, you may search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.",
                "When this creature dies, you may draw a card.",
            ],
            |d, _| {
                let to = SearchTo::Battlefield { tapped: true };
                may(d, When::Enters, vec![], vec![search(SearchFor::BasicLand, to)]);
                may(d, When::Dies, vec![], vec![draw(1)]);
            },
        ),
        "Micromancer" => s(
            &["When this creature enters, you may search your library for an instant or sorcery card with mana value 1, reveal it, put it into your hand, then shuffle."],
            |d, _| {
                let what = SearchFor::InstantOrSorceryWithManaValue(1);
                may(d, When::Enters, vec![], vec![search(what, SearchTo::Hand)])
            },
        ),
        "Rune-Scarred Demon" => s(
            &["When this creature enters, search your library for a card, put it into your hand, then shuffle."],
            |d, _| on(d, When::Enters, vec![], vec![search(SearchFor::Any, SearchTo::Hand)]),
        ),
        "Grim Tutor" => s(
            &["Search your library for a card, put that card into your hand, then shuffle. You lose 3 life."],
            |d, _| {
                spell(
                    d,
                    vec![],
                    vec![search(SearchFor::Any, SearchTo::Hand), Effect::LoseLife { who: Who::You, n: 3 }],
                )
            },
        ),
        "Grow from the Ashes" => s(
            &[
                "Kicker {2}",
                "Search your library for a basic land card, put it onto the battlefield, then shuffle. If this spell was kicked, instead search your library for two basic land cards, put them onto the battlefield, then shuffle.",
            ],
            |d, _| {
                d.kicker = ManaCost::parse("{2}");
                let to = SearchTo::Battlefield { tapped: false };
                spell(d, vec![], vec![Effect::Search { what: SearchFor::BasicLand, to, n: 1, kicked_n: Some(2) }])
            },
        ),
        "Pilfer" => s(
            &["Target opponent reveals their hand. You choose a nonland card from it. That player discards that card."],
            |d, _| {
                let f = Filter::PLAYER.controlled_by(Controller::Opponent);
                spell(d, vec![f], vec![Effect::ChooseDiscard { slot: 0 }])
            },
        ),
        "Hare Apparent" => s(
            &[
                "When this creature enters, create a number of 1/1 white Rabbit creature tokens equal to the number of other creatures you control named Hare Apparent.",
                "A deck can have any number of cards named Hare Apparent.",
            ],
            |d, t| on(d, When::Enters, vec![], vec![Effect::CreateTokenPerNamesake { def: t.rabbit }]),
        ),
        "Snakeskin Veil" => s(
            &["Put a +1/+1 counter on target creature you control. It gains hexproof until end of turn."],
            |d, _| {
                spell(
                    d,
                    vec![YOUR_CREATURE],
                    vec![Effect::Counters { slot: 0, n: 1 }, Effect::Grant { slot: 0, keywords: Keywords::HEXPROOF }],
                )
            },
        ),
        "Stroke of Midnight" => s(
            &["Destroy target nonland permanent. Its controller creates a 1/1 white Human creature token."],
            |d, t| {
                let f = Filter { controller: Controller::Any, ..OPP_NONLAND };
                spell(
                    d,
                    vec![f],
                    vec![Effect::Destroy(0), Effect::CreateToken { who: Who::ControllerOf(0), def: t.human, n: 1 }],
                )
            },
        ),
        "Bake into a Pie" => s(&["Destroy target creature. Create a Food token."], |d, t| {
            spell(
                d,
                vec![CREATURE],
                vec![Effect::Destroy(0), Effect::CreateToken { who: Who::You, def: t.food, n: 1 }],
            )
        }),
        "Gleaming Barrier" => s(&["When this creature dies, create a Treasure token."], |d, t| {
            on(d, When::Dies, vec![], vec![Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }])
        }),
        "An Offer You Can't Refuse" => s(
            &["Counter target noncreature spell. Its controller creates two Treasure tokens."],
            |d, t| {
                spell(
                    d,
                    vec![Filter { spell_creature: Some(false), ..SPELL }],
                    vec![Effect::Counter(0), Effect::CreateToken { who: Who::ControllerOf(0), def: t.treasure, n: 2 }],
                )
            },
        ),
        "Arbiter of Woe" => s(
            &[
                "As an additional cost to cast this spell, sacrifice a creature.",
                "When this creature enters, each opponent discards a card and loses 2 life. You draw a card and gain 2 life.",
            ],
            |d, _| {
                d.extra_cost = Some(ExtraCost::SacrificeCreature);
                on(
                    d,
                    When::Enters,
                    vec![],
                    vec![
                        Effect::Discard { who: Who::EachOpponent, n: 1 },
                        Effect::LoseLife { who: Who::EachOpponent, n: 2 },
                        draw(1),
                        Effect::GainLife { who: Who::You, n: 2 },
                    ],
                )
            },
        ),
        "Eaten Alive" => s(
            &[
                "As an additional cost to cast this spell, sacrifice a creature or pay {3}{B}.",
                "Exile target creature or planeswalker.",
            ],
            |d, _| {
                d.extra_cost = Some(ExtraCost::SacrificeCreatureOrPay(ManaCost::parse("{3}{B}").expect("cost")));
                spell(d, vec![Filter { players: false, ..Filter::ANY }], vec![Effect::Exile(0)])
            },
        ),
        "Thrill of Possibility" => s(
            &["As an additional cost to cast this spell, discard a card.", "Draw two cards."],
            |d, _| {
                d.extra_cost = Some(ExtraCost::Discard);
                spell(d, vec![], vec![draw(2)])
            },
        ),
        "Arcane Epiphany" => s(
            &["This spell costs {1} less to cast if you control a Wizard.", "Draw three cards."],
            |d, _| {
                d.reduction = Some(Reduction::IfYouControl("Wizard".into(), 1));
                spell(d, vec![], vec![draw(3)])
            },
        ),
        "Luminous Rebuke" => s(
            &["This spell costs {3} less to cast if it targets a tapped creature.", "Destroy target creature."],
            |d, _| {
                d.reduction = Some(Reduction::IfTargetsTapped(3));
                spell(d, vec![CREATURE], vec![Effect::Destroy(0)])
            },
        ),
        "Think Twice" => s(&["Draw a card.", "Flashback {2}{U}"], |d, _| {
            d.flashback = ManaCost::parse("{2}{U}");
            spell(d, vec![], vec![draw(1)])
        }),
        "Felidar Savior" => s(
            &["When this creature enters, put a +1/+1 counter on each of up to two other target creatures you control."],
            |d, _| {
                let f = Filter { other: true, ..YOUR_CREATURE };
                let ability =
                    up_to(vec![f, f], vec![Effect::Counters { slot: 0, n: 1 }, Effect::Counters { slot: 1, n: 1 }]);
                d.triggers.push(Trigger { when: When::Enters, ability });
            },
        ),
        "Macabre Waltz" => s(
            &["Return up to two target creature cards from your graveyard to your hand, then discard a card."],
            |d, _| {
                let f = graveyard_cards(true, Controller::You);
                d.spell = Some(up_to(vec![f, f], vec![Effect::Bounce(0), Effect::Bounce(1), discard(1)]));
            },
        ),
        "Soul-Shackled Zombie" => s(
            &["When this creature enters, exile up to two target cards from a single graveyard. If at least one creature card was exiled this way, each opponent loses 2 life and you gain 2 life."],
            |d, _| {
                let f = graveyard_cards(false, Controller::Any);
                let mut ability = up_to(
                    vec![f, f],
                    vec![Effect::Exile(0), Effect::Exile(1), Effect::DrainIfCreatureExiled { n: 2 }],
                );
                ability.same_graveyard = true;
                d.triggers.push(Trigger { when: When::Enters, ability });
            },
        ),
        "Faebloom Trick" => s(
            &["Create two 1/1 blue Faerie creature tokens with flying. When you do, tap target creature an opponent controls."],
            |d, t| {
                spell(
                    d,
                    vec![],
                    vec![Effect::CreateToken { who: Who::You, def: t.faerie, n: 2 }, Effect::Reflexive(0)],
                );
                on(d, When::Never, vec![OPP_CREATURE], vec![Effect::Tap(0)]);
            },
        ),
        "Banishing Light" => s(
            &["When this enchantment enters, exile target nonland permanent an opponent controls until this enchantment leaves the battlefield."],
            |d, _| on(d, When::Enters, vec![OPP_NONLAND], vec![Effect::ExileUntilSourceLeaves(0)]),
        ),
        "Fake Your Own Death" => s(
            &["Until end of turn, target creature gets +2/+0 and gains \"When this creature dies, return it to the battlefield tapped under its owner's control and you create a Treasure token.\""],
            |d, t| {
                spell(
                    d,
                    vec![CREATURE],
                    vec![Effect::Pump { slot: 0, power: 2, toughness: 0 }, Effect::GrantDiesTrigger { slot: 0, trigger: 0 }],
                );
                on(
                    d,
                    When::Never,
                    vec![graveyard_cards(true, Controller::Any)],
                    vec![Effect::ReturnTapped(0), Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }],
                );
            },
        ),
        "Bloodfell Caves" => s(
            &[GAIN_LAND[0], GAIN_LAND[1], "{T}: Add {B} or {R}."],
            |d, _| gain_land(d, Color::Black, Color::Red),
        ),
        "Blossoming Sands" => s(
            &[GAIN_LAND[0], GAIN_LAND[1], "{T}: Add {G} or {W}."],
            |d, _| gain_land(d, Color::Green, Color::White),
        ),
        "Dismal Backwater" => s(
            &[GAIN_LAND[0], GAIN_LAND[1], "{T}: Add {U} or {B}."],
            |d, _| gain_land(d, Color::Blue, Color::Black),
        ),
        "Jungle Hollow" => s(
            &[GAIN_LAND[0], GAIN_LAND[1], "{T}: Add {B} or {G}."],
            |d, _| gain_land(d, Color::Black, Color::Green),
        ),
        "Rugged Highlands" => s(
            &[GAIN_LAND[0], GAIN_LAND[1], "{T}: Add {R} or {G}."],
            |d, _| gain_land(d, Color::Red, Color::Green),
        ),
        "Scoured Barrens" => s(
            &[GAIN_LAND[0], GAIN_LAND[1], "{T}: Add {W} or {B}."],
            |d, _| gain_land(d, Color::White, Color::Black),
        ),
        "Swiftwater Cliffs" => s(
            &[GAIN_LAND[0], GAIN_LAND[1], "{T}: Add {U} or {R}."],
            |d, _| gain_land(d, Color::Blue, Color::Red),
        ),
        "Thornwood Falls" => s(
            &[GAIN_LAND[0], GAIN_LAND[1], "{T}: Add {G} or {U}."],
            |d, _| gain_land(d, Color::Green, Color::Blue),
        ),
        "Tranquil Cove" => s(
            &[GAIN_LAND[0], GAIN_LAND[1], "{T}: Add {W} or {U}."],
            |d, _| gain_land(d, Color::White, Color::Blue),
        ),
        "Wind-Scarred Crag" => s(
            &[GAIN_LAND[0], GAIN_LAND[1], "{T}: Add {R} or {W}."],
            |d, _| gain_land(d, Color::Red, Color::White),
        ),
        "Evolving Wilds" => s(
            &["{T}, Sacrifice this land: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle."],
            |d, _| {
                let n = activated(d, "", true, vec![], vec![search(SearchFor::BasicLand, SearchTo::Battlefield { tapped: true })]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Rogue's Passage" => s(
            &["{T}: Add {C}.", "{4}, {T}: Target creature can't be blocked this turn."],
            |d, _| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                activated(d, "{4}", true, vec![CREATURE], vec![Effect::Grant { slot: 0, keywords: Keywords::UNBLOCKABLE }]);
            },
        ),
        "Llanowar Elves" => s(&["{T}: Add {G}."], |d, _| d.mana = Some(ManaSource::of(&[Color::Green]))),
        "Dazzling Angel" => s(&["Whenever another creature you control enters, you gain 1 life."], |d, _| {
            on(d, When::AnotherCreatureYouControlEnters, vec![], vec![Effect::GainLife { who: Who::You, n: 1 }])
        }),
        "Beast-Kin Ranger" => s(
            &["Whenever another creature you control enters, this creature gets +1/+0 until end of turn."],
            |d, _| on(d, When::AnotherCreatureYouControlEnters, vec![], vec![Effect::PumpSelf { power: 1, toughness: 0 }]),
        ),
        "Hungry Ghoul" => s(
            &["{1}, Sacrifice another creature: Put a +1/+1 counter on this creature."],
            |d, _| {
                let n = activated(d, "{1}", false, vec![], vec![Effect::CountersSelf { n: 1 }]);
                d.activated[n].sacrifice_other_creature = true;
            },
        ),
        "Cathar Commando" => s(
            &["{1}, Sacrifice this creature: Destroy target artifact or enchantment."],
            |d, _| {
                let n = activated(d, "{1}", false, vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::Destroy(0)]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Strix Lookout" => s(&["{1}{U}, {T}: Draw a card, then discard a card."], |d, _| {
            activated(d, "{1}{U}", true, vec![], vec![draw(1), discard(1)]);
        }),
        "Spectral Sailor" => s(&["{3}{U}: Draw a card."], |d, _| {
            activated(d, "{3}{U}", false, vec![], vec![draw(1)]);
        }),
        "Treetop Snarespinner" => s(
            &["{2}{G}: Put a +1/+1 counter on target creature you control. Activate only as a sorcery."],
            |d, _| {
                let n = activated(d, "{2}{G}", false, vec![YOUR_CREATURE], vec![Effect::Counters { slot: 0, n: 1 }]);
                d.activated[n].sorcery = true;
            },
        ),
        "Bite Down" => s(
            &["Target creature you control deals damage equal to its power to target creature or planeswalker you don't control."],
            |d, _| {
                let theirs = Filter { players: false, ..Filter::ANY }.controlled_by(Controller::Opponent);
                spell(d, vec![YOUR_CREATURE, theirs], vec![Effect::Bite { from: 0, to: 1 }])
            },
        ),
        "Vampire Soulcaller" => s(
            &["This creature can't block.", "When this creature enters, return target creature card from your graveyard to your hand."],
            |d, _| {
                d.cant_block = true;
                on(d, When::Enters, vec![graveyard_cards(true, Controller::You)], vec![Effect::Bounce(0)]);
            },
        ),
        "Elementalist Adept" => s(&["Prowess"], |d, _| {
            on(d, When::YouCastNoncreature, vec![], vec![Effect::PumpSelf { power: 1, toughness: 1 }]);
        }),
        "Ambush Wolf" => s(&["When this creature enters, exile up to one target card from a graveyard."], |d, _| {
            let ability = up_to(vec![graveyard_cards(false, Controller::Any)], vec![Effect::Exile(0)]);
            d.triggers.push(Trigger { when: When::Enters, ability });
        }),
        "Goblin Boarders" => s(
            &["Raid — This creature enters with a +1/+1 counter on it if you attacked this turn."],
            |d, _| d.enters_counters = Some((1, Condition::Attacked)),
        ),
        "Gnarlid Colony" => s(
            &[
                "Kicker {2}{G}",
                "If this creature was kicked, it enters with two +1/+1 counters on it.",
                "Each creature you control with a +1/+1 counter on it has trample.",
            ],
            |d, _| {
                d.kicker = ManaCost::parse("{2}{G}");
                d.enters_counters = Some((2, Condition::Kicked));
                static_on(d, Scope::YourCreaturesWithCounters, false, Keywords::TRAMPLE);
            },
        ),
        "Inspiring Paladin" => s(
            &[
                "During your turn, this creature has first strike.",
                "During your turn, creatures you control with +1/+1 counters on them have first strike.",
            ],
            |d, _| {
                static_on(d, Scope::This, true, Keywords::FIRST_STRIKE);
                static_on(d, Scope::YourCreaturesWithCounters, true, Keywords::FIRST_STRIKE);
            },
        ),
        "Courageous Goblin" => s(
            &["Whenever this creature attacks while you control a creature with power 4 or greater, this creature gets +1/+0 and gains menace until end of turn."],
            |d, _| {
                let mut ability = Ability::new(
                    vec![],
                    vec![Effect::PumpSelf { power: 1, toughness: 0 }, Effect::GrantSelf { keywords: Keywords::MENACE }],
                );
                ability.condition = Some(Condition::ControlPowerAtLeast(4));
                d.triggers.push(Trigger { when: When::Attacks, ability });
            },
        ),
        "Sun-Blessed Healer" => s(
            &[
                "Kicker {1}{W}",
                "When this creature enters, if it was kicked, return target nonland permanent card with mana value 2 or less from your graveyard to the battlefield.",
            ],
            |d, _| {
                d.kicker = ManaCost::parse("{1}{W}");
                let f = Filter {
                    nonland_permanents: true,
                    max_mana_value: Some(2),
                    ..graveyard_cards(false, Controller::You)
                };
                on_if(d, Condition::Kicked, vec![f], vec![Effect::Reanimate(0)]);
            },
        ),
        "Vanguard Seraph" => s(&["Whenever you gain life for the first time each turn, surveil 1."], |d, _| {
            on(d, When::YouGainLifeFirstTime { your_turn_only: false }, vec![], vec![Effect::Surveil { n: 1 }])
        }),
        "Cat Collector" => s(
            &[
                "When this creature enters, create a Food token.",
                "Whenever you gain life for the first time during each of your turns, create a 1/1 white Cat creature token.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.food, n: 1 }]);
                on(
                    d,
                    When::YouGainLifeFirstTime { your_turn_only: true },
                    vec![],
                    vec![Effect::CreateToken { who: Who::You, def: t.cat, n: 1 }],
                );
            },
        ),
        "Mischievous Mystic" => s(
            &["Whenever you draw your second card each turn, create a 1/1 blue Faerie creature token with flying."],
            |d, t| on(d, When::YouDrawSecond, vec![], vec![Effect::CreateToken { who: Who::You, def: t.faerie, n: 1 }]),
        ),
        "Abrade" => s(
            &["Choose one —", "• Abrade deals 3 damage to target creature.", "• Destroy target artifact."],
            |d, _| {
                modes(
                    d,
                    vec![
                        Ability::new(vec![CREATURE], vec![damage(3, Recipient::Target(0))]),
                        Ability::new(vec![ARTIFACT], vec![Effect::Destroy(0)]),
                    ],
                )
            },
        ),
        "Bushwhack" => s(
            &[
                "Choose one —",
                "• Search your library for a basic land card, reveal it, put it into your hand, then shuffle.",
                "• Target creature you control fights target creature you don't control.",
            ],
            |d, _| {
                modes(
                    d,
                    vec![
                        Ability::new(vec![], vec![search(SearchFor::BasicLand, SearchTo::Hand)]),
                        Ability::new(vec![YOUR_CREATURE, OPP_CREATURE], vec![Effect::FightBetween { a: 0, b: 1 }]),
                    ],
                )
            },
        ),
        "Goblin Surprise" => s(
            &[
                "Choose one —",
                "• Creatures you control get +2/+0 until end of turn.",
                "• Create two 1/1 red Goblin creature tokens.",
            ],
            |d, t| {
                modes(
                    d,
                    vec![
                        Ability::new(vec![], vec![Effect::PumpAll { who: Who::You, power: 2, toughness: 0 }]),
                        Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.goblin, n: 2 }]),
                    ],
                )
            },
        ),
        "Seeker's Folly" => s(
            &[
                "Choose one —",
                "• Target opponent discards two cards.",
                "• Creatures your opponents control get -1/-1 until end of turn.",
            ],
            |d, _| {
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                modes(
                    d,
                    vec![
                        Ability::new(vec![opp], vec![Effect::Discard { who: Who::TargetPlayer(0), n: 2 }]),
                        Ability::new(vec![], vec![Effect::PumpAll { who: Who::EachOpponent, power: -1, toughness: -1 }]),
                    ],
                )
            },
        ),
        "Slagstorm" => s(
            &[
                "Choose one —",
                "• Slagstorm deals 3 damage to each creature.",
                "• Slagstorm deals 3 damage to each player.",
            ],
            |d, _| {
                modes(
                    d,
                    vec![
                        Ability::new(vec![], vec![damage(3, Recipient::EachCreature { without_flying: false })]),
                        Ability::new(vec![], vec![damage(3, Recipient::EachPlayer)]),
                    ],
                )
            },
        ),
        "Run Away Together" => s(
            &["Choose two target creatures controlled by different players. Return those creatures to their owners' hands."],
            |d, _| spell(d, vec![YOUR_CREATURE, OPP_CREATURE], vec![Effect::Bounce(0), Effect::Bounce(1)]),
        ),
        "Goldvein Pick" => s(
            &[
                "Equipped creature gets +1/+1.",
                "Whenever equipped creature deals combat damage to a player, create a Treasure token.",
                "Equip {1}",
            ],
            |d, t| {
                attached(d, false, 1, 1, Keywords::NONE);
                on(
                    d,
                    When::EquippedDealsCombatDamageToPlayer,
                    vec![],
                    vec![Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }],
                );
                equip(d, "{1}");
            },
        ),
        "Quick-Draw Katana" => s(
            &["During your turn, equipped creature gets +2/+0 and has first strike.", "Equip {2}"],
            |d, _| {
                attached(d, true, 2, 0, Keywords::FIRST_STRIKE);
                equip(d, "{2}");
            },
        ),
        "Swiftfoot Boots" => s(&["Equipped creature has hexproof and haste.", "Equip {1}"], |d, _| {
            attached(d, false, 0, 0, Keywords::HEXPROOF.with(Keywords::HASTE));
            equip(d, "{1}");
        }),
        "Adventuring Gear" => s(
            &["Landfall — Whenever a land you control enters, equipped creature gets +2/+2 until end of turn.", "Equip {1}"],
            |d, _| {
                on(d, When::LandYouControlEnters, vec![], vec![Effect::PumpAttached { power: 2, toughness: 2 }]);
                equip(d, "{1}");
            },
        ),
        "Celestial Armor" => s(
            &[
                "When this Equipment enters, attach it to target creature you control. That creature gains hexproof and indestructible until end of turn.",
                "Equipped creature gets +2/+0 and has flying.",
                "Equip {3}{W}",
            ],
            |d, _| {
                on(
                    d,
                    When::Enters,
                    vec![YOUR_CREATURE],
                    vec![
                        Effect::Attach { slot: 0 },
                        Effect::Grant { slot: 0, keywords: Keywords::HEXPROOF.with(Keywords::INDESTRUCTIBLE) },
                    ],
                );
                attached(d, false, 2, 0, Keywords::FLYING);
                equip(d, "{3}{W}");
            },
        ),
        "Twinblade Blessing" => s(&["Enchant creature", "Enchanted creature has double strike."], |d, _| {
            d.aura = true;
            d.spell = Some(Ability::new(vec![CREATURE], vec![]));
            attached(d, false, 0, 0, Keywords::DOUBLE_STRIKE);
        }),
                "Sanguine Syphoner" => s(
            &["Whenever this creature attacks, each opponent loses 1 life and you gain 1 life."],
            |d, _| {
                on(
                    d,
                    When::Attacks,
                    vec![],
                    vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }, Effect::GainLife { who: Who::You, n: 1 }],
                )
            },
        ),
        "Ajani's Pridemate" => s(&["Whenever you gain life, put a +1/+1 counter on this creature."], |d, _| {
            on(d, When::YouGainLife, vec![], vec![Effect::CountersSelf { n: 1 }])
        }),
        "Marauding Blight-Priest" => s(&["Whenever you gain life, each opponent loses 1 life."], |d, _| {
            on(d, When::YouGainLife, vec![], vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }])
        }),
        "Elfsworn Giant" => s(
            &["Landfall — Whenever a land you control enters, create a 1/1 green Elf Warrior creature token."],
            |d, t| on(d, When::LandYouControlEnters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.elf_warrior, n: 1 }]),
        ),
        "Tatyova, Benthic Druid" => s(
            &["Landfall — Whenever a land you control enters, you gain 1 life and draw a card."],
            |d, _| on(d, When::LandYouControlEnters, vec![], vec![Effect::GainLife { who: Who::You, n: 1 }, draw(1)]),
        ),
        "Eager Trufflesnout" => s(
            &["Whenever this creature deals combat damage to a player, create a Food token."],
            |d, t| on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::CreateToken { who: Who::You, def: t.food, n: 1 }]),
        ),
        "Elvish Regrower" => s(
            &["When this creature enters, return target permanent card from your graveyard to your hand."],
            |d, _| {
                let f = Filter { permanent_cards: true, ..graveyard_cards(false, Controller::You) };
                on(d, When::Enters, vec![f], vec![Effect::Bounce(0)])
            },
        ),
        "Erudite Wizard" => s(
            &["Whenever you draw your second card each turn, put a +1/+1 counter on this creature."],
            |d, _| on(d, When::YouDrawSecond, vec![], vec![Effect::CountersSelf { n: 1 }]),
        ),
        "Fanatical Firebrand" => s(&["{T}, Sacrifice this creature: It deals 1 damage to any target."], |d, _| {
            let n = activated(d, "", true, vec![Filter::ANY], vec![damage(1, Recipient::Target(0))]);
            d.activated[n].sacrifice_self = true;
        }),
        "Axgard Cavalry" => s(&["{T}: Target creature gains haste until end of turn."], |d, _| {
            activated(d, "", true, vec![CREATURE], vec![Effect::Grant { slot: 0, keywords: Keywords::HASTE }]);
        }),
        "Sure Strike" => s(&["Target creature gets +3/+0 and gains first strike until end of turn."], |d, _| {
            spell(
                d,
                vec![CREATURE],
                vec![Effect::Pump { slot: 0, power: 3, toughness: 0 }, Effect::Grant { slot: 0, keywords: Keywords::FIRST_STRIKE }],
            )
        }),
        "Overrun" => s(&["Creatures you control get +3/+3 and gain trample until end of turn."], |d, _| {
            spell(
                d,
                vec![],
                vec![
                    Effect::PumpAll { who: Who::You, power: 3, toughness: 3 },
                    Effect::GrantAll { who: Who::You, keywords: Keywords::TRAMPLE },
                ],
            )
        }),
        "Heroic Reinforcements" => s(
            &["Create two 1/1 white Soldier creature tokens. Until end of turn, creatures you control get +1/+1 and gain haste."],
            |d, t| {
                spell(
                    d,
                    vec![],
                    vec![
                        Effect::CreateToken { who: Who::You, def: t.soldier, n: 2 },
                        Effect::PumpAll { who: Who::You, power: 1, toughness: 1 },
                        Effect::GrantAll { who: Who::You, keywords: Keywords::HASTE },
                    ],
                )
            },
        ),
        "Battlesong Berserker" => s(
            &["Whenever you attack, target creature you control gets +1/+0 and gains menace until end of turn."],
            |d, _| {
                on(
                    d,
                    When::YouAttack,
                    vec![YOUR_CREATURE],
                    vec![Effect::Pump { slot: 0, power: 1, toughness: 0 }, Effect::Grant { slot: 0, keywords: Keywords::MENACE }],
                )
            },
        ),
        "Youthful Valkyrie" => s(
            &["Whenever another Angel you control enters, put a +1/+1 counter on this creature."],
            |d, _| on(d, When::AnotherYouControlEntersWithSubtype("Angel"), vec![], vec![Effect::CountersSelf { n: 1 }]),
        ),
        "Vengeful Bloodwitch" => s(
            &["Whenever this creature or another creature you control dies, target opponent loses 1 life and you gain 1 life."],
            |d, _| {
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                on(
                    d,
                    When::CreatureYouControlDies,
                    vec![opp],
                    vec![Effect::LoseLife { who: Who::TargetPlayer(0), n: 1 }, Effect::GainLife { who: Who::You, n: 1 }],
                )
            },
        ),
        "Nessian Hornbeetle" => s(
            &["At the beginning of combat on your turn, if you control another creature with power 4 or greater, put a +1/+1 counter on this creature."],
            |d, _| {
                let mut ability = Ability::new(vec![], vec![Effect::CountersSelf { n: 1 }]);
                ability.condition = Some(Condition::ControlAnotherPowerAtLeast(4));
                d.triggers.push(Trigger { when: When::BeginCombatYourTurn, ability });
            },
        ),
        "Brineborn Cutthroat" => s(
            &["Whenever you cast a spell during an opponent's turn, put a +1/+1 counter on this creature."],
            |d, _| on(d, When::YouCastOnOpponentsTurn, vec![], vec![Effect::CountersSelf { n: 1 }]),
        ),
        "Firebrand Archer" => s(
            &["Whenever you cast a noncreature spell, this creature deals 1 damage to each opponent."],
            |d, _| on(d, When::YouCastNoncreature, vec![], vec![damage(1, Recipient::EachOpponent)]),
        ),
        "Empyrean Eagle" => s(&["Other creatures you control with flying get +1/+1."], |d, _| {
            d.statics.push(Static {
                subtype: None,
                lose_abilities: false,
                base_pt: None,
                protection: [false; 5],
                scale: None,
                scope: Scope::OtherYourFliers,
                your_turn_only: false,
                condition: None,
                power: 1,
                toughness: 1,
                keywords: Keywords::NONE, set_colors: None, becomes_land: false,
            });
        }),
        "Mocking Sprite" => s(&["Instant and sorcery spells you cast cost {1} less to cast."], |d, _| {
            d.spell_discount = 1;
        }),
        "Slumbering Cerberus" => s(
            &[
                "This creature doesn't untap during your untap step.",
                "Morbid — At the beginning of each end step, if a creature died this turn, untap this creature.",
            ],
            |d, _| {
                d.doesnt_untap = true;
                let mut ability = Ability::new(vec![], vec![Effect::UntapSelf]);
                ability.condition = Some(Condition::CreatureDiedThisTurn);
                d.triggers.push(Trigger { when: When::EndStep { your_turn_only: false }, ability });
            },
        ),
        "Needletooth Pack" => s(
            &["Morbid — At the beginning of your end step, if a creature died this turn, put two +1/+1 counters on target creature you control."],
            |d, _| {
                let mut ability = Ability::new(vec![YOUR_CREATURE], vec![Effect::Counters { slot: 0, n: 2 }]);
                ability.condition = Some(Condition::CreatureDiedThisTurn);
                d.triggers.push(Trigger { when: When::EndStep { your_turn_only: true }, ability });
            },
        ),
        "Billowing Shriekmass" => s(
            &[
                "When this creature enters, mill three cards.",
                "Threshold — This creature gets +2/+1 as long as there are seven or more cards in your graveyard.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::Mill { who: Who::You, n: 3 }]);
                threshold(d, 2, 1, Keywords::NONE);
            },
        ),
        "Dreadwing Scavenger" => s(
            &[
                "Whenever this creature enters or attacks, draw a card, then discard a card.",
                "Threshold — This creature gets +1/+1 and has deathtouch as long as there are seven or more cards in your graveyard.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![draw(1), discard(1)]);
                on(d, When::Attacks, vec![], vec![draw(1), discard(1)]);
                threshold(d, 1, 1, Keywords::DEATHTOUCH);
            },
        ),
                "Diregraf Ghoul" => s(&["This creature enters tapped."], |d, _| d.enters_tapped = true),
        "Dauntless Veteran" => s(
            &["Whenever this creature attacks, creatures you control get +1/+1 until end of turn."],
            |d, _| on(d, When::Attacks, vec![], vec![Effect::PumpAll { who: Who::You, power: 1, toughness: 1 }]),
        ),
        "Shivan Dragon" => s(&["{R}: This creature gets +1/+0 until end of turn."], |d, _| {
            activated(d, "{R}", false, vec![], vec![Effect::PumpSelf { power: 1, toughness: 0 }]);
        }),
        "Zombify" => s(&["Return target creature card from your graveyard to the battlefield."], |d, _| {
            spell(d, vec![graveyard_cards(true, Controller::You)], vec![Effect::Reanimate(0)])
        }),
        "Day of Judgment" => s(&["Destroy all creatures."], |d, _| spell(d, vec![], vec![Effect::DestroyAllCreatures])),
        "Phyrexian Arena" => s(&["At the beginning of your upkeep, you draw a card and lose 1 life."], |d, _| {
            on(d, When::Upkeep { your_turn_only: true }, vec![], vec![draw(1), Effect::LoseLife { who: Who::You, n: 1 }])
        }),
        "Scrawling Crawler" => s(
            &[
                "At the beginning of your upkeep, each player draws a card.",
                "Whenever an opponent draws a card, that player loses 1 life.",
            ],
            |d, _| {
                on(
                    d,
                    When::Upkeep { your_turn_only: true },
                    vec![],
                    vec![draw(1), Effect::Draw { who: Who::EachOpponent, n: 1 }],
                );
                on(d, When::OpponentDraws, vec![], vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }]);
            },
        ),
        "Rune-Sealed Wall" => s(&["{T}: Surveil 1."], |d, _| {
            activated(d, "", true, vec![], vec![Effect::Surveil { n: 1 }]);
        }),
        "Searslicer Goblin" => s(
            &["Raid — At the beginning of your end step, if you attacked this turn, create a 1/1 red Goblin creature token."],
            |d, t| {
                conditional(
                    d,
                    When::EndStep { your_turn_only: true },
                    Condition::Attacked,
                    vec![],
                    vec![Effect::CreateToken { who: Who::You, def: t.goblin, n: 1 }],
                )
            },
        ),
        "Good-Fortune Unicorn" => s(
            &["Whenever another creature you control enters, put a +1/+1 counter on that creature."],
            |d, _| {
                let that = Filter { triggering: true, ..CREATURE };
                on(d, When::AnotherCreatureYouControlEnters, vec![that], vec![Effect::Counters { slot: 0, n: 1 }])
            },
        ),
        "Clinquant Skymage" => s(&["Whenever you draw a card, put a +1/+1 counter on this creature."], |d, _| {
            on(d, When::YouDraw, vec![], vec![Effect::CountersSelf { n: 1 }])
        }),
        "Archmage of Runes" => s(
            &[
                "Instant and sorcery spells you cast cost {1} less to cast.",
                "Whenever you cast an instant or sorcery spell, draw a card.",
            ],
            |d, _| {
                d.spell_discount = 1;
                on(d, When::YouCastInstantOrSorcery, vec![], vec![draw(1)]);
            },
        ),
        "Balmor, Battlemage Captain" => s(
            &["Whenever you cast an instant or sorcery spell, creatures you control get +1/+0 and gain trample until end of turn."],
            |d, _| {
                on(
                    d,
                    When::YouCastInstantOrSorcery,
                    vec![],
                    vec![
                        Effect::PumpAll { who: Who::You, power: 1, toughness: 0 },
                        Effect::GrantAll { who: Who::You, keywords: Keywords::TRAMPLE },
                    ],
                )
            },
        ),
        "Firespitter Whelp" => s(
            &["Whenever you cast a noncreature or Dragon spell, this creature deals 1 damage to each opponent."],
            |d, _| on(d, When::YouCastNoncreatureOrSubtype("Dragon"), vec![], vec![damage(1, Recipient::EachOpponent)]),
        ),
        "Sower of Chaos" => s(&["{2}{R}: Target creature can't block this turn."], |d, _| {
            activated(d, "{2}{R}", false, vec![CREATURE], vec![Effect::Grant { slot: 0, keywords: Keywords::CANT_BLOCK }]);
        }),
        "Heartfire Immolator" => s(
            &["Prowess", "{R}, Sacrifice this creature: It deals damage equal to its power to target creature or planeswalker."],
            |d, _| {
                on(d, When::YouCastNoncreature, vec![], vec![Effect::PumpSelf { power: 1, toughness: 1 }]);
                let f = Filter { players: false, ..Filter::ANY };
                let n = activated(d, "{R}", false, vec![f], vec![Effect::DamageEqualSourcePower(0)]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Cackling Prowler" => s(
            &[
                "Ward {2}",
                "Morbid — At the beginning of your end step, if a creature died this turn, put a +1/+1 counter on this creature.",
            ],
            |d, _| {
                d.ward = ManaCost::parse("{2}");
                conditional(
                    d,
                    When::EndStep { your_turn_only: true },
                    Condition::CreatureDiedThisTurn,
                    vec![],
                    vec![Effect::CountersSelf { n: 1 }],
                );
            },
        ),
        "Apothecary Stomper" => s(
            &[
                "When this creature enters, choose one —",
                "• Put two +1/+1 counters on target creature you control.",
                "• You gain 4 life.",
            ],
            |d, _| {
                on_modal(
                    d,
                    When::Enters,
                    None,
                    vec![
                        Ability::new(vec![YOUR_CREATURE], vec![Effect::Counters { slot: 0, n: 2 }]),
                        Ability::new(vec![], vec![Effect::GainLife { who: Who::You, n: 4 }]),
                    ],
                )
            },
        ),
        "Wardens of the Cycle" => s(
            &[
                "Morbid — At the beginning of your end step, if a creature died this turn, choose one —",
                "• You gain 2 life.",
                "• You draw a card and you lose 1 life.",
            ],
            |d, _| {
                on_modal(
                    d,
                    When::EndStep { your_turn_only: true },
                    Some(Condition::CreatureDiedThisTurn),
                    vec![
                        Ability::new(vec![], vec![Effect::GainLife { who: Who::You, n: 2 }]),
                        Ability::new(vec![], vec![draw(1), Effect::LoseLife { who: Who::You, n: 1 }]),
                    ],
                )
            },
        ),
        "Ruby, Daring Tracker" => s(
            &[
                "Whenever Ruby attacks while you control a creature with power 4 or greater, Ruby gets +2/+2 until end of turn.",
                "{T}: Add {R} or {G}.",
            ],
            |d, _| {
                conditional(
                    d,
                    When::Attacks,
                    Condition::ControlPowerAtLeast(4),
                    vec![],
                    vec![Effect::PumpSelf { power: 2, toughness: 2 }],
                );
                d.mana = Some(ManaSource::of(&[Color::Red, Color::Green]));
            },
        ),
                "Anthem of Champions" => s(&["Creatures you control get +1/+1."], |d, _| {
            d.statics.push(Static {
                subtype: None,
                lose_abilities: false,
                base_pt: None,
                protection: [false; 5],
                scale: None,
                scope: Scope::YourCreatures,
                your_turn_only: false,
                condition: None,
                power: 1,
                toughness: 1,
                keywords: Keywords::NONE, set_colors: None, becomes_land: false,
            });
        }),
        "Crackling Cyclops" => s(
            &["Whenever you cast a noncreature spell, this creature gets +3/+0 until end of turn."],
            |d, _| on(d, When::YouCastNoncreature, vec![], vec![Effect::PumpSelf { power: 3, toughness: 0 }]),
        ),
        "Spitfire Lagac" => s(
            &["Landfall — Whenever a land you control enters, this creature deals 1 damage to each opponent."],
            |d, _| on(d, When::LandYouControlEnters, vec![], vec![damage(1, Recipient::EachOpponent)]),
        ),
        "Preposterous Proportions" => s(
            &["Creatures you control get +10/+10 and gain vigilance until end of turn."],
            |d, _| {
                spell(
                    d,
                    vec![],
                    vec![
                        Effect::PumpAll { who: Who::You, power: 10, toughness: 10 },
                        Effect::GrantAll { who: Who::You, keywords: Keywords::VIGILANCE },
                    ],
                )
            },
        ),
        "Goblin Bushwhacker" => s(
            &[
                "Kicker {R}",
                "When this creature enters, if it was kicked, creatures you control get +1/+0 and gain haste until end of turn.",
            ],
            |d, _| {
                d.kicker = ManaCost::parse("{R}");
                on_if(
                    d,
                    Condition::Kicked,
                    vec![],
                    vec![
                        Effect::PumpAll { who: Who::You, power: 1, toughness: 0 },
                        Effect::GrantAll { who: Who::You, keywords: Keywords::HASTE },
                    ],
                );
            },
        ),
        "Burnished Hart" => s(
            &["{3}, Sacrifice this creature: Search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle."],
            |d, _| {
                let to = SearchTo::Battlefield { tapped: true };
                let n = activated(d, "{3}", false, vec![], vec![Effect::Search { what: SearchFor::BasicLand, to, n: 2, kicked_n: None }]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Rite of the Dragoncaller" => s(
            &["Whenever you cast an instant or sorcery spell, create a 5/5 red Dragon creature token with flying."],
            |d, t| on(d, When::YouCastInstantOrSorcery, vec![], vec![Effect::CreateToken { who: Who::You, def: t.dragon_5, n: 1 }]),
        ),
        "Crypt Feaster" => s(
            &["Threshold — Whenever this creature attacks, if there are seven or more cards in your graveyard, this creature gets +2/+0 until end of turn."],
            |d, _| conditional(d, When::Attacks, Condition::Threshold, vec![], vec![Effect::PumpSelf { power: 2, toughness: 0 }]),
        ),
        "Paradise Druid" => s(
            &["This creature has hexproof as long as it's untapped.", "{T}: Add one mana of any color."],
            |d, _| {
                d.statics.push(Static {
                    subtype: None,
                    lose_abilities: false,
                    base_pt: None,
                    protection: [false; 5],
                    scale: None,
                    scope: Scope::This,
                    your_turn_only: false,
                    condition: Some(StaticCondition::Untapped),
                    power: 0,
                    toughness: 0,
                    keywords: Keywords::HEXPROOF, set_colors: None, becomes_land: false,
                });
                d.mana = Some(ManaSource::of(&Color::ALL));
            },
        ),
        "Condemn" => s(
            &["Put target attacking creature on the bottom of its owner's library. Its controller gains life equal to its toughness."],
            |d, _| spell(d, vec![Filter { in_combat: true, ..CREATURE }], vec![Effect::BottomAndGainToughness(0)]),
        ),
        "Aetherize" => s(&["Return all attacking creatures to their owner's hand."], |d, _| {
            spell(d, vec![], vec![Effect::BounceAttackers])
        }),
        "Juggernaut" => s(
            &["This creature attacks each combat if able.", "This creature can't be blocked by Walls."],
            |d, _| {
                d.must_attack = true;
                d.not_blocked_by = Some("Wall");
            },
        ),
        "Raise the Past" => s(
            &["Return all creature cards with mana value 2 or less from your graveyard to the battlefield."],
            |d, _| spell(d, vec![], vec![Effect::ReanimateAll { all_graveyards: false, max_mana_value: Some(2) }]),
        ),
        "Rise of the Dark Realms" => s(
            &["Put all creature cards from all graveyards onto the battlefield under your control."],
            |d, _| spell(d, vec![], vec![Effect::ReanimateAll { all_graveyards: true, max_mana_value: None }]),
        ),
        "Authority of the Consuls" => s(
            &[
                "Creatures your opponents control enter tapped.",
                "Whenever a creature an opponent controls enters, you gain 1 life.",
            ],
            |d, _| {
                d.opponents_creatures_enter_tapped = true;
                on(d, When::CreatureOpponentControlsEnters, vec![], vec![Effect::GainLife { who: Who::You, n: 1 }]);
            },
        ),
        "Mossborn Hydra" => s(
            &[
                "This creature enters with a +1/+1 counter on it.",
                "Landfall — Whenever a land you control enters, double the number of +1/+1 counters on this creature.",
            ],
            |d, _| {
                d.enters_counters = Some((1, Condition::Always));
                on(d, When::LandYouControlEnters, vec![], vec![Effect::DoubleCountersSelf]);
            },
        ),
        "Herald of Eternal Dawn" => s(&["You can't lose the game and your opponents can't win the game."], |d, _| {
            d.cant_lose = true;
        }),
        "Bulk Up" => s(&["Double target creature's power until end of turn.", "Flashback {4}{R}{R}"], |d, _| {
            d.flashback = ManaCost::parse("{4}{R}{R}");
            spell(d, vec![CREATURE], vec![Effect::DoublePower(0)])
        }),
                "Vampire Gourmand" => s(
            &["Whenever this creature attacks, you may sacrifice another creature. If you do, draw a card and this creature can't be blocked this turn."],
            |d, _| {
                on(
                    d,
                    When::Attacks,
                    vec![],
                    vec![Effect::MaySacrificeAnother, draw(1), Effect::GrantSelf { keywords: Keywords::UNBLOCKABLE }],
                )
            },
        ),
        "High-Society Hunter" => s(
            &[
                "Whenever this creature attacks, you may sacrifice another creature. If you do, put a +1/+1 counter on this creature.",
                "Whenever another nontoken creature dies, draw a card.",
            ],
            |d, _| {
                on(d, When::Attacks, vec![], vec![Effect::MaySacrificeAnother, Effect::CountersSelf { n: 1 }]);
                on(d, When::AnotherNontokenCreatureDies, vec![], vec![draw(1)]);
            },
        ),
        "Reassembling Skeleton" => s(&["{1}{B}: Return this card from your graveyard to the battlefield tapped."], |d, _| {
            let n = activated(d, "{1}{B}", false, vec![], vec![Effect::ReturnSelfTapped]);
            d.activated[n].from_graveyard = true;
        }),
        "Gutless Plunderer" => s(
            &["Raid — When this creature enters, if you attacked this turn, look at the top three cards of your library. You may put one of those cards back on top of your library. Put the rest into your graveyard."],
            |d, _| {
                on_if(
                    d,
                    Condition::Attacked,
                    vec![],
                    vec![Effect::LookAtTop { n: 3, pick: Pick::Any, chosen_to: LookTo::Top, rest_to: LookRest::Graveyard }],
                )
            },
        ),
        "Squad Rallier" => s(
            &["{2}{W}: Look at the top four cards of your library. You may reveal a creature card with power 2 or less from among them and put it into your hand. Put the rest on the bottom of your library in a random order."],
            |d, _| {
                let look = Effect::LookAtTop {
                    n: 4,
                    pick: Pick::CreaturePowerAtMost(2),
                    chosen_to: LookTo::Hand,
                    rest_to: LookRest::BottomRandom,
                };
                activated(d, "{2}{W}", false, vec![], vec![look]);
            },
        ),
        "Inspiration from Beyond" => s(
            &[
                "Mill three cards, then return an instant or sorcery card from your graveyard to your hand.",
                "Flashback {5}{U}{U}",
            ],
            |d, _| {
                d.flashback = ManaCost::parse("{5}{U}{U}");
                spell(
                    d,
                    vec![],
                    vec![Effect::Mill { who: Who::You, n: 3 }, Effect::ReturnFromGraveyard { pick: Pick::InstantOrSorcery }],
                )
            },
        ),
        "Alesha, Who Laughs at Fate" => s(
            &[
                "Whenever Alesha attacks, put a +1/+1 counter on it.",
                "Raid — At the beginning of your end step, if you attacked this turn, return target creature card with mana value less than or equal to Alesha's power from your graveyard to the battlefield.",
            ],
            |d, _| {
                on(d, When::Attacks, vec![], vec![Effect::CountersSelf { n: 1 }]);
                let f = Filter { mana_value_at_most_source_power: true, ..graveyard_cards(true, Controller::You) };
                conditional(d, When::EndStep { your_turn_only: true }, Condition::Attacked, vec![f], vec![Effect::Reanimate(0)]);
            },
        ),
        "Fiendish Panda" => s(
            &[
                "Whenever you gain life, put a +1/+1 counter on this creature.",
                "When this creature dies, return another target non-Bear creature card with mana value less than or equal to this creature's power from your graveyard to the battlefield.",
            ],
            |d, _| {
                on(d, When::YouGainLife, vec![], vec![Effect::CountersSelf { n: 1 }]);
                let f = Filter {
                    other: true,
                    not_subtype: Some("Bear"),
                    mana_value_at_most_source_power: true,
                    ..graveyard_cards(true, Controller::You)
                };
                on(d, When::Dies, vec![f], vec![Effect::Reanimate(0)]);
            },
        ),
        "Kiora, the Rising Tide" => s(
            &[
                "When Kiora enters, draw two cards, then discard two cards.",
                "Threshold — Whenever Kiora attacks, if there are seven or more cards in your graveyard, you may create Scion of the Deep, a legendary 8/8 blue Octopus creature token.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![draw(2), discard(2)]);
                let mut ability = Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.scion, n: 1 }]).optional();
                ability.condition = Some(Condition::Threshold);
                d.triggers.push(Trigger { when: When::Attacks, ability });
            },
        ),
        "Cephalid Inkmage" => s(
            &[
                "When this creature enters, surveil 3.",
                "Threshold — This creature can't be blocked as long as there are seven or more cards in your graveyard.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::Surveil { n: 3 }]);
                threshold(d, 0, 0, Keywords::UNBLOCKABLE);
            },
        ),
        "Stromkirk Bloodthief" => s(
            &["At the beginning of your end step, if an opponent lost life this turn, put a +1/+1 counter on target Vampire you control."],
            |d, _| {
                let f = Filter { subtype: Some("Vampire"), ..YOUR_CREATURE };
                conditional(
                    d,
                    When::EndStep { your_turn_only: true },
                    Condition::OpponentLostLifeThisTurn,
                    vec![f],
                    vec![Effect::Counters { slot: 0, n: 1 }],
                )
            },
        ),
                "Mischievous Pup" => s(
            &["When this creature enters, return up to one other target permanent you control to its owner's hand."],
            |d, _| {
                let f = Filter {
                    players: false,
                    creatures: false,
                    planeswalkers: false,
                    any_permanent: true,
                    other: true,
                    controller: Controller::You,
                    ..Filter::ANY
                };
                d.triggers.push(Trigger { when: When::Enters, ability: up_to(vec![f], vec![Effect::Bounce(0)]) });
            },
        ),
        "Arahbo, the First Fang" => s(
            &[
                "Other Cats you control get +1/+1.",
                "Whenever Arahbo or another nontoken Cat you control enters, create a 1/1 white Cat creature token.",
            ],
            |d, t| {
                d.statics.push(Static {
                    subtype: None,
                    lose_abilities: false,
                    base_pt: None,
                    protection: [false; 5],
                    scale: None,
                    scope: Scope::OtherYourSubtype("Cat"),
                    your_turn_only: false,
                    condition: None,
                    power: 1,
                    toughness: 1,
                    keywords: Keywords::NONE, set_colors: None, becomes_land: false,
                });
                on(
                    d,
                    When::NontokenYouControlEntersWithSubtype("Cat"),
                    vec![],
                    vec![Effect::CreateToken { who: Who::You, def: t.cat, n: 1 }],
                );
            },
        ),
        "Scavenging Ooze" => s(
            &["{G}: Exile target card from a graveyard. If it was a creature card, put a +1/+1 counter on this creature and you gain 1 life."],
            |d, _| {
                activated(
                    d,
                    "{G}",
                    false,
                    vec![graveyard_cards(false, Controller::Any)],
                    vec![Effect::Exile(0), Effect::CounterAndGainIfCreatureExiled { life: 1 }],
                );
            },
        ),
        "Garruk's Uprising" => s(
            &[
                "When this enchantment enters, if you control a creature with power 4 or greater, draw a card.",
                "Creatures you control have trample.",
                "Whenever a creature you control with power 4 or greater enters, draw a card.",
            ],
            |d, _| {
                on_if(d, Condition::ControlPowerAtLeast(4), vec![], vec![draw(1)]);
                static_on(d, Scope::YourCreatures, false, Keywords::TRAMPLE);
                on(d, When::YourCreatureEntersWithPowerAtLeast(4), vec![], vec![draw(1)]);
            },
        ),
        "Armasaur Guide" => s(
            &["Whenever you attack with three or more creatures, put a +1/+1 counter on target creature you control."],
            |d, _| on(d, When::YouAttackWithAtLeast(3), vec![YOUR_CREATURE], vec![Effect::Counters { slot: 0, n: 1 }]),
        ),
        "Grappling Kraken" => s(
            &["Landfall — Whenever a land you control enters, tap target creature an opponent controls and put a stun counter on it."],
            |d, _| on(d, When::LandYouControlEnters, vec![OPP_CREATURE], vec![Effect::TapAndStun(0)]),
        ),
        "Sylvan Scavenging" => s(
            &[
                "At the beginning of your end step, choose one —",
                "• Put a +1/+1 counter on target creature you control.",
                "• Create a 3/3 green Raccoon creature token if you control a creature with power 4 or greater.",
            ],
            |d, t| {
                on_modal(
                    d,
                    When::EndStep { your_turn_only: true },
                    None,
                    vec![
                        Ability::new(vec![YOUR_CREATURE], vec![Effect::Counters { slot: 0, n: 1 }]),
                        Ability::new(
                            vec![],
                            vec![
                                Effect::GateControlPowerAtLeast(4),
                                Effect::CreateToken { who: Who::You, def: t.raccoon, n: 1 },
                            ],
                        ),
                    ],
                )
            },
        ),
        "Frenzied Goblin" => s(
            &["Whenever this creature attacks, you may pay {R}. If you do, target creature can't block this turn."],
            |d, _| {
                let r = ManaCost::parse("{R}").expect("cost");
                on(
                    d,
                    When::Attacks,
                    vec![CREATURE],
                    vec![Effect::MayPay(r), Effect::Grant { slot: 0, keywords: Keywords::CANT_BLOCK }],
                )
            },
        ),
        "Sire of Seven Deaths" => s(&["Ward—Pay 7 life."], |d, _| d.ward_life = Some(7)),
        "Koma, World-Eater" => s(
            &[
                "This spell can't be countered.",
                "Trample, ward {4}",
                "Whenever Koma deals combat damage to a player, create four 3/3 blue Serpent creature tokens named Koma's Coil.",
            ],
            |d, t| {
                d.uncounterable = true;
                d.keywords = d.keywords.with(Keywords::TRAMPLE);
                d.ward = ManaCost::parse("{4}");
                on(
                    d,
                    When::DealsCombatDamageToPlayer,
                    vec![],
                    vec![Effect::CreateToken { who: Who::You, def: t.komas_coil, n: 4 }],
                );
            },
        ),
        "Drakuseth, Maw of Flames" => s(
            &["Whenever Drakuseth attacks, Drakuseth deals 4 damage to any target and 3 damage to each of up to two other targets."],
            |d, _| {
                let mut ability = Ability::new(
                    vec![Filter::ANY, Filter::ANY, Filter::ANY],
                    vec![damage(4, Recipient::Target(0)), damage(3, Recipient::Target(1)), damage(3, Recipient::Target(2))],
                );
                ability.min_targets = Some(1);
                d.triggers.push(Trigger { when: When::Attacks, ability });
            },
        ),
        "Painful Quandary" => s(
            &["Whenever an opponent casts a spell, that player loses 5 life unless they discard a card."],
            |d, _| on(d, When::OpponentCasts, vec![], vec![Effect::LoseUnlessDiscard { n: 5 }]),
        ),
        "Exemplar of Light" => s(
            &[
                "Whenever you gain life, put a +1/+1 counter on this creature.",
                "Whenever you put one or more +1/+1 counters on this creature, draw a card. This ability triggers only once each turn.",
            ],
            |d, _| {
                on(d, When::YouGainLife, vec![], vec![Effect::CountersSelf { n: 1 }]);
                on(d, When::CountersPutOnThis, vec![], vec![draw(1)]);
            },
        ),
        "Claws Out" => s(&["Affinity for Cats", "Creatures you control get +2/+2 until end of turn."], |d, _| {
            d.reduction = Some(Reduction::PerYouControl("Cat"));
            spell(d, vec![], vec![Effect::PumpAll { who: Who::You, power: 2, toughness: 2 }])
        }),
        "Spinner of Souls" => s(
            &["Whenever another nontoken creature you control dies, you may reveal cards from the top of your library until you reveal a creature card. Put that card into your hand and the rest on the bottom of your library in a random order."],
            |d, _| may(d, When::AnotherNontokenCreatureYouControlDies, vec![], vec![Effect::RevealUntilCreature]),
        ),
        "Zimone, Paradox Sculptor" => s(
            &[
                "At the beginning of combat on your turn, put a +1/+1 counter on each of up to two target creatures you control.",
                "{G}{U}, {T}: Double the number of each kind of counter on up to two target creatures and/or artifacts you control.",
            ],
            |d, _| {
                d.triggers.push(Trigger {
                    when: When::BeginCombatYourTurn,
                    ability: up_to(
                        vec![YOUR_CREATURE, YOUR_CREATURE],
                        vec![Effect::Counters { slot: 0, n: 1 }, Effect::Counters { slot: 1, n: 1 }],
                    ),
                });
                let f = Filter { artifacts: true, ..YOUR_CREATURE };
                d.activated.push(Activated {
                    mana: ManaCost::parse("{G}{U}").expect("cost"),
                    tap: true,
                    ability: up_to(vec![f, f], vec![Effect::DoubleCounters(0), Effect::DoubleCounters(1)]),
                    ..Activated::default()
                });
            },
        ),
                "Bloodthirsty Conqueror" => s(&["Whenever an opponent loses life, you gain that much life."], |d, _| {
            on(d, When::OpponentLosesLife, vec![], vec![Effect::GainLifeAmount])
        }),
        "Niv-Mizzet, Visionary" => s(
            &[
                "You have no maximum hand size.",
                "Whenever a source you control deals noncombat damage to an opponent, you draw that many cards.",
            ],
            |d, _| {
                d.no_max_hand_size = true;
                on(d, When::YourSourceDealsNoncombatDamageToOpponent, vec![], vec![Effect::DrawAmount]);
            },
        ),
        "Drake Hatcher" => s(
            &[
                "Vigilance, prowess",
                "Whenever this creature deals combat damage to a player, put that many incubation counters on it.",
                "Remove three incubation counters from this creature: Create a 2/2 blue Drake creature token with flying.",
            ],
            |d, t| {
                d.keywords = d.keywords.with(Keywords::VIGILANCE);
                on(d, When::YouCastNoncreature, vec![], vec![Effect::PumpSelf { power: 1, toughness: 1 }]);
                on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::IncubationAmount]);
                let n = activated(d, "", false, vec![], vec![Effect::CreateToken { who: Who::You, def: t.drake, n: 1 }]);
                d.activated[n].remove_incubation = 3;
            },
        ),
        "Infernal Vessel" => s(
            &["When this creature dies, if it wasn't a Demon, return it to the battlefield under its owner's control with two +1/+1 counters on it. It's a Demon in addition to its other types."],
            |d, _| {
                conditional(
                    d,
                    When::Dies,
                    Condition::SourceWasNot("Demon"),
                    vec![],
                    vec![Effect::ReturnWith { slot: None, counters: 2, subtype: Some("Demon"), keywords: Keywords::NONE }],
                )
            },
        ),
        "Valkyrie's Call" => s(
            &["Whenever a nontoken, non-Angel creature you control dies, return that card to the battlefield under its owner's control with a +1/+1 counter on it. It has flying and is an Angel in addition to its other types."],
            |d, _| {
                let that = Filter { triggering: true, ..graveyard_cards(true, Controller::Any) };
                on(
                    d,
                    When::YourNontokenCreatureDiesWithout("Angel"),
                    vec![that],
                    vec![Effect::ReturnWith { slot: Some(0), counters: 1, subtype: Some("Angel"), keywords: Keywords::FLYING }],
                )
            },
        ),
        "Skyknight Squire" => s(
            &[
                "Whenever another creature you control enters, put a +1/+1 counter on this creature.",
                "As long as this creature has three or more +1/+1 counters on it, it has flying and is a Knight in addition to its other types.",
            ],
            |d, _| {
                on(d, When::AnotherCreatureYouControlEnters, vec![], vec![Effect::CountersSelf { n: 1 }]);
                d.statics.push(Static {
                    subtype: Some("Knight"),
                    lose_abilities: false,
                    base_pt: None,
                    protection: [false; 5],
                    scale: None,
                    scope: Scope::This,
                    your_turn_only: false,
                    condition: Some(StaticCondition::CountersAtLeast(3)),
                    power: 0,
                    toughness: 0,
                    keywords: Keywords::FLYING, set_colors: None, becomes_land: false,
                });
            },
        ),
        "Mild-Mannered Librarian" => s(
            &["{3}{G}: This creature becomes a Werewolf. Put two +1/+1 counters on it and you draw a card. Activate only once."],
            |d, _| {
                let n = activated(
                    d,
                    "{3}{G}",
                    false,
                    vec![],
                    vec![Effect::BecomeSubtype("Werewolf"), Effect::CountersSelf { n: 2 }, draw(1)],
                );
                d.activated[n].once = true;
            },
        ),
        "Divine Resilience" => s(
            &[
                "Kicker {2}{W}",
                "Target creature you control gains indestructible until end of turn. If this spell was kicked, instead any number of target creatures you control gain indestructible until end of turn.",
            ],
            |d, _| {
                d.kicker = ManaCost::parse("{2}{W}");
                let grant = |slot| Effect::Grant { slot, keywords: Keywords::INDESTRUCTIBLE };
                spell(d, vec![YOUR_CREATURE], vec![grant(0)]);
                // "Any number": up to eight targets (see docs/engine/QUESTIONS.md).
                d.kicked_spell = Some(up_to(vec![YOUR_CREATURE; 8], (0..8).map(grant).collect()));
            },
        ),
        "Loot, Exuberant Explorer" => s(
            &[
                "You may play an additional land on each of your turns.",
                "{4}{G}{G}, {T}: Look at the top six cards of your library. You may reveal a creature card with mana value less than or equal to the number of lands you control from among them and put it onto the battlefield. Put the rest on the bottom in a random order.",
            ],
            |d, _| {
                d.extra_land_drops = 1;
                let look = Effect::LookAtTop {
                    n: 6,
                    pick: Pick::CreatureMvAtMostLands,
                    chosen_to: LookTo::Battlefield,
                    rest_to: LookRest::BottomRandom,
                };
                activated(d, "{4}{G}{G}", true, vec![], vec![look]);
            },
        ),
        "Perforating Artist" => s(
            &["Raid — At the beginning of your end step, if you attacked this turn, each opponent loses 3 life unless that player sacrifices a nonland permanent of their choice or discards a card."],
            |d, _| {
                conditional(
                    d,
                    When::EndStep { your_turn_only: true },
                    Condition::Attacked,
                    vec![],
                    vec![Effect::LoseUnlessSacrificeOrDiscard { n: 3 }],
                )
            },
        ),
        "Consuming Aberration" => s(
            &[
                "Consuming Aberration's power and toughness are each equal to the number of cards in your opponents' graveyards.",
                "Whenever you cast a spell, each opponent reveals cards from the top of their library until they reveal a land card, then puts those cards into their graveyard.",
            ],
            |d, _| {
                d.pt_opponents_graveyards = true;
                on(d, When::YouCast, vec![], vec![Effect::OpponentsMillUntilLand]);
            },
        ),
        "Elenda, Saint of Dusk" => s(
            &[
                "Lifelink, hexproof from instants",
                "As long as your life total is greater than your starting life total, Elenda gets +1/+1 and has menace. Elenda gets an additional +5/+5 as long as your life total is at least 10 greater than your starting life total.",
            ],
            |d, _| {
                d.keywords = d.keywords.with(Keywords::LIFELINK);
                d.hexproof_from_instants = true;
                let life = |at_least, power, keywords| Static {
                    subtype: None,
                    lose_abilities: false,
                    base_pt: None,
                    protection: [false; 5],
                    scale: None,
                    scope: Scope::This,
                    your_turn_only: false,
                    condition: Some(StaticCondition::LifeAtLeast(at_least)),
                    power,
                    toughness: power,
                    keywords, set_colors: None, becomes_land: false,
                };
                d.statics.push(life(21, 1, Keywords::MENACE));
                d.statics.push(life(30, 5, Keywords::NONE));
            },
        ),
                "Blasphemous Edict" => s(
            &[
                "You may pay {B} rather than pay this spell's mana cost if there are thirteen or more creatures on the battlefield.",
                "Each player sacrifices thirteen creatures of their choice.",
            ],
            |d, _| {
                d.alt_cost = Some((ManaCost::parse("{B}").expect("cost"), 13));
                spell(
                    d,
                    vec![],
                    vec![
                        Effect::SacrificeCreatures { who: Who::You, n: 13 },
                        Effect::SacrificeCreatures { who: Who::EachOpponent, n: 13 },
                    ],
                )
            },
        ),
        "Leyline Axe" => s(
            &[
                "If this card is in your opening hand, you may begin the game with it on the battlefield.",
                "Equipped creature gets +1/+1 and has double strike and trample.",
                "Equip {3}",
            ],
            |d, _| {
                d.leyline = true;
                attached(d, false, 1, 1, Keywords::DOUBLE_STRIKE.with(Keywords::TRAMPLE));
                equip(d, "{3}");
            },
        ),
        "High Fae Trickster" => s(&["You may cast spells as though they had flash."], |d, _| d.grants_flash = true),
        "Flamewake Phoenix" => s(
            &[
                "This creature attacks each combat if able.",
                "Ferocious — At the beginning of combat on your turn, if you control a creature with power 4 or greater, you may pay {R}. If you do, return this card from your graveyard to the battlefield.",
            ],
            |d, _| {
                d.must_attack = true;
                conditional(
                    d,
                    When::BeginCombatYourTurnFromGraveyard,
                    Condition::ControlPowerAtLeast(4),
                    vec![],
                    vec![
                        Effect::MayPay(ManaCost::parse("{R}").expect("cost")),
                        Effect::ReturnWith { slot: None, counters: 0, subtype: None, keywords: Keywords::NONE },
                    ],
                );
            },
        ),
        "Sphinx of Forgotten Lore" => s(
            &["Whenever this creature attacks, target instant or sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to that card's mana cost."],
            |d, _| {
                let f = Filter { instant_or_sorcery: true, ..graveyard_cards(false, Controller::You) };
                on(d, When::Attacks, vec![f], vec![Effect::GrantFlashback(0)]);
            },
        ),
        "Ravenous Amulet" => s(
            &[
                "{1}, {T}, Sacrifice a creature: Draw a card and put a soul counter on this artifact. Activate only as a sorcery.",
                "{4}, {T}, Sacrifice this artifact: Each opponent loses life equal to the number of soul counters on this artifact.",
            ],
            |d, _| {
                let n = activated(d, "{1}", true, vec![], vec![draw(1), Effect::SoulCounterSelf]);
                d.activated[n].sacrifice_other_creature = true;
                d.activated[n].sorcery = true;
                let m = activated(d, "{4}", true, vec![], vec![Effect::OpponentsLoseSoul]);
                d.activated[m].sacrifice_self = true;
            },
        ),
        "Lathril, Blade of the Elves" => s(
            &[
                "Whenever Lathril deals combat damage to a player, create that many 1/1 green Elf Warrior creature tokens.",
                "{T}, Tap ten untapped Elves you control: Each opponent loses 10 life and you gain 10 life.",
            ],
            |d, t| {
                on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::CreateTokensAmount { def: t.elf_warrior }]);
                let n = activated(
                    d,
                    "",
                    true,
                    vec![],
                    vec![Effect::LoseLife { who: Who::EachOpponent, n: 10 }, Effect::GainLife { who: Who::You, n: 10 }],
                );
                d.activated[n].tap_untapped = Some(("Elf", 10));
            },
        ),
                "Kykar, Zephyr Awakener" => s(
            &[
                "Whenever you cast a noncreature spell, choose one —",
                "• Exile another target creature you control. Return that card to the battlefield under its owner's control at the beginning of the next end step.",
                "• Create a 1/1 white Spirit creature token with flying.",
            ],
            |d, t| {
                let another = Filter { other: true, ..YOUR_CREATURE };
                on_modal(
                    d,
                    When::YouCastNoncreature,
                    None,
                    vec![
                        Ability::new(vec![another], vec![Effect::Flicker(0)]),
                        Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.spirit, n: 1 }]),
                    ],
                )
            },
        ),
        "Nine-Lives Familiar" => s(
            &[
                "This creature enters with eight revival counters on it if you cast it.",
                "When this creature dies, if it had a revival counter on it, return it to the battlefield with one fewer revival counter on it at the beginning of the next end step.",
            ],
            |d, _| {
                d.revival_on_cast = true;
                conditional(d, When::Dies, Condition::HadRevival, vec![], vec![Effect::ReturnNextEndStepWithFewerRevival]);
            },
        ),
        "Time Stop" => s(&["End the turn."], |d, _| spell(d, vec![], vec![Effect::EndTheTurn])),
        "Witness Protection" => s(
            &[
                "Enchant creature",
                "Enchanted creature loses all abilities and is a green and white Citizen creature with base power and toughness 1/1 named Legitimate Businessperson.",
            ],
            |d, _| {
                d.aura = true;
                d.spell = Some(Ability::new(vec![CREATURE], vec![]));
                d.statics.push(Static {
                    subtype: None,
                    lose_abilities: true,
                    base_pt: Some((1, 1)),
                    protection: [false; 5],
                    scale: None,
                    scope: Scope::Attached,
                    your_turn_only: false,
                    condition: None,
                    power: 0,
                    toughness: 0,
                    keywords: Keywords::NONE, set_colors: Some([true, false, false, false, true]), becomes_land: false,
                });
            },
        ),
        "Heraldic Banner" => s(
            &[
                "As this artifact enters, choose a color.",
                "Creatures you control of the chosen color get +1/+0.",
                "{T}: Add one mana of the chosen color.",
            ],
            |d, _| {
                d.choose_color = true;
                d.mana_of_chosen_color = true;
                d.statics.push(Static {
                    subtype: None,
                    lose_abilities: false,
                    base_pt: None,
                    protection: [false; 5],
                    scale: None,
                    scope: Scope::YourCreaturesOfChosenColor,
                    your_turn_only: false,
                    condition: None,
                    power: 1,
                    toughness: 0,
                    keywords: Keywords::NONE, set_colors: None, becomes_land: false,
                });
            },
        ),
        "Strongbox Raider" => s(
            &["Raid — When this creature enters, if you attacked this turn, exile the top two cards of your library. Choose one of them. Until the end of your next turn, you may play that card."],
            |d, _| on_if(d, Condition::Attacked, vec![], vec![Effect::ExileTopChooseOneToPlay { n: 2, this_turn: false }]),
        ),
        "Curator of Destinies" => s(
            &[
                "This spell can't be countered.",
                "When this creature enters, look at the top five cards of your library and separate them into a face-down pile and a face-up pile. An opponent chooses one of those piles. Put that pile into your hand and the other into your graveyard.",
            ],
            |d, _| {
                d.uncounterable = true;
                on(d, When::Enters, vec![], vec![Effect::Piles { n: 5 }]);
            },
        ),
                "Akroma's Memorial" => s(
            &["Creatures you control have flying, first strike, vigilance, trample, haste, and protection from black and from red."],
            |d, _| {
                let k = Keywords::FLYING
                    .with(Keywords::FIRST_STRIKE)
                    .with(Keywords::VIGILANCE)
                    .with(Keywords::TRAMPLE)
                    .with(Keywords::HASTE);
                d.statics.push(Static {
                    subtype: None,
                    lose_abilities: false,
                    base_pt: None,
                    protection: [false, false, true, true, false],
                    scale: None,
                    scope: Scope::YourCreatures,
                    your_turn_only: false,
                    condition: None,
                    power: 0,
                    toughness: 0,
                    keywords: k, set_colors: None, becomes_land: false,
                });
            },
        ),
        "Sphinx's Tutelage" => s(
            &[
                "Whenever you draw a card, target opponent mills two cards. If two nonland cards that share a color were milled this way, repeat this process.",
                "{5}{U}: Draw a card, then discard a card.",
            ],
            |d, _| {
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                on(d, When::YouDraw, vec![opp], vec![Effect::TutelageMill(0)]);
                activated(d, "{5}{U}", false, vec![], vec![draw(1), discard(1)]);
            },
        ),
        "Fishing Pole" => s(
            &[
                "Equipped creature has \"{1}, {T}, Tap Fishing Pole: Put a bait counter on Fishing Pole.\"",
                "Whenever equipped creature becomes untapped, remove a bait counter from this Equipment. If you do, create a 1/1 blue Fish creature token.",
                "Equip {2}",
            ],
            |d, t| {
                let n = activated(d, "{1}", true, vec![], vec![Effect::BaitCounterSelf]);
                d.activated[n].tap_attached = true;
                on(d, When::EquippedBecomesUntapped, vec![], vec![Effect::RemoveBaitForToken { def: t.fish }]);
                equip(d, "{2}");
            },
        ),
        "Tinybones, Bauble Burglar" => s(
            &[
                "Whenever an opponent discards a card, exile it from their graveyard with a stash counter on it.",
                "During your turn, you may play cards you don't own with stash counters on them from exile, and mana of any type can be spent to cast those spells.",
                "{3}{B}, {T}: Each opponent discards a card. Activate only as a sorcery.",
            ],
            |d, _| {
                let that = Filter { triggering: true, ..graveyard_cards(false, Controller::Any) };
                on(d, When::OpponentDiscards, vec![that], vec![Effect::ExileWithStash(0)]);
                d.stash_player = true;
                let n = activated(d, "{3}{B}", true, vec![], vec![Effect::Discard { who: Who::EachOpponent, n: 1 }]);
                d.activated[n].sorcery = true;
            },
        ),
                "Fleeting Flight" => s(
            &["Put a +1/+1 counter on target creature. It gains flying until end of turn. Prevent all combat damage that would be dealt to it this turn."],
            |d, _| {
                spell(
                    d,
                    vec![CREATURE],
                    vec![
                        Effect::Counters { slot: 0, n: 1 },
                        Effect::Grant { slot: 0, keywords: Keywords::FLYING },
                        Effect::PreventCombatDamageTo(0),
                    ],
                )
            },
        ),
        "Tragic Banshee" => s(
            &["Morbid — When this creature enters, target creature an opponent controls gets -1/-1 until end of turn. If a creature died this turn, that creature gets -13/-13 until end of turn instead."],
            |d, _| {
                on(
                    d,
                    When::Enters,
                    vec![OPP_CREATURE],
                    vec![Effect::PumpMorbid { slot: 0, power: -1, toughness: -1, morbid_power: -13, morbid_toughness: -13 }],
                )
            },
        ),
        "Tolarian Terror" => s(
            &["This spell costs {1} less to cast for each instant and sorcery card in your graveyard.", "Ward {2}"],
            |d, _| {
                d.reduction = Some(Reduction::PerInstantSorceryInGraveyard);
                d.ward = ManaCost::parse("{2}");
            },
        ),
        "Dwynen, Gilt-Leaf Daen" => s(
            &[
                "Other Elf creatures you control get +1/+1.",
                "Whenever Dwynen attacks, you gain 1 life for each attacking Elf you control.",
            ],
            |d, _| {
                lord(d, "Elf");
                on(d, When::Attacks, vec![], vec![Effect::GainLifePerAttacking("Elf")]);
            },
        ),
        "Krenko, Mob Boss" => s(
            &["{T}: Create X 1/1 red Goblin creature tokens, where X is the number of Goblins you control."],
            |d, t| {
                activated(d, "", true, vec![], vec![Effect::CreateTokensPerYouControl { def: t.goblin, sub: "Goblin" }]);
            },
        ),
        "Revenge of the Rats" => s(
            &["Create a tapped 1/1 black Rat creature token for each creature card in your graveyard.", "Flashback {2}{B}{B}"],
            |d, t| {
                d.flashback = ManaCost::parse("{2}{B}{B}");
                spell(d, vec![], vec![Effect::CreateTokensPerGraveyardCreature { def: t.rat, tapped: true }])
            },
        ),
        "Elvish Archdruid" => s(
            &["Other Elf creatures you control get +1/+1.", "{T}: Add {G} for each Elf you control."],
            |d, _| {
                lord(d, "Elf");
                d.mana = Some(ManaSource::of(&[Color::Green]));
                d.mana_per_subtype = Some("Elf");
            },
        ),
        "Lunar Insight" => s(
            &["Draw a card for each different mana value among nonland permanents you control."],
            |d, _| spell(d, vec![], vec![Effect::DrawPerDistinctManaValue]),
        ),
        "Inspiring Call" => s(
            &["Draw a card for each creature you control with a +1/+1 counter on it. Those creatures gain indestructible until end of turn."],
            |d, _| spell(d, vec![], vec![Effect::InspiringCall]),
        ),
        "Midnight Snack" => s(
            &[
                "Raid — At the beginning of your end step, if you attacked this turn, create a Food token.",
                "{2}{B}, Sacrifice this enchantment: Target opponent loses X life, where X is the amount of life you gained this turn.",
            ],
            |d, t| {
                conditional(
                    d,
                    When::EndStep { your_turn_only: true },
                    Condition::Attacked,
                    vec![],
                    vec![Effect::CreateToken { who: Who::You, def: t.food, n: 1 }],
                );
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                let n = activated(d, "{2}{B}", false, vec![opp], vec![Effect::LoseLifeEqualGainedThisTurn(0)]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Crystal Barricade" => s(
            &["You have hexproof.", "Prevent all noncombat damage that would be dealt to other creatures you control."],
            |d, _| {
                d.player_hexproof = true;
                d.prevent_noncombat_to_others = true;
            },
        ),
        "Blanchwood Armor" => s(&["Enchant creature", "Enchanted creature gets +1/+1 for each Forest you control."], |d, _| {
            d.aura = true;
            d.spell = Some(Ability::new(vec![CREATURE], vec![]));
            d.statics.push(Static {
                subtype: None,
                lose_abilities: false,
                base_pt: None,
                protection: [false; 5],
                scale: Some(Scale::PerYouControlSubtype("Forest")),
                scope: Scope::Attached,
                your_turn_only: false,
                condition: None,
                power: 1,
                toughness: 1,
                keywords: Keywords::NONE, set_colors: None, becomes_land: false,
            });
        }),
        "Brass's Bounty" => s(&["For each land you control, create a Treasure token."], |d, t| {
            spell(d, vec![], vec![Effect::CreateTokensPerLand { def: t.treasure }])
        }),
        "Temporal Manipulation" => s(&["Take an extra turn after this one."], |d, _| {
            spell(d, vec![], vec![Effect::ExtraTurn])
        }),
        "Ghalta, Primal Hunger" => s(
            &["This spell costs {X} less to cast, where X is the total power of creatures you control."],
            |d, _| d.reduction = Some(Reduction::TotalPowerOfYourCreatures),
        ),
        "Ashroot Animist" => s(
            &["Whenever this creature attacks, another target creature you control gains trample and gets +X/+X until end of turn, where X is this creature's power."],
            |d, _| {
                let another = Filter { other: true, ..YOUR_CREATURE };
                on(
                    d,
                    When::Attacks,
                    vec![another],
                    vec![Effect::Grant { slot: 0, keywords: Keywords::TRAMPLE }, Effect::PumpBySourcePower(0)],
                )
            },
        ),
        "Embercleave" => s(
            &[
                "This spell costs {1} less to cast for each attacking creature you control.",
                "When Embercleave enters, attach it to target creature you control.",
                "Equipped creature gets +1/+1 and has double strike and trample.",
                "Equip {3}",
            ],
            |d, _| {
                d.reduction = Some(Reduction::PerAttackingCreature);
                on(d, When::Enters, vec![YOUR_CREATURE], vec![Effect::Attach { slot: 0 }]);
                attached(d, false, 1, 1, Keywords::DOUBLE_STRIKE.with(Keywords::TRAMPLE));
                equip(d, "{3}");
            },
        ),
                "Wildwood Scourge" => s(
            &[
                "This creature enters with X +1/+1 counters on it.",
                "Whenever one or more +1/+1 counters are put on another non-Hydra creature you control, put a +1/+1 counter on this creature.",
            ],
            |d, _| {
                d.enters_with_x_counters = true;
                on(d, When::CountersOnAnotherNonHydraYouControl, vec![], vec![Effect::CountersSelf { n: 1 }]);
            },
        ),
        "Goblin Negotiation" => s(
            &["Goblin Negotiation deals X damage to target creature. Create a number of 1/1 red Goblin creature tokens equal to the amount of excess damage dealt to that creature this way."],
            |d, t| spell(d, vec![CREATURE], vec![Effect::DamageXExcessTokens { slot: 0, def: t.goblin }]),
        ),
        "Exsanguinate" => s(
            &["Each opponent loses X life. You gain life equal to the life lost this way."],
            |d, _| spell(d, vec![], vec![Effect::DrainX]),
        ),
        "Genesis Wave" => s(
            &["Reveal the top X cards of your library. You may put any number of permanent cards with mana value X or less from among them onto the battlefield. Then put all cards revealed this way that weren't put onto the battlefield into your graveyard."],
            |d, _| spell(d, vec![], vec![Effect::GenesisWave]),
        ),
        "Fiend Artisan" => s(
            &[
                "This creature gets +1/+1 for each creature card in your graveyard.",
                "{X}{B/G}, {T}, Sacrifice another creature: Search your library for a creature card with mana value X or less, put it onto the battlefield, then shuffle. Activate only as a sorcery.",
            ],
            |d, _| {
                d.pt_per_graveyard_creature = true;
                let to = SearchTo::Battlefield { tapped: false };
                let n = activated(
                    d,
                    "{X}{B/G}",
                    true,
                    vec![],
                    vec![Effect::Search { what: SearchFor::CreatureMvAtMostX, to, n: 1, kicked_n: None }],
                );
                d.activated[n].sacrifice_other_creature = true;
                d.activated[n].sorcery = true;
            },
        ),
                "Self-Reflection" => s(
            &["Create a token that's a copy of target creature you control.", "Flashback {3}{U}"],
            |d, _| {
                d.flashback = ManaCost::parse("{3}{U}");
                spell(
                    d,
                    vec![YOUR_CREATURE],
                    vec![Effect::CopyToken { slot: Some(0), haste_and_sacrifice: false, extra_subtype: None }],
                )
            },
        ),
        "Electroduplicate" => s(
            &[
                "Create a token that's a copy of target creature you control, except it has haste and \"At the beginning of the end step, sacrifice this token.\"",
                "Flashback {2}{R}{R}",
            ],
            |d, _| {
                d.flashback = ManaCost::parse("{2}{R}{R}");
                spell(
                    d,
                    vec![YOUR_CREATURE],
                    vec![Effect::CopyToken { slot: Some(0), haste_and_sacrifice: true, extra_subtype: None }],
                )
            },
        ),
        "Homunculus Horde" => s(
            &["Whenever you draw your second card each turn, create a token that's a copy of this creature."],
            |d, _| {
                on(
                    d,
                    When::YouDrawSecond,
                    vec![],
                    vec![Effect::CopyToken { slot: None, haste_and_sacrifice: false, extra_subtype: None }],
                )
            },
        ),
        "Extravagant Replication" => s(
            &["At the beginning of your upkeep, create a token that's a copy of another target nonland permanent you control."],
            |d, _| {
                let f = Filter {
                    players: false,
                    creatures: false,
                    planeswalkers: false,
                    nonland_permanents: true,
                    other: true,
                    controller: Controller::You,
                    ..Filter::ANY
                };
                on(
                    d,
                    When::Upkeep { your_turn_only: true },
                    vec![f],
                    vec![Effect::CopyToken { slot: Some(0), haste_and_sacrifice: false, extra_subtype: None }],
                )
            },
        ),
        "Abyssal Harvester" => s(
            &["{T}: Exile target creature card from a graveyard that was put there this turn. Create a token that's a copy of it, except it's a Nightmare in addition to its other types. Then exile all other Nightmare tokens you control."],
            |d, _| {
                let f = Filter { put_there_this_turn: true, ..graveyard_cards(true, Controller::Any) };
                activated(
                    d,
                    "",
                    true,
                    vec![f],
                    vec![
                        Effect::Exile(0),
                        Effect::CopyToken { slot: Some(0), haste_and_sacrifice: false, extra_subtype: Some("Nightmare") },
                        Effect::ExileOtherTokensWithSubtype("Nightmare"),
                    ],
                );
            },
        ),
                "Liliana, Dreadhorde General" => s(
            &[
                "Whenever a creature you control dies, draw a card.",
                "+1: Create a 2/2 black Zombie creature token.",
                "−4: Each player sacrifices two creatures of their choice.",
                "−9: Each opponent chooses a permanent they control of each permanent type and sacrifices the rest.",
            ],
            |d, t| {
                on(d, When::CreatureYouControlDies, vec![], vec![draw(1)]);
                loyalty(d, 1, Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.zombie, n: 1 }]));
                loyalty(
                    d,
                    -4,
                    Ability::new(
                        vec![],
                        vec![
                            Effect::SacrificeCreatures { who: Who::You, n: 2 },
                            Effect::SacrificeCreatures { who: Who::EachOpponent, n: 2 },
                        ],
                    ),
                );
                loyalty(d, -9, Ability::new(vec![], vec![Effect::KeepOneOfEachType]));
            },
        ),
        "Kaito, Cunning Infiltrator" => s(
            &[
                "Whenever a creature you control deals combat damage to a player, put a loyalty counter on Kaito.",
                "+1: Up to one target creature you control can't be blocked this turn. Draw a card, then discard a card.",
                "−2: Create a 2/1 blue Ninja creature token.",
                "−9: You get an emblem with \"Whenever a player casts a spell, you create a 2/1 blue Ninja creature token.\"",
            ],
            |d, t| {
                on(d, When::YourCreatureDealsCombatDamageToPlayer, vec![], vec![Effect::LoyaltySelf(1)]);
                // Trigger 1 belongs to the emblem.
                on(d, When::Never, vec![], vec![Effect::CreateToken { who: Who::You, def: t.ninja, n: 1 }]);
                loyalty(
                    d,
                    1,
                    up_to(vec![YOUR_CREATURE], vec![Effect::Grant { slot: 0, keywords: Keywords::UNBLOCKABLE }, draw(1), discard(1)]),
                );
                loyalty(d, -2, Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.ninja, n: 1 }]));
                loyalty(d, -9, Ability::new(vec![], vec![Effect::Emblem(EmblemKind::OnAnySpellCast { trigger: 1 })]));
            },
        ),
        "Ajani, Caller of the Pride" => s(
            &[
                "+1: Put a +1/+1 counter on up to one target creature.",
                "−3: Target creature gains flying and double strike until end of turn.",
                "−8: Create X 2/2 white Cat creature tokens, where X is your life total.",
            ],
            |d, t| {
                loyalty(d, 1, up_to(vec![CREATURE], vec![Effect::Counters { slot: 0, n: 1 }]));
                loyalty(
                    d,
                    -3,
                    Ability::new(vec![CREATURE], vec![Effect::Grant { slot: 0, keywords: Keywords::FLYING.with(Keywords::DOUBLE_STRIKE) }]),
                );
                loyalty(d, -8, Ability::new(vec![], vec![Effect::CreateTokensPerLife { def: t.cat_2 }]));
            },
        ),
        "Chandra, Flameshaper" => s(
            &[
                "+2: Add {R}{R}{R}. Exile the top three cards of your library. Choose one. You may play that card this turn.",
                "+1: Create a token that's a copy of target creature you control, except it has haste and \"At the beginning of the end step, sacrifice this token.\"",
                "−4: Chandra deals 8 damage divided as you choose among any number of target creatures and/or planeswalkers.",
            ],
            |d, _| {
                let mana = Effect::AddMana { color: Color::Red, n: 3 };
                let exile = Effect::ExileTopChooseOneToPlay { n: 3, this_turn: true };
                loyalty(d, 2, Ability::new(vec![], vec![mana, exile]));
                let copy = Effect::CopyToken { slot: Some(0), haste_and_sacrifice: true, extra_subtype: None };
                loyalty(d, 1, Ability::new(vec![YOUR_CREATURE], vec![copy]));
                // "Any number": up to eight targets, each getting at least 1 of the 8 (see
                // docs/engine/QUESTIONS.md).
                let creature_or_walker = Filter { players: false, ..Filter::ANY };
                let mut divided = up_to(vec![creature_or_walker; 8], vec![Effect::DividedDamage]);
                divided.divide = 8;
                loyalty(d, -4, divided);
            },
        ),
        "Doubling Season" => s(
            &[
                "If an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.",
                "If an effect would put one or more counters on a permanent you control, it puts twice that many of those counters on that permanent instead.",
            ],
            |d, _| d.doubles_tokens_and_counters = true,
        ),
        "Twinflame Tyrant" => s(
            &["If a source you control would deal damage to an opponent or a permanent an opponent controls, it deals double that damage instead."],
            |d, _| d.doubles_damage_to_opponents = true,
        ),
        "Omniscience" => s(
            &["You may cast spells from your hand without paying their mana costs."],
            |d, _| d.free_from_hand = true,
        ),
                "Progenitus" => s(
            &[
                "Protection from everything",
                "If Progenitus would be put into a graveyard from anywhere, reveal Progenitus and shuffle it into its owner's library instead.",
            ],
            |d, _| {
                d.protection_from_everything = true;
                d.shuffle_instead_of_graveyard = true;
            },
        ),
        "Zul Ashur, Lich Lord" => s(
            &[
                "Ward—Pay 2 life.",
                "{T}: You may cast target Zombie creature card from your graveyard this turn.",
            ],
            |d, _| {
                d.ward_life = Some(2);
                let zombie = Filter { subtype: Some("Zombie"), ..graveyard_cards(true, Controller::You) };
                d.activated.push(Activated {
                    tap: true,
                    ability: Ability::new(vec![zombie], vec![Effect::PlayableThisTurn(0)]),
                    ..Activated::default()
                });
            },
        ),
        "Involuntary Employment" => s(
            &["Gain control of target creature until end of turn. Untap that creature. It gains haste until end of turn. Create a Treasure token."],
            |d, t| {
                spell(
                    d,
                    vec![CREATURE],
                    vec![
                        Effect::GainControlUntilEot(0),
                        Effect::UntapTarget(0),
                        Effect::Grant { slot: 0, keywords: Keywords::HASTE },
                        Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 },
                    ],
                )
            },
        ),
        "Fiery Annihilation" => s(
            &["Fiery Annihilation deals 5 damage to target creature. Exile up to one target Equipment attached to that creature. If that creature would die this turn, exile it instead."],
            |d, _| {
                let equipment = Filter {
                    creatures: false,
                    artifacts: true,
                    subtype: Some("Equipment"),
                    attached_to_first_target: true,
                    ..Filter::ANY
                };
                let mut a = Ability::new(
                    vec![CREATURE, Filter { players: false, planeswalkers: false, ..equipment }],
                    vec![
                        Effect::Damage { amount: 5, kicked: None, to: Recipient::Target(0) },
                        Effect::Exile(1),
                        Effect::ExileIfDiesThisTurn(0),
                    ],
                );
                a.min_targets = Some(1);
                d.spell = Some(a);
            },
        ),
        "Bloom Tender" => s(
            &["Vivid — {T}: For each color among permanents you control, add one mana of that color."],
            |d, _| d.vivid_mana = true,
        ),
        "Giada, Font of Hope" => s(
            &[
                "Each other Angel you control enters with an additional +1/+1 counter on it for each Angel you already control.",
                "{T}: Add {W}. Spend this mana only to cast an Angel spell.",
            ],
            |d, _| {
                d.other_subtype_enter_bonus = Some("Angel");
                d.mana = Some(ManaSource::of(&[Color::White]));
                d.mana_only_for_subtype = Some("Angel");
            },
        ),
        "Banner of Kinship" => s(
            &[
                "As this artifact enters, choose a creature type. This artifact enters with a fellowship counter on it for each creature you control of the chosen type.",
                "Creatures you control of the chosen type get +1/+1 for each fellowship counter on this artifact.",
            ],
            |d, _| {
                d.choose_creature_type = true;
                d.statics.push(Static {
                    subtype: None,
                    lose_abilities: false,
                    base_pt: None,
                    protection: [false; 5],
                    scale: Some(Scale::PerFellowshipCounter),
                    scope: Scope::YourCreaturesOfChosenType,
                    your_turn_only: false,
                    condition: None,
                    power: 1,
                    toughness: 1,
                    keywords: Keywords::NONE, set_colors: None, becomes_land: false,
                });
            },
        ),
        "Quilled Greatwurm" => s(
            &[
                "Whenever a creature you control deals combat damage during your turn, put that many +1/+1 counters on it.",
                "You may cast this card from your graveyard by removing six counters from among creatures you control in addition to paying its other costs.",
            ],
            |d, _| {
                let it = Filter { triggering: true, ..YOUR_CREATURE };
                on(d, When::YourCreatureDealsCombatDamageOnYourTurn, vec![it], vec![Effect::CountersAmount(0)]);
                d.graveyard_cast_remove_counters = 6;
            },
        ),
                "Thousand-Year Storm" => s(
            &["Whenever you cast an instant or sorcery spell, copy it for each other instant and sorcery spell you've cast before it this turn. You may choose new targets for the copies."],
            // The copies keep the original's targets (see docs/engine/QUESTIONS.md).
            |d, _| on(d, When::YouCastInstantOrSorceryCounted, vec![], vec![Effect::CopySpellPerAmount]),
        ),
                "Etali, Primal Storm" => s(
            &["Whenever Etali attacks, exile the top card of each player's library, then you may cast any number of spells from among those cards without paying their mana costs."],
            |d, _| on(d, When::Attacks, vec![], vec![Effect::ExileTopOfEachCastFree]),
        ),
        "Imprisoned in the Moon" => s(
            &[
                "Enchant creature, land, or planeswalker",
                "Enchanted permanent is a colorless land with \"{T}: Add {C}\" and loses all other card types and abilities.",
            ],
            |d, _| {
                d.aura = true;
                let f = Filter { players: false, lands: true, ..Filter::ANY };
                d.spell = Some(Ability::new(vec![f], vec![]));
                d.statics.push(Static {
                    subtype: None,
                    lose_abilities: true,
                    base_pt: None,
                    protection: [false; 5],
                    scale: None,
                    scope: Scope::Attached,
                    your_turn_only: false,
                    condition: None,
                    power: 0,
                    toughness: 0,
                    keywords: Keywords::NONE,
                    set_colors: Some([false; 5]),
                    becomes_land: true,
                });
            },
        ),
        "Soulstone Sanctuary" => s(
            &[
                "{T}: Add {C}.",
                "{4}: This land becomes a 3/3 creature with vigilance and all creature types. It's still a land.",
            ],
            |d, _| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                d.activated.push(Activated {
                    mana: ManaCost::parse("{4}").expect("cost"),
                    ability: Ability::new(vec![], vec![Effect::BecomeCreatureSelf { power: 3, toughness: 3 }]),
                    ..Activated::default()
                });
            },
        ),
        "Kellan, Planar Trailblazer" => s(
            &[
                "{1}{R}: If Kellan is a Scout, it becomes a Human Faerie Detective and gains \"Whenever Kellan deals combat damage to a player, exile the top card of your library. You may play that card this turn.\"",
                "{2}{R}: If Kellan is a Detective, it becomes a 3/2 Human Faerie Rogue and gains double strike.",
            ],
            |d, _| {
                d.stages = vec![
                    Stage { subtypes: &["Human", "Faerie", "Detective"], base_pt: None, keywords: Keywords::NONE },
                    Stage { subtypes: &["Human", "Faerie", "Rogue"], base_pt: Some((3, 2)), keywords: Keywords::DOUBLE_STRIKE },
                ];
                for (cost, from, to) in [("{1}{R}", "Scout", 1), ("{2}{R}", "Detective", 2)] {
                    d.activated.push(Activated {
                        mana: ManaCost::parse(cost).expect("cost"),
                        ability: Ability::new(vec![], vec![Effect::AdvanceStage { if_subtype: from, to }]),
                        ..Activated::default()
                    });
                }
                // The gained trigger, live from stage 1 on (it stays once Kellan is a Rogue).
                let mut gained = Ability::new(vec![], vec![Effect::ExileTopChooseOneToPlay { n: 1, this_turn: true }]);
                gained.condition = Some(Condition::StageAtLeast(1));
                d.triggers.push(Trigger { when: When::DealsCombatDamageToPlayer, ability: gained });
            },
        ),
        "Secluded Courtyard" => s(
            &[
                "As this land enters, choose a creature type.",
                "{T}: Add {C}.",
                "{T}: Add one mana of any color. Spend this mana only to cast a creature spell of the chosen type or activate an ability of a creature source of the chosen type.",
            ],
            |d, _| {
                d.choose_creature_type = true;
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                d.any_color_for_chosen_type = true;
            },
        ),
                "Muldrotha, the Gravetide" => s(
            &["During each of your turns, you may play a land and cast a permanent spell of each permanent type from your graveyard."],
            |d, _| d.graveyard_permanents = true,
        ),
        "Vivien Reid" => s(
            &[
                "+1: Look at the top four cards of your library. You may reveal a creature or land card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
                "−3: Destroy target artifact, enchantment, or creature with flying.",
                "−8: You get an emblem with \"Creatures you control get +2/+2 and have vigilance, trample, and indestructible.\"",
            ],
            |d, _| {
                let look = Effect::LookAtTop { n: 4, pick: Pick::CreatureOrLand, chosen_to: LookTo::Hand, rest_to: LookRest::BottomRandom };
                loyalty(d, 1, Ability::new(vec![], vec![look]));
                let f = Filter { creature_needs_flying: true, ..ART_ENCH };
                loyalty(d, -3, Ability::new(vec![f], vec![Effect::Destroy(0)]));
                loyalty(d, -8, Ability::new(vec![], vec![Effect::Emblem(EmblemKind::VivienAnthem)]));
            },
        ),
        // ---- Tarkir: Dragonstorm (TDM), batch 1: the most-picked commons on existing rules ----
        "Sibsig Appraiser" => s(
            &["When this creature enters, look at the top two cards of your library. Put one of them into your hand and the other into your graveyard."],
            |d, _| {
                let look = Effect::LookAtTop { n: 2, pick: Pick::Any, chosen_to: LookTo::Hand, rest_to: LookRest::GraveyardTakeOne };
                on(d, When::Enters, vec![], vec![look]);
            },
        ),
        "Iceridge Serpent" => s(
            &["When this creature enters, return target creature an opponent controls to its owner's hand."],
            |d, _| on(d, When::Enters, vec![OPP_CREATURE], vec![Effect::Bounce(0)]),
        ),
        "Boulderborn Dragon" => s(&["Whenever this creature attacks, surveil 1."], |d, _| {
            on(d, When::Attacks, vec![], vec![Effect::Surveil { n: 1 }])
        }),
        "Humbling Elder" => s(
            &["When this creature enters, target creature an opponent controls gets -2/-0 until end of turn."],
            |d, _| on(d, When::Enters, vec![OPP_CREATURE], vec![Effect::Pump { slot: 0, power: -2, toughness: 0 }]),
        ),
        "Rebellious Strike" => s(&["Target creature gets +3/+0 until end of turn.", "Draw a card."], |d, _| {
            spell(d, vec![CREATURE], vec![Effect::Pump { slot: 0, power: 3, toughness: 0 }, draw(1)])
        }),
        "Sarkhan's Resolve" => s(
            &["Choose one —", "• Target creature gets +3/+3 until end of turn.", "• Destroy target creature with flying."],
            |d, _| {
                let flier = Filter { creature_needs_flying: true, ..CREATURE };
                modes(
                    d,
                    vec![
                        Ability::new(vec![CREATURE], vec![Effect::Pump { slot: 0, power: 3, toughness: 3 }]),
                        Ability::new(vec![flier], vec![Effect::Destroy(0)]),
                    ],
                )
            },
        ),
        "Underfoot Underdogs" => s(
            &[
                "When this creature enters, create a 1/1 red Goblin creature token.",
                "{1}, {T}: Target creature you control with power 2 or less can't be blocked this turn.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.goblin, n: 1 }]);
                let small = Filter { creature_max_power: Some(2), ..YOUR_CREATURE };
                activated(d, "{1}", true, vec![small], vec![Effect::Grant { slot: 0, keywords: Keywords::UNBLOCKABLE }]);
            },
        ),
        "Watcher of the Wayside" => s(
            &["When this creature enters, target player mills two cards. You gain 2 life."],
            |d, _| {
                on(
                    d,
                    When::Enters,
                    vec![Filter::PLAYER],
                    vec![Effect::Mill { who: Who::TargetPlayer(0), n: 2 }, Effect::GainLife { who: Who::You, n: 2 }],
                )
            },
        ),
        "Meticulous Artisan" => s(&["Prowess", "When this creature enters, create a Treasure token."], |d, t| {
            on(d, When::YouCastNoncreature, vec![], vec![Effect::PumpSelf { power: 1, toughness: 1 }]);
            on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }]);
        }),
        "Stormplain Detainment" => s(
            &["When this enchantment enters, exile target nonland permanent an opponent controls until this enchantment leaves the battlefield."],
            |d, _| on(d, When::Enters, vec![OPP_NONLAND], vec![Effect::ExileUntilSourceLeaves(0)]),
        ),
        // ---- TDM batch 2: behold (the Exhale spells) and the next most-picked commons ----
        "Caustic Exhale" => s(
            &["As an additional cost to cast this spell, behold a Dragon or pay {1}.", "Target creature gets -3/-3 until end of turn."],
            |d, _| {
                d.behold = Behold::OrPay(ManaCost { generic: 1, ..ManaCost::default() });
                spell(d, vec![CREATURE], vec![Effect::Pump { slot: 0, power: -3, toughness: -3 }]);
            },
        ),
        "Dispelling Exhale" => s(
            &[
                "As an additional cost to cast this spell, you may behold a Dragon.",
                "Counter target spell unless its controller pays {2}. If a Dragon was beheld, counter that spell unless its controller pays {4} instead.",
            ],
            |d, _| {
                d.behold = Behold::May;
                spell(d, vec![SPELL], vec![Effect::CounterUnlessPays(0, 2, 4)]);
            },
        ),
        "Molten Exhale" => s(
            &[
                "You may cast this spell as though it had flash if you behold a Dragon as an additional cost to cast it.",
                "Molten Exhale deals 4 damage to target creature or planeswalker.",
            ],
            |d, _| {
                d.behold = Behold::MayForFlash;
                spell(d, vec![CREATURE_OR_PLANESWALKER], vec![damage(4, Recipient::Target(0))]);
            },
        ),
        "Osseous Exhale" => s(
            &[
                "As an additional cost to cast this spell, you may behold a Dragon.",
                "Osseous Exhale deals 5 damage to target attacking or blocking creature. If a Dragon was beheld, you gain 2 life.",
            ],
            |d, _| {
                d.behold = Behold::May;
                let fighting = Filter { in_combat: true, ..CREATURE };
                spell(d, vec![fighting], vec![damage(5, Recipient::Target(0))]);
                d.kicked_spell = Some(Ability::new(
                    vec![fighting],
                    vec![damage(5, Recipient::Target(0)), Effect::GainLife { who: Who::You, n: 2 }],
                ));
            },
        ),
        "Piercing Exhale" => s(
            &[
                "As an additional cost to cast this spell, you may behold a Dragon.",
                "Target creature you control deals damage equal to its power to target creature or planeswalker. If a Dragon was beheld, surveil 2.",
            ],
            |d, _| {
                d.behold = Behold::May;
                let targets = vec![YOUR_CREATURE, CREATURE_OR_PLANESWALKER];
                spell(d, targets.clone(), vec![Effect::Bite { from: 0, to: 1 }]);
                d.kicked_spell = Some(Ability::new(targets, vec![Effect::Bite { from: 0, to: 1 }, Effect::Surveil { n: 2 }]));
            },
        ),
        "Riverwalk Technique" => s(
            &[
                "Choose one —",
                "• The owner of target nonland permanent puts it on their choice of the top or bottom of their library.",
                "• Counter target noncreature spell.",
            ],
            |d, _| {
                let nonland = Filter { controller: Controller::Any, ..OPP_NONLAND };
                modes(
                    d,
                    vec![
                        Ability::new(vec![nonland], vec![Effect::Tuck(0)]),
                        Ability::new(vec![Filter { spell_creature: Some(false), ..SPELL }], vec![Effect::Counter(0)]),
                    ],
                )
            },
        ),
        "Ainok Wayfarer" => s(
            &["When this creature enters, mill three cards. You may put a land card from among them into your hand. If you don't, put a +1/+1 counter on this creature."],
            |d, _| {
                let look = Effect::LookAtTop { n: 3, pick: Pick::Land, chosen_to: LookTo::Hand, rest_to: LookRest::GraveyardElseCounter };
                on(d, When::Enters, vec![], vec![look]);
            },
        ),
        "Dragonstorm Globe" => s(
            &["Each Dragon you control enters with an additional +1/+1 counter on it.", "{T}: Add one mana of any color."],
            |d, _| {
                d.subtype_enters_with_counter = Some("Dragon");
                d.mana = Some(ManaSource::of(&Color::ALL));
            },
        ),
        "Undergrowth Leopard" => s(&["{1}, Sacrifice this creature: Destroy target artifact or enchantment."], |d, _| {
            let n = activated(d, "{1}", false, vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::Destroy(0)]);
            d.activated[n].sacrifice_self = true;
        }),
        "Narset's Rebuke" => s(
            &["Narset's Rebuke deals 5 damage to target creature. Add {U}{R}{W}. If that creature would die this turn, exile it instead."],
            |d, _| {
                spell(
                    d,
                    vec![CREATURE],
                    vec![
                        Effect::ExileIfDiesThisTurn(0),
                        damage(5, Recipient::Target(0)),
                        Effect::AddMana { color: Color::Blue, n: 1 },
                        Effect::AddMana { color: Color::Red, n: 1 },
                        Effect::AddMana { color: Color::White, n: 1 },
                    ],
                )
            },
        ),
        // ---- TDM batch 3: {2/C} hybrid costs (the cost parser handles them) and their neighbors ----
        "Defibrillating Current" => s(
            &["Defibrillating Current deals 4 damage to target creature or planeswalker and you gain 2 life."],
            |d, _| {
                spell(
                    d,
                    vec![CREATURE_OR_PLANESWALKER],
                    vec![damage(4, Recipient::Target(0)), Effect::GainLife { who: Who::You, n: 2 }],
                )
            },
        ),
        "Dragonclaw Strike" => s(
            &["Double the power and toughness of target creature you control until end of turn. Then it fights up to one target creature an opponent controls."],
            |d, _| {
                let mut a = Ability::new(
                    vec![YOUR_CREATURE, OPP_CREATURE],
                    vec![Effect::DoublePowerToughness(0), Effect::FightBetween { a: 0, b: 1 }],
                );
                a.min_targets = Some(1); // the second target is "up to one"
                d.spell = Some(a);
            },
        ),
        "Gurmag Nightwatch" => s(
            &["When this creature enters, look at the top three cards of your library. You may put one of those cards back on top of your library. Put the rest into your graveyard."],
            |d, _| {
                let look = Effect::LookAtTop { n: 3, pick: Pick::Any, chosen_to: LookTo::Top, rest_to: LookRest::Graveyard };
                on(d, When::Enters, vec![], vec![look]);
            },
        ),
        "Kin-Tree Severance" => s(&["Exile target permanent with mana value 3 or greater."], |d, _| {
            let big = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, min_mana_value: Some(3), ..Filter::ANY };
            spell(d, vec![big], vec![Effect::Exile(0)])
        }),
        "Monastery Messenger" => s(
            &["When this creature enters, put up to one target noncreature, nonland card from your graveyard on top of your library."],
            |d, _| {
                let card = Filter {
                    players: false,
                    creatures: false,
                    planeswalkers: false,
                    graveyard: Some(false),
                    noncreature_nonland: true,
                    controller: Controller::You,
                    ..Filter::ANY
                };
                d.triggers.push(Trigger { when: When::Enters, ability: up_to(vec![card], vec![Effect::ToLibrary(0, true)]) });
            },
        ),
        "Rakshasa's Bargain" => s(
            &["Look at the top four cards of your library. Put two of them into your hand and the rest into your graveyard."],
            |d, _| {
                let look = Effect::LookAtTop { n: 4, pick: Pick::Any, chosen_to: LookTo::Hand, rest_to: LookRest::GraveyardTakeTwo };
                spell(d, vec![], vec![look]);
            },
        ),
        "Reputable Merchant" => s(
            &["When this creature enters or dies, put a +1/+1 counter on target creature you control."],
            |d, _| {
                on(d, When::Enters, vec![YOUR_CREATURE], vec![Effect::Counters { slot: 0, n: 1 }]);
                on(d, When::Dies, vec![YOUR_CREATURE], vec![Effect::Counters { slot: 0, n: 1 }]);
            },
        ),
        "Riverwheel Sweep" => s(
            &[
                "Tap target creature. Put three stun counters on it.",
                "Exile the top two cards of your library. Choose one of them. Until the end of your next turn, you may play that card.",
            ],
            |d, _| {
                // TapAndStun taps and adds one stun counter; three of them make three counters.
                let stun = Effect::TapAndStun(0);
                spell(
                    d,
                    vec![CREATURE],
                    vec![stun, stun, stun, Effect::ExileTopChooseOneToPlay { n: 2, this_turn: false }],
                )
            },
        ),
        "Temur Tawnyback" => s(&["When this creature enters, draw a card, then discard a card."], |d, _| {
            on(d, When::Enters, vec![], vec![draw(1), discard(1)])
        }),
        "Jade-Cast Sentinel" => s(
            &["{2}, {T}: Put target card from a graveyard on the bottom of its owner's library."],
            |d, _| {
                let card = Filter { players: false, creatures: false, planeswalkers: false, graveyard: Some(false), ..Filter::ANY };
                activated(d, "{2}", true, vec![card], vec![Effect::ToLibrary(0, false)]);
            },
        ),
        // ---- TDM batch 4: mobilize ----
        "Dalkovan Packbeasts" => s(&["Mobilize 3"], |d, t| mobilize(d, t, Some(3))),
        "Dragonback Lancer" => s(&["Mobilize 1"], |d, t| mobilize(d, t, Some(1))),
        "Shock Brigade" => s(&["Mobilize 1"], |d, t| mobilize(d, t, Some(1))),
        "Nightblade Brigade" => s(&["Mobilize 1", "When this creature enters, surveil 1."], |d, t| {
            mobilize(d, t, Some(1));
            on(d, When::Enters, vec![], vec![Effect::Surveil { n: 1 }]);
        }),
        "Reigning Victor" => s(
            &["Mobilize 1", "When this creature enters, target creature gets +1/+0 and gains indestructible until end of turn."],
            |d, t| {
                mobilize(d, t, Some(1));
                on(
                    d,
                    When::Enters,
                    vec![CREATURE],
                    vec![Effect::Pump { slot: 0, power: 1, toughness: 0 }, Effect::Grant { slot: 0, keywords: Keywords::INDESTRUCTIBLE }],
                );
            },
        ),
        "Avenger of the Fallen" => s(&["Mobilize X, where X is the number of creature cards in your graveyard."], |d, t| {
            mobilize(d, t, None)
        }),
        "Venerated Stormsinger" => s(
            &["Mobilize 1", "Whenever this creature or another creature you control dies, each opponent loses 1 life and you gain 1 life."],
            |d, t| {
                mobilize(d, t, Some(1));
                on(
                    d,
                    When::CreatureYouControlDies,
                    vec![],
                    vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }, Effect::GainLife { who: Who::You, n: 1 }],
                );
            },
        ),
        "Bone-Cairn Butcher" => s(&["Mobilize 2", "Attacking tokens you control have deathtouch."], |d, t| {
            mobilize(d, t, Some(2));
            static_on(d, Scope::YourAttackingTokens, false, Keywords::DEATHTOUCH);
        }),
        "Zurgo's Vanguard" => s(
            &["Mobilize 1", "Zurgo's Vanguard's power is equal to the number of creatures you control."],
            |d, t| {
                mobilize(d, t, Some(1));
                static_on(d, Scope::This, false, Keywords::NONE);
                let st = d.statics.last_mut().expect("just added");
                st.power = 1;
                st.scale = Some(Scale::PerCreatureYouControl);
            },
        ),
        // ---- TDM batch 5: the Devotees' mana, costs less (and more), scry ----
        "Abzan Devotee" => s(
            &["{1}: Add {W}, {B}, or {G}. Activate only once each turn.", "{2}{B}: Return this card from your graveyard to your hand."],
            |d, _| {
                devotee(d, &[Color::White, Color::Black, Color::Green]);
                let n = activated(d, "{2}{B}", false, vec![], vec![Effect::ReturnSelfToHand]);
                d.activated[n].from_graveyard = true;
            },
        ),
        "Mardu Devotee" => s(
            &["When this creature enters, scry 2.", "{1}: Add {R}, {W}, or {B}. Activate only once each turn."],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::Scry { n: 2 }]);
                devotee(d, &[Color::White, Color::Black, Color::Red]);
            },
        ),
        "Sultai Devotee" => s(&["{1}: Add {B}, {G}, or {U}. Activate only once each turn."], |d, _| {
            devotee(d, &[Color::Blue, Color::Black, Color::Green])
        }),
        "Temur Devotee" => s(&["{1}: Add {G}, {U}, or {R}. Activate only once each turn."], |d, _| {
            devotee(d, &[Color::Blue, Color::Red, Color::Green])
        }),
        "Focus the Mind" => s(
            &["This spell costs {2} less to cast if you've cast another spell this turn.", "Draw three cards, then discard a card."],
            |d, _| {
                d.reduction = Some(Reduction::IfCastAnotherSpell(2));
                spell(d, vec![], vec![draw(3), discard(1)]);
            },
        ),
        "Salt Road Packbeast" => s(&["Affinity for creatures", "When this creature enters, draw a card."], |d, _| {
            d.reduction = Some(Reduction::PerCreatureYouControl);
            on(d, When::Enters, vec![], vec![draw(1)]);
        }),
        "Highspire Bell-Ringer" => s(&["The second spell you cast each turn costs {1} less to cast."], |d, _| {
            d.second_spell_discount = 1;
        }),
        "Dragon's Prey" => s(
            &["This spell costs {2} more to cast if it targets a Dragon.", "Destroy target creature."],
            |d, _| {
                d.reduction = Some(Reduction::MoreIfTargetsSubtype("Dragon", 2));
                spell(d, vec![CREATURE], vec![Effect::Destroy(0)]);
            },
        ),
        "Static Snare" => s(
            &[
                "This spell costs {1} less to cast for each attacking creature.",
                "When this enchantment enters, exile target artifact or creature an opponent controls until this enchantment leaves the battlefield.",
            ],
            |d, _| {
                d.reduction = Some(Reduction::PerAnyAttackingCreature);
                let f = Filter { artifacts: true, ..OPP_CREATURE };
                on(d, When::Enters, vec![f], vec![Effect::ExileUntilSourceLeaves(0)]);
            },
        ),
        // ---- TDM batch 6a: flurry and endure ----
        "Cori Mountain Stalwart" => s(
            &["Flurry — Whenever you cast your second spell each turn, this creature deals 2 damage to each opponent and you gain 2 life."],
            |d, _| flurry(d, vec![], vec![damage(2, Recipient::EachOpponent), Effect::GainLife { who: Who::You, n: 2 }]),
        ),
        "Devoted Duelist" => s(
            &["Flurry — Whenever you cast your second spell each turn, this creature deals 1 damage to each opponent."],
            |d, _| flurry(d, vec![], vec![damage(1, Recipient::EachOpponent)]),
        ),
        "Equilibrium Adept" => s(
            &[
                "When this creature enters, exile the top card of your library. Until the end of your next turn, you may play that card.",
                "Flurry — Whenever you cast your second spell each turn, this creature gains double strike until end of turn.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::ExileTopChooseOneToPlay { n: 1, this_turn: false }]);
                flurry(d, vec![], vec![Effect::GrantSelf { keywords: Keywords::DOUBLE_STRIKE }]);
            },
        ),
        "Jeskai Devotee" => s(
            &[
                "Flurry — Whenever you cast your second spell each turn, this creature gets +1/+1 until end of turn.",
                "{1}: Add {U}, {R}, or {W}. Activate only once each turn.",
            ],
            |d, _| {
                flurry(d, vec![], vec![Effect::PumpSelf { power: 1, toughness: 1 }]);
                devotee(d, &[Color::White, Color::Blue, Color::Red]);
            },
        ),
        "Poised Practitioner" => s(
            &["Flurry — Whenever you cast your second spell each turn, put a +1/+1 counter on this creature. Scry 1."],
            |d, _| flurry(d, vec![], vec![Effect::CountersSelf { n: 1 }, Effect::Scry { n: 1 }]),
        ),
        "Wayspeaker Bodyguard" => s(
            &[
                "When this creature enters, return target nonland permanent card with mana value 2 or less from your graveyard to your hand.",
                "Flurry — Whenever you cast your second spell each turn, tap target creature an opponent controls.",
            ],
            |d, _| {
                let card = Filter {
                    players: false,
                    creatures: false,
                    planeswalkers: false,
                    graveyard: Some(false),
                    nonland_permanents: true,
                    max_mana_value: Some(2),
                    controller: Controller::You,
                    ..Filter::ANY
                };
                on(d, When::Enters, vec![card], vec![Effect::Bounce(0)]);
                flurry(d, vec![OPP_CREATURE], vec![Effect::Tap(0)]);
            },
        ),
        "Wingblade Disciple" => s(
            &["Flurry — Whenever you cast your second spell each turn, create a 1/1 white Bird creature token with flying."],
            |d, t| flurry(d, vec![], vec![Effect::CreateToken { who: Who::You, def: t.bird, n: 1 }]),
        ),
        "Dusyut Earthcarver" => s(&["When this creature enters, it endures 3."], |d, t| on(d, When::Enters, vec![], vec![endure(3, t)])),
        "Fortress Kin-Guard" => s(&["When this creature enters, it endures 1."], |d, t| on(d, When::Enters, vec![], vec![endure(1, t)])),
        "Kin-Tree Nurturer" => s(&["When this creature enters, it endures 1."], |d, t| on(d, When::Enters, vec![], vec![endure(1, t)])),
        "Sandskitter Outrider" => s(&["When this creature enters, it endures 2."], |d, t| on(d, When::Enters, vec![], vec![endure(2, t)])),
        "Inspirited Vanguard" => s(&["Whenever this creature enters or attacks, it endures 2."], |d, t| {
            on(d, When::Enters, vec![], vec![endure(2, t)]);
            on(d, When::Attacks, vec![], vec![endure(2, t)]);
        }),
        "Descendant of Storms" => s(
            &["Whenever this creature attacks, you may pay {1}{W}. If you do, it endures 1."],
            |d, t| {
                let cost = ManaCost::parse("{1}{W}").expect("cost");
                on(d, When::Attacks, vec![], vec![Effect::MayPay(cost), endure(1, t)]);
            },
        ),
        // ---- TDM batch 6b: renew and harmonize ----
        "Adorned Crocodile" => s(
            &[
                "When this creature dies, create a 2/2 black Zombie Druid creature token.",
                "Renew — {B}, Exile this card from your graveyard: Put a +1/+1 counter on target creature. Activate only as a sorcery.",
            ],
            |d, t| {
                on(d, When::Dies, vec![], vec![Effect::CreateToken { who: Who::You, def: t.zombie_druid, n: 1 }]);
                renew(d, "{B}", vec![CREATURE], vec![Effect::Counters { slot: 0, n: 1 }]);
            },
        ),
        "Agent of Kotis" => s(
            &["Renew — {3}{U}, Exile this card from your graveyard: Put two +1/+1 counters on target creature. Activate only as a sorcery."],
            |d, _| renew(d, "{3}{U}", vec![CREATURE], vec![Effect::Counters { slot: 0, n: 2 }]),
        ),
        "Alchemist's Assistant" => s(
            &["Renew — {1}{B}, Exile this card from your graveyard: Put a lifelink counter on target creature. Activate only as a sorcery."],
            |d, _| renew(d, "{1}{B}", vec![CREATURE], vec![Effect::KeywordCounter { slot: 0, keywords: Keywords::LIFELINK }]),
        ),
        "Champion of Dusan" => s(
            &["Renew — {1}{G}, Exile this card from your graveyard: Put a +1/+1 counter and a trample counter on target creature. Activate only as a sorcery."],
            |d, _| {
                renew(
                    d,
                    "{1}{G}",
                    vec![CREATURE],
                    vec![Effect::Counters { slot: 0, n: 1 }, Effect::KeywordCounter { slot: 0, keywords: Keywords::TRAMPLE }],
                )
            },
        ),
        "Constrictor Sage" => s(
            &[
                "When this creature enters, tap target creature an opponent controls and put a stun counter on it.",
                "Renew — {2}{U}, Exile this card from your graveyard: Tap target creature an opponent controls and put a stun counter on it. Activate only as a sorcery.",
            ],
            |d, _| {
                on(d, When::Enters, vec![OPP_CREATURE], vec![Effect::TapAndStun(0)]);
                renew(d, "{2}{U}", vec![OPP_CREATURE], vec![Effect::TapAndStun(0)]);
            },
        ),
        "Sagu Pummeler" => s(
            &["Renew — {4}{G}, Exile this card from your graveyard: Put two +1/+1 counters and a reach counter on target creature. Activate only as a sorcery."],
            |d, _| {
                renew(
                    d,
                    "{4}{G}",
                    vec![CREATURE],
                    vec![Effect::Counters { slot: 0, n: 2 }, Effect::KeywordCounter { slot: 0, keywords: Keywords::REACH }],
                )
            },
        ),
        "Sage of the Fang" => s(
            &[
                "When this creature enters, put a +1/+1 counter on target creature.",
                "Renew — {3}{G}, Exile this card from your graveyard: Put a +1/+1 counter on target creature, then double the number of +1/+1 counters on that creature. Activate only as a sorcery.",
            ],
            |d, _| {
                on(d, When::Enters, vec![CREATURE], vec![Effect::Counters { slot: 0, n: 1 }]);
                renew(d, "{3}{G}", vec![CREATURE], vec![Effect::Counters { slot: 0, n: 1 }, Effect::DoubleCounters(0)]);
            },
        ),
        "Unending Whisper" => s(&["Draw a card.", "Harmonize {5}{U}"], |d, _| {
            spell(d, vec![], vec![draw(1)]);
            harmonize(d, "{5}{U}");
        }),
        "Wild Ride" => s(&["Target creature gets +3/+0 and gains haste until end of turn.", "Harmonize {4}{R}"], |d, _| {
            spell(d, vec![CREATURE], vec![Effect::Pump { slot: 0, power: 3, toughness: 0 }, Effect::Grant { slot: 0, keywords: Keywords::HASTE }]);
            harmonize(d, "{4}{R}");
        }),
        "Roamer's Routine" => s(
            &["Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.", "Harmonize {4}{G}"],
            |d, _| {
                spell(d, vec![], vec![search(SearchFor::BasicLand, SearchTo::Battlefield { tapped: true })]);
                harmonize(d, "{4}{G}");
            },
        ),
        "Ureni's Rebuff" => s(&["Return target creature to its owner's hand.", "Harmonize {5}{U}"], |d, _| {
            spell(d, vec![CREATURE], vec![Effect::Bounce(0)]);
            harmonize(d, "{5}{U}");
        }),
        "Channeled Dragonfire" => s(&["Channeled Dragonfire deals 2 damage to any target.", "Harmonize {5}{R}{R}"], |d, _| {
            spell(d, vec![Filter::ANY], vec![damage(2, Recipient::Target(0))]);
            harmonize(d, "{5}{R}{R}");
        }),
        "Mammoth Bellow" => s(&["Create a 5/5 green Elephant creature token.", "Harmonize {5}{G}{U}{R}"], |d, t| {
            spell(d, vec![], vec![Effect::CreateToken { who: Who::You, def: t.elephant, n: 1 }]);
            harmonize(d, "{5}{G}{U}{R}");
        }),
        // ---- TDM batch 7: Omens ----
        "Dirgur Island Dragon" => s(&["Ward {2}", "Omen: Tap up to one target creature. Draw a card."], |d, _| {
            d.ward = ManaCost::parse("{2}");
            omen(d, up_to(vec![CREATURE], vec![Effect::Tap(0), draw(1)]));
        }),
        "Disruptive Stormbrood" => s(
            &[
                "When this creature enters, destroy up to one target artifact or enchantment.",
                "Omen: Destroy target creature with power 3 or less.",
            ],
            |d, _| {
                d.triggers.push(Trigger { when: When::Enters, ability: up_to(vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::Destroy(0)]) });
                let small = Filter { creature_max_power: Some(3), ..CREATURE };
                omen(d, Ability::new(vec![small], vec![Effect::Destroy(0)]));
            },
        ),
        "Riling Dawnbreaker" => s(
            &[
                "At the beginning of combat on your turn, another target creature you control gets +1/+0 until end of turn.",
                "Omen: Create a 2/2 white Soldier creature token.",
            ],
            |d, t| {
                let other = Filter { other: true, ..YOUR_CREATURE };
                on(d, When::BeginCombatYourTurn, vec![other], vec![Effect::Pump { slot: 0, power: 1, toughness: 0 }]);
                omen(d, Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.soldier_2, n: 1 }]));
            },
        ),
        "Runescale Stormbrood" => s(
            &[
                "Whenever you cast a noncreature spell or a Dragon spell, this creature gets +2/+0 until end of turn.",
                "Omen: Counter target spell with mana value 2 or less.",
            ],
            |d, _| {
                on(d, When::YouCastNoncreatureOrSubtype("Dragon"), vec![], vec![Effect::PumpSelf { power: 2, toughness: 0 }]);
                let cheap = Filter { max_mana_value: Some(2), ..SPELL };
                omen(d, Ability::new(vec![cheap], vec![Effect::Counter(0)]));
            },
        ),
        "Sagu Wildling" => s(
            &[
                "When this creature enters, you gain 3 life.",
                "Omen: Search your library for a basic land card, reveal it, put it into your hand, then shuffle.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::GainLife { who: Who::You, n: 3 }]);
                omen(d, Ability::new(vec![], vec![search(SearchFor::BasicLand, SearchTo::Hand)]));
            },
        ),
        "Twinmaw Stormbrood" => s(
            &["When this creature enters, you gain 5 life.", "Omen: Charring Bite deals 5 damage to target creature without flying."],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::GainLife { who: Who::You, n: 5 }]);
                let ground = Filter { creature_without_flying: true, ..CREATURE };
                omen(d, Ability::new(vec![ground], vec![damage(5, Recipient::Target(0))]));
            },
        ),
        "Whirlwing Stormbrood" => s(
            &[
                "You may cast sorcery spells and Dragon spells as though they had flash.",
                "Omen: Put three +1/+1 counters on target creature you control.",
            ],
            |d, _| {
                d.flash_sorceries_and_dragons = true;
                omen(d, Ability::new(vec![YOUR_CREATURE], vec![Effect::Counters { slot: 0, n: 3 }]));
            },
        ),
        "Marang River Regent" => s(
            &[
                "When this creature enters, return up to two other target nonland permanents to their owners' hands.",
                "Omen: Draw three cards, then discard a card.",
            ],
            |d, _| {
                let other = Filter { players: false, creatures: false, planeswalkers: false, nonland_permanents: true, other: true, ..Filter::ANY };
                d.triggers.push(Trigger { when: When::Enters, ability: up_to(vec![other, other], vec![Effect::Bounce(0), Effect::Bounce(1)]) });
                omen(d, Ability::new(vec![], vec![draw(3), discard(1)]));
            },
        ),
        "Stormshriek Feral" => s(
            &["{1}{R}: This creature gets +1/+0 until end of turn.", "Omen: Discard a card. If you do, draw two cards."],
            |d, _| {
                activated(d, "{1}{R}", false, vec![], vec![Effect::PumpSelf { power: 1, toughness: 0 }]);
                omen(d, Ability::new(vec![], vec![Effect::DiscardThenDraw(2)]));
            },
        ),
        // ---- TDM batch 8: self-bounce (the Dragonstorms), land-type searches (Monuments, fetches) ----
        "Roiling Dragonstorm" => s(
            &["When this enchantment enters, draw two cards, then discard a card.", DRAGONSTORM_RETURN],
            |d, _| {
                on(d, When::Enters, vec![], vec![draw(2), discard(1)]);
                dragonstorm_return(d);
            },
        ),
        "Corroding Dragonstorm" => s(
            &["When this enchantment enters, each opponent loses 2 life and you gain 2 life. Surveil 2.", DRAGONSTORM_RETURN],
            |d, _| {
                on(
                    d,
                    When::Enters,
                    vec![],
                    vec![
                        Effect::LoseLife { who: Who::EachOpponent, n: 2 },
                        Effect::GainLife { who: Who::You, n: 2 },
                        Effect::Surveil { n: 2 },
                    ],
                );
                dragonstorm_return(d);
            },
        ),
        "Encroaching Dragonstorm" => s(
            &[
                "When this enchantment enters, search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle.",
                DRAGONSTORM_RETURN,
            ],
            |d, _| {
                let two = Effect::Search { what: SearchFor::BasicLand, to: SearchTo::Battlefield { tapped: true }, n: 2, kicked_n: None };
                on(d, When::Enters, vec![], vec![two]);
                dragonstorm_return(d);
            },
        ),
        "Teeming Dragonstorm" => s(
            &["When this enchantment enters, create two 2/2 white Soldier creature tokens.", DRAGONSTORM_RETURN],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.soldier_2, n: 2 }]);
                dragonstorm_return(d);
            },
        ),
        "Fleeting Effigy" => s(
            &["At the beginning of your end step, return this creature to its owner's hand.", "{2}{R}: This creature gets +2/+0 until end of turn."],
            |d, _| {
                on(d, When::EndStep { your_turn_only: true }, vec![], vec![Effect::BounceSelf]);
                activated(d, "{2}{R}", false, vec![], vec![Effect::PumpSelf { power: 2, toughness: 0 }]);
            },
        ),
        "Abzan Monument" => s(
            &[
                "When this artifact enters, search your library for a basic Plains, Swamp, or Forest card, reveal it, put it into your hand, then shuffle.",
                "{1}{W}{B}{G}, {T}, Sacrifice this artifact: Create an X/X white Spirit creature token, where X is the greatest toughness among creatures you control. Activate only as a sorcery.",
            ],
            |d, t| monument(d, &["Plains", "Swamp", "Forest"], "{1}{W}{B}{G}", vec![Effect::SpiritGreatestToughness { spirit: t.spirit_endure }]),
        ),
        "Jeskai Monument" => s(
            &[
                "When this artifact enters, search your library for a basic Island, Mountain, or Plains card, reveal it, put it into your hand, then shuffle.",
                "{1}{U}{R}{W}, {T}, Sacrifice this artifact: Create two 1/1 white Bird creature tokens with flying. Activate only as a sorcery.",
            ],
            |d, t| monument(d, &["Island", "Mountain", "Plains"], "{1}{U}{R}{W}", vec![Effect::CreateToken { who: Who::You, def: t.bird, n: 2 }]),
        ),
        "Mardu Monument" => s(
            &[
                "When this artifact enters, search your library for a basic Mountain, Plains, or Swamp card, reveal it, put it into your hand, then shuffle.",
                "{2}{R}{W}{B}, {T}, Sacrifice this artifact: Create three 1/1 red Warrior creature tokens. They gain menace and haste until end of turn. Activate only as a sorcery.",
            ],
            |d, t| {
                monument(
                    d,
                    &["Mountain", "Plains", "Swamp"],
                    "{2}{R}{W}{B}",
                    vec![
                        Effect::CreateToken { who: Who::You, def: t.warrior, n: 3 },
                        Effect::GrantCreated(Keywords::MENACE.with(Keywords::HASTE)),
                    ],
                )
            },
        ),
        "Sultai Monument" => s(
            &[
                "When this artifact enters, search your library for a basic Swamp, Forest, or Island card, reveal it, put it into your hand, then shuffle.",
                "{2}{B}{G}{U}, {T}, Sacrifice this artifact: Create two 2/2 black Zombie Druid creature tokens. Activate only as a sorcery.",
            ],
            |d, t| monument(d, &["Swamp", "Forest", "Island"], "{2}{B}{G}{U}", vec![Effect::CreateToken { who: Who::You, def: t.zombie_druid, n: 2 }]),
        ),
        "Temur Monument" => s(
            &[
                "When this artifact enters, search your library for a basic Forest, Island, or Mountain card, reveal it, put it into your hand, then shuffle.",
                "{3}{G}{U}{R}, {T}, Sacrifice this artifact: Create a 5/5 green Elephant creature token. Activate only as a sorcery.",
            ],
            |d, t| monument(d, &["Forest", "Island", "Mountain"], "{3}{G}{U}{R}", vec![Effect::CreateToken { who: Who::You, def: t.elephant, n: 1 }]),
        ),
        "Misty Rainforest" => s(
            &["{T}, Pay 1 life, Sacrifice this land: Search your library for a Forest or Island card, put it onto the battlefield, then shuffle."],
            |d, _| fetch_land(d, &["Forest", "Island"]),
        ),
        "Arid Mesa" => s(
            &["{T}, Pay 1 life, Sacrifice this land: Search your library for a Mountain or Plains card, put it onto the battlefield, then shuffle."],
            |d, _| fetch_land(d, &["Mountain", "Plains"]),
        ),
        "Scalding Tarn" => s(
            &["{T}, Pay 1 life, Sacrifice this land: Search your library for an Island or Mountain card, put it onto the battlefield, then shuffle."],
            |d, _| fetch_land(d, &["Island", "Mountain"]),
        ),
        "Verdant Catacombs" => s(
            &["{T}, Pay 1 life, Sacrifice this land: Search your library for a Swamp or Forest card, put it onto the battlefield, then shuffle."],
            |d, _| fetch_land(d, &["Swamp", "Forest"]),
        ),
        "Marsh Flats" => s(
            &["{T}, Pay 1 life, Sacrifice this land: Search your library for a Plains or Swamp card, put it onto the battlefield, then shuffle."],
            |d, _| fetch_land(d, &["Plains", "Swamp"]),
        ),
        // ---- TDM batch 9: Sagas, and the rest of the Omens ----
        "Roar of Endless Song" => s(
            &[
                "I, II — Create a 5/5 green Elephant creature token.",
                "III — Double the power and toughness of each creature you control until end of turn.",
            ],
            |d, t| {
                for n in [1, 2] {
                    chapter(d, n, Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.elephant, n: 1 }]));
                }
                chapter(d, 3, Ability::new(vec![], vec![Effect::DoublePowerToughnessAll]));
            },
        ),
        "Revival of the Ancestors" => s(
            &[
                "I — Create three 1/1 white Spirit creature tokens.",
                "II — Distribute three +1/+1 counters among one, two, or three target creatures you control.",
                "III — Creatures you control gain trample and lifelink until end of turn.",
            ],
            |d, t| {
                chapter(d, 1, Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.spirit_1, n: 3 }]));
                let mut two = Ability::new(vec![YOUR_CREATURE; 3], vec![Effect::DividedCounters]);
                two.min_targets = Some(1);
                two.divide = 3;
                chapter(d, 2, two);
                let team = Effect::GrantAll { who: Who::You, keywords: Keywords::TRAMPLE.with(Keywords::LIFELINK) };
                chapter(d, 3, Ability::new(vec![], vec![team]));
            },
        ),
        "Feral Deathgorger" => s(
            &[
                "When this creature enters, exile up to two target cards from a single graveyard.",
                "Omen: Put a +1/+1 counter on up to one target creature. Draw a card.",
            ],
            |d, _| {
                let f = graveyard_cards(false, Controller::Any);
                let mut a = up_to(vec![f, f], vec![Effect::Exile(0), Effect::Exile(1)]);
                a.same_graveyard = true;
                d.triggers.push(Trigger { when: When::Enters, ability: a });
                omen(d, up_to(vec![CREATURE], vec![Effect::Counters { slot: 0, n: 1 }, draw(1)]));
            },
        ),
        "Purging Stormbrood" => s(
            &[
                "Ward—Pay 2 life.",
                "When this creature enters, remove all counters from up to one target creature.",
                "Omen: Target creature gets +2/+2 and gains lifelink and hexproof until end of turn.",
            ],
            |d, _| {
                d.ward_life = Some(2);
                d.triggers.push(Trigger { when: When::Enters, ability: up_to(vec![CREATURE], vec![Effect::RemoveAllCounters(0)]) });
                omen(
                    d,
                    Ability::new(
                        vec![CREATURE],
                        vec![
                            Effect::Pump { slot: 0, power: 2, toughness: 2 },
                            Effect::Grant { slot: 0, keywords: Keywords::LIFELINK.with(Keywords::HEXPROOF) },
                        ],
                    ),
                );
            },
        ),
        // ---- TDM batch 10 (2026-11-25): X in an Omen or harmonize cost, ward—discard ----
        "Scavenger Regent" => s(
            &["Ward—Discard a card.", "Omen: Each non-Dragon creature gets -X/-X until end of turn."],
            |d, _| {
                d.ward_discard = true;
                omen(d, Ability::new(vec![], vec![Effect::ShrinkAllWithoutSubtypeX("Dragon")]));
            },
        ),
        "Taigam, Master Opportunist" => s(
            &["Flurry — Whenever you cast your second spell each turn, copy it, then exile the spell you cast with four time counters on it. If it doesn't have suspend, it gains suspend."],
            |d, _| flurry(d, vec![], vec![Effect::CopyThenSuspendSpell]),
        ),
        "Naga Fleshcrafter" => s(
            &[
                "You may have this creature enter as a copy of any creature on the battlefield.",
                "Renew — {2}{U}, Exile this card from your graveyard: Put a +1/+1 counter on target nonlegendary creature you control. Each other creature you control becomes a copy of that creature until end of turn. Activate only as a sorcery.",
            ],
            |d, _| {
                d.enter_as_copy = true;
                let target = Filter { nonlegendary: true, ..YOUR_CREATURE };
                renew(d, "{2}{U}", vec![target], vec![Effect::Counters { slot: 0, n: 1 }, Effect::OthersBecomeCopyOfTarget(0)]);
            },
        ),
        "Teval, Arbiter of Virtue" => s(
            &["Spells you cast have delve.", "Whenever you cast a spell, you lose life equal to its mana value."],
            |d, _| {
                d.spells_have_delve = true;
                on(d, When::YouCast, vec![], vec![Effect::LoseLifeCastSpellManaValue]);
            },
        ),
        "Nature's Rhythm" => s(
            &[
                "Search your library for a creature card with mana value X or less, put it onto the battlefield, then shuffle.",
                "Harmonize {X}{G}{G}{G}{G}",
            ],
            |d, _| {
                let to = SearchTo::Battlefield { tapped: false };
                spell(d, vec![], vec![Effect::Search { what: SearchFor::CreatureMvAtMostX, to, n: 1, kicked_n: None }]);
                harmonize(d, "{X}{G}{G}{G}{G}");
            },
        ),
        _ => None,
    }
}

/// Every card with a script, for the drift test.
pub const SCRIPTED: &[&str] = &[
    "Stab",
    "Giant Growth",
    "Hero's Downfall",
    "Boltwave",
    "Burst Lightning",
    "Joust Through",
    "Seismic Rupture",
    "Hidetsugu's Second Rite",
    "Make Your Move",
    "Broken Wings",
    "Helpful Hunter",
    "Fleeting Distraction",
    "Felling Blow",
    "Infestation Sage",
    "Guarded Heir",
    "Prideful Parent",
    "Dragon Trainer",
    "Resolute Reinforcements",
    "Angel of Finality",
    "Gorehorn Raider",
    "Skyship Buccaneer",
    "Dwynen's Elite",
    "Burglar Rat",
    "Icewind Elemental",
    "Incinerating Blast",
    "Wary Thespian",
    "Lightshell Duo",
    "Uncharted Voyage",
    "Refute",
    "Essence Scatter",
    "Reclamation Sage",
    "Affectionate Indrik",
    "Meteor Golem",
    "Bigfin Bouncer",
    "Campus Guide",
    "Solemn Simulacrum",
    "Micromancer",
    "Rune-Scarred Demon",
    "Grim Tutor",
    "Grow from the Ashes",
    "Pilfer",
    "Hare Apparent",
    "Snakeskin Veil",
    "Stroke of Midnight",
    "Bake into a Pie",
    "Gleaming Barrier",
    "An Offer You Can't Refuse",
    "Arbiter of Woe",
    "Eaten Alive",
    "Thrill of Possibility",
    "Arcane Epiphany",
    "Luminous Rebuke",
    "Think Twice",
    "Felidar Savior",
    "Macabre Waltz",
    "Soul-Shackled Zombie",
    "Faebloom Trick",
    "Banishing Light",
    "Fake Your Own Death",
    "Bloodfell Caves",
    "Blossoming Sands",
    "Dismal Backwater",
    "Jungle Hollow",
    "Rugged Highlands",
    "Scoured Barrens",
    "Swiftwater Cliffs",
    "Thornwood Falls",
    "Tranquil Cove",
    "Wind-Scarred Crag",
    "Evolving Wilds",
    "Rogue's Passage",
    "Llanowar Elves",
    "Dazzling Angel",
    "Beast-Kin Ranger",
    "Hungry Ghoul",
    "Cathar Commando",
    "Strix Lookout",
    "Spectral Sailor",
    "Treetop Snarespinner",
    "Bite Down",
    "Vampire Soulcaller",
    "Elementalist Adept",
    "Ambush Wolf",
    "Goblin Boarders",
    "Gnarlid Colony",
    "Inspiring Paladin",
    "Courageous Goblin",
    "Sun-Blessed Healer",
    "Vanguard Seraph",
    "Cat Collector",
    "Mischievous Mystic",
    "Abrade",
    "Bushwhack",
    "Goblin Surprise",
    "Seeker's Folly",
    "Slagstorm",
    "Run Away Together",
    "Goldvein Pick",
    "Quick-Draw Katana",
    "Swiftfoot Boots",
    "Adventuring Gear",
    "Celestial Armor",
    "Twinblade Blessing",
    "Sanguine Syphoner",
    "Ajani's Pridemate",
    "Marauding Blight-Priest",
    "Elfsworn Giant",
    "Tatyova, Benthic Druid",
    "Eager Trufflesnout",
    "Elvish Regrower",
    "Erudite Wizard",
    "Fanatical Firebrand",
    "Axgard Cavalry",
    "Sure Strike",
    "Overrun",
    "Heroic Reinforcements",
    "Battlesong Berserker",
    "Youthful Valkyrie",
    "Vengeful Bloodwitch",
    "Nessian Hornbeetle",
    "Brineborn Cutthroat",
    "Firebrand Archer",
    "Empyrean Eagle",
    "Mocking Sprite",
    "Slumbering Cerberus",
    "Needletooth Pack",
    "Billowing Shriekmass",
    "Dreadwing Scavenger",
    "Diregraf Ghoul",
    "Dauntless Veteran",
    "Shivan Dragon",
    "Zombify",
    "Day of Judgment",
    "Phyrexian Arena",
    "Scrawling Crawler",
    "Rune-Sealed Wall",
    "Searslicer Goblin",
    "Good-Fortune Unicorn",
    "Clinquant Skymage",
    "Archmage of Runes",
    "Balmor, Battlemage Captain",
    "Firespitter Whelp",
    "Sower of Chaos",
    "Heartfire Immolator",
    "Cackling Prowler",
    "Apothecary Stomper",
    "Wardens of the Cycle",
    "Ruby, Daring Tracker",
    "Anthem of Champions",
    "Crackling Cyclops",
    "Spitfire Lagac",
    "Preposterous Proportions",
    "Goblin Bushwhacker",
    "Burnished Hart",
    "Rite of the Dragoncaller",
    "Crypt Feaster",
    "Paradise Druid",
    "Condemn",
    "Aetherize",
    "Juggernaut",
    "Raise the Past",
    "Rise of the Dark Realms",
    "Authority of the Consuls",
    "Mossborn Hydra",
    "Herald of Eternal Dawn",
    "Bulk Up",
    "Vampire Gourmand",
    "High-Society Hunter",
    "Reassembling Skeleton",
    "Gutless Plunderer",
    "Squad Rallier",
    "Inspiration from Beyond",
    "Alesha, Who Laughs at Fate",
    "Fiendish Panda",
    "Kiora, the Rising Tide",
    "Cephalid Inkmage",
    "Stromkirk Bloodthief",
    "Mischievous Pup",
    "Arahbo, the First Fang",
    "Scavenging Ooze",
    "Garruk's Uprising",
    "Armasaur Guide",
    "Grappling Kraken",
    "Sylvan Scavenging",
    "Frenzied Goblin",
    "Sire of Seven Deaths",
    "Koma, World-Eater",
    "Drakuseth, Maw of Flames",
    "Painful Quandary",
    "Exemplar of Light",
    "Claws Out",
    "Spinner of Souls",
    "Zimone, Paradox Sculptor",
    "Bloodthirsty Conqueror",
    "Niv-Mizzet, Visionary",
    "Drake Hatcher",
    "Infernal Vessel",
    "Valkyrie's Call",
    "Skyknight Squire",
    "Mild-Mannered Librarian",
    "Divine Resilience",
    "Loot, Exuberant Explorer",
    "Perforating Artist",
    "Consuming Aberration",
    "Elenda, Saint of Dusk",
    "Blasphemous Edict",
    "Leyline Axe",
    "High Fae Trickster",
    "Flamewake Phoenix",
    "Sphinx of Forgotten Lore",
    "Ravenous Amulet",
    "Lathril, Blade of the Elves",
    "Kykar, Zephyr Awakener",
    "Nine-Lives Familiar",
    "Time Stop",
    "Witness Protection",
    "Heraldic Banner",
    "Strongbox Raider",
    "Curator of Destinies",
    "Akroma's Memorial",
    "Sphinx's Tutelage",
    "Fishing Pole",
    "Tinybones, Bauble Burglar",
    "Fleeting Flight",
    "Tragic Banshee",
    "Tolarian Terror",
    "Dwynen, Gilt-Leaf Daen",
    "Krenko, Mob Boss",
    "Revenge of the Rats",
    "Elvish Archdruid",
    "Lunar Insight",
    "Inspiring Call",
    "Midnight Snack",
    "Crystal Barricade",
    "Blanchwood Armor",
    "Brass's Bounty",
    "Temporal Manipulation",
    "Ghalta, Primal Hunger",
    "Ashroot Animist",
    "Embercleave",
    "Wildwood Scourge",
    "Goblin Negotiation",
    "Exsanguinate",
    "Genesis Wave",
    "Fiend Artisan",
    "Self-Reflection",
    "Electroduplicate",
    "Homunculus Horde",
    "Extravagant Replication",
    "Abyssal Harvester",
    "Liliana, Dreadhorde General",
    "Kaito, Cunning Infiltrator",
    "Ajani, Caller of the Pride",
    "Vivien Reid",
    "Chandra, Flameshaper",
    "Doubling Season",
    "Twinflame Tyrant",
    "Omniscience",
    "Progenitus",
    "Zul Ashur, Lich Lord",
    "Involuntary Employment",
    "Fiery Annihilation",
    "Bloom Tender",
    "Giada, Font of Hope",
    "Banner of Kinship",
    "Quilled Greatwurm",
    "Thousand-Year Storm",
    "Etali, Primal Storm",
    "Imprisoned in the Moon",
    "Soulstone Sanctuary",
    "Kellan, Planar Trailblazer",
    "Secluded Courtyard",
    "Muldrotha, the Gravetide",
];

/// Tarkir: Dragonstorm cards with scripts (each with a test in tests/tdm.rs); `lib.rs` checks every one
/// is fully supported in the TDM set. Progress: docs/tdm-progress.md.
pub const TDM_SCRIPTED: &[&str] = &[
    "Scavenger Regent",
    "Nature's Rhythm",
    "Teval, Arbiter of Virtue",
    "Naga Fleshcrafter",
    "Taigam, Master Opportunist",
    "Sibsig Appraiser",
    "Iceridge Serpent",
    "Boulderborn Dragon",
    "Humbling Elder",
    "Rebellious Strike",
    "Sarkhan's Resolve",
    "Underfoot Underdogs",
    "Watcher of the Wayside",
    "Meticulous Artisan",
    "Stormplain Detainment",
    "Caustic Exhale",
    "Dispelling Exhale",
    "Molten Exhale",
    "Osseous Exhale",
    "Piercing Exhale",
    "Riverwalk Technique",
    "Ainok Wayfarer",
    "Dragonstorm Globe",
    "Undergrowth Leopard",
    "Narset's Rebuke",
    "Defibrillating Current",
    "Dragonclaw Strike",
    "Gurmag Nightwatch",
    "Kin-Tree Severance",
    "Monastery Messenger",
    "Rakshasa's Bargain",
    "Reputable Merchant",
    "Riverwheel Sweep",
    "Temur Tawnyback",
    "Jade-Cast Sentinel",
    "Dalkovan Packbeasts",
    "Dragonback Lancer",
    "Shock Brigade",
    "Nightblade Brigade",
    "Reigning Victor",
    "Avenger of the Fallen",
    "Venerated Stormsinger",
    "Bone-Cairn Butcher",
    "Zurgo's Vanguard",
    "Abzan Devotee",
    "Mardu Devotee",
    "Sultai Devotee",
    "Temur Devotee",
    "Focus the Mind",
    "Salt Road Packbeast",
    "Highspire Bell-Ringer",
    "Dragon's Prey",
    "Static Snare",
    "Cori Mountain Stalwart",
    "Devoted Duelist",
    "Equilibrium Adept",
    "Jeskai Devotee",
    "Poised Practitioner",
    "Wayspeaker Bodyguard",
    "Wingblade Disciple",
    "Dusyut Earthcarver",
    "Fortress Kin-Guard",
    "Kin-Tree Nurturer",
    "Sandskitter Outrider",
    "Inspirited Vanguard",
    "Descendant of Storms",
    "Adorned Crocodile",
    "Agent of Kotis",
    "Alchemist's Assistant",
    "Champion of Dusan",
    "Constrictor Sage",
    "Sagu Pummeler",
    "Sage of the Fang",
    "Unending Whisper",
    "Wild Ride",
    "Roamer's Routine",
    "Ureni's Rebuff",
    "Channeled Dragonfire",
    "Mammoth Bellow",
    "Dirgur Island Dragon",
    "Disruptive Stormbrood",
    "Riling Dawnbreaker",
    "Runescale Stormbrood",
    "Sagu Wildling",
    "Twinmaw Stormbrood",
    "Whirlwing Stormbrood",
    "Marang River Regent",
    "Stormshriek Feral",
    "Roiling Dragonstorm",
    "Corroding Dragonstorm",
    "Encroaching Dragonstorm",
    "Teeming Dragonstorm",
    "Fleeting Effigy",
    "Abzan Monument",
    "Jeskai Monument",
    "Mardu Monument",
    "Sultai Monument",
    "Temur Monument",
    "Misty Rainforest",
    "Arid Mesa",
    "Scalding Tarn",
    "Verdant Catacombs",
    "Marsh Flats",
    "Roar of Endless Song",
    "Revival of the Ancestors",
    "Feral Deathgorger",
    "Purging Stormbrood",
];
