"""The turn model (Phase 4, P1): which cards does a person choose this turn?

A position is a set of cards, each tagged with its zone (my hand, my lands, the opponent's
creatures, ...), plus one token for the numbers (turn, both life totals, the opponent's hand size,
on the play). A transformer lets every card attend to every other; a head then scores each slot of
the target zone: the hand (cast or played), or the user's creatures (attacked, blocked). No
positions within a zone: a hand is a set. Empty slots score -inf.
"""

import torch
from torch import nn

ZONES = ("hand", "my_lands", "my_creatures", "my_others", "opp_lands", "opp_creatures", "opp_others", "attackers")
N_SCALARS = 5  # turn, my life, opponent's life, opponent's hand size, on the play


class TurnModel(nn.Module):
    def __init__(self, n_cards: int, dim: int = 128, layers: int = 3, heads: int = 4, dropout: float = 0.1):
        super().__init__()
        self.card = nn.Embedding(n_cards + 2, dim, padding_idx=0)  # + empty slot + unknown card
        self.zone = nn.Embedding(len(ZONES) + 1, dim)  # + the numbers token
        self.scalars = nn.Sequential(nn.Linear(N_SCALARS, dim), nn.GELU(), nn.Linear(dim, dim))
        layer = nn.TransformerEncoderLayer(dim, heads, 4 * dim, dropout, batch_first=True, norm_first=True)
        self.encoder = nn.TransformerEncoder(layer, layers, enable_nested_tensor=False)
        self.norm = nn.LayerNorm(dim)
        self.head = nn.Linear(dim, 1)

    def forward(self, zones: dict, scalars: torch.Tensor, target: str) -> torch.Tensor:
        """zones: name → (B, w) card number + 1 (0 = empty); returns (B, w) logits for `target`."""
        tokens, pads, spans, start = [self.scalars(scalars)[:, None] + self.zone.weight[len(ZONES)]], [], {}, 1
        pads.append(torch.zeros(scalars.shape[0], 1, dtype=torch.bool, device=scalars.device))
        for i, name in enumerate(ZONES):
            if name not in zones:
                continue
            x = zones[name].long()
            tokens.append(self.card(x) + self.zone.weight[i])
            pads.append(x == 0)
            spans[name] = (start, start + x.shape[1])
            start += x.shape[1]
        h = self.norm(self.encoder(torch.cat(tokens, 1), src_key_padding_mask=torch.cat(pads, 1)))
        a, b = spans[target]
        logits = self.head(h[:, a:b]).squeeze(-1)
        return logits.masked_fill(zones[target] == 0, float("-inf"))


def slot_loss(logits: torch.Tensor, target: torch.Tensor, valid: torch.Tensor) -> torch.Tensor:
    """Binary cross-entropy over real slots only. Empty slots are selected out *before* the loss,
    so their -inf logits never enter the arithmetic (the NaN lesson from bolt.rl.masked_kl)."""
    return nn.functional.binary_cross_entropy_with_logits(logits[valid], target[valid].float())
