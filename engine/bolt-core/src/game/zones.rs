//! Mac C (objects and zones): Auras put onto the battlefield attached without being cast (CR
//! 303.4f), and Auras moved from one object to another (CR 701.3). game.rs calls these from
//! `run_effect` (`Effect::{AttachAurasToSource, SearchAuraForSourceIfCast,
//! SearchAuraForLoneAttacker}`), `decision_actions` and `decide` (`Decision::AttachAura`).

use super::*;

/// Dragonhawk's delayed end-step damage: (source, trigger number, controller, the turn whose end step
/// it waits for, the exiled cards with their zone-change counts).
pub type EndStepExiled = (ObjId, usize, PlayerId, u32, Vec<(ObjId, u32)>);

impl Game {
    /// Whether `aura` could enchant `x` (CR 303.4a), for an Aura controlled by `p`: the Aura's
    /// "enchant" restriction (its spell's first target) and protection. It isn't targeted, so
    /// hexproof doesn't matter (CR 303.4f); `x` is checked as a permanent on the battlefield.
    pub(super) fn could_enchant(&self, aura: ObjId, x: ObjId, p: PlayerId) -> bool {
        let d = self.def_of(aura);
        if !d.aura || d.reanimating_aura || aura == x || self.obj(x).zone != Zone::Battlefield {
            return false;
        }
        let Some(f) = d.spell.as_ref().and_then(|s| s.targets.first()).copied() else {
            return false;
        };
        // `triggering` skips the shroud and hexproof checks, which are about targeting.
        let f = Filter {
            triggering: true,
            ..f
        };
        !self.protected_from(x, aura) && self.target_ok(&f, p, Target::Object(x))
    }

    /// Put the Aura card `aura` onto the battlefield under `p`'s control attached to `to` (whose
    /// zone-change count is `zcc`), without casting it (CR 303.4f). If it can't legally enchant
    /// `to`, it stays where it is (CR 303.4g, 303.4i). Returns whether it entered.
    pub fn put_aura_attached(&mut self, aura: ObjId, to: ObjId, zcc: u32, p: PlayerId) -> bool {
        if self.obj(to).zcc != zcc || !self.could_enchant(aura, to, p) {
            return false;
        }
        self.move_to(aura, Zone::Battlefield);
        if self.obj(aura).zone != Zone::Battlefield {
            return false;
        }
        let turn = self.turn;
        let o = self.obj_mut(aura);
        o.controller = p;
        o.since_turn = turn;
        o.attached_to = Some((to, zcc));
        true
    }

    /// The cards `Decision::AttachAura` offers to `p`.
    pub(super) fn attach_aura_options(
        &self,
        p: PlayerId,
        to: ObjId,
        zcc: u32,
        library: bool,
        max_mv: u32,
        new_name: bool,
    ) -> Vec<ObjId> {
        let t = self.obj(to);
        if t.zone != Zone::Battlefield || t.zcc != zcc {
            return Vec::new();
        }
        if library {
            let names: Vec<&str> = self
                .permanents(p)
                .filter(|&a| self.has_subtype(a, "Aura"))
                .map(|a| self.def_of(a).name.as_str())
                .collect();
            return self.players[p]
                .library
                .iter()
                .copied()
                .filter(|&c| {
                    let d = self.def_of(c);
                    d.cost.mana_value() <= max_mv
                        && (!new_name || !names.contains(&d.name.as_str()))
                        && self.could_enchant(c, to, p)
                })
                .collect();
        }
        let on_battlefield = self.battlefield.iter().copied().filter(|&a| {
            self.obj(a).attached_to != Some((to, zcc)) && self.could_enchant(a, to, self.obj(a).controller)
        });
        let cards = self.players[p]
            .graveyard
            .iter()
            .chain(self.players[p].hand.iter())
            .copied()
            .filter(|&c| self.could_enchant(c, to, p));
        on_battlefield.chain(cards).collect()
    }

    /// `Decision::AttachAura`'s legal answers: the Auras offered, and `No`.
    pub(super) fn attach_aura_actions(&self, p: PlayerId, d: &Decision) -> Vec<Action> {
        let Decision::AttachAura {
            to,
            zcc,
            library,
            max_mv,
            new_name,
        } = *d
        else {
            return vec![Action::No];
        };
        let mut v: Vec<Action> = self
            .attach_aura_options(p, to, zcc, library, max_mv, new_name)
            .into_iter()
            .map(Action::Pick)
            .collect();
        v.push(Action::No);
        v
    }

    /// Ask `p` for an Aura to attach to `to` (nothing is asked when there's none to offer); a
    /// library search shuffles either way.
    pub(super) fn ask_attach_aura(
        &mut self,
        p: PlayerId,
        to: ObjId,
        library: bool,
        max_mv: u32,
        new_name: bool,
    ) {
        let zcc = self.obj(to).zcc;
        if self
            .attach_aura_options(p, to, zcc, library, max_mv, new_name)
            .is_empty()
        {
            if library {
                self.shuffle_library(p);
            }
            return;
        }
        self.ask(
            p,
            Decision::AttachAura {
                to,
                zcc,
                library,
                max_mv,
                new_name,
            },
        );
    }

    /// `Decision::AttachAura`'s answer: a card from the library, graveyard or hand enters attached;
    /// an Aura on the battlefield moves onto `to` (CR 701.3a). A library search takes one card and
    /// then shuffles; otherwise it's asked again while more can be attached.
    pub(super) fn attach_aura_decide(&mut self, p: PlayerId, d: Decision, a: Action) {
        let Decision::AttachAura {
            to,
            zcc,
            library,
            max_mv,
            new_name,
        } = d
        else {
            return;
        };
        if let Action::Pick(c) = a {
            if self.obj(c).zone == Zone::Battlefield {
                self.obj_mut(c).attached_to = Some((to, zcc));
            } else {
                self.put_aura_attached(c, to, zcc, p);
            }
            if !library
                && !self
                    .attach_aura_options(p, to, zcc, false, max_mv, new_name)
                    .is_empty()
            {
                self.ask(p, d);
                return;
            }
        }
        if library {
            self.shuffle_library(p);
        }
    }

