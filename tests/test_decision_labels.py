"""Decision labels (service_view._decision_label): what a human player reads for each answer. No engine needed:
the decision dicts are written by hand, the shape the engine's observation gives them."""

import pytest

import bolt.service_view as sv

CARDS = {0: "Darkbore Pathway // Slitherbore Pathway", 1: "Thriving Heath", 2: "Secluded Courtyard"}


@pytest.fixture(autouse=True)
def names(monkeypatch):
    monkeypatch.setattr(sv, "name", lambda i: CARDS[i])


def label(dec, kind, value):
    return sv._decision_label(dec, kind, value, "")


def test_a_pathway_names_the_face_it_enters_as():
    dec = {"kind": "ChooseLandFace", "card": [7, 0]}
    assert label(dec, "Mode", 0) == "Play it as Darkbore Pathway"
    assert label(dec, "Mode", 1) == "Play it as Slitherbore Pathway"


def test_a_color_choice_names_the_color():
    dec = {"kind": "ChooseColor", "card": [7, 1]}
    assert label(dec, "Mode", 4) == "Thriving Heath: choose Green"


def test_a_creature_type_choice_names_the_type():
    dec = {"kind": "ChooseCreatureType", "card": [7, 2], "options": ["Elf", "Goblin"]}
    assert label(dec, "Mode", 1) == "Secluded Courtyard: choose Goblin"


def test_lane_d3_decisions_read_as_their_moves():
    # Tooth and Nail, Dream Stalker, Oriq Loremage: Pick a card; Ponder: Yes or No.
    assert sv._decision_label({"kind": "CreaturesFromHandUpTo", "left": 2}, "Pick", 0, "Ashen Rider") == (
        "Put onto the battlefield: Ashen Rider"
    )
    assert label({"kind": "CreaturesFromHandUpTo", "left": 1}, "No", 0) == "Done choosing"
    assert sv._decision_label({"kind": "ReturnOwnPermanent", "nonland": True}, "Pick", 0, "Sol Ring") == (
        "Return Sol Ring"
    )
    dec = {"kind": "SearchToGraveyardCounter", "source": [7, 1]}
    assert sv._decision_label(dec, "Pick", 0, "Counterspell") == "Put into your graveyard: Counterspell"
    assert label(dec, "No", 0) == "Find nothing"
    assert label({"kind": "MayShuffle"}, "Yes", 0) == "Shuffle your library"
    assert label({"kind": "MayShuffle"}, "No", 0) == "Don't shuffle"


def test_an_unknown_decision_keeps_the_generic_label():
    assert label({"kind": "SomethingNew"}, "Mode", 0) is None
