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


def fake(values, z, games, seats):
    return {"value": np.asarray(values, np.float32), "z": np.asarray(z, np.int8),
            "game": np.asarray(games, np.int32), "seat": np.asarray(seats, np.int8)}  # fmt: skip


def test_gae_with_lambda_one_is_the_result_minus_the_value():
    from bolt.play_ppo import gae

    a = fake([0.1, -0.2, 0.5, 0.3, 0.0], [1, 1, 1, -1, -1], [0, 0, 0, 0, 0], [0, 0, 0, 1, 1])
    adv, ret = gae(a, lam=1.0)
    assert np.allclose(adv, a["z"] - a["value"]) and np.allclose(ret, a["z"])


def test_gae_with_lambda_zero_credits_each_move_with_the_change_it_made():
    from bolt.play_ppo import gae

    a = fake([0.1, -0.2, 0.5], [1, 1, 1], [3, 3, 3], [0, 0, 0])
    adv, ret = gae(a, lam=0.0)
    assert np.allclose(adv, [-0.2 - 0.1, 0.5 + 0.2, 1 - 0.5]) and np.allclose(ret, adv + a["value"])


def test_gae_keeps_games_and_seats_apart_even_when_their_steps_interleave():
    from bolt.play_ppo import gae

    # game 0 seat 0, game 1 seat 0 and game 0 seat 1, interleaved in recording order
    a = fake([0.2, 0.0, 0.4, -0.5, 0.6, 0.1], [1, -1, 1, -1, 1, -1], [0, 1, 0, 1, 0, 0], [0, 0, 0, 0, 0, 1])
    adv, _ = gae(a, lam=0.5)
    one = fake([0.2, 0.4, 0.6], [1, 1, 1], [0, 0, 0], [0, 0, 0])
    two = fake([0.0, -0.5], [-1, -1], [1, 1], [0, 0])
    three = fake([0.1], [-1], [0], [1])
    assert np.allclose(adv[[0, 2, 4]], gae(one, lam=0.5)[0])
    assert np.allclose(adv[[1, 3]], gae(two, lam=0.5)[0])
    assert np.allclose(adv[[5]], gae(three, lam=0.5)[0])


def test_the_value_head_regresses_to_the_given_returns():
    m = net(2).train()
    out = run_games(m.eval(), jobs(2), seats=("net", "rules"), sample=True, seed=5)
    arr = to_arrays(out["steps"], out["results"])
    idx = np.arange(len(arr["action"]))
    x, _, _ = batch(arr, idx)
    target = torch.full((len(idx),), 0.7)
    opt = torch.optim.Adam(m.parameters(), lr=3e-3)
    m.train()
    with torch.no_grad():
        before = ((m(x)[1] - target) ** 2).mean().item()
    for _ in range(5):
        ppo_step(m, None, opt, arr, idx, torch.zeros(len(idx)), clip=0.2, kl_coef=0.0, value_coef=1.0, ret=target)
    with torch.no_grad():
        after = ((m(x)[1] - target) ** 2).mean().item()
    assert after < before


def test_value_mse_z_measures_the_value_against_the_result_whatever_the_target():
    import numpy as np
    import torch
    from test_play_commander import obs

    from bolt.play_data import to_arrays
    from bolt.play_encoding import encode
    from bolt.play_net import PlayNet
    from bolt.play_ppo import ppo_step

    steps = [{"game": 0, "seat": 0, "enc": encode(obs(), 0), "action": 1, "logp": -0.7, "value": 0.0}] * 4
    arr = to_arrays(steps, [{"result": "p0"}])  # z = +1
    net = PlayNet(n_cards=8, dim=16, layers=1, heads=2)
    idx = np.arange(4)
    with torch.no_grad():
        v = net(__import__("bolt.play_data", fromlist=["batch"]).batch(arr, idx)[0])[1]
    opt = torch.optim.SGD(net.parameters(), lr=0.0)
    ret = torch.full((4,), float(v[0]))  # a target equal to V: value_mse 0, but V is still far from z
    st = ppo_step(net, None, opt, arr, idx, torch.zeros(4), ret=ret)
    assert st["value_mse"] < 1e-8
    assert abs(st["value_mse_z"] - float(((v - 1) ** 2).mean())) < 1e-5


def test_a_uniform_policy_has_normalized_entropy_one_whatever_its_move_count():
    import math

    import numpy as np
    import torch
    from test_play_commander import obs

    from bolt.play_data import to_arrays
    from bolt.play_encoding import encode
    from bolt.play_net import PlayNet
    from bolt.play_ppo import ppo_step

    three = obs(legal_actions=[("PassPriority", -1, -1, -1)] * 3, legal_cards=[(-1, -1, -1)] * 3)
    two = obs()  # Priority, two moves
    steps = [{"game": 0, "seat": 0, "enc": encode(o, 0), "action": 0, "logp": -1.0, "value": 0.0}
             for o in (two, two, three, three)]  # fmt: skip
    arr = to_arrays(steps, [{"result": "p0"}])
    net = PlayNet(n_cards=8, dim=16, layers=1, heads=2)
    torch.nn.init.zeros_(net.policy[2].weight)
    torch.nn.init.zeros_(net.policy[2].bias)  # every move scores 0: uniform
    st = ppo_step(net, None, torch.optim.SGD(net.parameters(), lr=0.0), arr, np.arange(4), torch.zeros(4))
    assert abs(st["entropy"] - (math.log(2) + math.log(3)) / 2) < 1e-5, "raw entropy grows with the move count"
    assert abs(st["entropy_norm"] - 1) < 1e-5, "normalized: 1 for any uniform policy"
    k = st["by_kind"]["Priority"]
    assert k[0] == 4 and abs(k[2] / k[0] - 1) < 1e-5
