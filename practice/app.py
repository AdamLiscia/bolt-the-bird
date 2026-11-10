"""Draft practice: you against 7 bots on real Foundations packs, graded by playing your deck out.

    python -m practice                       # http://localhost:8765
    PRACTICE_PASSWORD=... python -m practice # require a password (for sharing over ngrok)

Everything runs on this machine: the draft models, the deck builder and the Rust engine. Nothing
here talks to Nerd Leagues. Drafts live in memory and disappear when the server stops.
"""

import secrets
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from fastapi import FastAPI, Form, HTTPException, Request
from fastapi.responses import FileResponse, HTMLResponse, JSONResponse, RedirectResponse
from pydantic import BaseModel

from bolt.practice import PracticeDraft, check_deck, grade

HERE = Path(__file__).parent
MAX_DRAFTS = 200  # oldest are dropped past this


@dataclass
class Kit:
    """What the app needs; `load_kit` builds the real one, tests pass stand-ins."""

    packs: Any  # (n, 15) opening packs, ids + 1
    bots: Any  # the 7 other seats (human-imitation model)
    coach: Any  # Bolt
    build: Any  # pools (list of card-id lists) -> (decks, labels)
    play: Any  # bolt_engine.play_games
    cards: list[dict]  # name, color, rarity, types per card id
    basics: set[int]
    images: dict[str, str]  # card name -> image URL


def load_kit() -> Kit:
    """The real models: the human-imitation bots, the played-reward bot as coach, judge B's builder."""
    import json

    import bolt_engine as be
    import numpy as np
    import torch

    from bolt.cards import CardInfo
    from bolt.data import ASSETS, ROOT
    from bolt.deckbuild import build_decks
    from bolt.judge import DeckJudge
    from bolt.models import DraftTransformer
    from bolt.played import deck_lists

    torch.set_grad_enabled(False)
    info = CardInfo()
    ck = ROOT / "checkpoints"

    def load(cls, f):
        m = cls(info.n)
        m.load_state_dict(torch.load(ck / f, map_location="cpu"))
        return m.eval()

    judge = load(DeckJudge, "judge_b.pt")

    def build(pools):
        counts = np.zeros((len(pools), info.n))
        for i, p in enumerate(pools):
            for c in p:
                counts[i, c] += 1
        decks, _, labels = build_decks(counts, judge, info, torch.device("cpu"))
        return deck_lists(decks), labels

    return Kit(
        packs=np.load(ASSETS / "FDN_sample_packs.npy"),
        bots=load(DraftTransformer, "human_model.pt"),
        coach=load(DraftTransformer, "bot_rl_played_it135.pt"),
        build=build,
        play=be.play_games,
        cards=info.info,
        basics=set(np.flatnonzero(info.basic).tolist()),
        images=json.loads((HERE / "fdn_images.json").read_text()),
    )


class Pick(BaseModel):
    index: int


class GradeRequest(BaseModel):
    deck: list[int]
    games: int = 30


