"""Train the judge on real games and check that it actually learned something about decks.

    python scripts/train_judge.py

Three models are compared on held-out games:
  skill-only  knows the player's skill and play/draw, nothing about the deck
  linear      + one weight per card
  mlp         + cards interacting
If the deck models don't beat skill-only, the judge has learned nothing about decks.
"""
import json
import sys
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as F

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))
from bolt.data import DATA, load_cards, load_games, subset  # noqa: E402
from bolt.judge import JUDGES, SKILL_REF  # noqa: E402


def auc(y, s):
    order = np.argsort(s)
    ranks = np.empty(len(s))
    ranks[order] = np.arange(1, len(s) + 1)
    pos = y.astype(bool)
    return (ranks[pos].sum() - pos.sum() * (pos.sum() + 1) / 2) / (pos.sum() * (~pos).sum())


def tensors(g, device):
    return (torch.from_numpy(g["deck"].astype(np.float32)).to(device),
            torch.from_numpy(np.where(g["user_wr"] < 0, SKILL_REF, g["user_wr"]).astype(np.float32)).to(device),
            torch.from_numpy(g["on_play"].astype(np.float32)).to(device),
            torch.from_numpy(g["won"].astype(np.float32)).to(device))


def fit(model, train, val, device, epochs, lr, wd, bs=4096):
    opt = torch.optim.AdamW(model.parameters(), lr=lr, weight_decay=wd)
    n = len(train[0])
    best, best_state = 9.0, None
    for ep in range(epochs):
        model.train()
        perm = torch.randperm(n, device=device)
        for i in range(0, n, bs):
            idx = perm[i:i + bs]
            loss = F.binary_cross_entropy_with_logits(model(train[0][idx], train[1][idx], train[2][idx]), train[3][idx])
            opt.zero_grad(set_to_none=True)
            loss.backward()
            opt.step()
        vl = evaluate(model, val)["log_loss"]
        if vl < best:
            best, best_state = vl, {k: v.clone() for k, v in model.state_dict().items()}
        print(f"    epoch {ep + 1}: val log loss {vl:.4f}", flush=True)
    model.load_state_dict(best_state)
    return model


@torch.no_grad()
def evaluate(model, data):
    model.eval()
    logits = torch.cat([model(data[0][i:i + 65536], data[1][i:i + 65536], data[2][i:i + 65536])
                        for i in range(0, len(data[0]), 65536)])
    y = data[3]
    p = torch.sigmoid(logits).cpu().numpy()
    return {"log_loss": float(F.binary_cross_entropy_with_logits(logits, y)), "auc": float(auc(y.cpu().numpy(), p))}


class SkillOnly(torch.nn.Module):
    def __init__(self):
        super().__init__()
        self.lin = torch.nn.Linear(2, 1)

    def forward(self, deck, skill, on_play):
        return self.lin(torch.stack([skill - SKILL_REF, on_play - .5], 1)).squeeze(1)


def main():
    import argparse
    ap = argparse.ArgumentParser()
    ap.add_argument("--half", type=int, choices=[0, 1], help="train the MLP judge on one half of the training drafts only")
    a = ap.parse_args()
    device = torch.device("mps" if torch.backends.mps.is_available() else "cpu")
    torch.manual_seed(0)
    cards = load_cards()
    g = load_games()
    g["deck"] = g["deck"][:, :len(cards)]
    train_mask = g["split"] == "train"
    if a.half is not None:
        train_mask &= g["half"] == a.half
    tr = tensors(subset(g, train_mask), device)
    va, te = (tensors(subset(g, g["split"] == s), device) for s in ("val", "test"))
    print(f"games: train {len(tr[0]):,}  val {len(va[0]):,}  test {len(te[0]):,}   base win rate {float(tr[3].mean()):.1%}")

    results = {}
    runs = [
        ("skill-only", SkillOnly(), 4, 1e-2, 0.0),
        ("linear", JUDGES["linear"](len(cards)), 8, 3e-3, 1e-2),
        ("mlp", JUDGES["mlp"](len(cards)), 30, 2e-3, 1e-2),
    ]
    if a.half is not None:
        runs = runs[-1:]
    tag = "" if a.half is None else f"_h{a.half}"
    for name, model, epochs, lr, wd in runs:
        print(f"{name}:")
        model = fit(model.to(device), tr, va, device, epochs, lr, wd)
        results[name] = evaluate(model, te)
        print(f"  TEST log loss {results[name]['log_loss']:.4f}  AUC {results[name]['auc']:.4f}")
        if name == "mlp":
            ck = DATA.parent / "checkpoints"
            ck.mkdir(exist_ok=True)
            torch.save(model.state_dict(), ck / f"judge{tag}.pt")
            # Sanity check: judged win rate of the real decks, by actual result
            wr = model.win_rate(te[0]).cpu().numpy()
            won = te[3].cpu().numpy().astype(bool)
            q = np.quantile(wr, [0, .2, .4, .6, .8, 1])
            buckets = np.clip(np.searchsorted(q, wr, side="right") - 1, 0, 4)
            results["calibration_by_judged_quintile"] = [
                {"judged": float(wr[buckets == b].mean()), "actual": float(won[buckets == b].mean()), "games": int((buckets == b).sum())}
                for b in range(5)]
            for row in results["calibration_by_judged_quintile"]:
                print(f"    decks the judge rates {row['judged']:.1%} actually won {row['actual']:.1%}  ({row['games']:,} games)")
        if name == "linear":
            w = model.card.weight.detach().cpu().numpy()[0]
            top = np.argsort(-w)[:10]
            results["linear_top_cards"] = [cards[i] for i in top]
            print("  cards the linear judge values most:", ", ".join(cards[i] for i in top))

    res = DATA.parent / "results"
    res.mkdir(exist_ok=True)
    (res / f"judge{tag}.json").write_text(json.dumps(results, indent=1))


if __name__ == "__main__":
    main()
