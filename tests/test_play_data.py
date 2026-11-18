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
