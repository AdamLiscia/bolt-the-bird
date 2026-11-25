"""Builds assets/CMD_oracle.json: the Commander pool, every card the engine knows across sets.

    python scripts/make_cmd_set.py

FDN, TDM, BLC, then the playgroup cards (PG), each name once (first set wins). Scripts are looked up
by name, so a card scripted for any set plays here too. Rerun after fetching more PG cards.
"""

import json

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
    print(f"CMD: {len(out)} cards from {', '.join(SOURCES)}")


if __name__ == "__main__":
    main()
