//! Playgroup decks (Nerd Leagues), lane D1 (the desktop): the cards for the deck in handoff/lanes/d1.json. Conventions:
//! docs/lane-brief.md. Card text: assets/PG_oracle.json (in the CMD pool). Tests: tests/pg_d1.rs.
#![allow(unused_imports)]

use super::*;
use bolt_core::effect::ManaCount;

/// Any creature card (a search with no mana value limit).
const ANY_CREATURE_CARD: SearchFor = SearchFor::CreatureMvAtLeast(0);

/// Exalted (CR 702.83): "Whenever a creature you control attacks alone, that creature gets +1/+1
/// until end of turn." Each instance triggers on your attack and checks for a lone attacker.
fn exalted(d: &mut CardDef) {
    on(d, When::YouAttack, vec![], vec![Effect::Exalted]);
}

/// "Enchant land", with "Whenever enchanted land is tapped for mana, its controller adds an
/// additional one mana of any color."
fn mana_aura(d: &mut CardDef) {
    d.aura = true;
    let land = Filter {
        players: false,
        creatures: false,
        planeswalkers: false,
        lands: true,
        ..Filter::ANY
    };
    d.spell = Some(Ability::new(vec![land], vec![]));
    d.mana_ext.enchanted_land_extra_any = true;
}

/// Extort (CR 702.101): "Whenever you cast a spell, you may pay {W/B}. If you do, each opponent
/// loses 1 life and you gain that much life."
fn extort(d: &mut CardDef) {
    let pay = Effect::MayPay(ManaCost::parse("{W/B}").expect("cost"));
    on(
        d,
        When::YouCast,
        vec![],
        vec![
            pay,
            Effect::LoseLife {
                who: Who::EachOpponent,
                n: 1,
            },
            Effect::GainLife { who: Who::You, n: 1 },
        ],
    );
}

/// "Evoke {cost}" (CR 702.74): an alternative cost, and sacrificed when it enters if paid.
fn evoke(d: &mut CardDef, cost: &str) {
    d.alt_cost = Some((ManaCost::parse(cost).expect("cost"), 0));
    d.mana_ext.evoke = true;
    on(d, When::Enters, vec![], vec![Effect::SacrificeIfEvoked]);
}

/// "Cycling {cost}": "{cost}, Discard this card: Draw a card." from your hand.
fn cycling(d: &mut CardDef, cost: &str) {
    let n = activated(d, cost, false, vec![], vec![draw(1)]);
    d.activated[n].from_hand_discard = true;
}

