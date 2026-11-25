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
        // ---- lane C round 3, batch 5 ----
        "Ureni, the Song Unending" => s(
            &[
                "Flying, protection from white and from black",
                "When Ureni enters, it deals X damage divided as you choose among any number of target creatures and/or planeswalkers your opponents control, where X is the number of lands you control.",
            ],
            |d, _| {
                d.keywords = d.keywords.with(Keywords::FLYING);
                static_on(d, Scope::This, false, Keywords::NONE);
                d.statics.last_mut().expect("just added").protection = [true, false, true, false, false];
                let theirs = Filter { controller: Controller::Opponent, ..CREATURE_OR_PLANESWALKER };
                // "Any number": a slot per possible target; each needs at least 1 of X.
                let mut a = up_to(vec![theirs; 30], vec![Effect::DividedDamage]);
                a.divide_lands = true;
                d.triggers.push(Trigger { when: When::Enters, ability: a });
            },
        ),
        "Sidisi, Regent of the Mire" => s(
            &["{T}, Sacrifice a creature you control with mana value X other than Sidisi: Return target creature card with mana value X plus 1 from your graveyard to the battlefield. Activate only as a sorcery."],
            |d, _| {
                let card = Filter { mana_value_is_x: true, ..graveyard_cards(true, Controller::You) };
                let n = activated(d, "", true, vec![card], vec![Effect::Reanimate(0)]);
                let a = &mut d.activated[n];
                a.sacrifice_mv_x = true;
                a.sorcery = true;
            },
        ),
        "Dragonbroods' Relic" => s(
            &[
                "{T}, Tap an untapped creature you control: Add one mana of any color.",
                "{3}{W}{U}{B}{R}{G}, Sacrifice this artifact: Create a 4/4 Dragon creature token named Reliquary Dragon that's all colors. It has flying, lifelink, and \"When this token enters, it deals 3 damage to any target.\" Activate only as a sorcery.",
            ],
            |d, t| {
                d.mana = Some(ManaSource::of(&Color::ALL));
                d.mana_taps_creature = true;
                let n = activated(d, "{3}{W}{U}{B}{R}{G}", false, vec![], vec![Effect::CreateToken { who: Who::You, def: t.reliquary_dragon, n: 1 }]);
                let a = &mut d.activated[n];
                a.sacrifice_self = true;
                a.sorcery = true;
            },
        ),
        "All-Out Assault" => s(
            &[
                "Creatures you control get +1/+1 and have deathtouch.",
                "When this enchantment enters, if it's your main phase, there is an additional combat phase after this phase followed by an additional main phase. When you next attack this turn, untap each creature you control.",
            ],
            |d, _| {
                static_on(d, Scope::YourCreatures, false, Keywords::DEATHTOUCH);
                let st = d.statics.last_mut().expect("just added");
                st.power = 1;
                st.toughness = 1;
                conditional(
                    d,
                    When::Enters,
                    Condition::YourMainPhase,
                    vec![],
                    vec![Effect::AdditionalCombatAndMain, Effect::DelayedOnNextAttackThisTurn(1)],
                );
                on(d, When::Never, vec![], vec![Effect::UntapCreaturesYouControl]);
            },
        ),
        "Maelstrom of the Spirit Dragon" => s(
            &[
                "{T}: Add {C}.",
                "{T}: Add one mana of any color. Spend this mana only to cast a Dragon spell or an Omen spell.",
                "{4}, {T}, Sacrifice this land: Search your library for a Dragon card, reveal it, put it into your hand, then shuffle.",
            ],
            |d, _| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                d.any_color_for_dragon_or_omen = true;
                let n = activated(d, "{4}", true, vec![], vec![search(SearchFor::Subtype("Dragon"), SearchTo::Hand)]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Flamehold Grappler" => s(
            &["When this creature enters, copy the next spell you cast this turn when you cast it. You may choose new targets for the copy."],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::CopyNextSpellThisTurn(1)]);
                on(d, When::Never, vec![], vec![Effect::CopySpellNewTargets]);
            },
        ),
        "Ugin, Eye of the Storms" => s(
            &[
                "When you cast this spell, exile up to one target permanent that's one or more colors.",
                "Whenever you cast a colorless spell, exile up to one target permanent that's one or more colors.",
                "+2: You gain 3 life and draw a card.",
                "0: Add {C}{C}{C}.",
                "−11: Search your library for any number of colorless nonland cards, exile them, then shuffle. Until end of turn, you may cast those cards without paying their mana costs.",
            ],
            |d, _| {
                let colored = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, colored: true, ..Filter::ANY };
                d.triggers.push(Trigger { when: When::CastThisSpell, ability: up_to(vec![colored], vec![Effect::Exile(0)]) });
                d.triggers.push(Trigger { when: When::YouCastColorless, ability: up_to(vec![colored], vec![Effect::Exile(0)]) });
                loyalty(d, 2, Ability::new(vec![], vec![Effect::GainLife { who: Who::You, n: 3 }, draw(1)]));
                loyalty(d, 0, Ability::new(vec![], vec![Effect::AddColorless(3)]));
                let all = Effect::Search { what: SearchFor::ColorlessNonland, to: SearchTo::ExileCastFreeThisTurn, n: u8::MAX, kicked_n: None };
                loyalty(d, -11, Ability::new(vec![], vec![all]));
            },
        ),
        // ---- lane C round 3, batch 7: high-pick uncommons ----
        "Spectral Denial" => s(
            &[
                "This spell costs {1} less to cast for each creature you control with power 4 or greater.",
                "Counter target spell unless its controller pays {X}.",
            ],
            |d, _| {
                d.reduction = Some(Reduction::PerCreatureYouControlPowerAtLeast(4));
                spell(d, vec![SPELL], vec![Effect::CounterUnlessPaysX(0)]);
            },
        ),
        "Rally the Monastery" => s(
            &[
                "This spell costs {2} less to cast if you've cast another spell this turn.",
                "Choose one —",
                "• Create two 1/1 white Monk creature tokens with prowess.",
                "• Up to two target creatures you control each get +2/+2 until end of turn.",
                "• Destroy target creature with power 4 or greater.",
            ],
            |d, t| {
                d.reduction = Some(Reduction::IfCastAnotherSpell(2));
                let big = Filter { creature_min_power: Some(4), ..CREATURE };
                modes(
                    d,
                    vec![
                        Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.monk, n: 2 }]),
                        up_to(
                            vec![YOUR_CREATURE, YOUR_CREATURE],
                            vec![Effect::Pump { slot: 0, power: 2, toughness: 2 }, Effect::Pump { slot: 1, power: 2, toughness: 2 }],
                        ),
                        Ability::new(vec![big], vec![Effect::Destroy(0)]),
                    ],
                );
            },
        ),
        "Glacial Dragonhunt" => s(
            &[
                "Draw a card, then you may discard a card. When you discard a nonland card this way, Glacial Dragonhunt deals 3 damage to target creature.",
                "Harmonize {4}{U}{R}",
            ],
            |d, _| {
                spell(d, vec![], vec![draw(1), Effect::MayDiscardReflexiveIfNonland(0)]);
                on(d, When::Never, vec![CREATURE], vec![damage(3, Recipient::Target(0))]);
                harmonize(d, "{4}{U}{R}");
            },
        ),
        "Synchronized Charge" => s(
            &[
                "Distribute two +1/+1 counters among one or two target creatures you control. Creatures you control with counters on them gain vigilance and trample until end of turn.",
                "Harmonize {4}{G}",
            ],
            |d, _| {
                let mut a = up_to(
                    vec![YOUR_CREATURE, YOUR_CREATURE],
                    vec![Effect::DividedCounters, Effect::GrantYourCreaturesWithCounters(Keywords::VIGILANCE.with(Keywords::TRAMPLE))],
                );
                a.min_targets = Some(1);
                a.divide = 2;
                d.spell = Some(a);
                harmonize(d, "{4}{G}");
            },
        ),
        "Winternight Stories" => s(
            &["Draw three cards. Then discard two cards unless you discard a creature card.", "Harmonize {4}{U}"],
            |d, _| {
                spell(d, vec![], vec![draw(3), Effect::DiscardTwoUnlessCreature]);
                harmonize(d, "{4}{U}");
            },
        ),
        "Krumar Initiate" => s(
            &["{X}{B}, {T}, Pay X life: This creature endures X. Activate only as a sorcery."],
            |d, t| {
                let n = activated(d, "{X}{B}", true, vec![], vec![Effect::EndureX { spirit: t.spirit_endure }]);
                let a = &mut d.activated[n];
                a.life_x = true;
                a.sorcery = true;
            },
        ),
        "Sinkhole Surveyor" => s(
            &["Whenever this creature attacks, you lose 1 life and this creature endures 1."],
            |d, t| on(d, When::Attacks, vec![], vec![Effect::LoseLife { who: Who::You, n: 1 }, endure(1, t)]),
        ),
        "Qarsi Revenant" => s(
            &["Renew — {2}{B}, Exile this card from your graveyard: Put a flying counter, a deathtouch counter, and a lifelink counter on target creature. Activate only as a sorcery."],
            |d, _| {
                let k = Keywords::FLYING.with(Keywords::DEATHTOUCH).with(Keywords::LIFELINK);
                renew(d, "{2}{B}", vec![CREATURE], vec![Effect::KeywordCounter { slot: 0, keywords: k }]);
            },
        ),
        "Stadium Headliner" => s(
            &["Mobilize 1", "{1}{R}, Sacrifice this creature: It deals damage equal to the number of creatures you control to target creature."],
            |d, t| {
                mobilize(d, t, Some(1));
                let n = activated(d, "{1}{R}", false, vec![CREATURE], vec![Effect::DamageEqualCreaturesYouControl(0)]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Lasyd Prowler" => s(
            &[
                "When this creature enters, you may mill cards equal to the number of lands you control.",
                "Renew — {1}{G}, Exile this card from your graveyard: Put X +1/+1 counters on target creature, where X is the number of land cards in your graveyard. Activate only as a sorcery.",
            ],
            |d, _| {
                may(d, When::Enters, vec![], vec![Effect::MillLandsYouControl]);
                renew(d, "{1}{G}", vec![CREATURE], vec![Effect::CountersPerLandCardInGraveyard(0)]);
            },
        ),
        "Warden of the Grove" => s(
            &[
                "At the beginning of your end step, put a +1/+1 counter on this creature.",
                "Whenever another nontoken creature you control enters, it endures X, where X is the number of counters on this creature.",
            ],
            |d, t| {
                on(d, When::EndStep { your_turn_only: true }, vec![], vec![Effect::CountersSelf { n: 1 }]);
                let that = Filter { triggering: true, ..CREATURE };
                on(
                    d,
                    When::AnotherNontokenCreatureYouControlEnters,
                    vec![that],
                    vec![Effect::SlotEnduresSourceCounters { slot: 0, spirit: t.spirit_endure }],
                );
            },
        ),
        "Cori-Steel Cutter" => s(
            &[
                "Equipped creature gets +1/+1 and has trample and haste.",
                "Flurry — Whenever you cast your second spell each turn, create a 1/1 white Monk creature token with prowess. You may attach this Equipment to it.",
                "Equip {1}{R}",
            ],
            |d, t| {
                attached(d, false, 1, 1, Keywords::TRAMPLE.with(Keywords::HASTE));
                flurry(
                    d,
                    vec![],
                    vec![Effect::CreateToken { who: Who::You, def: t.monk, n: 1 }, Effect::MayContinue, Effect::AttachToCreated],
                );
                equip(d, "{1}{R}");
            },
        ),
        "Songcrafter Mage" => s(
            &["When this creature enters, target instant or sorcery card in your graveyard gains harmonize until end of turn. Its harmonize cost is equal to its mana cost."],
            |d, _| {
                let card = Filter { instant_or_sorcery: true, ..graveyard_cards(false, Controller::You) };
                on(d, When::Enters, vec![card], vec![Effect::GrantHarmonize(0)]);
            },
        ),
        // ---- lane C round 3, batch 8: the rest of Tarkir ----
        "Anafenza, Unyielding Lineage" => s(
            &["Whenever another nontoken creature you control dies, Anafenza endures 2."],
            |d, t| on(d, When::AnotherNontokenCreatureYouControlDies, vec![], vec![endure(2, t)]),
        ),
        "Voice of Victory" => s(&["Mobilize 2", "Your opponents can't cast spells during your turn."], |d, t| {
            mobilize(d, t, Some(2));
            d.opponents_cant_cast_your_turn = true;
        }),
        "Zurgo, Thunder's Decree" => s(
            &["Mobilize 2", "During your end step, Warrior tokens you control have \"This token can't be sacrificed.\""],
            |d, t| {
                mobilize(d, t, Some(2));
                d.warrior_tokens_unsacrificeable_end_step = true;
            },
        ),
        "The Sibsig Ceremony" => s(
            &[
                "Creature spells you cast cost {2} less to cast.",
                "Whenever a creature you control enters, if you cast it, destroy that creature, then create a 2/2 black Zombie Druid creature token.",
            ],
            |d, t| {
                d.creature_spell_discount = 2;
                let that = Filter { triggering: true, ..CREATURE };
                on(
                    d,
                    When::CastCreatureYouControlEnters,
                    vec![that],
                    vec![Effect::Destroy(0), Effect::CreateToken { who: Who::You, def: t.zombie_druid, n: 1 }],
                );
            },
        ),
        "Rite of Renewal" => s(
            &["Return up to two target permanent cards from your graveyard to your hand. Target player shuffles up to four target cards from their graveyard into their library. Exile Rite of Renewal."],
            |d, _| {
                // The cards are chosen as it resolves (a decision), not as targets.
                spell(
                    d,
                    vec![Filter::PLAYER],
                    vec![
                        Effect::ReturnUpToPermanentCardsToHand(2),
                        Effect::ShuffleUpToFromGraveyard { slot: 0, n: 4 },
                        Effect::ExileSourceSpell,
                    ],
                );
            },
        ),
        "Sarkhan, Dragon Ascendant" => s(
            &[
                "When Sarkhan enters, you may behold a Dragon. If you do, create a Treasure token.",
                "Whenever a Dragon you control enters, put a +1/+1 counter on Sarkhan. Until end of turn, Sarkhan becomes a Dragon in addition to his other types and gains flying.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::MayBeholdThen, Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }]);
                on(
                    d,
                    When::AnotherYouControlEntersWithSubtype("Dragon"),
                    vec![],
                    vec![
                        Effect::CountersSelf { n: 1 },
                        Effect::SubtypeUntilEotSelf("Dragon"),
                        Effect::GrantSelf { keywords: Keywords::FLYING },
                    ],
                );
            },
        ),
        "Mardu Siegebreaker" => s(
            &[
                "When this creature enters, exile up to one other target creature you control until this creature leaves the battlefield.",
                "Whenever this creature attacks, for each opponent, create a tapped token that's a copy of the exiled card attacking that opponent. At the beginning of your next end step, sacrifice those tokens.",
            ],
            |d, _| {
                let other = Filter { other: true, ..YOUR_CREATURE };
                d.triggers.push(Trigger { when: When::Enters, ability: up_to(vec![other], vec![Effect::ExileUntilSourceLeaves(0)]) });
                on(d, When::Attacks, vec![], vec![Effect::CopyLinkedExileAttacking]);
            },
        ),
        "Dragonfire Blade" => s(
            &[
                "Equipped creature gets +2/+2 and has hexproof from monocolored.",
                "Equip {4}. This ability costs {1} less to activate for each color of the creature it targets.",
            ],
            |d, _| {
                attached(d, false, 2, 2, Keywords::NONE);
                d.equipped_hexproof_from_monocolored = true;
                equip(d, "{4}");
                d.activated.last_mut().expect("equip").less_per_target_color = true;
            },
        ),
        "Shiko, Paragon of the Way" => s(
            &["When Shiko enters, exile target nonland card with mana value 3 or less from your graveyard. Copy it, then you may cast the copy without paying its mana cost."],
            |d, _| {
                let card = Filter { nonland_cards: true, max_mana_value: Some(3), ..graveyard_cards(false, Controller::You) };
                on(d, When::Enters, vec![card], vec![Effect::ExileCastCopyFree(0)]);
            },
        ),
        "Emergent Ultimatum" => s(
            &["Search your library for up to three monocolored cards with different names and exile them. An opponent chooses one of those cards. Shuffle that card into your library. You may cast the other cards without paying their mana costs. Exile Emergent Ultimatum."],
            |d, _| spell(d, vec![], vec![Effect::SearchMonocoloredOpponentPicks, Effect::ExileSourceSpell]),
        ),
        "Rot-Curse Rakshasa" => s(
            &[
                "Decayed",
                "Renew — {X}{B}{B}, Exile this card from your graveyard: Put a decayed counter on each of X target creatures. Activate only as a sorcery.",
            ],
            |d, _| {
                d.decayed = true;
                // "Each of X target creatures": a slot per possible target, exactly X of them filled.
                let effects = (0..20).map(Effect::DecayedCounter).collect();
                renew(d, "{X}{B}{B}", vec![CREATURE; 20], effects);
                d.activated.last_mut().expect("renew").ability.targets_exactly_x = true;
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
    "Ureni, the Song Unending",
    "Sidisi, Regent of the Mire",
    "Dragonbroods' Relic",
    "All-Out Assault",
    "Maelstrom of the Spirit Dragon",
    "Flamehold Grappler",
    "Ugin, Eye of the Storms",
    "Spectral Denial",
    "Rally the Monastery",
    "Glacial Dragonhunt",
    "Synchronized Charge",
    "Winternight Stories",
    "Krumar Initiate",
    "Sinkhole Surveyor",
    "Qarsi Revenant",
    "Stadium Headliner",
    "Lasyd Prowler",
    "Warden of the Grove",
    "Cori-Steel Cutter",
    "Songcrafter Mage",
    "Anafenza, Unyielding Lineage",
    "Voice of Victory",
    "Zurgo, Thunder's Decree",
    "The Sibsig Ceremony",
    "Rite of Renewal",
    "Sarkhan, Dragon Ascendant",
    "Mardu Siegebreaker",
    "Dragonfire Blade",
    "Shiko, Paragon of the Way",
    "Emergent Ultimatum",
    "Rot-Curse Rakshasa",
];
