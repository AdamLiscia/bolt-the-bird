//! Card definitions, one card set at a time.
//!
//! [`fdn`] returns all 286 Foundations cards in the same order as `assets/FDN_cards.json`, so a card's
//! index here is its index in the draft models, and a drafted deck plays without translation. Other
//! sets ([`set`], e.g. Tarkir: Dragonstorm, "TDM") work the same way from `assets/{SET}_oracle.json`:
//! each set is its own numbering, and scripts are shared by card name, so a reprint plays the same.
//!
//! [`supported`] is the source of truth for restricted drafts: a card is playable only if every
//! line of its rules text is implemented. Unsupported cards are listed, never skipped.

pub mod play;
pub mod scripts;

use std::sync::{Arc, OnceLock};

use bolt_core::{CardDef, Color, Keywords, ManaCost, ManaSource, Types};
use serde::Deserialize;

const ORACLE: &str = include_str!("../../../assets/FDN_oracle.json");
const TDM_ORACLE: &str = include_str!("../../../assets/TDM_oracle.json");
/// Bloomburrow Commander (BLC): the four precons' cards, from `scripts/fetch_oracle.py --names`.
const BLC_ORACLE: &str = include_str!("../../../assets/BLC_oracle.json");
/// The Commander pool: every known card across sets (`scripts/make_cmd_set.py`).
const CMD_ORACLE: &str = include_str!("../../../assets/CMD_oracle.json");

/// The sets the engine knows, by code.
pub const SETS: &[&str] = &["FDN", "TDM", "BLC"];

/// One card set: its cards in model order, their definitions (then the token definitions) and
/// which cards the engine fully implements.
pub struct CardSet {
    pub code: &'static str,
    pub oracle: Vec<Oracle>,
    pub defs: Arc<Vec<CardDef>>,
    pub supported: Vec<bool>,
}

impl CardSet {
    fn load(code: &'static str, json: &str) -> CardSet {
        let oracle: Vec<Oracle> =
            serde_json::from_str(json).unwrap_or_else(|e| panic!("assets/{code}_oracle.json: {e}"));
        let tokens = scripts::Tokens::starting_at(oracle.len());
        let built: Vec<(CardDef, bool)> = oracle.iter().map(|o| definition(o, &tokens)).collect();
        let supported = built.iter().map(|b| b.1).collect();
        let mut defs: Vec<CardDef> = built.into_iter().map(|b| b.0).collect();
        defs.extend(scripts::token_defs());
        // Nesting Dragon's Egg: "When this token dies, create a 2/2 red Dragon ...". Wired here because a
        // row in `token_defs` can't name another token (it doesn't know where the tokens start).
        defs[tokens.dragon_egg].triggers.push(bolt_core::Trigger {
            when: bolt_core::When::Dies,
            ability: bolt_core::Ability::new(
                vec![],
                vec![bolt_core::Effect::CreateToken {
                    who: bolt_core::Who::You,
                    def: tokens.dragon_2_firebreathing,
                    n: 1,
                }],
            ),
        });
        CardSet {
            code,
            oracle,
            defs: Arc::new(defs),
            supported,
        }
    }

    /// Number of real cards (tokens follow them in `defs`).
    pub fn card_count(&self) -> usize {
        self.oracle.len()
    }

    /// Model index of a card by exact name.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.oracle.iter().position(|o| o.name == name)
    }

    /// Names of the cards the engine fully implements, in model order.
    pub fn supported_names(&self) -> Vec<&str> {
        self.oracle
            .iter()
            .zip(&self.supported)
            .filter(|(_, &ok)| ok)
            .map(|(o, _)| o.name.as_str())
            .collect()
    }
}

