"""The play network (Phase 4, P3a): scores every legal move, and estimates the chance of winning.

A transformer reads the game token and one token per visible object (bolt.play_encoding). Each
legal move becomes a token from its kind, the encoder's output for the objects it points at, the
cards it names, its numbers and the player it targets; a head scores it against the game token.
Softmax over the legal moves gives the policy. The value head reads the game token: a number in
[-1, 1], the expected result (+1 win, -1 loss). No positions anywhere: objects are a set.
"""

import torch
from torch import nn

from bolt.play_encoding import FEATS, GLOBAL_NAMES, KINDS, ZONE


class PlayNet(nn.Module):
    def __init__(self, n_cards: int, dim: int = 128, layers: int = 4, heads: int = 4, dropout: float = 0.0):
        super().__init__()
        self.config = {"n_cards": n_cards, "dim": dim, "layers": layers, "heads": heads, "dropout": dropout}
        self.card = nn.Embedding(n_cards + 2, dim, padding_idx=0)
        self.zone = nn.Embedding(len(ZONE), dim)
        self.feats = nn.Linear(len(FEATS), dim)
        self.glob = nn.Sequential(nn.Linear(len(GLOBAL_NAMES), dim), nn.GELU(), nn.Linear(dim, dim))
        layer = nn.TransformerEncoderLayer(dim, heads, 4 * dim, dropout, batch_first=True, norm_first=True)
        self.encoder = nn.TransformerEncoder(layer, layers, enable_nested_tensor=False)
        self.norm = nn.LayerNorm(dim)
        self.kind = nn.Embedding(len(KINDS) + 1, dim)
        self.slot = nn.ModuleList(nn.Linear(dim, dim) for _ in range(3))
        self.num = nn.Linear(2, dim)
        self.player = nn.Embedding(3, dim)
        self.policy = nn.Sequential(nn.Linear(2 * dim, dim), nn.GELU(), nn.Linear(dim, 1))
        self.value = nn.Linear(dim, 1)

    def forward(self, x: dict) -> tuple[torch.Tensor, torch.Tensor]:
        """x from bolt.play_encoding.collate. Returns (logits (B, A) with -inf on padding, value (B,))."""
        objs = self.card(x["card"]) + self.zone(x["zone"]) + self.feats(x["feats"])
        g = self.glob(x["globals"])[:, None]
        pad = torch.cat([torch.zeros_like(x["obj_pad"][:, :1]), x["obj_pad"]], 1)
        h = self.norm(self.encoder(torch.cat([g, objs], 1), src_key_padding_mask=pad))
        hg, ho = h[:, 0], h[:, 1:]
        act = self.kind(x["act_kind"]) + self.num(x["act_num"]) + self.player(x["act_player"])
        for s in range(3):
            ptr = x["act_ptr"][..., s]
            got = torch.gather(ho, 1, ptr.clamp(min=0)[..., None].expand(-1, -1, ho.shape[-1]))
            act = act + self.slot[s](got) * (ptr >= 0)[..., None] + self.card(x["act_card"][..., s])
        both = torch.cat([act, hg[:, None].expand_as(act)], -1)
        logits = self.policy(both).squeeze(-1).masked_fill(x["act_pad"], float("-inf"))
        return logits, torch.tanh(self.value(hg)).squeeze(-1)
