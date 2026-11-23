//! Python bindings: `bolt_engine.Game` plays a game of Magic with FDN cards.
//!
//! ```python
//! import bolt_engine as be
//! deck = [be.index_of("Savannah Lions")] * 23 + [be.index_of("Plains")] * 17
//! g = be.Game(deck, deck, seed=1)
//! while not g.is_over():
//!     actions = g.legal_actions()          # [(kind, a, b, c), ...]; -1 means unused
//!     g.step(0)                            # apply actions[0]
//! print(g.result())                        # "p0", "p1" or "draw"
//! ```
//!
//! Cards are numbered by their index in `card_names()` (FDN in model order, then tokens).
//! `observation(player)` shows what that player can see: their own hand, not the opponent's.

use bolt_core::{Action, Game as Core, ObjId, Outcome, Pending};
use pyo3::exceptions::{PyIndexError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

/// An action as `(kind, a, b, c)`, with -1 for unused slots. Object ids and card numbers are
/// plain integers.
pub type Encoded = (&'static str, i64, i64, i64);

fn id(o: ObjId) -> i64 {
    o.0 as i64
}

/// Encodes an action for Python (and for anything else that wants flat tuples).
pub fn encode(a: &Action) -> Encoded {
    use Action::*;
    match *a {
        Keep => ("Keep", -1, -1, -1),
        Mulligan => ("Mulligan", -1, -1, -1),
        Bottom(c) => ("Bottom", id(c), -1, -1),
        PassPriority => ("PassPriority", -1, -1, -1),
        PlayLand(c) => ("PlayLand", id(c), -1, -1),
        Cast(c) => ("Cast", id(c), -1, -1),
        Activate(c, n) => ("Activate", id(c), n as i64, -1),
        ActivateWith(c, n, v) => ("ActivateWith", id(c), n as i64, id(v)),
        CastKicked(c) => ("CastKicked", id(c), -1, -1),
        CastWith(c, x) => ("CastWith", id(c), id(x), -1),
        CastFlashback(c) => ("CastFlashback", id(c), -1, -1),
        CastModal(c, m) => ("CastModal", id(c), m as i64, -1),
        CastAlternative(c) => ("CastAlternative", id(c), -1, -1),
        CastX(c, x) => ("CastX", id(c), x as i64, -1),
        AttackPlaneswalker(c, w) => ("AttackPlaneswalker", id(c), id(w), -1),
        Loyalty(c, n) => ("Loyalty", id(c), n as i64, -1),
        ActivateX { id: c, n, victim, x } => (
            "ActivateX",
            id(c),
            n as i64 * 256 + x as i64,
            victim.map_or(-1, id),
        ),
        Target(bolt_core::Target::Player(p)) => ("TargetPlayer", p as i64, -1, -1),
        Target(bolt_core::Target::Object(c)) => ("Target", id(c), -1, -1),
        Yes => ("Yes", -1, -1, -1),
        No => ("No", -1, -1, -1),
        Pick(c) => ("Pick", id(c), -1, -1),
        Mode(m) => ("Mode", m as i64, -1, -1),
        Attack(c) => ("Attack", id(c), -1, -1),
        FinishAttacks => ("FinishAttacks", -1, -1, -1),
        Block { blocker, attacker } => ("Block", id(blocker), id(attacker), -1),
        ResetBlocks => ("ResetBlocks", -1, -1, -1),
        FinishBlocks => ("FinishBlocks", -1, -1, -1),
        Discard(c) => ("Discard", id(c), -1, -1),
        Amount(n) => ("Amount", n as i64, -1, -1),
        CastFromGraveyard(c) => ("CastFromGraveyard", id(c), -1, -1),
        CastFromGraveyardAs(c, t) => ("CastFromGraveyardAs", id(c), t as i64, -1),
    }
}

/// The objects an action names, in the slots `encode` puts them in (a, b, c); `None` where that
/// slot is a number or unused.
pub fn objects(a: &Action) -> [Option<ObjId>; 3] {
    use Action::*;
    match *a {
        Bottom(c)
        | PlayLand(c)
        | Cast(c)
        | Activate(c, _)
        | CastKicked(c)
        | CastFlashback(c)
        | CastModal(c, _)
        | CastAlternative(c)
        | CastX(c, _)
        | Loyalty(c, _)
        | Pick(c)
        | Attack(c)
        | Discard(c)
        | CastFromGraveyard(c)
        | CastFromGraveyardAs(c, _) => [Some(c), None, None],
        Target(bolt_core::Target::Object(c)) => [Some(c), None, None],
        ActivateWith(c, _, v) => [Some(c), None, Some(v)],
        CastWith(c, x) => [Some(c), Some(x), None],
        AttackPlaneswalker(c, w) => [Some(c), Some(w), None],
        ActivateX { id: c, victim, .. } => [Some(c), None, victim],
        Block { blocker, attacker } => [Some(blocker), Some(attacker), None],
        _ => [None, None, None],
    }
}

/// A mana cost in Scryfall notation, e.g. `{2}{W}`.
fn cost_text(c: &bolt_core::ManaCost) -> String {
    let mut out = String::new();
    if c.x > 0 {
        out.push_str(&"{X}".repeat(c.x as usize));
    }
    if c.generic > 0 || (c.colored.iter().all(|&n| n == 0) && c.hybrid.iter().all(|&n| n == 0)) {
        out.push_str(&format!("{{{}}}", c.generic));
    }
    for (i, sym) in ["W", "U", "B", "R", "G"].iter().enumerate() {
        out.push_str(&format!("{{{sym}}}").repeat(c.colored[i] as usize));
    }
    out
}

/// What the pending choice is and who makes it.
pub fn pending(p: &Pending) -> (&'static str, i64) {
    use Pending::*;
    match *p {
        Mulligan(q) => ("Mulligan", q as i64),
        Bottom(q, _) => ("Bottom", q as i64),
        Priority(q) => ("Priority", q as i64),
        DeclareAttackers(q) => ("DeclareAttackers", q as i64),
        DeclareBlockers(q) => ("DeclareBlockers", q as i64),
        Discard(q) => ("Discard", q as i64),
        ChooseTarget(q) => ("ChooseTarget", q as i64),
        Decide(q) => ("Decide", q as i64),
        Divide(q) => ("Divide", q as i64),
        RemoveCounters(q) => ("RemoveCounters", q as i64),
        FreeCast(q) => ("FreeCast", q as i64),
        GameOver => ("GameOver", -1),
    }
}

/// A game in progress.
#[pyclass(module = "bolt_engine")]
#[derive(Clone)]
pub struct Game {
    g: Core,
    legal: Vec<Action>,
    /// Every event so far, collected from each move's `apply` (the core hands them back and forgets
    /// them). Capped so a runaway game can't grow it without bound.
    log: Vec<bolt_core::Event>,
}

const MAX_LOG: usize = 200_000;

impl Game {
    fn refresh(&mut self) {
        self.legal = if self.g.is_over() {
            Vec::new()
        } else {
            self.g.legal_actions()
        };
    }
}

#[pymethods]
impl Game {
    /// A new game between two decks of card numbers, shuffled by `seed`; `first` plays first.
    #[new]
    #[pyo3(signature = (deck_a, deck_b, seed, first = 0))]
    fn new(deck_a: Vec<usize>, deck_b: Vec<usize>, seed: u64, first: usize) -> PyResult<Self> {
        let defs = bolt_cards::fdn();
        let mask = bolt_cards::supported_mask();
        for &c in deck_a.iter().chain(&deck_b) {
            if c >= mask.len() || !mask[c] {
                return Err(PyValueError::new_err(format!(
                    "card {c} is not a supported FDN card"
                )));
            }
        }
        if first > 1 {
            return Err(PyValueError::new_err("first must be 0 or 1"));
        }
        let mut g = Game {
            g: Core::new(defs, [&deck_a, &deck_b], seed, first),
            legal: Vec::new(),
            log: Vec::new(),
        };
        g.refresh();
        Ok(g)
    }

    /// The legal actions now, as `(kind, a, b, c)` tuples. `step(i)` applies the i-th.
    fn legal_actions(&self) -> Vec<Encoded> {
        self.legal.iter().map(encode).collect()
    }

    /// Applies `legal_actions()[i]`. Returns whether the game is over.
    fn step(&mut self, i: usize) -> PyResult<bool> {
        let a = *self
            .legal
            .get(i)
            .ok_or_else(|| PyIndexError::new_err(format!("{i} is not a legal action index")))?;
        let ev = self
            .g
            .apply(a)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        if self.log.len() < MAX_LOG {
            self.log.extend(ev);
        }
        self.refresh();
        Ok(self.g.is_over())
    }

    /// The index in `legal_actions()` the rules-based baseline player would choose.
    fn baseline(&self) -> PyResult<usize> {
        if self.legal.is_empty() {
            return Err(PyValueError::new_err("the game is over"));
        }
        let a = bolt_core::baseline::choose(&self.g, &self.legal);
        Ok(self
            .legal
            .iter()
            .position(|&x| x == a)
            .expect("the baseline picks a legal action"))
    }

    /// The index of the move the smart (rollout) player would make here: `candidates` moves, each
    /// played out `rollouts` times in guessed worlds, searched with `seed`. With the seed the
    /// engine's own smart games use (game seed × 1,000,003 + move number) it plays the same game.
    #[pyo3(signature = (rollouts = 1, candidates = 2, seed = 0))]
    fn smart(&self, rollouts: u32, candidates: usize, seed: u64) -> PyResult<usize> {
        if self.legal.is_empty() {
            return Err(PyValueError::new_err("the game is over"));
        }
        let cfg = bolt_core::rollout::Config {
            rollouts,
            max_candidates: candidates,
            ..Default::default()
        };
        let a = bolt_core::rollout::choose(&self.g, &self.legal, &cfg, seed);
        Ok(self
            .legal
            .iter()
            .position(|&x| x == a)
            .expect("the smart player picks a legal move"))
    }

    /// Who must act now (0 or 1), or None when the game is over.
    fn to_act(&self) -> Option<i64> {
        let (_, p) = pending(&self.g.pending);
        (p >= 0).then_some(p)
    }

    /// The kind of choice pending, e.g. "Priority" or "DeclareBlockers".
    fn pending(&self) -> &'static str {
        pending(&self.g.pending).0
    }

    fn is_over(&self) -> bool {
        self.g.is_over()
    }

    /// "p0", "p1", "draw", or None while the game goes on.
    fn result(&self) -> Option<&'static str> {
        self.g.result.map(|o| match o {
            Outcome::Winner(0) => "p0",
            Outcome::Winner(_) => "p1",
            Outcome::Draw => "draw",
        })
    }

    #[getter]
    fn turn(&self) -> u32 {
        self.g.turn
    }

    /// Whether priority stops nobody would act at are skipped (the default).
    #[getter]
    fn get_auto_pass(&self) -> bool {
        self.g.auto_pass
    }

    #[setter]
    fn set_auto_pass(&mut self, on: bool) {
        self.g.auto_pass = on;
        self.refresh();
    }

    /// An independent copy, for search.
    fn clone(&self) -> Self {
        Clone::clone(self)
    }

    fn __copy__(&self) -> Self {
        Clone::clone(self)
    }

    /// What `player` can see: everything public, plus their own hand.
    /// The game's events from index `since` on, as dicts: `kind` plus its fields. Objects are
    /// given as `id` (stable within the game) and `card` (the card number); zones by name.
    #[pyo3(signature = (since = 0))]
    fn events<'py>(&self, py: Python<'py>, since: usize) -> PyResult<Vec<Bound<'py, PyDict>>> {
        use bolt_core::Event::*;
        let g = &self.g;
        let zone = |z: &bolt_core::Zone| format!("{z:?}");
        let mut out = Vec::new();
        for e in self.log.iter().skip(since) {
            let d = PyDict::new(py);
            let obj = |d: &Bound<'py, PyDict>, key: &str, oid: bolt_core::ObjId| -> PyResult<()> {
                d.set_item(key, id(oid))?;
                let o = g.obj(oid);
                let card_key = if key == "id" {
                    "card".to_string()
                } else {
                    format!("{key}_card")
                };
                d.set_item(card_key, o.def as i64)?;
                if key == "id" {
                    d.set_item("owner", o.owner as i64)?;
                }
                Ok(())
            };
            match e {
                TurnBegan { turn, active } => {
                    d.set_item("kind", "TurnBegan")?;
                    d.set_item("turn", *turn)?;
                    d.set_item("player", *active as i64)?;
                }
                Drew { player, card } => {
                    d.set_item("kind", "Drew")?;
                    d.set_item("player", *player as i64)?;
                    obj(&d, "id", *card)?;
                }
                Moved { card, from, to } => {
                    d.set_item("kind", "Moved")?;
                    obj(&d, "id", *card)?;
                    d.set_item("from", zone(from))?;
                    d.set_item("to", zone(to))?;
                }
                Cast { card } => {
                    d.set_item("kind", "Cast")?;
                    obj(&d, "id", *card)?;
                }
                Resolved { card } => {
                    d.set_item("kind", "Resolved")?;
                    obj(&d, "id", *card)?;
                }
                Fizzled { source } => {
                    d.set_item("kind", "Fizzled")?;
                    obj(&d, "id", *source)?;
                }
                Triggered { source } => {
                    d.set_item("kind", "Triggered")?;
                    obj(&d, "id", *source)?;
                }
                Damage {
                    source,
                    target,
                    amount,
                } => {
                    d.set_item("kind", "Damage")?;
                    obj(&d, "id", *source)?;
                    match target {
                        bolt_core::Target::Player(p) => d.set_item("target_player", *p as i64)?,
                        bolt_core::Target::Object(t) => obj(&d, "target", *t)?,
                    }
                    d.set_item("amount", *amount)?;
                }
                LifeGained { player, amount } => {
                    d.set_item("kind", "LifeGained")?;
                    d.set_item("player", *player as i64)?;
                    d.set_item("amount", *amount)?;
                }
                LifeLost { player, amount } => {
                    d.set_item("kind", "LifeLost")?;
                    d.set_item("player", *player as i64)?;
                    d.set_item("amount", *amount)?;
                }
                Lost { player } => {
                    d.set_item("kind", "Lost")?;
                    d.set_item("player", *player as i64)?;
                }
            }
            out.push(d);
        }
        Ok(out)
    }

    fn observation<'py>(&self, py: Python<'py>, player: usize) -> PyResult<Bound<'py, PyDict>> {
        if player > 1 {
            return Err(PyValueError::new_err("player must be 0 or 1"));
        }
        let g = &self.g;
        let card = |c: ObjId| (id(c), g.obj(c).def as i64);
        let d = PyDict::new(py);
        d.set_item("turn", g.turn)?;
        d.set_item("step", format!("{:?}", g.step))?;
        d.set_item("active", g.active)?;
        let (kind, who) = pending(&g.pending);
        d.set_item("pending", (kind, who))?;
        // What a "Decide" is about, for a UI: its kind (May, Surveil, Tuck, ChooseColor, MayPay, ...),
        // the spell or ability asking (`source`), and the cards, amounts or costs it involves. Cards
        // the decider is looking at (surveil, a tucked card, a look-and-pick) are only in the
        // decider's own view. Mode(i) for ChooseColor is a WUBRG index.
        match (&g.pending, &g.decision) {
            (Pending::Decide(q), Some(dec)) => {
                use bolt_core::Decision as D;
                let e = PyDict::new(py);
                let mine = *q == player;
                let dbg = format!("{dec:?}");
                e.set_item("kind", dbg.split([' ', '{', '(']).next().unwrap_or(""))?;
                if let Some(r) = &g.resolving {
                    e.set_item("source", (id(r.item.source), g.obj(r.item.source).def as i64))?;
                }
                let objs = |v: &[ObjId]| -> Vec<(i64, i64)> {
                    v.iter().map(|&c| (id(c), g.obj(c).def as i64)).collect()
                };
                let one = |c: ObjId| -> (i64, i64) { (id(c), g.obj(c).def as i64) };
                match dec {
                    D::ChooseColor { card } => e.set_item("card", one(*card))?,
                    D::ChooseCreatureType { card, options } => {
                        e.set_item("card", one(*card))?;
                        e.set_item("options", options.clone())?;
                    }
                    D::Surveil { cards } if mine => e.set_item("cards", objs(cards))?,
                    D::Tuck { card } if mine => e.set_item("card", one(*card))?,
                    D::LookPick { cards, .. } if mine => e.set_item("cards", objs(cards))?,
                    D::GenesisPick { cards, x } => {
                        e.set_item("cards", objs(cards))?;
                        e.set_item("x", *x)?;
                    }
                    D::SplitPiles { rest, a, b, .. } => {
                        if let Some(&c) = rest.first() {
                            e.set_item("card", one(c))?;
                        }
                        e.set_item("a", objs(a))?;
                        e.set_item("b", objs(b))?;
                    }
                    D::ChoosePile { a, b, .. } => {
                        e.set_item("a", objs(a))?;
                        e.set_item("b", objs(b))?;
                    }
                    D::MayPay { cost } => e.set_item("cost", cost_text(cost))?,
                    D::PayWard { cost, against } => {
                        e.set_item("cost", cost_text(cost))?;
                        e.set_item("card", one(*against))?;
                    }
                    D::PayWardLife { life, against } => {
                        e.set_item("life", *life)?;
                        e.set_item("card", one(*against))?;
                    }
                    D::LoseOrDiscard { n, .. } => e.set_item("life", *n)?,
                    D::Discard { left, optional, .. } => {
                        e.set_item("left", *left)?;
                        e.set_item("optional", *optional)?;
                    }
                    D::SacrificeAnother { source } => e.set_item("card", one(*source))?,
                    D::SacrificeCreatures { left } => e.set_item("left", *left)?,
                    D::Search { what, .. } => e.set_item("what", format!("{what:?}"))?,
                    _ => {}
                }
                d.set_item("decision", e)?;
            }
            _ => d.set_item("decision", py.None())?,
        }
        d.set_item("result", self.result())?;
        let players = PyList::empty(py);
        for (p, pl) in g.players.iter().enumerate() {
            let e = PyDict::new(py);
            e.set_item("life", pl.life)?;
            e.set_item("library", pl.library.len())?;
            e.set_item("hand_size", pl.hand.len())?;
            let hand: Option<Vec<(i64, i64)>> =
                (p == player).then(|| pl.hand.iter().map(|&c| card(c)).collect());
            e.set_item("hand", hand)?;
            let grave: Vec<(i64, i64)> = pl.graveyard.iter().map(|&c| card(c)).collect();
            e.set_item("graveyard", grave)?;
            e.set_item("lands_played", pl.lands_played)?;
            e.set_item("pool", pl.pool.len())?;
            players.append(e)?;
        }
        d.set_item("players", players)?;
        let field = PyList::empty(py);
        for &c in &g.battlefield {
            let o = g.obj(c);
            let e = PyDict::new(py);
            e.set_item("id", id(c))?;
            e.set_item("card", o.def)?;
            e.set_item("controller", o.controller)?;
            e.set_item("owner", o.owner)?;
            e.set_item("tapped", o.tapped)?;
            let t = g.types_of(c);
            e.set_item("creature", t.creature)?;
            e.set_item("land", t.land)?;
            if t.creature {
                e.set_item("power", g.power(c))?;
                e.set_item("toughness", g.toughness(c))?;
            }
            e.set_item("damage", o.damage)?;
            e.set_item("counters", o.counters)?;
            e.set_item("loyalty", o.loyalty)?;
            e.set_item("attached_to", o.attached_to.map(|(a, _)| id(a)))?;
            e.set_item("entered_turn", o.since_turn)?;
            field.append(e)?;
        }
        d.set_item("battlefield", field)?;
        let exile: Vec<(i64, i64)> = g.exile.iter().map(|&c| card(c)).collect();
        d.set_item("exile", exile)?;
        let stack: Vec<(i64, i64, usize)> = g
            .items
            .iter()
            .map(|i| (id(i.source), g.obj(i.source).def as i64, i.controller))
            .collect();
        d.set_item("stack", stack)?;
        d.set_item("legal_actions", self.legal_actions())?;
        // The card behind each object a move names (-1 where the slot isn't an object), so a move
        // that names a card the player can't otherwise see (a library search) still says which.
        let legal_cards: Vec<(i64, i64, i64)> = self
            .legal
            .iter()
            .map(|a| {
                let o = objects(a).map(|c| c.map_or(-1, |c| g.obj(c).def as i64));
                (o[0], o[1], o[2])
            })
            .collect();
        d.set_item("legal_cards", legal_cards)?;
        match &g.combat {
            Some(c) => {
                let e = PyDict::new(py);
                e.set_item(
                    "attackers",
                    c.attackers.iter().map(|&a| id(a)).collect::<Vec<_>>(),
                )?;
                let blocks: Vec<(i64, i64)> = c.blocks.iter().map(|&(b, a)| (id(b), id(a))).collect();
                e.set_item("blocks", blocks)?;
                d.set_item("combat", e)?;
            }
            None => d.set_item("combat", py.None())?,
        }
        Ok(d)
    }

    fn __repr__(&self) -> String {
        format!(
            "Game(turn={}, step={:?}, life={:?}, pending={})",
            self.g.turn,
            self.g.step,
            [self.g.players[0].life, self.g.players[1].life],
            pending(&self.g.pending).0
        )
    }
}

