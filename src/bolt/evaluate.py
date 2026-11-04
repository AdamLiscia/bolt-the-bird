"""Pick-accuracy metrics shared by every bot."""
import numpy as np

from .data import STRONG_WR


def pick_from_scores(card_scores: np.ndarray, pack: np.ndarray) -> np.ndarray:
    """card_scores: (n, n_cards) or (n_cards,). pack: (n, 15) ids+1, 0 = empty. -> chosen card id."""
    if card_scores.ndim == 1:
        s = np.where(pack > 0, card_scores[np.maximum(pack - 1, 0)], -np.inf)
    else:
        s = np.where(pack > 0, np.take_along_axis(card_scores, np.maximum(pack - 1, 0), 1), -np.inf)
    return pack[np.arange(len(pack)), s.argmax(1)] - 1


def report(name: str, chosen: np.ndarray, picks: dict) -> dict:
    hit = chosen == picks["pick"]
    strong = picks["user_wr"] >= STRONG_WR
    first_pack_pick = picks["pick_number"]
    r = {
        "bot": name,
        "acc": float(hit.mean()),
        "acc_strong": float(hit[strong].mean()),
        "acc_by_pick": [float(hit[first_pack_pick == k].mean()) for k in range(14)],
        "n": int(len(hit)),
    }
    print(f"{name:<22} all {r['acc']:.1%}   strong drafters {r['acc_strong']:.1%}   "
          f"pick1 {r['acc_by_pick'][0]:.1%}  pick5 {r['acc_by_pick'][4]:.1%}  pick9 {r['acc_by_pick'][8]:.1%}")
    return r
