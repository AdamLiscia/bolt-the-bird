import numpy as np
import torch
from conftest import RandomDrafter

from bolt.sim import PACK, ROUNDS, SEATS, pools_to_counts, run_tables

N = 30


def one_pack() -> np.ndarray:
    pack = np.zeros((1, 15), np.int16)
    pack[0, :PACK] = np.arange(1, PACK + 1)  # cards 0..13, one each
    return pack


def draft(tables=3, seed=0, greedy=False):
    gen = torch.Generator().manual_seed(seed)
    return run_tables(
        one_pack(), tables, RandomDrafter(N, 1), RandomDrafter(N, 2), torch.device("cpu"), gen, seat0_greedy=greedy
    )


def test_every_seat_drafts_42_cards():
    pools, _ = draft()
    assert pools.shape == (3, SEATS, ROUNDS * PACK)
    assert ((pools > 0).sum(-1) == 42).all()


def test_no_card_is_created_or_lost():
    # Every pack is the same 14 cards, so each table must end with exactly 24 copies of each.
    pools, _ = draft()
    counts = pools_to_counts(pools, N).sum(1)  # (tables, cards)
    assert (counts[:, :PACK] == SEATS * ROUNDS).all()
    assert (counts[:, PACK:] == 0).all()


def test_seat_zero_log_lines_up_with_its_pool():
    pools, log = draft()
    actions = log["action"].reshape(3, ROUNDS * PACK)
    assert torch.equal(actions + 1, pools[:, 0].long())
    assert log["pool"].shape == (3 * 42, 42)
    first_pick_pack = log["pack"].reshape(3, 42, -1)[:, 0]
    assert ((first_pick_pack > 0).sum(-1) == PACK).all()


def test_same_seed_same_draft():
    a, _ = draft(seed=5, greedy=True)
    b, _ = draft(seed=5, greedy=True)
    assert torch.equal(a, b)