/// A played game stops here as "timeout": some boards can never end (two Herald of Eternal Dawn,
/// "you can't lose the game"), and real games end long before (see docs/engine/QUESTIONS.md).
const MAX_TURNS: u32 = 200;

/// Plays a whole game with the rules-based baseline on both sides, in Rust. Returns the result
/// ("p0", "p1", "draw", or "timeout" past 200 turns) and the number of turns.
#[pyfunction]
#[pyo3(signature = (deck_a, deck_b, seed, first = 0))]
fn play_baseline(
    deck_a: Vec<usize>,
    deck_b: Vec<usize>,
    seed: u64,
    first: usize,
) -> PyResult<(&'static str, u32)> {
    let mut g = Game::new(deck_a, deck_b, seed, first)?;
    for _ in 0..100_000 {
        if g.g.is_over() {
            return Ok((g.result().expect("over"), g.g.turn));
        }
        if g.g.turn > MAX_TURNS {
            return Ok(("timeout", g.g.turn));
        }
        let a = bolt_core::baseline::choose(&g.g, &g.legal);
        g.g.apply(a).map_err(|e| PyValueError::new_err(e.to_string()))?;
        g.refresh();
    }
    Err(PyValueError::new_err(format!(
        "seed {seed}: no result after 100,000 actions"
    )))
}

