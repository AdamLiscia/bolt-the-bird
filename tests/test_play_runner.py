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
