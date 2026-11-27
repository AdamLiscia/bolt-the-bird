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
import collections
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
from bolt.play_data import concat, finite_rows, within  # noqa: E402
from bolt.play_net import load_into, pad_optimizer  # noqa: E402
from bolt.play_ppo import gae, league, ppo_step  # noqa: E402
from bolt.play_runner import Stopped, run_parallel  # noqa: E402

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


PLAY_SET = os.environ.get("BOLT_PLAY_SET", "FDN").upper()


def load_net(path):
    """A play checkpoint on today's card pool for BOLT_PLAY_SET (rows moved if the pool grew since it was saved)."""
    from bolt.play_net import load_checkpoint

    return load_checkpoint(path, set_name=PLAY_SET)


def resume_into(net, opt, s, lr=None):
    """A saved {"model", "opt", ...} into net and opt: inputs grown, card rows moved to today's pool (by the names
    it was saved with, or for a state saved before names were kept, the pool of its size; bolt.play_net).
    Adam's moments carry over; `lr` (this run's --lr) replaces the saved one, which load_state_dict restores."""
    from bolt.play_net import legacy_names

    names = s.get("card_names")
    if names is None and net.config.get("set_name"):
        names = legacy_names(PLAY_SET, s["model"]["card.weight"].shape[0] - 2)
    rows = load_into(net, s["model"], names)
    opt.load_state_dict(s["opt"])
    pad_optimizer(opt, net, rows)
    if lr is not None:
        for group in opt.param_groups:
            group["lr"] = lr


def pending_kind(globals_row) -> str:
    """The pending decision's kind from an encoded globals row (its pending one-hot)."""
    from bolt.play_encoding import GLOBAL_NAMES, PENDING

    at = GLOBAL_NAMES.index(f"pending_{PENDING[0]}")
    return PENDING[int(np.argmax(globals_row[at : at + len(PENDING)]))]


def by_kind_row(kinds: dict) -> dict:
    """Per pending kind: its share of the updated decisions, mean entropy, normalized entropy and |ratio - 1|."""
    total = sum(b[0] for b in kinds.values()) or 1
    return {k: {"share": round(b[0] / total, 4), "entropy": round(b[1] / b[0], 4),
                "entropy_norm": round(b[2] / b[0], 4), "ratio_dev": round(b[3] / b[0], 4)}
            for k, b in sorted(kinds.items(), key=lambda kv: -kv[1][0])}  # fmt: skip


def snapshot(net, opt, state):
    return {
        "model": net.state_dict(),
        "opt": opt.state_dict(),
        "state": state,
        "card_names": net.config.get("card_names"),
    }


def save_atomic(obj, path: Path):
    tmp = path.with_suffix(".tmp")
    torch.save(obj, tmp)
    tmp.replace(path)


def evaluate(net, pairs, workers, smart_ms=None, stop=None):
    from eval_play import match, summary

    jobs = eval_jobs(deck_sets()[1], pairs)
    net.eval()
    out = {}
    for opp in ("rules", "smart"):
        t = time.time()
        smart = (1, 2) if smart_ms is None else (1, 2, smart_ms)
        out[opp] = summary(match(net, "net", opp, jobs, workers, smart=smart, stop=stop))["win_rate"]
        out[f"{opp}_s"] = round(time.time() - t)  # wall seconds for its 2 x pairs games
    return out


def parser() -> argparse.ArgumentParser:
    """The trainer's flags; tests/test_job_files.py parses every jobs/*.json command line with it."""
    ap = argparse.ArgumentParser()
    ap.add_argument("--name", default="ppo1")
    ap.add_argument("--init", default="checkpoints/play/distill2/step_17353.pt")
    ap.add_argument("--ref", default=None,
                    help="the KL leash's reference net (default --init); kept in the run's state, so a resume without "
                         "--ref keeps the one it had")  # fmt: skip
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
    ap.add_argument(
        "--smart-ms",
        type=int,
        default=200,
        help="cap on every smart-player move in training games (0 = none): uncapped, one move can take minutes",
    )
    ap.add_argument("--eval-smart-ms", type=int, default=None, help="the same cap in evaluations (default --smart-ms)")
    ap.add_argument(
        "--max-objects",
        type=int,
        default=256,
        help="drop positions with more objects than this from the update, whole (0 keeps all)",
    )
    ap.add_argument("--resume", action="store_true")
    ap.add_argument(
        "--self-play",
        action="store_true",
        help="train only against itself and its past checkpoints (no rules or smart opponents)",
    )
    return ap


