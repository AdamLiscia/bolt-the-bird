"""Smoke test for the Commander service (default 127.0.0.1:8768): cards, validate a real playgroup deck and
a basics-only deck, a two-human Commander game, a game against the smart pilot, and the refused network pilot and draft.

    python scripts/smoke_cmd_service.py [port]
"""

import json
import sys
import urllib.error
import urllib.request
from pathlib import Path

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 8768
KEY = (Path.home() / ".config/bolt/key").read_text().strip()


def call(method: str, path: str, body: dict | None = None) -> tuple[int, dict]:
    req = urllib.request.Request(
        f"http://127.0.0.1:{PORT}/v1{path}",
        method=method,
        data=json.dumps(body).encode() if body is not None else None,
        headers={"X-Bolt-Key": KEY, "Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=120) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def main():
    s, c = call("GET", "/cards?set=CMD")
    print("cards", s, len(c["cards"]), "tokens", sum(x["token"] for x in c["cards"]))
    pg = json.loads((Path.home() / "bolt-handoff/playgroup_all.json").read_text())
    x = pg["deck_order"][0]
    deck = [{"name": n, "qty": 1} for n in pg["decks"][str(x["deck_id"])]]
    s, v = call("POST", "/deck/validate", {"format": "commander", "commander": x["commander"], "deck": deck})
    print("validate", x["commander"], s, "ok", v.get("ok"), "missing", len(v.get("missing", [])))
    a = [{"name": "Forest", "qty": 50}, {"name": "Mountain", "qty": 49}]
    b = [{"name": "Plains", "qty": 50}, {"name": "Island", "qty": 49}]
    body = {"set": "CMD", "format": "commander", "commander_a": "Bello, Bard of the Brambles", "deck_a": a,
            "commander_b": "Ms. Bumbleflower", "deck_b": b, "human_seats": [0, 1], "frames": True, "seed": 3,
            "auto_command_zone": [False, True]}  # fmt: skip
    s, g = call("POST", "/game", body)
    gid, st = g["game_id"], g["state"]
    print("game", s, gid, "life", [p["life"] for p in st["view"]["players"]],
          "command", [[o["name"] for o in p["command"]] for p in st["view"]["players"]])  # fmt: skip
    for k in range(10):
        who = call("GET", f"/game/{gid}")[1]["to_act"]
        view = call("GET", f"/game/{gid}?seat={who}")[1]
        acts = view["legal_actions"]
        pick = next((a for a in acts if a["kind"] in ("Keep", "PlayLand")), acts[0])
        s, _ = call(
            "POST", f"/game/{gid}/act", {"seat": who, "action_index": pick["index"], "version": view["version"]}
        )
        print(f"move {k}: seat {who} {pick['label']!r} -> {s}")
    # Play vs Bolt: a human seat against the smart pilot (no network needed); the network pilot is refused.
    s, bot = call("POST", "/game", {**body, "human_seats": None, "human_seat": 0, "pilot": "smart", "frames": False})
    assert s == 200, (s, bot)
    gid = bot["game_id"]
    for k in range(8):
        view = call("GET", f"/game/{gid}?seat=0")[1]
        acts = view.get("legal_actions") or []
        if not acts:
            break
        pick = next((a for a in acts if a["kind"] in ("Keep", "PlayLand")), acts[0])
        s, out = call(
            "POST", f"/game/{gid}/act", {"seat": 0, "action_index": pick["index"], "version": view["version"]}
        )
        assert s == 200, (s, out)
        print(f"vs Bolt move {k}: {pick['label']!r} -> {s}")
    s, net = call("POST", "/game", {**body, "human_seats": None, "human_seat": 0, "pilot": "network"})
    print("network pilot refused", s, net.get("error"))
    s, d = call("POST", "/deck/random", {"set": "CMD"})
    print("draft refused", s, d.get("error"))


if __name__ == "__main__":
    main()
