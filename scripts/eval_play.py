"""P3c/P3d evaluation: the play network (greedy) against fixed players on unseen decks.

    python scripts/eval_play.py --ckpt checkpoints/play/distill1/best.pt --pairs 500 --name play_eval_distill1

Decks: data/m7/decks.txt lines 512-1023 (never used in training). Pair i plays deck i against deck i+1,
the network once in each seat, same seed and who goes first. Opponents: the rules-based player and the
smart 1 x 2 player; the control is smart 1 x 2 against the rules-based player on the same pairs
(the level distillation aims for).
"""

import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np
import torch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

import bolt_engine as be  # noqa: E402

from bolt.play_net import PlayNet  # noqa: E402
from bolt.play_runner import run_parallel  # noqa: E402


def score(results, seat):
    s = [1.0 if r["result"] == f"p{seat}" else 0.5 if r["result"] in ("draw", "timeout") else 0.0 for r in results]
    return s


def match(net, me, opp, jobs, workers, smart=(1, 2)):
    """`me` against `opp`, `me` in each seat in turn. Returns the score list from `me`'s side."""
    a = run_parallel(net, jobs, workers=workers, seats=(me, opp), sample=False, chunk=20, record=(), smart=smart)
    b = run_parallel(net, jobs, workers=workers, seats=(opp, me), sample=False, chunk=20, record=(), smart=smart)
    return score(a["results"], 0) + score(b["results"], 1)


def summary(s):
    s = np.array(s)
    return {"win_rate": float(s.mean()), "se": float(s.std() / np.sqrt(len(s))), "games": len(s),
            "draws_or_timeouts": int((s == 0.5).sum())}  # fmt: skip


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", default=None, help="best.pt or step_N.pt; omitted = untrained network")
    ap.add_argument("--pairs", type=int, default=500)
    ap.add_argument("--workers", type=int, default=6)
    ap.add_argument("--opponents", nargs="+", default=["rules", "smart"])
    ap.add_argument("--no-control", action="store_true")
    ap.add_argument("--name", default="play_eval")
    ap.add_argument("--smart", type=int, nargs=2, default=[1, 2], metavar=("ROLLOUTS", "CANDIDATES"),
                    help="the smart player's search: 1 2 (fast) or 4 6 (the strongest)")  # fmt: skip
    a = ap.parse_args()
    if a.ckpt:
        c = torch.load(a.ckpt, map_location="cpu", weights_only=False)
        from bolt.play_net import load_checkpoint

        net = load_checkpoint(c)
    else:
        torch.manual_seed(0)
        net = PlayNet(len(be.card_names()))
    net.eval()
    lines = (ROOT / "data" / "m7" / "decks.txt").read_text().split("\n")
    decks = [list(map(int, x.split())) for x in lines[512:1024]]
    jobs = [(decks[i % 512], decks[(i + 1) % 512], 9_000 + i, i % 2) for i in range(a.pairs)]
    t = time.time()
    out = {"ckpt": a.ckpt, "pairs": a.pairs, "smart": a.smart}
    for opp in a.opponents:
        out[f"net_vs_{opp}"] = summary(match(net, "net", opp, jobs, a.workers, tuple(a.smart)))
        print(opp, out[f"net_vs_{opp}"], flush=True)
    if not a.no_control:
        out["control_smart_vs_rules"] = summary(match(None, "smart", "rules", jobs, a.workers, tuple(a.smart)))
    out["seconds"] = round(time.time() - t)
    (ROOT / "results" / f"{a.name}.json").write_text(json.dumps(out, indent=2) + "\n")
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
