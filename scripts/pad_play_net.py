"""A play network for another card set, started from a trained FDN one (the Tarkir play job's start).

    python scripts/pad_play_net.py --ckpt checkpoints/play/play_v3_ppo4_i170.pt --set TDM \
        --out checkpoints/play/tdm_init_from_v3.pt

Everything the network learned carries over except the card table, which is per set (card numbers differ):
a card or token whose name is also in FDN (the basics, shared tokens) keeps its trained row; every other card
gets a new row with the same scale as the trained ones. Row 0 (padding) and the last row (a card the player
can't see) are copied. New action kinds are padded by `load_checkpoint`.
"""

import argparse
import sys
from pathlib import Path

import bolt_engine as be
import torch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from bolt.play_net import PlayNet, load_checkpoint  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ckpt", required=True)
    ap.add_argument("--set", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--seed", type=int, default=0)
    a = ap.parse_args()
    torch.manual_seed(a.seed)
    old = load_checkpoint(a.ckpt)
    fdn_names = be.card_names()
    names = be.card_names(a.set)
    config = {**old.config, "n_cards": len(names)}
    new = PlayNet(**config)
    state = old.state_dict()
    new_state = new.state_dict()
    for k, v in state.items():
        if k != "card.weight":
            new_state[k] = v.clone()
    w_old = state["card.weight"]
    scale = w_old[1:-1].std().item()
    w = torch.randn(len(names) + 2, w_old.shape[1]) * scale
    w[0] = w_old[0]
    w[-1] = w_old[-1]
    index = {n: i for i, n in enumerate(fdn_names)}  # the first FDN card of that name
    kept = 0
    trained = old.config["n_cards"]  # tokens added to FDN after training have no trained row
    for i, n in enumerate(names):
        if n in index and index[n] < trained:
            w[i + 1] = w_old[index[n] + 1]
            kept += 1
    new_state["card.weight"] = w
    new.load_state_dict(new_state)
    torch.save({"config": config, "model": new.state_dict(), "from": a.ckpt, "set": a.set.upper()}, a.out)
    print(f"{a.set}: {len(names)} cards ({kept} rows carried over by name) -> {a.out}")


if __name__ == "__main__":
    main()
