"""load_checkpoint: a checkpoint saved before action kinds were appended still loads, and plays the same on
every kind it knew."""

import torch

from bolt.play_encoding import KINDS
from bolt.play_net import PlayNet, load_checkpoint


def test_an_older_kind_table_is_padded_without_changing_known_rows():
    torch.manual_seed(0)
    net = PlayNet(n_cards=10, dim=16, layers=1, heads=2)
    state = net.state_dict()
    full = state["kind.weight"]
    old_kinds = len(KINDS) - 2  # saved before CastHarmonize and CastOmen existed
    old = torch.cat([full[:old_kinds], full[-1:]])  # its unknown-kind row was last
    loaded = load_checkpoint({"config": net.config, "model": {**state, "kind.weight": old}})
    w = loaded.kind.weight.detach()
    assert w.shape[0] == len(KINDS) + 1
    assert torch.equal(w[:old_kinds], full[:old_kinds]), "every known kind keeps its row"
    assert torch.equal(w[-1], full[-1]), "the unknown-kind row stays last"
    cast = full[KINDS.index("Cast")]
    assert torch.equal(w[KINDS.index("CastOmen")], cast), "a new kind starts as Cast"


def test_a_current_checkpoint_loads_unchanged():
    net = PlayNet(n_cards=10, dim=16, layers=1, heads=2)
    loaded = load_checkpoint({"config": net.config, "model": net.state_dict()})
    assert torch.equal(loaded.kind.weight, net.kind.weight)
