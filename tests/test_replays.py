"""Turning 17lands replay rows into per-turn decision records."""

from bolt.replays import CardMap, block_turns, user_turns

# A tiny card list: 17lands id -> name, and the engine's card names (index = card number).
IDS = {"101": "Plains", "102": "Savannah Lions", "103": "Giant Growth", "104": "Serra Angel", "105": "Mystery Card"}
ENGINE = ["Plains", "Savannah Lions", "Giant Growth", "Serra Angel"]
CM = CardMap(IDS, ENGINE)


def row(**kw):
    """A replay row with sensible defaults: on the play, 2 user turns, one opponent turn between."""
    r = {
        "on_play": "True", "won": "True", "num_turns": "2", "num_mulligans": "0", "opening_hand": "101|101|102|103|104",
        "user_turn_1_cards_drawn": "", "user_turn_1_lands_played": "101", "user_turn_1_creatures_cast": "",
        "user_turn_1_non_creatures_cast": "", "user_turn_1_user_instants_sorceries_cast": "",
        "user_turn_1_creatures_attacked": "", "user_turn_1_cards_discarded": "",
        "user_turn_1_eot_user_cards_in_hand": "101|102|103|104", "user_turn_1_eot_user_lands_in_play": "101",
        "user_turn_1_eot_user_creatures_in_play": "", "user_turn_1_eot_oppo_creatures_in_play": "",
        "user_turn_1_eot_user_life": "20.0", "user_turn_1_eot_oppo_life": "20.0", "user_turn_1_eot_oppo_cards_in_hand": "7.0",
        "oppo_turn_1_creatures_attacked": "", "oppo_turn_1_creatures_blocking": "",
        "oppo_turn_1_eot_user_cards_in_hand": "101|102|103|104", "oppo_turn_1_eot_user_lands_in_play": "101",
        "oppo_turn_1_eot_user_creatures_in_play": "", "oppo_turn_1_eot_oppo_creatures_in_play": "102",
        "oppo_turn_1_eot_user_life": "20.0", "oppo_turn_1_eot_oppo_life": "20.0", "oppo_turn_1_eot_oppo_cards_in_hand": "6.0",
        "user_turn_2_cards_drawn": "102", "user_turn_2_lands_played": "101", "user_turn_2_creatures_cast": "102|102",
        "user_turn_2_non_creatures_cast": "", "user_turn_2_user_instants_sorceries_cast": "",
        "user_turn_2_creatures_attacked": "", "user_turn_2_cards_discarded": "",
        "user_turn_2_eot_user_cards_in_hand": "103|104", "user_turn_2_eot_user_lands_in_play": "101|101",
        "user_turn_2_eot_user_creatures_in_play": "102|102", "user_turn_2_eot_oppo_creatures_in_play": "102",
        "user_turn_2_eot_user_life": "20.0", "user_turn_2_eot_oppo_life": "20.0", "user_turn_2_eot_oppo_cards_in_hand": "6.0",
        "oppo_turn_2_creatures_attacked": "102", "oppo_turn_2_creatures_blocking": "102",
        "oppo_turn_2_eot_user_cards_in_hand": "103|104", "oppo_turn_2_eot_user_lands_in_play": "101|101",
        "oppo_turn_2_eot_user_creatures_in_play": "102", "oppo_turn_2_eot_oppo_creatures_in_play": "",
        "oppo_turn_2_eot_user_life": "20.0", "oppo_turn_2_eot_oppo_life": "20.0", "oppo_turn_2_eot_oppo_cards_in_hand": "6.0",
    }  # fmt: skip
    r.update(kw)
    return r


def test_card_ids_map_to_engine_cards_and_unknowns_are_marked():
    assert CM["102"] == 1 and CM["101"] == 0
    assert CM["105"] == CM.unknown == len(ENGINE)
    assert CM["999"] == CM.unknown
    assert CM.ids("101|104||") == [0, 3]


def test_on_the_play_turn_one_starts_from_the_opening_hand():
    t1, t2 = user_turns(row(), CM)
    assert t1["turn"] == 1 and t1["hand"] == [0, 0, 1, 2, 3] and t1["lands_played"] == [0]
    assert t1["my_lands"] == [] and t1["opp_creatures"] == []


def test_a_later_turn_starts_from_the_previous_snapshot_plus_the_draw():
    _, t2 = user_turns(row(), CM)
    assert t2["hand"] == [0, 1, 2, 3, 1], "end of the opponent's turn 1, plus the card drawn"
    assert t2["my_lands"] == [0] and t2["opp_creatures"] == [1]
    assert t2["cast"] == [1, 1] and t2["consistent"]
    assert (t2["my_life"], t2["opp_life"], t2["opp_hand"]) == (20, 20, 6)


def test_on_the_draw_a_user_turn_starts_after_the_opponent_turn_with_the_same_number():
    r = row(on_play="False", user_turn_1_cards_drawn="104",
            oppo_turn_1_eot_user_cards_in_hand="101|101|102|103|104", oppo_turn_1_eot_oppo_creatures_in_play="102",
            user_turn_1_eot_user_cards_in_hand="101|102|103|104|104")  # fmt: skip
    t1 = user_turns(r, CM)[0]
    assert t1["hand"] == [0, 0, 1, 2, 3, 3] and t1["opp_creatures"] == [1]


def test_instants_count_as_cast_and_a_card_that_vanishes_is_flagged():
    r = row(user_turn_2_user_instants_sorceries_cast="103", user_turn_2_eot_user_cards_in_hand="104")
    t2 = user_turns(r, CM)[1]
    assert t2["cast"] == [1, 1, 2] and t2["consistent"]
    bad = user_turns(row(user_turn_2_eot_user_cards_in_hand="104"), CM)[1]
    assert not bad["consistent"], "Giant Growth left the hand without being cast"


def test_blocks_come_from_the_opponents_turns():
    b = block_turns(row(), CM)
    assert len(b) == 1, "only turns where the opponent attacked"
    assert b[0]["attackers"] == [1] and b[0]["blockers"] == [1] and b[0]["my_creatures"] == [1, 1]


def test_turns_after_the_game_ended_are_ignored():
    """17lands fills every turn column up to 30, with zeros once the game is over; num_turns is
    the number of turns each player took."""
    r = row(user_turn_3_eot_user_life="0.0", user_turn_3_cards_drawn="", oppo_turn_3_eot_user_life="0.0",
            oppo_turn_3_creatures_attacked="102")  # fmt: skip
    assert len(user_turns(r, CM)) == 2
    assert [b["turn"] for b in block_turns(r, CM)] == [2]


def test_pack_stores_card_lists_as_fixed_width_rows():
    import numpy as np

    from bolt.replays import pack

    out, cut = pack([[0, 3], [], [1] * 5], width=4)
    assert out.dtype == np.int16 and out.shape == (3, 4)
    assert out[0].tolist() == [1, 4, 0, 0], "card numbers + 1, zero-padded"
    assert out[2].tolist() == [2, 2, 2, 2] and cut == 1, "a list longer than the width is cut and counted"


def test_odd_numbers_in_the_data_are_clamped_not_fatal():
    """One real game records a life total of infinity."""
    r = row(oppo_turn_1_eot_user_life="inf", oppo_turn_1_eot_oppo_life="nan", oppo_turn_1_eot_oppo_cards_in_hand="-inf")
    t2 = user_turns(r, CM)[1]
    assert t2["my_life"] == 999 and t2["opp_life"] == 20 and t2["opp_hand"] == -999
