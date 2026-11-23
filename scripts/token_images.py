"""Writes assets/FDN_token_images.json: each engine token (card numbers 286+) with its Scryfall FDN token
printing (set tfdn), matched on name and power/toughness. Run with Scryfall's tfdn search results:

    python scripts/token_images.py tfdn.json
"""

import json
import sys
from pathlib import Path

import bolt_engine as be

ROOT = Path(__file__).resolve().parents[1]
# The engine's tokens in definition order (bolt-cards/src/scripts.rs): (power, toughness), None for noncreatures
STATS = [(1, 1), (3, 3), (1, 1), (4, 4), (1, 1), (1, 1), (1, 1), (1, 1), None, None, (1, 1), (1, 1), (5, 5),
         (8, 8), (3, 3), (3, 3), (2, 2), (1, 1), (1, 1), (1, 1), (2, 2), (2, 1), (2, 2)]  # fmt: skip


def main():
    printings = json.loads(Path(sys.argv[1]).read_text())
    names = be.card_names()
    first = 286
    assert len(names) - first == len(STATS), "the engine's token list changed: update STATS"
    out = []
    for k, stats in enumerate(STATS):
        name = names[first + k]
        pt = None if stats is None else (str(stats[0]), str(stats[1]))
        match = [
            c for c in printings if c["name"] == name and (pt is None or (c.get("power"), c.get("toughness")) == pt)
        ]
        assert match, f"no tfdn printing for {name} {stats}"
        c = sorted(match, key=lambda c: int(c["collector_number"]))[0]  # the lowest-numbered printing
        out.append({"card_id": first + k, "name": name, "power": stats[0] if stats else None,
                    "toughness": stats[1] if stats else None, "colors": "".join(c.get("colors", [])),
                    "types": c["type_line"].split(" — ")[0].replace("Token ", "").split(),
                    "image": c["image_uris"]["normal"], "scryfall_number": c["collector_number"]})  # fmt: skip
    (ROOT / "assets" / "FDN_token_images.json").write_text(json.dumps(out, indent=1) + "\n")
    print(f"{len(out)} tokens")


if __name__ == "__main__":
    main()