def main():
    a = parser().parse_args()
    if a.eval_smart_ms is None:
        a.eval_smart_ms = a.smart_ms or None
    if os.environ.get("BOLT_THREADS"):  # farm.py --job-threads (the 12-thread nights)
        a.workers = a.threads = int(os.environ["BOLT_THREADS"])
    torch.set_num_threads(a.threads)
    ck = ROOT / "checkpoints" / "play" / a.name
    ck.mkdir(parents=True, exist_ok=True)
    results = ROOT / "results" / f"ppo_play_{a.name}.json"
    import bolt.play_runner as play_runner

    if not play_runner.HANG_LOG:  # games whose engine call never returned: one JSON line each, for lane S
        play_runner.HANG_LOG = str(ROOT / "results" / f"hangs_{a.name}.jsonl")
    net = load_net(ROOT / a.init)
    opt = torch.optim.AdamW(net.parameters(), lr=a.lr, weight_decay=0.0)
    state = {"iter": 0, "history": [], "evals": [], "past": [], "best": None}
    if a.resume and (ck / "state.pt").exists():
        s = torch.load(ck / "state.pt", map_location="cpu", weights_only=False)
        resume_into(net, opt, s, lr=a.lr)
        state = s["state"]
        print(f"resumed at iteration {state['iter']}", flush=True)
    # On the farm (BOLT_JOB set by farm.py): power-cut-safe checkpoints on S3 (bolt.jobs), resumed first.
    job = None
    if os.environ.get("BOLT_JOB"):
        from bolt.jobs import Job

        job = Job(os.environ["BOLT_JOB"])
        s = job.resume()
        if s is not None:
            resume_into(net, opt, s, lr=a.lr)
            state = s["state"]
            for f in state["past"]:  # a fresh disk: fetch the past opponents too
                if not (ck / f).exists():
                    data = job._get(f"{job.prefix}/past/{f}")
                    if data is not None:
                        (ck / f).write_bytes(data)
            state["past"] = [f for f in state["past"] if (ck / f).exists()]
            print(f"resumed from S3 at iteration {state['iter']}", flush=True)
    decks = deck_sets()[0]
    # The KL leash's reference: --ref, else the one this run had (resume), else --init; loaded on today's pool
    state["ref"] = a.ref or state.get("ref") or a.init
    ref = load_net(ROOT / state["ref"]).eval()
    for p in ref.parameters():
        p.requires_grad_(False)
    print(f"KL reference: {state['ref']}", flush=True)

    def stopping() -> bool:
        return job is not None and job.stop_requested

    def stop_now() -> None:
        """SIGTERM: put the last finished iteration (state.pt, written before its line was printed) on S3, exit."""
        if (ck / "state.pt").exists():
            s = torch.load(ck / "state.pt", map_location="cpu", weights_only=False)
            job.save(s, step=s["state"]["iter"])
            print(f"stopped at iteration {s['state']['iter']} (SIGTERM); checkpoint saved", flush=True)
        else:
            print("stopped before the first iteration finished (SIGTERM)", flush=True)

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
        if stopping():  # a SIGTERM between iterations: the finished one is saved before anything else starts
            return stop_now()
        it = state["iter"] + 1
        t0 = time.time()
        rng = random.Random(it)
        kl_coef = a.kl + (a.kl_final - a.kl) * min(1.0, (it - 1) / max(1, a.kl_iters))
        net.eval()
        parts, won, ends = [], {}, {"error": 0, "timeout": 0}
        dropped_error = 0  # decisions of games an engine error ended, dropped before GAE (run_games)
        for k, (seats, n) in enumerate(league(a.games, state["past"], self_only=a.self_play)):
            jobs = []
            for i in range(n):
                d1, d2 = rng.sample(range(len(decks)), 2)
                jobs.append((decks[d1], decks[d2], 20_000_000 + it * 10_000 + k * 1_000 + i, i % 2))
            opp = load_net(ck / rng.choice(state["past"])) if "opp" in seats else None
            record = tuple(i for i, s in enumerate(seats) if s == "net")
            # Commander games vary a lot in length: small chunks (4 a worker) keep every worker busy to the end
            chunk = max(1, -(-len(jobs) // (4 * a.workers))) if COMMANDER else 20
            smart = (1, 2, a.smart_ms) if a.smart_ms else (1, 2)
            try:
                r = run_parallel(net, jobs, workers=a.workers, chunk=chunk, seats=seats, sample=True,
                                 seed=it * 100 + k, smart=smart, record=record, opp_net=opp, compact=True,
                                 stop=stopping)  # fmt: skip
            except Stopped:  # SIGTERM mid-play: this iteration is dropped, the last finished one saved
                return stop_now()
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
                if x["result"] == "error":
                    dropped_error += x.get("dropped_steps", 0)
            beat["t"] = time.time()
        arr = concat(parts)
        n_all = len(arr["action"])
        # Positions over --max-objects are dropped whole, never truncated (2026-11-27: Scute Swarm boards of
        # 213-559 permanents made the update grow 185 -> 344 s). Advantages were computed per game beforehand.
        keep = within(arr, a.max_objects)
        dropped_big = n_all - len(keep)
        # Rows with anything non-finite are dropped too (2026-11-27: a non-finite loss ended iteration 30)
        finite = finite_rows(arr)
        dropped_nan = int((~finite[keep]).sum())
        if dropped_nan:
            bad = keep[~finite[keep]]
            kinds_bad = collections.Counter(pending_kind(arr["globals"][i]) for i in bad)
            print(f"NAN: {dropped_nan} decisions with non-finite inputs dropped; kinds {dict(kinds_bad)}, results "
                  f"{dict(collections.Counter(int(arr['z'][i]) for i in bad))}", flush=True)  # fmt: skip
            keep = keep[finite[keep]]
        n = len(keep)
        adv = arr["adv"]
        if n > 1:
            adv = (adv - adv[keep].mean()) / (adv[keep].std() + 1e-6)
        adv = torch.as_tensor(adv)
        ret = torch.as_tensor(arr["ret"])
        t1 = time.time()
        net.train()
        sums, steps = (
            {
                "loss": 0.0,
                "clipped": 0.0,
                "kl": 0.0,
                "value_mse": 0.0,
                "entropy": 0.0,
                "ratio_dev": 0.0,
                "value_mse_z": 0.0,
                "entropy_norm": 0.0,
                "log_legal": 0.0,
            },
            0,
        )
        kinds: dict = {}  # pending kind -> summed [decisions, entropy, normalized entropy, |ratio - 1|]
        prng = np.random.default_rng(it)
        skipped = 0
        nan_dir = ROOT / "results"

        def dump_bad(rows, x, parts, it=it, arr=arr):
            """A group whose loss wasn't finite: saved for a look, then left out of the step."""
            path = ROOT / "results" / f"nan_{a.name}_{it}_{int(rows[0])}.pt"
            torch.save({"rows": np.asarray(rows), "x": x, **parts,
                        "z": arr["z"][rows], "kinds": [pending_kind(arr["globals"][i]) for i in rows]}, path)  # fmt: skip
            print(f"NAN: a group of {len(rows)} decisions had a non-finite loss; skipped, saved to {path}", flush=True)

        for _ in range(a.epochs):
            perm = keep[prng.permutation(n)]
            for s in range(0, n - a.batch + 1, a.batch):
                if stopping():
                    return stop_now()
                idx = np.sort(perm[s : s + a.batch])
                st = ppo_step(
                    net, ref, opt, arr, idx, adv[idx], a.clip, kl_coef, a.value_coef, ret=ret[idx], on_bad=dump_bad
                )
                skipped += st["bad_groups"] + st["bad_grad"]
                if skipped > 3:
                    raise FloatingPointError(f"iteration {it}: {skipped} groups or steps non-finite; see {nan_dir}")
                for key in sums:
                    sums[key] += st[key]
                for kind, b in st["by_kind"].items():
                    kinds[kind] = [x + y for x, y in zip(kinds.get(kind, [0, 0.0, 0.0, 0.0]), b, strict=True)]
                steps += 1
                beat["t"] = time.time()
        state["iter"] = it
        torch.save({"config": net.config, "model": net.state_dict()}, ck / f"iter_{it}.pt")
        upload_quietly(ck / f"iter_{it}.pt")
        if it % a.past_every == 0:
            state["past"].append(f"iter_{it}.pt")
            if job:
                job._put(f"{job.prefix}/past/iter_{it}.pt", (ck / f"iter_{it}.pt").read_bytes())
        row = {"iter": it, "games": a.games, "decisions": n, "dropped_big": dropped_big, "dropped_nan": dropped_nan,
               "dropped_error": dropped_error,
               "skipped_nan": skipped, "kl_coef": round(kl_coef, 4), "errors": ends["error"],
               "timeouts": ends["timeout"],
               **{f"won_{k}": round(float(np.mean(v)), 3) for k, v in won.items()},
               **{k: round(v / max(1, steps), 4) for k, v in sums.items()},
               "by_kind": by_kind_row(kinds),
               "play_s": round(t1 - t0), "update_s": round(time.time() - t1), "time": time.strftime("%H:%M:%S")}  # fmt: skip
        state["history"].append(row)
        # Saved before its line is printed, so a printed iteration is a saved one (2026-11-27: a SIGTERM 4 s
        # after iteration 8's line resumed the farm at iteration 7).
        save_atomic(snapshot(net, opt, state), ck / "state.pt")
        write_results()
        if job and job.due():
            job.save(snapshot(net, opt, state), step=it)
        print(json.dumps(row), flush=True)
        if stopping():
            return stop_now()
        if it % a.eval_every == 0:
            try:
                e = evaluate(net, a.eval_pairs, a.workers, a.eval_smart_ms, stop=stopping)
            except Stopped:
                return stop_now()
            state["evals"].append({"iter": it, **e})
            score = (e["rules"] + e["smart"]) / 2
            if state["best"] is None or score > state["best"]["score"]:
                state["best"] = {"iter": it, "score": score}
                torch.save({"config": net.config, "model": net.state_dict()}, ck / "best.pt")
                upload_quietly(ck / "best.pt")
            print(json.dumps({"eval": it, **e, "best_iter": state["best"]["iter"]}), flush=True)
            beat["t"] = time.time()
            save_atomic(snapshot(net, opt, state), ck / "state.pt")  # with its eval
            write_results()
            if job and job.due():
                job.save(snapshot(net, opt, state), step=it)
    if job:
        job.save(snapshot(net, opt, state), step=state["iter"], done=True)


if __name__ == "__main__":
    main()
