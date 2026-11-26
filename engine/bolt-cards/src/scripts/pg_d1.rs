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
        "Eternal Witness" => s(&["When this creature enters, you may return target card from your graveyard to your hand."], |d, _| {
            may(d, When::Enters, vec![graveyard_cards(false, Controller::You)], vec![Effect::Bounce(0)])
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
        "Darkbore Pathway // Slitherbore Pathway" => s(&["{T}: Add {B}."], |d, _| {
            // The card data has no back face (Slitherbore Pathway, {T}: Add {G}): it plays as its front.
            d.mana = Some(ManaSource::of(&[Color::Black]));
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
        "Cragcrown Pathway // Timbercrown Pathway" => s(&["{T}: Add {R}."], |d, _| {
            // The card data has no back face (Timbercrown Pathway, {T}: Add {G}): it plays as its front.
            d.mana = Some(ManaSource::of(&[Color::Red]));
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
        "Jwari Disruption // Jwari Ruins" => s(&["Counter target spell unless its controller pays {1}."], |d, _| {
            // The card data has no back face (Jwari Ruins, a land): it plays as its front.
            spell(d, vec![SPELL], vec![Effect::CounterUnlessPays(0, 1, 1)])
        }),
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
        _ => None,
    }
}

/// This lane's scripted playgroup cards (checked against their text in `lib.rs`).
pub const PG_D1_SCRIPTED: &[&str] = &[
    // Ygra, Eater of All (Grande)
    "Ashnod's Altar",
    "Eternal Witness",
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
];
