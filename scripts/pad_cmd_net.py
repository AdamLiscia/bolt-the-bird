"""The Commander play network's start: a trained Tarkir (TDM) play network grown to the whole CMD card pool.

    .venv/bin/python scripts/pad_cmd_net.py --from checkpoints/play/tdm_play_1/iter_230.pt \
        --out checkpoints/play/cmd_init_from_tdm.pt

On the desktop, after fetching s3://.../farm/tdm_play_1/past/iter_230.pt (docs/desktop-farm.md, "Before
jobs/cmd_play_1.json starts"). --from takes any play checkpoint holding {"config", "model"}: a per-iteration
iter_N.pt, best.pt, or an init net such as tdm_init_from_v3.pt (the local stand-in).

Everything the network learned carries over. The card table is rebuilt for CMD's numbers: a CMD card whose
name is also a TDM card keeps its trained row (matched by name, the first TDM card of that name); every
other card starts at the mean of the trained rows plus a little noise, so an unseen card reads as "a card"
and its printed facts (bolt.play_encoding.STATIC) tell them apart. Row 0 (padding) and the last row (a card
the player can't see) are copied. The new inputs (command zones, Commander numbers, printed facts) are
zero-padded by `pad_state`, so on a TDM position the result plays exactly as the TDM network did.

The vocabulary is all of CMD, not just today's playable decks, so decks that become playable later need no
re-padding.
"""

import argparse
import sys
from pathlib import Path

import bolt_engine as be
import torch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from bolt.play_net import PlayNet, load_checkpoint  # noqa: E402


def pad_cards(old: PlayNet, old_names: list[str], names: list[str], noise: float = 0.05, seed: int = 0):
    """(a PlayNet over `names`, rows carried over by name). `noise` is relative to the trained rows' std."""
    trained = old.config["n_cards"]
    config = {**old.config, "n_cards": len(names)}
    new = PlayNet(**config)
    state = {k: v.clone() for k, v in old.state_dict().items()}
    w_old = state["card.weight"]
    rows = w_old[1 : trained + 1]
    g = torch.Generator().manual_seed(seed)
    w = rows.mean(0) + torch.randn(len(names) + 2, w_old.shape[1], generator=g) * rows.std() * noise
    w[0] = w_old[0]
    w[-1] = w_old[-1]
    index: dict[str, int] = {}
    for i, n in enumerate(old_names[:trained]):
        index.setdefault(n, i)
    kept = 0
    for i, n in enumerate(names):
        if n in index:
            w[i + 1] = w_old[index[n] + 1]
            kept += 1
    state["card.weight"] = w
    new.load_state_dict(state)
    return new, kept


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--from", dest="src", required=True, help="the TDM play checkpoint to start from")
    ap.add_argument("--from-set", default="TDM")
    ap.add_argument("--set", default="CMD")
    ap.add_argument("--out", default="checkpoints/play/cmd_init_from_tdm.pt")
    ap.add_argument("--noise", type=float, default=0.05)
    ap.add_argument("--seed", type=int, default=0)
    a = ap.parse_args()
    old = load_checkpoint(a.src)  # grown to today's inputs first
    old_names, names = be.card_names(a.from_set), be.card_names(a.set)
    if old.config["n_cards"] != len(old_names):
        print(f"note: {a.src} has {old.config['n_cards']} card rows, {a.from_set} now has {len(old_names)}")
    new, kept = pad_cards(old, old_names, names, a.noise, a.seed)
    out = Path(a.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    tmp = out.with_suffix(".tmp")
    torch.save({"config": new.config, "model": new.state_dict(), "from": a.src, "set": a.set.upper()}, tmp)
    tmp.replace(out)
    print(f"{a.set}: {len(names)} cards ({kept} rows carried over by name from {a.from_set}) -> {out}")


if __name__ == "__main__":
    main()
