"""Self-play PPO for the play network (Phase 4, P3d; docs/play/P3_DESIGN.md).

Reward: +1 for a win, -1 for a loss, 0 otherwise, at the end of the game only. Advantages come from
`gae` (generalised advantage estimation, gamma = 1): with lambda = 1 a move's advantage is the
result minus the value estimate made when it was played; lower lambda credits each move more with the
change in the value estimate it caused, which is less noisy in a ~100-move game. The update is the drafter's clipped PPO
objective (bolt.rl.ppo_loss) with a KL leash to a fixed reference policy (the distilled network),
plus value regression to the result.
"""

import numpy as np
import torch

from bolt.play_data import batch, groups
from bolt.rl import ppo_loss

FIXED = (("net", "rules"), ("rules", "net"), ("net", "smart"), ("smart", "net"))


def league(n_games: int, past: list, self_only: bool = False) -> list:
    """(seats, games) for one iteration: a quarter against the rules-based player, a quarter against the
    smart player, a quarter against past checkpoints (seat "opp"), a quarter self-play (both seats
    recorded); with no past checkpoints yet, self-play takes that quarter too. Every pairing is
    played from both seats."""
    if self_only:  # half against past checkpoints (once there are any), the rest against itself
        parts = {}
        if past:
            parts[("net", "opp")] = parts[("opp", "net")] = n_games // 4
        parts[("net", "net")] = n_games - sum(parts.values())
        return list(parts.items())
    parts = {s: n_games // 8 for s in FIXED}
    if past:
        parts[("net", "opp")] = parts[("opp", "net")] = n_games // 8
    parts[("net", "net")] = n_games - sum(parts.values())
    return list(parts.items())


def gae(arr: dict, lam: float = 0.95, gamma: float = 1.0):
    """(advantages, returns) per recorded move. A trajectory is one seat's moves in one game, in
    recording order (steps of different games interleave). The only reward is the result `z` after
    the last move; delta_t = gamma * V_{t+1} - V_t, and z - V_last for the last move."""
    v = arr["value"].astype(np.float64)
    z = arr["z"].astype(np.float64)
    adv = np.zeros(len(v))
    traj: dict = {}
    for i, key in enumerate(zip(arr["game"].tolist(), arr["seat"].tolist(), strict=True)):
        traj.setdefault(key, []).append(i)
    for rows in traj.values():
        nxt = 0.0
        for k in range(len(rows) - 1, -1, -1):
            i = rows[k]
            target = z[i] if k == len(rows) - 1 else gamma * v[rows[k + 1]]
            nxt = (target - v[i]) + gamma * lam * nxt
            adv[i] = nxt
    return adv.astype(np.float32), (adv + v).astype(np.float32)


def ppo_step(net, ref_net, opt, arr, idx, adv, clip=0.2, kl_coef=0.02, value_coef=0.5, max_grad=1.0,
             ret=None) -> dict:  # fmt: skip
    """One gradient step on the decisions at idx (advantages `adv`, aligned with idx). The value head
    regresses to `ret` (aligned with idx) when given, otherwise to the game result."""
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
        if ret is not None:
            z = ret[[pos[int(i)] for i in g]]
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
    stats["loss"] = float(total.detach())
    return stats
