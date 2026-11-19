"""Self-play PPO for the play network (Phase 4, P3d; docs/play/P3_DESIGN.md).

Reward: +1 for a win, -1 for a loss, 0 otherwise, at the end of the game only. Each recorded move's
advantage is the result minus the value estimate made when it was played (no bootstrapping:
gamma = lambda = 1, so the only signal is who won). The update is the drafter's clipped PPO
objective (bolt.rl.ppo_loss) with a KL leash to a fixed reference policy (the distilled network),
plus value regression to the result.
"""

import torch

from bolt.play_data import batch, groups
from bolt.rl import ppo_loss

FIXED = (("net", "rules"), ("rules", "net"), ("net", "smart"), ("smart", "net"))


def league(n_games: int, past: list) -> list:
    """(seats, games) for one iteration: a quarter against the rules-based player, a quarter against the
    smart player, a quarter against past checkpoints (seat "opp"), a quarter self-play (both seats
    recorded); with no past checkpoints yet, self-play takes that quarter too. Every pairing is
    played from both seats."""
    parts = {s: n_games // 8 for s in FIXED}
    if past:
        parts[("net", "opp")] = parts[("opp", "net")] = n_games // 8
    parts[("net", "net")] = n_games - sum(parts.values())
    return list(parts.items())


def ppo_step(net, ref_net, opt, arr, idx, adv, clip=0.2, kl_coef=0.02, value_coef=0.5, max_grad=1.0) -> dict:
    """One gradient step on the decisions at idx (advantages `adv`, aligned with idx)."""
    n = len(idx)
    pos = {int(i): k for k, i in enumerate(idx)}
    total = 0.0
    stats = {"clipped": 0.0, "kl": 0.0, "value_mse": 0.0}
    for g in groups(arr, idx):
        x, action, z = batch(arr, g)
        logits, value = net(x)
        if ref_net is not None:
            with torch.no_grad():
                ref = ref_net(x)[0]
        else:
            ref = logits.detach()
        a = adv[[pos[int(i)] for i in g]]
        old = torch.as_tensor(arr["logp"][g])
        pl, st = ppo_loss(logits, ref, action, old, a, clip, kl_coef)
        vl = torch.nn.functional.mse_loss(value, z)
        w = len(g) / n
        total = total + w * (pl + value_coef * vl)
        stats["clipped"] += w * st["clipped"]
        stats["kl"] += w * st["kl"]
        stats["value_mse"] += w * vl.item()
    if not torch.isfinite(total):
        raise FloatingPointError("non-finite PPO loss")
    opt.zero_grad()
    total.backward()
    gn = torch.nn.utils.clip_grad_norm_(net.parameters(), max_grad)
    if not torch.isfinite(gn):
        raise FloatingPointError("non-finite gradient")
    opt.step()
    stats["loss"] = float(total)
    return stats
