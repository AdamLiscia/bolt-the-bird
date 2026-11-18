"""Train a turn model (Phase 4, P1) on the parsed replays: cast, attack or block.

    python scripts/train_turns.py --task cast --name cast1                  # all shards, 2 epochs
    python scripts/train_turns.py --task cast --name trial --shards 1 --epochs 1
    python scripts/train_turns.py --task cast --name cast1 --resume         # after a crash

Safeguards, so a crash costs minutes, not the run:
- every --save-every steps and at the end of every epoch, a checkpoint with the model, the
  optimizer and the position in the data (checkpoints/turns/<name>/step_<N>.pt); all are kept;
- --resume continues from the newest one, at the same epoch and shard;
- a non-finite loss or gradient stops the run with the last good checkpoint on disk;
- the validation history goes to results/turns_<name>.json at every checkpoint;
- run it through scripts/guard.sh for a time and memory cap.
At the end: held-out test metrics on every shard's test games, next to the baselines.
"""

import argparse
import json
import random
import time

import bolt_engine as be
import numpy as np
import torch

from bolt.data import ROOT
from bolt.turn_model import TurnModel, slot_loss
from bolt.turns import CardStats, baseline_cast, mark, metrics, split_of

REPLAYS = ROOT / "data" / "replays"
BOARD = ("hand", "my_lands", "my_creatures", "my_others", "opp_lands", "opp_creatures", "opp_others")
TASKS = {  # shard kind, zones the model sees, target zone, chosen cards
    "cast": ("user", BOARD, "hand", ("cast", "lands_played")),
    "attack": ("user", BOARD, "my_creatures", ("attacked",)),
    "block": ("block", (*BOARD, "attackers"), "my_creatures", ("blockers",)),
}
torch.backends.mha.set_fastpath_enabled(False)  # the fused path hangs on MPS (see train_rl.py)


def load(task: str, shard: int, split: str) -> dict | None:
    kind, zones, target, chosen = TASKS[task]
    f = REPLAYS / f"{kind}_{shard:03d}.npz"
    if not f.exists():
        return None
    d = np.load(f)
    keep = split_of(d["game"]) == split
    keep &= (d[target] > 0).any(1)
    if task == "cast":
        keep &= d["consistent"]
    out = {z: d[z][keep] for z in zones}
    out["target"] = mark(out[target], np.concatenate([d[c][keep] for c in chosen], 1))
    s = np.stack([d["turn"] / 20, d["my_life"] / 20, d["opp_life"] / 20, d["opp_hand"] / 7, d["on_play"]], 1)
    out["scalars"] = s[keep].astype(np.float32)
    if task == "cast":
        out["baseline"] = None
    return out


def tensors(data: dict, idx, zones, device):
    """One batch, each zone trimmed to the widest slot actually used in the batch."""
    z = {}
    for name in zones:
        x = data[name][idx]
        used = np.flatnonzero((x > 0).any(0))
        z[name] = torch.as_tensor(x[:, : (used[-1] + 1 if len(used) else 1)], device=device)
    return z, torch.as_tensor(data["scalars"][idx], device=device), torch.as_tensor(data["target"][idx], device=device)


@torch.no_grad()
def evaluate(model, data: dict, task: str, device, batch: int = 2048) -> dict:
    _, zones, target, _ = TASKS[task]
    model.eval()
    preds, losses = [], []
    n = len(data["target"])
    for s in range(0, n, batch):
        idx = np.arange(s, min(n, s + batch))
        z, sc, t = tensors(data, idx, zones, device)
        logits = model(z, sc, target)
        losses.append(slot_loss(logits, t[:, : logits.shape[1]], z[target] > 0).item() * len(idx))
        p = np.zeros(data[target][idx].shape, dtype=bool)
        p[:, : logits.shape[1]] = (logits > 0).cpu().numpy()
        preds.append(p)
    model.train()
    m = metrics(np.concatenate(preds), data["target"], data[target])
    return m | {"loss": sum(losses) / n}


