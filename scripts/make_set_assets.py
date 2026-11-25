"""Builds the service's per-set files for a set other than FDN, from its oracle data and 17lands picks.

    python scripts/make_set_assets.py --set TDM

Writes, in assets/:
- <SET>_card_info.json: name, color, rarity, types for each card, in the engine's (and the drafter's) order
- <SET>_sample_packs.npy: real opening packs (the first pick of each 17lands draft), ids + 1, 15 wide
- <SET>_card_images.json: card name -> Scryfall "normal" image URL (Scryfall's by-name image redirect,
  so nothing is downloaded here)
"""

import argparse
import json
import urllib.parse

import numpy as np

from bolt.data import ASSETS, DATA

COLOR_OF = {"White": "W", "Blue": "U", "Black": "B", "Red": "R", "Green": "G"}


def card_types(type_line: str) -> list[str]:
    front = type_line.split("//")[0].split("—")[0].strip()
    if front.startswith("Basic Land"):
        return ["Basic Land"]
    return [t for t in front.split() if t not in ("Legendary", "Snow", "World", "Basic")]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--set", required=True)
    ap.add_argument("--packs", type=int, default=3000)
    a = ap.parse_args()
    code = a.set.upper()
    oracle = json.loads((ASSETS / f"{code}_oracle.json").read_text())
    names = json.loads((ASSETS / f"{code}_cards.json").read_text())
    assert names == [c["name"] for c in oracle], "the drafter's card order must equal the engine's"

    info = []
    for c in oracle:
        basic = c["type_line"].startswith("Basic Land")
        colors = "".join(x for x in "WUBRG" if x in c.get("colors", []))
        info.append({"name": c["name"], "color": colors, "rarity": "basic" if basic else c["rarity"],
                     "types": card_types(c["type_line"])})  # fmt: skip
    (ASSETS / f"{code}_card_info.json").write_text(json.dumps(info, indent=1))

    z = np.load(DATA / f"{code}_picks.npz")
    first = np.flatnonzero((z["pick_number"] == 0) & (z["pack_number"] == 0))
    rng = np.random.default_rng(0)
    pick = rng.choice(first, size=min(a.packs, len(first)), replace=False)
    packs = z["pack"][np.sort(pick)].astype(np.int64)
    np.save(ASSETS / f"{code}_sample_packs.npy", packs)

    def image(name: str) -> str:
        q = urllib.parse.urlencode({"exact": name, "set": code.lower(), "format": "image", "version": "normal"})
        return f"https://api.scryfall.com/cards/named?{q}"

    (ASSETS / f"{code}_card_images.json").write_text(
        json.dumps({c["name"]: image(c["name"]) for c in oracle}, indent=1)
    )
    print(f"{code}: {len(info)} cards, {len(packs)} packs, {len(oracle)} image links")


if __name__ == "__main__":
    main()
