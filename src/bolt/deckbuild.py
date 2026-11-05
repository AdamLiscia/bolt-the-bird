"""Turn a drafted pool into a 40-card deck, chosen to please the judge.

For each of the 10 two-color pairs:
  1. Lands: every on-color dual land and any-color fixer in the pool (up to 3), then basics,
     split by how many cards of each color made the deck.
  2. Spells: greedily add whichever castable card raises the judged win rate most.
  3. Try both 23 spells + 17 lands and 24 spells + 16 lands.
The best (pair, land count) wins. Everything is batched: many pools x 10 pairs x every candidate.
"""
import numpy as np
import torch

from .cards import COLORS, PAIRS, CardInfo

DECK = 40
LAND_COUNTS = (17, 16)  # greedy is prefix-stable, so the 24-spell build extends the 23-spell one
MAX_FIXERS = 3


def _lands(spells, fixers, n_lands, pidx, colors, basic_cols):
    """fixers + basics totalling n_lands, basics split by colored-card count per color."""
    weight = spells @ colors
    a, b = pidx[:, 0], pidx[:, 1]
    wa = weight.gather(1, a[:, None]).squeeze(1) + 1
    wb = weight.gather(1, b[:, None]).squeeze(1) + 1
    n_basic = n_lands - fixers.sum(1)
    na = torch.round(n_basic * wa / (wa + wb))
    out = fixers.clone()
    out.scatter_add_(1, basic_cols[a][:, None], na[:, None])
    out.scatter_add_(1, basic_cols[b][:, None], (n_basic - na)[:, None])
    return out


@torch.no_grad()
def build_decks(pools: np.ndarray, judge, cards: CardInfo, device, chunk: int = 32):
    """pools: (N, n_cards) card counts. Returns (decks (N, n_cards), judged win rates (N,), labels)."""
    decks, scores, labels = [], [], []
    for s in range(0, len(pools), chunk):
        d, w, lab = _build(torch.as_tensor(pools[s:s + chunk], dtype=torch.float32, device=device), judge, cards, device)
        decks.append(d), scores.append(w), labels.extend(lab)
    return np.concatenate(decks), np.concatenate(scores), labels


def _build(pools, judge, cards: CardInfo, device):
    n, nc = pools.shape
    P = len(PAIRS)
    rows_all = n * P
    colors = torch.as_tensor(cards.colors, dtype=torch.float32, device=device)
    basic_cols = torch.tensor([cards.basic_id[c] for c in COLORS], device=device)
    castable = torch.as_tensor(np.stack([cards.castable_in(p) for p in PAIRS]), device=device).float()
    fixer_mask = torch.as_tensor(np.stack([cards.fixers_for(p) for p in PAIRS]), device=device).float()
    pidx = torch.tensor([[COLORS.index(p[0]), COLORS.index(p[1])] for p in PAIRS], device=device).repeat(n, 1)

    fixers = (pools[:, None, :] * fixer_mask[None]).reshape(rows_all, nc)
    # keep at most MAX_FIXERS fixer lands, in card order
    fixers = torch.minimum(fixers, (MAX_FIXERS - (fixers.cumsum(1) - fixers)).clamp(min=0))
    avail = (pools[:, None, :] * castable[None]).reshape(rows_all, nc)
    spells = torch.zeros_like(avail)
    count = torch.zeros(rows_all, device=device)
    eye = torch.eye(nc, device=device)
    builds = []
    target = DECK - min(LAND_COUNTS)
    for step in range(target):
        n_lands = DECK - (step + 1) if step + 1 > DECK - max(LAND_COUNTS) else max(LAND_COUNTS)
        can = avail > 0
        if can.any():
            rows, cols = can.nonzero(as_tuple=True)
            cand = spells[rows] + eye[cols]
            lands = _lands(cand, fixers[rows], torch.full((len(rows),), float(n_lands), device=device), pidx[rows], colors, basic_cols)
            wr = judge.win_rate(cand + lands)
            best = torch.full((rows_all,), -1.0, device=device).scatter_reduce(0, rows, wr, "amax")
            hit = wr == best[rows]
            pos = torch.full((rows_all,), len(rows), device=device, dtype=torch.long)
            pos = pos.scatter_reduce(0, rows[hit], torch.arange(len(rows), device=device)[hit], "amin")
            ok = pos < len(rows)
            r, c = rows[pos[ok]], cols[pos[ok]]
            spells[r, c] += 1
            avail[r, c] -= 1
            count[r] += 1
        if DECK - (step + 1) in LAND_COUNTS:
            n_l = DECK - (step + 1)
            # pools short on playables fill the gap with extra basics, so every deck is 40 cards
            n_l_rows = torch.full((rows_all,), float(n_l), device=device) + (step + 1 - count)
            deck = spells + _lands(spells, fixers, n_l_rows, pidx, colors, basic_cols)
            builds.append((deck.clone(), judge.win_rate(deck), n_l))
    stacked = torch.stack([b[1] for b in builds], 1).reshape(n, P, len(builds))
    flat = stacked.reshape(n, -1)
    best = flat.argmax(1)
    pair_i, build_i = best // len(builds), best % len(builds)
    decks = torch.stack([b[0] for b in builds], 1).reshape(n, P, len(builds), nc)
    deck = decks[torch.arange(n), pair_i, build_i]
    labels = [f"{PAIRS[p]} {builds[b][2]} lands" for p, b in zip(pair_i.tolist(), build_i.tolist())]
    return deck.cpu().numpy(), flat.max(1).values.cpu().numpy(), labels
