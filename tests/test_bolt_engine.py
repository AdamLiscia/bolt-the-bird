"""The Rust engine's Python bindings (engine/bolt-py).

Skipped unless `bolt_engine` is built: `cd engine/bolt-py && maturin develop --release`.
"""

import random

import pytest

be = pytest.importorskip("bolt_engine")

BASICS = ["Plains", "Island", "Swamp", "Mountain", "Forest"]


def deck(rng: random.Random) -> list[int]:
    """Two colors' basics and 23 random supported nonland cards."""
    lands = [be.index_of(n) for n in rng.sample(BASICS, 2)]
    names = be.card_names()
    spells = [be.index_of(n) for n in be.supported() if n not in BASICS]
    spells = [c for c in spells if "Land" not in names[c]]
    return [lands[i % 2] for i in range(17)] + [rng.choice(spells) for _ in range(23)]


def play_random(g, rng: random.Random, cap: int = 20_000) -> int:
    for n in range(cap):
        if g.is_over():
            return n
        g.step(rng.randrange(len(g.legal_actions())))
    raise AssertionError("game did not end")


def test_every_fdn_card_is_supported():
    assert len(be.supported()) == 286
    assert be.card_names()[be.index_of("Savannah Lions")] == "Savannah Lions"
    with pytest.raises(ValueError):
        be.index_of("Black Lotus")


def test_unsupported_or_unknown_cards_are_rejected():
    with pytest.raises(ValueError):
        be.Game([10_000] * 40, [0] * 40, seed=1)


def test_random_games_finish_with_a_result():
    rng = random.Random(7)
    for seed in range(20):
        g = be.Game(deck(rng), deck(rng), seed=seed, first=seed % 2)
        play_random(g, rng)
        assert g.result() in {"p0", "p1", "draw"}
        assert g.legal_actions() == []
        assert g.to_act() is None


def test_actions_are_flat_tuples_and_step_checks_the_index():
    g = be.Game(deck(random.Random(1)), deck(random.Random(2)), seed=3)
    kind, a, b, c = g.legal_actions()[0]
    assert isinstance(kind, str) and all(isinstance(x, int) for x in (a, b, c))
    assert {k for k, *_ in g.legal_actions()} <= {"Keep", "Mulligan"}
    with pytest.raises(IndexError):
        g.step(len(g.legal_actions()))


def test_observation_hides_the_opponents_hand():
    g = be.Game(deck(random.Random(1)), deck(random.Random(2)), seed=3)
    o = g.observation(0)
    me, them = o["players"]
    assert len(me["hand"]) == me["hand_size"] == 7
    assert them["hand"] is None and them["hand_size"] == 7
    assert o["players"][1]["life"] == 20
    assert g.observation(1)["players"][1]["hand"] is not None


def test_clones_are_independent_and_games_replay_exactly():
    a, b = deck(random.Random(4)), deck(random.Random(5))
    g = be.Game(a, b, seed=9)
    h = g.clone()
    play_random(g, random.Random(0))
    assert h.turn == 0 and not h.is_over()
    play_random(h, random.Random(0))
    assert (g.result(), g.turn) == (h.result(), h.turn)


def test_the_baseline_player_beats_random():
    rng = random.Random(11)
    wins = 0
    for seed in range(40):
        g = be.Game(deck(rng), deck(rng), seed=seed)
        smart = seed % 2
        while not g.is_over():
            n = len(g.legal_actions())
            g.step(g.baseline() if g.to_act() == smart else rng.randrange(n))
        wins += g.result() == f"p{smart}"
    assert wins >= 28, wins


def test_play_baseline_runs_a_whole_game_in_rust():
    a, b = deck(random.Random(1)), deck(random.Random(2))
    result, turns = be.play_baseline(a, b, seed=5)
    assert result in {"p0", "p1", "draw", "timeout"} and turns > 0
    assert be.play_baseline(a, b, seed=5) == (result, turns), "deterministic"
