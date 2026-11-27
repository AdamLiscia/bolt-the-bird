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
    def __init__(self, n_cards: int, dim: int = 128, layers: int = 4, heads: int = 4, dropout: float = 0.0,
                 set_name: str | None = None, card_names: list[str] | None = None):  # fmt: skip
        """`set_name` and `card_names` (the card set and its names, by card number) travel with every checkpoint
        so a later engine whose pool grew can move each row to its card's new number (`load_checkpoint`)."""
        super().__init__()
        self.config = {"n_cards": n_cards, "dim": dim, "layers": layers, "heads": heads, "dropout": dropout,
                       "set_name": set_name, "card_names": None if card_names is None else list(card_names)}  # fmt: skip
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


# Checkpoints saved before card names were stored: how many of the set's cards (not tokens) their pool had.
# Cards and tokens are both only ever appended, so the old pool is the first LEGACY_CARDS cards of today's,
# then the first (rows - LEGACY_CARDS) of today's tokens. CMD: 3,131 cards until tarkir 9327e46 added 172.
LEGACY_CARDS = {"CMD": 3131}


def current_names(set_name: str) -> list[str]:
    """The engine's card names for a set, by card number (cards, then tokens)."""
    import bolt_engine as be

    return list(be.card_names() if set_name.upper() == "FDN" else be.card_names(set_name.upper()))


def legacy_names(
    set_name: str, n_cards: int, now: list[str] | None = None, n_tokens_now: int | None = None
) -> list[str]:
    """The names of an n_cards pool of `set_name` saved before checkpoints carried names (see LEGACY_CARDS), from
    today's names and token count (the engine's unless given)."""
    s = set_name.upper()
    if now is None:
        now = current_names(s)
    if n_tokens_now is None:
        import bolt_engine as be

        n_tokens_now = len(be.token_info() if s == "FDN" else be.token_info(s))
    cards_now = len(now) - n_tokens_now
    cards = LEGACY_CARDS.get(s, cards_now)
    tokens = n_cards - cards
    if not 0 <= tokens <= n_tokens_now or cards > cards_now:
        raise ValueError(f"can't rebuild a {n_cards}-card {s} pool: {cards} cards then {tokens} tokens")
    return now[:cards] + now[cards_now : cards_now + tokens]


def card_rows(old: list[str], new: list[str]) -> list[int]:
    """For each card number in `new`, its number in `old` (-1 for a card `old` lacks). A name used more than once
    (token names: several Spirits) is matched by occurrence: the k-th Spirit to the k-th Spirit."""
    seen: dict[str, list[int]] = {}
    for i, n in enumerate(old):
        seen.setdefault(n, []).append(i)
    used: dict[str, int] = {}
    rows = []
    for n in new:
        k = used.get(n, 0)
        used[n] = k + 1
        rows.append(seen[n][k] if k < len(seen.get(n, [])) else -1)
    return rows


def remap_cards(state: dict, rows: list[int]) -> dict:
    """card.weight moved to the new card numbers: row 0 (padding) and the last row (a card the player can't see)
    kept, each card's row moved to its new number, a new card starting as the can't-see row ("some card")."""
    w = state["card.weight"]
    out = w[-1].repeat(len(rows) + 2, 1)
    out[0] = w[0]
    for i, r in enumerate(rows):
        if r >= 0:
            out[i + 1] = w[r + 1]
    return {**state, "card.weight": out}


def _pool(c: dict, set_name: str | None) -> tuple[str | None, list[str] | None]:
    """(set, its names when the checkpoint was saved) of a checkpoint dict, or (None, None) when unknown."""
    config = c.get("config", {})
    s = set_name or config.get("set_name") or c.get("set")
    if not s:
        return None, None
    names = config.get("card_names") or c.get("card_names")
    if names is None:
        names = legacy_names(s, c["model"]["card.weight"].shape[0] - 2)
    return s.upper(), list(names)


def load_into(net: "PlayNet", state: dict, old_names: list[str] | None = None) -> list[int] | None:
    """Load a state dict into `net`, growing its inputs (`pad_state`) and, given the names its card rows were
    saved under, moving them to `net`'s card numbers. Returns the row map (for `pad_optimizer`), or None."""
    state = pad_state(state)
    rows = None
    names = net.config.get("card_names")
    if old_names is not None and names is not None and list(old_names) != list(names):
        rows = card_rows(old_names, names)
        state = remap_cards(state, rows)
    net.load_state_dict(state)
    return rows


def pad_optimizer(opt: torch.optim.Optimizer, net: nn.Module, rows: list[int] | None = None) -> None:
    """After `opt.load_state_dict` of a state saved before inputs were appended or the card pool grew: Adam's
    moment estimates for the grown tensors, zero-padded the way pad_state pads the weights (kinds: new rows
    zero too), and the card table's moved like its rows (`rows` from `load_into`; new cards start at zero)."""
    for p in net.parameters():
        st = opt.state.get(p)
        if not st:
            continue
        for k in ("exp_avg", "exp_avg_sq"):
            m = st.get(k)
            if m is None or (m.shape == p.shape and not (rows is not None and p is net.card.weight)):
                continue
            out = m.new_zeros(p.shape)
            if p is net.card.weight and rows is not None:
                out[0], out[-1] = m[0], m[-1]
                for i, r in enumerate(rows):
                    if r >= 0:
                        out[i + 1] = m[r + 1]
            elif p is net.kind.weight:  # old rows before the unknown-kind row, which stays last
                out[: m.shape[0] - 1] = m[:-1]
                out[-1] = m[-1]
            else:
                out[tuple(slice(0, d) for d in m.shape)] = m
            st[k] = out


def load_checkpoint(path_or_obj, set_name: str | None = None) -> "PlayNet":
    """A PlayNet from a checkpoint (a path, or the loaded {"config", "model"} dict); one saved before inputs
    were appended is grown by `pad_state`, so it plays exactly as before on everything it knew.

    When the card set is known (`set_name`, or stored in the checkpoint) and the engine's pool for it differs
    from the one the checkpoint was saved with (its stored names, or for older checkpoints `legacy_names`),
    the card table is moved to the engine's numbers: each card keeps its row, new cards start as the
    can't-see row (2026-11-27: 172 cards added to CMD moved every token's number)."""
    c = (
        path_or_obj
        if isinstance(path_or_obj, dict)
        else torch.load(path_or_obj, map_location="cpu", weights_only=False)
    )
    try:
        s, old = _pool(c, set_name)
        now = current_names(s) if s is not None else None
    except (ImportError, AttributeError, TypeError):  # an engine without card sets: load as saved
        s, old, now = None, None, None
    config = dict(c["config"])
    if now is not None:
        config.update(n_cards=len(now), set_name=s, card_names=now)
    net = PlayNet(**config)
    load_into(net, c["model"], old)
    return net
