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

`extend` adds games against every field opponent a deck hasn't met yet (a full round-robin),
appended after the existing jobs and ordered opponent-round first, so a run stopped partway still
covers every table evenly. `analyze` scores each table on the games both of its decks finished.

`--set b` and `--set c` are replications: 64-deck fields with no deck in common with each other or
the first, new seeds, a full round-robin from the start (`prepare --set b`), in data/m7b (m7c),
results in results/engine_pilot_validation_b.json (_c).
"""

import argparse
import json
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
DIR = ROOT / "data" / "m7"
TABLES = 512
FIELD = 64
SUFFIX = ""


def field_of(set_: str) -> np.ndarray:
    a = np.random.default_rng(0).choice(TABLES, size=FIELD, replace=False)  # M6's field
    if set_ == "a":
        return a
    rest = np.array([t for t in range(TABLES) if t not in set(a.tolist())])
    b = np.random.default_rng(10).choice(rest, size=FIELD, replace=False)
    if set_ == "b":
        return b
    rest = np.array([t for t in rest if t not in set(b.tolist())])
    return np.random.default_rng(20).choice(rest, size=FIELD, replace=False)


def prepare_rr(set_: str):
    """Every deck against every deck of field `set_` (b or c) once, round-major."""
    d = json.loads((ROOT / "data" / f"engine_decks_{TABLES}.json").read_text())
    decks = d["bot"]["decks"] + d["human_model"]["decks"]
    DIR.mkdir(parents=True, exist_ok=True)
    (DIR / "decks.txt").write_text("\n".join(" ".join(map(str, x)) for x in decks) + "\n")
    field = [int(f) for f in field_of(set_)]
    rng = np.random.default_rng({"b": 11, "c": 21}[set_])
    order = {t: [int(x) for x in rng.permutation([f for f in field if f != t])] for t in range(TABLES)}
    jobs, meta = [], []
    for k in range(FIELD):
        for t in range(TABLES):
            if k >= len(order[t]):
                continue
            f, seed = order[t][k], int(rng.integers(2**62))
            for side, deck in (("bot", t), ("human_model", TABLES + t)):
                jobs.append(f"{len(jobs)} {deck} {TABLES + f} {seed} {k % 2}")
                meta.append({"table": t, "side": side, "opp": f})
    (DIR / "jobs.txt").write_text("\n".join(jobs) + "\n")
    (DIR / "meta.json").write_text(json.dumps(meta))
    print(f"{len(jobs)} games: {TABLES} tables x 2 decks x the whole field {set_}")


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


def extend():
    """Append games so every deck meets all 63 other field decks, keeping the existing jobs."""
    jobs = (DIR / "jobs.txt").read_text().split("\n")
    jobs = [j for j in jobs if j]
    meta = json.loads((DIR / "meta.json").read_text())
    assert len(jobs) == len(meta)
    field = np.random.default_rng(0).choice(TABLES, size=FIELD, replace=False)
    met = {t: set() for t in range(TABLES)}
    for m in meta:
        met[m["table"]].add(m["opp"])
    rng = np.random.default_rng(2)
    left = {t: [int(x) for x in rng.permutation([int(f) for f in field if f != t and f not in met[t]])] for t in range(TABLES)}
    added = 0
    for k in range(max(len(v) for v in left.values())):
        for t in range(TABLES):
            if k >= len(left[t]):
                continue
            f, seed = left[t][k], int(rng.integers(2**62))
            for side, deck in (("bot", t), ("human_model", TABLES + t)):
                jobs.append(f"{len(jobs)} {deck} {TABLES + f} {seed} {(len(met[t]) + k) % 2}")
                meta.append({"table": t, "side": side, "opp": f})
                added += 1
    (DIR / "jobs.txt").write_text("\n".join(jobs) + "\n")
    (DIR / "meta.json").write_text(json.dumps(meta))
    print(f"added {added} games; {len(jobs)} in all")


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
            if r == "error":  # the engine panicked: no result, so the pair drops out
                continue
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
    # baseline, rollout (4 x 6) and any other search size (games_r<R>c<C>.txt)
    names = sorted(f.stem[len("games_"):] for f in DIR.glob("games_*.txt"))
    for pilot in sorted(names, key=lambda n: (n != "baseline", n != "rollout", n)):
        got = scores(pilot, meta)
        if got is None:
            continue
        s, timeouts = got
        # Jobs come in pairs (bot deck, then human-model deck, same opponent and seed); a pair
        # counts only when both games finished, so the comparison stays paired.
        pair_done = ~np.isnan(s[0::2]) & ~np.isnan(s[1::2])
        done = np.repeat(pair_done, 2)
        played = {side: np.full(TABLES, np.nan) for side in judge}
        for side, mask in (("bot", bot), ("human_model", ~bot)):
            for t in range(TABLES):
                x = s[mask & (table == t) & done]
                if len(x):
                    played[side][t] = x.mean()
        ok = ~np.isnan(played["bot"]) & ~np.isnan(played["human_model"])
        gap = played["bot"][ok] - played["human_model"][ok]
        all_j = np.concatenate([judge["bot"][ok], judge["human_model"][ok]])
        all_p = np.concatenate([played["bot"][ok], played["human_model"][ok]])
        out[pilot] = {
            "tables": int(ok.sum()),
            "games": int((~np.isnan(s)).sum()),
            "paired_games": int(done.sum()),
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
    (ROOT / "results" / f"engine_pilot_validation{SUFFIX}.json").write_text(json.dumps(res, indent=2) + "\n")
    print(json.dumps(res, indent=2))


# Strength against the baseline, from `rollout_bench` (200 games each, 2026-11-07/08; LOG.md).
LADDER = [
    ("baseline", "Rules-based", 0.5, 0.0),
    ("r1c2", "Smart 1 x 2", 0.650, 0.034),
    ("r2c4", "Smart 2 x 4", 0.700, 0.031),
    ("rollout", "Smart 4 x 6", 0.745, 0.029),
]


def ladder():
    """Pool each pilot's per-table gap over fields a, b and c into results/engine_pilot_ladder.json."""
    gaps = {}
    for set_, d in (("a", "m7"), ("b", "m7b"), ("c", "m7c")):
        meta = json.loads((ROOT / "data" / d / "meta.json").read_text())
        table = np.array([m["table"] for m in meta])
        bot = np.array([m["side"] == "bot" for m in meta])
        for name, *_ in LADDER:
            s = np.full(len(meta), np.nan)
            for line in (ROOT / "data" / d / f"games_{name}.txt").read_text().split("\n"):
                x = line.split()
                if len(x) == 3 and x[1] != "error":
                    s[int(x[0])] = {"p0": 1.0, "p1": 0.0}.get(x[1], 0.5)
            assert not np.isnan(s).any(), f"{d} {name} unfinished"
            g = [s[(table == t) & bot].mean() - s[(table == t) & ~bot].mean() for t in range(TABLES)]
            gaps[(name, set_)] = np.array(g)
    se = lambda x: float(x.std() / np.sqrt(len(x)))  # noqa: E731
    base = sum(gaps[("baseline", f)] for f in "abc") / 3
    out = []
    for name, label, wr, wr_se in LADDER:
        pooled = sum(gaps[(name, f)] for f in "abc") / 3
        out.append({
            "pilot": name, "label": label, "beats_baseline": wr, "beats_baseline_se": wr_se,
            "gap": float(pooled.mean()), "gap_se": se(pooled),
            "change_from_baseline": float((pooled - base).mean()), "change_se": se(pooled - base),
            "by_field": {f: float(gaps[(name, f)].mean()) for f in "abc"},
        })  # fmt: skip
    d = json.loads((ROOT / "data" / f"engine_decks_{TABLES}.json").read_text())
    res = {
        "judge_b_gap": float((np.array(d["bot"]["judge_b"]) - np.array(d["human_model"]["judge_b"])).mean()),
        "games_per_pilot": 3 * len(json.loads((ROOT / "data" / "m7" / "meta.json").read_text())),
        "pilots": out,
    }
    (ROOT / "results" / "engine_pilot_ladder.json").write_text(json.dumps(res, indent=2) + "\n")
    print(json.dumps(res, indent=2))


