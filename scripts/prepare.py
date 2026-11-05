"""Stream 17lands draft + game data into compact arrays. Never unpacks the gzip to disk.

    python scripts/prepare.py            # FDN PremierDraft
Writes data/{SET}_picks.npz, data/{SET}_games.npz, assets/{SET}_cards.json
"""

import argparse
import json
import time
from pathlib import Path

import numpy as np
import pandas as pd

from bolt.data import ASSETS, DATA

RANKS = ["bronze", "silver", "gold", "platinum", "diamond", "mythic"]
POOL_MAX = 45  # 42 picks max; pool before the last pick is 41


def counts_to_ids(m: np.ndarray, width: int) -> np.ndarray:
    """(n, cards) count matrix -> (n, width) card ids + 1, zero-padded."""
    rows, cols = np.nonzero(m)
    reps = m[rows, cols].astype(np.int64)
    rows, cols = np.repeat(rows, reps), np.repeat(cols, reps)
    starts = np.searchsorted(rows, np.arange(m.shape[0]))
    pos = np.arange(len(rows)) - starts[rows]
    if len(pos) and pos.max() >= width:
        raise ValueError(f"row has more than {width} cards")
    out = np.zeros((m.shape[0], width), np.int16)
    out[rows, pos] = cols + 1
    return out


def prepare_picks(path: Path, out: Path):
    header = pd.read_csv(path, nrows=0).columns
    cards = [c[len("pack_card_") :] for c in header if c.startswith("pack_card_")]
    idx = {c: i for i, c in enumerate(cards)}
    pack_cols = [f"pack_card_{c}" for c in cards]
    pool_cols = [f"pool_{c}" for c in cards]
    dtypes = {c: np.int8 for c in pack_cols + pool_cols}

    parts, draft_ids = [], {}
    t = time.time()
    reader = pd.read_csv(path, chunksize=200_000, dtype=dtypes)
    for i, df in enumerate(reader):
        did = df["draft_id"].map(lambda d: draft_ids.setdefault(d, len(draft_ids)))
        parts.append(
            dict(
                pack=counts_to_ids(df[pack_cols].to_numpy(), 15),
                pool=counts_to_ids(df[pool_cols].to_numpy(), POOL_MAX),
                pick=df["pick"].map(idx).to_numpy(np.int16),
                pack_number=df["pack_number"].to_numpy(np.int8),
                pick_number=df["pick_number"].to_numpy(np.int8),
                rank=df["rank"].map({r: i for i, r in enumerate(RANKS)}).fillna(-1).to_numpy(np.int8),
                user_wr=df["user_game_win_rate_bucket"].fillna(-1).to_numpy(np.float32),
                user_games=df["user_n_games_bucket"].fillna(-1).to_numpy(np.int16),
                event_wins=df["event_match_wins"].to_numpy(np.int8),
                draft=did.to_numpy(np.int32),
            )
        )
        print(f"  picks chunk {i}: {sum(len(p['pick']) for p in parts):,} rows, {time.time() - t:.0f}s", flush=True)
    arrays = {k: np.concatenate([p[k] for p in parts]) for k in parts[0]}
    arrays["draft_keys"] = np.array(list(draft_ids), dtype="U32")  # index -> 17lands draft_id
    np.savez_compressed(out, **arrays)
    return cards, len(arrays["pick"]), len(draft_ids)


def prepare_games(path: Path, out: Path, cards: list[str]):
    header = pd.read_csv(path, nrows=0).columns
    groups = ["deck", "opening_hand", "drawn", "sideboard"]
    cols = {g: [f"{g}_{c}" for c in cards] for g in groups}
    missing = [c for g in groups for c in cols[g] if c not in header]
    if missing:
        raise ValueError(f"game data missing {len(missing)} card columns, e.g. {missing[:3]}")
    meta = ["draft_id", "won", "on_play", "num_turns", "rank", "user_game_win_rate_bucket"]
    dtypes = {c: np.int8 for g in groups for c in cols[g]}
    parts = []
    reader = pd.read_csv(path, chunksize=200_000, dtype=dtypes, usecols=meta + [c for g in groups for c in cols[g]])
    for i, df in enumerate(reader):
        parts.append(
            dict(
                deck=df[cols["deck"]].to_numpy(np.int8),
                in_hand=(df[cols["opening_hand"]].to_numpy() + df[cols["drawn"]].to_numpy()).astype(np.int8),
                sideboard=df[cols["sideboard"]].to_numpy(np.int8),
                won=df["won"].astype(bool).to_numpy(),
                on_play=df["on_play"].astype(bool).to_numpy(),
                num_turns=df["num_turns"].to_numpy(np.int16),
                rank=df["rank"].map({r: i for i, r in enumerate(RANKS)}).fillna(-1).to_numpy(np.int8),
                user_wr=df["user_game_win_rate_bucket"].fillna(-1).to_numpy(np.float32),
                draft_key=df["draft_id"].to_numpy(dtype=object),
            )
        )
        print(f"  games chunk {i}", flush=True)
    arrays = {k: np.concatenate([p[k] for p in parts]) for k in parts[0]}
    arrays["draft_key"] = arrays["draft_key"].astype("U32")
    np.savez_compressed(out, **arrays)
    return len(arrays["won"])


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--set", default="FDN")
    ap.add_argument("--event", default="PremierDraft")
    a = ap.parse_args()
    raw = DATA / "raw"
    cards, n, d = prepare_picks(raw / f"draft_data_public.{a.set}.{a.event}.csv.gz", DATA / f"{a.set}_picks.npz")
    print(f"picks: {n:,} from {d:,} drafts, {len(cards)} cards")
    (ASSETS / f"{a.set}_cards.json").write_text(json.dumps(cards, indent=0))
    g = prepare_games(raw / f"game_data_public.{a.set}.{a.event}.csv.gz", DATA / f"{a.set}_games.npz", cards)
    print(f"games: {g:,}")


if __name__ == "__main__":
    main()
