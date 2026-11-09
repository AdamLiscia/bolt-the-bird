"""Is a played-game reward steady enough to learn from? Measure before training on it.

Drafts T practice tables (every seat the human model, seat 0 sampled as in training), builds all
8 decks, then plays seat 0's deck against its 7 tablemates G times each with the smart pilot on
both sides. That is done twice with independent seeds. If a deck's reward in the first half
predicts its reward in the second, the reward carries signal the bot can learn from; the
split-half correlation says how much. Also reports how the played reward tracks judge A, and the
seconds per batch (one training iteration's games).

    python scripts/played_reward_check.py --tables 128 --games 2
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


def load(cls, f, n):
    m = cls(n)
    m.load_state_dict(torch.load(CK / f, map_location="cpu"))
    return m.eval()


def deck_lists(counts):
    return [np.repeat(np.arange(len(c)), c.astype(int)).tolist() for c in counts]


def played_reward(decks, games, seed, rollouts, candidates, threads):
    """decks[t][s]: seat s's deck at table t. Seat 0's win rate against seats 1-7, `games` each."""
    jobs, owner = [], []
    for t, table in enumerate(decks):
        for s in range(1, 8):
            for k in range(games):
                jobs.append((table[0], table[s], seed + 1000 * t + 10 * s + k, k % 2))
                owner.append(t)
    res = be.play_games(jobs, pilot="smart", rollouts=rollouts, candidates=candidates, threads=threads)
    score = np.array([{"p0": 1.0, "p1": 0.0}.get(r, 0.5) for r, _ in res])
    owner = np.array(owner)
    errors = sum(r == "error" for r, _ in res)
    return np.array([score[owner == t].mean() for t in range(len(decks))]), errors


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tables", type=int, default=128)
    ap.add_argument("--games", type=int, default=2, help="games against each tablemate, per half")
    ap.add_argument("--rollouts", type=int, default=1)
    ap.add_argument("--candidates", type=int, default=2)
    ap.add_argument("--threads", type=int, default=8)
    a = ap.parse_args()
    torch.manual_seed(0)
    cards = CardInfo()
    cpu = torch.device("cpu")
    ref = load(DraftTransformer, "transformer.pt", cards.n)
    judge_a = load(DeckJudge, "judge_h0.pt", cards.n)
    t0 = time.time()
    pools, _ = run_tables(opening_packs("train"), a.tables, ref, ref, cpu, torch.Generator().manual_seed(1))
    counts = pools_to_counts(pools.reshape(-1, 42), cards.n)
    built, wr_a, _ = build_decks(counts, judge_a, cards, cpu)
    lists = deck_lists(built)
    decks = [lists[8 * t : 8 * t + 8] for t in range(a.tables)]
    assert all(len(d) == 40 for d in lists)
    print(f"drafted and built {len(lists)} decks in {time.time() - t0:.0f}s", flush=True)
    halves = []
    for h in range(2):
        t1 = time.time()
        r, err = played_reward(decks, a.games, 10_000_000 * (h + 1), a.rollouts, a.candidates, a.threads)
        secs = time.time() - t1
        print(f"half {h}: {7 * a.games * a.tables} games in {secs:.0f}s, mean reward {r.mean():.3f}, errors {err}", flush=True)
        halves.append((r, secs))
    r0, r1 = halves[0][0], halves[1][0]
    split = float(np.corrcoef(r0, r1)[0, 1])
    full = 2 * split / (1 + split)  # Spearman-Brown: reliability of the two halves together
    judge0 = wr_a.reshape(a.tables, 8)[:, 0]
    out = {
        "tables": a.tables,
        "games_per_tablemate_per_half": a.games,
        "pilot": f"smart {a.rollouts} x {a.candidates}",
        "seconds_per_half": [round(s) for _, s in halves],
        "reward_sd_across_tables": float(np.concatenate([r0, r1]).std()),
        "split_half_corr": split,
        "reliability_both_halves": full,
        "corr_with_judge_a": float(np.corrcoef((r0 + r1) / 2, judge0)[0, 1]),
    }
    print(json.dumps(out, indent=2))
    (ROOT / "results" / "played_reward_check.json").write_text(json.dumps(out, indent=2) + "\n")


if __name__ == "__main__":
    main()
