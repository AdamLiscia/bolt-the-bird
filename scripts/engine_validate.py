"""M6: play the drafted decks in the rules engine and compare with judge B.

Drafts the same held-out tables the RL evaluation uses (test packs, seed 12345, seat 0 greedy)
once with the RL bot and once with the human model, builds both decks with the usual builder, and
scores them with judge B. Then the decks play real games in the Rust engine, with the rules-based
baseline piloting both sides:

- head-to-head: each table's bot deck against the human-model deck from the same packs;
- gauntlet: every deck against a fixed field of human-model decks from other tables.

The question is whether judge B's verdict (the bot's decks are better) shows up in played games,
and whether judge B's per-deck scores predict played win rates. The baseline is a weak pilot,
so this tests the decks under simple play, not under expert play.

    python scripts/engine_validate.py            # full run, results/engine_validation.json
    python scripts/engine_validate.py --smoke    # a minute-long check

The drafted decks are cached in data/engine_decks_<tables>.json (drafting takes a few minutes).
"""

import argparse
import json
import time

import bolt_engine as be
import numpy as np
import torch

from bolt.cards import CardInfo
from bolt.data import ROOT
from bolt.deckbuild import build_decks
from bolt.judge import DeckJudge
from bolt.models import DraftTransformer
from bolt.sim import opening_packs, pools_to_counts, run_tables

CK = ROOT / "checkpoints"
SEED = 12345  # the RL evaluation's tables


def load(cls, f, n):
    m = cls(n).eval()
    m.load_state_dict(torch.load(CK / f, map_location="cpu"))
    return m


def draft(tables: int, cards: CardInfo):
    """Decks (as card-number lists) and judge B scores for the bot and the human model."""
    torch.set_grad_enabled(False)
    cpu = torch.device("cpu")
    human = load(DraftTransformer, "human_model.pt", cards.n)
    judge = load(DeckJudge, "judge_b.pt", cards.n)
    packs = opening_packs("test")
    out = {}
    for name, f in (("bot", "bot.pt"), ("human_model", "human_model.pt")):
        model = load(DraftTransformer, f, cards.n)
        pools, _ = run_tables(packs, tables, model, human, cpu, torch.Generator().manual_seed(SEED), seat0_greedy=True)
        decks, wr, _ = build_decks(pools_to_counts(pools[:, 0], cards.n), judge, cards, cpu)
        lists = [np.repeat(np.arange(cards.n), d.astype(int)).tolist() for d in decks]
        assert all(len(x) == 40 for x in lists), "every built deck has 40 cards"
        out[name] = {"decks": lists, "judge_b": wr.astype(float)}
    return out


TIMEOUTS = [0]  # games stopped at the turn limit, counted as draws


def score(result: str, seat: int) -> float:
    """1 for a win by `seat`, 0.5 for a draw or a timeout, 0 for a loss."""
    if result == "timeout":
        TIMEOUTS[0] += 1
    return 0.5 if result in ("draw", "timeout") else float(result == f"p{seat}")


def match(a: list[int], b: list[int], games: int, seed: int) -> float:
    """Deck a's played win rate against deck b, alternating who plays first."""
    total = 0.0
    for k in range(games):
        result, _ = be.play_baseline(a, b, seed=seed + k, first=k % 2)
        total += score(result, 0)
    return total / games


def ranks(x: np.ndarray) -> np.ndarray:
    r = np.empty(len(x))
    r[np.argsort(x, kind="stable")] = np.arange(len(x))
    return r


