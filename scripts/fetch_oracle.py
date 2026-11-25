"""Fetch every card's rules text from Scryfall, in the set's model order (the 17lands draft data's
column order), to assets/{SET}_oracle.json: the engine's card list for that set.

    python scripts/fetch_oracle.py --set TDM

Fields as in assets/FDN_oracle.json. Two-faced cards keep their front face's characteristics
(name joined with " // " as 17lands writes it is matched on the front name).
"""

import argparse
import csv
import gzip
import json
import time
import urllib.parse
import urllib.request

from bolt.data import ASSETS, DATA

KEEP = ("name", "mana_cost", "cmc", "type_line", "oracle_text", "power", "toughness", "keywords", "colors",
        "rarity", "layout", "loyalty", "produced_mana", "collector_number")  # fmt: skip
HEADERS = {"User-Agent": "BoltTheBird/0.4", "Accept": "application/json"}


def names_in_model_order(code: str) -> list[str]:
    with gzip.open(DATA / "raw" / f"draft_data_public.{code}.PremierDraft.csv.gz", "rt") as f:
        header = next(csv.reader(f))  # quoted: some names contain commas ("Ugin, Eye of the Storms")
    return [h.removeprefix("pack_card_") for h in header if h.startswith("pack_card_")]


def fetch_set(code: str) -> dict:
    out, url = {}, "https://api.scryfall.com/cards/search?" + urllib.parse.urlencode(
        {"q": f"e:{code.lower()} (is:booster or t:basic)", "unique": "cards"}
    )
    while url:
        page = json.load(urllib.request.urlopen(urllib.request.Request(url, headers=HEADERS)))
        for c in page["data"]:
            out.setdefault(c["name"], c)
            out.setdefault(c["name"].split(" // ")[0], c)
        url = page.get("next_page")
        time.sleep(0.15)  # Scryfall asks for 50-100 ms between requests
    return out


def flat(c: dict) -> dict:
    face = c["card_faces"][0] if "card_faces" in c and "oracle_text" not in c else c
    d = {k: face.get(k, c.get(k)) for k in KEEP}
    d["name"] = c["name"]
    if c.get("layout") == "adventure" and len(c.get("card_faces", [])) > 1:
        # Tarkir's Omens use the adventure layout: the second face is the spell half.
        o = c["card_faces"][1]
        d["omen"] = {k: o.get(k) for k in ("name", "mana_cost", "type_line", "oracle_text")}
    return d


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--set", required=True)
    a = ap.parse_args()
    names = names_in_model_order(a.set)
    cards = fetch_set(a.set)
    missing = [n for n in names if n not in cards and n.split(" // ")[0] not in cards]
    for n in missing:  # not in the booster search (e.g. a bonus sheet): look it up by name
        q = urllib.parse.urlencode({"exact": n.split(" // ")[0]})
        cards[n] = json.load(urllib.request.urlopen(urllib.request.Request(
            "https://api.scryfall.com/cards/named?" + q, headers=HEADERS)))  # fmt: skip
        time.sleep(0.15)
    rows = [flat(cards.get(n) or cards[n.split(" // ")[0]]) for n in names]
    for n, r in zip(names, rows, strict=True):
        r["name"] = n  # the model's spelling
    (ASSETS / f"{a.set}_oracle.json").write_text(json.dumps(rows, indent=0))
    print(f"{len(rows)} cards -> assets/{a.set}_oracle.json ({len(missing)} looked up by name)")


if __name__ == "__main__":
    main()
