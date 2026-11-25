"""P4: play an engine job file with the trained network on both sides (or the rules-based player).

    python scripts/play_jobs_network.py DECKS JOBS OUT [--pilot network|rules] [--hours 10]

DECKS: one deck per line (card numbers). JOBS: lines "index deck_a deck_b seed first", the format
scripts/engine_tournament.py and engine_pilot_validate.py write. OUT gets "index result turns" lines,
the same as the Rust runner's games_<pilot>.txt, so the existing analyses read it unchanged.
Resumable: jobs already in OUT are skipped. Results are appended a batch at a time, so a stop loses at
most one batch. The network plays greedily (its best move), so every game is reproducible.
"""

import argparse
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from bolt.play_runner import run_parallel  # noqa: E402

CKPT = ROOT / "checkpoints" / "play" / "play_v2_ppo3_i850.pt"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("decks")
    ap.add_argument("jobs")
    ap.add_argument("out")
    ap.add_argument("--pilot", choices=["network", "rules"], default="network")
    ap.add_argument("--ckpt", default=str(CKPT))
    ap.add_argument("--workers", type=int, default=4)
    ap.add_argument("--batch", type=int, default=2000, help="games per append")
    ap.add_argument("--hours", type=float, default=10)
    ap.add_argument("--limit", type=int, default=None, help="play at most this many (tests)")
    a = ap.parse_args()
    decks = [list(map(int, x.split())) for x in Path(a.decks).read_text().split("\n") if x.strip()]
    out = Path(a.out)
    done = set()
    if out.exists():
        done = {int(x.split()[0]) for x in out.read_text().split("\n") if x.strip()}
    todo = []
    for line in Path(a.jobs).read_text().split("\n"):
        x = line.split()
        if len(x) == 5 and int(x[0]) not in done:
            todo.append((int(x[0]), int(x[1]), int(x[2]), int(x[3]), int(x[4])))
    if a.limit is not None:
        todo = todo[: a.limit]
    net = None
    if a.pilot == "network":
        import torch

        c = torch.load(a.ckpt, map_location="cpu", weights_only=False)
        from bolt.play_net import load_checkpoint

        net = load_checkpoint(c)
        net.eval()
    seat = "net" if a.pilot == "network" else "rules"
    end = time.time() + a.hours * 3600
    t0, played = time.time(), 0
    for s in range(0, len(todo), a.batch):
        if time.time() > end:
            print(f"time limit reached; {len(todo) - played} jobs left", flush=True)
            break
        part = todo[s : s + a.batch]
        jobs = [(decks[i], decks[j], seed % 2**64, first) for _, i, j, seed, first in part]
        r = run_parallel(net, jobs, workers=a.workers, chunk=25, seats=(seat, seat), sample=False, record=())
        with out.open("a") as f:
            for (idx, *_), res in zip(part, r["results"], strict=True):
                f.write(f"{idx} {res['result']} {res['turn']}\n")
        played += len(part)
        rate = played / (time.time() - t0)
        print(f"{played} / {len(todo)} games, {rate:.1f}/s, about {(len(todo) - played) / rate / 60:.0f} min left",
              flush=True)  # fmt: skip
    print(f"done: {played} played; {len(todo) - played} left", flush=True)


if __name__ == "__main__":
    main()
