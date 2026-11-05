//! Card scripts: the rules text the engine implements, card by card, as effect-language data.
//!
//! Each script lists the exact ability lines it implements (reminder text removed). A card is
//! supported only when its scripts and keywords account for every line, and a test checks each
//! script's lines against the Scryfall text, so a script can't drift from its card.

use bolt_core::effect::{Ability, Condition, Controller, Effect, Filter, Recipient, Trigger, When, Who};
use bolt_core::{CardDef, Keywords, ManaCost};

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
    ]
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

/// The script for a card, if one exists.
pub fn script(name: &str) -> Option<Script> {
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
            |d, t| on(d, When::Dies, vec![], vec![Effect::CreateToken { def: t.insect, n: 1 }]),
        ),
        "Guarded Heir" => s(&["When this creature enters, create two 3/3 white Knight creature tokens."], |d, t| {
            on(d, When::Enters, vec![], vec![Effect::CreateToken { def: t.knight, n: 2 }])
        }),
        "Prideful Parent" => s(&["When this creature enters, create a 1/1 white Cat creature token."], |d, t| {
            on(d, When::Enters, vec![], vec![Effect::CreateToken { def: t.cat, n: 1 }])
        }),
        "Dragon Trainer" => s(
            &["When this creature enters, create a 4/4 red Dragon creature token with flying."],
            |d, t| on(d, When::Enters, vec![], vec![Effect::CreateToken { def: t.dragon, n: 1 }]),
        ),
        "Resolute Reinforcements" => s(
            &["When this creature enters, create a 1/1 white Soldier creature token."],
            |d, t| on(d, When::Enters, vec![], vec![Effect::CreateToken { def: t.soldier, n: 1 }]),
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
                    vec![Effect::CreateToken { def: t.elf_warrior, n: 1 }],
                )
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
];
