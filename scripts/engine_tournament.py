"""A deck tournament between the drafting bots: every bot's deck against every other bot's deck
from the same opening packs, on the 512 held-out test tables.

Each bot already drafted those tables for the round-robins (data/m7*/decks.txt, lines 0-511; the
human model's decks are lines 512-1023 of data/m7/decks.txt). For each pair of bots and each table,
the two decks play GAMES games, alternating who goes first, with the same seeds for every pair.
Games run with scripts/engine_pilot_overnight.sh (guarded, resumable) in data/tournament/.

    python scripts/engine_tournament.py prepare [--games 20]
    DIR=data/tournament scripts/engine_pilot_overnight.sh 12 t 4 6     # smart 4 x 6
    python scripts/engine_tournament.py analyze
"""

import argparse
import itertools
import json
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "data" / "tournament"
TABLES = 512

# name, label, decks file, first line of its 512 decks
BOTS = [
    ("human_model", "Human-style (imitation)", "data/m7/decks.txt", TABLES),
    ("judge", "Trained on the judge", "data/m7/decks.txt", 0),
    ("reinforce250", "Played games, first method (round 250)", "data/m7_rl_played250/a/decks.txt", 0),
    ("ppo345", "PPO, continued (round 345)", "data/m7_rl_ppo345/a/decks.txt", 0),
    ("ppo_scratch125", "PPO from scratch (round 125)", "data/m7_rl_ppo_scratch2/a/decks.txt", 0),
]


def prepare(games: int):
    OUT.mkdir(parents=True, exist_ok=True)
    decks = []
    for _, _, f, start in BOTS:
        lines = (ROOT / f).read_text().split("\n")[start : start + TABLES]
        assert len(lines) == TABLES and all(len(x.split()) >= 40 for x in lines), f
        decks += lines
    (OUT / "decks.txt").write_text("\n".join(decks) + "\n")
    rng = np.random.default_rng(7)
    seeds = rng.integers(2**62, size=(TABLES, games))  # the same games for every pair
    jobs, meta = [], []
    for (i, a), (j, b) in itertools.combinations(enumerate(BOTS), 2):
        for t in range(TABLES):
            for k in range(games):
                jobs.append(f"{len(jobs)} {i * TABLES + t} {j * TABLES + t} {int(seeds[t, k])} {k % 2}")
                meta.append({"a": a[0], "b": b[0], "table": t})
    (OUT / "jobs.txt").write_text("\n".join(jobs) + "\n")
    (OUT / "meta.json").write_text(json.dumps(meta))
    print(f"{len(jobs)} games: {len(BOTS)} bots, {len(jobs) // (TABLES * games)} pairs, {TABLES} tables x {games}")


def analyze():
    meta = json.loads((OUT / "meta.json").read_text())
    names = [b[0] for b in BOTS]
    out = {"bots": {b[0]: b[1] for b in BOTS}, "pilots": {}}
    for pilot in ("baseline", "rollout", "network"):
        f = OUT / f"games_{pilot}.txt"
        if not f.exists():
            continue
        s = np.full(len(meta), np.nan)
        for line in f.read_text().split("\n"):
            x = line.split()
            if len(x) == 3 and x[1] != "error":
                s[int(x[0])] = {"p0": 1.0, "p1": 0.0}.get(x[1], 0.5)
        a = np.array([m["a"] for m in meta])
        b = np.array([m["b"] for m in meta])
        table = np.array([m["table"] for m in meta])
        pairs, score = {}, {n: [] for n in names}
        for x, y in itertools.combinations(names, 2):
            m = (a == x) & (b == y) & ~np.isnan(s)
            if not m.any():
                continue
            per = np.array([s[m & (table == t)].mean() for t in np.unique(table[m])])  # per table
            pairs[f"{x} vs {y}"] = {
                "win_rate": float(per.mean()), "se": float(per.std() / np.sqrt(len(per))),
                "games": int(m.sum()), "tables": int(len(per)),
            }  # fmt: skip
            score[x].append(per.mean())
            score[y].append(1 - per.mean())
        out["pilots"][pilot] = {
            "pairs": pairs,
            "average_win_rate": {n: float(np.mean(v)) for n, v in score.items() if v},
        }
    (ROOT / "results" / "engine_tournament.json").write_text(json.dumps(out, indent=2) + "\n")
    for pilot, r in out["pilots"].items():
        print(f"\n{pilot}")
        for n, w in sorted(r["average_win_rate"].items(), key=lambda kv: -kv[1]):
            print(f"  {w:6.1%}  {out['bots'][n]}")
        for p, v in r["pairs"].items():
            print(f"  {p}: {v['win_rate']:.1%} ± {v['se']:.1%} ({v['games']} games)")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("step", choices=["prepare", "analyze"])
    ap.add_argument("--games", type=int, default=20, help="games per table for each pair")
    a = ap.parse_args()
    prepare(a.games) if a.step == "prepare" else analyze()


if __name__ == "__main__":
    main()
