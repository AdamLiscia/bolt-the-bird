"""P3c: train the play network to copy the smart player (behavior cloning), plus the value head.

    python scripts/train_distill.py --name distill1                 # fresh
    python scripts/train_distill.py --name distill1 --resume        # after a crash

Loss: cross-entropy on the smart player's move + value_weight × MSE(value, game result from the
decider's side). Shards stream one at a time (data/distill); the last --valid shards are held out.
Every --every steps: a checkpoint (all kept, checkpoints/play/<name>/step_N.pt) and a validation
pass (move agreement, losses); best.pt is the lowest validation loss. Non-finite loss or gradient
stops the run. results/play_<name>.json holds the history.
"""

import argparse
import json
import os
import sys
import threading
import time
from pathlib import Path

# A hard ceiling on graphics memory (a fraction of what macOS recommends for the GPU, ~3 GB on the
# 16 GB laptop): past it, allocation fails with an error instead of pushing the Mac into swap.
# 2026-11-18: training's graphics memory, invisible to guard.sh, swapped the laptop until it lagged.
os.environ.setdefault("PYTORCH_MPS_HIGH_WATERMARK_RATIO", "0.3")
os.environ.setdefault("PYTORCH_MPS_LOW_WATERMARK_RATIO", "0.2")  # must sit below the high one

import numpy as np  # noqa: E402
import torch  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

import bolt_engine as be  # noqa: E402

from bolt.play_data import batch, concat, groups, load  # noqa: E402
from bolt.play_net import PlayNet  # noqa: E402


def losses(net, arr, idx, device, value_weight):
    """Mean over the batch of (total, policy CE, value MSE, agreement), the same as one big batch."""
    n = len(idx)
    tot = ce_t = mse_t = agree_t = 0.0
    for g in groups(arr, idx):
        x, action, z = batch(arr, g, device)
        logits, value = net(x)
        ce = torch.nn.functional.cross_entropy(logits, action, reduction="sum")
        mse = torch.nn.functional.mse_loss(value, z, reduction="sum")
        tot = tot + (ce + value_weight * mse) / n
        ce_t, mse_t = ce_t + ce.detach() / n, mse_t + mse.detach() / n
        agree_t = agree_t + (logits.argmax(1) == action).float().sum().detach() / n
    return tot, ce_t, mse_t, agree_t


@torch.no_grad()
def gpu_mb(device) -> float:
    return torch.mps.driver_allocated_memory() / 2**20 if device.type == "mps" else 0.0


def release(device) -> None:
    if device.type == "mps":
        torch.mps.empty_cache()


