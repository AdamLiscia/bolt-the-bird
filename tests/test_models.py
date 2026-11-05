import numpy as np
import pytest
import torch

from bolt.evaluate import pick_from_scores
from bolt.models import DraftTransformer, PoolMLP, to_vocab

N = 30


def batch(b=4, seed=0):
    g = torch.Generator().manual_seed(seed)
    pool = torch.zeros(b, 42, dtype=torch.int16)
    pool[:, :10] = torch.randint(1, N + 1, (b, 10), generator=g, dtype=torch.int16)
    pack = torch.zeros(b, 15, dtype=torch.int16)
    pack[:, :8] = torch.randint(1, N + 1, (b, 8), generator=g, dtype=torch.int16)
    pn = torch.ones(b, dtype=torch.int8)
    pk = torch.full((b,), 3, dtype=torch.int8)
    return pool, pack, pn, pk


def test_to_vocab_masks_cards_not_in_the_pack():
    pack = torch.tensor([[3, 5, 0]])
    out = to_vocab(torch.tensor([[1.0, 2.0, 9.0]]), pack, 6)
    assert out[0, 2] == 1.0 and out[0, 4] == 2.0  # card ids 2 and 4 (stored +1)
    assert torch.isinf(out[0, [0, 1, 3, 5]]).all()  # padding slot's 9.0 never leaks in


@pytest.mark.parametrize("cls", [PoolMLP, DraftTransformer])
def test_models_only_score_cards_in_the_pack(cls):
    torch.manual_seed(0)
    model = cls(N).eval()
    pool, pack, pn, pk = batch()
    with torch.no_grad():
        logits = model(pool, pack, pn, pk)
    assert logits.shape == (4, N)
    for row, p in zip(logits, pack, strict=True):
        in_pack = set((p[p > 0] - 1).tolist())
        finite = set(torch.isfinite(row).nonzero().flatten().tolist())
        assert finite == in_pack


def test_transformer_treats_the_pool_as_a_set():
    torch.manual_seed(0)
    model = DraftTransformer(N).eval()
    pool, pack, pn, pk = batch()
    shuffled = pool[:, torch.randperm(42)]
    with torch.no_grad():
        a, b = model(pool, pack, pn, pk), model(shuffled, pack, pn, pk)
    assert torch.allclose(a[torch.isfinite(a)], b[torch.isfinite(b)], atol=1e-5)


def test_pick_from_scores_ignores_padding_and_takes_the_best_card():
    pack = np.array([[2, 4, 0], [1, 0, 0]])
    scores = np.array([0.0, 0.5, 0.1, 0.9, 5.0])  # card 4 (id+1 = 5) is best overall but not offered
    assert pick_from_scores(scores, pack).tolist() == [3, 0]
