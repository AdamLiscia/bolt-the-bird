"""The practice web app's API, with stand-in models (no checkpoints needed)."""

import pytest

pytest.importorskip("fastapi")
from fastapi.testclient import TestClient  # noqa: E402

from practice.app import Kit, create_app  # noqa: E402
from tests.test_practice import N_CARDS, LowestFirst, packs  # noqa: E402

CARDS = [{"name": f"Card {i}", "color": "W", "rarity": "common", "types": ["Creature"]} for i in range(N_CARDS)]


def builder(pools):
    """Stand-in deck builder: the first 23 cards of the pool plus 17 of card 0 (a 'basic')."""
    return [p[:23] + [0] * 17 for p in pools], ["W 17 lands"] * len(pools)


def fake_play(jobs, **_):
    return [("p0", 9)] * len(jobs)


def kit():
    return Kit(packs(), LowestFirst(), LowestFirst(), builder, fake_play, CARDS, {0}, {})


def client(password=None):
    return TestClient(create_app(kit(), password=password))


def test_cards_are_listed_with_names():
    r = client().get("/api/cards").json()
    assert len(r) == N_CARDS and r[3]["name"] == "Card 3"


def test_draft_pick_by_pick_to_the_end():
    c = client()
    d = c.post("/api/draft").json()
    did = d["id"]
    assert len(d["pack"]) == 14 and d["pool"] == [] and len(d["coach"]) == 14
    for _ in range(42):
        d = c.post(f"/api/draft/{did}/pick", json={"index": 0}).json()
    assert d["done"] and len(d["pool"]) == 42


def test_a_bad_pick_is_a_400():
    c = client()
    did = c.post("/api/draft").json()["id"]
    assert c.post(f"/api/draft/{did}/pick", json={"index": 99}).status_code == 400
    assert c.post("/api/draft/nope/pick", json={"index": 0}).status_code == 404


def test_deck_suggestion_and_grade_after_the_draft():
    c = client()
    did = c.post("/api/draft").json()["id"]
    assert c.get(f"/api/draft/{did}/deck").status_code == 409, "not before the draft ends"
    for _ in range(42):
        c.post(f"/api/draft/{did}/pick", json={"index": 0})
    s = c.get(f"/api/draft/{did}/deck").json()
    assert len(s["deck"]) == 40
    bad = c.post(f"/api/draft/{did}/grade", json={"deck": s["deck"][:30]})
    assert bad.status_code == 400 and bad.json()["detail"]
    g = c.post(f"/api/draft/{did}/grade", json={"deck": s["deck"], "games": 2}).json()
    assert g["you"]["games"] == 14 and g["bolt"]["win_rate"] == 1.0 and len(g["picks"]) == 42


def test_a_password_keeps_strangers_out():
    c = client(password="hunter2")
    assert c.get("/api/cards").status_code == 401
    assert c.post("/login", data={"password": "wrong"}, follow_redirects=False).status_code == 401
    ok = c.post("/login", data={"password": "hunter2"}, follow_redirects=False)
    assert ok.status_code == 303
    assert c.get("/api/cards").status_code == 200, "the cookie lets you in"
