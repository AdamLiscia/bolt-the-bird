"""P3d: self-play PPO for the play network (docs/play/P3_DESIGN.md).

    python scripts/train_ppo_play.py --name ppo1                 # fresh, from --init
    python scripts/train_ppo_play.py --name ppo1 --resume        # after a stop or a crash

Each iteration: --games games on the training decks (data/m7/decks.txt lines 0-511) against a league
(bolt.play_ppo.league: the rules-based player, the smart 1 x 2 player, past checkpoints, itself), the
network sampling its moves; then --epochs passes of clipped PPO over the recorded moves, advantage =
result - value estimate (standardised), with a KL leash to --init that decays over --kl-iters.
Every iteration's weights are kept (checkpoints/play/<name>/iter_N.pt); state.pt holds everything
needed to resume, written atomically. Every --eval-every iterations: greedy play against the
rules-based and smart players on unseen decks (lines 512-1023), into results/ppo_play_<name>.json.
"""

import argparse
import json
import os
import random
import sys
import threading
import time
from pathlib import Path

import numpy as np
import torch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "scripts"))

from bolt.backup import upload_quietly  # noqa: E402
from bolt.play_data import concat  # noqa: E402
from bolt.play_net import pad_optimizer, pad_state  # noqa: E402
from bolt.play_ppo import gae, league, ppo_step  # noqa: E402
from bolt.play_runner import run_parallel  # noqa: E402

# One deck per line (card numbers of BOLT_PLAY_SET): lines 0-511 train, 512-1023 evaluate. In Commander
# (BOLT_PLAY_FORMAT=commander) a bolt.cmd_decks file, every deck both trained and evaluated on: the
# playgroup's decks are the decks it will play, and there are only a handful.
DECKS = Path(os.environ.get("BOLT_DECKS", str(ROOT / "data" / "m7" / "decks.txt")))
COMMANDER = os.environ.get("BOLT_PLAY_FORMAT", "").lower() == "commander"


def deck_sets():
    """(training decks, evaluation decks)."""
    if COMMANDER:
        from bolt.cmd_decks import read

        decks = read(DECKS)
        if len(decks) < 2:
            raise SystemExit(f"{DECKS}: Commander needs at least two decks, found {len(decks)}")
        return decks, decks
    lines = DECKS.read_text().split("\n")
    return [list(map(int, x.split())) for x in lines[:512]], [list(map(int, x.split())) for x in lines[512:1024]]


