"""Load prepared arrays and split them by draft, so no draft is ever in two splits."""

import hashlib
import json
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"  # large, downloaded, gitignored
ASSETS = ROOT / "assets"  # small, committed: card metadata and sample packs
STRONG_WR = 0.58  # user_game_win_rate_bucket at or above this = "strong drafter"


def split_of(draft_key: str) -> str:
    b = int(hashlib.md5(draft_key.encode()).hexdigest()[:8], 16) % 100
    return "test" if b < 10 else "val" if b < 15 else "train"


def half_of(draft_key: str) -> int:
    """Independent 50/50 split of drafts, for training two judges on disjoint games."""
    return int(hashlib.md5(draft_key.encode()).hexdigest()[8:16], 16) % 2


def load_cards(set_code="FDN") -> list[str]:
    return json.loads((ASSETS / f"{set_code}_cards.json").read_text())


def load_picks(set_code="FDN") -> dict:
    z = np.load(DATA / f"{set_code}_picks.npz")
    p = {k: z[k] for k in z.files}
    draft_split = np.array([split_of(k) for k in p.pop("draft_keys")])
    p["split"] = draft_split[p["draft"]]
    return p


def load_games(set_code="FDN") -> dict:
    z = np.load(DATA / f"{set_code}_games.npz", allow_pickle=False)
    g = {k: z[k] for k in z.files}
    keys = g.pop("draft_key")
    g["split"] = np.array([split_of(k) for k in keys])
    g["half"] = np.array([half_of(k) for k in keys], dtype=np.int8)
    return g


def subset(d: dict, mask: np.ndarray) -> dict:
    return {k: v[mask] for k, v in d.items()}