def evaluate(net, arr, device, value_weight, bs=128):
    net.eval()
    tot = np.zeros(4)
    n = len(arr["action"])
    for s in range(0, n, bs):
        idx = np.arange(s, min(n, s + bs))
        tot += np.array([float(v) for v in losses(net, arr, idx, device, value_weight)]) * len(idx)
    net.train()
    release(device)  # grading batches differ in shape from training ones; hand their memory back
    loss, ce, mse, agree = tot / n
    return {"loss": loss, "policy_ce": ce, "value_mse": mse, "agreement": agree}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--name", default="distill1")
    ap.add_argument("--data", nargs="+", default=["data/distill"], help="one or more shard folders")
    ap.add_argument("--init", default=None, help="start from these weights (a best.pt) instead of scratch")
    ap.add_argument("--valid", type=int, default=2)
    ap.add_argument("--valid-rows", type=int, default=40_000)
    ap.add_argument("--epochs", type=int, default=3)
    ap.add_argument("--batch", type=int, default=256)
    ap.add_argument("--lr", type=float, default=3e-4)
    ap.add_argument("--dim", type=int, default=128)
    ap.add_argument("--layers", type=int, default=4)
    ap.add_argument("--value-weight", type=float, default=0.25)
    ap.add_argument("--every", type=int, default=2000)
    ap.add_argument("--resume", action="store_true")
    ap.add_argument("--device", default="cpu", help="cpu (guard.sh sees all its memory), mps or auto")
    ap.add_argument("--threads", type=int, default=4, help="CPU threads; leaves the other cores free")
    ap.add_argument("--max-gpu-mb", type=float, default=3000, help="stop cleanly above this much graphics memory")
    ap.add_argument("--stall-minutes", type=float, default=10, help="exit if no training step completes for this long")
    ap.add_argument("--max-steps", type=int, default=None, help="stop after this many steps (tests)")
    a = ap.parse_args()
    torch.set_num_threads(a.threads)
    device = torch.device(("mps" if torch.backends.mps.is_available() else "cpu") if a.device == "auto" else a.device)
    train, valid_paths = [], []
    for d in a.data:  # the last --valid shards of each folder are held out
        shards = sorted((ROOT / d).glob("shard_*.npz"))
        train += shards[: -a.valid]
        valid_paths += shards[-a.valid :]
    valid = concat([load(p) for p in valid_paths])
    keep = np.random.default_rng(0).permutation(len(valid["action"]))[: a.valid_rows]
    valid = subset(valid, np.sort(keep))
    torch.manual_seed(0)
    net = PlayNet(len(be.card_names()), dim=a.dim, layers=a.layers).to(device)
    ck_dir = ROOT / "checkpoints" / "play" / a.name
    resuming = a.resume and any(ck_dir.glob("step_*.pt"))
    if a.init and not resuming:  # --resume with nothing saved yet still starts from --init
        net.load_state_dict(torch.load(a.init, map_location=device, weights_only=False)["model"])
        print(f"starting from {a.init}", flush=True)
    opt = torch.optim.AdamW(net.parameters(), lr=a.lr, weight_decay=0.01)
    total = a.epochs * sum(int(np.load(p)["action"].shape[0]) for p in train) // a.batch
    sched = torch.optim.lr_scheduler.LambdaLR(opt, lambda s: min(1.0, s / 1000) * max(0.05, 1 - s / max(1, total)))
    ck = ROOT / "checkpoints" / "play" / a.name
    ck.mkdir(parents=True, exist_ok=True)
    results = ROOT / "results" / f"play_{a.name}.json"
    state = {"epoch": 0, "shard_pos": 0, "batch_pos": 0, "step": 0, "history": [], "best": None, "order": None}
    if a.resume:
        saved = sorted(ck.glob("step_*.pt"), key=lambda p: int(p.stem.split("_")[1]))
        if saved:
            c = torch.load(saved[-1], map_location=device, weights_only=False)
            net.load_state_dict(c["model"])
            opt.load_state_dict(c["opt"])
            sched.load_state_dict(c["sched"])
            state = c["state"]
            print(f"resumed from {saved[-1].name}", flush=True)
    print(f"device {device}; {len(train)} train shards, {len(valid['action'])} validation rows; ~{total} steps",
          flush=True)  # fmt: skip

    def checkpoint():
        v = evaluate(net, valid, device, a.value_weight)
        v.update(step=state["step"], epoch=state["epoch"], time=time.strftime("%H:%M:%S"))
        state["history"].append(v)
        if state["best"] is None or v["loss"] < state["best"]:
            state["best"] = v["loss"]
            torch.save({"config": net.config, "model": net.state_dict()}, ck / "best.pt")
        torch.save({"model": net.state_dict(), "opt": opt.state_dict(), "sched": sched.state_dict(), "state": state,
                    "config": net.config}, ck / f"step_{state['step']}.pt")  # fmt: skip
        results.write_text(json.dumps({"args": vars(a), "history": state["history"]}, indent=1))
        print(json.dumps({k: round(x, 4) if isinstance(x, float) else x for k, x in v.items()}), flush=True)

    beat = {"t": time.time()}

    def watchdog():  # a stalled run (swapping, hung) ends itself rather than lingering
        while True:
            time.sleep(30)
            if time.time() - beat["t"] > a.stall_minutes * 60:
                print(f"STALL: no training step for {a.stall_minutes} minutes at step {state['step']}; exiting",
                      flush=True)  # fmt: skip
                os._exit(3)

    threading.Thread(target=watchdog, daemon=True).start()
    t_last = time.time()
    net.train()
    while state["epoch"] < a.epochs:
        if state["order"] is None:
            state["order"] = np.random.default_rng(state["epoch"]).permutation(len(train)).tolist()
        while state["shard_pos"] < len(train):
            arr = load(train[state["order"][state["shard_pos"]]])
            perm = np.random.default_rng(state["epoch"] * 1000 + state["shard_pos"]).permutation(len(arr["action"]))
            for s in range(state.get("batch_pos", 0), len(perm) - a.batch + 1, a.batch):
                loss, *_ = losses(net, arr, np.sort(perm[s : s + a.batch]), device, a.value_weight)
                if not torch.isfinite(loss):
                    raise SystemExit(f"non-finite loss at step {state['step']}: stopping")
                opt.zero_grad()
                loss.backward()
                gnorm = torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
                if not torch.isfinite(gnorm):
                    raise SystemExit(f"non-finite gradient at step {state['step']}: stopping")
                opt.step()
                sched.step()
                state["step"] += 1
                state["batch_pos"] = s + a.batch  # a resume continues inside the shard, not from its start
                beat["t"] = time.time()
                if state["step"] % 200 == 0:
                    mb = gpu_mb(device)
                    rate = 200 / (time.time() - t_last)
                    t_last = time.time()
                    print(json.dumps({"progress": state["step"], "steps_per_s": round(rate, 2), "gpu_mb": round(mb)}),
                          flush=True)  # fmt: skip
                    if mb > a.max_gpu_mb:
                        raise SystemExit(f"graphics memory {mb:.0f} MB over {a.max_gpu_mb:.0f} at step "
                                         f"{state['step']}: stopping (resume from the last checkpoint)")  # fmt: skip
                if state["step"] % a.every == 0:
                    checkpoint()
                    beat["t"] = time.time()
                if a.max_steps and state["step"] >= a.max_steps:
                    return
            state["shard_pos"] += 1
            state["batch_pos"] = 0
        state["epoch"] += 1
        state["shard_pos"] = 0
        state["order"] = None
    checkpoint()


def subset(arr, idx):
    """The decisions at idx as their own arrays."""
    oo, ao = arr["obj_off"], arr["act_off"]
    objs = np.concatenate([np.arange(oo[i], oo[i + 1]) for i in idx])
    acts = np.concatenate([np.arange(ao[i], ao[i + 1]) for i in idx])
    out = {k: arr[k][objs] for k in ("card", "zone", "feats")}
    out.update({k: arr[k][acts] for k in ("act_kind", "act_ptr", "act_card", "act_num", "act_player")})
    out.update({k: arr[k][idx] for k in ("globals", "action", "seat", "game", "z")})
    out["obj_off"] = np.concatenate([[0], np.cumsum(oo[idx + 1] - oo[idx])]).astype(np.int64)
    out["act_off"] = np.concatenate([[0], np.cumsum(ao[idx + 1] - ao[idx])]).astype(np.int64)
    return out


if __name__ == "__main__":
    main()