pub fn script(name: &str) -> Option<Script> {
    let s = |lines, build| Some(Script { lines, build });
    match name {
        // ---- Ygra, Eater of All (Grande) ----
        "Ashnod's Altar" => s(&["Sacrifice a creature: Add {C}{C}."], |d, _| {
            // A mana ability played on the stack, like Gilded Goose's: the automatic payer never
            // sacrifices creatures.
            let n = activated(d, "", false, vec![], vec![Effect::AddColorless(2)]);
            d.activated[n].sacrifice_other_creature = true; // an artifact: "another" is any creature
        }),
        "Worldly Tutor" => s(&["Search your library for a creature card, reveal it, then shuffle and put the card on top."], |d, _| {
            spell(d, vec![], vec![search(ANY_CREATURE_CARD, SearchTo::Top)])
        }),
        "Warren Soultrader" => s(&["Pay 1 life, Sacrifice another creature: Create a Treasure token."], |d, t| {
            let n = activated(d, "", false, vec![], vec![Effect::CreateToken { who: Who::You, def: t.treasure, n: 1 }]);
            d.activated[n].life = 1;
            d.activated[n].sacrifice_other_creature = true;
        }),
        "Blood Artist" => s(&["Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life."], |d, _| {
            let drain = vec![Effect::LoseLife { who: Who::TargetPlayer(0), n: 1 }, Effect::GainLife { who: Who::You, n: 1 }];
            on(d, When::Dies, vec![Filter::PLAYER], drain.clone());
            on(d, When::AnotherCreatureDies, vec![Filter::PLAYER], drain);
        }),
        "Night's Whisper" => s(&["You draw two cards and lose 2 life."], |d, _| {
            spell(d, vec![], vec![draw(2), Effect::LoseLife { who: Who::You, n: 2 }])
        }),
        "Dictate of Erebos" => s(&["Whenever a creature you control dies, each opponent sacrifices a creature of their choice."], |d, _| {
            on(d, When::CreatureYouControlDies, vec![], vec![Effect::SacrificeCreatures { who: Who::EachOpponent, n: 1 }])
        }),
        "Phyrexian Altar" => s(&["Sacrifice a creature: Add one mana of any color."], |d, _| {
            // On the stack, like Ashnod's Altar.
            let n = activated(d, "", false, vec![], vec![Effect::AddManaAnyColor]);
            d.activated[n].sacrifice_other_creature = true;
        }),
        "Sign in Blood" => s(&["Target player draws two cards and loses 2 life."], |d, _| {
            spell(
                d,
                vec![Filter::PLAYER],
                vec![Effect::Draw { who: Who::TargetPlayer(0), n: 2 }, Effect::LoseLife { who: Who::TargetPlayer(0), n: 2 }],
            )
        }),
        "Viscera Seer" => s(&["Sacrifice a creature: Scry 1."], |d, _| {
            let n = activated(d, "", false, vec![], vec![Effect::Scry { n: 1 }]);
            d.activated[n].sacrifice_creature = true;
        }),
        "Blasting Station" => s(
            &["{T}, Sacrifice a creature: This artifact deals 1 damage to any target.", "Whenever a creature enters, you may untap this artifact."],
            |d, _| {
                let n = activated(d, "", true, vec![Filter::ANY], vec![damage(1, Recipient::Target(0))]);
                d.activated[n].sacrifice_other_creature = true;
                // "A creature": one of yours or one of the opponent's.
                may(d, When::AnotherCreatureYouControlEnters, vec![], vec![Effect::UntapSelf]);
                may(d, When::CreatureOpponentControlsEnters, vec![], vec![Effect::UntapSelf]);
            },
        ),
        "Carrion Feeder" => s(&["This creature can't block.", "Sacrifice a creature: Put a +1/+1 counter on this creature."], |d, _| {
            d.cant_block = true;
            let n = activated(d, "", false, vec![], vec![Effect::CountersSelf { n: 1 }]);
            d.activated[n].sacrifice_creature = true;
        }),
        "Darkbore Pathway // Slitherbore Pathway" => s(&["{T}: Add {B}.", "Back: {T}: Add {G}."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Black]));
            d.modal_back = true;
            d.back.as_mut().expect("Slitherbore Pathway").mana = Some(ManaSource::of(&[Color::Green]));
        }),
        "Fyndhorn Elves" => s(&["{T}: Add {G}."], |d, _| d.mana = Some(ManaSource::of(&[Color::Green]))),
        "Krosan Grip" => s(&["Split second", "Destroy target artifact or enchantment."], |d, _| {
            d.split_second = true;
            spell(d, vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::Destroy(0)]);
        }),
        "Abrupt Decay" => s(&["This spell can't be countered.", "Destroy target nonland permanent with mana value 3 or less."], |d, _| {
            d.uncounterable = true;
            let f = Filter { controller: Controller::Any, max_mana_value: Some(3), ..OPP_NONLAND };
            spell(d, vec![f], vec![Effect::Destroy(0)]);
        }),
        "Elves of Deep Shadow" => s(&["{T}: Add {B}. This creature deals 1 damage to you."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Black]));
            d.mana_tap_damage = 1;
        }),
        // ---- Raggadragga, Goreguts Boss (Adam) ----
        "Beast Whisperer" => s(&["Whenever you cast a creature spell, draw a card."], |d, _| {
            on(d, When::YouCastCreature, vec![], vec![draw(1)])
        }),
        "Staff of Domination" => s(
            &[
                "{1}: Untap this artifact.",
                "{2}, {T}: You gain 1 life.",
                "{3}, {T}: Untap target creature.",
                "{4}, {T}: Tap target creature.",
                "{5}, {T}: Draw a card.",
            ],
            |d, _| {
                activated(d, "{1}", false, vec![], vec![Effect::UntapSelf]);
                activated(d, "{2}", true, vec![], vec![Effect::GainLife { who: Who::You, n: 1 }]);
                activated(d, "{3}", true, vec![CREATURE], vec![Effect::UntapTarget(0)]);
                activated(d, "{4}", true, vec![CREATURE], vec![Effect::Tap(0)]);
                activated(d, "{5}", true, vec![], vec![draw(1)]);
            },
        ),
        "Fires of Yavimaya" => s(
            &["Creatures you control have haste.", "Sacrifice this enchantment: Target creature gets +2/+2 until end of turn."],
            |d, _| {
                static_on(d, Scope::YourCreatures, false, Keywords::HASTE);
                let n = activated(d, "", false, vec![CREATURE], vec![Effect::Pump { slot: 0, power: 2, toughness: 2 }]);
                d.activated[n].sacrifice_self = true;
            },
        ),
        "Wirewood Lodge" => s(&["{T}: Add {C}.", "{G}, {T}: Untap target Elf."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            // Elves are creatures (an Elf can only be a creature or a kindred card).
            let elf = Filter { subtype: Some("Elf"), ..CREATURE };
            activated(d, "{G}", true, vec![elf], vec![Effect::UntapTarget(0)]);
        }),
        "Boreal Druid" => s(&["{T}: Add {C}."], |d, _| d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() })),
        "Cragcrown Pathway // Timbercrown Pathway" => s(&["{T}: Add {R}.", "Back: {T}: Add {G}."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Red]));
            d.modal_back = true;
            d.back.as_mut().expect("Timbercrown Pathway").mana = Some(ManaSource::of(&[Color::Green]));
        }),
        "Marwyn, the Nurturer" => s(
            &["Whenever another Elf you control enters, put a +1/+1 counter on Marwyn.", "{T}: Add an amount of {G} equal to Marwyn's power."],
            |d, _| {
                on(d, When::AnotherYouControlEntersWithSubtype("Elf"), vec![], vec![Effect::CountersSelf { n: 1 }]);
                d.mana = Some(ManaSource::of(&[Color::Green]));
                d.mana_equal_power = true;
            },
        ),
        "Nature's Claim" => s(&["Destroy target artifact or enchantment. Its controller gains 4 life."], |d, _| {
            spell(d, vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::Destroy(0), Effect::GainLife { who: Who::ControllerOf(0), n: 4 }])
        }),
        "Viridian Joiner" => s(&["{T}: Add an amount of {G} equal to this creature's power."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Green]));
            d.mana_equal_power = true;
        }),
        "Vitalize" => s(&["Untap all creatures you control."], |d, _| spell(d, vec![], vec![Effect::UntapCreaturesYouControl])),
        // ---- Atla Palani, Nest Tender (RuricMo) ----
        "Aura Shards" => s(&["Whenever a creature you control enters, you may destroy target artifact or enchantment."], |d, _| {
            // An enchantment: "a creature you control" is always another permanent.
            may(d, When::AnotherCreatureYouControlEnters, vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::Destroy(0)])
        }),
        "Impact Tremors" => s(&["Whenever a creature you control enters, this enchantment deals 1 damage to each opponent."], |d, _| {
            on(d, When::AnotherCreatureYouControlEnters, vec![], vec![damage(1, Recipient::EachOpponent)])
        }),
        "Goblin Bombardment" => s(&["Sacrifice a creature: This enchantment deals 1 damage to any target."], |d, _| {
            let n = activated(d, "", false, vec![Filter::ANY], vec![damage(1, Recipient::Target(0))]);
            d.activated[n].sacrifice_other_creature = true; // an enchantment: any creature
        }),
        "High Market" => s(&["{T}: Add {C}.", "{T}, Sacrifice a creature: You gain 1 life."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            let n = activated(d, "", true, vec![], vec![Effect::GainLife { who: Who::You, n: 1 }]);
            d.activated[n].sacrifice_other_creature = true; // a land: any creature
        }),
        "Jetmir's Garden" => s(&["This land enters tapped.", "Cycling {3}"], |d, _| {
            d.enters_tapped = true;
            d.mana = Some(ManaSource::of(&[Color::Red, Color::Green, Color::White]));
            cycling(d, "{3}");
        }),
        "Jungle Shrine" => s(&["This land enters tapped.", "{T}: Add {R}, {G}, or {W}."], |d, _| {
            d.enters_tapped = true;
            d.mana = Some(ManaSource::of(&[Color::Red, Color::Green, Color::White]));
        }),
        "Fanatical Devotion" => s(&["Sacrifice a creature: Regenerate target creature."], |d, _| {
            let n = activated(d, "", false, vec![CREATURE], vec![Effect::Regenerate(0)]);
            d.activated[n].sacrifice_other_creature = true;
        }),
        "Samut, Voice of Dissent" => s(&["Other creatures you control have haste.", "{W}, {T}: Untap another target creature."], |d, _| {
            static_on(d, Scope::OtherYourCreatures, false, Keywords::HASTE);
            activated(d, "{W}", true, vec![Filter { other: true, ..CREATURE }], vec![Effect::UntapTarget(0)]);
        }),
        "Sheltering Landscape" => s(
            &[
                "{T}: Add {C}.",
                "{T}, Sacrifice this land: Search your library for a basic Mountain, Forest, or Plains card, put it onto the battlefield tapped, then shuffle.",
                "Cycling {R}{G}{W}",
            ],
            |d, _| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                let what = SearchFor::LandTypes { types: land_types(&["Mountain", "Forest", "Plains"]), basic: true };
                let n = activated(d, "", true, vec![], vec![search(what, SearchTo::Battlefield { tapped: true })]);
                d.activated[n].sacrifice_self = true;
                cycling(d, "{R}{G}{W}");
            },
        ),
        // ---- Karador, Ghost Chieftain (Grande) ----
        "Wood Elves" => s(
            &["When this creature enters, search your library for a Forest card, put that card onto the battlefield, then shuffle."],
            |d, _| {
                let forest = SearchFor::LandTypes { types: land_types(&["Forest"]), basic: false };
                on(d, When::Enters, vec![], vec![search(forest, SearchTo::Battlefield { tapped: false })]);
            },
        ),
        "Acidic Slime" => s(&["When this creature enters, destroy target artifact, enchantment, or land."], |d, _| {
            let f = Filter { lands: true, ..ARTIFACT_OR_ENCHANTMENT };
            on(d, When::Enters, vec![f], vec![Effect::Destroy(0)]);
        }),
        "Eladamri's Call" => s(&["Search your library for a creature card, reveal that card, put it into your hand, then shuffle."], |d, _| {
            spell(d, vec![], vec![search(ANY_CREATURE_CARD, SearchTo::Hand)])
        }),
        "Avacyn's Pilgrim" => s(&["{T}: Add {W}."], |d, _| d.mana = Some(ManaSource::of(&[Color::White]))),
        "Utter End" => s(&["Exile target nonland permanent."], |d, _| {
            spell(d, vec![Filter { controller: Controller::Any, ..OPP_NONLAND }], vec![Effect::Exile(0)])
        }),
        "Wight of the Reliquary" => s(
            &[
                "This creature gets +1/+1 for each creature card in your graveyard.",
                "{T}, Sacrifice another creature: Search your library for a land card, put it onto the battlefield tapped, then shuffle.",
            ],
            |d, _| {
                d.pt_per_graveyard_creature = true;
                let n = activated(d, "", true, vec![], vec![search(SearchFor::Land, SearchTo::Battlefield { tapped: true })]);
                d.activated[n].sacrifice_other_creature = true;
            },
        ),
        // ---- Ral, Monsoon Mage // Ral, Leyline Prodigy (Time) ----
        "Gilded Lotus" => s(&["{T}: Add three mana of any one color."], |d, _| {
            d.mana = Some(ManaSource::of(&Color::ALL));
            d.mana_amount = 3;
        }),
        "Ash Barrens" => s(&["{T}: Add {C}.", "Basic landcycling {1}"], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            let n = activated(d, "{1}", false, vec![], vec![search(SearchFor::BasicLand, SearchTo::Hand)]);
            d.activated[n].from_hand_discard = true;
        }),
        "Dismiss" => s(&["Counter target spell.", "Draw a card."], |d, _| spell(d, vec![SPELL], vec![Effect::Counter(0), draw(1)])),
        "Exclude" => s(&["Counter target creature spell.", "Draw a card."], |d, _| {
            spell(d, vec![Filter { spell_creature: Some(true), ..SPELL }], vec![Effect::Counter(0), draw(1)])
        }),
        "Gitaxian Probe" => s(&["Look at target player's hand.", "Draw a card."], |d, _| {
            // Looking at a hand is information only: no rules effect.
            spell(d, vec![Filter::PLAYER], vec![draw(1)])
        }),
        "Jwari Disruption // Jwari Ruins" => s(
            &["Counter target spell unless its controller pays {1}.", "Back: This land enters tapped.", "Back: {T}: Add {U}."],
            |d, _| {
                spell(d, vec![SPELL], vec![Effect::CounterUnlessPays(0, 1, 1)]);
                d.modal_back = true;
                let b = d.back.as_mut().expect("Jwari Ruins");
                b.enters_tapped = true;
                b.mana = Some(ManaSource::of(&[Color::Blue]));
            },
        ),
        "Lightning Bolt" => s(&["Lightning Bolt deals 3 damage to any target."], |d, _| {
            spell(d, vec![Filter::ANY], vec![damage(3, Recipient::Target(0))])
        }),
        "Neutralize" => s(&["Counter target spell.", "Cycling {2}"], |d, _| {
            spell(d, vec![SPELL], vec![Effect::Counter(0)]);
            cycling(d, "{2}");
        }),
        "Opportunity" => s(&["Target player draws four cards."], |d, _| {
            spell(d, vec![Filter::PLAYER], vec![Effect::Draw { who: Who::TargetPlayer(0), n: 4 }])
        }),
        "Prophetic Bolt" => s(
            &["Prophetic Bolt deals 4 damage to any target. Look at the top four cards of your library. Put one of those cards into your hand and the rest on the bottom of your library in any order."],
            |d, _| {
                let look = Effect::LookAtTop { n: 4, pick: Pick::Any, chosen_to: LookTo::Hand, rest_to: LookRest::BottomAnyOrderTakeOne };
                spell(d, vec![Filter::ANY], vec![damage(4, Recipient::Target(0)), look]);
            },
        ),
        "Rewind" => s(&["Counter target spell. Untap up to four lands."], |d, _| {
            // "Up to four lands": your own tapped ones, as Palinchron's.
            spell(d, vec![SPELL], vec![Effect::Counter(0), Effect::UntapLands(4)])
        }),
        // ---- Zacama, Primal Calamity (DonSpider) ----
        "Karametra, God of Harvests" => s(
            &[
                "As long as your devotion to green and white is less than seven, Karametra isn't a creature.",
                "Whenever you cast a creature spell, you may search your library for a Forest or Plains card, put it onto the battlefield tapped, then shuffle.",
            ],
            |d, _| {
                d.creature_unless_devotion = Some(([true, false, false, false, true], 7));
                let what = SearchFor::LandTypes { types: land_types(&["Forest", "Plains"]), basic: false };
                may(d, When::YouCastCreature, vec![], vec![search(what, SearchTo::Battlefield { tapped: true })]);
            },
        ),
        "Stony Silence" => s(&["Activated abilities of artifacts can't be activated."], |d, _| d.stops_artifact_abilities = true),
        "Sunhome, Fortress of the Legion" => s(&["{T}: Add {C}.", "{2}{R}{W}, {T}: Target creature gains double strike until end of turn."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            activated(d, "{2}{R}{W}", true, vec![CREATURE], vec![Effect::Grant { slot: 0, keywords: Keywords::DOUBLE_STRIKE }]);
        }),
        "Wilderness Reclamation" => s(&["At the beginning of your end step, untap all lands you control."], |d, _| {
            on(d, When::EndStep { your_turn_only: true }, vec![], vec![Effect::UntapLandsYouControl])
        }),
        "Zacama, Primal Calamity" => s(
            &[
                "When Zacama enters, if you cast it, untap all lands you control.",
                "{2}{R}: Zacama deals 3 damage to target creature.",
                "{2}{G}: Destroy target artifact or enchantment.",
                "{2}{W}: You gain 3 life.",
            ],
            |d, _| {
                on_if(d, Condition::CastIt, vec![], vec![Effect::UntapLandsYouControl]);
                activated(d, "{2}{R}", false, vec![CREATURE], vec![damage(3, Recipient::Target(0))]);
                activated(d, "{2}{G}", false, vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::Destroy(0)]);
                activated(d, "{2}{W}", false, vec![], vec![Effect::GainLife { who: Who::You, n: 3 }]);
            },
        ),
        // ---- core theme d1 (mana, costs and keywords): the cards its pieces unblock ----
        "Ignoble Hierarch" => s(&["Exalted", "{T}: Add {B}, {R}, or {G}."], |d, _| {
            exalted(d);
            d.mana = Some(ManaSource::of(&[Color::Black, Color::Red, Color::Green]));
        }),
        "Glen Elendra Archmage" => s(&["{U}, Sacrifice this creature: Counter target noncreature spell.", "Persist"], |d, _| {
            let n = activated(d, "{U}", false, vec![Filter { spell_creature: Some(false), ..SPELL }], vec![Effect::Counter(0)]);
            d.activated[n].sacrifice_self = true;
            on(d, When::Dies, vec![], vec![Effect::Persist]);
        }),
        "Gray Merchant of Asphodel" => s(
            &["When this creature enters, each opponent loses X life, where X is your devotion to black. You gain life equal to the life lost this way."],
            |d, _| on(d, When::Enters, vec![], vec![Effect::DrainDevotion([false, false, true, false, false])]),
        ),
        "Nykthos, Shrine to Nyx" => s(
            &["{T}: Add {C}.", "{2}, {T}: Choose a color. Add an amount of mana of that color equal to your devotion to that color."],
            |d, _| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                // The color is chosen as it's activated: one ability per color (as Koma's modes). It uses
                // the stack, like the Altars' mana.
                for c in Color::ALL {
                    activated(d, "{2}", true, vec![], vec![Effect::AddManaDevotion(c)]);
                }
            },
        ),
        "Cabal Coffers" => s(&["{2}, {T}: Add {B} for each Swamp you control."], |d, _| {
            let count = ManaCount::YouControlSubtype("Swamp");
            activated(d, "{2}", true, vec![], vec![Effect::AddManaPerCount { color: Color::Black, count }]);
        }),
        "Cabal Stronghold" => s(&["{T}: Add {C}.", "{3}, {T}: Add {B} for each basic Swamp you control."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            let count = ManaCount::YouControlBasicSubtype("Swamp");
            activated(d, "{3}", true, vec![], vec![Effect::AddManaPerCount { color: Color::Black, count }]);
        }),
        "Priest of Titania" => s(&["{T}: Add {G} for each Elf on the battlefield."], |d, _| {
            let count = ManaCount::OnBattlefieldSubtype("Elf");
            activated(d, "", true, vec![], vec![Effect::AddManaPerCount { color: Color::Green, count }]);
        }),
        "Heronblade Elite" => s(
            &[
                "Whenever another Human you control enters, put a +1/+1 counter on this creature.",
                "{T}: Add X mana of any one color, where X is this creature's power.",
            ],
            |d, _| {
                on(d, When::AnotherYouControlEntersWithSubtype("Human"), vec![], vec![Effect::CountersSelf { n: 1 }]);
                // One ability per color, on the stack (as Nykthos).
                for c in Color::ALL {
                    activated(d, "", true, vec![], vec![Effect::AddManaSourcePower(c)]);
                }
            },
        ),
        "Into the Story" => s(
            &["This spell costs {3} less to cast if an opponent has seven or more cards in their graveyard.", "Draw four cards."],
            |d, _| {
                d.reduction = Some(Reduction::IfOpponentGraveyardAtLeast(7, 3));
                spell(d, vec![], vec![draw(4)]);
            },
        ),
        "Phyrexian Tower" => s(&["{T}: Add {C}.", "{T}, Sacrifice a creature: Add {B}{B}."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            // On the stack, like the Altars' mana.
            let n = activated(d, "", true, vec![], vec![Effect::AddMana { color: Color::Black, n: 2 }]);
            d.activated[n].sacrifice_other_creature = true; // a land: any creature
        }),
        "Sunbaked Canyon" => s(&["{T}, Pay 1 life: Add {R} or {W}.", "{1}, {T}, Sacrifice this land: Draw a card."], |d, _| {
            // On the stack, like the Altars' mana.
            let n = activated(d, "", true, vec![], vec![Effect::AddManaEither(Color::Red, Color::White)]);
            d.activated[n].life = 1;
            let n = activated(d, "{1}", true, vec![], vec![draw(1)]);
            d.activated[n].sacrifice_self = true;
        }),
        "Wild Cantor" => s(&["Sacrifice this creature: Add one mana of any color."], |d, _| {
            // On the stack, like the Altars' mana.
            let n = activated(d, "", false, vec![], vec![Effect::AddManaAnyColor]);
            d.activated[n].sacrifice_self = true;
        }),
        "Kozilek, Butcher of Truth" => s(
            &[
                "When you cast this spell, draw four cards.",
                "Annihilator 4",
                "When Kozilek is put into a graveyard from anywhere, its owner shuffles their graveyard into their library.",
            ],
            |d, _| {
                on(d, When::CastThisSpell, vec![], vec![draw(4)]);
                on(d, When::Attacks, vec![], vec![Effect::SacrificePermanents { who: Who::EachOpponent, n: 4 }]);
                on(d, When::PutIntoGraveyardFromAnywhere, vec![], vec![Effect::OwnerShufflesGraveyardIntoLibrary]);
            },
        ),
        "Noise Marine" => s(
            &[
                "Cascade",
                "Sonic Blaster — When this creature enters, it deals damage equal to the number of spells you've cast this turn to any target.",
            ],
            |d, _| {
                on(d, When::CastThisSpell, vec![], vec![Effect::Cascade]);
                on(d, When::Enters, vec![Filter::ANY], vec![Effect::DamagePerSpellsCastThisTurn(0)]);
            },
        ),
        "Baleful Mastery" => s(
            &[
                "You may pay {1}{B} rather than pay this spell's mana cost.",
                "If the {1}{B} cost was paid, an opponent draws a card.",
                "Exile target creature or planeswalker.",
            ],
            |d, _| {
                d.alt_cost = Some((ManaCost::parse("{1}{B}").expect("cost"), 0)); // 0 creatures: always
                spell(d, vec![CREATURE_OR_PLANESWALKER], vec![Effect::OpponentDrawsIfAltCost(1), Effect::Exile(0)]);
            },
        ),
        "Nature's Will" => s(
            &["Whenever one or more creatures you control deal combat damage to a player, tap all lands that player controls and untap all lands you control."],
            |d, _| {
                // One opponent: "that player" is the opponent.
                on(d, When::YourCreaturesDealCombatDamageToPlayer, vec![], vec![Effect::TapOpponentsLands, Effect::UntapLandsYouControl]);
            },
        ),
        "Jeska's Will" => s(
            &[
                "Choose one. If you control a commander as you cast this spell, you may choose both instead.",
                "• Add {R} for each card in target opponent's hand.",
                "• Exile the top three cards of your library. You may play them this turn.",
            ],
            |d, _| {
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                let mana = Effect::AddManaPerHandSize { color: Color::Red, slot: 0 };
                let exile = Effect::ExileTopToPlayThisTurn(3);
                d.modes = vec![
                    Ability::new(vec![opp], vec![mana]),
                    Ability::new(vec![], vec![exile]),
                    Ability::new(vec![opp], vec![mana, exile]),
                ];
                d.last_mode_needs_commander = true;
            },
        ),
        "Karmic Guide" => s(
            &[
                "Flying, protection from black",
                "Echo {3}{W}{W}",
                "When this creature enters, return target creature card from your graveyard to the battlefield.",
            ],
            |d, _| {
                d.keywords = d.keywords.with(Keywords::FLYING);
                static_on(d, Scope::This, false, Keywords::NONE);
                d.statics.last_mut().expect("just added").protection = [false, false, true, false, false];
                let echo = ManaCost::parse("{3}{W}{W}").expect("cost");
                on(d, When::Upkeep { your_turn_only: true }, vec![], vec![Effect::Echo(echo)]);
                on(d, When::Enters, vec![graveyard_cards(true, Controller::You)], vec![Effect::Reanimate(0)]);
            },
        ),
        // ---- d1 core, batch 3: static cost reductions ----
        "Jukai Naturalist" => s(&["Enchantment spells you cast cost {1} less to cast."], |d, _| {
            d.mana_ext.enchantment_spell_discount = 1;
        }),
        "Bontu's Monument" => s(
            &[
                "Black creature spells you cast cost {1} less to cast.",
                "Whenever you cast a creature spell, each opponent loses 1 life and you gain 1 life.",
            ],
            |d, _| {
                d.mana_ext.black_creature_spell_discount = 1;
                on(
                    d,
                    When::YouCastCreature,
                    vec![],
                    vec![Effect::LoseLife { who: Who::EachOpponent, n: 1 }, Effect::GainLife { who: Who::You, n: 1 }],
                );
            },
        ),
        "Animar, Soul of Elements" => s(
            &[
                "Protection from white and from black",
                "Whenever you cast a creature spell, put a +1/+1 counter on Animar.",
                "Creature spells you cast cost {1} less to cast for each +1/+1 counter on Animar.",
            ],
            |d, _| {
                static_on(d, Scope::This, false, Keywords::NONE);
                d.statics.last_mut().expect("just added").protection = [true, false, true, false, false];
                on(d, When::YouCastCreature, vec![], vec![Effect::CountersSelf { n: 1 }]);
                d.mana_ext.creature_discount_per_counter = true;
            },
        ),
        // ---- d1 core, batch 3: {C} in costs ----
        "Rise of the Eldrazi" => s(
            &[
                "This spell can't be countered.",
                "Destroy target permanent. Target player draws four cards. Take an extra turn after this one.",
                "Exile Rise of the Eldrazi.",
            ],
            |d, _| {
                d.uncounterable = true;
                let permanent = Filter { players: false, creatures: false, planeswalkers: false, any_permanent: true, ..Filter::ANY };
                spell(
                    d,
                    vec![permanent, Filter::PLAYER],
                    vec![Effect::Destroy(0), Effect::Draw { who: Who::TargetPlayer(1), n: 4 }, Effect::ExtraTurn, Effect::ExileSourceSpell],
                );
            },
        ),
        // ---- d1 core, batch 3: fear, and lands that enter tapped unless ----
        "Shizo, Death's Storehouse" => s(
            &["{T}: Add {B}.", "{B}, {T}: Target legendary creature gains fear until end of turn."],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::Black]));
                let legendary = Filter { legendary_only: true, ..Filter::CREATURE };
                activated(d, "{B}", true, vec![legendary], vec![Effect::Grant { slot: 0, keywords: Keywords::FEAR }]);
            },
        ),
        "Haunted Ridge" => s(
            &["This land enters tapped unless you control two or more other lands.", "{T}: Add {B} or {R}."],
            |d, _| {
                d.mana_ext.enters_tapped_unless_other_lands = 2;
                d.mana = Some(ManaSource::of(&[Color::Black, Color::Red]));
            },
        ),
        "Overgrown Farmland" => s(
            &["This land enters tapped unless you control two or more other lands.", "{T}: Add {G} or {W}."],
            |d, _| {
                d.mana_ext.enters_tapped_unless_other_lands = 2;
                d.mana = Some(ManaSource::of(&[Color::Green, Color::White]));
            },
        ),
        // ---- d1 core, batch 3: pay X life ----
        "Toxic Deluge" => s(
            &["As an additional cost to cast this spell, pay X life.", "All creatures get -X/-X until end of turn."],
            |d, _| {
                d.mana_ext.pay_x_life = true;
                spell(d, vec![], vec![Effect::AllCreaturesGetMinusX]);
            },
        ),
        // ---- d1 core, batch 3: mana Auras and extra mana ----
        "Fertile Ground" => s(
            &["Enchant land", "Whenever enchanted land is tapped for mana, its controller adds an additional one mana of any color."],
            |d, _| mana_aura(d),
        ),
        "Trace of Abundance" => s(
            &[
                "Enchant land",
                "Enchanted land has shroud.",
                "Whenever enchanted land is tapped for mana, its controller adds an additional one mana of any color.",
            ],
            |d, _| {
                mana_aura(d);
                attached(d, false, 0, 0, Keywords::SHROUD);
            },
        ),
        "Buried in the Garden" => s(
            &[
                "Enchant land",
                "When this Aura enters, exile target nonland permanent you don't control until this Aura leaves the battlefield.",
                "Whenever enchanted land is tapped for mana, its controller adds an additional one mana of any color.",
            ],
            |d, _| {
                mana_aura(d);
                // "you don't control": the one opponent's.
                on(d, When::Enters, vec![OPP_NONLAND], vec![Effect::ExileUntilSourceLeaves(0)]);
            },
        ),
        "Crypt Ghast" => s(
            &["Extort", "Whenever you tap a Swamp for mana, add an additional {B}."],
            |d, _| {
                extort(d);
                d.mana_ext.your_subtype_lands_add = Some(("Swamp", Color::Black));
            },
        ),
        "Chromatic Lantern" => s(
            &["Lands you control have \"{T}: Add one mana of any color.\"", "{T}: Add one mana of any color."],
            |d, _| {
                d.mana_ext.lands_tap_any_color = true;
                d.mana = Some(ManaSource::of(&Color::ALL));
            },
        ),
        "Grove of the Burnwillows" => s(
            &["{T}: Add {C}.", "{T}: Add {R} or {G}. Each opponent gains 1 life."],
            |d, _| {
                let mut m = ManaSource::of(&[Color::Red, Color::Green]);
                m.colorless = true;
                d.mana = Some(m);
                d.mana_ext.colored_gives_opponent_life = true;
            },
        ),
        // ---- d1 core, batch 3: evoke ----
        "Mulldrifter" => s(
            &["When this creature enters, draw two cards.", "Evoke {2}{U}"],
            |d, _| {
                on(d, When::Enters, vec![], vec![draw(2)]);
                evoke(d, "{2}{U}");
            },
        ),
        "Reveillark" => s(
            &[
                "When this creature leaves the battlefield, return up to two target creature cards with power 2 or less from your graveyard to the battlefield.",
                "Evoke {5}{W}",
            ],
            |d, _| {
                let f = Filter { creature_max_power: Some(2), ..graveyard_cards(true, Controller::You) };
                let ability = up_to(vec![f, f], vec![Effect::Reanimate(0), Effect::Reanimate(1)]);
                d.triggers.push(Trigger { when: When::LeavesBattlefield, ability });
                evoke(d, "{5}{W}");
            },
        ),
        // ---- d1 core, batch 3: convoke ----
        "Chord of Calling" => s(
            &[
                "Convoke",
                "Search your library for a creature card with mana value X or less, put it onto the battlefield, then shuffle.",
            ],
            |d, _| {
                d.mana_ext.convoke = true;
                let to = SearchTo::Battlefield { tapped: false };
                spell(d, vec![], vec![Effect::Search { what: SearchFor::CreatureMvAtMostX, to, n: 1, kicked_n: None }]);
            },
        ),
        // ---- d1 core, batch 3: alternative costs ----
        "Force of Will" => s(
            &[
                "You may pay 1 life and exile a blue card from your hand rather than pay this spell's mana cost.",
                "Counter target spell.",
            ],
            |d, _| {
                d.mana_ext.pitch_with_life = Some((1, 1));
                spell(d, vec![SPELL], vec![Effect::Counter(0)]);
            },
        ),
        // ---- d1 core, batch 3: Eldrazi Spawn, Spawn and Elf Druid tokens ----
        "Unfathomable Truths" => s(
            &[
                "Devoid",
                "Draw three cards and create a 0/1 colorless Eldrazi Spawn creature token with \"Sacrifice this token: Add {C}.\"",
            ],
            |d, t| {
                // Devoid: the card's colors are already none (CR 702.114a).
                d.colors = [false; 5];
                spell(d, vec![], vec![draw(3), Effect::CreateToken { who: Who::You, def: t.eldrazi_spawn, n: 1 }]);
            },
        ),
        "Magnus the Red" => s(
            &[
                "Unearthly Power — Instant and sorcery spells you cast cost {1} less to cast for each creature token you control.",
                "Blade of Magnus — Whenever Magnus the Red deals combat damage to a player, create a 3/3 red Spawn creature token.",
            ],
            |d, t| {
                d.mana_ext.instant_sorcery_discount_per_creature_token = true;
                on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::CreateToken { who: Who::You, def: t.spawn_3, n: 1 }]);
            },
        ),
        "Freyalise, Llanowar's Fury" => s(
            &[
                "+2: Create a 1/1 green Elf Druid creature token with \"{T}: Add {G}.\"",
                "−2: Destroy target artifact or enchantment.",
                "−6: Draw a card for each green creature you control.",
                "Freyalise, Llanowar's Fury can be your commander.",
            ],
            |d, t| {
                loyalty(d, 2, Ability::new(vec![], vec![Effect::CreateToken { who: Who::You, def: t.elf_druid, n: 1 }]));
                loyalty(d, -2, Ability::new(vec![ARTIFACT_OR_ENCHANTMENT], vec![Effect::Destroy(0)]));
                loyalty(d, -6, Ability::new(vec![], vec![Effect::DrawPerCreatureYouControlOfColor(Color::Green)]));
                // "can be your commander": a deckbuilding permission; the engine takes any commander.
            },
        ),
        _ => None,
    }
}

