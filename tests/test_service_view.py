"""The service's view of a game (docs/api.md): one seat's information only, readable moves."""

import pytest

be = pytest.importorskip("bolt_engine")

from bolt.service_view import state  # noqa: E402


def decks():
    with open("data/m7/decks.txt") as f:
        lines = f.read().split("\n")
    return [list(map(int, x.split())) for x in lines[512:514]]


def states(n=300, seed=4):
    a, b = decks()
    g = be.Game(a, b, seed=seed, first=0)
    out = []
    for v in range(n):
        if g.is_over():
            break
        who = g.to_act()
        out.append((g, who, state(g, who, version=v, game_id="t")))
        g.step(g.baseline())
    return out


def test_a_seat_never_sees_the_opponents_hand():
    for _g, who, s in states():
        assert s["view"]["players"][1 - who]["hand"] is None
        assert len(s["view"]["players"][who]["hand"]) == s["view"]["players"][who]["hand_size"]


def test_every_move_has_a_label_and_names_real_objects():
    for _g, _who, s in states():
        ids = {o["id"] for o in s["view"]["battlefield"]} | {o["id"] for o in s["view"]["stack"]}
        ids |= {o["id"] for p in s["view"]["players"] for o in (p["hand"] or []) + p["graveyard"]}
        ids |= {o["id"] for o in s["view"]["exile"]}
        for i, a in enumerate(s["legal_actions"]):
            assert a["index"] == i and a["label"] and a["kind"]
            assert len(a["objects"]) == len(a["cards"])
            for oid, name in zip(a["objects"], a["cards"], strict=True):
                assert isinstance(name, str) and name
                if a["kind"] not in ("Pick",):  # library searches name hidden cards
                    assert oid in ids, (a, oid)


def test_names_come_from_the_engine():
    names = set(be.card_names())
    for _g, _who, s in states(80):
        for o in s["view"]["battlefield"]:
            assert o["name"] in names
