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


def test_lane_d3_batch_5_decisions_read_as_their_moves():
    pick = lambda d, n: sv._decision_label(d, "Pick", 0, n)  # noqa: E731
    assert pick({"kind": "KeepNonlandOfType", "ty": 0}, "Sol Ring") == "Keep Sol Ring"
    assert pick({"kind": "HandToBottom", "n": 3}, "Opt") == "Put on the bottom: Opt"
    assert pick({"kind": "SacrificeAnyNumber", "n": 0}, "Sol Ring") == "Sacrifice Sol Ring"
    assert label({"kind": "SacrificeAnyNumber", "n": 1}, "No", 0) == "Done sacrificing"
    assert pick({"kind": "SacrificeCreatureOrPlaneswalkerEach"}, "Grizzly Bears") == "Sacrifice Grizzly Bears"
    assert pick({"kind": "ReturnAnotherPermanentCard"}, "Sol Ring") == "Return to your hand: Sol Ring"
    assert label({"kind": "ReturnAnotherPermanentCard"}, "No", 0) == "Return nothing"
    assert pick({"kind": "BasicsTappedUpTo", "left": 2}, "Forest") == "Put onto the battlefield tapped: Forest"
    assert label({"kind": "BasicsTappedUpTo", "left": 2}, "No", 0) == "Find nothing"
    assert pick({"kind": "SignalTheClans"}, "Merfolk Looter") == "Reveal Merfolk Looter"
    assert label({"kind": "SignalTheClans"}, "No", 0) == "Done revealing"
    assert pick({"kind": "BraidsSacrifice"}, "Sol Ring") == "Sacrifice Sol Ring"
    assert label({"kind": "BraidsSacrifice"}, "No", 0) == "Don't sacrifice"
    assert label({"kind": "BraidsOpponent"}, "No", 0) == "Don't sacrifice (lose 2 life)"
    assert label({"kind": "TargetToSource"}, "Mode", 0) == "Change target 1 to this creature"
    assert label({"kind": "TaintedPact"}, "Yes", 0) == "Put it into your hand"
    assert label({"kind": "TaintedPact"}, "No", 0) == "Exile the next card"
    assert label({"kind": "ProtectionColor"}, "Mode", 3) == "Protection from red"
    assert label({"kind": "ChooseCardType"}, "Mode", 3) == "It: protection from instants"
    assert pick({"kind": "ConniveDiscard", "left": 2}, "Plains") == "Discard (connive): Plains"
    vow = pick({"kind": "VowCreature", "caster": 0, "kept": [], "last": False}, "Llanowar Elves")
    assert vow == "Put a vow counter on (sacrifice the rest): Llanowar Elves"
    assert pick({"kind": "DiscardOrLoseLife", "left": 6, "life": 3}, "Opt") == "Discard Opt"
    assert label({"kind": "DiscardOrLoseLife", "left": 6, "life": 3}, "No", 0) == "Don't discard (lose 3 life)"
    assert label({"kind": "DiscardOrLoseLife"}, "No", 0) == "Don't discard (lose the life)"
    assert pick({"kind": "LoseUnless", "life": 2, "sacrifice": True}, "Forest") == "Sacrifice Forest"
    assert pick({"kind": "LoseUnless", "life": 2, "sacrifice": False}, "Opt") == "Discard Opt"
    assert label({"kind": "LoseUnless", "life": 2, "sacrifice": True}, "No", 0) == "Don't sacrifice (lose 2 life)"
    assert pick({"kind": "SacrificeOfType", "ty": 0}, "Sol Ring") == "Sacrifice Sol Ring"
    assert pick({"kind": "RevealedCreatureOnto"}, "Serra Angel") == "Put onto the battlefield: Serra Angel"
    assert pick({"kind": "SacrificeAnotherPermanent", "keep": 4}, "Sol Ring") == "Sacrifice Sol Ring"
    assert label({"kind": "MayPayLife", "life": 2}, "Yes", 0) == "Pay 2 life"
    assert label({"kind": "MayPayLife", "life": 2}, "No", 0) == "Don't pay"
    assert label({"kind": "MayPayManaAndLife", "cost": "{1}", "life": 3}, "Yes", 0) == "Pay {1} and 3 life"
    assert label({"kind": "MayPayManaAndLife", "cost": "{1}", "life": 3}, "No", 0) == "Don't pay"
    assert pick({"kind": "TakeMilledCard", "cards": []}, "Opt") == "Put into your hand: Opt"
    assert pick({"kind": "ReturnSharingType", "with": 3}, "Sol Ring") == "Return to its owner's hand: Sol Ring"
    assert label({"kind": "ReturnSharingType", "with": 3}, "No", 0) == "Return nothing"
    assert pick({"kind": "ExileOwnPermanent", "left": 1}, "Sol Ring") == "Exile Sol Ring"
    assert pick({"kind": "IterationPick", "cards": [], "to_hand": True}, "Opt") == "Put into your hand: Opt"
    assert pick({"kind": "IterationPick", "cards": [], "to_hand": False}, "Opt") == "Put on the bottom: Opt"
    cast_later = "You may cast this turn (then exile it): Opt"
    assert pick({"kind": "CastFromGraveyardThisTurn", "cards": []}, "Opt") == cast_later
    assert pick({"kind": "TakeTwoBottomRest", "cards": [], "left": 2}, "Opt") == "Put into your hand: Opt"
    torment = {"kind": "SacrificeDiscardOrLoseLife", "left": 2, "life": 3}
    assert pick(torment, "Opt") == "Sacrifice (or discard, from your hand): Opt"
    assert label(torment, "No", 0) == "Don't sacrifice or discard (lose 3 life)"
    brass = {"kind": "ReanimateTotalManaValue", "left": 9}
    assert pick(brass, "Serra Angel") == "Put onto the battlefield under your control: Serra Angel"
    assert label(brass, "No", 0) == "Done choosing"
    marauder = {"kind": "SacrificeNontokenCreatureEach", "next": 1}
    assert pick(marauder, "Serra Angel") == "Sacrifice Serra Angel"


