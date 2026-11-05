"""Pieces of the reinforcement-learning loop that are worth testing on their own."""

import torch


def masked_kl(logits: torch.Tensor, ref_logits: torch.Tensor) -> torch.Tensor:
    """KL(policy || ref) per row, over the cards actually in the pack.

    Cards not in the pack have logit -inf. The out-of-pack slots are zeroed *before* any arithmetic:
    -inf - -inf is NaN, and torch.where only hides that in the forward pass. Its gradient still comes
    back NaN, and on Apple GPUs a NaN-poisoned model hung the machine (2026-11-04).
    """
    valid = torch.isfinite(logits)
    lp = torch.log_softmax(logits, 1).masked_fill(~valid, 0.0)
    lq = torch.log_softmax(ref_logits, 1).masked_fill(~valid, 0.0)
    return (lp.exp() * valid * (lp - lq)).sum(1)
