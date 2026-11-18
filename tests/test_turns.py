"""Targets, baselines and metrics for the turn-level imitation tasks (Phase 4, P1)."""

import numpy as np

from bolt.turns import CardStats, baseline_cast, mark, metrics, split_of

# Card numbers + 1 as stored in the shards (0 = empty slot).
LION, ANGEL, GROWTH, PLAINS = 2, 4, 3, 1


def test_mark_flags_each_chosen_card_once_even_with_copies():
    slots = np.array([[LION, LION, ANGEL, 0], [PLAINS, GROWTH, 0, 0]], dtype=np.int16)
    chosen = np.array([[LION, 0], [GROWTH, PLAINS]], dtype=np.int16)
    m = mark(slots, chosen)
    assert m.tolist() == [[True, False, False, False], [True, True, False, False]]


def test_a_chosen_card_that_is_not_in_the_slots_marks_nothing():
    m = mark(np.array([[LION, 0]], dtype=np.int16), np.array([[ANGEL]], dtype=np.int16))
    assert not m.any()


def stats():
    # card number (0-based) → mana value and land flag; index 0 of the arrays is the empty slot
    cmc = np.array([0, 0, 1, 1, 5], dtype=np.int16)  # empty, Plains, Lion, Growth, Angel
    land = np.array([False, True, False, False, False])
    return CardStats(cmc, land)


def test_baseline_plays_a_land_then_casts_the_most_expensive_spells_it_can_afford():
    hand = np.array([[PLAINS, LION, ANGEL, GROWTH, 0]], dtype=np.int16)
    lands = np.array([[PLAINS, 0, 0, 0]], dtype=np.int16)  # one land in play + one to play = 2 mana
    pred = baseline_cast(hand, lands, stats())
    assert pred.tolist() == [[True, True, False, True, False]], "land, then Lion and Growth (Angel costs 5)"


def test_metrics_score_valid_slots_and_whole_turns():
    slots = np.array([[1, 2, 0], [3, 0, 0]], dtype=np.int16)
    target = np.array([[True, False, False], [True, False, False]])
    pred = np.array([[True, True, True], [True, False, False]])
    m = metrics(pred, target, slots)
    assert m["slot_accuracy"] == 2 / 3, "padding is ignored: 2 of 3 real slots right"
    assert m["turn_exact"] == 0.5


def test_split_is_by_game():
    g = np.arange(40)
    s = split_of(g)
    assert set(s[g % 20 == 0]) == {"test"} and set(s[g % 20 == 1]) == {"valid"}
    assert (s == "train").sum() == 36
