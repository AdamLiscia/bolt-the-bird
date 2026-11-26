"""Random play on the Tarkir sealed decks: reports positions with no legal actions, and engine panics.

    PYTHONPATH=src ~/bolt-tdm-venv/bin/python scripts/debug/random_play_tdm.py [games]

Found 2026-11-26 by the Tarkir PPO smoke test: (1) Pending ChooseTarget with no legal targets (Piercing Exhale:
its second target excludes the creature chosen for the first, though CR 115.3 allows the same object for a different
"target"); (2) a "payable" panic in pay_mana (a cost judged payable that then can't be paid).
"""

import random
import sys
from pathlib import Path

import bolt_engine as be

names = be.card_names("TDM")
lines = Path("data/tdm/sealed_decks.txt").read_text().strip().split("\n")
n = int(sys.argv[1]) if len(sys.argv) > 1 else 400
empty, panics = [], []
for k in range(n):
    a = list(map(int, lines[k % 512].split()))
    b = list(map(int, lines[512 + k % 512].split()))
    g = be.Game(a, b, seed=k, set="TDM")
    rng = random.Random(k)
    hist = []
    try:
        for _ in range(4000):
            if g.is_over():
                break
            legal = g.legal_actions()
            if not legal:
                o = g.observation(g.to_act())
                ch = o.get("choosing")
                empty.append((k, names[ch["card"]] if ch else None, o["pending"]))
                break
            i = rng.randrange(len(legal))
            hist.append(legal[i])
            g.step(i)
    except BaseException as e:  # noqa: BLE001 - an engine panic surfaces as PanicException
        panics.append((k, str(e)[:80], hist[-3:]))
print(f"{n} games: {len(empty)} with no legal actions, {len(panics)} panics")
for x in empty[:10]:
    print("  empty:", x)
for x in panics[:10]:
    print("  panic:", x)
