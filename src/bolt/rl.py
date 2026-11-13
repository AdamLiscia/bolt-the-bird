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


def ppo_loss(logits, ref_logits, act, old_lp, adv, clip: float, kl_coef: float):
    """PPO's clipped objective for one minibatch of picks, plus the same KL leash to the human model.

    `old_lp` is each chosen card's log-probability when the drafts were played. The ratio between
    now and then is clipped to [1 - clip, 1 + clip], so a pick whose probability has already moved
    that far gets no further push from this batch. That is what lets PPO take several steps on the
    same games safely. With the policy unchanged (ratio 1) the gradient is exactly REINFORCE's.
    Returns (loss, stats) with the share of clipped picks and the KL to the human model.
    """
    lp = torch.log_softmax(logits, 1).gather(1, act[:, None]).squeeze(1)  # the chosen card is in the pack
    ratio = torch.exp(lp - old_lp)
    surrogate = torch.minimum(ratio * adv, torch.clamp(ratio, 1 - clip, 1 + clip) * adv)
    kl = masked_kl(logits, ref_logits)
    loss = (-surrogate + kl_coef * kl).mean()
    with torch.no_grad():
        clipped = ((ratio - 1).abs() > clip).float().mean().item()
    return loss, {"clipped": clipped, "kl": kl.mean().item()}
