"""Builds assets/CMD_oracle.json: the Commander pool, every card the engine knows across sets.

    python scripts/make_cmd_set.py

FDN, TDM, BLC, then the playgroup cards (PG), each name once (first set wins). Scripts are looked up
by name, so a card scripted for any set plays here too. Rerun after fetching more PG cards.
"""

import json
import urllib.parse

from make_set_assets import card_types  # noqa: E402 (scripts/ is on the path when run as a script)

from bolt.data import ASSETS

SOURCES = ["FDN", "TDM", "BLC", "PG"]


def main():
    seen, out = set(), []
    for code in SOURCES:
        for c in json.loads((ASSETS / f"{code}_oracle.json").read_text()):
            if c["name"] not in seen:
                seen.add(c["name"])
                out.append(c)
    (ASSETS / "CMD_oracle.json").write_text(json.dumps(out, indent=1))
    # The service's per-set files (as scripts/make_set_assets.py writes for TDM; no packs: no drafting).
    info = []
    for c in out:
        basic = c["type_line"].startswith("Basic Land")
        colors = "".join(x for x in "WUBRG" if x in c.get("colors", []))
        info.append({"name": c["name"], "color": colors, "rarity": "basic" if basic else c["rarity"],
                     "types": card_types(c["type_line"])})  # fmt: skip
    (ASSETS / "CMD_card_info.json").write_text(json.dumps(info, indent=1))

    def image(name: str) -> str:
        q = urllib.parse.urlencode({"exact": name, "format": "image", "version": "normal"})
        return f"https://api.scryfall.com/cards/named?{q}"

    (ASSETS / "CMD_card_images.json").write_text(json.dumps({c["name"]: image(c["name"]) for c in out}, indent=1))
    print(f"CMD: {len(out)} cards from {', '.join(SOURCES)}")


if __name__ == "__main__":
    main()
