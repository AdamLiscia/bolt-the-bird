"""Context-free baselines: bots that rank cards the same way no matter what you've already drafted.

python scripts/baselines.py
"""

import json

import numpy as np

from bolt.data import DATA, load_cards, load_games, load_picks, subset
from bolt.evaluate import pick_from_scores, report


def gih_wr(games: dict, n_cards: int, prior_games: int = 200) -> np.ndarray:
    """Games-in-hand win rate per card (the stat 17lands drafters sort by), shrunk toward the
    mean so a card seen in 30 games can't top the list on luck."""
    in_hand = games["in_hand"][:, :n_cards] > 0
    won = games["won"].astype(np.float64)
    n = in_hand.sum(0)
    w = (in_hand * won[:, None]).sum(0)
    base = won.mean()
    return (w + prior_games * base) / (n + prior_games)


def take_rate(picks: dict, n_cards: int) -> np.ndarray:
    """How often a card is taken when it's in the pack: drafters' revealed pick order."""
    seen = np.bincount((picks["pack"][picks["pack"] > 0] - 1).astype(np.int64), minlength=n_cards)
    taken = np.bincount(picks["pick"].astype(np.int64), minlength=n_cards)
    return (taken + 1) / (seen + 20)


def main():
    cards = load_cards()
    n = len(cards)
    picks = load_picks()
    games = load_games()
    train, test = subset(picks, picks["split"] == "train"), subset(picks, picks["split"] == "test")
    gtrain = subset(games, games["split"] == "train")
    print(f"train picks {len(train['pick']):,}  test picks {len(test['pick']):,}  train games {len(gtrain['won']):,}\n")

    rng = np.random.default_rng(0)
    results = [
        report("random", pick_from_scores(rng.random((len(test["pick"]), n)), test["pack"]), test),
        report("ratings bot (GIH WR)", pick_from_scores(gih_wr(gtrain, n), test["pack"]), test),
        report("pick-order bot", pick_from_scores(take_rate(train, n), test["pack"]), test),
    ]
    out = DATA.parent / "results"
    out.mkdir(exist_ok=True)
    (out / "baselines.json").write_text(json.dumps(results, indent=1))


if __name__ == "__main__":
    main()
