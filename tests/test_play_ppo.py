"""P3d: self-play PPO for the play network."""

import random

import numpy as np
import pytest
import torch

be = pytest.importorskip("bolt_engine")

from bolt.play_data import batch, to_arrays  # noqa: E402
from bolt.play_net import PlayNet  # noqa: E402
from bolt.play_ppo import league, ppo_step  # noqa: E402
from bolt.play_runner import run_games  # noqa: E402


def deck(seed):
    rng = random.Random(seed)
    names = be.card_names()
    spells = [be.index_of(n) for n in be.supported() if "Land" not in names[be.index_of(n)]]
    return [be.index_of("Plains")] * 9 + [be.index_of("Mountain")] * 8 + rng.sample(spells, 23)


def jobs(n):
    return [(deck(2 * i), deck(2 * i + 1), 500 + i, i % 2) for i in range(n)]


def net(seed=0):
    torch.manual_seed(seed)
    return PlayNet(len(be.card_names()), dim=32, layers=2, heads=4).eval()


def test_records_keep_the_moves_probability_and_the_value_estimate():
    out = run_games(net(), jobs(2), seats=("net", "rules"), sample=True, seed=2)
    arr = to_arrays(out["steps"], out["results"])
    assert np.allclose(arr["logp"], [s["logp"] for s in out["steps"]], atol=1e-6)
    assert np.allclose(arr["value"], [s["value"] for s in out["steps"]], atol=1e-6)
    teacher = run_games(None, jobs(1), seats=("rules", "rules"), record=(0,))
    assert np.isnan(to_arrays(teacher["steps"], teacher["results"])["logp"]).all()


def test_an_opponent_network_plays_but_is_not_recorded():
    a = run_games(net(), jobs(3), seats=("net", "net"), sample=False, record=(0,))
    b = run_games(net(), jobs(3), seats=("net", "opp"), sample=False, opp_net=net())
    assert a["results"] == b["results"], "the same weights in the opponent seat play the same games"
    assert all(s["seat"] == 0 for s in b["steps"]) and len(a["steps"]) == len(b["steps"])


def test_a_ppo_step_raises_the_probability_of_moves_that_won():
    m = net(1).train()
    out = run_games(m.eval(), jobs(3), seats=("net", "rules"), sample=True, seed=4)
    arr = to_arrays(out["steps"], out["results"])
    idx = np.arange(len(arr["action"]))
    x, action, _ = batch(arr, idx)
    adv = torch.ones(len(idx))  # every move "won"
    m.train()
    with torch.no_grad():
        before = torch.log_softmax(m(x)[0], 1).gather(1, action[:, None]).mean().item()
    opt = torch.optim.Adam(m.parameters(), lr=1e-3)
    stats = ppo_step(m, None, opt, arr, idx, adv, clip=0.2, kl_coef=0.0, value_coef=0.5)
    with torch.no_grad():
        after = torch.log_softmax(m(x)[0], 1).gather(1, action[:, None]).mean().item()
    assert after > before and np.isfinite(stats["loss"])


def test_the_league_mixes_fixed_players_past_selves_and_self_play():
    first = league(n_games=100, past=[])
    assert {s for s, _ in first} <= {("net", "rules"), ("rules", "net"), ("net", "smart"), ("smart", "net"),
                                     ("net", "net")}  # fmt: skip
    assert sum(n for _, n in first) == 100
    later = league(n_games=100, past=["step_1.pt", "step_2.pt"])
    assert any("opp" in s for s, _ in later) and sum(n for _, n in later) == 100
