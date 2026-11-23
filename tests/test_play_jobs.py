"""P4: the network-pilot job runner plays the engine's job files and writes the same result lines."""

import subprocess
import sys
from pathlib import Path

import pytest

be = pytest.importorskip("bolt_engine")
ROOT = Path(__file__).resolve().parents[1]


def setup(tmp_path, n=12):
    lines = (ROOT / "data/m7/decks.txt").read_text().split("\n")[512:516]
    (tmp_path / "decks.txt").write_text("\n".join(lines) + "\n")
    jobs = [f"{k} {k % 4} {(k + 1) % 4} {1000 + k} {k % 2}" for k in range(n)]
    (tmp_path / "jobs.txt").write_text("\n".join(jobs) + "\n")
    decks = [list(map(int, x.split())) for x in lines]
    return [(decks[k % 4], decks[(k + 1) % 4], 1000 + k, k % 2) for k in range(n)]


def run(tmp_path, pilot, limit=None):
    cmd = [sys.executable, str(ROOT / "scripts/play_jobs_network.py"), str(tmp_path / "decks.txt"),
           str(tmp_path / "jobs.txt"), str(tmp_path / f"games_{pilot}.txt"), "--pilot", pilot, "--workers", "2"]  # fmt: skip
    if limit:
        cmd += ["--limit", str(limit)]
    subprocess.run(cmd, check=True, capture_output=True, timeout=600)
    return [line.split() for line in (tmp_path / f"games_{pilot}.txt").read_text().split("\n") if line]


def test_rules_pilot_replays_the_engines_own_games(tmp_path):
    jobs = setup(tmp_path)
    got = {int(i): (r, int(t)) for i, r, t in run(tmp_path, "rules")}
    want = be.play_games(jobs, pilot="baseline", threads=2)
    assert got == {k: (r, t) for k, (r, t) in enumerate(want)}


def test_network_pilot_finishes_every_job_and_resumes(tmp_path):
    setup(tmp_path)
    first = run(tmp_path, "network", limit=5)
    assert len(first) == 5
    rows = run(tmp_path, "network")
    assert sorted(int(r[0]) for r in rows) == list(range(12)), "the rest, no repeats"
    assert all(r[1] in ("p0", "p1", "draw", "timeout", "error") for r in rows)
    assert run(tmp_path, "network") == rows, "nothing left: nothing added"
