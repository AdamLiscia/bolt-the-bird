"""The Bolt service: draft against Bolt and play against it over HTTP (contract: docs/api.md).

    BOLT_KEY=... python -m bolt.service        # http://127.0.0.1:8766, one process

Foundations only. Every request needs the header X-Bolt-Key. Games live in memory. Nothing here talks
to Nerd Leagues or any database: NL calls this; this never calls NL.
"""

import os
import secrets
import threading
import time
from dataclasses import dataclass, field
from importlib.metadata import version as pkg_version
from typing import Any

import bolt_engine as be
import numpy as np
import torch
from fastapi import APIRouter, FastAPI, Request
from fastapi.responses import JSONResponse
from pydantic import BaseModel

from bolt.practice import PracticeDraft
from bolt.service_view import actions, state

SETS = ["FDN"]
MAX_TURNS, MAX_MOVES = 200, 20_000
IDLE_SECONDS = 6 * 3600
BASIC_NAMES = {"W": "Plains", "U": "Island", "B": "Swamp", "R": "Mountain", "G": "Forest"}


class ApiError(Exception):
    def __init__(self, status: int, error: str, **extra):
        self.status, self.body = status, {"error": error, **extra}


@dataclass
class Kit:
    """What the service needs; `load_kit` builds the real one."""

    info: Any  # bolt.cards.CardInfo (card numbering = the engine's)
    bolt: Any  # DraftTransformer, the trained drafter
    human: Any  # DraftTransformer, the human-imitation drafter
    judge: Any  # DeckJudge, for the deck builder
    packs: Any  # (n, 15) real FDN opening packs, ids + 1
    net: Any  # PlayNet, the trained player
    models: dict = field(default_factory=dict)
    images: dict = field(default_factory=dict)  # card name -> Scryfall "normal" image URL (hotlinked by NL)


def load_kit() -> Kit:
    from bolt.cards import CardInfo
    from bolt.data import ASSETS, ROOT
    from bolt.judge import DeckJudge
    from bolt.models import DraftTransformer
    from bolt.play_net import PlayNet

    torch.set_grad_enabled(False)
    info = CardInfo()
    ck = ROOT / "checkpoints"

    def load(cls, f):
        m = cls(info.n)
        m.load_state_dict(torch.load(ck / f, map_location="cpu"))
        return m.eval()

    import json

    images = json.loads((ROOT / "practice" / "fdn_images.json").read_text())
    c = torch.load(ck / "play" / "play_v2_ppo3_i850.pt", map_location="cpu", weights_only=False)
    net = PlayNet(**c["config"])
    net.load_state_dict(c["model"])
    return Kit(info=info, bolt=load(DraftTransformer, "bot_ppo.pt"), human=load(DraftTransformer, "human_model.pt"),
               judge=load(DeckJudge, "judge_b.pt"), packs=np.load(ASSETS / "FDN_sample_packs.npy"), net=net.eval(),
               models={"drafter": "bot_ppo.pt", "human_drafter": "human_model.pt",
                       "player": "play_v2_ppo3_i850.pt"}, images=images)  # fmt: skip


# ---- request bodies -------------------------------------------------------------------------------


class Seat(BaseModel):
    seat: int
    pack: list[str]
    pool: list[str] = []
    pack_number: int
    pick_number: int


class PicksBody(BaseModel):
    set: str
    bot: str = "bolt"
    seats: list[Seat]


class PickBody(BaseModel):
    set: str
    pack: list[str]
    pool: list[str] = []
    pack_number: int
    pick_number: int
    bot: str = "bolt"


class BuildBody(BaseModel):
    set: str
    pool: list[str]


class RandomBody(BaseModel):
    set: str
    seed: int | None = None
    seat: int = 0
    drafters: str = "human"


class Card(BaseModel):
    name: str
    qty: int = 1


class GameBody(BaseModel):
    set: str
    deck_a: list[Card]
    deck_b: list[Card]
    human_seat: int = 0
    pilot: str = "network"
    seed: int | None = None
    first: int | None = None


class ActBody(BaseModel):
    seat: int
    action_index: int
    version: int


# ---- the service ----------------------------------------------------------------------------------


