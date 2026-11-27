"""Plays many games at once with the play network (Phase 4, P3b; docs/play/P3_DESIGN.md).

`run_games` keeps a batch of games in lockstep. Seats are "net" (the network chooses), "rules" (the
rules-based player) or "smart" (the smart player, via Game.smart with the seed the engine's own smart
games use, so its games match the engine's exactly). Whenever games wait on the network, all their
decisions go through one forward pass. Moves with one legal option are played without asking.

Recorded steps (the network's seats by default, or the seats in `record`, e.g. the smart player as a
teacher for distillation) carry the encoding, the chosen move, and, for network moves, its log
probability and the value estimate. `run_parallel` splits the jobs across worker processes; a worker
that dies (the engine's rare memory corruption) has its share rerun in a fresh process.
"""

import contextlib
import multiprocessing as mp
import os
from concurrent.futures import ProcessPoolExecutor
from concurrent.futures.process import BrokenProcessPool

import bolt_engine as be
import torch

from bolt.play_encoding import collate, encode

# The card set games are played in (BOLT_PLAY_SET, default FDN: the engine's original call).
PLAY_SET = os.environ.get("BOLT_PLAY_SET", "FDN").upper()
SET_KW = {} if PLAY_SET == "FDN" else {"set": PLAY_SET}
# BOLT_PLAY_FORMAT=commander: 1v1 Commander (be.Game.commander, 40 life), each deck a (commanders, cards)
# pair (bolt.cmd_decks); otherwise the 40-card game, each deck a list of card numbers.
PLAY_FORMAT = os.environ.get("BOLT_PLAY_FORMAT", "limited").lower()


def new_game(a, b, seed, first):
    """The engine game for one job's decks, in PLAY_FORMAT."""
    if PLAY_FORMAT == "commander":
        return be.Game.commander(a[1], b[1], a[0], b[0], seed, first=first, set=PLAY_SET)
    return be.Game(a, b, seed=seed, first=first, **SET_KW)


MAX_TURNS = 200  # the engine's own limit (bolt-cards play.rs)
# A player can loop inside one turn (activating an ability forever), where the turn limit never
# triggers; normal games take a few hundred moves. Capped games count as timeouts.
MAX_MOVES = 20_000


def _result(g) -> str:
    return g.result()


@torch.no_grad()
def _forward(net, encs, chunk=256):
    logits, values = [], []
    for i in range(0, len(encs), chunk):
        lg, v = net(collate(encs[i : i + chunk]))
        logits += list(lg)
        values += v.tolist()
    return logits, values


def run_games(net, jobs, seats=("net", "rules"), sample=True, seed=0, smart=(1, 2), record=None,
              max_turns=MAX_TURNS, game_offset=0, max_moves=MAX_MOVES, label_with=None, opp_net=None):  # fmt: skip
    """jobs: (deck_a, deck_b, seed, first), a Commander deck being (commanders, cards). Returns
    {"results": [{"result", "turn"}], "steps": [...]}.

    label_with=(rollouts, candidates): recorded network steps get the smart player's choice in that
    position as "action" (the label) and the network's own move as "played", which is what the game
    follows (DAgger: the teacher labels the positions the student reaches)."""
    record = {i for i, s in enumerate(seats) if s == "net"} if record is None else set(record)
    # seat "opp": another network (a past checkpoint) samples its moves; it is never recorded
    gen = torch.Generator().manual_seed(seed)
    games = [new_game(a, b, s, f) for a, b, s, f in jobs]
    moves = [0] * len(games)
    results: list = [None] * len(games)
    steps = []
    live = list(range(len(games)))

    def play(gi, i):
        games[gi].step(i)
        moves[gi] += 1

    while live:
        waiting = []
        for gi in live:
            g = games[gi]
            try:
                while True:
                    if g.is_over():
                        results[gi] = {"result": _result(g), "turn": g.turn}
                        break
                    if g.turn > max_turns or moves[gi] >= max_moves:
                        results[gi] = {"result": "timeout", "turn": g.turn}
                        break
                    who = g.to_act()
                    seat = seats[who]
                    if seat in ("net", "opp"):
                        obs = g.observation(who)
                        if not obs["legal_actions"]:  # an engine bug: nothing legal to do
                            results[gi] = {"result": "error", "turn": g.turn}
                            break
                        if len(obs["legal_actions"]) == 1:
                            play(gi, 0)
                            continue
                        waiting.append((gi, who, encode(obs, who)))
                        break
                    if seat == "rules":
                        i = g.baseline()
                    else:
                        i = g.smart(smart[0], smart[1], (jobs[gi][2] * 1_000_003 + moves[gi]) % 2**64)
                    if who in record:
                        obs = g.observation(who)
                        if len(obs["legal_actions"]) > 1:
                            steps.append({"game": game_offset + gi, "seat": who, "enc": encode(obs, who),
                                          "action": i, "logp": None, "value": None})  # fmt: skip
                    play(gi, i)
            except BaseException as e:  # an engine error (a Rust panic is a BaseException) ends that game
                if isinstance(e, (KeyboardInterrupt, SystemExit)):
                    raise
                results[gi] = {"result": "error", "turn": g.turn}
        if waiting:
            mine = [w for w in waiting if seats[w[1]] == "net"]
            theirs = [w for w in waiting if seats[w[1]] == "opp"]
            logits, values = _forward(net, [e for _, _, e in mine]) if mine else ([], [])
            if theirs:
                lo, vo = _forward(opp_net, [e for _, _, e in theirs])
                logits, values = logits + lo, values + vo
            for (gi, who, enc), lg, v in zip(mine + theirs, logits, values, strict=True):
                lp = torch.log_softmax(lg[: len(enc["act_kind"])], 0)
                i = int(torch.multinomial(lp.exp(), 1, generator=gen)) if sample else int(lp.argmax())
                if who in record and seats[who] == "net":
                    step = {"game": game_offset + gi, "seat": who, "enc": enc, "action": i, "logp": float(lp[i]),
                            "value": v}  # fmt: skip
                    if label_with is not None:
                        seed_ = (jobs[gi][2] * 1_000_003 + moves[gi]) % 2**64
                        step["played"] = i
                        try:
                            step["action"] = games[gi].smart(label_with[0], label_with[1], seed_)
                        except BaseException as e:  # a Rust panic too
                            if isinstance(e, (KeyboardInterrupt, SystemExit)):
                                raise
                            step = None
                    if step is not None:
                        steps.append(step)
                try:
                    play(gi, i)
                except BaseException as e:  # a Rust panic too
                    if isinstance(e, (KeyboardInterrupt, SystemExit)):
                        raise
                    results[gi] = {"result": "error", "turn": games[gi].turn}
        live = [gi for gi in live if results[gi] is None]
    return {"results": results, "steps": steps}


