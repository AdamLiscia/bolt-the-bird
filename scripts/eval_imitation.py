"""P2: the imitation player against the rules-based player, on pairs of human-model decks.

    python scripts/eval_imitation.py --pairs 500            # 1,000 games, plus a 1,000-game control
    python scripts/eval_imitation.py --pairs 4 --stub       # smoke test with stand-in models

Each pair of decks (512 human-model decks from the round-robins, deck i against deck i+1) is played
twice with the same seed: the imitation player with deck i, then with deck i+1, so deck strength
cancels. The control plays the rules-based player against itself the same way and should come out
near 50%. Games run in worker processes; results go to results/imitation_eval.json.
"""

import argparse
import json
import multiprocessing as mp
import time

import numpy as np

from bolt.data import ROOT

CK = ROOT / "checkpoints" / "turns"


class Stub:
    """A stand-in model for the smoke test: wants everything."""

    def __call__(self, zones, scalars, target):
        import torch

        x = zones[target]
        return torch.full(x.shape, 5.0).masked_fill(x == 0, float("-inf"))

    def eval(self):
        return self


def load_models(names: list[str], stub: bool):
    import bolt_engine as be
    import torch

    from bolt.turn_model import TurnModel

    if stub:
        return [Stub(), Stub(), Stub()]
    models = []
    for name in names:
        args = json.loads((ROOT / "results" / f"turns_{name}.json").read_text())["args"]
        m = TurnModel(len(be.card_names()), dim=args["dim"], layers=args["layers"])
        m.load_state_dict(torch.load(CK / name / "best.pt", map_location="cpu"))
        models.append(m.eval())
    return models


_PLAYER = None


def _init(names, stub, options=None):
    global _PLAYER
    import torch

    from bolt.imitation_player import ImitationPlayer

    torch.set_num_threads(1)
    _PLAYER = ImitationPlayer(*load_models(names, stub), **(options or {}))


def _game(job):
    from bolt.imitation_player import play_game

    a, b, seed, first, seat, control = job
    players = [None, None]
    if not control:
        players[seat] = _PLAYER
    r = play_game(a, b, seed=seed, first=first, players=players)
    return {"p0": 1.0, "p1": 0.0}.get(r, 0.5) if seat == 0 else {"p0": 0.0, "p1": 1.0}.get(r, 0.5), r


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pairs", type=int, default=500)
    ap.add_argument("--models", nargs=3, default=["cast1", "attack1", "block1"])
    ap.add_argument("--workers", type=int, default=6)
    ap.add_argument("--stub", action="store_true")
    ap.add_argument("--name", default="imitation_eval")
    ap.add_argument("--attack-threshold", type=float, default=None)
    ap.add_argument("--main2-fallback", action="store_true")
    a = ap.parse_args()
    lines = (ROOT / "data" / "m7" / "decks.txt").read_text().split("\n")
    decks = [list(map(int, x.split())) for x in lines[512:1024]]
    jobs = []
    for i in range(a.pairs):
        d1, d2 = decks[i % 512], decks[(i + 1) % 512]
        seed, first = 9_000 + i, i % 2
        for control in (False, True):
            # the player under test sits at seat 0 with d1, then at seat 1 with d1 (the other side has d2)
            jobs.append((d1, d2, seed, first, 0, control))
            jobs.append((d2, d1, seed, first, 1, control))
    options = {"attack_threshold": a.attack_threshold, "main2_fallback": a.main2_fallback}
    t0 = time.time()
    with mp.get_context("spawn").Pool(a.workers, initializer=_init, initargs=(a.models, a.stub, options)) as pool:
        res = pool.map(_game, jobs, chunksize=4)
    out = {"pairs": a.pairs, "models": a.models, "stub": a.stub, "options": options, "seconds": round(time.time() - t0)}
    for label, control in (("imitation_vs_rules", False), ("control_rules_vs_rules", True)):
        s = np.array([r[0] for r, j in zip(res, jobs, strict=True) if j[5] == control])
        kinds = [r[1] for r, j in zip(res, jobs, strict=True) if j[5] == control]
        out[label] = {
            "win_rate": float(s.mean()), "se": float(s.std() / np.sqrt(len(s))), "games": len(s),
            "draws_or_timeouts": sum(k in ("draw", "timeout") for k in kinds),
        }  # fmt: skip
    (ROOT / "results" / f"{a.name}.json").write_text(json.dumps(out, indent=2) + "\n")
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