def create_app(kit: Kit, key: str, max_games: int = 20) -> FastAPI:
    if not key:
        raise ValueError("a non-empty key is required (BOLT_KEY)")
    app = FastAPI(title="Bolt service", version="v1")
    lock = threading.Lock()  # models and games are used one request at a time
    games: dict[str, dict] = {}
    names = kit.info.names

    @app.exception_handler(ApiError)
    async def api_error(_request: Request, e: ApiError):
        return JSONResponse(e.body, status_code=e.status)

    @app.middleware("http")
    async def auth(request: Request, call_next):
        if not secrets.compare_digest(request.headers.get("x-bolt-key", ""), key):
            return JSONResponse({"error": "unauthorized"}, status_code=401)
        return await call_next(request)

    def check_set(s: str):
        if s.upper() not in SETS:
            raise ApiError(422, f"unsupported set {s!r}; supported: {SETS}")

    def ids_of(card_names: list[str]) -> list[int]:
        unknown = sorted({n for n in card_names if n not in kit.info.index})
        if unknown:
            raise ApiError(422, "unknown card names", unknown_names=unknown)
        return [kit.info.index[n] for n in card_names]

    def model(bot: str):
        if bot not in ("bolt", "human"):
            raise ApiError(422, "bot must be 'bolt' or 'human'")
        return kit.bolt if bot == "bolt" else kit.human

    def rank(m, rows: list[tuple[list[int], list[int], int, int]]) -> list[list[dict]]:
        """rows: (pack ids, pool ids, pack_number 1-3, pick_number 1-14). Probabilities per pack card."""
        for pack, pool, pn, pk in rows:
            if not pack or len(pack) > 15 or len(pool) > 42 or not 1 <= pn <= 3 or not 1 <= pk <= 15:
                raise ApiError(422, "pack must hold 1-15 cards, pool at most 42; pack_number 1-3, pick_number 1-15")
        b = len(rows)
        pack_t = torch.zeros(b, 15, dtype=torch.long)
        pool_t = torch.zeros(b, 42, dtype=torch.long)
        for i, (pack, pool, _, _) in enumerate(rows):
            pack_t[i, : len(pack)] = torch.tensor(pack) + 1
            if pool:
                pool_t[i, : len(pool)] = torch.tensor(pool) + 1
        pn = torch.tensor([min(r[2], 3) - 1 for r in rows], dtype=torch.int8)
        pk = torch.tensor([min(r[3], 14) - 1 for r in rows], dtype=torch.int8)
        with torch.no_grad():
            probs = torch.softmax(m(pool_t, pack_t, pn, pk), 1)
        out = []
        for i, (pack, _, _, _) in enumerate(rows):
            counts: dict[int, int] = {}
            for c in pack:
                counts[c] = counts.get(c, 0) + 1
            ranked = [{"name": names[c], "score": float(probs[i, c]), "count": n} for c, n in counts.items()]
            out.append(sorted(ranked, key=lambda x: -x["score"]))
        return out

    def build(pool: list[int]) -> dict:
        from bolt.deckbuild import build_decks

        counts = np.zeros((1, kit.info.n))
        for c in pool:
            counts[0, c] += 1
        decks, scores, labels = build_decks(counts, kit.judge, kit.info, torch.device("cpu"))
        deck = np.asarray(decks[0]).astype(int)
        basics = {kit.info.basic_id[col]: col for col in BASIC_NAMES}
        main = [{"name": names[c], "qty": int(n)} for c, n in enumerate(deck) if n and c not in basics]
        lands = {basics[c]: int(deck[c]) for c in basics if deck[c]}
        return {"main": main, "lands": lands, "colors": str(labels[0]).split()[0], "cards": int(deck.sum()),
                "judged_win_rate": float(np.asarray(scores).reshape(-1)[0])}  # fmt: skip

    router = APIRouter()

    @router.get("/health")
    def health():
        return {"ok": True, "api": "v1", "sets": SETS, "models": kit.models,
                "engine": f"bolt-engine {pkg_version('bolt-engine')}", "games_live": len(games),
                "games_limit": max_games}  # fmt: skip

    @router.get("/cards")
    def cards(set: str = "FDN"):  # noqa: A002 (the API's parameter name)
        check_set(set)
        return {"cards": [{"name": c["name"], "rarity": c["rarity"], "colors": c["color"], "types": c["types"],
                           "image": kit.images.get(c["name"]) or None} for c in kit.info.info]}  # fmt: skip

    @router.post("/draft/picks")
    def draft_picks(body: PicksBody):
        check_set(body.set)
        m = model(body.bot)
        unknown = sorted({n for s in body.seats for n in s.pack + s.pool if n not in kit.info.index})
        if unknown:
            raise ApiError(422, "unknown card names", unknown_names=unknown)
        if not body.seats:
            return {"picks": [], "model": kit.models["drafter" if body.bot == "bolt" else "human_drafter"]}
        rows = [(ids_of(s.pack), ids_of(s.pool), s.pack_number, s.pick_number) for s in body.seats]
        with lock:
            ranked = rank(m, rows)
        picks = []
        for s, r in zip(body.seats, ranked, strict=True):
            top = r[0]["name"]
            picks.append({"seat": s.seat, "name": next(n for n in s.pack if n == top), "ranked": r})
        return {"picks": picks, "model": kit.models["drafter" if body.bot == "bolt" else "human_drafter"]}

    @router.post("/draft/pick")
    def draft_pick(body: PickBody):
        check_set(body.set)
        m = model(body.bot)
        row = (ids_of(body.pack), ids_of(body.pool), body.pack_number, body.pick_number)
        with lock:
            return {"ranked": rank(m, [row])[0]}

    @router.post("/deck/build")
    def deck_build(body: BuildBody):
        check_set(body.set)
        pool = ids_of(body.pool)
        with lock:
            return build(pool)

    @router.post("/deck/random")
    def deck_random(body: RandomBody):
        check_set(body.set)
        if body.drafters not in ("human", "bolt"):
            raise ApiError(422, "drafters must be 'human' or 'bolt'")
        seed = body.seed if body.seed is not None else secrets.randbelow(2**31)
        with lock:
            others = kit.human if body.drafters == "human" else kit.bolt
            d = PracticeDraft(kit.packs, others, kit.bolt, kit.info.n, seed)
            picks = []
            while not d.done:
                probs = d.coach()
                i = int(np.argmax(probs))
                picks.append({"pack_number": d.round + 1, "pick_number": d.pick_number + 1,
                              "name": names[d.human_pack()[i]]})  # fmt: skip
                d.pick(i)
            deck = build(d.pools()[0])
        return {"seed": seed, "seat": body.seat, "picks": picks, "deck": deck}

    # ---- games --------------------------------------------------------------------------------

    def deck_ids(cards_: list[Card]) -> list[int]:
        unknown = sorted({c.name for c in cards_ if c.name not in kit.info.index})
        if unknown:
            raise ApiError(422, "unknown card names", unknown_names=unknown)
        out = []
        for c in cards_:
            if c.qty < 1:
                raise ApiError(422, f"qty must be at least 1 ({c.name})")
            out += [kit.info.index[c.name]] * c.qty
        if not 40 <= len(out) <= 250:
            raise ApiError(422, f"a deck needs 40 to 250 cards, got {len(out)}")
        return out

    def purge():
        now = time.time()
        for gid in [g for g, r in games.items() if now - r["used"] > IDLE_SECONDS]:
            games.pop(gid, None)

    def bot_move(rec) -> int:
        g, pilot = rec["game"], rec["pilot"]
        seed = (rec["seed"] * 1_000_003 + rec["moves"]) % 2**64
        if pilot == "rules":
            return g.baseline()
        if pilot == "smart":
            return g.smart(1, 2, seed)
        if pilot == "smart_strong":
            return g.smart(4, 6, seed)
        from bolt.play_encoding import collate, encode

        who = g.to_act()
        with torch.no_grad():
            return int(kit.net(collate([encode(g.observation(who), who)]))[0][0].argmax())

    def apply(rec, i: int):
        g = rec["game"]
        who = g.to_act()
        obs = g.observation(who)
        acts = actions(obs, who)
        a = acts[i]
        hidden = who != rec["human"] and a["kind"] in ("Bottom", "Pick")
        label = {"Bottom": "Puts a card on the bottom", "Pick": "Chooses a card"}[a["kind"]] if hidden else a["label"]
        g.step(i)
        rec["moves"] += 1
        rec["version"] += 1
        rec["events"].append({"n": len(rec["events"]), "turn": obs["turn"], "kind": "Action", "seat": who,
                              "move": a["kind"], "label": label})  # fmt: skip

    def advance(rec):
        """Bot moves and forced moves until the human must decide, or the game ends."""
        g = rec["game"]
        while not g.is_over():
            if g.turn > MAX_TURNS or rec["moves"] >= MAX_MOVES:
                rec["timeout"] = True
                return
            who = g.to_act()
            n = len(g.legal_actions())
            if who == rec["human"]:
                if n > 1:
                    return
                apply(rec, 0)
            else:
                apply(rec, 0 if n == 1 else bot_move(rec))

    def view(rec, gid: str) -> dict:
        s = state(rec["game"], rec["human"], version=rec["version"], game_id=gid)
        if rec.get("timeout"):
            s.update(over=True, to_act=None, legal_actions=[], result={"winner": None, "outcome": "timeout"})
        return s

    def get(gid: str) -> dict:
        purge()
        if gid not in games:
            raise ApiError(404, "no such game")
        rec = games[gid]
        rec["used"] = time.time()
        return rec

    @router.post("/game")
    def new_game(body: GameBody):
        check_set(body.set)
        if body.human_seat not in (0, 1):
            raise ApiError(422, "human_seat must be 0 or 1")
        if body.pilot not in ("network", "smart", "smart_strong", "rules"):
            raise ApiError(422, "pilot must be network, smart, smart_strong or rules")
        a, b = deck_ids(body.deck_a), deck_ids(body.deck_b)
        seed = body.seed if body.seed is not None else secrets.randbelow(2**31)
        first = body.first if body.first in (0, 1) else seed % 2
        with lock:
            purge()
            if len(games) >= max_games:
                raise ApiError(429, "too many games", limit=max_games)
            gid = "g_" + secrets.token_hex(6)
            rec = {"game": be.Game(a, b, seed=seed, first=first), "human": body.human_seat, "pilot": body.pilot,
                   "seed": seed, "version": 0, "moves": 0, "events": [], "used": time.time()}  # fmt: skip
            games[gid] = rec
            advance(rec)
            return {"game_id": gid, "state": view(rec, gid)}

    @router.get("/game/{gid}")
    def show(gid: str, seat: int):
        with lock:
            rec = get(gid)
            if seat != rec["human"]:
                raise ApiError(422, "only the human seat's view is served")
            return view(rec, gid)

    @router.post("/game/{gid}/act")
    def act(gid: str, body: ActBody):
        with lock:
            rec = get(gid)
            cur = view(rec, gid)
            if body.seat != rec["human"] or cur["over"] or body.version != rec["version"]:
                raise ApiError(409, "stale version, game over, or not that seat's turn", state=cur)
            if not 0 <= body.action_index < len(cur["legal_actions"]):
                raise ApiError(422, f"action_index must be 0-{len(cur['legal_actions']) - 1}")
            apply(rec, body.action_index)
            advance(rec)
            return view(rec, gid)

    @router.get("/game/{gid}/log")
    def log(gid: str, since: int = 0):
        with lock:
            rec = get(gid)
            ev = rec["events"][max(0, since) :]
            return {"events": ev, "next": len(rec["events"])}

    @router.delete("/game/{gid}")
    def end(gid: str):
        with lock:
            get(gid)
            games.pop(gid, None)
            return {"ok": True}

    app.include_router(router, prefix="/v1")
    app.include_router(router)
    return app


def main():
    import uvicorn

    key = os.environ.get("BOLT_KEY", "")
    if not key:
        raise SystemExit("set BOLT_KEY (the shared secret Nerd Leagues sends as X-Bolt-Key)")
    torch.set_num_threads(int(os.environ.get("BOLT_THREADS", "2")))
    uvicorn.run(create_app(load_kit(), key=key), host="127.0.0.1", port=int(os.environ.get("BOLT_PORT", "8766")))


if __name__ == "__main__":
    main()
