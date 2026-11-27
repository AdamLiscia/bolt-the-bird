"""The desktop farm's runner (docs/desktop-farm.md): starts every job in jobs/*.json that isn't done.

    python scripts/farm.py [--budget 12] [--job-threads 12] [--once]

A job file:

    {"name": "tdm_play_1", "threads": 12, "seconds": 172800, "mb": 12000,
     "env": {"BOLT_PLAY_SET": "TDM"}, "cmd": ["python", "scripts/train_ppo_play.py", ...]}

Each job runs under scripts/guard.sh in active-time mode (a paused job's time doesn't count) with its own
`OMP_NUM_THREADS`, and is started only while the total threads of running jobs stays within `--budget`. A job
is done when its S3 pointer (`farm/<name>/latest.json`) says `"done": true`. The job writes that itself with
`bolt.jobs.Job.save(..., done=True)`. A job that exits without finishing is started again on the next pass,
and resumes from its last checkpoint. SIGTERM to farm.py stops every job gracefully (each gets `grace` seconds, default
180, to save a checkpoint) and exits without relaunching: send it to farm.py alone, not its process group.

`--job-threads N` runs every job at N threads instead of its file's `threads`, and passes `BOLT_THREADS=N`,
which train_ppo_play.py uses for --workers and --threads. Nights at 12: stop farm.py gracefully (SIGTERM), then
restart it with `--budget 12 --job-threads 12`; the job resumes from its last checkpoint. Days: restart without it.
"""

import argparse
import json
import os
import signal
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
    env = {**os.environ, **j.get("env", {}), **({"BOLT_THREADS": str(j["threads"])} if j.get("threads_override") else {}), "GUARD_ACTIVE_TIME": "1", "OMP_NUM_THREADS": str(j["threads"]),
           "BOLT_JOB": j["name"], "GUARD_GRACE": str(j.get("grace", 180))}  # fmt: skip
    log = open(ROOT / "logs" / f"farm_{j['name']}.log", "a")  # noqa: SIM115 - kept open for the child
    cmd = [str(ROOT / "scripts" / "guard.sh"), str(j["seconds"]), str(j["mb"]), "--", *j["cmd"]]
    print(f"{time.strftime('%H:%M:%S')} start {j['name']}: {' '.join(j['cmd'])}", flush=True)
    return subprocess.Popen(cmd, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--budget", type=int, default=12, help="threads for all running jobs together")
    ap.add_argument("--job-threads", type=int, default=0, help="run every job at this many threads (0: its file's)")
    ap.add_argument("--once", action="store_true", help="one pass: start what fits, wait for it, exit")
    ap.add_argument("--every", type=int, default=300, help="seconds between passes")
    a = ap.parse_args()
    if a.job_threads > a.budget:
        ap.error("--job-threads can't exceed --budget")
    (ROOT / "logs").mkdir(exist_ok=True)
    running: dict[str, tuple[subprocess.Popen, int]] = {}
    stopping = {"now": False}

    def stop(*_):
        """SIGTERM: pass it to every job (each saves a checkpoint within its grace), wait, and exit without
        starting anything again."""
        stopping["now"] = True
        for name, (proc, _) in running.items():
            print(f"{time.strftime('%H:%M:%S')} stopping {name}", flush=True)
            proc.terminate()
        for proc, _ in running.values():
            proc.wait()
        sys.exit(0)

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    while True:
        for name, (proc, _) in list(running.items()):
            if proc.poll() is not None:
                print(f"{time.strftime('%H:%M:%S')} {name} exited {proc.returncode}", flush=True)
                del running[name]
        used = sum(t for _, t in running.values())
        for j in load_jobs():
            if a.job_threads:
                j["threads"], j["threads_override"] = a.job_threads, True
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
