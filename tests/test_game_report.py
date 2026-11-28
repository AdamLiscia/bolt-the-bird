"""scripts/game_report.py on a synthetic game log."""

import json

from game_report import load, report


def test_per_deck_rows_and_cards_by_win_rate_when_cast(tmp_path):
    rows = []
    for g in range(10):  # deck 0 (A) wins 7 of 10 against deck 1 (B); card 3 is in all of A's wins
        a_wins = g < 7
        rows.append({"iter": 1, "phase": "self", "deck_a": {"key": 0, "commander": "A"},
                     "deck_b": {"key": 1, "commander": "B"}, "result": "p0" if a_wins else "p1", "turn": 10 + g,
                     "end_reason": "life" if g else "commander_damage",
                     "cards_played": [[[2, 3]] if a_wins else [[2, 4]], [[1, 8]]]})  # fmt: skip
    path = tmp_path / "games.jsonl"
    path.write_text("".join(json.dumps(r) + "\n" for r in rows) + json.dumps({**rows[0], "phase": "eval_rules"}) + "\n")
    out = report(load(path, ["self"]), names=[f"card{i}" for i in range(10)], min_casts=3)
    a, b = out[0], out[1]
    assert a["name"] == "A" and a["games"] == 10 and abs(a["win_rate"] - 0.7) < 1e-9 and b["win_rate"] == 0.3
    assert a["avg_win_turn"] == sum(range(10, 17)) / 7
    assert a["ends"] == {"life": 9, "commander_damage": 1}
    assert a["best"][0] == ("card3", 7, 1.0) and ("card4", 3, 0.0) in a["best"]
    assert b["best"] == [("card8", 10, 0.3)] and a["worst"] == []
    assert len(load(path)) == 11
