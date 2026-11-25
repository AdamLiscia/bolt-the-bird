"""Check that every public Bolt writeup carries the current headline numbers and none of the stale ones.

    python scripts/check_writeups.py

The numbers live in docs/headline.json; the list of writeups is there too (docs/WRITEUPS.md explains
each). Exit code 1 if anything is missing or stale. It checks numbers, not prose: read the diff too.
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
    spec = json.loads((ROOT / "docs" / "headline.json").read_text())
    facts, bad = spec["facts"], 0
    for rel, needed in spec["writeups"].items():
        path = Path(rel).expanduser() if rel.startswith("~") else ROOT / rel
        if not path.exists():
            print(f"  skip  {rel} (not on this machine)")
            continue
        raw = path.read_text()
        # What a reader sees: drop HTML comments and markdown note lines ("> ..."), join wrapped lines.
        text = re.sub(r"<!--.*?-->", " ", raw, flags=re.S)
        text = "\n".join(line for line in text.split("\n") if not line.lstrip().startswith(">"))
        text = re.sub(r"\s+", " ", text)
        problems = []
        for name in needed:
            f = facts[name]
            if not any(s in text for s in f["any_of"]):
                problems.append(f"missing {name} ({f['value']})")
        for name, f in facts.items() if rel in spec.get("headline_only", []) else []:
            for s in f.get("stale", []):
                if s in text:
                    problems.append(f"stale '{s}' ({name} is now {f['value']})")
        bad += bool(problems)
        print(f"  {'ok  ' if not problems else 'FIX '}  {rel}" + "".join(f"\n          - {p}" for p in problems))
    print("All writeups current." if not bad else f"{bad} writeup(s) need an update.")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
