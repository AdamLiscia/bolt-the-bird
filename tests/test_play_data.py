"""P3c: compact storage of recorded decisions, and batches back out of it."""

import random

import numpy as np
import pytest
import torch

be = pytest.importorskip("bolt_engine")

from bolt.play_data import batch, load, save, to_arrays  # noqa: E402
from bolt.play_encoding import collate  # noqa: E402
from bolt.play_runner import run_games  # noqa: E402


def deck(seed):
    rng = random.Random(seed)
    names = be.card_names()
    spells = [be.index_of(n) for n in be.supported() if "Land" not in names[be.index_of(n)]]
    return [be.index_of("Plains")] * 9 + [be.index_of("Mountain")] * 8 + rng.sample(spells, 23)


@pytest.fixture(scope="module")
def played():
    jobs = [(deck(i), deck(i + 50), 300 + i, i % 2) for i in range(3)]
    return run_games(None, jobs, seats=("rules", "rules"), record=(0, 1))


def test_a_batch_from_storage_equals_collating_the_original_encodings(played, tmp_path):
    arr = to_arrays(played["steps"], played["results"])
    save(tmp_path / "s.npz", arr)
    back = load(tmp_path / "s.npz")
    idx = np.array([0, 5, 17, len(played["steps"]) - 1])
    x, action, z = batch(back, idx)
    want = collate([played["steps"][i]["enc"] for i in idx])
    for k, v in want.items():
        assert torch.equal(x[k], v) if v.dtype != torch.float32 else torch.allclose(x[k], v, atol=2e-3), k
    assert action.tolist() == [played["steps"][i]["action"] for i in idx]


def test_the_value_target_is_the_result_from_the_deciders_side(played):
    arr = to_arrays(played["steps"], played["results"])
    for i, s in enumerate(played["steps"]):
        r = played["results"][s["game"]]["result"]
        want = 0 if r not in ("p0", "p1") else (1 if r == f"p{s['seat']}" else -1)
        assert arr["z"][i] == want


def test_groups_partition_the_rows_by_size_within_the_padding_budget():
    from bolt.play_data import GROUP_BUDGET, GROUP_ROWS, groups

    rng = np.random.default_rng(0)
    n_obj = np.concatenate([rng.integers(5, 60, 500), [460, 300]])
    n_act = np.concatenate([rng.integers(1, 20, 500), [1600, 900]])
    arr = {"obj_off": np.concatenate([[0], np.cumsum(n_obj)]), "act_off": np.concatenate([[0], np.cumsum(n_act)])}
    idx = rng.permutation(len(n_obj))[:400].tolist() + [500, 501]
    gs = groups(arr, idx)
    assert sorted(int(i) for g in gs for i in g) == sorted(idx), "every row in exactly one group"
    for g in gs:
        width = max(int(n_obj[i] + n_act[i]) for i in g)
        assert len(g) <= GROUP_ROWS and (len(g) == 1 or len(g) * width <= GROUP_BUDGET)


def test_giant_boards_share_a_group_with_few_others_and_can_be_left_out():
    from bolt.play_data import GROUP_ATTENTION, groups, within

    n_obj = np.array([40] * 300 + [213, 472, 559, 300, 290, 280])
    n_act = np.array([3] * 300 + [20, 40, 60, 10, 10, 10])
    arr = {"obj_off": np.concatenate([[0], np.cumsum(n_obj)]), "act_off": np.concatenate([[0], np.cumsum(n_act)])}
    gs = groups(arr, np.arange(len(n_obj)))
    for g in gs:
        assert len(g) * max(int(n_obj[i]) for i in g) ** 2 <= GROUP_ATTENTION or len(g) == 1
    alone = [g for g in gs if 302 in g or 301 in g]  # 559 and 472 objects
    assert all(len(g) <= 4 for g in alone)
    assert within(arr, 256).tolist() == list(range(301)), "drops the boards over 256 objects, whole"
    assert len(within(arr, 0)) == len(n_obj)
