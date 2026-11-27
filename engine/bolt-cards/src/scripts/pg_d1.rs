//! Playgroup decks (Nerd Leagues), lane D1 (the desktop): the cards for the deck in handoff/lanes/d1.json. Conventions:
//! docs/lane-brief.md. Card text: assets/PG_oracle.json (in the CMD pool). Tests: tests/pg_d1.rs.
#![allow(unused_imports)]

use super::*;
use bolt_core::effect::ManaCount;

/// Any creature card (a search with no mana value limit).
const ANY_CREATURE_CARD: SearchFor = SearchFor::CreatureMvAtLeast(0);

/// An Urza land: {C}, or `n` colorless while you control Urza's Mine, Power-Plant and Tower.
fn urza_land(d: &mut CardDef, n: u8) {
    d.mana = Some(ManaSource {
        colorless: true,
        ..ManaSource::default()
    });
    d.mana_ext.tron_amount = n;
}

/// Foretell (CR 702.143): "During your turn, you may pay {2} and exile this card from your hand
/// face down. Cast it on a later turn for its foretell cost." The card is exiled face up here.
fn foretell(d: &mut CardDef, cost: &str) {
    d.mana_ext.foretell = ManaCost::parse(cost);
    let n = activated(d, "{2}", false, vec![], vec![]);
    d.activated[n].foretell = true;
    d.activated[n].your_turn_only = true;
}

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
        // ---- d1 core, batch 4: colored discounts and extra mana ----
        "Ruby Medallion" => s(&["Red spells you cast cost {1} less to cast."], |d, _| {
            d.red_spell_discount = 1;
        }),
        "Jet Medallion" => s(&["Black spells you cast cost {1} less to cast."], |d, _| {
            d.mana_ext.colored_spell_discount = Some(([false, false, true, false, false], 1));
        }),
        "Stormscape Familiar" => s(&["White spells and black spells you cast cost {1} less to cast."], |d, _| {
            d.mana_ext.colored_spell_discount = Some(([true, false, true, false, false], 1));
        }),
        "Zendikar Resurgent" => s(
            &[
                "Whenever you tap a land for mana, add one mana of any type that land produced.",
                "Whenever you cast a creature spell, draw a card.",
            ],
            |d, _| {
                d.lands_add_same = true;
                on(d, When::YouCastCreature, vec![], vec![draw(1)]);
            },
        ),
        "Dictate of Karametra" => s(
            &["Whenever a player taps a land for mana, that player adds one mana of any type that land produced."],
            |d, _| d.mana_ext.all_lands_add_same = true,
        ),
        "Leyline of Abundance" => s(
            &[
                "If this card is in your opening hand, you may begin the game with it on the battlefield.",
                "Whenever you tap a creature for mana, add an additional {G}.",
                "{6}{G}{G}: Put a +1/+1 counter on each creature you control.",
            ],
            |d, _| {
                d.leyline = true;
                d.mana_ext.your_creatures_add = Some(Color::Green);
                activated(d, "{6}{G}{G}", false, vec![], vec![Effect::CountersAll { who: Who::You, n: 1 }]);
            },
        ),
        // ---- d1 core, batch 4: escape ----
        "Woe Strider" => s(
            &[
                "When this creature enters, create a 0/1 white Goat creature token.",
                "Sacrifice another creature: Scry 1.",
                "Escape—{3}{B}{B}, Exile four other cards from your graveyard.",
                "This creature escapes with two +1/+1 counters on it.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.goat, n: 1 }]);
                let n = activated(d, "", false, vec![], vec![Effect::Scry { n: 1 }]);
                d.activated[n].sacrifice_other_creature = true;
                d.mana_ext.escape = Some((ManaCost::parse("{3}{B}{B}").expect("cost"), 4, 2));
            },
        ),
        // ---- d1 core, batch 6: alternative costs of life, sacrifices and taps; ability cost
        // reduction; {Q}; shadow; first-creature-spell discount ----
        "Demon of Death's Gate" => s(
            &["You may pay 6 life and sacrifice three black creatures rather than pay this spell's mana cost."],
            |d, _| {
                d.mana_ext.alt_cost_ext = Some(bolt_core::card::AltCostExt {
                    life: 6,
                    sacrifice: 3,
                    sacrifice_color: Some(2),
                    ..Default::default()
                });
            },
        ),
        "Sephara, Sky's Blade" => s(
            &[
                "You may pay {W} and tap four untapped creatures you control with flying rather than pay this spell's mana cost.",
                "Other creatures you control with flying have indestructible.",
            ],
            |d, _| {
                d.mana_ext.alt_cost_ext = Some(bolt_core::card::AltCostExt {
                    mana: ManaCost::parse("{W}").expect("cost"),
                    tap: 4,
                    tap_keywords: Keywords::FLYING,
                    ..Default::default()
                });
                d.statics.push(Static {
                    subtype: None,
                    lose_abilities: false,
                    base_pt: None,
                    protection: [false; 5],
                    scale: None,
                    scope: Scope::OtherYourFliers,
                    your_turn_only: false,
                    condition: None,
                    power: 0,
                    toughness: 0,
                    keywords: Keywords::INDESTRUCTIBLE,
                    set_colors: None,
                    becomes_land: false,
                });
            },
        ),
        "Zirda, the Dawnwaker" => s(
            &[
                "Companion — Each permanent card in your starting deck has an activated ability.",
                "Abilities you activate that aren't mana abilities cost {2} less to activate. This effect can't reduce the mana in that cost to less than one mana.",
                "{1}, {T}: Target creature can't block this turn.",
            ],
            |d, _| {
                // Companion is a deckbuilding choice; the engine plays it from the deck.
                d.mana_ext.ability_discount = 2;
                activated(d, "{1}", true, vec![CREATURE], vec![Effect::Grant { slot: 0, keywords: Keywords::CANT_BLOCK }]);
            },
        ),
        "Umbral Mantle" => s(
            &["Equipped creature has \"{3}, {Q}: This creature gets +2/+2 until end of turn.\"", "Equip {0}"],
            |d, _| {
                let n = activated(d, "{3}", false, vec![], vec![Effect::PumpAttached { power: 2, toughness: 2 }]);
                d.activated[n].granted_to_attached = true;
                d.activated[n].untap_attached = true;
                equip(d, "{0}");
            },
        ),
        "Survival of the Fittest" => s(
            &["{G}, Discard a creature card: Search your library for a creature card, reveal that card, put it into your hand, then shuffle."],
            |d, _| {
                let n = activated(d, "{G}", false, vec![], vec![search(SearchFor::CreatureMvAtLeast(0), SearchTo::Hand)]);
                d.activated[n].discard_creature_card = true;
            },
        ),
        "Selhoff Entomber" => s(&["{T}, Discard a creature card: Draw a card."], |d, _| {
            let n = activated(d, "", true, vec![], vec![draw(1)]);
            d.activated[n].discard_creature_card = true;
        }),
        "Tortured Existence" => s(
            &["{B}, Discard a creature card: Return target creature card from your graveyard to your hand."],
            |d, _| {
                let n = activated(d, "{B}", false, vec![graveyard_cards(true, Controller::You)], vec![Effect::Bounce(0)]);
                d.activated[n].discard_creature_card = true;
            },
        ),
        "Mind Over Matter" => s(&["Discard a card: You may tap or untap target artifact, creature, or land."], |d, _| {
            let f = Filter { artifacts: true, lands: true, ..CREATURE };
            d.activated.push(Activated {
                discard_any_card: true,
                ability: up_to(vec![f], vec![Effect::TapOrUntap(0)]),
                ..Activated::default()
            });
        }),
        "Looter il-Kor" => s(&["Whenever this creature deals damage to an opponent, draw a card, then discard a card."], |d, _| {
            on(d, When::DealsDamageToOpponent, vec![], vec![draw(1), discard(1)])
        }),
        "Shadow in the Warp" => s(
            &[
                "The first creature spell you cast each turn costs {2} less to cast.",
                "Whenever an opponent casts their first noncreature spell each turn, this enchantment deals 2 damage to that player.",
            ],
            |d, _| {
                d.mana_ext.first_creature_spell_discount = 2;
                on(d, When::OpponentCastsFirstNoncreature, vec![], vec![damage(2, Recipient::EachOpponent)]);
            },
        ),
        // ---- d1 core, batch 7: return-lands alternative costs, sacrifice-a-land, land costs on
        // abilities, partial pain ----
        "Gush" => s(
            &[
                "You may return two Islands you control to their owner's hand rather than pay this spell's mana cost.",
                "Draw two cards.",
            ],
            |d, _| {
                d.mana_ext.alt_cost_ext = Some(bolt_core::card::AltCostExt {
                    return_lands: 2,
                    return_subtype: Some("Island"),
                    ..Default::default()
                });
                spell(d, vec![], vec![draw(2)]);
            },
        ),
        "Thwart" => s(
            &[
                "You may return three Islands you control to their owner's hand rather than pay this spell's mana cost.",
                "Counter target spell.",
            ],
            |d, _| {
                d.mana_ext.alt_cost_ext = Some(bolt_core::card::AltCostExt {
                    return_lands: 3,
                    return_subtype: Some("Island"),
                    ..Default::default()
                });
                spell(d, vec![SPELL], vec![Effect::Counter(0)]);
            },
        ),
        "Harrow" => s(
            &[
                "As an additional cost to cast this spell, sacrifice a land.",
                "Search your library for up to two basic land cards, put them onto the battlefield, then shuffle.",
            ],
            |d, _| {
                d.extra_cost = Some(ExtraCost::SacrificeLand);
                let to = SearchTo::Battlefield { tapped: false };
                spell(d, vec![], vec![Effect::Search { what: SearchFor::BasicLand, to, n: 2, kicked_n: None }]);
            },
        ),
        "Trade Routes" => s(
            &["{1}: Return target land you control to its owner's hand.", "{1}, Discard a land card: Draw a card."],
            |d, _| {
                let land = Filter {
                    players: false,
                    creatures: false,
                    planeswalkers: false,
                    lands: true,
                    controller: Controller::You,
                    ..Filter::ANY
                };
                activated(d, "{1}", false, vec![land], vec![Effect::Bounce(0)]);
                let n = activated(d, "{1}", false, vec![], vec![draw(1)]);
                d.activated[n].discard_land_card = true;
            },
        ),
        "Murmuring Bosk" => s(
            &[
                "As this land enters, you may reveal a Treefolk card from your hand. If you don't, this land enters tapped.",
                "{T}: Add {W} or {B}. This land deals 1 damage to you.",
            ],
            |d, _| {
                d.enters_tapped_unless_reveal = Some(["Treefolk", "Treefolk"]);
                // Its Forest type's {G}, and the painful {W} or {B}.
                d.mana = Some(ManaSource::of(&[Color::White, Color::Black, Color::Green]));
                d.mana_ext.pain_colors = [true, false, true, false, false];
            },
        ),
        "Natural Order" => s(
            &[
                "As an additional cost to cast this spell, sacrifice a green creature.",
                "Search your library for a green creature card, put it onto the battlefield, then shuffle.",
            ],
            |d, _| {
                d.extra_cost = Some(ExtraCost::SacrificeCreatureOfColor(4));
                let to = SearchTo::Battlefield { tapped: false };
                spell(d, vec![], vec![search(SearchFor::ColoredCreatureMvAtMost(4, u32::MAX), to)]);
            },
        ),
        "Sylvan Safekeeper" => s(&["Sacrifice a land: Target creature you control gains shroud until end of turn."], |d, _| {
            let n = activated(d, "", false, vec![YOUR_CREATURE], vec![Effect::Grant { slot: 0, keywords: Keywords::SHROUD }]);
            d.activated[n].sacrifice_land = true;
        }),
        "Once Upon a Time" => s(
            &[
                "If this spell is the first spell you've cast this game, you may cast it without paying its mana cost.",
                "Look at the top five cards of your library. You may reveal a creature or land card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
            ],
            |d, _| {
                d.mana_ext.alt_cost_ext =
                    Some(bolt_core::card::AltCostExt { first_spell_this_game: true, ..Default::default() });
                let look = Effect::LookAtTop { n: 5, pick: Pick::CreatureOrLand, chosen_to: LookTo::Hand, rest_to: LookRest::BottomRandom };
                spell(d, vec![], vec![look]);
            },
        ),
        "Mana Confluence" => s(&["{T}, Pay 1 life: Add one mana of any color."], |d, _| {
            d.mana = Some(ManaSource::of(&Color::ALL));
            d.mana_ext.mana_life_cost = 1;
        }),
        "Urza's Mine" => s(
            &["{T}: Add {C}. If you control an Urza's Power-Plant and an Urza's Tower, add {C}{C} instead."],
            |d, _| urza_land(d, 2),
        ),
        "Urza's Power Plant" => s(
            &["{T}: Add {C}. If you control an Urza's Mine and an Urza's Tower, add {C}{C} instead."],
            |d, _| urza_land(d, 2),
        ),
        "Urza's Tower" => s(
            &["{T}: Add {C}. If you control an Urza's Mine and an Urza's Power-Plant, add {C}{C}{C} instead."],
            |d, _| urza_land(d, 3),
        ),
        // ---- d1 core, batch 8 ----
        "K'rrik, Son of Yawgmoth" => s(
            &[
                "For each {B} in a cost, you may pay 2 life rather than pay that mana.",
                "Whenever you cast a black spell, put a +1/+1 counter on K'rrik.",
            ],
            |d, _| {
                // The engine pays mana first and 2 life only for the {B} the mana can't pay.
                d.mana_ext.black_pips_for_life = true;
                on(d, When::YouCastColor(2), vec![], vec![Effect::CountersSelf { n: 1 }]);
            },
        ),
        "Meloku the Clouded Mirror" => s(
            &["{1}, Return a land you control to its owner's hand: Create a 1/1 blue Illusion creature token with flying."],
            |d, t| {
                let n = activated(d, "{1}", false, vec![], vec![Effect::CreateToken { who: Who::You, def: t.illusion_flying, n: 1 }]);
                d.activated[n].return_land = true;
            },
        ),
        "The Shire" => s(
            &[
                "The Shire enters tapped unless you control a legendary creature.",
                "{T}: Add {G}.",
                "{1}{G}, {T}, Tap an untapped creature you control: Create a Food token.",
            ],
            |d, t| {
                d.mana_ext.enters_tapped_unless_legendary_creature = true;
                d.mana = Some(ManaSource::of(&[Color::Green]));
                let n = activated(d, "{1}{G}", true, vec![], vec![Effect::CreateToken { who: Who::You, def: t.food, n: 1 }]);
                d.activated[n].tap_untapped_creature = true;
            },
        ),
        // ---- d1 core, batch 9: improvise, multikicker, forage, restricted and conditional mana ----
        "Universal Surveillance" => s(&["Improvise", "Draw X cards."], |d, _| {
            d.mana_ext.improvise = true;
            spell(d, vec![], vec![Effect::DrawX]);
        }),
        "Inspiring Statuary" => s(&["Nonartifact spells you cast have improvise."], |d, _| {
            d.mana_ext.nonartifact_spells_improvise = true;
        }),
        "Kappa Cannoneer" => s(
            &[
                "Improvise",
                "Ward {4}",
                "Whenever this creature or another artifact you control enters, put a +1/+1 counter on this creature. It can't be blocked this turn.",
            ],
            |d, _| {
                d.mana_ext.improvise = true;
                d.ward = ManaCost::parse("{4}");
                // It's an artifact itself, so "an artifact you control enters" covers it too.
                let unblockable = Effect::GrantSelf { keywords: Keywords::UNBLOCKABLE };
                on(d, When::ArtifactYouControlEnters, vec![], vec![Effect::CountersSelf { n: 1 }, unblockable]);
            },
        ),
        "Drift of Phantasms" => s(&["Transmute {1}{U}{U}"], |d, _| {
            // Transmute (CR 702.53): discard it from your hand, as a sorcery.
            let n = activated(d, "{1}{U}{U}", false, vec![], vec![search(SearchFor::ManaValue(3), SearchTo::Hand)]);
            d.activated[n].from_hand_discard = true;
            d.activated[n].sorcery = true;
        }),
        "Crystal Vein" => s(&["{T}: Add {C}.", "{T}, Sacrifice this land: Add {C}{C}."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            // Played on the stack, like Ashnod's Altar's: the automatic payer only taps it for {C}.
            let n = activated(d, "", true, vec![], vec![Effect::AddColorless(2)]);
            d.activated[n].sacrifice_self = true;
        }),
        "Eldrazi Temple" => s(
            &[
                "{T}: Add {C}.",
                "{T}: Add {C}{C}. Spend this mana only to cast colorless Eldrazi spells or activate abilities of colorless Eldrazi.",
            ],
            |d, _| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                d.mana_ext.colorless_eldrazi_amount = 2;
            },
        ),
        "Shrine of the Forsaken Gods" => s(
            &[
                "{T}: Add {C}.",
                "{T}: Add {C}{C}. Spend this mana only to cast colorless spells. Activate only if you control seven or more lands.",
            ],
            |d, _| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                d.mana_ext.colorless_spell_amount_with_lands = Some((2, 7));
            },
        ),
        "Nimbus Maze" => s(
            &[
                "{T}: Add {C}.",
                "{T}: Add {W}. Activate only if you control an Island.",
                "{T}: Add {U}. Activate only if you control a Plains.",
            ],
            |d, _| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                d.mana_ext.colors_if_you_control = [Some("Island"), Some("Plains"), None, None, None];
            },
        ),
        "Everflowing Chalice" => s(
            &[
                "Multikicker {2}",
                "This artifact enters with a charge counter on it for each time it was kicked.",
                "{T}: Add {C} for each charge counter on this artifact.",
            ],
            |d, _| {
                d.mana_ext.multikicker = ManaCost::parse("{2}");
                d.mana_ext.charge_per_kick = true;
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                d.mana_ext.mana_per_kind_counter = true;
            },
        ),
        "Hall of the Bandit Lord" => s(
            &[
                "Hall of the Bandit Lord enters tapped.",
                "{T}, Pay 3 life: Add {C}. If that mana is spent on a creature spell, it gains haste.",
            ],
            |d, _| {
                d.enters_tapped = true;
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                d.mana_ext.mana_life_cost = 3;
                d.mana_ext.creature_spell_haste = true;
            },
        ),
        "Feed the Cycle" => s(
            &["As an additional cost to cast this spell, forage or pay {B}.", "Destroy target creature or planeswalker."],
            |d, _| {
                d.extra_cost = Some(ExtraCost::ForageOrPay(ManaCost::parse("{B}").expect("cost")));
                spell(d, vec![CREATURE_OR_PLANESWALKER], vec![Effect::Destroy(0)]);
            },
        ),
        "Blazemire Verge" => s(&["{T}: Add {B}.", "{T}: Add {R}. Activate only if you control a Swamp or a Mountain."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Black]));
            d.extra_color_if_you_control = Some((Color::Red, ["Swamp", "Mountain"]));
        }),
        "Bleachbone Verge" => s(&["{T}: Add {B}.", "{T}: Add {W}. Activate only if you control a Plains or a Swamp."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Black]));
            d.extra_color_if_you_control = Some((Color::White, ["Plains", "Swamp"]));
        }),
        "Floodfarm Verge" => s(&["{T}: Add {W}.", "{T}: Add {U}. Activate only if you control a Plains or an Island."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::White]));
            d.extra_color_if_you_control = Some((Color::Blue, ["Plains", "Island"]));
        }),
        "Gloomlake Verge" => s(&["{T}: Add {U}.", "{T}: Add {B}. Activate only if you control an Island or a Swamp."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Blue]));
            d.extra_color_if_you_control = Some((Color::Black, ["Island", "Swamp"]));
        }),
        "Wastewood Verge" => s(&["{T}: Add {G}.", "{T}: Add {B}. Activate only if you control a Swamp or a Forest."], |d, _| {
            d.mana = Some(ManaSource::of(&[Color::Green]));
            d.extra_color_if_you_control = Some((Color::Black, ["Swamp", "Forest"]));
        }),
        "Tainted Peak" => s(&["{T}: Add {C}.", "{T}: Add {B} or {R}. Activate only if you control a Swamp."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::of(&[Color::Black, Color::Red]) });
            d.colored_mana_only_if_you_control = Some("Swamp");
        }),
        // ---- d1 core, batch 11: undying keyword, activation conditions ----
        "Minas Tirith" => s(
            &[
                "Minas Tirith enters tapped unless you control a legendary creature.",
                "{T}: Add {W}.",
                "{1}{W}, {T}: Draw a card. Activate only if you attacked with two or more creatures this turn.",
            ],
            |d, _| {
                d.mana_ext.enters_tapped_unless_legendary_creature = true;
                d.mana = Some(ManaSource::of(&[Color::White]));
                let n = activated(d, "{1}{W}", true, vec![], vec![draw(1)]);
                d.activated[n].min_attackers_this_turn = 2;
            },
        ),
        "Watcher of the Spheres" => s(
            &[
                "Creature spells with flying you cast cost {1} less to cast.",
                "Whenever another creature you control with flying enters, this creature gets +1/+1 until end of turn.",
            ],
            |d, _| {
                d.mana_ext.flying_creature_spell_discount = 1;
                let pump = Effect::PumpSelf { power: 1, toughness: 1 };
                on(d, When::AnotherCreatureWithFlyingYouControlEnters, vec![], vec![pump]);
            },
        ),
        "Desecrate Reality" => s(
            &[
                "For each opponent, exile up to one target permanent that player controls with an even mana value.",
                "Adamant — If at least three colorless mana was spent to cast this spell, return a permanent card with an odd mana value from your graveyard to the battlefield.",
            ],
            |d, _| {
                // One opponent in a 1v1 game: up to one target.
                let even = Filter {
                    players: false,
                    planeswalkers: false,
                    creatures: false,
                    any_permanent: true,
                    mana_value_even: true,
                    controller: Controller::Opponent,
                    ..Filter::ANY
                };
                let mut a = up_to(
                    vec![even],
                    vec![Effect::Exile(0), Effect::StopUnlessCondition, Effect::ReturnOddManaValuePermanentToBattlefield],
                );
                a.condition = Some(Condition::ColorlessSpentAtLeast(3));
                d.spell = Some(a);
            },
        ),
        "Spire of Industry" => s(
            &["{T}: Add {C}.", "{T}, Pay 1 life: Add one mana of any color. Activate only if you control an artifact."],
            |d, _| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::of(&Color::ALL) });
                d.mana_ext.colored_life_cost = true;
                d.mana_ext.colored_only_with_artifact = true;
            },
        ),
        "Chandra's Incinerator" => s(
            &[
                "This spell costs {X} less to cast, where X is the total amount of noncombat damage dealt to your opponents this turn.",
                "Whenever a source you control deals noncombat damage to an opponent, this creature deals that much damage to target creature or planeswalker that player controls.",
            ],
            |d, _| {
                d.reduction = Some(Reduction::NoncombatDamageToOpponentsThisTurn);
                // One opponent: "that player" is them.
                let theirs = Filter { players: false, ..Filter::ANY }.controlled_by(Controller::Opponent);
                on(d, When::YourSourceDealsNoncombatDamageToOpponent, vec![theirs], vec![Effect::DamageAmountTo(0)]);
            },
        ),
        "Behold the Multiverse" => s(&["Scry 2, then draw two cards.", "Foretell {1}{U}"], |d, _| {
            spell(d, vec![], vec![Effect::Scry { n: 2 }, draw(2)]);
            foretell(d, "{1}{U}");
        }),
        "Eye of Ugin" => s(
            &[
                "Colorless Eldrazi spells you cast cost {2} less to cast.",
                "{7}, {T}: Search your library for a colorless creature card, reveal it, put it into your hand, then shuffle.",
            ],
            |d, _| {
                d.mana_ext.colorless_eldrazi_spell_discount = 2;
                activated(d, "{7}", true, vec![], vec![search(SearchFor::ColorlessCreature, SearchTo::Hand)]);
            },
        ),
        "War Room" => s(
            &[
                "{T}: Add {C}.",
                "{3}, {T}, Pay life equal to the number of colors in your commanders' color identity: Draw a card.",
            ],
            |d, _| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                let n = activated(d, "{3}", true, vec![], vec![draw(1)]);
                d.activated[n].life_per_commander_color = true;
            },
        ),
        // ---- d1 core, batch 14: kick count with {X}, fabricate, Treasure and Talent mana, retrace,
        // creature-only mana, Darksteel Monolith ----
        "Pride of the Clouds" => s(
            &[
                "This creature gets +1/+1 for each other creature on the battlefield with flying.",
                "Forecast — {2}{W}{U}, Reveal this creature from your hand: Create a 1/1 white and blue Bird creature token with flying.",
            ],
            |d, t| {
                static_on(d, Scope::This, false, Keywords::NONE);
                let st = d.statics.last_mut().expect("just added");
                (st.power, st.toughness, st.scale) = (1, 1, Some(Scale::PerOtherCreatureWithFlying));
                let bird = Effect::CreateToken { who: Who::You, def: t.bird_flying_wu, n: 1 };
                let n = activated(d, "{2}{W}{U}", false, vec![], vec![bird]);
                d.activated[n].forecast = true;
                d.activated[n].once_each_turn = true;
            },
        ),
        "Marionette Apprentice" => s(
            &[
                "Fabricate 1",
                "Whenever another creature or artifact you control is put into a graveyard from the battlefield, each opponent loses 1 life.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::Fabricate { n: 1, servo: t.servo }]);
                let drain = Effect::LoseLife { who: Who::EachOpponent, n: 1 };
                on(d, When::AnotherCreatureOrArtifactYouControlToGraveyardFromBattlefield, vec![], vec![drain]);
            },
        ),
        "Marionette Master" => s(
            &[
                "Fabricate 3",
                "Whenever an artifact you control is put into a graveyard from the battlefield, target opponent loses life equal to this creature's power.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::Fabricate { n: 3, servo: t.servo }]);
                let opp = Filter::PLAYER.controlled_by(Controller::Opponent);
                on(
                    d,
                    When::ArtifactYouControlToGraveyardFromBattlefield,
                    vec![opp],
                    vec![Effect::LoseLifeSourcePower(Who::TargetPlayer(0))],
                );
            },
        ),
        "Food Chain" => s(
            &["Exile a creature you control: Add X mana of any one color, where X is 1 plus the exiled creature's mana value. Spend this mana only to cast creature spells."],
            |d, _| {
                // On the stack, like Ashnod's Altar's.
                let n = activated(d, "", false, vec![], vec![Effect::AddCreatureSpellManaExtraMvPlusOne]);
                d.activated[n].exile_creature_you_control = true;
                d.activated[n].victim_in_extra = true;
            },
        ),
        "Waves of Aggression" => s(
            &[
                "Untap all creatures that attacked this turn. After this main phase, there is an additional combat phase followed by an additional main phase.",
                "Retrace",
            ],
            |d, _| {
                spell(d, vec![], vec![Effect::UntapCreaturesAttackedThisTurn, Effect::AdditionalCombatAndMain]);
                d.mana_ext.retrace = true;
            },
        ),
        "Darksteel Monolith" => s(
            &["Once each turn, you may pay {0} rather than pay the mana cost for a colorless spell you cast from your hand."],
            |d, _| d.mana_ext.colorless_free_once_each_turn = true,
        ),
        // ---- The Gitrog, Ravenous Ride (RuricMo) ----
        "Witch's Cottage" => s(
            &[
                "This land enters tapped unless you control three or more other Swamps.",
                "When this land enters untapped, you may put target creature card from your graveyard on top of your library.",
            ],
            |d, _| {
                d.mana = Some(ManaSource::of(&[Color::Black]));
                d.mana_ext.enters_tapped_unless_other_subtype = Some(("Swamp", 3));
                may(d, When::EntersUntapped, vec![graveyard_cards(true, Controller::You)], vec![Effect::ToLibrary(0, true)]);
            },
        ),
        "Jarad, Golgari Lich Lord" => s(
            &[
                "Jarad gets +1/+1 for each creature card in your graveyard.",
                "{1}{B}{G}, Sacrifice another creature: Each opponent loses life equal to the sacrificed creature's power.",
                "Sacrifice a Swamp and a Forest: Return this card from your graveyard to your hand.",
            ],
            |d, _| {
                d.pt_per_graveyard_creature = true;
                let n = activated(d, "{1}{B}{G}", false, vec![], vec![Effect::LoseLifeAmount(Who::EachOpponent)]);
                d.activated[n].sacrifice_other_creature = true;
                d.activated[n].amount_from_victim_power = true;
                let n = activated(d, "", false, vec![], vec![Effect::ReturnSelfToHand]);
                d.activated[n].from_graveyard = true;
                d.activated[n].sacrifice_two_subtypes = Some(["Swamp", "Forest"]);
            },
        ),
        "Sunbird's Invocation" => s(
            &["Whenever you cast a spell from your hand, reveal the top X cards of your library, where X is that spell's mana value. You may cast a spell with mana value X or less from among cards revealed this way without paying its mana cost. Put the rest on the bottom of your library in a random order."],
            |d, _| on(d, When::YouCastSpellFromHand, vec![], vec![Effect::RevealTopMvMayCastOneFree]),
        ),
        "The Gitrog, Ravenous Ride" => s(
            &[
                "Whenever The Gitrog deals combat damage to a player, you may sacrifice a creature that saddled it this turn. If you do, draw X cards, then put up to X land cards from your hand onto the battlefield tapped, where X is the sacrificed creature's power.",
                "Saddle 1",
            ],
            |d, _| {
                on(d, When::DealsCombatDamageToPlayer, vec![], vec![Effect::SacrificeSaddlerDrawXLandsX]);
                let n = activated(d, "", false, vec![], vec![Effect::BecomeSaddled]);
                d.activated[n].saddle = 1;
                d.activated[n].sorcery = true;
                d.activated[n].victim_in_extra = true;
            },
        ),
        // ---- Karador, Ghost Chieftain ----
        "Karador, Ghost Chieftain" => s(
            &[
                "This spell costs {1} less to cast for each creature card in your graveyard.",
                "Once during each of your turns, you may cast a creature spell from your graveyard.",
            ],
            |d, _| {
                d.reduction = Some(Reduction::PerCreatureCardInYourGraveyard);
                d.mana_ext.creature_from_graveyard_once_each_turn = true;
            },
        ),
        // ---- Animar, Soul of Elements ----
        "Walking Ballista" => s(
            &[
                "This creature enters with X +1/+1 counters on it.",
                "{4}: Put a +1/+1 counter on this creature.",
                "Remove a +1/+1 counter from this creature: It deals 1 damage to any target.",
            ],
            |d, _| {
                d.enters_with_x_counters = true;
                activated(d, "{4}", false, vec![], vec![Effect::CountersSelf { n: 1 }]);
                let n = activated(d, "", false, vec![Filter::ANY], vec![damage(1, Recipient::Target(0))]);
                d.activated[n].remove_plus_counters = 1;
            },
        ),
        "Somberwald Sage" => s(
            &["{T}: Add three mana of any one color. Spend this mana only to cast creature spells."],
            |d, _| {
                // One ability per color (as Kami of Whispered Hopes), all three of that color.
                for c in Color::ALL {
                    activated(d, "", true, vec![], vec![Effect::AddCreatureSpellMana(3, c)]);
                }
            },
        ),
        "Vizier of the Menagerie" => s(
            &[
                "You may look at the top card of your library any time.",
                "You may cast creature spells from the top of your library.",
                "You can spend mana of any type to cast creature spells.",
            ],
            |d, _| {
                // Looking at the top card any time is information only.
                d.top_spells_creatures = true;
                d.mana_ext.creature_spells_any_type_mana = true;
            },
        ),
        // ---- Lonis, Genetics Expert ----
        "Argent Sphinx" => s(
            &["Metalcraft — {U}: Exile this creature. Return it to the battlefield under your control at the beginning of the next end step. Activate only if you control three or more artifacts."],
            |d, _| {
                let n = activated(d, "{U}", false, vec![], vec![Effect::FlickerSelf]);
                d.activated[n].min_artifacts = 3;
            },
        ),
        // ---- Oloro, Ageless Ascetic ----
        "Mystic Gate" => s(&["{T}: Add {C}.", "{W/U}, {T}: Add {W}{W}, {W}{U}, or {U}{U}."], |d, _| {
            d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
            let wu = [Color::White, Color::Blue];
            d.paid_mana = Some((ManaCost::parse("{W/U}").expect("a mana cost"), [ManaSource::of(&wu), ManaSource::of(&wu)]));
        }),
        "Karlov of the Ghost Council" => s(
            &[
                "Whenever you gain life, put two +1/+1 counters on Karlov.",
                "{W}{B}, Remove six +1/+1 counters from Karlov: Exile target creature.",
            ],
            |d, _| {
                on(d, When::YouGainLife, vec![], vec![Effect::CountersSelf { n: 2 }]);
                let n = activated(d, "{W}{B}", false, vec![CREATURE], vec![Effect::Exile(0)]);
                d.activated[n].remove_plus_counters = 6;
            },
        ),
        // ---- Korvold, Fae-Cursed King ----
        "Miren, the Moaning Well" => s(
            &["{T}: Add {C}.", "{3}, {T}, Sacrifice a creature: You gain life equal to the sacrificed creature's toughness."],
            |d, _| {
                d.mana = Some(ManaSource { colorless: true, ..ManaSource::default() });
                let n = activated(d, "{3}", true, vec![], vec![Effect::GainLifeAmount]);
                d.activated[n].sacrifice_other_creature = true;
                d.activated[n].amount_from_victim_toughness = true;
            },
        ),
        // ---- Raggadragga, Goreguts Boss ----
        "Topiary Lecturer" => s(
            &["Increment", "{T}: Add an amount of {G} equal to this creature's power."],
            |d, _| {
                on(d, When::YouCast, vec![], vec![Effect::IncrementSelf]);
                activated(d, "", true, vec![], vec![Effect::AddManaSourcePower(Color::Green)]);
            },
        ),
        // ---- Najeela, the Blade-Blossom ----
        "Champion of Lambholt" => s(
            &[
                "Creatures with power less than this creature's power can't block creatures you control.",
                "Whenever another creature you control enters, put a +1/+1 counter on this creature.",
            ],
            |d, _| {
                d.mana_ext.smaller_cant_block_yours = true;
                on(d, When::AnotherCreatureYouControlEnters, vec![], vec![Effect::CountersSelf { n: 1 }]);
            },
        ),
        // ---- Oloro, Ageless Ascetic ----
        "Will, Scion of Peace" => s(
            &["{T}: Spells you cast this turn that are white and/or blue cost {X} less to cast, where X is the amount of life you gained this turn. Activate only as a sorcery."],
            |d, _| {
                let n = activated(d, "", true, vec![], vec![Effect::WhiteBlueSpellsCostLessLifeGainedThisTurn]);
                d.activated[n].sorcery = true;
            },
        ),
        // ---- Ral, Monsoon Mage ----
        "High Tide" => s(
            &["Until end of turn, whenever a player taps an Island for mana, that player adds an additional {U}."],
            |d, _| spell(d, vec![], vec![Effect::IslandsAddExtraBlueThisTurn]),
        ),
        // ---- Ghyrson Starn, Kelermorph ----
        "Pyromatics" => s(&["Replicate {1}{R}", "Pyromatics deals 1 damage to any target."], |d, _| {
            d.mana_ext.multikicker = ManaCost::parse("{1}{R}");
            on(d, When::YouCastThis, vec![], vec![Effect::CopySourceSpellPerReplicate]);
            spell(d, vec![Filter::ANY], vec![damage(1, Recipient::Target(0))]);
        }),
        // ---- Atla Palani, Nest Tender ----
        "Rith's Grove" => s(
            &[
                "When this land enters, sacrifice it unless you return a non-Lair land you control to its owner's hand.",
                "{T}: Add {R}, {G}, or {W}.",
            ],
            |d, _| {
                on(d, When::Enters, vec![], vec![Effect::ReturnNonSubtypeLandOrSacrificeSelf("Lair")]);
                d.mana = Some(ManaSource::of(&[Color::Red, Color::Green, Color::White]));
            },
        ),
        // ---- Raffine, Scheming Seer ----
        "Momo, Friendly Flier" => s(
            &[
                "The first non-Lemur creature spell with flying you cast during each of your turns costs {1} less to cast.",
                "Whenever another creature you control with flying enters, Momo gets +1/+1 until end of turn.",
            ],
            |d, _| {
                d.mana_ext.first_flier_discount_your_turn = 1;
                on(d, When::AnotherCreatureWithFlyingYouControlEnters, vec![], vec![Effect::PumpSelf { power: 1, toughness: 1 }]);
            },
        ),
        // ---- idea decks ----
        "Path of Ancestry" => s(
            &[
                "This land enters tapped.",
                "{T}: Add one mana of any color in your commander's color identity. When that mana is spent to cast a creature spell that shares a creature type with your commander, scry 1.",
            ],
            |d, _| {
                d.enters_tapped = true;
                d.mana_commander_identity = true;
                d.mana_ext.scry_if_commander_type_creature = true;
                on(d, When::ManaSpentOnCommanderTypeCreature, vec![], vec![Effect::Scry { n: 1 }]);
            },
        ),
        "Rain of Riches" => s(
            &[
                "When this enchantment enters, create two Treasure tokens.",
                "The first spell you cast each turn that mana from a Treasure was spent to cast has cascade.",
            ],
            |d, t| {
                on(d, When::Enters, vec![], vec![Effect::CreateToken { who: Who::You, def: t.treasure, n: 2 }]);
                on(d, When::YouCast, vec![], vec![Effect::StopUnlessFirstTreasureSpellThisTurn, Effect::Cascade]);
            },
        ),
        // ---- Najeela, the Blade-Blossom ----
        "Make Disappear" => s(
            &["Casualty 1", "Counter target spell unless its controller pays {2}."],
            |d, _| {
                d.extra_cost = Some(ExtraCost::Casualty(1));
                on(d, When::YouCastThis, vec![], vec![Effect::CopySourceSpellIfCasualty]);
                spell(d, vec![SPELL], vec![Effect::CounterUnlessPays(0, 2, 2)]);
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
    "Ruby Medallion",
    "Jet Medallion",
    "Stormscape Familiar",
    "Zendikar Resurgent",
    "Dictate of Karametra",
    "Leyline of Abundance",
    "Woe Strider",
    "Demon of Death's Gate",
    "Sephara, Sky's Blade",
    "Zirda, the Dawnwaker",
    "Umbral Mantle",
    "Survival of the Fittest",
    "Selhoff Entomber",
    "Tortured Existence",
    "Mind Over Matter",
    "Looter il-Kor",
    "Shadow in the Warp",
    "Gush",
    "Thwart",
    "Harrow",
    "Trade Routes",
    "Murmuring Bosk",
    "Natural Order",
    "Sylvan Safekeeper",
    "Once Upon a Time",
    "Mana Confluence",
    "Urza's Mine",
    "Urza's Power Plant",
    "Urza's Tower",
    "K'rrik, Son of Yawgmoth",
    "Meloku the Clouded Mirror",
    "The Shire",
    "Universal Surveillance",
    "Inspiring Statuary",
    "Kappa Cannoneer",
    "Drift of Phantasms",
    "Crystal Vein",
    "Eldrazi Temple",
    "Shrine of the Forsaken Gods",
    "Nimbus Maze",
    "Everflowing Chalice",
    "Hall of the Bandit Lord",
    "Feed the Cycle",
    "Blazemire Verge",
    "Bleachbone Verge",
    "Floodfarm Verge",
    "Gloomlake Verge",
    "Wastewood Verge",
    "Tainted Peak",
    "Minas Tirith",
    "Watcher of the Spheres",
    "Desecrate Reality",
    "Spire of Industry",
    "Chandra's Incinerator",
    "Behold the Multiverse",
    "Eye of Ugin",
    "War Room",
    "Pride of the Clouds",
    "Marionette Apprentice",
    "Marionette Master",
    "Food Chain",
    "Waves of Aggression",
    "Darksteel Monolith",
    "Witch's Cottage",
    "Jarad, Golgari Lich Lord",
    "Sunbird's Invocation",
    "The Gitrog, Ravenous Ride",
    "Karador, Ghost Chieftain",
    "Walking Ballista",
    "Somberwald Sage",
    "Vizier of the Menagerie",
    "Argent Sphinx",
    "Mystic Gate",
    "Karlov of the Ghost Council",
    "Miren, the Moaning Well",
    "Topiary Lecturer",
    "Champion of Lambholt",
    "Will, Scion of Peace",
    "High Tide",
    "Pyromatics",
    "Rith's Grove",
    "Momo, Friendly Flier",
    "Path of Ancestry",
    "Rain of Riches",
    "Make Disappear",
];
