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
        # a card the player can't see (the engine's -2, encoded -1: face down, hidden in exile) is the last row
        card = torch.where(x["card"] < 0, self.card.num_embeddings - 1, x["card"])
        objs = self.card(card) + self.zone(x["zone"]) + self.feats(x["feats"])
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


def pad_state(state: dict) -> dict:
    """A state dict saved before inputs were appended, grown to the current ones so it computes exactly what
    it did on every input it knew:

    - action kinds appended to KINDS: each new row a copy of "Cast"'s, inserted before the unknown-kind row,
      which stays last;
    - zones appended to ZONE: zero rows;
    - per-object features appended to FEATS and game numbers appended to GLOBAL_NAMES: zero input weights,
      so the new inputs add nothing until training moves them.

    A current state dict comes back unchanged."""
    state = dict(state)
    old = state["kind.weight"]
    want = len(KINDS) + 1
    if old.shape[0] < want:
        known = old.shape[0] - 1  # the old unknown-kind row was the last
        cast = old[KINDS.index("Cast")]
        new = cast.unsqueeze(0).repeat(want - 1 - known, 1)
        state["kind.weight"] = torch.cat([old[:known], new, old[known:]])
    z = state["zone.weight"]
    if z.shape[0] < len(ZONE):
        state["zone.weight"] = torch.cat([z, z.new_zeros(len(ZONE) - z.shape[0], z.shape[1])])
    for key, n in (("feats.weight", len(FEATS)), ("glob.0.weight", len(GLOBAL_NAMES))):
        w = state[key]
        if w.shape[1] < n:
            state[key] = torch.cat([w, w.new_zeros(w.shape[0], n - w.shape[1])], 1)
    return state


def pad_optimizer(opt: torch.optim.Optimizer, net: nn.Module) -> None:
    """After `opt.load_state_dict` of a state saved before inputs were appended: Adam's moment estimates for
    the grown tensors, zero-padded the way pad_state pads the weights (kinds: new rows zero too)."""
    for p in net.parameters():
        st = opt.state.get(p)
        if not st:
            continue
        for k in ("exp_avg", "exp_avg_sq"):
            m = st.get(k)
            if m is None or m.shape == p.shape:
                continue
            out = m.new_zeros(p.shape)
            if p is net.kind.weight:  # old rows before the unknown-kind row, which stays last
                out[: m.shape[0] - 1] = m[:-1]
                out[-1] = m[-1]
            else:
                out[tuple(slice(0, d) for d in m.shape)] = m
            st[k] = out


def load_checkpoint(path_or_obj) -> "PlayNet":
    """A PlayNet from a checkpoint (a path, or the loaded {"config", "model"} dict); one saved before inputs
    were appended is grown by `pad_state`, so it plays exactly as before on everything it knew."""
    c = (
        path_or_obj
        if isinstance(path_or_obj, dict)
        else torch.load(path_or_obj, map_location="cpu", weights_only=False)
    )
    net = PlayNet(**c["config"])
    net.load_state_dict(pad_state(c["model"]))
    return net