def test_lonis_puts_a_revealed_card_under_your_control():
    dec = {"kind": "PutFromOpponentsTop", "cards": []}
    got = sv._decision_label(dec, "Pick", 0, "Sol Ring")
    assert got == "Put onto the battlefield under your control: Sol Ring"
    assert label(dec, "No", 0) == "Put none onto the battlefield"


def test_nahiris_resolve_exiles_until_your_next_upkeep():
    dec = {"kind": "ExileUntilYourNextUpkeep"}
    got = sv._decision_label(dec, "Pick", 0, "Sol Ring")
    assert got == "Exile until your next upkeep: Sol Ring"
    assert label(dec, "No", 0) == "Done exiling"


def test_venturing_names_the_dungeon_or_room():
    dec = {"kind": "ChooseDungeon", "options": ["Lost Mine of Phandelver", "Dungeon of the Mad Mage"]}
    assert label(dec, "Mode", 1) == "Venture into Dungeon of the Mad Mage"
    dec = {"kind": "ChooseRoom", "options": ["Goblin Lair", "Mine Tunnels"]}
    assert label(dec, "Mode", 0) == "Venture into the room Goblin Lair"


def test_white_plume_adventurer_untaps_a_creature_by_name():
    dec = {"kind": "UntapOwnCreature"}
    assert sv._decision_label(dec, "Pick", 0, "Llanowar Elves") == "Untap Llanowar Elves"


def test_the_ring_tempting_you_names_the_ring_bearer():
    dec = {"kind": "ChooseRingBearer"}
    label = sv._decision_label(dec, "Pick", 0, "Llanowar Elves")
    assert label == "Choose as your Ring-bearer: Llanowar Elves"


def test_a_split_or_omen_second_half_is_cast_by_its_name():
    assert sv._label("CastOmen", ["Fire // Ice"], (), 0, (0, 0, 0)) == "Cast Ice"
    assert sv._label("CastOmen", ["Sol Ring"], (), 0, (0, 0, 0)) == "Cast Sol Ring's Omen"


def test_an_unknown_decision_keeps_the_generic_label():
    assert label({"kind": "SomethingNew"}, "Mode", 0) is None


def test_a_pact_says_not_paying_loses_the_game():
    dec = {"kind": "PayOrLoseGame", "cost": "{3}{U}{U}"}
    assert label(dec, "Yes", 0) == "Pay {3}{U}{U}"
    assert label(dec, "No", 0) == "Don't pay (you lose the game)"
    sourced = {**dec, "source": [5, 1]}
    assert label(sourced, "Yes", 0) == "Pay {3}{U}{U} (Thriving Heath)"


def test_rootcast_and_frugivore_decisions_read_as_their_moves():
    rootcast = {"kind": "SacrificeNontokenArtifact"}
    assert sv._decision_label(rootcast, "Pick", 0, "Sol Ring") == "Sacrifice Sol Ring"
    frugivore = {"kind": "ExileThreeForFood", "food": 0, "chosen": []}
    assert sv._decision_label(frugivore, "Pick", 0, "Plains") == "Exile (three for a Food): Plains"
    assert label(frugivore, "No", 0) == "Don't exile (no more Food)"
