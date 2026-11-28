"""Per-deck results from a trainer's game log (results/games_<run>.jsonl, train_ppo_play.GameLog).

    PYTHONPATH=src python scripts/game_report.py results/games_cmd_play_1.jsonl [--phase self] [--min-casts 5]

For each deck (by its line in the deck file): games, win rate, average turn of its wins, and how its games ended;
then its 5 best and 5 worst cards by win rate in the games where it cast or played them (at least --min-casts
such games), names from be.card_names(--set).
"""

import argparse
import collections
import json
from pathlib import Path


def load(path, phases=None) -> list[dict]:
    rows = [json.loads(x) for x in Path(path).read_text().splitlines() if x.strip()]
    return [r for r in rows if not phases or r["phase"] in phases]


def report(rows: list[dict], names: list[str], min_casts: int = 5) -> dict:
    """{deck key: {"name", "games", "win_rate", "avg_win_turn", "ends", "best", "worst"}}; best and worst are
    [(card name, games cast, win rate)]."""
    decks: dict = {}
    for r in rows:
        winner = {"p0": 0, "p1": 1}.get(r["result"])
        for seat, side in ((0, "deck_a"), (1, "deck_b")):
            d = decks.setdefault(r[side]["key"], {"name": r[side].get("commander"), "games": 0, "wins": 0.0,
                                                  "win_turns": [], "ends": collections.Counter(),
                                                  "cards": collections.defaultdict(lambda: [0, 0.0])})  # fmt: skip
            d["games"] += 1
            won = 1.0 if winner == seat else 0.5 if winner is None else 0.0
            d["wins"] += won
            if winner == seat:
                d["win_turns"].append(r["turn"])
            d["ends"][r.get("end_reason", "?")] += 1
            for c in {c for _t, c in (r.get("cards_played") or [[], []])[seat]}:
                d["cards"][c][0] += 1
                d["cards"][c][1] += won
    out = {}
    for k, d in sorted(decks.items()):
        rated = [(names[c] if 0 <= c < len(names) else str(c), n, w / n) for c, (n, w) in d["cards"].items()
                 if n >= min_casts]  # fmt: skip
        rated.sort(key=lambda x: (-x[2], -x[1], x[0]))
        out[k] = {"name": d["name"], "games": d["games"], "win_rate": d["wins"] / d["games"],
                  "avg_win_turn": sum(d["win_turns"]) / len(d["win_turns"]) if d["win_turns"] else None,
                  "ends": dict(d["ends"].most_common()), "best": rated[:5],
                  "worst": list(reversed(rated[-5:])) if len(rated) > 5 else []}  # fmt: skip
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("games")
    ap.add_argument("--phase", nargs="*", help="self, opp, eval_rules, eval_smart (default: all)")
    ap.add_argument("--min-casts", type=int, default=5)
    ap.add_argument("--set", default="CMD")
    a = ap.parse_args()
    import bolt_engine as be

    names = list(be.card_names(a.set))
    rows = load(a.games, a.phase)
    print(f"{len(rows)} games")
    for k, d in report(rows, names, a.min_casts).items():
        turn = f"{d['avg_win_turn']:.1f}" if d["avg_win_turn"] is not None else "-"
        print(f"\n[{k}] {d['name']}: {d['games']} games, win rate {d['win_rate']:.3f}, average win turn {turn}")
        print("   ends: " + ", ".join(f"{e} {n}" for e, n in d["ends"].items()))
        for label, cards in (("best", d["best"]), ("worst", d["worst"])):
            if cards:
                print(f"   {label}: " + "; ".join(f"{n} {w:.2f} ({g})" for n, g, w in cards))


if __name__ == "__main__":
    main()