    /// Light-Paws: "Whenever an Aura you control enters, if you cast it, you may search your
    /// library for an Aura card with mana value less than or equal to that Aura and with a
    /// different name than each Aura you control, put that card onto the battlefield attached to
    /// Light-Paws, then shuffle." `slot` holds the entering Aura; "if you cast it" is
    /// `Object::was_cast` (set as a spell resolves onto the battlefield, cleared as anything
    /// enters), checked as the trigger resolves.
    pub(super) fn search_aura_for_source_if_cast(&mut self, item: &StackItem, legal: &[bool], slot: u8) {
        let Some(aura) = Game::slot_object(item, legal, slot) else {
            return;
        };
        if !self.obj(aura).was_cast || self.obj(item.source).zone != Zone::Battlefield {
            return;
        }
        let mv = self.def_of(aura).cost.mana_value();
        self.ask_attach_aura(item.controller, item.source, true, mv, true);
    }

    /// Sovereigns of Lost Alara, from a "whenever you attack" trigger (as exalted is): if exactly
    /// one creature you control is attacking, you may search your library for an Aura card that
    /// could enchant it, put it onto the battlefield attached to it, then shuffle.
    pub(super) fn search_aura_for_lone_attacker(&mut self, p: PlayerId) {
        let mine: Vec<ObjId> = self
            .combat
            .as_ref()
            .map(|c| {
                c.attackers
                    .iter()
                    .copied()
                    .filter(|&a| self.obj(a).controller == p)
                    .collect()
            })
            .unwrap_or_default();
        if let [a] = mine[..] {
            if self.obj(a).zone == Zone::Battlefield {
                self.ask_attach_aura(p, a, true, u32::MAX, false);
            }
        }
    }

    /// Olivia, Crimson Bride: `c` (a creature card in `p`'s graveyard) onto the battlefield tapped
    /// and attacking, with "When you don't control a legendary Vampire, exile this creature."
    pub(super) fn return_attacking_until_no_legendary_vampire(&mut self, c: ObjId, p: PlayerId) {
        if self.obj(c).zone != Zone::Graveyard || !self.def_of(c).types.creature {
            return;
        }
        self.obj_mut(c).controller = p;
        self.move_to(c, Zone::Battlefield);
        if self.obj(c).zone != Zone::Battlefield {
            return;
        }
        let turn = self.turn;
        let o = self.obj_mut(c);
        o.controller = p;
        o.since_turn = turn;
        o.tapped = true;
        o.exile_without_legendary_vampire = Some(o.zcc);
        if let Some(combat) = self.combat.as_mut() {
            combat.attackers.push(c); // attacking, but never declared (CR 506.3a)
        }
    }

    /// Olivia's granted ability, checked whenever a player would receive priority: a returned
    /// creature whose controller controls no legendary Vampire is exiled. (A state trigger, CR
    /// 603.8, done at once rather than on the stack.)
    pub(super) fn exile_without_legendary_vampire(&mut self) {
        let doomed: Vec<ObjId> = self
            .battlefield
            .iter()
            .copied()
            .filter(|&x| {
                let o = self.obj(x);
                o.exile_without_legendary_vampire == Some(o.zcc)
                    && !self
                        .permanents(o.controller)
                        .any(|v| self.types_of(v).legendary && self.has_subtype(v, "Vampire"))
            })
            .collect();
        for x in doomed {
            self.move_to(x, Zone::Exile);
        }
    }

    /// The turn whose end step is `p`'s next one: this turn's, unless it has begun already or it
    /// isn't `p`'s turn.
    fn next_end_step_turn(&self, p: PlayerId) -> u32 {
        if self.active == p && !matches!(self.step, Step::End | Step::Cleanup) {
            self.turn
        } else {
            self.next_turn_of(p)
        }
    }

    /// Dragonhawk, Fate's Tempest: exile the top X, playable until `p`'s next end step, and set up
    /// the end step's damage (the source's trigger number `trigger`).
    pub(super) fn exile_top_power_four(&mut self, src: ObjId, trigger: usize, p: PlayerId) {
        let x = self.creatures(p).filter(|&c| self.power(c) >= 4).count();
        let until = self.next_end_step_turn(p);
        let mut exiled = Vec::new();
        for _ in 0..x {
            let Some(&c) = self.players[p].library.last() else {
                break;
            };
            self.move_to(c, Zone::Exile);
            if self.obj(c).zone == Zone::Exile {
                self.obj_mut(c).playable = Some((p, until));
                exiled.push((c, self.obj(c).zcc));
            }
        }
        if !exiled.is_empty() {
            self.delayed_end_step_exiled
                .push((src, trigger, p, until, exiled));
        }
    }

    /// At the beginning of `a`'s end step: Dragonhawk's delayed triggers that wait for it, each
    /// with 2 damage for each of its cards still exiled as its amount.
    pub(super) fn delayed_end_step_exiled_due(&mut self, a: PlayerId) {
        let turn = self.turn;
        let (due, wait): (Vec<_>, Vec<_>) = std::mem::take(&mut self.delayed_end_step_exiled)
            .into_iter()
            .partition(|e| e.2 == a && e.3 <= turn);
        self.delayed_end_step_exiled = wait;
        for (src, n, q, _, cards) in due {
            let still = cards
                .iter()
                .filter(|&&(c, z)| self.obj(c).zone == Zone::Exile && self.obj(c).zcc == z)
                .count() as i32;
            self.triggered.push(StackItem {
                kind: StackKind::Trigger(n),
                amount: 2 * still,
                ..Game::casting(src, q)
            });
        }
    }

    /// Suspend (CR 702.62a): the card the plot-style ability exiled from the hand gets `n` time
    /// counters and isn't plotted; the upkeep removes them and casts it free.
    pub(super) fn suspend_source(&mut self, src: ObjId, n: u8) {
        let o = self.obj_mut(src);
        let zcc = o.zcc;
        if o.zone == Zone::Exile && o.plotted.is_some_and(|(_, z)| z == zcc) {
            o.plotted = None;
            o.cast_free = false;
            o.time_counters = n;
        }
    }

    /// What the Aura card `aura` could enter attached to for `p` (CR 303.4f).
    pub(super) fn aura_hosts(&self, aura: ObjId, p: PlayerId) -> Vec<ObjId> {
        self.battlefield
            .iter()
            .copied()
            .filter(|&x| self.could_enchant(aura, x, p))
            .collect()
    }

