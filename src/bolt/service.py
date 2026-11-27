"""The Bolt service: draft against Bolt and play against it over HTTP (contract: docs/api.md).

    BOLT_KEY=... python -m bolt.service        # http://127.0.0.1:8766, one process

Foundations only. Every request needs the header X-Bolt-Key. Games live in memory. Nothing here talks
to Nerd Leagues or any database: NL calls this; this never calls NL.
"""

import os
import re
import secrets
import threading
import time
from contextlib import contextmanager
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
from bolt.service_view import BACKS, SET, actions, back_image, state

SETS = [SET]  # one set per process: BOLT_SET (default FDN); the Tarkir service runs on 8767 with TDM
MAX_TURNS, MAX_MOVES = 200, 20_000
IDLE_SECONDS = 6 * 3600
# The smart pilot's time per decision in Commander, in ms: some 1v1 Commander positions took minutes to
# search in full. Past it, the pilot plays the best move scored so far. FDN and TDM search in full.
CASUAL_SIZE_SLACK = 2  # strict=False: decks of 98-102 cards are allowed, with a warning
CMD_SMART_MS = 2500
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


def engine_game(a: list[int], b: list[int], seed: int, first: int = 0):
    """A game of this process's set (FDN keeps the engine's original call)."""
    if SET == "FDN":
        return be.Game(a, b, seed=seed, first=first)
    return be.Game(a, b, seed=seed, first=first, set=SET)


def load_cmd_kit() -> Kit:
    """The Commander pool (every scripted card across sets): no drafting, no bot, no judge."""
    import json

    from bolt.cards import CardInfo
    from bolt.data import ASSETS

    images = json.loads((ASSETS / "CMD_card_images.json").read_text())
    tokens = [{"card_id": i, "name": n, "power": pw, "toughness": t, "colors": c, "types": ty, "image": None}
              for i, n, pw, t, c, ty in be.token_info("CMD")]  # fmt: skip
    return Kit(info=CardInfo("CMD"), bolt=None, human=None, judge=None, packs=None, net=None,
               models={"drafter": None, "human_drafter": None, "player": None}, images=images, tokens=tokens)  # fmt: skip


def load_set_kit(code: str) -> Kit:
    """A set without a trained player or deck judge (Tarkir): one drafter, trained on 17lands human picks,
    plays both drafting roles; decks are built by a simple rule (`simple_build`)."""
    import json

    from bolt.cards import CardInfo
    from bolt.data import ASSETS, ROOT
    from bolt.models import DraftTransformer

    info = CardInfo(code)
    drafter = DraftTransformer(info.n)
    name = f"{code}_transformer.pt"
    drafter.load_state_dict(torch.load(ROOT / "checkpoints" / name, map_location="cpu"))
    drafter.eval()
    images = json.loads((ASSETS / f"{code}_card_images.json").read_text())
    tokens = [{"card_id": i, "name": n, "power": pw, "toughness": t, "colors": c, "types": ty, "image": None}
              for i, n, pw, t, c, ty in be.token_info(code)]  # fmt: skip
    return Kit(info=info, bolt=drafter, human=drafter, judge=None, packs=np.load(ASSETS / f"{code}_sample_packs.npy"),
               net=None, models={"drafter": name, "human_drafter": name, "player": None}, images=images,
               tokens=tokens)  # fmt: skip


