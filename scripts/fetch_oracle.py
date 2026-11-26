"""Fetch every card's rules text from Scryfall, in the set's model order (the 17lands draft data's
column order), to assets/{SET}_oracle.json: the engine's card list for that set.

    python scripts/fetch_oracle.py --set TDM
    python scripts/fetch_oracle.py --set BLC --names ~/bolt-handoff/blc.json

With --names, the card list is a JSON file instead of the draft data: a list of names, or an object
with "shared_order" (and "decks", whose commanders, then any other cards such as basic lands, are
added after it), as for a Commander precon set. Names are looked up 75 at a time with Scryfall's /cards/collection.

    python scripts/fetch_oracle.py --set PG --refresh "Vorinclex // The Grand Evolution"

With --refresh, only the named cards are fetched again and replaced where they are in the existing
assets/{SET}_oracle.json; everything else in it is kept as it is.

Fields as in assets/FDN_oracle.json. Two-faced cards keep their front face's characteristics
(name joined with " // " as 17lands writes it is matched on the front name); a transforming or
modal double-faced card also gets `back`: its back face's name, mana_cost, type_line, oracle_text,
power, toughness, colors and loyalty. A split card also gets `split`: its right half's name, mana_cost,
type_line and oracle_text.
"""

import argparse
import csv
import gzip
import json
import time
import urllib.parse
import urllib.request
from pathlib import Path

from bolt.data import ASSETS, DATA

KEEP = ("name", "mana_cost", "cmc", "type_line", "oracle_text", "power", "toughness", "keywords", "colors",
        "rarity", "layout", "loyalty", "produced_mana", "collector_number")  # fmt: skip
BACK = ("name", "mana_cost", "type_line", "oracle_text", "power", "toughness", "colors", "loyalty")
HEADERS = {"User-Agent": "BoltTheBird/0.4", "Accept": "application/json"}


def names_in_model_order(code: str) -> list[str]:
    with gzip.open(DATA / "raw" / f"draft_data_public.{code}.PremierDraft.csv.gz", "rt") as f:
        header = next(csv.reader(f))  # quoted: some names contain commas ("Ugin, Eye of the Storms")
    return [h.removeprefix("pack_card_") for h in header if h.startswith("pack_card_")]


def fetch_set(code: str) -> dict:
    out, url = (
        {},
        "https://api.scryfall.com/cards/search?"
        + urllib.parse.urlencode({"q": f"e:{code.lower()} (is:booster or t:basic)", "unique": "cards"}),
    )
    while url:
        page = json.load(urllib.request.urlopen(urllib.request.Request(url, headers=HEADERS)))
        for c in page["data"]:
            out.setdefault(c["name"], c)
            out.setdefault(c["name"].split(" // ")[0], c)
        url = page.get("next_page")
        time.sleep(0.15)  # Scryfall asks for 50-100 ms between requests
    return out


def names_from_file(path: str) -> list[str]:
    data = json.loads(Path(path).read_text())
    if isinstance(data, list):
        return data
    names = list(data["shared_order"])
    for deck in data.get("decks", []):
        for n in (deck.get("commander"), deck.get("partner")):
            if n and n not in names:
                names.append(n)
    for deck in data.get("decks", []):  # then anything else the decks hold (their basic lands)
        for c in deck.get("cards", []):
            if c["name"] not in names:
                names.append(c["name"])
    return names


def fetch_named(names: list[str]) -> dict:
    """Cards by name, 75 per request to /cards/collection (no auth), with Scryfall's requested pause."""
    out = {}
    for i in range(0, len(names), 75):
        ids = [{"name": n.split(" // ")[0]} for n in names[i : i + 75]]
        req = urllib.request.Request(
            "https://api.scryfall.com/cards/collection",
            data=json.dumps({"identifiers": ids}).encode(),
            headers={**HEADERS, "Content-Type": "application/json"},
        )
        page = json.load(urllib.request.urlopen(req))
        for c in page["data"]:
            out.setdefault(c["name"], c)
            out.setdefault(c["name"].split(" // ")[0], c)
        if page.get("not_found"):
            raise SystemExit(f"not found on Scryfall: {page['not_found']}")
        time.sleep(0.15)
    return out


def flat(c: dict) -> dict:
    face = c["card_faces"][0] if "card_faces" in c and "oracle_text" not in c else c
    d = {k: face.get(k, c.get(k)) for k in KEEP}
    d["name"] = c["name"]
    if c.get("layout") == "adventure" and len(c.get("card_faces", [])) > 1:
        # Tarkir's Omens use the adventure layout: the second face is the spell half.
        o = c["card_faces"][1]
        d["omen"] = {k: o.get(k) for k in ("name", "mana_cost", "type_line", "oracle_text")}
    if c.get("layout") in ("transform", "modal_dfc") and len(c.get("card_faces", [])) > 1:
        b = c["card_faces"][1]
        d["back"] = {k: b.get(k) for k in BACK}
        d["back"]["colors"] = b.get("colors") or []
    if c.get("layout") == "split" and len(c.get("card_faces", [])) > 1:
        # A split card (CR 709): the front fields are its left half, `split` its right half.
        s = c["card_faces"][1]
        d["split"] = {k: s.get(k) for k in ("name", "mana_cost", "type_line", "oracle_text")}
    return d


def refresh(code: str, names: list[str]) -> None:
    """Fetch `names` again and replace them in place in assets/{code}_oracle.json."""
    path = ASSETS / f"{code}_oracle.json"
    rows = json.loads(path.read_text())
    have = {r["name"] for r in rows}
    unknown = [n for n in names if n not in have]
    if unknown:
        raise SystemExit(f"not in {path.name}: {unknown}")
    cards = fetch_named(names)
    for i, r in enumerate(rows):
        if r["name"] in names:
            new = flat(cards.get(r["name"]) or cards[r["name"].split(" // ")[0]])
            new["name"] = r["name"]
            rows[i] = new
    path.write_text(json.dumps(rows, indent=0))
    print(f"{len(names)} cards refreshed in assets/{code}_oracle.json")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--set", required=True)
    ap.add_argument("--names", help="a JSON file of card names, instead of the draft data's card list")
    ap.add_argument("--refresh", nargs="+", help="fetch only these cards again, in place in the set's file")
    a = ap.parse_args()
    if a.refresh:
        refresh(a.set, a.refresh)
        return
    if a.names:
        names = names_from_file(a.names)
        cards = fetch_named(names)
    else:
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