def _die_with_parent(parent: int) -> None:
    """Pool initializer: a worker whose parent is gone exits instead of playing on (2026-11-18:
    workers of killed evaluations looped for two hours at full CPU)."""
    import os
    import threading
    import time

    def watch():
        while True:
            time.sleep(1)
            if os.getppid() != parent:
                os._exit(1)

    threading.Thread(target=watch, daemon=True).start()


def _load(net_state):
    if net_state is None:
        return None
    from bolt.play_net import load_checkpoint

    config, state = net_state
    return load_checkpoint({"config": config, "model": state}).eval()


def _state(net):
    return None if net is None else (net.config, {k: v.cpu() for k, v in net.state_dict().items()})


def _worker(args):
    torch.set_num_threads(1)
    net_state, jobs, kw = args
    compact = kw.pop("compact", False)
    net = _load(net_state)
    if "opp_net" in kw:
        kw["opp_net"] = _load(kw["opp_net"])
    out = run_games(net, jobs, **kw)
    if compact:  # arrays, not Python dicts: ~30x smaller to send back and to hold
        from bolt.play_data import to_arrays

        off = kw.get("game_offset", 0)
        for st in out["steps"]:
            st["game"] -= off
        arr = to_arrays(out["steps"], out["results"])
        arr["game"] += off
        out = {"results": out["results"], "arrays": arr}
    return out


def run_parallel(net, jobs, workers=6, chunk=None, seed=0, retries=2, compact=False, **kw):
    """run_games split across spawn worker processes; results in job order. With compact=True the
    workers return recorded steps as bolt.play_data arrays ("arrays") instead of "steps"."""
    net_state = _state(net)
    if kw.get("opp_net") is not None:
        kw["opp_net"] = _state(kw["opp_net"])
    chunk = chunk or max(1, -(-len(jobs) // workers))
    parts = {k: jobs[k : k + chunk] for k in range(0, len(jobs), chunk)}
    done: dict = {}
    for _ in range(retries + 1):
        todo = [k for k in parts if k not in done]
        if not todo:
            break
        with ProcessPoolExecutor(workers, mp_context=mp.get_context("spawn"), initializer=_die_with_parent,
                                 initargs=(os.getpid(),)) as ex:  # fmt: skip
            futs = {k: ex.submit(_worker, (net_state, parts[k], {**kw, "seed": seed + k, "game_offset": k, "compact": compact}))
                    for k in todo}  # fmt: skip
            for k, f in futs.items():
                with contextlib.suppress(BrokenProcessPool):  # rerun in a fresh pool
                    done[k] = f.result()
    missing = [k for k in parts if k not in done]
    if missing:
        raise RuntimeError(f"worker processes kept dying on job chunks {missing}")
    keys = sorted(done)
    results = [r for k in keys for r in done[k]["results"]]
    if compact:
        from bolt.play_data import concat

        return {"results": results, "arrays": concat([done[k]["arrays"] for k in keys])}
    return {"results": results, "steps": [st for k in keys for st in done[k]["steps"]]}
