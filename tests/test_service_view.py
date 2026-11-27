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


def test_color_and_creature_type_choices_read_as_what_they_are():
    from bolt.service_view import actions, name

    banner = be.index_of("Heraldic Banner")
    base = {"legal_cards": [(-1, -1, -1)] * 5, "decision": {"kind": "ChooseColor", "card": (9, banner)}}
    obs = {**base, "legal_actions": [("Mode", m, -1, -1) for m in range(5)]}
    labels = [a["label"] for a in actions(obs, 0)]
    assert labels == [f"{name(banner)}: choose {c}" for c in ("White", "Blue", "Black", "Red", "Green")]
    obs = {"legal_actions": [("Mode", 0, -1, -1), ("Mode", 1, -1, -1)], "legal_cards": [(-1, -1, -1)] * 2,
           "decision": {"kind": "ChooseCreatureType", "card": (9, banner), "options": ["Elf", "Goblin"]}}  # fmt: skip
    assert [a["label"] for a in actions(obs, 0)] == [f"{name(banner)}: choose Elf", f"{name(banner)}: choose Goblin"]
    obs = {"legal_actions": [("Mode", 0, -1, -1)], "legal_cards": [(-1, -1, -1)], "decision": {"kind": "ChooseMode"}}
    assert actions(obs, 0)[0]["label"] == "Choose mode 1", "other Mode choices keep the plain label"


def yes_no(decision):
    from bolt.service_view import actions

    obs = {"legal_actions": [("Yes", -1, -1, -1), ("No", -1, -1, -1)], "legal_cards": [(-1, -1, -1)] * 2,
           "decision": decision}  # fmt: skip
    return actions(obs, 0)


def test_yes_and_no_say_what_they_do():
    forest, thespian = be.index_of("Forest"), be.index_of("Wary Thespian")
    yes, no = yes_no({"kind": "Surveil", "source": (4, thespian), "cards": [(17, forest)]})
    assert (yes["label"], no["label"]) == ("Put Forest into your graveyard", "Keep Forest on top")
    assert yes["objects"] == no["objects"] == [17] and yes["cards"] == ["Forest"]
    yes, no = yes_no({"kind": "Tuck", "card": (17, forest)})
    assert (yes["label"], no["label"]) == ("Put Forest on top of your library", "Put Forest on the bottom")
    yes, no = yes_no({"kind": "May", "source": (4, thespian)})
    assert yes["label"] == "Wary Thespian: yes" and no["label"] == "Wary Thespian: no"
    yes, no = yes_no({"kind": "MayPay", "source": (4, thespian), "cost": "{2}"})
    assert (yes["label"], no["label"]) == ("Pay {2} (Wary Thespian)", "Don't pay")
    yes, no = yes_no({"kind": "PayWard", "cost": "{1}", "card": (4, thespian)})
    assert yes["label"] == "Pay ward {1} for Wary Thespian" and "countered" in no["label"]


def test_the_decider_sees_what_it_is_deciding():
    from bolt.service_view import _pending

    forest, thespian = be.index_of("Forest"), be.index_of("Wary Thespian")
    p = _pending("Decide", 0, {"kind": "Surveil", "source": (4, thespian), "cards": [(17, forest), (18, forest)]})
    assert p["choice"] == "Surveil" and p["source"] == {"id": 4, "name": "Wary Thespian", "card_id": thespian}
    assert [c["id"] for c in p["cards"]] == [17, 18] and p["cards"][0]["name"] == "Forest"
    assert "surveil" in p["source_text"].lower(), "the asking card's rules text, for the prompt"


def test_every_object_carries_its_card_number():
    for _g, _who, s in states(120):
        objs = s["view"]["battlefield"] + s["view"]["stack"] + s["view"]["exile"]
        objs += [o for p in s["view"]["players"] for o in (p["hand"] or []) + p["graveyard"]]
        for o in objs:
            assert be.card_names()[o["card_id"]] == o["name"]


def test_a_target_choice_names_what_the_targets_are_for():
    from bolt.service_view import _pending

    gg = be.index_of("Giant Growth")
    p = _pending("ChooseTarget", 1, None, {"id": 40, "card": gg, "controller": 1, "spell": True, "targets_chosen": 0})
    assert p["source"] == {"id": 40, "name": "Giant Growth", "card_id": gg} and p["spell"] is True
    assert "+3/+3" in p["source_text"]


def test_a_mulligan_says_how_many_cards_are_left_to_bottom():
    deck = [be.index_of("Plains")] * 20 + [be.index_of("Savannah Lions")] * 20
    g = be.Game(deck, deck, seed=4, first=0)
    g.auto_pass = False
    who = g.to_act()
    mull = next(i for i, a in enumerate(g.legal_actions()) if a[0] == "Mulligan")
    g.step(mull)
    for _ in range(4):  # the other player keeps; then the mulliganed player keeps and bottoms
        obs = g.observation(g.to_act())
        if obs["pending"][0] == "Bottom":
            break
        g.step(next(i for i, a in enumerate(g.legal_actions()) if a[0] == "Keep"))
    from bolt.service_view import state

    s = state(g, who, version=0, game_id="t")
    assert s["pending"]["kind"] == "Bottom" and s["pending"]["left"] == 1


def test_the_view_says_who_the_monarch_is():
    _, _, s = states(n=1)[0]
    assert "monarch" in s and s["monarch"] is None, "nobody is the monarch at the start"
