"""A stand-in play worker for tests/test_play_hang.py: plays nothing, beats its heartbeat like run_games, and
hangs forever on a job whose seed is 666 (an engine call that never returns); seed 777 comes back as a game
stopped for time, seed 888 as an engine panic."""

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
        if job[2] == 888:  # an engine panic, as run_games reports it
            results.append({"result": "error", "turn": 4, "panic": {"message": "payable", "moves": 12, "turn": 4,
                                                                    "last": [("Cast", "Sol Ring")]}})  # fmt: skip
            continue
        if job[2] == 777:  # stopped by its own time budget in run_games
            results.append({"result": "timeout", "turn": 9, "hung": True, "slow": {"moves": 50, "seconds": 151}})
            continue
        results.append({"result": "p0", "turn": 1})
    return {"results": results, "steps": []}