    /// Glimpse of Tomorrow, for its controller `p`.
    pub(super) fn glimpse_of_tomorrow(&mut self, p: PlayerId) {
        let mine: Vec<ObjId> = self
            .battlefield
            .iter()
            .copied()
            .filter(|&x| self.obj(x).owner == p)
            .collect();
        let n = mine.len();
        for x in mine {
            if self.obj(x).zone != Zone::Battlefield {
                continue; // gone already (an Animate Dead's creature)
            }
            self.move_to(x, Zone::Library);
            if self.obj(x).zone == Zone::Library && (self.is_token(x) || self.obj(x).card_copy) {
                self.move_to(x, Zone::Gone); // a token shuffled in ceases to exist (CR 111.7)
            }
        }
        self.shuffle_library(p);
        let lib = &self.players[p].library;
        let revealed: Vec<ObjId> = lib.iter().rev().take(n).copied().collect();
        let mut auras = Vec::new();
        let mut rest = Vec::new();
        for c in revealed {
            let d = self.def_of(c);
            if !d.types.is_permanent() {
                rest.push(c);
            } else if d.aura || d.reanimating_aura {
                auras.push(c);
            } else {
                self.move_to(c, Zone::Battlefield);
                if self.obj(c).zone == Zone::Battlefield {
                    self.obj_mut(c).controller = p;
                }
            }
        }
        self.glimpse_next_aura(p, auras, rest);
    }

    /// Glimpse of Tomorrow's Aura cards, one at a time; one that can't enchant anything stays in
    /// the library and goes to the bottom with the rest (CR 303.4g).
    pub(super) fn glimpse_next_aura(&mut self, p: PlayerId, mut auras: Vec<ObjId>, mut rest: Vec<ObjId>) {
        while !auras.is_empty() {
            let aura = auras.remove(0);
            if self.obj(aura).zone == Zone::Library && !self.aura_hosts(aura, p).is_empty() {
                self.ask(p, Decision::AuraEntersAttachedTo { aura, auras, rest });
                return;
            }
            rest.push(aura);
        }
        let rest: Vec<ObjId> = rest
            .into_iter()
            .filter(|&c| self.obj(c).zone == Zone::Library)
            .collect();
        self.put_among_rest(p, rest, false);
    }

    /// An Aura spell `aura` just resolved onto the battlefield: its controller's "whenever an Aura
    /// you control enters, if you cast it" triggers (`When::CastAuraYouControlEnters`).
    pub(super) fn cast_aura_entered(&mut self, aura: ObjId) {
        if self.obj(aura).zone != Zone::Battlefield {
            return;
        }
        let p = self.obj(aura).controller;
        for x in self.permanents(p).collect::<Vec<_>>() {
            if self.lost_abilities(x) {
                continue;
            }
            let whens: Vec<When> = self.def_of(x).triggers.iter().map(|t| t.when).collect();
            for (n, w) in whens.into_iter().enumerate() {
                if w == When::CastAuraYouControlEnters {
                    let before = self.triggered.len();
                    self.triggered.push(StackItem {
                        kind: StackKind::Trigger(n),
                        ..Game::casting(x, p)
                    });
                    self.fill_triggering(before, aura);
                }
            }
        }
    }

    /// Gisa, Glorious Resurrector's upkeep: the creature cards exiled with `src` (this object of
    /// it) that are still those exiled cards come back under `p`'s control with decayed.
    pub(super) fn return_exiled_with_decayed(&mut self, src: ObjId, p: PlayerId) {
        let szcc = self.obj(src).zcc;
        let (mine, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.exiled_with)
            .into_iter()
            .partition(|e| e.0 == src && e.1 == szcc);
        self.exiled_with = rest;
        for (_, _, c, czcc) in mine {
            let o = self.obj(c);
            if o.zone != Zone::Exile || o.zcc != czcc || !self.def_of(c).types.creature {
                continue;
            }
            self.obj_mut(c).controller = p;
            self.move_to(c, Zone::Battlefield);
            if self.obj(c).zone == Zone::Battlefield {
                let o = self.obj_mut(c);
                o.controller = p;
                o.decayed = true;
            }
        }
    }

    /// Whether `p` may cast the modal double-faced card `c` from hand as its back face now: a
    /// nonland permanent back (not an Aura) with no {X}, at its timing (`timing_ok` is sorcery timing
    /// or flash for everything; a back with flash ignores it), with its mana cost affordable. Offered
    /// as `Action::CastModal(c, 1)` (the front has no modes).
    pub(super) fn back_face_castable(&self, p: PlayerId, c: ObjId, timing_ok: bool) -> bool {
        let d = &self.defs[self.obj(c).def];
        let Some(b) = d.back.as_ref() else {
            return false;
        };
        d.modal_back
            && d.modes.is_empty()
            && self.obj(c).zone == Zone::Hand
            && !self.obj(c).transformed
            && !b.types.land
            && b.types.is_permanent()
            && !b.aura
            && b.cost.x == 0
            && (timing_ok || b.keywords.has(Keywords::FLASH))
            && !self.cant_cast_now(p)
            && self.can_pay_for(p, &b.cost, None, PayFor::Spell(c))
    }

    /// `Action::CastModal(c, 1)` means "cast its back face" for a modal double-faced card in hand
    /// whose front has no modes.
    pub(super) fn casts_back_face(&self, c: ObjId) -> bool {
        let d = &self.defs[self.obj(c).def];
        d.modal_back && d.modes.is_empty() && self.obj(c).zone == Zone::Hand && d.back.is_some()
    }

    /// The creatures `p` controls that the Aura `aura` could be moved onto (Breath of Fury).
    pub(super) fn move_aura_options(&self, aura: ObjId, p: PlayerId) -> Vec<ObjId> {
        let on = self.obj(aura).attached_to.map(|(c, _)| c);
        self.creatures(p)
            .filter(|&c| Some(c) != on && self.could_enchant(aura, c, p))
            .collect()
    }

    /// Breath of Fury: "sacrifice it and attach this Aura to a creature you control. If you do,
    /// ..." The Aura is moved before state-based actions could put it into the graveyard; with no
    /// creature to move it to, the rest doesn't happen.
    pub(super) fn sacrifice_enchanted_move_aura(&mut self, aura: ObjId, p: PlayerId) {
        let gone = |g: &mut Game| g.resolving.as_mut().expect("resolving").remaining.clear();
        if self.obj(aura).zone != Zone::Battlefield {
            return gone(self);
        }
        if let Some((c, z)) = self.obj(aura).attached_to {
            let o = self.obj(c);
            if o.zone == Zone::Battlefield && o.zcc == z && o.controller == p && !self.cant_be_sacrificed(c) {
                self.move_to(c, Zone::Graveyard);
                self.sacrificed(c, p);
            }
        }
        if self.move_aura_options(aura, p).is_empty() {
            return gone(self);
        }
        self.ask(p, Decision::MoveAuraTo { aura });
    }

