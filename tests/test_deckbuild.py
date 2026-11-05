import numpy as np
import torch
from conftest import ValueJudge

from bolt.deckbuild import build_decks


def make_pool(cards, per_card: dict[str, int]) -> np.ndarray:
    pool = np.zeros((1, cards.n), np.float32)
    for name, k in per_card.items():
        pool[0, cards.index[name]] = k
    return pool


def test_builds_a_legal_40_card_deck_in_the_best_colors(cards):
    # A strong white-blue pool with a few off-color cards mixed in.
    pool = make_pool(
        cards,
        {
            **{f"W Creature {i}": 2 for i in range(6)},
            **{f"U Creature {i}": 2 for i in range(6)},
            "R Creature 0": 1,
            "G Creature 0": 1,
            "WU Dual": 1,
            "Golem": 1,
        },
    )
    values = torch.zeros(cards.n)
    for i, name in enumerate(cards.names):
        values[i] = 3.0 if name[0] in "WU" and "Creature" in name else 1.0 if "Creature" in name else 0.0
    deck, wr, labels = build_decks(pool, ValueJudge(values), cards, torch.device("cpu"))
    deck = deck[0]

    assert deck.sum() == 40
    assert labels[0].startswith("WU")
    lands = deck[cards.land].sum()
    assert lands in (16, 17)
    spells = np.flatnonzero(deck * ~cards.land)
    assert all(cards.castable_in("WU")[s] for s in spells)  # nothing off-color made the deck
    nonbasic = ~cards.basic
    assert (deck[nonbasic] <= pool[0][nonbasic]).all()  # never plays a card it didn't draft
    assert deck[cards.index["WU Dual"]] == 1  # on-color dual land counts as a land


def test_short_pools_are_padded_with_basics_to_40(cards):
    pool = make_pool(cards, {"W Creature 1": 1, "U Creature 1": 1})
    deck, _, _ = build_decks(pool, ValueJudge(torch.ones(cards.n)), cards, torch.device("cpu"))
    assert deck[0].sum() == 40
    assert deck[0][~cards.land].sum() == 2