fn check_decks(a: &[usize], b: &[usize], first: usize) -> PyResult<()> {
    let mask = bolt_cards::supported_mask();
    for &c in a.iter().chain(b) {
        if c >= mask.len() || !mask[c] {
            return Err(PyValueError::new_err(format!(
                "card {c} is not a supported FDN card"
            )));
        }
    }
    if first > 1 {
        return Err(PyValueError::new_err("first must be 0 or 1"));
    }
    Ok(())
}

/// Plays many whole games in Rust, on up to `threads` threads, with Python's lock released.
///
/// `jobs` is a list of `(deck_a, deck_b, seed, first)`; deck_a is player 0. `pilot` is
/// "baseline" (the rules-based player) or "smart" (plays each of `candidates` moves out
/// `rollouts` times before choosing). Returns `(result, turns)` per job, in order, with result
/// "p0", "p1", "draw", "timeout", or "error" if the engine panicked.
#[pyfunction]
#[pyo3(signature = (jobs, pilot = "smart", rollouts = 1, candidates = 2, threads = 8))]
fn play_games(
    py: Python<'_>,
    jobs: Vec<(Vec<usize>, Vec<usize>, u64, usize)>,
    pilot: &str,
    rollouts: u32,
    candidates: usize,
    threads: usize,
) -> PyResult<Vec<(&'static str, u32)>> {
    let pilot = match pilot {
        "baseline" => bolt_cards::play::Pilot::Baseline,
        "smart" => bolt_cards::play::Pilot::Smart(bolt_core::rollout::Config {
            rollouts,
            max_candidates: candidates,
            ..Default::default()
        }),
        p => {
            return Err(PyValueError::new_err(format!(
                "unknown pilot {p:?}: baseline or smart"
            )))
        }
    };
    let mut js = Vec::with_capacity(jobs.len());
    for (a, b, seed, first) in jobs {
        check_decks(&a, &b, first)?;
        js.push(bolt_cards::play::Job { a, b, seed, first });
    }
    let out = py.detach(|| bolt_cards::play::play_many(&js, &pilot, threads));
    Ok(out.into_iter().map(|r| (r.result, r.turns)).collect())
}

/// Every card name, by card number: FDN in model order, then tokens.
#[pyfunction]
fn card_names() -> Vec<String> {
    bolt_cards::fdn().iter().map(|d| d.name.clone()).collect()
}

/// The card number of an FDN card, by exact name.
#[pyfunction]
fn index_of(name: &str) -> PyResult<usize> {
    bolt_cards::index_of(name).ok_or_else(|| PyValueError::new_err(format!("no FDN card named {name:?}")))
}

/// Names of the FDN cards the engine supports (all 286).
#[pyfunction]
fn supported() -> Vec<&'static str> {
    bolt_cards::supported()
}

#[pymodule]
fn bolt_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Game>()?;
    m.add_function(wrap_pyfunction!(card_names, m)?)?;
    m.add_function(wrap_pyfunction!(play_baseline, m)?)?;
    m.add_function(wrap_pyfunction!(play_games, m)?)?;
    m.add_function(wrap_pyfunction!(index_of, m)?)?;
    m.add_function(wrap_pyfunction!(supported, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