    /// Creature cards in `p`'s hand with one of `subtypes` (Kaalia: Angel, Demon, Dragon).
    pub(super) fn creatures_in_hand_with(&self, p: PlayerId, subtypes: &[&str]) -> Vec<ObjId> {
        self.players[p]
            .hand
            .iter()
            .copied()
            .filter(|&c| {
                let d = self.def_of(c);
                d.types.creature
                    && (d.changeling || d.subtypes.iter().any(|s| subtypes.contains(&s.as_str())))
            })
            .collect()
    }

    /// Put `c` onto the battlefield under `p`'s control tapped and attacking (never declared as an
    /// attacker, so "attacks" triggers don't fire, CR 506.3a).
    pub(super) fn put_onto_battlefield_attacking(&mut self, c: ObjId, p: PlayerId) {
        self.move_to(c, Zone::Battlefield);
        if self.obj(c).zone != Zone::Battlefield {
            return;
        }
        let o = self.obj_mut(c);
        o.controller = p;
        o.tapped = true;
        if let Some(combat) = self.combat.as_mut() {
            combat.attackers.push(c);
        }
    }

    /// Lithoform Engine's {2}: copy the topmost ability of the targeted source if you control it
    /// (`Filter::abilities` checks no controller). The copy goes on the stack after this ability
    /// finishes resolving, choosing new targets (`pending_copies`).
    pub(super) fn copy_target_ability(&mut self, item: &StackItem, legal: &[bool], slot: u8) {
        let Some(src) = Game::slot_object(item, legal, slot) else {
            return;
        };
        let Some(n) = self.ability_on_stack(src) else {
            return;
        };
        if self.items[n].controller == item.controller {
            let copy = self.items[n].clone();
            self.pending_copies.push(copy);
        }
    }

    /// Lithoform Engine's {3} and {4}: copy the targeted spell you control (an instant or sorcery
    /// by its filter, or with `permanent` a permanent spell, whose copy resolves into a token).
    pub(super) fn copy_target_spell(&mut self, item: &StackItem, legal: &[bool], slot: u8, permanent: bool) {
        let Some(card) = Game::slot_object(item, legal, slot) else {
            return;
        };
        if permanent && !self.types_of(card).is_permanent() {
            return;
        }
        let spell = self
            .items
            .iter()
            .rev()
            .find(|i| i.kind == StackKind::Spell && i.source == card)
            .cloned();
        let Some(spell) = spell.filter(|s| s.controller == item.controller) else {
            return;
        };
        self.pending_copies.push(StackItem {
            kind: StackKind::SpellCopy,
            ..spell
        });
        self.copied_or_cast_instant_sorcery(item.controller, card);
    }

    /// The source becomes a copy of the permanent in `slot` for good (CR 707.2). With `keep_own`
    /// it keeps its own card's `copy_only` abilities (Thespian's Stage), as a clone with copy
    /// exceptions does.
    pub(super) fn source_becomes_copy_of_slot(
        &mut self,
        item: &StackItem,
        legal: &[bool],
        slot: u8,
        keep_own: bool,
    ) {
        let src = item.source;
        let Some(t) = Game::slot_object(item, legal, slot) else {
            return;
        };
        if self.obj(src).zone != Zone::Battlefield || t == src {
            return;
        }
        let of = self.obj(t).def;
        self.become_copy(src, of);
        self.obj_mut(src).copy_exceptions = keep_own;
    }

    /// `fire`, with the triggers it adds carrying `card` as their `extra` ("that spell": Wandering
    /// Archaic's copy).
    pub(super) fn fire_with_extra(&mut self, p: PlayerId, when: When, card: ObjId) {
        let before = self.triggered.len();
        self.fire(p, when, None);
        for t in &mut self.triggered[before..] {
            t.extra = Some(card);
        }
    }

    /// "Whenever you play a card from exile" (Prosper): `p` is about to cast `card` or play it as a
    /// land; called before it moves, so its zone is still the one it's played from.
    pub(super) fn played_from_exile(&mut self, p: PlayerId, card: ObjId) {
        if self.obj(card).zone == Zone::Exile {
            self.fire(p, When::YouPlayCardFromExile, None);
        }
    }

    /// A searched-for card onto the battlefield with an additional +1/+1 counter (Neoform). The
    /// counter is put on as it arrives.
    pub(super) fn enter_with_plus_counter(&mut self, c: ObjId) {
        self.move_to(c, Zone::Battlefield);
        if self.obj(c).zone == Zone::Battlefield {
            self.add_counters(c, 1);
        }
    }

    /// Blood Money: destroy all creatures, then a tapped `def` token for each nontoken creature
    /// destroyed this way (one that's indestructible or regenerated doesn't count).
    pub(super) fn destroy_all_creatures_tapped_token_per_nontoken(&mut self, p: PlayerId, def: usize) {
        let all: Vec<ObjId> = self
            .battlefield
            .iter()
            .copied()
            .filter(|&c| self.types_of(c).creature)
            .collect();
        let mut n = 0;
        for c in all {
            let nontoken = !self.is_token(c);
            self.destroy(c);
            if nontoken && self.obj(c).zone != Zone::Battlefield {
                n += 1;
            }
        }
        for _ in 0..n {
            self.create_token(p, def);
            for t in self.last_created.clone() {
                self.obj_mut(t).tapped = true;
            }
        }
    }

    /// Peer into the Abyss: `q` draws half the cards in their library and loses half their life,
    /// each rounded up.
    pub(super) fn draw_half_library_lose_half_life(&mut self, q: PlayerId) {
        let n = self.players[q].library.len().div_ceil(2);
        for _ in 0..n {
            self.draw(q);
        }
        let life = self.players[q].life;
        if life > 0 {
            self.lose_life(q, (life + 1) / 2);
        }
    }

    /// Otherworld Atlas: each player, the active player first, draws a card for each of the
    /// source's kind counters.
    pub(super) fn each_player_draws_per_kind_counter(&mut self, src: ObjId) {
        let n = self.obj(src).kind_counters.max(0);
        let first = self.active;
        for q in [first, opponent(first)] {
            for _ in 0..n {
                self.draw(q);
            }
        }
    }

