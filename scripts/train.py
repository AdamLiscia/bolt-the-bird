"""Train a draft model on human picks; report test accuracy in the same format as the baselines.

    python scripts/train.py --model mlp
    python scripts/train.py --model transformer --epochs 3
"""
import argparse
import json
import sys
import time
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as F

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))
from bolt.data import DATA, load_cards, load_picks, subset  # noqa: E402
from bolt.evaluate import report  # noqa: E402
from bolt.models import MODELS  # noqa: E402

FIELDS = ["pool", "pack", "pack_number", "pick_number", "pick"]


def tensors(d: dict) -> dict:
    out = {k: torch.from_numpy(d[k].astype(np.int64) if k == "pick" else d[k]) for k in FIELDS}
    out["pool"] = out["pool"][:, :42]  # pool before a pick never exceeds 41
    return out


def batches(t: dict, bs: int, device, shuffle: bool, rng=None):
    n = len(t["pick"])
    order = torch.from_numpy(rng.permutation(n)) if shuffle else torch.arange(n)
    for i in range(0, n, bs):
        idx = order[i:i + bs]
        yield {k: v[idx].to(device) for k, v in t.items()}


@torch.no_grad()
def predict(model, t: dict, device, bs=4096) -> np.ndarray:
    model.eval()
    out = []
    for b in batches(t, bs, device, False):
        out.append(model(b["pool"], b["pack"], b["pack_number"], b["pick_number"]).argmax(1).cpu())
    model.train()
    return torch.cat(out).numpy()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--model", choices=list(MODELS), default="mlp")
    ap.add_argument("--epochs", type=int, default=3)
    ap.add_argument("--bs", type=int, default=1024)
    ap.add_argument("--lr", type=float, default=1e-3)
    ap.add_argument("--min-wr", type=float, default=0.0, help="train only on drafters at/above this win-rate bucket")
    ap.add_argument("--limit", type=int, default=0, help="train on the first N picks only (smoke test)")
    a = ap.parse_args()

    device = torch.device("mps" if torch.backends.mps.is_available() else "cpu")
    torch.manual_seed(0)
    rng = np.random.default_rng(0)
    cards = load_cards()
    picks = load_picks()
    train = subset(picks, (picks["split"] == "train") & (picks["user_wr"] >= a.min_wr))
    if a.limit:
        train = subset(train, np.arange(len(train["pick"])) < a.limit)
    val, test = subset(picks, picks["split"] == "val"), subset(picks, picks["split"] == "test")
    tt, tv, tte = tensors(train), tensors(val), tensors(test)
    print(f"{a.model} on {device}: train {len(train['pick']):,}  val {len(val['pick']):,}  test {len(test['pick']):,}")

    model = MODELS[a.model](len(cards)).to(device)
    print(f"params {sum(p.numel() for p in model.parameters()):,}")
    opt = torch.optim.AdamW(model.parameters(), lr=a.lr, weight_decay=0.01)
    steps = a.epochs * ((len(train["pick"]) + a.bs - 1) // a.bs)
    sched = torch.optim.lr_scheduler.OneCycleLR(opt, a.lr, total_steps=steps, pct_start=0.05)

    history, step, t0 = [], 0, time.time()
    for ep in range(a.epochs):
        run = 0.0
        for b in batches(tt, a.bs, device, True, rng):
            loss = F.cross_entropy(model(b["pool"], b["pack"], b["pack_number"], b["pick_number"]), b["pick"])
            opt.zero_grad(set_to_none=True)
            loss.backward()
            opt.step()
            sched.step()
            step += 1
            run += loss.item() if step % 50 == 0 else 0
            if step % 1000 == 0:
                print(f"  step {step}/{steps}  loss {run / 20:.3f}  {time.time() - t0:.0f}s", flush=True)
                run = 0.0
        val_acc = float((predict(model, tv, device) == val["pick"]).mean())
        history.append({"epoch": ep + 1, "val_acc": val_acc, "seconds": round(time.time() - t0)})
        print(f"epoch {ep + 1}: val acc {val_acc:.1%}", flush=True)

    r = report(a.model, predict(model, tte, device), test)
    r.update(history=history, args=vars(a))
    out = DATA.parent / "results"
    out.mkdir(exist_ok=True)
    (out / f"{a.model}.json").write_text(json.dumps(r, indent=1))
    ck = DATA.parent / "checkpoints"
    ck.mkdir(exist_ok=True)
    torch.save(model.state_dict(), ck / f"{a.model}.pt")


if __name__ == "__main__":
    main()
