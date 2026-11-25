"""P3c data: the smart player's decisions in smart-vs-smart games, both sides recorded.

    python scripts/gen_distill.py --shards 40          # rerun to resume: finished shards are skipped
    python scripts/gen_distill.py --ckpt checkpoints/play/distill1/best.pt --out data/dagger1 --shards 25

With --ckpt (DAgger): the network plays, against the rules-based player, the smart player and itself
(each seat in turn), and the smart player labels every position the network reaches.

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

from bolt.play_data import concat, save  # noqa: E402
from bolt.play_runner import run_parallel  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--shards", type=int, default=40)
    ap.add_argument("--games", type=int, default=480)
    ap.add_argument("--workers", type=int, default=6)
    ap.add_argument("--rollouts", type=int, default=1)
    ap.add_argument("--candidates", type=int, default=2)
    ap.add_argument("--out", default="data/distill")
    ap.add_argument("--ckpt", default=None, help="DAgger: the network to play with")
    a = ap.parse_args()
    net = None
    if a.ckpt:
        import torch

        c = torch.load(a.ckpt, map_location="cpu", weights_only=False)
        from bolt.play_net import load_checkpoint

        net = load_checkpoint(c)
        net.eval()
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
        smart = (a.rollouts, a.candidates)
        if net is None:
            r = run_parallel(None, jobs, workers=a.workers, chunk=20, seats=("smart", "smart"), record=(0, 1),
                             smart=smart, compact=True)  # fmt: skip
        else:
            matchups = [("net", "rules"), ("rules", "net"), ("net", "smart"), ("smart", "net"), ("net", "net")]
            parts = []
            for m, seats in enumerate(matchups):
                parts.append(run_parallel(net, jobs[m :: len(matchups)], workers=a.workers, chunk=20, seats=seats,
                                          sample=False, smart=smart, label_with=smart, compact=True))  # fmt: skip
            r = {"results": [x for q in parts for x in q["results"]], "arrays": concat([q["arrays"] for q in parts])}
        save(path, r["arrays"])
        kinds = {x: sum(g["result"] == x for g in r["results"]) for x in ("p0", "p1", "draw", "timeout", "error")}
        print(json.dumps({"shard": k, "decisions": len(r["arrays"]["action"]), "seconds": round(time.time() - t), **kinds}),
              flush=True)  # fmt: skip


if __name__ == "__main__":
    main()
