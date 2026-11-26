"""Sealed decks for training a play network: packs made by the set's slot rules, pools built by simple_build.

    python scripts/make_sealed_decks.py --set TDM --decks 1024 --out data/tdm/sealed_decks.txt

No human or 17lands data goes in (Adam, 2026-11-25): every pack is random within its slots, and the deck is
built by the same fixed rule the Tarkir service uses. One deck per line, as engine card numbers of the set
(the format train_ppo_play.py reads through BOLT_DECKS); lines 0-511 train, 512-1023 evaluate.

A pack approximates a Play Booster (14 cards):
  - 7 commons (no lands)
  - 3 uncommons
  - 1 rare, or a mythic one time in 8
  - 1 land: a common land (the gain lands, Evolving Wilds) half the time, else a basic
  - 2 wildcards of any rarity, weighted common 55%, uncommon 30%, rare 12%, mythic 3%
A pool is 6 packs (sealed). Duplicates across packs are allowed, as in real boosters.
"""

import argparse
import random
import sys
from pathlib import Path

import bolt_engine as be

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from bolt.cards import CardInfo  # noqa: E402
from bolt.service import simple_build  # noqa: E402

BASIC = {"W": "Plains", "U": "Island", "B": "Swamp", "R": "Mountain", "G": "Forest"}
WILD = [("common", 0.55), ("uncommon", 0.30), ("rare", 0.12), ("mythic", 0.03)]


def slots(info: CardInfo) -> dict[str, list[int]]:
    def is_land(c):
        return "Land" in " ".join(c["types"])

    by: dict[str, list[int]] = {"common": [], "uncommon": [], "rare": [], "mythic": [], "land": [], "basic": []}
    for i, c in enumerate(info.info):
        r = c["rarity"]
        if r == "basic":
            by["basic"].append(i)
        elif r == "common" and is_land(c):
            by["land"].append(i)
        else:
            by[r].append(i)
    return by


def pack(rng: random.Random, by: dict[str, list[int]]) -> list[int]:
    p = rng.sample(by["common"], 7) + rng.sample(by["uncommon"], 3)
    p.append(rng.choice(by["mythic"] if rng.random() < 1 / 8 else by["rare"]))
    p.append(rng.choice(by["land"] if rng.random() < 0.5 else by["basic"]))
    for _ in range(2):
        rarity = rng.choices([r for r, _ in WILD], [w for _, w in WILD])[0]
        p.append(rng.choice(by[rarity]))
    return p


def deck_numbers(deck: dict, names: list[str]) -> list[int]:
    index = {n: i for i, n in enumerate(names)}
    out = []
    for e in deck["main"]:
        out += [index[e["name"]]] * e["qty"]
    for col, n in deck["lands"].items():
        out += [index[BASIC[col]]] * n
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--set", default="TDM")
    ap.add_argument("--decks", type=int, default=1024)
    ap.add_argument("--packs", type=int, default=6)
    ap.add_argument("--seed", type=int, default=20261125)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    info = CardInfo(a.set)
    names = be.card_names(a.set)
    assert all(info.info[i]["name"] == names[i] for i in range(info.n)), "card info must match engine numbering"
    by = slots(info)
    rng = random.Random(a.seed)
    lines = []
    for _ in range(a.decks):
        pool = [c for _ in range(a.packs) for c in pack(rng, by)]
        deck = deck_numbers(simple_build(info, pool), names)
        assert len(deck) == 40, len(deck)
        lines.append(" ".join(map(str, deck)))
    out = Path(a.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n")
    print(f"{a.decks} {a.set} sealed decks ({a.packs} packs each) -> {out}")


if __name__ == "__main__":
    main()
