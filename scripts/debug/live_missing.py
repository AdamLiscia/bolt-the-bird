"""handoff/lanes/live_missing.json: every playgroup and idea deck's unsupported cards on the installed
engine, each with `pending_on`, the lane branches on origin whose card scripts already name it.

    PYTHONPATH=src .venv/bin/python scripts/debug/live_missing.py

Run it after `git fetch origin`, with the engine you are about to push installed. Deck lists come from
~/bolt-handoff exactly as scripts/make_cmd_decks.py reads them.
"""

import json
import subprocess
import sys
from pathlib import Path

import bolt_engine as be

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
from make_cmd_decks import HAND, SOURCES, deck_cards  # noqa: E402

SET = "CMD"
LANES = ["lane-d1", "lane-d2", "lane-d3", "tarkir-b", "tarkir-c", "tarkir-s", "tarkir-t"]


def scripted_on(branch: str) -> str:
    """Every card-script source file on origin/`branch`, as one string (empty when the branch is absent)."""
    ref = f"origin/{branch}"
    files = subprocess.run(["git", "ls-tree", "-r", "--name-only", ref, "engine/bolt-cards/src"],
                           cwd=ROOT, capture_output=True, text=True).stdout.split()  # fmt: skip
    return "\n".join(
        subprocess.run(["git", "show", f"{ref}:{f}"], cwd=ROOT, capture_output=True, text=True).stdout
        for f in files
        if f.endswith(".rs")
    )


def main():
    names = be.card_names(SET)
    supported = be.supported_mask(SET)
    index, front = {}, {}
    for i, n in enumerate(names):
        index.setdefault(n, i)
        front.setdefault(n.split(" // ")[0], i)
    lanes = {b: scripted_on(b) for b in LANES}
    head = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, capture_output=True, text=True)
    out = []
    for fname, cap in SOURCES:
        d = json.loads((HAND / fname).read_text())
        for x in d["deck_order"][:cap]:
            cmd, cards = deck_cards(d, x)
            missing = set()
            for n in cmd + cards:
                i = index.get(n, front.get(n.split(" // ")[0]))
                if i is None:
                    missing.add(f"{n} (not in the {SET} pool)")
                elif i >= len(supported):
                    missing.add(f"{names[i]} (a token in the decklist, not a card)")
                elif not supported[i]:
                    missing.add(names[i])
            rows = []
            for m in sorted(missing):
                key = f'"{m.split(" // ")[0]}'
                rows.append({"card": m, "pending_on": [b for b, src in lanes.items() if key in src]})
            out.append({"owner": x["owner"], "commander": cmd[0] if cmd else x.get("commander"),
                        "list": "31" if fname.startswith("playgroup") else "idea", "missing": rows})  # fmt: skip
    doc = {"engine": head.stdout.strip(),
           "note": "pending_on = lane branches on origin that already script the card (not merged yet)",
           "decks": out}  # fmt: skip
    path = ROOT / "handoff" / "lanes" / "live_missing.json"
    path.write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")
    done = sum(1 for e in out if not e["missing"])
    print(f"{len(out)} decks, {done} with nothing missing -> {path}")


if __name__ == "__main__":
    main()
