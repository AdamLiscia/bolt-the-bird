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
}

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
        self.g
            .apply(a)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
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
    m.add_function(wrap_pyfunction!(index_of, m)?)?;
    m.add_function(wrap_pyfunction!(supported, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
