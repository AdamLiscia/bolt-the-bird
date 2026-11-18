"""Parse the 17lands FDN replays into per-turn decision records (bolt.replays), in shards.

    python scripts/parse_replays.py                 # all games → data/replays/{user,block}_NNN.npz
    python scripts/parse_replays.py --games 20000   # just the first N games

Each shard holds up to --shard games. Card lists are fixed-width int16 rows (card number + 1,
0 = empty); a list longer than its width is cut, and the cuts are counted in data/replays/stats.json.
Games are numbered in file order; split by game, never by turn, when training.
"""

import argparse
import csv
import gzip
import json
import time

import bolt_engine as be
import numpy as np

from bolt.data import ROOT
from bolt.replays import CardMap, block_turns, pack, user_turns

RAW = ROOT / "data" / "raw"
OUT = ROOT / "data" / "replays"
USER_WIDTHS = {"hand": 16, "my_lands": 16, "my_creatures": 24, "my_others": 12, "opp_lands": 16,
               "opp_creatures": 24, "opp_others": 12, "lands_played": 2, "cast": 6, "attacked": 16}  # fmt: skip
BLOCK_WIDTHS = {"hand": 16, "my_lands": 16, "my_creatures": 24, "my_others": 12, "opp_lands": 16,
                "opp_creatures": 24, "opp_others": 12, "attackers": 16, "blockers": 16}  # fmt: skip
SCALARS = ("turn", "my_life", "opp_life", "opp_hand", "on_play", "won")


def write(kind: str, recs: list[dict], widths: dict, shard: int, cuts: dict) -> int:
    if not recs:
        return 0
    arrays = {}
    for k, w in widths.items():
        arrays[k], cut = pack([r[k] for r in recs], w)
        cuts[f"{kind}.{k}"] = cuts.get(f"{kind}.{k}", 0) + cut
    for k in SCALARS:
        arrays[k] = np.array([r[k] for r in recs], dtype=np.int16)
    arrays["game"] = np.array([r["game"] for r in recs], dtype=np.int32)
    arrays["skill"] = np.array([r["skill"] for r in recs], dtype=np.float32)
    if kind == "user":
        arrays["consistent"] = np.array([r["consistent"] for r in recs], dtype=bool)
    np.savez_compressed(OUT / f"{kind}_{shard:03d}.npz", **arrays)
    return len(recs)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--games", type=int, default=0, help="stop after this many games (0 = all)")
    ap.add_argument("--shard", type=int, default=50_000, help="games per shard")
    a = ap.parse_args()
    csv.field_size_limit(10**9)
    with open(RAW / "cards.csv", newline="") as cf:
        cm = CardMap({r["id"]: r["name"] for r in csv.DictReader(cf)}, be.card_names())
    OUT.mkdir(parents=True, exist_ok=True)
    with gzip.open(RAW / "replay_data_public.FDN.PremierDraft.csv.gz", "rt", newline="") as f:
        parse(csv.reader(f), cm, a)


def parse(rd, cm: CardMap, a) -> None:
    header = next(rd)
    totals = {"games": 0, "user": 0, "block": 0, "consistent": 0}
    cuts: dict = {}
    users, blocks, shard, t0 = [], [], 0, time.time()
    for g, values in enumerate(rd):
        if a.games and g >= a.games:
            break
        row = dict(zip(header, values, strict=False))
        rate = row.get("user_game_win_rate_bucket", "")
        extra = {"game": g, "skill": float(rate) if rate else np.nan}
        users += [u | extra for u in user_turns(row, cm)]
        blocks += [b | extra for b in block_turns(row, cm)]
        totals["games"] += 1
        if totals["games"] % a.shard == 0:
            totals["consistent"] += sum(u["consistent"] for u in users)
            totals["user"] += write("user", users, USER_WIDTHS, shard, cuts)
            totals["block"] += write("block", blocks, BLOCK_WIDTHS, shard, cuts)
            users, blocks, shard = [], [], shard + 1
            print(f"{totals['games']} games, {time.time() - t0:.0f}s", flush=True)
    totals["consistent"] += sum(u["consistent"] for u in users)
    totals["user"] += write("user", users, USER_WIDTHS, shard, cuts)
    totals["block"] += write("block", blocks, BLOCK_WIDTHS, shard, cuts)
    stats = totals | {"cut": cuts, "seconds": round(time.time() - t0)}
    (OUT / "stats.json").write_text(json.dumps(stats, indent=2) + "\n")
    print(json.dumps(stats, indent=2))


if __name__ == "__main__":
    main()
