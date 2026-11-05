"""Add starting loyalty to the planeswalkers in assets/FDN_oracle.json, from Scryfall.

    python scripts/fetch_loyalty.py

The oracle file was written without Scryfall's `loyalty` field; the rules engine needs it.
"""

import json
import time
import urllib.parse
import urllib.request

from bolt.data import ASSETS

API = "https://api.scryfall.com/cards/named?set=fdn&exact="


def main():
    path = ASSETS / "FDN_oracle.json"
    cards = json.loads(path.read_text())
    for c in cards:
        if "Planeswalker" not in c["type_line"]:
            continue
        req = urllib.request.Request(
            API + urllib.parse.quote(c["name"]),
            headers={"User-Agent": "bolt-the-bird/0.2", "Accept": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=30) as r:
            c["loyalty"] = json.load(r)["loyalty"]
        print(f"{c['name']}: {c['loyalty']}")
        time.sleep(0.1)  # Scryfall asks for 50-100 ms between requests
    path.write_text(json.dumps(cards, indent=0))


if __name__ == "__main__":
    main()