def simple_build(info, pool: list[int]) -> dict:
    """A 40-card deck from a pool without a deck judge: the two colors with the most playable cards,
    up to 23 spells (rares first, then by mana value), on-color nonbasic lands, basics split by pips."""
    from itertools import combinations

    cards = [info.info[c] for c in pool]
    nonbasic = [i for i, c in enumerate(cards) if c["rarity"] != "basic"]

    def fits(c, pair):
        return set(c["color"]) <= set(pair)

    def spells_of(pair):
        return [i for i in nonbasic if fits(cards[i], pair) and "Land" not in " ".join(cards[i]["types"])]

    pairs = ["".join(p) for p in combinations("WUBRG", 2)]
    best = max(pairs, key=lambda pr: (len(spells_of(pr)), sum(len(cards[i]["color"]) for i in spells_of(pr))))
    order = {"mythic": 0, "rare": 1, "uncommon": 2, "common": 3}
    spells = sorted(spells_of(best), key=lambda i: (order.get(cards[i]["rarity"], 4), len(cards[i]["color"]) == 0))
    spells = spells[:23]
    lands = [i for i in nonbasic if "Land" in " ".join(cards[i]["types"]) and fits(cards[i], best)][:3]
    n_basics = 40 - len(spells) - len(lands)
    pips = {col: sum(cards[i]["color"].count(col) for i in spells) for col in best}
    total = sum(pips.values()) or 1
    split = {col: round(n_basics * pips[col] / total) for col in best}
    split[best[0]] += n_basics - sum(split.values())
    main: dict[str, int] = {}
    for i in spells + lands:
        main[cards[i]["name"]] = main.get(cards[i]["name"], 0) + 1
    return {"main": [{"name": n, "qty": q} for n, q in main.items()],
            "lands": {col: n for col, n in split.items() if n > 0},  # keyed by colour, like FDN
            "colors": best, "cards": 40, "judged_win_rate": None}  # fmt: skip


def load_kit() -> Kit:
    if SET == "CMD":
        return load_cmd_kit()
    if SET != "FDN":
        return load_set_kit(SET)
    from bolt.cards import CardInfo
    from bolt.data import ASSETS, ROOT
    from bolt.judge import DeckJudge
    from bolt.models import DraftTransformer

    info = CardInfo()
    ck = ROOT / "checkpoints"

    def load(cls, f):
        m = cls(info.n)
        m.load_state_dict(torch.load(ck / f, map_location="cpu"))
        return m.eval()

    import json

    images = json.loads((ROOT / "practice" / "fdn_images.json").read_text())
    tokens = json.loads((ASSETS / "FDN_token_images.json").read_text())
    c = torch.load(ck / "play" / "play_v3_ppo4_i170.pt", map_location="cpu", weights_only=False)
    from bolt.play_net import load_checkpoint

    net = load_checkpoint(c)
    return Kit(info=info, bolt=load(DraftTransformer, "bot_ppo.pt"), human=load(DraftTransformer, "human_model.pt"),
               judge=load(DeckJudge, "judge_b.pt"), packs=np.load(ASSETS / "FDN_sample_packs.npy"), net=net.eval(),
               models={"drafter": "bot_ppo.pt", "human_drafter": "human_model.pt",
                       "player": "play_v3_ppo4_i170.pt"}, images=images, tokens=tokens)  # fmt: skip


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
    manual_mana_seats: list[int] = []  # human seats that tap their own mana (TapForMana moves)
    format: str | None = None  # "commander" (docs/api.md, Commander games)
    commander_a: str | list[str] | None = None  # a name, or two for partners
    commander_b: str | list[str] | None = None
    starting_life: int = 40
    auto_command_zone: list[bool] = [False, False]  # per seat: always move the commander there
    strict: bool = True  # False (casual tables): a deck within 2 of 100 cards is a warning, not a problem


class ValidateBody(BaseModel):
    format: str = "commander"
    commander: str | list[str]
    deck: list[Card]
    strict: bool = True  # False (casual tables): a deck within 2 of 100 cards is a warning, not a problem


class ManaBody(BaseModel):
    seat: int
    manual: bool


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


def smart(g, rollouts: int, candidates: int, seed: int) -> int:
    """The smart pilot's move; Commander's search is capped at CMD_SMART_MS."""
    if SET == "CMD":
        return g.smart(rollouts, candidates, seed, ms=CMD_SMART_MS)
    return g.smart(rollouts, candidates, seed)