def create_app(kit: Kit, password: str | None = None) -> FastAPI:
    app = FastAPI(title="Bolt draft practice")
    drafts: dict[str, dict] = {}
    lock = threading.Lock()  # the models aren't shared across threads mid-call
    token = secrets.token_urlsafe(24)

    @app.middleware("http")
    async def gate(request: Request, call_next):
        if password and request.url.path not in ("/login",) and request.cookies.get("bolt") != token:
            if request.url.path.startswith("/api/"):
                return JSONResponse({"detail": "log in first"}, status_code=401)
            return HTMLResponse(LOGIN, status_code=401)
        return await call_next(request)

    @app.post("/login")
    def login(password_: str = Form(alias="password")):
        if not secrets.compare_digest(password_, password or ""):
            return HTMLResponse(LOGIN.replace("<!--msg-->", "<p class=err>Wrong password.</p>"), status_code=401)
        r = RedirectResponse("/", status_code=303)
        r.set_cookie("bolt", token, httponly=True, samesite="lax", max_age=7 * 86400)
        return r

    @app.get("/")
    def index():
        return FileResponse(HERE / "static" / "index.html")

    @app.get("/api/cards")
    def cards():
        return [{**c, "id": i, "image": kit.images.get(c["name"], "")} for i, c in enumerate(kit.cards)]

    def get(did: str) -> dict:
        if did not in drafts:
            raise HTTPException(404, "no such draft (the server may have restarted)")
        return drafts[did]

    def state(did: str) -> dict:
        d = get(did)["draft"]
        out = {"id": did, "done": d.done, "pool": d.pools()[0], "picks": len(d.log)}
        if not d.done:
            out |= {"round": d.round + 1, "pick": d.pick_number + 1, "pack": d.human_pack(), "coach": d.coach()}
        return out

    @app.post("/api/draft")
    def new_draft():
        did = secrets.token_urlsafe(8)
        with lock:
            seed = int(time.time() * 1000) % 2**31
            drafts[did] = {"draft": PracticeDraft(kit.packs, kit.bots, kit.coach, len(kit.cards), seed)}
            while len(drafts) > MAX_DRAFTS:
                drafts.pop(next(iter(drafts)))
            return state(did)

    @app.get("/api/draft/{did}")
    def show(did: str):
        with lock:
            return state(did)

    @app.post("/api/draft/{did}/pick")
    def pick(did: str, body: Pick):
        with lock:
            try:
                get(did)["draft"].pick(body.index)
            except ValueError as e:
                raise HTTPException(400, str(e)) from e
            return state(did)

    def table(did: str) -> dict:
        """Every seat's auto-built deck and Bolt's from your seat, built once per draft."""
        rec = get(did)
        d = rec["draft"]
        if not d.done:
            raise HTTPException(409, "finish the draft first")
        if "decks" not in rec:
            bolt = d.redraft_with_coach()
            decks, labels = kit.build(d.pools() + [bolt.pools()[0]])
            rec["decks"], rec["labels"], rec["bolt_pool"] = decks, labels, bolt.pools()[0]
        return rec

    @app.get("/api/draft/{did}/deck")
    def suggestion(did: str):
        with lock:
            rec = table(did)
            return {"deck": rec["decks"][0], "label": rec["labels"][0], "pool": rec["draft"].pools()[0]}

    @app.post("/api/draft/{did}/grade")
    def grade_draft(did: str, body: GradeRequest):
        with lock:
            rec = table(did)
            d = rec["draft"]
            problems = check_deck(body.deck, d.pools()[0], kit.basics)
            names = [c["name"] for c in kit.cards]
            if problems:
                raise HTTPException(400, [_named(p, names) for p in problems])
            games = max(2, min(body.games, 100))
            decks = rec["decks"]
        g = grade(d, body.deck, decks[1:8], decks[8], games=games, play=kit.play,
                  pilot="smart", rollouts=1, candidates=2, threads=8)  # fmt: skip
        g["bolt_deck"], g["bolt_label"], g["table_labels"] = decks[8], rec["labels"][8], rec["labels"][1:8]
        return g

    return app


def _named(problem: str, names: list[str]) -> str:
    if problem.startswith("card "):
        num, rest = problem[5:].split(":", 1)
        return f"{names[int(num)]}:{rest}"
    return problem


LOGIN = """<!doctype html><meta charset=utf-8><meta name=viewport content="width=device-width,initial-scale=1">
<title>Bolt draft practice</title>
<style>body{font:16px system-ui;background:#12161c;color:#eef1f5;display:grid;place-items:center;min-height:90vh}
form{display:grid;gap:10px;width:min(320px,90vw)}input,button{font:inherit;padding:10px;border-radius:8px;border:1px solid #2e3742}
button{background:#3987e5;color:#fff;border:0}.err{color:#ff8a7a}</style>
<form method=post action=/login><h2>Bolt draft practice</h2><!--msg--><input type=password name=password
placeholder=Password autofocus><button>Enter</button></form>"""
