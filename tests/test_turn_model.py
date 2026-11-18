"""The turn model: tagged card tokens in, one score per slot of the target zone out."""

import torch

from bolt.turn_model import ZONES, TurnModel

N_CARDS = 50


def batch(b=3, seed=0):
    g = torch.Generator().manual_seed(seed)
    z = {}
    for name in ZONES:
        w = 6
        x = torch.randint(1, N_CARDS + 1, (b, w), generator=g)
        x[:, 4:] = 0  # two empty slots per zone
        z[name] = x
    scalars = torch.rand(b, 5, generator=g)
    return z, scalars


def model():
    torch.manual_seed(0)
    return TurnModel(N_CARDS, dim=32, layers=2, heads=4).eval()


def test_one_score_per_target_slot_and_empty_slots_are_masked():
    m = model()
    z, s = batch()
    out = m(z, s, target="hand")
    assert out.shape == z["hand"].shape
    assert torch.isinf(out[:, 4:]).all() and (out[:, 4:] < 0).all(), "empty slots score -inf"
    assert torch.isfinite(out[:, :4]).all()


def test_the_order_of_cards_does_not_matter():
    m = model()
    z, s = batch()
    perm = torch.tensor([2, 0, 3, 1, 4, 5])
    z2 = dict(z)
    z2["hand"] = z["hand"][:, perm]
    z2["opp_creatures"] = z["opp_creatures"][:, torch.tensor([3, 1, 0, 2, 4, 5])]
    a = m(z, s, target="hand")
    b = m(z2, s, target="hand")
    assert torch.allclose(a[:, perm][:, :4], b[:, :4], atol=1e-5)


def test_the_target_zone_can_be_any_zone():
    m = model()
    z, s = batch()
    assert m(z, s, target="my_creatures").shape == z["my_creatures"].shape


def test_loss_ignores_empty_slots_and_has_finite_gradients():
    from bolt.turn_model import slot_loss

    m = model().train()
    z, s = batch()
    logits = m(z, s, target="hand")
    target = torch.zeros_like(logits, dtype=torch.bool)
    target[:, 0] = True
    loss = slot_loss(logits, target, z["hand"] > 0)
    loss.backward()
    assert torch.isfinite(loss)
    assert all(torch.isfinite(p.grad).all() for p in m.parameters() if p.grad is not None)
