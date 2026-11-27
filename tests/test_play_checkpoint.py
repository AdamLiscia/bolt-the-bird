"""load_checkpoint: a checkpoint saved before action kinds were appended still loads, and plays the same on
every kind it knew."""

import pytest
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


# The card pool growing under a checkpoint (2026-11-27: 172 CMD cards added, every token's number moved up).
OLD = ["Bolt", "Shock", "Island", "Spirit", "Soldier", "Spirit"]  # 3 cards, then tokens
NEW = ["Bolt", "Shock", "Island", "Counterspell", "Opt", "Spirit", "Soldier", "Spirit", "Treasure"]


def test_card_rows_match_by_name_and_occurrence():
    from bolt.play_net import card_rows

    assert card_rows(OLD, NEW) == [0, 1, 2, -1, -1, 3, 4, 5, -1]


def test_legacy_pools_are_todays_first_cards_then_first_tokens(monkeypatch):
    import bolt.play_net as pn

    monkeypatch.setitem(pn.LEGACY_CARDS, "XYZ", 3)
    assert pn.legacy_names("XYZ", 6, now=NEW, n_tokens_now=4) == OLD
    with pytest.raises(ValueError):
        pn.legacy_names("XYZ", 9, now=NEW, n_tokens_now=4)  # 6 tokens: more than exist


def _old_checkpoint(names):
    torch.manual_seed(0)
    net = PlayNet(n_cards=len(names), dim=16, layers=1, heads=2)
    return net, {"config": {k: v for k, v in net.config.items() if k not in ("set_name", "card_names")},
                 "model": net.state_dict()}  # fmt: skip


@pytest.mark.parametrize("stored", [True, False])
def test_a_checkpoint_from_a_smaller_pool_keeps_every_cards_row(monkeypatch, stored):
    import bolt.play_net as pn

    monkeypatch.setattr(pn, "current_names", lambda s: list(NEW))
    monkeypatch.setitem(pn.LEGACY_CARDS, "XYZ", 3)
    monkeypatch.setattr(pn, "legacy_names", lambda s, n: OLD[:n])  # what the engine would rebuild
    old, c = _old_checkpoint(OLD)
    if stored:
        c["config"].update(set_name="XYZ", card_names=OLD)
    net = load_checkpoint(c, set_name=None if stored else "XYZ")
    w, w0 = net.card.weight.detach(), old.card.weight.detach()
    assert w.shape[0] == len(NEW) + 2 and net.config["card_names"] == NEW
    for new_i, old_i in [(0, 0), (1, 1), (2, 2), (5, 3), (6, 4), (7, 5)]:  # Bolt, Shock, Island, the tokens
        assert torch.equal(w[new_i + 1], w0[old_i + 1]), NEW[new_i]
    for new_i in (3, 4, 8):  # Counterspell, Opt, Treasure: new, start as the can't-see row
        assert torch.equal(w[new_i + 1], w0[-1])
    assert torch.equal(w[0], w0[0]) and torch.equal(w[-1], w0[-1])


def test_a_checkpoint_on_todays_pool_loads_unchanged(monkeypatch):
    import bolt.play_net as pn

    monkeypatch.setattr(pn, "current_names", lambda s: list(NEW))
    old, c = _old_checkpoint(NEW)
    c["config"].update(set_name="XYZ", card_names=NEW)
    assert torch.equal(load_checkpoint(c).card.weight, old.card.weight)


def test_adam_moments_move_with_the_card_rows():
    from bolt.play_net import card_rows, load_into, pad_optimizer

    old, _ = _old_checkpoint(OLD)
    opt = torch.optim.AdamW(old.parameters())
    old.card.weight.grad = torch.randn_like(old.card.weight)
    opt.step()
    net = PlayNet(n_cards=len(NEW), dim=16, layers=1, heads=2, set_name="XYZ", card_names=NEW)
    rows = load_into(net, old.state_dict(), OLD)
    assert rows == card_rows(OLD, NEW)
    opt2 = torch.optim.AdamW(net.parameters())
    opt2.load_state_dict(opt.state_dict())
    pad_optimizer(opt2, net, rows)
    m, m0 = opt2.state[net.card.weight]["exp_avg"], opt.state[old.card.weight]["exp_avg"]
    assert m.shape == net.card.weight.shape
    assert torch.equal(m[6], m0[4]) and torch.equal(m[1], m0[1]) and (m[4] == 0).all()
