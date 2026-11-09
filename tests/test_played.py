"""The played-game reward: seat 0's deck against its 7 tablemates, scored by the engine."""

import random

import numpy as np
import pytest

from bolt.played import score, table_jobs


def fake_decks(tables: int) -> list[list[list[int]]]:
    return [[[100 * t + s] * 40 for s in range(8)] for t in range(tables)]


def test_seat_zero_plays_each_tablemate_with_alternating_first_player():
    jobs, owner = table_jobs(fake_decks(3), games=2, seed=7)
    assert len(jobs) == 3 * 7 * 2
    for (a, b, _seed, _first), t in zip(jobs, owner, strict=True):
        assert a == [100 * t] * 40, "seat 0 is always player 0"
        assert b != a and b[0] // 100 == t, "a tablemate from the same table"
    assert [j[3] for j in jobs[:4]] == [0, 1, 0, 1]
    assert len({j[2] for j in jobs}) == len(jobs), "every game has its own seed"
    assert table_jobs(fake_decks(3), games=2, seed=8)[0][0][2] != jobs[0][2]


def test_score_is_seat_zeros_win_rate_per_table_with_draws_as_half():
    owner = np.array([0, 0, 0, 0, 1, 1])
    results = [("p0", 9), ("p1", 9), ("draw", 9), ("timeout", 200), ("p0", 5), ("p0", 6)]
    r, errors = score(results, owner, 2)
    assert r.tolist() == [0.5, 1.0] and errors == 0


def test_a_crashed_game_is_left_out_and_counted():
    owner = np.array([0, 0, 1])
    r, errors = score([("p0", 9), ("error", 0), ("p1", 9)], owner, 2)
    assert r.tolist() == [1.0, 0.0] and errors == 1


be = pytest.importorskip("bolt_engine")


def test_played_rewards_runs_real_games():
    from bolt.played import played_rewards

    names = be.card_names()
    spells = [be.index_of(n) for n in be.supported() if "Land" not in names[be.index_of(n)]]
    rng = random.Random(3)
    decks = [
        [[be.index_of("Plains")] * 9 + [be.index_of("Swamp")] * 8 + rng.sample(spells, 23) for _ in range(8)]
        for _ in range(2)
    ]
    r, errors = played_rewards(decks, games=1, seed=5, rollouts=1, candidates=2, threads=4)
    assert r.shape == (2,) and ((r >= 0) & (r <= 1)).all() and errors == 0
    again, _ = played_rewards(decks, games=1, seed=5, rollouts=1, candidates=2, threads=2)
    assert (again == r).all(), "deterministic"
