"""The Bolt service (docs/api.md), against the real models and engine."""

import pytest

pytest.importorskip("bolt_engine")
from fastapi.testclient import TestClient  # noqa: E402

from bolt.service import create_app, load_kit  # noqa: E402

KEY = {"X-Bolt-Key": "test-key"}


@pytest.fixture(scope="module")
def client():
    c = TestClient(create_app(load_kit(), key="test-key", max_games=3))
    c.cleanup = []
    return c


@pytest.fixture(scope="module")
def names(client):
    return [c["name"] for c in client.get("/v1/cards?set=FDN", headers=KEY).json()["cards"]]


def test_every_call_needs_the_key(client):
    assert client.get("/v1/health").status_code == 401
    assert client.get("/v1/health", headers={"X-Bolt-Key": "wrong"}).status_code == 401
    h = client.get("/v1/health", headers=KEY).json()
    assert h["ok"] and h["sets"] == ["FDN"]


def test_the_card_list_is_the_286_fdn_cards(client, names):
    assert len(names) == 286 and "Savannah Lions" in names and "Plains" in names
    cards = client.get("/v1/cards?set=FDN", headers=KEY).json()["cards"]
    assert all(c["image"].startswith("https://cards.scryfall.io/normal/") for c in cards), "a Scryfall image for each"


def test_batched_picks_return_a_name_from_each_seats_own_pack(client, names):
    seats = [{"seat": s, "pack": names[s * 14 : s * 14 + 14], "pool": names[200 : 200 + s], "pack_number": 1,
              "pick_number": 1 + s} for s in range(7)]  # fmt: skip
    for path in ("/v1/draft/picks", "/draft/picks"):
        r = client.post(path, headers=KEY, json={"set": "FDN", "bot": "bolt", "seats": seats})
        assert r.status_code == 200, r.text
        picks = r.json()["picks"]
        assert [p["seat"] for p in picks] == list(range(7))
        for p, s in zip(picks, seats, strict=True):
            assert p["name"] in s["pack"]


def test_a_single_pick_ranks_the_whole_pack(client, names):
    pack = names[:10] + names[:2]  # two duplicates
    r = client.post("/v1/draft/pick", headers=KEY,
                    json={"set": "FDN", "pack": pack, "pool": [], "pack_number": 1, "pick_number": 1, "bot": "human"})  # fmt: skip
    ranked = r.json()["ranked"]
    assert {x["name"] for x in ranked} == set(pack)
    assert abs(sum(x["score"] for x in ranked) - 1) < 1e-4
    assert sum(x["count"] for x in ranked) == len(pack)
    assert ranked == sorted(ranked, key=lambda x: -x["score"])


def test_unknown_names_are_named_never_dropped(client, names):
    seats = [{"seat": 2, "pack": [names[0], "Black Lotus"], "pool": ["Not A Card"], "pack_number": 1, "pick_number": 1}]
    r = client.post("/v1/draft/picks", headers=KEY, json={"set": "FDN", "bot": "bolt", "seats": seats})
    assert r.status_code == 422 and set(r.json()["unknown_names"]) == {"Black Lotus", "Not A Card"}
    r = client.post("/v1/draft/picks", headers=KEY, json={"set": "MKM", "bot": "bolt", "seats": seats})
    assert r.status_code == 422


def test_a_random_bot_draft_gives_42_picks_and_a_40_card_deck(client):
    a = client.post("/v1/deck/random", headers=KEY, json={"set": "FDN", "seed": 7}).json()
    b = client.post("/v1/deck/random", headers=KEY, json={"set": "FDN", "seed": 7}).json()
    assert a == b, "same seed, same draft"
    assert len(a["picks"]) == 42 and a["picks"][0] == {
        "pack_number": 1,
        "pick_number": 1,
        "name": a["picks"][0]["name"],
    }
    deck = a["deck"]
    assert sum(x["qty"] for x in deck["main"]) + sum(deck["lands"].values()) == deck["cards"] == 40
    assert set(deck["colors"]) <= set("WUBRG") and 1 <= len(deck["colors"]) <= 2
    pool = [p["name"] for p in a["picks"]]
    for x in deck["main"]:
        assert pool.count(x["name"]) >= x["qty"], "the deck only uses drafted cards (and basics)"


def deck_of(client, seed):
    d = client.post("/v1/deck/random", headers=KEY, json={"set": "FDN", "seed": seed}).json()["deck"]
    basic = {"W": "Plains", "U": "Island", "B": "Swamp", "R": "Mountain", "G": "Forest"}
    return d["main"] + [{"name": basic[c], "qty": n} for c, n in d["lands"].items()]


ORDER = [
    "Keep",
    "PlayLand",
    "Cast",
    "CastX",
    "CastKicked",
    "Attack",
    "Block",
    "Target",
    "TargetPlayer",
    "Yes",
    "Bottom",
    "Discard",
    "Pick",
    "Mode",
    "Amount",
    "FinishAttacks",
    "FinishBlocks",
    "PassPriority",
    "No",
]