    /// Gilded Drake: exchange control of the source and the creature in `slot` (CR 701.12b: only if
    /// both are still on the battlefield and controlled by different players); otherwise sacrifice
    /// the source.
    pub(super) fn exchange_control_or_sacrifice(&mut self, item: &StackItem, legal: &[bool], slot: u8) {
        let src = item.source;
        let on = |g: &Game, x: ObjId| g.obj(x).zone == Zone::Battlefield;
        let other = Game::slot_object(item, legal, slot).filter(|&t| on(self, t));
        if let Some(t) =
            other.filter(|&t| on(self, src) && self.obj(t).controller != self.obj(src).controller)
        {
            let (a, b) = (self.obj(src).controller, self.obj(t).controller);
            let turn = self.turn;
            for (x, to) in [(src, b), (t, a)] {
                let o = self.obj_mut(x);
                o.controller = to;
                o.since_turn = turn; // CR 302.6
            }
        } else if on(self, src) && !self.cant_be_sacrificed(src) {
            let p = self.obj(src).controller;
            self.move_to(src, Zone::Graveyard);
            self.sacrificed(src, p);
        }
    }

    /// The creature `src` (an Equipment or Aura) is attached to, if it's still that object on the
    /// battlefield.
    fn attached_creature(&self, src: ObjId) -> Option<ObjId> {
        self.obj(src).attached_to.and_then(|(c, zcc)| {
            let o = self.obj(c);
            (o.zcc == zcc && o.zone == Zone::Battlefield).then_some(c)
        })
    }

    /// Bone Sabres: `n` +1/+1 counters on the equipped creature.
    pub(super) fn counters_on_attached(&mut self, src: ObjId, n: i32) {
        if let Some(c) = self.attached_creature(src) {
            self.add_counters(c, n);
        }
    }

    /// Mechanized Production: a token copy of the enchanted artifact for `p` (doubled like any
    /// token), then `p` wins if they control eight or more artifacts with the same name.
    pub(super) fn copy_attached_then_eight_same_name_wins(&mut self, src: ObjId, p: PlayerId) {
        if let Some(a) = self.attached_creature(src) {
            self.last_created.clear();
            for _ in 0..self.token_doubling(p) {
                if let Some(id) = self.token_copy_of(p, a) {
                    self.last_created.push(id);
                }
            }
        }
        let mut names: Vec<&str> = self
            .permanents(p)
            .filter(|&x| self.types_of(x).artifact)
            .map(|x| self.def_of(x).name.as_str())
            .collect();
        names.sort_unstable();
        let most = names
            .chunk_by(|a, b| a == b)
            .map(<[&str]>::len)
            .max()
            .unwrap_or(0);
        if most < 8 {
            return;
        }
        let opp = opponent(p);
        let protected = self.permanents(opp).any(|c| self.def_of(c).cant_lose);
        if !protected && !self.players[opp].lost {
            self.players[opp].lost = true;
            self.events.push(Event::Lost { player: opp });
            self.result = Some(Outcome::Winner(p));
            self.pending = Pending::GameOver;
        }
    }

    /// Final Act: destroy all planeswalkers.
    pub(super) fn destroy_all_planeswalkers(&mut self) {
        let all: Vec<ObjId> = self
            .battlefield
            .iter()
            .copied()
            .filter(|&c| self.types_of(c).planeswalker)
            .collect();
        for c in all {
            self.destroy(c);
        }
    }

    /// Final Act: each opponent of `p` (the one opponent) loses all counters: poison and energy,
    /// the player counters the engine has.
    pub(super) fn opponent_loses_all_counters(&mut self, p: PlayerId) {
        let q = &mut self.players[opponent(p)];
        q.poison = 0;
        q.energy = 0;
    }

    /// Rakdos, Patron of Chaos: `q`'s nonland, nontoken permanents that can be sacrificed.
    fn patron_of_chaos_options(&self, q: PlayerId) -> Vec<ObjId> {
        self.permanents(q)
            .filter(|&x| !self.types_of(x).land && !self.is_token(x) && !self.cant_be_sacrificed(x))
            .collect()
    }

    /// Ask `q` whether to sacrifice two, if they can; otherwise `controller` draws two.
    pub(super) fn patron_of_chaos_ask(&mut self, q: PlayerId, controller: PlayerId) {
        if self.patron_of_chaos_options(q).len() >= 2 {
            self.ask(
                q,
                Decision::PatronOfChaosSacrifice {
                    controller,
                    first: None,
                },
            );
        } else {
            for _ in 0..2 {
                self.draw(controller);
            }
        }
    }

    pub(super) fn patron_of_chaos_actions(&self, q: PlayerId, first: Option<ObjId>) -> Vec<Action> {
        let mut v: Vec<Action> = self
            .patron_of_chaos_options(q)
            .into_iter()
            .filter(|&x| Some(x) != first)
            .map(Action::Pick)
            .collect();
        if first.is_none() {
            v.push(Action::No);
        }
        v
    }

    /// The first pick waits for the second; both are then sacrificed together. `No` (before the
    /// first) means they don't, so `controller` draws two.
    pub(super) fn patron_of_chaos_decide(
        &mut self,
        q: PlayerId,
        controller: PlayerId,
        first: Option<ObjId>,
        a: Action,
    ) {
        match (first, a) {
            (None, Action::Pick(c)) => {
                self.ask(
                    q,
                    Decision::PatronOfChaosSacrifice {
                        controller,
                        first: Some(c),
                    },
                );
            }
            (Some(f), Action::Pick(c)) => {
                for x in [f, c] {
                    if self.obj(x).zone == Zone::Battlefield {
                        self.move_to(x, Zone::Graveyard);
                        self.sacrificed(x, q);
                    }
                }
            }
            _ => {
                for _ in 0..2 {
                    self.draw(controller);
                }
            }
        }
    }

