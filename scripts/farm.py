"""The desktop farm's runner (docs/desktop-farm.md): starts every job in jobs/*.json that isn't done.

    python scripts/farm.py [--budget 12] [--once]

A job file:

    {"name": "tdm_play_1", "threads": 12, "seconds": 172800, "mb": 12000,
     "env": {"BOLT_PLAY_SET": "TDM"}, "cmd": ["python", "scripts/train_ppo_play.py", ...]}

Each job runs under scripts/guard.sh in active-time mode (a paused job's time doesn't count) with its own
`OMP_NUM_THREADS`, and is started only while the total threads of running jobs stays within `--budget`. A job
is done when its S3 pointer (`farm/<name>/latest.json`) says `"done": true`. The job writes that itself with
`bolt.jobs.Job.save(..., done=True)`. A job that exits without finishing is started again on the next pass,
and resumes from its last checkpoint.
"""

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from bolt.jobs import Job  # noqa: E402


def load_jobs() -> list[dict]:
    jobs = []
    for f in sorted((ROOT / "jobs").glob("*.json")):
        j = json.loads(f.read_text())
        j.setdefault("threads", 1)
        j.setdefault("seconds", 86400)
        j.setdefault("mb", 8000)
        jobs.append(j)
    return jobs


def start(j: dict) -> subprocess.Popen:
    env = {**os.environ, **j.get("env", {}), "GUARD_ACTIVE_TIME": "1", "OMP_NUM_THREADS": str(j["threads"]),
           "BOLT_JOB": j["name"]}  # fmt: skip
    log = open(ROOT / "logs" / f"farm_{j['name']}.log", "a")  # noqa: SIM115 - kept open for the child
    cmd = [str(ROOT / "scripts" / "guard.sh"), str(j["seconds"]), str(j["mb"]), "--", *j["cmd"]]
    print(f"{time.strftime('%H:%M:%S')} start {j['name']}: {' '.join(j['cmd'])}", flush=True)
    return subprocess.Popen(cmd, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--budget", type=int, default=12, help="threads for all running jobs together")
    ap.add_argument("--once", action="store_true", help="one pass: start what fits, wait for it, exit")
    ap.add_argument("--every", type=int, default=300, help="seconds between passes")
    a = ap.parse_args()
    (ROOT / "logs").mkdir(exist_ok=True)
    running: dict[str, tuple[subprocess.Popen, int]] = {}
    while True:
        for name, (proc, _) in list(running.items()):
            if proc.poll() is not None:
                print(f"{time.strftime('%H:%M:%S')} {name} exited {proc.returncode}", flush=True)
                del running[name]
        used = sum(t for _, t in running.values())
        for j in load_jobs():
            if j["name"] in running or used + j["threads"] > a.budget:
                continue
            try:
                if Job(j["name"]).done():
                    continue
            except Exception as e:  # noqa: BLE001 - S3 unreachable: try again next pass
                print(f"{j['name']}: can't read its pointer ({e}); skipping this pass", flush=True)
                continue
            running[j["name"]] = (start(j), j["threads"])
            used += j["threads"]
        if a.once:
            for proc, _ in running.values():
                proc.wait()
            return
        time.sleep(a.every)


if __name__ == "__main__":
    main()
