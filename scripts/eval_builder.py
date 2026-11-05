"""Give the deck builder the exact pools real players drafted and compare its decks with theirs.

python scripts/eval_builder.py --n 2000
"""

import argparse
import json

import numpy as np
import torch

from bolt.cards import CardInfo
from bolt.data import DATA, split_of
from bolt.deckbuild import build_decks
from bolt.judge import DeckJudge


def final_pools(n_cards):
    z = np.load(DATA / "FDN_picks.npz")
    draft, pool, pick, pk, pn = z["draft"], z["pool"], z["pick"], z["pick_number"], z["pack_number"]
    keys = z["draft_keys"]
    order = np.lexsort((pk, pn, draft))
    last = order[np.r_[np.flatnonzero(np.diff(draft[order])), len(order) - 1]]
    complete = (pn[last] == 2) & (pk[last] >= 12)
    last = last[complete]
    pools = np.zeros((len(last), n_cards), np.float32)
    for j, i in enumerate(last):
        ids = pool[i][pool[i] > 0] - 1
        np.add.at(pools[j], ids, 1)
        pools[j, pick[i]] += 1
    return {keys[draft[i]]: pools[j] for j, i in enumerate(last)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=2000)
    a = ap.parse_args()
    device = torch.device("mps" if torch.backends.mps.is_available() else "cpu")
    cards = CardInfo()
    judge = DeckJudge(cards.n).to(device)
    judge.load_state_dict(torch.load(DATA.parent / "checkpoints" / "judge.pt", map_location=device))
    judge.eval()

    pools = final_pools(cards.n)
    g = np.load(DATA / "FDN_games.npz")
    keys, decks = g["draft_key"], g["deck"]
    first = {}
    for i, k in enumerate(keys):  # the deck from each draft's first recorded game
        if k not in first and k in pools and split_of(k) == "test":
            first[k] = i
    chosen = list(first)[: a.n]
    human = decks[[first[k] for k in chosen]][:, : cards.n].astype(np.float32)
    pool = np.stack([pools[k] for k in chosen])
    ok = (human.sum(1) == 40) & (human <= pool + 20 * cards.basic).all(1)  # human deck came from this pool
    human, pool = human[ok], pool[ok]

    with torch.no_grad():
        human_wr = judge.win_rate(torch.as_tensor(human, device=device)).cpu().numpy()
    built, built_wr, pairs = build_decks(pool, judge, cards, device)
    nonbasic = ~cards.basic
    overlap = np.minimum(built, human)[:, nonbasic].sum(1) / np.maximum(human[:, nonbasic].sum(1), 1)

    r = {
        "pools": int(len(pool)),
        "human_judged_wr": float(human_wr.mean()),
        "builder_judged_wr": float(built_wr.mean()),
        "builder_better_share": float((built_wr > human_wr).mean()),
        "spell_overlap_with_human": float(overlap.mean()),
        "builder_rares_per_deck": float(built[:, cards.rare].sum(1).mean()),
        "human_rares_per_deck": float(human[:, cards.rare].sum(1).mean()),
    }
    for k, v in r.items():
        print(f"{k:<28} {v:.4f}" if isinstance(v, float) else f"{k:<28} {v}")
    res = DATA.parent / "results"
    (res / "builder_vs_human.json").write_text(json.dumps(r, indent=1))


if __name__ == "__main__":
    main()