    /// Elder Brain, attacking a player (1v1: the defending player): exile their hand, they draw
    /// that many, and `p` may play the exiled cards while they stay exiled, spending mana as though
    /// it were any color (`Object::any_color_mana`, see `Game::spell_cost`).
    pub(super) fn exile_defending_hand_draw_that_many(&mut self, src: ObjId, p: PlayerId) {
        let at_planeswalker = self
            .combat
            .as_ref()
            .is_some_and(|c| c.at_planeswalker.iter().any(|&(x, _)| x == src));
        if at_planeswalker {
            return;
        }
        let q = opponent(p);
        let hand = self.players[q].hand.clone();
        let mut n = 0;
        for c in hand {
            self.move_to(c, Zone::Exile);
            if self.obj(c).zone == Zone::Exile {
                let o = self.obj_mut(c);
                o.playable = Some((p, u32::MAX));
                o.any_color_mana = true;
                n += 1;
            }
        }
        for _ in 0..n {
            self.draw(q);
        }
    }

    /// Deep-Cavern Bat: `p` looks at `victim`'s hand and may exile a nonland card from it, if the
    /// source is still on the battlefield (CR 610.3c).
    pub(super) fn hand_link_ask(&mut self, src: ObjId, p: PlayerId, victim: PlayerId) {
        if self.obj(src).zone != Zone::Battlefield || self.hand_link_actions(victim).len() < 2 {
            return;
        }
        let zcc = self.obj(src).zcc;
        self.ask(
            p,
            Decision::ExileFromHandUntilSourceLeaves {
                victim,
                source: src,
                zcc,
            },
        );
    }

    pub(super) fn hand_link_actions(&self, victim: PlayerId) -> Vec<Action> {
        let mut v: Vec<Action> = self.players[victim]
            .hand
            .iter()
            .filter(|&&c| !self.types_of(c).land)
            .map(|&c| Action::Pick(c))
            .collect();
        v.push(Action::No);
        v
    }

    pub(super) fn hand_link_decide(&mut self, src: ObjId, zcc: u32, a: Action) {
        if let Action::Pick(c) = a {
            let o = self.obj(src);
            if o.zone == Zone::Battlefield && o.zcc == zcc {
                self.move_to(c, Zone::Exile);
                if self.obj(c).zone == Zone::Exile {
                    self.hand_links.push((src, zcc, c));
                }
            }
        }
    }

    /// `id` left the battlefield: the cards it exiled from a hand go back to their owners' hands.
    pub(super) fn return_hand_links(&mut self, id: ObjId) {
        let left_zcc = self.obj(id).zcc - 1;
        let (back, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut self.hand_links)
            .into_iter()
            .partition(|&(src, z, _)| src == id && z == left_zcc);
        self.hand_links = keep;
        for (_, _, card) in back {
            if self.obj(card).zone == Zone::Exile {
                self.move_to(card, Zone::Hand);
            }
        }
    }

    /// Rona, Tolarian Obliterator: `source`'s controller exiles a card from their hand at random;
    /// `p` may put a land onto the battlefield under their control, or cast anything else free.
    pub(super) fn exile_random_from_damager_hand(&mut self, source: ObjId, p: PlayerId) {
        let q = self.obj(source).controller;
        let n = self.players[q].hand.len();
        if n == 0 {
            return;
        }
        let c = self.players[q].hand[self.rng.below(n)];
        self.move_to(c, Zone::Exile);
        if self.obj(c).zone != Zone::Exile {
            return;
        }
        if self.types_of(c).land {
            self.ask(p, Decision::MayPutExiledLandOntoBattlefield { card: c });
        } else {
            self.obj_mut(c).cast_free = true;
            self.free_casts.push(c);
            self.free_caster = p;
        }
    }

    /// Nontoken creatures and planeswalkers `q` controls that can be sacrificed (Sheoldred).
    pub(super) fn nontoken_creature_or_planeswalker_options(&self, q: PlayerId) -> Vec<ObjId> {
        self.permanents(q)
            .filter(|&x| {
                let t = self.types_of(x);
                (t.creature || t.planeswalker) && !self.is_token(x) && !self.cant_be_sacrificed(x)
            })
            .collect()
    }

    /// Elspeth Tirel's −5: destroy every permanent other than `src` that isn't a land or a token.
    pub(super) fn destroy_all_other_nonland_nontoken(&mut self, src: ObjId) {
        let all: Vec<ObjId> = self
            .battlefield
            .iter()
            .copied()
            .filter(|&c| c != src && !self.types_of(c).land && !self.is_token(c))
            .collect();
        for c in all {
            self.destroy(c);
        }
    }

    /// Arthur, Marigold Knight: `c` onto the battlefield tapped and attacking, back to its owner's
    /// hand at end of combat.
    pub(super) fn put_attacking_until_end_of_combat(&mut self, c: ObjId, p: PlayerId) {
        self.put_onto_battlefield_attacking(c, p);
        if self.obj(c).zone == Zone::Battlefield {
            let zcc = self.obj(c).zcc;
            self.hand_at_end_combat.push((c, zcc));
        }
    }

    /// At end of combat: Arthur's creatures still on the battlefield go back to their owners' hands.
    pub(super) fn return_hand_at_end_combat(&mut self) {
        for (c, zcc) in std::mem::take(&mut self.hand_at_end_combat) {
            if self.obj(c).zone == Zone::Battlefield && self.obj(c).zcc == zcc {
                self.move_to(c, Zone::Hand);
            }
        }
    }

    /// Goliath Daydreamer: a resolving spell marked to be exiled with a dream counter. Returns
    /// whether it was exiled that way.
    pub(super) fn exile_with_dream_counter(&mut self, c: ObjId) -> bool {
        let zcc = self.obj(c).zcc;
        let Some(pos) = self.dream_marked.iter().position(|&x| x == (c, zcc)) else {
            return false;
        };
        self.dream_marked.remove(pos);
        self.move_to(c, Zone::Exile);
        if self.obj(c).zone == Zone::Exile {
            let zcc = self.obj(c).zcc;
            self.dream_counters.push((c, zcc));
        }
        true
    }

    /// Goliath Daydreamer's attack trigger: `p` may cast one card they own in exile with a dream
    /// counter on it, free.
    pub(super) fn open_dream_free_cast(&mut self, p: PlayerId) {
        self.dream_counters.retain(|&(c, zcc)| {
            self.objects[c.0 as usize].zone == Zone::Exile && self.objects[c.0 as usize].zcc == zcc
        });
        let cards: Vec<ObjId> = self
            .dream_counters
            .iter()
            .map(|&(c, _)| c)
            .filter(|&c| self.obj(c).owner == p)
            .collect();
        if cards.is_empty() {
            return;
        }
        for c in cards {
            self.obj_mut(c).cast_free = true;
            self.free_casts.push(c);
        }
        self.free_casts_one = true;
        self.free_caster = p;
    }