/// This lane's scripted playgroup cards (checked against their text in `lib.rs`).
pub const PG_D1_SCRIPTED: &[&str] = &[
    // Ygra, Eater of All (Grande)
    "Ashnod's Altar",
    "Worldly Tutor",
    "Warren Soultrader",
    "Blood Artist",
    "Night's Whisper",
    "Dictate of Erebos",
    "Phyrexian Altar",
    "Sign in Blood",
    "Viscera Seer",
    "Blasting Station",
    "Carrion Feeder",
    "Darkbore Pathway // Slitherbore Pathway",
    "Fyndhorn Elves",
    "Krosan Grip",
    "Abrupt Decay",
    "Elves of Deep Shadow",
    // Raggadragga, Goreguts Boss (Adam)
    "Beast Whisperer",
    "Staff of Domination",
    "Fires of Yavimaya",
    "Wirewood Lodge",
    "Boreal Druid",
    "Cragcrown Pathway // Timbercrown Pathway",
    "Marwyn, the Nurturer",
    "Nature's Claim",
    "Viridian Joiner",
    "Vitalize",
    // Atla Palani, Nest Tender (RuricMo)
    "Aura Shards",
    "Impact Tremors",
    "Goblin Bombardment",
    "High Market",
    "Jetmir's Garden",
    "Jungle Shrine",
    "Fanatical Devotion",
    "Samut, Voice of Dissent",
    "Sheltering Landscape",
    // Karador, Ghost Chieftain (Grande)
    "Wood Elves",
    "Acidic Slime",
    "Eladamri's Call",
    "Avacyn's Pilgrim",
    "Utter End",
    "Wight of the Reliquary",
    // Ral, Monsoon Mage // Ral, Leyline Prodigy (Time)
    "Gilded Lotus",
    "Ash Barrens",
    "Dismiss",
    "Exclude",
    "Gitaxian Probe",
    "Jwari Disruption // Jwari Ruins",
    "Lightning Bolt",
    "Neutralize",
    "Opportunity",
    "Prophetic Bolt",
    "Rewind",
    // Zacama, Primal Calamity (DonSpider)
    "Karametra, God of Harvests",
    "Stony Silence",
    "Sunhome, Fortress of the Legion",
    "Wilderness Reclamation",
    "Zacama, Primal Calamity",
    // core theme d1 (mana, costs and keywords)
    "Ignoble Hierarch",
    "Glen Elendra Archmage",
    "Gray Merchant of Asphodel",
    "Nykthos, Shrine to Nyx",
    "Cabal Coffers",
    "Cabal Stronghold",
    "Priest of Titania",
    "Heronblade Elite",
    "Into the Story",
    "Phyrexian Tower",
    "Sunbaked Canyon",
    "Wild Cantor",
    "Kozilek, Butcher of Truth",
    "Noise Marine",
    "Baleful Mastery",
    "Nature's Will",
    "Jeska's Will",
    "Karmic Guide",
    "Jukai Naturalist",
    "Bontu's Monument",
    "Animar, Soul of Elements",
    "Rise of the Eldrazi",
    "Shizo, Death's Storehouse",
    "Haunted Ridge",
    "Overgrown Farmland",
    "Toxic Deluge",
    "Fertile Ground",
    "Trace of Abundance",
    "Buried in the Garden",
    "Crypt Ghast",
    "Chromatic Lantern",
    "Grove of the Burnwillows",
    "Mulldrifter",
    "Reveillark",
    "Chord of Calling",
    "Force of Will",
    "Unfathomable Truths",
    "Magnus the Red",
    "Freyalise, Llanowar's Fury",
];
