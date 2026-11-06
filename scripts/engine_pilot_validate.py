"""M7: rerun the M6 gauntlet with a stronger pilot (rollout search) on both sides.

Uses the decks M6 drafted and built (data/engine_decks_512.json). Each table's bot deck and
human-model deck play the same opponents from the same 64-deck human-model field, with the same
seeds, so the gap between them is a paired comparison. The rollout pilot is ~1,000x slower than the
baseline, so each deck plays a sample of the field rather than all of it. The baseline plays the
same games too, as a control.

    python scripts/engine_pilot_validate.py prepare [--games 8]
    engine/target/release/examples/pilot_games data/m7/decks.txt data/m7/jobs.txt \\
        data/m7/games_rollout.txt rollout 540        # resumable; repeat until nothing is left
    python scripts/engine_pilot_validate.py analyze
"""

import argparse
import json
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
DIR = ROOT / "data" / "m7"
TABLES = 512
FIELD = 64


def prepare(games: int):
    d = json.loads((ROOT / "data" / f"engine_decks_{TABLES}.json").read_text())
    decks = d["bot"]["decks"] + d["human_model"]["decks"]  # bot t is line t, human t is 512 + t
    DIR.mkdir(parents=True, exist_ok=True)
    (DIR / "decks.txt").write_text("\n".join(" ".join(map(str, x)) for x in decks) + "\n")
    field = np.random.default_rng(0).choice(TABLES, size=FIELD, replace=False)  # M6's field
    rng = np.random.default_rng(1)
    jobs, meta = [], []
    for t in range(TABLES):
        opps = rng.choice([f for f in field if f != t], size=games, replace=False)
        for k, f in enumerate(opps):
            seed = int(rng.integers(2**62))
            for side, deck in (("bot", t), ("human_model", TABLES + t)):
                # The deck under test is player 0; who goes first alternates by game.
                jobs.append(f"{len(jobs)} {deck} {TABLES + int(f)} {seed} {k % 2}")
                meta.append({"table": t, "side": side, "opp": int(f)})
    (DIR / "jobs.txt").write_text("\n".join(jobs) + "\n")
    (DIR / "meta.json").write_text(json.dumps(meta))
    print(f"{len(jobs)} games: {TABLES} tables x 2 decks x {games} opponents")


def scores(pilot: str, meta: list) -> np.ndarray | None:
    """Player 0's score per job (nan if not played), or None if nothing was played."""
    f = DIR / f"games_{pilot}.txt"
    if not f.exists():
        return None
    s = np.full(len(meta), np.nan)
    timeouts = 0
    for line in f.read_text().split("\n"):
        if len(line.split()) == 3:  # a run killed mid-write can leave a partial last line
            i, r, _ = line.split()
            s[int(i)] = {"p0": 1.0, "p1": 0.0}.get(r, 0.5)
            timeouts += r == "timeout"
    return s, timeouts


def analyze():
    meta = json.loads((DIR / "meta.json").read_text())
    d = json.loads((ROOT / "data" / f"engine_decks_{TABLES}.json").read_text())
    judge = {"bot": np.array(d["bot"]["judge_b"]), "human_model": np.array(d["human_model"]["judge_b"])}
    table = np.array([m["table"] for m in meta])
    bot = np.array([m["side"] == "bot" for m in meta])
    out = {}
    for pilot in ("baseline", "rollout"):
        got = scores(pilot, meta)
        if got is None:
            continue
        s, timeouts = got
        played = {side: np.full(TABLES, np.nan) for side in judge}
        for side, mask in (("bot", bot), ("human_model", ~bot)):
            for t in range(TABLES):
                x = s[mask & (table == t)]
                if len(x) and not np.isnan(x).any():
                    played[side][t] = x.mean()
        # Only tables where both decks played every game.
        ok = ~np.isnan(played["bot"]) & ~np.isnan(played["human_model"])
        gap = played["bot"][ok] - played["human_model"][ok]
        all_j = np.concatenate([judge["bot"][ok], judge["human_model"][ok]])
        all_p = np.concatenate([played["bot"][ok], played["human_model"][ok]])
        out[pilot] = {
            "tables": int(ok.sum()),
            "games": int((~np.isnan(s)).sum()),
            "bot": float(played["bot"][ok].mean()),
            "human_model": float(played["human_model"][ok].mean()),
            "gap": float(gap.mean()),
            "gap_se": float(gap.std() / np.sqrt(len(gap))),
            "corr_with_judge_b": float(np.corrcoef(all_j, all_p)[0, 1]),
            "timeouts": timeouts,
        }
    res = {
        "judge_b_gap": float((judge["bot"] - judge["human_model"]).mean()),
        "pilots": out,
    }
    if {"baseline", "rollout"} <= out.keys():
        both = [p for p in out]
        res["note"] = f"same games for {both}; tables differ only if a run is unfinished"
    (ROOT / "results" / "engine_pilot_validation.json").write_text(json.dumps(res, indent=2) + "\n")
    print(json.dumps(res, indent=2))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("step", choices=["prepare", "analyze"])
    ap.add_argument("--games", type=int, default=8, help="field opponents per deck, one game each")
    a = ap.parse_args()
    prepare(a.games) if a.step == "prepare" else analyze()


if __name__ == "__main__":
    main()
