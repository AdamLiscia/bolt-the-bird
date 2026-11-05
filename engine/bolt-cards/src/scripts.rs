//! Card scripts: the rules text the engine implements, card by card, as effect-language data.
//!
//! Each script lists the exact ability lines it implements (reminder text removed). A card is
//! supported only when its scripts and keywords account for every line, and a test checks each
//! script's lines against the Scryfall text, so a script can't drift from its card.

use bolt_core::effect::{
    Ability, Condition, Controller, Effect, Filter, Recipient, Scope, SearchFor, SearchTo, Static,
    StaticCondition, Trigger, When, Who,
};
use bolt_core::{Activated, CardDef, Color, ExtraCost, Keywords, ManaCost, ManaSource, Reduction, Types};

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
        scope,
        your_turn_only,
        condition: None,
        power: 0,
        toughness: 0,
        keywords,
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
        scope: Scope::Attached,
        your_turn_only,
        condition: None,
        power,
        toughness,
        keywords,
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
        scope: Scope::This,
        your_turn_only: false,
        condition: Some(StaticCondition::Threshold),
        power,
        toughness,
        keywords,
    });
}

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
                scope: Scope::OtherYourFliers,
                your_turn_only: false,
                condition: None,
                power: 1,
                toughness: 1,
                keywords: Keywords::NONE,
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
];