def checkpoints(name: str):
    d = ROOT / "checkpoints" / "turns" / name
    d.mkdir(parents=True, exist_ok=True)
    return d


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--task", choices=list(TASKS), required=True)
    ap.add_argument("--name", required=True)
    ap.add_argument("--epochs", type=int, default=2)
    ap.add_argument("--shards", type=int, default=0, help="use only the first N shards (0 = all)")
    ap.add_argument("--batch", type=int, default=512)
    ap.add_argument("--lr", type=float, default=3e-4)
    ap.add_argument("--dim", type=int, default=128)
    ap.add_argument("--layers", type=int, default=3)
    ap.add_argument("--save-every", type=int, default=2000)
    ap.add_argument("--resume", action="store_true")
    ap.add_argument("--device", default="auto")
    a = ap.parse_args()
    device = torch.device(("mps" if torch.backends.mps.is_available() else "cpu") if a.device == "auto" else a.device)
    _, zones, target, _ = TASKS[a.task]
    kind = TASKS[a.task][0]
    n_shards = len(list(REPLAYS.glob(f"{kind}_*.npz")))
    if a.shards:
        n_shards = min(n_shards, a.shards)
    names = be.card_names()
    model = TurnModel(len(names), dim=a.dim, layers=a.layers).to(device)
    opt = torch.optim.AdamW(model.parameters(), lr=a.lr, weight_decay=0.01)
    ck = checkpoints(a.name)
    state = {"epoch": 0, "shard_pos": 0, "step": 0, "order": None, "history": [], "best": None}
    if a.resume:
        saved = sorted(ck.glob("step_*.pt"), key=lambda p: int(p.stem.split("_")[1]))
        if saved:
            c = torch.load(saved[-1], map_location=device, weights_only=False)
            model.load_state_dict(c["model"])
            opt.load_state_dict(c["opt"])
            state = c["state"]
            print(f"resumed from {saved[-1].name}: epoch {state['epoch']}, shard {state['shard_pos']}", flush=True)
    valid = load(a.task, 0, "valid")
    results = ROOT / "results" / f"turns_{a.name}.json"

    def save(tag: str):
        v = evaluate(model, valid, a.task, device)
        state["history"].append({"step": state["step"], "epoch": state["epoch"], "valid": v, "tag": tag})
        torch.save({"model": model.state_dict(), "opt": opt.state_dict(), "state": state, "args": vars(a)},
                   ck / f"step_{state['step']}.pt")  # fmt: skip
        if state["best"] is None or v["loss"] < state["best"]:
            state["best"] = v["loss"]
            torch.save(model.state_dict(), ck / "best.pt")
        results.write_text(json.dumps({"args": vars(a), "history": state["history"]}, indent=1))
        print(f"  step {state['step']} ({tag}): valid loss {v['loss']:.4f}, slots {v['slot_accuracy']:.1%}, "
              f"turns exact {v['turn_exact']:.1%}", flush=True)  # fmt: skip

    t0 = time.time()
    while state["epoch"] < a.epochs:
        if state["order"] is None:
            state["order"] = random.Random(state["epoch"]).sample(range(n_shards), n_shards)
        while state["shard_pos"] < n_shards:
            data = load(a.task, state["order"][state["shard_pos"]], "train")
            n = len(data["target"])
            perm = np.random.default_rng(state["step"]).permutation(n)
            for s in range(0, n - a.batch + 1, a.batch):
                z, sc, t = tensors(data, perm[s : s + a.batch], zones, device)
                logits = model(z, sc, target)
                loss = slot_loss(logits, t[:, : logits.shape[1]], z[target] > 0)
                if not torch.isfinite(loss):
                    raise RuntimeError(f"non-finite loss at step {state['step']}; last good checkpoint kept")
                opt.zero_grad(set_to_none=True)
                loss.backward()
                gnorm = torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
                if not torch.isfinite(gnorm):
                    raise RuntimeError(f"non-finite gradient at step {state['step']}; last good checkpoint kept")
                opt.step()
                state["step"] += 1
                if state["step"] % 200 == 0:
                    print(f"step {state['step']} epoch {state['epoch']} shard {state['shard_pos']}: "
                          f"loss {loss.item():.4f} ({time.time() - t0:.0f}s)", flush=True)  # fmt: skip
                if state["step"] % a.save_every == 0:
                    save("periodic")
            state["shard_pos"] += 1
        state["epoch"], state["shard_pos"], state["order"] = state["epoch"] + 1, 0, None
        save(f"end of epoch {state['epoch']}")

    # Held-out test games from every shard, with the best checkpoint, next to the baselines.
    model.load_state_dict(torch.load(ck / "best.pt", map_location=device))
    tests = [d for i in range(n_shards) if (d := load(a.task, i, "test")) is not None]
    test = {k: np.concatenate([d[k] for d in tests]) for k in tests[0] if tests[0][k] is not None}
    out = {"model": evaluate(model, test, a.task, device), "rows": len(test["target"])}
    if a.task == "cast":
        st = CardStats.load(ROOT / "data" / "raw" / "cards.csv", names)
        out["baseline_land_then_priciest"] = metrics(
            baseline_cast(test["hand"], test["my_lands"], st), test["target"], test["hand"]
        )
    else:
        for label, value in (("baseline_all", True), ("baseline_none", False)):
            out[label] = metrics(np.full(test["target"].shape, value), test["target"], test[target])
    results.write_text(json.dumps({"args": vars(a), "history": state["history"], "test": out}, indent=1))
    print(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
