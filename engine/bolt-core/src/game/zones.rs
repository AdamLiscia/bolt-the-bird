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
}