    /// Rev, Tithe Extractor: the top card of `q`'s library exiled face down; `p` may cast it for as
    /// long as it remains exiled (`playable` clears as it leaves exile). A land can't be cast.
    pub(super) fn exile_top_face_down_castable(&mut self, p: PlayerId, q: PlayerId) {
        let Some(&c) = self.players[q].library.last() else {
            return;
        };
        self.move_to(c, Zone::Exile);
        if self.obj(c).zone != Zone::Exile {
            return;
        }
        let zcc = self.obj(c).zcc;
        self.face_down_exiled.push((c, zcc));
        if !self.def_of(c).types.land {
            self.obj_mut(c).playable = Some((p, u32::MAX));
        }
    }

    /// Whether the exiled card `c` is face down (`Object::face_down`, or Rev's `face_down_exiled`).
    pub fn exiled_face_down(&self, c: ObjId) -> bool {
        let o = self.obj(c);
        o.face_down || self.face_down_exiled.contains(&(c, o.zcc))
    }

    /// Ashiok, Nightmare Muse's −7: a free-cast window for `p` over the face-up cards `p`'s opponents
    /// own in exile, closing after `max` spells.
    pub(super) fn open_free_casts_from_opponents_exile(&mut self, p: PlayerId, max: u8) {
        self.face_down_exiled.retain(|&(c, zcc)| {
            self.objects[c.0 as usize].zone == Zone::Exile && self.objects[c.0 as usize].zcc == zcc
        });
        let cards: Vec<ObjId> = self
            .exile
            .iter()
            .copied()
            .filter(|&c| self.obj(c).owner != p && !self.exiled_face_down(c))
            .collect();
        if cards.is_empty() {
            return;
        }
        for c in cards {
            self.obj_mut(c).cast_free = true;
            self.free_casts.push(c);
        }
        self.free_casts_max = max;
        self.free_caster = p;
    }

    /// How many cards of the open free-cast window have been cast (have left exile).
    pub(super) fn free_casts_cast(&self) -> usize {
        self.free_casts
            .iter()
            .filter(|&&c| self.obj(c).zone != Zone::Exile)
            .count()
    }

    /// Fortune Teller's Talent, level 2: whether `x` (a permanent `p` controls) lets `p` play the top
    /// card of their library now: at its Class level, and once `p` has cast a spell this turn.
    pub(super) fn top_cards_after_a_spell(&self, p: PlayerId, x: ObjId) -> bool {
        let level = self.def_of(x).zones.top_cards_if_cast_spell_at_level;
        level > 0 && self.class_level(x) >= level && self.spells_cast[p] > 0
    }

    /// Evercoat Ursine's second hideaway: keep the first card, then hide away again.
    pub(super) fn hideaway_another(&mut self, src: ObjId, p: PlayerId, n: u8) {
        if let Some(c) = self
            .obj(src)
            .hideaway
            .filter(|&c| self.obj(c).zone == Zone::Exile)
        {
            let entry = (src, self.obj(src).zcc, c, self.obj(c).zcc);
            self.hideaway_more.push(entry);
        }
        let cards: Vec<ObjId> = self.players[p]
            .library
            .iter()
            .rev()
            .take(n as usize)
            .copied()
            .collect();
        if !cards.is_empty() {
            self.ask(p, Decision::HideawayPick { source: src, cards });
        }
    }

    /// Evercoat Ursine's combat damage trigger: `p` may cast one of the cards hidden away with `src`,
    /// free.
    pub(super) fn open_hideaway_free_cast(&mut self, src: ObjId, p: PlayerId) {
        let zcc = self.obj(src).zcc;
        let mut cards: Vec<ObjId> = self
            .hideaway_more
            .iter()
            .filter(|&&(s, z, c, cz)| {
                s == src && z == zcc && self.obj(c).zone == Zone::Exile && self.obj(c).zcc == cz
            })
            .map(|&(_, _, c, _)| c)
            .collect();
        if let Some(c) = self
            .obj(src)
            .hideaway
            .filter(|&c| self.obj(c).zone == Zone::Exile)
        {
            if !cards.contains(&c) {
                cards.push(c);
            }
        }
        if cards.is_empty() {
            return;
        }
        for c in cards {
            self.obj_mut(c).cast_free = true;
            self.free_casts.push(c);
        }
        self.free_casts_one = true;
        self.free_caster = p;
    }

    /// Blade of the Bloodchief: `n` +1/+1 counters on the creature `src` is attached to, `n_if` if it
    /// has `subtype`.
    pub(super) fn counters_on_attached_by_subtype(&mut self, src: ObjId, n: i32, subtype: &str, n_if: i32) {
        if let Some(c) = self.attached_creature(src) {
            let k = if self.has_subtype(c, subtype) { n_if } else { n };
            self.add_counters(c, k);
        }
    }

    /// Wishclaw Talisman: "An opponent gains control of this artifact." (for good, CR 613.1b).
    pub(super) fn give_source_to_opponent(&mut self, src: ObjId, p: PlayerId) {
        if self.obj(src).zone == Zone::Battlefield && self.obj(src).controller == p {
            let turn = self.turn;
            let o = self.obj_mut(src);
            o.controller = opponent(p);
            o.since_turn = turn; // CR 302.6
        }
    }

    /// "This artifact enters with N [kind] counters on it." (`ZonesExt::enters_with_kind_counters`)
    pub(super) fn enter_with_kind_counters(&mut self, id: ObjId) {
        let n = self.def_of(id).zones.enters_with_kind_counters as i32;
        if n > 0 {
            self.obj_mut(id).kind_counters += n;
            let controller = self.obj(id).controller;
            self.counters_placed(controller, n);
        }
    }

    /// The monarch's end-step draw (CR 724.2): a triggered ability of the active player, if they're
    /// the monarch.
    pub(super) fn monarch_end_step(&mut self) {
        if let Some((q, source)) = self.monarch {
            if q == self.active {
                self.triggered.push(StackItem {
                    kind: StackKind::MonarchDraw,
                    ..Game::casting(source, q)
                });
            }
        }
    }

