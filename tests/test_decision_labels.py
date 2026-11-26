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


def test_an_unknown_decision_keeps_the_generic_label():
    assert label({"kind": "SomethingNew"}, "Mode", 0) is None
