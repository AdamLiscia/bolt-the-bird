//! Tarkir: Dragonstorm, lane C: plain cards on existing engine rules (card scripts only; any
//! card needing engine-core work goes back to the main lane). Tests: tests/tdm_c.rs.
#![allow(unused_imports)]

use super::*;

const ENCHANTMENT: Filter = Filter {
    players: false,
    creatures: false,
    planeswalkers: false,
    enchantments: true,
    ..Filter::ANY
};

/// "As this enchantment enters, choose [A] or [B]." Mode 0 is the first clan named.
fn siege(d: &mut CardDef) {
    d.enter_modes = true;
}

/// A triggered ability that exists only in chosen mode `m`.
fn in_mode(d: &mut CardDef, m: u8, when: When, targets: Vec<Filter>, effects: Vec<Effect>) {
    conditional(d, when, Condition::ChosenMode(m), targets, effects);
}

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        "Summit Intimidator" => s(&["When this creature enters, target creature can't block this turn."], |d, _| {
            on(d, When::Enters, vec![CREATURE], vec![Effect::Grant { slot: 0, keywords: Keywords::CANT_BLOCK }])
        }),
        "Arashin Sunshield" => s(
            &["When this creature enters, exile up to two target cards from a single graveyard.", "{W}, {T}: Tap target creature."],
            |d, _| {
                let f = graveyard_cards(false, Controller::Any);
                let mut ability = up_to(vec![f, f], vec![Effect::Exile(0), Effect::Exile(1)]);
                ability.same_graveyard = true;
                d.triggers.push(Trigger { when: When::Enters, ability });
                activated(d, "{W}", true, vec![CREATURE], vec![Effect::Tap(0)]);
            },
        ),
        "Lightfoot Technique" => s(
            &["Put a +1/+1 counter on target creature. It gains flying and indestructible until end of turn."],
            |d, _| {
                let k = Keywords::FLYING.with(Keywords::INDESTRUCTIBLE);
                spell(d, vec![CREATURE], vec![Effect::Counters { slot: 0, n: 1 }, Effect::Grant { slot: 0, keywords: k }])
            },
        ),
        "Unburied Earthcarver" => s(&["{2}, Sacrifice another creature: Put a +1/+1 counter on this creature."], |d, _| {
            let n = activated(d, "{2}", false, vec![], vec![Effect::CountersSelf { n: 1 }]);
            d.activated[n].sacrifice_other_creature = true;
        }),
        "Sunset Strikemaster" => s(
            &["{T}: Add {R}.", "{2}{R}, {T}, Sacrifice this creature: It deals 6 damage to target creature with flying."],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::Red]));
                let flier = Filter { creature_needs_flying: true, ..CREATURE };
                let n = activated(d, "{2}{R}", true, vec![flier], vec![damage(6, Recipient::Target(0))]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Opulent Palace" => s(&["This land enters tapped.", "{T}: Add {B}, {G}, or {U}."], |d, _| {
            d.enters_tapped = true;
            d.mana = Some(ManaSource::of(&[Color::Black, Color::Green, Color::Blue]));
        }),
        "Jeskai Shrinekeeper" => s(
            &["Whenever this creature deals combat damage to a player, you gain 1 life and draw a card."],
            |d, _| on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::GainLife { who: Who::You, n: 1 }, draw(1)]),
        ),
        "War Effort" => s(
            &[
                "Creatures you control get +1/+0.",
                "Whenever you attack, create a 1/1 red Warrior creature token that's tapped and attacking. Sacrifice it at the beginning of the next end step.",
            ],
            |d, t| {
                static_on(d, Scope::YourCreatures, false, Keywords::NONE);
                d.statics.last_mut().expect("just added").power = 1;
                // Mobilize's effect: one Warrior, tapped and attacking, sacrificed at the next end step.
                on(d, When::YouAttack, vec![], vec![Effect::Mobilize { def: t.warrior, n: Some(1) }]);
            },
        ),
        "Wail of War" => s(
            &[
                "Choose one —",
                "• Creatures target opponent controls get -1/-1 until end of turn.",
                "• Return up to two target creature cards from your graveyard to your hand.",
            ],
            |d, _| {
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                let f = graveyard_cards(true, Controller::You);
                modes(
                    d,
                    vec![
                        Ability::new(vec![opp], vec![Effect::PumpAll { who: Who::TargetPlayer(0), power: -1, toughness: -1 }]),
                        up_to(vec![f, f], vec![Effect::Bounce(0), Effect::Bounce(1)]),
                    ],
                )
            },
        ),
        "Jeskai Brushmaster" => s(&["Prowess"], |d, _| {
            on(d, When::YouCastNoncreature, vec![], vec![Effect::PumpSelf { power: 1, toughness: 1 }])
        }),
        "Starry-Eyed Skyrider" => s(
            &[
                "Whenever this creature attacks, another target creature you control gains flying until end of turn.",
                "Attacking tokens you control have flying.",
            ],
            |d, _| {
                let other = Filter { other: true, ..YOUR_CREATURE };
                on(d, When::Attacks, vec![other], vec![Effect::Grant { slot: 0, keywords: Keywords::FLYING }]);
                static_on(d, Scope::YourAttackingTokens, false, Keywords::FLYING);
            },
        ),
        "Loxodon Battle Priest" => s(
            &["At the beginning of combat on your turn, put a +1/+1 counter on another target creature you control."],
            |d, _| {
                let other = Filter { other: true, ..YOUR_CREATURE };
                on(d, When::BeginCombatYourTurn, vec![other], vec![Effect::Counters { slot: 0, n: 1 }]);
            },
        ),
        "Fangkeeper's Familiar" => s(
            &[
                "When this creature enters, choose one —",
                "• You gain 3 life and surveil 3.",
                "• Destroy target enchantment.",
                "• Counter target creature spell.",
            ],
            |d, _| {
                let creature_spell = Filter { spell_creature: Some(true), ..SPELL };
                on_modal(
                    d,
                    When::Enters,
                    None,
                    vec![
                        Ability::new(vec![], vec![Effect::GainLife { who: Who::You, n: 3 }, Effect::Surveil { n: 3 }]),
                        Ability::new(vec![ENCHANTMENT], vec![Effect::Destroy(0)]),
                        Ability::new(vec![creature_spell], vec![Effect::Counter(0)]),
                    ],
                );
            },
        ),
        "Inevitable Defeat" => s(
            &["This spell can't be countered.", "Exile target nonland permanent. Its controller loses 3 life and you gain 3 life."],
            |d, _| {
                d.uncounterable = true;
                let nonland = Filter { controller: Controller::Any, ..OPP_NONLAND };
                // The life loss reads the controller before the exile moves the card; same result.
                spell(
                    d,
                    vec![nonland],
                    vec![
                        Effect::LoseLife { who: Who::ControllerOf(0), n: 3 },
                        Effect::Exile(0),
                        Effect::GainLife { who: Who::You, n: 3 },
                    ],
                );
            },
        ),
        "Perennation" => s(
            &["Return target permanent card from your graveyard to the battlefield with a hexproof counter and an indestructible counter on it."],
            |d, _| {
                let card = Filter { permanent_cards: true, ..graveyard_cards(false, Controller::You) };
                // Keyword counters are kept as keywords it has for as long as it's on the battlefield.
                let k = Keywords::HEXPROOF.with(Keywords::INDESTRUCTIBLE);
                spell(d, vec![card], vec![Effect::ReturnWith { slot: Some(0), counters: 0, subtype: None, keywords: k }]);
            },
        ),
        // ---- lane C round 3: divided targets, damaged-this-turn, sacrifice a token, Aura enters, defender, mass removal ----
        "Twin Bolt" => s(&["Twin Bolt deals 2 damage divided as you choose among one or two targets."], |d, _| {
            let mut a = up_to(vec![Filter::ANY, Filter::ANY], vec![Effect::DividedDamage]);
            a.min_targets = Some(1);
            a.divide = 2;
            d.spell = Some(a);
        }),
        "Unsparing Boltcaster" => s(
            &["When this creature enters, it deals 5 damage to target creature an opponent controls that was dealt damage this turn."],
            |d, _| {
                let hurt = Filter { was_dealt_damage: true, ..OPP_CREATURE };
                on(d, When::Enters, vec![hurt], vec![damage(5, Recipient::Target(0))]);
            },
        ),
        "Smile at Death" => s(
            &["At the beginning of your upkeep, return up to two target creature cards with power 2 or less from your graveyard to the battlefield. Put a +1/+1 counter on each of those creatures."],
            |d, _| {
                let f = Filter { creature_max_power: Some(2), ..graveyard_cards(true, Controller::You) };
                let ability = up_to(
                    vec![f, f],
                    vec![
                        Effect::Reanimate(0),
                        Effect::Reanimate(1),
                        Effect::Counters { slot: 0, n: 1 },
                        Effect::Counters { slot: 1, n: 1 },
                    ],
                );
                d.triggers.push(Trigger { when: When::Upkeep { your_turn_only: true }, ability });
            },
        ),
        "Hardened Tactician" => s(&["{1}, Sacrifice a token: Draw a card."], |d, _| {
            let n = activated(d, "{1}", false, vec![], vec![draw(1)]);
            d.activated[n].sacrifice_token = true;
        }),
        "Fire-Rim Form" => s(
            &[
                "Enchant creature",
                "When this Aura enters, enchanted creature gains first strike until end of turn.",
                "Enchanted creature gets +2/+0.",
            ],
            |d, _| {
                d.aura = true;
                d.spell = Some(Ability::new(vec![CREATURE], vec![]));
                on(d, When::Enters, vec![], vec![Effect::GrantAttached(Keywords::FIRST_STRIKE)]);
                attached(d, false, 2, 0, Keywords::NONE);
            },
        ),
        "Krotiq Nestguard" => s(&["{2}{G}: This creature can attack this turn as though it didn't have defender."], |d, _| {
            activated(d, "{2}{G}", false, vec![], vec![Effect::AttackDespiteDefenderSelf]);
        }),
        "Ruinous Ultimatum" => s(&["Destroy all nonland permanents your opponents control."], |d, _| {
            spell(d, vec![], vec![Effect::DestroyOpponentsNonland])
        }),
        "Death Begets Life" => s(
            &["Destroy all creatures and enchantments. Draw a card for each permanent destroyed this way."],
            |d, _| spell(d, vec![], vec![Effect::DestroyCreaturesAndEnchantmentsDraw]),
        ),
        "Dragonback Assault" => s(
            &[
                "When this enchantment enters, it deals 3 damage to each creature and each planeswalker.",
                "Landfall — Whenever a land you control enters, create a 4/4 red Dragon creature token with flying.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![damage(3, Recipient::EachCreatureAndPlaneswalker)]);
                on(d, When::LandYouControlEnters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.dragon, n: 1 }]);
            },
        ),
        "Windcrag Siege" => s(
            &[
                "As this enchantment enters, choose Mardu or Jeskai.",
                "• Mardu — If a creature attacking causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.",
                "• Jeskai — At the beginning of your upkeep, create a 1/1 red Goblin creature token. It gains lifelink and haste until end of turn.",
            ],
            |d, t| {
                siege(d);
                d.attack_triggers_again_in_mode = Some(0);
                in_mode(
                    d,
                    1,
                    When::Upkeep { your_turn_only: true },
                    vec![],
                    vec![
                        Effect::CreateToken { who: Who::You, def: t.goblin, n: 1 },
                        Effect::GrantCreated(Keywords::LIFELINK.with(Keywords::HASTE)),
                    ],
                );
            },
        ),
        "Frostcliff Siege" => s(
            &[
                "As this enchantment enters, choose Jeskai or Temur.",
                "• Jeskai — Whenever one or more creatures you control deal combat damage to a player, draw a card.",
                "• Temur — Creatures you control get +1/+0 and have trample and haste.",
            ],
            |d, _| {
                siege(d);
                in_mode(d, 0, When::YourCreaturesDealCombatDamageToPlayer, vec![], vec![draw(1)]);
                static_on(d, Scope::YourCreatures, false, Keywords::TRAMPLE.with(Keywords::HASTE));
                let st = d.statics.last_mut().expect("just added");
                st.power = 1;
                st.condition = Some(StaticCondition::ChosenMode(1));
            },
        ),
        "Hollowmurk Siege" => s(
            &[
                "As this enchantment enters, choose Sultai or Abzan.",
                "• Sultai — Whenever a counter is put on a creature you control, draw a card. This ability triggers only once each turn.",
                "• Abzan — Whenever you attack, put a +1/+1 counter on target attacking creature. It gains menace until end of turn.",
            ],
            |d, _| {
                siege(d);
                in_mode(d, 0, When::CounterPutOnYourCreatureOnceEachTurn, vec![], vec![draw(1)]);
                let attacking = Filter { in_combat: true, ..CREATURE };
                in_mode(
                    d,
                    1,
                    When::YouAttack,
                    vec![attacking],
                    vec![Effect::Counters { slot: 0, n: 1 }, Effect::Grant { slot: 0, keywords: Keywords::MENACE }],
                );
            },
        ),
        "Glacierwood Siege" => s(
            &[
                "As this enchantment enters, choose Temur or Sultai.",
                "• Temur — Whenever you cast an instant or sorcery spell, target player mills four cards.",
                "• Sultai — You may play lands from your graveyard.",
            ],
            |d, _| {
                siege(d);
                in_mode(d, 0, When::YouCastInstantOrSorcery, vec![Filter::PLAYER], vec![Effect::Mill { who: Who::TargetPlayer(0), n: 4 }]);
                d.graveyard_lands_in_mode = Some(1);
            },
        ),
        "Barrensteppe Siege" => s(
            &[
                "As this enchantment enters, choose Abzan or Mardu.",
                "• Abzan — At the beginning of your end step, put a +1/+1 counter on each creature you control.",
                "• Mardu — At the beginning of your end step, if a creature died under your control this turn, each opponent sacrifices a creature of their choice.",
            ],
            |d, _| {
                siege(d);
                let end = When::EndStep { your_turn_only: true };
                in_mode(d, 0, end, vec![], vec![Effect::CountersAll { who: Who::You, n: 1 }]);
                let died = Condition::Both(Box::new(Condition::ChosenMode(1)), Box::new(Condition::CreatureDiedUnderYourControlThisTurn));
                conditional(d, end, died, vec![], vec![Effect::SacrificeCreatures { who: Who::EachOpponent, n: 1 }]);
            },
        ),
        // ---- lane C round 3, batch 3: hands, graveyards, power-scaled effects, Dragon-only rules ----
        "Aggressive Negotiations" => s(
            &["Target opponent reveals their hand. You choose a nonland card from it and exile that card. Put a +1/+1 counter on up to one target creature you control."],
            |d, _| {
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                let mut a = Ability::new(vec![opp, YOUR_CREATURE], vec![Effect::ChooseExileFromHand { slot: 0 }, Effect::Counters { slot: 1, n: 1 }]);
                a.min_targets = Some(1); // the creature is "up to one"
                d.spell = Some(a);
            },
        ),
        "Sunpearl Kirin" => s(
            &["When this creature enters, return up to one other target nonland permanent you control to its owner's hand. If it was a token, draw a card."],
            |d, _| {
                let mine = Filter { other: true, controller: Controller::You, ..OPP_NONLAND };
                d.triggers.push(Trigger { when: When::Enters, ability: up_to(vec![mine], vec![Effect::BounceDrawIfToken(0)]) });
            },
        ),
        "Veteran Ice Climber" => s(
            &[
                "This creature can't be blocked.",
                "Whenever this creature attacks, up to one target player mills cards equal to this creature's power.",
            ],
            |d, _| {
                static_on(d, Scope::This, false, Keywords::UNBLOCKABLE);
                d.triggers.push(Trigger { when: When::Attacks, ability: up_to(vec![Filter::PLAYER], vec![Effect::MillSourcePower(0)]) });
            },
        ),
        "Kishla Trawlers" => s(
            &["When this creature enters, you may exile a creature card from your graveyard. When you do, return target instant or sorcery card from your graveyard to your hand."],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::MayExileFromGraveyard { n: 1, creature_only: true }, Effect::Reflexive(1)]);
                let spell_card = Filter { instant_or_sorcery: true, ..graveyard_cards(false, Controller::You) };
                on(d, When::Never, vec![spell_card], vec![Effect::Bounce(0)]);
            },
        ),
        "Rainveil Rejuvenator" => s(
            &["When this creature enters, you may mill three cards.", "{T}: Add an amount of {G} equal to this creature's power."],
            |d, _| {
                may(d, When::Enters, vec![], vec![Effect::Mill { who: Who::You, n: 3 }]);
                d.mana = Some(ManaSource::of(&[Color::Green]));
                d.mana_equal_power = true;
            },
        ),
        "Aegis Sculptor" => s(
            &["Ward {2}", "At the beginning of your upkeep, you may exile two cards from your graveyard. If you do, put a +1/+1 counter on this creature."],
            |d, _| {
                d.ward = ManaCost::parse("{2}");
                on(
                    d,
                    When::Upkeep { your_turn_only: true },
                    vec![],
                    vec![Effect::MayExileFromGraveyard { n: 2, creature_only: false }, Effect::CountersSelf { n: 1 }],
                );
            },
        ),
        "Stormbeacon Blade" => s(
            &[
                "Equipped creature gets +3/+0.",
                "Whenever equipped creature attacks, draw a card if you control three or more attacking creatures.",
                "Equip {2}",
            ],
            |d, _| {
                attached(d, false, 3, 0, Keywords::NONE);
                on(d, When::EquippedAttacks, vec![], vec![Effect::GateAttackingAtLeast(3), draw(1)]);
                equip(d, "{2}");
            },
        ),
        "Dragonologist" => s(
            &[
                "When this creature enters, look at the top six cards of your library. You may reveal an instant, sorcery, or Dragon card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
                "Untapped Dragons you control have hexproof.",
            ],
            |d, _| {
                let look = Effect::LookAtTop {
                    n: 6,
                    pick: Pick::InstantSorceryOrSubtype("Dragon"),
                    chosen_to: LookTo::Hand,
                    rest_to: LookRest::BottomRandom,
                };
                on(d, When::Enters, vec![], vec![look]);
                static_on(d, Scope::YourUntappedSubtype("Dragon"), false, Keywords::HEXPROOF);
            },
        ),
        "Furious Forebear" => s(
            &["Whenever a creature you control dies while this card is in your graveyard, you may pay {1}{W}. If you do, return this card from your graveyard to your hand."],
            |d, _| {
                let cost = ManaCost::parse("{1}{W}").expect("cost");
                on(d, When::CreatureYouControlDiesFromGraveyard, vec![], vec![Effect::MayPay(cost), Effect::ReturnSelfToHand]);
            },
        ),
        "Mox Jasper" => s(&["{T}: Add one mana of any color. Activate only if you control a Dragon."], |d, _| {
            d.mana = Some(ManaSource::of(&Color::ALL));
            d.mana_only_if_you_control = Some("Dragon");
        }),
        "Dracogenesis" => s(&["You may cast Dragon spells without paying their mana costs."], |d, _| {
            d.free_subtype_spells = Some("Dragon");
        }),
        // ---- lane C round 3, batch 4 ----
        "Yathan Tombguard" => s(
            &["Whenever a creature you control with a counter on it deals combat damage to a player, you draw a card and you lose 1 life."],
            |d, _| {
                on(
                    d,
                    When::YourCreatureWithCounterDealsCombatDamageToPlayer,
                    vec![],
                    vec![draw(1), Effect::LoseLife { who: Who::You, n: 1 }],
                )
            },
        ),
        "Armament Dragon" => s(
            &["When this creature enters, distribute three +1/+1 counters among one, two, or three target creatures you control."],
            |d, _| {
                let mut a = up_to(vec![YOUR_CREATURE; 3], vec![Effect::DividedCounters]);
                a.min_targets = Some(1);
                a.divide = 3;
                d.triggers.push(Trigger { when: When::Enters, ability: a });
            },
        ),
        "Genesis Ultimatum" => s(
            &["Look at the top five cards of your library. Put any number of permanent cards from among them onto the battlefield and the rest into your hand. Exile Genesis Ultimatum."],
            |d, _| {
                let look = Effect::TopCardsOntoBattlefield { n: 5, up_to: 0, noncreature_nonland: false, max_mv: None, rest_to_hand: true };
                spell(d, vec![], vec![look, Effect::ExileSourceSpell]);
            },
        ),
        "United Battlefront" => s(
            &["Look at the top seven cards of your library. Put up to two noncreature, nonland permanent cards with mana value 3 or less from among them onto the battlefield. Put the rest on the bottom of your library in a random order."],
            |d, _| {
                let look = Effect::TopCardsOntoBattlefield { n: 7, up_to: 2, noncreature_nonland: true, max_mv: Some(3), rest_to_hand: false };
                spell(d, vec![], vec![look]);
            },
        ),
        "Eerie Ultimatum" => s(
            &["Return any number of permanent cards with different names from your graveyard to the battlefield."],
            |d, _| spell(d, vec![], vec![Effect::ReturnPermanentsDifferentNames]),
        ),
        "Tersa Lightshatter" => s(
            &[
                "When Tersa Lightshatter enters, discard up to two cards, then draw that many cards.",
                "Whenever Tersa Lightshatter attacks, if there are seven or more cards in your graveyard, exile a card at random from your graveyard. You may play that card this turn.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::DiscardUpToThenDrawThatMany(2)]);
                conditional(d, When::Attacks, Condition::Threshold, vec![], vec![Effect::ExileRandomFromGraveyardPlayable]);
            },
        ),
        "Surrak, Elusive Hunter" => s(
            &[
                "This spell can't be countered.",
                "Whenever a creature you control or a creature spell you control becomes the target of a spell or ability an opponent controls, draw a card.",
            ],
            |d, _| {
                d.uncounterable = true;
                on(d, When::YourCreatureTargetedByOpponent, vec![], vec![draw(1)]);
            },
        ),
        "Kotis, the Fangkeeper" => s(
            &["Whenever Kotis deals combat damage to a player, exile the top X cards of their library, where X is the amount of damage dealt. You may cast any number of spells with mana value X or less from among them without paying their mana costs."],
            |d, _| on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::ExileTopOfDamagedCastFree]),
        ),
        "Severance Priest" => s(
            &[
                "When this creature enters, target opponent reveals their hand. You may choose a nonland card from it. If you do, exile that card.",
                "When this creature leaves the battlefield, the exiled card's owner creates an X/X white Spirit creature token, where X is the mana value of the exiled card.",
            ],
            |d, t| {
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                on(d, When::Enters, vec![opp], vec![Effect::ChooseExileFromHandLinked { slot: 0 }]);
                on(d, When::LeavesBattlefield, vec![], vec![Effect::LinkedOwnerCreatesSpirit { spirit: t.spirit_endure }]);
            },
        ),
        _ => None,
    }
}