    /// The monarch passes (CR 724.2): combat damage from `p`'s creatures to the monarch makes `p` the
    /// monarch, as does Forth Eorlingas! this turn (1v1: a player dealt damage is the opponent).
    pub(super) fn fire_monarch(&mut self, p: PlayerId, when: When) {
        if when != When::YourCreaturesDealCombatDamageToPlayer {
            return;
        }
        match (self.monarch, self.monarch_on_combat_damage[p]) {
            (Some((q, s)), _) if q == opponent(p) => self.monarch = Some((p, s)),
            (_, Some((turn, s))) if turn == self.turn => self.monarch = Some((p, s)),
            _ => {}
        }
    }

    /// Mirrorform: each nonland permanent `p` controls becomes a copy of `t` (for good; `t` itself,
    /// if it's one of them, is unchanged).
    pub(super) fn nonland_permanents_become_copy(&mut self, p: PlayerId, t: ObjId) {
        let def = self.obj(t).def;
        let mine: Vec<ObjId> = self
            .permanents(p)
            .filter(|&x| x != t && !self.types_of(x).land)
            .collect();
        for id in mine {
            self.become_copy(id, def);
        }
    }

    /// Sanar's vivid reveal: X is the colors among `p`'s permanents; reveal from the top until X
    /// nonland cards (they stay in the library while `p` chooses).
    pub(super) fn vivid_reveal(&mut self, p: PlayerId) {
        let mut colors = [false; 5];
        for x in self.permanents(p).collect::<Vec<_>>() {
            for (c, has) in self.colors_of(x).into_iter().enumerate() {
                colors[c] |= has;
            }
        }
        let x = colors.iter().filter(|&&b| b).count();
        let mut cards = Vec::new();
        let mut nonland = 0;
        for &c in self.players[p].library.iter().rev() {
            if nonland >= x {
                break;
            }
            cards.push(c);
            if !self.def_of(c).types.land {
                nonland += 1;
            }
        }
        self.vivid_next(p, cards, colors);
    }

    /// The revealed cards `p` may still exile for one of `colors`.
    fn vivid_candidates(&self, cards: &[ObjId], colors: &[bool; 5]) -> Vec<ObjId> {
        cards
            .iter()
            .copied()
            .filter(|&c| self.obj(c).zone == Zone::Library)
            .filter(|&c| self.colors_of(c).iter().zip(colors).any(|(&a, &b)| a && b))
            .collect()
    }

    /// Asks for the next card to exile, or shuffles once there's nothing left to choose.
    pub(super) fn vivid_next(&mut self, p: PlayerId, cards: Vec<ObjId>, colors: [bool; 5]) {
        if self.vivid_candidates(&cards, &colors).is_empty() {
            self.shuffle_library(p);
        } else {
            self.ask(p, Decision::VividExile { cards, colors });
        }
    }

    pub(super) fn vivid_exile_actions(&self, cards: &[ObjId], colors: &[bool; 5]) -> Vec<Action> {
        let mut v: Vec<Action> = self
            .vivid_candidates(cards, colors)
            .into_iter()
            .map(Action::Pick)
            .collect();
        v.push(Action::No);
        v
    }

    pub(super) fn vivid_color_actions(&self, card: ObjId, colors: &[bool; 5]) -> Vec<Action> {
        let has = self.colors_of(card);
        (0..5u8)
            .filter(|&i| has[i as usize] && colors[i as usize])
            .map(Action::Mode)
            .collect()
    }

    /// `Pick(c)`: exile it, castable this turn; it uses up one of its colors (asked if it has more
    /// than one left). `No`: stop and shuffle.
    pub(super) fn vivid_exile_decide(
        &mut self,
        p: PlayerId,
        cards: Vec<ObjId>,
        colors: [bool; 5],
        a: Action,
    ) {
        let Action::Pick(c) = a else {
            self.shuffle_library(p);
            return;
        };
        let has = self.colors_of(c);
        self.move_to(c, Zone::Exile);
        if self.obj(c).zone == Zone::Exile {
            let turn = self.turn;
            self.obj_mut(c).playable = Some((p, turn));
        }
        let left: Vec<usize> = (0..5).filter(|&i| has[i] && colors[i]).collect();
        if left.len() > 1 {
            self.ask(
                p,
                Decision::VividColor {
                    cards,
                    colors,
                    card: c,
                },
            );
        } else {
            let mut colors = colors;
            for i in left {
                colors[i] = false;
            }
            self.vivid_next(p, cards, colors);
        }
    }

    /// Whether the card `c` shares a creature type with a creature `p` controls (changeling on
    /// either side counts as every creature type; `has_subtype` covers the creature's own).
    pub(super) fn shares_creature_type_with_yours(&self, p: PlayerId, c: ObjId) -> bool {
        let d = self.def_of(c);
        self.creatures(p).any(|x| {
            if d.changeling {
                self.def_of(x).changeling || !self.def_of(x).subtypes.is_empty()
            } else {
                d.subtypes.iter().any(|s| self.has_subtype(x, s))
            }
        })
    }

    /// Descendants' Path's upkeep: the top card, castable free if it's a creature card sharing a
    /// creature type with one of `p`'s creatures (the window's pass is "you don't cast it"); else, or
    /// if it isn't cast, to the bottom.
    pub(super) fn descendants_path(&mut self, p: PlayerId) {
        let Some(&c) = self.players[p].library.last() else {
            return;
        };
        if self.def_of(c).types.creature && self.shares_creature_type_with_yours(p, c) {
            self.obj_mut(c).cast_free = true;
            self.free_casts.push(c);
            self.free_caster = p;
            self.free_casts_one = true;
            self.free_casts_from_library = true;
        } else {
            self.put_on_bottom_of_library(c);
        }
    }

    /// Moves the library card `c` to the bottom of its owner's library.
    fn put_on_bottom_of_library(&mut self, c: ObjId) {
        let owner = self.obj(c).owner;
        let lib = &mut self.players[owner].library;
        if let Some(i) = lib.iter().position(|&x| x == c) {
            lib.remove(i);
            lib.insert(0, c); // index 0 is the bottom
        }
    }

    /// As a free-cast window closes: a library card it offered and that wasn't cast goes to the
    /// bottom (Descendants' Path).
    pub(super) fn library_free_casts_to_bottom(&mut self) {
        if std::mem::take(&mut self.free_casts_from_library) {
            let left: Vec<ObjId> = self
                .free_casts
                .iter()
                .copied()
                .filter(|&c| self.obj(c).zone == Zone::Library)
                .collect();
            for c in left {
                self.put_on_bottom_of_library(c);
            }
        }
    }
}
