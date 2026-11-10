"""The practice draft: one human seat at a table of bots, pick by pick, with Bolt as a coach."""

import numpy as np
import pytest
import torch

from bolt.models import to_vocab
from bolt.practice import PracticeDraft

N_CARDS = 40


class LowestFirst(torch.nn.Module):
    """A stand-in draft model: always prefers the lowest card id in the pack."""

    def forward(self, pool, pack, pack_number, pick_number):
        return to_vocab(-pack.float(), pack, N_CARDS)


def packs(n=30, seed=0):
    rng = np.random.default_rng(seed)
    out = np.zeros((n, 15), dtype=np.int64)
    for i in range(n):
        out[i, :14] = rng.choice(N_CARDS, 14) + 1  # ids+1, duplicates allowed like real packs
    return out


def new(seed=1):
    return PracticeDraft(packs(), bots=LowestFirst(), coach=LowestFirst(), n_cards=N_CARDS, seed=seed)


def test_a_full_draft_is_42_picks_and_every_seat_ends_with_42_cards():
    d = new()
    shown = []
    while not d.done:
        pack = d.human_pack()
        shown.append(pack)
        d.pick(len(pack) - 1)  # take the last card shown
    assert len(shown) == 42
    assert [len(p) for p in shown[:14]] == list(range(14, 0, -1)), "packs shrink by one each pick"
    pools = d.pools()
    assert all(len(p) == 42 for p in pools)
    assert pools[0] == [p[-1] for p in shown], "your pool is exactly what you took"


def test_packs_pass_left_then_right_then_left():
    d = new()
    first_round = d.packs_by_round[0]  # (8, 15) ids+1: seat s opens first_round[s]
    seat7 = [int(c) - 1 for c in first_round[7] if c]
    d.pick(0)
    # Round 1 passes left: you get the pack seat 7 opened, minus the card seat 7 took.
    got = d.human_pack()
    assert len(got) == 13 and all(got.count(c) <= seat7.count(c) for c in got)
    while d.round == 0:
        d.pick(0)
    second = d.packs_by_round[1]
    seat1 = [int(c) - 1 for c in second[1] if c]
    d.pick(0)
    # Round 2 passes right: you get seat 1's pack.
    got = d.human_pack()
    assert all(got.count(c) <= seat1.count(c) for c in got)


def test_the_coach_scores_every_card_in_your_pack():
    d = new()
    probs = d.coach()
    assert len(probs) == len(d.human_pack())
    assert abs(sum(probs) - 1) < 1e-5
    pack = d.human_pack()
    assert probs.index(max(probs)) == pack.index(min(pack)), "the stand-in coach likes low ids"


def test_each_pick_is_logged_with_the_coachs_view():
    d = new()
    pack = d.human_pack()
    d.pick(2)
    entry = d.log[0]
    assert entry["pack"] == pack and entry["took"] == pack[2]
    assert entry["coach_top"] == min(pack)
    assert 0 <= entry["coach_prob_took"] <= entry["coach_prob_top"] <= 1


def test_same_seed_same_draft():
    a, b = new(5), new(5)
    for _ in range(42):
        assert a.human_pack() == b.human_pack()
        a.pick(0)
        b.pick(0)
    assert a.pools() == b.pools()


def test_a_bad_pick_is_rejected_and_changes_nothing():
    d = new()
    before = d.human_pack()
    with pytest.raises(ValueError):
        d.pick(14)
    with pytest.raises(ValueError):
        d.pick(-1)
    assert d.human_pack() == before


def test_bolt_in_your_seat_redrafts_from_the_same_opening_packs():
    d = new(3)
    bolt = d.redraft_with_coach()
    assert bolt.packs_by_round[0].tolist() == d.packs_by_round[0].tolist()
    assert bolt.done and len(bolt.pools()[0]) == 42
    assert all(e["took"] == e["coach_top"] for e in bolt.log), "Bolt takes its own top pick"


# ---------------------------------------------------------------- decks and grades

from bolt.practice import check_deck, grade  # noqa: E402

BASICS = {0, 1}  # stand-in basic lands: any number allowed


def test_a_deck_uses_only_your_pool_plus_basics_and_has_40_cards():
    pool = [5, 5, 6, 7] + list(range(10, 36))
    deck = [5, 5, 6] + list(range(10, 30)) + [0] * 9 + [1] * 8
    assert check_deck(deck, pool, BASICS) == []
    assert check_deck(deck[:-1], pool, BASICS), "39 cards is too few"
    assert check_deck(deck[:-1] + [5], pool, BASICS), "a third copy you didn't draft"
    assert check_deck(deck[:-1] + [38], pool, BASICS), "a card you never drafted"


def fake_play(jobs, **_):
    """Player 0 wins exactly when its deck contains card 99 (the 'good' card)."""
    return [("p0" if 99 in a else "p1", 10) for a, _b, _s, _f in jobs]


def test_grades_compare_your_deck_and_bolts_against_the_same_table():
    d = new(4)
    while not d.done:
        d.pick(0)
    table = [[1] * 40 for _ in range(7)]
    yours, bolts = [2] * 40, [99] + [2] * 39
    g = grade(d, yours, table, bolts, games=4, play=fake_play)
    assert g["you"]["win_rate"] == 0.0 and g["bolt"]["win_rate"] == 1.0
    assert g["you"]["games"] == 28 and g["bolt"]["games"] == 28
    assert g["difference"]["points"] == pytest.approx(-100.0)
    assert len(g["picks"]) == 42 and 0 <= g["agreement"] <= 1
    assert g["you"]["low"] <= g["you"]["win_rate"] <= g["you"]["high"]


def test_grades_pair_the_games_same_opponents_and_seeds():
    seen = []

    def record(jobs, **_):
        seen.append(jobs)
        return [("p0", 9)] * len(jobs)

    d = new(6)
    while not d.done:
        d.pick(0)
    grade(d, [2] * 40, [[1] * 40] * 7, [3] * 40, games=2, play=record)
    jobs = seen[0]
    half = len(jobs) // 2
    assert [(b, s, f) for _a, b, s, f in jobs[:half]] == [(b, s, f) for _a, b, s, f in jobs[half:]]
