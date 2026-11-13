import torch

from bolt.judge import DeckJudge
from bolt.rl import masked_kl

INF = float("-inf")


def test_kl_is_zero_for_identical_policies():
    logits = torch.tensor([[1.0, 2.0, INF, 0.5]])
    assert masked_kl(logits, logits.clone()).item() == 0.0


def test_kl_matches_the_textbook_formula():
    a = torch.tensor([[1.0, 2.0, INF]])
    b = torch.tensor([[2.0, 0.0, INF]])
    p, q = torch.softmax(a[:, :2], 1), torch.softmax(b[:, :2], 1)
    expected = (p * (p.log() - q.log())).sum()
    assert torch.allclose(masked_kl(a, b), expected)


def test_kl_gradient_stays_finite_with_masked_cards():
    """Regression test: the first version produced NaN gradients here and hung the GPU."""
    logits = torch.tensor([[1.0, INF, 2.0, INF]], requires_grad=True)
    ref = torch.tensor([[0.5, INF, 1.0, INF]])
    masked_kl(logits * 1.5, ref).sum().backward()
    assert torch.isfinite(logits.grad).all()


def test_judge_win_rate_is_a_probability_and_ignores_pilot_skill():
    torch.manual_seed(0)
    judge = DeckJudge(20).eval()
    decks = torch.zeros(3, 20)
    decks[:, :10] = 2.0
    decks[1, 0] = 4.0
    wr = judge.win_rate(decks)
    assert ((wr > 0) & (wr < 1)).all()
    assert wr[0] == wr[2]  # same deck, same score: skill is held fixed inside win_rate


# ---------------------------------------------------------------- PPO

from bolt.rl import ppo_loss  # noqa: E402


def _batch(seed=0, n=6, cards=10):
    g = torch.Generator().manual_seed(seed)
    logits = torch.randn(n, cards, generator=g)
    logits[:, 7:] = float("-inf")  # cards not in the pack
    act = torch.randint(0, 7, (n,), generator=g)
    adv = torch.randn(n, generator=g)
    return logits, act, adv


def test_ppo_matches_reinforce_before_the_policy_moves():
    logits, act, adv = _batch()
    ref = logits.clone()
    a = logits.clone().requires_grad_(True)
    old_lp = torch.log_softmax(logits, 1).gather(1, act[:, None]).squeeze(1)
    loss, _ = ppo_loss(a, ref, act, old_lp, adv, clip=0.2, kl_coef=0.0)
    loss.backward()
    b = logits.clone().requires_grad_(True)
    lp = torch.log_softmax(b, 1).gather(1, act[:, None]).squeeze(1)
    (-(adv * lp).mean()).backward()
    assert torch.allclose(a.grad, b.grad, atol=1e-6)


def test_ppo_stops_pushing_a_pick_that_already_moved_too_far():
    logits, act, _ = _batch(1, n=1)
    adv = torch.ones(1)
    old_lp = torch.log_softmax(logits, 1).gather(1, act[:, None]).squeeze(1) - 1.0  # ratio e^1 > 1.2
    a = logits.clone().requires_grad_(True)
    loss, stats = ppo_loss(a, logits, act, old_lp, adv, clip=0.2, kl_coef=0.0)
    loss.backward()
    assert torch.count_nonzero(a.grad[:, :7]) == 0, "clipped: no gradient past 1 + clip"
    assert stats["clipped"] == 1.0


def test_ppo_gradient_stays_finite_with_masked_cards():
    logits, act, adv = _batch(2)
    a = logits.clone().requires_grad_(True)
    old_lp = torch.log_softmax(logits, 1).gather(1, act[:, None]).squeeze(1)
    loss, _ = ppo_loss(a, logits + 0.3, act, old_lp, adv, clip=0.2, kl_coef=0.05)
    loss.backward()
    assert torch.isfinite(loss) and torch.isfinite(a.grad).all()
