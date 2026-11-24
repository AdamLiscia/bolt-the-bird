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
    tokens: list = field(default_factory=list)  # assets/FDN_token_images.json: the engine's tokens


def load_kit() -> Kit:
    from bolt.cards import CardInfo
    from bolt.data import ASSETS, ROOT
    from bolt.judge import DeckJudge
    from bolt.models import DraftTransformer
    from bolt.play_net import PlayNet

    info = CardInfo()
    ck = ROOT / "checkpoints"

    def load(cls, f):
        m = cls(info.n)
        m.load_state_dict(torch.load(ck / f, map_location="cpu"))
        return m.eval()

    import json

    images = json.loads((ROOT / "practice" / "fdn_images.json").read_text())
    tokens = json.loads((ASSETS / "FDN_token_images.json").read_text())
    c = torch.load(ck / "play" / "play_v2_ppo3_i850.pt", map_location="cpu", weights_only=False)
    net = PlayNet(**c["config"])
    net.load_state_dict(c["model"])
    return Kit(info=info, bolt=load(DraftTransformer, "bot_ppo.pt"), human=load(DraftTransformer, "human_model.pt"),
               judge=load(DeckJudge, "judge_b.pt"), packs=np.load(ASSETS / "FDN_sample_packs.npy"), net=net.eval(),
               models={"drafter": "bot_ppo.pt", "human_drafter": "human_model.pt",
                       "player": "play_v2_ppo3_i850.pt"}, images=images, tokens=tokens)  # fmt: skip


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
    seat: int | str = 0  # 0-7, or "random"
    drafters: str = "human"  # the other 7 seats
    drafter: str = "bolt"  # seat 0: "bolt" (strongest; leans green-white) or "human" (varied, like people)
    colors: list[str] | None = None  # a soft preference: the deck among the 8 that best matches


class DraftBody(BaseModel):
    set: str
    seed: int | None = None
    human_seat: int = 0
    bots: str = "mix"  # "human", "bolt", or "mix" (one Bolt, six human-style)


class DraftPickBody(BaseModel):
    version: int
    card_id: int | None = None
    id: int | None = None  # the card's position in the pack, instead of card_id


class TableBody(BaseModel):
    set: str
    seed: int | None = None
    drafters: str = "mix"  # "human", "bolt", or "mix" (seat 0 Bolt, seven human-style)


class Mix:
    """Bot seats for a mixed table: the first row (seat 1) is Bolt, the rest are the human model."""

    def __init__(self, bolt, human):
        self.bolt, self.human = bolt, human

    def __call__(self, pool, pack, pn, pk):
        out = self.human(pool, pack, pn, pk)
        out[:1] = self.bolt(pool[:1], pack[:1], pn[:1], pk[:1])
        return out


RARITY = {"mythic": 0, "rare": 1, "uncommon": 2, "common": 3}
MAX_DRAFTS = 50


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
    frames: bool = False
    human_seats: list[int] | None = None  # [0, 1]: two people, no bot seat


class ConcedeBody(BaseModel):
    seat: int


class ActBody(BaseModel):
    seat: int
    action_index: int
    version: int
    frames: bool = False


MAX_FRAMES = 40
SEAT_FRAMES = 300  # boards kept per human seat for GET /frames


# ---- the service ----------------------------------------------------------------------------------


