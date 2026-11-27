"""Commander soak before the playgroup demo: every pair of playable decks plays full 1v1 games, bots on both
seats, and reports engine panics, positions with no legal actions, and games that never finish.

    PYTHONPATH=src ~/bolt-cmd-venv/bin/python scripts/debug/soak_cmd.py [games_per_pair] [max_steps]

Decks come from ~/bolt-handoff/playgroup_all.json (the 31) and lowgame_all.json (the idea decks), with their
quantities, filtered as scripts/make_cmd_decks.py filters them. Seat 0 uses the smart player, seat 1 the rules player, and the
pairing is played both ways round. Writes results/soak_cmd.json.
"""

import collections
import json
import sys
import time
from pathlib import Path

import bolt_engine as be

SET = "CMD"
HAND = Path.home() / "bolt-handoff"
ROOT = Path(__file__).resolve().parents[2]
names = be.card_names(SET)
supported = be.supported_mask(SET)  # by card number, tokens excluded
index = {n: i for i, n in enumerate(names)}
front = {n.split(" // ")[0]: i for i, n in enumerate(names)}


def card_id(n: str) -> int | None:
    if n in index:
        return index[n]
    return front.get(n.split(" // ")[0])


def decks() -> list[dict]:
    """The playable decks, built as scripts/make_cmd_decks.py builds the training file: quantities from the
    export's decks_qty, commanders from commanders_by_deck, every card supported, the Commander rules met
    apart from size, 98-102 cards."""
    sys.path.insert(0, str(ROOT / "scripts"))
    from make_cmd_decks import playable

    return [{"name": n, "cmd": c, "deck": d} for c, d, n in playable(SET, HAND)[0]]


KINDS = collections.Counter()  # every pending decision kind the bots met (the command-zone choice among them)
ENDS = collections.Counter()  # how games ended (commander damage among them)


def play(a: dict, b: dict, seed: int, max_steps: int) -> dict:
    g = be.Game.commander(a["deck"], b["deck"], a["cmd"], b["cmd"], seed, first=seed % 2, set=SET)
    for step in range(max_steps):
        if g.is_over():
            ENDS[str(g.result())[:80]] += 1
            return {"result": "over", "steps": step}
        KINDS[str(g.pending())[:40]] += 1
        legal = g.legal_actions()
        if not legal:
            o = g.observation(g.to_act())
            return {"result": "no_legal_actions", "steps": step, "pending": str(o.get("pending"))[:120]}
        i = g.smart(1, 2, seed) if g.to_act() == 0 else g.baseline()
        g.step(i)
    return {"result": "step_cap", "steps": max_steps}


def main():
    per_pair = int(sys.argv[1]) if len(sys.argv) > 1 else 2
    max_steps = int(sys.argv[2]) if len(sys.argv) > 2 else 20000
    ds = decks()
    print(f"{len(ds)} playable decks: {[d['name'] for d in ds]}", flush=True)
    tally, problems, t0 = collections.Counter(), [], time.time()
    for i, a in enumerate(ds):
        for j, b in enumerate(ds):
            if i == j:
                continue
            for k in range(per_pair):
                seed = 1000 * i + 10 * j + k
                try:
                    r = play(a, b, seed, max_steps)
                except BaseException as e:  # noqa: BLE001 - an engine panic surfaces as PanicException
                    if isinstance(e, KeyboardInterrupt):
                        raise
                    r = {"result": "panic", "error": str(e)[:200]}
                tally[r["result"]] += 1
                if r["result"] != "over":
                    problems.append({"a": a["name"], "b": b["name"], "seed": seed, **r})
                    print(
                        f"  {r['result']}: {a['name']} vs {b['name']} seed {seed} {r.get('error') or r.get('pending') or ''}",
                        flush=True,
                    )
        print(f"after {a['name']}: {dict(tally)} ({time.time() - t0:.0f}s)", flush=True)
    out = ROOT / "results" / "soak_cmd.json"
    out.parent.mkdir(exist_ok=True)
    out.write_text(json.dumps({"decks": [d["name"] for d in ds], "tally": tally, "problems": problems,
                               "pending_kinds": KINDS.most_common(), "endings": ENDS.most_common()}, indent=1))  # fmt: skip
    print("endings:", ENDS.most_common(8))
    print("decision kinds:", [k for k, _ in KINDS.most_common()])
    print(f"done: {dict(tally)}; {len(problems)} problems -> {out}")


if __name__ == "__main__":
    main()
