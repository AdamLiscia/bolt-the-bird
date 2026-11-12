"""Engine games in a separate process: a crash there costs a retry, not the training run."""

import os
import random
from pathlib import Path

import pytest

from bolt.engine_worker import SafePlayer

be = pytest.importorskip("bolt_engine")


def jobs(n=6):
    names = be.card_names()
    spells = [be.index_of(x) for x in be.supported() if "Land" not in names[be.index_of(x)]]
    rng = random.Random(9)

    def deck():
        return [be.index_of("Plains")] * 9 + [be.index_of("Mountain")] * 8 + rng.sample(spells, 23)

    return [(deck(), deck(), s, s % 2) for s in range(n)]


def test_results_match_playing_in_this_process():
    js = jobs()
    with SafePlayer() as p:
        got = p.play_games(js, pilot="smart", rollouts=1, candidates=2, threads=2)
    assert got == be.play_games(js, pilot="smart", rollouts=1, candidates=2, threads=2)


def crash_once(marker, jobs, **kw):
    """Stand-in for the worker's job: dies hard the first time, plays normally after."""
    if not Path(marker).exists():
        Path(marker).write_text("crashed")
        os._exit(134)  # like the abort the engine has hit: no exception, the process just ends
    return be.play_games(jobs, **kw)


def test_a_worker_that_dies_is_restarted_and_the_batch_replayed(tmp_path):
    js = jobs(4)
    marker = tmp_path / "crashed"
    with SafePlayer(target=crash_once, target_args=(str(marker),)) as p:
        got = p.play_games(js, pilot="baseline", threads=1)
        assert p.restarts == 1
    assert marker.exists()
    assert got == be.play_games(js, pilot="baseline", threads=1)


def always_crash(jobs, **kw):
    os._exit(134)


def test_it_gives_up_after_three_restarts():
    with SafePlayer(target=always_crash, retries=3) as p, pytest.raises(RuntimeError, match="3"):
        p.play_games(jobs(2), pilot="baseline", threads=1)
