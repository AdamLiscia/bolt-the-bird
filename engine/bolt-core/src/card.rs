//! Card definitions: the printed characteristics an engine needs (CR 200-209).

use crate::effect::{Ability, Condition, Static, Trigger};
use crate::mana::{ManaCost, ManaSource};

/// Keyword abilities as a bit set (CR 702). Only the evergreen combat keywords exist so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Keywords(u32);

macro_rules! keywords {
    ($($name:ident = $bit:expr, $text:expr;)*) => {
        impl Keywords {
            $(pub const $name: Keywords = Keywords(1 << $bit);)*

            /// Every keyword the engine implements, with its rules-text name.
            pub const ALL: &'static [(Keywords, &'static str)] = &[$((Keywords::$name, $text)),*];
        }
    };
}

keywords! {
    FLYING = 0, "Flying";
    REACH = 1, "Reach";
    FIRST_STRIKE = 2, "First strike";
    DOUBLE_STRIKE = 3, "Double strike";
    DEATHTOUCH = 4, "Deathtouch";
    LIFELINK = 5, "Lifelink";
    TRAMPLE = 6, "Trample";
    VIGILANCE = 7, "Vigilance";
    HASTE = 8, "Haste";
    MENACE = 9, "Menace";
    DEFENDER = 10, "Defender";
    INDESTRUCTIBLE = 11, "Indestructible";
    HEXPROOF = 12, "Hexproof";
    FLASH = 13, "Flash";
    UNBLOCKABLE = 14, "Can't be blocked";
    CANT_BLOCK = 15, "Can't block";
    ISLANDWALK = 16, "Islandwalk";
    SHROUD = 17, "Shroud";
    FORESTWALK = 18, "Forestwalk";
    FEAR = 19, "Fear";
    INTIMIDATE = 20, "Intimidate";
    SHADOW = 21, "Shadow";
    SWAMPWALK = 22, "Swampwalk";
    UNDYING = 23, "Undying";
}

impl Keywords {
    pub const NONE: Keywords = Keywords(0);

    pub fn has(self, k: Keywords) -> bool {
        self.0 & k.0 == k.0
    }

    pub fn with(self, k: Keywords) -> Keywords {
        Keywords(self.0 | k.0)
    }

    /// Looks up a keyword by its rules-text name, ignoring case.
    pub fn named(name: &str) -> Option<Keywords> {
        Keywords::ALL
            .iter()
            .find(|(_, n)| n.eq_ignore_ascii_case(name))
            .map(|&(k, _)| k)
    }
}

/// What a permanent becomes at a stage: its creature subtypes, base power and toughness, and
/// keywords gained (kept at later stages).
#[derive(Clone, Debug, PartialEq)]
pub struct Stage {
    pub subtypes: &'static [&'static str],
    pub base_pt: Option<(i32, i32)>,
    pub keywords: Keywords,
}

/// Card types and supertypes (CR 205).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Types {
    pub land: bool,
    pub creature: bool,
    pub artifact: bool,
    pub enchantment: bool,
    pub instant: bool,
    pub sorcery: bool,
    pub planeswalker: bool,
    pub legendary: bool,
    pub basic: bool,
    /// The snow supertype (CR 205.4g): its mana is {S} when spent.
    pub snow: bool,
}

impl Types {
    /// Parses the part of a type line before the em dash, e.g. `Legendary Creature`.
    pub fn parse(type_line: &str) -> Types {
        let main = type_line.split('—').next().unwrap_or("");
        let mut t = Types::default();
        for word in main.split_whitespace() {
            match word {
                "Land" => t.land = true,
                "Creature" => t.creature = true,
                "Artifact" => t.artifact = true,
                "Enchantment" => t.enchantment = true,
                "Instant" => t.instant = true,
                "Sorcery" => t.sorcery = true,
                "Planeswalker" => t.planeswalker = true,
                "Legendary" => t.legendary = true,
                "Basic" => t.basic = true,
                "Snow" => t.snow = true,
                _ => {}
            }
        }
        t
    }

    /// A permanent card enters the battlefield when it resolves (CR 110.4).
    pub fn is_permanent(&self) -> bool {
        !(self.instant || self.sorcery)
    }
}

/// An additional cost to cast a spell (CR 601.2b, 601.2h).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtraCost {
    SacrificeCreature,
    /// Discard a card other than the spell.
    Discard,
    /// Sacrifice a creature, or pay this mana instead.
    SacrificeCreatureOrPay(ManaCost),
    /// Sacrifice an artifact or creature (Deadly Dispute).
    SacrificeArtifactOrCreature,
    /// Sacrifice a land (Harrow); cast with `Action::CastWith` naming it. It may tap for the
    /// spell's mana first.
    SacrificeLand,
    /// Sacrifice a creature of this color (WUBRG index) (Natural Order: green); cast with
    /// `Action::CastWith` naming it.
    SacrificeCreatureOfColor(u8),
    /// "As an additional cost to cast this spell, forage or pay [mana]." (Feed the Cycle): cast
    /// with `Action::Cast` paying the mana, or `Action::CastWith` naming a Food to sacrifice or a
    /// card in your graveyard (it and two more, the engine's picks, are exiled).
    ForageOrPay(ManaCost),
    /// "As an additional cost to cast this spell, reveal a colorless creature card from your hand."
    /// (Titan's Presence): cast with `Action::CastWith` naming it; it stays in your hand, and is the
    /// spell's `extra`.
    RevealColorlessCreature,
    /// Casualty N (CR 702.153): "As you cast this spell, you may sacrifice a creature with power N
    /// or greater." Cast with `Action::Cast` (none) or `Action::CastWith` naming it (it's the
    /// spell's `extra`); `Effect::CopySourceSpellIfCasualty` makes the copy.
    Casualty(u8),
    /// "As an additional cost to cast this spell, blight N or pay [mana]." (Bogslither's Embrace:
    /// 1, {3}; Wild Unraveling: 2, {1}): `Action::Cast` pays the mana, `Action::CastWith` names a
    /// creature you control to put N -1/-1 counters on (to blight N).
    BlightOrPay(u8, ManaCost),
}

