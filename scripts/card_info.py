"""Fetch colors, rarity and types for every card in the set from 17lands' card ratings."""
import json
import sys
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))
from bolt.data import DATA, load_cards  # noqa: E402

SET = "FDN"
URL = (f"https://www.17lands.com/card_ratings/data?expansion={SET}&format=PremierDraft"
       "&start_date=2024-11-01&end_date=2024-12-31")
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
(DATA / f"{SET}_card_info.json").write_text(json.dumps(info, indent=0))
print(len(info), "cards;", sum(i["rarity"] in ("rare", "mythic") for i in info), "rares/mythics")