/// A card set by code ("FDN", "TDM"), or None for a set the engine doesn't have.
pub fn set(code: &str) -> Option<&'static CardSet> {
    static FDN: OnceLock<CardSet> = OnceLock::new();
    static TDM: OnceLock<CardSet> = OnceLock::new();
    static BLC: OnceLock<CardSet> = OnceLock::new();
    static CMD: OnceLock<CardSet> = OnceLock::new();
    match code {
        "FDN" => Some(FDN.get_or_init(|| CardSet::load("FDN", ORACLE))),
        "TDM" => Some(TDM.get_or_init(|| CardSet::load("TDM", TDM_ORACLE))),
        "BLC" => Some(BLC.get_or_init(|| CardSet::load("BLC", BLC_ORACLE))),
        "CMD" => Some(CMD.get_or_init(|| CardSet::load("CMD", CMD_ORACLE))),
        _ => None,
    }
}

fn fdn_set() -> &'static CardSet {
    set("FDN").expect("FDN")
}

/// One card as Scryfall describes it (`assets/FDN_oracle.json`, written by `scripts/card_audit.py`).
#[derive(Clone, Debug, Deserialize)]
pub struct Oracle {
    pub name: String,
    pub mana_cost: Option<String>,
    pub type_line: String,
    pub oracle_text: Option<String>,
    pub power: Option<String>,
    pub toughness: Option<String>,
    pub rarity: String,
    #[serde(default)]
    pub colors: Vec<String>,
    /// Starting loyalty, for planeswalkers (`scripts/fetch_loyalty.py`).
    #[serde(default)]
    pub loyalty: Option<String>,
    /// The Omen half of a Tarkir: Dragonstorm Omen card.
    #[serde(default)]
    pub omen: Option<OmenFace>,
    /// The back face of a transforming double-faced card (CR 712).
    #[serde(default)]
    pub back: Option<BackFace>,
    /// A split card's right half (CR 709); the front fields are its left half.
    #[serde(default)]
    pub split: Option<OmenFace>,
}

/// A double-faced card's back face, as Scryfall describes it (`scripts/fetch_oracle.py`).
#[derive(Clone, Debug, Deserialize)]
pub struct BackFace {
    pub name: String,
    pub mana_cost: Option<String>,
    pub type_line: String,
    pub oracle_text: Option<String>,
    pub power: Option<String>,
    pub toughness: Option<String>,
    #[serde(default)]
    pub colors: Vec<String>,
    #[serde(default)]
    pub loyalty: Option<String>,
}

/// An Omen card's spell half, as Scryfall describes it.
#[derive(Clone, Debug, Deserialize)]
pub struct OmenFace {
    pub name: String,
    pub mana_cost: Option<String>,
    pub type_line: String,
    pub oracle_text: Option<String>,
}

/// The Scryfall data for every FDN card, in model order.
pub fn oracle() -> &'static [Oracle] {
    &fdn_set().oracle
}

/// Card definitions for all of FDN in model order, then the token definitions. Unsupported cards
/// still get their printed characteristics, so decks load whole; [`supported`] says which ones the
/// engine can play.
pub fn fdn() -> Arc<Vec<CardDef>> {
    fdn_set().defs.clone()
}

/// Number of real cards (tokens follow them in [`fdn`]).
pub fn card_count() -> usize {
    oracle().len()
}

/// Whether the engine implements every rule on each card, by model index.
pub fn supported_mask() -> &'static [bool] {
    &fdn_set().supported
}

/// Names of the cards the engine fully implements, in model order.
pub fn supported() -> Vec<&'static str> {
    oracle()
        .iter()
        .zip(supported_mask())
        .filter(|(_, &ok)| ok)
        .map(|(o, _)| o.name.as_str())
        .collect()
}

/// Model index of a card by exact name.
pub fn index_of(name: &str) -> Option<usize> {
    oracle().iter().position(|o| o.name == name)
}

