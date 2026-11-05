import numpy as np
import pytest
from prepare import counts_to_ids

from bolt.data import half_of, split_of

KEYS = [f"{i:032x}" for i in range(20_000)]


def test_split_is_deterministic():
    assert [split_of(k) for k in KEYS[:100]] == [split_of(k) for k in KEYS[:100]]


def test_split_proportions():
    splits = np.array([split_of(k) for k in KEYS])
    assert abs((splits == "train").mean() - 0.85) < 0.01
    assert abs((splits == "val").mean() - 0.05) < 0.01
    assert abs((splits == "test").mean() - 0.10) < 0.01


def test_split_is_by_draft_so_picks_cannot_leak():
    # Every pick of a draft carries the same draft_id, so it lands in the same split.
    picks = [(key, pick) for key in KEYS[:50] for pick in range(42)]
    by_draft = {}
    for key, _ in picks:
        by_draft.setdefault(key, set()).add(split_of(key))
    assert all(len(s) == 1 for s in by_draft.values())


def test_judge_halves_are_balanced_and_independent_of_split():
    halves = np.array([half_of(k) for k in KEYS])
    train = np.array([split_of(k) == "train" for k in KEYS])
    assert abs(halves.mean() - 0.5) < 0.02
    assert abs(halves[train].mean() - 0.5) < 0.02


def test_counts_to_ids_round_trip():
    counts = np.array([[0, 2, 0, 1], [1, 0, 0, 0], [0, 0, 0, 0]], dtype=np.int8)
    ids = counts_to_ids(counts, 5)
    assert ids.shape == (3, 5)
    back = np.zeros_like(counts)
    for r, row in enumerate(ids):
        for i in row[row > 0]:
            back[r, i - 1] += 1
    assert (back == counts).all()


def test_counts_to_ids_refuses_overflow():
    with pytest.raises(ValueError):
        counts_to_ids(np.array([[3, 3]], dtype=np.int8), 5)
