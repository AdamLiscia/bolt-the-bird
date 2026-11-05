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
