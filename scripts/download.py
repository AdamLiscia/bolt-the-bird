"""Download the 17lands public datasets for one set into data/raw/.

    python scripts/download.py                 # FDN Premier Draft: picks + games (~250 MB)
    python scripts/download.py --replays       # also the turn-by-turn replays (~440 MB)

Data: 17lands public datasets, CC BY 4.0. https://www.17lands.com/public_datasets
"""

import argparse
import sys
import urllib.request

from bolt.data import DATA

BASE = "https://17lands-public.s3.amazonaws.com/analysis_data"


def fetch(url: str, dest) -> None:
    if dest.exists():
        print(f"  have {dest.name}")
        return
    tmp = dest.with_suffix(dest.suffix + ".part")
    with urllib.request.urlopen(url) as r, open(tmp, "wb") as f:
        total = int(r.headers.get("Content-Length", 0))
        done = 0
        while chunk := r.read(1 << 20):
            f.write(chunk)
            done += len(chunk)
            if total:
                sys.stdout.write(f"\r  {dest.name}: {done / 1e6:,.0f} / {total / 1e6:,.0f} MB")
                sys.stdout.flush()
    tmp.rename(dest)  # only a complete download gets the real name
    print()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--set", default="FDN")
    ap.add_argument("--event", default="PremierDraft")
    ap.add_argument("--replays", action="store_true")
    a = ap.parse_args()
    raw = DATA / "raw"
    raw.mkdir(parents=True, exist_ok=True)
    kinds = ["draft_data", "game_data"] + (["replay_data"] if a.replays else [])
    for kind in kinds:
        name = f"{kind}_public.{a.set}.{a.event}.csv.gz"
        fetch(f"{BASE}/{kind}/{name}", raw / name)


if __name__ == "__main__":
    main()