/// "This spell costs {N} less to cast if ..." (CR 601.2f).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reduction {
    /// ... you control a permanent with this subtype.
    IfYouControl(String, u8),
    /// ... it targets a tapped creature.
    IfTargetsTapped(u8),
    /// ... {1} less for each permanent you control with this subtype (affinity).
    PerYouControl(&'static str),
    /// ... {1} less for each instant and sorcery card in your graveyard.
    PerInstantSorceryInGraveyard,
    /// ... {X} less, X the total power of creatures you control.
    TotalPowerOfYourCreatures,
    /// ... {1} less for each attacking creature you control.
    PerAttackingCreature,
    /// ... {n} less if you've cast another spell this turn.
    IfCastAnotherSpell(u8),
    /// ... {1} less for each attacking creature, anyone's (Static Snare).
    PerAnyAttackingCreature,
    /// ... {1} less for each creature you control (affinity for creatures).
    PerCreatureYouControl,
    /// ... {n} MORE if it targets a permanent with this subtype (Dragon's Prey).
    MoreIfTargetsSubtype(&'static str, u8),
    /// ... {1} less for each creature you control with power N or greater (Spectral Denial).
    PerCreatureYouControlPowerAtLeast(i32),
    /// ... {1} less for each creature on the battlefield (Blasphemous Act).
    PerCreatureOnBattlefield,
    /// ... {1} less for each token you control (affinity for tokens).
    PerTokenYouControl,
    /// ... {1} less for each creature that attacked this turn (Rowdy Research).
    PerCreatureAttackedThisTurn,
    /// Delirium: ... {n} less with four or more card types among cards in your graveyard.
    Delirium(u8),
    /// ... {X} less, X the greatest power among creatures you control (The Great Henge).
    GreatestPowerAmongYourCreatures,
    /// ... {X} less, X the greatest number of artifacts an opponent controls (Cavern-Hoard Dragon).
    GreatestArtifactsAnOpponentControls,
    /// ... {n} less (the second number) if an opponent has at least this many cards (the first) in
    /// their graveyard (Into the Story: seven, {3}).
    IfOpponentGraveyardAtLeast(u8, u8),
    /// {X} less, X the noncombat damage dealt to your opponents this turn (Chandra's Incinerator).
    NoncombatDamageToOpponentsThisTurn,
    /// {1} less for each creature card in your graveyard (Karador, Ghost Chieftain).
    PerCreatureCardInYourGraveyard,
    /// Affinity for artifacts (CR 702.41): {1} less for each artifact you control (Emry, Lurker of
    /// the Loch).
    PerArtifactYouControl,
    /// "This spell costs {N} less to cast if an opponent controls a [color] permanent." (Oakhame
    /// Adversary: green (WUBRG 4), 2)
    IfOpponentControlsColor(u8, u8),
}

/// An activated ability: "[cost]: [effect]" (CR 602).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Activated {
    pub mana: ManaCost,
    /// {T} in the cost.
    pub tap: bool,
    /// "Sacrifice this ..." in the cost.
    pub sacrifice_self: bool,
    /// "Sacrifice another creature" in the cost.
    pub sacrifice_other_creature: bool,
    /// "Activate only as a sorcery."
    pub sorcery: bool,
    /// Activated from the graveyard ("Return this card from your graveyard ...").
    pub from_graveyard: bool,
    /// Remove this many incubation counters from the source as a cost.
    pub remove_incubation: i32,
    /// "Pay N life" in the cost (fetch lands).
    pub life: i32,
    /// "Activate only once."
    pub once: bool,
    /// "Tap N untapped [subtype] you control" (other than the source) as a cost.
    pub tap_untapped: Option<(&'static str, u8)>,
    /// Tap the creature this is attached to as part of the cost (Fishing Pole).
    pub tap_attached: bool,
    /// "Exile this card from your graveyard" in the cost (renew).
    pub exile_self: bool,
    pub ability: Ability,
    /// "Sacrifice a token" in the cost.
    pub sacrifice_token: bool,
    /// "Exile a creature card from your graveyard" in the cost.
    pub exile_creature_card: bool,
    /// An Aura's ability its enchanted creature has ("Enchanted creature has \"{5}: Untap this
    /// creature.\""): activated by that creature's controller, with the creature in `extra`.
    pub granted_to_attached: bool,
    /// "Sacrifice a creature you control with mana value X other than this": activated as
    /// `ActivateX` with that victim and X its mana value plus 1, the mana value its target needs
    /// (Sidisi; see `Filter::mana_value_is_x`).
    pub sacrifice_mv_x: bool,
    /// "Discard your hand" in the cost.
    pub discard_hand: bool,
    /// "Pay X life" in the cost of an ability with {X} (Krumar Initiate).
    pub life_x: bool,
    /// "Activate only during your turn."
    pub your_turn_only: bool,
    /// "Activate only if a card left your graveyard this turn."
    pub if_card_left_graveyard: bool,
    /// "This ability costs {1} less to activate for each color of the creature it targets."
    pub less_per_target_color: bool,
    /// "Tap X untapped tokens you control" in the cost (activated with `Action::ActivateX`).
    pub tap_x_untapped_tokens: bool,
    /// Activated from your hand, discarding this card as a cost (cycling: "{cost}, Discard this
    /// card: Draw a card.").
    pub from_hand_discard: bool,
    /// The stack item's amount is the sacrificed creature's power (Greater Good).
    pub amount_from_victim_power: bool,
    /// "Sacrifice a [subtype]" in the cost (Siege-Gang Commander: "Sacrifice a Goblin"); the source
    /// itself counts if it has the subtype. Activated with `Action::ActivateWith`.
    pub sacrifice_subtype: Option<&'static str>,
    /// "Sacrifice a creature" in the cost: any creature you control, this one included.
    pub sacrifice_creature: bool,
    /// "Activate only if you created a token this turn."
    pub if_created_token_this_turn: bool,
    /// "Activate only once each turn."
    pub once_each_turn: bool,
    /// Remove this many of the source's own kind of counters as a cost (Hoofprints of the Stag).
    pub remove_kind_counters: i32,
    /// With `sacrifice_subtype`: "Sacrifice another [subtype]" (not the source; Koma).
    pub sacrifice_another: bool,
    /// "Activate only if you control N or more permanents." (Temple of Cultivation)
    pub min_permanents: u8,
    /// Crew N (CR 702.122): tap any number of other untapped creatures you control with total power
    /// N or more (`Action::ActivateWith` names the first; `Decision::CrewTap` picks the rest when it
    /// isn't enough alone).
    pub crew: u8,
    /// Only while it's a copy that keeps its own abilities (Sakashima the Impostor's "{2}{U}{U}:
    /// Return ..."): the printed card itself doesn't have it.
    pub copy_only: bool,
    /// "Activate only if it's not your turn." (Ghost Town)
    pub not_your_turn_only: bool,
    /// "This ability costs {1} less to activate for each legendary creature you control." (Otawara)
    pub less_per_legendary_creature: bool,
    /// "Activate only if there are four or more card types among cards in your graveyard."
    /// (delirium)
    pub needs_delirium: bool,
    /// "Activate only if an opponent controls N or more lands." (Tectonic Edge)
    pub opponent_min_lands: u8,
    /// "Discard a creature card" in the cost: activated with `Action::ActivateWith`, that card the
    /// victim (Fauna Shaman).
    pub discard_creature_card: bool,
    /// "Put a -1/-1 counter on this creature" in the cost (Devoted Druid).
    pub minus_counter_cost: bool,
    /// "Discard a card" in the cost: activated as `ActivateWith` naming the card (Yawgmoth).
    pub discard_cost: bool,
    /// "Discard a card" in the cost: activated with `Action::ActivateWith`, that card the victim.
    pub discard_any_card: bool,
    /// "Sacrifice two other creatures" in the cost: the victim, then the engine sacrifices another
    /// (the one with the least power) (Zopandrel).
    pub sacrifice_two_others: bool,
    /// Phyrexian pips in `mana` by color: each is paid with 2 life when the mana can't be (the
    /// engine's choice: mana first).
    pub phyrexian: [u8; 5],
    /// The sacrificed creature is kept as the stack entry's `extra` (Birthing Pod).
    pub victim_in_extra: bool,
    /// "Sacrifice two other artifacts and/or creatures" in the cost (Mondrak): the first named by
    /// `ActivateWith`, the second the engine's choice (a token, else the lowest mana value).
    pub sacrifice_two_artifacts_or_creatures: bool,
    /// With `sacrifice_subtype`: sacrifice this many of them ("Sacrifice four Foods"): the victim, then
    /// the engine picks the rest; 0 or 1: one.
    pub sacrifice_subtype_count: u8,
    /// "Tap an untapped legendary creature you control" in the cost (Dungeon Descent); the engine
    /// picks which (the first such creature).
    pub tap_untapped_legendary: bool,
    /// {Q} in a granted ability of the creature this is attached to (Umbral Mantle: "Equipped
    /// creature has '{3}, {Q}: ...'"): the creature must be tapped and able to use {Q} (CR
    /// 302.6), and untaps as the cost.
    pub untap_attached: bool,
    /// "Return a land you control to its owner's hand" in the cost (Meloku): activated with
    /// `Action::ActivateWith`, that land the victim. It may tap for the ability's mana first.
    pub return_land: bool,
    /// "Discard a land card" in the cost (Trade Routes): activated with `Action::ActivateWith`,
    /// that card the victim.
    pub discard_land_card: bool,
    /// "Sacrifice a land" in the cost (Sylvan Safekeeper): activated with `Action::ActivateWith`,
    /// that land the victim.
    pub sacrifice_land: bool,
    /// Threshold: "Activate only if there are seven or more cards in your graveyard." (Barbarian Ring)
    pub needs_threshold: bool,
    /// "Activate only if you attacked with N or more creatures this turn." (Minas Tirith: 2)
    pub min_attackers_this_turn: u8,
    /// "Sacrifice X [subtype]s" in the cost (Chatterfang: Squirrels): `Action::ActivateX`, X up to
    /// how many you control; the engine picks which (tokens first, then the lowest mana value,
    /// the source last).
    pub sacrifice_x_of_subtype: Option<&'static str>,
    /// "Forage" in the cost (Camellia): exile three cards from your graveyard or sacrifice a Food.
    /// The engine picks: a Food if you have one, else three graveyard cards (as for delve).
    pub forage: bool,
    /// A Class's level-up ("{1}{R}: Level 2", CR 716.2): activatable only at the level before this
    /// one (and as a sorcery, `sorcery`); its effect is `Effect::ClassLevelUp`.
    pub level_up_to: u8,
    /// Activatable only while its Class is at this level or higher (0: always).
    pub min_class_level: u8,
    /// "Activate only if you control N or more artifacts." (Metallurgic Summonings: 6)
    pub min_artifacts: u8,
    /// Plot (CR 702.170): activated from your hand at sorcery speed; as the cost the card is exiled,
    /// plotted. On a later turn its owner may cast it from exile as a sorcery without paying its
    /// mana cost (`Object::plotted`).
    pub plot: bool,
    /// "Sacrifice N nonland permanents" in the cost (Bolas's Citadel: 10): the engine picks which
    /// (tokens first, then the lowest mana value, the source last).
    pub sacrifice_nonland_n: u8,
    /// Foretell's "During your turn, you may pay {2} and exile this card from your hand": an
    /// ability of the card in your hand (with `your_turn_only`); it's cast later for
    /// `ManaExt::foretell`. (A special action in the rules, CR 702.143a; here it uses the stack.)
    pub foretell: bool,
    /// Forecast (CR 702.57): "[cost], Reveal this card from your hand: [effect]. (Activate only
    /// during your upkeep and only once each turn.)" The card stays in your hand; set
    /// `once_each_turn` too.
    pub forecast: bool,
    /// "Pay life equal to the number of colors in your commanders' color identity" in the cost
    /// (War Room).
    pub life_per_commander_color: bool,
    /// "Sacrifice an artifact" in the cost: any artifact you control, kept out of the mana payment
    /// (Goblin Engineer). Activated with `Action::ActivateWith`.
    pub sacrifice_artifact: bool,
    /// "Exile a creature you control" in the cost (Food Chain): one `Action::ActivateWith` per
    /// creature you control; with `victim_in_extra` the effect sees the exiled card.
    pub exile_creature_you_control: bool,
    /// "Activate only during combat." (Najeela, the Blade-Blossom)
    pub only_during_combat: bool,
    /// "Sacrifice a [first] and a [second]" in the cost: two different permanents you control,
    /// one with each subtype (Jarad, Golgari Lich Lord: a Swamp and a Forest); the engine picks,
    /// keeping permanents that have both subtypes when it can.
    pub sacrifice_two_subtypes: Option<[&'static str; 2]>,
    /// Saddle N (CR 702.171): tap any number of other untapped creatures you control with total
    /// power N or more, as a sorcery: crew's cost (`Action::ActivateWith` names the first,
    /// `Decision::CrewTap` the rest). Scripted with `sorcery` and `victim_in_extra`;
    /// `Effect::BecomeSaddled` records the riders.
    pub saddle: u8,
    /// "Activate only if you've cast N or more instant and/or sorcery spells this turn." (Urabrask:
    /// 3), from `Game::instants_sorceries_cast`.
    pub min_instants_sorceries_this_turn: u8,
    /// "Activate only if an opponent has N or more cards in their graveyard." (Sheoldred // The
    /// True Scriptures: 8)
    pub opponent_min_graveyard: u8,
    /// "Remove N +1/+1 counters from this creature" in the cost (Walking Ballista: 1, Karlov of
    /// the Ghost Council: 6).
    pub remove_plus_counters: u8,
    /// The stack item's amount is the sacrificed creature's toughness (Miren, the Moaning Well).
    pub amount_from_victim_toughness: bool,
    /// "Remove N counters from among other artifacts, creatures, and planeswalkers you control" in
    /// the cost (Tekuthal, Inquiry Dominus: 3): the player picks each permanent
    /// (`Decision::RemoveCountersFromOthers`) before the ability is activated. Demo rule for the
    /// kind (until it's a Mode pick): the kind with the most counters on that permanent, never
    /// loyalty (or lore) while another kind is present (`Game::remove_any_counter`).
    pub remove_counters_from_others: u8,
    /// "Sacrifice an artifact or creature" in the cost (Ravenous Squirrel): one
    /// `Action::ActivateWith` per artifact or creature you control (the player's pick), kept out of
    /// the mana payment.
    pub sacrifice_artifact_or_creature: bool,
    /// Activated only while the card is in the command zone, by its owner (Derevi: "{1}{G}{W}{U}:
    /// Put Derevi onto the battlefield from the command zone.").
    pub from_command_zone: bool,
    /// Activated from your hand, exiling this card as a cost (Elvish Spirit Guide: "Exile this
    /// card from your hand: Add {G}.").
    pub from_hand_exile: bool,
    /// "Sacrifice N creatures" in the cost (Westvale Abbey: five): the player picks each one
    /// (`Decision::SacrificeCreaturesForCost`) before the ability is activated; they're
    /// sacrificed as it's activated, before its mana is paid.
    pub sacrifice_n_creatures: u8,
    /// "Remove N counters from this creature" in the cost (Glen Elendra Guardian: 1, Reaping Willow:
    /// 2): any kind; with more than one kind on it, `Game::remove_any_counter`'s demo rule picks (as
    /// Tekuthal's).
    pub remove_counters_self: u8,
    /// "Return a [subtype] you control to its owner's hand" in the cost (Quirion Ranger: a Forest;
    /// Wirewood Symbiote: an Elf): activated with `Action::ActivateWith`, that permanent the victim.
    pub return_own_subtype: Option<&'static str>,
    /// "Tap an untapped creature you control" in the cost, the player naming it: activated with
    /// `Action::ActivateWith`, that creature the victim (Earthcraft, The Shire). Summoning sickness
    /// doesn't matter (it isn't the creature's own {T}); it's tapped before the mana is paid, so it
    /// can't also tap for that mana.
    pub tap_named_creature: bool,
}

/// A loyalty ability's cost of −X (CR 606.4, 107.3): `loyalty_abilities` uses this as the cost,
/// and X is chosen as it's activated (`Decision::ChooseLoyaltyX`), up to the loyalty.
pub const LOYALTY_MINUS_X: i32 = i32::MIN;

/// The printed card. Shared by every copy; per-copy state lives on the game object.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CardDef {
    pub name: String,
    pub cost: ManaCost,
    pub types: Types,
    pub subtypes: Vec<String>,
    pub power: i32,
    pub toughness: i32,
    pub keywords: Keywords,
    /// What the permanent taps for, if it is a mana source (lands, for now).
    pub mana: Option<ManaSource>,
    /// What an instant or sorcery does when it resolves.
    pub spell: Option<Ability>,
    /// A modal spell's modes ("Choose one —"); the mode is chosen as it's cast (CR 700.2).
    pub modes: Vec<Ability>,
    /// An Aura: cast with a target (its `spell` ability) and attached to it on entering (CR 303.4).
    pub aura: bool,
    /// Triggered abilities.
    pub triggers: Vec<Trigger>,
    /// Kicker cost (CR 702.33).
    pub kicker: Option<ManaCost>,
    /// A token, which ceases to exist outside the battlefield (CR 111.7, 704.5d).
    pub token: bool,
    pub activated: Vec<Activated>,
    /// A mana source that is sacrificed when used, like a Treasure.
    pub sac_for_mana: bool,
    pub extra_cost: Option<ExtraCost>,
    pub reduction: Option<Reduction>,
    /// Flashback cost (CR 702.34).
    pub flashback: Option<ManaCost>,
    pub statics: Vec<Static>,
    /// "This ... enters tapped."
    pub enters_tapped: bool,
    /// "This creature can't block."
    pub cant_block: bool,
    /// "This creature doesn't untap during your untap step."
    pub doesnt_untap: bool,
    /// "Instant and sorcery spells you cast cost {N} less to cast."
    pub spell_discount: u8,
    /// "The second spell you cast each turn costs {n} less to cast." (Highspire Bell-Ringer)
    pub second_spell_discount: u8,
    /// "{1}: Add {A}, {B}, or {C}. Activate only once each turn." (the Devotees): the colors.
    pub filter_mana: Option<[bool; 5]>,
    /// Harmonize (Tarkir: Dragonstorm): cast from the graveyard for `flashback` (the harmonize cost),
    /// optionally tapping an untapped creature you control to pay {X} less, X its power; then exile.
    pub harmonize: bool,
    /// The Omen half (Tarkir: Dragonstorm): cast instead as this instant or sorcery; it then shuffles
    /// into its owner's library.
    pub omen: Option<Omen>,
    /// "You may cast sorcery spells and Dragon spells as though they had flash." (Whirlwing Stormbrood)
    pub flash_sorceries_and_dragons: bool,
    /// Doubling Season: tokens you'd create and counters you'd put on your permanents are doubled
    /// (CR 614.1a).
    pub doubles_tokens_and_counters: bool,
    /// Twinflame Tyrant: damage your sources would deal to an opponent or their permanents is
    /// doubled.
    pub doubles_damage_to_opponents: bool,
    /// Omniscience: you may cast spells from your hand without paying their mana costs.
    pub free_from_hand: bool,
    /// Protection from everything (CR 702.16j).
    pub protection_from_everything: bool,
    /// "If this would be put into a graveyard from anywhere, shuffle it into its owner's library
    /// instead."
    pub shuffle_instead_of_graveyard: bool,
    /// "Enters with N +1/+1 counters if ..." (a replacement effect, CR 614.1c).
    pub enters_counters: Option<(i32, Condition)>,
    /// Ward {cost} (CR 702.21).
    pub ward: Option<ManaCost>,
    /// "Attacks each combat if able" (CR 508.1d).
    pub must_attack: bool,
    /// "Can't be blocked by [subtype]".
    pub not_blocked_by: Option<&'static str>,
    /// "Creatures your opponents control enter tapped."
    pub opponents_creatures_enter_tapped: bool,
    /// "You can't lose the game and your opponents can't win the game."
    pub cant_lose: bool,
    /// "Ward—Pay N life."
    pub ward_life: Option<i32>,
    /// "This spell can't be countered."
    pub uncounterable: bool,
    /// "You have no maximum hand size."
    pub no_max_hand_size: bool,
    /// With kicker paid, this replaces `spell` (different targets).
    pub kicked_spell: Option<Ability>,
    /// "You may play an additional land on each of your turns."
    pub extra_land_drops: u8,
    /// Power and toughness each equal to the number of cards in opponents' graveyards.
    pub pt_opponents_graveyards: bool,
    /// "Hexproof from instants".
    pub hexproof_from_instants: bool,
    /// "You may pay {cost} rather than pay this spell's mana cost if there are N or more creatures
    /// on the battlefield."
    pub alt_cost: Option<(ManaCost, u8)>,
    /// "If this card is in your opening hand, you may begin the game with it on the battlefield."
    pub leyline: bool,
    /// "You may cast spells as though they had flash."
    pub grants_flash: bool,
    /// Colors, WUBRG (CR 105).
    pub colors: [bool; 5],
    /// "Enters with eight revival counters on it if you cast it."
    pub revival_on_cast: bool,
    /// "As this enters, choose a color", and "{T}: Add one mana of the chosen color".
    pub choose_color: bool,
    /// "As this enters, choose a creature type" (CR 614.12); also gives it a fellowship counter for
    /// each creature you control of that type (Banner of Kinship).
    pub choose_creature_type: bool,
    /// Vivid: one activation adds one mana of each color among permanents you control.
    pub vivid_mana: bool,
    /// Its mana can be spent only to cast spells with this subtype.
    pub mana_only_for_subtype: Option<&'static str>,
    /// "Each other [subtype] you control enters with an additional +1/+1 counter on it for each
    /// [subtype] you already control" (Giada).
    pub other_subtype_enter_bonus: Option<&'static str>,
    /// "You may cast this card from your graveyard by removing N counters from among creatures you
    /// control in addition to paying its other costs."
    pub graveyard_cast_remove_counters: u8,
    /// Its "{T}: Add one mana of any color" can pay only for creature spells of its chosen
    /// creature type or abilities of creature sources of that type (Secluded Courtyard). Its plain
    /// mana is in `mana`.
    pub any_color_for_chosen_type: bool,
    /// Muldrotha: during each of your turns, play a land and cast a permanent spell of each
    /// permanent type from your graveyard.
    pub graveyard_permanents: bool,
    /// Stages it can "become" (Kellan): stage `n` (1-based) uses `stages[n - 1]`.
    pub stages: Vec<Stage>,
    pub mana_of_chosen_color: bool,
    /// Its controller may play opponents' stashed cards from exile during their turn (Tinybones).
    pub stash_player: bool,
    /// "You have hexproof."
    pub player_hexproof: bool,
    /// "Prevent all noncombat damage that would be dealt to other creatures you control."
    pub prevent_noncombat_to_others: bool,
    /// "{T}: Add {C} for each [subtype] you control": the source makes this many of its mana.
    pub mana_per_subtype: Option<&'static str>,
    /// "This creature enters with X +1/+1 counters on it."
    pub enters_with_x_counters: bool,
    /// "+1/+1 for each creature card in your graveyard."
    pub pt_per_graveyard_creature: bool,
    /// Starting loyalty (CR 306.5b) and loyalty abilities as (cost, ability) (CR 606).
    pub loyalty: Option<i32>,
    pub loyalty_abilities: Vec<(i32, Ability)>,
    /// Behold a Dragon (Tarkir: Dragonstorm): choose a Dragon you control or reveal a Dragon card from
    /// your hand, as an optional additional cost. Cast with it, the spell is "kicked" (`kicked_spell`).
    pub behold: Behold,
    /// "Each <subtype> you control enters with an additional +1/+1 counter on it" (Dragonstorm Globe).
    pub subtype_enters_with_counter: Option<&'static str>,
    /// "This land enters tapped unless you control a [type] or a [type]."
    pub enters_tapped_unless: Option<[&'static str; 2]>,
    /// "Creatures with power less than this creature's power can't block it."
    pub not_blocked_by_lesser_power: bool,
    /// "Activated abilities of artifacts, creatures, and planeswalkers can't be activated."
    pub stops_activated_abilities: bool,
    /// "As this enters, choose [A] or [B]": a mode, 0 or 1, chosen as it resolves (the Sieges).
    pub enter_modes: bool,
    /// In this chosen mode: "If a creature attacking causes a triggered ability of a permanent you
    /// control to trigger, that ability triggers an additional time." (Windcrag Siege)
    pub attack_triggers_again_in_mode: Option<u8>,
    /// In this chosen mode: "You may play lands from your graveyard." (Glacierwood Siege)
    pub graveyard_lands_in_mode: Option<u8>,
    /// Its mana ability works only while you control a permanent with this subtype ("Activate only
    /// if you control a Dragon", Mox Jasper).
    pub mana_only_if_you_control: Option<&'static str>,
    /// "{T}: Add an amount of {G} equal to this creature's power" (one activation, that many mana).
    pub mana_equal_power: bool,
    /// "You may cast [subtype] spells without paying their mana costs" (Dracogenesis).
    pub free_subtype_spells: Option<&'static str>,
    /// "Enchanted creature doesn't untap during its controller's untap step."
    pub enchanted_doesnt_untap: bool,
    /// "Spend this mana only to cast a creature spell."
    pub mana_only_for_creatures: bool,
    /// Neriv: "If a creature you control that entered this turn would deal damage, it deals twice
    /// that much damage instead."
    pub doubles_damage_of_new_creatures: bool,
    /// "If one or more tokens would be created under your control, twice that many of those tokens
    /// are created instead." (tokens only, not counters)
    pub doubles_tokens: bool,
    /// "You may cast this card from your graveyard. If you do, it enters with a finality counter on
    /// it."
    pub cast_from_graveyard_finality: bool,
    /// "During your turn, spells you cast cost {1} less to cast for each creature you control with
    /// power 4 or greater." (Temur Battlecrier)
    pub your_turn_discount_per_big_creature: bool,
    /// Its mana ability also costs "Tap an untapped creature you control" (Dragonbroods' Relic). The
    /// payer taps an untapped creature that isn't itself a mana source.
    pub mana_taps_creature: bool,
    /// "{T}: Add one mana of any color. Spend this mana only to cast a Dragon spell or an Omen
    /// spell." Its plain mana is in `mana` (Maelstrom of the Spirit Dragon).
    pub any_color_for_dragon_or_omen: bool,
    /// "Your opponents can't cast spells during your turn." (Voice of Victory)
    pub opponents_cant_cast_your_turn: bool,
    /// "During your end step, Warrior tokens you control have 'This token can't be sacrificed.'"
    pub warrior_tokens_unsacrificeable_end_step: bool,
    /// "Creature spells you cast cost {N} less to cast." (The Sibsig Ceremony)
    pub creature_spell_discount: u8,
    /// Its color identity (CR 903.4), WUBRG: set from the rules text when the set loads.
    pub color_identity: [bool; 5],
    /// "{T}: Add one mana of any color in your commander's color identity." (no commander: no mana)
    pub mana_commander_identity: bool,
    /// "{T}: Add one mana of any color that a land an opponent controls could produce."
    pub mana_opponent_land_colors: bool,
    /// One activation of its mana ability makes this many mana (Sol Ring: 2); 0 means 1.
    pub mana_amount: u8,
    /// Bello: "During your turn, each non-Equipment artifact and non-Aura enchantment you control
    /// with mana value 4 or greater is a 4/4 Elemental creature in addition to its other types and
    /// has indestructible, haste, and \"Whenever this creature deals combat damage to a player, draw
    /// a card.\"" (its trigger number 0 is that draw)
    pub animates_big_noncreatures_your_turn: bool,
    /// "Equipped creature ... has hexproof from monocolored." (Dragonfire Blade)
    pub equipped_hexproof_from_monocolored: bool,
    /// "This land enters tapped unless you control N or more basic lands." (0: no such rule)
    pub enters_tapped_unless_basics: u8,
    /// "This land enters tapped unless you control N or fewer other lands."
    pub enters_tapped_unless_at_most_lands: Option<u8>,
    /// "Offspring {cost}": an optional additional cost (cast with `Action::CastKicked`); if paid,
    /// when it enters, create a 1/1 token copy of it.
    pub offspring: Option<ManaCost>,
    /// Zinnia: "Creature spells you cast gain offspring {cost} as you cast them."
    pub grants_offspring: Option<ManaCost>,
    /// Decayed (CR 702.147): it can't block; when it attacks, sacrifice it at end of combat.
    pub decayed: bool,
    /// "Ward—Discard a card." (Scavenger Regent)
    pub ward_discard: bool,
    /// "Spells you cast have delve." (Teval)
    pub spells_have_delve: bool,
    /// "You may have this creature enter as a copy of any creature on the battlefield."
    pub enter_as_copy: bool,
    /// A pain land: "{T}: Add [A] or [B]. This land deals 1 damage to you." ({C} is free).
    pub pain_colored: bool,
    /// The Thriving lands: "As it enters, choose a color other than {this}. {T}: Add {this} or one
    /// mana of the chosen color."
    pub thriving_color: Option<crate::Color>,
    /// Gratuitous Violence: "If a creature you control would deal damage to a permanent or player,
    /// it deals double that damage instead."
    pub doubles_your_creatures_damage: bool,
    /// "{cost}, {T}: Add [two mana]." (Signets, filter lands): the cost and the two mana it makes.
    pub paid_mana: Option<(ManaCost, [ManaSource; 2])>,
    /// The snarls: "As this land enters, you may reveal a [A] or [B] card from your hand. If you don't, it
    /// enters tapped." (revealed whenever there is one).
    pub enters_tapped_unless_reveal: Option<[&'static str; 2]>,
    /// "Power and toughness are each equal to the number of cards in your hand."
    pub pt_hand_size: bool,
    /// "Creatures you control with power or toughness 1 or less can't be blocked." (Tetsuko)
    pub small_creatures_unblockable: bool,
    /// Vanishing N (CR 702.63): enters with N time counters (the upkeep removal is a trigger).
    pub vanishing: u8,
    /// "Creature spells you cast with power 4 or greater cost {N} less to cast." (Goreclaw)
    pub big_creature_spell_discount: u8,
    /// "Each other non-[subtype] creature you control enters with an additional +1/+1 counter on
    /// it." (Grumgully: "Human")
    pub other_non_subtype_enters_with_counter: Option<&'static str>,
    /// "Each creature you control with a counter on it has \"{T}: Add {G}.\"" (Rishkar)
    pub counter_creatures_tap_for_green: bool,
    /// "Your maximum hand size is N." (0: the usual seven)
    pub max_hand_size: u8,
    /// "This creature enters with a number of +1/+1 counters on it equal to one plus the number of
    /// other creatures you control." (Boss's Chauffeur)
    pub enters_counters_one_plus_others: bool,
    /// Ascend (CR 702.131): with ten or more permanents, you get the city's blessing for the game.
    pub ascend: bool,
    /// "Each player may play an additional land on each of their turns." (Rites of Flourishing)
    pub extra_land_drop_each_player: bool,
    /// Its colored mana works only while you control a permanent with this subtype (Tainted Wood:
    /// "{T}: Add {B} or {G}. Activate only if you control a Swamp."); its {C} always works.
    pub colored_mana_only_if_you_control: Option<&'static str>,
    /// Its mana ability also deals this much damage to you each time it's activated (Ancient Tomb).
    pub mana_tap_damage: u8,
    /// Fiery Emancipation: damage a source you control would deal is tripled.
    pub triples_your_damage: bool,
    /// "Each spell you cast that's red or green costs {N} less to cast." (Goblin Anarchomancer)
    pub red_green_spell_discount: u8,
    /// "As this land enters, you may pay 2 life. If you don't, it enters tapped." (shock lands)
    pub shock_land: bool,
    /// "You may play lands from your graveyard."
    pub lands_from_graveyard: bool,
    /// "You may play lands from the top of your library."
    pub lands_from_top: bool,
    /// "You may cast spells with mana value N or greater from the top of your library." (Glarb: 4)
    pub top_spells_min_mana_value: Option<u32>,
    /// "You may cast [subtype] spells from the top of your library." (Traveling Chocobo: Bird)
    pub top_spells_subtype: Option<&'static str>,
    /// "If a [kind] entering causes a triggered ability of a permanent you control to trigger, that
    /// ability triggers an additional time."
    pub entering_triggers_again: Option<EntryDoubler>,
    /// "Untap all permanents you control during each other player's untap step." (Seedborn Muse)
    pub untaps_on_others_untap: bool,
    /// Power and toughness each equal to the number of lands you control.
    pub pt_lands: bool,
    /// The Theros gods: "As long as your devotion to [colors] is less than N, this isn't a creature."
    pub creature_unless_devotion: Option<([bool; 5], u8)>,
    /// Its mana ability also gains you this much life each time it's activated (The Great Henge).
    pub mana_tap_life: i32,
    /// Its mana can also be this color while you control a permanent with one of these subtypes (Thornspire Verge).
    pub extra_color_if_you_control: Option<(crate::Color, [&'static str; 2])>,
    /// "{T}: Add one mana of any type that a land you control could produce." (Reflecting Pool)
    pub mana_own_land_colors: bool,
    /// Power and toughness each equal to half the highest life total among your opponents, rounded up (Malignus).
    pub pt_half_highest_opponent_life: bool,
    /// "Damage that would be dealt by this creature can't be prevented."
    pub damage_cant_be_prevented: bool,
    /// "Combat damage that would be dealt by creatures you control can't be prevented." (Questing Beast)
    pub your_combat_damage_cant_be_prevented: bool,
    /// "Can't be blocked by creatures with power N or less."
    pub not_blocked_by_power_at_most: Option<i32>,
    /// Zilortha: lethal damage to creatures you control is determined by their power.
    pub lethal_damage_by_power: bool,
    /// "This creature can't attack a player it has already attacked this turn." (two players: it attacks once a turn)
    pub cant_attack_twice_per_turn: bool,
    /// "Each land is a Forest in addition to its other land types." (lands with plain mana also tap for {G})
    pub lands_are_forests: bool,
    /// Unleash (CR 702.98): "you may have this creature enter with a +1/+1 counter on it" (asked as
    /// the spell resolves, like the Sieges' modes); it can't block while it has one (its script).
    pub unleash: bool,
    /// "This creature can't attack or block unless you have the city's blessing."
    pub fights_only_with_blessing: bool,
    /// "Lands you control are every basic land type in addition to their other types."
    pub lands_every_basic_type: bool,
    /// `enter_as_copy` of any nonland permanent, not just a creature (Clever Impersonator).
    pub enter_as_copy_nonland: bool,
    /// "Protection from planeswalkers and from [subtype]" (Greensleeves: Wizards).
    pub protection_from_planeswalkers_and: Option<&'static str>,
    /// Entwine {cost} (CR 702.42): the last of its modes is all the others, for this much more.
    pub entwine: Option<ManaCost>,
    /// "Cast this spell only during combat on your turn."
    pub cast_only_during_your_combat: bool,
    /// Ferocious mana: one activation makes this many mana while you control a creature with power 4
    /// or greater (Fanatic of Rhonas: {G}{G}{G}{G}); otherwise its plain `mana_amount`.
    pub ferocious_mana_amount: u8,
    /// "Each creature you control gets +1/+0 for each time it has attacked this turn." (Moraug)
    pub power_per_attack_this_turn: bool,
    /// Defiler of Vigor: "As an additional cost to cast green permanent spells, you may pay 2 life. Those
    /// spells cost {G} less": cast with `Action::CastAlternative`.
    pub green_permanents_pay_life_for_g: bool,
    /// Roaming Throne: "This creature is the chosen type in addition to its other types" and "If a
    /// triggered ability of another creature you control of the chosen type triggers, it triggers an
    /// additional time."
    pub doubles_chosen_type_triggers: bool,
    /// "{T}: Add one mana of any color. Spend this mana only to cast a legendary spell, and that
    /// spell can't be countered." Its plain mana is in `mana` (Delighted Halfling).
    pub any_color_for_legendary: bool,
    /// "Flashback—Sacrifice N creatures." (Dread Return; `flashback` is then {0}).
    pub flashback_sacrifice: u8,
    /// "If it's not your turn, you may exile a [color] card from your hand rather than pay this
    /// spell's mana cost." (Force of Vigor: green, WUBRG index 4); cast with `Action::CastWith`.
    pub pitch_not_your_turn: Option<u8>,
    /// Its last mode (both modes together) can be chosen only while you control a commander
    /// (Will of the Abzan).
    pub last_mode_needs_commander: bool,
    /// A transforming double-faced card's back face (CR 712): what the permanent is while it's
    /// transformed (`Object::transformed`). Its own `back` is `None`.
    pub back: Option<Box<CardDef>>,
    /// `alt_cost` is offered only while you control a commander (Deflecting Swat: "If you control a
    /// commander, you may cast this spell without paying its mana cost.").
    pub alt_cost_needs_commander: bool,
    /// "You may pay N {E} rather than pay the mana cost for permanent spells you cast." (Nissa,
    /// Worldsoul Speaker: 8); cast with `Action::CastWith(card, this permanent)`.
    pub permanents_for_energy: u8,
    /// Phyrexian pips in its mana cost, by color (CR 107.4f): counted in `cost.colored`, and each
    /// may be paid with 2 life instead, all of them at once with `Action::CastAlternative`.
    pub phyrexian: [u8; 5],
    /// Enters as a copy of an artifact or creature, "except it's an artifact in addition to its
    /// other types" (Phyrexian Metamorph).
    pub copy_artifact: bool,
    /// As a copy it keeps its own name and is legendary, and has its `copy_only` activated
    /// abilities (Sakashima the Impostor).
    pub copy_keeps_name: bool,
    /// Enters as a copy of another creature you control only (Sakashima of a Thousand Faces).
    pub copy_only_yours: bool,
    /// As a copy it keeps its own other abilities: `no_legend_rule` and `copy_only` activated
    /// abilities (Sakashima of a Thousand Faces).
    pub copy_keeps_abilities: bool,
    /// "The \"legend rule\" doesn't apply to permanents you control."
    pub no_legend_rule: bool,
    /// Split second (CR 702.61): while it's on the stack, players can't cast spells or activate
    /// abilities that aren't mana abilities.
    pub split_second: bool,
    /// "Activated abilities of artifacts can't be activated." (Collector Ouphe)
    pub stops_artifact_abilities: bool,
    /// "{T}: Add one mana of any color among legendary creatures and planeswalkers you control." (Mox
    /// Amber; none: no mana)
    pub mana_legendary_colors: bool,
    /// "You don't lose unspent green mana as steps and phases end." and "+1/+1 for each unspent
    /// green mana you have." (Omnath, Locus of Mana)
    pub keeps_green_mana: bool,
    /// Spree (CR 702.172): the additional cost of each mode in `modes` (a mode there is one
    /// combination of the chosen + modes).
    pub mode_costs: Vec<ManaCost>,
    /// Plaza of Heroes: "{T}: Add one mana of any color. Spend this mana only to cast a legendary
    /// spell." and "{T}: Add one mana of any color among legendary permanents you control." (its
    /// {C} is in `mana`).
    pub plaza_mana: bool,
    /// Umbra armor (CR 702.89): if the enchanted creature would be destroyed, instead remove all
    /// damage from it and destroy this Aura.
    pub umbra_armor: bool,
    /// A modal double-faced card (CR 712.12): its `back` is played or cast instead of its front (a
    /// land back is played as a land, with `Object::transformed` set as it enters).
    pub modal_back: bool,
    /// Soulbond (CR 702.95) with "As long as this is paired with another creature, each of those
    /// creatures has \"{1}{U}: Exile this creature, then return it to the battlefield under your
    /// control.\"" (Deadeye Navigator): the granted ability (see `Game::activated_of`).
    pub soulbond_flicker: bool,
    /// "Green spells you cast cost {N} less to cast." (Emerald Medallion)
    pub green_spell_discount: u8,
    /// "Whenever you tap a Forest for mana, add an additional {G}." (Nissa, Who Shakes the World)
    pub forests_add_g: bool,
    /// Buyback by sacrificing N permanents with this subtype (Walk the Aeons: three Islands), cast
    /// with `Action::CastKicked`; it returns to its owner's hand as it resolves.
    pub buyback_sacrifice: Option<(&'static str, u8)>,
    /// X is the amount of {S} spent to cast it (Blessing of Frost): stored as the stack entry's X.
    pub x_is_snow_spent: bool,
    /// "{T}: Add {G} for each creature you control." (Circle of Dreams Druid)
    pub mana_per_creature: bool,
    /// Its mana ability works only while you control at least this many lands (Temple of the False
    /// God: 5); 0: always.
    pub mana_only_with_lands: u8,
    /// "Shuffle this card into its owner's library" as it resolves (Green Sun's Zenith).
    pub shuffle_on_resolve: bool,
    /// "Ward—Sacrifice N permanents." (Ulamog, the Defiler: 2); 0: none.
    pub ward_sacrifice: u8,
    /// "Enters with a number of +1/+1 counters on it equal to the greatest mana value among cards
    /// in exile." (Ulamog, the Defiler)
    pub enters_counters_greatest_exiled_mv: bool,
    /// Infect (CR 702.90), printed (Blightsteel Colossus).
    pub infect: bool,
    /// "As a historic permanent you control enters, it becomes a 7/7 Dinosaur creature in addition
    /// to its other types." (Displaced Dinosaurs)
    pub historic_become_dinosaurs: bool,
    /// Vorinclex, Voice of Hunger: your lands make twice the mana, and an opponent's land tapped for
    /// mana doesn't untap during its controller's next untap step.
    pub vorinclex: bool,
    /// "Spells your opponents cast that target this creature cost an additional N life to cast."
    /// (Terror of the Peaks: 3)
    pub targeted_life_tax: i32,
    /// "Red spells you cast cost {N} less to cast." (The Fire Crystal)
    pub red_spell_discount: u8,
    /// Vigor: "If damage would be dealt to another creature you control, prevent that damage. Put
    /// a +1/+1 counter on that creature for each 1 damage prevented this way."
    pub damage_to_yours_becomes_counters: bool,
    /// Its mana ability makes one mana for each enchantment you control (Serra's Sanctum, Sanctum
    /// Weaver), instead of one.
    pub mana_per_enchantment: bool,
    /// "During your turn, your opponents can't ... activate abilities of artifacts, creatures, or
    /// enchantments." (Grand Abolisher; the casting half is `opponents_cant_cast_your_turn`)
    pub opponents_cant_activate_your_turn: bool,
    /// "Protection from <subtype>s" (Yawgmoth: Humans).
    pub protection_from_subtype: Option<&'static str>,
    /// "As long as your life total is less than or equal to half your starting life total, this
    /// has indestructible." (Myrkul)
    pub indestructible_at_half_life: bool,
    /// "Creatures your opponents control have base toughness 1." (Maha)
    pub opponents_creatures_base_toughness_one: bool,
    /// With `enter_as_copy`: the copy lasts until end of turn and has haste (Cursed Mirror).
    pub copy_until_eot_haste: bool,
    /// "You may cast creature spells from the top of your library." (Vivien, Monsters' Advocate)
    pub top_spells_creatures: bool,
    /// Blitz (CR 702.152), its cost in `alt_cost`: cast for it, the permanent gains haste and "When
    /// this creature dies, draw a card" (its trigger number `.0`), and is sacrificed at the next
    /// end step.
    pub blitz: Option<usize>,
    /// Soulbond with "each of those creatures has 'At the beginning of combat on your turn, create a
    /// token that's a copy of this creature, except it has haste and loses soulbond. Exile it at end
    /// of combat.'" (Mirage Phalanx; the pairing works as for `soulbond_flicker`).
    pub soulbond_combat_copies: bool,
    /// "Creatures you control have \"{T}: Add one mana of any color.\"" (Enduring Vitality)
    pub creatures_tap_any_color: bool,
    /// "Tokens you control have \"{T}: Add {G}.\"" (Jaheira, Friend of the Forest)
    pub tokens_tap_for_green: bool,
    /// "Whenever you tap a land for mana, add one mana of any type that land produced." (Mirari's Wake)
    pub lands_add_same: bool,
    /// "If you would lose the game, instead exile this and your life total becomes 1." (The Golden
    /// Throne)
    pub lose_replacement: bool,
    /// Warp (Exalted Sunborn): cast from your hand for this with `Action::CastAlternative`; exiled at
    /// the next end step, then castable from exile.
    pub warp: Option<ManaCost>,
    /// Morph (CR 702.37): cast face down for {3} with `Action::CastAlternative`; turned face up for
    /// this cost.
    pub morph: Option<ManaCost>,
    /// Changeling: every creature type (CR 702.73).
    pub changeling: bool,
    /// An adventurer card (CR 715): its Adventure is cast as the `omen` half; on resolving it's exiled,
    /// and its owner may cast the card itself from exile later.
    pub adventure: bool,
    /// "Enchanted permanent's activated abilities can't be activated." (Hold for Questioning)
    /// "As this enters, choose a card type." and "You and creatures you control have protection
    /// from the chosen card type." (Serra's Emissary).
    pub protection_chosen_card_type: bool,
    pub enchanted_stops_activations: bool,
    /// "If one or more +1/+1 counters would be put on a creature you control, that many plus one
    /// ... instead." (Hardened Scales)
    pub counters_plus_one: bool,
    /// "If an opponent would lose life during your turn, they lose twice that much life instead."
    /// (Bloodletter of Aclazotz)
    pub opponents_lose_double_your_turn: bool,
    /// "If you would gain life, you gain twice that much life instead." (Alhammarret's Archive)
    pub life_gain_doubled: bool,
    /// "If you would draw a card except the first one you draw in each of your draw steps, draw two
    /// cards instead." (Alhammarret's Archive, Teferi's Ageless Insight)
    pub extra_draws_doubled: bool,
    /// "If one or more tokens would be created under your control, those tokens plus an additional
    /// [token] are created instead": this token definition, once per creation (Peregrin Took, Quina).
    pub tokens_plus_one: Option<usize>,
    /// "... those tokens plus that many [tokens] are created instead": one of this token for each
    /// (Chatterfang).
    pub tokens_plus_each: Option<usize>,
    /// "If a card would be put into an opponent's graveyard from anywhere, instead exile it with a
    /// void counter on it." (Dauthi Voidwalker)
    pub opponents_cards_exiled_with_void: bool,
    /// "Nonbasic lands are Mountains." (Blood Moon, CR 613.1d, 305.7)
    pub nonbasic_lands_are_mountains: bool,
    /// "Each land is a Swamp in addition to its other land types." (Urborg, Tomb of Yawgmoth)
    pub lands_are_swamps: bool,
    /// "You may activate abilities of creatures you control as though those creatures had haste."
    /// (Thousand-Year Elixir)
    pub creatures_abilities_haste: bool,
    /// `ward_sacrifice` counts nonland permanents only (Valgavoth: "Ward—Sacrifice three nonland
    /// permanents.").
    pub ward_sacrifice_nonland: bool,
    /// Animate Dead, Dance of the Dead, Necromancy: it reanimates its target creature card as it
    /// enters (an Aura spell's target, or its enters trigger's) and is attached to it; when it
    /// leaves the battlefield, that creature's controller sacrifices the creature.
    pub reanimating_aura: bool,
    /// The reanimated creature enters tapped (Dance of the Dead).
    pub reanimates_tapped: bool,
    /// "If you cast it any time a sorcery couldn't have been cast, ... sacrifices it at the
    /// beginning of the next cleanup step." (Necromancy: at the next end step here)
    pub flash_cast_sacrificed: bool,
    /// Mana, costs and keywords (the d1 theme), grouped so adding one touches only [`ManaExt`].
    pub mana_ext: ManaExt,
    /// "... don't cause abilities to trigger" (Hushbringer, Doorkeeper Thrull, Elesh Norn).
    pub suppresses_triggers: Option<Suppression>,
    /// "If a triggered ability of [a kind of source] you control triggers, that ability triggers an
    /// additional time" (Harmonic Prodigy, Delney).
    pub doubles_triggers_of: Option<TriggerDoubler>,
    /// "Skip your draw step." (Damia, Sage of Stone)
    pub skip_draw_step: bool,
    /// "Untap all creatures you control during each other player's untap step." (Drumbellower)
    pub creatures_untap_on_others_untap: bool,
    /// "Noncreature spells with mana value 4 or greater can't be cast. Noncreature spells with {X} in
    /// their mana costs can't be cast." (Gaddock Teeg)
    pub teeg_lock: bool,
    /// "Your maximum hand size is increased by N." (Trusted Advisor)
    pub max_hand_plus: u8,
    /// "Each opponent's maximum hand size is reduced by N." (Jin-Gitaxias, Core Augur)
    pub opponents_max_hand_minus: u8,
    /// "If you would draw a card while your library has no cards in it, you win the game instead."
    /// (Laboratory Maniac)
    pub win_on_empty_draw: bool,
    /// An Aura with "Enchant player" whose enchanted player gets an additional upkeep step after
    /// their first upkeep each turn (Paradox Haze).
    pub extra_upkeep_for_enchanted: bool,
    /// The d4 subsystems' pieces (dungeons, Backgrounds, split cards), grouped so adding one
    /// touches only [`SubsystemExt`].
    pub subsystems: SubsystemExt,
    /// "All Swamps are 1/1 black creatures that are still lands." (Kormus Bell; give it an empty
    /// static too, so it's among the static sources)
    pub swamps_are_creatures: bool,
    /// "Creatures can't attack you unless their controller pays {N} for each creature they
    /// control that's attacking you." (Ghostly Prison, Propaganda: 2), see `Game::attack_tax`.
    pub attack_tax: u8,
    /// The attack tax is {X} per creature, X the enchantments you control (Sphere of Safety).
    pub attack_tax_per_enchantment: bool,
    /// Academy Manufactor: "If you would create a Clue, Food, or Treasure token, instead create one of
    /// each." (the three token definitions)
    pub clue_food_treasure: Option<[usize; 3]>,
    /// Kira: "Creatures you control have 'Whenever this creature becomes the target of a spell or
    /// ability for the first time each turn, counter that spell or ability.'" (its trigger number)
    pub kira_trigger: Option<usize>,
    /// A modal trigger's modes can each be chosen once each turn ("choose one that hasn't been
    /// chosen this turn", Monument to Endurance); see `Game::modes_chosen`.
    pub modes_once_each_turn: bool,
    /// "If you would lose unspent mana, that mana becomes colorless instead." (Kruphix, Horizon Stone)
    pub unspent_mana_becomes_colorless: bool,
    /// "Cast this spell only during your end step." (Necrologia)
    pub cast_only_during_your_end_step: bool,
    /// The attack tax applies only to attacks on you, not on your planeswalkers (Ghostly Prison,
    /// Propaganda: "Creatures can't attack you unless ...").
    pub attack_tax_player_only: bool,
    /// "Cast this spell only during the declare blockers step on an opponent's turn."
    /// (Illusionist's Gambit)
    pub cast_only_during_opponents_declare_blockers: bool,
    /// "Foods you control have all activated abilities of all creature cards exiled with this
    /// creature." (Hazel's Brewmaster); see `Game::exiled_with`.
    pub foods_have_exiled_abilities: bool,
    /// The Mac C objects-and-zones pieces (copy exceptions, ...), grouped so adding one touches
    /// only [`ZonesExt`].
    pub zones: ZonesExt,
    /// "Your opponents can't cast spells with even mana values." and "Your opponents can't block
    /// with creatures with even mana values." (Void Winnower)
    pub opponents_even_locked: bool,
    /// "Enchanted creature ... is goaded." (Martial Impetus): see `Game::is_goaded`.
    pub goads_enchanted: bool,
    /// Activated abilities lands you control have (Bootleggers' Stash: "{T}: Create a Treasure
    /// token.").
    pub lands_you_control_have: Vec<Activated>,
    /// Activated abilities creatures of this subtype you control have (The Odd Acorn Gang:
    /// Squirrels).
    pub subtype_you_control_has: Option<(&'static str, Vec<Activated>)>,
    /// Aftermath (CR 702.127): the split card's right half (`omen`) is cast only from the
    /// graveyard, and exiled as it resolves; the left half isn't cast from the graveyard.
    pub aftermath: bool,
    /// Bolas's Citadel: "You may play lands and cast spells from the top of your library. If you
    /// cast a spell this way, pay life equal to its mana value rather than pay its mana cost."
    pub citadel: bool,
    /// "[This] can't attack unless you control N or more artifacts." (Gadrak: 4)
    pub attacks_only_with_artifacts: u8,
    /// Xorn: "If you would create one or more Treasure tokens, instead create those tokens plus an
    /// additional Treasure token." (the Treasure definition)
    pub treasures_plus_one: Option<usize>,
    /// Silent Arbiter: "No more than one creature can attack each combat. No more than one creature
    /// can block each combat."
    pub one_attacker_one_blocker: bool,
    /// "Colorless spells you cast cost {N} less to cast." (Ugin, the Ineffable: 2), in `spell_cost`.
    pub colorless_spell_discount: u8,
    /// Toxic N (CR 702.164): a player dealt combat damage by it also gets N poison counters
    /// (Bloodroot Apothecary: 2).
    pub toxic: u8,
    /// An Aura or Equipment: "Enchanted creature has ward {N}." (Sheltered by Ghosts: {2}), in
    /// `Game::ward_check` with the creature's own ward.
    pub enchanted_ward: Option<ManaCost>,
    /// Gisa, Glorious Resurrector: "If a creature an opponent controls would die, exile it
    /// instead." (in `move_to`; the card is recorded in `Game::exiled_with`).
    pub opponents_creatures_die_exiled: bool,
    /// A Class's token replacements, as (the level it needs, the token it replaces, the token made
    /// instead), each applied once in order (Fisher's Talent: level 2 Fish to Shark, level 3 Shark
    /// to Octopus), in `Game::create_token`.
    pub class_token_replacements: Vec<(u8, usize, usize)>,
    /// The Immortal Sun: "Players can't activate planeswalkers' loyalty abilities." (in
    /// `Game::loyalty_actions`).
    pub players_cant_activate_loyalty: bool,
    /// "Your opponents can't gain life." (Archfiend of Despair), in `Game::gain_life`.
    pub opponents_cant_gain_life: bool,
    /// Gisela, Blade of Goldnight: "If a source would deal damage to an opponent or a permanent an
    /// opponent controls, that source deals double that damage to that player or permanent instead.
    /// If a source would deal damage to you or a permanent you control, prevent half that damage,
    /// rounded up." Any source, in `Game::deal_damage` after the other doublers.
    pub prevents_half_damage_to_you_and_yours: bool,
    /// Sower of Discord, with both players chosen (two players): "Whenever damage is dealt to one of
    /// the chosen players, the other chosen player also loses that much life." (its trigger number)
    pub damage_to_a_player_other_loses: Option<usize>,
    /// Cavern of Souls, with `any_color_for_chosen_type`: its any-color mana is only for creature
    /// spells of the chosen type (not abilities), and a spell it pays for can't be countered.
    pub chosen_type_mana_uncounterable: bool,
    /// Nyxbloom Ancient: "If you tap a permanent for mana, it produces three times as much of that
    /// mana instead." (in `untapped_sources`; abilities that add mana without tapping aren't).
    pub triples_mana_tapped: bool,
    /// Gourmand's Talent: "During your turn, artifacts you control are Foods in addition to their
    /// other types" (see `Game::has_subtype`; the Food ability is its `subtype_you_control_has`).
    pub your_artifacts_are_food_your_turn: bool,
    /// Ygra, Eater of All: "Other creatures are Food artifacts in addition to their other types"
    /// (`Game::with_kormus_bell` and `Game::has_subtype`). Give the card a static too, so it's among
    /// the static sources.
    pub other_creatures_are_food_artifacts: bool,
    /// Sigarda, Host of Herons: "Spells and abilities your opponents control can't cause you to
    /// sacrifice permanents." (`Game::forced_sacrifice_blocked`, in `cant_be_sacrificed`).
    pub opponents_cant_make_you_sacrifice: bool,
    /// Valgavoth, Terror Eater: "If a card you didn't control would be put into an opponent's
    /// graveyard from anywhere, exile it instead. During your turn, you may play cards exiled with
    /// Valgavoth. If you cast a spell this way, pay life equal to its mana value rather than pay its
    /// mana cost." (`move_to`, `Game::exiled_with`, `Game::valgavoth_exiled`).
    pub exiles_opponents_cards_to_play: bool,
    /// "You may activate loyalty abilities of [this] on any player's turn any time you could cast an
    /// instant." (Teferi, Master of Time), in `Game::loyalty_actions`.
    pub loyalty_at_instant_speed: bool,
    /// Activated abilities every other creature has, whoever controls it (Ygra, Eater of All: "{2},
    /// {T}, Sacrifice this permanent: You gain 3 life."), in `Game::borrowed_activated`.
    pub other_creatures_have: Vec<Activated>,
    /// "Creature spells you control can't be countered." (Rhythm of the Wild, Surrak Dragonclaw), in
    /// `Game::uncounterable`.
    pub your_creature_spells_uncounterable: bool,
    /// "[Color] spells you control can't be countered." (Allosaurus Shepherd: green, WUBRG index 4).
    pub your_color_spells_uncounterable: Option<u8>,
    /// "Nontoken creatures you control have riot." (Rhythm of the Wild): a nontoken creature spell
    /// you control resolving onto the battlefield asks `Decision::Riot`.
    pub your_nontoken_creatures_have_riot: bool,
    /// Plague Drone: "If an opponent would gain life, that player loses that much life instead."
    pub opponents_life_gain_becomes_loss: bool,
    /// Narset, Parter of Veils: "Each opponent can't draw more than one card each turn." (in `draw`)
    pub opponents_draw_at_most_one: bool,
    /// Overload (CR 702.96): the spell cast for its `alt_cost` (with no creature requirement) does
    /// this instead ("each" for "target"; Damn).
    pub overload_spell: Option<crate::effect::Ability>,
    /// Metallic Mimic: "Each other creature you control of the chosen type enters with an
    /// additional +1/+1 counter on it." (its `chosen_type`, in the enter path)
    pub others_of_chosen_type_enter_with_counter: bool,
    /// Ojer Taq: "If one or more creature tokens would be created under your control, three times
    /// that many of those tokens are created instead." (`Game::creature_token_tripling`)
    pub triples_creature_tokens: bool,
    /// The Necrobloom: "Land cards in your graveyard have dredge N." (2), see `Game::dredge_options`.
    pub lands_in_graveyard_have_dredge: Option<u8>,
    /// Mirrormind Crown: "As long as this Equipment is attached to a creature, the first time you
    /// would create one or more tokens each turn, you may instead create that many tokens that are
    /// copies of equipped creature." (`Game::mirrormind_crown`; the engine always takes the copies)
    pub first_tokens_copy_equipped: bool,
    /// "[This] can't attack or block unless a creature died under your control this turn." (Bontu
    /// the Glorified), from `Game::creature_died_under`.
    pub fights_only_if_creature_died: bool,
    /// Tandem Lookout: while paired (soulbond), each of the pair has "Whenever this creature deals
    /// damage to an opponent, draw a card.": this card's `When::PairedDealsDamageToOpponent` trigger.
    pub soulbond_damage_draw: bool,
    /// "Each [subtype] and [subtype] you control has \"{T}: Add {G}.\"" (Brightcap Badger:
    /// Fungus, Saproling), in the mana-source pass.
    pub your_subtypes_tap_for_green: &'static [&'static str],
    /// "If you would proliferate, proliferate twice instead." (Tekuthal, Inquiry Dominus), in
    /// `Effect::Proliferate`.
    pub proliferate_twice: bool,
    /// "[This] can't be the target of abilities your opponents control." (Shanna, Sisay's
    /// Legacy), in `target_legal`.
    pub untargetable_by_opponents_abilities: bool,
    /// An Aura: "Enchanted creature ... is a [type]" losing all other creature types (Noggle the
    /// Mind: "Noggle"), in `Game::has_subtype`; its static gives the rest.
    pub enchanted_creature_type_becomes: Option<&'static str>,
    /// "Creatures you control are every creature type. The same is true for creature spells you
    /// control and creature cards you own that aren't on the battlefield." (Maskwood Nexus)
    pub your_creatures_every_creature_type: bool,
    /// "Your opponents can't cast spells from anywhere other than their hands." (Drannith
    /// Magistrate), in the priority and free-cast actions.
    pub opponents_cast_only_from_hand: bool,
    /// "If one or more -1/-1 counters would be put on a creature you control, that many -1/-1
    /// counters minus one are put on it instead." (Vizier of Remedies), `Game::minus_counters_for`.
    pub your_minus_counters_minus_one: bool,
    /// Opposition Agent: "While an opponent is searching their library, they exile each card they
    /// find. You may play those cards for as long as they remain exiled, and you may spend mana as
    /// though it were mana of any color to cast them." (`SearchTo::ExiledForOpponent`). Not
    /// modeled: "You control your opponents while they're searching their libraries" (1v1, the
    /// searcher still chooses).
    pub opponents_searches_exiled_for_you: bool,
    /// Loyalty abilities planeswalkers you control have (Ichormoon Gauntlet: "[0]: Proliferate" and
    /// "[−12]: Take an extra turn after this one."), after their own (`Game::loyalty_ability`).
    pub your_planeswalkers_have_loyalty: Vec<(i32, crate::effect::Ability)>,
    /// Stridehangar Automaton: "If one or more artifact tokens would be created under your control,
    /// those tokens plus an additional [Thopter] are created instead." (the token, once per event)
    pub artifact_tokens_plus_one: Option<usize>,
    /// Gemstone Caverns (CR 103.6): "If [this] is in your opening hand and you're not the starting
    /// player, you may begin the game with [this] on the battlefield with a luck counter on it. If
    /// you do, exile a card from your hand." Asked in `Game::pregame_actions`
    /// (`Decision::GemstoneCaverns`); the luck counter is `Object::kind_counters`.
    pub begins_game_with_luck_counter: bool,
    /// "[This] enters with an additional loyalty counter on [it] for each instant and sorcery spell
    /// you've cast this turn." (Ral, Leyline Prodigy), from `Game::instants_sorceries_cast`.
    pub loyalty_plus_instants_sorceries_cast: bool,
}

/// A card's pieces from the d4 subsystems (see [`CardDef::subsystems`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SubsystemExt {
    /// A split card (CR 709): `omen` holds its right half, cast like an Omen's, and either half goes
    /// to its owner's graveyard as it resolves.
    pub split_card: bool,
    /// "Room abilities of dungeons you own trigger an additional time." (Hama Pashar)
    pub rooms_trigger_again: bool,
    /// "Commander creatures you own have 'Room abilities of dungeons you own trigger an additional
    /// time.'" (Dungeon Delver)
    pub commanders_rooms_trigger_again: bool,
    /// A split card's left half's own mana cost, which casting it pays; `cost` is both halves'
    /// together, its mana value off the stack (CR 709.4).
    pub left_cost: Option<ManaCost>,
    /// A Saga's chapter grants (Urza's Saga: "II — This Saga gains \"{2}, {T}: ...\""): (chapter,
    /// index into `activated`) for each activated ability it has only once that chapter's
    /// `SubsystemEffect::GainChapterAbilities` has resolved (see `Game::lacks_chapter_activated`).
    pub chapter_activated: Vec<(u8, usize)>,
    /// The chapter whose resolving grants its `mana` ability (Urza's Saga: "I — This Saga gains
    /// \"{T}: Add {C}.\""; see `Game::lacks_chapter_mana`).
    pub chapter_mana: Option<u8>,
    /// "Commander creatures you own have \"[trigger n]\"" (a Background: Agent of the Iron Throne).
    /// The trigger is on this card's own `triggers`; `Game::fire_subsystems` makes it trigger once
    /// for each commander creature its controller owns and controls (with none, not at all).
    pub commanders_have_trigger: Option<usize>,
}

/// A card's mana, cost and keyword pieces added by the d1 theme (see [`CardDef::mana_ext`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ManaExt {
    /// Its mana ability's only cost is sacrificing it, no {T} (Eldrazi Spawn: "Sacrifice this
    /// creature: Add {C}."), so it works tapped or the turn it enters. Needs `sac_for_mana`.
    pub sac_mana_no_tap: bool,
    /// "Enchantment spells you cast cost {N} less to cast." (Jukai Naturalist)
    pub enchantment_spell_discount: u8,
    /// "Black creature spells you cast cost {N} less to cast." (Bontu's Monument)
    pub black_creature_spell_discount: u8,
    /// "Creature spells you cast cost {1} less to cast for each +1/+1 counter on [this]." (Animar)
    pub creature_discount_per_counter: bool,
    /// "Instant and sorcery spells you cast cost {1} less to cast for each creature token you
    /// control." (Magnus the Red)
    pub instant_sorcery_discount_per_creature_token: bool,
    /// "This land enters tapped unless you control N or more other lands." (Haunted Ridge: 2)
    pub enters_tapped_unless_other_lands: u8,
    /// "As an additional cost to cast this spell, pay X life." Cast with `Action::CastX`, X up to
    /// your life total (Toxic Deluge).
    pub pay_x_life: bool,
    /// An Aura: "Whenever enchanted land is tapped for mana, its controller adds an additional one
    /// mana of any color." (Fertile Ground)
    pub enchanted_land_extra_any: bool,
    /// "Whenever you tap a [subtype] for mana, add an additional [color]." (Crypt Ghast: Swamp, {B})
    pub your_subtype_lands_add: Option<(&'static str, crate::Color)>,
    /// "Lands you control have '{T}: Add one mana of any color.'" (Chromatic Lantern)
    pub lands_tap_any_color: bool,
    /// Its colored mana makes each opponent gain 1 life (Grove of the Burnwillows: "{T}: Add {R} or
    /// {G}. Each opponent gains 1 life."); its {C} doesn't.
    pub colored_gives_opponent_life: bool,
    /// Evoke (CR 702.74): `alt_cost` is its evoke cost, and a cast paying it is recorded so its
    /// "sacrificed when it enters" trigger ([`crate::Effect::SacrificeIfEvoked`]) knows.
    pub evoke: bool,
    /// Convoke (CR 702.51): each untapped creature you control can tap to pay {1} or one mana of
    /// its color while you cast this spell.
    pub convoke: bool,
    /// "You may pay N life and exile a [color] card from your hand rather than pay this spell's mana
    /// cost." (Force of Will: blue, 1): (WUBRG index, life). Cast with `Action::CastWith` naming
    /// the card.
    pub pitch_with_life: Option<(u8, i32)>,
    /// "[Color] spells (and [color] spells) you cast cost {N} less to cast": spells of any of these
    /// colors (WUBRG), once each (Jet Medallion: black; Stormscape Familiar: white and black).
    pub colored_spell_discount: Option<([bool; 5], u8)>,
    /// "Whenever a player taps a land for mana, that player adds one mana of any type that land
    /// produced." (Dictate of Karametra)
    pub all_lands_add_same: bool,
    /// "Whenever you tap a creature for mana, add an additional [color]." (Leyline of Abundance)
    pub your_creatures_add: Option<crate::Color>,
    /// Escape (CR 702.138): (cost, other graveyard cards to exile, +1/+1 counters it escapes with).
    /// Cast from your graveyard with `Action::CastAlternative`; the engine picks the exiled cards,
    /// as for delve.
    pub escape: Option<(ManaCost, u8, i32)>,
    /// "This creature can't be blocked as long as it's attacking alone." (Yuan-Ti Malison)
    pub unblockable_attacking_alone: bool,
    /// "Creatures you control can't be blocked except by legendary creatures." (Midnight
    /// Pathlighter)
    pub your_creatures_blocked_only_by_legendary: bool,
    /// "Spells you cast cost {N} less to cast." (The Immortal Sun)
    pub spell_discount_all: u8,
    /// An alternative cost of mana plus life, sacrifices or taps (Demon of Death's Gate, Sephara):
    /// cast with `Action::CastAlternative`; the engine picks the creatures.
    pub alt_cost_ext: Option<AltCostExt>,
    /// "Abilities you activate that aren't mana abilities cost {N} less to activate. This effect
    /// can't reduce the mana in that cost to less than one mana." (Zirda, the Dawnwaker: 2)
    pub ability_discount: u8,
    /// "The first creature spell you cast each turn costs {N} less to cast." (Shadow in the Warp)
    pub first_creature_spell_discount: u8,
    /// Its mana of these colors (WUBRG) deals 1 damage to you; its other mana doesn't (Murmuring
    /// Bosk: "{T}: Add {W} or {B}. This land deals 1 damage to you.", and its Forest {G}).
    pub pain_colors: [bool; 5],
    /// "Whenever you tap a nonland permanent for mana, add one additional mana of any type that
    /// permanent produced." (Kinnan, Bonder Prodigy)
    pub nonland_add_same: bool,
    /// "{T}, Pay N life: Add ..." (Mana Confluence: 1): its mana costs this much life, and it
    /// can't be tapped for mana with less life than that.
    pub mana_life_cost: i32,
    /// "If you control an Urza's Power-Plant and an Urza's Tower, add {C}{C} instead." (the Urza
    /// lands): it makes this much mana while you control all three of Urza's Mine, Power-Plant
    /// and Tower.
    pub tron_amount: u8,
    /// "For each {B} in a cost, you may pay 2 life rather than pay that mana." (K'rrik): the engine
    /// pays mana first and only the {B} it can't pay with life.
    pub black_pips_for_life: bool,
    /// "Creature spells with flying you cast cost {N} less to cast." (Watcher of the Spheres)
    pub flying_creature_spell_discount: u8,
    /// "The first non-Lemur creature spell with flying you cast during each of your turns costs
    /// {N} less to cast." (Momo, Friendly Flier)
    pub first_flier_discount_your_turn: u8,
    /// "This land enters tapped unless you control a legendary creature." (The Shire, Minas
    /// Tirith)
    pub enters_tapped_unless_legendary_creature: bool,
    /// Improvise (CR 702.126): each untapped artifact you control can tap to pay {1} while you
    /// cast this spell (Kappa Cannoneer).
    pub improvise: bool,
    /// "Nonartifact spells you cast have improvise." (Inspiring Statuary)
    pub nonartifact_spells_improvise: bool,
    /// "{T}: Add {W}. Activate only if you control an Island." (Nimbus Maze): each color (WUBRG)
    /// it adds only while you control a permanent with that subtype, on top of `mana`.
    pub colors_if_you_control: [Option<&'static str>; 5],
    /// "{T}: Add {C}{C}. Spend this mana only to cast colorless Eldrazi spells or activate
    /// abilities of colorless Eldrazi." (Eldrazi Temple): it makes this much {C} for those.
    pub colorless_eldrazi_amount: u8,
    /// Multikicker (CR 702.33c): cast with `Action::CastX`, X the times kicked, paying this that
    /// many times (for a card without {X} in its cost).
    pub multikicker: Option<ManaCost>,
    /// "This artifact enters with a charge counter on it for each time it was kicked." (Everflowing
    /// Chalice): its kind counters.
    pub charge_per_kick: bool,
    /// "{T}: Add {C} for each charge counter on this artifact.": its mana, once per kind counter.
    pub mana_per_kind_counter: bool,
    /// "If that mana is spent on a creature spell, it gains haste." (Hall of the Bandit Lord). The
    /// payer spends it automatically (it isn't tapped by hand), so a creature spell it pays for
    /// gains haste as it enters.
    pub creature_spell_haste: bool,
    /// Delve (CR 702.66): each card exiled from your graveyard while casting it pays for {1}.
    pub delve: bool,
    /// "{T}: Add {C}{C}. Spend this mana only to cast colorless spells. Activate only if you
    /// control seven or more lands." (Shrine of the Forsaken Gods): (amount, lands needed).
    pub colorless_spell_amount_with_lands: Option<(u8, u8)>,
    /// "{T}, Pay 1 life: Add one mana of any color." (Spire of Industry): its colored mana costs 1
    /// life (paid like a pain land's damage: only when the payment needs that color).
    pub colored_life_cost: bool,
    /// "Activate only if you control an artifact." on its colored mana (Spire of Industry); without
    /// one it makes only its {C}.
    pub colored_only_with_artifact: bool,
    /// Foretell's cost (CR 702.143): cast from exile for it on a later turn, after an
    /// `Activated::foretell` ability exiled the card from your hand.
    pub foretell: Option<ManaCost>,
    /// "Colorless Eldrazi spells you cast cost {N} less to cast." (Eye of Ugin: 2)
    pub colorless_eldrazi_spell_discount: u8,
    /// "Colorless spells you cast with mana value N or greater cost {M} less to cast." as (N, M)
    /// (It That Heralds the End: (7, 1)).
    pub colorless_big_spell_discount: Option<(u8, u8)>,
    /// "Treasures you control have '{T}, Sacrifice this artifact: Add two mana of any one color.'"
    /// (Alchemist's Talent, level 2): while this permanent is at this Class level or higher (1:
    /// always), each Treasure you control makes two mana instead of one. 0: none.
    pub treasures_add_two_at_level: u8,
    /// "Spells you cast from anywhere other than your hand cost {N} less to cast." (Fortune
    /// Teller's Talent, level 3): (Class level needed, 1 for always; N).
    pub not_from_hand_discount: Option<(u8, u8)>,
    /// "Once each turn, you may pay {0} rather than pay the mana cost for a colorless spell you
    /// cast from your hand." (Darksteel Monolith): cast with `Action::CastAlternative`.
    pub colorless_free_once_each_turn: bool,
    /// Retrace (CR 702.81): cast this card from your graveyard by discarding a land card in
    /// addition to its other costs (`Action::CastWith` naming the land; it goes back to the
    /// graveyard as it resolves, not to exile).
    pub retrace: bool,
    /// `CardDef::ward_sacrifice` counts only permanents with this subtype (Ygra, Eater of All:
    /// "Ward—Sacrifice a Food.", with `ward_sacrifice: 1`).
    pub ward_sacrifice_subtype: Option<&'static str>,
    /// "This land enters tapped unless you control N or more other [subtype]." (Witch's Cottage:
    /// ("Swamp", 3)).
    pub enters_tapped_unless_other_subtype: Option<(&'static str, u8)>,
    /// Freerunning (CR 702.173): "You may cast this spell for its freerunning cost if you dealt
    /// combat damage to a player this turn with an Assassin or commander." Cast with
    /// `Action::CastAlternative` (Overpowering Attack: {2}{R}).
    pub freerunning: Option<ManaCost>,
    /// "{T}, Remove a charge counter from this land: Add one mana of any color." (Tendo Ice
    /// Bridge): its colored mana (its `mana` has `colorless` and the colors) needs one of its own
    /// kind of counter (`Object::kind_counters`), removed when the payment needs a color.
    pub colored_removes_kind_counter: bool,
    /// "This creature can only attack alone." (Master of Cruelties)
    pub attacks_only_alone: bool,
    /// "Once during each of your turns, you may cast a creature spell from your graveyard."
    /// (Karador, Ghost Chieftain): `Action::CastFromGraveyardAs(card, KARADOR_CAST)`.
    pub creature_from_graveyard_once_each_turn: bool,
    /// "You can spend mana of any type to cast creature spells." (Vizier of the Menagerie): your
    /// creature spells' colored, hybrid and {C} pips are paid as generic.
    pub creature_spells_any_type_mana: bool,
    /// "Creatures with power less than this creature's power can't block creatures you control."
    /// (Champion of Lambholt)
    pub smaller_cant_block_yours: bool,
    /// Compleated (CR 702.150): for each Phyrexian pip paid with life (its
    /// `Action::CastAlternative`), it enters with two fewer loyalty counters.
    pub compleated: bool,
    /// Path of Ancestry: when its mana is spent on a creature spell that shares a creature type
    /// with your commander, `When::ManaSpentOnCommanderTypeCreature` fires on it.
    pub scry_if_commander_type_creature: bool,
    /// Cleave (CR 702.148): cast for this cost with `Action::CastAlternative`; the spell's effect
    /// reads the item's `alt` (Wash Away: {1}{U}{U}).
    pub cleave: Option<ManaCost>,
    /// Sorcerer Class, level 2: "Creatures you control have '{T}: Add {U} or {R}. Spend this mana
    /// only to cast an instant or sorcery spell or to gain a Class level.'" as (Class level
    /// needed, colors): each untapped creature you control that could tap is a source of those
    /// colors when paying for an instant or sorcery spell or a Class's level-up.
    pub creatures_tap_for_instant_sorcery: Option<(u8, [bool; 5])>,
    /// "Whenever a player taps a basic land for mana, that player adds one mana of any type that
    /// land produced." (Lavaleaper): each basic land makes one more of its mana.
    pub basic_lands_add_extra: bool,
    /// "Lands you control enter untapped." (Horizon Explorer): overrides every "enters tapped".
    pub your_lands_enter_untapped: bool,
    /// "Instant and sorcery spells you cast cost {N} less to cast." (Case of the Ransacked Lab: 1)
    pub instant_sorcery_discount: u8,
    /// With `ExtraCost::BlightOrPay` on a modal spell: "If this spell's additional cost was paid,
    /// choose both instead." (Pyrrhic Strike). `Action::CastModal` offers every mode but the last
    /// (both); `Action::CastWith` naming a creature to blight casts the last mode.
    pub blight_chooses_last_mode: bool,
    /// "Creatures you control with power 2 or less can't be blocked by creatures with power 3 or
    /// greater." (Delney, Streetwise Lookout)
    pub small_creatures_evade_big: bool,
    /// "{T}: Add one mana of any of the exiled card's colors." (Chrome Mox): its `mana` is limited
    /// to the colors of its imprinted card (`Object::hideaway`); none: no mana.
    pub mana_imprinted_colors: bool,
    /// "If this artifact would enter, you may discard a land card instead. If you do, put this
    /// artifact onto the battlefield. If you don't, put it into its owner's graveyard." (Mox
    /// Diamond): a replacement (CR 614.1c) asked as the spell resolves, before it would enter, as
    /// `Decision::DiscardLandOrGraveyard`; declined, it goes from the stack to the graveyard.
    pub enters_discard_land_or_graveyard: bool,
    /// "Enchanted creature can attack as though it had haste." (Instill Energy)
    pub enchanted_attacks_as_haste: bool,
    /// "This creature enters with N -1/-1 counters on it." (Glen Elendra Guardian, Flitterwing
    /// Nuisance: 1)
    pub enters_with_minus_counters: u8,
    /// "You may sacrifice a nontoken [color] creature rather than pay this spell's mana cost."
    /// (Flare of Malice: black): `Action::CastWith` names the creature (the player's pick), as a
    /// pitch spell names its card.
    pub alt_sacrifice_nontoken_color: Option<u8>,
    /// With `enchanted_land_extra_any`: the additional mana is one of this color (WUBRG 0-4; Wild
    /// Growth: green), or 5 for the Aura's chosen color (Utopia Sprawl), instead of any color.
    pub enchanted_land_extra_color: Option<u8>,
    /// Its mana ability has no {T}: its cost is "Remove a [kind] counter from this" (Druids'
    /// Repository: "Remove a charge counter from this enchantment: Add one mana of any color.").
    /// Set with `mana` and `mana_per_kind_counter`: one mana per counter, each removing one, usable
    /// while tapped; a manual activation makes one mana.
    pub counter_mana_no_tap: bool,
    /// "You may cast this card from your graveyard as long as you control a [subtype]."
    /// (Gravecrawler: a Zombie): from there with its normal cost and timing.
    pub cast_from_graveyard_if_control: Option<&'static str>,
    /// "You may cast this card from your graveyard using its warp ability." (Timeline Culler): from
    /// there only for its `CardDef::warp` cost (`Action::CastAlternative`).
    pub warp_from_graveyard: bool,
    /// The life its warp cost also asks for (Timeline Culler: "Warp—{B}, Pay 2 life."): payable
    /// only with at least that much life (CR 119.4).
    pub warp_life: i32,
    /// "This creature gets +1/+1 for each land you control but don't own." (Herald of Leshrac)
    pub pt_per_land_controlled_not_owned: bool,
    /// "{T}: Add {C}. If [this] has a luck counter on it, instead add one mana of any color."
    /// (Gemstone Caverns): with a kind counter (`Object::kind_counters`) its `mana` is any color.
    pub any_color_with_kind_counter: bool,
    /// "Creature spells of the chosen type cost {N} less to cast." (Urza's Incubator: 2), the type
    /// chosen as it enters (`CardDef::choose_creature_type`, `Object::chosen_type`).
    pub chosen_type_creature_discount: u8,
    /// "{T}: Add one mana of any color. Spend this mana only to cast a [subtype] creature spell."
    /// (Haven of the Spirit Dragon: Dragon); its plain mana is in `mana`.
    pub any_color_for_subtype_creature: Option<&'static str>,
    /// "Noncreature spells cost {N} more to cast." (Thalia, Guardian of Thraben: 1), every player's.
    pub noncreature_spell_tax: u8,
    /// "[Subtype] spells you cast cost {N} less to cast." (Nogi, Draco-Zealot: Dragon, 1)
    pub subtype_spell_discount: Option<(&'static str, u8)>,
}

/// "You may pay [mana], [life] and sacrifice or tap [creatures] rather than pay this spell's mana
/// cost." (see [`ManaExt::alt_cost_ext`])
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AltCostExt {
    pub mana: ManaCost,
    pub life: i32,
    /// Sacrifice this many creatures you control (of this WUBRG color, if any): tokens first, then
    /// the lowest mana value.
    pub sacrifice: u8,
    pub sacrifice_color: Option<u8>,
    /// Tap this many untapped creatures you control with these keywords (Sephara: four with
    /// flying).
    pub tap: u8,
    pub tap_keywords: Keywords,
    /// Return this many permanents you control with `return_subtype` to their owner's hand (Gush:
    /// two Islands): tapped ones first, the engine's choice.
    pub return_lands: u8,
    pub return_subtype: Option<&'static str>,
    /// Only while you haven't cast a spell this game (Once Upon a Time: "If this spell is the first
    /// spell you've cast this game, you may cast it without paying its mana cost.").
    pub first_spell_this_game: bool,
    /// Only while you control a permanent with this subtype (Snuff Out: "If you control a Swamp").
    pub if_you_control: Option<&'static str>,
    /// Instead of paying, the opponent gains this much life (Invigorate: "you may have an opponent
    /// gain 3 life"; one opponent).
    pub opponent_gains: i32,
}

/// Which events don't cause abilities to trigger (see [`CardDef::suppresses_triggers`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Suppression {
    /// "Creatures entering or dying don't cause abilities to trigger." (Hushbringer)
    CreaturesEnteringOrDying,
    /// "Artifacts and creatures entering don't cause abilities to trigger." (Doorkeeper Thrull)
    ArtifactsOrCreaturesEntering,
    /// "Permanents entering don't cause abilities of permanents your opponents control to
    /// trigger." (Elesh Norn, Mother of Machines)
    PermanentsEnteringForOpponents,
}

/// Whose triggered abilities trigger an additional time (see [`CardDef::doubles_triggers_of`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerDoubler {
    /// "a Shaman or another Wizard you control" (Harmonic Prodigy).
    ShamanOrOtherWizard,
    /// "a creature you control with power N or less" (Delney).
    CreaturePowerAtMost(i32),
}

/// What entering makes triggers happen again (see [`CardDef::entering_triggers_again`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryDoubler {
    /// Any permanent (Yarok).
    Permanent,
    /// A land (Ancient Greenwarden).
    Land,
    /// A land or a permanent with this subtype (Traveling Chocobo: Bird).
    LandOrSubtype(&'static str),
    /// An artifact or a creature, anyone's (Panharmonicon).
    ArtifactOrCreature,
}

/// An Omen spell half: its name, cost, whether it's an instant, and what it does.
#[derive(Clone, Debug, PartialEq)]
pub struct Omen {
    pub name: String,
    pub cost: ManaCost,
    pub instant: bool,
    pub spell: Ability,
}

/// How a card uses behold (see [`CardDef::behold`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Behold {
    #[default]
    No,
    /// "As an additional cost ..., you may behold a Dragon."
    May,
    /// "You may cast this spell as though it had flash if you behold a Dragon ..." (Molten Exhale).
    MayForFlash,
    /// "... behold a Dragon or pay {cost}." (Caustic Exhale)
    OrPay(ManaCost),
}

impl CardDef {
    /// A vanilla creature, mostly for tests.
    pub fn creature(name: &str, cost: &str, power: i32, toughness: i32, keywords: Keywords) -> CardDef {
        CardDef {
            name: name.to_string(),
            cost: ManaCost::parse(cost).expect("valid cost"),
            types: Types {
                creature: true,
                ..Types::default()
            },
            subtypes: Vec::new(),
            power,
            toughness,
            keywords,
            mana: None,
            spell: None,
            modes: Vec::new(),
            aura: false,
            triggers: Vec::new(),
            kicker: None,
            token: false,
            activated: Vec::new(),
            sac_for_mana: false,
            extra_cost: None,
            reduction: None,
            flashback: None,
            statics: Vec::new(),
            enters_tapped: false,
            cant_block: false,
            doesnt_untap: false,
            spell_discount: 0,
            doubles_tokens_and_counters: false,
            doubles_damage_to_opponents: false,
            free_from_hand: false,
            protection_from_everything: false,
            shuffle_instead_of_graveyard: false,
            enters_counters: None,
            ward: None,
            must_attack: false,
            not_blocked_by: None,
            opponents_creatures_enter_tapped: false,
            cant_lose: false,
            ward_life: None,
            uncounterable: false,
            no_max_hand_size: false,
            kicked_spell: None,
            extra_land_drops: 0,
            pt_opponents_graveyards: false,
            hexproof_from_instants: false,
            alt_cost: None,
            leyline: false,
            grants_flash: false,
            colors: [false; 5],
            revival_on_cast: false,
            choose_color: false,
            choose_creature_type: false,
            vivid_mana: false,
            mana_only_for_subtype: None,
            other_subtype_enter_bonus: None,
            graveyard_cast_remove_counters: 0,
            any_color_for_chosen_type: false,
            graveyard_permanents: false,
            stages: Vec::new(),
            mana_of_chosen_color: false,
            stash_player: false,
            player_hexproof: false,
            prevent_noncombat_to_others: false,
            mana_per_subtype: None,
            enters_with_x_counters: false,
            pt_per_graveyard_creature: false,
            loyalty: None,
            loyalty_abilities: Vec::new(),
            behold: Behold::No,
            second_spell_discount: 0,
            filter_mana: None,
            harmonize: false,
            omen: None,
            flash_sorceries_and_dragons: false,
            subtype_enters_with_counter: None,
            enters_tapped_unless: None,
            not_blocked_by_lesser_power: false,
            stops_activated_abilities: false,
            enter_modes: false,
            attack_triggers_again_in_mode: None,
            graveyard_lands_in_mode: None,
            mana_only_if_you_control: None,
            mana_equal_power: false,
            free_subtype_spells: None,
            enchanted_doesnt_untap: false,
            mana_only_for_creatures: false,
            doubles_damage_of_new_creatures: false,
            doubles_tokens: false,
            cast_from_graveyard_finality: false,
            your_turn_discount_per_big_creature: false,
            mana_taps_creature: false,
            any_color_for_dragon_or_omen: false,
            opponents_cant_cast_your_turn: false,
            warrior_tokens_unsacrificeable_end_step: false,
            creature_spell_discount: 0,
            color_identity: [false; 5],
            mana_commander_identity: false,
            mana_opponent_land_colors: false,
            mana_amount: 0,
            animates_big_noncreatures_your_turn: false,
            equipped_hexproof_from_monocolored: false,
            enters_tapped_unless_basics: 0,
            enters_tapped_unless_at_most_lands: None,
            offspring: None,
            grants_offspring: None,
            decayed: false,
            ward_discard: false,
            spells_have_delve: false,
            enter_as_copy: false,
            pain_colored: false,
            thriving_color: None,
            doubles_your_creatures_damage: false,
            paid_mana: None,
            enters_tapped_unless_reveal: None,
            pt_hand_size: false,
            small_creatures_unblockable: false,
            vanishing: 0,
            big_creature_spell_discount: 0,
            other_non_subtype_enters_with_counter: None,
            counter_creatures_tap_for_green: false,
            max_hand_size: 0,
            enters_counters_one_plus_others: false,
            ascend: false,
            extra_land_drop_each_player: false,
            colored_mana_only_if_you_control: None,
            mana_tap_damage: 0,
            triples_your_damage: false,
            red_green_spell_discount: 0,
            shock_land: false,
            lands_from_graveyard: false,
            lands_from_top: false,
            top_spells_min_mana_value: None,
            top_spells_subtype: None,
            entering_triggers_again: None,
            untaps_on_others_untap: false,
            pt_lands: false,
            creature_unless_devotion: None,
            mana_tap_life: 0,
            extra_color_if_you_control: None,
            mana_own_land_colors: false,
            pt_half_highest_opponent_life: false,
            damage_cant_be_prevented: false,
            your_combat_damage_cant_be_prevented: false,
            not_blocked_by_power_at_most: None,
            lethal_damage_by_power: false,
            cant_attack_twice_per_turn: false,
            lands_are_forests: false,
            unleash: false,
            fights_only_with_blessing: false,
            lands_every_basic_type: false,
            enter_as_copy_nonland: false,
            protection_from_planeswalkers_and: None,
            entwine: None,
            cast_only_during_your_combat: false,
            ferocious_mana_amount: 0,
            power_per_attack_this_turn: false,
            green_permanents_pay_life_for_g: false,
            doubles_chosen_type_triggers: false,
            any_color_for_legendary: false,
            flashback_sacrifice: 0,
            pitch_not_your_turn: None,
            last_mode_needs_commander: false,
            back: None,
            alt_cost_needs_commander: false,
            permanents_for_energy: 0,
            phyrexian: [0; 5],
            copy_artifact: false,
            copy_keeps_name: false,
            copy_only_yours: false,
            copy_keeps_abilities: false,
            no_legend_rule: false,
            split_second: false,
            stops_artifact_abilities: false,
            mana_legendary_colors: false,
            keeps_green_mana: false,
            mode_costs: Vec::new(),
            plaza_mana: false,
            umbra_armor: false,
            modal_back: false,
            soulbond_flicker: false,
            green_spell_discount: 0,
            forests_add_g: false,
            buyback_sacrifice: None,
            x_is_snow_spent: false,
            mana_per_creature: false,
            mana_only_with_lands: 0,
            shuffle_on_resolve: false,
            ward_sacrifice: 0,
            enters_counters_greatest_exiled_mv: false,
            infect: false,
            historic_become_dinosaurs: false,
            vorinclex: false,
            targeted_life_tax: 0,
            red_spell_discount: 0,
            damage_to_yours_becomes_counters: false,
            mana_per_enchantment: false,
            opponents_cant_activate_your_turn: false,
            protection_from_subtype: None,
            indestructible_at_half_life: false,
            opponents_creatures_base_toughness_one: false,
            copy_until_eot_haste: false,
            top_spells_creatures: false,
            blitz: None,
            soulbond_combat_copies: false,
            creatures_tap_any_color: false,
            tokens_tap_for_green: false,
            lands_add_same: false,
            lose_replacement: false,
            warp: None,
            morph: None,
            changeling: false,
            adventure: false,
            enchanted_stops_activations: false,
            counters_plus_one: false,
            opponents_lose_double_your_turn: false,
            life_gain_doubled: false,
            extra_draws_doubled: false,
            tokens_plus_one: None,
            tokens_plus_each: None,
            opponents_cards_exiled_with_void: false,
            nonbasic_lands_are_mountains: false,
            lands_are_swamps: false,
            creatures_abilities_haste: false,
            ward_sacrifice_nonland: false,
            reanimating_aura: false,
            reanimates_tapped: false,
            flash_cast_sacrificed: false,
            mana_ext: ManaExt::default(),
            suppresses_triggers: None,
            doubles_triggers_of: None,
            subsystems: Default::default(),
            swamps_are_creatures: false,
            attack_tax: 0,
            attack_tax_per_enchantment: false,
            clue_food_treasure: None,
            kira_trigger: None,
            modes_once_each_turn: false,
            unspent_mana_becomes_colorless: false,
            cast_only_during_your_end_step: false,
            attack_tax_player_only: false,
            cast_only_during_opponents_declare_blockers: false,
            foods_have_exiled_abilities: false,
            opponents_even_locked: false,
            goads_enchanted: false,
            lands_you_control_have: Vec::new(),
            subtype_you_control_has: None,
            aftermath: false,
            citadel: false,
            attacks_only_with_artifacts: 0,
            treasures_plus_one: None,
            one_attacker_one_blocker: false,
            colorless_spell_discount: 0,
            toxic: 0,
            enchanted_ward: None,
            opponents_creatures_die_exiled: false,
            class_token_replacements: Vec::new(),
            players_cant_activate_loyalty: false,
            opponents_cant_gain_life: false,
            prevents_half_damage_to_you_and_yours: false,
            damage_to_a_player_other_loses: None,
            chosen_type_mana_uncounterable: false,
            triples_mana_tapped: false,
            your_artifacts_are_food_your_turn: false,
            other_creatures_are_food_artifacts: false,
            opponents_cant_make_you_sacrifice: false,
            exiles_opponents_cards_to_play: false,
            loyalty_at_instant_speed: false,
            other_creatures_have: Vec::new(),
            your_creature_spells_uncounterable: false,
            your_color_spells_uncounterable: None,
            your_nontoken_creatures_have_riot: false,
            opponents_life_gain_becomes_loss: false,
            opponents_draw_at_most_one: false,
            overload_spell: None,
            others_of_chosen_type_enter_with_counter: false,
            triples_creature_tokens: false,
            lands_in_graveyard_have_dredge: None,
            first_tokens_copy_equipped: false,
            fights_only_if_creature_died: false,
            soulbond_damage_draw: false,
            your_subtypes_tap_for_green: &[],
            proliferate_twice: false,
            untargetable_by_opponents_abilities: false,
            enchanted_creature_type_becomes: None,
            your_creatures_every_creature_type: false,
            opponents_cast_only_from_hand: false,
            your_minus_counters_minus_one: false,
            opponents_searches_exiled_for_you: false,
            your_planeswalkers_have_loyalty: Vec::new(),
            artifact_tokens_plus_one: None,
            begins_game_with_luck_counter: false,
            loyalty_plus_instants_sorceries_cast: false,
            protection_chosen_card_type: false,
            skip_draw_step: false,
            creatures_untap_on_others_untap: false,
            teeg_lock: false,
            max_hand_plus: 0,
            opponents_max_hand_minus: 0,
            win_on_empty_draw: false,
            extra_upkeep_for_enchanted: false,
            zones: ZonesExt::default(),
        }
    }

    /// An instant or sorcery, mostly for tests.
    pub fn spell(name: &str, cost: &str, instant: bool, ability: Ability) -> CardDef {
        CardDef {
            name: name.to_string(),
            cost: ManaCost::parse(cost).expect("valid cost"),
            types: Types {
                instant,
                sorcery: !instant,
                ..Types::default()
            },
            subtypes: Vec::new(),
            power: 0,
            toughness: 0,
            keywords: Keywords::NONE,
            mana: None,
            spell: Some(ability),
            modes: Vec::new(),
            aura: false,
            triggers: Vec::new(),
            kicker: None,
            token: false,
            activated: Vec::new(),
            sac_for_mana: false,
            extra_cost: None,
            reduction: None,
            flashback: None,
            statics: Vec::new(),
            enters_tapped: false,
            cant_block: false,
            doesnt_untap: false,
            spell_discount: 0,
            doubles_tokens_and_counters: false,
            doubles_damage_to_opponents: false,
            free_from_hand: false,
            protection_from_everything: false,
            shuffle_instead_of_graveyard: false,
            enters_counters: None,
            ward: None,
            must_attack: false,
            not_blocked_by: None,
            opponents_creatures_enter_tapped: false,
            cant_lose: false,
            ward_life: None,
            uncounterable: false,
            no_max_hand_size: false,
            kicked_spell: None,
            extra_land_drops: 0,
            pt_opponents_graveyards: false,
            hexproof_from_instants: false,
            alt_cost: None,
            leyline: false,
            grants_flash: false,
            colors: [false; 5],
            revival_on_cast: false,
            choose_color: false,
            choose_creature_type: false,
            vivid_mana: false,
            mana_only_for_subtype: None,
            other_subtype_enter_bonus: None,
            graveyard_cast_remove_counters: 0,
            any_color_for_chosen_type: false,
            graveyard_permanents: false,
            stages: Vec::new(),
            mana_of_chosen_color: false,
            stash_player: false,
            player_hexproof: false,
            prevent_noncombat_to_others: false,
            mana_per_subtype: None,
            enters_with_x_counters: false,
            pt_per_graveyard_creature: false,
            loyalty: None,
            loyalty_abilities: Vec::new(),
            behold: Behold::No,
            second_spell_discount: 0,
            filter_mana: None,
            harmonize: false,
            omen: None,
            flash_sorceries_and_dragons: false,
            subtype_enters_with_counter: None,
            enters_tapped_unless: None,
            not_blocked_by_lesser_power: false,
            stops_activated_abilities: false,
            enter_modes: false,
            attack_triggers_again_in_mode: None,
            graveyard_lands_in_mode: None,
            mana_only_if_you_control: None,
            mana_equal_power: false,
            free_subtype_spells: None,
            enchanted_doesnt_untap: false,
            mana_only_for_creatures: false,
            doubles_damage_of_new_creatures: false,
            doubles_tokens: false,
            cast_from_graveyard_finality: false,
            your_turn_discount_per_big_creature: false,
            mana_taps_creature: false,
            any_color_for_dragon_or_omen: false,
            opponents_cant_cast_your_turn: false,
            warrior_tokens_unsacrificeable_end_step: false,
            creature_spell_discount: 0,
            color_identity: [false; 5],
            mana_commander_identity: false,
            mana_opponent_land_colors: false,
            mana_amount: 0,
            animates_big_noncreatures_your_turn: false,
            equipped_hexproof_from_monocolored: false,
            enters_tapped_unless_basics: 0,
            enters_tapped_unless_at_most_lands: None,
            offspring: None,
            grants_offspring: None,
            decayed: false,
            ward_discard: false,
            spells_have_delve: false,
            enter_as_copy: false,
            pain_colored: false,
            thriving_color: None,
            doubles_your_creatures_damage: false,
            paid_mana: None,
            enters_tapped_unless_reveal: None,
            pt_hand_size: false,
            small_creatures_unblockable: false,
            vanishing: 0,
            big_creature_spell_discount: 0,
            other_non_subtype_enters_with_counter: None,
            counter_creatures_tap_for_green: false,
            max_hand_size: 0,
            enters_counters_one_plus_others: false,
            ascend: false,
            extra_land_drop_each_player: false,
            colored_mana_only_if_you_control: None,
            mana_tap_damage: 0,
            triples_your_damage: false,
            red_green_spell_discount: 0,
            shock_land: false,
            lands_from_graveyard: false,
            lands_from_top: false,
            top_spells_min_mana_value: None,
            top_spells_subtype: None,
            entering_triggers_again: None,
            untaps_on_others_untap: false,
            pt_lands: false,
            creature_unless_devotion: None,
            mana_tap_life: 0,
            extra_color_if_you_control: None,
            mana_own_land_colors: false,
            pt_half_highest_opponent_life: false,
            damage_cant_be_prevented: false,
            your_combat_damage_cant_be_prevented: false,
            not_blocked_by_power_at_most: None,
            lethal_damage_by_power: false,
            cant_attack_twice_per_turn: false,
            lands_are_forests: false,
            unleash: false,
            fights_only_with_blessing: false,
            lands_every_basic_type: false,
            enter_as_copy_nonland: false,
            protection_from_planeswalkers_and: None,
            entwine: None,
            cast_only_during_your_combat: false,
            ferocious_mana_amount: 0,
            power_per_attack_this_turn: false,
            green_permanents_pay_life_for_g: false,
            doubles_chosen_type_triggers: false,
            any_color_for_legendary: false,
            flashback_sacrifice: 0,
            pitch_not_your_turn: None,
            last_mode_needs_commander: false,
            back: None,
            alt_cost_needs_commander: false,
            permanents_for_energy: 0,
            phyrexian: [0; 5],
            copy_artifact: false,
            copy_keeps_name: false,
            copy_only_yours: false,
            copy_keeps_abilities: false,
            no_legend_rule: false,
            split_second: false,
            stops_artifact_abilities: false,
            mana_legendary_colors: false,
            keeps_green_mana: false,
            mode_costs: Vec::new(),
            plaza_mana: false,
            umbra_armor: false,
            modal_back: false,
            soulbond_flicker: false,
            green_spell_discount: 0,
            forests_add_g: false,
            buyback_sacrifice: None,
            x_is_snow_spent: false,
            mana_per_creature: false,
            mana_only_with_lands: 0,
            shuffle_on_resolve: false,
            ward_sacrifice: 0,
            enters_counters_greatest_exiled_mv: false,
            infect: false,
            historic_become_dinosaurs: false,
            vorinclex: false,
            targeted_life_tax: 0,
            red_spell_discount: 0,
            damage_to_yours_becomes_counters: false,
            mana_per_enchantment: false,
            opponents_cant_activate_your_turn: false,
            protection_from_subtype: None,
            indestructible_at_half_life: false,
            opponents_creatures_base_toughness_one: false,
            copy_until_eot_haste: false,
            top_spells_creatures: false,
            blitz: None,
            soulbond_combat_copies: false,
            creatures_tap_any_color: false,
            tokens_tap_for_green: false,
            lands_add_same: false,
            lose_replacement: false,
            warp: None,
            morph: None,
            changeling: false,
            adventure: false,
            enchanted_stops_activations: false,
            counters_plus_one: false,
            opponents_lose_double_your_turn: false,
            life_gain_doubled: false,
            extra_draws_doubled: false,
            tokens_plus_one: None,
            tokens_plus_each: None,
            opponents_cards_exiled_with_void: false,
            nonbasic_lands_are_mountains: false,
            lands_are_swamps: false,
            creatures_abilities_haste: false,
            ward_sacrifice_nonland: false,
            reanimating_aura: false,
            reanimates_tapped: false,
            flash_cast_sacrificed: false,
            mana_ext: ManaExt::default(),
            suppresses_triggers: None,
            doubles_triggers_of: None,
            subsystems: Default::default(),
            swamps_are_creatures: false,
            attack_tax: 0,
            attack_tax_per_enchantment: false,
            clue_food_treasure: None,
            kira_trigger: None,
            modes_once_each_turn: false,
            unspent_mana_becomes_colorless: false,
            cast_only_during_your_end_step: false,
            attack_tax_player_only: false,
            cast_only_during_opponents_declare_blockers: false,
            foods_have_exiled_abilities: false,
            opponents_even_locked: false,
            goads_enchanted: false,
            lands_you_control_have: Vec::new(),
            subtype_you_control_has: None,
            aftermath: false,
            citadel: false,
            attacks_only_with_artifacts: 0,
            treasures_plus_one: None,
            one_attacker_one_blocker: false,
            colorless_spell_discount: 0,
            toxic: 0,
            enchanted_ward: None,
            opponents_creatures_die_exiled: false,
            class_token_replacements: Vec::new(),
            players_cant_activate_loyalty: false,
            opponents_cant_gain_life: false,
            prevents_half_damage_to_you_and_yours: false,
            damage_to_a_player_other_loses: None,
            chosen_type_mana_uncounterable: false,
            triples_mana_tapped: false,
            your_artifacts_are_food_your_turn: false,
            other_creatures_are_food_artifacts: false,
            opponents_cant_make_you_sacrifice: false,
            exiles_opponents_cards_to_play: false,
            loyalty_at_instant_speed: false,
            other_creatures_have: Vec::new(),
            your_creature_spells_uncounterable: false,
            your_color_spells_uncounterable: None,
            your_nontoken_creatures_have_riot: false,
            opponents_life_gain_becomes_loss: false,
            opponents_draw_at_most_one: false,
            overload_spell: None,
            others_of_chosen_type_enter_with_counter: false,
            triples_creature_tokens: false,
            lands_in_graveyard_have_dredge: None,
            first_tokens_copy_equipped: false,
            fights_only_if_creature_died: false,
            soulbond_damage_draw: false,
            your_subtypes_tap_for_green: &[],
            proliferate_twice: false,
            untargetable_by_opponents_abilities: false,
            enchanted_creature_type_becomes: None,
            your_creatures_every_creature_type: false,
            opponents_cast_only_from_hand: false,
            your_minus_counters_minus_one: false,
            opponents_searches_exiled_for_you: false,
            your_planeswalkers_have_loyalty: Vec::new(),
            artifact_tokens_plus_one: None,
            begins_game_with_luck_counter: false,
            loyalty_plus_instants_sorceries_cast: false,
            protection_chosen_card_type: false,
            skip_draw_step: false,
            creatures_untap_on_others_untap: false,
            teeg_lock: false,
            max_hand_plus: 0,
            opponents_max_hand_minus: 0,
            win_on_empty_draw: false,
            extra_upkeep_for_enchanted: false,
            zones: ZonesExt::default(),
        }
    }

    /// A basic land that taps for one color.
    pub fn basic_land(name: &str, color: crate::Color) -> CardDef {
        CardDef {
            name: name.to_string(),
            cost: ManaCost::default(),
            types: Types {
                land: true,
                basic: true,
                ..Types::default()
            },
            subtypes: vec![name.to_string()],
            power: 0,
            toughness: 0,
            keywords: Keywords::NONE,
            mana: Some(ManaSource::of(&[color])),
            spell: None,
            modes: Vec::new(),
            aura: false,
            triggers: Vec::new(),
            kicker: None,
            token: false,
            activated: Vec::new(),
            sac_for_mana: false,
            extra_cost: None,
            reduction: None,
            flashback: None,
            statics: Vec::new(),
            enters_tapped: false,
            cant_block: false,
            doesnt_untap: false,
            spell_discount: 0,
            doubles_tokens_and_counters: false,
            doubles_damage_to_opponents: false,
            free_from_hand: false,
            protection_from_everything: false,
            shuffle_instead_of_graveyard: false,
            enters_counters: None,
            ward: None,
            must_attack: false,
            not_blocked_by: None,
            opponents_creatures_enter_tapped: false,
            cant_lose: false,
            ward_life: None,
            uncounterable: false,
            no_max_hand_size: false,
            kicked_spell: None,
            extra_land_drops: 0,
            pt_opponents_graveyards: false,
            hexproof_from_instants: false,
            alt_cost: None,
            leyline: false,
            grants_flash: false,
            colors: [false; 5],
            revival_on_cast: false,
            choose_color: false,
            choose_creature_type: false,
            vivid_mana: false,
            mana_only_for_subtype: None,
            other_subtype_enter_bonus: None,
            graveyard_cast_remove_counters: 0,
            any_color_for_chosen_type: false,
            graveyard_permanents: false,
            stages: Vec::new(),
            mana_of_chosen_color: false,
            stash_player: false,
            player_hexproof: false,
            prevent_noncombat_to_others: false,
            mana_per_subtype: None,
            enters_with_x_counters: false,
            pt_per_graveyard_creature: false,
            loyalty: None,
            loyalty_abilities: Vec::new(),
            behold: Behold::No,
            second_spell_discount: 0,
            filter_mana: None,
            harmonize: false,
            omen: None,
            flash_sorceries_and_dragons: false,
            subtype_enters_with_counter: None,
            enters_tapped_unless: None,
            not_blocked_by_lesser_power: false,
            stops_activated_abilities: false,
            enter_modes: false,
            attack_triggers_again_in_mode: None,
            graveyard_lands_in_mode: None,
            mana_only_if_you_control: None,
            mana_equal_power: false,
            free_subtype_spells: None,
            enchanted_doesnt_untap: false,
            mana_only_for_creatures: false,
            doubles_damage_of_new_creatures: false,
            doubles_tokens: false,
            cast_from_graveyard_finality: false,
            your_turn_discount_per_big_creature: false,
            mana_taps_creature: false,
            any_color_for_dragon_or_omen: false,
            opponents_cant_cast_your_turn: false,
            warrior_tokens_unsacrificeable_end_step: false,
            creature_spell_discount: 0,
            color_identity: [false; 5],
            mana_commander_identity: false,
            mana_opponent_land_colors: false,
            mana_amount: 0,
            animates_big_noncreatures_your_turn: false,
            equipped_hexproof_from_monocolored: false,
            enters_tapped_unless_basics: 0,
            enters_tapped_unless_at_most_lands: None,
            offspring: None,
            grants_offspring: None,
            decayed: false,
            ward_discard: false,
            spells_have_delve: false,
            enter_as_copy: false,
            pain_colored: false,
            thriving_color: None,
            doubles_your_creatures_damage: false,
            paid_mana: None,
            enters_tapped_unless_reveal: None,
            pt_hand_size: false,
            small_creatures_unblockable: false,
            vanishing: 0,
            big_creature_spell_discount: 0,
            other_non_subtype_enters_with_counter: None,
            counter_creatures_tap_for_green: false,
            max_hand_size: 0,
            enters_counters_one_plus_others: false,
            ascend: false,
            extra_land_drop_each_player: false,
            colored_mana_only_if_you_control: None,
            mana_tap_damage: 0,
            triples_your_damage: false,
            red_green_spell_discount: 0,
            shock_land: false,
            lands_from_graveyard: false,
            lands_from_top: false,
            top_spells_min_mana_value: None,
            top_spells_subtype: None,
            entering_triggers_again: None,
            untaps_on_others_untap: false,
            pt_lands: false,
            creature_unless_devotion: None,
            mana_tap_life: 0,
            extra_color_if_you_control: None,
            mana_own_land_colors: false,
            pt_half_highest_opponent_life: false,
            damage_cant_be_prevented: false,
            your_combat_damage_cant_be_prevented: false,
            not_blocked_by_power_at_most: None,
            lethal_damage_by_power: false,
            cant_attack_twice_per_turn: false,
            lands_are_forests: false,
            unleash: false,
            fights_only_with_blessing: false,
            lands_every_basic_type: false,
            enter_as_copy_nonland: false,
            protection_from_planeswalkers_and: None,
            entwine: None,
            cast_only_during_your_combat: false,
            ferocious_mana_amount: 0,
            power_per_attack_this_turn: false,
            green_permanents_pay_life_for_g: false,
            doubles_chosen_type_triggers: false,
            any_color_for_legendary: false,
            flashback_sacrifice: 0,
            pitch_not_your_turn: None,
            last_mode_needs_commander: false,
            back: None,
            alt_cost_needs_commander: false,
            permanents_for_energy: 0,
            phyrexian: [0; 5],
            copy_artifact: false,
            copy_keeps_name: false,
            copy_only_yours: false,
            copy_keeps_abilities: false,
            no_legend_rule: false,
            split_second: false,
            stops_artifact_abilities: false,
            mana_legendary_colors: false,
            keeps_green_mana: false,
            mode_costs: Vec::new(),
            plaza_mana: false,
            umbra_armor: false,
            modal_back: false,
            soulbond_flicker: false,
            green_spell_discount: 0,
            forests_add_g: false,
            buyback_sacrifice: None,
            x_is_snow_spent: false,
            mana_per_creature: false,
            mana_only_with_lands: 0,
            shuffle_on_resolve: false,
            ward_sacrifice: 0,
            enters_counters_greatest_exiled_mv: false,
            infect: false,
            historic_become_dinosaurs: false,
            vorinclex: false,
            targeted_life_tax: 0,
            red_spell_discount: 0,
            damage_to_yours_becomes_counters: false,
            mana_per_enchantment: false,
            opponents_cant_activate_your_turn: false,
            protection_from_subtype: None,
            indestructible_at_half_life: false,
            opponents_creatures_base_toughness_one: false,
            copy_until_eot_haste: false,
            top_spells_creatures: false,
            blitz: None,
            soulbond_combat_copies: false,
            creatures_tap_any_color: false,
            tokens_tap_for_green: false,
            lands_add_same: false,
            lose_replacement: false,
            warp: None,
            morph: None,
            changeling: false,
            adventure: false,
            enchanted_stops_activations: false,
            counters_plus_one: false,
            opponents_lose_double_your_turn: false,
            life_gain_doubled: false,
            extra_draws_doubled: false,
            tokens_plus_one: None,
            tokens_plus_each: None,
            opponents_cards_exiled_with_void: false,
            nonbasic_lands_are_mountains: false,
            lands_are_swamps: false,
            creatures_abilities_haste: false,
            ward_sacrifice_nonland: false,
            reanimating_aura: false,
            reanimates_tapped: false,
            flash_cast_sacrificed: false,
            mana_ext: ManaExt::default(),
            suppresses_triggers: None,
            doubles_triggers_of: None,
            subsystems: Default::default(),
            swamps_are_creatures: false,
            attack_tax: 0,
            attack_tax_per_enchantment: false,
            clue_food_treasure: None,
            kira_trigger: None,
            modes_once_each_turn: false,
            unspent_mana_becomes_colorless: false,
            cast_only_during_your_end_step: false,
            attack_tax_player_only: false,
            cast_only_during_opponents_declare_blockers: false,
            foods_have_exiled_abilities: false,
            opponents_even_locked: false,
            goads_enchanted: false,
            lands_you_control_have: Vec::new(),
            subtype_you_control_has: None,
            aftermath: false,
            citadel: false,
            attacks_only_with_artifacts: 0,
            treasures_plus_one: None,
            one_attacker_one_blocker: false,
            colorless_spell_discount: 0,
            toxic: 0,
            enchanted_ward: None,
            opponents_creatures_die_exiled: false,
            class_token_replacements: Vec::new(),
            players_cant_activate_loyalty: false,
            opponents_cant_gain_life: false,
            prevents_half_damage_to_you_and_yours: false,
            damage_to_a_player_other_loses: None,
            chosen_type_mana_uncounterable: false,
            triples_mana_tapped: false,
            your_artifacts_are_food_your_turn: false,
            other_creatures_are_food_artifacts: false,
            opponents_cant_make_you_sacrifice: false,
            exiles_opponents_cards_to_play: false,
            loyalty_at_instant_speed: false,
            other_creatures_have: Vec::new(),
            your_creature_spells_uncounterable: false,
            your_color_spells_uncounterable: None,
            your_nontoken_creatures_have_riot: false,
            opponents_life_gain_becomes_loss: false,
            opponents_draw_at_most_one: false,
            overload_spell: None,
            others_of_chosen_type_enter_with_counter: false,
            triples_creature_tokens: false,
            lands_in_graveyard_have_dredge: None,
            first_tokens_copy_equipped: false,
            fights_only_if_creature_died: false,
            soulbond_damage_draw: false,
            your_subtypes_tap_for_green: &[],
            proliferate_twice: false,
            untargetable_by_opponents_abilities: false,
            enchanted_creature_type_becomes: None,
            your_creatures_every_creature_type: false,
            opponents_cast_only_from_hand: false,
            your_minus_counters_minus_one: false,
            opponents_searches_exiled_for_you: false,
            your_planeswalkers_have_loyalty: Vec::new(),
            artifact_tokens_plus_one: None,
            begins_game_with_luck_counter: false,
            loyalty_plus_instants_sorceries_cast: false,
            protection_chosen_card_type: false,
            skip_draw_step: false,
            creatures_untap_on_others_untap: false,
            teeg_lock: false,
            max_hand_plus: 0,
            opponents_max_hand_minus: 0,
            win_on_empty_draw: false,
            extra_upkeep_for_enchanted: false,
            zones: ZonesExt::default(),
        }
    }
}

/// A card's objects-and-zones pieces added by the Mac C theme (see [`CardDef::zones`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ZonesExt {
    /// As a clone that entered as a copy (`enter_as_copy`): these creature types in addition to the
    /// copied ones (Glasspool Mimic: Shapeshifter Rogue; Phantasmal Image: Illusion; Mockingbird:
    /// Bird).
    pub copy_adds_subtypes: &'static [&'static str],
    /// As a clone that entered as a copy: these keywords too (Mockingbird: flying).
    pub copy_adds_keywords: Keywords,
    /// As a clone that entered as a copy: "When this creature becomes the target of a spell or
    /// ability, sacrifice it." (Phantasmal Image)
    pub copy_sacrifice_when_targeted: bool,
    /// It may copy only a creature with mana value at most the mana spent to cast it (Mockingbird).
    pub copy_mv_at_most_spent: bool,
    /// With `shock_land`: the life it asks for instead of 2 (Volcanic Fissure: 3).
    pub shock_life: i32,
    /// With `enter_as_copy`: it may copy an artifact or a non-Aura enchantment only (Mirrormade).
    pub copy_artifact_or_enchantment: bool,
    /// "You may have this land enter tapped as a copy of any land on the battlefield." (Vesuva): asked
    /// as it enters, however it enters (`Object::land_copy_pending`).
    pub enters_as_land_copy_tapped: bool,
    /// The card has no mana cost (CR 202.1b): it can't be cast unless something lets it be cast
    /// without paying its mana cost (Glimpse of Tomorrow: suspend).
    pub no_mana_cost: bool,
    /// "You may cast artifact spells and colorless spells from the top of your library." (Mystic
    /// Forge)
    pub top_spells_artifact_or_colorless: bool,
    /// "You may cast this card from your graveyard or from exile." (Squee, the Immortal): its owner
    /// may cast it from either zone, with its normal timing and cost.
    pub cast_from_graveyard_or_exile: bool,
    /// "You may cast spells with flash or flying from the top of your library." (Errant and Giada):
    /// printed flash or flying.
    pub top_spells_flash_or_flying: bool,
    /// "As long as you've cast a spell this turn, you may play cards from the top of your library."
    /// (Fortune Teller's Talent, level 2): while this permanent is at this Class level or higher (1:
    /// always). 0: none.
    pub top_cards_if_cast_spell_at_level: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_set_ops() {
        let k = Keywords::FLYING.with(Keywords::LIFELINK);
        assert!(k.has(Keywords::FLYING) && k.has(Keywords::LIFELINK));
        assert!(!k.has(Keywords::TRAMPLE));
        assert_eq!(Keywords::named("first strike"), Some(Keywords::FIRST_STRIKE));
        assert_eq!(Keywords::named("Ward"), None);
    }

    #[test]
    fn parses_type_lines() {
        let t = Types::parse("Legendary Creature — Human Soldier");
        assert!(t.legendary && t.creature && !t.land && t.is_permanent());
        let b = Types::parse("Basic Land — Forest");
        assert!(b.basic && b.land);
        assert!(!Types::parse("Instant").is_permanent());
    }
}
