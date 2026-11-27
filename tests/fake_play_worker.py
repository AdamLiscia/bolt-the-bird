"""A stand-in play worker for tests/test_play_hang.py: plays nothing, beats its heartbeat like run_games, and
hangs forever on a job whose seed is 666 (an engine call that never returns)."""

import time

from bolt import play_runner as pr


def worker(args):
    _net_state, jobs, kw = args
    results = []
    for gi, job in enumerate(jobs):
        pr._beat(gi)
        if gi in kw["skip"]:
            results.append({"result": "timeout", "turn": 0, "hung": True})
            continue
        if job[2] == 666:
            time.sleep(3600)
        results.append({"result": "p0", "turn": 1})
    return {"results": results, "steps": []}
