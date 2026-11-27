"""Commander inputs for the play network (2026-11-27): the encoder's appended zones, features and globals, the
printed-facts table, deck files, and a network saved before all of it, which must play exactly as before."""

import numpy as np
import pytest
import torch
from torch import nn

from bolt import cmd_decks
from bolt.play_data import batch, to_arrays
from bolt.play_encoding import DYNAMIC, FEATS, GLOBAL_NAMES, STATIC, ZONE, collate, encode, static_rows
from bolt.play_net import PlayNet, load_checkpoint, pad_optimizer, pad_state

OLD_FEATS, OLD_ZONES = 13, 8
OLD_GLOBALS = GLOBAL_NAMES.index("stack") + 1


def obs(commander=False, life=(20, 20), start=None, **kw):
    """A small hand-made observation: a hand card, a creature each, and (in Commander) command zones."""
    players = []
    for p in (0, 1):
        players.append({"life": life[p], "library": 30, "hand_size": 1, "hand": [(10 + p, 3)] if p == 0 else None,
                        "graveyard": [], "lands_played": 0, "pool": 0, "command": [], "commander_damage": []})  # fmt: skip
    o = {"turn": 3, "step": "Main1", "active": 0, "pending": ("Priority", 0), "choosing": None, "combat": None,
         "players": players, "exile": [], "stack": [], "commanders": [],
         "battlefield": [{"id": 20 + p, "card": 4, "controller": p, "owner": p, "tapped": False, "creature": True,
                          "land": False, "power": 2, "toughness": 2, "damage": 0, "counters": 0, "loyalty": 0,
                          "attached_to": None, "entered_turn": 1} for p in (0, 1)],
         "legal_actions": [("PassPriority", -1, -1, -1), ("Cast", 10, -1, -1)],
         "legal_cards": [(-1, -1, -1), (3, -1, -1)]}  # fmt: skip
    if commander:
        players[0]["command"] = [(30, 5, 2)]
        players[1]["command"] = [(31, 6, 0)]
        players[0]["commander_damage"] = [(21, 4, 1, 7)]  # their commander (on the field) hit me for 7
        players[1]["commander_damage"] = [(30, 5, 0, 12)]
        players[1]["commander_tax"] = [(21, 4)]
        players[0]["commander_tax"] = [(30, 2)]
        o["commanders"] = [(30, 0), (21, 1)]
    if start is not None:
        o["starting_life"] = start
    return {**o, **kw}


TABLE = [[0.1 * (k + 1)] * len(STATIC) for k in range(8)]


def test_inputs_are_appended_never_reordered():
    assert ZONE["opp_stack"] == 7 and ZONE["my_command"] == 8 and ZONE["opp_command"] == 9
    assert FEATS[:OLD_FEATS] == DYNAMIC[:OLD_FEATS] and FEATS.index("blocked") == 12
    assert GLOBAL_NAMES[OLD_GLOBALS:] == ["is_commander_game", "cmd_damage_taken", "cmd_damage_dealt",
                                          "my_cmd_tax", "opp_cmd_tax"]  # fmt: skip


