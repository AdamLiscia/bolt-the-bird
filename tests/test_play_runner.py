"""P3b: the batched game runner."""

import random

import pytest
import torch

be = pytest.importorskip("bolt_engine")

from bolt.play_net import PlayNet  # noqa: E402
from bolt.play_runner import run_games, run_parallel  # noqa: E402


def deck(seed):
    rng = random.Random(seed)
    names = be.card_names()
    spells = [be.index_of(n) for n in be.supported() if "Land" not in names[be.index_of(n)]]
    return [be.index_of("Plains")] * 9 + [be.index_of("Mountain")] * 8 + rng.sample(spells, 23)


def jobs(n):
    return [(deck(2 * i), deck(2 * i + 1), 100 + i, i % 2) for i in range(n)]


def net():
    torch.manual_seed(0)
    return PlayNet(len(be.card_names()), dim=32, layers=2, heads=4).eval()


def test_every_game_finishes_with_legal_moves_and_a_full_record():
    out = run_games(net(), jobs(6), seats=("net", "rules"), sample=True, seed=1)
    assert len(out["results"]) == 6
    for r in out["results"]:
        assert r["result"] in ("p0", "p1", "draw", "timeout")
    steps = out["steps"]
    assert steps, "the network made decisions"
    for s in steps:
        assert s["seat"] == 0, "only the network's seat is recorded"
        assert 0 <= s["action"] < len(s["enc"]["act_kind"]) and len(s["enc"]["act_kind"]) > 1
        assert s["logp"] <= 0 and -1 <= s["value"] <= 1
    assert {s["game"] for s in steps} == set(range(6))


def test_greedy_play_is_reproducible():
    a = run_games(net(), jobs(4), seats=("net", "net"), sample=False, seed=3)
    b = run_games(net(), jobs(4), seats=("net", "net"), sample=False, seed=3)
    assert a["results"] == b["results"]
    assert [s["action"] for s in a["steps"]] == [s["action"] for s in b["steps"]]


def test_rules_vs_rules_matches_the_engine():
    js = jobs(5)
    out = run_games(None, js, seats=("rules", "rules"))
    whole = be.play_games(js, pilot="baseline", threads=1)
    assert [(r["result"], r["turn"]) for r in out["results"]] == [tuple(w) for w in whole]


def test_the_smart_seat_plays_the_engines_smart_games():
    js = jobs(3)
    out = run_games(None, js, seats=("smart", "smart"), smart=(1, 2))
    whole = be.play_games(js, pilot="smart", rollouts=1, candidates=2, threads=1)
    assert [(r["result"], r["turn"]) for r in out["results"]] == [tuple(w) for w in whole]


def test_record_teacher_keeps_the_smart_players_choices_as_labels():
    out = run_games(None, jobs(2), seats=("smart", "rules"), smart=(1, 2), record=(0,))
    assert out["steps"] and all(s["seat"] == 0 and 0 <= s["action"] < len(s["enc"]["act_kind"]) for s in out["steps"])


def test_parallel_workers_give_the_same_games():
    js = jobs(8)
    one = run_games(None, js, seats=("rules", "rules"))
    many = run_parallel(None, js, seats=("rules", "rules"), workers=2)
    assert one["results"] == many["results"]


def test_compact_workers_return_the_same_arrays_as_compacting_afterwards():
    import numpy as np

    from bolt.play_data import to_arrays

    js = jobs(6)
    full = run_games(None, js, seats=("rules", "rules"), record=(0, 1))
    arr = run_parallel(None, js, seats=("rules", "rules"), record=(0, 1), workers=2, chunk=2, compact=True)
    want = to_arrays(full["steps"], full["results"])
    assert arr["results"] == full["results"]
    for k, v in want.items():
        assert np.array_equal(arr["arrays"][k], v), k


def test_a_game_that_never_ends_its_turn_stops_at_the_move_cap():
    out = run_games(None, jobs(2), seats=("rules", "rules"), max_moves=30)
    assert [r["result"] for r in out["results"]] == ["timeout", "timeout"]


def test_labelled_games_follow_the_network_but_record_the_smart_players_choice():
    js = jobs(3)
    plain = run_games(net(), js, seats=("net", "rules"), sample=False)
    lab = run_games(net(), js, seats=("net", "rules"), sample=False, label_with=(1, 2))
    assert lab["results"] == plain["results"], "the network's moves are still the ones played"
    assert [s["played"] for s in lab["steps"]] == [s["action"] for s in plain["steps"]]
    assert any(s["action"] != s["played"] for s in lab["steps"]), "labels come from the smart player"
    # replay game 0 and check each label is what the smart player picks in that exact position
    a, b, seed, first = js[0]
    g = be.Game(a, b, seed=seed, first=first)
    own = iter([s for s in lab["steps"] if s["game"] == 0])
    moves = 0
    while not g.is_over() and g.turn <= 200:
        who = g.to_act()
        legal = g.legal_actions()
        if who == 0 and len(legal) > 1:
            s = next(own)
            assert s["action"] == g.smart(1, 2, (seed * 1_000_003 + moves) % 2**64)
            i = s["played"]
        else:
            i = 0 if len(legal) == 1 else g.baseline()
        g.step(i)
        moves += 1