/// Lane C's scripted cards (checked against their text in `lib.rs`).
pub const TDM_C_SCRIPTED: &[&str] = &[
    "Summit Intimidator",
    "Arashin Sunshield",
    "Lightfoot Technique",
    "Unburied Earthcarver",
    "Sunset Strikemaster",
    "Opulent Palace",
    "Jeskai Shrinekeeper",
    "War Effort",
    "Wail of War",
    "Jeskai Brushmaster",
    "Starry-Eyed Skyrider",
    "Loxodon Battle Priest",
    "Fangkeeper's Familiar",
    "Inevitable Defeat",
    "Perennation",
    "Twin Bolt",
    "Unsparing Boltcaster",
    "Smile at Death",
    "Hardened Tactician",
    "Fire-Rim Form",
    "Krotiq Nestguard",
    "Ruinous Ultimatum",
    "Death Begets Life",
    "Dragonback Assault",
    "Windcrag Siege",
    "Frostcliff Siege",
    "Hollowmurk Siege",
    "Glacierwood Siege",
    "Barrensteppe Siege",
    "Aggressive Negotiations",
    "Sunpearl Kirin",
    "Veteran Ice Climber",
    "Kishla Trawlers",
    "Rainveil Rejuvenator",
    "Aegis Sculptor",
    "Stormbeacon Blade",
    "Dragonologist",
    "Furious Forebear",
    "Mox Jasper",
    "Dracogenesis",
    "Yathan Tombguard",
    "Armament Dragon",
    "Genesis Ultimatum",
    "United Battlefront",
    "Eerie Ultimatum",
    "Tersa Lightshatter",
    "Surrak, Elusive Hunter",
    "Kotis, the Fangkeeper",
    "Severance Priest",
];