def eval_jobs(decks, pairs):
    """Evaluation games: each deck against each other in turn (never itself), seeds fixed."""
    n = len(decks)
    return [(decks[i % n], decks[(i + 1 + (i // n) % (n - 1)) % n], 9_000 + i, i % 2) for i in range(pairs)]


def load_net(path):
    from bolt.play_net import load_checkpoint

    return load_checkpoint(path)


def save_atomic(obj, path: Path):
    tmp = path.with_suffix(".tmp")
    torch.save(obj, tmp)
    tmp.replace(path)


def evaluate(net, pairs, workers, smart_ms=None):
    from eval_play import match, summary

    jobs = eval_jobs(deck_sets()[1], pairs)
    net.eval()
    out = {}
    for opp in ("rules", "smart"):
        t = time.time()
        smart = (1, 2) if smart_ms is None else (1, 2, smart_ms)
        out[opp] = summary(match(net, "net", opp, jobs, workers, smart=smart))["win_rate"]
        out[f"{opp}_s"] = round(time.time() - t)  # wall seconds for its 2 x pairs games
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--name", default="ppo1")
    ap.add_argument("--init", default="checkpoints/play/distill2/step_17353.pt")
    ap.add_argument("--iters", type=int, default=1000)
    ap.add_argument("--games", type=int, default=320)
    ap.add_argument("--workers", type=int, default=4)
    ap.add_argument("--threads", type=int, default=4)
    ap.add_argument("--epochs", type=int, default=2)
    ap.add_argument("--batch", type=int, default=256)
    ap.add_argument("--lr", type=float, default=2e-5)
    ap.add_argument("--clip", type=float, default=0.2)
    ap.add_argument("--kl", type=float, default=0.05)
    ap.add_argument("--kl-final", type=float, default=0.005)
    ap.add_argument("--kl-iters", type=int, default=100)
    ap.add_argument("--value-coef", type=float, default=0.5)
    ap.add_argument("--lam", type=float, default=1.0, help="GAE lambda: 1 = result minus value; lower = per-step")
    ap.add_argument("--past-every", type=int, default=10)
    ap.add_argument("--eval-every", type=int, default=10)
    ap.add_argument("--eval-pairs", type=int, default=200)
    ap.add_argument("--stall-minutes", type=float, default=30)
    ap.add_argument("--eval-smart-ms", type=int, default=None, help="cap the smart opponent's time per move")
    ap.add_argument("--resume", action="store_true")
    ap.add_argument(
        "--self-play",
        action="store_true",
        help="train only against itself and its past checkpoints (no rules or smart opponents)",
    )
    a = ap.parse_args()
    if os.environ.get("BOLT_THREADS"):  # farm.py --job-threads (the 12-thread nights)
        a.workers = a.threads = int(os.environ["BOLT_THREADS"])
    torch.set_num_threads(a.threads)
    ck = ROOT / "checkpoints" / "play" / a.name
    ck.mkdir(parents=True, exist_ok=True)
    results = ROOT / "results" / f"ppo_play_{a.name}.json"
    net = load_net(ROOT / a.init)
    ref = load_net(ROOT / a.init).eval()
    for p in ref.parameters():
        p.requires_grad_(False)
    opt = torch.optim.AdamW(net.parameters(), lr=a.lr, weight_decay=0.0)
    state = {"iter": 0, "history": [], "evals": [], "past": [], "best": None}
    if a.resume and (ck / "state.pt").exists():
        s = torch.load(ck / "state.pt", map_location="cpu", weights_only=False)
        net.load_state_dict(pad_state(s["model"]))
        opt.load_state_dict(s["opt"])
        pad_optimizer(opt, net)
        state = s["state"]
        print(f"resumed at iteration {state['iter']}", flush=True)
    # On the farm (BOLT_JOB set by farm.py): power-cut-safe checkpoints on S3 (bolt.jobs), resumed first.
    job = None
    if os.environ.get("BOLT_JOB"):
        from bolt.jobs import Job

        job = Job(os.environ["BOLT_JOB"])
        s = job.resume()
        if s is not None:
            net.load_state_dict(pad_state(s["model"]))  # saved before inputs were appended: grown
            opt.load_state_dict(s["opt"])
            pad_optimizer(opt, net)
            state = s["state"]
            for f in state["past"]:  # a fresh disk: fetch the past opponents too
                if not (ck / f).exists():
                    data = job._get(f"{job.prefix}/past/{f}")
                    if data is not None:
                        (ck / f).write_bytes(data)
            state["past"] = [f for f in state["past"] if (ck / f).exists()]
            print(f"resumed from S3 at iteration {state['iter']}", flush=True)
    decks = deck_sets()[0]

    beat = {"t": time.time()}

    def watchdog():
        while True:
            time.sleep(30)
            if time.time() - beat["t"] > a.stall_minutes * 60:
                print(f"STALL: no progress for {a.stall_minutes} minutes at iteration {state['iter']}; exiting",
                      flush=True)  # fmt: skip
                os._exit(3)

    threading.Thread(target=watchdog, daemon=True).start()

    def write_results():
        results.write_text(json.dumps({"args": vars(a), "history": state["history"], "evals": state["evals"]},
                                      indent=1))  # fmt: skip

    if state["iter"] == 0 and not state["evals"]:
        e = evaluate(net, a.eval_pairs, a.workers, a.eval_smart_ms)
        state["evals"].append({"iter": 0, **e})
        print(json.dumps({"eval": 0, **e}), flush=True)
        write_results()

    while state["iter"] < a.iters:
        it = state["iter"] + 1
        t0 = time.time()
        rng = random.Random(it)
        kl_coef = a.kl + (a.kl_final - a.kl) * min(1.0, (it - 1) / max(1, a.kl_iters))
        net.eval()
        parts, won, ends = [], {}, {"error": 0, "timeout": 0}
        for k, (seats, n) in enumerate(league(a.games, state["past"], self_only=a.self_play)):
            jobs = []
            for i in range(n):
                d1, d2 = rng.sample(range(len(decks)), 2)
                jobs.append((decks[d1], decks[d2], 20_000_000 + it * 10_000 + k * 1_000 + i, i % 2))
            opp = load_net(ck / rng.choice(state["past"])) if "opp" in seats else None
            record = tuple(i for i, s in enumerate(seats) if s == "net")
            # Commander games vary a lot in length: small chunks (4 a worker) keep every worker busy to the end
            chunk = max(1, -(-len(jobs) // (4 * a.workers))) if COMMANDER else 20
            r = run_parallel(net, jobs, workers=a.workers, chunk=chunk, seats=seats, sample=True, seed=it * 100 + k,
                             record=record, opp_net=opp, compact=True)  # fmt: skip
            part = r["arrays"]
            part["adv"], part["ret"] = gae(part, lam=a.lam)  # per matchup: game numbers restart per call
            parts.append(part)
            other = [s for s in seats if s != "net"]
            label = other[0] if other else "self"
            me = seats.index("net")
            w = [1.0 if x["result"] == f"p{me}" else 0.5 if x["result"] in ("draw", "timeout") else 0.0
                 for x in r["results"]]  # fmt: skip
            won.setdefault(label, []).extend(w)
            for x in r["results"]:
                if x["result"] in ends:
                    ends[x["result"]] += 1
            beat["t"] = time.time()
        arr = concat(parts)
        n = len(arr["action"])
        adv = arr["adv"]
        adv = torch.as_tensor((adv - adv.mean()) / (adv.std() + 1e-6))
        ret = torch.as_tensor(arr["ret"])
        t1 = time.time()
        net.train()
        sums, steps = {"loss": 0.0, "clipped": 0.0, "kl": 0.0, "value_mse": 0.0}, 0
        prng = np.random.default_rng(it)
        for _ in range(a.epochs):
            perm = prng.permutation(n)
            for s in range(0, n - a.batch + 1, a.batch):
                idx = np.sort(perm[s : s + a.batch])
                st = ppo_step(net, ref, opt, arr, idx, adv[idx], a.clip, kl_coef, a.value_coef, ret=ret[idx])
                for key in sums:
                    sums[key] += st[key]
                steps += 1
                beat["t"] = time.time()
        state["iter"] = it
        torch.save({"config": net.config, "model": net.state_dict()}, ck / f"iter_{it}.pt")
        upload_quietly(ck / f"iter_{it}.pt")
        if it % a.past_every == 0:
            state["past"].append(f"iter_{it}.pt")
            if job:
                job._put(f"{job.prefix}/past/iter_{it}.pt", (ck / f"iter_{it}.pt").read_bytes())
        row = {"iter": it, "games": a.games, "decisions": n, "kl_coef": round(kl_coef, 4), "errors": ends["error"],
               "timeouts": ends["timeout"],
               **{f"won_{k}": round(float(np.mean(v)), 3) for k, v in won.items()},
               **{k: round(v / max(1, steps), 4) for k, v in sums.items()},
               "play_s": round(t1 - t0), "update_s": round(time.time() - t1), "time": time.strftime("%H:%M:%S")}  # fmt: skip
        state["history"].append(row)
        print(json.dumps(row), flush=True)
        if it % a.eval_every == 0:
            e = evaluate(net, a.eval_pairs, a.workers, a.eval_smart_ms)
            state["evals"].append({"iter": it, **e})
            score = (e["rules"] + e["smart"]) / 2
            if state["best"] is None or score > state["best"]["score"]:
                state["best"] = {"iter": it, "score": score}
                torch.save({"config": net.config, "model": net.state_dict()}, ck / "best.pt")
                upload_quietly(ck / "best.pt")
            print(json.dumps({"eval": it, **e, "best_iter": state["best"]["iter"]}), flush=True)
            beat["t"] = time.time()
        save_atomic({"model": net.state_dict(), "opt": opt.state_dict(), "state": state}, ck / "state.pt")
        write_results()
        if job and job.due():
            job.save({"model": net.state_dict(), "opt": opt.state_dict(), "state": state}, step=it)
            if job.stop_requested:
                print(f"stopped at iteration {it} (SIGTERM); checkpoint saved", flush=True)
                return
    if job:
        job.save({"model": net.state_dict(), "opt": opt.state_dict(), "state": state}, step=state["iter"], done=True)


if __name__ == "__main__":
    main()
