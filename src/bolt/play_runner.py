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
from concurrent.futures import ProcessPoolExecutor
from concurrent.futures.process import BrokenProcessPool

import bolt_engine as be
import torch

from bolt.play_encoding import collate, encode

MAX_TURNS = 200  # the engine's own limit (bolt-cards play.rs)


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
              max_turns=MAX_TURNS, game_offset=0):  # fmt: skip
    """jobs: (deck_a, deck_b, seed, first). Returns {"results": [{"result", "turn"}], "steps": [...]}."""
    record = {i for i, s in enumerate(seats) if s == "net"} if record is None else set(record)
    gen = torch.Generator().manual_seed(seed)
    games = [be.Game(a, b, seed=s, first=f) for a, b, s, f in jobs]
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
                    if g.turn > max_turns:
                        results[gi] = {"result": "timeout", "turn": g.turn}
                        break
                    who = g.to_act()
                    seat = seats[who]
                    if seat == "net":
                        obs = g.observation(who)
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
            except Exception:  # an engine error ends that game, not the batch
                results[gi] = {"result": "error", "turn": g.turn}
        if waiting:
            logits, values = _forward(net, [e for _, _, e in waiting])
            for (gi, who, enc), lg, v in zip(waiting, logits, values, strict=True):
                lp = torch.log_softmax(lg[: len(enc["act_kind"])], 0)
                i = int(torch.multinomial(lp.exp(), 1, generator=gen)) if sample else int(lp.argmax())
                if who in record:
                    steps.append({"game": game_offset + gi, "seat": who, "enc": enc, "action": i,
                                  "logp": float(lp[i]), "value": v})  # fmt: skip
                try:
                    play(gi, i)
                except Exception:
                    results[gi] = {"result": "error", "turn": games[gi].turn}
        live = [gi for gi in live if results[gi] is None]
    return {"results": results, "steps": steps}


def _worker(args):
    torch.set_num_threads(1)
    net_state, jobs, kw = args
    net = None
    if net_state is not None:
        from bolt.play_net import PlayNet

        config, state = net_state
        net = PlayNet(**config)
        net.load_state_dict(state)
        net.eval()
    return run_games(net, jobs, **kw)


def run_parallel(net, jobs, workers=6, chunk=None, seed=0, retries=2, **kw):
    """run_games split across spawn worker processes; results in job order."""
    net_state = None if net is None else (net.config, {k: v.cpu() for k, v in net.state_dict().items()})
    chunk = chunk or max(1, -(-len(jobs) // workers))
    parts = {k: jobs[k : k + chunk] for k in range(0, len(jobs), chunk)}
    done: dict = {}
    for _ in range(retries + 1):
        todo = [k for k in parts if k not in done]
        if not todo:
            break
        with ProcessPoolExecutor(workers, mp_context=mp.get_context("spawn")) as ex:
            futs = {k: ex.submit(_worker, (net_state, parts[k], {**kw, "seed": seed + k, "game_offset": k}))
                    for k in todo}  # fmt: skip
            for k, f in futs.items():
                with contextlib.suppress(BrokenProcessPool):  # rerun in a fresh pool
                    done[k] = f.result()
    missing = [k for k in parts if k not in done]
    if missing:
        raise RuntimeError(f"worker processes kept dying on job chunks {missing}")
    out = {"results": [], "steps": []}
    for k in sorted(done):
        out["results"] += done[k]["results"]
        out["steps"] += done[k]["steps"]
    return out