def create_app(kit: Kit, key: str, max_games: int = 20) -> FastAPI:
    if not key:
        raise ValueError("a non-empty key is required (BOLT_KEY)")
    app = FastAPI(title="Bolt service", version="v1")
    lock = threading.Lock()  # models and games are used one request at a time
    games: dict[str, dict] = {}
    names = kit.info.names
    names_all = be.card_names()  # the engine's objects, tokens included

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

    def build_many(pools: list[list[int]]) -> list[dict]:
        from bolt.deckbuild import build_decks

        counts = np.zeros((len(pools), kit.info.n))
        for i, pool in enumerate(pools):
            for c in pool:
                counts[i, c] += 1
        with torch.no_grad():
            decks, scores, labels = build_decks(counts, kit.judge, kit.info, torch.device("cpu"))
        basics = {kit.info.basic_id[col]: col for col in BASIC_NAMES}
        out = []
        for i in range(len(pools)):
            deck = np.asarray(decks[i]).astype(int)
            main = [{"name": names[c], "qty": int(n)} for c, n in enumerate(deck) if n and c not in basics]
            lands = {basics[c]: int(deck[c]) for c in basics if deck[c]}
            out.append({"main": main, "lands": lands, "colors": str(labels[i]).split()[0], "cards": int(deck.sum()),
                        "judged_win_rate": float(np.asarray(scores).reshape(-1)[i])})  # fmt: skip
        return out

    def build(pool: list[int]) -> dict:
        return build_many([pool])[0]

    router = APIRouter()

    @router.get("/health")
    def health():
        probe = be.Game([0] * 40, [0] * 40, seed=0).observation(0)  # what the installed engine reports
        features = ["frames", "tokens", "card_id", "human_seats", "wait", "concede"]
        if "decision" in probe:
            features.append("decision_details")
        return {"ok": True, "api": "v1", "sets": SETS, "models": kit.models, "features": features,
                "engine": f"bolt-engine {pkg_version('bolt-engine')}", "games_live": len(games),
                "games_limit": max_games}  # fmt: skip

    @router.get("/cards")
    def cards(set: str = "FDN"):  # noqa: A002 (the API's parameter name)
        check_set(set)
        cards_ = [{"card_id": i, "name": c["name"], "rarity": c["rarity"], "colors": c["color"], "types": c["types"],
                   "image": kit.images.get(c["name"]) or None, "token": False} for i, c in enumerate(kit.info.info)]  # fmt: skip
        cards_ += [{"card_id": t["card_id"], "name": t["name"], "rarity": "token", "colors": t["colors"],
                    "types": t["types"], "image": t["image"], "token": True, "power": t["power"],
                    "toughness": t["toughness"]} for t in kit.tokens]  # fmt: skip
        return {"cards": cards_}

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
        if body.drafters not in ("human", "bolt") or body.drafter not in ("human", "bolt"):
            raise ApiError(422, "drafter and drafters must be 'human' or 'bolt'")
        if body.seat != "random" and body.seat not in range(8):
            raise ApiError(422, "seat must be 0-7 or 'random'")
        wanted = {c.upper() for c in body.colors or []}
        if not wanted <= set("WUBRG"):
            raise ApiError(422, "colors are W, U, B, R, G")
        seed = body.seed if body.seed is not None else secrets.randbelow(2**31)
        rng = np.random.default_rng(seed)
        with lock:
            others = kit.human if body.drafters == "human" else kit.bolt
            mine = kit.bolt if body.drafter == "bolt" else kit.human
            d = PracticeDraft(kit.packs, others, mine, kit.info.n, seed)
            while not d.done:
                probs = np.asarray(d.coach(), dtype=float)
                # Bolt takes its best card; the human model samples, the way people's picks vary
                i = (
                    int(np.argmax(probs))
                    if body.drafter == "bolt"
                    else int(rng.choice(len(probs), p=probs / probs.sum()))
                )
                d.pick(i)
            pools = d.pools()
            seats = list(range(8)) if body.seat == "random" or wanted else [int(body.seat)]
            decks = build_many([pools[s] for s in seats]) if len(seats) > 1 else [build(pools[seats[0]])]
            if wanted:
                score = [len(wanted & set(x["colors"])) for x in decks]
                best = [k for k, v in enumerate(score) if v == max(score)]
                k = 0 if 0 in best and body.seat != "random" else int(rng.choice(best))
            elif body.seat == "random":
                k = int(rng.integers(8))
            else:
                k = 0
            seat, deck = seats[k], decks[k]
        picks = [{"pack_number": pos // 14 + 1, "pick_number": pos % 14 + 1, "name": names[c]}
                 for pos, c in enumerate(pools[seat])]  # fmt: skip
        drafter = body.drafter if seat == 0 else body.drafters
        return {"seed": seed, "seat": seat, "drafter": drafter, "picks": picks, "deck": deck,
                "colors_matched": bool(wanted & set(deck["colors"])) if wanted else None}  # fmt: skip

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

    public = {"Battlefield", "Graveyard", "Stack", "Exile", "Gone"}
    hidden_moves = {"Bottom": "Puts a card on the bottom", "Pick": "Chooses a card"}

    def engine_events(rec, turn: int):
        """The engine's events since the last call, named. Hidden details are marked per seat
        (`_hide`: the seats that may not see the card) and masked when a seat reads the log."""
        g = rec["game"]
        for e in g.events(rec["engine_n"]):
            rec["engine_n"] += 1
            kind = e["kind"]
            if kind == "TurnBegan":
                rec["turn"] = e["turn"]
            out = {"n": len(rec["events"]), "turn": rec.get("turn", turn), "kind": kind}
            for k in ("player", "amount", "from", "to", "owner", "target_player"):
                if k in e:
                    out[k] = e[k]
            hide = []
            if kind == "Drew":
                hide = [s for s in (0, 1) if s != e["player"]]
            elif kind == "Moved" and not ({e["from"], e["to"]} & public):
                hide = [s for s in (0, 1) if s != e["owner"]]
            if "id" in e:
                out["id"], out["name"], out["_hide"] = e["id"], names_all[e["card"]], hide
            if "target" in e:
                out["target"], out["target_name"] = e["target"], names_all[e["target_card"]]
            rec["events"].append(out)

    def seat_event(e: dict, seat: int) -> dict:
        """One log entry as `seat` may see it."""
        out = {k: v for k, v in e.items() if not k.startswith("_")}
        if seat in e.get("_hide", []):
            out.pop("id", None)
            out.pop("name", None)
        if e["kind"] == "Action" and e["seat"] != seat and e["move"] in hidden_moves:
            out["label"] = hidden_moves[e["move"]]
            out["objects"], out["cards"] = [], []  # which card stays hidden
        return out

    def snapshot(rec, gid: str, frames: list | None, seat: int):
        """Add `seat`'s current view to `frames` if anything visible to it changed."""
        if frames is None:
            return
        f = state(rec["game"], seat, version=rec["version"], game_id=gid)
        f["legal_actions"] = []
        f["event_n"] = len(rec["events"])
        if frames and (frames[-1]["view"], frames[-1]["turn"]) == (f["view"], f["turn"]):
            return  # nothing visible changed (a step change alone doesn't make a frame)
        frames.append(f)

    def remember(rec, gid: str):
        """Every human seat's frames since its last look, for GET /frames (a waiting player's boards)."""
        for seat in rec["humans"]:
            snapshot(rec, gid, rec["seat_frames"][seat], seat)
            del rec["seat_frames"][seat][:-SEAT_FRAMES]

    def apply(rec, i: int, gid: str = ""):
        g = rec["game"]
        who = g.to_act()
        obs = g.observation(who)
        a = actions(obs, who)[i]
        g.step(i)
        rec["moves"] += 1
        rec["version"] += 1
        entry = {"n": len(rec["events"]), "turn": obs["turn"], "kind": "Action", "seat": who, "move": a["kind"],
                 "label": a["label"], "objects": a["objects"], "cards": a["cards"]}  # fmt: skip
        if "player" in a:
            entry["player"] = a["player"]
        rec["events"].append(entry)
        engine_events(rec, obs["turn"])
        remember(rec, gid)

    def trim(frames: list) -> list:
        half = MAX_FRAMES // 2
        return frames if len(frames) <= MAX_FRAMES else frames[:half] + frames[-half:]

    def advance(rec, gid: str = "", frames: list | None = None, seat: int = 0):
        """Bot moves and forced moves until a human seat must decide, or the game ends."""
        g = rec["game"]
        while not g.is_over() and rec.get("conceded") is None:
            if g.turn > MAX_TURNS or rec["moves"] >= MAX_MOVES:
                rec["timeout"] = True
                return
            who = g.to_act()
            n = len(g.legal_actions())
            if who in rec["humans"]:
                if n > 1:
                    return
                apply(rec, 0, gid)
            else:
                apply(rec, 0 if n == 1 else bot_move(rec), gid)
            snapshot(rec, gid, frames, seat)

    def finished(rec) -> dict | None:
        if rec.get("conceded") is not None:
            return {"winner": 1 - rec["conceded"], "outcome": "concede", "conceded": rec["conceded"]}
        if rec.get("timeout"):
            return {"winner": None, "outcome": "timeout"}
        return None

    def view(rec, gid: str, seat: int | None = None) -> dict:
        seat = rec["human"] if seat is None else seat
        s = state(rec["game"], seat, version=rec["version"], game_id=gid)
        if seat in rec["humans"] and s.get("to_act") in rec["humans"] and s["to_act"] != seat:
            s["legal_actions"] = []  # the other player's decision
        end = finished(rec)
        if end:
            s.update(over=True, to_act=None, legal_actions=[], result=end)
        return s

    def get(gid: str) -> dict:
        purge()
        if gid not in games:
            raise ApiError(404, "no such game")
        rec = games[gid]
        rec["used"] = time.time()
        return rec

    def seat_of(rec, seat: int | None) -> int:
        seat = rec["human"] if seat is None else seat
        if seat not in rec["humans"]:
            raise ApiError(422, f"seat {seat} isn't a human seat in this game ({sorted(rec['humans'])})")
        return seat

    @router.post("/game")
    def new_game(body: GameBody):
        check_set(body.set)
        humans = body.human_seats if body.human_seats is not None else [body.human_seat]
        if not humans or not set(humans) <= {0, 1} or len(set(humans)) != len(humans):
            raise ApiError(422, "human_seats must be [0], [1] or [0, 1] (or human_seat 0 or 1)")
        if len(humans) == 1 and body.pilot not in ("network", "smart", "smart_strong", "rules"):
            raise ApiError(422, "pilot must be network, smart, smart_strong or rules")
        a, b = deck_ids(body.deck_a), deck_ids(body.deck_b)
        seed = body.seed if body.seed is not None else secrets.randbelow(2**31)
        first = body.first if body.first in (0, 1) else seed % 2
        with lock:
            purge()
            if len(games) >= max_games:
                raise ApiError(429, "too many games", limit=max_games)
            gid = "g_" + secrets.token_hex(6)
            game = be.Game(a, b, seed=seed, first=first)
            game.auto_pass = False  # forced passes go through `advance`, so frames can catch every moment
            rec = {"game": game, "human": humans[0], "humans": set(humans),
                   "pilot": body.pilot if len(humans) == 1 else None, "seed": seed, "version": 0, "moves": 0,
                   "events": [], "engine_n": 0, "used": time.time(), "seat_frames": {s: [] for s in humans}}  # fmt: skip
            games[gid] = rec
            frames = [] if body.frames else None
            advance(rec, gid, frames, humans[0])
            out = {"game_id": gid, "human_seats": sorted(humans), "state": view(rec, gid)}
            if frames is not None:
                out["frames"] = trim(frames)
            return out

    @router.get("/game/{gid}")
    def show(gid: str, seat: int | None = None):
        with lock:
            rec = get(gid)
            return view(rec, gid, seat_of(rec, seat))

    @router.post("/game/{gid}/act")
    def act(gid: str, body: ActBody):
        with lock:
            rec = get(gid)
            if body.seat not in rec["humans"]:  # e.g. the bot's seat: not yours to play (409, as for a wrong turn)
                raise ApiError(409, "not a human seat in this game", state=view(rec, gid))
            seat = body.seat
            cur = view(rec, gid, seat)
            if cur["over"] or cur["to_act"] != seat or body.version != rec["version"]:
                raise ApiError(409, "stale version, game over, or not that seat's turn", state=cur)
            if not 0 <= body.action_index < len(cur["legal_actions"]):
                raise ApiError(422, f"action_index must be 0-{len(cur['legal_actions']) - 1}")
            frames = [] if body.frames else None
            apply(rec, body.action_index, gid)
            snapshot(rec, gid, frames, seat)  # your own move, resolved as far as it goes (lands tapped, spell cast)
            advance(rec, gid, frames, seat)
            out = view(rec, gid, seat)
            if frames is not None:
                out["frames"] = trim(frames)
            return out

    @router.get("/game/{gid}/frames")
    def seat_frames(gid: str, seat: int | None = None, since: int = 0):
        """`seat`'s boards after `since` (a version), for a player who was waiting."""
        with lock:
            rec = get(gid)
            seat = seat_of(rec, seat)
            fr = [f for f in rec["seat_frames"][seat] if f["version"] > since]
            return {"frames": trim(fr), "version": rec["version"]}

    @router.get("/game/{gid}/version")
    def version(gid: str):
        with lock:
            rec = get(gid)
            v = view(rec, gid)
            return {"version": rec["version"], "to_act": v["to_act"], "over": v["over"]}

    @router.get("/game/{gid}/wait")
    async def wait(gid: str, version: int, seat: int | None = None, timeout: float = 25.0):
        """Long-poll: returns as soon as the game's version differs from `version`, or after `timeout`."""
        import asyncio

        end = time.time() + max(0.0, min(timeout, 30.0))
        while True:
            rec = games.get(gid)
            if rec is None:
                raise ApiError(404, "no such game")
            over = rec["game"].is_over() or finished(rec) is not None
            if rec["version"] != version or over or time.time() >= end:
                to_act = None if over else rec["game"].to_act()
                return {"version": rec["version"], "to_act": to_act, "over": over, "changed": rec["version"] != version}
            await asyncio.sleep(0.2)

    @router.post("/game/{gid}/concede")
    def concede(gid: str, body: ConcedeBody):
        with lock:
            rec = get(gid)
            seat = seat_of(rec, body.seat)
            if finished(rec) is None and not rec["game"].is_over():
                rec["conceded"] = seat
                rec["version"] += 1
                rec["events"].append({"n": len(rec["events"]), "turn": rec["game"].turn, "kind": "Conceded",
                                      "player": seat})  # fmt: skip
            return view(rec, gid, seat)

    @router.get("/game/{gid}/log")
    def log(gid: str, since: int = 0, seat: int | None = None):
        with lock:
            rec = get(gid)
            seat = seat_of(rec, seat)
            ev = [seat_event(e, seat) for e in rec["events"][max(0, since) :]]
            return {"events": ev, "next": len(rec["events"])}

    @router.delete("/game/{gid}")
    def end(gid: str):
        with lock:
            get(gid)
            games.pop(gid, None)
            return {"ok": True}

    # ---- drafting against bots, and whole bot tables ------------------------------------------

    drafts: dict[str, dict] = {}

    def bot_models(kind: str):
        if kind not in ("human", "bolt", "mix"):
            raise ApiError(422, "bots / drafters must be 'human', 'bolt' or 'mix'")
        return {"human": kit.human, "bolt": kit.bolt, "mix": Mix(kit.bolt, kit.human)}[kind]

    def card(c: int) -> dict:
        return {"name": names[c], "card_id": c}

    def highlights(deck: dict, order: list[int]) -> list[str]:
        """The deck's three standouts: rarest first, then the earliest picked."""
        first = {}
        for pos, c in enumerate(order):
            first.setdefault(names[c], pos)
        main = [x["name"] for x in deck["main"]]
        rank = sorted(main, key=lambda n: (RARITY.get(kit.info.info[kit.info.index[n]]["rarity"], 4), first.get(n, 99)))
        return rank[:3]

    def draft_state(did: str) -> dict:
        rec = drafts[did]
        d = rec["draft"]
        out = {"draft_id": did, "version": len(d.log), "done": d.done, "picks_left": 42 - len(d.log),
               "pool": [card(c) for c in d.pools()[0]], "pack_number": None, "pick_number": None, "pack": []}  # fmt: skip
        if not d.done:
            out.update(pack_number=d.round + 1, pick_number=d.pick_number + 1,
                       pack=[{"id": i, **card(c)} for i, c in enumerate(d.human_pack())])  # fmt: skip
        return out

    def get_draft(did: str) -> dict:
        now = time.time()
        for k in [k for k, r in drafts.items() if now - r["used"] > IDLE_SECONDS]:
            drafts.pop(k, None)
        if did not in drafts:
            raise ApiError(404, "no such draft")
        drafts[did]["used"] = now
        return drafts[did]

    @router.post("/draft")
    def new_draft(body: DraftBody):
        check_set(body.set)
        if body.human_seat != 0:
            raise ApiError(422, "human_seat must be 0 (the other seats are bots)")
        bots = bot_models(body.bots)
        seed = body.seed if body.seed is not None else secrets.randbelow(2**31)
        with lock:
            if len(drafts) >= MAX_DRAFTS:
                drafts.pop(min(drafts, key=lambda k: drafts[k]["used"]))
            did = "d_" + secrets.token_hex(6)
            drafts[did] = {"draft": PracticeDraft(kit.packs, bots, kit.bolt, kit.info.n, seed), "bots": body.bots,
                           "seed": seed, "used": time.time()}  # fmt: skip
            return {"draft_id": did, "seed": seed, "state": draft_state(did)}

    @router.get("/draft/{did}")
    def show_draft(did: str):
        with lock:
            get_draft(did)
            return draft_state(did)

    @router.post("/draft/{did}/pick")
    def draft_pick_card(did: str, body: DraftPickBody):
        with lock:
            d = get_draft(did)["draft"]
            if d.done or body.version != len(d.log):
                raise ApiError(409, "stale version, or the draft is over", state=draft_state(did))
            pack = d.human_pack()
            if body.id is not None:
                i = body.id
            elif body.card_id is not None and body.card_id in pack:
                i = pack.index(body.card_id)
            else:
                raise ApiError(422, "pick a card in your pack: card_id, or id (its position)")
            if not 0 <= i < len(pack):
                raise ApiError(422, f"id must be 0-{len(pack) - 1}")
            d.pick(i)
            return draft_state(did)

    @router.get("/draft/{did}/table")
    def draft_table(did: str):
        """After the draft: what the 7 bots took, and their built decks (to choose an opponent)."""
        with lock:
            rec = get_draft(did)
            d = rec["draft"]
            if not d.done:
                raise ApiError(409, "the table is shown once the draft is done", state=draft_state(did))
            if "table" not in rec:
                pools = d.pools()
                decks = build_many(pools[1:])
                kind = rec["bots"]
                rec["table"] = [{"seat": s, "drafter": ("bolt" if kind == "mix" and s == 1 else
                                 "human" if kind == "mix" else kind), "picks": [card(c) for c in pools[s]],
                                 "deck": decks[s - 1], "colors": decks[s - 1]["colors"],
                                 "highlights": highlights(decks[s - 1], pools[s])} for s in range(1, 8)]  # fmt: skip
            return {"seats": rec["table"]}

    @router.post("/draft/table")
    def whole_table(body: TableBody):
        """A whole table drafted by bots: all 8 seats' decks, to choose from."""
        check_set(body.set)
        if body.drafters not in ("human", "bolt", "mix"):
            raise ApiError(422, "drafters must be 'human', 'bolt' or 'mix'")
        seed = body.seed if body.seed is not None else secrets.randbelow(2**31)
        rng = np.random.default_rng(seed)
        first = "human" if body.drafters == "human" else "bolt"
        with lock:
            others = kit.bolt if body.drafters == "bolt" else kit.human
            d = PracticeDraft(kit.packs, others, kit.bolt if first == "bolt" else kit.human, kit.info.n, seed)
            while not d.done:
                probs = np.asarray(d.coach(), dtype=float)
                i = int(np.argmax(probs)) if first == "bolt" else int(rng.choice(len(probs), p=probs / probs.sum()))
                d.pick(i)
            pools = d.pools()
            decks = build_many(pools)
        seats = []
        for s in range(8):
            drafter = first if s == 0 else ("bolt" if body.drafters == "bolt" else "human")
            seats.append({"seat": s, "drafter": drafter, "colors": decks[s]["colors"], "cards": decks[s]["cards"],
                          "deck": decks[s], "picks": [card(c) for c in pools[s]],
                          "highlights": highlights(decks[s], pools[s])})  # fmt: skip
        return {"seed": seed, "seats": seats}

    app.include_router(router, prefix="/v1")
    app.include_router(router)
    return app


def main():
    import uvicorn

    key = os.environ.get("BOLT_KEY", "")
    if not key:
        raise SystemExit("set BOLT_KEY (the shared secret Nerd Leagues sends as X-Bolt-Key)")
    torch.set_num_threads(int(os.environ.get("BOLT_THREADS", "2")))
    torch.set_grad_enabled(False)  # the service only runs models forward (process-wide, so not in load_kit)
    uvicorn.run(create_app(load_kit(), key=key), host="127.0.0.1", port=int(os.environ.get("BOLT_PORT", "8766")))


if __name__ == "__main__":
    main()