def corr(x: np.ndarray, y: np.ndarray) -> dict:
    return {"pearson": float(np.corrcoef(x, y)[0, 1]), "spearman": float(np.corrcoef(ranks(x), ranks(y))[0, 1])}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tables", type=int, default=512)
    ap.add_argument("--h2h-games", type=int, default=40, help="games per head-to-head pairing")
    ap.add_argument("--field", type=int, default=64, help="human-model decks in the gauntlet field")
    ap.add_argument("--field-games", type=int, default=4, help="games against each field deck")
    ap.add_argument("--smoke", action="store_true", help="tiny run to check the plumbing")
    ap.add_argument("--out", default=str(ROOT / "results" / "engine_validation.json"))
    a = ap.parse_args()
    if a.smoke:
        a.tables, a.h2h_games, a.field, a.field_games = 16, 4, 4, 2
        a.out = str(ROOT / "results" / "engine_validation_smoke.json")

    t0 = time.time()
    cards = CardInfo()
    cache = ROOT / "data" / f"engine_decks_{a.tables}.json"
    if cache.exists():
        d = json.loads(cache.read_text())
        for side in d.values():
            side["judge_b"] = np.array(side["judge_b"])
    else:
        d = draft(a.tables, cards)
        cache.write_text(json.dumps({k: {"decks": v["decks"], "judge_b": v["judge_b"].tolist()} for k, v in d.items()}))
    bot, hum = d["bot"], d["human_model"]
    print(f"drafted and built {2 * a.tables} decks in {time.time() - t0:.0f}s", flush=True)

    # Head-to-head: the bot's deck against the human model's deck from the same packs.
    t1 = time.time()
    h2h = np.array([match(bot["decks"][t], hum["decks"][t], a.h2h_games, seed=t * 1000) for t in range(a.tables)])
    print(f"head-to-head: {a.tables * a.h2h_games} games in {time.time() - t1:.0f}s", flush=True)

    # Gauntlet: every deck against a fixed field of human-model decks from other tables.
    rng = np.random.default_rng(0)
    field = rng.choice(a.tables, size=min(a.field, a.tables), replace=False)
    t2 = time.time()
    played = {}
    for name, side in (("bot", bot), ("human_model", hum)):
        wr = np.zeros(a.tables)
        for t in range(a.tables):
            opp = [f for f in field if f != t]
            wr[t] = np.mean(
                [match(side["decks"][t], hum["decks"][f], a.field_games, seed=(t * 7919 + f) * 16) for f in opp]
            )
        played[name] = wr
    n_gauntlet = 2 * a.tables * (len(field) - 1) * a.field_games
    print(f"gauntlet: ~{n_gauntlet} games in {time.time() - t2:.0f}s", flush=True)

    gap_judge = bot["judge_b"] - hum["judge_b"]
    gap_played = played["bot"] - played["human_model"]
    all_judge = np.concatenate([bot["judge_b"], hum["judge_b"]])
    all_played = np.concatenate([played["bot"], played["human_model"]])
    quint = np.quantile(all_judge, [0.2, 0.4, 0.6, 0.8])
    bins = np.digitize(all_judge, quint)
    se = lambda x: float(x.std() / np.sqrt(len(x)))  # noqa: E731

    res = {
        "setup": {
            "tables": a.tables,
            "h2h_games_per_table": a.h2h_games,
            "field_decks": len(field),
            "games_per_field_deck": a.field_games,
            "pilot": "rules-based baseline on both sides (bolt_core::baseline)",
            "draft": "test packs, seed 12345, seat 0 greedy; decks by bolt.deckbuild, scored by judge B",
        },
        "judge_b": {
            "bot": float(bot["judge_b"].mean()),
            "human_model": float(hum["judge_b"].mean()),
            "gap": float(gap_judge.mean()),
            "gap_se": se(gap_judge),
        },
        "head_to_head": {
            "bot_win_rate": float(h2h.mean()),
            "se": se(h2h),
            "tables_bot_ahead": float((h2h > 0.5).mean()),
            "corr_with_judge_gap": corr(gap_judge, h2h),
        },
        "gauntlet": {
            "bot": float(played["bot"].mean()),
            "human_model": float(played["human_model"].mean()),
            "gap": float(gap_played.mean()),
            "gap_se": se(gap_played),
            "corr_with_judge_b": corr(all_judge, all_played),
            "by_judge_quintile": [
                {
                    "judge_b": float(all_judge[bins == q].mean()),
                    "played": float(all_played[bins == q].mean()),
                    "decks": int((bins == q).sum()),
                }
                for q in range(5)
            ],
        },
        "timeouts": TIMEOUTS[0],
        "seconds": round(time.time() - t0),
    }
    with open(a.out, "w") as f:
        json.dump(res, f, indent=2)
    print(json.dumps({k: res[k] for k in ("judge_b", "head_to_head", "gauntlet")}, indent=2))


if __name__ == "__main__":
    main()
