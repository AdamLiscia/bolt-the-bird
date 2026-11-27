"""run_parallel's watchdog: a worker stuck in one engine call is killed, and its chunk rerun without that game."""

import json
import time

from fake_play_worker import worker

from bolt.play_runner import run_parallel


def test_a_hung_game_is_a_timeout_and_the_rest_of_its_chunk_still_plays(tmp_path):
    jobs = [(f"deck{s}", "other", s, s % 2) for s in (1, 2, 666, 4, 5)]
    log = tmp_path / "hangs.jsonl"
    t = time.time()
    out = run_parallel(None, jobs, workers=2, chunk=3, hang_s=2, hang_log=str(log), worker=worker,
                       seats=("net", "net"))  # fmt: skip
    assert time.time() - t < 60
    assert [r["result"] for r in out["results"]] == ["p0", "p0", "timeout", "p0", "p0"]
    assert out["results"][2].get("hung")
    rows = [json.loads(x) for x in log.read_text().splitlines()]
    assert len(rows) == 1 and rows[0]["seed"] == 666 and rows[0]["job"] == 2 and rows[0]["deck_a"] == "deck666"
    assert rows[0]["seats"] == ["net", "net"]