def newbot(checkpoint: str, name: str):
    """A new bot's decks through the same round-robins: data/m7_<name>/{a,b,c}.

    Drafts the 512 test tables exactly as M6 did (engine_validate.draft: same packs, seed, judge B
    builder), checks that the human-model side reproduces data/engine_decks_512.json, then writes
    each field's decks with the bot lines replaced and only the bot-side jobs. The human-model side
    of every pair is already played (data/m7*/games_<pilot>.txt), with the same opponents and seeds.
    """
    import sys

    import torch

    sys.path.insert(0, str(ROOT / "scripts"))
    import engine_validate as ev

    from bolt.cards import CardInfo
    from bolt.deckbuild import build_decks
    from bolt.models import DraftTransformer
    from bolt.sim import opening_packs, pools_to_counts, run_tables

    torch.set_grad_enabled(False)
    cards = CardInfo()
    cpu = torch.device("cpu")
    human = ev.load(DraftTransformer, "human_model.pt", cards.n)
    judge = ev.load(ev.DeckJudge, "judge_b.pt", cards.n)
    stored = json.loads((ROOT / "data" / f"engine_decks_{TABLES}.json").read_text())
    lists = {}
    for side, model in (("human_model", human), ("new", ev.load(DraftTransformer, checkpoint, cards.n))):
        g = torch.Generator().manual_seed(ev.SEED)
        pools, _ = run_tables(opening_packs("test"), TABLES, model, human, cpu, g, seat0_greedy=True)
        decks, _, _ = build_decks(pools_to_counts(pools[:, 0], cards.n), judge, cards, cpu)
        lists[side] = [np.repeat(np.arange(cards.n), d.astype(int)).tolist() for d in decks]
    assert lists["human_model"] == stored["human_model"]["decks"], "the draft must reproduce M6's decks"
    out = ROOT / "data" / f"m7_{name}"
    for set_, d in (("a", "m7"), ("b", "m7b"), ("c", "m7c")):
        src = ROOT / "data" / d
        decks = (src / "decks.txt").read_text().split("\n")
        decks[:TABLES] = [" ".join(map(str, x)) for x in lists["new"]]
        meta = json.loads((src / "meta.json").read_text())
        jobs = [j for j, m in zip((src / "jobs.txt").read_text().split("\n"), meta, strict=False) if m["side"] == "bot"]
        (out / set_).mkdir(parents=True, exist_ok=True)
        (out / set_ / "decks.txt").write_text("\n".join(decks))
        (out / set_ / "jobs.txt").write_text("\n".join(jobs) + "\n")
        print(f"{set_}: {len(jobs)} bot-side games")
    (out / "decks.json").write_text(json.dumps(lists["new"]))