/// Rules text with reminder text removed, one ability per line.
fn abilities(text: &str) -> Vec<String> {
    let mut out = String::new();
    let mut depth = 0;
    for ch in text.chars() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    out.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

/// Whether an ability line is only implemented keywords, e.g. `Flying, vigilance`.
fn keyword_line(line: &str) -> Option<Keywords> {
    line.split(',').try_fold(Keywords::NONE, |k, w| {
        Keywords::named(w.trim()).map(|x| k.with(x))
    })
}

/// Builds the definition and decides support. A card is supported when its cost parses and every
/// ability line is either implemented keywords or claimed by its script.
fn definition(o: &Oracle, tokens: &scripts::Tokens) -> (CardDef, bool) {
    let types = Types::parse(&o.type_line);
    let subtypes = o
        .type_line
        .split('—')
        .nth(1)
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_default();
    // Phyrexian pips ({U/P}) parse as their color; `CardDef::phyrexian` remembers them.
    let mut phyrexian = [0u8; 5];
    let plain_cost = o.mana_cost.as_deref().map(|c| {
        let mut out = c.to_string();
        for (i, sym) in ["W", "U", "B", "R", "G"].iter().enumerate() {
            let p = format!("{{{sym}/P}}");
            phyrexian[i] = out.matches(&p).count() as u8;
            out = out.replace(&p, &format!("{{{sym}}}"));
        }
        out
    });
    let cost = plain_cost.as_deref().map(ManaCost::parse);
    let mut keywords = Keywords::NONE;
    let mut rest = Vec::new();
    for line in abilities(o.oracle_text.as_deref().unwrap_or("")) {
        match keyword_line(&line) {
            Some(k) => keywords = keywords.with(k),
            None => rest.push(line),
        }
    }
    let basic_color = types
        .basic
        .then(|| match o.type_line.rsplit(' ').next().unwrap_or("") {
            "Plains" => Color::White,
            "Island" => Color::Blue,
            "Swamp" => Color::Black,
            "Mountain" => Color::Red,
            _ => Color::Green,
        });
    let stat = |s: &Option<String>| s.as_deref().and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut def = CardDef {
        name: o.name.clone(),
        cost: cost.flatten().unwrap_or_default(),
        types,
        subtypes,
        power: stat(&o.power),
        toughness: stat(&o.toughness),
        keywords,
        mana: basic_color.map(|c| ManaSource::of(&[c])),
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
        colors: {
            let mut c = [false; 5];
            for s in &o.colors {
                if let Some(col) = s.chars().next().and_then(Color::from_symbol) {
                    c[col as usize] = true;
                }
            }
            c
        },
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
        loyalty: o.loyalty.as_deref().and_then(|v| v.parse().ok()),

        loyalty_abilities: Vec::new(),
        behold: bolt_core::Behold::No,
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
        mana_ext: Default::default(),
        suppresses_triggers: None,
        doubles_triggers_of: None,
        subsystems: Default::default(),
        protection_chosen_card_type: false,
        skip_draw_step: false,
        creatures_untap_on_others_untap: false,
        teeg_lock: false,
        max_hand_plus: 0,
        opponents_max_hand_minus: 0,
        win_on_empty_draw: false,
        extra_upkeep_for_enchanted: false,
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
        zones: Default::default(),
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
        enchanted_becomes_artifact_creature: false,
        your_nontoken_creatures_are_forest_lands: false,
    };
    def.color_identity = color_identity(o);
    def.phyrexian = phyrexian;
    // An Omen's spell half: its lines must be claimed too, written "Omen: <line>" in the script.
    if let Some(face) = &o.omen {
        rest.extend(
            abilities(face.oracle_text.as_deref().unwrap_or(""))
                .into_iter()
                .map(|l| format!("Omen: {l}")),
        );
    }
    // A split card's right half: its lines are claimed too, written "Split: <line>".
    if let Some(face) = &o.split {
        rest.extend(
            abilities(face.oracle_text.as_deref().unwrap_or(""))
                .into_iter()
                .map(|l| format!("Split: {l}")),
        );
    }
    // A transforming card's back face: its own definition, with its lines claimed by the front's
    // script, written "Back: <line>" (keyword-only lines are its keywords).
    if let Some(b) = &o.back {
        let face = Oracle {
            name: b.name.clone(),
            mana_cost: None,
            type_line: b.type_line.clone(),
            oracle_text: b.oracle_text.clone(),
            power: b.power.clone(),
            toughness: b.toughness.clone(),
            rarity: o.rarity.clone(),
            colors: b.colors.clone(),
            loyalty: b.loyalty.clone(),
            omen: None,
            back: None,
            split: None,
        };
        let (mut back, _) = definition(&face, tokens);
        back.cost = ManaCost::default();
        back.color_identity = def.color_identity;
        def.back = Some(Box::new(back));
        rest.extend(
            abilities(b.oracle_text.as_deref().unwrap_or(""))
                .into_iter()
                .filter(|l| keyword_line(l).is_none())
                .map(|l| format!("Back: {l}")),
        );
    }
    let mut claimed = rest.is_empty();
    if let Some(s) = scripts::script(&o.name) {
        if s.lines == rest.as_slice() {
            (s.build)(&mut def, tokens);
            claimed = true;
            if let (Some(face), Some(half)) = (&o.omen, def.omen.as_mut()) {
                half.name = face.name.clone();
                half.instant = face.type_line.contains("Instant");
                match face.mana_cost.as_deref().map(ManaCost::parse) {
                    Some(Some(c)) => half.cost = c,
                    _ => claimed = false,
                }
            }
            // A split card's right half is cast the way an Omen half is, then goes to the graveyard.
            def.subsystems.split_card = o.split.is_some() && def.omen.is_some();
            if let (Some(face), Some(half)) = (&o.split, def.omen.as_mut()) {
                half.name = face.name.clone();
                half.instant = face.type_line.contains("Instant");
                match face.mana_cost.as_deref().map(ManaCost::parse) {
                    Some(Some(c)) => half.cost = c,
                    _ => claimed = false,
                }
            }
            // Off the stack its mana value is both halves' (CR 709.4): `cost` is their sum, and
            // casting the left half pays `left_cost`.
            if def.subsystems.split_card {
                let right = def.omen.as_ref().expect("the right half").cost;
                let left = def.cost;
                def.subsystems.left_cost = Some(left);
                def.cost.generic = left.generic.saturating_add(right.generic);
                def.cost.colorless = left.colorless.saturating_add(right.colorless);
                for i in 0..5 {
                    def.cost.colored[i] = left.colored[i].saturating_add(right.colored[i]);
                    def.cost.twobrid[i] = left.twobrid[i].saturating_add(right.twobrid[i]);
                }
                for i in 0..10 {
                    def.cost.hybrid[i] = left.hybrid[i].saturating_add(right.hybrid[i]);
                }
            }
        }
    }
    let castable = types.is_permanent() || def.spell.is_some() || !def.modes.is_empty(); // lands: played
    let supported = basic_color.is_some() || (castable && claimed && matches!(cost, Some(Some(_))));
    (def, supported)
}

/// A card's color identity (CR 903.4), WUBRG: the colors of the mana symbols in its mana cost and
/// rules text (reminder text isn't rules text), on both halves of an Omen card, plus its colors
/// (which include a color indicator).
pub fn color_identity(o: &Oracle) -> [bool; 5] {
    let mut id = [false; 5];
    let mut scan = |text: &str| {
        let mut inside = false;
        for ch in text.chars() {
            match ch {
                '{' => inside = true,
                '}' => inside = false,
                _ if inside => {
                    if let Some(c) = Color::from_symbol(ch) {
                        id[c as usize] = true;
                    }
                }
                _ => {}
            }
        }
    };
    scan(o.mana_cost.as_deref().unwrap_or(""));
    scan(&abilities(o.oracle_text.as_deref().unwrap_or("")).join("\n"));
    if let Some(face) = &o.omen {
        scan(face.mana_cost.as_deref().unwrap_or(""));
        scan(&abilities(face.oracle_text.as_deref().unwrap_or("")).join("\n"));
    }
    if let Some(face) = &o.back {
        scan(&abilities(face.oracle_text.as_deref().unwrap_or("")).join("\n"));
        for s in &face.colors {
            if let Some(c) = s.chars().next().and_then(Color::from_symbol) {
                id[c as usize] = true;
            }
        }
    }
    for s in &o.colors {
        if let Some(c) = s.chars().next().and_then(Color::from_symbol) {
            id[c as usize] = true;
        }
    }
    id
}

/// Whether a deck may hold any number of copies of this card (CR 903.5b): basic lands, and cards
/// that say so.
fn any_number_allowed(o: &Oracle) -> bool {
    // Any card with the Basic supertype (CR 205.4, 903.5b): "Basic Land", "Basic Snow Land" (Snow-Covered
    // basics) and Wastes.
    let supertypes = o.type_line.split('\u{2014}').next().unwrap_or("");
    let words: Vec<&str> = supertypes.split_whitespace().collect();
    (words.contains(&"Basic") && words.contains(&"Land"))
        || o.oracle_text
            .as_deref()
            .is_some_and(|t| t.contains("A deck can have any number of cards named"))
}

/// Names that break the singleton rule (CR 903.5b): more than one copy, other than basic lands and
/// cards that allow any number. `cards` are card indices in `set`; each name is listed once.
pub fn singleton_problems(set: &CardSet, cards: &[usize]) -> Vec<String> {
    let mut seen = std::collections::HashMap::new();
    for &c in cards {
        *seen.entry(c).or_insert(0usize) += 1;
    }
    let mut out: Vec<String> = seen
        .into_iter()
        .filter(|&(c, n)| n > 1 && !any_number_allowed(&set.oracle[c]))
        .map(|(c, _)| set.oracle[c].name.clone())
        .collect();
    out.sort();
    out
}

/// What makes a 1v1 Commander deck illegal (CR 903.3-903.5), as messages; empty if it's legal.
/// `deck` is the other 99 cards, as card indices in `set`.
pub fn commander_deck_problems(set: &CardSet, commander: usize, deck: &[usize]) -> Vec<String> {
    commander_deck_problems_for(set, &[commander], deck)
}

/// Whether a card's rules text gives it partner (CR 702.124): "Partner", or "Partner with
/// `other`" naming the other commander.
fn partners_with(o: &Oracle, other: &Oracle) -> bool {
    abilities(o.oracle_text.as_deref().unwrap_or(""))
        .iter()
        .any(|l| l == "Partner" || l.strip_prefix("Partner with ").is_some_and(|n| n == other.name))
}

/// A legendary Background enchantment card (CR 702.124k).
fn is_background(o: &Oracle) -> bool {
    let t = &o.type_line;
    t.contains("Legendary") && t.contains("Enchantment") && t.contains("Background")
}

/// Choose a Background (CR 702.124k): a creature commander with "Choose a Background" and a
/// legendary Background enchantment as the second commander, in either order.
fn background_pair(a: &Oracle, b: &Oracle) -> bool {
    let chooses = |o: &Oracle| {
        abilities(o.oracle_text.as_deref().unwrap_or(""))
            .iter()
            .any(|l| l == "Choose a Background")
    };
    (chooses(a) && is_background(b)) || (chooses(b) && is_background(a))
}

/// [`commander_deck_problems`] for one or two commanders (two must be partners, CR 702.124);
/// `deck` is the other cards (100 in all with the commanders), and the color identity is both
/// commanders' together.
pub fn commander_deck_problems_for(set: &CardSet, commanders: &[usize], deck: &[usize]) -> Vec<String> {
    let mut out = Vec::new();
    if !(1..=2).contains(&commanders.len()) {
        out.push(format!("{} commanders, not one or two", commanders.len()));
        return out;
    }
    let cs: Vec<&Oracle> = commanders.iter().map(|&c| &set.oracle[c]).collect();
    let with_background = matches!(cs[..], [a, b] if background_pair(a, b));
    for c in &cs {
        let legendary_creature = c.type_line.contains("Legendary") && c.type_line.contains("Creature");
        let ok = legendary_creature || (with_background && is_background(c));
        if !ok {
            out.push(format!("{} isn't a legendary creature", c.name));
        }
    }
    if let [a, b] = cs[..] {
        let partners = partners_with(a, b) && partners_with(b, a);
        if !partners && !with_background {
            out.push(format!("{} and {} aren't partners", a.name, b.name));
        }
    }
    let total = deck.len() + commanders.len();
    if total != 100 {
        out.push(format!("{total} cards with the commander, not 100"));
    }
    let mut all = deck.to_vec();
    all.extend(commanders);
    for name in singleton_problems(set, &all) {
        out.push(format!("more than one {name}"));
    }
    let mut allowed = [false; 5];
    for c in &cs {
        for (a, has) in allowed.iter_mut().zip(color_identity(c)) {
            *a |= has;
        }
    }
    let mut outside: Vec<&str> = deck
        .iter()
        .map(|&d| &set.oracle[d])
        .filter(|o| color_identity(o).iter().zip(allowed).any(|(&has, ok)| has && !ok))
        .map(|o| o.name.as_str())
        .collect();
    outside.sort_unstable();
    outside.dedup();
    for name in outside {
        out.push(format!("{name} is outside the commander's color identity"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_cards_load_in_model_order() {
        let defs = fdn();
        assert_eq!(card_count(), 286);
        assert_eq!(defs[0].name, "Abrade", "same order as assets/FDN_cards.json");
    }

    #[test]
    fn support_is_basics_keyword_creatures_and_scripts() {
        // Changing this is a deliberate act: each new card needs its own rules test.
        let keyword_only = [
            "Aegis Turtle",
            "Brazen Scourge",
            "Healer's Hawk",
            "Quakestrider Ceratops",
            "Savannah Lions",
            "Serra Angel",
            "Swiftblade Vindicator",
            "Vampire Nighthawk",
        ];
        let basics = ["Forest", "Island", "Mountain", "Plains", "Swamp"];
        let mut expected: Vec<&str> = keyword_only
            .iter()
            .chain(&basics)
            .chain(scripts::SCRIPTED)
            .copied()
            .collect();
        expected.sort_unstable();
        let mut got = supported();
        got.sort_unstable();
        assert_eq!(got, expected);
    }

    #[test]
    fn every_script_matches_its_card_text() {
        for &name in scripts::SCRIPTED {
            let i = index_of(name).unwrap_or_else(|| panic!("{name} is not in FDN"));
            assert!(
                supported_mask()[i],
                "{name}: script lines don't match the card text"
            );
        }
    }

    #[test]
    fn tokens_follow_the_cards() {
        let defs = fdn();
        assert_eq!(defs.len(), card_count() + scripts::token_defs().len());
        assert!(defs[card_count()..].iter().all(|d| d.token));
    }

    #[test]
    fn supported_has_no_duplicates() {
        let mut names = supported();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), supported().len());
    }

    #[test]
    fn foundations_is_unchanged_by_the_set_list() {
        let f = set("FDN").unwrap();
        assert!(Arc::ptr_eq(&f.defs, &fdn()), "fdn() is the FDN set");
        assert_eq!(f.card_count(), 286);
        assert!(set("XYZ").is_none());
    }

    #[test]
    fn tarkir_loads_in_model_order_with_its_own_tokens() {
        let t = set("TDM").unwrap();
        assert_eq!(t.card_count(), 286);
        assert_eq!(
            t.oracle[0].name, "Abzan Devotee",
            "the 17lands draft data's column order"
        );
        assert_eq!(t.defs.len(), 286 + scripts::token_defs().len());
        assert!(t.defs[286..].iter().all(|d| d.token));
        for basic in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            let i = t.index_of(basic).unwrap();
            assert!(t.supported[i], "{basic}");
        }
    }

    #[test]
    fn every_bloomburrow_commander_script_matches_its_card_text() {
        let t = set("BLC").unwrap();
        for &name in scripts::BLC_B_SCRIPTED.iter().chain(scripts::BLC_C_SCRIPTED) {
            let i = t.index_of(name).unwrap_or_else(|| panic!("{name} is not in BLC"));
            assert!(t.supported[i], "{name}: script lines don't match the card text");
        }
    }

    #[test]
    fn every_playgroup_script_matches_its_card_text() {
        let t = set("CMD").unwrap();
        for &name in scripts::PG_B_SCRIPTED
            .iter()
            .chain(scripts::PG_C_SCRIPTED)
            .chain(scripts::PG_D1_SCRIPTED)
            .chain(scripts::PG_D2_SCRIPTED)
            .chain(scripts::PG_D3_SCRIPTED)
            .chain(scripts::PG_M_SCRIPTED)
            .chain(scripts::PG_D4_SCRIPTED)
        {
            let i = t
                .index_of(name)
                .unwrap_or_else(|| panic!("{name} is not in the CMD pool"));
            assert!(t.supported[i], "{name}: script lines don't match the card text");
        }
    }

    #[test]
    fn every_tarkir_script_matches_its_card_text() {
        let t = set("TDM").unwrap();
        let all = scripts::TDM_SCRIPTED
            .iter()
            .chain(scripts::TDM_B_SCRIPTED)
            .chain(scripts::TDM_C_SCRIPTED);
        for &name in all {
            let i = t.index_of(name).unwrap_or_else(|| panic!("{name} is not in TDM"));
            assert!(t.supported[i], "{name}: script lines don't match the card text");
        }
    }

    #[test]
    fn a_reprint_plays_the_same_in_both_sets() {
        // Scripts are keyed by name: a card in both sets is supported in both.
        let (f, t) = (set("FDN").unwrap(), set("TDM").unwrap());
        for name in t.supported_names() {
            if let Some(i) = f.index_of(name) {
                assert!(f.supported[i], "{name}");
            }
        }
    }

    #[test]
    fn color_identity_counts_costs_rules_text_and_omen_halves() {
        let t = set("TDM").unwrap();
        let id = |name: &str| color_identity(&t.oracle[t.index_of(name).unwrap()]);
        // {W}{U}{B}{R}{G} in its rules text, colorless cost.
        assert_eq!(id("Iridescent Tiger"), [true, true, true, true, true]);
        assert_eq!(
            id("Frontier Bivouac"),
            [false, true, false, true, true],
            "{{T}}: Add {{G}}, {{U}}, or {{R}}."
        );
        assert_eq!(id("Plains"), [false; 5], "basic land types don't count");
        assert!(id("Dirgur Island Dragon")[1]);
        assert_eq!(id("Boulderborn Dragon"), [false; 5]);
    }

    #[test]
    fn snow_covered_basics_may_repeat_in_commander() {
        // "Basic Snow Land — Forest" has the Basic supertype (CR 205.4), so the singleton rule exempts it.
        let t = set("CMD").unwrap();
        let i = |name: &str| t.index_of(name).unwrap();
        let deck = [
            i("Snow-Covered Forest"),
            i("Snow-Covered Forest"),
            i("Forest"),
            i("Forest"),
        ];
        assert!(
            singleton_problems(t, &deck).is_empty(),
            "{:?}",
            singleton_problems(t, &deck)
        );
    }

    #[test]
    fn singleton_and_commander_deck_checks() {
        let t = set("TDM").unwrap();
        let i = |name: &str| t.index_of(name).unwrap();
        let deck = [
            i("Humbling Elder"),
            i("Humbling Elder"),
            i("Island"),
            i("Island"),
            i("Tempest Hawk"),
            i("Tempest Hawk"),
        ];
        assert_eq!(
            singleton_problems(t, &deck),
            ["Humbling Elder"],
            "basic lands and Tempest Hawk may repeat"
        );
        let commander = i("Neriv, Heart of the Storm"); // {1}{R}{W}{B}
        let mut ok: Vec<usize> = vec![i("Plains"); 98];
        ok.push(i("Bearer of Glory"));
        assert!(
            commander_deck_problems(t, commander, &ok).is_empty(),
            "{:?}",
            commander_deck_problems(t, commander, &ok)
        );
        let mut bad = ok.clone();
        bad[0] = i("Humbling Elder"); // blue
        bad.push(i("Bearer of Glory"));
        let problems = commander_deck_problems(t, commander, &bad);
        assert!(problems.iter().any(|p| p.contains("Humbling Elder is outside")));
        assert!(problems
            .iter()
            .any(|p| p.contains("more than one Bearer of Glory")));
        assert!(problems.iter().any(|p| p.contains("101 cards")));
        assert!(
            !commander_deck_problems(t, i("Snowmelt Stag"), &ok).is_empty(),
            "not legendary"
        );
    }

    #[test]
    fn reminder_text_is_ignored() {
        assert_eq!(
            abilities("Flying (This can't be blocked.)\nLifelink"),
            ["Flying", "Lifelink"]
        );
    }
}
