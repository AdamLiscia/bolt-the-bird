"""Turn-level imitation (Phase 4, P1): targets, baselines and metrics on the parsed replays.

Every task is "which of these slots did the human choose": the hand's cards (cast, or played as
the land), the user's creatures (attacked), or the user's creatures on the opponent's turn
(blocked). Card arrays are the shards' int16 rows: card number + 1, 0 = empty.
"""

from dataclasses import dataclass

import numpy as np


def mark(slots: np.ndarray, chosen: np.ndarray) -> np.ndarray:
    """(n, w) bool: which slots hold a chosen card, each chosen copy marking one slot."""
    marked = np.zeros(slots.shape, dtype=bool)
    for j in range(chosen.shape[1]):
        card = chosen[:, j : j + 1]
        cand = (slots == card) & ~marked & (card > 0)
        marked |= cand & (np.cumsum(cand, axis=1) == 1)
    return marked


def split_of(game: np.ndarray) -> np.ndarray:
    """By game, never by turn: 5% test, 5% validation, 90% training."""
    out = np.full(game.shape, "train", dtype=object)
    out[game % 20 == 0] = "test"
    out[game % 20 == 1] = "valid"
    return out


@dataclass
class CardStats:
    """Per card number + 1 (index 0 = empty slot): mana value and whether it's a land."""

    cmc: np.ndarray
    land: np.ndarray

    @classmethod
    def load(cls, cards_csv, engine_names: list[str]) -> "CardStats":
        import csv

        by_name = {}
        with open(cards_csv, newline="") as f:
            for r in csv.DictReader(f):
                by_name.setdefault(r["name"], r)
        n = len(engine_names) + 2  # + empty slot + unknown card
        cmc, land = np.zeros(n, dtype=np.int16), np.zeros(n, dtype=bool)
        for i, name in enumerate(engine_names):
            r = by_name.get(name)
            if r:
                cmc[i + 1] = int(float(r["mana_value"] or 0))
                land[i + 1] = "Land" in r["types"]
        return cls(cmc, land)


def baseline_cast(hand: np.ndarray, my_lands: np.ndarray, st: CardStats) -> np.ndarray:
    """Play a land if there is one, then cast the most expensive spells the lands can pay for
    (mana value only; colors ignored)."""
    pred = np.zeros(hand.shape, dtype=bool)
    is_land = st.land[hand] & (hand > 0)
    has_land = is_land.any(1)
    first_land = is_land & (np.cumsum(is_land, axis=1) == 1)
    pred |= first_land
    mana = (my_lands > 0).sum(1) + has_land
    cost = np.where((hand > 0) & ~is_land, st.cmc[hand], 10_000)
    order = np.argsort(-np.where(cost < 10_000, cost, -1), axis=1, kind="stable")
    rows = np.arange(len(hand))
    left = mana.astype(np.int32).copy()
    for k in range(hand.shape[1]):
        j = order[:, k]
        c = cost[rows, j]
        ok = (c <= left) & (c < 10_000)
        pred[rows[ok], j[ok]] = True
        left = left - np.where(ok, c, 0)
    return pred


def metrics(pred: np.ndarray, target: np.ndarray, slots: np.ndarray) -> dict:
    """Accuracy over real (non-empty) slots, and the share of turns with every slot right."""
    valid = slots > 0
    right = (pred == target) | ~valid
    return {
        "slot_accuracy": float((pred == target)[valid].mean()),
        "turn_exact": float(right.all(1).mean()),
        "chosen_rate": float(target[valid].mean()),
    }
