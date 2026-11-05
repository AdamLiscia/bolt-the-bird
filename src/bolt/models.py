"""Draft models. Both take (pool, pack, pack_number, pick_number) and return a logit per card in
the vocabulary; cards not in the pack are masked to -inf.

Card ids are stored +1 so 0 can mean "empty slot".
"""

import torch
from torch import nn

N_POS = 42  # 3 packs x 14 picks


def to_vocab(token_logits: torch.Tensor, pack: torch.Tensor, n_cards: int) -> torch.Tensor:
    """Scatter per-slot logits onto the card vocabulary; everything not in the pack is -inf."""
    out = torch.full((pack.shape[0], n_cards + 1), float("-inf"), device=pack.device)
    out = out.scatter_reduce(1, pack.long(), token_logits.float().masked_fill(pack == 0, float("-inf")), "amax")
    return out[:, 1:]


class PoolMLP(nn.Module):
    """Baseline: sum the pool's card embeddings, run an MLP, and dot the result with each pack card.
    Knows what you've taken, but not how the cards relate to each other."""

    def __init__(self, n_cards: int, d: int = 128):
        super().__init__()
        self.n_cards = n_cards
        self.pool_emb = nn.Embedding(n_cards + 1, d, padding_idx=0)
        self.card_emb = nn.Embedding(n_cards + 1, d, padding_idx=0)
        self.card_bias = nn.Embedding(n_cards + 1, 1, padding_idx=0)
        self.pos = nn.Embedding(N_POS, d)
        self.mlp = nn.Sequential(nn.Linear(d, 2 * d), nn.GELU(), nn.Linear(2 * d, d))

    def forward(self, pool, pack, pack_number, pick_number):
        pos = pack_number.long() * 14 + pick_number.long()
        h = self.mlp(self.pool_emb(pool.long()).sum(1) / 8 + self.pos(pos))
        c = self.card_emb(pack.long())
        logits = (c * h[:, None]).sum(-1) + self.card_bias(pack.long()).squeeze(-1)
        return to_vocab(logits, pack, self.n_cards)


class DraftTransformer(nn.Module):
    """Main model: one token per card in the pool and the pack, plus a draft-position token.
    Self-attention lets every pack card look at every pool card ("does this fit what I have?")
    and at the rest of the pack ("what will I lose by passing this?"). No sequence positions:
    a pool is a set."""

    def __init__(self, n_cards: int, d: int = 192, layers: int = 4, heads: int = 6, dropout: float = 0.1):
        super().__init__()
        self.n_cards = n_cards
        self.card = nn.Embedding(n_cards + 1, d, padding_idx=0)
        self.kind = nn.Embedding(3, d)  # 0 position token, 1 pool card, 2 pack card
        self.pos = nn.Embedding(N_POS, d)
        layer = nn.TransformerEncoderLayer(d, heads, 4 * d, dropout, batch_first=True, norm_first=True)
        self.encoder = nn.TransformerEncoder(layer, layers, enable_nested_tensor=False)
        self.norm = nn.LayerNorm(d)
        self.head = nn.Linear(d, 1)

    def forward(self, pool, pack, pack_number, pick_number):
        b = pool.shape[0]
        pos = self.pos(pack_number.long() * 14 + pick_number.long())[:, None] + self.kind.weight[0]
        pool_t = self.card(pool.long()) + self.kind.weight[1]
        pack_t = self.card(pack.long()) + self.kind.weight[2]
        x = torch.cat([pos, pool_t, pack_t], 1)
        pad = torch.cat([torch.zeros(b, 1, dtype=torch.bool, device=pool.device), pool == 0, pack == 0], 1)
        h = self.norm(self.encoder(x, src_key_padding_mask=pad))
        logits = self.head(h[:, 1 + pool.shape[1] :]).squeeze(-1)
        return to_vocab(logits, pack, self.n_cards)


MODELS = {"mlp": PoolMLP, "transformer": DraftTransformer}