def bot_move(rec, kit: Kit, model_lock: threading.Lock) -> int:
    """The bot seat's move (index into the legal actions). The engine releases Python's lock while the
    smart pilot searches, so other games and requests carry on meanwhile."""
    g, pilot = rec["game"], rec["pilot"]
    seed = (rec["seed"] * 1_000_003 + rec["moves"]) % 2**64
    if pilot == "rules":
        return g.baseline()
    if pilot == "smart":
        return smart(g, 1, 2, seed)
    if pilot == "smart_strong":
        return smart(g, 4, 6, seed)
    if g.observation(g.to_act())["pending"][0] == "ChooseTarget":
        # The network can't see the spell it's targeting for (the engine's view shows it only after
        # targets are chosen), so it aims pumps at the opponent's creatures 65% of the time. Until it's
        # retrained with that input, the smart player chooses its targets (it never made that mistake).
        return smart(g, 1, 2, seed)
    from bolt.play_encoding import collate, encode

    who = g.to_act()
    with model_lock, torch.no_grad():
        return int(kit.net(collate([encode(g.observation(who), who)]))[0][0].argmax())


def create_app(kit: Kit, key: str, max_games: int = 20) -> FastAPI:
    if not key:
        raise ValueError("a non-empty key is required (BOLT_KEY)")
    app = FastAPI(title="Bolt service", version="v1")
    # Locks: `lock` for the models and drafts (one request at a time); `games_lock` only for the games
    # dict (create, delete, purge, look up); each game's own `rec["lock"]` while it's read or played, so a
    # bot thinking in one game never holds up another game, /health or /deck/validate.
    lock = threading.Lock()
    games_lock = threading.Lock()
    games: dict[str, dict] = {}
    names = kit.info.names
    names_all = be.card_names() if SET == "FDN" else be.card_names(SET)  # the engine's objects, tokens included

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

    def packs():
        if kit.packs is None:
            raise ApiError(422, f"no drafting for {SET}")
        return kit.packs

    def model(bot: str):
        if kit.bolt is None:
            raise ApiError(422, f"no drafting for {SET}")
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
        if kit.judge is None:
            return [simple_build(kit.info, pool) for pool in pools]
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
        probe = engine_game([0] * 40, [0] * 40, seed=0).observation(0)  # what the installed engine reports
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
        for c in cards_:  # double-faced cards: the back face too (docs/api.md)
            if not c["token"] and c["name"] in BACKS:
                c["back_name"] = BACKS[c["name"]]
                c["back_image"] = back_image(c["name"])
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
            d = PracticeDraft(packs(), others, mine, kit.info.n, seed)
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

    supported = be.supported_mask(SET) if SET != "FDN" else None

    def commander_names(c) -> list[str]:
        names_ = [c] if isinstance(c, str) else list(c or [])
        if not 1 <= len(names_) <= 2:
            raise ApiError(422, "a commander is one name, or two for partners")
        return names_

    # Some deck lists truncate long double-faced names ("Etali, Primal Conqueror // Etali, Primal Sick"), and
    # the pool carries those as separate unscripted rows. Such a name means the one full name it begins.
    full_name: dict[str, str] = {}
    if supported is not None:
        names = list(kit.info.index)
        for n in names:
            if " // " in n and not supported[kit.info.index[n]]:
                longer = [m for m in names if m != n and m.startswith(n) and supported[kit.info.index[m]]]
                if len(longer) == 1:
                    full_name[n] = longer[0]

    def canon(n: str) -> str:
        return full_name.get(n, n)

    def check_commander_deck(
        commanders: list[str], cards_: list[Card], strict: bool = True
    ) -> tuple[list[int], list[int], dict]:
        """Ids for the commanders and the rest of the deck (a listed commander is taken out once),
        plus what's wrong: names outside the pool, cards not scripted yet, and deck problems."""
        commanders = [canon(n) for n in commanders]
        listed = []
        for c in cards_:
            listed += [canon(c.name)] * max(c.qty, 0)
        for n in commanders:
            if n in listed:
                listed.remove(n)
        unknown = sorted({n for n in listed + commanders if n not in kit.info.index})
        known = [n for n in listed if n in kit.info.index]
        cmd_ids = [kit.info.index[n] for n in commanders if n in kit.info.index]
        missing = sorted({n for n in known + [n for n in commanders if n in kit.info.index]
                          if supported is not None and not supported[kit.info.index[n]]})  # fmt: skip
        ids = [kit.info.index[n] for n in known]
        problems = [f"not a card in the pool: {n}" for n in unknown]
        if len(cmd_ids) == len(commanders):
            problems += be.commander_deck_problems(cmd_ids, ids, set=SET)
        warnings = []
        if not strict:  # casual tables: a slightly-off card count is the owner's to fix, not a block
            for p in list(problems):
                m = re.fullmatch(r"(\d+) cards with the commander, not 100", p)
                if m and abs(int(m.group(1)) - 100) <= CASUAL_SIZE_SLACK:
                    problems.remove(p)
                    warnings.append(p)
        # A name outside the pool is also a card still to script (and fetch): list it in both.
        return cmd_ids, ids, {"problems": problems, "warnings": warnings,
                              "missing": sorted(set(missing) | set(unknown))}  # fmt: skip

    @router.post("/deck/validate")
    def deck_validate(body: ValidateBody):
        if body.format != "commander":
            raise ApiError(422, "format must be 'commander'")
        _, _, found = check_commander_deck(commander_names(body.commander), body.deck, body.strict)
        return {"ok": not found["problems"] and not found["missing"], **found}

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

    def purge():  # under games_lock
        now = time.time()
        for gid in [g for g, r in games.items() if now - r["used"] > IDLE_SECONDS]:
            games.pop(gid, None)

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
            legal = g.legal_actions()
            n = len(legal)
            if who in rec["humans"]:
                # Tapping mana isn't a reason to stop: with nothing else to do, the pass is forced.
                real = [i for i, a in enumerate(legal) if a[0] != "TapForMana"]
                if len(real) > 1:
                    return
                apply(rec, real[0], gid)
            else:
                apply(rec, 0 if n == 1 else bot_move(rec, kit, lock), gid)
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
        with games_lock:
            purge()
            if gid not in games:
                raise ApiError(404, "no such game")
            rec = games[gid]
            rec["used"] = time.time()
            return rec

    @contextmanager
    def locked(gid: str):
        """The game `gid`, held for this request alone."""
        rec = get(gid)
        with rec["lock"]:
            yield rec

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
        if len(humans) == 1 and kit.net is None and body.pilot == "network":
            # No trained network for this set (CMD): the smart and rules pilots don't need one.
            raise ApiError(422, f"{SET} has no trained network yet: use pilot smart, smart_strong or rules")
        if len(humans) == 1 and body.pilot not in ("network", "smart", "smart_strong", "rules"):
            raise ApiError(422, "pilot must be network, smart, smart_strong or rules")
        commander = body.format == "commander"
        if commander:
            if len(body.auto_command_zone) != 2:
                raise ApiError(422, "auto_command_zone is two booleans, one per seat")
            sides = []
            for seat, (c, deck) in enumerate([(body.commander_a, body.deck_a), (body.commander_b, body.deck_b)]):
                ids_c, ids, found = check_commander_deck(commander_names(c), deck, body.strict)
                if found["missing"]:
                    raise ApiError(422, "cards the engine doesn't play yet", seat=seat, missing=found["missing"])
                if found["problems"]:
                    raise ApiError(422, "invalid_deck", seat=seat, problems=found["problems"])
                sides.append((ids_c, ids))
            (ca, a), (cb, b) = sides
        elif body.format is not None:
            raise ApiError(422, "format must be 'commander' (or left out for a 40-card game)")
        else:
            a, b = deck_ids(body.deck_a), deck_ids(body.deck_b)
        seed = body.seed if body.seed is not None else secrets.randbelow(2**31)
        first = body.first if body.first in (0, 1) else seed % 2
        with games_lock:
            purge()
            if len(games) >= max_games:
                raise ApiError(429, "too many games", limit=max_games)
        gid = "g_" + secrets.token_hex(6)
        game_lock = threading.Lock()
        with game_lock:
            if commander:
                game = be.Game.commander(a, b, ca, cb, seed, first=first, set=SET,
                                         starting_life=body.starting_life,
                                         auto_command_zone=tuple(body.auto_command_zone))  # fmt: skip
            else:
                game = engine_game(a, b, seed=seed, first=first)
            game.auto_pass = False  # forced passes go through `advance`, so frames can catch every moment
            game.smart_pay = True  # keep the colors the hand still needs (both seats)
            for s in body.manual_mana_seats:
                if s not in humans:
                    raise ApiError(422, f"manual_mana_seats: seat {s} isn't a human seat")
                game.set_manual_mana(s, True)
            rec = {"game": game, "human": humans[0], "humans": set(humans),
                   "pilot": body.pilot if len(humans) == 1 else None, "seed": seed, "version": 0, "moves": 0,
                   "events": [], "engine_n": 0, "used": time.time(), "seat_frames": {s: [] for s in humans},
                   "lock": game_lock}  # fmt: skip
            with games_lock:
                if len(games) >= max_games:  # another game was made meanwhile
                    raise ApiError(429, "too many games", limit=max_games)
                games[gid] = rec
            frames = [] if body.frames else None
            advance(rec, gid, frames, humans[0])
            out = {"game_id": gid, "human_seats": sorted(humans), "state": view(rec, gid)}
            if frames is not None:
                out["frames"] = trim(frames)
            return out

    @router.get("/game/{gid}")
    def show(gid: str, seat: int | None = None, spectate: bool = False):
        with locked(gid) as rec:
            if not spectate:
                return view(rec, gid, seat_of(rec, seat))
            # Read-only view of any seat, the bot's included (a streamer showing Bolt's hand). The caller
            # holds the service key and decides who may see it; nothing here can act for that seat.
            if seat not in (0, 1):
                raise ApiError(422, "spectate needs seat 0 or 1")
            s = view(rec, gid, seat)
            s["legal_actions"] = []
            s["spectate"] = True
            return s

    @router.post("/game/{gid}/mana")
    def set_mana(gid: str, body: ManaBody):
        """Switch a human seat between Auto (the engine pays) and Manual (TapForMana moves)."""
        with locked(gid) as rec:
            if body.seat not in rec["humans"]:
                raise ApiError(409, "not a human seat in this game", state=view(rec, gid))
            rec["game"].set_manual_mana(body.seat, body.manual)
            rec["version"] += 1
            advance(rec, gid, None, body.seat)
            return view(rec, gid, body.seat)

    @router.post("/game/{gid}/act")
    def act(gid: str, body: ActBody):
        with locked(gid) as rec:
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
        with locked(gid) as rec:
            seat = seat_of(rec, seat)
            fr = [f for f in rec["seat_frames"][seat] if f["version"] > since]
            return {"frames": trim(fr), "version": rec["version"]}

    @router.get("/game/{gid}/version")
    def version(gid: str):
        with locked(gid) as rec:
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
        with locked(gid) as rec:
            seat = seat_of(rec, body.seat)
            if finished(rec) is None and not rec["game"].is_over():
                rec["conceded"] = seat
                rec["version"] += 1
                rec["events"].append({"n": len(rec["events"]), "turn": rec["game"].turn, "kind": "Conceded",
                                      "player": seat})  # fmt: skip
            return view(rec, gid, seat)

    @router.get("/game/{gid}/log")
    def log(gid: str, since: int = 0, seat: int | None = None):
        with locked(gid) as rec:
            seat = seat_of(rec, seat)
            ev = [seat_event(e, seat) for e in rec["events"][max(0, since) :]]
            return {"events": ev, "next": len(rec["events"])}

    @router.delete("/game/{gid}")
    def end(gid: str):
        get(gid)
        with games_lock:
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
            drafts[did] = {"draft": PracticeDraft(packs(), bots, kit.bolt, kit.info.n, seed), "bots": body.bots,
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
            d = PracticeDraft(packs(), others, kit.bolt if first == "bolt" else kit.human, kit.info.n, seed)
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
