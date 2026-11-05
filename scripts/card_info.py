"""Fetch colors, rarity and types for every card in the set from 17lands' card ratings."""

import json
import urllib.parse
import urllib.request

from bolt.data import ASSETS, load_cards

SET = "FDN"
URL = (
    f"https://www.17lands.com/card_ratings/data?expansion={SET}&format=PremierDraft"
    "&start_date=2024-11-01&end_date=2024-12-31"
)
BASICS = {"Plains": "W", "Island": "U", "Swamp": "B", "Mountain": "R", "Forest": "G"}

rows = json.load(urllib.request.urlopen(urllib.request.Request(URL, headers={"User-Agent": "bolt-the-bird research"})))
by_name = {r["name"]: r for r in rows}
info = []
for name in load_cards(SET):
    if name in BASICS:
        info.append({"name": name, "color": "", "rarity": "basic", "types": ["Basic Land"]})
        continue
    r = by_name[name]
    info.append({"name": name, "color": r["color"], "rarity": r["rarity"], "types": r["types"]})
# Which colors each nonbasic land makes (17lands lists lands as colorless). Evolving Wilds -> any.
q = urllib.parse.urlencode({"q": f"set:{SET.lower()} type:land -type:basic", "unique": "cards"})
lands = json.load(
    urllib.request.urlopen(
        urllib.request.Request(
            "https://api.scryfall.com/cards/search?" + q,
            headers={"User-Agent": "BoltTheBird/0.1", "Accept": "application/json"},
        )
    )
)
produces = {c["name"]: "".join(m for m in "WUBRG" if m in (c.get("produced_mana") or "WUBRG")) for c in lands["data"]}
for i in info:
    if i["name"] in produces:
        i["produces"] = produces[i["name"]]
(ASSETS / f"{SET}_card_info.json").write_text(json.dumps(info, indent=0))
print(len(info), "cards;", sum(i["rarity"] in ("rare", "mythic") for i in info), "rares/mythics")
