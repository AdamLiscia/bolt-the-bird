"""Smoke test for the Tarkir service on 127.0.0.1:8767, and that the FDN one on 8766 still answers:
cards, two random TDM decks, a two-human game played a few moves, log, spectate, concede, a refused bot game.

    python scripts/smoke_tdm_service.py
"""

import json
import urllib.error
import urllib.request
from pathlib import Path

KEY = (Path.home() / ".config/bolt/key").read_text().strip()


def call(port: int, method: str, path: str, body: dict | None = None) -> tuple[int, dict]:
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}/v1{path}",
        method=method,
        data=json.dumps(body).encode() if body is not None else None,
        headers={"X-Bolt-Key": KEY, "Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=120) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def as_list(deck: dict) -> list[dict]:
    return deck["main"] + [{"name": n, "qty": q} for n, q in deck["lands"].items()]


def main():
    s, c = call(8767, "GET", "/cards?set=TDM")
    cards = c["cards"]
    print("cards", s, len(cards), "tokens", sum(x["token"] for x in cards), "first", cards[0]["name"])

    decks = []
    for seat, drafter in [(0, "human"), (3, "bolt")]:
        s, d = call(8767, "POST", "/deck/random", {"set": "TDM", "drafter": drafter, "seat": seat, "seed": 11 + seat})
        decks.append(d["deck"])
        print("deck", s, "seat", d["seat"], d["deck"]["colors"], d["deck"]["cards"], "cards", d["deck"]["lands"])

    body = {"set": "TDM", "deck_a": as_list(decks[0]), "deck_b": as_list(decks[1]), "human_seats": [0, 1]}
    s, g = call(8767, "POST", "/game", {**body, "frames": True, "seed": 5})
    gid = g["game_id"]
    print("game", s, gid)
    for k in range(14):
        who = call(8767, "GET", f"/game/{gid}")[1]["to_act"]
        view = call(8767, "GET", f"/game/{gid}?seat={who}")[1]
        acts = view["legal_actions"]
        if not acts:
            print("no actions")
            break
        pick = next((a for a in acts if a["kind"] in ("Keep", "PlayLand", "Cast")), acts[0])
        move = {"seat": who, "action_index": pick["index"], "version": view["version"], "frames": True}
        s, r = call(8767, "POST", f"/game/{gid}/act", move)
        print(f"move {k}: seat {who} {pick['label']!r} -> {s}")

    s, lg = call(8767, "GET", f"/game/{gid}/log?since=0")
    print("log", s, len(lg.get("events", [])), "events")
    s, sp = call(8767, "GET", f"/game/{gid}?seat=1&spectate=1")
    print("spectate", s, sp.get("spectate"))
    s, cc = call(8767, "POST", f"/game/{gid}/concede", {"seat": 0})
    print("concede", s, cc.get("result") or cc.get("state", {}).get("result"))
    s, bot = call(8767, "POST", "/game", {**body, "human_seat": 0, "human_seats": None, "pilot": "smart"})
    print("bot game refused:", s, bot.get("error"))
    s, h = call(8766, "GET", "/health")
    print("8766:", s, h.get("sets"), "games_live", h.get("games_live"))


if __name__ == "__main__":
    main()
