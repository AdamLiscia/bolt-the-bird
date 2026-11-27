"""Commander on the real engine: a TDM observation plays identically through the grown network, real Commander
positions encode, and the CMD card table carries TDM's trained rows by name. Skipped on an engine without the
TDM and CMD sets."""

import sys
from pathlib import Path

import pytest
import torch

be = pytest.importorskip("bolt_engine")
try:
    be.card_names("CMD"), be.card_names("TDM")
except TypeError:
    pytest.skip("this engine has no card sets", allow_module_level=True)

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))

from pad_cmd_net import pad_cards  # noqa: E402
from test_play_commander import OLD_FEATS, OLD_GLOBALS, old_net, older  # noqa: E402

from bolt.cmd_decks import read  # noqa: E402
from bolt.play_encoding import FEATS, GLOBAL_NAMES, ZONE, collate, encode  # noqa: E402
from bolt.play_net import PlayNet, load_checkpoint  # noqa: E402


def tdm_positions(n=12):
    names = be.card_names("TDM")
    ok = be.supported_mask("TDM")
    spells = [i for i, x in enumerate(ok) if x and "Plains" not in names[i] and "Swamp" not in names[i]][:23]
    deck = [names.index("Plains")] * 9 + [names.index("Swamp")] * 8 + spells
    g = be.Game(deck, deck, seed=3, first=0, set="TDM")
    out = []
    while not g.is_over() and len(out) < n:
        who = g.to_act()
        o = g.observation(who)
        if len(o["legal_actions"]) > 1:
            out.append(encode(o, who, "TDM"))
        g.step(g.baseline())
    return out


def test_a_tdm_net_grown_for_commander_plays_a_tdm_position_identically():
    torch.manual_seed(1)
    n = len(be.card_names("TDM"))
    full = PlayNet(n_cards=n, dim=32, layers=2, heads=4)
    state = older(full.state_dict())
    before = old_net(PlayNet(n_cards=n, dim=32, layers=2, heads=4), state)
    after = load_checkpoint({"config": full.config, "model": state}).eval()
    x = collate(tdm_positions())
    old_x = {**x, "feats": x["feats"][..., :OLD_FEATS], "globals": x["globals"][:, :OLD_GLOBALS]}
    with torch.no_grad():
        (la, va), (lb, vb) = before(old_x), after(x)
    # equal up to float rounding: a matmul with extra zero columns may sum in another order
    assert torch.allclose(la, lb, rtol=0, atol=1e-6) and torch.allclose(va, vb, rtol=0, atol=1e-6)
    # and the CMD net built from it scores a TDM card's rows the same: the trained row moved by name
    cmd, kept = pad_cards(after, be.card_names("TDM"), be.card_names("CMD"))
    assert cmd.config["n_cards"] == len(be.card_names("CMD")) and kept > 100
    t, c = be.card_names("TDM"), be.card_names("CMD")
    shared = next(k for k, name in enumerate(c) if name in t and name not in ("Plains", "Swamp"))
    assert torch.equal(cmd.card.weight[shared + 1], after.card.weight[t.index(c[shared]) + 1])


DECKS = Path(__file__).resolve().parents[1] / "handoff" / "cmd" / "playable_decks.txt"


def test_the_committed_deck_file_is_playable():
    decks = read(DECKS)
    ok = be.supported_mask("CMD")
    assert len(decks) >= 2 and all(ok[c] for cmd, cards in decks for c in cmd + cards)


def test_real_commander_positions_encode():
    (ca, da), (cb, db) = read(DECKS)[:2]
    g = be.Game.commander(da, db, ca, cb, 5, first=0, set="CMD")
    seen = set()
    for _ in range(300):
        if g.is_over():
            break
        who = g.to_act()
        o = g.observation(who)
        if len(o["legal_actions"]) > 1:
            e = encode(o, who, "CMD")
            assert len(e["globals"]) == len(GLOBAL_NAMES) and all(len(f) == len(FEATS) for f in e["feats"])
            assert e["globals"][GLOBAL_NAMES.index("is_commander_game")] == 1
            seen |= set(e["zone"])
        g.step(g.baseline())
    assert ZONE["my_command"] in seen and ZONE["opp_command"] in seen


def test_the_runner_plays_commander_games(monkeypatch):
    import bolt.play_runner as pr

    monkeypatch.setattr(pr, "PLAY_FORMAT", "commander")
    monkeypatch.setattr(pr, "PLAY_SET", "CMD")
    monkeypatch.setenv("BOLT_PLAY_SET", "CMD")
    d = read(DECKS)[:3]
    torch.manual_seed(0)
    net = PlayNet(n_cards=len(be.card_names("CMD")), dim=16, layers=1, heads=2).eval()
    out = pr.run_games(net, [(d[0], d[1], 7, 0), (d[2], d[0], 8, 1)], seats=("net", "net"), max_moves=300)
    assert all(r["result"] in ("timeout", "p0", "p1", "draw") for r in out["results"])
    assert out["steps"] and {s["seat"] for s in out["steps"]} == {0, 1}
    assert all(len(s["enc"]["globals"]) == len(GLOBAL_NAMES) for s in out["steps"])
