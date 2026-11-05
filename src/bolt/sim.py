"""Simulated draft tables. 8 seats, 3 packs of 14, passing left-right-left.

Packs are real opening packs from the 17lands data (pick 1 of each pack shows the full pack), so
we never have to model how Foundations boosters are put together. Seat 0 is the bot under test;
the other 7 seats are the human-imitation model, sampling picks the way people make them.
Many tables run at once, batched on the GPU.
"""

import numpy as np
import torch

from .data import load_picks

SEATS, ROUNDS, PACK = 8, 3, 14


def opening_packs(split: str) -> np.ndarray:
    """(n, 15) full 14-card opening packs (ids+1, zero-padded to the model's width)."""
    p = load_picks()
    m = (p["pick_number"] == 0) & (p["split"] == split) & ((p["pack"] > 0).sum(1) == PACK)
    return p["pack"][m]


@torch.no_grad()
def run_tables(
    packs: np.ndarray,
    n_tables: int,
    seat0,
    others,
    device,
    gen: torch.Generator,
    seat0_greedy: bool = False,
    temp: float = 1.0,
):
    """Draft n_tables tables. seat0/others: models with the draft-model signature.

    Returns pools (T, 8, 42) of ids+1 in pick order, and seat 0's decision log for learning:
    dict of pool (T*42, 42), pack (T*42, 15), pack_number, pick_number, action (card id).
    """
    T = n_tables
    pools = torch.zeros(T, SEATS, ROUNDS * PACK, dtype=torch.int16, device=device)
    log = {k: [] for k in ("pool", "pack", "pack_number", "pick_number", "action")}
    all_packs = torch.as_tensor(packs, device=device)
    for rnd in range(ROUNDS):
        idx = torch.randint(len(packs), (T * SEATS,), generator=gen, device="cpu").to(device)
        cur = all_packs[idx].reshape(T, SEATS, -1).clone()
        direction = 1 if rnd % 2 == 0 else -1
        for k in range(PACK):
            pos = rnd * PACK + k
            pool_in = pools[:, :, : ROUNDS * PACK].reshape(T * SEATS, -1)
            pack_in = cur.reshape(T * SEATS, -1)
            pn = torch.full((T * SEATS,), rnd, dtype=torch.int8, device=device)
            pk = torch.full((T * SEATS,), k, dtype=torch.int8, device=device)
            logits = others(pool_in, pack_in, pn, pk).reshape(T, SEATS, -1)
            l0 = seat0(pools[:, 0], cur[:, 0], pn[:T], pk[:T])
            logits[:, 0] = l0
            if seat0_greedy:
                choice0 = l0.argmax(1)
            else:
                choice0 = torch.multinomial(torch.softmax(l0 / temp, 1).cpu(), 1, generator=gen).squeeze(1).to(device)
            probs = torch.softmax(logits[:, 1:].reshape(T * (SEATS - 1), -1), 1).cpu()
            choice = torch.multinomial(probs, 1, generator=gen).squeeze(1).to(device).reshape(T, SEATS - 1)
            choice = torch.cat([choice0[:, None], choice], 1)  # (T, 8) card ids
            log["pool"].append(pools[:, 0].clone())
            log["pack"].append(cur[:, 0].clone())
            log["pack_number"].append(pn[:T])
            log["pick_number"].append(pk[:T])
            log["action"].append(choice0)
            # take the card: clear the first matching slot in each pack
            match = cur == (choice[..., None] + 1).to(cur.dtype)
            first = match & (match.cumsum(-1) == 1)
            cur = cur.masked_fill(first, 0)
            pools[:, :, pos] = (choice + 1).to(torch.int16)
            cur = torch.roll(cur, direction, dims=1)
    log = {k: torch.stack(v, 1).reshape(T * ROUNDS * PACK, *v[0].shape[1:]) for k, v in log.items()}
    return pools, log


def pools_to_counts(pools: torch.Tensor, n_cards: int) -> np.ndarray:
    """(..., 42) ids+1 -> (..., n_cards) counts."""
    flat = pools.reshape(-1, pools.shape[-1]).long()
    out = torch.zeros(flat.shape[0], n_cards + 1, device=flat.device)
    out.scatter_add_(1, flat, torch.ones_like(flat, dtype=torch.float32))
    return out[:, 1:].reshape(*pools.shape[:-1], n_cards).cpu().numpy()
