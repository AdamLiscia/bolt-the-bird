//! Lane D4 (the desktop): the subsystems theme (dungeons and the initiative, Backgrounds, emblems,
//! dice, the Ring, sagas, split cards). Conventions: docs/lane-brief.md. Card text:
//! assets/PG_oracle.json (in the CMD pool). Tests: tests/pg_d4.rs; the core pieces' own tests are in
//! tests/core_subsystems.rs.
#![allow(unused_imports)]

use super::*;
use bolt_core::SubsystemEffect;

/// A land, with this controller restriction.
const fn land(controller: Controller) -> Filter {
    Filter {
        players: false,
        creatures: false,
        planeswalkers: false,
        lands: true,
        controller,
        ..Filter::ANY
    }
}

/// "target permanent"
const PERMANENT: Filter = Filter {
    players: false,
    any_permanent: true,
    ..Filter::ANY
};

/// "target opponent"
const OPPONENT: Filter = Filter {
    controller: Controller::Opponent,
    ..Filter::PLAYER
};

fn sub(e: SubsystemEffect) -> Effect {
    Effect::Subsystem(e)
}

/// "you lose 1 life"
fn lose_one() -> Effect {
    Effect::LoseLife { who: Who::You, n: 1 }
}

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        // ---- Split cards (CR 709): the right half is cast through the Omen path ----
        "Boom // Bust" => s(
            &[
                "Destroy target land you control and target land you don't control.",
                "Split: Destroy all lands.",
            ],
            |d, _| {
                spell(
                    d,
                    vec![land(Controller::You), land(Controller::Opponent)],
                    vec![Effect::Destroy(0), Effect::Destroy(1)],
                );
                omen(d, Ability::new(vec![], vec![Effect::DestroyAllLands]));
            },
        ),
        "Fire // Ice" => s(
            &[
                "Fire deals 2 damage divided as you choose among one or two targets.",
                "Split: Tap target permanent.",
                "Split: Draw a card.",
            ],
            |d, _| {
                let mut fire = up_to(vec![Filter::ANY, Filter::ANY], vec![Effect::DividedDamage]);
                fire.min_targets = Some(1);
                fire.divide = 2;
                d.spell = Some(fire);
                omen(d, Ability::new(vec![PERMANENT], vec![Effect::Tap(0), draw(1)]));
            },
        ),
        // ---- Dungeons (game/d4.rs) ----
        "Hama Pashar, Ruin Seeker" => s(
            &["Room abilities of dungeons you own trigger an additional time."],
            |d, _| d.subsystems.rooms_trigger_again = true,
        ),
        "Dungeon Delver" => s(
            &["Commander creatures you own have \"Room abilities of dungeons you own trigger an additional time.\""],
            |d, _| d.subsystems.commanders_rooms_trigger_again = true,
        ),
        "Radiant Solar" => s(
            &[
                "Whenever this creature or another nontoken creature you control enters, venture into the dungeon.",
                "{W}, Discard this card: Venture into the dungeon and you gain 3 life.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![sub(SubsystemEffect::Venture)]);
                let others = When::AnotherNontokenCreatureYouControlEnters;
                on(d, others, vec![], vec![sub(SubsystemEffect::Venture)]);
                let gain = Effect::GainLife { who: Who::You, n: 3 };
                let n = activated(d, "{W}", false, vec![], vec![sub(SubsystemEffect::Venture), gain]);
                d.activated[n].from_hand_discard = true;
            },
        ),
        "Barrowin of Clan Undurr" => s(
            &[
                "When Barrowin enters, venture into the dungeon.",
                "Whenever Barrowin attacks, return up to one creature card with mana value 3 or less from your graveyard to the battlefield if you've completed a dungeon.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![sub(SubsystemEffect::Venture)]);
                // d3's untargeted return (a LookPick of the creature cards with mana value 3 or less).
                let back = Effect::ReturnCreatureMvAtMostToBattlefield(3);
                conditional(d, When::Attacks, Condition::CompletedDungeon, vec![], vec![back]);
            },
        ),
        "Nadaar, Selfless Paladin" => s(
            &[
                "Whenever Nadaar enters or attacks, venture into the dungeon.",
                "Other creatures you control get +1/+1 as long as you've completed a dungeon.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![sub(SubsystemEffect::Venture)]);
                on(d, When::Attacks, vec![], vec![sub(SubsystemEffect::Venture)]);
                d.statics.push(Static {
                    subtype: None,
                    lose_abilities: false,
                    base_pt: None,
                    protection: [false; 5],
                    scale: None,
                    scope: Scope::OtherYourCreatures,
                    your_turn_only: false,
                    condition: Some(StaticCondition::CompletedDungeon),
                    power: 1,
                    toughness: 1,
                    keywords: Keywords::NONE,
                    set_colors: None,
                    becomes_land: false,
                });
            },
        ),
        "Yuan-Ti Malison" => s(
            &[
                "This creature can't be blocked as long as it's attacking alone.",
                "Whenever this creature deals combat damage to a player, venture into the dungeon.",
            ],
            |d, _| {
                d.mana_ext.unblockable_attacking_alone = true;
                on(d, When::DealsCombatDamageToPlayer, vec![], vec![sub(SubsystemEffect::Venture)]);
            },
        ),
        "Midnight Pathlighter" => s(
            &[
                "Creatures you control can't be blocked except by legendary creatures.",
                "Whenever one or more creatures you control deal combat damage to a player, venture into the dungeon.",
            ],
            |d, _| {
                d.mana_ext.your_creatures_blocked_only_by_legendary = true;
                let when = When::YourCreaturesDealCombatDamageToPlayer;
                on(d, when, vec![], vec![sub(SubsystemEffect::Venture)]);
            },
        ),
        "Dungeon Descent" => s(
            &[
                "This land enters tapped.",
                "{T}: Add {C}.",
                "{4}, {T}, Tap an untapped legendary creature you control: Venture into the dungeon. Activate only as a sorcery.",
            ],
            |d, _| {
                d.enters_tapped = true;
                d.mana = Some(ManaSource {
                    colorless: true,
                    ..ManaSource::default()
                });
                let n = activated(d, "{4}", true, vec![], vec![sub(SubsystemEffect::Venture)]);
                d.activated[n].sorcery = true;
                // The engine picks the legendary creature it taps (d1's cost).
                d.activated[n].tap_untapped_legendary = true;
            },
        ),
        // ---- The initiative (game/d4.rs: `fire_subsystems` runs its upkeep venture and steal) ----
        "White Plume Adventurer" => s(
            &[
                "When this creature enters, you take the initiative.",
                "At the beginning of each opponent's upkeep, untap a creature you control. If you've completed a dungeon, untap all creatures you control instead.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![sub(SubsystemEffect::TakeInitiative)]);
                let untap = sub(SubsystemEffect::UntapOneOrAllByDungeon);
                let upkeep = When::Upkeep { your_turn_only: false };
                conditional(d, upkeep, Condition::NotYourTurn, vec![], vec![untap]);
            },
        ),
        "Imoen, Mystic Trickster" => s(
            &[
                "Ward {2}",
                "At the beginning of your end step, if you have the initiative, draw a card. Draw another card if you've completed a dungeon.",
                "Choose a Background",
            ],
            |d, _| {
                // Choose a Background is a deck-building rule (`commander_deck_problems_for`).
                d.ward = ManaCost::parse("{2}");
                let more = sub(SubsystemEffect::DrawIfCompletedDungeon(1));
                let end = When::EndStep { your_turn_only: true };
                conditional(d, end, Condition::HaveInitiative, vec![], vec![draw(1), more]);
            },
        ),
        "Safana, Calimport Cutthroat" => s(
            &[
                "At the beginning of your end step, if you have the initiative, create a Treasure token. Create three of those tokens instead if you've completed a dungeon.",
                "Choose a Background",
            ],
            |d, t| {
                let treasures = sub(SubsystemEffect::TokensByDungeon { def: t.treasure, n: 1, completed: 3 });
                let end = When::EndStep { your_turn_only: true };
                conditional(d, end, Condition::HaveInitiative, vec![], vec![treasures]);
            },
        ),
        // ---- Emblems with triggered abilities (`EmblemKind::Triggered`, a `When::Never` trigger) ----
        "Lolth, Spider Queen" => s(
            &[
                "Whenever a creature you control dies, put a loyalty counter on Lolth.",
                "0: You draw a card and you lose 1 life.",
                "−3: Create two 2/1 black Spider creature tokens with reach and menace.",
                "−8: You get an emblem with \"Whenever an opponent is dealt combat damage by one or more creatures you control, if that player lost less than 8 life this turn, they lose life equal to the difference.\"",
            ],
            |d, t| {
                on(d, When::CreatureYouControlDies, vec![], vec![Effect::LoyaltySelf(1)]);
                loyalty(d, 0, Ability::new(vec![], vec![draw(1), lose_one()]));
                let spiders = Effect::CreateToken { who: Who::You, def: t.spider_menace, n: 2 };
                loyalty(d, -3, Ability::new(vec![], vec![spiders]));
                let n = d.triggers.len();
                on(d, When::Never, vec![], vec![sub(SubsystemEffect::OpponentLosesDifferenceFrom(8))]);
                let when = When::YourCreaturesDealCombatDamageToPlayer;
                let emblem = Effect::Emblem(EmblemKind::Triggered { when, trigger: n });
                loyalty(d, -8, Ability::new(vec![], vec![emblem]));
            },
        ),
        "Ob Nixilis Reignited" => s(
            &[
                "+1: You draw a card and you lose 1 life.",
                "−3: Destroy target creature.",
                "−8: Target opponent gets an emblem with \"Whenever a player draws a card, you lose 2 life.\"",
            ],
            |d, _| {
                loyalty(d, 1, Ability::new(vec![], vec![draw(1), lose_one()]));
                loyalty(d, -3, Ability::new(vec![Filter::CREATURE], vec![Effect::Destroy(0)]));
                let n = d.triggers.len();
                on(d, When::Never, vec![], vec![Effect::LoseLife { who: Who::You, n: 2 }]);
                // "A player draws": its owner (YouDraw) or the other player (OpponentDraws), 1v1.
                let emblem = |when| {
                    sub(SubsystemEffect::EmblemForTarget {
                        slot: 0,
                        kind: EmblemKind::Triggered { when, trigger: n },
                    })
                };
                let both = vec![emblem(When::YouDraw), emblem(When::OpponentDraws)];
                loyalty(d, -8, Ability::new(vec![OPPONENT], both));
            },
        ),
        "Professor Dellian Fel" => s(
            &[
                "+2: You gain 3 life.",
                "0: You draw a card and lose 1 life.",
                "−3: Destroy target creature.",
                "−6: You get an emblem with \"Whenever you gain life, target opponent loses that much life.\"",
            ],
            |d, _| {
                loyalty(d, 2, Ability::new(vec![], vec![Effect::GainLife { who: Who::You, n: 3 }]));
                loyalty(d, 0, Ability::new(vec![], vec![draw(1), lose_one()]));
                loyalty(d, -3, Ability::new(vec![Filter::CREATURE], vec![Effect::Destroy(0)]));
                let n = d.triggers.len();
                on(d, When::Never, vec![OPPONENT], vec![Effect::LoseLifeAmount(Who::TargetPlayer(0))]);
                let kind = EmblemKind::Triggered { when: When::YouGainLife, trigger: n };
                loyalty(d, -6, Ability::new(vec![], vec![Effect::Emblem(kind)]));
            },
        ),
        // ---- Dice ----
        "Ancient Gold Dragon" => s(
            &["Whenever this creature deals combat damage to a player, roll a d20. You create a number of 1/1 blue Faerie Dragon creature tokens with flying equal to the result."],
            |d, t| {
                let roll = SubsystemEffect::RollDieCreateTokens { sides: 20, def: t.faerie_dragon };
                on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::Subsystem(roll)]);
            },
        ),
        "Ancient Copper Dragon" => s(
            &["Whenever this creature deals combat damage to a player, roll a d20. You create a number of Treasure tokens equal to the result."],
            |d, t| {
                let roll = SubsystemEffect::RollDieCreateTokens { sides: 20, def: t.treasure };
                on(d, When::DealsCombatDamageToPlayer, vec![], vec![sub(roll)]);
            },
        ),
        // ---- Coin flips (CR 705) ----
        "Mana Crypt" => s(
            &[
                "At the beginning of your upkeep, flip a coin. If you lose the flip, this artifact deals 3 damage to you.",
                "{T}: Add {C}{C}.",
            ],
            |d, _| {
                let flip = sub(SubsystemEffect::FlipCoinLoseDamageYou(3));
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![flip]);
                d.mana = Some(ManaSource {
                    colorless: true,
                    ..ManaSource::default()
                });
                d.mana_amount = 2;
            },
        ),
        // ---- The Ring tempts you (CR 701.54: `SubsystemEffect::RingTempts`) ----
        "Fiery Inscription" => s(
            &[
                "When this enchantment enters, the Ring tempts you.",
                "Whenever you cast an instant or sorcery spell, this enchantment deals 2 damage to each opponent.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![sub(SubsystemEffect::RingTempts)]);
                on(d, When::YouCastInstantOrSorcery, vec![], vec![damage(2, Recipient::EachOpponent)]);
            },
        ),
        _ => None,
    }
}

pub const PG_D4_SCRIPTED: &[&str] = &[
    // Split cards
    "Boom // Bust",
    "Fire // Ice",
    // Dungeons
    "Hama Pashar, Ruin Seeker",
    "Dungeon Delver",
    "Barrowin of Clan Undurr",
    "Radiant Solar",
    "Nadaar, Selfless Paladin",
    "Yuan-Ti Malison",
    "Midnight Pathlighter",
    "Dungeon Descent",
    // The initiative
    "White Plume Adventurer",
    "Imoen, Mystic Trickster",
    "Safana, Calimport Cutthroat",
    // Emblems
    "Lolth, Spider Queen",
    "Ob Nixilis Reignited",
    "Professor Dellian Fel",
    // Dice
    "Ancient Gold Dragon",
    "Ancient Copper Dragon",
    // Coin flips
    "Mana Crypt",
    // The Ring
    "Fiery Inscription",
];
