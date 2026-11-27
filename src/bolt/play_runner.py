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

import multiprocessing as mp
import os
import time

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
# Commander: 100-card decks at 40 life run longer per move, and one runaway game holds up its whole worker
# chunk (2026-11-27, the desktop's first cmd_play_1 iteration). Real 1v1 games end long before turn 80;
# capped games count as timeouts (a draw).
CMD_MAX_TURNS = 80
CMD_MAX_MOVES = 6_000


# A worker's heartbeat (set by _proc): a shared array, and this worker's slot in it, holding the time of
# its last engine call and the game (index in its chunk) the call was for. The parent kills a worker whose
# heartbeat stops for HANG_S seconds: a single engine call that never returns (2026-11-27, cmd_play_1: a
# worker 9.5 minutes at full CPU inside one call, so no move cap ever got a turn).
_BEAT = None
HANG_S = float(os.environ.get("BOLT_HANG_S", "120"))
HANG_LOG = os.environ.get("BOLT_HANG_LOG")  # where hung games are written (one JSON line each), if set


def _beat(gi: int) -> None:
    if _BEAT is not None:
        arr, slot = _BEAT
        arr[2 * slot] = time.time()
        arr[2 * slot + 1] = gi


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
              max_turns=None, game_offset=0, max_moves=None, label_with=None, opp_net=None, progress=0,
              skip=()):  # fmt: skip
    """jobs: (deck_a, deck_b, seed, first), a Commander deck being (commanders, cards). Returns
    {"results": [{"result", "turn"}], "steps": [...]}. smart=(rollouts, candidates[, ms]): the smart seat's
    search, ms capping its time per move.

    label_with=(rollouts, candidates): recorded network steps get the smart player's choice in that
    position as "action" (the label) and the network's own move as "played", which is what the game
    follows (DAgger: the teacher labels the positions the student reaches).

    skip: games (indices into jobs) not played: an earlier run hung on them. Each is a timeout with no steps."""
    record = {i for i, s in enumerate(seats) if s == "net"} if record is None else set(record)
    commander = PLAY_FORMAT == "commander"
    if max_turns is None:
        max_turns = CMD_MAX_TURNS if commander else MAX_TURNS
    if max_moves is None:
        max_moves = CMD_MAX_MOVES if commander else MAX_MOVES
    # seat "opp": another network (a past checkpoint) samples its moves; it is never recorded
    gen = torch.Generator().manual_seed(seed)
    skip = set(skip)
    games = []
    for gi, (a, b, s, f) in enumerate(jobs):
        _beat(gi)
        games.append(None if gi in skip else new_game(a, b, s, f))
    moves = [0] * len(games)
    results: list = [{"result": "timeout", "turn": 0, "hung": True} if gi in skip else None for gi in range(len(games))]
    steps = []
    live = [gi for gi in range(len(games)) if gi not in skip]
    t0 = time.time()

    def play(gi, i):
        _beat(gi)
        games[gi].step(i)
        moves[gi] += 1

    while live:
        waiting = []
        for gi in live:
            g = games[gi]
            try:
                while True:
                    _beat(gi)
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
                        i = g.smart(smart[0], smart[1], (jobs[gi][2] * 1_000_003 + moves[gi]) % 2**64, *smart[2:])
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
            _beat(-1)  # the network's forward pass: no game's engine call
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
                            _beat(gi)
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
        before = len(games) - len(live)
        live = [gi for gi in live if results[gi] is None]
        done = len(games) - len(live)
        if progress and (done // progress > before // progress or (done == len(games) and done != before)):
            # a stalled game shows as a worker that stops reporting (2026-11-27, the desktop's cmd_play_1)
            slow = max(live, key=lambda gi: moves[gi], default=None)
            tail = (
                f"; longest still playing: game {game_offset + slow}, {moves[slow]} moves" if slow is not None else ""
            )
            print(f"[worker {os.getpid()}] {done}/{len(games)} games done, {time.time() - t0:.0f}s{tail}", flush=True)
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
    kw.setdefault("progress", 16)
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


def _proc(slot, beats, out_q, worker, key, args, parent):
    """One worker process: plays its chunk, heartbeat in beats[slot], and sends (key, result) back."""
    global _BEAT
    _die_with_parent(parent)
    _BEAT = (beats, slot)
    out_q.put((key, worker(args)))


def _hang_record(job, seats, kw, index):
    a, b, seed_, first = job
    return {"time": time.strftime("%Y-%m-%d %H:%M:%S"), "set": PLAY_SET, "format": PLAY_FORMAT, "job": index,
            "seed": seed_, "first": first, "seats": list(seats), "smart": list(kw.get("smart", (1, 2))),
            "sample": kw.get("sample", True), "deck_a": a, "deck_b": b}  # fmt: skip


def run_parallel(net, jobs, workers=6, chunk=None, seed=0, retries=2, compact=False, hang_s=None, hang_log=None,
                 worker=None, **kw):  # fmt: skip
    """run_games split across spawn worker processes; results in job order. With compact=True the
    workers return recorded steps as bolt.play_data arrays ("arrays") instead of "steps".

    A worker whose heartbeat stops for hang_s seconds (default HANG_S; 0 turns it off), an engine call that
    never returns, is killed, and its chunk rerun without that game: a timeout with no steps, written to
    hang_log (default BOLT_HANG_LOG) as one JSON line with its decks, seed, seats and pilot settings. A worker
    that dies (the engine's rare memory corruption) has its chunk rerun, up to `retries` times."""
    import collections
    import json
    import queue

    worker = worker or _worker
    hang_s = HANG_S if hang_s is None else hang_s
    hang_log = hang_log or HANG_LOG
    net_state = _state(net)
    if kw.get("opp_net") is not None:
        kw["opp_net"] = _state(kw["opp_net"])
    chunk = chunk or max(1, -(-len(jobs) // workers))
    parts = {k: jobs[k : k + chunk] for k in range(0, len(jobs), chunk)}
    ctx = mp.get_context("spawn")
    out_q = ctx.Queue()
    beats = ctx.RawArray("d", 2 * workers)
    pending = collections.deque(sorted(parts))
    running: dict = {}  # chunk key -> (process, slot)
    free = list(range(workers))
    done: dict = {}
    crashes: collections.Counter = collections.Counter()
    skip: dict = collections.defaultdict(set)
    seats = kw.get("seats", ("net", "rules"))

    def drain(wait):
        while True:
            try:
                key, out = out_q.get(timeout=wait)
            except queue.Empty:
                return
            done[key] = out
            wait = 0.05

    def release(k):
        p, slot = running.pop(k)
        p.join(timeout=10)
        if p.is_alive():
            p.kill()
            p.join()
        free.append(slot)

    while pending or running:
        while pending and free:
            k = pending.popleft()
            slot = free.pop()
            beats[2 * slot], beats[2 * slot + 1] = time.time(), -1
            args = (
                net_state,
                parts[k],
                {**kw, "seed": seed + k, "game_offset": k, "compact": compact, "skip": skip[k]},
            )
            p = ctx.Process(target=_proc, args=(slot, beats, out_q, worker, k, args, os.getpid()), daemon=True)
            p.start()
            running[k] = (p, slot)
        drain(1.0)
        for k in list(running):
            p, slot = running[k]
            if k in done:
                release(k)
            elif not p.is_alive():
                drain(0.5)  # a result sent just before it exited
                if k not in done:
                    release(k)
                    crashes[k] += 1
                    if crashes[k] > retries:
                        raise RuntimeError(f"worker processes kept dying on job chunks {[k]}")
                    pending.append(k)
            elif hang_s and time.time() - beats[2 * slot] > hang_s:
                gi = int(beats[2 * slot + 1])
                p.kill()
                release(k)
                if 0 <= gi < len(parts[k]) and gi not in skip[k]:
                    skip[k].add(gi)
                    rec = _hang_record(parts[k][gi], seats, kw, k + gi)
                    print(f"HANG: game {k + gi} (seed {rec['seed']}) stalled {hang_s:.0f}s; worker killed, chunk rerun "
                          f"without it", flush=True)  # fmt: skip
                    if hang_log:
                        with open(hang_log, "a") as f:
                            f.write(json.dumps(rec) + "\n")
                else:  # stalled outside a game's engine call: rerun the chunk as for a crash
                    crashes[k] += 1
                    if crashes[k] > retries:
                        raise RuntimeError(f"job chunk {k} kept stalling outside a game")
                pending.appendleft(k)
    keys = sorted(done)
    results = [r for k in keys for r in done[k]["results"]]
    if compact:
        from bolt.play_data import concat

        return {"results": results, "arrays": concat([done[k]["arrays"] for k in keys])}
    return {"results": results, "steps": [st for k in keys for st in done[k]["steps"]]}
