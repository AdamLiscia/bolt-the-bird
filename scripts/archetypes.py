"""Which color pairs win, and which ones does the RL bot draft?

Part 1 uses real 17lands games. A deck's pair is the two colors with the most nonland cards, kept
only when the second color has at least 4. Win rate is compared to the pilots' own overall win rate,
so a pair that strong players favor doesn't look good just because of who plays it.

Part 2 drafts 256 held-out tables with each drafter in seat 0 (same packs and seed for all three),
then builds and scores the decks with judge B, and reports their shape.

    python scripts/archetypes.py
"""

import collections
import json

import numpy as np
import torch

from bolt.cards import COLORS, CardInfo
from bolt.data import ASSETS, ROOT, load_games
from bolt.deckbuild import build_decks
from bolt.judge import DeckJudge
from bolt.models import DraftTransformer
from bolt.sim import pools_to_counts, run_tables

CK = ROOT / "checkpoints"
DRAFTERS = {"human_model": "human_model.pt", "bot_iter150": "bot_rl.pt", "bot_iter550": "bot_rl_long.pt"}
N_TABLES, SEED = 256, 7


def pair_of(color_counts: np.ndarray) -> np.ndarray:
    top = np.argsort(-color_counts, 1, kind="stable")[:, :2]
    return np.array(["".join(sorted(COLORS[a] + COLORS[b], key=COLORS.index)) for a, b in top])


def real_games(cards: CardInfo) -> dict:
    g = load_games()
    by_color = np.array([[col in (c["color"] or "") for col in COLORS] for c in cards.info], dtype=np.int32)
    counts = (g["deck"] * ~cards.land).astype(np.int32) @ by_color
    two_color = np.sort(counts, 1)[:, -2] >= 4
    pair = pair_of(counts)
    won = g["won"].astype(float)
    skill = np.nan_to_num(g["user_wr"], nan=float(np.nanmean(g["user_wr"])))
    out = {}
    for p in sorted(set(pair[two_color])):
        m = two_color & (pair == p)
        out[p] = {
            "games": int(m.sum()),
            "share": float(m.sum() / two_color.sum()),
            "win_rate": float(won[m].mean()),
            "vs_pilot_skill": float(won[m].mean() - skill[m].mean()),
        }
    return dict(sorted(out.items(), key=lambda kv: -kv[1]["vs_pilot_skill"]))


def drafted_decks(cards: CardInfo) -> dict:
    torch.set_grad_enabled(False)
    cpu = torch.device("cpu")

    def load(f):
        m = DraftTransformer(cards.n).eval()
        m.load_state_dict(torch.load(CK / f, map_location="cpu"))
        return m

    human = load("human_model.pt")
    judge = DeckJudge(cards.n).eval()
    judge.load_state_dict(torch.load(CK / "judge_b.pt", map_location="cpu"))
    creature = np.array(["Creature" in " ".join(c["types"]) for c in cards.info])
    rare = np.array([c["rarity"] in ("rare", "mythic") for c in cards.info])
    packs = np.load(ASSETS / "FDN_sample_packs.npy")
    out = {}
    for name, f in DRAFTERS.items():
        pools, _ = run_tables(packs, N_TABLES, load(f), human, cpu, torch.Generator().manual_seed(SEED),
                              seat0_greedy=True)  # fmt: skip
        decks, wr, labels = build_decks(pools_to_counts(pools[:, :1], cards.n)[:, 0], judge, cards, cpu)
        decks = np.asarray(decks)
        pairs = collections.Counter(lab.split()[0] for lab in labels)
        out[name] = {
            "judged_win_rate": float(np.mean(wr)),
            "creatures": float((decks * creature).sum(1).mean()),
            "lands": float((decks * cards.land).sum(1).mean()),
            "rares_in_deck": float((decks * (rare & ~cards.land)).sum(1).mean()),
            "pair_share": {p: c / N_TABLES for p, c in pairs.most_common()},
        }
    return out


def main():
    cards = CardInfo()
    res = {"real_games": real_games(cards), "drafted": drafted_decks(cards), "tables": N_TABLES, "seed": SEED}
    (ROOT / "results" / "archetypes.json").write_text(json.dumps(res, indent=1) + "\n")
    for p, r in res["real_games"].items():
        print(f"{p}  {r['games']:7d} games  {r['share']:5.1%} of games  win {r['win_rate']:.1%}  "
              f"vs pilot skill {100 * r['vs_pilot_skill']:+.1f}")  # fmt: skip
    for name, r in res["drafted"].items():
        top = ", ".join(f"{p} {s:.0%}" for p, s in list(r["pair_share"].items())[:3])
        print(f"{name:12} judged {r['judged_win_rate']:.1%}  creatures {r['creatures']:.1f}  "
              f"lands {r['lands']:.1f}  rares {r['rares_in_deck']:.2f}  {top}")  # fmt: skip


if __name__ == "__main__":
    main()