def test_a_20_life_game_encodes_its_old_inputs_as_before(monkeypatch):
    monkeypatch.setattr("bolt.play_encoding.card_table", lambda s: TABLE)
    e = encode(obs(life=(17, 9)), 0)
    g = dict(zip(GLOBAL_NAMES, e["globals"], strict=True))
    assert g["my_life"] == 17 / 20 and g["opp_life"] == 9 / 20 and g["my_library"] == 30 / 40
    assert g["is_commander_game"] == 0 and g["cmd_damage_taken"] == 0
    assert e["feats"][1][:OLD_FEATS] == [0.0, 1.0, 0.0, 0.2, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    assert e["feats"][0][len(DYNAMIC) :] == TABLE[3], "the hand card carries its printed facts"
    assert ZONE["my_command"] not in e["zone"]


def test_commander_zones_damage_tax_and_40_life(monkeypatch):
    monkeypatch.setattr("bolt.play_encoding.card_table", lambda s: TABLE)
    e = encode(obs(commander=True, life=(40, 20), start=40), 0)
    g = dict(zip(GLOBAL_NAMES, e["globals"], strict=True))
    assert g["my_life"] == 1.0 and g["opp_life"] == 0.5, "40 at 40 reads as 20 at 20"
    assert g["my_library"] == 30 / 80
    assert g["is_commander_game"] == 1
    assert g["cmd_damage_taken"] == 7 / 21 and g["cmd_damage_dealt"] == 12 / 21
    assert g["my_cmd_tax"] == 2 / 10 and g["opp_cmd_tax"] == 4 / 10, "tax counts on the field too"
    mine = e["obj_id"].index(30)
    assert e["zone"][mine] == ZONE["my_command"] and e["zone"][e["obj_id"].index(31)] == ZONE["opp_command"]
    c, t = FEATS.index("commander"), FEATS.index("commander_tax")
    assert e["feats"][mine][c] == 1 and e["feats"][mine][t] == 0.2
    theirs = e["obj_id"].index(21)  # their commander, on the battlefield
    assert e["feats"][theirs][c] == 1 and e["feats"][theirs][t] == 0.4
    assert e["feats"][e["obj_id"].index(20)][c] == 0


def test_without_starting_life_a_commander_game_assumes_40(monkeypatch):
    monkeypatch.setattr("bolt.play_encoding.card_table", lambda s: TABLE)
    o = obs(commander=True, life=(40, 40))
    for p in o["players"]:
        p.pop("commander_tax")
    g = dict(zip(GLOBAL_NAMES, encode(o, 0)["globals"], strict=True))
    assert g["my_life"] == 1.0 and g["my_cmd_tax"] == 0.2 and g["opp_cmd_tax"] == 0, "the command zone's tax"


def test_printed_facts_from_the_oracle():
    oracle = [
        {"name": "Serra Angel", "cmc": 5.0, "type_line": "Creature — Angel", "power": "4", "toughness": "4",
         "keywords": ["Flying", "Vigilance"], "colors": ["W"], "oracle_text": "Flying, vigilance"},
        {"name": "Tarmogoyf", "cmc": 2.0, "type_line": "Creature — Lhurgoyf", "power": "*", "toughness": "1+*",
         "keywords": [], "colors": ["G"], "oracle_text": "..."},
        {"name": "Fire // Ice", "cmc": 4.0, "type_line": "Instant // Instant", "colors": ["U", "R"],
         "keywords": [], "oracle_text": ""},
    ]  # fmt: skip
    rows = static_rows(["Serra Angel", "Soldier", "Tarmogoyf", "Fire"], oracle)
    f = {n: dict(zip(STATIC, r, strict=True)) for n, r in zip(["angel", "token", "goyf", "fire"], rows, strict=True)}
    assert f["angel"]["mana_value"] == 0.5 and f["angel"]["color_W"] == 1 and f["angel"]["type_creature"] == 1
    assert f["angel"]["kw_flying"] == 1 and f["angel"]["kw_vigilance"] == 1 and f["angel"]["kw_trample"] == 0
    assert f["angel"]["base_power"] == 0.4 and f["angel"]["printed"] == 1
    assert all(v == 0 for v in f["token"].values()), "a token: zeros"
    assert f["goyf"]["base_power"] == 0 and f["goyf"]["printed"] == 1
    assert f["fire"]["type_instant"] == 1 and f["fire"]["color_U"] == 1 and f["fire"]["color_R"] == 1


def old_net(net: PlayNet, state: dict) -> PlayNet:
    """`net` rebuilt with the input sizes it had before Commander, holding `state`."""
    dim = net.config["dim"]
    net.feats = nn.Linear(OLD_FEATS, dim)
    net.glob[0] = nn.Linear(OLD_GLOBALS, dim)
    net.zone = nn.Embedding(OLD_ZONES, dim)
    net.load_state_dict(state)
    return net.eval()


def older(state: dict) -> dict:
    return {**state, "feats.weight": state["feats.weight"][:, :OLD_FEATS],
            "glob.0.weight": state["glob.0.weight"][:, :OLD_GLOBALS], "zone.weight": state["zone.weight"][:OLD_ZONES]}  # fmt: skip


def test_a_net_saved_before_commander_plays_exactly_as_before(monkeypatch):
    monkeypatch.setattr("bolt.play_encoding.card_table", lambda s: TABLE)
    torch.manual_seed(0)
    full = PlayNet(n_cards=8, dim=16, layers=1, heads=2)
    state = older(full.state_dict())
    before = old_net(PlayNet(n_cards=8, dim=16, layers=1, heads=2), state)
    after = load_checkpoint({"config": full.config, "model": state}).eval()
    x = collate([encode(obs(life=(13, 20)), 0), encode(obs(), 1)])
    assert x["feats"][..., OLD_FEATS:].abs().sum() > 0, "the new inputs are there, and ignored"
    old_x = {**x, "feats": x["feats"][..., :OLD_FEATS], "globals": x["globals"][:, :OLD_GLOBALS]}
    with torch.no_grad():
        (la, va), (lb, vb) = before(old_x), after(x)
    # equal up to float rounding: a matmul with extra zero columns may sum in another order
    assert torch.allclose(la, lb, rtol=0, atol=1e-6) and torch.allclose(va, vb, rtol=0, atol=1e-6)
    assert torch.equal(pad_state(after.state_dict())["feats.weight"], after.feats.weight)


def test_old_optimizer_moments_are_padded():
    torch.manual_seed(0)
    full = PlayNet(n_cards=8, dim=16, layers=1, heads=2)
    old = old_net(PlayNet(n_cards=8, dim=16, layers=1, heads=2), older(full.state_dict()))
    opt = torch.optim.AdamW(old.parameters())
    x = collate([encode(obs(), 0)])
    old({**x, "feats": x["feats"][..., :OLD_FEATS], "globals": x["globals"][:, :OLD_GLOBALS]})[1].sum().backward()
    opt.step()
    net = load_checkpoint({"config": full.config, "model": old.state_dict()})
    opt2 = torch.optim.AdamW(net.parameters())
    opt2.load_state_dict(opt.state_dict())
    pad_optimizer(opt2, net)
    m = opt2.state[net.feats.weight]["exp_avg"]
    assert m.shape == net.feats.weight.shape and torch.equal(m[:, :OLD_FEATS], opt.state[old.feats.weight]["exp_avg"])
    net(x)[1].sum().backward()
    opt2.step()  # no shape errors


def test_a_shard_recorded_before_commander_still_batches(monkeypatch):
    monkeypatch.setattr("bolt.play_encoding.card_table", lambda s: TABLE)
    e = encode(obs(), 0)
    arr = to_arrays([{"game": 0, "seat": 0, "enc": e, "action": 1}], [{"result": "p0"}])
    arr["feats"], arr["globals"] = arr["feats"][:, :OLD_FEATS], arr["globals"][:, :OLD_GLOBALS]
    x, _, _ = batch(arr, np.array([0]))
    assert x["feats"].shape[-1] == len(FEATS) and x["globals"].shape[-1] == len(GLOBAL_NAMES)
    assert (x["feats"][..., OLD_FEATS:] == 0).all()


def test_commander_deck_files_round_trip(tmp_path):
    p = tmp_path / "decks.txt"
    p.write_text(cmd_decks.format_line([5], [1, 1, 2], "A: Xenagos") + "\n# a note\n\n"
                 + cmd_decks.format_line([6, 7], [3]) + "\n")  # fmt: skip
    assert cmd_decks.read(p) == [([5], [1, 1, 2]), ([6, 7], [3])]
    assert cmd_decks.names(p) == ["A: Xenagos", ""]
    with pytest.raises(ValueError):
        cmd_decks.parse_line("1 2 3")


def test_deck_exports_with_quantities_expand_to_one_name_per_copy():
    pytest.importorskip("bolt_engine")
    from make_cmd_decks import expand

    assert expand(["A", "Forest", "Forest"]) == ["A", "Forest", "Forest"]
    assert expand({"A": 1, "Forest": 3}) == ["A", "Forest", "Forest", "Forest"]
    assert expand([["A", 1], ["Forest", 2]]) == ["A", "Forest", "Forest"]
    assert expand([{"name": "Forest", "qty": 2}, {"name": "A"}]) == ["Forest", "Forest", "A"]
