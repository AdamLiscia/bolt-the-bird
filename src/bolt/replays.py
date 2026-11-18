"""17lands replay rows → per-turn decision records, in engine card numbers.

One row is one game from the 17lands user's side (docs/play/PLAN.md). Turns alternate: on the play
U1, O1, U2, O2, ...; on the draw O1, U1, O2, U2, .... Each turn ends with a snapshot (the user's
hand as cards, both boards, both life totals, the opponent's hand size). So the position at the
start of a user turn is the previous snapshot plus the cards drawn this turn, and what the user did
is in that turn's columns: the land played, the spells cast (creatures, other permanents, and
instants and sorceries are three separate columns) and the creatures that attacked. Blocks come
from the opponent's turns.

`num_turns` is the number of turns each player took; every turn column exists up to 30 and is
zero-filled after the game ends, so turns past `num_turns` are ignored.

A record is marked `consistent` when the start-of-turn hand, minus what was played, cast and
discarded, equals the end-of-turn hand (97% of turns; most others are mulligans, where the opening
hand is recorded before cards go to the bottom, or cards returned to hand).
"""

from collections import Counter


class CardMap:
    """17lands card id → engine card number, by name. Anything unknown maps to `unknown`."""

    def __init__(self, id_to_name: dict[str, str], engine_names: list[str]):
        index = {}
        for i, n in enumerate(engine_names):
            index.setdefault(n, i)
        self.unknown = len(engine_names)
        self.map = {i: index.get(n, self.unknown) for i, n in id_to_name.items()}

    def __getitem__(self, card_id: str) -> int:
        return self.map.get(card_id, self.unknown)

    def ids(self, cell: str) -> list[int]:
        return [self[x] for x in cell.split("|") if x] if cell else []


SNAPSHOT = {
    "hand": "eot_user_cards_in_hand",
    "my_lands": "eot_user_lands_in_play",
    "my_creatures": "eot_user_creatures_in_play",
    "my_others": "eot_user_non_creatures_in_play",
    "opp_lands": "eot_oppo_lands_in_play",
    "opp_creatures": "eot_oppo_creatures_in_play",
    "opp_others": "eot_oppo_non_creatures_in_play",
}
CAST = ("creatures_cast", "non_creatures_cast", "user_instants_sorceries_cast")


def _num(v: str, default: int) -> int:
    """A number cell as an int; empty or NaN gives `default`, and anything else is clamped to
    ±999 (one real game records a life total of infinity)."""
    if v in ("", None):
        return default
    x = float(v)
    if x != x:  # NaN
        return default
    return int(max(-999.0, min(999.0, x)))


def _snapshot(row: dict, prefix: str | None, cm: CardMap) -> dict:
    """The end-of-turn snapshot of turn `prefix` (e.g. "oppo_turn_3"), or the game's start."""
    if prefix is None:
        out = {k: [] for k in SNAPSHOT}
        out["hand"] = cm.ids(row.get("opening_hand", ""))
        return out | {"my_life": 20, "opp_life": 20, "opp_hand": 7}
    out = {k: cm.ids(row.get(f"{prefix}_{col}", "")) for k, col in SNAPSHOT.items()}
    return out | {
        "my_life": _num(row.get(f"{prefix}_eot_user_life"), 20),
        "opp_life": _num(row.get(f"{prefix}_eot_oppo_life"), 20),
        "opp_hand": _num(row.get(f"{prefix}_eot_oppo_cards_in_hand"), 7),
    }


def _played(row: dict, prefix: str) -> bool:
    return bool(row.get(f"{prefix}_eot_user_life"))


def user_turns(row: dict, cm: CardMap) -> list[dict]:
    """One record per user turn: the position at its start and what the user did."""
    play = row.get("on_play") == "True"
    out = []
    for t in range(1, _num(row.get("num_turns"), 0) + 1):  # later columns are zero-filled
        me = f"user_turn_{t}"
        if not _played(row, me):
            break
        before = None if (play and t == 1) else (f"oppo_turn_{t - 1}" if play else f"oppo_turn_{t}")
        if before is not None and not _played(row, before):
            break
        rec = _snapshot(row, before, cm)
        drawn = cm.ids(row.get(f"{me}_cards_drawn", ""))
        rec["hand"] = rec["hand"] + drawn
        rec["lands_played"] = cm.ids(row.get(f"{me}_lands_played", ""))
        rec["cast"] = [c for col in CAST for c in cm.ids(row.get(f"{me}_{col}", ""))]
        rec["attacked"] = cm.ids(row.get(f"{me}_creatures_attacked", ""))
        discarded = cm.ids(row.get(f"{me}_cards_discarded", ""))
        end = Counter(cm.ids(row.get(f"{me}_eot_user_cards_in_hand", "")))
        rec["consistent"] = Counter(rec["hand"]) - Counter(rec["lands_played"] + rec["cast"] + discarded) == end
        rec |= {"turn": t, "on_play": play, "won": row.get("won") == "True"}
        out.append(rec)
    return out


def block_turns(row: dict, cm: CardMap) -> list[dict]:
    """One record per opponent turn in which the opponent attacked: the user's creatures before
    the turn, the attackers, and which of the user's creatures blocked."""
    play = row.get("on_play") == "True"
    out = []
    for t in range(1, _num(row.get("num_turns"), 0) + 1):
        opp = f"oppo_turn_{t}"
        if not _played(row, opp):
            break
        attackers = cm.ids(row.get(f"{opp}_creatures_attacked", ""))
        if not attackers:
            continue
        before = f"user_turn_{t}" if play else (f"user_turn_{t - 1}" if t > 1 else None)
        rec = _snapshot(row, before, cm)
        rec |= {
            "turn": t,
            "attackers": attackers,
            "blockers": cm.ids(row.get(f"{opp}_creatures_blocking", "")),
            "on_play": play,
            "won": row.get("won") == "True",
        }
        out.append(rec)
    return out


def pack(lists: list[list[int]], width: int):
    """Card lists → (n, width) int16 rows of card number + 1, zero-padded; also how many lists were
    longer than `width` and got cut."""
    import numpy as np

    out = np.zeros((len(lists), width), dtype=np.int16)
    cut = 0
    for i, cards in enumerate(lists):
        cut += len(cards) > width
        cards = cards[:width]
        out[i, : len(cards)] = np.asarray(cards, dtype=np.int16) + 1
    return out, cut
