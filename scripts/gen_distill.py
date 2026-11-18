"""P3c data: the smart player's decisions in smart-vs-smart games, both sides recorded.

    python scripts/gen_distill.py --shards 40          # rerun to resume: finished shards are skipped

Training decks only (data/m7/decks.txt lines 0-511); lines 512-1023 stay unseen for evaluation.
Each shard: --games games between two random training decks, seats and who goes first varied,
written atomically to data/distill/shard_NNN.npz.
"""

import argparse
import json
import random
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from bolt.play_data import save, to_arrays  # noqa: E402
from bolt.play_runner import run_parallel  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--shards", type=int, default=40)
    ap.add_argument("--games", type=int, default=480)
    ap.add_argument("--workers", type=int, default=6)
    ap.add_argument("--rollouts", type=int, default=1)
    ap.add_argument("--candidates", type=int, default=2)
    ap.add_argument("--out", default="data/distill")
    a = ap.parse_args()
    out = ROOT / a.out
    out.mkdir(parents=True, exist_ok=True)
    lines = (ROOT / "data" / "m7" / "decks.txt").read_text().split("\n")
    decks = [list(map(int, x.split())) for x in lines[:512]]
    for k in range(a.shards):
        path = out / f"shard_{k:03d}.npz"
        if path.exists():
            continue
        rng = random.Random(1_000 + k)
        jobs = []
        for i in range(a.games):
            d1, d2 = rng.sample(range(512), 2)
            jobs.append((decks[d1], decks[d2], 7_000_000 + k * 10_000 + i, i % 2))
        t = time.time()
        r = run_parallel(None, jobs, workers=a.workers, chunk=20, seats=("smart", "smart"), record=(0, 1),
                         smart=(a.rollouts, a.candidates))  # fmt: skip
        save(path, to_arrays(r["steps"], r["results"]))
        kinds = {x: sum(g["result"] == x for g in r["results"]) for x in ("p0", "p1", "draw", "timeout", "error")}
        print(json.dumps({"shard": k, "decisions": len(r["steps"]), "seconds": round(time.time() - t), **kinds}),
              flush=True)  # fmt: skip


if __name__ == "__main__":
    main()