def choose(legal):
    """A plain stand-in for a person: play lands and spells, attack and block, then move on; never
    reset blocks or re-activate abilities (either can repeat forever)."""
    for kind in ORDER:
        for a in legal:
            if a["kind"] == kind:
                return a
    return next(
        (a for a in legal if a["kind"] not in ("ResetBlocks", "Activate", "ActivateX", "ActivateWith")), legal[0]
    )


def test_a_whole_game_against_the_bot(client):
    r = client.post("/v1/game", headers=KEY, json={"set": "FDN", "deck_a": deck_of(client, 1), "deck_b": deck_of(client, 2),
                                                   "human_seat": 0, "pilot": "network", "seed": 5, "first": 0})  # fmt: skip
    assert r.status_code == 200, r.text
    gid, s = r.json()["game_id"], r.json()["state"]
    client.cleanup.append(gid)
    seen = set()
    for _ in range(4000):
        if s["over"]:
            break
        assert s["to_act"] == 0 and s["legal_actions"], "the bot has acted up to our decision"
        assert s["view"]["players"][1]["hand"] is None
        seen.add(s["pending"]["kind"])
        pick = choose(s["legal_actions"])
        r = client.post(
            f"/v1/game/{gid}/act", headers=KEY, json={"seat": 0, "action_index": pick["index"], "version": s["version"]}
        )
        assert r.status_code == 200, r.text
        assert r.json()["version"] > s["version"]
        s = r.json()
    assert s["over"] and s["result"]["outcome"] in ("win", "draw", "timeout")
    assert "Priority" in seen and len(seen) >= 3
    log = client.get(f"/v1/game/{gid}/log", headers=KEY).json()
    assert log["events"] and all(e["n"] == i for i, e in enumerate(log["events"]))
    assert client.get(f"/v1/game/{gid}?seat=0", headers=KEY).json()["over"]
    assert client.delete(f"/v1/game/{gid}", headers=KEY).status_code == 200
    assert client.get(f"/v1/game/{gid}?seat=0", headers=KEY).status_code == 404


def test_stale_versions_and_bad_moves_are_refused(client):
    r = client.post("/v1/game", headers=KEY, json={"set": "FDN", "deck_a": deck_of(client, 3), "deck_b": deck_of(client, 4),
                                                   "human_seat": 1, "pilot": "rules", "seed": 6})  # fmt: skip
    gid, s = r.json()["game_id"], r.json()["state"]
    client.cleanup.append(gid)
    act = {"seat": 1, "action_index": 0, "version": s["version"]}
    assert client.post(f"/v1/game/{gid}/act", headers=KEY, json=act).status_code == 200
    again = client.post(f"/v1/game/{gid}/act", headers=KEY, json=act)
    assert again.status_code == 409 and "state" in again.json(), "a repeated click is refused, with the current state"
    cur = again.json()["state"]
    bad = client.post(
        f"/v1/game/{gid}/act", headers=KEY, json={"seat": 1, "action_index": 999, "version": cur["version"]}
    )
    assert bad.status_code == 422
    wrong_seat = client.post(
        f"/v1/game/{gid}/act", headers=KEY, json={"seat": 0, "action_index": 0, "version": cur["version"]}
    )
    assert wrong_seat.status_code == 409
    client.delete(f"/v1/game/{gid}", headers=KEY)


def test_bad_decks_and_the_game_limit(client, names):
    for g in client.cleanup:  # games other tests left behind if they failed
        client.delete(f"/v1/game/{g}", headers=KEY)
    short = [{"name": "Plains", "qty": 39}]
    r = client.post("/v1/game", headers=KEY, json={"set": "FDN", "deck_a": short, "deck_b": short, "human_seat": 0})
    assert r.status_code == 422
    r = client.post("/v1/game", headers=KEY, json={"set": "FDN", "deck_a": [{"name": "Black Lotus", "qty": 40}],
                                                   "deck_b": [{"name": "Plains", "qty": 40}], "human_seat": 0})  # fmt: skip
    assert r.status_code == 422 and r.json()["unknown_names"] == ["Black Lotus"]
    deck = [{"name": "Plains", "qty": 40}]
    ids = []
    for _ in range(3):
        r = client.post("/v1/game", headers=KEY, json={"set": "FDN", "deck_a": deck, "deck_b": deck, "human_seat": 0})
        assert r.status_code == 200
        ids.append(r.json()["game_id"])
    r = client.post("/v1/game", headers=KEY, json={"set": "FDN", "deck_a": deck, "deck_b": deck, "human_seat": 0})
    assert r.status_code == 429
    for g in ids:
        client.delete(f"/v1/game/{g}", headers=KEY)