def newbot_analyze(name: str):
    """The new bot's gap per pilot and field, paired with the human-model games already played,
    and pooled per table over the three fields ("all") once every table has finished on all three."""
    res, pooled = {}, {}
    for set_, d in (("a", "m7"), ("b", "m7b"), ("c", "m7c")):
        meta = json.loads((ROOT / "data" / d / "meta.json").read_text())
        table = np.array([m["table"] for m in meta])
        bot = np.array([m["side"] == "bot" for m in meta])
        for pilot in ("baseline", "r1c2", "r2c4", "rollout"):
            new_f = ROOT / "data" / f"m7_{name}" / set_ / f"games_{pilot}.txt"
            old_f = ROOT / "data" / d / f"games_{pilot}.txt"
            if not new_f.exists() or not old_f.exists():
                continue
            s = np.full(len(meta), np.nan)
            for f, side in ((old_f, False), (new_f, True)):
                for line in f.read_text().split("\n"):
                    x = line.split()
                    if len(x) == 3 and x[1] != "error" and bot[int(x[0])] == side:
                        s[int(x[0])] = {"p0": 1.0, "p1": 0.0}.get(x[1], 0.5)
            done = ~np.isnan(s[0::2]) & ~np.isnan(s[1::2])
            if not done.any():
                continue
            pair_t = table[0::2][done]
            gap = s[0::2][done] - s[1::2][done]
            per = np.array([gap[pair_t == t].mean() for t in np.unique(pair_t)])
            res.setdefault(pilot, {})[set_] = {
                "gap": float(per.mean()), "gap_se": float(per.std() / np.sqrt(len(per))),
                "pairs": int(done.sum()), "tables": int(len(per)),
                "bot_win_rate": float(np.mean(s[0::2][done])), "human_model_win_rate": float(np.mean(s[1::2][done])),
            }  # fmt: skip
            if len(per) == TABLES:
                pooled.setdefault(pilot, []).append(per)
    for pilot, fields in pooled.items():
        if len(fields) == 3:  # every table on all three fields
            g = sum(fields) / 3
            res[pilot]["all"] = {"gap": float(g.mean()), "gap_se": float(g.std() / np.sqrt(len(g)))}
    (ROOT / "results" / f"engine_pilot_{name}.json").write_text(json.dumps(res, indent=2) + "\n")
    print(json.dumps(res, indent=2))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("step", choices=["prepare", "extend", "analyze", "ladder", "newbot", "newbot-analyze"])
    ap.add_argument("--checkpoint", default="bot_rl_played.pt", help="newbot: the bot to evaluate")
    ap.add_argument("--name", default="rl_played", help="newbot: data/m7_<name>, results/engine_pilot_<name>.json")
    ap.add_argument("--games", type=int, default=8, help="field opponents per deck, one game each")
    ap.add_argument("--set", default="a", choices=["a", "b", "c"], help="b, c: replication fields")
    a = ap.parse_args()
    global DIR, SUFFIX
    if a.set != "a":
        DIR, SUFFIX = ROOT / "data" / f"m7{a.set}", f"_{a.set}"
        {"prepare": lambda: prepare_rr(a.set), "analyze": analyze}[a.step]()
        return
    steps = {"prepare": lambda: prepare(a.games), "extend": extend, "analyze": analyze, "ladder": ladder}
    steps["newbot"] = lambda: newbot(a.checkpoint, a.name)
    steps["newbot-analyze"] = lambda: newbot_analyze(a.name)
    steps[a.step]()


if __name__ == "__main__":
    main()
